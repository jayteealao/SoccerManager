//! The possession decision. The ball carrier scores its options (shoot, pass to each open
//! team-mate, dribble, clear, hold) and takes the highest; the team's pressing instruction
//! sets how many opponents press and from how far; every other player holds its formation
//! anchor, which the team's plan moves. Every agent decides every tick (product-owner choice;
//! `Tuning::decision_interval_ticks`).
//! A sent-off player takes no part in any decision. While the ball is dead, the referee sets
//! every target instead, and a restart kick comes from `restart_pass`.

use crate::data::rules::StoppageKind;
#[cfg(feature = "scenario")]
use crate::gate::audit::{self, Candidate, Control};
use crate::math::{self, DVec2, segment_distance, toward};
use crate::pitch;
use crate::plugin::{DecisionContext, HookPoint, OptionOffsets};
use crate::rules::offside;
use crate::sim::{ScriptCache, Simulation, WIDE_SPREAD, wide_of_goal};
use crate::streams::{Action, Key};
use crate::team::PLAYERS_PER_TEAM;
use crate::tuning::Tuning;

/// A kick the carrier decided on this tick. A clearance is kicked away from danger to no
/// team-mate, and is counted apart from a pass.
#[derive(Debug, Clone, Copy)]
pub enum Kick {
    Pass { dir: DVec2, speed: f64, loft: f64 },
    Shot { dir: DVec2, speed: f64, loft: f64 },
    Clear { dir: DVec2, speed: f64, loft: f64 },
}

impl Kick {
    /// The direction, ground speed and vertical speed of the kick.
    pub fn flight(self) -> (DVec2, f64, f64) {
        match self {
            Kick::Pass { dir, speed, loft }
            | Kick::Shot { dir, speed, loft }
            | Kick::Clear { dir, speed, loft } => (dir, speed, loft),
        }
    }
}

impl Simulation {
    /// Sets every player's target for this tick and returns the carrier's kick, if any.
    pub(crate) fn decide(&mut self) -> Option<Kick> {
        let ball_xy = self.ball.xy();
        let n = self.players.len();
        let keepers = [self.keeper(0), self.keeper(1)];
        match self.carrier {
            Some(c) => {
                let team = self.players[c].team;
                let def = 1 - team;
                // Teammates hold their anchors; the nearest opponents press, as many and from
                // as far as the defending team's pressing instruction says, less one for each
                // player the defending team has lost.
                let count = self.teams[def].pressers().min(MAX_PRESSERS);
                let reach = self.teams[def].plan.press_distance;
                let mut pressers = [(f64::INFINITY, usize::MAX); MAX_PRESSERS];
                for i in 0..n {
                    let p = self.players[i];
                    if !p.active() {
                        continue;
                    }
                    let anchor = self.teams[p.team].anchor(p.slot, ball_xy, &self.config.tuning);
                    self.players[i].target = anchor;
                    if p.team != team && i != keepers[def] && count > 0 {
                        let d = (p.pos - ball_xy).length();
                        if d < reach && d < pressers[count - 1].0 {
                            // Insert in distance order among the first `count` places.
                            let mut k = count - 1;
                            while k > 0 && pressers[k - 1].0 > d {
                                pressers[k] = pressers[k - 1];
                                k -= 1;
                            }
                            pressers[k] = (d, i);
                        }
                    }
                }
                self.hold_the_line(team, c);
                // Pressers run at the point where they can meet the carrier's run, and go for the
                // ball once they are close.
                let run = self.players[c].vel;
                let engage = self.config.tuning.press_engage;
                for &(_, i) in &pressers[..count] {
                    if i != usize::MAX {
                        let p = self.players[i];
                        // Within engaging range the presser goes for the ball itself.
                        self.players[i].target = if (p.pos - ball_xy).length() < engage {
                            ball_xy
                        } else {
                            pitch::clamp(intercept(p.pos, p.max_speed(), ball_xy, run), 0.5)
                        };
                    }
                }
                self.cover(def, c, &pressers[..count], keepers[def]);
                self.decide_carrier(c)
            }
            None => {
                // The nearest player of each team chases; a goalkeeper chases a ball near its goal.
                let predicted = pitch::clamp(
                    ball_xy + DVec2::new(self.ball.vel.x, self.ball.vel.y) * 0.3,
                    0.3,
                );
                let mut nearest = [usize::MAX; 2];
                let mut nearest_dist = [f64::INFINITY; 2];
                for i in 0..n {
                    let p = self.players[i];
                    if !p.active() {
                        continue;
                    }
                    let anchor = self.teams[p.team].anchor(p.slot, ball_xy, &self.config.tuning);
                    self.players[i].target = anchor;
                    let d = (p.pos - ball_xy).length();
                    if i != keepers[p.team] && d < nearest_dist[p.team] {
                        nearest_dist[p.team] = d;
                        nearest[p.team] = i;
                    }
                }
                for (team, &i) in nearest.iter().enumerate() {
                    if i != usize::MAX {
                        self.players[i].target = predicted;
                    }
                    let gk = keepers[team];
                    if self.players[gk].active() && (self.players[gk].pos - ball_xy).length() < 16.0
                    {
                        self.players[gk].target = predicted;
                    }
                }
                None
            }
        }
    }

    /// The line hold (named mechanism) of team `team`'s lone forward while his team has the
    /// ball and carrier `c` is someone else: his target moves from his anchor toward the
    /// onside line, 0.5 m short of the second-last defender, by the share `lone_line_hold`,
    /// and never past that line. His place across the pitch stays the anchor's.
    fn hold_the_line(&mut self, team: usize, c: usize) {
        let share = self.config.tuning.lone_line_hold;
        if share == 0.0 {
            return;
        }
        let Some(slot) = self.teams[team].lone_forward() else {
            return;
        };
        let i = team * PLAYERS_PER_TEAM + slot;
        if i == c || !self.players[i].active() {
            return;
        }
        let attack_x = self.teams[team].attack_x;
        let line = offside::second_last_depth(team, &self.players, attack_x) - ONSIDE_MARGIN;
        if !line.is_finite() {
            return;
        }
        let anchor = self.players[i].target;
        let depth = anchor.x * attack_x;
        let held = (depth + share * (line - depth)).min(line);
        self.players[i].target = pitch::clamp(DVec2::new(held * attack_x, anchor.y), 0.5);
    }

    /// Goal-side cover (named mechanism) while team `def` defends against carrier `c`. The
    /// covered attacker is the carrier when he is in `def`'s half inside the central channel,
    /// otherwise the most advanced attacker there. The nearest active back-line player of
    /// `def` who is neither pressing nor keeping goal covers him, on the line from the
    /// attacker to the centre of its own goal:
    /// - a carrier is tracked `cover_distance` goal-side of him, at his pace, so the covering
    ///   player stays between him and the goal as he runs;
    /// - any other attacker is covered at the depth of the defender's own place in the line,
    ///   so the cover never plays an attacker onside.
    fn cover(&mut self, def: usize, c: usize, pressers: &[(f64, usize)], keeper: usize) {
        let t = &self.config.tuning;
        let side = &self.teams[def];
        let first = def * PLAYERS_PER_TEAM;
        let goal = side.own_goal();
        // Distance from `def`'s goal line toward the halfway line.
        let depth = |at: DVec2| at.x * side.attack_x + pitch::HALF_LENGTH;
        let covered = |p: &crate::player::Player| {
            p.active() && p.pos.y.abs() <= t.cover_channel && depth(p.pos) < pitch::HALF_LENGTH
        };
        let attacker = if covered(&self.players[c]) {
            Some((depth(self.players[c].pos), c))
        } else {
            let mut most: Option<(f64, usize)> = None;
            for (j, p) in self.players.iter().enumerate() {
                if p.team == def || !covered(p) {
                    continue;
                }
                let d = depth(p.pos);
                if most.is_none_or(|(best, _)| d < best) {
                    most = Some((d, j));
                }
            }
            most
        };
        let Some((reach, j)) = attacker else {
            return;
        };
        let at = self.players[j].pos;
        let mut best: Option<(f64, usize)> = None;
        for slot in side.back_line() {
            let i = first + slot;
            let p = &self.players[i];
            if !p.active() || i == keeper || pressers.iter().any(|&(_, k)| k == i) {
                continue;
            }
            let d = (p.pos - at).length();
            if best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, i));
            }
        }
        let Some((_, i)) = best else {
            return;
        };
        let me = self.players[i];
        let hold = if j == c {
            reach - t.cover_distance
        } else {
            // The defender's own place in the line: its anchor, set earlier this tick.
            depth(me.target)
        }
        .max(0.5);
        let share = if reach > 1e-6 {
            (hold / reach).min(1.0)
        } else {
            0.0
        };
        let mut spot = DVec2::new(
            goal.x + side.attack_x * hold,
            goal.y + (at.y - goal.y) * share,
        );
        if j == c {
            // Lead the spot by the carrier's run over the covering player's braking time, so
            // steering's slow-down near the spot leaves it running at the carrier's pace.
            let braking =
                (me.max_speed() * me.max_speed() / (2.0 * me.max_accel())).max(t.arrive_radius);
            spot += self.players[c].vel * (braking / me.max_speed());
        }
        self.players[i].target = pitch::clamp(spot, 0.5);
    }

    /// Scores every option the carrier has (named mechanism: scored-options decision layer).
    /// Each score is a weighted sum of the option's features with weights from the tuning
    /// file, plus the offsets of the team's plan (mentality, instructions, the carrier's role
    /// and duty), plus noise that shrinks as the carrier's decisions and composure rise.
    pub(crate) fn options(&mut self, c: usize) -> Options {
        let t = &self.config.tuning;
        let w = &t.decision;
        let carrier = self.players[c];
        let team = carrier.team;
        let side = &self.teams[team];
        let plan = side.plan;
        let role = plan.slots[carrier.slot];
        let goal = side.target_goal();
        let attack = DVec2::new(side.attack_x, 0.0);
        let goal_dist = (goal - carrier.pos).length();
        let keeper = c == self.keeper(team);
        let their_keeper = self.keeper(1 - team);
        let skill = |v: f64| (v - 50.0) / 50.0 * w.skill;
        let noise =
            w.noise * (1.5 - (carrier.derived.decisions + carrier.derived.composure) / 200.0);

        // Pressure and space around the carrier.
        let mut nearest_opp = f64::INFINITY;
        let mut nearest_opp_pos = carrier.pos;
        let mut space_ahead: f64 = 10.0;
        for p in &self.players {
            if p.team == team || !p.active() {
                continue;
            }
            let d = p.pos - carrier.pos;
            let dist = d.length();
            if dist < nearest_opp {
                nearest_opp = dist;
                nearest_opp_pos = p.pos;
            }
            if dist < 10.0 && dist > 1e-6 && d.dot(attack) / dist > 0.5 {
                space_ahead = space_ahead.min(dist);
            }
        }
        let pressed = nearest_opp < 2.5;
        // A lone carrier: an outfield carrier with no active outfield team-mate ahead of him.
        let depth = carrier.pos.x * attack.x;
        let lone = !keeper
            && !self.players.iter().enumerate().any(|(j, p)| {
                p.team == team
                    && j != c
                    && j != self.keeper(team)
                    && p.active()
                    && p.pos.x * attack.x > depth
            });

        // Shot.
        let shot = if !keeper && goal_dist < t.shot_range {
            let lane = self
                .players
                .iter()
                .filter(|p| p.team != team && p.id != their_keeper && p.active())
                .map(|p| segment_distance(p.pos, carrier.pos, goal))
                .fold(f64::INFINITY, f64::min);
            let under_pressure = if nearest_opp < 2.0 && lane > 1.0 {
                w.pressure
            } else {
                0.0
            };
            Some(
                w.shot_base + w.shot_lane * ((lane.min(5.0) - 2.5) / 2.5)
                    - w.shot_distance * goal_dist / t.shot_range
                    + under_pressure
                    + skill(carrier.derived.finishing)
                    + plan.shoot
                    + role.shoot
                    + self
                        .streams
                        .range(Key::player(Action::ShotScore, &carrier), -noise, noise),
            )
        } else {
            None
        };

        // Passes.
        let mut pass: Option<(f64, usize)> = None;
        for j in 0..self.players.len() {
            let mate = &self.players[j];
            if mate.team != team || j == c || !mate.active() {
                continue;
            }
            let d = (mate.pos - carrier.pos).length();
            if !(4.0..=45.0).contains(&d) {
                #[cfg(feature = "scenario")]
                audit::record(&mut self.options_audit, Candidate::Distance(j));
                continue;
            }
            let mut lane: f64 = 6.0;
            let mut receiver_space: f64 = 8.0;
            #[cfg(feature = "scenario")]
            let mut visited = 0u32;
            for opp in &self.players {
                if opp.team == team || !opp.active() {
                    continue;
                }
                #[cfg(feature = "scenario")]
                {
                    visited += 1;
                }
                lane = lane.min(segment_distance(opp.pos, carrier.pos, mate.pos));
                // Early exit: `lane` only falls (`f64::min` ignores NaN), so this team-mate
                // is rejected just below whatever the other opponents give, and
                // `receiver_space` is read only in his score. The exit skips no draw, no
                // score, no tie-break, and no eligibility check.
                if lane < w.min_lane {
                    #[cfg(feature = "scenario")]
                    audit::record_break(&mut self.options_audit, &self.players, team, visited);
                    break;
                }
                receiver_space = receiver_space.min((opp.pos - mate.pos).length());
            }
            if lane < w.min_lane {
                #[cfg(feature = "scenario")]
                audit::record(&mut self.options_audit, Candidate::Lane(j));
                continue;
            }
            let progress = ((mate.pos - carrier.pos).dot(attack) / 40.0).clamp(-1.0, 1.0);
            let forward = progress.max(0.0);
            let layoff = if lone && progress <= 0.0 {
                w.lone_layoff
            } else {
                0.0
            };
            let score = (w.progress + plan.progress + role.progress) * progress
                + skill(carrier.derived.vision) * forward
                + w.lane * (lane / 6.0)
                + w.space * (receiver_space / 8.0)
                - w.distance * (d / 45.0)
                + plan.directness * (d / 45.0)
                + plan.tempo
                + layoff
                + self
                    .streams
                    .range(Key::player(Action::PassScore, &carrier), -noise, noise);
            #[cfg(feature = "scenario")]
            audit::record(
                &mut self.options_audit,
                Candidate::Scored {
                    mate: j,
                    score: score.to_bits(),
                },
            );
            if pass.is_none_or(|(s, _)| score > s) {
                pass = Some((score, j));
            }
        }

        let held = self.tick.saturating_sub(self.control_since);
        let held_s = f64::from(held) * t.dt;
        // The carry window: an unpressed carrier who has just gained the ball keeps it
        // rather than releasing it at once. A shot is never charged.
        let carry = if !pressed && held_s < w.carry_s {
            w.carry_cost
        } else {
            0.0
        };
        let pass = pass.map(|(s, j)| (s - carry, j));
        let (dribble, hold) = if keeper {
            (None, None)
        } else {
            let (lone_dribble, lone_hold) = if lone && pressed {
                (w.lone_dribble, w.lone_hold)
            } else {
                (0.0, 0.0)
            };
            let dribble = w.dribble_base + w.dribble_space * (space_ahead / 10.0)
                - if pressed { w.pressure } else { 0.0 }
                + if held < 10 { w.first_touch } else { 0.0 }
                + skill(carrier.derived.dribbling)
                + role.dribble
                - plan.tempo
                + lone_dribble
                + self
                    .streams
                    .range(Key::player(Action::DribbleScore, &carrier), -noise, noise);
            let hold = w.hold + plan.hold - plan.tempo - w.hold_per_s * held_s
                + lone_hold
                + self
                    .streams
                    .range(Key::player(Action::HoldScore, &carrier), -noise, noise);
            (Some(dribble), Some(hold))
        };
        let own_third = carrier.pos.x * attack.x < -pitch::HALF_LENGTH / 3.0;
        let clear = if keeper {
            w.keeper_clear
        } else {
            w.clear
                + if pressed && own_third {
                    w.clear_pressure
                } else {
                    0.0
                }
        } - carry
            + self
                .streams
                .range(Key::player(Action::ClearScore, &carrier), -noise, noise);
        Options {
            shot,
            pass,
            dribble,
            clear,
            hold,
            nearest_opp_pos,
        }
    }

    pub(crate) fn decide_carrier(&mut self, c: usize) -> Option<Kick> {
        #[cfg(feature = "scenario")]
        let mut o = if self.options_audit.is_some() {
            self.audited_options(c)
        } else {
            self.options(c)
        };
        #[cfg(not(feature = "scenario"))]
        let mut o = self.options(c);
        if let Some(off) = self.script_offsets(c) {
            o.shot = o.shot.map(|s| s + off.shoot);
            o.pass = o.pass.map(|(s, j)| (s + off.pass, j));
            o.dribble = o.dribble.map(|s| s + off.dribble);
            o.clear += off.clear;
            o.hold = o.hold.map(|s| s + off.hold);
        }
        let t = &self.config.tuning;
        let carrier = self.players[c];
        let team = carrier.team;
        let goal = self.teams[team].target_goal();
        let attack = DVec2::new(self.teams[team].attack_x, 0.0);
        let mut choice = (o.clear, Choice::Clear);
        for (score, option) in [
            (o.shot, Choice::Shot),
            (o.pass.map(|(s, _)| s), Choice::Pass),
            (o.dribble, Choice::Dribble),
            (o.hold, Choice::Hold),
        ] {
            if let Some(score) = score
                && score > choice.0
            {
                choice = (score, option);
            }
        }
        match choice.1 {
            Choice::Shot => {
                let keeper = self.keeper(1 - team);
                Some(self.shot_kick(c, goal, keeper, 1.0))
            }
            Choice::Pass => {
                let j = o.pass.map_or(c, |(_, j)| j);
                let mate_pos = self.players[j].pos;
                let d = (mate_pos - carrier.pos).length();
                let skill = carrier.derived.passing / 100.0;
                let noise = t.aim_noise * (1.5 - skill);
                let dir = rotate(
                    toward(carrier.pos, mate_pos),
                    self.streams
                        .range(Key::player(Action::PassAim, &carrier), -noise, noise),
                );
                let loft = if d > 25.0 { 3.0 + d * 0.08 } else { 0.0 };
                let speed = kick_speed(d, loft, t);
                Some(Kick::Pass { dir, speed, loft })
            }
            Choice::Clear => {
                // Clear long toward the far half; near his own goal line a clearance may go
                // wide toward that line instead.
                let own_goal_x = -self.teams[team].attack_x;
                let c = &t.clearances;
                let wide = c.wide_chance > 0.0
                    && carrier.pos.x * own_goal_x.signum() >= pitch::HALF_LENGTH - c.wide_depth
                    && self
                        .streams
                        .tested(Key::player(Action::ClearWide, &carrier), &[c.wide_chance])
                        < c.wide_chance;
                let (line, spread) = if wide {
                    (wide_of_goal(carrier.pos, own_goal_x), WIDE_SPREAD)
                } else {
                    (attack, c.aim_spread)
                };
                let dir = rotate(
                    line,
                    self.streams
                        .range(Key::player(Action::ClearAim, &carrier), -spread, spread),
                );
                Some(Kick::Clear {
                    dir,
                    speed: kick_speed(CLEARANCE_DISTANCE, CLEARANCE_LOFT, t),
                    loft: CLEARANCE_LOFT,
                })
            }
            Choice::Hold => {
                self.players[c].target = carrier.pos;
                None
            }
            Choice::Dribble => {
                // Dribble toward goal, drifting away from the nearest opponent.
                let to_goal = goal - carrier.pos;
                let goal_dist = to_goal.length();
                let dir_goal = if goal_dist > 1e-6 {
                    to_goal / goal_dist
                } else {
                    attack
                };
                let perp = DVec2::new(-dir_goal.y, dir_goal.x);
                let side = if (o.nearest_opp_pos - carrier.pos).dot(perp) > 0.0 {
                    -1.0
                } else {
                    1.0
                };
                let target = carrier.pos + dir_goal * 8.0 + perp * (4.0 * side);
                self.players[c].target = pitch::clamp(target, 1.0);
                None
            }
        }
    }
}

impl Simulation {
    /// The decision hook's offsets for carrier `c`, or `None` without a decision hook. The
    /// hook is asked again for a new carrier, after a stoppage, and when the pack's refresh
    /// interval has passed; in between, the cached offsets apply. A failed call gives zero
    /// offsets until the next refresh. Nothing here draws from the random stream.
    fn script_offsets(&mut self, c: usize) -> Option<OptionOffsets> {
        self.plugins.decision.as_ref()?;
        if let Some(cache) = self.script_cache
            && cache.carrier == c
            && self.tick < cache.until
        {
            return Some(cache.offsets);
        }
        let ctx = self.decision_context(c);
        let outcome = self.plugins.decision.as_mut()?.adjust(&ctx);
        let (value, notes) = self.plugins.settle(HookPoint::Decision, outcome, self.tick);
        self.push_script_notes(notes);
        let offsets = value.unwrap_or_default();
        self.script_cache = Some(ScriptCache {
            carrier: c,
            until: self.tick.saturating_add(self.plugins.refresh_ticks.max(1)),
            offsets,
        });
        Some(offsets)
    }

    /// What the decision hook sees about carrier `c`.
    pub(crate) fn decision_context(&self, c: usize) -> DecisionContext {
        let carrier = self.players[c];
        let team = carrier.team;
        let side = &self.teams[team];
        let nearest_opponent = self
            .players
            .iter()
            .filter(|p| p.team != team && p.active())
            .map(|p| (p.pos - carrier.pos).length())
            .fold(f64::INFINITY, f64::min);
        DecisionContext {
            tick: self.tick,
            minute: self.referee.clock.minute(self.tick).0,
            team,
            slot: carrier.slot,
            goals_for: self.summary.goals[team],
            goals_against: self.summary.goals[1 - team],
            goal_distance: (side.target_goal() - carrier.pos).length(),
            nearest_opponent: if nearest_opponent.is_finite() {
                nearest_opponent
            } else {
                pitch::HALF_LENGTH * 2.0
            },
            progress: (carrier.pos.x * side.attack_x / pitch::HALF_LENGTH).clamp(-1.0, 1.0),
        }
    }

    /// Player `c` shoots at the goal centred on `goal`, which `keeper` defends. The shot aims
    /// for the side of the goal away from the goalkeeper; a better finisher places it nearer
    /// the post and strikes it truer. `spread_scale` scales the aim noise: 1 in open play, and
    /// less for a placed kick from the penalty mark. An open-play shot and a shoot-out kick
    /// both come from here, and the draws are taken in the same order either way.
    pub(crate) fn shot_kick(
        &mut self,
        c: usize,
        goal: DVec2,
        keeper: usize,
        spread_scale: f64,
    ) -> Kick {
        let t = &self.config.tuning;
        let carrier = self.players[c];
        let finishing = (carrier.derived.finishing / 100.0).clamp(0.0, 1.0);
        let keeper = &self.players[keeper];
        let side = if !keeper.active() {
            if self
                .streams
                .chance(Key::player(Action::ShotSide, &carrier), 0.5)
            {
                1.0
            } else {
                -1.0
            }
        } else if keeper.pos.y > goal.y {
            -1.0
        } else {
            1.0
        };
        let reach = pitch::GOAL_WIDTH / 2.0 * (0.4 + 0.5 * finishing);
        let aim = DVec2::new(
            goal.x,
            side * self
                .streams
                .range(Key::player(Action::ShotAim, &carrier), 0.5 * reach, reach),
        );
        let spread = t.shot_noise * (1.5 - finishing) * spread_scale;
        let dir = rotate(
            toward(carrier.pos, aim),
            self.streams
                .range(Key::player(Action::ShotSpread, &carrier), -spread, spread),
        );
        Kick::Shot {
            dir,
            speed: t.shot_speed,
            loft: self.streams.range(
                Key::player(Action::ShotLoft, &carrier),
                0.0,
                t.shots.loft_max,
            ),
        }
    }
}

/// The carrier's scored options on one tick. `None` is an option the carrier does not have:
/// no shot beyond the shooting range, no pass without an open team-mate, and no dribble or
/// hold for a goalkeeper.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Options {
    pub shot: Option<f64>,
    /// The best pass and the team-mate's roster index.
    pub pass: Option<(f64, usize)>,
    pub dribble: Option<f64>,
    pub clear: f64,
    pub hold: Option<f64>,
    pub nearest_opp_pos: DVec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Choice {
    Shot,
    Pass,
    Dribble,
    Clear,
    Hold,
}

impl Simulation {
    /// The kick that takes a restart: a short lofted throw to the nearest team-mate, a cross
    /// toward the penalty mark from a corner, or a pass to an open team-mate. With no
    /// team-mate in range, the ball goes forward along the pitch.
    pub(crate) fn restart_pass(&mut self, taker: usize, kind: StoppageKind) -> Kick {
        let t = &self.config.tuning;
        let me = self.players[taker];
        let attack = DVec2::new(self.teams[me.team].attack_x, 0.0);
        let cross_at = (kind == StoppageKind::Corner).then(|| pitch::penalty_spot(attack.x));
        let range = if kind == StoppageKind::ThrowIn {
            3.0..=THROW_RANGE
        } else {
            4.0..=45.0
        };
        let mut best: Option<(f64, DVec2)> = None;
        for (j, mate) in self.players.iter().enumerate() {
            if j == taker || mate.team != me.team || !mate.active() {
                continue;
            }
            let d = (mate.pos - me.pos).length();
            if !range.contains(&d) {
                continue;
            }
            let score = match cross_at {
                Some(mark) => -(mate.pos - mark).length(),
                None => -d + 0.2 * (mate.pos - me.pos).dot(attack),
            };
            if best.is_none_or(|(s, _)| score > s) {
                best = Some((score, mate.pos));
            }
        }
        let to = best.map_or_else(|| pitch::clamp(me.pos + attack * 10.0, 4.0), |(_, at)| at);
        let d = (to - me.pos).length();
        let dir = match toward(me.pos, to) {
            v if v == DVec2::ZERO => attack,
            v => v,
        };
        let loft = match kind {
            StoppageKind::ThrowIn => THROW_LOFT,
            StoppageKind::Corner => 6.0 + 0.1 * d,
            _ if d > 25.0 => 3.0 + d * 0.08,
            _ => 0.0,
        };
        Kick::Pass {
            dir,
            speed: kick_speed(d, loft, t),
            loft,
        }
    }
}

/// The most opponents a pressing instruction can send at the carrier.
/// Metres short of the second-last defender at which a lone forward holds the line, so he
/// stays onside.
const ONSIDE_MARGIN: f64 = 0.5;

const MAX_PRESSERS: usize = 4;
/// The furthest ahead, in seconds, a presser aims along the carrier's run: beyond it the
/// carrier's run is too uncertain to chase.
const INTERCEPT_HORIZON_S: f64 = 1.0;

/// Where a player at `p` with top speed `s` meets a carrier at `c` running at `v`: `c + v t`,
/// where `t` is the smallest positive time with `|c + v t - p| = s t`, capped at
/// `INTERCEPT_HORIZON_S`. With no such time (a carrier running away faster than the player)
/// the point is the carrier.
pub(crate) fn intercept(p: DVec2, s: f64, c: DVec2, v: DVec2) -> DVec2 {
    let d = c - p;
    let a = v.dot(v) - s * s;
    let b = 2.0 * d.dot(v);
    let k = d.dot(d);
    let t = if a.abs() < 1e-9 {
        (b < 0.0).then(|| -k / b)
    } else {
        let disc = b * b - 4.0 * a * k;
        if disc < 0.0 {
            None
        } else {
            let r = disc.sqrt();
            let (t1, t2) = ((-b - r) / (2.0 * a), (-b + r) / (2.0 * a));
            let (lo, hi) = (t1.min(t2), t1.max(t2));
            if lo > 0.0 {
                Some(lo)
            } else if hi > 0.0 {
                Some(hi)
            } else {
                None
            }
        }
    };
    match t {
        Some(t) => c + v * t.min(INTERCEPT_HORIZON_S),
        None => c,
    }
}

/// The farthest team-mate a throw-in looks for, in metres, and the throw's vertical speed.
const THROW_RANGE: f64 = 15.0;
const THROW_LOFT: f64 = 2.0;
/// How far a goalkeeper's clearance travels before it stops, in metres, and its vertical
/// speed.
const CLEARANCE_DISTANCE: f64 = 55.0;
const CLEARANCE_LOFT: f64 = 6.0;

/// The ground speed of a kick that reaches a team-mate `d` metres away at the pass arrival
/// speed. A ground pass decelerates by friction the whole way. A lofted pass flies for
/// `2 * loft / g` seconds with no friction, so it needs less speed: the flight covers
/// `speed * T` and the roll `speed^2 / (2 f)`, which puts the ball at the team-mate with the
/// arrival speed left.
fn kick_speed(d: f64, loft: f64, t: &Tuning) -> f64 {
    let f = t.ground_friction;
    let a = t.pass_arrival_speed;
    let speed = if loft <= 0.0 {
        (2.0 * f * d).sqrt() + a
    } else {
        let flight = 2.0 * loft / t.gravity;
        let reach = d + a * a / (2.0 * f);
        f * (-flight + (flight * flight + 2.0 * reach / f).sqrt())
    };
    speed.min(t.pass_max_speed)
}

/// Rotates a unit vector by `angle` radians.
fn rotate(v: DVec2, angle: f64) -> DVec2 {
    let (s, c) = math::sin_cos(angle);
    DVec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// Every value of `o` as bits, for the audit's exact compare.
#[cfg(feature = "scenario")]
type OptionBits = (
    Option<u64>,
    Option<(u64, usize)>,
    Option<u64>,
    u64,
    Option<u64>,
    (u64, u64),
);

#[cfg(feature = "scenario")]
fn option_bits(o: &Options) -> OptionBits {
    (
        o.shot.map(f64::to_bits),
        o.pass.map(|(s, j)| (s.to_bits(), j)),
        o.dribble.map(f64::to_bits),
        o.clear.to_bits(),
        o.hold.map(f64::to_bits),
        (o.nearest_opp_pos.x.to_bits(), o.nearest_opp_pos.y.to_bits()),
    )
}

#[cfg(feature = "scenario")]
impl Simulation {
    /// The carrier-options audit's reference: the options code before the pass-lane early
    /// exit, verbatim except for the team-mate records and the audit's test controls.
    /// A later change to the draws of `options()` must change this copy in the same
    /// commit, or the audit test fails.
    fn options_reference(&mut self, c: usize) -> (Options, Vec<Candidate>) {
        let control = self.options_audit.as_ref().and_then(|a| a.control);
        let mut records = Vec::new();
        let mut rejected_one = false;
        let mut bumped = false;
        let t = &self.config.tuning;
        let w = &t.decision;
        let carrier = self.players[c];
        let team = carrier.team;
        let side = &self.teams[team];
        let plan = side.plan;
        let role = plan.slots[carrier.slot];
        let goal = side.target_goal();
        let attack = DVec2::new(side.attack_x, 0.0);
        let goal_dist = (goal - carrier.pos).length();
        let keeper = c == self.keeper(team);
        let their_keeper = self.keeper(1 - team);
        let skill = |v: f64| (v - 50.0) / 50.0 * w.skill;
        let noise =
            w.noise * (1.5 - (carrier.derived.decisions + carrier.derived.composure) / 200.0);

        // Pressure and space around the carrier.
        let mut nearest_opp = f64::INFINITY;
        let mut nearest_opp_pos = carrier.pos;
        let mut space_ahead: f64 = 10.0;
        for p in &self.players {
            if p.team == team || !p.active() {
                continue;
            }
            let d = p.pos - carrier.pos;
            let dist = d.length();
            if dist < nearest_opp {
                nearest_opp = dist;
                nearest_opp_pos = p.pos;
            }
            if dist < 10.0 && dist > 1e-6 && d.dot(attack) / dist > 0.5 {
                space_ahead = space_ahead.min(dist);
            }
        }
        let pressed = nearest_opp < 2.5;
        // A lone carrier: an outfield carrier with no active outfield team-mate ahead of him.
        let depth = carrier.pos.x * attack.x;
        let lone = !keeper
            && !self.players.iter().enumerate().any(|(j, p)| {
                p.team == team
                    && j != c
                    && j != self.keeper(team)
                    && p.active()
                    && p.pos.x * attack.x > depth
            });

        // Shot.
        let shot = if !keeper && goal_dist < t.shot_range {
            let lane = self
                .players
                .iter()
                .filter(|p| p.team != team && p.id != their_keeper && p.active())
                .map(|p| segment_distance(p.pos, carrier.pos, goal))
                .fold(f64::INFINITY, f64::min);
            let under_pressure = if nearest_opp < 2.0 && lane > 1.0 {
                w.pressure
            } else {
                0.0
            };
            Some(
                w.shot_base + w.shot_lane * ((lane.min(5.0) - 2.5) / 2.5)
                    - w.shot_distance * goal_dist / t.shot_range
                    + under_pressure
                    + skill(carrier.derived.finishing)
                    + plan.shoot
                    + role.shoot
                    + self
                        .streams
                        .range(Key::player(Action::ShotScore, &carrier), -noise, noise),
            )
        } else {
            None
        };

        // Passes.
        let mut pass: Option<(f64, usize)> = None;
        for j in 0..self.players.len() {
            let mate = &self.players[j];
            if mate.team != team || j == c || !mate.active() {
                continue;
            }
            let d = (mate.pos - carrier.pos).length();
            if !(4.0..=45.0).contains(&d) {
                records.push(Candidate::Distance(j));
                continue;
            }
            let mut lane: f64 = 6.0;
            let mut receiver_space: f64 = 8.0;
            for opp in &self.players {
                if opp.team == team || !opp.active() {
                    continue;
                }
                lane = lane.min(segment_distance(opp.pos, carrier.pos, mate.pos));
                receiver_space = receiver_space.min((opp.pos - mate.pos).length());
            }
            if lane < w.min_lane || (control == Some(Control::RejectMate) && !rejected_one) {
                rejected_one |= lane >= w.min_lane;
                records.push(Candidate::Lane(j));
                continue;
            }
            let progress = ((mate.pos - carrier.pos).dot(attack) / 40.0).clamp(-1.0, 1.0);
            let forward = progress.max(0.0);
            let layoff = if lone && progress <= 0.0 {
                w.lone_layoff
            } else {
                0.0
            };
            let score = (w.progress + plan.progress + role.progress) * progress
                + skill(carrier.derived.vision) * forward
                + w.lane * (lane / 6.0)
                + w.space * (receiver_space / 8.0)
                - w.distance * (d / 45.0)
                + plan.directness * (d / 45.0)
                + plan.tempo
                + layoff
                + self
                    .streams
                    .range(Key::player(Action::PassScore, &carrier), -noise, noise);
            let score = if control == Some(Control::OneUlp) && !bumped {
                bumped = true;
                score.next_up()
            } else {
                score
            };
            records.push(Candidate::Scored {
                mate: j,
                score: score.to_bits(),
            });
            if pass.is_none_or(|(s, _)| score > s) {
                pass = Some((score, j));
            }
        }

        let held = self.tick.saturating_sub(self.control_since);
        let held_s = f64::from(held) * t.dt;
        // The carry window: an unpressed carrier who has just gained the ball keeps it
        // rather than releasing it at once. A shot is never charged.
        let carry = if !pressed && held_s < w.carry_s {
            w.carry_cost
        } else {
            0.0
        };
        let pass = pass.map(|(s, j)| (s - carry, j));
        let (dribble, hold) = if keeper {
            (None, None)
        } else {
            let (lone_dribble, lone_hold) = if lone && pressed {
                (w.lone_dribble, w.lone_hold)
            } else {
                (0.0, 0.0)
            };
            let dribble = w.dribble_base + w.dribble_space * (space_ahead / 10.0)
                - if pressed { w.pressure } else { 0.0 }
                + if held < 10 { w.first_touch } else { 0.0 }
                + skill(carrier.derived.dribbling)
                + role.dribble
                - plan.tempo
                + lone_dribble
                + self
                    .streams
                    .range(Key::player(Action::DribbleScore, &carrier), -noise, noise);
            let hold = w.hold + plan.hold - plan.tempo - w.hold_per_s * held_s
                + lone_hold
                + self
                    .streams
                    .range(Key::player(Action::HoldScore, &carrier), -noise, noise);
            (Some(dribble), Some(hold))
        };
        let own_third = carrier.pos.x * attack.x < -pitch::HALF_LENGTH / 3.0;
        let clear = if keeper {
            w.keeper_clear
        } else {
            w.clear
                + if pressed && own_third {
                    w.clear_pressure
                } else {
                    0.0
                }
        } - carry
            + self
                .streams
                .range(Key::player(Action::ClearScore, &carrier), -noise, noise);
        if control == Some(Control::ExtraDraw) {
            self.streams.draw(Key::player(Action::ClearScore, &carrier));
        }
        (
            Options {
                shot,
                pass,
                dribble,
                clear,
                hold,
                nearest_opp_pos,
            },
            records,
        )
    }

    /// The carrier's options with the audit on: the reference copy scores them first from a
    /// copy of the stream, then the live code scores them from the real stream, and the two
    /// results are compared. The match goes on from the live call.
    fn audited_options(&mut self, c: usize) -> Options {
        let real = self.streams.clone();
        let (want, want_records) = self.options_reference(c);
        let want_stream = self.streams.stream_state();
        self.streams = real;
        if let Some(a) = self.options_audit.as_mut() {
            a.candidates.clear();
        }
        let got = self.options(c);
        let got_stream = self.streams.stream_state();
        let tick = self.tick;
        let a = self
            .options_audit
            .as_mut()
            .expect("the audit is on while it runs");
        let got_records = std::mem::take(&mut a.candidates);
        a.calls += 1;
        let scored = |r: &Candidate| matches!(r, Candidate::Scored { .. });
        if got_records.iter().any(|r| !scored(r)) && got_records.iter().any(scored) {
            a.mixed_calls += 1;
        }
        let differs = if got_records != want_records {
            Some("team-mate records")
        } else if option_bits(&got) != option_bits(&want) {
            Some("option scores or pass")
        } else if got_stream != want_stream {
            Some("stream position")
        } else {
            None
        };
        if let Some(what) = differs {
            a.mismatch(tick, c, what);
        }
        got
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_config;

    #[test]
    fn a_lofted_pass_reaches_its_target_slow_enough_to_control() {
        let t = crate::tuning::Tuning::default();
        for d in [30.0, 40.0, 45.0] {
            let loft = 3.0 + d * 0.08;
            let mut ball = crate::ball::Ball::at(DVec2::ZERO);
            ball.kick(DVec2::X, kick_speed(d, loft, &t), loft, &t);
            let mut at_target = None;
            for _ in 0..1000 {
                ball.integrate(&t);
                if at_target.is_none() && ball.pos.x >= d {
                    at_target = Some(ball.speed());
                }
            }
            let speed = at_target.unwrap_or_else(|| panic!("a {d} m pass fell short"));
            assert!(
                speed <= t.control_speed,
                "a {d} m pass arrived at {speed} m/s"
            );
            // The bounce after landing carries an uncontrolled ball on by up to a third.
            assert!(
                ball.pos.x < d * 1.34,
                "a {d} m pass rolled on to {}",
                ball.pos.x
            );
        }
    }

    #[test]
    fn an_open_teammate_draws_a_pass_or_a_shot() {
        let mut sim = Simulation::new(shipped_config(7, 1).unwrap()).unwrap();
        // Give the ball to the home striker (slot 9) deep in the away half with a teammate open.
        let striker = 9;
        sim.players[striker].pos = DVec2::new(20.0, 0.0);
        sim.players[10].pos = DVec2::new(30.0, 6.0);
        for p in sim.players.iter_mut().filter(|p| p.team == 1) {
            p.pos = DVec2::new(-40.0, p.pos.y);
        }
        sim.carrier = Some(striker);
        sim.control_since = 0;
        sim.tick = 100;
        let mut kicked = false;
        for _ in 0..50 {
            if sim.decide().is_some() {
                kicked = true;
                break;
            }
        }
        assert!(kicked, "the carrier never passed or shot");
    }

    #[test]
    fn an_attacking_mentality_raises_the_shoot_score_for_the_same_scene() {
        let content = crate::data::test_support::shipped_content();
        let mut config = shipped_config(7, 1).unwrap();
        config.tuning.decision.noise = 0.0;
        let shot_score = |mentality: &str| {
            let mut tactics = crate::tactics::Tactics::defaults(&content.tactics);
            tactics.mentality = content.tactics.mentality_index(mentality).unwrap() as u8;
            let mut sim = Simulation::new(config.clone().with_tactics(0, tactics)).unwrap();
            let striker = 9;
            sim.players[striker].pos = DVec2::new(40.0, 2.0);
            let their_keeper = sim.keeper(1);
            for p in sim
                .players
                .iter_mut()
                .filter(|p| p.team == 1 && p.id != their_keeper)
            {
                p.pos = DVec2::new(-40.0, p.pos.y);
            }
            sim.carrier = Some(striker);
            sim.tick = 100;
            sim.options(striker)
                .shot
                .expect("inside the shooting range")
        };
        let defensive = shot_score("defensive");
        let balanced = shot_score("balanced");
        let attacking = shot_score("attacking");
        assert!(
            defensive < balanced && balanced < attacking,
            "{defensive} {balanced} {attacking}"
        );
    }

    /// The options of the home striker at `(40, 2)` with an open team-mate, noise off, the
    /// carry cost `cost`, having held the ball `held` ticks, with an opponent `opp` metres
    /// away or none; and the next draw of the stream after scoring them.
    fn carry_scene(cost: f64, held: u32, opp: Option<f64>) -> (Options, f64) {
        let mut config = shipped_config(7, 1).unwrap();
        config.tuning.decision.noise = 0.0;
        config.tuning.decision.carry_s = 1.5;
        config.tuning.decision.carry_cost = cost;
        let mut sim = Simulation::new(config).unwrap();
        let striker = 9;
        sim.players[striker].pos = DVec2::new(40.0, 2.0);
        sim.players[10].pos = DVec2::new(30.0, 12.0);
        let their_keeper = sim.keeper(1);
        for p in sim
            .players
            .iter_mut()
            .filter(|p| p.team == 1 && p.id != their_keeper)
        {
            p.pos = DVec2::new(-40.0, p.pos.y);
        }
        if let Some(d) = opp {
            sim.players[13].pos = DVec2::new(40.0 + d, 2.0);
        }
        sim.carrier = Some(striker);
        sim.tick = 1_000;
        sim.control_since = 1_000 - held;
        let o = sim.options(striker);
        (o, sim.streams.draw(Key::of_match(Action::AddedTime)))
    }

    #[test]
    fn an_unpressed_carrier_pays_the_carry_cost_on_a_pass_and_a_clearance_only() {
        let (free, next_free) = carry_scene(0.0, 0, None);
        let (charged, next_charged) = carry_scene(1.0, 0, None);
        let pass = |o: &Options| o.pass.expect("an open team-mate").0;
        assert!((pass(&free) - pass(&charged) - 1.0).abs() < 1e-12);
        assert!((free.clear - charged.clear - 1.0).abs() < 1e-12);
        assert_eq!(free.shot, charged.shot, "a shot is never charged");
        assert_eq!(free.dribble, charged.dribble);
        assert_eq!(free.hold, charged.hold);
        assert_eq!(next_free, next_charged, "the window draws nothing");
    }

    #[test]
    fn a_pressed_carrier_and_one_past_the_window_pay_no_carry_cost() {
        let pass = |o: &Options| o.pass.expect("an open team-mate").0;
        let (free, _) = carry_scene(0.0, 0, Some(2.0));
        let (charged, _) = carry_scene(1.0, 0, Some(2.0));
        assert_eq!(pass(&free), pass(&charged), "pressed");
        assert_eq!(free.clear, charged.clear, "pressed");
        let (free, _) = carry_scene(0.0, 80, None);
        let (charged, _) = carry_scene(1.0, 80, None);
        assert_eq!(pass(&free), pass(&charged), "past the window");
        assert_eq!(free.clear, charged.clear, "past the window");
    }

    /// The home striker (slot 9) at `(40, 2)` with one team-mate (slot 10) at `(30, 2)`,
    /// every other player out of passing range, noise on; one opponent (roster index
    /// `blocker`) at `at` when given. Returns the audit of one audited options call.
    #[cfg(feature = "scenario")]
    fn lane_scene(blocker: Option<(usize, DVec2)>) -> crate::gate::audit::OptionsAudit {
        let mut sim = Simulation::new(shipped_config(7, 1).unwrap()).unwrap();
        let striker = 9;
        for (i, p) in sim.players.iter_mut().enumerate() {
            p.pos = DVec2::new(-50.0, -30.0 + 2.5 * i as f64);
        }
        sim.players[striker].pos = DVec2::new(40.0, 2.0);
        sim.players[10].pos = DVec2::new(30.0, 2.0);
        if let Some((i, at)) = blocker {
            assert_eq!(sim.players[i].team, 1, "the blocker is an opponent");
            sim.players[i].pos = at;
        }
        sim.carrier = Some(striker);
        sim.tick = 1_000;
        sim.control_since = 990;
        sim.options_audit = Some(crate::gate::audit::OptionsAudit::default());
        sim.audited_options(striker);
        sim.options_audit.take().unwrap()
    }

    #[cfg(feature = "scenario")]
    #[test]
    fn the_lane_early_exit_matches_the_reference_for_first_last_absent_and_equal_blockers() {
        let first = 11;
        let last = 21;
        let mid = DVec2::new(35.0, 2.0);
        // The blocker on the lane, first among the opponents: the loop ends at once.
        let a = lane_scene(Some((first, mid)));
        assert_eq!((a.calls, a.mismatches, a.breaks), (1, 0, 1), "{a:?}");
        assert_eq!(a.skipped_opponents, 10);
        // Last among the opponents: the break fires with nothing left to skip.
        let a = lane_scene(Some((last, mid)));
        assert_eq!((a.calls, a.mismatches, a.breaks), (1, 0, 1), "{a:?}");
        assert_eq!(a.skipped_opponents, 0);
        // No blocker: no break, and the team-mate is scored.
        let a = lane_scene(None);
        assert_eq!((a.calls, a.mismatches, a.breaks), (1, 0, 0), "{a:?}");
        // A blocker exactly the minimum lane (1.5 m) from the lane: not rejected, no break.
        let t = crate::tuning::Tuning::default();
        assert_eq!(t.decision.min_lane, 1.5);
        let at = DVec2::new(35.0, 3.5);
        assert_eq!(
            segment_distance(at, DVec2::new(40.0, 2.0), DVec2::new(30.0, 2.0)),
            1.5
        );
        let a = lane_scene(Some((first, at)));
        assert_eq!((a.calls, a.mismatches, a.breaks), (1, 0, 0), "{a:?}");
    }

    #[cfg(feature = "scenario")]
    #[test]
    fn the_lane_scene_audit_sees_each_control() {
        use crate::gate::audit::{Control, OptionsAudit};
        let mid = DVec2::new(35.0, 2.0);
        for (control, blocker) in [
            (Control::ExtraDraw, Some((11, mid))),
            (Control::OneUlp, None),
            (Control::RejectMate, None),
        ] {
            let mut sim = Simulation::new(shipped_config(7, 1).unwrap()).unwrap();
            let striker = 9;
            for (i, p) in sim.players.iter_mut().enumerate() {
                p.pos = DVec2::new(-50.0, -30.0 + 2.5 * i as f64);
            }
            sim.players[striker].pos = DVec2::new(40.0, 2.0);
            sim.players[10].pos = DVec2::new(30.0, 2.0);
            if let Some((i, at)) = blocker {
                sim.players[i].pos = at;
            }
            sim.carrier = Some(striker);
            sim.tick = 1_000;
            sim.control_since = 990;
            sim.options_audit = Some(OptionsAudit {
                control: Some(control),
                ..OptionsAudit::default()
            });
            sim.audited_options(striker);
            let a = sim.options_audit.take().unwrap();
            assert_eq!(a.mismatches, 1, "{control:?}: {a:?}");
        }
    }
}

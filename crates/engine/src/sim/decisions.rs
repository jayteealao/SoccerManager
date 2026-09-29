//! The decision pass of the central loop: every player's target and the carrier's kick.

use serde_json::json;

use crate::data::rules::StoppageKind;
use crate::decision::{
    CLEARANCE_DISTANCE, CLEARANCE_LOFT, Choice, Kick, MAX_PRESSERS, ONSIDE_MARGIN, Options,
    THROW_LOFT, THROW_RANGE, intercept, kick_speed, offsets_json, rotate,
};
use crate::math::{DVec2, segment_distance, toward};
use crate::pitch;
use crate::plugin::{DecisionContext, HookPoint, OptionOffsets};
use crate::rules::offside;
use crate::sim::{ScriptCache, Simulation, WIDE_SPREAD, wide_of_goal};
use crate::streams::{Action, Key};
use crate::team::PLAYERS_PER_TEAM;
use crate::trace::Point;

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
                if self.trace_on() {
                    let chosen: Vec<_> = pressers[..count]
                        .iter()
                        .filter(|&&(_, i)| i != usize::MAX)
                        .map(|&(d, i)| json!({"player": i, "distance": d}))
                        .collect();
                    self.trace_point(
                        Point::Press,
                        json!({"team": def, "count": count, "reach": reach, "pressers": chosen}),
                    );
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
                if self.trace_on() {
                    let chasers: Vec<_> = (0..2)
                        .map(|team| {
                            let i = nearest[team];
                            let gk = keepers[team];
                            let keeper = self.players[gk].active()
                                && (self.players[gk].pos - ball_xy).length() < 16.0;
                            json!({
                                "team": team,
                                "player": (i != usize::MAX).then_some(i),
                                "distance": (i != usize::MAX).then_some(nearest_dist[team]),
                                "keeper": keeper.then_some(gk),
                            })
                        })
                        .collect();
                    self.trace_point(Point::Chase, json!({ "chasers": chasers }));
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
            if let Some(trace) = self.streams.trace_mut() {
                trace.push_point(Point::Cover, json!({"team": def, "attacker": null}));
            }
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
        let Some((distance, i)) = best else {
            if let Some(trace) = self.streams.trace_mut() {
                trace.push_point(
                    Point::Cover,
                    json!({"team": def, "attacker": j, "defender": null}),
                );
            }
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
        if let Some(trace) = self.streams.trace_mut() {
            trace.push_point(
                Point::Cover,
                json!({"team": def, "attacker": j, "defender": i, "distance": distance}),
            );
        }
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
            if lane < w.min_lane {
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
            if let Some(trace) = self.streams.trace_mut() {
                trace.push_candidate(j, score);
            }
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
        let mut o = self.options(c);
        let offsets = self.script_offsets(c);
        if let Some(off) = offsets {
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
        if let Some(trace) = self.streams.trace_mut() {
            let candidates: Vec<_> = trace
                .take_candidates()
                .into_iter()
                .map(|(mate, score)| json!({"mate": mate, "score": score}))
                .collect();
            let detail = json!({
                "carrier": c,
                "candidates": candidates,
                "shot": o.shot,
                "pass": o.pass.map(|(score, mate)| json!({"mate": mate, "score": score})),
                "dribble": o.dribble,
                "hold": o.hold,
                "clear": o.clear,
                "offsets": offsets.as_ref().map(offsets_json),
                "choice": choice.1.name(),
            });
            trace.push_point(Point::Carrier, detail);
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
            if self.trace_on() {
                self.trace_point(
                    Point::ScriptDecision,
                    json!({"carrier": c, "cached": true, "offsets": offsets_json(&cache.offsets)}),
                );
            }
            return Some(cache.offsets);
        }
        let ctx = self.decision_context(c);
        let outcome = self.plugins.decision.as_mut()?.adjust(&ctx);
        let (value, notes) = self.plugins.settle(HookPoint::Decision, outcome, self.tick);
        let failed = value.is_none();
        let note_count = notes.len();
        let switched_off = self.plugins.decision.is_none();
        self.push_script_notes(notes);
        let offsets = value.unwrap_or_default();
        if self.trace_on() {
            let result = if !failed {
                "value"
            } else if switched_off {
                "switched_off"
            } else {
                "failed"
            };
            self.trace_point(
                Point::ScriptDecision,
                json!({
                    "carrier": c,
                    "cached": false,
                    "offsets": offsets_json(&offsets),
                    "result": result,
                    "notes": note_count,
                }),
            );
        }
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
        let traced = self.streams.trace_on();
        let mut candidates = Vec::new();
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
            if traced {
                candidates.push(json!({"mate": j, "score": score}));
            }
            if best.is_none_or(|(s, _)| score > s) {
                best = Some((score, mate.pos));
            }
        }
        let to = best.map_or_else(|| pitch::clamp(me.pos + attack * 10.0, 4.0), |(_, at)| at);
        if let Some(trace) = self.streams.trace_mut() {
            trace.push_point(
                Point::RestartPass,
                json!({
                    "taker": taker,
                    "kind": kind.code(),
                    "candidates": candidates,
                    "target": [to.x, to.y],
                    "forward_fallback": best.is_none(),
                }),
            );
        }
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

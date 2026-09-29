//! The possession decision. The ball carrier scores its options (shoot, pass to each open
//! team-mate, dribble, clear, hold) and takes the highest; the team's pressing instruction
//! sets how many opponents press and from how far; every other player holds its formation
//! anchor, which the team's plan moves. Every agent decides every tick (product-owner choice;
//! `Tuning::decision_interval_ticks`).
//! A sent-off player takes no part in any decision. While the ball is dead, the referee sets
//! every target instead, and a restart kick comes from `restart_pass`.

use crate::data::rules::StoppageKind;
use crate::math::{self, DVec2, segment_distance, toward};
use crate::modules::{
    CarrierPlan, CoverTrace, DecisionModule, MATES, MatchView, ModuleCard, OptionDraft,
    OptionDraws, ROSTER, RestartPass, Scored, ShotDraws, Targets, TargetsTrace,
};
use crate::pitch;
use crate::plugin::OptionOffsets;
use crate::rules::offside;
use crate::sim::{WIDE_SPREAD, wide_of_goal};
use crate::streams::{Action, map_range};
use crate::team::PLAYERS_PER_TEAM;
use crate::tuning::Tuning;
use serde_json::json;

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

/// The carrier's scored options on one tick. `None` is an option the carrier does not have:
/// no shot beyond the shooting range, no pass without an open team-mate, and no dribble or
/// hold for a goalkeeper.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub shot: Option<f64>,
    /// The best pass and the team-mate's roster index.
    pub pass: Option<(f64, usize)>,
    pub dribble: Option<f64>,
    pub clear: f64,
    pub hold: Option<f64>,
    pub nearest_opp_pos: DVec2,
}

/// The option a carrier takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Shot,
    Pass,
    Dribble,
    Clear,
    Hold,
}

impl Choice {
    pub fn name(self) -> &'static str {
        match self {
            Choice::Shot => "shot",
            Choice::Pass => "pass",
            Choice::Dribble => "dribble",
            Choice::Clear => "clear",
            Choice::Hold => "hold",
        }
    }
}

/// The most opponents a pressing instruction can send at the carrier.
/// Metres short of the second-last defender at which a lone forward holds the line, so he
/// stays onside.
pub(crate) const ONSIDE_MARGIN: f64 = 0.5;

pub const MAX_PRESSERS: usize = 4;
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
pub(crate) const THROW_RANGE: f64 = 15.0;
pub(crate) const THROW_LOFT: f64 = 2.0;
/// How far a goalkeeper's clearance travels before it stops, in metres, and its vertical
/// speed.
pub(crate) const CLEARANCE_DISTANCE: f64 = 55.0;
pub(crate) const CLEARANCE_LOFT: f64 = 6.0;

/// The ground speed of a kick that reaches a team-mate `d` metres away at the pass arrival
/// speed. A ground pass decelerates by friction the whole way. A lofted pass flies for
/// `2 * loft / g` seconds with no friction, so it needs less speed: the flight covers
/// `speed * T` and the roll `speed^2 / (2 f)`, which puts the ball at the team-mate with the
/// arrival speed left.
pub(crate) fn kick_speed(d: f64, loft: f64, t: &Tuning) -> f64 {
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

/// The decision hook's offsets as trace detail.
pub(crate) fn offsets_json(f: &OptionOffsets) -> serde_json::Value {
    json!({"shoot": f.shoot, "pass": f.pass, "dribble": f.dribble, "clear": f.clear, "hold": f.hold})
}

/// Rotates a unit vector by `angle` radians.
pub(crate) fn rotate(v: DVec2, angle: f64) -> DVec2 {
    let (s, c) = math::sin_cos(angle);
    DVec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// Decision maker version 1: formation anchors, the lone forward's line hold, pressing,
/// goal-side cover and chasing for every player, and the scored options, the choice, and
/// the kicks for the carrier.
pub struct DecisionV1;

impl DecisionModule for DecisionV1 {
    fn targets(&self, view: &MatchView<'_>) -> Targets {
        let players = view.players();
        let teams = view.teams();
        let t = view.tuning();
        let ball = view.ball();
        let ball_xy = ball.xy();
        let n = players.len();
        let keepers = [view.keeper(0), view.keeper(1)];
        let mut targets = [DVec2::ZERO; ROSTER];
        for (slot, p) in targets.iter_mut().zip(players) {
            *slot = p.target;
        }
        match view.carrier() {
            Some(c) => {
                let team = players[c].team;
                let def = 1 - team;
                // Teammates hold their anchors; the nearest opponents press, as many and from
                // as far as the defending team's pressing instruction says, less one for each
                // player the defending team has lost.
                let count = teams[def].pressers().min(MAX_PRESSERS);
                let reach = teams[def].plan.press_distance;
                let mut pressers = [(f64::INFINITY, usize::MAX); MAX_PRESSERS];
                for i in 0..n {
                    let p = players[i];
                    if !p.active() {
                        continue;
                    }
                    let anchor = teams[p.team].anchor(p.slot, ball_xy, t);
                    targets[i] = anchor;
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
                hold_the_line(view, &mut targets, team, c);
                // Pressers run at the point where they can meet the carrier's run, and go for
                // the ball once they are close.
                let run = players[c].vel;
                let engage = t.press_engage;
                for &(_, i) in &pressers[..count] {
                    if i != usize::MAX {
                        let p = players[i];
                        // Within engaging range the presser goes for the ball itself.
                        targets[i] = if (p.pos - ball_xy).length() < engage {
                            ball_xy
                        } else {
                            pitch::clamp(intercept(p.pos, p.max_speed(), ball_xy, run), 0.5)
                        };
                    }
                }
                let cover = cover(view, &mut targets, def, c, &pressers[..count], keepers[def]);
                Targets {
                    targets,
                    trace: TargetsTrace::Carrier {
                        def,
                        count,
                        reach,
                        pressers,
                        cover,
                    },
                }
            }
            None => {
                // The nearest player of each team chases; a goalkeeper chases a ball near its
                // goal.
                let predicted =
                    pitch::clamp(ball_xy + DVec2::new(ball.vel.x, ball.vel.y) * 0.3, 0.3);
                let mut nearest = [usize::MAX; 2];
                let mut nearest_dist = [f64::INFINITY; 2];
                for i in 0..n {
                    let p = players[i];
                    if !p.active() {
                        continue;
                    }
                    let anchor = teams[p.team].anchor(p.slot, ball_xy, t);
                    targets[i] = anchor;
                    let d = (p.pos - ball_xy).length();
                    if i != keepers[p.team] && d < nearest_dist[p.team] {
                        nearest_dist[p.team] = d;
                        nearest[p.team] = i;
                    }
                }
                let mut chasing_keepers = [None; 2];
                for (team, &i) in nearest.iter().enumerate() {
                    if i != usize::MAX {
                        targets[i] = predicted;
                    }
                    let gk = keepers[team];
                    if players[gk].active() && (players[gk].pos - ball_xy).length() < 16.0 {
                        targets[gk] = predicted;
                        chasing_keepers[team] = Some(gk);
                    }
                }
                Targets {
                    targets,
                    trace: TargetsTrace::Loose {
                        nearest,
                        nearest_dist,
                        keepers: chasing_keepers,
                    },
                }
            }
        }
    }

    /// Scores every option the carrier has (named mechanism: scored-options decision layer).
    /// Each score is a weighted sum of the option's features with weights from the tuning
    /// file, plus the offsets of the team's plan (mentality, instructions, the carrier's role
    /// and duty); the noise that shrinks as the carrier's decisions and composure rise is the
    /// last term, which [`DecisionModule::scored`] adds.
    fn options(&self, view: &MatchView<'_>, c: usize) -> OptionDraft {
        let t = view.tuning();
        let w = &t.decision;
        let players = view.players();
        let carrier = players[c];
        let team = carrier.team;
        let side = &view.teams()[team];
        let plan = side.plan;
        let role = plan.slots[carrier.slot];
        let goal = side.target_goal();
        let attack = DVec2::new(side.attack_x, 0.0);
        let goal_dist = (goal - carrier.pos).length();
        let keeper = c == view.keeper(team);
        let their_keeper = view.keeper(1 - team);
        let skill = |v: f64| (v - 50.0) / 50.0 * w.skill;
        let noise =
            w.noise * (1.5 - (carrier.derived.decisions + carrier.derived.composure) / 200.0);

        // Pressure and space around the carrier.
        let mut nearest_opp = f64::INFINITY;
        let mut nearest_opp_pos = carrier.pos;
        let mut space_ahead: f64 = 10.0;
        for p in players {
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
            && !players.iter().enumerate().any(|(j, p)| {
                p.team == team
                    && j != c
                    && j != view.keeper(team)
                    && p.active()
                    && p.pos.x * attack.x > depth
            });

        // Shot.
        let shot = if !keeper && goal_dist < t.shot_range {
            let lane = players
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
                    + role.shoot,
            )
        } else {
            None
        };

        // Passes.
        let mut passes = [(0usize, 0.0f64); MATES];
        let mut pass_count = 0;
        for j in 0..players.len() {
            let mate = &players[j];
            if mate.team != team || j == c || !mate.active() {
                continue;
            }
            let d = (mate.pos - carrier.pos).length();
            if !(4.0..=45.0).contains(&d) {
                continue;
            }
            let mut lane: f64 = 6.0;
            let mut receiver_space: f64 = 8.0;
            for opp in players {
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
                + layoff;
            passes[pass_count] = (j, score);
            pass_count += 1;
        }

        let held = view.tick().saturating_sub(view.control_since());
        let held_s = f64::from(held) * t.dt;
        // The carry window: an unpressed carrier who has just gained the ball keeps it
        // rather than releasing it at once. A shot is never charged.
        let carry = if !pressed && held_s < w.carry_s {
            w.carry_cost
        } else {
            0.0
        };
        let dribble_hold = if keeper {
            None
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
                + lone_dribble;
            let hold = w.hold + plan.hold - plan.tempo - w.hold_per_s * held_s + lone_hold;
            Some((dribble, hold))
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
        } - carry;
        OptionDraft {
            noise,
            shot,
            passes,
            pass_count,
            dribble_hold,
            clear,
            carry,
            nearest_opp_pos,
        }
    }

    fn scored(&self, draft: &OptionDraft, draws: &OptionDraws) -> Scored {
        let noise = draft.noise;
        let jitter = |u: f64| map_range(-noise, noise, u);
        let shot = draft.shot.map(|s| s + jitter(draws.shot));
        let mut pass: Option<(f64, usize)> = None;
        let mut candidates = [(0usize, 0.0f64); MATES];
        let drafted = draft.passes[..draft.pass_count].iter().zip(&draws.passes);
        for (candidate, (&(j, s), &u)) in candidates.iter_mut().zip(drafted) {
            let score = s + jitter(u);
            *candidate = (j, score);
            if pass.is_none_or(|(best, _)| score > best) {
                pass = Some((score, j));
            }
        }
        let pass = pass.map(|(s, j)| (s - draft.carry, j));
        let (dribble, hold) = match draft.dribble_hold {
            Some((dribble, hold)) => (
                Some(dribble + jitter(draws.dribble_hold.0)),
                Some(hold + jitter(draws.dribble_hold.1)),
            ),
            None => (None, None),
        };
        let clear = draft.clear + jitter(draws.clear);
        Scored {
            options: Options {
                shot,
                pass,
                dribble,
                clear,
                hold,
                nearest_opp_pos: draft.nearest_opp_pos,
            },
            candidates,
            count: draft.pass_count,
        }
    }

    fn choose(
        &self,
        _: &MatchView<'_>,
        _: usize,
        options: &Options,
        offsets: Option<OptionOffsets>,
    ) -> (Options, Choice) {
        let mut o = *options;
        if let Some(off) = offsets {
            o.shot = o.shot.map(|s| s + off.shoot);
            o.pass = o.pass.map(|(s, j)| (s + off.pass, j));
            o.dribble = o.dribble.map(|s| s + off.dribble);
            o.clear += off.clear;
            o.hold = o.hold.map(|s| s + off.hold);
        }
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
        (o, choice.1)
    }

    fn plan(
        &self,
        view: &MatchView<'_>,
        c: usize,
        options: &Options,
        choice: Choice,
    ) -> CarrierPlan {
        let t = view.tuning();
        let carrier = view.player(c);
        let team = carrier.team;
        let side = &view.teams()[team];
        let goal = side.target_goal();
        let attack = DVec2::new(side.attack_x, 0.0);
        match choice {
            Choice::Shot => CarrierPlan::Shot {
                goal,
                keeper: view.keeper(1 - team),
            },
            Choice::Pass => CarrierPlan::Pass {
                j: options.pass.map_or(c, |(_, j)| j),
            },
            Choice::Clear => {
                // Clear long toward the far half; near his own goal line a clearance may go
                // wide toward that line instead.
                let own_goal_x = -side.attack_x;
                let c = &t.clearances;
                let wide = c.wide_chance > 0.0
                    && carrier.pos.x * own_goal_x.signum() >= pitch::HALF_LENGTH - c.wide_depth;
                CarrierPlan::Clear {
                    wide_chance: wide.then_some(c.wide_chance),
                }
            }
            Choice::Hold => CarrierPlan::Move {
                target: carrier.pos,
            },
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
                let side = if (options.nearest_opp_pos - carrier.pos).dot(perp) > 0.0 {
                    -1.0
                } else {
                    1.0
                };
                let target = carrier.pos + dir_goal * 8.0 + perp * (4.0 * side);
                CarrierPlan::Move {
                    target: pitch::clamp(target, 1.0),
                }
            }
        }
    }

    fn pass_kick(&self, view: &MatchView<'_>, c: usize, j: usize, aim: f64) -> Kick {
        let t = view.tuning();
        let carrier = view.player(c);
        let mate_pos = view.player(j).pos;
        let d = (mate_pos - carrier.pos).length();
        let skill = carrier.derived.passing / 100.0;
        let noise = t.aim_noise * (1.5 - skill);
        let dir = rotate(toward(carrier.pos, mate_pos), map_range(-noise, noise, aim));
        let loft = if d > 25.0 { 3.0 + d * 0.08 } else { 0.0 };
        let speed = kick_speed(d, loft, t);
        Kick::Pass { dir, speed, loft }
    }

    fn clear_kick(&self, view: &MatchView<'_>, c: usize, wide: bool, aim: f64) -> Kick {
        let t = view.tuning();
        let carrier = view.player(c);
        let attack_x = view.teams()[carrier.team].attack_x;
        let own_goal_x = -attack_x;
        let (line, spread) = if wide {
            (wide_of_goal(carrier.pos, own_goal_x), WIDE_SPREAD)
        } else {
            (DVec2::new(attack_x, 0.0), t.clearances.aim_spread)
        };
        let dir = rotate(line, map_range(-spread, spread, aim));
        Kick::Clear {
            dir,
            speed: kick_speed(CLEARANCE_DISTANCE, CLEARANCE_LOFT, t),
            loft: CLEARANCE_LOFT,
        }
    }

    fn shot_draws_side(&self, view: &MatchView<'_>, keeper: usize) -> bool {
        !view.player(keeper).active()
    }

    /// Player `c` shoots at the goal centred on `goal`, which `keeper` defends. The shot aims
    /// for the side of the goal away from the goalkeeper; a better finisher places it nearer
    /// the post and strikes it truer. `spread_scale` scales the aim noise: 1 in open play,
    /// and less for a placed kick from the penalty mark.
    fn shot_kick(
        &self,
        view: &MatchView<'_>,
        c: usize,
        goal: DVec2,
        keeper: usize,
        spread_scale: f64,
        draws: &ShotDraws,
    ) -> Kick {
        let t = view.tuning();
        let carrier = view.player(c);
        let finishing = (carrier.derived.finishing / 100.0).clamp(0.0, 1.0);
        let keeper = view.player(keeper);
        let side = if !keeper.active() {
            if draws.side == Some(true) { 1.0 } else { -1.0 }
        } else if keeper.pos.y > goal.y {
            -1.0
        } else {
            1.0
        };
        let reach = pitch::GOAL_WIDTH / 2.0 * (0.4 + 0.5 * finishing);
        let aim = DVec2::new(goal.x, side * map_range(0.5 * reach, reach, draws.aim));
        let spread = t.shot_noise * (1.5 - finishing) * spread_scale;
        let dir = rotate(
            toward(carrier.pos, aim),
            map_range(-spread, spread, draws.spread),
        );
        Kick::Shot {
            dir,
            speed: t.shot_speed,
            loft: map_range(0.0, t.shots.loft_max, draws.loft),
        }
    }

    /// The kick that takes a restart: a short lofted throw to the nearest team-mate, a cross
    /// toward the penalty mark from a corner, or a pass to an open team-mate. With no
    /// team-mate in range, the ball goes forward along the pitch.
    fn restart_pass(&self, view: &MatchView<'_>, taker: usize, kind: StoppageKind) -> RestartPass {
        let t = view.tuning();
        let players = view.players();
        let me = players[taker];
        let attack = DVec2::new(view.teams()[me.team].attack_x, 0.0);
        let cross_at = (kind == StoppageKind::Corner).then(|| pitch::penalty_spot(attack.x));
        let range = if kind == StoppageKind::ThrowIn {
            3.0..=THROW_RANGE
        } else {
            4.0..=45.0
        };
        let mut candidates = [(0usize, 0.0f64); MATES];
        let mut count = 0;
        let mut best: Option<(f64, DVec2)> = None;
        for (j, mate) in players.iter().enumerate() {
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
            candidates[count] = (j, score);
            count += 1;
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
        RestartPass {
            kick: Kick::Pass {
                dir,
                speed: kick_speed(d, loft, t),
                loft,
            },
            target: to,
            fallback: best.is_none(),
            candidates,
            count,
        }
    }
}

/// The line hold (named mechanism) of team `team`'s lone forward while his team has the
/// ball and carrier `c` is someone else: his target moves from his anchor toward the
/// onside line, 0.5 m short of the second-last defender, by the share `lone_line_hold`,
/// and never past that line. His place across the pitch stays the anchor's.
fn hold_the_line(view: &MatchView<'_>, targets: &mut [DVec2; ROSTER], team: usize, c: usize) {
    let share = view.tuning().lone_line_hold;
    if share == 0.0 {
        return;
    }
    let side = &view.teams()[team];
    let Some(slot) = side.lone_forward() else {
        return;
    };
    let i = team * PLAYERS_PER_TEAM + slot;
    if i == c || !view.player(i).active() {
        return;
    }
    let attack_x = side.attack_x;
    let line = offside::second_last_depth(team, view.players(), attack_x) - ONSIDE_MARGIN;
    if !line.is_finite() {
        return;
    }
    let anchor = targets[i];
    let depth = anchor.x * attack_x;
    let held = (depth + share * (line - depth)).min(line);
    targets[i] = pitch::clamp(DVec2::new(held * attack_x, anchor.y), 0.5);
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
fn cover(
    view: &MatchView<'_>,
    targets: &mut [DVec2; ROSTER],
    def: usize,
    c: usize,
    pressers: &[(f64, usize)],
    keeper: usize,
) -> CoverTrace {
    let t = view.tuning();
    let players = view.players();
    let side = &view.teams()[def];
    let first = def * PLAYERS_PER_TEAM;
    let goal = side.own_goal();
    // Distance from `def`'s goal line toward the halfway line.
    let depth = |at: DVec2| at.x * side.attack_x + pitch::HALF_LENGTH;
    let covered = |p: &crate::player::Player| {
        p.active() && p.pos.y.abs() <= t.cover_channel && depth(p.pos) < pitch::HALF_LENGTH
    };
    let attacker = if covered(&players[c]) {
        Some((depth(players[c].pos), c))
    } else {
        let mut most: Option<(f64, usize)> = None;
        for (j, p) in players.iter().enumerate() {
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
        return CoverTrace::NoAttacker;
    };
    let at = players[j].pos;
    let mut best: Option<(f64, usize)> = None;
    for slot in side.back_line() {
        let i = first + slot;
        let p = &players[i];
        if !p.active() || i == keeper || pressers.iter().any(|&(_, k)| k == i) {
            continue;
        }
        let d = (p.pos - at).length();
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, i));
        }
    }
    let Some((distance, i)) = best else {
        return CoverTrace::NoDefender { attacker: j };
    };
    let me = players[i];
    let hold = if j == c {
        reach - t.cover_distance
    } else {
        // The defender's own place in the line: its anchor, set earlier this tick.
        depth(targets[i])
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
        spot += players[c].vel * (braking / me.max_speed());
    }
    targets[i] = pitch::clamp(spot, 0.5);
    CoverTrace::Covered {
        attacker: j,
        defender: i,
        distance,
    }
}

pub const DECISION_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Sets every player's target (anchors, the lone forward's line hold, pressing, goal-side cover, chasing) and decides the carrier's option, pass, clearance, shot, and restart kick.",
    inputs: "Every player's position, velocity, target, activity, slot, and effective values; both teams' shapes and plans; the ball; the carrier and how long he has held the ball; the tick; and the engine tuning.",
    outputs: "Every player's target, the carrier's option scores and choice, and the kick of a pass, clearance, shot, or restart.",
    tuning: &[
        "decision",
        "shot_range",
        "aim_noise",
        "shot_noise",
        "shot_speed",
        "shots.loft_max",
        "press_engage",
        "cover_channel",
        "cover_distance",
        "arrive_radius",
        "lone_line_hold",
        "clearances.aim_spread",
        "clearances.wide_chance",
        "clearances.wide_depth",
        "pass_arrival_speed",
        "pass_max_speed",
        "ground_friction",
        "gravity",
        "dt",
    ],
    calibration: "pass_accuracy_pct",
    keys: &[
        Action::ShotScore,
        Action::PassScore,
        Action::DribbleScore,
        Action::HoldScore,
        Action::ClearScore,
        Action::PassAim,
        Action::ClearWide,
        Action::ClearAim,
        Action::ShotSide,
        Action::ShotAim,
        Action::ShotSpread,
        Action::ShotLoft,
    ],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_config;
    use crate::sim::Simulation;
    use crate::streams::{Action, Key};

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
}

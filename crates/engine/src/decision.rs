//! The possession decision. The ball carrier scores its options (shoot, pass to each open
//! team-mate, dribble, clear, hold) and takes the highest; the team's pressing instruction
//! sets how many opponents press and from how far; every other player holds its formation
//! anchor, which the team's plan moves. Every agent decides every tick (product-owner choice;
//! `Tuning::decision_interval_ticks`).
//! A sent-off player takes no part in any decision. While the ball is dead, the referee sets
//! every target instead, and a restart kick comes from `restart_pass`.
//!
//! Every attribute read goes through the attribute contract ([`crate::contract`]): each
//! option's skill term reads the carrier's choose stage, the noise his choose and pressure
//! stages, each kick its execute stage (with the pressure stage when an opponent is close),
//! and each player's press reach, run reading, keeper range, and place keeping their own
//! per-player factors. The skill gate keeps a carrier from trying a chip or a take-on his
//! technique does not allow.
//!
//! The nearest hooks of the newer attributes act here too: marking sets how tight goal-side
//! cover tracks a carrier, off-ball movement brings an attacker beyond the offside line back
//! toward it and gives him space at his anchor, flair raises a forward pass's score,
//! determination widens a presser's reach while his side trails, eccentricity widens a
//! keeper's chase range, and the set-piece skills set a restart's delivery, a direct free
//! kick's strike and a throw-in's range.

use crate::contract::{self, Stage};
use crate::data::rules::StoppageKind;
use crate::math::{self, DVec2, segment_distance, toward};
use crate::modules::{
    CarrierPlan, CoverTrace, DecisionModule, MATES, MatchView, ModuleCard, OptionDraft,
    OptionDraws, ROSTER, RestartPass, Scored, ShotDraws, ShotKind, Targets, TargetsTrace,
};
use crate::pace::Job;
use crate::pitch;
use crate::plugin::OptionOffsets;
use crate::rules::offside;
use crate::shape;
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
/// The furthest ahead, in seconds, an average presser aims along the carrier's run: beyond
/// it the carrier's run is too uncertain to chase. A presser's intercept stage (anticipation)
/// scales it.
const INTERCEPT_HORIZON_S: f64 = 1.0;

/// The distance in metres from the ball within which an average keeper comes off his line
/// for a loose ball; his rush stage scales it.
const KEEPER_CHASE_M: f64 = 16.0;

/// Where a player at `p` with top speed `s` meets a carrier at `c` running at `v`: `c + v t`,
/// where `t` is the smallest positive time with `|c + v t - p| = s t`, capped at `horizon`
/// seconds. With no such time (a carrier running away faster than the player) the point is
/// the carrier.
pub(crate) fn intercept(p: DVec2, s: f64, c: DVec2, v: DVec2, horizon: f64) -> DVec2 {
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
        Some(t) => c + v * t.min(horizon),
        None => c,
    }
}

/// What kind of pass a kick is, for the stage it reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassKind {
    /// A ground pass up to 25 m.
    Pass,
    /// A lofted pass over 25 m: the gated chip.
    Chip,
    /// A pass from a wide channel into the penalty area the carrier attacks.
    Cross,
    /// A keeper's throw, up to 30 m.
    Throw,
    /// A keeper's kick, over 30 m.
    Kick,
}

impl PassKind {
    /// The kind of a pass `d` metres long from `from` to `to` by a carrier attacking
    /// `attack_x`; `keeper` when the carrier keeps goal.
    pub fn of(
        from: DVec2,
        to: DVec2,
        d: f64,
        attack_x: f64,
        keeper: bool,
        pitch: &crate::pitch::Pitch,
    ) -> Self {
        if keeper {
            if d <= contract::THROW_M {
                PassKind::Throw
            } else {
                PassKind::Kick
            }
        } else if from.y.abs() > contract::CROSS_WIDE_M && pitch.in_penalty_area(to, attack_x) {
            PassKind::Cross
        } else if d > contract::CHIP_M {
            PassKind::Chip
        } else {
            PassKind::Pass
        }
    }

    /// The choose stage of an outfield pass of this kind.
    fn choose(self) -> Stage {
        match self {
            PassKind::Chip => Stage::CHIP_CHOOSE,
            PassKind::Cross => Stage::CROSS_CHOOSE,
            _ => Stage::PASS_CHOOSE,
        }
    }

    /// The execute and pressure stages of a pass of this kind; a keeper's distribution has
    /// no pressure stage.
    fn execute(self) -> (Stage, Option<Stage>) {
        match self {
            PassKind::Pass => (Stage::PASS_EXECUTE, Some(Stage::PASS_PRESSURE)),
            PassKind::Chip => (Stage::CHIP_EXECUTE, Some(Stage::CHIP_PRESSURE)),
            PassKind::Cross => (Stage::CROSS_EXECUTE, Some(Stage::CROSS_PRESSURE)),
            PassKind::Throw => (Stage::KEEPER_THROW_EXECUTE, None),
            PassKind::Kick => (Stage::KEEPER_KICK_EXECUTE, None),
        }
    }
}

/// `true` when an active opponent of player `c` stands within the pressing distance.
fn pressed(view: &MatchView<'_>, c: usize) -> bool {
    let me = view.player(c);
    view.players()
        .iter()
        .any(|p| p.team != me.team && p.active() && (p.pos - me.pos).length() < contract::PRESSED_M)
}

/// The execution share of a kick: the execute stage's share, blended with the pressure
/// stage's when an opponent is close, then moved down by `penalty` in log-odds (a gated skill
/// the body cannot pull off).
fn execution(
    stages: contract::Skills<'_>,
    execute: Stage,
    pressure: Option<Stage>,
    pressed: bool,
    penalty: f64,
) -> f64 {
    let share = match pressure {
        Some(p) if pressed => stages.pressed_share(execute, p),
        _ => stages.share(execute),
    };
    if penalty > 0.0 && share > 0.0 && share < 1.0 {
        contract::logistic(contract::logit(share) - penalty)
    } else {
        share
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
    fn lapse_chance(&self, view: &MatchView<'_>, i: usize) -> f64 {
        lapse_chance(view, i)
    }

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
                // While the defending side trails, each presser's determination widens his
                // reach.
                let goals = view.goals();
                let behind = goals[def] < goals[team];
                let behind_effort = t.contract.hooks().behind_effort;
                let mut pressers = [(f64::INFINITY, usize::MAX); MAX_PRESSERS];
                // The carrier's team is in possession.
                let phase = Some(team);
                let anchors = shape::anchors(view, phase);
                for i in 0..n {
                    let p = &players[i];
                    if !p.active() {
                        continue;
                    }
                    let anchor = anchors[i];
                    // Out of possession, a player who keeps his place less well than the
                    // average (shape stage under 1) trails his anchor by the shortfall.
                    let lag = 1.0 - p.derived.knobs.shape;
                    targets[i] = if p.team != team && lag > 0.0 && i != keepers[def] {
                        anchor + (p.pos - anchor) * lag
                    } else {
                        anchor
                    };
                    if p.team != team && i != keepers[def] && count > 0 {
                        let d = (p.pos - ball_xy).length();
                        // Each presser's own reach: his press stage (work rate) scales the
                        // team's pressing distance.
                        let effort = if behind {
                            1.0 + behind_effort * p.derived.occasion.determination
                        } else {
                            1.0
                        };
                        if d < reach * p.derived.knobs.press * effort && d < pressers[count - 1].0 {
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
                // Stand options from the role menus and the rest-defence guard; pressing,
                // the line hold and cover then take their players from there.
                let shaped = shape::apply(view, phase, &anchors, &mut targets);
                hold_the_line(view, &mut targets, team, c);
                time_runs(view, &mut targets, team, c);
                // Pressers run at the point where they can meet the carrier's run, and go for
                // the ball once they are close.
                let run = players[c].vel;
                let engage = t.press_engage;
                for &(_, i) in &pressers[..count] {
                    if i != usize::MAX {
                        let p = &players[i];
                        // Within engaging range the presser goes for the ball itself.
                        targets[i] = if (p.pos - ball_xy).length() < engage {
                            ball_xy
                        } else {
                            let horizon = INTERCEPT_HORIZON_S * p.derived.knobs.intercept;
                            view.pitch()
                                .clamp(intercept(p.pos, p.max_speed(), ball_xy, run, horizon), 0.5)
                        };
                    }
                }
                let cover = cover(view, &mut targets, def, c, &pressers[..count], keepers[def]);
                hold_lapses(view, &mut targets);
                let mut jobs = shape_jobs(view, phase, &targets, &shaped);
                jobs[c] = Job::Carry;
                for &(_, i) in &pressers[..count] {
                    if i != usize::MAX {
                        jobs[i] = Job::Press;
                    }
                }
                recovery_jobs(view, &mut jobs, def, keepers[def]);
                if let CoverTrace::Covered { defender, .. } = cover {
                    // The cover defender runs urgently only while the carrier is goal-side
                    // of him: between him and his own goal.
                    let ax = teams[def].attack_x;
                    if (players[c].pos.x - players[defender].pos.x) * ax < 0.0 {
                        jobs[defender] = Job::Recover;
                    }
                }
                lapse_jobs(view, &mut jobs);
                Targets {
                    targets,
                    jobs,
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
                let predicted = view
                    .pitch()
                    .clamp(ball_xy + DVec2::new(ball.vel.x, ball.vel.y) * 0.3, 0.3);
                let mut nearest = [usize::MAX; 2];
                let mut nearest_dist = [f64::INFINITY; 2];
                // A loose ball or a pass in flight: each team keeps its phase until a player
                // of the other team controls the ball.
                let phase = view.possession_phase();
                let anchors = shape::anchors(view, phase);
                for i in 0..n {
                    let p = &players[i];
                    if !p.active() {
                        continue;
                    }
                    targets[i] = anchors[i];
                    let d = (p.pos - ball_xy).length();
                    if i != keepers[p.team] && d < nearest_dist[p.team] {
                        nearest_dist[p.team] = d;
                        nearest[p.team] = i;
                    }
                }
                let shaped = shape::apply(view, phase, &anchors, &mut targets);
                let mut chasing_keepers = [None; 2];
                for (team, &i) in nearest.iter().enumerate() {
                    if i != usize::MAX {
                        targets[i] = predicted;
                    }
                    let gk = keepers[team];
                    let chase = keeper_chase_m(view, gk);
                    if players[gk].active() && (players[gk].pos - ball_xy).length() < chase {
                        targets[gk] = predicted;
                        chasing_keepers[team] = Some(gk);
                    }
                }
                hold_lapses(view, &mut targets);
                let mut jobs = shape_jobs(view, phase, &targets, &shaped);
                // The chaser who reaches the ball first, at his top speed, goes for a loose
                // ball; the other team's chaser chases.
                let time =
                    |i: usize| (players[i].pos - predicted).length() / players[i].max_speed();
                let first = match nearest {
                    [a, b] if a != usize::MAX && b != usize::MAX => {
                        Some(if time(b) < time(a) { b } else { a })
                    }
                    [a, b] => (a != usize::MAX)
                        .then_some(a)
                        .or((b != usize::MAX).then_some(b)),
                };
                for &i in &nearest {
                    if i != usize::MAX {
                        jobs[i] = if Some(i) == first {
                            Job::LooseBall
                        } else {
                            Job::Chase
                        };
                    }
                }
                for gk in chasing_keepers.into_iter().flatten() {
                    jobs[gk] = Job::LooseBall;
                }
                // A pass in flight or a loose ball: the team out of possession (both, with no
                // phase) recovers as it does against a carrier. A chaser keeps his chase.
                for (def, &keeper) in keepers.iter().enumerate() {
                    if phase != Some(def) {
                        recovery_jobs(view, &mut jobs, def, keeper);
                    }
                }
                lapse_jobs(view, &mut jobs);
                Targets {
                    targets,
                    jobs,
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
    /// and duty); the noise that shrinks as the carrier's choose stage (decisions) rises, and
    /// with an opponent close his pressure stage (composure), is the last term, which
    /// [`DecisionModule::scored`] adds.
    fn options(&self, view: &MatchView<'_>, c: usize) -> OptionDraft {
        let t = view.tuning();
        let w = &t.decision;
        let players = view.players();
        let carrier = &players[c];
        let stages = view.skills(c);
        let gates = carrier.derived.gates;
        let team = carrier.team;
        let side = &view.teams()[team];
        let plan = side.plan;
        let role = plan.slots[carrier.slot];
        let goal = side.target_goal();
        let attack = DVec2::new(side.attack_x, 0.0);
        let goal_dist = (goal - carrier.pos).length();
        let keeper = c == view.keeper(team);
        let their_keeper = view.keeper(1 - team);
        // An option's skill term: `(2 · share − 1) · skill`, 0 at rating 10.
        let skill = |s: Stage| stages.skill(s) * w.skill;
        let flair_risk = t.contract.hooks().flair_risk;
        // Mood: a carrier in good mood shoots and plays forward more and holds less; in low
        // mood the reverse. 0 at a neutral mood and with no person block.
        let mood_risk = t.contract.person().mood.risk * view.mood(c) / crate::sim::mood::FULL;

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
        let pressed = nearest_opp < contract::PRESSED_M;
        let judgement = if pressed {
            stages.pressed_share(Stage::PASS_CHOOSE, Stage::PASS_PRESSURE)
        } else {
            stages.share(Stage::PASS_CHOOSE)
        };
        let noise = w.noise * (1.5 - judgement);
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
            let choose = if view.pitch().in_penalty_area(carrier.pos, side.attack_x) {
                Stage::SHOT_CHOOSE
            } else {
                Stage::LONG_SHOT_CHOOSE
            };
            Some(
                w.shot_base + w.shot_lane * ((lane.min(5.0) - 2.5) / 2.5)
                    - w.shot_distance * goal_dist / t.shot_range
                    + under_pressure
                    + skill(choose)
                    + plan.shoot
                    + role.shoot
                    + mood_risk,
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
            let kind = PassKind::of(
                carrier.pos,
                mate.pos,
                d,
                side.attack_x,
                keeper,
                view.pitch(),
            );
            // The skill gate: a carrier whose technique is under the chip's threshold does
            // not try one.
            if kind == PassKind::Chip && !gates.chip_try {
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
            // Flair: a carrier with it rates a forward pass higher, one without it lower.
            let flair = flair_risk * carrier.derived.occasion.flair * forward;
            let layoff = if lone && progress <= 0.0 {
                w.lone_layoff
            } else {
                0.0
            };
            let score = (w.progress + plan.progress + role.progress) * progress
                + skill(Stage::PASS_SEE) * forward
                + w.teamwork_bonus * stages.skill(kind.choose())
                + w.lane * (lane / 6.0)
                + w.space * (receiver_space / 8.0)
                - w.distance * (d / 45.0)
                + plan.directness * (d / 45.0)
                + plan.tempo
                + layoff
                + plan.slots[mate.slot].long_ball_target * (d / 45.0)
                + flair
                + mood_risk * forward;
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
                + skill(Stage::DRIBBLE_CHOOSE)
                + role.dribble
                - plan.tempo
                + lone_dribble;
            let hold =
                w.hold + plan.hold - plan.tempo - w.hold_per_s * held_s + lone_hold - mood_risk;
            // The skill gate: a pressed carrier whose technique is under the take-on's
            // threshold does not try to dribble past; he may still hold.
            let dribble = (!pressed || gates.take_on_try).then_some(dribble);
            Some((dribble, hold))
        };
        let own_third = carrier.pos.x * attack.x < -view.pitch().half_length() / 3.0;
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
                dribble.map(|d| d + jitter(draws.dribble_hold.0)),
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
                    && carrier.pos.x * own_goal_x.signum()
                        >= view.pitch().half_length() - c.wide_depth;
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
                    target: view.pitch().clamp(target, 1.0),
                }
            }
        }
    }

    /// A pass's aim noise falls as its execution share rises: the execute stage of its kind
    /// (passing, technique for a chip, crossing, a keeper's throwing or kicking), with the
    /// pressure stage when an opponent is close, and a chip the body cannot pull off loses
    /// its gate penalty.
    fn pass_kick(&self, view: &MatchView<'_>, c: usize, j: usize, aim: f64) -> Kick {
        let t = view.tuning();
        let carrier = view.player(c);
        let mate_pos = view.player(j).pos;
        let d = (mate_pos - carrier.pos).length();
        let attack_x = view.teams()[carrier.team].attack_x;
        let keeper = c == view.keeper(carrier.team);
        let kind = PassKind::of(carrier.pos, mate_pos, d, attack_x, keeper, view.pitch());
        let (execute, pressure) = kind.execute();
        let penalty = if kind == PassKind::Chip {
            carrier.derived.gates.chip_penalty
        } else {
            0.0
        };
        let under = pressure.is_some() && pressed(view, c);
        let skill = execution(view.skills(c), execute, pressure, under, penalty);
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
        // A keeper's clearance is a kick: his kicking stage narrows its spread.
        let spread = if c == view.keeper(carrier.team) {
            spread * (1.5 - view.skills(c).share(Stage::KEEPER_KICK_EXECUTE))
        } else {
            spread
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
    /// the post and strikes it truer. A placed kick from the penalty mark has a smaller aim
    /// noise (`shots.penalty_spread`). The execution share is the shot execute stage
    /// (finishing) from inside the penalty area and the long-shot stage from outside it, each
    /// with its pressure stage when an opponent is close; the penalty stage for a penalty,
    /// with its pressure stage; and the free-kick stage, with nobody close, for a direct free
    /// kick.
    fn shot_kick(
        &self,
        view: &MatchView<'_>,
        c: usize,
        goal: DVec2,
        keeper: usize,
        kind: ShotKind,
        draws: &ShotDraws,
    ) -> Kick {
        let t = view.tuning();
        let carrier = view.player(c);
        let attack_x = view.teams()[carrier.team].attack_x;
        let (execute, pressure) = match kind {
            ShotKind::Penalty => (Stage::PENALTY_EXECUTE, Some(Stage::PENALTY_PRESSURE)),
            ShotKind::FreeKick => (Stage::FREE_KICK_EXECUTE, None),
            ShotKind::Open if view.pitch().in_penalty_area(carrier.pos, attack_x) => {
                (Stage::SHOT_EXECUTE, Some(Stage::SHOT_PRESSURE))
            }
            ShotKind::Open => (Stage::LONG_SHOT_EXECUTE, Some(Stage::LONG_SHOT_PRESSURE)),
        };
        // Pressure handling narrows a penalty's spread, a shoot-out kick's included.
        let spread_scale = match kind {
            ShotKind::Penalty => {
                t.shots.penalty_spread
                    * (1.0
                        - t.contract.person().occasions.penalty_pressure
                            * view.personality(c).pressure_handling)
            }
            _ => 1.0,
        };
        let under = pressure.is_some() && pressed(view, c);
        let finishing = execution(view.skills(c), execute, pressure, under, 0.0).clamp(0.0, 1.0);
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
    /// toward the penalty mark from a corner or a direct free kick, or a pass to an open
    /// team-mate. With no team-mate in range, the ball goes forward along the pitch. A
    /// throw-in looks for team-mates as far as his long-throw factor carries `THROW_RANGE`; a
    /// corner's or free kick's delivery turns from its aim by the aim draw, up to
    /// `hooks.restart_noise` for an average taker, less for a better one.
    fn restart_pass(
        &self,
        view: &MatchView<'_>,
        taker: usize,
        kind: StoppageKind,
        direct: bool,
        aim: f64,
    ) -> RestartPass {
        let t = view.tuning();
        let c = &t.contract;
        let players = view.players();
        let me = players[taker];
        let skills = view.skills(taker);
        let attack = DVec2::new(view.teams()[me.team].attack_x, 0.0);
        let delivered = kind == StoppageKind::Corner || (kind == StoppageKind::FreeKick && direct);
        let cross_at = delivered.then(|| view.pitch().penalty_spot(attack.x));
        let range = if kind == StoppageKind::ThrowIn {
            let long = contract::factor(
                c.actions.of(contract::ActionKind::LongThrow).spread(),
                skills.share(Stage::LONG_THROW_EXECUTE),
            );
            3.0..=(THROW_RANGE * long)
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
        let to = best.map_or_else(
            || view.pitch().clamp(me.pos + attack * 10.0, 4.0),
            |(_, at)| at,
        );
        let d = (to - me.pos).length();
        let dir = match toward(me.pos, to) {
            v if v == DVec2::ZERO => attack,
            v => v,
        };
        // The delivery error of a corner or a free kick: its stage's share narrows the turn.
        let delivery = match kind {
            StoppageKind::Corner => Some(Stage::CORNER_EXECUTE),
            StoppageKind::FreeKick => Some(Stage::FREE_KICK_EXECUTE),
            _ => None,
        };
        let noise = c.hooks().restart_noise;
        let dir = match delivery {
            Some(stage) if noise > 0.0 => {
                let turn = noise * (1.5 - skills.share(stage));
                rotate(dir, map_range(-turn, turn, aim))
            }
            _ => dir,
        };
        let loft = match kind {
            StoppageKind::ThrowIn => THROW_LOFT,
            StoppageKind::Corner => 6.0 + 0.1 * d,
            StoppageKind::FreeKick if delivered => 6.0 + 0.1 * d,
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

/// The chance per minute of play that player `i` lapses: the tuned chance, growing past
/// the late minute by `late_growth` per 30 minutes, times `2 · (1 − share)` of his shape
/// choose stage (concentration), which is 1 at rating 10.
pub fn lapse_chance(view: &MatchView<'_>, i: usize) -> f64 {
    let l = &view.tuning().contract.lapse;
    let minute = view.tick() / crate::rules::clock::TICKS_PER_MINUTE;
    let late = f64::from(minute.saturating_sub(l.late_from_minute)) / 30.0;
    let share = view.skills(i).share(Stage::SHAPE_CHOOSE);
    // Professionalism keeps a player's concentration.
    let professional = 1.0
        - view.tuning().contract.person().occasions.professionalism
            * view.personality(i).professionalism;
    (l.per_minute * (1.0 + l.late_growth * late) * 2.0 * (1.0 - share) * professional)
        .clamp(0.0, 1.0)
}

/// The job of every active player from his shape: `Recover` when the rest-defence guard
/// held him back to keep the mentality minimum goal-side and his spot is farther than
/// `return_far_m` (goal-side cover is missing until he gets there), `Offer` when his team is
/// in possession and his stand option fired, `Return` farther than `return_far_m` from his
/// target, `Follow` within `calm_ball_m` of the ball, otherwise `Hold`. An inactive player's
/// job is `Hold`. Out of possession a stand option offers no pass, so its player's job comes
/// from the distances.
fn shape_jobs(
    view: &MatchView<'_>,
    phase: Option<usize>,
    targets: &[DVec2; ROSTER],
    shaped: &[shape::ShapeOutcome; 2],
) -> [Job; ROSTER] {
    let t = &view.tuning().pace;
    let ball = view.ball().xy();
    let mut jobs = [Job::Hold; ROSTER];
    for ((job, p), &target) in jobs.iter_mut().zip(view.players()).zip(targets) {
        if !p.active() {
            continue;
        }
        let far = (target - p.pos).length() > t.return_far_m;
        *job = if far && shaped[p.team].held & (1 << p.slot) != 0 {
            Job::Recover
        } else if phase == Some(p.team) && shaped[p.team].option[p.slot].is_some() {
            Job::Offer
        } else if far {
            Job::Return
        } else if (p.pos - ball).length() <= t.calm_ball_m {
            Job::Follow
        } else {
            Job::Hold
        };
    }
    jobs
}

/// Recovery runs while goal-side cover is missing: while team `def` has fewer active
/// outfield players goal-side of the ball than its mentality minimum (never more than its
/// active outfield players less one), the players ahead of the ball nearest to getting
/// goal-side recover, one per missing player. A presser keeps his press, and a chaser his
/// chase.
fn recovery_jobs(view: &MatchView<'_>, jobs: &mut [Job; ROSTER], def: usize, keeper: usize) {
    let players = view.players();
    let side = &view.teams()[def];
    let ball = view.ball().xy();
    let first = def * PLAYERS_PER_TEAM;
    // Metres ahead of the ball toward the other goal: below zero is goal-side.
    let ahead = |i: usize| (players[i].pos.x - ball.x) * side.attack_x;
    let outfield =
        || (first..first + PLAYERS_PER_TEAM).filter(|&i| i != keeper && players[i].active());
    let active = outfield().count();
    let minimum = side.plan.rest_defence.min(active.saturating_sub(1));
    let behind = outfield().filter(|&i| ahead(i) < 0.0).count();
    let mut missing = minimum.saturating_sub(behind);
    if missing == 0 {
        return;
    }
    let mut candidates: Vec<(f64, usize)> = outfield()
        .filter(|&i| {
            ahead(i) >= 0.0 && !matches!(jobs[i], Job::Press | Job::LooseBall | Job::Chase)
        })
        .map(|i| (ahead(i), i))
        .collect();
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    for (_, i) in candidates {
        if missing == 0 {
            break;
        }
        jobs[i] = Job::Recover;
        missing -= 1;
    }
}

/// A player in a concentration lapse holds: his job is `Hold` until the lapse ends.
fn lapse_jobs(view: &MatchView<'_>, jobs: &mut [Job; ROSTER]) {
    let tick = view.tick();
    for (job, p) in jobs.iter_mut().zip(view.players()) {
        if tick < p.lapse_until && p.active() {
            *job = Job::Hold;
        }
    }
}

/// A player in a concentration lapse holds where he stands: his target is his position until
/// the lapse ends.
fn hold_lapses(view: &MatchView<'_>, targets: &mut [DVec2; ROSTER]) {
    let tick = view.tick();
    for (slot, p) in targets.iter_mut().zip(view.players()) {
        if tick < p.lapse_until && p.active() {
            *slot = p.pos;
        }
    }
}

/// The distance from the ball within which keeper `gk` comes off his line for a loose ball:
/// the average keeper's chase distance times his rush factor, widened by his eccentricity
/// (`hooks.rash_chase`).
pub(crate) fn keeper_chase_m(view: &MatchView<'_>, gk: usize) -> f64 {
    let d = &view.player(gk).derived;
    let rash = view.tuning().contract.hooks().rash_chase;
    KEEPER_CHASE_M * d.knobs.rush * (1.0 + rash * d.occasion.eccentricity)
}

/// `true` when keeper `gk` chasing a ball `distance` metres away is a rash chase: one an
/// average keeper would not start.
#[cfg(feature = "sensitivity")]
pub(crate) fn rash_chase(distance: f64) -> bool {
    distance > KEEPER_CHASE_M
}

/// Timed runs (the off-ball movement hook) of team `team` while it has the ball and carrier
/// `c` is someone else: every active outfield attacker with off-ball movement above the
/// average (skill `s` of his off-ball run stage, 0 to 1) whose target lies beyond the onside
/// line, 0.5 m short of the second-last defender, is brought back toward that line by the
/// share `s`; and his target moves `hooks.anchor_space_m · s` across the pitch away from the
/// nearest defender, kept on the pitch.
fn time_runs(view: &MatchView<'_>, targets: &mut [DVec2; ROSTER], team: usize, c: usize) {
    let space = view.tuning().contract.hooks().anchor_space_m;
    let side = &view.teams()[team];
    let attack_x = side.attack_x;
    let line = offside::second_last_depth(team, view.players(), attack_x) - ONSIDE_MARGIN;
    let keeper = view.keeper(team);
    for (i, p) in view.players().iter().enumerate() {
        if p.team != team || i == c || i == keeper || !p.active() {
            continue;
        }
        let s = view.skills(i).skill(Stage::OFF_BALL_RUN_EXECUTE).max(0.0);
        if s <= 0.0 {
            continue;
        }
        let mut at = targets[i];
        let depth = at.x * attack_x;
        if line.is_finite() && depth > line {
            at.x = (depth + s * (line - depth)) * attack_x;
        }
        if space > 0.0 {
            let nearest = view
                .players()
                .iter()
                .filter(|q| q.team != team && q.active())
                .map(|q| q.pos)
                .min_by(|a, b| (*a - at).length().total_cmp(&(*b - at).length()));
            if let Some(q) = nearest {
                let away = if at.y >= q.y { 1.0 } else { -1.0 };
                at.y += away * space * s;
            }
        }
        targets[i] = view.pitch().clamp(at, 0.5);
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
    targets[i] = view
        .pitch()
        .clamp(DVec2::new(held * attack_x, anchor.y), 0.5);
}

/// Goal-side cover (named mechanism) while team `def` defends against carrier `c`. The
/// covered attacker is the carrier when he is in `def`'s half inside the central channel,
/// otherwise the most advanced attacker there. The nearest active back-line player of
/// `def` who is neither pressing nor keeping goal covers him, on the line from the
/// attacker to the centre of its own goal:
/// - a carrier is tracked `cover_distance` goal-side of him, divided by the covering
///   player's marking factor (a better marker stays tighter), at his pace, so the covering
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
    let depth = |at: DVec2| at.x * side.attack_x + view.pitch().half_length();
    let covered = |p: &crate::player::Player| {
        p.active() && p.pos.y.abs() <= t.cover_channel && depth(p.pos) < view.pitch().half_length()
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
    let me = &players[i];
    let hold = if j == c {
        let mark = contract::factor(
            t.contract.actions.of(contract::ActionKind::Mark).spread(),
            view.skills(i).share(Stage::MARK_EXECUTE),
        );
        reach - t.cover_distance / mark
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
            (me.max_speed() * me.max_speed() / (2.0 * me.max_decel())).max(t.arrive_radius);
        spot += players[c].vel * (braking / me.max_speed());
    }
    targets[i] = view.pitch().clamp(spot, 0.5);
    CoverTrace::Covered {
        attacker: j,
        defender: i,
        distance,
    }
}

pub const DECISION_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Sets every player's target (anchors at each position's stand for the possession phase, stand options from the role menus, the rest-defence guard, the lone forward's line hold, timed runs against the offside line, pressing, goal-side cover, chasing) and the job each target is for (carry, press, recovery run, loose ball, chase, offer, return, follow, hold), and decides the carrier's option, pass, clearance, shot, and restart kick.",
    inputs: "Every player's position, velocity, target, activity, slot, lapse, and effective values through the attribute contract (the carrier's see, choose, execute and pressure stages of passes, chips, crosses, shots, long shots, penalties, dribbles and keeper distribution, his skill gates and flair, the restart taker's corner, free-kick and long-throw stages, and every player's press, intercept, shape, rush, marking and off-ball run values, determination and eccentricity); the carrier's mood, pressure handling and professionalism; both teams' shapes and plans (the stand table, the role menus, the stand-option sizes, the mentality's rest-defence minimum and the guard); the possession phase; each player's familiarity (the familiarity family); the score; the ball; the carrier and how long he has held the ball; the tick; and the engine tuning.",
    outputs: "Every player's target and its job, the carrier's option scores and choice, and the kick of a pass, clearance, shot, or restart. The loop grades each job, draws an urgent job's sprint on its key, and picks the pace level steering arrives at.",
    tuning: &[
        "decision",
        "pace",
        "contract.curve",
        "contract.actions",
        "contract.lapse",
        "contract.hooks",
        "contract.person.mood.risk",
        "contract.person.occasions.professionalism",
        "contract.person.occasions.penalty_pressure",
        "shots.penalty_spread",
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
        Action::Lapse,
        Action::PaceSprint,
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

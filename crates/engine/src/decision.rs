//! The possession decision. The ball carrier scores its options (shoot, pass to each open
//! team-mate, dribble, clear, hold) and takes the highest; the team's pressing instruction
//! sets how many opponents press and from how far; every other player holds its formation
//! anchor, which the team's plan moves. Every agent decides every tick (product-owner choice;
//! `Tuning::decision_interval_ticks`).
//! A sent-off player takes no part in any decision. While the ball is dead, the referee sets
//! every target instead, and a restart kick comes from `restart_pass`.

use crate::math::{self, DVec2};
use crate::plugin::OptionOffsets;
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
pub(crate) enum Choice {
    Shot,
    Pass,
    Dribble,
    Clear,
    Hold,
}

impl Choice {
    pub(crate) fn name(self) -> &'static str {
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

pub(crate) const MAX_PRESSERS: usize = 4;
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

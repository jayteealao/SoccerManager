//! Fatigue and injuries.
//!
//! Energy runs from 1.0 (fresh) down to 0.0. Every tick it drains by a base rate plus an
//! effort term that grows with the square of the player's speed over the average player's
//! top speed, so every sprint costs stamina and a fast runner pays more for his speed; the
//! drain is scaled by stamina, and late in a match by age. Every 50 ticks the modifiers set
//! each player's state deltas: the fatigue modifier lowers his ratings by the fatigue curve,
//! a piecewise-linear lookup that is 1.0 at and above the threshold, read as a delta on the
//! rating curve and weighted per group (physical first). Half-time gives some energy back,
//! scaled by natural fitness.
//!
//! Injuries roll once for the tackled player on every tackle that wins the ball or is a foul,
//! and once per simulated minute for every player on the pitch, scaled by injury resistance
//! and, after a short rest, by congestion.
//! The rolls draw through the stream registry on the rolling player's injury keys, which a
//! test scene may script.

use crate::TICKS_PER_SECOND;
use crate::contract::Skills;
use crate::contract::Stage;
use crate::contract::params::SpeedTuning;
use crate::data::tuning::FatigueTuning;
use crate::modules::modifier::{Effect, Family, Modifier, Neutral};
use crate::modules::{FatigueModule, MatchView, ModuleCard};
use crate::player::Player;
use crate::streams::Action;

/// Ticks between two recomputations of the effective values.
pub const REFRESH_TICKS: u32 = TICKS_PER_SECOND;

/// What caused an injury.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjurySource {
    /// The tackled player, on a tackle that won the ball or was a foul.
    Tackle,
    /// The per-minute roll for every player on the pitch.
    Background,
}

impl InjurySource {
    pub fn code(&self) -> &'static str {
        match self {
            InjurySource::Tackle => "tackle",
            InjurySource::Background => "background",
        }
    }
}

/// The fatigue curve's multiplier at `energy`: 1.0 at and above the threshold, otherwise the
/// straight line between the two curve points around `energy`.
pub fn multiplier(energy: f64, f: &FatigueTuning) -> f64 {
    if energy >= f.threshold {
        return 1.0;
    }
    for pair in f.curve.windows(2) {
        let [[e_hi, m_hi], [e_lo, m_lo]] = [pair[0], pair[1]];
        if energy <= e_hi && energy >= e_lo {
            let span = e_hi - e_lo;
            if span <= 0.0 {
                return m_lo;
            }
            return m_lo + (m_hi - m_lo) * (energy - e_lo) / span;
        }
    }
    f.curve.last().map_or(1.0, |p| p[1])
}

/// The fatigue modifier's delta in rating points on a group of weight `weight` at `energy`:
/// the fatigue curve's multiplier read on the rating curve, `width · ln(multiplier)`. 0 at
/// and above the threshold, with no logarithm taken.
pub fn delta(energy: f64, f: &FatigueTuning, width: f64, weight: f64) -> f64 {
    let m = multiplier(energy, f);
    if m == 1.0 || weight == 0.0 {
        return 0.0;
    }
    width * crate::math::ln(m) * weight
}

/// Energy player `p`, of base values `base`, loses in one tick of `dt` seconds. The effort
/// term reads his speed over the average player's top speed, up to `sprint_cap` times it: a
/// pace-10 player drains as before the contract, and a faster player at full sprint pays
/// for his speed. The endure stage (stamina) scales it by `1.5 − share`: an average player's
/// drain, half of it at the top of the scale. `fade` is the late-match factor of his age.
pub fn drain(
    p: &Player,
    base: Skills<'_>,
    f: &FatigueTuning,
    speed: &SpeedTuning,
    dt: f64,
    fade: f64,
) -> f64 {
    let frac = (p.vel.length() / speed.anchor_ms).min(f.sprint_cap);
    let stamina_scale = 1.5 - base.share(Stage::ENDURE_EXECUTE);
    (f.drain_base_per_s + f.drain_effort_per_s * frac * frac) * dt * stamina_scale * fade
}

/// The chance that one roll injures a player of base values `base`, from a base rate for an
/// average player, scaled by `1.5 − share` of the injury stage (injury resistance).
pub fn injury_chance(base: Skills<'_>, rate: f64) -> f64 {
    (rate * (1.5 - base.share(Stage::INJURY_EXECUTE))).clamp(0.0, 1.0)
}

/// Fatigue version 1: the drain and the injury chance above. The fatigue curve's effect on
/// the effective values is the fatigue modifier, [`FatigueCurveV1`].
pub struct FatigueV1;

impl FatigueModule for FatigueV1 {
    fn drain(&self, view: &MatchView<'_>, i: usize) -> f64 {
        let t = view.tuning();
        let minute = f64::from(view.tick()) / f64::from(crate::rules::clock::TICKS_PER_MINUTE);
        let body = &t.contract.body;
        // Before the late minute the fade is 1 for every age, so his age is not read.
        let fade = if minute > f64::from(body.age.late_from_minute) {
            body.late_fade(view.body(i), minute)
        } else {
            1.0
        };
        drain(
            view.player(i),
            view.base_skills(i),
            view.fatigue(),
            &t.contract.speed,
            t.dt,
            fade,
        )
    }

    fn injury_chance(&self, view: &MatchView<'_>, i: usize, source: InjurySource) -> f64 {
        let t = view.tuning();
        let rate = match source {
            InjurySource::Tackle => t.injury_per_tackle,
            InjurySource::Background => t.injury_per_minute,
        };
        let congestion = t
            .contract
            .body
            .congestion(view.condition(i).rest_days, view.body(i));
        injury_chance(view.base_skills(i), rate * congestion)
    }
}

pub const FATIGUE_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Drains each player's energy by speed and stamina, faster late in a match for an older player, and sets each injury chance, higher after a short rest.",
    inputs: "Each player's velocity, his endure and injury stages through the attribute contract (stamina and injury resistance), his age and days of rest, the top-speed map, the fatigue tuning, and the engine tuning.",
    outputs: "The energy drained per tick and the injury chance of a roll.",
    tuning: &[
        "fatigue.drain_base_per_s",
        "fatigue.drain_effort_per_s",
        "fatigue.sprint_cap",
        "contract.speed",
        "contract.body.age",
        "contract.body.rest",
        "injury_per_minute",
        "injury_per_tackle",
        "dt",
    ],
    calibration: "none: no fatigue or injury band in realism-bands.json",
    keys: &[Action::InjuryMinute, Action::InjuryTackle],
};

/// Fatigue switched off: nobody tires and nobody is injured. The loop still takes every
/// injury draw on its key. Energy stays full, so the fatigue modifier has no effect either.
pub struct FatigueOff;

impl FatigueModule for FatigueOff {
    fn drain(&self, _: &MatchView<'_>, _: usize) -> f64 {
        0.0
    }

    fn injury_chance(&self, _: &MatchView<'_>, _: usize, _: InjurySource) -> f64 {
        0.0
    }
}

pub const FATIGUE_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "Fatigue switched off: energy never drains and no roll injures.",
    inputs: "Nothing.",
    outputs: "A drain of 0 and an injury chance of 0.",
    tuning: &["none"],
    calibration: "none: off version, no fatigue",
    keys: &[Action::InjuryMinute, Action::InjuryTackle],
};

/// The fatigue modifier (body family): the fatigue curve's multiplier at the player's
/// energy, as a delta on the rating curve, on each group by its weight: physical first,
/// technical and goalkeeping less, mental not at all.
pub struct FatigueCurveV1;

impl Modifier for FatigueCurveV1 {
    fn family(&self) -> Family {
        Family::Body
    }

    fn effect(&self, view: &MatchView<'_>, i: usize) -> Effect {
        let f = view.fatigue();
        let width = view.tuning().contract.curve.width;
        let energy = view.player(i).energy;
        Effect {
            groups: f
                .group_weights
                .by_group()
                .map(|w| delta(energy, f, width, w)),
        }
    }
}

pub const FATIGUE_CURVE_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Lowers a tired player's ratings by the fatigue curve at his energy (body family): physical first, technical and goalkeeping less.",
    inputs: "Each player's energy, the fatigue tuning, and the curve width.",
    outputs: "A delta on the technical, physical and goalkeeping groups, each the curve's multiplier read on the rating curve times the group's weight.",
    tuning: &[
        "fatigue.threshold",
        "fatigue.curve",
        "fatigue.group_weights",
    ],
    calibration: "none: no fatigue band in realism-bands.json",
    keys: &[],
};

/// The fatigue modifier switched off: tired or not, the effective values stay at base.
pub const FATIGUE_CURVE_OFF: Neutral = Neutral(Family::Body);

pub const FATIGUE_CURVE_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "The fatigue modifier switched off (body family): no effect at any energy.",
    inputs: "Nothing.",
    outputs: "A delta of 0 on every attribute group.",
    tuning: &["none"],
    calibration: "none: off version, no fatigue effect",
    keys: &[],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_content;
    use crate::player::test_support::{flat, flat_player};
    use crate::tuning::Tuning;

    fn fatigue() -> FatigueTuning {
        shipped_content().tuning.fatigue
    }

    #[test]
    fn the_curve_is_exact_at_each_point_and_linear_between() {
        let f = fatigue();
        for [e, m] in f.curve.iter().copied().map(|p| [p[0], p[1]]) {
            let expect = if e >= f.threshold { 1.0 } else { m };
            assert!((multiplier(e, &f) - expect).abs() < 1e-12, "at {e}");
        }
        // Halfway between 0.5 (0.92) and 0.3 (0.82).
        assert!((multiplier(0.4, &f) - 0.87).abs() < 1e-12);
    }

    #[test]
    fn above_the_threshold_the_multiplier_is_one() {
        let f = fatigue();
        for e in [1.0, 0.95, 0.8, f.threshold] {
            assert_eq!(multiplier(e, &f), 1.0);
        }
        assert!(multiplier(f.threshold - 0.01, &f) < 1.0);
    }

    #[test]
    fn a_player_standing_still_drains_only_the_base_rate() {
        let f = fatigue();
        let t = Tuning::default();
        let speed = &t.contract.speed;
        let mut p = flat_player(0, 100, &t);
        let (d, stages) = flat(100, &t);
        let base = Skills::new(&stages, &d);
        let still = drain(&p, base, &f, speed, t.dt, 1.0);
        assert!((still - f.drain_base_per_s * t.dt * 1.0).abs() < 1e-15);
        p.vel = crate::math::DVec2::new(d.max_speed, 0.0);
        let running = drain(&p, base, &f, speed, t.dt, 1.0);
        assert!(
            (running - (f.drain_base_per_s + f.drain_effort_per_s) * t.dt).abs() < 1e-15,
            "{running}"
        );
    }

    /// At rating 10 the drain and the injury chance read exactly what they read before the
    /// contract at the old value 50.
    #[test]
    fn a_rating_ten_player_drains_and_is_injured_as_before() {
        let f = fatigue();
        let t = Tuning::default();
        let mut p = flat_player(0, 100, &t);
        p.vel = crate::math::DVec2::new(3.0, 0.0);
        let (d, stages) = flat(100, &t);
        let base = Skills::new(&stages, &d);
        let frac: f64 = 3.0 / d.max_speed;
        let before = (f.drain_base_per_s + f.drain_effort_per_s * frac * frac) * t.dt * (1.5 - 0.5);
        let now = drain(&p, base, &f, &t.contract.speed, t.dt, 1.0);
        assert!((now - before).abs() < 1e-12);
        assert!((injury_chance(base, 0.004) - 0.004 * (1.5 - 0.5)).abs() < 1e-12);
    }

    /// Every sprint costs stamina: at full sprint the effort term reads speed over the
    /// average top speed, up to the cap, so a faster runner drains more than a slower one.
    #[test]
    fn a_faster_runner_at_full_sprint_drains_more() {
        let f = fatigue();
        let t = Tuning::default();
        let speed = &t.contract.speed;
        let (d, stages) = flat(100, &t);
        let base = Skills::new(&stages, &d);
        let effort = |r: f64| f.drain_base_per_s + f.drain_effort_per_s * r * r;
        let mut p = flat_player(0, 100, &t);
        p.vel = crate::math::DVec2::new(speed.top_speed(10.0), 0.0);
        let average = drain(&p, base, &f, speed, t.dt, 1.0);
        p.vel = crate::math::DVec2::new(speed.top_speed(4.0), 0.0);
        let slow = drain(&p, base, &f, speed, t.dt, 1.0);
        let slow_ratio = speed.top_speed(4.0) / speed.anchor_ms;
        assert!(slow < average, "a slower runner at full sprint drains less");
        assert!((slow / average - effort(slow_ratio) / effort(1.0)).abs() < 1e-12);
        p.vel = crate::math::DVec2::new(speed.top_speed(20.0), 0.0);
        let fast = drain(&p, base, &f, speed, t.dt, 1.0);
        let ratio = (speed.top_speed(20.0) / speed.anchor_ms).min(f.sprint_cap);
        assert!((fast / average - effort(ratio) / effort(1.0)).abs() < 1e-12);
        assert!(fast >= average);
        // A cap above 1 lets a runner faster than the average pay more than him.
        let mut wide = f.clone();
        wide.sprint_cap = 1.5;
        assert!(drain(&p, base, &wide, speed, t.dt, 1.0) > average);
        // Faster than the cap counts as the cap.
        p.vel = crate::math::DVec2::new(speed.anchor_ms * f.sprint_cap, 0.0);
        let capped = drain(&p, base, &f, speed, t.dt, 1.0);
        p.vel = crate::math::DVec2::new(speed.anchor_ms * f.sprint_cap * 2.0, 0.0);
        assert_eq!(drain(&p, base, &f, speed, t.dt, 1.0), capped);
        // The late fade of age multiplies it.
        assert!((drain(&p, base, &f, speed, t.dt, 1.2) - fast * 1.2).abs() < 1e-15);
    }

    /// The fatigue curve reads as a delta on the rating curve: `width · ln(multiplier)`,
    /// times the group weight, and nothing at or above the threshold.
    #[test]
    fn the_fatigue_delta_is_the_curve_read_on_the_rating_curve() {
        let f = fatigue();
        let width = Tuning::default().contract.curve.width;
        assert_eq!(delta(0.9, &f, width, 1.0), 0.0);
        let m = multiplier(0.4, &f);
        assert!((delta(0.4, &f, width, 1.0) - width * crate::math::ln(m)).abs() < 1e-15);
        assert!((delta(0.4, &f, width, 0.5) - 0.5 * width * crate::math::ln(m)).abs() < 1e-15);
        assert_eq!(delta(0.0, &f, width, 0.0), 0.0);
        // On the rating curve a delta d is the factor e^(d / width): the old multiplier.
        let c = &Tuning::default().contract.curve;
        let r: f64 = 12.0;
        let f_r = crate::contract::curve::f(r, c);
        let f_d = crate::contract::curve::f(r + delta(0.4, &f, width, 1.0), c);
        assert!((f_d / f_r - m).abs() < 1e-12);
    }
}

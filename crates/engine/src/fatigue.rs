//! Fatigue and injuries.
//!
//! Energy runs from 1.0 (fresh) down to 0.0. Every tick it drains by a base rate plus an
//! effort term that grows with the square of the player's speed fraction, scaled by stamina.
//! Every 50 ticks each player's effective values are recomputed from the unfatigued base
//! through the modifiers: the fatigue modifier multiplies pace (maximum speed and
//! acceleration) and the decision values (passing, finishing, decisions, composure) by the
//! fatigue curve, a piecewise-linear lookup that is 1.0 at and above the threshold. Half-time gives some energy back, scaled by natural
//! fitness.
//!
//! Injuries roll once for the tackled player on every tackle that wins the ball or is a foul,
//! and once per simulated minute for every player on the pitch, scaled by injury resistance.
//! The rolls draw through the stream registry on the rolling player's injury keys, which a
//! test scene may script.

use crate::TICKS_PER_SECOND;
use crate::contract::Skills;
use crate::contract::Stage;
use crate::data::tuning::FatigueTuning;
use crate::modules::modifier::{Effect, Family, Modifier, Neutral};
use crate::modules::{FatigueModule, MatchView, ModuleCard};
use crate::player::{Derived, Player};
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

/// The effective values of `base` at `energy`: the fatigue curve's multiplier on every value
/// the fatigue modifier scales.
pub fn effective(base: &Derived, energy: f64, f: &FatigueTuning) -> Derived {
    base.scaled([multiplier(energy, f); 6])
}

/// Energy player `p`, of base values `base`, loses in one tick of `dt` seconds. The endure
/// stage (stamina) scales it by `1.5 − share`: an average player's drain, half of it at the
/// top of the scale.
pub fn drain(p: &Player, base: Skills<'_>, f: &FatigueTuning, dt: f64) -> f64 {
    let top = base.derived.max_speed.max(1e-6);
    let frac = (p.vel.length() / top).min(1.0);
    let stamina_scale = 1.5 - base.share(Stage::ENDURE_EXECUTE);
    (f.drain_base_per_s + f.drain_effort_per_s * frac * frac) * dt * stamina_scale
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
        drain(
            view.player(i),
            view.base_skills(i),
            view.fatigue(),
            view.tuning().dt,
        )
    }

    fn injury_chance(&self, view: &MatchView<'_>, i: usize, source: InjurySource) -> f64 {
        let t = view.tuning();
        let rate = match source {
            InjurySource::Tackle => t.injury_per_tackle,
            InjurySource::Background => t.injury_per_minute,
        };
        injury_chance(view.base_skills(i), rate)
    }
}

pub const FATIGUE_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Drains each player's energy by speed and stamina and sets each injury chance.",
    inputs: "Each player's velocity, base top speed, and endure and injury stages through the attribute contract (stamina and injury resistance), the fatigue tuning, and the engine tuning.",
    outputs: "The energy drained per tick and the injury chance of a roll.",
    tuning: &[
        "fatigue.drain_base_per_s",
        "fatigue.drain_effort_per_s",
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
/// energy, on every value the curve scales.
pub struct FatigueCurveV1;

impl Modifier for FatigueCurveV1 {
    fn family(&self) -> Family {
        Family::Body
    }

    fn effect(&self, view: &MatchView<'_>, i: usize) -> Effect {
        Effect::all(multiplier(view.player(i).energy, view.fatigue()))
    }
}

pub const FATIGUE_CURVE_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Scales pace and the decision values by the fatigue curve at the player's energy (body family).",
    inputs: "Each player's energy and the fatigue tuning.",
    outputs: "One factor on max speed, max acceleration, and the passing, finishing, decision, and composure stage groups.",
    tuning: &["fatigue.threshold", "fatigue.curve"],
    calibration: "none: no fatigue band in realism-bands.json",
    keys: &[],
};

/// The fatigue modifier switched off: tired or not, the effective values stay at base.
pub const FATIGUE_CURVE_OFF: Neutral = Neutral(Family::Body);

pub const FATIGUE_CURVE_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "The fatigue modifier switched off (body family): no effect at any energy.",
    inputs: "Nothing.",
    outputs: "A factor of 1.0 on every effective value.",
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
        let mut p = flat_player(0, 100, &t);
        let (d, stages) = flat(100, &t);
        let base = Skills::new(&stages, &d);
        let still = drain(&p, base, &f, t.dt);
        assert!((still - f.drain_base_per_s * t.dt * 1.0).abs() < 1e-15);
        p.vel = crate::math::DVec2::new(d.max_speed, 0.0);
        let running = drain(&p, base, &f, t.dt);
        assert!(
            (running - (f.drain_base_per_s + f.drain_effort_per_s) * t.dt).abs() < 1e-15,
            "{running}"
        );
    }

    /// At rating 10 the drain, the injury chance, and the fatigue scaling read exactly what
    /// they read before the contract at the old value 50.
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
        assert!((drain(&p, base, &f, t.dt) - before).abs() < 1e-12);
        assert!((injury_chance(base, 0.004) - 0.004 * (1.5 - 0.5)).abs() < 1e-12);
        let tired_values = effective(&d, 0.4, &f);
        let tired = Skills::new(&stages, &tired_values);
        let m = multiplier(0.4, &f);
        let s = crate::contract::Stage::PASS_EXECUTE;
        assert!((tired.share(s) - 50.0 * m / 100.0).abs() < 1e-12);
        assert!((tired_values.max_speed - d.max_speed * m).abs() < 1e-12);
        assert_eq!(
            tired.share(crate::contract::Stage::TACKLE_EXECUTE),
            base.share(crate::contract::Stage::TACKLE_EXECUTE),
            "the curve leaves tackling alone, as before"
        );
    }
}

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
use crate::data::tuning::FatigueTuning;
use crate::modules::modifier::{Effect, Family, Modifier, Neutral};
use crate::modules::{FatigueModule, MatchView, ModuleCard};
use crate::player::{Derived, Player};
use crate::rules::Phase;
use crate::rules::clock::TICKS_PER_MINUTE;
use crate::sim::Simulation;
use crate::streams::{Action, Key};
use crate::trace::Point;
use serde_json::json;

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

/// The effective values of `base` at `energy`.
pub fn effective(base: &Derived, energy: f64, f: &FatigueTuning) -> Derived {
    let m = multiplier(energy, f);
    Derived {
        max_speed: base.max_speed * m,
        max_accel: base.max_accel * m,
        passing: base.passing * m,
        finishing: base.finishing * m,
        decisions: base.decisions * m,
        composure: base.composure * m,
        ..*base
    }
}

/// Energy one player loses in one tick of `dt` seconds.
pub fn drain(p: &Player, f: &FatigueTuning, dt: f64) -> f64 {
    let top = p.base.max_speed.max(1e-6);
    let frac = (p.vel.length() / top).min(1.0);
    let stamina_scale = 1.5 - p.base.stamina;
    (f.drain_base_per_s + f.drain_effort_per_s * frac * frac) * dt * stamina_scale
}

/// The chance that one roll injures `p`, from a base rate for an average player.
pub fn injury_chance(p: &Player, rate: f64) -> f64 {
    (rate * (1.5 - p.base.injury_resistance)).clamp(0.0, 1.0)
}

/// Fatigue version 1: the drain and the injury chance above. The fatigue curve's effect on
/// the effective values is the fatigue modifier, [`FatigueCurveV1`].
pub struct FatigueV1;

impl FatigueModule for FatigueV1 {
    fn drain(&self, view: &MatchView<'_>, i: usize) -> f64 {
        drain(view.player(i), view.fatigue(), view.tuning().dt)
    }

    fn injury_chance(&self, view: &MatchView<'_>, i: usize, source: InjurySource) -> f64 {
        let t = view.tuning();
        let rate = match source {
            InjurySource::Tackle => t.injury_per_tackle,
            InjurySource::Background => t.injury_per_minute,
        };
        injury_chance(view.player(i), rate)
    }
}

pub const FATIGUE_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Drains each player's energy by speed and stamina and sets each injury chance.",
    inputs: "Each player's velocity and base values, the fatigue tuning, and the engine tuning.",
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
    outputs: "One factor on max speed, max acceleration, passing, finishing, decisions, and composure.",
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

impl Simulation {
    /// One tick of fatigue: every player on the pitch drains; every 50 ticks the effective
    /// values follow the energy; once per simulated minute of open play each player on the
    /// pitch rolls for an injury.
    pub(crate) fn fatigue_tick(&mut self) {
        let fatigue = self.config.modules.fatigue;
        for i in 0..self.players.len() {
            if !self.players[i].active() {
                continue;
            }
            let d = fatigue.drain(&self.view(), i);
            let p = &mut self.players[i];
            p.energy = (p.energy - d).max(0.0);
        }
        let now = self.tick + 1;
        if now.is_multiple_of(REFRESH_TICKS) {
            self.refresh_effective();
        }
        // No injury roll during the shoot-out: an injury would stop a kick for a dropped ball.
        if now.is_multiple_of(TICKS_PER_MINUTE)
            && self.referee.phase == Phase::Live
            && self.referee.shootout.is_none()
        {
            for i in 0..self.players.len() {
                if !self.players[i].active() {
                    continue;
                }
                let p = self.players[i];
                let chance = fatigue.injury_chance(&self.view(), i, InjurySource::Background);
                let injured = self
                    .streams
                    .tested(Key::player(Action::InjuryMinute, &p), &[chance])
                    < chance;
                self.trace_injury_roll(i, InjurySource::Background, chance, injured);
                if injured {
                    self.injure(i, InjurySource::Background);
                }
            }
        }
    }

    /// Recomputes every player's effective values from its base through the modifiers.
    pub(crate) fn refresh_effective(&mut self) {
        let modifiers = self.config.modules.modifiers;
        for i in 0..self.players.len() {
            let derived = modifiers.effective(&self.view(), i);
            self.players[i].derived = derived;
        }
    }

    /// Half-time: every player on the pitch recovers some energy, scaled by natural fitness.
    pub(crate) fn half_time_recovery(&mut self) {
        let gain = self.config.fatigue.half_time_recovery;
        for p in self.players.iter_mut().filter(|p| p.active()) {
            p.energy = (p.energy + gain * (0.5 + p.base.natural_fitness)).min(1.0);
        }
        self.refresh_effective();
    }

    /// The tackled player `c` rolls for an injury after a tackle that won the ball or was a
    /// foul.
    pub(crate) fn tackle_injury_roll(&mut self, c: usize) {
        if !self.players[c].active() {
            return;
        }
        let p = self.players[c];
        let chance =
            self.config
                .modules
                .fatigue
                .injury_chance(&self.view(), c, InjurySource::Tackle);
        let injured = self
            .streams
            .tested(Key::player(Action::InjuryTackle, &p), &[chance])
            < chance;
        self.trace_injury_roll(c, InjurySource::Tackle, chance, injured);
        if injured {
            self.injure(c, InjurySource::Tackle);
        }
    }

    /// Records one injury roll of player `i`.
    fn trace_injury_roll(&mut self, i: usize, source: InjurySource, chance: f64, injured: bool) {
        if self.trace_on() {
            self.trace_point(
                Point::Injury,
                json!({
                    "roll": true,
                    "player": i,
                    "source": source.code(),
                    "chance": chance,
                    "injured": injured,
                }),
            );
        }
    }

    /// The mean energy of the players on the pitch, as a percentage with two decimals.
    pub fn fatigue_mean_pct(&self) -> f64 {
        let on: Vec<f64> = self
            .players
            .iter()
            .filter(|p| p.active())
            .map(|p| p.energy)
            .collect();
        if on.is_empty() {
            return 0.0;
        }
        let mean = on.iter().sum::<f64>() / on.len() as f64;
        (mean * 10_000.0).round() / 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_content;
    use crate::player::test_support::flat_player;
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
        let mut p = flat_player(0, 50, &t);
        let still = drain(&p, &f, t.dt);
        assert!((still - f.drain_base_per_s * t.dt * 1.0).abs() < 1e-15);
        p.vel = crate::math::DVec2::new(p.base.max_speed, 0.0);
        let running = drain(&p, &f, t.dt);
        assert!(
            (running - (f.drain_base_per_s + f.drain_effort_per_s) * t.dt).abs() < 1e-15,
            "{running}"
        );
    }
}

//! The fatigue pass of the central loop: energy drain, the effective values, and the injury rolls.

use serde_json::json;

use crate::fatigue::{InjurySource, REFRESH_TICKS};
use crate::modules::ROSTER;
use crate::rules::Phase;
use crate::rules::clock::TICKS_PER_MINUTE;
use crate::sim::Simulation;
use crate::streams::{Action, Key};
use crate::trace::Point;

impl Simulation {
    /// One tick of fatigue: every player on the pitch drains; every 50 ticks the effective
    /// values follow the energy; once per simulated minute of open play each player on the
    /// pitch rolls for an injury.
    pub(crate) fn fatigue_tick(&mut self) {
        let fatigue = self.config.modules.fatigue;
        // A player's drain reads only that player, so every drain is taken from the old
        // state first, then applied.
        let mut drains = [0.0; ROSTER];
        fatigue.drains(&self.view(), &mut drains);
        for (p, d) in self.players.iter_mut().zip(drains) {
            if p.active() {
                p.energy = (p.energy - d).max(0.0);
            }
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
                let p = crate::streams::PlayerKey::of(&self.players[i]);
                let chance = fatigue.injury_chance(&self.view(), i, InjurySource::Background);
                let injured = self
                    .streams
                    .tested(Key::player(Action::InjuryMinute, p), &[chance])
                    < chance;
                self.trace_injury_roll(i, InjurySource::Background, chance, injured);
                if injured {
                    self.injure(i, InjurySource::Background);
                }
            }
            self.lapse_rolls();
        }
    }

    /// While a team has the ball, each active outfield player of the other side rolls once
    /// a minute for a concentration lapse; a lapse holds him where he stands for the tuned
    /// number of ticks.
    fn lapse_rolls(&mut self) {
        let Some(c) = self.carrier else {
            return;
        };
        let def = 1 - self.players[c].team;
        let keeper = self.view().keeper(def);
        let decision = self.config.modules.decision;
        let ticks = self.config.tuning.contract.lapse.ticks;
        for i in 0..self.players.len() {
            let player = &self.players[i];
            if player.team != def || i == keeper || !player.active() {
                continue;
            }
            let p = crate::streams::PlayerKey::of(player);
            let chance = decision.lapse_chance(&self.view(), i);
            let lapsed = self
                .streams
                .tested(Key::player(Action::Lapse, p), &[chance])
                < chance;
            if self.trace_on() {
                self.trace_point(
                    Point::Cover,
                    json!({"lapse": i, "chance": chance, "lapsed": lapsed}),
                );
            }
            if lapsed {
                self.players[i].lapse_until = self.tick + ticks;
            }
        }
    }

    /// Takes every player's state deltas from the modifiers; a player whose deltas changed
    /// has his effective values derived again ([`Simulation::set_deltas`]). Deltas move in
    /// tenths, so most refreshes change nobody.
    pub(crate) fn refresh_effective(&mut self) {
        let modifiers = self.config.modules.modifiers;
        for i in 0..self.players.len() {
            let deltas = modifiers.effective(&self.view(), i);
            if deltas != self.players[i].deltas {
                self.set_deltas(i, deltas);
            }
        }
    }

    /// Half-time: every player on the pitch recovers some energy, scaled by `0.5 + share` of
    /// his recover stage (natural fitness): the average player's gain at rating 10.
    pub(crate) fn half_time_recovery(&mut self) {
        let gain = self.config.fatigue.half_time_recovery;
        for i in 0..self.players.len() {
            if !self.players[i].active() {
                continue;
            }
            let fitness = self
                .base_skills(i)
                .share(crate::contract::Stage::RECOVER_EXECUTE);
            let p = &mut self.players[i];
            p.energy = (p.energy + gain * (0.5 + fitness)).min(1.0);
        }
        self.refresh_effective();
    }

    /// The tackled player `c` rolls for an injury after a tackle that won the ball or was a
    /// foul.
    pub(crate) fn tackle_injury_roll(&mut self, c: usize) {
        if !self.players[c].active() {
            return;
        }
        let p = crate::streams::PlayerKey::of(&self.players[c]);
        let chance =
            self.config
                .modules
                .fatigue
                .injury_chance(&self.view(), c, InjurySource::Tackle);
        let injured = self
            .streams
            .tested(Key::player(Action::InjuryTackle, p), &[chance])
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

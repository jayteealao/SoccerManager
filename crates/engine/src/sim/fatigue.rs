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
            // A new period of form falls on a refresh tick (a period is whole minutes), and is
            // drawn before the refresh so one rebuild covers both.
            let period = self.config.tuning.contract.consistency.period_minutes * TICKS_PER_MINUTE;
            let changed = if now.is_multiple_of(period)
                && self.referee.shootout.is_none()
                && self.referee.phase != Phase::FullTime
            {
                self.draw_form_period()
            } else {
                0
            };
            self.tally_ticks(REFRESH_TICKS);
            self.refresh_marked(changed);
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
    /// has his effective values derived again ([`Simulation::set_state`]). Deltas move in
    /// tenths, so most refreshes change nobody.
    pub(crate) fn refresh_effective(&mut self) {
        self.refresh_marked(0);
    }

    /// [`Simulation::refresh_effective`], also rebuilding each player whose bit is set in
    /// `changed` (his form moved), once.
    pub(crate) fn refresh_marked(&mut self, changed: u32) {
        let modifiers = self.config.modules.modifiers;
        for i in 0..self.players.len() {
            let deltas = modifiers.effective(&self.view(), i);
            if deltas != self.players[i].deltas || changed & (1 << i) != 0 {
                self.set_state(i, deltas, self.players[i].form);
            }
        }
    }

    /// Draws both parts of every player's form at kick-off. Returns the roster bits of the
    /// players whose form moved; the caller rebuilds them.
    pub(crate) fn draw_form_kick_off(&mut self) -> u32 {
        self.draw_form_parts(true) | self.draw_form_parts(false)
    }

    /// Draws a new period part of every active player's form. Returns the roster bits of the
    /// players whose form moved.
    fn draw_form_period(&mut self) -> u32 {
        self.draw_form_parts(false)
    }

    /// Draws one part of the form of every active player, the match part (`match_part`) or
    /// the period part: per side, each player's draw is sized by his consistency and the
    /// draws are centred, so the side's mean moves by nothing. A part whose maximum is 0
    /// draws nothing. Sets each player's form and returns the roster bits of those whose form
    /// moved.
    fn draw_form_parts(&mut self, match_part: bool) -> u32 {
        use crate::contract::consistency::{centre, draw, spread_of, to_tenths};
        let c = self.config.tuning.contract.consistency;
        let (max, action) = if match_part {
            (c.match_max, Action::FormMatch)
        } else {
            (c.period_max, Action::FormPeriod)
        };
        if max == 0.0 {
            return 0;
        }
        let bound = c.bound_tenths();
        let consistency = self.consistency_index();
        let mut changed = 0u32;
        for team in 0..2 {
            let mut who = [0usize; ROSTER];
            let mut parts = [0.0f64; ROSTER];
            let mut n = 0;
            for i in 0..self.players.len() {
                let p = &self.players[i];
                if p.team != team || !p.active() {
                    continue;
                }
                let size = spread_of(
                    &self.teams[team].squad[p.squad].attributes,
                    consistency,
                    max,
                );
                let key = crate::streams::PlayerKey::of(p);
                let u1 = self.streams.draw(Key::player(action, key));
                let u2 = self.streams.draw(Key::player(action, key));
                who[n] = i;
                parts[n] = size * draw(u1, u2);
                n += 1;
            }
            if self.trace_on() {
                self.trace_point(
                    Point::Form,
                    json!({
                        "part": if match_part { "match" } else { "period" },
                        "team": team,
                        "players": n,
                    }),
                );
            }
            centre(&mut parts[..n]);
            for (&i, &x) in who[..n].iter().zip(&parts[..n]) {
                let part = i16::from(to_tenths(x, bound));
                let p = &mut self.players[i];
                let old_period = i16::from(p.form) - i16::from(p.form_match);
                let (form_match, period) = if match_part {
                    (part, old_period)
                } else {
                    (i16::from(p.form_match), part)
                };
                // Inside ±bound, at most 60 tenths.
                p.form_match = form_match as i8;
                let form = (form_match + period).clamp(-i16::from(bound), i16::from(bound));
                // Inside ±bound, at most 60 tenths, after the clamp.
                let form = form as i8;
                if form != p.form {
                    p.form = form;
                    changed |= 1 << i;
                }
            }
        }
        changed
    }

    /// A substitute who comes on at roster index `i` draws the match part of his form; his
    /// period part is 0 until the next period. He is rebuilt at once.
    pub(crate) fn draw_form_on_entry(&mut self, i: usize) {
        use crate::contract::consistency::{draw, spread_of, to_tenths};
        let c = self.config.tuning.contract.consistency;
        let p = self.players[i];
        let (mut form_match, mut form) = (0, 0);
        if c.match_max != 0.0 {
            let size = spread_of(
                &self.teams[p.team].squad[p.squad].attributes,
                self.consistency_index(),
                c.match_max,
            );
            let key = crate::streams::PlayerKey::of(&p);
            let u1 = self.streams.draw(Key::player(Action::FormMatch, key));
            let u2 = self.streams.draw(Key::player(Action::FormMatch, key));
            form_match = to_tenths(size * draw(u1, u2), c.bound_tenths());
            form = form_match;
        }
        self.players[i].form_match = form_match;
        self.set_state(i, p.deltas, form);
        self.tally_played(i);
        if self.trace_on() {
            self.trace_point(
                Point::Form,
                json!({"part": "match", "player": i, "form": form}),
            );
        }
    }

    /// The schema index of consistency, which every validated schema holds.
    fn consistency_index(&self) -> usize {
        self.config.attributes.required_indices()[3]
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

//! Mood: how each player feels in the match, from −100 (a full low) to 100 (a full high),
//! starting at his kick-off mood: his recent form and his progress toward his season goal,
//! weighted by his ambition, as the launcher's season record gives them in his condition
//! ([`kick_off_mood`]); 0 without them. His own events and his side's goals move it at once,
//! by the impulse of the event times his swing; at each refresh it drifts toward a target the
//! score sets, which is also how an impulse fades. Temperament and pressure handling set the
//! swing; pressure handling also sets how much a late close game weighs on him. Each side
//! keeps a bench mood that takes the side's events and the drift, and a substitute enters at
//! it times his swing.
//!
//! This module holds the match state of mood; the rules it applies are pure functions in
//! [`crate::contract::mood`], re-exported here.
//!
//! Mood draws nothing and writes no event and no trace point: it moves only through play.
//! With no person block in the tuning every impulse and the drift are 0, so mood stays at the
//! kick-off mood; with no season block, or no season inputs, that is 0.

pub use crate::contract::mood::{
    CONFIDENT_FROM, FULL, LIFTED_FROM, LOW_ABOVE, MoodEvent, STEADY_ABOVE, drift_target,
    kick_off_mood, mood_word, pull, swing,
};

use crate::contract::PersonalityScales;
use crate::contract::mood::toward;
use crate::contract::params::MoodTuning;
use crate::fatigue::REFRESH_TICKS;
use crate::rules::clock::TICKS_PER_MINUTE;
use crate::sim::Simulation;

impl Simulation {
    /// The mood tuning of the match.
    fn mood_tuning(&self) -> MoodTuning {
        self.config.tuning.contract.person().mood
    }

    /// Player `i`'s personality scales, from his squad entry.
    pub fn personality(&self, i: usize) -> PersonalityScales {
        let p = &self.players[i];
        self.teams[p.team].squad[p.squad].personality
    }

    /// Player `i` takes the impulse of `event`, times his swing.
    pub(crate) fn mood_event(&mut self, i: usize, event: MoodEvent) {
        let m = self.mood_tuning();
        let impulse = event.impulse(&m);
        if impulse == 0.0 {
            return;
        }
        let swing = swing(&self.personality(i), &m);
        let p = &mut self.players[i];
        p.mood = (p.mood + impulse * swing).clamp(-FULL, FULL);
    }

    /// Every player of `team` on the pitch takes the impulse of `event`, times his swing; the
    /// side's bench mood takes it in full.
    pub(crate) fn team_mood_event(&mut self, team: usize, event: MoodEvent) {
        let m = self.mood_tuning();
        let impulse = event.impulse(&m);
        if impulse == 0.0 {
            return;
        }
        for i in 0..self.players.len() {
            if self.players[i].team == team && self.players[i].active() {
                self.mood_event(i, event);
            }
        }
        self.bench_mood[team] = (self.bench_mood[team] + impulse).clamp(-FULL, FULL);
    }

    /// One refresh of drift: every player on the pitch, and each side's bench mood, moves
    /// toward its target by `drift_per_minute` spread over the minute's refreshes.
    pub(crate) fn drift_moods(&mut self) {
        let m = self.mood_tuning();
        if m.drift_per_minute == 0.0 {
            return;
        }
        let step = m.drift_per_minute * f64::from(REFRESH_TICKS) / f64::from(TICKS_PER_MINUTE);
        let goals = self.summary.goals;
        let minute = self.tick / TICKS_PER_MINUTE;
        for i in 0..self.players.len() {
            if !self.players[i].active() {
                continue;
            }
            let team = self.players[i].team;
            let pressure = self.personality(i).pressure_handling;
            let target = drift_target(goals, team, minute, pressure, &m);
            let p = &mut self.players[i];
            p.mood = toward(p.mood, target, step);
        }
        for team in 0..2 {
            let target = drift_target(goals, team, minute, 0.0, &m);
            self.bench_mood[team] = toward(self.bench_mood[team], target, step);
        }
    }

    /// A substitute who comes on at roster index `i` starts at his kick-off mood plus his
    /// side's bench mood times his swing.
    pub(crate) fn enter_mood(&mut self, i: usize) {
        let m = self.mood_tuning();
        let (team, squad) = (self.players[i].team, self.players[i].squad);
        let swing = swing(&self.personality(i), &m);
        let kick_off = self.teams[team].squad[squad].kick_off_mood;
        self.players[i].mood = (kick_off + self.bench_mood[team] * swing).clamp(-FULL, FULL);
    }

    /// Player `i`'s mood, −100 to 100.
    pub fn mood(&self, i: usize) -> f64 {
        self.players[i].mood
    }

    /// The mood of `team`: the mean of its players' moods on the pitch, each weighted by his
    /// pull (leadership). 0 when nobody is on the pitch.
    pub fn team_mood(&self, team: usize) -> f64 {
        let m = self.mood_tuning();
        let (mut sum, mut weight) = (0.0, 0.0);
        for i in 0..self.players.len() {
            let p = &self.players[i];
            if p.team != team || !p.active() {
                continue;
            }
            let w = pull(&self.personality(i), &m);
            sum += w * p.mood;
            weight += w;
        }
        if weight > 0.0 { sum / weight } else { 0.0 }
    }

    /// `team`'s bench mood, −100 to 100.
    pub fn bench_mood(&self, team: usize) -> f64 {
        self.bench_mood[team]
    }
}

//! Mood: how each player feels in the match, from −100 (a full low) to 100 (a full high),
//! starting at his kick-off mood: his recent form and his progress toward his season goal,
//! weighted by his ambition, as the launcher's season record gives them in his condition
//! ([`kick_off_mood`]); 0 without them. His own events and his side's goals move it at once, by the impulse of the
//! event times his swing; at each refresh it drifts toward a target the score sets, which is
//! also how an impulse fades. Temperament and pressure handling set the swing; pressure
//! handling also sets how much a late close game weighs on him. Each side keeps a bench mood
//! that takes the side's events and the drift, and a substitute enters at it times his swing.
//!
//! Mood draws nothing and writes no event and no trace point: it moves only through play.
//! With no person block in the tuning every impulse and the drift are 0, so mood stays at the
//! kick-off mood; with no season block, or no season inputs, that is 0.

use crate::contract::PersonalityScales;
use crate::contract::params::{MoodTuning, SeasonTuning};
use crate::data::team::Condition;
use crate::fatigue::REFRESH_TICKS;
use crate::rules::clock::TICKS_PER_MINUTE;
use crate::sim::Simulation;

/// The most a mood reaches either way, in percent of a full mood.
pub const FULL: f64 = 100.0;

/// An event that moves a mood.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoodEvent {
    /// He scored.
    Goal,
    /// His completed pass reached the scorer.
    Assist,
    /// His shot of at least the big-chance expected goals ended without a goal.
    BigMiss,
    /// His open-play pass was controlled by the other side.
    Error,
    /// He was shown a yellow card.
    Card,
    /// He was sent off.
    RedCard,
    /// He was fouled.
    Fouled,
    /// He won the ball in a tackle.
    DuelWon,
    /// He lost the ball to a tackle.
    DuelLost,
    /// His side scored.
    TeamGoalFor,
    /// His side conceded.
    TeamGoalAgainst,
}

impl MoodEvent {
    /// The event's impulse in `m`, percent of a full mood.
    pub fn impulse(self, m: &MoodTuning) -> f64 {
        match self {
            MoodEvent::Goal => m.goal,
            MoodEvent::Assist => m.assist,
            MoodEvent::BigMiss => m.big_miss,
            MoodEvent::Error => m.error,
            MoodEvent::Card => m.card,
            MoodEvent::RedCard => m.red_card,
            MoodEvent::Fouled => m.fouled,
            MoodEvent::DuelWon => m.duel_won,
            MoodEvent::DuelLost => m.duel_lost,
            MoodEvent::TeamGoalFor => m.team_goal_for,
            MoodEvent::TeamGoalAgainst => m.team_goal_against,
        }
    }
}

/// How far an event moves a player of personality `p`:
/// `(1 − temperament_weight · temperament)(1 − pressure_weight · pressure handling)`, 1 for
/// a player rated 10 in both, less for a calm one, more for a volatile one.
pub fn swing(p: &PersonalityScales, m: &MoodTuning) -> f64 {
    (1.0 - m.temperament_weight * p.temperament) * (1.0 - m.pressure_weight * p.pressure_handling)
}

/// The weight of a player of personality `p` in his side's mood: `1 + leadership_weight ·
/// leadership`, at least 0.1.
pub fn pull(p: &PersonalityScales, m: &MoodTuning) -> f64 {
    (1.0 + m.leadership_weight * p.leadership).max(0.1)
}

/// The mood the drift moves a player of side `team` toward: `score_weight` per goal of the
/// side's lead (at most two either way); from `late_from_minute` on, in a game within one
/// goal, lower by `late_pressure · (1 − pressure handling) / 2`.
pub fn drift_target(
    goals: [u32; 2],
    team: usize,
    minute: u32,
    pressure: f64,
    m: &MoodTuning,
) -> f64 {
    let lead = (i64::from(goals[team]) - i64::from(goals[1 - team])).clamp(-2, 2);
    let mut target = m.score_weight * lead as f64;
    if minute >= m.late_from_minute && lead.abs() <= 1 {
        target -= m.late_pressure * (1.0 - pressure) / 2.0;
    }
    target.clamp(-FULL, FULL)
}

/// The mood a player of personality `p` starts a match at, from the season inputs of his
/// condition `c`: `form_weight` per rating point of his recent form above `form_neutral`, plus
/// `goal_weight · (1 + ambition_weight · ambition)` times how far ahead of the pace of his
/// season goal he is (−1 a full pace behind, 1 a full pace ahead), kept within `cap` either
/// way. A missing input adds nothing.
pub fn kick_off_mood(c: &Condition, p: &PersonalityScales, s: &SeasonTuning) -> f64 {
    let form = c.recent_rating.map_or(0.0, |r| {
        s.form_weight * (f64::from(r) / 10.0 - s.form_neutral)
    });
    let goal = c.goal_progress.map_or(0.0, |g| {
        let ahead = ((f64::from(g) - 100.0) / 100.0).clamp(-1.0, 1.0);
        s.goal_weight * (1.0 + s.ambition_weight * p.ambition) * ahead
    });
    (form + goal).clamp(-s.cap, s.cap)
}

/// `mood` moved toward `target` by at most `step`.
fn toward(mood: f64, target: f64, step: f64) -> f64 {
    if mood < target {
        (mood + step).min(target)
    } else {
        (mood - step).max(target)
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::params::PersonTuning;

    fn shipped() -> MoodTuning {
        crate::contract::ContractTuning::default().person().mood
    }

    #[test]
    fn a_rating_ten_player_swings_by_one_and_pulls_by_one() {
        let m = shipped();
        assert_eq!(swing(&PersonalityScales::NEUTRAL, &m), 1.0);
        assert_eq!(pull(&PersonalityScales::NEUTRAL, &m), 1.0);
        let calm = PersonalityScales {
            temperament: 0.8,
            pressure_handling: 0.5,
            ..PersonalityScales::NEUTRAL
        };
        assert!(swing(&calm, &m) < 1.0);
    }

    #[test]
    fn the_kick_off_mood_reads_form_and_goal_progress_weighted_by_ambition() {
        let s = SeasonTuning::shipped();
        let p = |ambition: f64| PersonalityScales {
            ambition,
            ..PersonalityScales::NEUTRAL
        };
        let c = |form: Option<u8>, progress: Option<u8>| Condition {
            recent_rating: form,
            goal_progress: progress,
            ..Condition::default()
        };
        assert_eq!(kick_off_mood(&c(None, None), &p(0.8), &s), 0.0);
        let good = c(Some(75), Some(150));
        assert_eq!(kick_off_mood(&good, &p(0.8), &SeasonTuning::NEUTRAL), 0.0);
        assert!((kick_off_mood(&good, &p(0.0), &s) - 27.5).abs() < 1e-9);
        assert!(kick_off_mood(&good, &p(0.8), &s) > kick_off_mood(&good, &p(-0.8), &s));
        let behind = c(None, Some(40));
        assert!(kick_off_mood(&behind, &p(0.8), &s) < kick_off_mood(&behind, &p(-0.8), &s));
        assert!(kick_off_mood(&behind, &p(-0.8), &s) < 0.0);
        let best = c(Some(100), Some(200));
        assert_eq!(kick_off_mood(&best, &p(1.0), &s), s.cap);
        let worst = c(Some(10), Some(0));
        assert_eq!(kick_off_mood(&worst, &p(1.0), &s), -s.cap);
    }

    #[test]
    fn the_drift_target_follows_the_lead_and_a_late_close_game() {
        let m = shipped();
        assert_eq!(drift_target([0, 0], 0, 10, 0.0, &m), 0.0);
        assert_eq!(drift_target([3, 0], 0, 10, 0.0, &m), 2.0 * m.score_weight);
        assert_eq!(drift_target([0, 4], 0, 10, 0.0, &m), -2.0 * m.score_weight);
        let late = drift_target([1, 1], 0, m.late_from_minute, 0.0, &m);
        assert_eq!(late, -m.late_pressure / 2.0);
        let calm = drift_target([1, 1], 0, m.late_from_minute, 1.0, &m);
        assert_eq!(calm, 0.0, "a top pressure handler feels no late pressure");
        let neutral = PersonTuning::NEUTRAL.mood;
        assert_eq!(drift_target([2, 0], 0, 89, -1.0, &neutral), 0.0);
    }

    #[test]
    fn toward_never_overshoots() {
        assert_eq!(toward(0.0, 10.0, 3.0), 3.0);
        assert_eq!(toward(9.0, 10.0, 3.0), 10.0);
        assert_eq!(toward(-5.0, -10.0, 3.0), -8.0);
        assert_eq!(toward(-9.0, -10.0, 3.0), -10.0);
    }
}

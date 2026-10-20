//! The mood rules, as pure functions of the tuning, a player's personality and the score:
//! how far an event moves a player, how much he weighs in his side's mood, the target the
//! drift moves toward, the kick-off mood from his season inputs, and the word a mood reads as.
//! The match state that holds each mood and applies these rules is in `crate::sim::mood`.

use crate::contract::PersonalityScales;
use crate::contract::params::{MoodTuning, SeasonTuning};
use crate::data::team::Condition;

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

/// The lowest mood, in percent of a full mood, that reads `confident`.
pub const CONFIDENT_FROM: f64 = 30.0;
/// The lowest mood that reads `lifted`.
pub const LIFTED_FROM: f64 = 10.0;
/// The mood above which a mood reads `steady` (below `lifted`).
pub const STEADY_ABOVE: f64 = -10.0;
/// The mood above which a mood reads `low`; at or below it, `rattled`.
pub const LOW_ABOVE: f64 = -30.0;

/// The word key of `mood` (percent of a full mood): `confident` from 30, `lifted` from 10,
/// `steady` above −10, `low` above −30, `rattled` at −30 and below. The page owns the text of
/// each key; a mood never leaves the engine as a number.
pub fn mood_word(mood: f64) -> &'static str {
    if mood >= CONFIDENT_FROM {
        "confident"
    } else if mood >= LIFTED_FROM {
        "lifted"
    } else if mood > STEADY_ABOVE {
        "steady"
    } else if mood > LOW_ABOVE {
        "low"
    } else {
        "rattled"
    }
}

/// `mood` moved toward `target` by at most `step`.
pub(crate) fn toward(mood: f64, target: f64, step: f64) -> f64 {
    if mood < target {
        (mood + step).min(target)
    } else {
        (mood - step).max(target)
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
    fn the_mood_word_follows_its_five_bands() {
        assert_eq!(mood_word(FULL), "confident");
        assert_eq!(mood_word(30.0), "confident");
        assert_eq!(mood_word(29.9), "lifted");
        assert_eq!(mood_word(10.0), "lifted");
        assert_eq!(mood_word(9.9), "steady");
        assert_eq!(mood_word(0.0), "steady");
        assert_eq!(mood_word(-9.9), "steady");
        assert_eq!(mood_word(-10.0), "low");
        assert_eq!(mood_word(-29.9), "low");
        assert_eq!(mood_word(-30.0), "rattled");
        assert_eq!(mood_word(-FULL), "rattled");
    }

    #[test]
    fn toward_never_overshoots() {
        assert_eq!(toward(0.0, 10.0, 3.0), 3.0);
        assert_eq!(toward(9.0, 10.0, 3.0), 10.0);
        assert_eq!(toward(-5.0, -10.0, 3.0), -8.0);
        assert_eq!(toward(-9.0, -10.0, 3.0), -10.0);
    }
}

//! Tackles, fouls, and cards as pure functions of the players and one draw each.
//!
//! A tackle attempt reads one draw and splits it three ways: a clean win below `p_win`, a
//! foul in the next `p_foul`, and a miss above both. The foul band is split once more: its
//! first `ball_loss` share is a foul after which the fouled team loses the ball, and the rest
//! is a foul after which the fouled team keeps it. So a tackle costs exactly one draw.

use crate::player::Derived;
use crate::tuning::Tuning;

/// What one tackle attempt did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tackle {
    /// The tackler takes the ball cleanly.
    Win,
    /// A foul. `ball_lost` is true when the fouled team lost the ball.
    Foul { ball_lost: bool },
    /// Nothing happens.
    Miss,
}

/// A card a referee shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Card {
    Yellow,
    /// A second caution in the match: a yellow card followed by a red.
    SecondYellow,
    Red,
}

impl Card {
    /// Every card, in declaration order.
    pub const ALL: [Card; 3] = [Card::Yellow, Card::SecondYellow, Card::Red];

    /// The card as a match event writes it.
    pub fn code(&self) -> &'static str {
        match self {
            Card::Yellow => "yellow",
            Card::SecondYellow => "second-yellow",
            Card::Red => "red",
        }
    }

    /// `true` when the player leaves the pitch.
    pub fn sends_off(&self) -> bool {
        !matches!(self, Card::Yellow)
    }
}

/// The outcome of a tackle attempt with win chance `p_win` and foul chance `p_foul`.
pub fn tackle_outcome(p_win: f64, p_foul: f64, ball_loss: f64, draw: f64) -> Tackle {
    if draw < p_win {
        Tackle::Win
    } else if draw < p_win + p_foul {
        Tackle::Foul {
            ball_lost: (draw - p_win) < p_foul * ball_loss,
        }
    } else {
        Tackle::Miss
    }
}

/// The chance that one tackle attempt by `tackler` is a foul: the tuned base rate, raised by
/// aggression and lowered by tackling skill, both measured from the middle of the scale.
pub fn foul_chance(tackler: &Derived, t: &Tuning) -> f64 {
    let aggression = 1.0 + t.foul_aggression_weight * (tackler.aggression - 0.5);
    let skill = 1.0 - t.foul_tackling_weight * (tackler.tackling / 100.0 - 0.5);
    (t.foul_base * aggression * skill).clamp(0.0, 1.0)
}

/// The card, if any, for a foul by a player with `aggression` (0 to 1) who has already been
/// shown `yellows` yellow cards.
pub fn card_outcome(aggression: f64, yellows: u8, t: &Tuning, draw: f64) -> Option<Card> {
    let red = t.red_base;
    let yellow = t.yellow_base + t.yellow_aggression_weight * aggression;
    if draw < red {
        Some(Card::Red)
    } else if draw < red + yellow {
        Some(if yellows >= 1 {
            Card::SecondYellow
        } else {
            Card::Yellow
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::test_support::flat_player;

    #[test]
    fn one_draw_splits_into_win_foul_and_miss() {
        // Bands: win [0, 0.25), foul with the ball lost [0.25, 0.375), foul with the ball
        // kept [0.375, 0.5), miss from 0.5. Every edge is exact in binary.
        let outcome = |draw| tackle_outcome(0.25, 0.25, 0.5, draw);
        assert_eq!(outcome(0.0), Tackle::Win);
        assert_eq!(outcome(0.249), Tackle::Win);
        assert_eq!(outcome(0.25), Tackle::Foul { ball_lost: true });
        assert_eq!(outcome(0.374), Tackle::Foul { ball_lost: true });
        assert_eq!(outcome(0.375), Tackle::Foul { ball_lost: false });
        assert_eq!(outcome(0.499), Tackle::Foul { ball_lost: false });
        assert_eq!(outcome(0.5), Tackle::Miss);
        assert_eq!(tackle_outcome(0.25, 0.0, 0.5, 0.25), Tackle::Miss);
    }

    #[test]
    fn aggression_raises_and_skill_lowers_the_foul_chance() {
        let t = Tuning::default();
        let average = flat_player(0, 50, &t).derived;
        assert!((foul_chance(&average, &t) - t.foul_base).abs() < 1e-12);
        let mut hothead = average;
        hothead.aggression = 1.0;
        assert!(foul_chance(&hothead, &t) > t.foul_base);
        let mut expert = average;
        expert.tackling = 100.0;
        assert!(foul_chance(&expert, &t) < t.foul_base);
    }

    #[test]
    fn a_second_caution_is_a_second_yellow() {
        let t = Tuning::default();
        assert_eq!(card_outcome(0.5, 0, &t, 0.0), Some(Card::Red));
        let yellow_edge = t.red_base + t.yellow_base + t.yellow_aggression_weight * 0.5;
        assert_eq!(card_outcome(0.5, 0, &t, t.red_base), Some(Card::Yellow));
        assert_eq!(
            card_outcome(0.5, 1, &t, t.red_base),
            Some(Card::SecondYellow)
        );
        assert_eq!(
            card_outcome(0.5, 0, &t, yellow_edge - 1e-9),
            Some(Card::Yellow)
        );
        assert_eq!(card_outcome(0.5, 0, &t, yellow_edge), None);
        assert!(Card::SecondYellow.sends_off() && Card::Red.sends_off());
        assert!(!Card::Yellow.sends_off());
    }
}

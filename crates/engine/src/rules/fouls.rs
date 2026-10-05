//! Tackles, fouls, and cards as pure functions of the players and one draw each.
//!
//! A tackle attempt reads one draw and splits it three ways: a clean win below `p_win`, a
//! foul in the next `p_foul`, and a miss above both. The foul band is split once more: its
//! first `ball_loss` share is a foul after which the fouled team loses the ball, and the rest
//! is a foul after which the fouled team keeps it. So a tackle costs exactly one draw.

use crate::modules::{FoulsModule, MatchView, ModuleCard, TackleChances};
use crate::player::Derived;
use crate::streams::Action;
use crate::tuning::Tuning;

/// Metres per second above which a carrier runs with the ball rather than shields it, for
/// the extra tackle chance against a running carrier.
pub const RUNNING_SPEED: f64 = 2.0;

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

    /// How severe the card is: a caution below a sending-off, and a second caution below a
    /// straight red.
    pub fn severity(&self) -> u8 {
        match self {
            Card::Yellow => 0,
            Card::SecondYellow => 1,
            Card::Red => 2,
        }
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

/// The chance that one tackle attempt by `tackler` wins the ball cleanly from `carrier`: the
/// tuned base, scaled by the tackler's tackling against the carrier's dribbling.
pub fn win_chance(tackler: &Derived, carrier: &Derived, t: &Tuning) -> f64 {
    t.tackle_win_base * tackler.tackling / (tackler.tackling + carrier.dribbling)
}

/// The extra chance that one tackle attempt by `tackler` wins the ball from `carrier` while
/// he runs with it: `tackle_dribble_win`, scaled by the same skill ratio as `win_chance`.
pub fn dribble_win_chance(tackler: &Derived, carrier: &Derived, t: &Tuning) -> f64 {
    t.tackle_dribble_win * tackler.tackling / (tackler.tackling + carrier.dribbling)
}

/// The chance that one tackle attempt by `tackler` is a foul: the tuned base rate, raised by
/// aggression and lowered by tackling skill, both measured from the middle of the scale. A
/// player already shown `yellows` cards tackles more carefully: the chance is multiplied by
/// the booked factor.
pub fn foul_chance(tackler: &Derived, yellows: u8, t: &Tuning) -> f64 {
    let aggression = 1.0 + t.foul_aggression_weight * (tackler.aggression - 0.5);
    let skill = 1.0 - t.foul_tackling_weight * (tackler.tackling / 100.0 - 0.5);
    let booked = if yellows >= 1 {
        t.foul_booked_factor
    } else {
        1.0
    };
    (t.foul_base * aggression * skill * booked).clamp(0.0, 1.0)
}

/// The card, if any, for a foul by a player with `aggression` (0 to 1) who has already been
/// shown `yellows` yellow cards.
pub fn card_outcome(aggression: f64, yellows: u8, t: &Tuning, draw: f64) -> Option<Card> {
    let [red, red_or_yellow] = card_thresholds(aggression, t);
    if draw < red {
        Some(Card::Red)
    } else if draw < red_or_yellow {
        Some(if yellows >= 1 {
            Card::SecondYellow
        } else {
            Card::Yellow
        })
    } else {
        None
    }
}

/// The two cumulative thresholds a foul's card draw is tested against: below the first a
/// red card, below the second a yellow.
pub fn card_thresholds(aggression: f64, t: &Tuning) -> [f64; 2] {
    let red = t.red_base;
    let yellow = t.yellow_base + t.yellow_aggression_weight * aggression;
    [red, red + yellow]
}

/// The fouls module, version 1: the functions above, called with the same operands in the
/// same order as before the move, so every chance keeps its bits.
pub struct FoulsV1;

impl FoulsModule for FoulsV1 {
    #[inline]
    fn tackle_chances(
        &self,
        view: &MatchView<'_>,
        tackler: usize,
        carrier: usize,
    ) -> TackleChances {
        let t = view.tuning();
        let p = view.player(tackler);
        let carrier = view.player(carrier);
        let mut p_win = win_chance(&p.derived, &carrier.derived, t);
        if carrier.vel.length() > RUNNING_SPEED {
            p_win += dribble_win_chance(&p.derived, &carrier.derived, t);
        }
        TackleChances {
            p_win,
            p_foul: foul_chance(&p.derived, p.yellow, t),
            ball_loss: t.foul_ball_loss,
        }
    }

    #[inline]
    fn tackle_outcome(&self, c: &TackleChances, draw: f64) -> Tackle {
        tackle_outcome(c.p_win, c.p_foul, c.ball_loss, draw)
    }

    #[inline]
    fn card_thresholds(&self, view: &MatchView<'_>, offender: usize) -> [f64; 2] {
        card_thresholds(view.player(offender).derived.aggression, view.tuning())
    }

    #[inline]
    fn card(&self, view: &MatchView<'_>, offender: usize, draw: f64) -> Option<Card> {
        let p = view.player(offender);
        card_outcome(p.derived.aggression, p.yellow, view.tuning(), draw)
    }
}

pub const FOULS_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Decides each tackle attempt (clean win, foul, or miss) and the card for a foul.",
    inputs: "The tackler's and carrier's derived tackling, dribbling, and aggression, the carrier's speed, the tackler's yellow cards, and the engine tuning.",
    outputs: "The tackle chances and outcome, the card thresholds, and the card; the loop applies the foul and the card.",
    tuning: &[
        "tackle_win_base",
        "tackle_dribble_win",
        "foul_base",
        "foul_aggression_weight",
        "foul_tackling_weight",
        "foul_booked_factor",
        "foul_ball_loss",
        "red_base",
        "yellow_base",
        "yellow_aggression_weight",
    ],
    calibration: "yellow_cards_per_team",
    keys: &[Action::Tackle, Action::FoulCard],
};

/// The fouls module switched off: tackles win and miss as in version 1, but never foul, so
/// no card is ever drawn. The tackle draw still happens, so every other draw stays in step.
pub struct FoulsOff;

impl FoulsModule for FoulsOff {
    #[inline]
    fn tackle_chances(
        &self,
        view: &MatchView<'_>,
        tackler: usize,
        carrier: usize,
    ) -> TackleChances {
        TackleChances {
            p_foul: 0.0,
            ..FoulsV1.tackle_chances(view, tackler, carrier)
        }
    }

    #[inline]
    fn tackle_outcome(&self, c: &TackleChances, draw: f64) -> Tackle {
        tackle_outcome(c.p_win, 0.0, c.ball_loss, draw)
    }

    #[inline]
    fn card_thresholds(&self, _view: &MatchView<'_>, _offender: usize) -> [f64; 2] {
        [0.0, 0.0]
    }

    #[inline]
    fn card(&self, _view: &MatchView<'_>, _offender: usize, _draw: f64) -> Option<Card> {
        None
    }
}

pub const FOULS_OFF_CARD: ModuleCard = ModuleCard {
    purpose: "Fouls switched off: tackles win or miss, and nobody fouls or is shown a card.",
    inputs: "The tackler's and carrier's derived tackling and dribbling, the carrier's speed, and the engine tuning.",
    outputs: "The tackle chances with a foul chance of 0, and the tackle outcome.",
    tuning: &["tackle_win_base", "tackle_dribble_win"],
    calibration: "none: off version, no fouls",
    keys: &[Action::Tackle],
};

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
        assert!((foul_chance(&average, 0, &t) - t.foul_base).abs() < 1e-12);
        let mut hothead = average;
        hothead.aggression = 1.0;
        assert!(foul_chance(&hothead, 0, &t) > t.foul_base);
        let mut expert = average;
        expert.tackling = 100.0;
        assert!(foul_chance(&expert, 0, &t) < t.foul_base);
    }

    #[test]
    fn the_win_chance_at_the_neutral_base_is_the_earlier_formula() {
        let t = Tuning {
            tackle_win_base: 0.05,
            ..Tuning::default()
        };
        for (tackling, dribbling) in [(30, 80), (50, 50), (90, 20)] {
            let tackler = flat_player(0, tackling, &t).derived;
            let carrier = flat_player(1, dribbling, &t).derived;
            let earlier = 0.05 * tackler.tackling / (tackler.tackling + carrier.dribbling);
            assert_eq!(win_chance(&tackler, &carrier, &t), earlier);
        }
    }

    #[test]
    fn the_win_chance_scales_linearly_with_the_base() {
        let t = Tuning::default();
        let tackler = flat_player(0, 60, &t).derived;
        let carrier = flat_player(1, 40, &t).derived;
        let at = |base| {
            win_chance(
                &tackler,
                &carrier,
                &Tuning {
                    tackle_win_base: base,
                    ..t.clone()
                },
            )
        };
        assert_eq!(at(0.0), 0.0);
        assert!((at(0.2) - 2.0 * at(0.1)).abs() < 1e-15);
        assert!((at(0.1) - 0.1 * 0.6).abs() < 1e-12, "{}", at(0.1));
    }

    #[test]
    fn the_running_win_chance_scales_like_the_win_chance() {
        let t = Tuning {
            tackle_dribble_win: 0.3,
            ..Tuning::default()
        };
        let tackler = flat_player(0, 60, &t).derived;
        let carrier = flat_player(1, 40, &t).derived;
        assert!((dribble_win_chance(&tackler, &carrier, &t) - 0.3 * 0.6).abs() < 1e-12);
        let off = Tuning {
            tackle_dribble_win: 0.0,
            ..t
        };
        assert_eq!(dribble_win_chance(&tackler, &carrier, &off), 0.0);
    }

    #[test]
    fn a_booked_player_fouls_by_the_booked_factor_less() {
        let t = Tuning::default();
        let p = flat_player(0, 70, &t).derived;
        let unbooked = foul_chance(&p, 0, &t);
        for yellows in [1, 2] {
            let booked = foul_chance(&p, yellows, &t);
            assert!((booked - t.foul_booked_factor * unbooked).abs() < 1e-12);
        }
    }

    #[test]
    fn a_red_card_is_more_severe_than_a_second_yellow_and_a_yellow() {
        assert!(Card::Red.severity() > Card::SecondYellow.severity());
        assert!(Card::SecondYellow.severity() > Card::Yellow.severity());
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

    /// `card_outcome` through `card_thresholds` gives the same card as the body before the
    /// split, bit for bit, over the whole draw range.
    #[test]
    fn the_card_thresholds_keep_every_card() {
        fn before(aggression: f64, yellows: u8, t: &Tuning, draw: f64) -> Option<Card> {
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
        let shipped = crate::data::test_support::shipped_config(1, 1)
            .unwrap()
            .tuning;
        for t in [Tuning::default(), shipped] {
            for a in 0..=100 {
                let aggression = f64::from(a) / 100.0;
                let [red, both] = card_thresholds(aggression, &t);
                assert_eq!(red.to_bits(), t.red_base.to_bits());
                for yellows in 0..=2 {
                    for d in 0..=1000 {
                        let draw = f64::from(d) / 1000.0;
                        assert_eq!(
                            card_outcome(aggression, yellows, &t, draw),
                            before(aggression, yellows, &t, draw),
                            "aggression {aggression} yellows {yellows} draw {draw}"
                        );
                    }
                    // The edges themselves.
                    for draw in [red, both] {
                        assert_eq!(
                            card_outcome(aggression, yellows, &t, draw),
                            before(aggression, yellows, &t, draw)
                        );
                    }
                }
            }
        }
    }
}

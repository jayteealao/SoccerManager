//! Tackles, fouls, and cards as pure functions of the players and one draw each.
//!
//! A tackle attempt reads one draw and splits it three ways: a clean win below `p_win`, a
//! foul in the next `p_foul`, and a miss above both. The foul band is split once more: its
//! first `ball_loss` share is a foul after which the fouled team loses the ball, and the rest
//! is a foul after which the fouled team keeps it. So a tackle costs exactly one draw.
//!
//! Every chance reads the players through the attribute contract: the win is a contest of
//! the tackler's tackle stage against the carrier's dribble stage (running) or shield stage
//! (standing); the foul rises with the tackler's commitment (the tackle choose stage) and
//! falls with his tackling; the fouled carrier's balance keeps the ball.

use crate::contract::{self, ActionKind, Skills, Stage, curve};
use crate::modules::{FoulsModule, MatchView, ModuleCard, TackleChances};
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

/// A contest of the tackler's tackle stage against `side`, the carrier's stage on the curve,
/// anchored on `base` and moved by `shift` more in log-odds, inside the tackle limits.
fn tackle_contest(tackler: Skills<'_>, side: f64, base: f64, shift: f64, t: &Tuning) -> f64 {
    let p = t.contract.actions.of(ActionKind::Tackle);
    let diff = tackler.f(Stage::TACKLE_EXECUTE) - side;
    curve::shifted(base, p.k() * diff + shift, p.floor(), p.ceiling())
}

/// The chance that one tackle attempt by `tackler` wins the ball cleanly from a standing
/// `carrier`: a contest of the tackle stage against his shield stage (strength), anchored on
/// `tackle_win_base / 2`, the even tackle before the contract.
pub fn win_chance(tackler: Skills<'_>, carrier: Skills<'_>, t: &Tuning) -> f64 {
    let side = carrier.f(Stage::SHIELD_EXECUTE);
    tackle_contest(tackler, side, t.tackle_win_base / 2.0, 0.0, t)
}

/// The carrier's side of a tackle while he runs with the ball: his dribble stage blended with
/// its pressure stage (a tackler is always close), on the curve.
fn dribble_side(carrier: Skills<'_>) -> f64 {
    (carrier.f(Stage::DRIBBLE_EXECUTE) + carrier.f(Stage::DRIBBLE_PRESSURE)) / 2.0
}

/// The chance that one tackle attempt by `tackler` wins the ball cleanly from `carrier` while
/// he runs with it: the contest against his dribble side, and a take-on his body cannot pull
/// off (agility under the gate) gives the tackler its penalty in log-odds.
pub fn running_win_chance(tackler: Skills<'_>, carrier: Skills<'_>, t: &Tuning) -> f64 {
    tackle_contest(
        tackler,
        dribble_side(carrier),
        t.tackle_win_base / 2.0,
        carrier.derived.gates.take_on_penalty,
        t,
    )
}

/// The extra chance that one tackle attempt by `tackler` wins the ball from `carrier` while
/// he runs with it: the same contest anchored on `tackle_dribble_win / 2`; nothing when that
/// is 0.
pub fn dribble_win_chance(tackler: Skills<'_>, carrier: Skills<'_>, t: &Tuning) -> f64 {
    tackle_contest(
        tackler,
        dribble_side(carrier),
        t.tackle_dribble_win / 2.0,
        carrier.derived.gates.take_on_penalty,
        t,
    )
}

/// The chance that one tackle attempt by `tackler` is a foul: the tuned base rate, moved in
/// log-odds up by his commitment (tackle choose stage) and down by his tackling (tackle
/// execute stage), each per curve point from rating 10, inside the tackle limits. A player
/// already shown `yellows` cards tackles more carefully: the chance is multiplied by the
/// booked factor.
pub fn foul_chance(tackler: Skills<'_>, yellows: u8, t: &Tuning) -> f64 {
    let c = &t.contract;
    let ten = curve::f(10.0, &c.curve);
    let shift = t.foul_aggression_weight * (tackler.f(Stage::TACKLE_CHOOSE) - ten)
        - t.foul_tackling_weight * (tackler.f(Stage::TACKLE_EXECUTE) - ten);
    let p = c.actions.of(ActionKind::Tackle);
    let booked = if yellows >= 1 {
        t.foul_booked_factor
    } else {
        1.0
    };
    (curve::shifted(t.foul_base, shift, p.floor(), p.ceiling()) * booked).clamp(0.0, 1.0)
}

/// The share of fouls on `carrier` after which his team loses the ball: `foul_ball_loss` for
/// a player of rating 10, falling as his stay-up stage (balance) rises.
pub fn ball_loss(carrier: Skills<'_>, t: &Tuning) -> f64 {
    let c = &t.contract;
    let p = c.actions.of(ActionKind::StayUp);
    contract::contest(
        t.foul_ball_loss,
        p.k(),
        curve::f(10.0, &c.curve),
        carrier.f(Stage::STAY_UP_EXECUTE),
        p.floor(),
        p.ceiling(),
    )
}

/// How committed `p` is, 0 to 1: the share of his tackle choose stage (aggression), 0.5 at
/// rating 10. The card thresholds read it.
pub fn commitment(p: Skills<'_>) -> f64 {
    p.share(Stage::TACKLE_CHOOSE)
}

/// The card, if any, for a foul by a player with commitment `aggression` (0 to 1) who has
/// already been shown `yellows` yellow cards.
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
        let (me, them) = (view.skills(tackler), view.skills(carrier));
        let carrier = view.player(carrier);
        let p_win = if carrier.vel.length() > RUNNING_SPEED {
            let mut p_win = running_win_chance(me, them, t);
            if t.tackle_dribble_win > 0.0 {
                p_win += dribble_win_chance(me, them, t);
            }
            p_win
        } else {
            win_chance(me, them, t)
        };
        TackleChances {
            p_win,
            p_foul: foul_chance(me, p.yellow, t),
            ball_loss: ball_loss(them, t),
        }
    }

    #[inline]
    fn tackle_outcome(&self, c: &TackleChances, draw: f64) -> Tackle {
        tackle_outcome(c.p_win, c.p_foul, c.ball_loss, draw)
    }

    #[inline]
    fn card_thresholds(&self, view: &MatchView<'_>, offender: usize) -> [f64; 2] {
        card_thresholds(commitment(view.skills(offender)), view.tuning())
    }

    #[inline]
    fn card(&self, view: &MatchView<'_>, offender: usize, draw: f64) -> Option<Card> {
        let p = view.player(offender);
        card_outcome(
            commitment(view.skills(offender)),
            p.yellow,
            view.tuning(),
            draw,
        )
    }
}

pub const FOULS_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Decides each tackle attempt (clean win, foul, or miss) and the card for a foul.",
    inputs: "Through the attribute contract: the tackler's tackle execute and choose stages (tackling, aggression), the carrier's dribble, shield and stay-up stages (dribbling, strength, balance) and take-on gate; the carrier's speed, the tackler's yellow cards, and the engine tuning.",
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
        "contract.curve",
        "contract.actions.tackle",
        "contract.actions.stay_up",
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
    inputs: "Through the attribute contract: the tackler's tackle stage and the carrier's dribble, shield and stay-up stages and take-on gate; the carrier's speed, and the engine tuning.",
    outputs: "The tackle chances with a foul chance of 0, and the tackle outcome.",
    tuning: &[
        "tackle_win_base",
        "tackle_dribble_win",
        "foul_ball_loss",
        "contract.curve",
        "contract.actions.tackle",
        "contract.actions.stay_up",
    ],
    calibration: "none: off version, no fouls",
    keys: &[Action::Tackle],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{Skills, StageValues};
    use crate::player::Derived;
    use crate::player::test_support::{flat, flat_player};

    type Flat = (Derived, StageValues);

    fn sk(f: &Flat) -> Skills<'_> {
        Skills::new(&f.1, &f.0)
    }

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

    /// A player rated `tenths` throughout, but `name` at `at` tenths.
    fn with(tenths: u8, name: &str, at: u8, t: &Tuning) -> Flat {
        let content = crate::data::test_support::shipped_content();
        let s = &content.attributes;
        let mut a = flat_player(0, tenths, t).attributes;
        a.values[s.index(name).unwrap()] = crate::rating::Rating::from_tenths(at);
        Derived::from_attributes(&a, s, t)
    }

    #[test]
    fn commitment_raises_and_skill_lowers_the_foul_chance() {
        let t = Tuning::default();
        let average = flat(100, &t);
        assert_eq!(foul_chance(sk(&average), 0, &t), t.foul_base);
        assert!(foul_chance(sk(&with(100, "aggression", 200, &t)), 0, &t) > t.foul_base);
        assert!(foul_chance(sk(&with(100, "tackling", 200, &t)), 0, &t) < t.foul_base);
    }

    #[test]
    fn an_even_tackle_wins_half_the_base_as_before_and_a_better_tackler_wins_more() {
        let t = Tuning::default();
        let even = flat(100, &t);
        assert_eq!(
            win_chance(sk(&even), sk(&even), &t),
            t.tackle_win_base / 2.0
        );
        assert_eq!(
            running_win_chance(sk(&even), sk(&even), &t),
            t.tackle_win_base / 2.0
        );
        // Away from rating 10 each stage's blend rounds on its own, so two equal players sit
        // on the base to within rounding.
        let twelve = flat(120, &t);
        assert!((win_chance(sk(&twelve), sk(&twelve), &t) - t.tackle_win_base / 2.0).abs() < 1e-12);
        let ten = flat(100, &t);
        let good = with(100, "tackling", 180, &t);
        assert!(win_chance(sk(&good), sk(&ten), &t) > win_chance(sk(&ten), sk(&ten), &t));
        let strong = with(100, "strength", 180, &t);
        assert!(win_chance(sk(&ten), sk(&strong), &t) < win_chance(sk(&ten), sk(&ten), &t));
        let dribbler = with(100, "dribbling", 180, &t);
        assert!(
            running_win_chance(sk(&ten), sk(&dribbler), &t)
                < running_win_chance(sk(&ten), sk(&ten), &t)
        );
    }

    #[test]
    fn every_tackle_chance_stays_inside_the_limits() {
        let t = Tuning::default();
        let p = t.contract.actions.of(ActionKind::Tackle);
        let top = flat(200, &t);
        let bottom = flat(10, &t);
        for (a, b) in [(&top, &bottom), (&bottom, &top)] {
            for chance in [
                win_chance(sk(a), sk(b), &t),
                running_win_chance(sk(a), sk(b), &t),
                foul_chance(sk(a), 0, &t),
            ] {
                assert!((p.floor()..=p.ceiling()).contains(&chance), "{chance}");
            }
        }
        assert_eq!(win_chance(sk(&top), sk(&bottom), &t), p.ceiling());
    }

    #[test]
    fn a_take_on_the_body_cannot_pull_off_loses_more_tackles() {
        let t = Tuning::default();
        let tackler = flat(100, &t);
        let nimble = flat(100, &t);
        let mut stiff = nimble;
        stiff.0.gates.take_on_penalty = 0.6;
        assert!(
            running_win_chance(sk(&tackler), sk(&stiff), &t)
                > running_win_chance(sk(&tackler), sk(&nimble), &t)
        );
    }

    #[test]
    fn the_running_extra_is_anchored_on_its_own_base() {
        let t = Tuning {
            tackle_dribble_win: 0.3,
            ..Tuning::default()
        };
        let even = flat(100, &t);
        assert_eq!(dribble_win_chance(sk(&even), sk(&even), &t), 0.15);
        let off = Tuning {
            tackle_dribble_win: 0.0,
            ..t
        };
        assert_eq!(dribble_win_chance(sk(&even), sk(&even), &off), 0.0);
    }

    #[test]
    fn balance_keeps_the_ball_after_a_foul() {
        let t = Tuning::default();
        let ten = flat(100, &t);
        assert_eq!(ball_loss(sk(&ten), &t), t.foul_ball_loss);
        assert!(ball_loss(sk(&with(100, "balance", 180, &t)), &t) < t.foul_ball_loss);
        assert_eq!(commitment(sk(&ten)), 0.5);
    }

    #[test]
    fn a_booked_player_fouls_by_the_booked_factor_less() {
        let t = Tuning::default();
        let p = flat(140, &t);
        let unbooked = foul_chance(sk(&p), 0, &t);
        for yellows in [1, 2] {
            let booked = foul_chance(sk(&p), yellows, &t);
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

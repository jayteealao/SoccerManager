//! The pressure model: one definition of how hard a player is pressed, which play and the
//! counts both read.
//!
//! - **Intensity** (0 to 1, per player, per moment). Each active opponent adds a share that
//!   falls with his distance (full at [`NEAR_M`], none from [`FAR_M`], straight between),
//!   rises with how fast he closes the gap (full at [`CLOSING_MPS`]) and is weighed by where
//!   he stands against the direction the player attacks: in front of him (between him and the
//!   goal he attacks) 1, level with him [`LEVEL_WEIGHT`], behind him [`BEHIND_WEIGHT`], straight
//!   between, fitted to the StatsBomb 360 freeze frames ([`position_weight`]). The shares combine
//!   as independent chances, `1 - Π(1 - share)`, so more opponents press harder and the
//!   intensity never passes 1.
//! - **Pressed.** A player is pressed when his intensity passes [`THRESHOLD`], the share of
//!   one still opponent standing in front of him at [`crate::contract::PRESSED_M`]; an
//!   opponent presses him (a pressure the counts see) when that opponent's own share passes
//!   it. The pressure counts, the press triggers' counts and the counter-press counts read
//!   this one rule.
//! - **Penalty** of a pressed action = intensity x action difficulty x (1 - player
//!   resistance). The difficulty of a pass follows the real pressed-against-free completion
//!   ratio by its length ([`PASS_DIFFICULTY`]: a short pass under pressure suffers most, a long
//!   ball least) and grows when it is lofted; of a reception, with the ball's speed against
//!   what the receiver can control and when he faces his own goal. The tuning's `decision.pass_under_pressure` and
//!   `decision.touch_under_pressure` size the penalty so that the engine's pressed-against-free
//!   gaps match the StatsBomb open data.
//! - **Resistance** (0 to 1, 0.5 for a player rated 10 throughout) comes from the player's
//!   attributes through the stages of the attribute contract, several stages per action, each
//!   read where its main attribute leads it:
//!   - receiving: first touch (receive execute), composure (dribble pressure: keeping the ball
//!     with an opponent close), strength (shield execute) and balance (stay-up execute);
//!   - passing: passing (the pass kind's execute stage), composure (its pressure stage),
//!     vision (pass see) and decisions (pass choose: finding the safe option in time);
//!   - dribbling: dribbling, agility and balance (dribble execute) and composure (dribble
//!     pressure), the two stages the tackle contest on a running carrier reads
//!     ([`crate::rules::fouls::running_win_chance`]), so a press-resistant dribbler keeps the
//!     ball against more of the tackles a press brings.
//! - **Choice** of a pressed carrier ([`crate::decision`]): the intensity he feels grades his
//!   pressed option terms ([`grade`]), and each option that keeps the ball pays his own penalty
//!   for it (the pass's difficulty and resistance above; a dribble and a hold their own,
//!   [`DRIBBLE_DIFFICULTY`], [`HOLD_DIFFICULTY`], [`choice_resistance`]), so a carrier who
//!   resists the press badly goes long or clears more as it rises. Sized so the rise of the
//!   long-ball and clearance share under pressure matches the StatsBomb open data's (x1.87).
//! - **Read** of the press (anticipation, through the intercept see stage): a player who reads
//!   the press sees a presser earlier. It scales how much a presser on his blind side (behind
//!   his facing) presses him ([`from_read`], [`READ_WEIGHT`]) and, on a reception, the
//!   difficulty of facing his own goal (he sets his
//!   body before the ball arrives); a player rated 10 reads as the observer does. The counts
//!   (pressed, a pressure) read the observer's intensity, [`intensity`], the way the real data's
//!   flag is an observer's: one definition, [`from_read`], with the neutral read.

use crate::contract::{PRESSED_M, Skills, Stage};
use crate::player::Player;

/// The distance, in metres, within which an opponent's distance share is full.
pub const NEAR_M: f64 = 0.5;
/// The distance, in metres, from which an opponent adds no pressure.
pub const FAR_M: f64 = 5.0;
/// The closing speed, in metres per second, at which an opponent's closing share is full.
pub const CLOSING_MPS: f64 = 4.0;
/// The weight of a full closing share, against 1 for the distance alone.
pub const CLOSING_WEIGHT: f64 = 0.5;
/// The weight of an opponent level with the player (square to the direction he attacks),
/// against 1 for one straight in front of him. StatsBomb open data (the four competitions of
/// [`PASS_DIFFICULTY`]): on open-play outfield passes and receipts with one opponent within
/// 5 m in the 360 freeze frame, the share the observer flags as under pressure by that
/// opponent's distance, in front (within 60 degrees of the attacking direction), level or
/// behind (beyond 120 degrees). Each weight is the one that lays its side's curve on the
/// front curve through the distance share (least squares, weighted by count): an opponent
/// level presses a little more than one in front, one behind clearly less.
pub const LEVEL_WEIGHT: f64 = 1.053;
/// The weight of an opponent straight behind the player, against 1 for one straight in front;
/// fitted as [`LEVEL_WEIGHT`].
pub const BEHIND_WEIGHT: f64 = 0.824;
/// How much a player's read of the press moves the pressure of an opponent on his blind side
/// (straight behind his facing): a share x `1 + READ_WEIGHT x (unread - 1)` there, so 1.5
/// times with no read, half with a full read, as it stands at the neutral read; nothing for
/// an opponent he faces.
pub const READ_WEIGHT: f64 = 0.5;
/// The scale the shares are divided by: one still opponent straight in front adds his
/// distance share over it. It is the scale they had when an opponent behind the player's
/// facing counted half again, kept so that a presser in front presses as before.
pub const SHARE_SCALE: f64 = 2.0;

/// The intensity at which a player is pressed: the share of one still opponent straight in
/// front of him at [`PRESSED_M`].
pub const THRESHOLD: f64 = (FAR_M - PRESSED_M) / (FAR_M - NEAR_M) / SHARE_SCALE;

/// A pass's difficulty by its length, at the middle of each length band (5, 15, 25 and 35 m;
/// straight between, flat outside): the real loss of completion under pressure in the band,
/// `1 - pressed / free`, over the loss in the shortest band. StatsBomb open data (World Cup
/// 2022, Euro 2020, Euro 2024, Bundesliga 2023/24; outfield open-play passes by
/// `under_pressure`): ratios 0.810 (under 10 m), 0.890 (10 to 20 m), 0.856 (20 to 30 m) and
/// 0.904 (from 30 m), so a short pressed pass is the hardest and a long ball the easiest.
pub const PASS_DIFFICULTY: [f64; 4] = [1.0, 0.577, 0.759, 0.507];
/// The extra difficulty of a lofted pass (a chip or a cross) over one along the ground of the
/// same length; the difficulty is at most 1.
pub const LOFT_DIFFICULTY: f64 = 0.2;

/// A reception's difficulty: [`RECEIVE_EASE`] for a slow ball to a receiver facing play,
/// with [`SPEED_DIFFICULTY`] at the fastest ball he can control and [`BACK_DIFFICULTY`] when
/// he faces his own goal; at most 1.
pub const RECEIVE_EASE: f64 = 0.4;
pub const SPEED_DIFFICULTY: f64 = 0.3;
pub const BACK_DIFFICULTY: f64 = 0.3;

/// The distance share of an opponent `d` metres away: 1 within [`NEAR_M`], 0 from
/// [`FAR_M`], straight between.
#[inline]
pub fn distance_share(d: f64) -> f64 {
    ((FAR_M - d) / (FAR_M - NEAR_M)).clamp(0.0, 1.0)
}

/// The read of a player rated 10, and of the observer the counts use: the blind-side weight
/// and the facing difficulty as they stand.
pub const NEUTRAL_READ: f64 = 0.5;

/// How much of a weight a player with read `read` (0 to 1) feels: 1 at the neutral read,
/// none at a full read, twice at none.
#[inline]
fn unread(read: f64) -> f64 {
    2.0 * (1.0 - read.clamp(0.0, 1.0))
}

/// The weight of an opponent by where he stands: `ahead` is the cosine between the direction
/// the player attacks and the line to the opponent (1 straight in front, 0 level, -1 straight
/// behind); 1 in front, [`LEVEL_WEIGHT`] level, [`BEHIND_WEIGHT`] behind, straight between.
#[inline]
pub fn position_weight(ahead: f64) -> f64 {
    let a = ahead.clamp(-1.0, 1.0);
    if a >= 0.0 {
        LEVEL_WEIGHT + (1.0 - LEVEL_WEIGHT) * a
    } else {
        LEVEL_WEIGHT + (LEVEL_WEIGHT - BEHIND_WEIGHT) * a
    }
}

/// The pressure opponent `o` puts on player `p`, who attacks along `attack_x`, 0 to 1, as the
/// observer sees it: his distance share, raised by his closing speed and weighed by where he
/// stands ([`position_weight`]).
#[inline]
pub fn from(p: &Player, o: &Player, attack_x: f64) -> f64 {
    from_read(p, o, NEUTRAL_READ, attack_x)
}

/// [`from`] for a player with read `read`: an opponent on his blind side (behind his facing)
/// presses him more the less he reads the press ([`READ_WEIGHT`]; [`NEUTRAL_READ`] gives
/// [`from`]).
#[inline]
pub fn from_read(p: &Player, o: &Player, read: f64, attack_x: f64) -> f64 {
    let gap = o.pos - p.pos;
    let d = gap.length();
    let near = distance_share(d);
    if near <= 0.0 {
        return 0.0;
    }
    // The direction from the player to the opponent; at the same spot, straight ahead.
    let toward = if d > 1e-9 { gap / d } else { p.facing };
    let closing = ((p.vel - o.vel).dot(toward) / CLOSING_MPS).clamp(0.0, 1.0);
    let place = position_weight(toward.x * attack_x);
    let blind = ((1.0 - p.facing.dot(toward)) / 2.0).clamp(0.0, 1.0);
    let seen = 1.0 + READ_WEIGHT * blind * (unread(read) - 1.0);
    (near * place * (1.0 + CLOSING_WEIGHT * closing) * seen / SHARE_SCALE).min(1.0)
}

/// Player `p`'s pressure intensity, 0 to 1, from the active opponents among `players`, as the
/// observer sees it (the counts read this); `attack_x` is the direction `p` attacks.
pub fn intensity(players: &[Player], p: &Player, attack_x: f64) -> f64 {
    felt(players, p, NEUTRAL_READ, attack_x)
}

/// The intensity player `p` with read `read` feels, 0 to 1: [`intensity`] with each presser's
/// share from [`from_read`].
pub fn felt(players: &[Player], p: &Player, read: f64, attack_x: f64) -> f64 {
    let mut free = 1.0;
    for o in players {
        if o.team != p.team && o.active() {
            free *= 1.0 - from_read(p, o, read, attack_x);
        }
    }
    1.0 - free
}

/// A player's read of the press: his intercept see stage share (anticipation leading).
#[inline]
pub fn read(s: Skills<'_>) -> f64 {
    s.share(Stage::INTERCEPT_SEE)
}

/// The weights of a reception's resistance: first touch, composure, strength, balance.
pub const RECEIVE_WEIGHTS: [f64; 4] = [0.4, 0.3, 0.15, 0.15];
/// The weights of a pass's resistance: passing, composure, vision, decisions.
pub const PASS_WEIGHTS: [f64; 4] = [0.35, 0.35, 0.15, 0.15];

/// A dribble's resistance, as the harness reports it beside the running keep share: dribbling,
/// agility and balance (dribble execute) and composure (dribble pressure), the two stages the
/// tackle contest on a running carrier reads, in equal parts.
pub fn dribble_resistance(s: Skills<'_>) -> f64 {
    0.5 * s.share(Stage::DRIBBLE_EXECUTE) + 0.5 * s.share(Stage::DRIBBLE_PRESSURE)
}

/// The weights of holding the ball while standing: strength (shield), balance (stay-up),
/// composure (dribble pressure).
pub const HOLD_WEIGHTS: [f64; 3] = [0.4, 0.3, 0.3];

/// Holding the ball while standing: strength (shield), balance (stay-up) and composure
/// (dribble pressure).
pub fn hold_resistance(s: Skills<'_>) -> f64 {
    let [a, b, c] = HOLD_WEIGHTS;
    a * s.share(Stage::SHIELD_EXECUTE)
        + b * s.share(Stage::STAY_UP_EXECUTE)
        + c * s.share(Stage::DRIBBLE_PRESSURE)
}

/// Where a pressed carrier's graded choice terms start, as a share of [`THRESHOLD`]: they grow
/// from this share of the threshold intensity, 0, to the threshold itself, 1, so a carrier the
/// counts call pressed weighs his options as a pressed carrier in full, and one nearly pressed
/// in part.
pub const GRADE_START: f64 = 0.5;

/// How far a carrier's intensity `i` has graded his pressed choice terms, 0 to 1
/// ([`GRADE_START`]).
#[inline]
pub fn grade(i: f64) -> f64 {
    ((i / THRESHOLD - GRADE_START) / (1.0 - GRADE_START)).clamp(0.0, 1.0)
}

/// The share of decisions (the action's choose stage) in a pressed carrier's resistance when
/// he weighs a dribble or a hold: finding the way out in time.
pub const CHOICE_DECISIONS: f64 = 0.25;

/// The resistance a pressed carrier weighs a dribble or a hold by: the action's resistance
/// (`action`) blended with his decisions (`decisions`, the action's choose stage share).
#[inline]
pub fn choice_resistance(action: f64, decisions: f64) -> f64 {
    (1.0 - CHOICE_DECISIONS) * action + CHOICE_DECISIONS * decisions
}

/// The difficulty of keeping the ball on a dribble past a presser and of holding it while
/// standing, on the pass scale ([`PASS_DIFFICULTY`]): a carry at a presser is a hard keep, a
/// still shield easier.
pub const DRIBBLE_DIFFICULTY: f64 = 0.8;
pub const HOLD_DIFFICULTY: f64 = 0.6;

/// A reception's resistance: first touch (receive execute), composure (dribble pressure),
/// strength (shield) and balance (stay-up).
pub fn reception_resistance(s: Skills<'_>) -> f64 {
    let [a, b, c, d] = RECEIVE_WEIGHTS;
    a * s.share(Stage::RECEIVE_EXECUTE)
        + b * s.share(Stage::DRIBBLE_PRESSURE)
        + c * s.share(Stage::SHIELD_EXECUTE)
        + d * s.share(Stage::STAY_UP_EXECUTE)
}

/// A pass's resistance: passing (the kind's `execute` stage), composure (its `pressure`
/// stage), vision (pass see) and decisions (pass choose).
pub fn pass_resistance(s: Skills<'_>, execute: Stage, pressure: Stage) -> f64 {
    let [a, b, c, d] = PASS_WEIGHTS;
    a * s.share(execute)
        + b * s.share(pressure)
        + c * s.share(Stage::PASS_SEE)
        + d * s.share(Stage::PASS_CHOOSE)
}

/// `true` when player `p` is pressed: his intensity passes [`THRESHOLD`].
#[inline]
pub fn pressed(players: &[Player], p: &Player, attack_x: f64) -> bool {
    intensity(players, p, attack_x) > THRESHOLD
}

/// `true` when opponent `o` presses player `p` on his own: his share passes [`THRESHOLD`].
#[inline]
pub fn presses(p: &Player, o: &Player, attack_x: f64) -> bool {
    o.team != p.team && o.active() && from(p, o, attack_x) > THRESHOLD
}

/// The difficulty of a pass `length` metres long, `lofted` or along the ground: the length
/// bands' [`PASS_DIFFICULTY`] between their middles, plus [`LOFT_DIFFICULTY`] when lofted.
#[inline]
pub fn pass_difficulty(length: f64, lofted: bool) -> f64 {
    let loft = if lofted { LOFT_DIFFICULTY } else { 0.0 };
    let t = PASS_DIFFICULTY;
    // The position among the band middles, 0 at 5 m to 3 at 35 m.
    let x = ((length - 5.0) / 10.0).clamp(0.0, 3.0);
    // The float-to-int cast truncates a value from 0 to 3.
    let i = (x as usize).min(2);
    let f = x - i as f64;
    (t[i] + (t[i + 1] - t[i]) * f + loft).min(1.0)
}

/// The difficulty of a reception: `speed_share` is the ball's speed over the fastest the
/// receiver controls (0 to 1); `facing_back` is 0 facing the goal he attacks and 1 facing
/// his own; `read` is the receiver's read of the press, which sets his body before the ball
/// arrives and so scales the facing term ([`NEUTRAL_READ`] leaves it as it stands).
#[inline]
pub fn reception_difficulty(speed_share: f64, facing_back: f64, read: f64) -> f64 {
    (RECEIVE_EASE
        + SPEED_DIFFICULTY * speed_share.clamp(0.0, 1.0)
        + BACK_DIFFICULTY * facing_back.clamp(0.0, 1.0) * unread(read))
    .min(1.0)
}

/// The penalty of a pressed action: intensity x difficulty x (1 - resistance), 0 to 1.
#[inline]
pub fn penalty(intensity: f64, difficulty: f64, resistance: f64) -> f64 {
    intensity.clamp(0.0, 1.0) * difficulty.clamp(0.0, 1.0) * (1.0 - resistance).clamp(0.0, 1.0)
}

/// The distance band of the nearest opponent `d` metres away, as the StatsBomb 360 check
/// reads it: 0 under 1 m, 1 under 2 m, 2 under 3 m, 3 under 5 m, 4 from 5 m.
pub fn band(d: f64) -> usize {
    if d < 1.0 {
        0
    } else if d < 2.0 {
        1
    } else if d < 3.0 {
        2
    } else if d < 5.0 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::DVec2;

    fn player(team: usize, x: f64, y: f64) -> Player {
        let mut p = crate::player::test_support::flat_player(team, 100, &Default::default());
        p.team = team;
        p.pos = DVec2::new(x, y);
        p.vel = DVec2::ZERO;
        p.facing = DVec2::new(1.0, 0.0);
        p
    }

    #[test]
    fn one_still_opponent_in_front_at_the_pressed_distance_sits_on_the_threshold() {
        let p = player(0, 0.0, 0.0);
        let o = player(1, PRESSED_M, 0.0);
        assert!((from(&p, &o, 1.0) - THRESHOLD).abs() < 1e-12);
        let nearer = player(1, PRESSED_M - 0.1, 0.0);
        assert!(presses(&p, &nearer, 1.0));
        assert!(pressed(&[p, nearer], &p, 1.0));
        let farther = player(1, PRESSED_M + 0.1, 0.0);
        assert!(!presses(&p, &farther, 1.0));
        assert!(!pressed(&[p, farther], &p, 1.0));
    }

    #[test]
    fn closer_more_faster_and_level_opponents_press_harder() {
        let p = player(0, 0.0, 0.0);
        let at = |x: f64, y: f64| player(1, x, y);
        let intensity = |all: &[Player], p: &Player| intensity(all, p, 1.0);
        // Closer.
        assert!(intensity(&[p, at(1.5, 0.0)], &p) > intensity(&[p, at(3.0, 0.0)], &p));
        assert_eq!(intensity(&[p, at(FAR_M + 1.0, 0.0)], &p), 0.0);
        // More.
        assert!(
            intensity(&[p, at(3.0, 0.0), at(0.0, 3.0)], &p) > intensity(&[p, at(3.0, 0.0)], &p)
        );
        // Faster: closing at 4 m/s against still.
        let mut closing = at(3.0, 0.0);
        closing.vel = DVec2::new(-4.0, 0.0);
        assert!(intensity(&[p, closing], &p) > intensity(&[p, at(3.0, 0.0)], &p));
        // Where he stands, against the direction the player attacks: level presses most, in
        // front less, behind least, as in the real flags; the same for a player facing back.
        let front = intensity(&[p, at(3.0, 0.0)], &p);
        let side = intensity(&[p, at(0.0, 3.0)], &p);
        let behind = intensity(&[p, at(-3.0, 0.0)], &p);
        assert!(behind < front && front < side);
        let mut back = p;
        back.facing = DVec2::new(-1.0, 0.0);
        assert_eq!(intensity(&[back, at(-3.0, 0.0)], &back), behind);
        // The other way up: for a player attacking -x, the same opponent is in front.
        assert_eq!(super::intensity(&[p, at(-3.0, 0.0)], &p, -1.0), front);
        // Never above 1.
        let crowd: Vec<Player> = std::iter::once(p)
            .chain((0..8).map(|k| {
                let mut o = at(-0.2 + 0.05 * f64::from(k), 0.1);
                o.vel = DVec2::new(5.0, 0.0);
                o
            }))
            .collect();
        let i = intensity(&crowd, &p);
        assert!(i <= 1.0 && i > 0.9);
    }

    #[test]
    fn a_player_who_reads_the_press_feels_a_blind_side_presser_less() {
        let p = player(0, 0.0, 0.0);
        let behind = player(1, -2.0, 0.0);
        let front = player(1, 2.0, 0.0);
        let all = [p, behind];
        // The observer and a player rated 10 feel the same.
        assert_eq!(felt(&all, &p, NEUTRAL_READ, 1.0), intensity(&all, &p, 1.0));
        assert!(felt(&all, &p, 0.9, 1.0) < felt(&all, &p, NEUTRAL_READ, 1.0));
        assert!(felt(&all, &p, 0.1, 1.0) > felt(&all, &p, NEUTRAL_READ, 1.0));
        // A presser straight in front is seen by everyone.
        let ahead = [p, front];
        assert_eq!(felt(&ahead, &p, 0.9, 1.0), felt(&ahead, &p, 0.1, 1.0));
    }

    #[test]
    fn short_and_lofted_passes_and_fast_balls_to_a_turned_receiver_are_harder() {
        // A short pass under pressure is the hardest, a long ball the easiest, as in the data.
        assert!(pass_difficulty(6.0, false) > pass_difficulty(15.0, false));
        assert!(pass_difficulty(6.0, false) > pass_difficulty(40.0, false));
        assert!(pass_difficulty(25.0, false) > pass_difficulty(40.0, false));
        assert_eq!(pass_difficulty(2.0, false), PASS_DIFFICULTY[0]);
        assert_eq!(pass_difficulty(60.0, false), PASS_DIFFICULTY[3]);
        assert!(pass_difficulty(20.0, true) > pass_difficulty(20.0, false));
        assert!(pass_difficulty(80.0, true) <= 1.0);
        let n = NEUTRAL_READ;
        assert!(reception_difficulty(0.9, 0.0, n) > reception_difficulty(0.2, 0.0, n));
        assert!(reception_difficulty(0.5, 1.0, n) > reception_difficulty(0.5, 0.0, n));
        // A receiver who reads the press sets his body: facing back costs him less.
        assert!(reception_difficulty(0.5, 1.0, 0.9) < reception_difficulty(0.5, 1.0, n));
        assert!(reception_difficulty(0.5, 1.0, 0.1) > reception_difficulty(0.5, 1.0, n));
    }

    #[test]
    fn the_choice_grades_from_half_the_threshold_to_the_threshold() {
        assert_eq!(grade(0.0), 0.0);
        assert_eq!(grade(GRADE_START * THRESHOLD), 0.0);
        assert!((grade(THRESHOLD) - 1.0).abs() < 1e-12);
        assert_eq!(grade(2.0 * THRESHOLD), 1.0);
        let mid = grade((1.0 + GRADE_START) / 2.0 * THRESHOLD);
        assert!((mid - 0.5).abs() < 1e-12);
    }

    #[test]
    fn the_penalty_is_intensity_times_difficulty_times_the_resistance_left() {
        assert!((penalty(0.5, 0.8, 0.25) - 0.3).abs() < 1e-12);
        assert!(penalty(0.5, 0.8, 0.75) < penalty(0.5, 0.8, 0.25));
        assert_eq!(penalty(0.0, 1.0, 0.0), 0.0);
    }
}

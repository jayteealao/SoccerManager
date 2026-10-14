//! The pressure model: one definition of how hard a player is pressed, which play and the
//! counts both read.
//!
//! - **Intensity** (0 to 1, per player, per moment). Each active opponent adds a share that
//!   falls with his distance (full at [`NEAR_M`], none from [`FAR_M`], straight between),
//!   rises with how fast he closes the gap (full at [`CLOSING_MPS`]) and with how far he
//!   stands from the player's facing (in front counts least, behind most). The shares combine
//!   as independent chances, `1 - Π(1 - share)`, so more opponents press harder and the
//!   intensity never passes 1.
//! - **Pressed.** A player is pressed when his intensity passes [`THRESHOLD`], the share of
//!   one still opponent standing in front of him at [`crate::contract::PRESSED_M`]; an
//!   opponent presses him (a pressure the counts see) when that opponent's own share passes
//!   it. The pressure counts, the press triggers' counts and the counter-press counts read
//!   this one rule.
//! - **Penalty** of a pressed action = intensity x action difficulty x (1 - player
//!   resistance). The difficulty of a pass grows with its length and when it is lofted; of a
//!   reception, with the ball's speed against what the receiver can control and when he faces
//!   his own goal. The resistance is the player's own stage share: passing and composure for
//!   a pass, the reception stage (first touch leading) for a reception. The tuning's
//!   `decision.pass_under_pressure` and `decision.touch_under_pressure` size the penalty so
//!   that the engine's pressed-against-free gaps match the StatsBomb open data.

use crate::contract::PRESSED_M;
use crate::player::Player;

/// The distance, in metres, within which an opponent's distance share is full.
pub const NEAR_M: f64 = 0.5;
/// The distance, in metres, from which an opponent adds no pressure.
pub const FAR_M: f64 = 5.0;
/// The closing speed, in metres per second, at which an opponent's closing share is full.
pub const CLOSING_MPS: f64 = 4.0;
/// The weight of a full closing share, against 1 for the distance alone.
pub const CLOSING_WEIGHT: f64 = 0.5;
/// The weight of an opponent straight behind the player, against 1 for the distance alone;
/// one at his side counts half of it, one in front none.
pub const BLIND_WEIGHT: f64 = 0.5;

/// The intensity at which a player is pressed: the share of one still opponent straight in
/// front of him at [`PRESSED_M`].
pub const THRESHOLD: f64 =
    (FAR_M - PRESSED_M) / (FAR_M - NEAR_M) / (1.0 + CLOSING_WEIGHT + BLIND_WEIGHT);

/// A pass's difficulty: [`PASS_EASE`] for a pass at the feet, growing straight to 1 at
/// [`LONG_PASS_M`]; a lofted pass (a chip or a cross) adds [`LOFT_DIFFICULTY`]; at most 1.
pub const PASS_EASE: f64 = 0.4;
pub const LONG_PASS_M: f64 = 40.0;
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

/// The pressure opponent `o` puts on player `p`, 0 to 1: his distance share, raised by his
/// closing speed and by his angle from the player's facing.
#[inline]
pub fn from(p: &Player, o: &Player) -> f64 {
    let gap = o.pos - p.pos;
    let d = gap.length();
    let near = distance_share(d);
    if near <= 0.0 {
        return 0.0;
    }
    // The direction from the player to the opponent; at the same spot, straight ahead.
    let toward = if d > 1e-9 { gap / d } else { p.facing };
    let closing = ((p.vel - o.vel).dot(toward) / CLOSING_MPS).clamp(0.0, 1.0);
    let blind = ((1.0 - p.facing.dot(toward)) / 2.0).clamp(0.0, 1.0);
    near * (1.0 + CLOSING_WEIGHT * closing + BLIND_WEIGHT * blind)
        / (1.0 + CLOSING_WEIGHT + BLIND_WEIGHT)
}

/// Player `p`'s pressure intensity, 0 to 1, from the active opponents among `players`.
pub fn intensity(players: &[Player], p: &Player) -> f64 {
    let mut free = 1.0;
    for o in players {
        if o.team != p.team && o.active() {
            free *= 1.0 - from(p, o);
        }
    }
    1.0 - free
}

/// `true` when player `p` is pressed: his intensity passes [`THRESHOLD`].
#[inline]
pub fn pressed(players: &[Player], p: &Player) -> bool {
    intensity(players, p) > THRESHOLD
}

/// `true` when opponent `o` presses player `p` on his own: his share passes [`THRESHOLD`].
#[inline]
pub fn presses(p: &Player, o: &Player) -> bool {
    o.team != p.team && o.active() && from(p, o) > THRESHOLD
}

/// The difficulty of a pass `length` metres long, `lofted` or along the ground.
#[inline]
pub fn pass_difficulty(length: f64, lofted: bool) -> f64 {
    let loft = if lofted { LOFT_DIFFICULTY } else { 0.0 };
    (PASS_EASE + (1.0 - PASS_EASE) * (length / LONG_PASS_M).clamp(0.0, 1.0) + loft).min(1.0)
}

/// The difficulty of a reception: `speed_share` is the ball's speed over the fastest the
/// receiver controls (0 to 1); `facing_back` is 0 facing the goal he attacks and 1 facing
/// his own.
#[inline]
pub fn reception_difficulty(speed_share: f64, facing_back: f64) -> f64 {
    (RECEIVE_EASE
        + SPEED_DIFFICULTY * speed_share.clamp(0.0, 1.0)
        + BACK_DIFFICULTY * facing_back.clamp(0.0, 1.0))
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
        assert!((from(&p, &o) - THRESHOLD).abs() < 1e-12);
        let nearer = player(1, PRESSED_M - 0.1, 0.0);
        assert!(presses(&p, &nearer));
        assert!(pressed(&[p, nearer], &p));
        let farther = player(1, PRESSED_M + 0.1, 0.0);
        assert!(!presses(&p, &farther));
        assert!(!pressed(&[p, farther], &p));
    }

    #[test]
    fn closer_more_faster_and_blind_side_opponents_press_harder() {
        let p = player(0, 0.0, 0.0);
        let at = |x: f64, y: f64| player(1, x, y);
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
        // Blind side: behind against in front, beside between.
        let front = intensity(&[p, at(3.0, 0.0)], &p);
        let side = intensity(&[p, at(0.0, 3.0)], &p);
        let behind = intensity(&[p, at(-3.0, 0.0)], &p);
        assert!(front < side && side < behind);
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
    fn longer_lofted_passes_and_fast_balls_to_a_turned_receiver_are_harder() {
        assert!(pass_difficulty(30.0, false) > pass_difficulty(8.0, false));
        assert!(pass_difficulty(20.0, true) > pass_difficulty(20.0, false));
        assert!(pass_difficulty(80.0, true) <= 1.0);
        assert!(reception_difficulty(0.9, 0.0) > reception_difficulty(0.2, 0.0));
        assert!(reception_difficulty(0.5, 1.0) > reception_difficulty(0.5, 0.0));
    }

    #[test]
    fn the_penalty_is_intensity_times_difficulty_times_the_resistance_left() {
        assert!((penalty(0.5, 0.8, 0.25) - 0.3).abs() < 1e-12);
        assert!(penalty(0.5, 0.8, 0.75) < penalty(0.5, 0.8, 0.25));
        assert_eq!(penalty(0.0, 1.0, 0.0), 0.0);
    }
}

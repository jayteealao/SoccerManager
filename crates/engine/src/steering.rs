//! Steering behaviors per Reynolds (1999): seek, arrive, separation.
//! Source: https://www.red3d.com/cwr/papers/1999/gdc99steer.pdf
//! A sent-off player stands still at its parking spot and pushes nobody.

use crate::math::{DVec2, clamp_len, sq_skip_limit, toward};
use crate::modules::{MatchView, ModuleCard, SteeringModule};
use crate::pitch;
use crate::player::Player;
use crate::tuning::Tuning;

/// Desired velocity that moves `pos` toward `target` at `max_speed` and slows inside the
/// braking distance, which is the larger of the arrive radius and `v²/(2a)`.
pub fn arrive(pos: DVec2, target: DVec2, max_speed: f64, max_accel: f64, radius: f64) -> DVec2 {
    let offset = target - pos;
    let dist = offset.length();
    if dist < 1e-6 {
        return DVec2::ZERO;
    }
    let braking = (max_speed * max_speed / (2.0 * max_accel)).max(radius);
    let speed = if dist < braking {
        max_speed * (dist / braking)
    } else {
        max_speed
    };
    offset * (speed / dist)
}

/// Desired velocity straight toward `target` at full speed.
pub fn seek(pos: DVec2, target: DVec2, max_speed: f64) -> DVec2 {
    toward(pos, target) * max_speed
}

/// Push away from every other player closer than the separation radius. A pair whose squared
/// distance is at or above `sq_skip_limit` of the radius is outside it and skips the square
/// root; every other pair runs the square-root form unchanged.
pub fn separation(players: &[Player], i: usize, t: &Tuning) -> DVec2 {
    let me = players[i].pos;
    let mut push = DVec2::ZERO;
    let skip = sq_skip_limit(t.separation_radius);
    for (j, other) in players.iter().enumerate() {
        if j == i || !other.active() {
            continue;
        }
        let d = me - other.pos;
        if d.length_squared() >= skip {
            continue;
        }
        let dist = d.length();
        if dist < t.separation_radius {
            let away = if dist > 1e-6 {
                d / dist
            } else {
                DVec2::new(1.0, 0.0)
            };
            push += away * (1.0 - dist / t.separation_radius);
        }
    }
    push * t.separation_strength
}

/// The velocity of player `i` after one tick of steering toward its target.
pub fn next_velocity(players: &[Player], i: usize, t: &Tuning) -> DVec2 {
    let p = &players[i];
    let max_speed = p.max_speed();
    let max_accel = p.max_accel();
    let desired =
        arrive(p.pos, p.target, max_speed, max_accel, t.arrive_radius) + separation(players, i, t);
    let change = clamp_len(desired - p.vel, max_accel * t.dt);
    clamp_len(p.vel + change, max_speed)
}

/// Steers every player one tick: new velocities are computed from the old state, then applied.
pub fn step_all(players: &mut [Player], scratch: &mut Vec<DVec2>, t: &Tuning) {
    scratch.clear();
    for i in 0..players.len() {
        scratch.push(if players[i].active() {
            next_velocity(players, i, t)
        } else {
            DVec2::ZERO
        });
    }
    apply_velocities(players, scratch, t);
}

/// Moves every player on the pitch one tick at its new velocity `velocities[i]`, and turns it
/// to face where it goes.
pub fn apply_velocities(players: &mut [Player], velocities: &[DVec2], t: &Tuning) {
    for (p, &v) in players.iter_mut().zip(velocities.iter()) {
        if !p.active() {
            continue;
        }
        p.vel = v;
        p.pos = pitch::clamp(p.pos + v * t.dt, 0.2);
        if v.length_squared() > 1e-6 {
            p.facing = v.normalize();
        }
    }
}

/// Steering version 1: arrive plus separation, as [`next_velocity`].
pub struct SteeringV1;

impl SteeringModule for SteeringV1 {
    fn next_velocity(&self, view: &MatchView<'_>, i: usize) -> DVec2 {
        next_velocity(view.players(), i, view.tuning())
    }
}

pub const STEERING_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Steers each player toward its target: arrive with braking, plus separation from close players, within its speed and acceleration.",
    inputs: "Every player's position, velocity, target, activity, and effective pace, and the engine tuning.",
    outputs: "The next velocity of one player.",
    tuning: &[
        "arrive_radius",
        "separation_radius",
        "separation_strength",
        "dt",
    ],
    calibration: "none: movement maths, no realism band",
    keys: &[],
};

/// Pushes apart every pair of players closer than the minimum distance, in index order. A
/// pair whose squared distance is at or above `sq_skip_limit` of the minimum distance skips
/// the square root; every other pair runs the square-root form unchanged.
pub fn resolve_overlaps(players: &mut [Player], t: &Tuning) {
    let n = players.len();
    let skip = sq_skip_limit(t.min_player_distance);
    for i in 0..n {
        if !players[i].active() {
            continue;
        }
        for j in (i + 1)..n {
            if !players[j].active() {
                continue;
            }
            let d = players[j].pos - players[i].pos;
            if d.length_squared() >= skip {
                continue;
            }
            let dist = d.length();
            if dist < t.min_player_distance {
                let axis = if dist > 1e-9 {
                    d / dist
                } else {
                    DVec2::new(1.0, 0.0)
                };
                let push = axis * ((t.min_player_distance - dist) / 2.0);
                players[i].pos = pitch::clamp(players[i].pos - push, 0.2);
                players[j].pos = pitch::clamp(players[j].pos + push, 0.2);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::test_support::flat_player;

    fn player(id: usize, pos: DVec2, target: DVec2) -> Player {
        let mut p = flat_player(id, 50, &Tuning::default());
        p.pos = pos;
        p.target = target;
        p
    }

    #[test]
    fn arrive_converges_without_overshoot() {
        let t = Tuning::default();
        let mut players = vec![player(0, DVec2::ZERO, DVec2::new(20.0, 0.0))];
        let mut scratch = Vec::new();
        let mut max_x: f64 = 0.0;
        for _ in 0..400 {
            step_all(&mut players, &mut scratch, &t);
            max_x = max_x.max(players[0].pos.x);
        }
        assert!(
            (players[0].pos.x - 20.0).abs() < 0.5,
            "ended at {}",
            players[0].pos.x
        );
        assert!(max_x < 20.5, "overshot to {max_x}");
    }

    #[test]
    fn separation_pushes_overlapping_players_apart() {
        let t = Tuning::default();
        let mut players = vec![
            player(0, DVec2::new(0.0, 0.0), DVec2::new(0.0, 0.0)),
            player(1, DVec2::new(0.2, 0.0), DVec2::new(0.2, 0.0)),
        ];
        let mut scratch = Vec::new();
        for _ in 0..100 {
            step_all(&mut players, &mut scratch, &t);
            resolve_overlaps(&mut players, &t);
        }
        let dist = (players[0].pos - players[1].pos).length();
        assert!(dist >= t.min_player_distance, "distance {dist}");
    }

    /// Today's separation before the squared-distance skip, for the boundary tests.
    fn separation_before(players: &[Player], i: usize, t: &Tuning) -> DVec2 {
        let me = players[i].pos;
        let mut push = DVec2::ZERO;
        for (j, other) in players.iter().enumerate() {
            if j == i || !other.active() {
                continue;
            }
            let d = me - other.pos;
            let dist = d.length();
            if dist < t.separation_radius {
                let away = if dist > 1e-6 {
                    d / dist
                } else {
                    DVec2::new(1.0, 0.0)
                };
                push += away * (1.0 - dist / t.separation_radius);
            }
        }
        push * t.separation_strength
    }

    /// Today's overlap resolution before the squared-distance skip, for the boundary tests.
    fn resolve_overlaps_before(players: &mut [Player], t: &Tuning) {
        let n = players.len();
        for i in 0..n {
            if !players[i].active() {
                continue;
            }
            for j in (i + 1)..n {
                if !players[j].active() {
                    continue;
                }
                let d = players[j].pos - players[i].pos;
                let dist = d.length();
                if dist < t.min_player_distance {
                    let axis = if dist > 1e-9 {
                        d / dist
                    } else {
                        DVec2::new(1.0, 0.0)
                    };
                    let push = axis * ((t.min_player_distance - dist) / 2.0);
                    players[i].pos = pitch::clamp(players[i].pos - push, 0.2);
                    players[j].pos = pitch::clamp(players[j].pos + push, 0.2);
                }
            }
        }
    }

    /// An offset whose squared length is exactly `d2`: an x near the square root and a small
    /// y that makes up the rest, or `None` when the search finds none.
    pub(super) fn offset_for(d2: f64) -> Option<DVec2> {
        let mut x = d2.sqrt();
        for _ in 0..8 {
            x = x.next_down();
        }
        for _ in 0..16 {
            x = x.next_up();
            let rest = d2 - x * x;
            if rest < 0.0 {
                continue;
            }
            let mut y = rest.sqrt();
            for _ in 0..4 {
                y = y.next_down();
            }
            for _ in 0..8 {
                let v = DVec2::new(x, y);
                if v.length_squared().to_bits() == d2.to_bits() {
                    return Some(v);
                }
                y = y.next_up();
            }
        }
        None
    }

    fn bits(v: DVec2) -> (u64, u64) {
        (v.x.to_bits(), v.y.to_bits())
    }

    /// The squared distances of a boundary test at radius `r`: the skip limit, the two values
    /// below it (the one-ulp band and the square), the value above, and one more below.
    fn boundary(r: f64) -> [f64; 5] {
        let skip = sq_skip_limit(r);
        [
            skip,
            skip.next_down(),
            skip.next_down().next_down(),
            skip.next_up(),
            (r * r).next_down(),
        ]
    }

    #[test]
    fn separation_at_the_skip_limit_matches_the_square_root_form() {
        let t = Tuning::default();
        assert_eq!(t.separation_radius, 1.5, "the shipped radius");
        let mut tested = 0;
        for d2 in boundary(t.separation_radius) {
            let Some(v) = offset_for(d2) else {
                continue;
            };
            let players = vec![
                player(0, DVec2::ZERO, DVec2::ZERO),
                player(1, v, DVec2::ZERO),
            ];
            let d = players[0].pos - players[1].pos;
            assert_eq!(d.length_squared().to_bits(), d2.to_bits());
            for i in 0..2 {
                assert_eq!(
                    bits(separation(&players, i, &t)),
                    bits(separation_before(&players, i, &t)),
                    "d2 = {d2:e}"
                );
            }
            tested += 1;
        }
        assert_eq!(tested, 5, "every boundary squared distance is built");
        // A coincident pair runs the unchanged zero-distance axis.
        let players = vec![
            player(0, DVec2::new(3.0, 3.0), DVec2::ZERO),
            player(1, DVec2::new(3.0, 3.0), DVec2::ZERO),
        ];
        assert_eq!(
            bits(separation(&players, 0, &t)),
            bits(separation_before(&players, 0, &t))
        );
    }

    /// Both overlap resolutions on the same pair; returns both players' positions as bits.
    fn overlap_pair(a: DVec2, b: DVec2, t: &Tuning) -> [[(u64, u64); 2]; 2] {
        let mut new = vec![player(0, a, a), player(1, b, b)];
        let mut old = new.clone();
        resolve_overlaps(&mut new, t);
        resolve_overlaps_before(&mut old, t);
        [
            [bits(new[0].pos), bits(new[1].pos)],
            [bits(old[0].pos), bits(old[1].pos)],
        ]
    }

    #[test]
    fn overlap_at_the_skip_limit_matches_the_square_root_form() {
        let t = Tuning::default();
        let r = t.min_player_distance;
        assert_eq!(r, 0.4, "the shipped minimum distance");
        // At 0.4 a plain squared compare picks another branch one ulp below the square.
        let below = (r * r).next_down();
        assert_ne!(below < r * r, below.sqrt() < r);
        let mut tested = 0;
        for d2 in boundary(r) {
            let Some(v) = offset_for(d2) else {
                continue;
            };
            assert_eq!(v.length_squared().to_bits(), d2.to_bits());
            let [new, old] = overlap_pair(DVec2::ZERO, v, &t);
            assert_eq!(new, old, "d2 = {d2:e}");
            tested += 1;
        }
        assert_eq!(tested, 5, "every boundary squared distance is built");
        // A coincident pair runs the unchanged zero-distance axis.
        let at = DVec2::new(3.0, 3.0);
        let [new, old] = overlap_pair(at, at, &t);
        assert_eq!(new, old);
        assert_ne!(new[0], new[1], "the coincident pair is pushed apart");
    }
}

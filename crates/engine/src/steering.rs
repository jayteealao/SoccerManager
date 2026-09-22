//! Steering behaviors per Reynolds (1999): seek, arrive, separation.
//! Source: https://www.red3d.com/cwr/papers/1999/gdc99steer.pdf

use crate::math::{DVec2, clamp_len, toward};
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

/// Push away from every other player closer than the separation radius.
pub fn separation(players: &[Player], i: usize, t: &Tuning) -> DVec2 {
    let me = players[i].pos;
    let mut push = DVec2::ZERO;
    for (j, other) in players.iter().enumerate() {
        if j == i {
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
        scratch.push(next_velocity(players, i, t));
    }
    for (p, &v) in players.iter_mut().zip(scratch.iter()) {
        p.vel = v;
        p.pos = pitch::clamp(p.pos + v * t.dt, 0.2);
        if v.length_squared() > 1e-6 {
            p.facing = v.normalize();
        }
    }
}

/// Pushes apart every pair of players closer than the minimum distance, in index order.
pub fn resolve_overlaps(players: &mut [Player], t: &Tuning) {
    let n = players.len();
    for i in 0..n {
        for j in (i + 1)..n {
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
}

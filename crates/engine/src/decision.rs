//! The minimal possession decision: the ball carrier passes to the best-placed teammate,
//! shoots, or dribbles; two opponents press; every other player holds its formation anchor.
//! Every agent decides every tick (product-owner choice; `Tuning::decision_interval_ticks`).
//! A sent-off player takes no part in any decision. While the ball is dead, the referee sets
//! every target instead, and a restart kick comes from `restart_pass`.

use crate::data::rules::StoppageKind;
use crate::math::{DVec2, segment_distance, toward};
use crate::pitch;
use crate::sim::Simulation;
use crate::tuning::Tuning;

/// A kick the carrier decided on this tick.
#[derive(Debug, Clone, Copy)]
pub enum Kick {
    Pass { dir: DVec2, speed: f64, loft: f64 },
    Shot { dir: DVec2, speed: f64, loft: f64 },
}

impl Simulation {
    /// Sets every player's target for this tick and returns the carrier's kick, if any.
    pub(crate) fn decide(&mut self) -> Option<Kick> {
        let ball_xy = self.ball.xy();
        let n = self.players.len();
        match self.carrier {
            Some(c) => {
                let team = self.players[c].team;
                // Teammates hold their anchors; the two nearest opponents press.
                let mut pressers = [usize::MAX; 2];
                let mut press_dist = [f64::INFINITY; 2];
                for i in 0..n {
                    let p = self.players[i];
                    if !p.active() {
                        continue;
                    }
                    let anchor = self.teams[p.team].anchor(p.slot, ball_xy, &self.config.tuning);
                    self.players[i].target = anchor;
                    if p.team != team && p.slot != 0 {
                        let d = (p.pos - ball_xy).length();
                        if d < self.config.tuning.press_distance {
                            if d < press_dist[0] {
                                pressers[1] = pressers[0];
                                press_dist[1] = press_dist[0];
                                pressers[0] = i;
                                press_dist[0] = d;
                            } else if d < press_dist[1] {
                                pressers[1] = i;
                                press_dist[1] = d;
                            }
                        }
                    }
                }
                for &i in &pressers {
                    if i != usize::MAX {
                        self.players[i].target = ball_xy;
                    }
                }
                self.decide_carrier(c)
            }
            None => {
                // The nearest player of each team chases; a goalkeeper chases a ball near its goal.
                let predicted = pitch::clamp(
                    ball_xy + DVec2::new(self.ball.vel.x, self.ball.vel.y) * 0.3,
                    0.3,
                );
                let mut nearest = [usize::MAX; 2];
                let mut nearest_dist = [f64::INFINITY; 2];
                for i in 0..n {
                    let p = self.players[i];
                    if !p.active() {
                        continue;
                    }
                    let anchor = self.teams[p.team].anchor(p.slot, ball_xy, &self.config.tuning);
                    self.players[i].target = anchor;
                    let d = (p.pos - ball_xy).length();
                    if p.slot != 0 && d < nearest_dist[p.team] {
                        nearest_dist[p.team] = d;
                        nearest[p.team] = i;
                    }
                }
                for (team, &i) in nearest.iter().enumerate() {
                    if i != usize::MAX {
                        self.players[i].target = predicted;
                    }
                    let gk = team * crate::team::PLAYERS_PER_TEAM;
                    if self.players[gk].active() && (self.players[gk].pos - ball_xy).length() < 16.0
                    {
                        self.players[gk].target = predicted;
                    }
                }
                None
            }
        }
    }

    pub(crate) fn decide_carrier(&mut self, c: usize) -> Option<Kick> {
        let t = &self.config.tuning;
        let carrier = self.players[c];
        let team = carrier.team;
        let goal = self.teams[team].target_goal();
        let attack = DVec2::new(self.teams[team].attack_x, 0.0);
        let to_goal = goal - carrier.pos;
        let goal_dist = to_goal.length();

        // Pressure and space around the carrier.
        let mut nearest_opp = f64::INFINITY;
        let mut nearest_opp_pos = carrier.pos;
        let mut space_ahead: f64 = 10.0;
        for p in &self.players {
            if p.team == team || !p.active() {
                continue;
            }
            let d = p.pos - carrier.pos;
            let dist = d.length();
            if dist < nearest_opp {
                nearest_opp = dist;
                nearest_opp_pos = p.pos;
            }
            if dist < 10.0 && dist > 1e-6 && d.dot(attack) / dist > 0.5 {
                space_ahead = space_ahead.min(dist);
            }
        }

        // Shot.
        if goal_dist < t.shot_range {
            let lane = self
                .players
                .iter()
                .filter(|p| p.team != team && p.slot != 0 && p.active())
                .map(|p| segment_distance(p.pos, carrier.pos, goal))
                .fold(f64::INFINITY, f64::min);
            let want = lane > 2.5 || (nearest_opp < 2.0 && lane > 1.0) || self.rng.chance(0.02);
            if want {
                let aim = DVec2::new(goal.x, self.rng.range_f64(-3.3, 3.3));
                let dir = rotate(
                    toward(carrier.pos, aim),
                    self.rng.range_f64(-t.shot_noise, t.shot_noise),
                );
                return Some(Kick::Shot {
                    dir,
                    speed: t.shot_speed,
                    loft: self.rng.range_f64(0.0, 2.5),
                });
            }
        }

        // Pass options.
        let mut best: Option<(f64, usize)> = None;
        for j in 0..self.players.len() {
            let mate = &self.players[j];
            if mate.team != team || j == c || !mate.active() {
                continue;
            }
            let d = (mate.pos - carrier.pos).length();
            if !(4.0..=45.0).contains(&d) {
                continue;
            }
            let mut lane: f64 = 6.0;
            let mut receiver_space: f64 = 8.0;
            for opp in &self.players {
                if opp.team == team || !opp.active() {
                    continue;
                }
                lane = lane.min(segment_distance(opp.pos, carrier.pos, mate.pos));
                receiver_space = receiver_space.min((opp.pos - mate.pos).length());
            }
            if lane < 1.5 {
                continue;
            }
            let progress = ((mate.pos - carrier.pos).dot(attack) / 40.0).clamp(-1.0, 1.0);
            let score = 1.2 * progress + 0.5 * (lane / 6.0) + 0.4 * (receiver_space / 8.0)
                - 0.3 * (d / 45.0)
                + self.rng.range_f64(-0.1, 0.1);
            if best.is_none_or(|(s, _)| score > s) {
                best = Some((score, j));
            }
        }
        let mut dribble = 0.8 * (space_ahead / 10.0) + 0.3 + self.rng.range_f64(-0.1, 0.1);
        if nearest_opp < 2.5 {
            dribble -= 0.5;
        }
        let held = self.tick.saturating_sub(self.control_since);
        if held < 10 {
            dribble += 0.6;
        }

        let keeper = carrier.slot == 0;
        if let Some((score, j)) = best {
            if keeper || score > dribble {
                let mate_pos = self.players[j].pos;
                let d = (mate_pos - carrier.pos).length();
                let skill = carrier.derived.passing / 100.0;
                let noise = t.aim_noise * (1.5 - skill);
                let dir = rotate(
                    toward(carrier.pos, mate_pos),
                    self.rng.range_f64(-noise, noise),
                );
                let loft = if d > 25.0 { 3.0 + d * 0.08 } else { 0.0 };
                let speed = kick_speed(d, loft, t);
                return Some(Kick::Pass { dir, speed, loft });
            }
        }

        if keeper {
            // Clear long toward the far half.
            let dir = rotate(attack, self.rng.range_f64(-0.6, 0.6));
            return Some(Kick::Pass {
                dir,
                speed: kick_speed(CLEARANCE_DISTANCE, CLEARANCE_LOFT, t),
                loft: CLEARANCE_LOFT,
            });
        }

        // Dribble toward goal, drifting away from the nearest opponent.
        let dir_goal = if goal_dist > 1e-6 {
            to_goal / goal_dist
        } else {
            attack
        };
        let perp = DVec2::new(-dir_goal.y, dir_goal.x);
        let side = if (nearest_opp_pos - carrier.pos).dot(perp) > 0.0 {
            -1.0
        } else {
            1.0
        };
        let target = carrier.pos + dir_goal * 8.0 + perp * (4.0 * side);
        self.players[c].target = pitch::clamp(target, 1.0);
        None
    }
}

impl Simulation {
    /// The kick that takes a restart: a short lofted throw to the nearest team-mate, a cross
    /// toward the penalty mark from a corner, or a pass to an open team-mate. With no
    /// team-mate in range, the ball goes forward along the pitch.
    pub(crate) fn restart_pass(&mut self, taker: usize, kind: StoppageKind) -> Kick {
        let t = &self.config.tuning;
        let me = self.players[taker];
        let attack = DVec2::new(self.teams[me.team].attack_x, 0.0);
        let cross_at = (kind == StoppageKind::Corner).then(|| pitch::penalty_spot(attack.x));
        let range = if kind == StoppageKind::ThrowIn {
            3.0..=THROW_RANGE
        } else {
            4.0..=45.0
        };
        let mut best: Option<(f64, DVec2)> = None;
        for (j, mate) in self.players.iter().enumerate() {
            if j == taker || mate.team != me.team || !mate.active() {
                continue;
            }
            let d = (mate.pos - me.pos).length();
            if !range.contains(&d) {
                continue;
            }
            let score = match cross_at {
                Some(mark) => -(mate.pos - mark).length(),
                None => -d + 0.2 * (mate.pos - me.pos).dot(attack),
            };
            if best.is_none_or(|(s, _)| score > s) {
                best = Some((score, mate.pos));
            }
        }
        let to = best.map_or_else(|| pitch::clamp(me.pos + attack * 10.0, 4.0), |(_, at)| at);
        let d = (to - me.pos).length();
        let dir = match toward(me.pos, to) {
            v if v == DVec2::ZERO => attack,
            v => v,
        };
        let loft = match kind {
            StoppageKind::ThrowIn => THROW_LOFT,
            StoppageKind::Corner => 6.0 + 0.1 * d,
            _ if d > 25.0 => 3.0 + d * 0.08,
            _ => 0.0,
        };
        Kick::Pass {
            dir,
            speed: kick_speed(d, loft, t),
            loft,
        }
    }
}

/// The farthest team-mate a throw-in looks for, in metres, and the throw's vertical speed.
const THROW_RANGE: f64 = 15.0;
const THROW_LOFT: f64 = 2.0;
/// How far a goalkeeper's clearance travels before it stops, in metres, and its vertical
/// speed.
const CLEARANCE_DISTANCE: f64 = 55.0;
const CLEARANCE_LOFT: f64 = 6.0;

/// The ground speed of a kick that reaches a team-mate `d` metres away at the pass arrival
/// speed. A ground pass decelerates by friction the whole way. A lofted pass flies for
/// `2 * loft / g` seconds with no friction, so it needs less speed: the flight covers
/// `speed * T` and the roll `speed^2 / (2 f)`, which puts the ball at the team-mate with the
/// arrival speed left.
fn kick_speed(d: f64, loft: f64, t: &Tuning) -> f64 {
    let f = t.ground_friction;
    let a = t.pass_arrival_speed;
    let speed = if loft <= 0.0 {
        (2.0 * f * d).sqrt() + a
    } else {
        let flight = 2.0 * loft / t.gravity;
        let reach = d + a * a / (2.0 * f);
        f * (-flight + (flight * flight + 2.0 * reach / f).sqrt())
    };
    speed.min(t.pass_max_speed)
}

/// Rotates a unit vector by `angle` radians.
fn rotate(v: DVec2, angle: f64) -> DVec2 {
    let (s, c) = angle.sin_cos();
    DVec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_config;

    #[test]
    fn a_lofted_pass_reaches_its_target_slow_enough_to_control() {
        let t = crate::tuning::Tuning::default();
        for d in [30.0, 40.0, 45.0] {
            let loft = 3.0 + d * 0.08;
            let mut ball = crate::ball::Ball::at(DVec2::ZERO);
            ball.kick(DVec2::X, kick_speed(d, loft, &t), loft, &t);
            let mut at_target = None;
            for _ in 0..1000 {
                ball.integrate(&t);
                if at_target.is_none() && ball.pos.x >= d {
                    at_target = Some(ball.speed());
                }
            }
            let speed = at_target.unwrap_or_else(|| panic!("a {d} m pass fell short"));
            assert!(
                speed <= t.control_speed,
                "a {d} m pass arrived at {speed} m/s"
            );
            // The bounce after landing carries an uncontrolled ball on by up to a third.
            assert!(
                ball.pos.x < d * 1.34,
                "a {d} m pass rolled on to {}",
                ball.pos.x
            );
        }
    }

    #[test]
    fn an_open_teammate_draws_a_pass_or_a_shot() {
        let mut sim = Simulation::new(shipped_config(7, 1).unwrap()).unwrap();
        // Give the ball to the home striker (slot 9) deep in the away half with a teammate open.
        let striker = 9;
        sim.players[striker].pos = DVec2::new(20.0, 0.0);
        sim.players[10].pos = DVec2::new(30.0, 6.0);
        for p in sim.players.iter_mut().filter(|p| p.team == 1) {
            p.pos = DVec2::new(-40.0, p.pos.y);
        }
        sim.carrier = Some(striker);
        sim.control_since = 0;
        sim.tick = 100;
        let mut kicked = false;
        for _ in 0..50 {
            if sim.decide().is_some() {
                kicked = true;
                break;
            }
        }
        assert!(kicked, "the carrier never passed or shot");
    }
}

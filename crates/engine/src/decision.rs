//! The minimal possession decision: the ball carrier passes to the best-placed teammate,
//! shoots, or dribbles; two opponents press; every other player holds its formation anchor.
//! Every agent decides every tick (product-owner choice; `Tuning::decision_interval_ticks`).

use crate::math::{DVec2, segment_distance, toward};
use crate::pitch;
use crate::sim::Simulation;

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
                    if (self.players[gk].pos - ball_xy).length() < 16.0 {
                        self.players[gk].target = predicted;
                    }
                }
                None
            }
        }
    }

    fn decide_carrier(&mut self, c: usize) -> Option<Kick> {
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
            if p.team == team {
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
                .filter(|p| p.team != team && p.slot != 0)
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
            if mate.team != team || j == c {
                continue;
            }
            let d = (mate.pos - carrier.pos).length();
            if !(4.0..=45.0).contains(&d) {
                continue;
            }
            let mut lane: f64 = 6.0;
            let mut receiver_space: f64 = 8.0;
            for opp in &self.players {
                if opp.team == team {
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
                let skill = f64::from(carrier.attributes.passing) / 100.0;
                let noise = t.aim_noise * (1.5 - skill);
                let dir = rotate(
                    toward(carrier.pos, mate_pos),
                    self.rng.range_f64(-noise, noise),
                );
                let speed = ((2.0 * t.ground_friction * d).sqrt() + t.pass_arrival_speed)
                    .min(t.pass_max_speed);
                let loft = if d > 25.0 { 3.0 + d * 0.08 } else { 0.0 };
                return Some(Kick::Pass { dir, speed, loft });
            }
        }

        if keeper {
            // Clear long toward the far half.
            let dir = rotate(attack, self.rng.range_f64(-0.6, 0.6));
            return Some(Kick::Pass {
                dir,
                speed: t.pass_max_speed,
                loft: 6.0,
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

/// Rotates a unit vector by `angle` radians.
fn rotate(v: DVec2, angle: f64) -> DVec2 {
    let (s, c) = angle.sin_cos();
    DVec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::MatchConfig;

    #[test]
    fn an_open_teammate_draws_a_pass_or_a_shot() {
        let mut sim = Simulation::new(MatchConfig::new(7, 1).unwrap()).unwrap();
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

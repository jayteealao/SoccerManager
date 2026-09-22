//! The tick-stream validator: pitch bounds, player separation, ball speed, and formation
//! anchor tolerance in open play. The anchor rule flags a player that is far from the ball,
//! far from its anchor, and closing on neither for the grace period (idle drift).

use crate::math::DVec2;
use crate::pitch;
use crate::record::{PLAYER_COUNT, TickRecord};
use crate::team::{PLAYERS_PER_TEAM, Team};
use crate::tuning::Tuning;

/// Minimum distance between two players.
pub const MIN_SEPARATION: f64 = 0.1;
/// Maximum ball speed in metres per second.
pub const MAX_BALL_SPEED: f64 = 40.0;

/// One rule violation.
#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    pub tick: u32,
    pub rule: &'static str,
    pub player: Option<usize>,
    pub value: f64,
}

/// Validates a stream of records against the tuning and the team formations.
pub struct Validator {
    tuning: Tuning,
    teams: [Team; 2],
}

impl Validator {
    pub fn new(tuning: Tuning, teams: [Team; 2]) -> Self {
        Self { tuning, teams }
    }

    /// Returns every violation in `records`, in tick order.
    pub fn check(&self, records: &[TickRecord]) -> Vec<Violation> {
        let mut out = Vec::new();
        let mut far_ticks = [0u32; PLAYER_COUNT];
        let mut prev_dist = [[f64::INFINITY; 2]; PLAYER_COUNT];
        let mut prev_ball: Option<DVec2> = None;
        for r in records {
            let ball = DVec2::new(f64::from(r.ball[0]), f64::from(r.ball[1]));
            let pos: Vec<DVec2> = r
                .players
                .iter()
                .map(|p| DVec2::new(f64::from(p[0]), f64::from(p[1])))
                .collect();

            for (i, p) in pos.iter().enumerate() {
                if !pitch::contains(*p) {
                    out.push(Violation {
                        tick: r.tick,
                        rule: "in_bounds",
                        player: Some(i),
                        value: p.x.abs().max(p.y.abs()),
                    });
                }
            }
            for i in 0..PLAYER_COUNT {
                for j in (i + 1)..PLAYER_COUNT {
                    let d = (pos[i] - pos[j]).length();
                    if d < MIN_SEPARATION {
                        out.push(Violation {
                            tick: r.tick,
                            rule: "separation",
                            player: Some(i),
                            value: d,
                        });
                    }
                }
            }
            if let Some(prev) = prev_ball
                && !r.restart
            {
                let speed = (ball - prev).length() / self.tuning.dt;
                if speed > MAX_BALL_SPEED + 0.5 {
                    out.push(Violation {
                        tick: r.tick,
                        rule: "ball_speed",
                        player: None,
                        value: speed,
                    });
                }
            }
            prev_ball = Some(ball);

            for (i, p) in pos.iter().enumerate() {
                let team = i / PLAYERS_PER_TEAM;
                let slot = i % PLAYERS_PER_TEAM;
                let anchor = self.teams[team].anchor(slot, ball, &self.tuning);
                let d_ball = (*p - ball).length();
                let d_anchor = (*p - anchor).length();
                let closing = d_ball < prev_dist[i][0] - 1e-3 || d_anchor < prev_dist[i][1] - 1e-3;
                prev_dist[i] = [d_ball, d_anchor];
                let drifting = d_ball > self.tuning.anchor_ball_distance
                    && d_anchor > self.tuning.anchor_tolerance
                    && !closing;
                far_ticks[i] = if drifting { far_ticks[i] + 1 } else { 0 };
                if far_ticks[i] >= self.tuning.anchor_grace_ticks {
                    out.push(Violation {
                        tick: r.tick,
                        rule: "anchor_tolerance",
                        player: Some(i),
                        value: d_anchor,
                    });
                }
            }
        }
        for v in &out {
            tracing::warn!(signal = "validate.violation", tick = v.tick, rule = v.rule, player = ?v.player, value = v.value);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team::test_support::bare;

    fn record(tick: u32) -> TickRecord {
        let teams = [bare(0), bare(1)];
        let mut players = [[0.0f32; 2]; PLAYER_COUNT];
        for (i, p) in players.iter_mut().enumerate() {
            let base = teams[i / PLAYERS_PER_TEAM].slot_base(i % PLAYERS_PER_TEAM);
            *p = [base.x as f32, base.y as f32];
        }
        TickRecord {
            tick,
            ball: [0.0, 0.0, 0.0],
            players,
            restart: false,
        }
    }

    #[test]
    fn a_hand_built_overlap_is_one_violation() {
        let v = Validator::new(Tuning::default(), [bare(0), bare(1)]);
        let mut r = record(10);
        r.players[1] = r.players[0];
        let out = v.check(&[record(9), r]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rule, "separation");
        assert_eq!(out[0].tick, 10);
    }

    #[test]
    fn a_ball_faster_than_the_cap_is_a_violation() {
        let v = Validator::new(Tuning::default(), [bare(0), bare(1)]);
        let mut r = record(2);
        r.ball = [0.9, 0.0, 0.0];
        let out = v.check(&[record(1), r]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rule, "ball_speed");
    }

    #[test]
    fn a_restart_tick_exempts_the_jump_and_the_same_jump_without_the_flag_does_not() {
        let v = Validator::new(Tuning::default(), [bare(0), bare(1)]);
        let mut jump = record(2);
        jump.ball = [20.0, 0.0, 0.0];

        let mut restart = jump;
        restart.restart = true;
        assert!(v.check(&[record(1), restart]).is_empty());

        let out = v.check(&[record(1), jump]);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].rule, "ball_speed");
    }
}

//! The tick-stream validator: pitch bounds, player separation, ball speed, formation anchor
//! tolerance in open play, and the ball's place at a restart. The anchor rule flags a player
//! that is far from the ball, far from its anchor, and closing on neither for the grace
//! period (idle drift). A sent-off player at its parking spot beside the pitch is exempt
//! from every player rule. The anchors follow the team shapes in force at each tick: the
//! teams change ends at half-time, and a line closes up after a sending-off.

use crate::math::DVec2;
use crate::pitch;
use crate::record::{PLAYER_COUNT, TickRecord};
use crate::sim::EngineEvent;
use crate::team::{PLAYERS_PER_TEAM, Team};
use crate::tuning::Tuning;

/// Minimum distance between two players.
pub const MIN_SEPARATION: f64 = 0.1;
/// Maximum ball speed in metres per second.
pub const MAX_BALL_SPEED: f64 = 40.0;
/// How far the ball may lie from the restart spot a restart event names.
pub const RESTART_SPOT_TOLERANCE: f64 = 0.5;

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
    /// The team shapes in force from each tick on, in tick order.
    timeline: Vec<(u32, [Team; 2])>,
    /// The restart spots the events name, in tick order.
    spots: Vec<(u32, DVec2)>,
}

impl Validator {
    /// A validator for teams whose shape never changes.
    pub fn new(tuning: Tuning, teams: [Team; 2]) -> Self {
        Self {
            tuning,
            timeline: vec![(0, teams)],
            spots: Vec::new(),
        }
    }

    /// A validator for a played match: the team shapes over time and the events, whose
    /// restart spots the ball must lie on at each restart tick.
    pub fn for_match(
        tuning: Tuning,
        timeline: &[(u32, [Team; 2])],
        events: &[EngineEvent],
    ) -> Self {
        let mut spots: Vec<(u32, DVec2)> = events
            .iter()
            .filter_map(|e| e.spot.map(|s| (e.tick, s)))
            .collect();
        spots.sort_by_key(|(tick, _)| *tick);
        Self {
            tuning,
            timeline: timeline.to_vec(),
            spots,
        }
    }

    /// Returns every violation in `records`, in tick order.
    pub fn check(&self, records: &[TickRecord]) -> Vec<Violation> {
        let mut out = Vec::new();
        let mut far_ticks = [0u32; PLAYER_COUNT];
        let mut prev_dist = [[f64::INFINITY; 2]; PLAYER_COUNT];
        let mut prev_ball: Option<DVec2> = None;
        let mut shape = 0;
        for r in records {
            while shape + 1 < self.timeline.len() && self.timeline[shape + 1].0 <= r.tick {
                shape += 1;
            }
            let teams = &self.timeline[shape].1;
            let ball = DVec2::new(f64::from(r.ball[0]), f64::from(r.ball[1]));
            let pos: Vec<DVec2> = r
                .players
                .iter()
                .map(|p| DVec2::new(f64::from(p[0]), f64::from(p[1])))
                .collect();
            let parked: Vec<bool> = pos.iter().map(|p| pitch::is_parking_spot(*p)).collect();

            for (i, p) in pos.iter().enumerate() {
                if !parked[i] && !pitch::contains(*p) {
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
                    if parked[i] || parked[j] {
                        continue;
                    }
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
            if r.restart
                && let Ok(at) = self.spots.binary_search_by_key(&r.tick, |(tick, _)| *tick)
            {
                let off = (ball - self.spots[at].1).length();
                if off > RESTART_SPOT_TOLERANCE {
                    out.push(Violation {
                        tick: r.tick,
                        rule: "restart_spot",
                        player: None,
                        value: off,
                    });
                }
            }

            for (i, p) in pos.iter().enumerate() {
                if parked[i] {
                    far_ticks[i] = 0;
                    continue;
                }
                let team = i / PLAYERS_PER_TEAM;
                let slot = i % PLAYERS_PER_TEAM;
                let anchor = teams[team].anchor(slot, ball, &self.tuning);
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

    #[test]
    fn a_parked_player_breaks_no_rule() {
        let v = Validator::new(Tuning::default(), [bare(0), bare(1)]);
        let mut r = record(2);
        let spot = pitch::parking_spot(0, 3);
        r.players[3] = [spot.x as f32, spot.y as f32];
        assert!(v.check(&[record(1), r]).is_empty());
    }

    #[test]
    fn a_ball_away_from_the_named_restart_spot_is_a_violation() {
        let config = crate::data::test_support::shipped_config(1, 90).unwrap();
        let sim = crate::sim::Simulation::new(config).unwrap();
        let mut event = sim.event(crate::sim::EngineEventKind::ThrowIn, Some(0));
        event.tick = 2;
        event.spot = Some(DVec2::new(10.0, 34.0));
        let v = Validator::for_match(Tuning::default(), &[(0, [bare(0), bare(1)])], &[event]);
        let mut restart = record(2);
        restart.restart = true;
        restart.ball = [10.0, 34.0, 0.0];
        assert!(v.check(&[record(1), restart]).is_empty());
        restart.ball = [11.0, 34.0, 0.0];
        let out = v.check(&[record(1), restart]);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].rule, "restart_spot");
    }
}

//! The walk-on: before the first tick of each half (match start, the half-time break and each
//! extra-time break) the players walk from the tunnel on the near touchline to the places the
//! half starts from. It is not match time: no tick plays it, so the clock, ball in play and
//! every statistic leave it out. The engine places the kick-off at once, as before, and hands
//! the walk-on to the sink ([`crate::record::TickSink::on_walk_on`]) before the half's first
//! record, so a watcher sees the walk instead of a jump.
//!
//! Each player walks in a straight line at the walk limit of the pace tuning and stands once
//! he arrives. The two teams come out of the tunnel at the halfway line in a file each, on
//! their own side of it, in slot order.

use crate::math::DVec2;
use crate::modules::ROSTER;
use crate::player::Player;
use crate::team::Team;

/// Metres between two players of one file at the tunnel.
pub const FILE_GAP_M: f64 = 1.5;

/// One player's walk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkPath {
    /// The player's roster index.
    pub player: usize,
    /// Where he comes out of the tunnel.
    pub from: DVec2,
    /// His place on the half's first tick.
    pub to: DVec2,
}

/// The walk-on before one half.
#[derive(Debug, Clone, PartialEq)]
pub struct WalkOn {
    /// The half it opens: 0 the first, 1 the second, 2 and 3 the extra-time halves.
    pub period: u32,
    /// The tick of the half's first record, which the walk-on ends on.
    pub first_tick: u32,
    /// The walking speed, in metres per second.
    pub speed: f64,
    /// Frames of one tick's length the walk takes: the longest walk at `speed`.
    pub ticks: u32,
    /// Every player on the pitch, in roster order.
    pub paths: Vec<WalkPath>,
}

impl WalkOn {
    /// The walk-on to the places `players` stand on for the half `period` that starts with
    /// tick `first_tick`, at `speed` metres per second with ticks of `dt` seconds. A player off
    /// the pitch (parked) does not walk on.
    pub fn build(
        period: u32,
        first_tick: u32,
        speed: f64,
        dt: f64,
        players: &[Player],
        teams: &[Team; 2],
    ) -> Self {
        let mut in_file = [0usize; 2];
        let mut paths = Vec::with_capacity(players.len());
        for (i, p) in players.iter().enumerate().take(ROSTER) {
            if !p.active() {
                continue;
            }
            let team = &teams[p.team];
            let k = in_file[p.team];
            in_file[p.team] += 1;
            let from = DVec2::new(
                -team.attack_x * (1.0 + FILE_GAP_M * k as f64),
                -team.pitch().half_width(),
            );
            paths.push(WalkPath {
                player: i,
                from,
                to: p.pos,
            });
        }
        let step = speed * dt;
        let longest = paths
            .iter()
            .map(|w| (w.to - w.from).length())
            .fold(0.0, f64::max);
        // A walk is at most a pitch diagonal, so the cast fits.
        let ticks = if step > 0.0 {
            (longest / step).ceil() as u32
        } else {
            0
        };
        Self {
            period,
            first_tick,
            speed,
            ticks,
            paths,
        }
    }

    /// Where each player stands `frame` ticks into the walk-on (0 at the tunnel, `ticks` at his
    /// place), with ticks of `dt` seconds, by roster index; `None` for a player who does not
    /// walk on.
    pub fn at(&self, frame: u32, dt: f64) -> [Option<DVec2>; ROSTER] {
        let mut out = [None; ROSTER];
        let walked = f64::from(frame) * self.speed * dt;
        for w in &self.paths {
            let way = w.to - w.from;
            let length = way.length();
            out[w.player] = Some(if walked >= length || length <= 0.0 {
                w.to
            } else {
                w.from + way * (walked / length)
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_config;
    use crate::sim::Simulation;

    #[test]
    fn every_player_starts_at_the_tunnel_and_ends_at_his_place() {
        let sim = Simulation::new(shipped_config(1, 90).unwrap()).unwrap();
        let dt = sim.tuning().dt;
        let walk = WalkOn::build(0, 1, 2.0, dt, &sim.players, &sim.teams);
        assert_eq!(walk.paths.len(), 22);
        let start = walk.at(0, dt);
        let end = walk.at(walk.ticks, dt);
        let half_width = sim.config.pitch().half_width();
        for w in &walk.paths {
            let p = &sim.players[w.player];
            assert_eq!(start[w.player], Some(w.from));
            assert_eq!(
                w.from.y, -half_width,
                "out of the tunnel on the near touchline"
            );
            assert!(
                w.from.x * sim.teams[p.team].attack_x < 0.0,
                "each file on its own side of halfway"
            );
            assert_eq!(end[w.player], Some(p.pos));
        }
    }

    #[test]
    fn no_frame_moves_a_player_faster_than_the_walk() {
        let sim = Simulation::new(shipped_config(1, 90).unwrap()).unwrap();
        let dt = sim.tuning().dt;
        let walk = WalkOn::build(0, 1, 2.0, dt, &sim.players, &sim.teams);
        for frame in 1..=walk.ticks {
            let (a, b) = (walk.at(frame - 1, dt), walk.at(frame, dt));
            for i in 0..ROSTER {
                if let (Some(a), Some(b)) = (a[i], b[i]) {
                    assert!(
                        (b - a).length() <= 2.0 * dt + 1e-9,
                        "frame {frame} player {i}"
                    );
                }
            }
        }
    }
}

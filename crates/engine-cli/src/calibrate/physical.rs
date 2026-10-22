//! Each player's running in a calibration match: a tick sink that reads every player's
//! position and speed once per step and keeps, per player per match, the metres covered,
//! the ball-in-play time in each speed zone of the real player file, the peak speed, and the
//! ticks on the pitch. Only a `measures` build has it.

use engine::{EngineError, Simulation, TickRecord, TickSink};

use crate::measure_rows::{PLAYER_SLOTS, PlayerRow};

/// The upper bounds of the walk, jog, run and high-intensity zones in m/s; sprint is 7 and
/// over (the zones of `idsse_players.csv`).
const ZONE_TOPS: [f64; 4] = [2.0, 4.0, 5.5, 7.0];

/// The zone of a speed in m/s, 0 (walk) to 4 (sprint).
fn zone(speed: f64) -> usize {
    ZONE_TOPS.iter().take_while(|&&top| speed >= top).count()
}

/// The running of every player of a match, by side and slot in order of first appearance.
#[derive(Debug, Clone)]
pub struct Physical {
    pub players: [[PlayerRow; PLAYER_SLOTS]; 2],
    /// Slots taken, per side.
    used: [usize; 2],
    /// Players of a side beyond the slots.
    pub overflow: [u32; 2],
    /// Each roster place's player (by squad index) and his slot, once seen: none for a
    /// player beyond the slots.
    seen: Vec<Option<(usize, Option<usize>)>>,
    /// The match's ball-in-play ticks after the last step.
    last_live: u32,
}

impl Default for Physical {
    fn default() -> Self {
        Self {
            players: [[PlayerRow::default(); PLAYER_SLOTS]; 2],
            used: [0; 2],
            overflow: [0; 2],
            seen: Vec::new(),
            last_live: 0,
        }
    }
}

impl Physical {
    /// The slot of player `squad` of `team` at roster place `id`: a new player takes the next
    /// free slot of his side, and none when the side has used them all.
    fn slot(&mut self, id: usize, team: usize, squad: usize) -> Option<usize> {
        if self.seen.len() <= id {
            self.seen.resize(id + 1, None);
        }
        if let Some((s, slot)) = self.seen[id]
            && s == squad
        {
            return slot;
        }
        // A substitute brings his own squad index to the roster place he takes; a player
        // seen before at another place keeps his slot.
        let used = self.used[team];
        let known = (0..used).find(|&k| usize::from(self.players[team][k].squad) == squad);
        let slot = match known {
            Some(k) => Some(k),
            None if used < PLAYER_SLOTS => {
                self.used[team] += 1;
                self.players[team][used].squad = u8::try_from(squad).unwrap_or(u8::MAX);
                Some(used)
            }
            None => {
                self.overflow[team] += 1;
                None
            }
        };
        self.seen[id] = Some((squad, slot));
        slot
    }

    /// One step: every active player's tick on the pitch, distance, zone and peak speed.
    pub fn step(&mut self, live: bool, players: &[engine::player::Player]) {
        let dt = 1.0 / f64::from(engine::TICKS_PER_SECOND);
        for p in players.iter().filter(|p| p.active()) {
            let Some(slot) = self.slot(p.id, p.team, p.squad) else {
                continue;
            };
            let row = &mut self.players[p.team][slot];
            let speed = p.vel.length();
            row.ticks_on += 1;
            row.distance_m += speed * dt;
            if live {
                row.zone_ticks[zone(speed)] += 1;
            }
            if speed > row.peak_mps {
                row.peak_mps = speed;
            }
        }
    }
}

impl TickSink for Physical {
    fn on_tick(&mut self, _record: &TickRecord) -> Result<(), EngineError> {
        Ok(())
    }

    fn reads_records(&self) -> bool {
        false
    }

    fn on_step(&mut self, sim: &Simulation) -> Result<(), EngineError> {
        let live_ticks = sim.summary().live_ticks;
        let live = live_ticks > self.last_live;
        self.last_live = live_ticks;
        self.step(live, sim.players());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A player of the default clubs at roster place `id`, moving at `speed`.
    fn player(id: usize, team: usize, squad: usize, speed: f64) -> engine::player::Player {
        static BASE: std::sync::OnceLock<engine::player::Player> = std::sync::OnceLock::new();
        let mut p = *BASE.get_or_init(|| {
            let dir = engine::ContentDir::at(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
            );
            let content = engine::Content::load(&dir).unwrap();
            let teams = [engine::data::TEAM_A_FILE, engine::data::TEAM_B_FILE]
                .map(|f| content.load_team(&dir, &dir.path(f)).unwrap().value);
            let config = engine::MatchConfig::new(1, 1, &content, [&teams[0], &teams[1]]).unwrap();
            Simulation::new(config).unwrap().players()[0]
        });
        p.id = id;
        p.team = team;
        p.squad = squad;
        p.vel.x = speed;
        p.vel.y = 0.0;
        p
    }

    #[test]
    fn the_zones_follow_the_real_file() {
        assert_eq!(zone(0.0), 0);
        assert_eq!(zone(1.99), 0);
        assert_eq!(zone(2.0), 1);
        assert_eq!(zone(5.49), 2);
        assert_eq!(zone(5.5), 3);
        assert_eq!(zone(7.0), 4);
        assert_eq!(zone(9.5), 4);
    }

    /// A run of known speeds gives the expected distance, zones and peak; a dead-ball tick
    /// adds distance and no zone tick.
    #[test]
    fn known_speeds_give_distance_zones_and_peak() {
        let mut ph = Physical::default();
        for (speed, live) in [(1.0, true), (3.0, true), (8.0, true), (5.0, false)] {
            ph.step(live, &[player(4, 0, 7, speed)]);
        }
        let row = ph.players[0][0];
        assert_eq!(row.squad, 7);
        assert_eq!(row.ticks_on, 4);
        assert!(
            (row.distance_m - 17.0 / 50.0).abs() < 1e-12,
            "{}",
            row.distance_m
        );
        assert_eq!(row.zone_ticks, [1, 1, 0, 0, 1]);
        assert!((row.peak_mps - 8.0).abs() < 1e-12);
        assert_eq!(ph.overflow, [0, 0]);
    }

    /// A substitute takes the roster place of the player he replaces and gets his own slot
    /// and minutes; a side's seventeenth player adds to the overflow.
    #[test]
    fn a_substitute_gets_his_own_slot_and_a_seventeenth_player_overflows() {
        let mut ph = Physical::default();
        ph.step(true, &[player(3, 1, 2, 4.0)]);
        ph.step(true, &[player(3, 1, 2, 4.0)]);
        ph.step(true, &[player(3, 1, 15, 6.0)]);
        assert_eq!(ph.players[1][0].squad, 2);
        assert_eq!(ph.players[1][0].ticks_on, 2);
        assert_eq!(ph.players[1][1].squad, 15);
        assert_eq!(ph.players[1][1].ticks_on, 1);
        assert_eq!(ph.players[1][1].zone_ticks, [0, 0, 0, 1, 0]);
        let mut full = Physical::default();
        for squad in 0..PLAYER_SLOTS + 1 {
            full.step(true, &[player(squad, 0, squad, 1.0)]);
        }
        full.step(true, &[player(PLAYER_SLOTS, 0, PLAYER_SLOTS, 1.0)]);
        assert_eq!(full.overflow, [1, 0], "a player, not a tick");
        assert!(full.players[0].iter().all(|r| r.ticks_on == 1));
    }
}

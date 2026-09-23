//! Shared helpers: the shipped content, the seed-42 match, and the fixture packs.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use engine::data::{TEAM_A_FILE, TEAM_B_FILE};
use engine::{Content, ContentDir, EngineEvent, MatchConfig, Simulation, TickRecord, VecSink};
use script::LoadedPack;

pub const SEED: u64 = 42;

/// A seed-42 match of `minutes` on the shipped content and default teams.
pub fn config(minutes: u32) -> MatchConfig {
    let dir = ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"));
    let content = Content::load(&dir).expect("the shipped content loads");
    let a = content
        .load_team(&dir, &dir.path(TEAM_A_FILE))
        .unwrap()
        .value;
    let b = content
        .load_team(&dir, &dir.path(TEAM_B_FILE))
        .unwrap()
        .value;
    MatchConfig::new(SEED, minutes, &content, [&a, &b]).unwrap()
}

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

pub fn pack(name: &str) -> LoadedPack {
    LoadedPack::load(&fixture(name)).unwrap_or_else(|e| panic!("{name} loads: {e}"))
}

/// A whole match of `minutes`, with the pack's hooks when one is given. Returns the match,
/// its tick records, and its events.
pub fn play(
    minutes: u32,
    pack: Option<&LoadedPack>,
) -> (Simulation, Vec<TickRecord>, Vec<EngineEvent>) {
    let mut config = config(minutes);
    if let Some(pack) = pack {
        config.fold_pack_hash(pack.sha());
    }
    let mut sim = Simulation::new(config).unwrap();
    if let Some(pack) = pack {
        sim.set_plugins(pack.plugins());
    }
    let mut sink = VecSink::default();
    sim.run(&mut sink).unwrap();
    let events = sim.take_events();
    (sim, sink.records, events)
}

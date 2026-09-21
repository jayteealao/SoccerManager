#![allow(dead_code)]

//! Shared fixture: the seed-42 configuration with the built-in teams, and temp paths.

use std::path::PathBuf;

use engine::MatchConfig;

pub const SEED: u64 = 42;

pub fn full_match() -> MatchConfig {
    MatchConfig::new(SEED, 90).unwrap()
}

pub fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("engine-test-{}-{name}.ticks", std::process::id()))
}

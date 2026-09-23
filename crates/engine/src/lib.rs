//! Headless football match engine.
//!
//! The engine simulates a match at a fixed 20 ms timestep (50 ticks per simulated second),
//! emits one [`TickRecord`] per tick to a [`TickSink`], and reproduces a match byte for byte
//! for the same seed, the same build, and the same machine.

pub mod ai;
pub mod ball;
pub mod commentary;
pub mod data;
pub mod decision;
pub mod error;
pub mod fatigue;
pub mod flags;
pub mod math;
pub mod observe;
pub mod pitch;
pub mod player;
pub mod plugin;
pub mod record;
pub mod rng;
pub mod rules;
#[cfg(feature = "scenario")]
pub mod scenario;
pub mod sim;
pub mod snapshot;
pub mod steering;
pub mod tactics;
pub mod team;
pub mod tuning;
pub mod validate;

pub use ai::{AiCode, Manager};
pub use commentary::{Commentary, Commentator};
pub use data::{Content, ContentDir, StoppageKind};
pub use error::EngineError;
pub use fatigue::InjurySource;
pub use flags::{ActiveFlags, CODE_FLAGS, FlagSetting, FlagState, FlagStates};
pub use plugin::{PLUGIN_API_VERSION, Plugins};
pub use record::{
    FanoutSink, FileSink, NullSink, TickHeader, TickRecord, TickSink, VecSink, read_ticks,
};
pub use rules::clock::max_ticks;
pub use rules::fouls::Card;
pub use rules::{DeadBall, Stoppage};
pub use sim::{
    DecidedBy, EngineEvent, EngineEventKind, EventDetail, MatchConfig, Simulation, Summary,
};
pub use snapshot::{Snapshot, SnapshotSink};
pub use tactics::change::{Change, ChangeId, ChangeKind, RejectReason, SubLedger, Unapplied};
pub use tactics::{RoleDuty, Tactics, TacticsPatch};
pub use tuning::Tuning;
pub use validate::{Validator, Violation};

/// Ticks per simulated second.
pub const TICKS_PER_SECOND: u32 = 50;

/// Number of ticks in `minutes` of simulated play.
pub fn ticks_for_minutes(minutes: u32) -> u32 {
    minutes * 60 * TICKS_PER_SECOND
}

/// The git build hash embedded at build time, or `unknown`.
pub fn build_hash() -> &'static str {
    env!("ENGINE_BUILD_HASH")
}

/// The engine crate version.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

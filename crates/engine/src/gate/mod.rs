//! The replay gate: 22 fixed matches, each hashed after every tick over the full match state
//! (state inventory version 1, [`inventory`]), with a checkpoint every 1,000 ticks and at the
//! last tick. A match whose checkpoints differ from the golden file's names the first window
//! of ticks in which its state changed.
//!
//! The running hash is SHA-256 (`sha2`, installed). A checkpoint is the digest of a clone of
//! the running hasher, so taking one does not end the run.

pub mod golden;
mod inventory;

#[cfg(feature = "scenario")]
pub mod fault;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ai::Manager;
use crate::canon::Writer;
use crate::data::Content;
use crate::data::team::TeamFile;
use crate::plugin::Plugins;
use crate::sim::{DecidedBy, EngineEventKind, EventDetail, MatchConfig, Simulation};
use crate::tactics::TacticsPatch;
use crate::tactics::change::{Change, ChangeKind};

/// The version of the gate's file layout and report.
pub const GATE_SCHEMA: u16 = 1;
/// The version of the state inventory the hashes are computed over.
pub const INVENTORY_VERSION: u16 = 1;
/// Ticks between two checkpoints.
pub const CHECKPOINT_EVERY: u32 = 1_000;

/// The 20 seeded fixtures, in gate order: five familiar seeds, the two edge seeds, and a
/// spread.
pub const SEEDS: [u64; 20] = [
    42,
    1,
    7,
    99,
    2026,
    0,
    u64::MAX,
    3,
    11,
    23,
    57,
    123,
    314,
    777,
    1000,
    4242,
    9001,
    31337,
    65535,
    1000003,
];

/// The seed of the knockout fixture: the lowest seed from 1 to 10,000 whose knockout match
/// with the sample script pack ends in a penalty shoot-out on the engine the golden file was
/// first written with. A later result change keeps it and regenerates the hashes.
pub const KNOCKOUT_SEED: u64 = 2;

/// The script pack the knockout fixture runs.
pub const KNOCKOUT_PACK: &str = "sample";

/// What a fixture plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureKind {
    /// A 90-minute match between the default teams, both AI-managed.
    Seed,
    /// Both managers human, with changes queued at fixed ticks.
    Change,
    /// A knockout match with a script pack.
    Knockout,
}

/// A change the change fixture queues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannedWhat {
    /// The player in lineup `slot` off, the bench player at `bench` on.
    Substitution { slot: usize, bench: usize },
    /// The team's mentality set to this index of the tactics file.
    Mentality(u8),
}

/// One change of the change fixture: queued before the step after tick `tick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedChange {
    pub tick: u32,
    pub team: usize,
    pub change: PlannedWhat,
}

/// One gate match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixture {
    pub id: String,
    pub kind: FixtureKind,
    pub seed: u64,
    pub minutes: u32,
    pub knockout: bool,
    /// The script pack the match runs, by pack id.
    pub pack: Option<&'static str>,
    pub changes: Vec<PlannedChange>,
}

impl Fixture {
    /// A seeded fixture.
    pub fn seed(seed: u64) -> Self {
        Self {
            id: format!("seed-{seed}"),
            kind: FixtureKind::Seed,
            seed,
            minutes: 90,
            knockout: false,
            pack: None,
            changes: Vec::new(),
        }
    }

    /// The change fixture: seed 42, both managers human, a substitution for the home team
    /// at tick 60,000 and a mentality change for the away team at tick 90,000.
    pub fn change() -> Self {
        Self {
            id: "change".into(),
            kind: FixtureKind::Change,
            seed: 42,
            minutes: 90,
            knockout: false,
            pack: None,
            changes: vec![
                PlannedChange {
                    tick: 60_000,
                    team: 0,
                    change: PlannedWhat::Substitution { slot: 9, bench: 0 },
                },
                PlannedChange {
                    tick: 90_000,
                    team: 1,
                    // "attacking" in the shipped tactics file.
                    change: PlannedWhat::Mentality(4),
                },
            ],
        }
    }

    /// The knockout fixture on `seed`: a knockout match with the sample script pack.
    pub fn knockout(seed: u64) -> Self {
        Self {
            id: "knockout".into(),
            kind: FixtureKind::Knockout,
            seed,
            minutes: 90,
            knockout: true,
            pack: Some(KNOCKOUT_PACK),
            changes: Vec::new(),
        }
    }
}

/// The 22 gate fixtures in gate order: the 20 seeds, then `change`, then `knockout`.
pub fn fixtures() -> Vec<Fixture> {
    let mut all: Vec<Fixture> = SEEDS.iter().map(|&s| Fixture::seed(s)).collect();
    all.push(Fixture::change());
    all.push(Fixture::knockout(KNOCKOUT_SEED));
    all
}

/// A script pack a fixture can run: its id, its SHA-256 (folded into the content hash), and
/// a maker of fresh hooks for one match.
pub struct PackInputs<'a> {
    pub id: &'a str,
    pub sha: [u8; 32],
    pub hooks: &'a dyn Fn() -> Plugins,
}

/// Everything loaded once for a gate run.
pub struct Inputs<'a> {
    pub content: &'a Content,
    pub teams: [&'a TeamFile; 2],
    /// The pack the knockout fixture needs; the other fixtures ignore it.
    pub pack: Option<PackInputs<'a>>,
}

/// Why a gate match could not be played or hashed.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GateError {
    #[error("fixture {fixture}: the match cannot be built: {reason}")]
    Setup { fixture: String, reason: String },
    #[error("fixture {fixture} needs the {pack} script pack, which is not loaded")]
    NeedsPack { fixture: String, pack: String },
    #[error("fixture {fixture}: the hashed value {field} is not a finite number after tick {tick}")]
    NonFinite {
        fixture: String,
        tick: u32,
        field: String,
    },
}

/// One checkpoint: the running hash after `tick`, as 64 lowercase hex characters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub tick: u32,
    pub hash: String,
}

/// The hashes of one match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchHashes {
    pub id: String,
    pub ticks: u32,
    pub final_hash: String,
    pub checkpoints: Vec<Checkpoint>,
}

/// What the match record of a gate match shows, for the special fixtures.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Facts {
    pub extra_time: bool,
    pub shootout: bool,
    pub decided_by: Option<DecidedBy>,
    pub goals: [u32; 2],
    /// Substitutions applied, both teams.
    pub substitutions: u32,
    /// Tactics changes applied, both teams.
    pub tactics_changes: u32,
    /// Script hook calls, and those stopped on budget or time.
    pub script_calls: u32,
    pub script_aborts: u32,
}

/// A played gate match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Played {
    pub hashes: MatchHashes,
    pub facts: Facts,
    /// `true` when the run stopped at the first checkpoint that differs from the one it was
    /// compared against.
    pub stopped: bool,
}

/// How a played match compares with its golden hashes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Same,
    /// The first checkpoint that differs is at tick `to`; the state first differs after tick
    /// `from`, the previous checkpoint (0 for the first).
    Differs {
        from: u32,
        to: u32,
    },
    /// The match lasted another number of ticks.
    TickCount {
        expected: u32,
        actual: u32,
    },
}

/// Plays `fixture` to the end and returns its hashes.
pub fn play_fixture(fixture: &Fixture, inputs: &Inputs<'_>) -> Result<Played, GateError> {
    run(fixture, inputs, &Probe::default())
}

/// What a run is compared against and, in engine tests, the fault it carries.
#[derive(Default)]
pub(crate) struct Probe<'a> {
    /// Stop at the first checkpoint that differs from these.
    pub against: Option<&'a MatchHashes>,
    #[cfg(feature = "scenario")]
    pub fault: Option<fault::Fault>,
}

pub(crate) fn run(
    fixture: &Fixture,
    inputs: &Inputs<'_>,
    probe: &Probe<'_>,
) -> Result<Played, GateError> {
    let mut sim = build(fixture, inputs)?;
    let mut hasher = Sha256::new();
    let mut w = Writer::default();
    inventory::header(&mut w, fixture, &sim);
    hasher.update(w.bytes());
    let mut checkpoints = Vec::new();
    let mut facts = Facts::default();
    let mut stopped = false;
    #[cfg(feature = "scenario")]
    let mut restore: Option<fault::Restore> = None;
    while !sim.is_over() {
        let now = sim.tick;
        for planned in fixture.changes.iter().filter(|c| c.tick == now) {
            queue(&mut sim, planned);
        }
        sim.step();
        if sim.is_over() {
            sim.finish();
        }
        let events = sim.take_events();
        for e in &events {
            match (e.kind, e.detail) {
                (EngineEventKind::Substitution, _) => facts.substitutions += 1,
                (
                    EngineEventKind::ChangeApplied,
                    Some(EventDetail::Change {
                        kind: ChangeKind::Tactics,
                        ..
                    }),
                ) => facts.tactics_changes += 1,
                _ => {}
            }
        }
        w.clear();
        inventory::state(&mut w, &sim, &events);
        if let Some(field) = w.fault() {
            return Err(GateError::NonFinite {
                fixture: fixture.id.clone(),
                tick: sim.tick,
                field: field.to_string(),
            });
        }
        hasher.update(w.bytes());
        #[cfg(feature = "scenario")]
        {
            if let Some(r) = restore.take() {
                r.apply(&mut sim);
            }
            if let Some(f) = probe.fault.as_ref().filter(|f| f.at_tick == sim.tick) {
                restore = f.apply(&mut sim);
            }
        }
        if sim.tick.is_multiple_of(CHECKPOINT_EVERY) || sim.is_over() {
            let checkpoint = Checkpoint {
                tick: sim.tick,
                hash: hex(&hasher.clone().finalize()),
            };
            let differs = probe.against.is_some_and(|golden| {
                golden.checkpoints.get(checkpoints.len()) != Some(&checkpoint)
            });
            checkpoints.push(checkpoint);
            if differs {
                stopped = true;
                break;
            }
        }
    }
    let summary = sim.summary();
    facts.extra_time = summary.extra_time;
    facts.shootout = summary.shootout.is_some();
    facts.decided_by = summary.decided_by;
    facts.goals = summary.goals;
    facts.script_calls = sim.plugins().stats.calls;
    facts.script_aborts = sim.plugins().stats.aborts;
    let last = checkpoints
        .last()
        .cloned()
        .expect("a match plays at least one tick");
    Ok(Played {
        hashes: MatchHashes {
            id: fixture.id.clone(),
            ticks: last.tick,
            final_hash: last.hash,
            checkpoints,
        },
        facts,
        stopped,
    })
}

fn build(fixture: &Fixture, inputs: &Inputs<'_>) -> Result<Simulation, GateError> {
    let setup = |reason: String| GateError::Setup {
        fixture: fixture.id.clone(),
        reason,
    };
    let mut config = MatchConfig::new(fixture.seed, fixture.minutes, inputs.content, inputs.teams)
        .map_err(|e| setup(e.to_string()))?;
    if fixture.knockout {
        config = config.with_knockout();
    }
    if fixture.kind == FixtureKind::Change {
        config = config
            .with_manager(0, Manager::Human)
            .with_manager(1, Manager::Human);
    }
    let pack = match fixture.pack {
        None => None,
        Some(id) => match &inputs.pack {
            Some(pack) if pack.id == id => Some(pack),
            _ => {
                return Err(GateError::NeedsPack {
                    fixture: fixture.id.clone(),
                    pack: id.to_string(),
                });
            }
        },
    };
    if let Some(pack) = pack {
        config.fold_pack_hash(&pack.sha);
    }
    let mut sim = Simulation::new(config).map_err(|e| setup(e.to_string()))?;
    if let Some(pack) = pack {
        sim.set_plugins((pack.hooks)());
    }
    Ok(sim)
}

fn queue(sim: &mut Simulation, planned: &PlannedChange) {
    let change = match planned.change {
        PlannedWhat::Substitution { slot, bench } => {
            let team = &sim.teams[planned.team];
            Change::Substitution {
                off: team.lineup[slot],
                on: team.bench.get(bench).copied().unwrap_or(usize::MAX),
            }
        }
        PlannedWhat::Mentality(m) => Change::Tactics(TacticsPatch::mentality(m)),
    };
    sim.queue_change(planned.team, change);
}

/// Compares a played match with its golden hashes. A checkpoint pair at different ticks
/// means the match lasted another number of ticks.
pub fn compare(golden: &MatchHashes, played: &MatchHashes) -> Verdict {
    let pairs = golden.checkpoints.iter().zip(&played.checkpoints);
    for (i, (g, p)) in pairs.enumerate() {
        if g.tick != p.tick {
            break;
        }
        if g.hash != p.hash {
            let from = if i == 0 {
                0
            } else {
                golden.checkpoints[i - 1].tick
            };
            return Verdict::Differs { from, to: g.tick };
        }
    }
    if golden.ticks != played.ticks || golden.checkpoints != played.checkpoints {
        return Verdict::TickCount {
            expected: golden.ticks,
            actual: played.ticks,
        };
    }
    Verdict::Same
}

/// The report line of one match: its id, `match` or `differs`, the tick count, the first 12
/// characters of the final hash, the special-match facts, and on a difference the window.
pub fn report_line(fixture: &Fixture, played: &Played, verdict: Verdict) -> String {
    let word = if verdict == Verdict::Same {
        "match"
    } else {
        "differs"
    };
    let mut line = format!(
        "{:<26} {:<7} {:>7} ticks  {}",
        fixture.id,
        word,
        played.hashes.ticks,
        &played.hashes.final_hash[..12]
    );
    let facts = facts_text(fixture, &played.facts);
    if !facts.is_empty() {
        line.push_str("  ");
        line.push_str(&facts);
    }
    match verdict {
        Verdict::Same => {}
        Verdict::Differs { from, to } => line.push_str(&format!(
            "\n  {} differs: the state first differs between tick {from} and tick {to}",
            fixture.id
        )),
        Verdict::TickCount { expected, actual } => line.push_str(&format!(
            "\n  {} differs: the golden file has {expected} ticks, this run played {actual}",
            fixture.id
        )),
    }
    line
}

/// The special-match facts in words: extra time and the shoot-out for the knockout
/// fixture, applied changes for the change fixture. Empty for a seeded fixture.
pub fn facts_text(fixture: &Fixture, facts: &Facts) -> String {
    let yes_no = |b: bool| if b { "yes" } else { "no" };
    match fixture.kind {
        FixtureKind::Seed => String::new(),
        FixtureKind::Knockout => format!(
            "extra time: {}, shoot-out: {}",
            yes_no(facts.extra_time),
            yes_no(facts.shootout)
        ),
        FixtureKind::Change => format!(
            "substitutions applied: {}, tactics changes applied: {}",
            facts.substitutions, facts.tactics_changes
        ),
    }
}

/// Lowercase hex of `bytes`.
pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

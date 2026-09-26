//! The golden file (`gate/golden.json`): the gate's versions, the fixture list, and one hash
//! set per machine (`<os>-<arch>`), each with every match's tick count, final hash, and
//! checkpoints. The strict load refuses a file that does not fit this build's gate and names
//! the fault; the bootstrap writes the first file and refuses when one exists.
//!
//! Seeds are decimal strings and hashes lowercase hex strings, because 18446744073709551615
//! is above the exact integer range of many JSON readers.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    CHECKPOINT_EVERY, Fixture, FixtureKind, GATE_SCHEMA, INVENTORY_VERSION, MatchHashes,
    PlannedChange,
};

/// The Rust toolchain the hashes are computed with (`rust-toolchain.toml`).
pub const TOOLCHAIN: &str = "1.92.0";
/// The golden file's path from the repository root.
pub const DEFAULT_PATH: &str = "gate/golden.json";

/// The golden file, fields in file order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenFile {
    pub gate_schema: u16,
    pub inventory_version: u16,
    pub checkpoint_every: u32,
    pub toolchain: String,
    pub bootstrap: Bootstrap,
    pub fixtures: Vec<FixtureEntry>,
    /// Machine key to the matches in fixture order.
    pub hash_sets: BTreeMap<String, Vec<MatchHashes>>,
}

/// Who wrote the first file, and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub engine_version: String,
    pub build: String,
    /// UTC time, `YYYY-MM-DDTHH:MM:SSZ`.
    pub utc: String,
    pub machine: String,
}

/// One fixture as the file lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureEntry {
    pub id: String,
    pub kind: FixtureKind,
    pub seed: String,
    pub minutes: u32,
    pub knockout: bool,
    pub pack: Option<String>,
    pub changes: Vec<PlannedChange>,
}

impl From<&Fixture> for FixtureEntry {
    fn from(f: &Fixture) -> Self {
        Self {
            id: f.id.clone(),
            kind: f.kind,
            seed: f.seed.to_string(),
            minutes: f.minutes,
            knockout: f.knockout,
            pack: f.pack.map(str::to_string),
            changes: f.changes.clone(),
        }
    }
}

/// Why a golden file was refused.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GoldenError {
    #[error("golden file {path}: cannot read it: {reason}")]
    Read { path: String, reason: String },
    #[error("golden file is malformed: {0}")]
    Malformed(String),
    #[error("golden file has gate schema {found}; this build reads gate schema {expected}")]
    Schema { found: u16, expected: u16 },
    #[error(
        "golden file has state inventory version {found}; this build hashes version {expected}"
    )]
    Inventory { found: u16, expected: u16 },
    #[error("golden file has a checkpoint every {found} ticks; this build uses {expected}")]
    CheckpointEvery { found: u32, expected: u32 },
    #[error("golden file fixture list differs from this build's: {0}")]
    Fixtures(String),
    #[error(
        "golden file has no hash set for this machine ({machine}); it has: {present}. \
         Write one with the bootstrap run on this machine"
    )]
    NoHashSet { machine: String, present: String },
    #[error("golden file hash set {machine}: match {id} is missing")]
    MissingMatch { machine: String, id: String },
    #[error("golden file hash set {machine}: match {id} appears more than once")]
    DuplicateMatch { machine: String, id: String },
    #[error("golden file hash set {machine}: match {id} is not a gate fixture")]
    UnknownMatch { machine: String, id: String },
    #[error("golden file hash set {machine}: match {id}: missing checkpoint: {detail}")]
    MissingCheckpoint {
        machine: String,
        id: String,
        detail: String,
    },
    #[error("golden file hash set {machine}: match {id}: {detail}")]
    BadHash {
        machine: String,
        id: String,
        detail: String,
    },
    #[error("golden file {path} already exists; the bootstrap writes only the first file")]
    Exists { path: String },
    #[error("golden file {path}: cannot write it: {reason}")]
    Write { path: String, reason: String },
}

/// This machine's hash-set key, `<os>-<arch>`, such as `windows-x86_64`.
pub fn machine_key() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// Reads and strictly checks the golden file at `path` against `fixtures`.
pub fn load(path: &Path, fixtures: &[Fixture]) -> Result<GoldenFile, GoldenError> {
    let text = std::fs::read_to_string(path).map_err(|e| GoldenError::Read {
        path: path.display().to_string(),
        reason: e.to_string(),
    })?;
    parse(&text, fixtures)
}

/// Parses and strictly checks a golden file: the versions, the fixture list, and every hash
/// set's matches and checkpoints. The tick count is checked when a match is compared.
pub fn parse(text: &str, fixtures: &[Fixture]) -> Result<GoldenFile, GoldenError> {
    let file: GoldenFile =
        serde_json::from_str(text).map_err(|e| GoldenError::Malformed(e.to_string()))?;
    if file.gate_schema != GATE_SCHEMA {
        return Err(GoldenError::Schema {
            found: file.gate_schema,
            expected: GATE_SCHEMA,
        });
    }
    if file.inventory_version != INVENTORY_VERSION {
        return Err(GoldenError::Inventory {
            found: file.inventory_version,
            expected: INVENTORY_VERSION,
        });
    }
    if file.checkpoint_every != CHECKPOINT_EVERY {
        return Err(GoldenError::CheckpointEvery {
            found: file.checkpoint_every,
            expected: CHECKPOINT_EVERY,
        });
    }
    let expected: Vec<FixtureEntry> = fixtures.iter().map(FixtureEntry::from).collect();
    if file.fixtures != expected {
        let at = file
            .fixtures
            .iter()
            .zip(&expected)
            .position(|(a, b)| a != b)
            .unwrap_or(file.fixtures.len().min(expected.len()));
        let shown = |list: &[FixtureEntry]| {
            list.get(at).map_or_else(
                || "nothing".to_string(),
                |f| format!("{} (seed {})", f.id, f.seed),
            )
        };
        return Err(GoldenError::Fixtures(format!(
            "entry {} is {} in the file and {} in this build",
            at + 1,
            shown(&file.fixtures),
            shown(&expected)
        )));
    }
    for (machine, set) in &file.hash_sets {
        check_set(machine, set, fixtures)?;
    }
    Ok(file)
}

fn check_set(machine: &str, set: &[MatchHashes], fixtures: &[Fixture]) -> Result<(), GoldenError> {
    let m = || machine.to_string();
    for (i, entry) in set.iter().enumerate() {
        if set[..i].iter().any(|e| e.id == entry.id) {
            return Err(GoldenError::DuplicateMatch {
                machine: m(),
                id: entry.id.clone(),
            });
        }
        if !fixtures.iter().any(|f| f.id == entry.id) {
            return Err(GoldenError::UnknownMatch {
                machine: m(),
                id: entry.id.clone(),
            });
        }
    }
    for fixture in fixtures {
        let Some(entry) = set.iter().find(|e| e.id == fixture.id) else {
            return Err(GoldenError::MissingMatch {
                machine: m(),
                id: fixture.id.clone(),
            });
        };
        let bad = |detail: String| GoldenError::BadHash {
            machine: m(),
            id: entry.id.clone(),
            detail,
        };
        if !is_hash(&entry.final_hash) {
            return Err(bad(
                "the final hash is not 64 lowercase hex characters".into()
            ));
        }
        let expected = checkpoint_ticks(entry.ticks);
        for (i, tick) in expected.iter().enumerate() {
            match entry.checkpoints.get(i) {
                Some(c) if c.tick == *tick => {
                    if !is_hash(&c.hash) {
                        return Err(bad(format!(
                            "the checkpoint at tick {tick} is not 64 lowercase hex characters"
                        )));
                    }
                }
                _ => {
                    return Err(GoldenError::MissingCheckpoint {
                        machine: m(),
                        id: entry.id.clone(),
                        detail: format!(
                            "no checkpoint at tick {tick} (one every {CHECKPOINT_EVERY} ticks \
                             and one at the last tick, {})",
                            entry.ticks
                        ),
                    });
                }
            }
        }
        if entry.checkpoints.len() != expected.len() {
            return Err(GoldenError::MissingCheckpoint {
                machine: m(),
                id: entry.id.clone(),
                detail: format!(
                    "{} checkpoints listed for {} ticks; expected {}",
                    entry.checkpoints.len(),
                    entry.ticks,
                    expected.len()
                ),
            });
        }
        if entry.checkpoints.last().map(|c| &c.hash) != Some(&entry.final_hash) {
            return Err(bad(
                "the final hash differs from the last-tick checkpoint".into()
            ));
        }
    }
    Ok(())
}

/// The checkpoint ticks of a match of `ticks`: every 1,000 ticks, and the last tick.
fn checkpoint_ticks(ticks: u32) -> Vec<u32> {
    let mut out: Vec<u32> = (1..)
        .map(|k| k * CHECKPOINT_EVERY)
        .take_while(|t| *t < ticks)
        .collect();
    out.push(ticks);
    out
}

fn is_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl GoldenFile {
    /// The hash set for `machine`.
    pub fn set_for(&self, machine: &str) -> Result<&[MatchHashes], GoldenError> {
        self.hash_sets
            .get(machine)
            .map(Vec::as_slice)
            .ok_or_else(|| GoldenError::NoHashSet {
                machine: machine.to_string(),
                present: if self.hash_sets.is_empty() {
                    "none".into()
                } else {
                    self.hash_sets
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                },
            })
    }

    /// A first golden file with this machine's hash set.
    pub fn first(fixtures: &[Fixture], matches: Vec<MatchHashes>) -> Self {
        let machine = machine_key();
        Self {
            gate_schema: GATE_SCHEMA,
            inventory_version: INVENTORY_VERSION,
            checkpoint_every: CHECKPOINT_EVERY,
            toolchain: TOOLCHAIN.into(),
            bootstrap: Bootstrap {
                engine_version: crate::version().into(),
                build: crate::build_hash().into(),
                utc: utc_now(),
                machine: machine.clone(),
            },
            fixtures: fixtures.iter().map(FixtureEntry::from).collect(),
            hash_sets: BTreeMap::from([(machine, matches)]),
        }
    }

    /// Writes the file to `path`, which must not exist yet.
    pub fn write_new(&self, path: &Path) -> Result<(), GoldenError> {
        let shown = path.display().to_string();
        let write = |reason: String| GoldenError::Write {
            path: shown.clone(),
            reason,
        };
        let mut text = serde_json::to_string_pretty(self).map_err(|e| write(e.to_string()))?;
        text.push('\n');
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(|e| write(e.to_string()))?;
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    GoldenError::Exists {
                        path: shown.clone(),
                    }
                } else {
                    write(e.to_string())
                }
            })?;
        file.write_all(text.as_bytes())
            .map_err(|e| write(e.to_string()))
    }
}

/// The current UTC time as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    utc(secs)
}

/// `secs` after the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ` (the civil-from-days algorithm of
/// H. Hinnant, http://howardhinnant.github.io/date_algorithms.html).
fn utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rest = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_formats_known_instants() {
        assert_eq!(utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(utc(1_790_428_531), "2026-09-26T13:15:31Z");
    }

    #[test]
    fn checkpoints_fall_every_thousand_ticks_and_on_the_last() {
        assert_eq!(checkpoint_ticks(2_500), vec![1_000, 2_000, 2_500]);
        assert_eq!(checkpoint_ticks(2_000), vec![1_000, 2_000]);
        assert_eq!(checkpoint_ticks(7), vec![7]);
    }
}

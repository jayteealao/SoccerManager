//! The golden file (`gate/golden.json`): the gate's versions, the fixture list, an
//! append-only ledger of who wrote each hash set and why, the recorded difference between the
//! machines' sets, and the hash sets, each with every match's tick count, final hash, and
//! checkpoints. A hash set is keyed by machine (`<os>-<arch>`), or by [`PORTABLE`]: the one
//! set every machine compares against, which a build with the same results on every machine
//! writes and which is then the file's only set.
//!
//! The strict load refuses a file that does not fit this build's gate or whose ledger does not
//! account for its hash sets, and names the fault. Only three writers exist: the bootstrap
//! (the first file), `with_machine_set` (a second machine's set for unchanged code), and
//! `regenerated` (a hash change). Each needs a reason and appends one ledger entry.
//!
//! The first file form (a top-level `bootstrap` object and no ledger) reads as a one-entry
//! bootstrap ledger; every write uses the ledger form.
//!
//! Seeds are decimal strings and hashes lowercase hex strings, because 18446744073709551615
//! is above the exact integer range of many JSON readers.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{
    CHECKPOINT_EVERY, Fixture, FixtureKind, GATE_SCHEMA, INVENTORY_VERSION, MatchHashes,
    PlannedChange,
};

/// The Rust toolchain the hashes are computed with (`rust-toolchain.toml`).
pub const TOOLCHAIN: &str = "1.92.0";
/// The golden file's path from the repository root.
pub const DEFAULT_PATH: &str = "gate/golden.json";
/// The reason of a bootstrap entry when none is given, and of the first file form's entry.
pub const BOOTSTRAP_REASON: &str = "first golden file, written by the bootstrap run";

/// The golden file, fields in file order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GoldenFile {
    pub gate_schema: u16,
    pub inventory_version: u16,
    pub checkpoint_every: u32,
    pub toolchain: String,
    pub fixtures: Vec<FixtureEntry>,
    /// Every write, oldest first. Entries are never edited or removed.
    pub ledger: Vec<LedgerEntry>,
    /// One record per pair of machine keys, in key order; empty with one hash set.
    pub set_differences: Vec<SetDifference>,
    /// Machine key to the matches in fixture order.
    pub hash_sets: BTreeMap<String, Vec<MatchHashes>>,
}

/// Both file forms as they are stored; [`read`] turns either into a [`GoldenFile`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    gate_schema: u16,
    inventory_version: u16,
    checkpoint_every: u32,
    toolchain: String,
    #[serde(default)]
    bootstrap: Option<Bootstrap>,
    fixtures: Vec<FixtureEntry>,
    #[serde(default)]
    ledger: Option<Vec<LedgerEntry>>,
    #[serde(default)]
    set_differences: Option<Vec<SetDifference>>,
    hash_sets: BTreeMap<String, Vec<MatchHashes>>,
}

/// The first file form's record of who wrote the file, and when.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bootstrap {
    engine_version: String,
    build: String,
    utc: String,
    machine: String,
}

/// What a ledger entry records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
    /// The first golden file.
    Bootstrap,
    /// A second machine's hash set for unchanged code.
    AddMachineSet,
    /// A hash change.
    Regenerate,
}

impl EntryKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap",
            Self::AddMachineSet => "add-machine-set",
            Self::Regenerate => "regenerate",
        }
    }
}

/// One ledger entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerEntry {
    pub kind: EntryKind,
    pub reason: String,
    pub engine_version: String,
    /// The build commit (`engine::build_hash()`): a short commit, with `-dirty` for a
    /// changed tree, or `unknown`.
    pub build: String,
    /// The random-stream scheme ([`crate::rng::STREAM_SCHEME`]).
    pub scheme: u8,
    /// UTC time, `YYYY-MM-DDTHH:MM:SSZ`.
    pub utc: String,
    /// The key of the hash set the entry writes: a machine (`<os>-<arch>`), or [`PORTABLE`]
    /// for the one set every machine compares against (the reason then names the host).
    pub machine: String,
    /// `regenerate` only: the commit whose code made the new hashes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate: Option<String>,
    /// `regenerate` only: the pre-assigned path of the band-run result,
    /// `gate/bands/ledger-<index>.json`, added later without an edit to the entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub band_result: Option<String>,
}

impl LedgerEntry {
    /// An entry of `kind` written now by this build for `machine`.
    fn now(kind: EntryKind, reason: &str, machine: &str) -> Self {
        Self {
            kind,
            reason: reason.to_string(),
            engine_version: crate::version().into(),
            build: crate::build_hash().into(),
            scheme: crate::rng::STREAM_SCHEME,
            utc: utc_now(),
            machine: machine.to_string(),
            candidate: None,
            band_result: None,
        }
    }
}

/// The band-result path of the ledger entry at `index`.
pub fn band_result_path(index: usize) -> String {
    format!("gate/bands/ledger-{index}.json")
}

/// Which matches differ between two machines' hash sets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetDifference {
    pub a: String,
    pub b: String,
    /// The ids of the matches whose tick count, final hash, or checkpoints differ, in the
    /// order of `a`'s set.
    pub differ: Vec<String>,
    /// The number of equal matches.
    pub same: u32,
}

/// The difference records of `sets`: one per pair of machine keys, in key order.
pub fn set_differences(sets: &BTreeMap<String, Vec<MatchHashes>>) -> Vec<SetDifference> {
    let keys: Vec<&String> = sets.keys().collect();
    let mut out = Vec::new();
    for (i, a) in keys.iter().enumerate() {
        for b in &keys[i + 1..] {
            let (sa, sb) = (&sets[*a], &sets[*b]);
            let mut differ = Vec::new();
            let mut same = 0;
            for m in sa {
                if sb.iter().any(|n| n == m) {
                    same += 1;
                } else {
                    differ.push(m.id.clone());
                }
            }
            out.push(SetDifference {
                a: (*a).clone(),
                b: (*b).clone(),
                differ,
                same,
            });
        }
    }
    out
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
         Add one with `engine-cli gate --add-machine-set --reason <TEXT>` on this machine"
    )]
    NoHashSet { machine: String, present: String },
    #[error(
        "golden file has no portable hash set; it has: {present}. This build compares every \
         machine against one portable set; write it with `engine-cli gate --regenerate \
         --reason <TEXT>`"
    )]
    NoPortableSet { present: String },
    #[error("golden file already has a hash set for this machine ({machine})")]
    SetExists { machine: String },
    #[error("{0}; a portable hash set is the only set, so no machine set is added beside it")]
    PortableOnly(String),
    #[error("golden file has {0}")]
    Form(String),
    #[error("golden file ledger: {0}")]
    Ledger(String),
    #[error("golden file set_differences: {0}")]
    Differences(String),
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

/// The key of the one hash set every machine compares against.
pub const PORTABLE: &str = "portable";

/// The hash-set key this build reads and writes: [`PORTABLE`], because the engine plays the
/// same results on every machine (the `libm` maths and the keyed random streams).
pub fn set_key() -> String {
    PORTABLE.to_string()
}

/// Reads and strictly checks the golden file at `path` against `fixtures`.
pub fn load(path: &Path, fixtures: &[Fixture]) -> Result<GoldenFile, GoldenError> {
    parse(&read_text(path)?, fixtures)
}

/// Reads the golden file at `path` in either form without checking it against this build.
pub fn load_lenient(path: &Path) -> Result<GoldenFile, GoldenError> {
    read(&read_text(path)?)
}

fn read_text(path: &Path) -> Result<String, GoldenError> {
    std::fs::read_to_string(path).map_err(|e| GoldenError::Read {
        path: path.display().to_string(),
        reason: e.to_string(),
    })
}

/// Parses a golden file in either form, without checking it against this build. The first
/// form (a `bootstrap` object, no `ledger`) reads as a one-entry bootstrap ledger, and its
/// difference record is computed from its hash sets.
pub fn read(text: &str) -> Result<GoldenFile, GoldenError> {
    let stored: Stored =
        serde_json::from_str(text).map_err(|e| GoldenError::Malformed(e.to_string()))?;
    let (ledger, set_differences) = match (stored.bootstrap, stored.ledger) {
        (Some(b), None) => {
            if stored.set_differences.is_some() {
                return Err(GoldenError::Form(
                    "a bootstrap object and set_differences; the first form has no \
                     set_differences"
                        .into(),
                ));
            }
            let entry = LedgerEntry {
                kind: EntryKind::Bootstrap,
                reason: BOOTSTRAP_REASON.into(),
                engine_version: b.engine_version,
                build: b.build,
                scheme: 0,
                utc: b.utc,
                machine: b.machine,
                candidate: None,
                band_result: None,
            };
            (vec![entry], set_differences(&stored.hash_sets))
        }
        (None, Some(ledger)) => (ledger, stored.set_differences.unwrap_or_default()),
        (Some(_), Some(_)) => {
            return Err(GoldenError::Form(
                "both a bootstrap object and a ledger; it must have one".into(),
            ));
        }
        (None, None) => {
            return Err(GoldenError::Form(
                "neither a ledger nor a bootstrap object".into(),
            ));
        }
    };
    Ok(GoldenFile {
        gate_schema: stored.gate_schema,
        inventory_version: stored.inventory_version,
        checkpoint_every: stored.checkpoint_every,
        toolchain: stored.toolchain,
        fixtures: stored.fixtures,
        ledger,
        set_differences,
        hash_sets: stored.hash_sets,
    })
}

/// Parses and strictly checks a golden file: the versions, the fixture list, the ledger, the
/// difference record, and every hash set's matches and checkpoints. The tick count is
/// checked when a match is compared.
pub fn parse(text: &str, fixtures: &[Fixture]) -> Result<GoldenFile, GoldenError> {
    let file = read(text)?;
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
    check_ledger(&file)?;
    for (machine, set) in &file.hash_sets {
        check_set(machine, set, fixtures)?;
    }
    Ok(file)
}

/// The ledger rules every file keeps: at least one entry; the first, and only the first, is
/// a `bootstrap`; every entry has a reason and a machine; only a `regenerate` entry holds a
/// candidate and its pre-assigned band-result path; the hash sets are exactly the machine of
/// the last `bootstrap` or `regenerate` entry plus the machines of the `add-machine-set`
/// entries after it; a [`PORTABLE`] set is the only set, written by that last `bootstrap` or
/// `regenerate` entry with no `add-machine-set` entry after it; and the difference record is
/// the one the hash sets give.
pub fn check_ledger(file: &GoldenFile) -> Result<(), GoldenError> {
    let fault = |m: String| Err(GoldenError::Ledger(m));
    if file.ledger.is_empty() {
        return fault("it is empty; the first entry must be a bootstrap".into());
    }
    let mut base = 0;
    for (i, e) in file.ledger.iter().enumerate() {
        let kind = e.kind.name();
        if (i == 0) != (e.kind == EntryKind::Bootstrap) {
            return fault(if i == 0 {
                format!("entry 0 is {kind}; the first entry must be a bootstrap")
            } else {
                format!("entry {i} is a second bootstrap")
            });
        }
        if e.reason.trim().is_empty() {
            return fault(format!("entry {i} ({kind}) has no reason"));
        }
        if e.machine.trim().is_empty() {
            return fault(format!("entry {i} ({kind}) names no machine"));
        }
        if e.kind == EntryKind::Regenerate {
            let path = band_result_path(i);
            if e.band_result.as_deref() != Some(path.as_str()) {
                return fault(format!(
                    "entry {i} (regenerate) must have band_result \"{path}\""
                ));
            }
            if e.candidate.as_deref().is_none_or(|c| c.trim().is_empty()) {
                return fault(format!("entry {i} (regenerate) names no candidate commit"));
            }
            base = i;
        } else if e.candidate.is_some() || e.band_result.is_some() {
            return fault(format!(
                "entry {i} ({kind}) has a candidate or a band_result; only a regenerate \
                 entry has them"
            ));
        }
    }
    let portable_base = file.ledger[base].machine == PORTABLE;
    for (i, e) in file.ledger.iter().enumerate().skip(base + 1) {
        if portable_base {
            return fault(format!(
                "entry {i} (add-machine-set) adds {} after the portable set of entry {base}; \
                 the portable set is the only set",
                e.machine
            ));
        }
        if e.machine == PORTABLE {
            return fault(format!(
                "entry {i} (add-machine-set) adds the portable set; only a bootstrap or a \
                 regenerate entry writes it"
            ));
        }
    }
    if file.hash_sets.contains_key(PORTABLE) && file.hash_sets.len() > 1 {
        return fault(format!(
            "the hash sets are {}; a portable set is the only set",
            list(&file.hash_sets.keys().cloned().collect::<Vec<_>>())
        ));
    }
    let mut accounted = vec![file.ledger[base].machine.clone()];
    for (i, e) in file.ledger.iter().enumerate().skip(base + 1) {
        if accounted.contains(&e.machine) {
            return fault(format!(
                "entry {i} (add-machine-set) adds {}, which already has a set",
                e.machine
            ));
        }
        accounted.push(e.machine.clone());
    }
    accounted.sort();
    let present: Vec<String> = file.hash_sets.keys().cloned().collect();
    if present != accounted {
        return fault(format!(
            "the hash sets are {}, but the entries since the last bootstrap or regenerate \
             account for {}",
            list(&present),
            list(&accounted)
        ));
    }
    if file.set_differences != set_differences(&file.hash_sets) {
        return Err(GoldenError::Differences(
            "the record differs from the one the hash sets give".into(),
        ));
    }
    Ok(())
}

fn list(items: &[String]) -> String {
    if items.is_empty() {
        "none".into()
    } else {
        items.join(", ")
    }
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
    /// The hash set keyed `key`: a machine key, or [`PORTABLE`].
    pub fn set_for(&self, key: &str) -> Result<&[MatchHashes], GoldenError> {
        self.hash_sets.get(key).map(Vec::as_slice).ok_or_else(|| {
            let present = list(&self.hash_sets.keys().cloned().collect::<Vec<_>>());
            if key == PORTABLE {
                GoldenError::NoPortableSet { present }
            } else {
                GoldenError::NoHashSet {
                    machine: key.to_string(),
                    present,
                }
            }
        })
    }

    /// A first golden file with `machine`'s hash set and one bootstrap entry.
    pub fn first(
        fixtures: &[Fixture],
        machine: &str,
        matches: Vec<MatchHashes>,
        reason: &str,
    ) -> Self {
        Self {
            gate_schema: GATE_SCHEMA,
            inventory_version: INVENTORY_VERSION,
            checkpoint_every: CHECKPOINT_EVERY,
            toolchain: TOOLCHAIN.into(),
            fixtures: fixtures.iter().map(FixtureEntry::from).collect(),
            ledger: vec![LedgerEntry::now(EntryKind::Bootstrap, reason, machine)],
            set_differences: Vec::new(),
            hash_sets: BTreeMap::from([(machine.to_string(), matches)]),
        }
    }

    /// This file with `machine`'s hash set added by one `add-machine-set` entry. Refused when
    /// `machine` already has a set, when `machine` is [`PORTABLE`], and when the file has a
    /// portable set.
    pub fn with_machine_set(
        mut self,
        machine: &str,
        matches: Vec<MatchHashes>,
        reason: &str,
    ) -> Result<Self, GoldenError> {
        if machine == PORTABLE {
            return Err(GoldenError::PortableOnly(
                "this build writes the portable set, which only a regeneration writes".into(),
            ));
        }
        if self.hash_sets.contains_key(PORTABLE) {
            return Err(GoldenError::PortableOnly(
                "the golden file has a portable hash set".into(),
            ));
        }
        if self.hash_sets.contains_key(machine) {
            return Err(GoldenError::SetExists {
                machine: machine.to_string(),
            });
        }
        self.ledger
            .push(LedgerEntry::now(EntryKind::AddMachineSet, reason, machine));
        self.hash_sets.insert(machine.to_string(), matches);
        self.set_differences = set_differences(&self.hash_sets);
        Ok(self)
    }

    /// The file a regeneration writes: this build's header and fixture list, this file's
    /// ledger plus one `regenerate` entry, and the new set keyed `machine` (a machine key or
    /// [`PORTABLE`]) as the only set. Also returns the keys whose sets were dropped: a set
    /// made by older code is stale after a hash change, so each such machine re-adds its set
    /// with `with_machine_set`, unless the new set is the portable one.
    pub fn regenerated(
        self,
        fixtures: &[Fixture],
        machine: &str,
        matches: Vec<MatchHashes>,
        reason: &str,
    ) -> (Self, Vec<String>) {
        let dropped = self
            .hash_sets
            .into_keys()
            .filter(|m| m != machine)
            .collect();
        let mut ledger = self.ledger;
        let mut entry = LedgerEntry::now(EntryKind::Regenerate, reason, machine);
        entry.candidate = Some(crate::build_hash().into());
        entry.band_result = Some(band_result_path(ledger.len()));
        ledger.push(entry);
        let file = Self {
            gate_schema: GATE_SCHEMA,
            inventory_version: INVENTORY_VERSION,
            checkpoint_every: CHECKPOINT_EVERY,
            toolchain: TOOLCHAIN.into(),
            fixtures: fixtures.iter().map(FixtureEntry::from).collect(),
            ledger,
            set_differences: Vec::new(),
            hash_sets: BTreeMap::from([(machine.to_string(), matches)]),
        };
        (file, dropped)
    }

    /// The file's text: pretty JSON with a final newline.
    pub fn to_text(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).expect("a golden file always serializes");
        text.push('\n');
        text
    }

    /// Writes the file to `path`, which must not exist yet.
    pub fn write_new(&self, path: &Path) -> Result<(), GoldenError> {
        let shown = path.display().to_string();
        let write = |reason: String| GoldenError::Write {
            path: shown.clone(),
            reason,
        };
        let text = self.to_text();
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

    /// Replaces the file at `path` through `<path>.tmp` in the same folder and a rename, so a
    /// failed write leaves the old file byte-identical.
    pub fn write_replace(&self, path: &Path) -> Result<(), GoldenError> {
        let write = |reason: String| GoldenError::Write {
            path: path.display().to_string(),
            reason,
        };
        let mut name = path
            .file_name()
            .ok_or_else(|| write("the path names no file".into()))?
            .to_os_string();
        name.push(".tmp");
        let tmp: PathBuf = path.with_file_name(name);
        let result = std::fs::write(&tmp, self.to_text().as_bytes())
            .and_then(|()| std::fs::rename(&tmp, path))
            .map_err(|e| write(e.to_string()));
        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        result
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

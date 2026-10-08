//! The compact rows of a calibration run: one row per match of unrounded counts,
//! numerators and denominators, keyed by fixture, in a small binary column file per
//! thread and session, `rows/<session>-<thread>.rows`.
//!
//! A file starts with a header: the magic `SMROWS`, the format version, the measures
//! version, and the column list (each column's name and type). Then come blocks, one per
//! finished work unit: the suite, the unit's place, the row count, each column's values for
//! the unit's rows in column order (little-endian), and the first 8 bytes of the SHA-256 of
//! the block. A block is written and synced before the unit's ledger line, so a unit with
//! no ledger line plays again and its block is never read: a row counts only when the
//! ledger entry of its key was written by the same session. Reading stops at a partial or
//! damaged block, which only a stopped run leaves at the end of a file. A file of another
//! format version is refused by name.
//!
//! The rows keep no rounded value. [`Row::band_record`] rebuilds the per-match figures the
//! bands read with the same rounding functions the statistics files use, so the bands of a
//! run from rows equal the bands of a run from statistics files.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use engine::Summary;
use engine::observe::{
    LawStats, MatchFigures, MatchStats, TacticsStats, TeamRef, ball_in_play_s, round_xg, share_pct,
};
use sha2::{Digest, Sha256};

use super::fixtures::FixtureKey;
use super::run_folder::Done;
use crate::report::MEASURES_VERSION;

/// The row file format this program writes and reads.
pub const ROWS_FORMAT: u32 = 1;
/// The first bytes of every row file.
const MAGIC: &[u8; 6] = b"SMROWS";
/// The folder of the row files in a run or arm folder.
pub const ROWS_DIR: &str = "rows";

/// How a match ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Outcome {
    #[default]
    Success,
    /// The engine returned an error.
    Error,
    /// The match panicked; the run caught it.
    Panic,
}

impl Outcome {
    fn code(self) -> u64 {
        match self {
            Self::Success => 0,
            Self::Error => 1,
            Self::Panic => 2,
        }
    }

    fn from_code(code: u64) -> Self {
        match code {
            0 => Self::Success,
            1 => Self::Error,
            _ => Self::Panic,
        }
    }
}

/// Why a match has a full recording (its statistics file and its event file), as bits of
/// [`Row::reasons`]; 0 for a match with none.
pub mod reason {
    /// About 1 in 16 matches, chosen by fixture key.
    pub const SAMPLE: u8 = 1;
    /// The match failed, panicked, or hit a dark path.
    pub const ERROR: u8 = 2;
    /// The rule checker found a violation.
    pub const VIOLATION: u8 = 4;
    /// A measure lay outside the 1st to 99th percentile of its suite so far.
    pub const EXTREME: u8 = 8;
    /// `--keep-events all`.
    pub const ALL: u8 = 16;
    /// The reasons that make a match an outlier.
    pub const OUTLIER: u8 = ERROR | VIOLATION | EXTREME;
}

/// One match's compact row. Per-team pairs are home first.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Row {
    pub key: FixtureKey,
    /// The engine seed the match was played with.
    pub seed: u64,
    pub outcome: Outcome,
    /// Bits of [`reason`]; not part of the match's result.
    pub reasons: u8,
    pub goals: [u32; 2],
    pub shots: [u32; 2],
    pub shots_on_target: [u32; 2],
    /// Expected goals, unrounded.
    pub xg: [f64; 2],
    pub passes: [u32; 2],
    pub passes_completed: [u32; 2],
    /// Open-play ticks credited to each team.
    pub possession_ticks: [u32; 2],
    /// Ticks with the ball in play.
    pub live_ticks: u32,
    pub fouls: [u32; 2],
    pub offsides: [u32; 2],
    pub corners: [u32; 2],
    pub throw_ins: [u32; 2],
    pub goal_kicks: [u32; 2],
    pub yellow: [u32; 2],
    pub red: [u32; 2],
    /// Injuries, both teams.
    pub injuries: u32,
    pub substitutions: [u32; 2],
    pub change_never_applied: u32,
    pub change_expired_at_full_time: u32,
    /// Rule violations the running checker found (tick and event rules).
    pub violations: u32,
    pub ticks: u32,
    /// The match's wall time in microseconds; not part of its result.
    pub duration_us: u64,
}

/// A column's type: its width in the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    U8,
    U32,
    U64,
    F64,
}

impl Kind {
    fn code(self) -> u8 {
        match self {
            Self::U8 => 1,
            Self::U32 => 4,
            Self::U64 => 8,
            Self::F64 => 9,
        }
    }

    fn width(self) -> usize {
        match self {
            Self::U8 => 1,
            Self::U32 => 4,
            Self::U64 | Self::F64 => 8,
        }
    }
}

/// One column: its name, its type, and how a row's value is read and set, as 64 bits.
struct Column {
    name: &'static str,
    kind: Kind,
    /// Part of the match's result: the digest reads it.
    result: bool,
    get: fn(&Row) -> u64,
    set: fn(&mut Row, u64),
}

const fn col(
    name: &'static str,
    kind: Kind,
    get: fn(&Row) -> u64,
    set: fn(&mut Row, u64),
) -> Column {
    Column {
        name,
        kind,
        result: true,
        get,
        set,
    }
}

/// A `u32` read back from a column: the file wrote at most 32 bits.
fn u32_of(v: u64) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

/// Every column of format 1, in file order.
const COLUMNS: &[Column] = &[
    col(
        "fixture.key",
        Kind::U64,
        |r| r.key.as_u64(),
        |r, v| {
            r.key = FixtureKey::from_u64(v);
        },
    ),
    col("engine.seed", Kind::U64, |r| r.seed, |r, v| r.seed = v),
    col(
        "outcome",
        Kind::U8,
        |r| r.outcome.code(),
        |r, v| {
            r.outcome = Outcome::from_code(v);
        },
    ),
    Column {
        name: "record.reasons",
        kind: Kind::U8,
        result: false,
        get: |r| u64::from(r.reasons),
        set: |r, v| r.reasons = u8::try_from(v).unwrap_or(u8::MAX),
    },
    col(
        "goals.home",
        Kind::U32,
        |r| r.goals[0].into(),
        |r, v| r.goals[0] = u32_of(v),
    ),
    col(
        "goals.away",
        Kind::U32,
        |r| r.goals[1].into(),
        |r, v| r.goals[1] = u32_of(v),
    ),
    col(
        "shots.home",
        Kind::U32,
        |r| r.shots[0].into(),
        |r, v| r.shots[0] = u32_of(v),
    ),
    col(
        "shots.away",
        Kind::U32,
        |r| r.shots[1].into(),
        |r, v| r.shots[1] = u32_of(v),
    ),
    col(
        "shots_on_target.home",
        Kind::U32,
        |r| r.shots_on_target[0].into(),
        |r, v| {
            r.shots_on_target[0] = u32_of(v);
        },
    ),
    col(
        "shots_on_target.away",
        Kind::U32,
        |r| r.shots_on_target[1].into(),
        |r, v| {
            r.shots_on_target[1] = u32_of(v);
        },
    ),
    col(
        "xg.home",
        Kind::F64,
        |r| r.xg[0].to_bits(),
        |r, v| r.xg[0] = f64::from_bits(v),
    ),
    col(
        "xg.away",
        Kind::F64,
        |r| r.xg[1].to_bits(),
        |r, v| r.xg[1] = f64::from_bits(v),
    ),
    col(
        "passes.home",
        Kind::U32,
        |r| r.passes[0].into(),
        |r, v| r.passes[0] = u32_of(v),
    ),
    col(
        "passes.away",
        Kind::U32,
        |r| r.passes[1].into(),
        |r, v| r.passes[1] = u32_of(v),
    ),
    col(
        "passes_completed.home",
        Kind::U32,
        |r| r.passes_completed[0].into(),
        |r, v| {
            r.passes_completed[0] = u32_of(v);
        },
    ),
    col(
        "passes_completed.away",
        Kind::U32,
        |r| r.passes_completed[1].into(),
        |r, v| {
            r.passes_completed[1] = u32_of(v);
        },
    ),
    col(
        "possession_ticks.home",
        Kind::U32,
        |r| r.possession_ticks[0].into(),
        |r, v| {
            r.possession_ticks[0] = u32_of(v);
        },
    ),
    col(
        "possession_ticks.away",
        Kind::U32,
        |r| r.possession_ticks[1].into(),
        |r, v| {
            r.possession_ticks[1] = u32_of(v);
        },
    ),
    col(
        "live_ticks",
        Kind::U32,
        |r| r.live_ticks.into(),
        |r, v| r.live_ticks = u32_of(v),
    ),
    col(
        "fouls.home",
        Kind::U32,
        |r| r.fouls[0].into(),
        |r, v| r.fouls[0] = u32_of(v),
    ),
    col(
        "fouls.away",
        Kind::U32,
        |r| r.fouls[1].into(),
        |r, v| r.fouls[1] = u32_of(v),
    ),
    col(
        "offsides.home",
        Kind::U32,
        |r| r.offsides[0].into(),
        |r, v| r.offsides[0] = u32_of(v),
    ),
    col(
        "offsides.away",
        Kind::U32,
        |r| r.offsides[1].into(),
        |r, v| r.offsides[1] = u32_of(v),
    ),
    col(
        "corners.home",
        Kind::U32,
        |r| r.corners[0].into(),
        |r, v| r.corners[0] = u32_of(v),
    ),
    col(
        "corners.away",
        Kind::U32,
        |r| r.corners[1].into(),
        |r, v| r.corners[1] = u32_of(v),
    ),
    col(
        "throw_ins.home",
        Kind::U32,
        |r| r.throw_ins[0].into(),
        |r, v| r.throw_ins[0] = u32_of(v),
    ),
    col(
        "throw_ins.away",
        Kind::U32,
        |r| r.throw_ins[1].into(),
        |r, v| r.throw_ins[1] = u32_of(v),
    ),
    col(
        "goal_kicks.home",
        Kind::U32,
        |r| r.goal_kicks[0].into(),
        |r, v| {
            r.goal_kicks[0] = u32_of(v);
        },
    ),
    col(
        "goal_kicks.away",
        Kind::U32,
        |r| r.goal_kicks[1].into(),
        |r, v| {
            r.goal_kicks[1] = u32_of(v);
        },
    ),
    col(
        "yellow.home",
        Kind::U32,
        |r| r.yellow[0].into(),
        |r, v| r.yellow[0] = u32_of(v),
    ),
    col(
        "yellow.away",
        Kind::U32,
        |r| r.yellow[1].into(),
        |r, v| r.yellow[1] = u32_of(v),
    ),
    col(
        "red.home",
        Kind::U32,
        |r| r.red[0].into(),
        |r, v| r.red[0] = u32_of(v),
    ),
    col(
        "red.away",
        Kind::U32,
        |r| r.red[1].into(),
        |r, v| r.red[1] = u32_of(v),
    ),
    col(
        "injuries",
        Kind::U32,
        |r| r.injuries.into(),
        |r, v| r.injuries = u32_of(v),
    ),
    col(
        "substitutions.home",
        Kind::U32,
        |r| r.substitutions[0].into(),
        |r, v| {
            r.substitutions[0] = u32_of(v);
        },
    ),
    col(
        "substitutions.away",
        Kind::U32,
        |r| r.substitutions[1].into(),
        |r, v| {
            r.substitutions[1] = u32_of(v);
        },
    ),
    col(
        "change_never_applied",
        Kind::U32,
        |r| r.change_never_applied.into(),
        |r, v| {
            r.change_never_applied = u32_of(v);
        },
    ),
    col(
        "change_expired_at_full_time",
        Kind::U32,
        |r| r.change_expired_at_full_time.into(),
        |r, v| {
            r.change_expired_at_full_time = u32_of(v);
        },
    ),
    col(
        "validate.violations",
        Kind::U32,
        |r| r.violations.into(),
        |r, v| r.violations = u32_of(v),
    ),
    col(
        "ticks",
        Kind::U32,
        |r| r.ticks.into(),
        |r, v| r.ticks = u32_of(v),
    ),
    Column {
        name: "duration_us",
        kind: Kind::U64,
        result: false,
        get: |r| r.duration_us,
        set: |r, v| r.duration_us = v,
    },
];

impl Row {
    /// The row of a match played to full time: its summary, its tactics counts, the
    /// rule checker's violations, the ticks played, and its wall time.
    pub fn played(
        (key, seed): (FixtureKey, u64),
        s: &Summary,
        tactics: &TacticsStats,
        violations: usize,
        ticks: u32,
        duration_us: u64,
    ) -> Self {
        Self {
            key,
            seed,
            outcome: Outcome::Success,
            reasons: 0,
            goals: s.goals,
            shots: s.shots,
            shots_on_target: s.shots_on_target,
            xg: s.xg,
            passes: s.passes,
            passes_completed: s.passes_completed,
            possession_ticks: s.possession_ticks,
            live_ticks: s.live_ticks,
            fouls: s.fouls,
            offsides: s.offsides,
            corners: s.corners,
            throw_ins: s.throw_ins,
            goal_kicks: s.goal_kicks,
            yellow: s.yellow,
            red: s.red,
            injuries: tactics.injury_count,
            substitutions: tactics.substitutions,
            change_never_applied: tactics.change_never_applied,
            change_expired_at_full_time: tactics.change_expired_at_full_time,
            violations: u32::try_from(violations).unwrap_or(u32::MAX),
            ticks,
            duration_us,
        }
    }

    /// The row of a match that failed or panicked: zero figures.
    pub fn failed((key, seed): (FixtureKey, u64), outcome: Outcome) -> Self {
        Self {
            key,
            seed,
            outcome,
            ..Self::default()
        }
    }

    /// The match's statistics record as the bands read it, rebuilt from the row with the
    /// rounding of the statistics files. The keys the bands do not read (the identifiers,
    /// the clubs, the content hash) are left empty.
    pub fn band_record(&self) -> MatchStats {
        let possession = self.possession_ticks[0] + self.possession_ticks[1];
        let success = self.outcome == Outcome::Success;
        MatchStats {
            owner_id: String::new(),
            match_id: String::new(),
            seed: self.seed,
            content_hash: String::new(),
            teams: [0, 1].map(|_| TeamRef {
                id: String::new(),
                name: String::new(),
            }),
            duration_ms: self.duration_us / 1000,
            outcome: if success { "success" } else { "error" }.into(),
            ticks_per_s: 0.0,
            ticks_written: self.ticks,
            validate_ran: success,
            validate_violations: self.violations as usize,
            possession_changes: 0,
            ball_max_speed: 0.0,
            ball_idle_ticks: 0,
            goals: self.goals,
            flags_on: Vec::new(),
            laws: LawStats {
                fouls: self.fouls,
                offsides: self.offsides,
                corners: self.corners,
                throw_ins: self.throw_ins,
                goal_kicks: self.goal_kicks,
                yellow: self.yellow,
                red: self.red,
                ticks_played: self.ticks,
                ..LawStats::default()
            },
            tactics: TacticsStats {
                shots: self.shots,
                injury_count: self.injuries,
                change_never_applied: self.change_never_applied,
                change_expired_at_full_time: self.change_expired_at_full_time,
                substitutions: self.substitutions,
                ..TacticsStats::default()
            },
            figures: MatchFigures {
                goals: self.goals,
                shots_on_target: self.shots_on_target,
                xg: self.xg.map(round_xg),
                passes: self.passes,
                passes_completed: self.passes_completed,
                pass_accuracy_pct: [0, 1]
                    .map(|t| share_pct(self.passes_completed[t], self.passes[t])),
                ball_in_play_s: ball_in_play_s(self.live_ticks),
                possession_pct: [0, 1].map(|t| share_pct(self.possession_ticks[t], possession)),
                ..MatchFigures::default()
            },
            script: Default::default(),
        }
    }

    /// The row's result columns as bytes, in column order: the record reasons and the wall
    /// time left out, so equal results give equal bytes however the run was played.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for c in COLUMNS.iter().filter(|c| c.result) {
            put(&mut out, c.kind, (c.get)(self));
        }
        out
    }
}

fn put(out: &mut Vec<u8>, kind: Kind, v: u64) {
    out.extend_from_slice(&v.to_le_bytes()[..kind.width()]);
}

/// The header of a row file of this program's format.
fn header() -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    out.extend_from_slice(&ROWS_FORMAT.to_le_bytes());
    out.extend_from_slice(&MEASURES_VERSION.to_le_bytes());
    out.extend_from_slice(&(COLUMNS.len() as u32).to_le_bytes());
    for c in COLUMNS {
        // Every column name is shorter than 256 bytes.
        out.push(c.name.len() as u8);
        out.extend_from_slice(c.name.as_bytes());
        out.push(c.kind.code());
    }
    out
}

/// The first 8 bytes of the SHA-256 of `bytes`.
fn checksum(bytes: &[u8]) -> [u8; 8] {
    let digest = Sha256::digest(bytes);
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    out
}

/// One thread's row file of one session.
pub struct RowsWriter {
    file: fs::File,
    /// Rows written by this writer.
    pub rows: u32,
}

impl RowsWriter {
    /// Creates `dir/rows/<session>-<thread>.rows` with its header.
    pub fn open(dir: &Path, session: &str, thread: u32) -> anyhow::Result<Self> {
        let rows = dir.join(ROWS_DIR);
        fs::create_dir_all(&rows).with_context(|| format!("cannot create {}", rows.display()))?;
        let path = rows.join(format!("{session}-{thread}.rows"));
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("cannot open {}", path.display()))?;
        if file.metadata()?.len() == 0 {
            file.write_all(&header())?;
        }
        Ok(Self { file, rows: 0 })
    }

    /// Appends one unit's rows as one block and syncs the file.
    pub fn append_block(&mut self, suite: &str, place: u32, rows: &[Row]) -> anyhow::Result<()> {
        let writing = super::stages::enter(super::stages::Stage::Writing);
        let mut block = Vec::new();
        // Every suite code is shorter than 256 bytes.
        block.push(suite.len() as u8);
        block.extend_from_slice(suite.as_bytes());
        block.extend_from_slice(&place.to_le_bytes());
        let count = u32::try_from(rows.len()).unwrap_or(u32::MAX);
        block.extend_from_slice(&count.to_le_bytes());
        for c in COLUMNS {
            for row in rows {
                put(&mut block, c.kind, (c.get)(row));
            }
        }
        let sum = checksum(&block);
        block.extend_from_slice(&sum);
        drop(writing);
        let _disk = super::stages::enter(super::stages::Stage::Disk);
        self.file.write_all(&block)?;
        self.file.sync_data()?;
        self.rows += count;
        Ok(())
    }
}

/// A reader over the bytes of one row file.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let end = self.at.checked_add(n)?;
        let out = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(out)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }

    fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("four bytes")))
    }

    fn value(&mut self, kind: Kind) -> Option<u64> {
        let b = self.take(kind.width())?;
        let mut word = [0u8; 8];
        word[..b.len()].copy_from_slice(b);
        Some(u64::from_le_bytes(word))
    }
}

/// The rows of one file, block by block, up to the first partial or damaged block. A file
/// of another format version, or of this version with other columns, is refused by name.
fn read_file(path: &Path) -> anyhow::Result<Vec<Row>> {
    let bytes = fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    let refuse = |why: String| anyhow::anyhow!("the row file {} {why}", path.display());
    let mut r = Reader {
        bytes: &bytes,
        at: 0,
    };
    if r.take(MAGIC.len()) != Some(&MAGIC[..]) {
        // A header cut off before its magic: the file was created and nothing synced.
        if bytes.len() < MAGIC.len() && MAGIC.starts_with(&bytes) {
            return Ok(Vec::new());
        }
        return Err(refuse("is not a row file".into()));
    }
    let Some(format) = r.u32() else {
        return Ok(Vec::new());
    };
    if format != ROWS_FORMAT {
        return Err(refuse(format!(
            "is row format version {format}, and this program reads version {ROWS_FORMAT}"
        )));
    }
    let expected = header();
    if bytes.len() < expected.len() {
        return Ok(Vec::new());
    }
    if bytes[..expected.len()] != expected[..] {
        return Err(refuse(format!(
            "has another measures version or column list than row format version \
             {ROWS_FORMAT} of this program"
        )));
    }
    r.at = expected.len();
    let mut out = Vec::new();
    while r.at < bytes.len() {
        let start = r.at;
        let Some(block) = read_block(&mut r) else {
            break;
        };
        let end = r.at;
        let Some(sum) = r.take(8) else { break };
        if checksum(&bytes[start..end]) != sum {
            break;
        }
        out.extend(block);
    }
    Ok(out)
}

/// One block's rows, or `None` when the block is cut off.
fn read_block(r: &mut Reader<'_>) -> Option<Vec<Row>> {
    let suite_len = r.u8()? as usize;
    r.take(suite_len)?;
    let _place = r.u32()?;
    let count = r.u32()? as usize;
    // A damaged count cannot ask for more rows than the bytes left could hold: every row
    // takes the width of every column, so the rows allocated never outgrow the file.
    let row_bytes: usize = COLUMNS.iter().map(|c| c.kind.width()).sum();
    let left = r.bytes.len().saturating_sub(r.at);
    if count.checked_mul(row_bytes).is_none_or(|need| need > left) {
        return None;
    }
    let mut rows = vec![Row::default(); count];
    for c in COLUMNS {
        for row in &mut rows {
            (c.set)(row, r.value(c.kind)?);
        }
    }
    Some(rows)
}

/// The session of the row file `path` (`<session>-<thread>.rows`).
fn session_of(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    stem.rsplit_once('-')
        .map_or(stem.clone(), |(session, _)| session.to_string())
}

/// What the row files of a folder hold.
#[derive(Debug, Default)]
pub struct ReadRows {
    /// The rows the ledger counts, by key.
    pub rows: BTreeMap<FixtureKey, Row>,
    /// Row files read.
    pub files: u32,
}

/// The rows of `dir/rows/` whose key `finished` holds, each from the session that finished
/// it with the same engine seed. Rows of a unit no ledger line finished are left out.
pub fn read_rows(dir: &Path, finished: &BTreeMap<FixtureKey, Done>) -> anyhow::Result<ReadRows> {
    let folder = dir.join(ROWS_DIR);
    let mut files: Vec<PathBuf> = match fs::read_dir(&folder) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "rows"))
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e).with_context(|| format!("cannot read {}", folder.display())),
    };
    files.sort();
    let mut out = ReadRows::default();
    for file in files {
        out.files += 1;
        let session = session_of(&file);
        for row in read_file(&file)? {
            let counts = finished
                .get(&row.key)
                .is_some_and(|d| d.session == session && d.seed == row.seed);
            if counts {
                out.rows.insert(row.key, row);
            }
        }
    }
    Ok(out)
}

/// SHA-256, as 64 hex characters, over every arm's rows in key order, each as its key and
/// its result columns: equal results give equal digests however the run was cut into
/// sessions and threads.
pub fn rows_digest<'a>(
    arms: impl IntoIterator<Item = (&'a str, &'a BTreeMap<FixtureKey, Row>)>,
) -> String {
    let mut hasher = Sha256::new();
    for (arm, rows) in arms {
        hasher.update(format!("arm {arm}\n"));
        for (key, row) in rows {
            hasher.update(key.as_u64().to_le_bytes());
            hasher.update(row.canonical_bytes());
        }
    }
    stream::record::hex(&hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("engine-cli-rows-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn row(n: u64) -> Row {
        Row {
            key: FixtureKey::from_u64(n),
            seed: n * 10,
            goals: [n as u32, 1],
            xg: [0.123_456_789 * n as f64, 1.5],
            passes: [400, 380],
            passes_completed: [333, 301],
            possession_ticks: [1234, 999],
            live_ticks: 2_700 + n as u32,
            duration_us: 1_000_000 + n,
            ..Row::default()
        }
    }

    fn done(n: u64, session: &str) -> (FixtureKey, Done) {
        (
            FixtureKey::from_u64(n),
            Done {
                key: FixtureKey::from_u64(n),
                seed: n * 10,
                match_id: String::new(),
                session: session.into(),
            },
        )
    }

    #[test]
    fn two_blocks_read_back_and_only_the_finishing_session_counts() {
        let dir = temp("blocks");
        let mut w = RowsWriter::open(&dir, "100-7", 0).unwrap();
        w.append_block("equal", 0, &[row(1), row(2)]).unwrap();
        w.append_block("red-card", 1, &[row(3)]).unwrap();
        assert_eq!(w.rows, 3);
        // Another session's row of key 3, which that session never finished.
        let mut other = RowsWriter::open(&dir, "200-8", 0).unwrap();
        let mut stale = row(3);
        stale.goals = [9, 9];
        other.append_block("red-card", 0, &[stale]).unwrap();

        let finished: BTreeMap<_, _> = [done(1, "100-7"), done(2, "100-7"), done(3, "100-7")]
            .into_iter()
            .collect();
        let read = read_rows(&dir, &finished).unwrap();
        assert_eq!(read.files, 2);
        assert_eq!(read.rows.len(), 3);
        assert_eq!(read.rows[&FixtureKey::from_u64(1)], row(1));
        assert_eq!(
            read.rows[&FixtureKey::from_u64(3)],
            row(3),
            "the stale row is ignored"
        );
        // A key the ledger does not hold has no row.
        let fewer: BTreeMap<_, _> = [done(1, "100-7")].into_iter().collect();
        assert_eq!(read_rows(&dir, &fewer).unwrap().rows.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_truncated_last_block_and_a_bad_checksum_are_ignored() {
        let dir = temp("damage");
        let mut w = RowsWriter::open(&dir, "1-1", 0).unwrap();
        w.append_block("equal", 0, &[row(1)]).unwrap();
        w.append_block("equal", 1, &[row(2)]).unwrap();
        let path = dir.join("rows/1-1-0.rows");
        let bytes = fs::read(&path).unwrap();
        let finished: BTreeMap<_, _> = [done(1, "1-1"), done(2, "1-1")].into_iter().collect();

        fs::write(&path, &bytes[..bytes.len() - 3]).unwrap();
        let keys: Vec<FixtureKey> = read_rows(&dir, &finished)
            .unwrap()
            .rows
            .into_keys()
            .collect();
        assert_eq!(keys, [FixtureKey::from_u64(1)], "a cut-off block");

        let mut bad = bytes.clone();
        let last = bad.len() - 1;
        bad[last] ^= 0xff;
        fs::write(&path, &bad).unwrap();
        assert_eq!(
            read_rows(&dir, &finished).unwrap().rows.len(),
            1,
            "a bad checksum"
        );

        // Only the header made it to disk, or not even all of it.
        fs::write(&path, &bytes[..header().len()]).unwrap();
        assert!(read_rows(&dir, &finished).unwrap().rows.is_empty());
        fs::write(&path, &bytes[..3]).unwrap();
        assert!(read_rows(&dir, &finished).unwrap().rows.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_block_whose_count_the_bytes_left_cannot_hold_is_not_read() {
        // Suite "equal", place 0, and 50 rows: fewer than the bytes left, more than they hold.
        let mut block = vec![5u8];
        block.extend_from_slice(b"equal");
        block.extend_from_slice(&0u32.to_le_bytes());
        block.extend_from_slice(&50u32.to_le_bytes());
        block.extend_from_slice(&[0u8; 64]);
        let mut r = Reader {
            bytes: &block,
            at: 0,
        };
        assert!(read_block(&mut r).is_none());
    }

    #[test]
    fn an_unknown_format_version_is_refused_by_name() {
        let dir = temp("version");
        RowsWriter::open(&dir, "1-1", 0).unwrap();
        let path = dir.join("rows/1-1-0.rows");
        let mut bytes = fs::read(&path).unwrap();
        bytes[MAGIC.len()..MAGIC.len() + 4].copy_from_slice(&7u32.to_le_bytes());
        fs::write(&path, bytes).unwrap();
        let err = read_rows(&dir, &BTreeMap::new()).unwrap_err().to_string();
        assert!(err.contains("row format version 7"), "{err}");
        assert!(
            err.contains(&format!("reads version {ROWS_FORMAT}")),
            "{err}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_canonical_bytes_leave_out_the_reasons_and_the_wall_time() {
        let a = row(4);
        let mut b = a;
        b.reasons = reason::EXTREME;
        b.duration_us += 99;
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
        let mut c = a;
        c.xg[0] += 1e-12;
        assert_ne!(a.canonical_bytes(), c.canonical_bytes());
        let one = BTreeMap::from([(a.key, a)]);
        let two = BTreeMap::from([(b.key, b)]);
        assert_eq!(rows_digest([("", &one)]), rows_digest([("", &two)]));
        assert_ne!(rows_digest([("", &one)]), rows_digest([("off", &one)]));
    }

    #[test]
    fn the_band_record_of_a_played_match_equals_the_statistics_file_field_by_field() {
        let dir =
            engine::ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"));
        let content = engine::Content::load(&dir).unwrap();
        let teams = [engine::data::TEAM_A_FILE, engine::data::TEAM_B_FILE]
            .map(|f| content.load_team(&dir, &dir.path(f)).unwrap().value);
        let config = engine::MatchConfig::new(7, 20, &content, [&teams[0], &teams[1]]).unwrap();
        let mut sim = engine::Simulation::new(config).unwrap();
        sim.run(&mut engine::NullSink).unwrap();
        let summary = sim.summary();
        let tactics = TacticsStats::new(&sim);
        let figures = MatchFigures::new(&summary, sim.managers());
        let laws = LawStats::new(&summary, 1, sim.tick(), 0);
        let row = Row::played(
            (FixtureKey::from_u64(5), 7),
            &summary,
            &tactics,
            3,
            sim.tick(),
            1_234_567,
        );
        // A real match: passes played, and possession on both sides.
        assert!(summary.passes[0] + summary.passes[1] > 0, "{summary:?}");
        assert!(
            summary.possession_ticks.iter().all(|&t| t > 0),
            "{summary:?}"
        );
        let rebuilt = row.band_record();
        assert_eq!(rebuilt.goals, summary.goals);
        assert_eq!(rebuilt.figures.goals, figures.goals);
        assert_eq!(rebuilt.figures.shots_on_target, figures.shots_on_target);
        assert_eq!(rebuilt.figures.xg, figures.xg);
        assert_eq!(rebuilt.figures.passes, figures.passes);
        assert_eq!(rebuilt.figures.passes_completed, figures.passes_completed);
        assert_eq!(rebuilt.figures.pass_accuracy_pct, figures.pass_accuracy_pct);
        assert_eq!(rebuilt.figures.possession_pct, figures.possession_pct);
        assert_eq!(rebuilt.figures.ball_in_play_s, figures.ball_in_play_s);
        assert_eq!(rebuilt.laws.fouls, laws.fouls);
        assert_eq!(rebuilt.laws.offsides, laws.offsides);
        assert_eq!(rebuilt.laws.corners, laws.corners);
        assert_eq!(rebuilt.laws.throw_ins, laws.throw_ins);
        assert_eq!(rebuilt.laws.goal_kicks, laws.goal_kicks);
        assert_eq!(rebuilt.laws.yellow, laws.yellow);
        assert_eq!(rebuilt.laws.red, laws.red);
        assert_eq!(rebuilt.laws.ticks_played, laws.ticks_played);
        assert_eq!(rebuilt.tactics.shots, tactics.shots);
        assert_eq!(rebuilt.tactics.injury_count, tactics.injury_count);
        assert_eq!(rebuilt.tactics.substitutions, tactics.substitutions);
        assert_eq!(
            rebuilt.tactics.change_never_applied,
            tactics.change_never_applied
        );
        assert_eq!(
            rebuilt.tactics.change_expired_at_full_time,
            tactics.change_expired_at_full_time
        );
        assert_eq!(rebuilt.validate_violations, 3);
        assert_eq!(rebuilt.duration_ms, 1_234);
        assert_eq!(rebuilt.outcome, "success");
        // The row keeps the unrounded expected goals.
        assert_eq!(row.xg, summary.xg);
        assert_eq!(
            Row::failed((row.key, 7), Outcome::Panic)
                .band_record()
                .outcome,
            "error"
        );
    }
}

//! Shared helpers of the record tests: the binary with a scratch data folder, and the
//! record schema files compiled by an independent JSON Schema validator.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// A scratch folder for one test, emptied first.
pub fn temp(prefix: &str, name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("{prefix}-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The binary with `data` as its data folder and the shipped content.
pub fn bin(data: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_engine-cli"));
    cmd.env("SM_DATA_DIR", data);
    cmd.env("SM_LOG", "warn");
    cmd.env(
        "SM_CONTENT_DIR",
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
    );
    cmd
}

/// The three record schemas, compiled from `schemas/observability/` without a network.
pub struct RecordSchemas {
    schemas: boon::Schemas,
    event: boon::SchemaIndex,
    stats: boon::SchemaIndex,
    report: boon::SchemaIndex,
}

impl RecordSchemas {
    pub fn load() -> Self {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/observability");
        let mut compiler = boon::Compiler::new();
        let mut schemas = boon::Schemas::new();
        let mut index = |name: &str, compiler: &mut boon::Compiler| {
            let text = std::fs::read_to_string(dir.join(name)).unwrap();
            let doc: Value = serde_json::from_str(&text).unwrap();
            let id = doc["$id"].as_str().unwrap().to_string();
            compiler.add_resource(&id, doc).unwrap();
            compiler.compile(&id, &mut schemas).unwrap()
        };
        let event = index("match-event.schema.json", &mut compiler);
        let stats = index("match-stats.schema.json", &mut compiler);
        let report = index("run-report.schema.json", &mut compiler);
        Self {
            schemas,
            event,
            stats,
            report,
        }
    }

    fn check(&self, index: boon::SchemaIndex, value: &Value) -> Result<(), String> {
        self.schemas
            .validate(value, index)
            .map_err(|e| format!("{e:#}"))
    }

    pub fn event(&self, value: &Value) -> Result<(), String> {
        self.check(self.event, value)
    }

    pub fn stats(&self, value: &Value) -> Result<(), String> {
        self.check(self.stats, value)
    }

    pub fn report(&self, value: &Value) -> Result<(), String> {
        self.check(self.report, value)
    }
}

/// The JSON object in `text`, as written: one record per line.
pub fn record(text: &str) -> Value {
    serde_json::from_str(text.trim()).unwrap()
}

/// `file` with `machine`'s hash set added and one `add-machine-set` entry, the way a ledger
/// written before the portable set records a second machine. The engine no longer writes
/// such an entry; the guard still reads it.
pub fn with_machine_set(
    mut file: engine::gate::golden::GoldenFile,
    machine: &str,
    matches: Vec<engine::gate::MatchHashes>,
    reason: &str,
) -> engine::gate::golden::GoldenFile {
    use engine::gate::golden;
    file.ledger.push(golden::LedgerEntry {
        kind: golden::EntryKind::AddMachineSet,
        reason: reason.to_string(),
        engine_version: engine::version().into(),
        build: engine::build_hash().into(),
        scheme: engine::rng::STREAM_SCHEME,
        utc: golden::utc_now(),
        machine: machine.to_string(),
        candidate: None,
        band_result: None,
    });
    file.hash_sets.insert(machine.to_string(), matches);
    file.set_differences = golden::set_differences(&file.hash_sets);
    file
}

/// Copies `from` into `to`, folders included.
pub fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A copy of the shipped content folder with `edit` applied to one of its JSON files.
pub fn edited_content(data: &Path, file: &str, edit: impl FnOnce(&mut Value)) -> PathBuf {
    let content = data.join("content");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        &content,
    );
    let path = content.join(file);
    let mut value: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    edit(&mut value);
    std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    content
}

/// One value of a compact row: an integer column, or an `f64` column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RowValue {
    Int(u64),
    Float(f64),
}

impl RowValue {
    pub fn int(self) -> u64 {
        match self {
            Self::Int(v) => v,
            Self::Float(v) => panic!("{v} is not an integer column"),
        }
    }

    pub fn float(self) -> f64 {
        match self {
            Self::Float(v) => v,
            Self::Int(v) => panic!("{v} is not an f64 column"),
        }
    }
}

/// One compact row as the test reads it: its suite, its row file, and every column by name.
#[derive(Debug, Clone)]
pub struct TestRow {
    pub suite: String,
    pub file: String,
    pub columns: std::collections::BTreeMap<String, RowValue>,
}

impl TestRow {
    pub fn get(&self, name: &str) -> RowValue {
        *self
            .columns
            .get(name)
            .unwrap_or_else(|| panic!("no column {name}"))
    }

    /// The fixture key as 16 hex characters: the start of the match identifier.
    pub fn key(&self) -> String {
        format!("{:016x}", self.get("fixture.key").int())
    }
}

/// Every row of every row file in `dir/rows/`, file by file and block by block, read by
/// the layout `docs/reference/data-files.md` gives ("The row file"), independently of the
/// program's own reader. Each block's checksum is checked; reading a file stops at a
/// partial block.
pub fn read_rows(dir: &Path) -> Vec<TestRow> {
    use sha2::{Digest, Sha256};
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir.join("rows"))
        .map(|e| e.map(|e| e.unwrap().path()).collect())
        .unwrap_or_default();
    files.sort();
    let mut out = Vec::new();
    for path in files {
        let bytes = std::fs::read(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let mut cur = Cursor {
            bytes: &bytes,
            at: 0,
        };
        let u32_of = |b: Vec<u8>| u32::from_le_bytes(b.try_into().unwrap());
        assert_eq!(cur.take(6).unwrap(), b"SMROWS", "{name}");
        assert_eq!(u32_of(cur.take(4).unwrap()), 1, "{name}: format version");
        let _measures = u32_of(cur.take(4).unwrap());
        let count = u32_of(cur.take(4).unwrap());
        let mut columns = Vec::new();
        for _ in 0..count {
            let len = cur.take(1).unwrap()[0] as usize;
            let col = String::from_utf8(cur.take(len).unwrap()).unwrap();
            let kind = cur.take(1).unwrap()[0];
            columns.push((col, kind));
        }
        loop {
            let start = cur.at;
            let Some(len) = cur.take(1) else { break };
            let suite = String::from_utf8(cur.take(len[0] as usize).unwrap()).unwrap();
            let _place = u32_of(cur.take(4).unwrap());
            let n = u32_of(cur.take(4).unwrap()) as usize;
            let mut rows: Vec<std::collections::BTreeMap<String, RowValue>> =
                vec![Default::default(); n];
            for (col, kind) in &columns {
                let width = match kind {
                    1 => 1,
                    4 => 4,
                    8 | 9 => 8,
                    other => panic!("{name}: unknown column type {other}"),
                };
                for row in &mut rows {
                    let mut word = [0u8; 8];
                    word[..width].copy_from_slice(&cur.take(width).unwrap());
                    let v = u64::from_le_bytes(word);
                    let value = if *kind == 9 {
                        RowValue::Float(f64::from_bits(v))
                    } else {
                        RowValue::Int(v)
                    };
                    row.insert(col.clone(), value);
                }
            }
            let end = cur.at;
            let sum = cur.take(8).unwrap();
            assert_eq!(
                &Sha256::digest(&bytes[start..end])[..8],
                &sum[..],
                "{name}: block checksum"
            );
            out.extend(rows.into_iter().map(|columns| TestRow {
                suite: suite.clone(),
                file: name.clone(),
                columns,
            }));
        }
    }
    out
}

/// A reader over a row file's bytes.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn take(&mut self, n: usize) -> Option<Vec<u8>> {
        let slice = self.bytes.get(self.at..self.at + n)?.to_vec();
        self.at += n;
        Some(slice)
    }
}

/// The file names in `dir`, without their extension, sorted; empty when `dir` is absent.
pub fn stems(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .map(|e| {
                    let path = e.unwrap().path();
                    path.file_stem().unwrap().to_string_lossy().into_owned()
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

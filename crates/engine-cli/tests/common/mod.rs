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

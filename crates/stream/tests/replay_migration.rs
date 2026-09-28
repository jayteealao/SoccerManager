//! The replay migration chain: the committed version-4 file lifts through two test-only
//! later formats with every frame, input, and record field kept; each broken file or broken
//! chain is refused by name, with no default filled; the committed file never changes; and
//! the steps match the shared step list that the viewer's reader is checked against too.
//!
//! The two later formats exist only in this file. Format 5 moves the engine identity out of
//! the record into its own entry (kind 4), first in the file. Format 6 merges the input
//! entries into one entry (kind 5) and renames the listed `bytes` field to `size`.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::Value;
use sha2::{Digest, Sha256};
use stream::{
    Chain, FORMAT_VERSION, Fixture, RawEntry, RawReplay, STEPS, Step, StreamError, Watchdog,
    check_frame_count, check_inputs, decode_frame, decode_record, parse_fixture,
    parse_fixture_with, split_input, write_fixture,
};

/// The SHA-256 of the committed version-4 file. The file must never change: a new format
/// adds its own committed file beside it.
const FIXTURE_SHA256: &str = "09dea6f958aeabfe3ff71173f2420674a138081465971e84caae990c8f56cd92";

const ENTRY_INPUT: u8 = 2;
const ENTRY_RECORD: u8 = 3;
/// Format 5 and later: the engine identity, as one JSON document.
const ENTRY_ENGINE: u8 = 4;
/// Format 6: every input file in one entry.
const ENTRY_INPUTS: u8 = 5;

fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../web/tests/data")
        .join(name)
}

fn fixture_bytes() -> Vec<u8> {
    std::fs::read(data("one-minute-v4.smfx")).unwrap()
}

fn fault(reason: String) -> StreamError {
    StreamError::Fixture(reason)
}

/// The place and the JSON of the record entry.
fn record_json(raw: &RawReplay) -> Result<(usize, Value), StreamError> {
    let at = raw
        .entries
        .iter()
        .position(|e| e.kind == ENTRY_RECORD)
        .ok_or_else(|| fault("the file has no record entry".into()))?;
    let json = serde_json::from_slice(&raw.entries[at].payload)
        .map_err(|e| fault(format!("the record entry is not JSON: {e}")))?;
    Ok((at, json))
}

/// Step 4→5: the record's `engine` object becomes a new first entry.
fn engine_entry(mut raw: RawReplay) -> Result<RawReplay, StreamError> {
    let (at, mut record) = record_json(&raw)?;
    let engine = record
        .as_object_mut()
        .and_then(|o| o.remove("engine"))
        .ok_or_else(|| fault("the record has no field engine".into()))?;
    raw.entries[at].payload = serde_json::to_vec(&record).unwrap();
    raw.entries.insert(
        0,
        RawEntry {
            kind: ENTRY_ENGINE,
            tick: 0,
            payload: serde_json::to_vec(&engine).unwrap(),
        },
    );
    raw.format = 5;
    Ok(raw)
}

/// Step 5→6: the input entries become one entry (a u16 count, then per file a u16 name
/// length, the name, a u32 byte length, and the bytes), and each listed input's `bytes`
/// becomes `size`.
fn input_bundle(mut raw: RawReplay) -> Result<RawReplay, StreamError> {
    let first = raw.entries.iter().position(|e| e.kind == ENTRY_INPUT);
    let mut files = Vec::new();
    for entry in raw.entries.iter().filter(|e| e.kind == ENTRY_INPUT) {
        files.push(split_input(&entry.payload)?);
    }
    let mut payload = (files.len() as u16).to_le_bytes().to_vec();
    for (name, bytes) in &files {
        payload.extend_from_slice(&(name.len() as u16).to_le_bytes());
        payload.extend_from_slice(name.as_bytes());
        payload.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        payload.extend_from_slice(bytes);
    }
    raw.entries.retain(|e| e.kind != ENTRY_INPUT);
    if let Some(first) = first {
        raw.entries.insert(
            first,
            RawEntry {
                kind: ENTRY_INPUTS,
                tick: 0,
                payload,
            },
        );
    }
    let (at, mut record) = record_json(&raw)?;
    rename_inputs(&mut record, "bytes", "size");
    raw.entries[at].payload = serde_json::to_vec(&record).unwrap();
    raw.format = 6;
    Ok(raw)
}

/// Renames `from` to `to` in each listed input. A missing field stays missing.
fn rename_inputs(record: &mut Value, from: &str, to: &str) {
    if let Some(inputs) = record.get_mut("inputs").and_then(Value::as_array_mut) {
        for input in inputs.iter_mut().filter_map(Value::as_object_mut) {
            if let Some(v) = input.remove(from) {
                input.insert(to.into(), v);
            }
        }
    }
}

/// The files of a format-6 input entry.
fn unbundle(payload: &[u8]) -> Result<Vec<(String, Vec<u8>)>, StreamError> {
    let short = || fault("the input bundle is truncated".into());
    let take = |at: &mut usize, n: usize| -> Result<&[u8], StreamError> {
        let part = payload.get(*at..*at + n).ok_or_else(short)?;
        *at += n;
        Ok(part)
    };
    let mut at = 0;
    let count = u16::from_le_bytes(take(&mut at, 2)?.try_into().unwrap());
    let mut files = Vec::new();
    for _ in 0..count {
        let len = u16::from_le_bytes(take(&mut at, 2)?.try_into().unwrap()) as usize;
        let name = String::from_utf8(take(&mut at, len)?.to_vec())
            .map_err(|_| fault("an input name is not UTF-8".into()))?;
        let size = u32::from_le_bytes(take(&mut at, 4)?.try_into().unwrap()) as usize;
        files.push((name, take(&mut at, size)?.to_vec()));
    }
    Ok(files)
}

/// Decodes a format-6 file through the reader's public pieces, so every check of the
/// production decode applies: the frames, the required record fields, and the inputs.
fn decode_v6(raw: RawReplay) -> Result<Fixture, StreamError> {
    assert_eq!(raw.format, 6);
    if raw.protocol != protocol::PROTOCOL_VERSION {
        return Err(fault(format!("frames protocol version {}", raw.protocol)));
    }
    let mut engine = None;
    let mut files = Vec::new();
    let mut record = None;
    let mut frames = Vec::new();
    for entry in &raw.entries {
        match entry.kind {
            ENTRY_ENGINE => engine = Some(entry.payload.clone()),
            ENTRY_INPUTS => files = unbundle(&entry.payload)?,
            ENTRY_RECORD => record = Some(entry.payload.clone()),
            ENTRY_INPUT => return Err(fault("an input entry in a version-6 file".into())),
            _ => frames.push(decode_frame(entry)?),
        }
    }
    let engine = engine.ok_or_else(|| fault("a version-6 file has no engine entry".into()))?;
    let record = record.ok_or_else(|| fault("a version-6 file has no record entry".into()))?;
    let mut json: Value = serde_json::from_slice(&record).unwrap();
    json.as_object_mut().unwrap().insert(
        "engine".into(),
        serde_json::from_slice(&engine).map_err(|e| fault(format!("engine entry: {e}")))?,
    );
    rename_inputs(&mut json, "size", "bytes");
    let record = decode_record(&serde_json::to_vec(&json).unwrap())?;
    let inputs = check_inputs(&record, files)?;
    let fixture = Fixture {
        format: raw.format,
        protocol_version: raw.protocol,
        match_millis: raw.match_millis,
        seed: raw.seed,
        ticks: raw.ticks,
        frames,
        inputs,
        record: Some(record),
        hash: raw.hash,
    };
    check_frame_count(&fixture, raw.frames)?;
    Ok(fixture)
}

const TEST_STEPS: [Step; 2] = [
    Step {
        from: 4,
        to: 5,
        name: "engine-entry",
        lift: engine_entry,
    },
    Step {
        from: 5,
        to: 6,
        name: "input-bundle",
        lift: input_bundle,
    },
];

fn test_chain() -> Chain<'static> {
    Chain {
        current: 6,
        steps: &TEST_STEPS,
        decode: decode_v6,
    }
}

/// A log writer the test can read back.
#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<u8>>>);

impl Write for Log {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Runs `f` on this thread with the log captured, and returns its value and the lines that
/// carry the `replay.migrated` signal.
fn with_log<T>(f: impl FnOnce() -> T) -> (T, Vec<String>) {
    let log = Log::default();
    let writer = log.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();
    let value = tracing::subscriber::with_default(subscriber, f);
    let text = String::from_utf8_lossy(&log.0.lock().unwrap()).into_owned();
    let lines = text
        .lines()
        .filter(|l| l.contains("replay.migrated"))
        .map(str::to_string)
        .collect();
    (value, lines)
}

/// The lifted file, checked against the production read of the same bytes in every field
/// but the format, with exactly one `replay.migrated` line.
fn lifts_intact(bytes: &[u8]) -> Fixture {
    let production = parse_fixture(bytes).unwrap();
    let (lifted, lines) = with_log(|| parse_fixture_with(bytes, &test_chain()));
    let lifted = lifted.unwrap();
    assert_eq!(lifted.format, 6);
    assert_eq!(
        Fixture {
            format: FORMAT_VERSION,
            ..lifted.clone()
        },
        production,
        "the lifted file differs from the production read"
    );
    assert_eq!(lines.len(), 1, "{lines:?}");
    for field in ["from=4", "to=6", "steps=2"] {
        assert!(lines[0].contains(field), "{field}: {}", lines[0]);
    }
    production
}

#[test]
fn the_committed_file_holds_what_the_migration_tests_rely_on() {
    let (fixture, lines) = with_log(|| parse_fixture(&fixture_bytes()).unwrap());
    let record = fixture.record.as_ref().unwrap();
    assert_eq!(fixture.format, FORMAT_VERSION);
    assert_eq!(fixture.inputs.len(), 9);
    assert_eq!(record.changes.len(), 2);
    let text = fixture.frames.iter().filter(|f| f.frame.is_text()).count();
    assert_eq!((fixture.frames.len() - text, text), (3000, 129));
    // A file of the current format is not lifted, so nothing is logged.
    assert!(lines.is_empty(), "{lines:?}");
}

#[test]
fn the_committed_file_lifts_through_two_later_formats_with_every_field_kept() {
    let bytes = fixture_bytes();

    // Each step changes the file's structure.
    let v5 = engine_entry(RawReplay::parse(&bytes).unwrap()).unwrap();
    assert_eq!(v5.entries[0].kind, ENTRY_ENGINE);
    assert!(record_json(&v5).unwrap().1.get("engine").is_none());
    let v6 = input_bundle(v5).unwrap();
    assert!(v6.entries.iter().all(|e| e.kind != ENTRY_INPUT));
    assert_eq!(
        v6.entries.iter().filter(|e| e.kind == ENTRY_INPUTS).count(),
        1
    );
    for input in record_json(&v6).unwrap().1["inputs"].as_array().unwrap() {
        assert!(input.get("size").is_some() && input.get("bytes").is_none());
    }

    let fixture = lifts_intact(&bytes);

    // A non-empty watchdog mark survives too.
    let mut marked = fixture;
    marked.record.as_mut().unwrap().watchdog = Watchdog {
        slow_calls: 3,
        invalid: Some("slow script".into()),
    };
    let dir = common::temp_dir("replay-migration-mark");
    let path = dir.join("marked.smfx");
    write_fixture(&path, &marked).unwrap();
    let lifted = lifts_intact(&std::fs::read(&path).unwrap());
    assert_eq!(lifted.record.unwrap().watchdog.slow_calls, 3);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Refuses `bytes` through `chain` and returns the message.
fn refused(bytes: &[u8], chain: &Chain<'_>) -> String {
    match parse_fixture_with(bytes, chain) {
        Ok(_) => panic!("the file was read"),
        Err(e) => e.to_string(),
    }
}

/// The fixture with its record JSON edited by `edit`, written with a matching trailer.
fn edited_record(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut raw = RawReplay::parse(&fixture_bytes()).unwrap();
    let (at, mut record) = record_json(&raw).unwrap();
    edit(&mut record);
    raw.entries[at].payload = serde_json::to_vec(&record).unwrap();
    raw.to_bytes()
}

#[test]
fn a_newer_format_is_refused_by_both_chains() {
    let mut newer = fixture_bytes();
    newer[4..6].copy_from_slice(&7u16.to_le_bytes());
    let err = refused(&newer, &Chain::production());
    assert!(
        err.contains("format 7 is newer than this reader knows; this build reads formats 3 and 4"),
        "{err}"
    );
    let err = refused(&newer, &test_chain());
    assert!(
        err.contains("format 7 is newer than this reader knows; this build reads formats 3 to 6"),
        "{err}"
    );
    // Control: the same bytes with format 4 read.
    newer[4..6].copy_from_slice(&4u16.to_le_bytes());
    parse_fixture(&newer).unwrap();
    parse_fixture_with(&newer, &test_chain()).unwrap();
}

fn same(raw: RawReplay) -> Result<RawReplay, StreamError> {
    Ok(raw)
}

#[test]
fn a_chain_with_a_missing_step_is_refused() {
    let gap = [
        TEST_STEPS[0],
        Step {
            from: 6,
            to: 7,
            name: "later",
            lift: same,
        },
    ];
    let chain = Chain {
        current: 7,
        steps: &gap,
        decode: decode_v6,
    };
    let err = refused(&fixture_bytes(), &chain);
    assert!(
        err.contains("no step from format 5 (it holds 4→5, 6→7)"),
        "{err}"
    );
    // Control: the contiguous chain reads.
    parse_fixture_with(&fixture_bytes(), &test_chain()).unwrap();
}

fn engine_entry_skipping(raw: RawReplay) -> Result<RawReplay, StreamError> {
    let mut raw = engine_entry(raw)?;
    raw.format = 6;
    Ok(raw)
}

#[test]
fn a_step_that_returns_the_wrong_format_is_refused() {
    let wrong = [
        Step {
            lift: engine_entry_skipping,
            ..TEST_STEPS[0]
        },
        TEST_STEPS[1],
    ];
    let chain = Chain {
        current: 6,
        steps: &wrong,
        decode: decode_v6,
    };
    let err = refused(&fixture_bytes(), &chain);
    assert!(
        err.contains("migration step 4→5 (engine-entry) returned format 6"),
        "{err}"
    );
    // Control: the correct step reads.
    parse_fixture_with(&fixture_bytes(), &test_chain()).unwrap();
}

#[test]
fn a_missing_record_field_is_refused_by_name_and_never_filled() {
    let remove = |path: &'static [&'static str]| {
        edited_record(move |record| {
            let (last, parents) = path.split_last().unwrap();
            let mut at = record;
            for key in parents {
                at = match key.parse::<usize>() {
                    Ok(i) => &mut at[i],
                    Err(_) => &mut at[*key],
                };
            }
            assert!(
                at.as_object_mut().unwrap().remove(*last).is_some(),
                "{path:?} is in the committed file"
            );
        })
    };
    let cases: [(&[&str], &str); 3] = [
        (&["watchdog", "invalid"], "missing field `invalid`"),
        (
            &["changes", "1", "change", "tactics", "formation"],
            "missing field `formation`",
        ),
        (&["engine", "scheme"], "missing field `scheme`"),
    ];
    for (path, named) in cases {
        let bytes = remove(path);
        for chain in [Chain::production(), test_chain()] {
            let err = refused(&bytes, &chain);
            assert!(err.contains(named), "{path:?}: {err}");
        }
    }
    // Control: the same edit that removes nothing reads, through both chains.
    let untouched = edited_record(|_| {});
    let read = parse_fixture(&untouched).unwrap();
    assert_eq!(read.record, parse_fixture(&fixture_bytes()).unwrap().record);
    parse_fixture_with(&untouched, &test_chain()).unwrap();
}

#[test]
fn a_corrupted_inputs_section_is_refused_by_name() {
    let raw = RawReplay::parse(&fixture_bytes()).unwrap();
    let inputs: Vec<usize> = raw
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind == ENTRY_INPUT)
        .map(|(i, _)| i)
        .collect();

    // One data byte of the second input, with a trailer hash that matches.
    let mut flipped = raw.clone();
    let payload = &mut flipped.entries[inputs[1]].payload;
    let last = payload.len() - 1;
    payload[last] ^= 0x01;
    let flipped = flipped.to_bytes();

    // The first input's name length runs past its payload.
    let mut cut = raw.clone();
    let len = cut.entries[inputs[0]].payload.len() as u16;
    cut.entries[inputs[0]].payload[0..2].copy_from_slice(&len.to_le_bytes());
    let cut = cut.to_bytes();

    for chain in [Chain::production(), test_chain()] {
        let err = refused(&flipped, &chain);
        assert!(
            err.contains("input tuning.json") && err.contains("does not match"),
            "{err}"
        );
        let err = refused(&cut, &chain);
        assert!(err.contains("name is truncated"), "{err}");
    }
    // Control: an unchanged round trip is the same file, and it reads.
    assert_eq!(raw.to_bytes(), fixture_bytes());
    parse_fixture(&raw.to_bytes()).unwrap();
    parse_fixture_with(&raw.to_bytes(), &test_chain()).unwrap();
}

#[test]
fn the_committed_version_4_file_is_byte_identical_to_its_committed_form() {
    let sha = stream::record::hex(&Sha256::digest(fixture_bytes()));
    assert_eq!(
        sha, FIXTURE_SHA256,
        "web/tests/data/one-minute-v4.smfx changed. It must never change: a new format adds \
         its own committed file beside it."
    );
}

/// `(from, to, name)` of each step.
fn listed(steps: &[Step]) -> Vec<(u16, u16, String)> {
    steps
        .iter()
        .map(|s| (s.from, s.to, s.name.to_string()))
        .collect()
}

fn shared(list: &Value) -> Vec<(u16, u16, String)> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["from"].as_u64().unwrap() as u16,
                s["to"].as_u64().unwrap() as u16,
                s["name"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn both_chains_match_the_shared_step_list() {
    let list: Value =
        serde_json::from_slice(&std::fs::read(data("replay-steps.json")).unwrap()).unwrap();
    assert_eq!(listed(STEPS), shared(&list["production"]));
    assert_eq!(listed(&TEST_STEPS), shared(&list["test"]));
    // Control: the same steps in the other order do not match.
    let swapped = [TEST_STEPS[1], TEST_STEPS[0]];
    assert_ne!(listed(&swapped), shared(&list["test"]));
}

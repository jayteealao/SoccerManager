//! The replay gate is sensitive and strict: a fault in any state-inventory group fails the
//! gate at the window of the fault, a one-tick fault still fails it, a NaN names its field,
//! a broken golden file is refused with the fault named, the hashes repeat in one process
//! and match the committed golden file, and the stream state has the documented bytes. The
//! golden file ledger: a regeneration appends one entry with its reason (AC-7), two hash sets
//! record which matches differ (AC-GG-1), a broken ledger is refused, and the first file form
//! reads as a one-entry bootstrap ledger.
//!
//! The faulted runs compare with a clean run of the same fixture in the same process, so
//! they do not depend on this machine's hash set.

mod common;

use std::path::Path;
use std::sync::OnceLock;

use engine::Content;
use engine::data::TeamFile;
use engine::gate::fault::{Fault, FaultKind, play_faulted};
use engine::gate::golden::{self, GoldenFile};
use engine::gate::{
    self, Checkpoint, Fixture, GateError, Inputs, MatchHashes, Played, Verdict, compare,
    report_line,
};
use engine::rng::EngineRng;

/// The fault tick of the charter step: tick 30,000 of seed 42.
const FAULT_TICK: u32 = 30_000;

fn loaded() -> &'static (Content, [TeamFile; 2]) {
    static LOADED: OnceLock<(Content, [TeamFile; 2])> = OnceLock::new();
    LOADED.get_or_init(|| {
        let content = common::content();
        let teams = common::default_teams(&content);
        (content, teams)
    })
}

fn inputs() -> Inputs<'static> {
    let (content, [a, b]) = loaded();
    Inputs {
        content,
        teams: [a, b],
        pack: None,
    }
}

/// The clean seed-42 run, played once per test binary.
fn clean() -> &'static Played {
    static CLEAN: OnceLock<Played> = OnceLock::new();
    CLEAN.get_or_init(|| gate::play_fixture(&Fixture::seed(42), &inputs()).unwrap())
}

/// Plays seed 42 with `kind` at tick 30,000 and returns the report and the verdict.
fn faulted(kind: FaultKind) -> (String, Verdict) {
    let fixture = Fixture::seed(42);
    let fault = Fault {
        at_tick: FAULT_TICK,
        kind,
    };
    let played = play_faulted(&fixture, &inputs(), fault, &clean().hashes).unwrap();
    assert!(
        played.stopped,
        "{kind:?}: the run stops at the first difference"
    );
    let verdict = compare(&clean().hashes, &played.hashes);
    (report_line(&fixture, &played, verdict), verdict)
}

const WINDOW: Verdict = Verdict::Differs {
    from: 30_000,
    to: 31_000,
};

#[test]
fn velocity_bit_fault() {
    let (report, verdict) = faulted(FaultKind::VelocityBit { player: 3, bit: 51 });
    println!("{report}");
    assert_eq!(verdict, WINDOW, "{report}");
    assert!(report.starts_with("seed-42 "), "{report}");
    assert!(report.contains(" differs "), "{report}");
    assert!(
        report
            .contains("seed-42 differs: the state first differs between tick 30000 and tick 31000"),
        "{report}"
    );
}

#[test]
fn a_fault_in_each_inventory_group_fails_at_its_window() {
    for kind in [
        FaultKind::ExtraDraw,
        FaultKind::Stamina {
            player: 3,
            delta: -0.01,
        },
        FaultKind::Card { player: 3 },
        FaultKind::Restart { player: 3 },
        FaultKind::PendingChange,
    ] {
        let (report, verdict) = faulted(kind);
        assert_eq!(verdict, WINDOW, "{kind:?}: {report}");
        assert!(
            report.contains("between tick 30000 and tick 31000"),
            "{kind:?}: {report}"
        );
    }
}

#[test]
fn a_fault_that_lasts_one_tick_still_fails() {
    let (report, verdict) = faulted(FaultKind::OneTickCard { player: 3 });
    assert_eq!(verdict, WINDOW, "{report}");
}

#[test]
fn a_nan_fails_the_gate_naming_the_field() {
    let fault = Fault {
        at_tick: FAULT_TICK,
        kind: FaultKind::Nan { player: 3 },
    };
    let err = play_faulted(&Fixture::seed(42), &inputs(), fault, &clean().hashes).unwrap_err();
    let GateError::NonFinite {
        fixture,
        tick,
        field,
    } = &err
    else {
        panic!("a NaN is a non-finite error: {err}");
    };
    assert_eq!(fixture, "seed-42");
    assert_eq!(*tick, FAULT_TICK + 1);
    println!("{err}");
    assert_eq!(field, "players[3].pos.x");
    assert!(err.to_string().contains("players[3].pos.x"), "{err}");
}

#[test]
fn two_runs_in_one_process_and_the_golden_file_agree() {
    let again = gate::play_fixture(&Fixture::seed(42), &inputs()).unwrap();
    assert_eq!(again.hashes, clean().hashes);
    assert_eq!(compare(&clean().hashes, &again.hashes), Verdict::Same);
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gate/golden.json");
    let file = golden::load(&path, &gate::fixtures()).expect("the committed golden file loads");
    let machine = golden::machine_key();
    match file.set_for(&machine) {
        Ok(set) => {
            let stored = set.iter().find(|m| m.id == "seed-42").unwrap();
            assert_eq!(
                stored, &again.hashes,
                "the bootstrap process wrote other hashes"
            );
        }
        Err(err) => println!("note: skipping the golden-file compare: {err}"),
    }
}

/// A valid golden file whose every match lasts 2,500 ticks.
fn synthetic() -> (GoldenFile, Vec<Fixture>) {
    let fixtures = gate::fixtures();
    let hash = |n: u32| format!("{n:064x}");
    let matches = fixtures
        .iter()
        .map(|f| MatchHashes {
            id: f.id.clone(),
            ticks: 2_500,
            final_hash: hash(3),
            checkpoints: vec![
                Checkpoint {
                    tick: 1_000,
                    hash: hash(1),
                },
                Checkpoint {
                    tick: 2_000,
                    hash: hash(2),
                },
                Checkpoint {
                    tick: 2_500,
                    hash: hash(3),
                },
            ],
        })
        .collect();
    let machine = golden::machine_key();
    (
        GoldenFile::first(&fixtures, &machine, matches, golden::BOOTSTRAP_REASON),
        fixtures,
    )
}

/// The matches of `machine`'s hash set in a golden file's JSON.
fn set<'a>(v: &'a mut serde_json::Value, machine: &str) -> &'a mut Vec<serde_json::Value> {
    v["hash_sets"][machine].as_array_mut().unwrap()
}

fn refused(file: &serde_json::Value, fixtures: &[Fixture]) -> String {
    golden::parse(&file.to_string(), fixtures)
        .expect_err("the broken file is refused")
        .to_string()
}

#[test]
fn a_broken_golden_file_is_refused_naming_the_fault() {
    let (file, fixtures) = synthetic();
    let text = serde_json::to_string_pretty(&file).unwrap();
    assert_eq!(golden::parse(&text, &fixtures).unwrap(), file);
    let machine = golden::machine_key();
    let base: serde_json::Value = serde_json::from_str(&text).unwrap();

    let mut missing = base.clone();
    set(&mut missing, &machine).remove(4);
    assert_eq!(
        refused(&missing, &fixtures),
        format!("golden file hash set {machine}: match seed-2026 is missing")
    );

    let mut duplicate = base.clone();
    let first = set(&mut duplicate, &machine)[0].clone();
    set(&mut duplicate, &machine).push(first);
    assert_eq!(
        refused(&duplicate, &fixtures),
        format!("golden file hash set {machine}: match seed-42 appears more than once")
    );

    let mut gap = base.clone();
    set(&mut gap, &machine)[1]["checkpoints"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    let why = refused(&gap, &fixtures);
    assert!(
        why.starts_with(&format!(
            "golden file hash set {machine}: match seed-1: missing checkpoint: no checkpoint at tick 2000"
        )),
        "{why}"
    );

    let mut schema = base.clone();
    schema["gate_schema"] = 2.into();
    assert_eq!(
        refused(&schema, &fixtures),
        "golden file has gate schema 2; this build reads gate schema 1"
    );

    let why = golden::parse(&text[..text.len() / 2], &fixtures)
        .unwrap_err()
        .to_string();
    assert!(why.starts_with("golden file is malformed: "), "{why}");

    // A wrong tick count loads, and the compare names it.
    let mut longer = base.clone();
    let entry = &mut set(&mut longer, &machine)[0];
    entry["ticks"] = 2_600.into();
    entry["checkpoints"][2]["tick"] = 2_600.into();
    let file = golden::parse(&longer.to_string(), &fixtures).unwrap();
    let stored = &file.set_for(&machine).unwrap()[0];
    let (synthetic, _) = synthetic();
    let played = synthetic.set_for(&machine).unwrap()[0].clone();
    let verdict = compare(stored, &played);
    assert_eq!(
        verdict,
        Verdict::TickCount {
            expected: 2_600,
            actual: 2_500
        }
    );
    let report = report_line(
        &fixtures[0],
        &Played {
            hashes: played,
            facts: Default::default(),
            stopped: false,
        },
        verdict,
    );
    assert!(
        report.contains("seed-42 differs: the golden file has 2600 ticks, this run played 2500"),
        "{report}"
    );
}

#[test]
fn the_fixture_list_is_fixed() {
    let fixtures = gate::fixtures();
    let ids: Vec<&str> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(fixtures.len(), 22);
    assert_eq!(
        &ids[..7],
        &[
            "seed-42",
            "seed-1",
            "seed-7",
            "seed-99",
            "seed-2026",
            "seed-0",
            "seed-18446744073709551615"
        ]
    );
    assert_eq!(&ids[20..], &["change", "knockout"]);
    assert_eq!(fixtures[21].seed, gate::KNOCKOUT_SEED);
    let toolchain = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rust-toolchain.toml"),
    )
    .unwrap();
    assert!(
        toolchain.contains(&format!("channel = \"{}\"", golden::TOOLCHAIN)),
        "the golden file records the pinned toolchain"
    );
}

#[test]
fn the_stream_state_is_scheme_0_with_one_entry() {
    let mut rng = EngineRng::from_seed(42);
    rng.next_f64();
    let state = rng.stream_state();
    assert_eq!(state.scheme, 0);
    assert_eq!(state.entries, vec![(0, 2)]);
    let bytes = state.to_bytes();
    assert_eq!(bytes[0], 0, "the scheme id");
    assert_eq!(&bytes[1..5], &1u32.to_le_bytes(), "the entry count");
    assert_eq!(&bytes[5..13], &0u64.to_le_bytes(), "the stream id");
    assert_eq!(&bytes[13..29], &2u128.to_le_bytes(), "the word position");
    assert_eq!(bytes.len(), 29);
}

/// `matches` with the final hash (and the last checkpoint) of each match in `ids` replaced.
fn with_changed(mut matches: Vec<MatchHashes>, ids: &[&str], n: u32) -> Vec<MatchHashes> {
    for m in matches.iter_mut().filter(|m| ids.contains(&m.id.as_str())) {
        m.final_hash = format!("{n:064x}");
        m.checkpoints.last_mut().unwrap().hash = m.final_hash.clone();
    }
    matches
}

#[test]
fn a_regeneration_writes_the_new_hashes_and_appends_one_entry() {
    let (file, fixtures) = synthetic();
    let machine = golden::machine_key();
    let old = file.set_for(&machine).unwrap().to_vec();
    let new = with_changed(old.clone(), &["seed-42", "knockout"], 9);
    assert_ne!(new, old);
    let (regen, dropped) = file.clone().regenerated(
        &fixtures,
        &machine,
        new.clone(),
        "the maths library replaces the platform sin and cos",
    );
    assert!(dropped.is_empty());
    assert_eq!(regen.set_for(&machine).unwrap(), new.as_slice());
    assert_eq!(regen.ledger.len(), file.ledger.len() + 1);
    assert_eq!(
        regen.ledger[0], file.ledger[0],
        "the old entry is unchanged"
    );
    let entry = &regen.ledger[1];
    assert_eq!(entry.kind, golden::EntryKind::Regenerate);
    assert_eq!(
        entry.reason,
        "the maths library replaces the platform sin and cos"
    );
    assert_eq!(entry.engine_version, engine::version());
    assert_eq!(entry.scheme, 0);
    assert_eq!(entry.scheme, engine::rng::STREAM_SCHEME);
    assert_eq!(entry.machine, machine);
    assert_eq!(entry.candidate.as_deref(), Some(engine::build_hash()));
    assert_eq!(
        entry.band_result.as_deref(),
        Some("gate/bands/ledger-1.json")
    );
    // The written text loads strictly and reads back equal.
    let loaded = golden::parse(&regen.to_text(), &fixtures).unwrap();
    assert_eq!(loaded, regen);

    // A regeneration drops the other machines' sets, which are stale after a hash change.
    let two = file
        .with_machine_set("linux-other", old, "second machine")
        .unwrap();
    let (regen, dropped) = two.regenerated(&fixtures, &machine, new, "a hash change");
    assert_eq!(dropped, vec!["linux-other".to_string()]);
    assert_eq!(regen.hash_sets.len(), 1);
    assert_eq!(
        regen.ledger[2].band_result.as_deref(),
        Some("gate/bands/ledger-2.json")
    );
    golden::parse(&regen.to_text(), &fixtures).unwrap();
}

#[test]
fn two_hash_sets_record_which_matches_differ() {
    let (file, fixtures) = synthetic();
    let machine = golden::machine_key();
    let other = with_changed(file.set_for(&machine).unwrap().to_vec(), &["seed-7"], 7);
    let two = file
        .with_machine_set("zz-other", other, "a second machine")
        .unwrap();
    assert_eq!(two.set_differences.len(), 1);
    let d = &two.set_differences[0];
    assert_eq!((d.a.as_str(), d.b.as_str()), (machine.as_str(), "zz-other"));
    assert_eq!(d.differ, vec!["seed-7".to_string()]);
    assert_eq!(d.same, 21);
    let entry = two.ledger.last().unwrap();
    assert_eq!(entry.kind, golden::EntryKind::AddMachineSet);
    assert_eq!(entry.candidate, None);
    assert_eq!(entry.band_result, None);

    // Read back, the record is the same.
    let text = two.to_text();
    let loaded = golden::parse(&text, &fixtures).unwrap();
    assert_eq!(loaded.set_differences, two.set_differences);

    // An edited record is refused.
    let mut edited: serde_json::Value = serde_json::from_str(&text).unwrap();
    edited["set_differences"][0]["differ"] = serde_json::json!([]);
    edited["set_differences"][0]["same"] = 22.into();
    assert_eq!(
        refused(&edited, &fixtures),
        "golden file set_differences: the record differs from the one the hash sets give"
    );

    // A second set for the same machine is refused.
    let again = loaded
        .with_machine_set("zz-other", Vec::new(), "again")
        .unwrap_err();
    assert_eq!(
        again.to_string(),
        "golden file already has a hash set for this machine (zz-other)"
    );
}

#[test]
fn a_broken_ledger_is_refused_naming_the_fault() {
    let (file, fixtures) = synthetic();
    let machine = golden::machine_key();
    let old = file.set_for(&machine).unwrap().to_vec();
    let (regen, _) = file.regenerated(&fixtures, &machine, old, "a reason");
    let base: serde_json::Value = serde_json::from_str(&regen.to_text()).unwrap();
    let with = |f: &dyn Fn(&mut serde_json::Value)| {
        let mut v = base.clone();
        f(&mut v);
        refused(&v, &fixtures)
    };

    assert_eq!(
        with(&|v| v["ledger"] = serde_json::json!([])),
        "golden file ledger: it is empty; the first entry must be a bootstrap"
    );
    assert_eq!(
        with(&|v| v["ledger"][0]["kind"] = "add-machine-set".into()),
        "golden file ledger: entry 0 is add-machine-set; the first entry must be a bootstrap"
    );
    assert_eq!(
        with(&|v| {
            let first = v["ledger"][0].clone();
            v["ledger"].as_array_mut().unwrap().push(first);
        }),
        "golden file ledger: entry 2 is a second bootstrap"
    );
    assert_eq!(
        with(&|v| v["ledger"][1]["reason"] = "   ".into()),
        "golden file ledger: entry 1 (regenerate) has no reason"
    );
    assert_eq!(
        with(&|v| v["ledger"][1]["band_result"] = "gate/bands/mine.json".into()),
        "golden file ledger: entry 1 (regenerate) must have band_result \"gate/bands/ledger-1.json\""
    );
    let why = with(&|v| {
        let set = v["hash_sets"][&machine].clone();
        v["hash_sets"]["zz-unrecorded"] = set;
    });
    assert!(
        why.starts_with(&format!(
            "golden file ledger: the hash sets are {machine}, zz-unrecorded, but the entries"
        )),
        "{why}"
    );
    let why = with(&|v| v["ledger"][1]["unknown_field"] = 1.into());
    assert!(why.starts_with("golden file is malformed: "), "{why}");
}

#[test]
fn the_first_file_form_reads_as_a_one_entry_bootstrap_ledger() {
    let (file, fixtures) = synthetic();
    let machine = golden::machine_key();
    let mut v: serde_json::Value = serde_json::from_str(&file.to_text()).unwrap();
    let obj = v.as_object_mut().unwrap();
    obj.remove("ledger");
    obj.remove("set_differences");
    obj.insert(
        "bootstrap".into(),
        serde_json::json!({
            "engine_version": "0.1.0",
            "build": "d3cd95d",
            "utc": "2026-09-26T13:47:07Z",
            "machine": machine,
        }),
    );
    let read = golden::parse(&v.to_string(), &fixtures).unwrap();
    assert_eq!(read.ledger.len(), 1);
    let e = &read.ledger[0];
    assert_eq!(e.kind, golden::EntryKind::Bootstrap);
    assert_eq!(e.reason, golden::BOOTSTRAP_REASON);
    assert_eq!(
        (e.engine_version.as_str(), e.build.as_str(), e.utc.as_str()),
        ("0.1.0", "d3cd95d", "2026-09-26T13:47:07Z")
    );
    assert_eq!(e.scheme, 0);
    assert!(read.set_differences.is_empty());
    // Every write uses the ledger form.
    let text = read.to_text();
    assert!(text.contains("\"ledger\""));
    assert!(!text.contains("\"bootstrap\": {"));

    // Both forms at once are refused.
    v["ledger"] = serde_json::json!([]);
    assert_eq!(
        refused(&v, &fixtures),
        "golden file has both a bootstrap object and a ledger; it must have one"
    );
}

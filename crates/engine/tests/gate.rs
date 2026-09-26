//! The replay gate is sensitive and strict: a fault in any state-inventory group fails the
//! gate at the window of the fault, a one-tick fault still fails it, a NaN names its field,
//! a broken golden file is refused with the fault named, the hashes repeat in one process
//! and match the committed golden file, and the stream state has the documented bytes.
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
    (GoldenFile::first(&fixtures, matches), fixtures)
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

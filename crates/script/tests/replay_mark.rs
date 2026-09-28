//! AC-19, replay part: a match whose script calls ran past the wall-clock limit carries the
//! watchdog mark in its replay file, and its twin with no clock carries none. The controlled
//! test clock forces the hit.

use std::path::Path;

use engine::data::{TEAM_A_FILE, TEAM_B_FILE};
use engine::record::TickSink;
use engine::{Content, ContentDir, MatchConfig, Simulation};
use script::sandbox::CALL_BACKSTOP;
use script::{Backstop, LoadedPack};
use stream::session::FrameSink;
use stream::{
    EngineIdentity, InputFile, ManagerKind, MatchSettings, Recorder, SharedRecorder, Watchdog,
    outcome_of, read_fixture,
};

/// Records a five-minute match with the sample pack under `backstop` and returns the
/// watchdog mark its replay file carries.
fn recorded_mark(name: &str, backstop: Backstop) -> Watchdog {
    let dir = ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"));
    let content = Content::load(&dir).unwrap();
    let a = content
        .load_team(&dir, &dir.path(TEAM_A_FILE))
        .unwrap()
        .value;
    let b = content
        .load_team(&dir, &dir.path(TEAM_B_FILE))
        .unwrap()
        .value;
    let pack_dir = dir.path("scripts/sample");
    let pack = LoadedPack::load_with(&pack_dir, backstop).unwrap();
    let mut config = MatchConfig::new(engine::gate::KNOCKOUT_SEED, 5, &content, [&a, &b]).unwrap();
    config.fold_pack_hash(pack.sha());
    let inputs = [
        InputFile {
            role: "pack_manifest".into(),
            name: "pack.json".into(),
            bytes: std::fs::read(pack_dir.join("pack.json")).unwrap(),
        },
        InputFile {
            role: "pack_script".into(),
            name: pack.pack.manifest.entry.clone(),
            bytes: std::fs::read(pack_dir.join(&pack.pack.manifest.entry)).unwrap(),
        },
    ];
    let path = std::env::temp_dir().join(format!(
        "script-replay-mark-{}-{name}.smfx",
        std::process::id()
    ));
    let recorder = SharedRecorder::new(
        Recorder::create_record(
            &path,
            1,
            EngineIdentity::current().unwrap(),
            MatchSettings {
                seed: engine::gate::KNOCKOUT_SEED,
                minutes: 5,
                knockout: false,
                managers: [ManagerKind::Ai, ManagerKind::Ai],
            },
            &inputs,
        )
        .unwrap(),
    );
    let mut sink = FrameSink::new(recorder.clone(), 50);
    let mut sim = Simulation::new(config).unwrap();
    sim.set_plugins(pack.plugins());
    while !sim.is_over() {
        sim.step();
        sink.on_tick(&sim.record()).unwrap();
    }
    sim.finish();
    drop(sink);
    recorder.finish_record(outcome_of(&sim)).unwrap();
    let record = read_fixture(&path).unwrap().record.unwrap();
    std::fs::remove_file(&path).unwrap();
    record.watchdog
}

#[test]
fn a_forced_timer_hit_puts_the_invalid_mark_in_the_replay_file() {
    let marked = recorded_mark(
        "slow",
        Backstop::Skewed {
            limit: Some(CALL_BACKSTOP),
            only_call: None,
        },
    );
    assert_eq!(marked.invalid.as_deref(), Some("slow script"), "{marked:?}");
    assert!(marked.slow_calls > 0, "{marked:?}");

    // The twin reads no clock and carries no mark.
    let clean = recorded_mark("never", Backstop::Never);
    assert_eq!(clean, Watchdog::default());
}

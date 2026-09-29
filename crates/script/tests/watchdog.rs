//! The watchdog mark: a script call past the 2 ms wall-clock limit is not stopped. It keeps
//! its result, marks the match invalid (`slow script`), and logs one `match.invalid` line;
//! the match, its hashes, and its failure counts stay as they are without the hit. The
//! operation budget still aborts a long call, and three aborts in a row still switch the
//! hook off. The controlled test clock (`Backstop::Skewed`) forces a hit without real
//! waiting; every forced run has an unforced twin that plays with no clock at all.

mod common;

use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use engine::data::{TEAM_A_FILE, TEAM_B_FILE, TeamFile};
use engine::gate::{self, Fixture, Inputs, PackInputs, Played, Verdict, golden};
use engine::observe::ScriptFigures;
use engine::plugin::{HookPoint, ScriptOutcome};
use engine::{Content, ContentDir, EngineEvent, EngineEventKind, EventDetail, Simulation};
use script::sandbox::CALL_BACKSTOP;
use script::{Backstop, LoadedPack};

/// Every call past the limit on the test clock.
const EVERY_CALL_SLOW: Backstop = Backstop::Skewed {
    limit: Some(CALL_BACKSTOP),
    only_call: None,
};
/// Only the first call after the load past the limit on the test clock.
const FIRST_CALL_SLOW: Backstop = Backstop::Skewed {
    limit: Some(CALL_BACKSTOP),
    only_call: Some(1),
};

fn loaded() -> &'static (ContentDir, Content, [TeamFile; 2]) {
    static LOADED: OnceLock<(ContentDir, Content, [TeamFile; 2])> = OnceLock::new();
    LOADED.get_or_init(|| {
        let dir = ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"));
        let content = Content::load(&dir).expect("the shipped content loads");
        let a = content
            .load_team(&dir, &dir.path(TEAM_A_FILE))
            .unwrap()
            .value;
        let b = content
            .load_team(&dir, &dir.path(TEAM_B_FILE))
            .unwrap()
            .value;
        (dir, content, [a, b])
    })
}

/// The knockout gate fixture with the sample pack under `backstop`. Each call loads the
/// pack afresh, so a test clock's call numbers start at 1.
fn knockout(backstop: Backstop) -> Played {
    let (dir, content, [a, b]) = loaded();
    let pack = LoadedPack::load_with(&dir.path("scripts/sample"), backstop).unwrap();
    let hooks = || pack.plugins();
    let inputs = Inputs {
        content,
        teams: [a, b],
        pack: Some(PackInputs {
            id: &pack.pack.manifest.id,
            sha: *pack.sha(),
            hooks: &hooks,
        }),
    };
    gate::play_fixture(&Fixture::knockout(gate::KNOCKOUT_SEED), &inputs).unwrap()
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
/// carry the `match.invalid` signal. The subscriber is thread-local, so `f` must play its
/// match on this thread.
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
        .filter(|l| l.contains("match.invalid"))
        .map(str::to_string)
        .collect();
    (value, lines)
}

/// A whole match of `minutes` with the fixture pack `name` under `backstop`.
fn play_pack(
    minutes: u32,
    name: &str,
    backstop: Backstop,
) -> (Simulation, Vec<engine::TickRecord>, Vec<EngineEvent>) {
    let pack = LoadedPack::load_with(&common::fixture(name), backstop).unwrap();
    common::play(minutes, Some(&pack))
}

/// The `script` events of a match: tick, hook, outcome, and detail text.
fn notes(sim: &Simulation, events: &[EngineEvent]) -> Vec<(u32, HookPoint, ScriptOutcome, String)> {
    events
        .iter()
        .filter(|e| e.kind == EngineEventKind::Script)
        .map(|e| {
            let Some(EventDetail::Script(note)) = e.detail else {
                panic!("a script event carries its note: {e:?}");
            };
            (
                e.tick,
                note.hook,
                note.outcome,
                sim.plugins().detail(&note).to_string(),
            )
        })
        .collect()
}

#[test]
fn one_slow_call_marks_the_match_and_changes_nothing() {
    // One slow call, on the knockout gate fixture.
    let unforced = knockout(Backstop::Never);
    let (forced, lines) = with_log(|| knockout(FIRST_CALL_SLOW));
    assert!(unforced.facts.script_calls > 0, "the pack's hooks run");
    assert_eq!(unforced.facts.slow_calls, 0);
    assert_eq!(forced.facts.slow_calls, 1, "exactly the first call is slow");
    assert_eq!(forced.facts.script_calls, unforced.facts.script_calls);
    assert_eq!(forced.facts.script_aborts, 0);
    assert_eq!(
        forced.hashes, unforced.hashes,
        "the hash is the undelayed one"
    );
    assert_eq!(lines.len(), 1, "one match.invalid line: {lines:?}");
    assert!(lines[0].contains("slow script"), "{lines:?}");

    // One slow call in a whole 90-minute match: the mark is on the match result.
    let (plain_sim, plain_records, plain_events) = play_pack(90, "shoot-bias", Backstop::Never);
    let ((sim, records, events), lines) = with_log(|| play_pack(90, "shoot-bias", FIRST_CALL_SLOW));
    let figures = ScriptFigures::new(sim.plugins());
    assert_eq!(figures.invalid.as_deref(), Some("slow script"));
    assert_eq!(figures.slow_calls, Some(1));
    assert_eq!(ScriptFigures::new(plain_sim.plugins()).invalid, None);
    assert_eq!(records, plain_records);
    assert_eq!(events, plain_events);
    assert_eq!(lines.len(), 1, "{lines:?}");
}

#[test]
fn every_call_slow_never_switches_a_hook_off() {
    // Every call slow, on the knockout gate fixture.
    let unforced = knockout(Backstop::Never);
    let forced = knockout(EVERY_CALL_SLOW);
    assert!(forced.facts.script_calls > 0);
    assert_eq!(
        forced.facts.slow_calls, forced.facts.script_calls,
        "every call is slow"
    );
    assert_eq!(forced.facts.script_aborts, 0);
    assert_eq!(forced.hashes, unforced.hashes);

    // Every call slow in a whole 90-minute match.
    let (plain_sim, plain_records, _) = play_pack(90, "shoot-bias", Backstop::Never);
    let ((sim, records, _), lines) = with_log(|| play_pack(90, "shoot-bias", EVERY_CALL_SLOW));
    let p = sim.plugins();
    assert!(p.stats.calls > 3, "more calls than the switch-off count");
    assert_eq!(p.slow_calls(), p.stats.calls);
    assert_eq!((p.stats.aborts, p.stats.disabled), (0, 0));
    assert_eq!(p.failures(), [0, 0, 0]);
    assert_eq!(p.hooks_present(), [true, false, false], "the hook stays on");
    assert_eq!(p.hooks_present(), plain_sim.plugins().hooks_present());
    assert_eq!(p.stats, plain_sim.plugins().stats);
    assert_eq!(records, plain_records);
    assert_eq!(
        lines.len(),
        1,
        "one line per match, not per call: {lines:?}"
    );
}

#[test]
fn the_operation_budget_still_aborts_with_or_without_a_hit() {
    // The `looping` pack exhausts its operation budget on every call.
    let (_, plain, _) = common::play(10, None);
    let runs = [Backstop::Never, EVERY_CALL_SLOW].map(|backstop| {
        let (sim, records, events) = play_pack(10, "looping", backstop);
        let notes = notes(&sim, &events);
        (sim, records, notes)
    });
    for (sim, records, notes) in &runs {
        assert_eq!(
            notes.len(),
            4,
            "three aborts, then the switch-off: {notes:?}"
        );
        for note in &notes[..3] {
            assert_eq!(
                (note.1, note.2, note.3.as_str()),
                (
                    HookPoint::Decision,
                    ScriptOutcome::Aborted,
                    "operation budget of 10000 exhausted"
                )
            );
        }
        assert_eq!(notes[3].2, ScriptOutcome::Disabled);
        let s = sim.plugins().stats;
        assert_eq!((s.calls, s.aborts, s.disabled), (3, 3, 1));
        assert_eq!(records, &plain, "every abort left the engine's own choice");
    }
    let [(without, _, without_notes), (with, _, with_notes)] = &runs;
    assert_eq!(without_notes, with_notes, "the same events and texts");
    assert_eq!(without.plugins().failures(), with.plugins().failures());
    assert_eq!(
        without.plugins().hooks_present(),
        with.plugins().hooks_present()
    );
    assert_eq!(without.plugins().slow_calls(), 0);
    assert_eq!(
        with.plugins().slow_calls(),
        3,
        "the mark only accompanies the aborts"
    );

    // The shipped clock gives the same events whether or not this machine hits the limit.
    let (sim, records, events) = play_pack(10, "looping", Backstop::default());
    assert_eq!(&notes(&sim, &events), without_notes);
    assert_eq!(records, plain);
}

#[test]
fn a_marked_gate_match_warns_and_is_still_compared() {
    // At the library level: the release command cannot force the clock.
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gate/golden.json");
    let file = golden::load(&path, &gate::fixtures()).expect("the committed golden file loads");
    let Ok(set) = file.set_for(&golden::set_key()) else {
        println!("note: skipping: no hash set for {}", golden::set_key());
        return;
    };
    let fixture = Fixture::knockout(gate::KNOCKOUT_SEED);
    let expected = set
        .iter()
        .find(|m| m.id == fixture.id)
        .expect("the golden set has the knockout fixture");

    let forced = knockout(EVERY_CALL_SLOW);
    assert_eq!(gate::compare(expected, &forced.hashes), Verdict::Same);
    let warning = gate::warning_line(&fixture, &forced).expect("a marked match warns");
    assert!(
        warning.contains("knockout") && warning.contains("slow script"),
        "{warning}"
    );

    // Control: the unforced match warns about nothing.
    let unforced = knockout(Backstop::Never);
    assert_eq!(gate::compare(expected, &unforced.hashes), Verdict::Same);
    assert_eq!(gate::warning_line(&fixture, &unforced), None);

    // Control: a warning never replaces the compare. Against altered golden hashes the
    // marked match still differs.
    let mut altered = expected.clone();
    let last = altered.checkpoints.last_mut().unwrap();
    last.hash = "0".repeat(64);
    assert!(matches!(
        gate::compare(&altered, &forced.hashes),
        Verdict::Differs { .. }
    ));
    assert!(gate::warning_line(&fixture, &forced).is_some());
}

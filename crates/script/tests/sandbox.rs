//! A script that exceeds its budget is aborted, logged, and replaced by the engine's own
//! decision; a script that tries to read a file or reach the network is denied and the
//! denial is recorded; either way the match plays on to full time.

mod common;

use std::io::Write;
use std::sync::{Arc, Mutex};

use common::{fixture, pack, play};
use engine::plugin::{HookPoint, ScriptOutcome};
use engine::{EngineEvent, EngineEventKind, EventDetail, Simulation};
use script::LoadedPack;

/// The `script` events of a match: hook, outcome, and detail.
fn notes(sim: &Simulation, events: &[EngineEvent]) -> Vec<(HookPoint, ScriptOutcome, String)> {
    events
        .iter()
        .filter(|e| e.kind == EngineEventKind::Script)
        .map(|e| {
            let Some(EventDetail::Script(note)) = e.detail else {
                panic!("a script event carries its note: {e:?}");
            };
            (
                note.hook,
                note.outcome,
                sim.plugins().detail(&note).to_string(),
            )
        })
        .collect()
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

impl Log {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

/// Plays `minutes` with the pack while capturing the log.
fn logged(
    minutes: u32,
    pack: &LoadedPack,
) -> (
    Simulation,
    Vec<engine::TickRecord>,
    Vec<EngineEvent>,
    String,
) {
    let log = Log::default();
    let writer = log.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();
    let (sim, records, events) =
        tracing::subscriber::with_default(subscriber, || play(minutes, Some(pack)));
    (sim, records, events, log.text())
}

#[test]
fn a_looping_decision_hook_is_aborted_logged_and_the_native_decision_stands() {
    let (_, plain, _) = play(10, None);
    let (sim, records, events, log) = logged(10, &pack("looping"));
    assert!(sim.is_over(), "the match reached full time");
    let notes = notes(&sim, &events);
    assert_eq!(
        notes.len(),
        4,
        "three aborts, then the hook is switched off: {notes:?}"
    );
    for note in &notes[..3] {
        assert_eq!(
            note,
            &(
                HookPoint::Decision,
                ScriptOutcome::Aborted,
                "operation budget of 10000 exhausted".to_string()
            )
        );
    }
    assert_eq!(notes[3].1, ScriptOutcome::Disabled);
    // Every aborted call left the engine's own choice, so the match is the unscripted one.
    assert_eq!(records, plain);
    assert!(log.contains("script.aborted"), "{log}");
    assert!(log.contains("script.disabled"), "{log}");
    let stats = sim.plugins().stats;
    assert_eq!((stats.calls, stats.aborts, stats.disabled), (3, 3, 1));
}

#[test]
fn an_import_is_denied_and_recorded_and_the_match_goes_on() {
    let (sim, _, events, log) = logged(10, &pack("import-escape"));
    assert!(sim.is_over());
    let notes = notes(&sim, &events);
    assert_eq!(
        notes[0],
        (
            HookPoint::Decision,
            ScriptOutcome::Denied,
            "import ../../../Cargo is not allowed".to_string()
        )
    );
    assert!(log.contains("script.denied"), "{log}");
    assert_eq!(sim.plugins().stats.denials, 3);
}

#[test]
fn a_network_call_is_denied_naming_the_function() {
    let (sim, _, events, _) = logged(10, &pack("network-call"));
    assert!(sim.is_over());
    let notes = notes(&sim, &events);
    assert_eq!(
        notes[0],
        (
            HookPoint::Decision,
            ScriptOutcome::Denied,
            "function http_get is not available".to_string()
        )
    );
}

#[test]
fn a_bad_return_value_is_aborted() {
    let (sim, _, events, _) = logged(10, &pack("bad-return"));
    let notes = notes(&sim, &events);
    assert_eq!(notes[0].0, HookPoint::Decision);
    assert_eq!(notes[0].1, ScriptOutcome::Aborted);
}

#[test]
fn a_pack_that_uses_eval_or_lacks_its_hook_function_is_refused_at_load() {
    let dir = std::env::temp_dir().join(format!("script-eval-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(
        fixture("shoot-bias").join("pack.json"),
        dir.join("pack.json"),
    )
    .unwrap();
    std::fs::write(dir.join("main.rhai"), r#"fn decide(ctx) { eval("1") }"#).unwrap();
    let err = LoadedPack::load(&dir).unwrap_err().to_string();
    assert!(err.contains("eval"), "{err}");
    std::fs::write(dir.join("main.rhai"), "fn choose(ctx) { #{} }").unwrap();
    let err = LoadedPack::load(&dir).unwrap_err().to_string();
    assert!(err.contains("hooks") && err.contains("decide"), "{err}");
}

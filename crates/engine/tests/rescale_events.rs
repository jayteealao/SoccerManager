//! The match events stay pinned: seeds 42, 1 and 7 and the change fixture, played on the
//! shipped content and default teams, give the event lists in `tests/data/rescale-events.json`.
//! The lists were first written by the engine before ratings moved to tenths of 1 to 20, which
//! kept every event, rewritten when every action began to read the attribute contract, and
//! again when states began to move ratings within caps; this test compares and names the
//! first event that differs.
//!
//! Set `SM_WRITE_RESCALE_EVENTS=1` to write the file instead of comparing. Rewrite it only in
//! a change that regenerates the golden set.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;

use engine::gate::{self, Fixture, Inputs};

fn data_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/rescale-events.json")
}

/// Every event of `fixture`, one line each: the tick, then the event as `Debug` writes it.
fn events(fixture: &Fixture, inputs: &Inputs<'_>) -> Vec<String> {
    let mut out = Vec::new();
    gate::play_traced(fixture, inputs, |_, events| {
        for e in events {
            out.push(format!("{} {e:?}", e.tick));
        }
    })
    .expect("the fixture plays");
    out
}

#[test]
fn the_rating_scale_change_keeps_every_event() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let inputs = Inputs {
        content: &content,
        teams: [&a, &b],
        pack: None,
    };
    let fixtures = [
        Fixture::seed(42),
        Fixture::seed(1),
        Fixture::seed(7),
        Fixture::change(),
    ];
    let played: BTreeMap<String, Vec<String>> = fixtures
        .iter()
        .map(|f| (f.id.clone(), events(f, &inputs)))
        .collect();
    if std::env::var_os("SM_WRITE_RESCALE_EVENTS").is_some_and(|v| v == "1") {
        let path = data_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut text = serde_json::to_string_pretty(&played).unwrap();
        text.push('\n');
        std::fs::write(&path, text).unwrap();
        return;
    }
    let text = std::fs::read_to_string(data_path()).expect("the base event file exists");
    let base: BTreeMap<String, Vec<String>> = serde_json::from_str(&text).unwrap();
    assert_eq!(
        base.keys().collect::<Vec<_>>(),
        played.keys().collect::<Vec<_>>(),
        "the fixtures differ from the base file"
    );
    for (id, list) in &played {
        let expected = &base[id];
        if let Some(i) = (0..list.len().min(expected.len())).find(|&i| list[i] != expected[i]) {
            panic!(
                "fixture {id}: event {i} differs\n  base: {}\n  this: {}",
                expected[i], list[i]
            );
        }
        assert_eq!(
            list.len(),
            expected.len(),
            "fixture {id}: {} events here, {} in the base file",
            list.len(),
            expected.len()
        );
    }
}

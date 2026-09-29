//! The record entry of a format-4 file, checked against the lists the viewer's reader is
//! checked against too: `web/tests/data/record-paths.json` names every leaf of a full record,
//! and `web/tests/data/damaged-records.json` lists edits to the committed file's record that
//! both readers must refuse. A field added to or removed from the record types fails here
//! until the list is updated, and the viewer's test then fails until its reader follows.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;
use stream::record::LoggedRole;
use stream::{
    ChangeEntry, ChangeSource, EngineIdentity, InputInfo, LoggedChange, ManagerKind, MatchSettings,
    ReplayRecord, Watchdog, check_inputs, decode_record, parse_fixture,
};

fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../web/tests/data")
        .join(name)
}

fn list(name: &str) -> Vec<Value> {
    let text = std::fs::read_to_string(data(name)).unwrap();
    serde_json::from_str::<Value>(&text)
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

/// A record that holds every field, both kinds of change, and one role.
fn full_record() -> ReplayRecord {
    let change = |order, change| ChangeEntry {
        order,
        team: 0,
        source: ChangeSource::Manager,
        queued_tick: 1,
        queue_number: 2,
        tick: 3,
        stoppage: "half_time".into(),
        change,
    };
    ReplayRecord {
        engine: EngineIdentity {
            commit: "c".into(),
            dirty: false,
            crate_version: "0.1.0".into(),
            scheme: 1,
            maths: "m".into(),
            executable_sha256: "e".into(),
            build: "b".into(),
        },
        settings: MatchSettings {
            seed: 42,
            minutes: 1,
            knockout: false,
            managers: [ManagerKind::Human, ManagerKind::Ai],
        },
        inputs: vec![InputInfo {
            role: "tuning".into(),
            name: "tuning.json".into(),
            bytes: 1,
            sha256: "s".into(),
        }],
        inputs_bytes: 1,
        changes: vec![
            change(0, LoggedChange::Substitution { off: 20, on: 11 }),
            change(
                1,
                LoggedChange::Tactics {
                    formation: Some(1),
                    mentality: None,
                    instructions: [None; 6],
                    roles: vec![LoggedRole {
                        squad: 1,
                        role: 2,
                        duty: 3,
                    }],
                },
            ),
        ],
        watchdog: Watchdog {
            slow_calls: 0,
            invalid: None,
        },
    }
}

/// Every leaf of `value` as a path, with `[]` for a list item.
fn leaves(value: &Value, path: &str, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                let at = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                leaves(inner, &at, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                leaves(item, &format!("{path}[]"), out);
            }
        }
        _ => {
            out.insert(path.to_string());
        }
    }
}

/// Removes the leaf at `path` from `value`, on the first list item that holds it. A path
/// that ends in `[]` removes the last item of the list.
fn delete_at(value: &mut Value, path: &str) -> bool {
    let (head, rest) = match path.split_once('.') {
        Some((head, rest)) => (head, Some(rest)),
        None => (path, None),
    };
    if let Some(key) = head.strip_suffix("[]") {
        let Some(items) = value.get_mut(key).and_then(Value::as_array_mut) else {
            return false;
        };
        let Some(rest) = rest else {
            return items.pop().is_some();
        };
        return items.iter_mut().any(|item| delete_at(item, rest));
    }
    match rest {
        None => value
            .as_object_mut()
            .is_some_and(|m| m.remove(head).is_some()),
        Some(rest) => value
            .get_mut(head)
            .is_some_and(|inner| delete_at(inner, rest)),
    }
}

#[test]
fn the_record_types_serialise_to_exactly_the_listed_leaves() {
    let listed: BTreeSet<String> = list("record-paths.json")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let mut found = BTreeSet::new();
    leaves(
        &serde_json::to_value(full_record()).unwrap(),
        "",
        &mut found,
    );
    assert_eq!(
        found, listed,
        "the record's leaves differ from web/tests/data/record-paths.json: update the list \
         and the viewer's reader together"
    );
}

#[test]
fn a_record_without_any_listed_leaf_is_refused_by_name() {
    let full = serde_json::to_value(full_record()).unwrap();
    for path in list("record-paths.json") {
        let path = path.as_str().unwrap();
        let mut record = full.clone();
        assert!(
            delete_at(&mut record, path),
            "{path} is not in the full record"
        );
        let error = decode_record(&serde_json::to_vec(&record).unwrap())
            .err()
            .unwrap_or_else(|| panic!("a record without {path} was read"))
            .to_string();
        // A list item that is gone is a wrong length, which names no key.
        if !path.ends_with("[]") {
            let key = path.rsplit('.').next().unwrap();
            assert!(error.contains(key), "without {path}: {error}");
        }
    }
}

/// Applies one edit, its path a list of keys and indices.
fn edit(record: &mut Value, edit: &Value) {
    let path = edit["path"].as_array().unwrap();
    let (last, parents) = path.split_last().unwrap();
    let mut at = record;
    for step in parents {
        at = match step {
            Value::String(key) => &mut at[key.as_str()],
            index => &mut at[index.as_u64().unwrap() as usize],
        };
    }
    let delete = edit["op"] == "delete";
    match last {
        Value::String(key) if delete => {
            at.as_object_mut().unwrap().remove(key);
        }
        Value::String(key) => at[key.as_str()] = edit["value"].clone(),
        index => at[index.as_u64().unwrap() as usize] = edit["value"].clone(),
    }
}

#[test]
fn every_damaged_record_is_refused() {
    let bytes = std::fs::read(data("one-minute-v4.smfx")).unwrap();
    let fixture = parse_fixture(&bytes).unwrap();
    let base = serde_json::to_value(fixture.record.as_ref().unwrap()).unwrap();
    let files = |f: &stream::Fixture| -> Vec<(String, Vec<u8>)> {
        f.inputs
            .iter()
            .map(|i| (i.name.clone(), i.bytes.clone()))
            .collect()
    };
    // The undamaged record passes both checks, so a refusal below is the edit's.
    let record = decode_record(&serde_json::to_vec(&base).unwrap()).unwrap();
    check_inputs(&record, files(&fixture)).unwrap();
    let damaged = list("damaged-records.json");
    assert!(damaged.len() > 30);
    for case in damaged {
        let mut value = base.clone();
        for step in case["edits"].as_array().unwrap() {
            edit(&mut value, step);
        }
        let refused = match decode_record(&serde_json::to_vec(&value).unwrap()) {
            Err(_) => true,
            Ok(record) => check_inputs(&record, files(&fixture)).is_err(),
        };
        assert!(refused, "the reader took a record with {}", case["name"]);
    }
}

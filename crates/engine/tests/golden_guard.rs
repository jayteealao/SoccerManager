//! The golden-file guard rules on prepared pairs of files (AC-9): a hash change with no new
//! ledger entry, an edited old entry, and a fixture-list or state-inventory change with no
//! gate-schema increase each fail with the rule named. So do an `add-machine-set` entry that
//! also changes an existing set, a removed set, a deleted file, and a first file with two
//! entries; a first bootstrap, an added machine set, and a regeneration with a schema
//! increase pass.

mod common;

use engine::gate::golden::{self, GoldenFile};
use engine::gate::guard::{Fault, check};
use engine::gate::{self, Checkpoint, MatchHashes};
use serde_json::Value;

const MACHINE: &str = "aa-machine";

/// A valid first golden file for `MACHINE` whose every match lasts 2,500 ticks.
fn first() -> GoldenFile {
    let fixtures = gate::fixtures();
    let hash = |n: u32| format!("{n:064x}");
    let matches = fixtures
        .iter()
        .map(|f| MatchHashes {
            id: f.id.clone(),
            ticks: 2_500,
            final_hash: hash(3),
            checkpoints: [1_000, 2_000, 2_500]
                .iter()
                .enumerate()
                .map(|(i, t)| Checkpoint {
                    tick: *t,
                    hash: hash(i as u32 + 1),
                })
                .collect(),
        })
        .collect();
    GoldenFile::first(&fixtures, MACHINE, matches, golden::BOOTSTRAP_REASON)
}

fn changed_set(file: &GoldenFile, machine: &str) -> Vec<MatchHashes> {
    let mut set = file.set_for(machine).unwrap().to_vec();
    set[0].final_hash = format!("{:064x}", 99);
    set[0].checkpoints.last_mut().unwrap().hash = set[0].final_hash.clone();
    set
}

/// `file`'s text after `edit` on its JSON.
fn edited(file: &GoldenFile, edit: impl Fn(&mut Value)) -> String {
    let mut v: Value = serde_json::from_str(&file.to_text()).unwrap();
    edit(&mut v);
    serde_json::to_string_pretty(&v).unwrap()
}

fn faults(parent: Option<&str>, child: Option<&str>) -> Vec<Fault> {
    let found = check(parent, child);
    for f in &found {
        println!("{f}");
    }
    found
}

fn has(found: &[Fault], rule: u8, text: &str) -> bool {
    found
        .iter()
        .any(|f| f.rule == rule && f.message.contains(text))
}

#[test]
fn a_hash_change_with_no_new_entry_fails() {
    let parent = first();
    let child = edited(&parent, |v| {
        v["hash_sets"][MACHINE][0]["final_hash"] = format!("{:064x}", 99).into();
        v["hash_sets"][MACHINE][0]["checkpoints"][2]["hash"] = format!("{:064x}", 99).into();
    });
    let found = faults(Some(&parent.to_text()), Some(&child));
    assert!(
        has(
            &found,
            5,
            "hash set aa-machine changes with no new regenerate entry"
        ),
        "{found:?}"
    );
}

#[test]
fn an_edited_old_entry_fails() {
    let parent = first();
    let child = edited(&parent, |v| v["ledger"][0]["reason"] = "rewritten".into());
    let found = faults(Some(&parent.to_text()), Some(&child));
    assert!(has(&found, 3, "ledger entry 0 is edited"), "{found:?}");

    // A removed entry fails too (the file then has no bootstrap either).
    let child = edited(&parent, |v| v["ledger"] = serde_json::json!([]));
    let found = faults(Some(&parent.to_text()), Some(&child));
    assert!(has(&found, 3, "ledger entry 0 is removed"), "{found:?}");
}

/// A regeneration of `parent` whose JSON is then edited by `edit`.
fn regenerated(parent: &GoldenFile, edit: impl Fn(&mut Value)) -> String {
    let (regen, _) = parent.clone().regenerated(
        &gate::fixtures(),
        MACHINE,
        changed_set(parent, MACHINE),
        "a hash change",
    );
    edited(&regen, edit)
}

#[test]
fn a_fixture_list_change_with_no_schema_increase_fails() {
    let parent = first();
    let child = regenerated(&parent, |v| v["fixtures"][0]["minutes"] = 45.into());
    let found = faults(Some(&parent.to_text()), Some(&child));
    assert!(
        has(
            &found,
            4,
            "the fixture list changes, but the gate schema is not increased"
        ),
        "{found:?}"
    );
    // With no regenerate entry at all, both parts of the rule fail.
    let child = edited(&parent, |v| v["fixtures"][0]["minutes"] = 45.into());
    let found = faults(Some(&parent.to_text()), Some(&child));
    assert!(has(&found, 4, "is not increased"), "{found:?}");
    assert!(
        has(&found, 4, "changes with no new regenerate entry"),
        "{found:?}"
    );
}

#[test]
fn an_inventory_version_change_with_no_schema_increase_fails() {
    let parent = first();
    let child = regenerated(&parent, |v| v["inventory_version"] = 2.into());
    let found = faults(Some(&parent.to_text()), Some(&child));
    assert!(
        has(
            &found,
            4,
            "the state inventory version changes, but the gate schema is not increased"
        ),
        "{found:?}"
    );
    let child = regenerated(&parent, |v| v["gate_schema"] = 0.into());
    let found = faults(Some(&parent.to_text()), Some(&child));
    assert!(
        has(&found, 4, "the gate schema decreases from 1 to 0"),
        "{found:?}"
    );
}

#[test]
fn an_add_machine_set_entry_that_changes_an_existing_set_fails() {
    let parent = first();
    let mut child = common::with_machine_set(
        parent.clone(),
        "zz-second",
        changed_set(&parent, MACHINE),
        "second",
    );
    child
        .hash_sets
        .insert(MACHINE.into(), changed_set(&parent, MACHINE));
    child.set_differences = golden::set_differences(&child.hash_sets);
    let found = faults(Some(&parent.to_text()), Some(&child.to_text()));
    assert!(
        has(
            &found,
            6,
            "the add-machine-set entry for zz-second also: hash set aa-machine changes"
        ),
        "{found:?}"
    );
    assert!(has(&found, 5, "hash set aa-machine changes"), "{found:?}");

    // An entry that names one machine but adds another's set fails too.
    let child = edited(
        &common::with_machine_set(
            parent.clone(),
            "zz-second",
            changed_set(&parent, MACHINE),
            "second",
        ),
        |v| v["ledger"][1]["machine"] = "zz-third".into(),
    );
    let found = faults(Some(&parent.to_text()), Some(&child));
    assert!(
        has(
            &found,
            6,
            "the add-machine-set entry for zz-third must add exactly that hash set"
        ),
        "{found:?}"
    );
}

#[test]
fn a_removed_set_a_deleted_file_and_a_crowded_first_file_fail() {
    let parent = first();
    let two = common::with_machine_set(
        parent.clone(),
        "zz-second",
        changed_set(&parent, MACHINE),
        "second",
    );
    let removed = edited(&two, |v| {
        v["hash_sets"].as_object_mut().unwrap().remove("zz-second");
        v["set_differences"] = serde_json::json!([]);
    });
    let found = faults(Some(&two.to_text()), Some(&removed));
    assert!(
        has(
            &found,
            5,
            "hash set zz-second is removed with no new regenerate entry"
        ),
        "{found:?}"
    );

    let found = faults(Some(&parent.to_text()), None);
    assert!(has(&found, 1, "the golden file is deleted"), "{found:?}");

    let found = faults(None, Some(&two.to_text()));
    assert!(
        has(&found, 2, "it holds 2 entries and 2 hash sets"),
        "{found:?}"
    );

    // A second set with no entry.
    let unrecorded = edited(&parent, |v| {
        let set = v["hash_sets"][MACHINE].clone();
        v["hash_sets"]["zz-second"] = set;
    });
    let found = faults(Some(&parent.to_text()), Some(&unrecorded));
    assert!(
        has(&found, 7, "hash set zz-second is added with no new entry"),
        "{found:?}"
    );
}

#[test]
fn a_bootstrap_an_added_set_and_a_regeneration_with_a_schema_increase_pass() {
    let parent = first();
    assert_eq!(faults(None, Some(&parent.to_text())), Vec::new());

    let two = common::with_machine_set(
        parent.clone(),
        "zz-second",
        changed_set(&parent, MACHINE),
        "second",
    );
    assert_eq!(
        faults(Some(&parent.to_text()), Some(&two.to_text())),
        Vec::new()
    );

    let regen = regenerated(&two, |v| {
        v["gate_schema"] = 2.into();
        v["fixtures"][0]["minutes"] = 45.into();
    });
    assert_eq!(faults(Some(&two.to_text()), Some(&regen)), Vec::new());

    // The committed file passes as a first file.
    let committed = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gate/golden.json"),
    )
    .unwrap();
    let as_first = golden::read(&committed).unwrap();
    if as_first.ledger.len() == 1 {
        assert_eq!(faults(None, Some(&committed)), Vec::new());
    }
}

/// `two` (two machine sets) regenerated to the one portable set.
fn to_portable(two: &GoldenFile) -> GoldenFile {
    two.clone()
        .regenerated(
            &gate::fixtures(),
            golden::PORTABLE,
            changed_set(two, MACHINE),
            "one set for every machine",
        )
        .0
}

#[test]
fn two_machine_sets_to_the_portable_set_pass_only_with_one_regenerate_entry() {
    let parent = first();
    let two = common::with_machine_set(
        parent.clone(),
        "zz-second",
        changed_set(&parent, MACHINE),
        "second",
    );
    let portable = to_portable(&two);
    assert_eq!(
        faults(Some(&two.to_text()), Some(&portable.to_text())),
        Vec::new()
    );
    // An identical file passes.
    assert_eq!(
        faults(Some(&portable.to_text()), Some(&portable.to_text())),
        Vec::new()
    );

    // The same change with no new entry fails rule 5 for both removed sets.
    let unrecorded = edited(&portable, |v| {
        v["ledger"].as_array_mut().unwrap().pop();
    });
    let found = faults(Some(&two.to_text()), Some(&unrecorded));
    assert!(
        has(
            &found,
            5,
            "hash set aa-machine is removed with no new regenerate entry"
        ),
        "{found:?}"
    );
    assert!(
        has(
            &found,
            5,
            "hash set zz-second is removed with no new regenerate entry"
        ),
        "{found:?}"
    );

    // A later add-machine-set entry beside the portable set fails.
    let added = edited(&portable, |v| {
        let mut entry = v["ledger"][1].clone();
        entry["machine"] = "zz-third".into();
        v["ledger"].as_array_mut().unwrap().push(entry);
        v["hash_sets"]["zz-third"] = v["hash_sets"][golden::PORTABLE].clone();
        v["set_differences"] = serde_json::to_value(golden::set_differences(
            &serde_json::from_value(v["hash_sets"].clone()).unwrap(),
        ))
        .unwrap();
    });
    let found = faults(Some(&portable.to_text()), Some(&added));
    assert!(
        has(&found, 7, "adds zz-third after the portable set of entry 2"),
        "{found:?}"
    );
}

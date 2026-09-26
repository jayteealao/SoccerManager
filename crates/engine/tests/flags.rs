//! Feature flags in the tuning file: a flag without an owner, a hypothesis, or a removal
//! condition is refused naming the flag; a flag must switch something; an override must
//! name a real tuning value and stay inside its bound; and a flag that is on changes only
//! the values it names and marks the content digest.

mod common;

use engine::data::{ATTRIBUTES_FILE, RULES_FILE, TACTICS_FILE, TUNING_FILE};
use engine::{Content, ContentDir, EngineError, FlagState, FlagStates};
use serde_json::{Value, json};

/// A content folder holding the shipped files, with `flags` as the tuning file's flags
/// block.
fn folder(name: &str, flags: Value) -> ContentDir {
    let root = std::env::temp_dir().join(format!("engine-flags-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("rules")).unwrap();
    let shipped = common::content_dir();
    for rel in [ATTRIBUTES_FILE, RULES_FILE, TACTICS_FILE] {
        std::fs::copy(shipped.path(rel), root.join(rel)).unwrap();
    }
    let mut tuning: Value =
        serde_json::from_str(&std::fs::read_to_string(shipped.path(TUNING_FILE)).unwrap()).unwrap();
    tuning["flags"] = flags;
    std::fs::write(
        root.join(TUNING_FILE),
        serde_json::to_string_pretty(&tuning).unwrap(),
    )
    .unwrap();
    ContentDir::at(root)
}

fn cleanup(dir: &ContentDir) {
    let _ = std::fs::remove_dir_all(dir.root());
}

/// A complete flag that sets the shot range to 12 when it is on.
fn probe() -> Value {
    json!({
        "owner": "engine team",
        "hypothesis": "a shorter shot range lowers goals per match toward 2.8",
        "removal_condition": "removed after one paired run of 1000 matches per suite",
        "overrides": { "engine.shot_range": 12.0 }
    })
}

fn refusal(dir: &ContentDir) -> (String, String) {
    let err = Content::load(dir).unwrap_err();
    cleanup(dir);
    match &err {
        EngineError::Data { field, .. } => (field.clone(), err.to_string()),
        other => panic!("expected a content refusal, got {other}"),
    }
}

fn states(pairs: &[(&str, FlagState)]) -> FlagStates {
    FlagStates(pairs.iter().map(|(n, s)| ((*n).to_string(), *s)).collect())
}

#[test]
fn a_flag_missing_a_required_field_is_refused_naming_the_flag() {
    for field in ["owner", "hypothesis", "removal_condition"] {
        for (case, flag) in [
            ("missing", {
                let mut f = probe();
                f.as_object_mut().unwrap().remove(field);
                f
            }),
            ("empty", {
                let mut f = probe();
                f[field] = json!("");
                f
            }),
        ] {
            let dir = folder(&format!("{field}-{case}"), json!({ "probe_flag": flag }));
            let (path, text) = refusal(&dir);
            assert_eq!(path, format!("flags.probe_flag.{field}"), "{case}: {text}");
            assert!(text.contains("probe_flag"), "{case}: {text}");
            assert!(text.contains("is required"), "{case}: {text}");
        }
    }
}

#[test]
fn a_flag_that_switches_nothing_is_refused_naming_it() {
    let mut flag = probe();
    flag.as_object_mut().unwrap().remove("overrides");
    let dir = folder("nothing", json!({ "idle_flag": flag }));
    let (_, text) = refusal(&dir);
    assert!(
        text.contains("flag idle_flag switches nothing: add overrides or register it in code"),
        "{text}"
    );
}

#[test]
fn a_flag_name_that_is_not_lower_snake_case_is_refused() {
    let dir = folder("name", json!({ "Probe-Flag": probe() }));
    let (_, text) = refusal(&dir);
    assert!(text.contains("flag Probe-Flag"), "{text}");
    assert!(text.contains("lower snake case"), "{text}");
}

#[test]
fn an_override_of_a_value_that_does_not_exist_is_refused_even_when_off() {
    let mut flag = probe();
    flag["overrides"] = json!({ "engine.no_such_knob": 1.0 });
    let dir = folder("knob", json!({ "probe_flag": flag }));
    let (path, text) = refusal(&dir);
    assert_eq!(
        path, "flags.probe_flag.overrides.engine.no_such_knob",
        "{text}"
    );
    assert!(text.contains("no such tuning value"), "{text}");
}

#[test]
fn an_override_outside_its_bound_is_refused_naming_the_flag() {
    let mut flag = probe();
    flag["overrides"] = json!({ "engine.shot_range": 999.0 });
    let dir = folder("bound", json!({ "probe_flag": flag }));
    let (path, text) = refusal(&dir);
    assert_eq!(path, "flags.probe_flag", "{text}");
    assert!(text.contains("engine.shot_range"), "{text}");
}

#[test]
fn an_override_of_the_wrong_type_or_of_the_flags_is_refused() {
    let mut flag = probe();
    flag["overrides"] = json!({ "engine.shot_range": "far" });
    let dir = folder("type", json!({ "probe_flag": flag }));
    let (_, text) = refusal(&dir);
    assert!(
        text.contains("a string where the file holds a number"),
        "{text}"
    );
    let mut flag = probe();
    flag["overrides"] = json!({ "schema_version": 3 });
    let dir = folder("version", json!({ "probe_flag": flag }));
    let (_, text) = refusal(&dir);
    assert!(text.contains("a flag cannot set this path"), "{text}");
}

#[test]
fn two_flags_on_that_set_the_same_value_are_refused_naming_both() {
    let dir = folder(
        "clash",
        json!({ "first_flag": probe(), "second_flag": probe() }),
    );
    let content = Content::load(&dir).unwrap();
    let err = content
        .with_flags(&states(&[
            ("first_flag", FlagState::On),
            ("second_flag", FlagState::On),
        ]))
        .unwrap_err();
    cleanup(&dir);
    let text = err.to_string();
    assert!(
        text.contains("first_flag") && text.contains("second_flag"),
        "{text}"
    );
}

#[test]
fn a_state_for_an_undeclared_flag_is_refused_naming_it() {
    let dir = folder("undeclared", json!({ "probe_flag": probe() }));
    let content = Content::load(&dir).unwrap();
    let err = content
        .with_flags(&states(&[("nope", FlagState::On)]))
        .unwrap_err();
    cleanup(&dir);
    assert_eq!(
        err.to_string(),
        "content refused: tuning tuning.json: flags.nope: not declared in tuning.json"
    );
}

#[test]
fn a_flag_that_is_on_changes_only_its_values_and_marks_the_digest() {
    let dir = folder("apply", json!({ "probe_flag": probe() }));
    let off = Content::load(&dir).unwrap();
    assert!(off.flags.is_empty());
    let on = off
        .with_flags(&states(&[("probe_flag", FlagState::On)]))
        .unwrap();
    assert_eq!(on.flags.names(), ["probe_flag"]);
    assert_eq!(on.tuning.engine.shot_range, 12.0);

    let mut a = serde_json::to_value(&off.tuning).unwrap();
    let mut b = serde_json::to_value(&on.tuning).unwrap();
    assert_eq!(a["engine"]["shot_range"], 21.0);
    for tree in [&mut a, &mut b] {
        tree["engine"]["shot_range"] = Value::Null;
        tree["flags"]["probe_flag"]["state"] = Value::Null;
    }
    assert_eq!(a, b, "only the shot range differs");

    assert_ne!(on.digest, off.digest);
    // Setting the state the file already gives changes nothing.
    let same = off
        .with_flags(&states(&[("probe_flag", FlagState::Off)]))
        .unwrap();
    assert_eq!(same.digest, off.digest);
    assert_eq!(same.tuning, off.tuning);
    cleanup(&dir);
}

#[test]
fn a_flag_the_file_turns_on_applies_at_load_and_the_command_line_turns_it_off() {
    let mut flag = probe();
    flag["state"] = json!("on");
    let dir = folder("file-on", json!({ "probe_flag": flag }));
    let content = Content::load(&dir).unwrap();
    assert_eq!(content.tuning.engine.shot_range, 12.0);
    assert_eq!(content.flags.names(), ["probe_flag"]);
    let off = content
        .with_flags(&states(&[("probe_flag", FlagState::Off)]))
        .unwrap();
    assert_eq!(off.tuning.engine.shot_range, 21.0);
    assert!(off.flags.is_empty());
    assert_ne!(off.digest, content.digest);
    cleanup(&dir);
}

#[test]
fn the_shipped_content_with_no_states_is_unchanged() {
    let content = common::content();
    assert!(content.flags.is_empty());
    let again = content.with_flags(&FlagStates::default()).unwrap();
    assert_eq!(again.digest, content.digest);
    assert_eq!(again.tuning, content.tuning);
    assert_eq!(&content.tuning, content.written_tuning());
}

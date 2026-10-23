//! A change run's intent and its surprises: the intent is asked for (or read from
//! `--intent`) before any match plays and kept in the run folder; every band that moved
//! against it is flagged, and the run stays unfinished until each surprise has its note; a
//! judgement again of a played folder plays nothing; a side-effect note writes a draft setting
//! rule, which counts only once confirmed; and the settings a change touched select the
//! sensitivity rules that play.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use common::{
    FEW_BANDS, FEW_RARE, RecordSchemas, bin, copy_tree, edited_content, few_bands, fixture_repo,
    git, intent_file, record, write_intent,
};
use serde_json::{Value, json};

fn exe() -> &'static str {
    env!("CARGO_BIN_EXE_engine-cli")
}

/// The keeper setting the side-effect note names.
const SAVE_HIGH: &str = "tuning.json#/engine/shots/save_high";

/// The shipped value of [`SAVE_HIGH`].
const SAVE_HIGH_SHIPPED: f64 = 0.891;

/// Sets the three keeper settings of `content` to 0, so keepers save nothing and more shots
/// become goals: the same settings the `open_goal` test flag overrides.
fn keepers_save_nothing(content: &Path) {
    edit_json(&content.join("tuning.json"), |t| {
        t["engine"]["keeper_catch_chance"] = json!(0.0);
        t["engine"]["shots"]["save_high"] = json!(0.0);
        t["engine"]["shots"]["save_low"] = json!(0.0);
    });
}

fn edit_json(path: &Path, edit: impl FnOnce(&mut Value)) {
    let mut v: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    edit(&mut v);
    std::fs::write(path, serde_json::to_string_pretty(&v).unwrap()).unwrap();
}

/// A change run of the equal suite on `content` into `out`, against this build playing the
/// content `content` holds at `HEAD` of its repository, with `args`.
fn calibrate(content: &Path, data: &Path, out: &Path, args: &[&str]) -> Output {
    bin(data)
        .arg("--content-dir")
        .arg(content)
        .arg("calibrate")
        .args(["--seed", "9", "--suite", "equal", "--minutes", "5"])
        .args(["--jobs", "4", "--base-binary", exe(), "--base-rev", "HEAD"])
        .args(args)
        .arg("--out")
        .arg(out)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The run's report, checked against its schema, after a run that judged (exit 0 or 2).
fn report(out: &Output, run: &Path) -> Value {
    let code = out.status.code();
    assert!(
        matches!(code, Some(0 | 2)),
        "exit {code:?}; stderr: {}",
        stderr(out)
    );
    let report = record(&std::fs::read_to_string(run.join("report.json")).unwrap());
    RecordSchemas::load()
        .report(&report)
        .unwrap_or_else(|e| panic!("{e}\n{report}"));
    report
}

/// The surprise row of `band`, if flagged.
fn surprise<'a>(r: &'a Value, band: &str) -> Option<&'a Value> {
    r["calib.surprises"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["band"] == band)
}

fn flagged(r: &Value) -> Vec<String> {
    r["calib.surprises"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["band"].as_str().unwrap().to_string())
        .collect()
}

/// Every source of `calib.played` is 0.
fn played_nothing(r: &Value) {
    let p = r["calib.played"].as_object().unwrap();
    assert!(p.values().all(|v| v == 0), "{p:?}");
}

/// The size of each row file of `run`.
fn row_sizes(run: &Path) -> BTreeMap<String, u64> {
    std::fs::read_dir(run.join("rows"))
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                e.metadata().unwrap().len(),
            )
        })
        .collect()
}

/// The setting rules of `content`'s rules file.
fn setting_rules(content: &Path) -> Vec<Value> {
    let v: Value =
        serde_json::from_slice(&std::fs::read(content.join("sensitivity.json")).unwrap()).unwrap();
    v["setting_rules"].as_array().cloned().unwrap_or_default()
}

/// The intent naming no band, with one note per band of `flagged`: a side effect of
/// [`SAVE_HIGH`] on the goals, a bug note on any other; and `confirm`.
fn notes_intent(dir: &Path, flagged: &[String], confirm: &[&str]) -> PathBuf {
    let notes: Vec<Value> = flagged
        .iter()
        .map(|b| {
            if b == "goals_per_match" {
                json!({
                    "band": b,
                    "kind": "side effect",
                    "note": "keepers that save nothing let more shots in",
                    "setting": SAVE_HIGH,
                })
            } else {
                json!({ "band": b, "kind": "bug", "note": "a test note on a chance move" })
            }
        })
        .collect();
    write_intent(
        dir,
        &json!({
            "schema_version": 1,
            "change": "keepers save nothing",
            "bands": [],
            "notes": notes,
            "confirm": confirm,
        }),
    )
}

/// One played change run, judged again on copies: an unintended move, a wrong-way move and a
/// missed band; the notes that finish the run; the draft rule a side-effect note writes, once
/// only; and its confirmation, which sets the expected move. Only the first call plays.
#[test]
fn surprises_notes_and_a_draft_rule_on_one_played_run() {
    let repo = fixture_repo(|d| few_bands(d, 200, 3, true));
    let content = repo.path().join("content");
    keepers_save_nothing(&content);
    let data = tempfile::TempDir::new().unwrap();
    let run = data.path().join("run");
    let none = intent_file(data.path(), &[]);
    let out = calibrate(
        &content,
        data.path(),
        &run,
        &["--intent", none.to_str().unwrap()],
    );
    let text = stderr(&out);
    let played = report(&out, &run);

    // Unintended: the goals rose against the old engine, and the intent names no band.
    let goals = surprise(&played, "goals_per_match").expect("the goals are flagged");
    assert_eq!(goals["kind"], "unintended", "{goals}");
    assert_eq!(goals["old_word"], "worse", "{goals}");
    assert_eq!(goals["moved"], "up", "{goals}");
    // The rare band is not judged in a change run: listed as not checked, never flagged.
    assert!(surprise(&played, FEW_RARE).is_none());
    let rare = played["calib.surprises"]["not_checked"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["band"] == FEW_RARE)
        .expect("the rare band is listed as not checked");
    assert_eq!(
        rare["why"],
        "not judged in a change run; not checked against the intent"
    );
    let first = flagged(&played);
    let n = first.len();
    assert_eq!(played["calib.surprises"]["open"], n);
    assert_eq!(played["calib.finished"], false);
    let headline = played["calib.headline"].as_str().unwrap();
    assert!(
        headline.ends_with(&format!("; open surprises: {n}; run unfinished")),
        "{headline}"
    );
    assert!(text.contains(headline), "{text}");
    assert!(played["calib.played"]["total"].as_u64().unwrap() > 0);
    assert_eq!(played["calib.intent"]["source"], "file");

    // Wrong way: the copy's intent says the goals should fall.
    let wrong = data.path().join("wrong-way");
    copy_tree(&run, &wrong);
    intent_file(&wrong, &[("goals_per_match", "down")]);
    let r = report(&calibrate(&content, data.path(), &wrong, &[]), &wrong);
    assert_eq!(
        surprise(&r, "goals_per_match").unwrap()["kind"],
        "wrong-way"
    );
    assert_eq!(r["calib.played"]["total"], 0);
    assert_eq!(r["calib.intent"]["source"], "run folder");

    // Missed: a band the played run reads unmoved, named in the copy's intent.
    let still = played["calib.verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| {
            v["band"] != "goals_per_match"
                && FEW_BANDS.iter().any(|b| v["band"] == *b)
                && (v["old_word"] == "equivalent"
                    || (v["old_word"] == "not sure" && v.get("moved").is_none()))
        })
        .expect("a fixture band the change did not move")["band"]
        .as_str()
        .unwrap()
        .to_string();
    let missed = data.path().join("missed");
    copy_tree(&run, &missed);
    intent_file(&missed, &[(&still, "up")]);
    let r = report(&calibrate(&content, data.path(), &missed, &[]), &missed);
    assert_eq!(surprise(&r, &still).unwrap()["kind"], "missed", "{r}");
    assert_eq!(
        surprise(&r, "goals_per_match").unwrap()["kind"],
        "unintended"
    );
    assert_eq!(r["calib.played"]["total"], 0);

    // Finished: one note per surprise; nothing plays and no row file changes.
    let notes_dir = data.path().join("notes");
    std::fs::create_dir_all(&notes_dir).unwrap();
    let notes = notes_intent(&notes_dir, &first, &[]);
    let rows_before = row_sizes(&run);
    let notes_arg = ["--intent", notes.to_str().unwrap()];
    let r = report(&calibrate(&content, data.path(), &run, &notes_arg), &run);
    played_nothing(&r);
    assert_eq!(row_sizes(&run), rows_before);
    assert_eq!(r["calib.finished"], true);
    assert_eq!(r["calib.surprises"]["open"], 0);
    let headline = r["calib.headline"].as_str().unwrap();
    assert!(headline.ends_with("run finished"), "{headline}");

    // The draft: the goals rose as the setting fell, so the band falls as it rises.
    let rules = setting_rules(&content);
    assert_eq!(rules.len(), 1, "{rules:?}");
    let draft = &rules[0];
    let id = draft["id"].as_str().unwrap().to_string();
    assert!(
        id.len() == 11 && id.starts_with("sr-") && id[3..].chars().all(|c| c.is_ascii_hexdigit()),
        "{id}"
    );
    assert_eq!(draft["setting"], SAVE_HIGH);
    assert_eq!(draft["band"], "goals_per_match");
    assert_eq!(draft["direction"], "down");
    assert_eq!(draft["status"], "draft");
    let levels: Vec<f64> = draft["levels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let want = [0.0, SAVE_HIGH_SHIPPED / 2.0, SAVE_HIGH_SHIPPED];
    assert!(
        levels.len() == 3 && levels.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-9),
        "{levels:?}"
    );
    let written = r["calib.drafts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["id"] == id.as_str() && d["action"] == "written");
    assert!(written, "{}", r["calib.drafts"]);
    // A draft sets no expected move: the goals are still a surprise, now with their note.
    assert!(surprise(&r, "goals_per_match").unwrap()["note"].is_object());

    // The same call again writes no second rule.
    let r = report(&calibrate(&content, data.path(), &run, &notes_arg), &run);
    assert_eq!(setting_rules(&content).len(), 1);
    played_nothing(&r);

    // Confirmed: the rule now expects the goals to rise, so they are no surprise.
    let notes = notes_intent(&notes_dir, &first, &[&id]);
    let confirm_arg = ["--intent", notes.to_str().unwrap()];
    let r = report(&calibrate(&content, data.path(), &run, &confirm_arg), &run);
    let rules = setting_rules(&content);
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["status"], "confirmed");
    assert!(surprise(&r, "goals_per_match").is_none(), "{r}");
    let s = &r["calib.surprises"];
    assert!(
        s["as_expected"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b == "goals_per_match"),
        "{s}"
    );
    assert!(
        s["expected"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["band"] == "goals_per_match" && e["rule"] == id.as_str()),
        "{s}"
    );
    played_nothing(&r);
    let row = r["calib.rules"]["setting"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == id.as_str())
        .expect("the confirmed rule's row")
        .clone();
    assert_eq!(row["word"], "not_played", "{row}");
    assert_eq!(
        row["note"],
        "confirmed after this run was played; plays in the next change run"
    );
}

/// A confirmed setting rule whose setting the change touched plays at three levels; a draft
/// one does not play and is listed as a draft.
#[test]
fn setting_rules_play_only_when_confirmed() {
    let repo = fixture_repo(|d| few_bands(d, 4, 3, false));
    let content = repo.path().join("content");
    edit_json(&content.join("sensitivity.json"), |s| {
        s["design"]["setting_matches"] = json!(4);
        s["setting_rules"] = json!([
            {
                "id": "sr-0000000a", "setting": SAVE_HIGH, "band": "goals_per_match",
                "direction": "down", "levels": [0.5, 0.7, SAVE_HIGH_SHIPPED],
                "status": "confirmed", "note": "a test rule", "from_run": "test",
                "since": "2026-10-23"
            },
            {
                "id": "sr-0000000b", "setting": "tuning.json#/engine/foul_base",
                "band": "fouls_per_team", "direction": "up", "levels": [0.05, 0.1, 0.2],
                "status": "draft", "note": "a test draft", "from_run": "test",
                "since": "2026-10-23"
            }
        ]);
    });
    git(repo.path(), &["commit", "-q", "-am", "setting rules"]);
    edit_json(&content.join("tuning.json"), |t| {
        t["engine"]["shots"]["save_high"] = json!(0.8);
        t["engine"]["foul_base"] = json!(0.12);
    });
    let data = tempfile::TempDir::new().unwrap();
    let run = data.path().join("run");
    let none = intent_file(data.path(), &[]);
    let out = calibrate(
        &content,
        data.path(),
        &run,
        &["--intent", none.to_str().unwrap()],
    );
    let text = stderr(&out);
    let r = report(&out, &run);
    let stage = &r["calib.rules"];
    let setting = stage["setting"].as_array().unwrap();
    let confirmed = setting
        .iter()
        .find(|x| x["id"] == "sr-0000000a")
        .unwrap_or_else(|| panic!("{stage}"));
    assert_eq!(
        confirmed["levels"].as_array().unwrap().len(),
        3,
        "{confirmed}"
    );
    assert_eq!(confirmed["matches"], 4, "{confirmed}");
    assert!(
        ["pass", "fail", "not_sure"]
            .iter()
            .any(|w| confirmed["word"] == *w),
        "{confirmed}"
    );
    for k in 0..3 {
        let level = run.join("rules").join("sr-0000000a").join(format!("l{k}"));
        assert!(level.is_dir(), "{}", level.display());
    }
    let draft = setting
        .iter()
        .find(|x| x["id"] == "sr-0000000b")
        .unwrap_or_else(|| panic!("{stage}"));
    assert_eq!(draft["word"], "not_played", "{draft}");
    assert!(
        draft["note"].as_str().unwrap().starts_with("draft"),
        "{draft}"
    );
    assert!(!run.join("rules").join("sr-0000000b").exists());
    assert_eq!(r["calib.played"]["setting_rules"], 3 * 4);
    assert!(text.contains("sr-0000000a"), "{text}");
}

/// The intent given with `--intent` is in the run folder, named by its SHA-256 in the
/// report, and written there before the first match row.
#[test]
fn the_intent_is_in_the_run_folder_before_any_match() {
    use sha2::{Digest, Sha256};
    let repo = fixture_repo(|d| few_bands(d, 4, 3, false));
    let content = repo.path().join("content");
    let data = tempfile::TempDir::new().unwrap();
    let given = intent_file(data.path(), &[("goals_per_match", "up")]);
    let run = data.path().join("run");
    let arg = ["--intent", given.to_str().unwrap()];
    let r = report(&calibrate(&content, data.path(), &run, &arg), &run);
    let parse = |p: &Path| -> Value { serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap() };
    let kept = run.join("intent.json");
    assert_eq!(parse(&kept), parse(&given));
    let sha = format!("{:x}", Sha256::digest(std::fs::read(&kept).unwrap()));
    assert_eq!(r["calib.intent"]["sha256"], sha.as_str());
    assert_eq!(r["calib.intent"]["file"], "intent.json");
    assert_eq!(r["calib.intent"]["bands"][0]["band"], "goals_per_match");

    let stopped = data.path().join("stopped");
    let out = calibrate(
        &content,
        data.path(),
        &stopped,
        &[
            "--intent",
            given.to_str().unwrap(),
            "--stop-after-units",
            "1",
        ],
    );
    let kept = stopped.join("intent.json");
    assert!(kept.is_file(), "{}", stderr(&out));
    let written = std::fs::metadata(&kept).unwrap().modified().unwrap();
    let first_row = std::fs::read_dir(stopped.join("rows"))
        .unwrap()
        .map(|e| e.unwrap().metadata().unwrap().modified().unwrap())
        .min()
        .expect("a row file");
    assert!(written <= first_row, "{written:?} after {first_row:?}");
}

/// Without `--intent` and with no terminal, a change run stops before the old engine and any
/// match, naming the missing intent; an intent naming a band the band file does not hold
/// stops it before the old engine is built.
#[test]
fn a_change_run_without_an_intent_stops_before_any_match() {
    let data = tempfile::TempDir::new().unwrap();
    let content = edited_content(data.path(), "tuning.json", |_| {});
    let run = data.path().join("run");
    let start = |extra: &[&str]| {
        bin(data.path())
            .arg("--content-dir")
            .arg(&content)
            .arg("calibrate")
            .args(["--seed", "9", "--suite", "equal", "--minutes", "3"])
            .args(extra)
            .arg("--out")
            .arg(&run)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let out = start(&["--base-binary", exe()]);
    let text = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("a change run needs its intent"), "{text}");
    assert!(text.contains("no match was played"), "{text}");
    assert!(!text.contains("results from the cache"), "{text}");
    assert!(!run.join("run.json").exists(), "the run folder was started");
    assert!(!run.join("rows").exists(), "a match was played");

    let bad = intent_file(data.path(), &[("no_such_band", "up")]);
    let out = start(&[
        "--base",
        "HEAD",
        "--inject-failure",
        "base-build",
        "--intent",
        bad.to_str().unwrap(),
    ]);
    let text = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("no_such_band"), "{text}");
    assert!(!text.contains("injected base build failure"), "{text}");
    assert!(!run.join("run.json").exists());
}

/// What the change touched selects the rules: a content edit its settings' rules, engine code
/// every rule, and a ready old engine with no revision every rule. A build without the
/// sensitivity feature plays none of them and says so.
#[cfg(not(feature = "sensitivity"))]
#[test]
fn the_settings_a_change_touched_select_its_rules() {
    let repo = fixture_repo(|d| few_bands(d, 2, 3, false));
    let content = repo.path().join("content");
    edit_json(&content.join("attributes.json"), |a| {
        a["actions"]["pass"]["execute"]["main"]["weight"] = json!(0.9);
    });
    let data = tempfile::TempDir::new().unwrap();
    let none = intent_file(data.path(), &[]);
    let intent = ["--intent", none.to_str().unwrap()];

    let run = data.path().join("content-edit");
    let out = calibrate(&content, data.path(), &run, &intent);
    let text = stderr(&out);
    let r = report(&out, &run);
    let touched = &r["calib.rules"]["touched"];
    assert_eq!(touched["reason"], "content", "{touched}");
    assert_eq!(
        touched["settings"],
        json!(["attributes.json#/actions/pass/execute/main/weight"])
    );
    assert_eq!(touched["rules"], json!(["passing", "technique", "vision"]));
    assert_eq!(touched["all"], false);
    assert!(
        text.contains("this build has no sensitivity rules"),
        "{text}"
    );
    assert!(r["calib.rules"]["rows"].as_array().unwrap().is_empty());

    // An untracked file under crates/ is engine code: every rule.
    std::fs::create_dir_all(repo.path().join("crates/x")).unwrap();
    std::fs::write(repo.path().join("crates/x/lib.rs"), "").unwrap();
    let run = data.path().join("code");
    let r = report(&calibrate(&content, data.path(), &run, &intent), &run);
    let touched = &r["calib.rules"]["touched"];
    assert_eq!(touched["reason"], "code", "{touched}");
    assert_eq!(touched["code"], json!(["crates/x/lib.rs"]));
    assert_eq!(touched["all"], true);

    // A ready old engine with no revision: nothing to compare with, every rule.
    let run = data.path().join("no-base");
    let out = bin(data.path())
        .arg("--content-dir")
        .arg(&content)
        .arg("calibrate")
        .args(["--seed", "9", "--suite", "equal", "--minutes", "5"])
        .args(["--jobs", "4", "--base-binary", exe()])
        .args(intent)
        .arg("--out")
        .arg(&run)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let r = report(&out, &run);
    let touched = &r["calib.rules"]["touched"];
    assert_eq!(touched["reason"], "no-base", "{touched}");
    assert_eq!(touched["all"], true);
}

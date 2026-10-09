//! The bands that only report, in `engine-cli calibrate`: judged by the same rules as the
//! gating bands but in a table and a report list of their own (`calib.reported`), they never
//! move the joint word, its threshold, `calib.pass` or the exit code; a band whose real
//! sample is under its own floor only reports; a not-sure band can be accepted by the
//! developer and is then marked "accepted, not shown equivalent" without counting as a pass;
//! and the report counts the matches each engine played.

mod common;

use std::path::Path;
use std::process::Output;

use common::{RecordSchemas, bin, edited_content, record, temp};
use serde_json::{Value, json};

fn exe() -> &'static str {
    env!("CARGO_BIN_EXE_engine-cli")
}

/// A short calibrate run of the equal suite into `out`, on `content`.
fn calibrate(data: &Path, content: &Path, out: &Path, args: &[&str]) -> Output {
    bin(data)
        .arg("--content-dir")
        .arg(content)
        .arg("calibrate")
        .args(["--seed", "9", "--suite", "equal", "--minutes", "5"])
        .args(["--jobs", "4"])
        .args(args)
        .arg("--out")
        .arg(out)
        .output()
        .unwrap()
}

/// `engine-cli calibrate accept <band> --run <run>`.
fn accept(data: &Path, band: &str, run: &Path) -> Output {
    bin(data)
        .args(["calibrate", "accept", band, "--run"])
        .arg(run)
        .output()
        .unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The run's report, checked against the run-report schema.
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
    assert_eq!(report["schema.version"], "6");
    report
}

/// The rows of `list` in the report, empty when it is absent.
fn rows<'a>(report: &'a Value, list: &str) -> &'a [Value] {
    report[list].as_array().map_or(&[], Vec::as_slice)
}

/// The row of `band` in `list`, if any.
fn row<'a>(report: &'a Value, list: &str, band: &str) -> Option<&'a Value> {
    rows(report, list).iter().find(|r| r["band"] == band)
}

/// Edits the band `name` of the registry in `content`.
fn edit_band(content: &Path, name: &str, edit: impl FnOnce(&mut Value)) {
    let path = content.join("realism-bands.json");
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let band = doc["bands"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|b| b["band"] == name)
        .unwrap_or_else(|| panic!("no band {name}"));
    edit(band);
    std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
}

/// A band of the equal suite no match can sit inside: fouls per team between 0 and 0.1.
fn impossible_band(gates: bool) -> Value {
    json!({
        "band": "probe_fail",
        "suites": ["equal"],
        "measure": { "kind": "mean", "per": "team", "of": ["fouls.{side}"] },
        "design": { "lo": 0.0, "hi": 0.1, "smallest_shift": 0.05 },
        "real": {
            "source": { "none": "a test band with no real counterpart" },
            "sample": 0,
            "date": "2026-10-09"
        },
        "evidence": "calibration",
        "confidence": {
            "measurement": { "count": 0 },
            "generalisation": { "competitions": 0, "seasons": 0, "clubs": 0 }
        },
        "condition": "dry",
        "gates": gates
    })
}

/// What a verdict hangs on: the joint word, its threshold, the pass list and the gating rows.
fn verdict_of(r: &Value) -> Value {
    json!({
        "word": r["calib.joint"]["word"],
        "critical": r["calib.joint"]["critical"],
        "pass": r["calib.pass"],
        "verdicts": r["calib.verdicts"],
    })
}

/// A band that only reports, even one that fails, leaves a change run's verdict exactly as
/// it was: the same joint word, threshold, pass list, gating rows and exit code as the run
/// without the band; the band sits in `calib.reported` and the reported table, never in the
/// gating rows. The same band set to gate is judged with them.
#[test]
fn a_reported_band_never_moves_the_verdict_of_a_change_run() {
    let data = temp("engine-cli-reported", "verdict");
    let content = edited_content(&data, "tuning.json", |_| {});
    let run = data.join("run");
    let args = ["--matches", "4", "--base-binary", exe()];
    let first = calibrate(&data, &content, &run, &args);
    let before = report(&first, &run);
    assert!(row(&before, "calib.reported", "probe_fail").is_none());

    let doc_path = content.join("realism-bands.json");
    let add = |gates: bool| {
        let mut doc: Value =
            serde_json::from_str(&std::fs::read_to_string(&doc_path).unwrap()).unwrap();
        let bands = doc["bands"].as_array_mut().unwrap();
        bands.retain(|b| b["band"] != "probe_fail");
        bands.push(impossible_band(gates));
        std::fs::write(&doc_path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
    };
    add(false);
    let again = calibrate(&data, &content, &run, &args);
    let text = stderr(&again);
    let after = report(&again, &run);
    assert_eq!(again.status.code(), first.status.code(), "{text}");
    assert_eq!(verdict_of(&after), verdict_of(&before));
    assert_eq!(after["calib.power"], before["calib.power"]);
    let probe = row(&after, "calib.reported", "probe_fail").expect("a reported row");
    assert_eq!(probe["report_only"], "gates: false in the band file");
    assert_eq!(probe["word"], "fail", "{probe}");
    assert!(row(&after, "calib.verdicts", "probe_fail").is_none());
    assert!(
        text.contains("reported bands (they never change the verdict):"),
        "{text}"
    );
    assert!(
        text.contains("they never change the verdict\n"),
        "the reported count line: {text}"
    );

    // Set to gate, the same band is judged with the others and leaves the reported list.
    add(true);
    let gated_out = calibrate(&data, &content, &run, &args);
    let gated = report(&gated_out, &run);
    let probe = row(&gated, "calib.verdicts", "probe_fail").expect("a gating row");
    assert_eq!(probe["word"], "fail", "{probe}");
    assert!(row(&gated, "calib.reported", "probe_fail").is_none());
    assert_eq!(gated["calib.joint"]["word"], "fail");
    assert_eq!(gated["calib.pass"], false);
    assert_eq!(gated_out.status.code(), Some(2));
    let _ = std::fs::remove_dir_all(&data);
}

/// A band whose real sample is under its own floor only reports, with the reason; the
/// counts of a plain run name its matches and its build; and the per-club spread of a run
/// too short for its window reads no value, not sure, with the reason.
#[test]
fn a_real_sample_under_its_floor_only_reports_and_a_plain_run_counts_its_matches() {
    let data = temp("engine-cli-reported", "floor");
    let content = edited_content(&data, "tuning.json", |_| {});
    edit_band(&content, "corners_per_team", |b| {
        b["real"]["floor"] = 5000.into();
    });
    let run = data.join("run");
    let out = calibrate(&data, &content, &run, &["--matches", "4"]);
    let text = stderr(&out);
    let r = report(&out, &run);

    let corners = row(&r, "calib.reported", "corners_per_team").expect("corners only report");
    assert_eq!(
        corners["report_only"],
        "real sample too small to gate (sample 1826 under floor 5000)"
    );
    assert!(row(&r, "calib.verdicts", "corners_per_team").is_none());

    // The shipped reported bands of the equal suite are reported, each judged by its range.
    for band in [
        "goals_poisson_ratio",
        "five_one_share",
        "worst_to_best_spread",
    ] {
        let row = row(&r, "calib.reported", band).unwrap_or_else(|| panic!("no {band}: {r}"));
        assert!(row["word"].is_string(), "{row}");
    }
    let spread = row(&r, "calib.reported", "worst_to_best_spread").unwrap();
    assert_eq!(spread["word"], "not sure");
    assert_eq!(
        spread["note"],
        "no value: no club has 34 matches in this run"
    );

    let counts = &r["calib.counts"];
    assert_eq!(counts["total_matches"], 4);
    assert_eq!(counts["pairs"], 0);
    let obs = counts["observations"].as_array().unwrap();
    assert_eq!(obs.len(), 1, "{counts}");
    assert_eq!(obs[0]["engine"], "this");
    assert_eq!(obs[0]["matches"], 4);
    assert!(obs[0]["build"].as_str().is_some_and(|b| !b.is_empty()));
    assert!(
        text.contains("matches: 4 in total; 0 pairs; observations: this engine 4"),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&data);
}

/// A not-sure band is accepted with `calibrate accept`; the next judgement marks it
/// "accepted, not shown equivalent", its word stays not sure and it never counts as a pass;
/// a band that reads pass or fail, or that the run never judged, is refused by name; and an
/// acceptance file of another run is ignored, with the reason, and marks nothing.
#[test]
fn a_not_sure_band_is_accepted_and_marked_but_never_passes() {
    let data = temp("engine-cli-reported", "accept");
    let content = edited_content(&data, "tuning.json", |_| {});
    let run = data.join("run");
    let args = ["--matches", "2"];
    let first = calibrate(&data, &content, &run, &args);
    let before = report(&first, &run);
    let reported = rows(&before, "calib.reported");
    let not_sure = reported
        .iter()
        .find(|r| r["word"] == "not sure")
        .unwrap_or_else(|| panic!("a not-sure reported band in two matches: {reported:?}"))["band"]
        .as_str()
        .unwrap()
        .to_string();
    let sure = reported
        .iter()
        .find(|r| r["word"] != "not sure")
        .unwrap_or_else(|| panic!("a pass or fail reported band: {reported:?}"));
    let sure_band = sure["band"].as_str().unwrap();

    // Refusals: a band that reads pass or fail, and a band the run never judged.
    let refused = accept(&data, sure_band, &run);
    assert_ne!(refused.status.code(), Some(0));
    assert!(
        stderr(&refused).contains(&format!(
            "{sure_band} reads {} in run",
            sure["word"].as_str().unwrap()
        )),
        "{}",
        stderr(&refused)
    );
    assert!(
        stderr(&refused).contains("only a not sure band can be accepted"),
        "{}",
        stderr(&refused)
    );
    let unknown = accept(&data, "no_such_band", &run);
    assert_ne!(unknown.status.code(), Some(0));
    assert!(
        stderr(&unknown).contains("no_such_band has no verdict row"),
        "{}",
        stderr(&unknown)
    );
    assert!(!run.join("accepted.json").exists());

    let done = accept(&data, &not_sure, &run);
    assert_eq!(done.status.code(), Some(0), "{}", stderr(&done));
    assert!(
        stderr(&done).contains(&format!("accepted {not_sure} in run")),
        "{}",
        stderr(&done)
    );

    let again = calibrate(&data, &content, &run, &args);
    let text = stderr(&again);
    let after = report(&again, &run);
    assert_eq!(again.status.code(), first.status.code());
    let marked = row(&after, "calib.reported", &not_sure).unwrap();
    assert_eq!(marked["word"], "not sure");
    assert_eq!(marked["accepted"], true);
    assert_eq!(after["calib.accepted"]["bands"], json!([not_sure]));
    assert!(after["calib.accepted"].get("ignored").is_none());
    assert_eq!(verdict_of(&after), verdict_of(&before));
    let passes = |r: &Value| {
        rows(r, "calib.reported")
            .iter()
            .filter(|r| r["word"] == "pass")
            .count()
    };
    assert_eq!(passes(&after), passes(&before), "an acceptance is no pass");
    assert!(
        text.contains("not sure (accepted, not shown equivalent)"),
        "{text}"
    );
    assert!(
        text.contains("(1 accepted, not shown equivalent)"),
        "{text}"
    );

    // An acceptance file of another run marks nothing and says why.
    let path = run.join("accepted.json");
    let mut file: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let run_id = file["run_id"].as_str().unwrap().to_string();
    file["run_id"] = "another-run".into();
    std::fs::write(&path, file.to_string()).unwrap();
    let other = calibrate(&data, &content, &run, &args);
    let text = stderr(&other);
    let other = report(&other, &run);
    let ignored = other["calib.accepted"]["ignored"].as_str().unwrap();
    assert!(
        ignored.contains("another-run") && ignored.contains(&run_id),
        "{ignored}"
    );
    assert_eq!(other["calib.accepted"]["bands"], json!([]));
    assert!(row(&other, "calib.reported", &not_sure).unwrap()["accepted"].is_null());
    assert!(text.contains("acceptances ignored: "), "{text}");
    let _ = std::fs::remove_dir_all(&data);
}

/// A change run against an old engine whose build has no measures judges each reported
/// measures band by its range alone, and says why; its counts name both engines.
#[cfg(feature = "measures")]
#[test]
fn a_base_without_measures_judges_a_measures_band_by_its_range() {
    let data = temp("engine-cli-reported", "base-measures");
    let content = edited_content(&data, "tuning.json", |_| {});
    let run = data.join("run");
    let out = calibrate(
        &data,
        &content,
        &run,
        &[
            "--matches",
            "4",
            "--base-binary",
            exe(),
            "--inject-failure",
            "base-without-measures",
        ],
    );
    let r = report(&out, &run);
    let shot = row(&r, "calib.reported", "shot_distance_mean").expect("a shot distance row");
    assert_eq!(shot["judged"], "range", "{shot}");
    assert_eq!(shot["note"], "range only: base has no measures");
    let counts = &r["calib.counts"];
    assert_eq!(counts["pairs"], 4);
    let engines: Vec<&str> = counts["observations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["engine"].as_str().unwrap())
        .collect();
    assert_eq!(engines, ["changed", "old"]);
    let _ = std::fs::remove_dir_all(&data);
}

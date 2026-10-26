//! The bands that only report, in `engine-cli calibrate`: judged by the same rules as the
//! gating bands but in a table and a report list of their own (`calib.reported`), they never
//! move the joint word, its threshold, `calib.pass` or the exit code; a band whose real
//! sample is under its own floor only reports; a not-sure band can be accepted by the
//! developer and is then marked "accepted, not shown equivalent" without counting as a pass;
//! and the report counts the matches each engine played.

mod common;

use std::path::Path;
use std::process::Output;

use common::{RecordSchemas, bin, edited_content, hold_floor, record, set_counts, temp, test_band};
use serde_json::{Value, json};

fn exe() -> &'static str {
    env!("CARGO_BIN_EXE_engine-cli")
}

/// A short calibrate run of the equal suite into `out`, on `content`.
fn calibrate(data: &Path, content: &Path, out: &Path, args: &[&str]) -> Output {
    let run = || {
        bin(data)
            .arg("--content-dir")
            .arg(content)
            .arg("calibrate")
            .args(["--seed", "9", "--suite", "equal", "--minutes", "5"])
            .args(["--jobs", "4"])
            .args(common::change_intent(data, args))
            .args(args)
            .arg("--out")
            .arg(out)
            .output()
            .unwrap()
    };
    // A test's real-data folder may put some margins under their floor: widen them.
    let mut out = run();
    for _ in 0..3 {
        if !hold_floor(content, &String::from_utf8_lossy(&out.stderr)) {
            break;
        }
        out = run();
    }
    out
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
    assert_eq!(report["schema.version"], "12");
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

/// A band of the equal suite no match can meet: passes per team against a design target of
/// 0.05 with a margin of 0.05 (fouls per team until matches with no foul came in). A band
/// that does not gate belongs to the band file's `facing` group, which holds none here.
fn impossible_band(gates: bool) -> Value {
    let measure = json!({ "kind": "mean", "per": "team", "of": ["passes.{side}"] });
    let mut band = test_band("probe_fail", &["equal"], measure, 0.05, 0.05);
    if !gates {
        band["gates"] = false.into();
        band["group"] = "facing".into();
    }
    band
}

/// The reason the band file gives the `facing` group, which a report-only row shows.
const FACING: &str = "body facing is measured on two pose matches only";

/// Edits the band list of the band file in `content`.
fn edited_band_file(content: &Path, edit: impl FnOnce(&mut Vec<Value>)) {
    let path = content.join("realism-bands.json");
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    edit(doc["bands"].as_array_mut().unwrap());
    std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
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
    set_counts(&content, 4);
    let run = data.join("run");
    let args = ["--base-binary", exe()];
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
    assert_eq!(after["calib.count"], before["calib.count"]);
    let probe = row(&after, "calib.reported", "probe_fail").expect("a reported row");
    assert!(
        probe["report_only"].as_str().unwrap().contains(FACING),
        "{probe}"
    );
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

/// A band of a report-only group only reports, with its group's reason; the counts of a
/// plain run name its matches and its build; and the per-club spread of a run too short for
/// its window reads no value, not sure, with the reason.
#[test]
fn a_band_of_a_report_only_group_only_reports_and_a_plain_run_counts_its_matches() {
    let data = temp("engine-cli-reported", "floor");
    let content = edited_content(&data, "tuning.json", |_| {});
    edit_band(&content, "corners_per_team", |b| {
        b["gates"] = false.into();
        b["group"] = "facing".into();
    });
    let run = data.join("run");
    let out = calibrate(&data, &content, &run, &["--matches", "4"]);
    let text = stderr(&out);
    let r = report(&out, &run);

    let corners = row(&r, "calib.reported", "corners_per_team").expect("corners only report");
    assert!(
        corners["report_only"].as_str().unwrap().contains(FACING),
        "{corners}"
    );
    assert!(row(&r, "calib.verdicts", "corners_per_team").is_none());

    // The shipped rare bands of the equal suite only report, each with its word; the goal
    // shares by phase are read from the measures, so a build without them leaves them out.
    let five_one = row(&r, "calib.reported", "five_one_share").expect("five_one_share reports");
    assert!(five_one["word"].is_string(), "{five_one}");
    for band in [
        "goals_free_kick_share",
        "goals_penalty_share",
        "goals_throw_in_share",
    ] {
        let reported = row(&r, "calib.reported", band);
        assert_eq!(reported.is_some(), cfg!(feature = "measures"), "{band}");
        if let Some(row) = reported {
            assert!(row["word"].is_string(), "{row}");
        }
    }
    // The per-club spread gates now, and reads not sure with the reason.
    let spread = row(&r, "calib.verdicts", "worst_to_best_spread").unwrap();
    assert_eq!(spread["word"], "not sure");
    assert!(
        spread["note"]
            .as_str()
            .is_some_and(|n| n.ends_with("no value: no club has 34 matches in this run")),
        "{spread}"
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
}

/// A not-sure band is accepted with `calibrate accept`; the next judgement marks it
/// "accepted, not shown equivalent", its word stays not sure and it never counts as a pass;
/// a band that reads pass or fail, or that the run never judged, is refused by name; and an
/// acceptance file of another run is ignored, with the reason, and marks nothing.
#[test]
fn a_not_sure_band_is_accepted_and_marked_but_never_passes() {
    let data = temp("engine-cli-reported", "accept");
    let content = edited_content(&data, "tuning.json", |_| {});
    // A reported band that fails, beside the shipped rare bands, which read not sure.
    edited_band_file(&content, |bands| bands.push(impossible_band(false)));
    let run = data.join("run");
    // Six matches: enough for the bootstrap to judge the failing band, never too few units.
    let args = ["--matches", "6"];
    let first = calibrate(&data, &content, &run, &args);
    let before = report(&first, &run);
    let reported = rows(&before, "calib.reported");
    let not_sure = reported
        .iter()
        .find(|r| r["word"] == "not sure")
        .unwrap_or_else(|| panic!("a not-sure reported band in six matches: {reported:?}"))["band"]
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
    set_counts(&content, 4);
    let run = data.join("run");
    let out = calibrate(
        &data,
        &content,
        &run,
        &[
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
    // No word against an old engine without the measures, and the reason names it.
    assert!(shot.get("old_word").is_none(), "{shot}");
    assert!(
        shot["old_reason"]
            .as_str()
            .is_some_and(|r| r.ends_with("has no measures feature")),
        "{shot}"
    );
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

/// The shipped team-shape bands, the bands of the `shape_` groups: each name, and whether it
/// reads the shared report's figures (every one of them does: they exist only in a measures
/// build).
fn shape_bands() -> Vec<(String, bool)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/realism-bands.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    doc["bands"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["group"].as_str().is_some_and(|g| g.starts_with("shape_")))
        .map(|b| {
            let name = b["band"].as_str().unwrap().to_string();
            (name, b["measure"].to_string().contains("trk."))
        })
        .collect()
}

/// A tracked-match target planted in a copy of the reference file: seven matches whose mean
/// is `mean`, the jackknife error of that mean (the sample deviation over the root of 7).
fn plant_tracked(targets: &mut Value, band: &str, per_match: [f64; 7]) {
    let n = per_match.len() as f64;
    let mean = per_match.iter().sum::<f64>() / n;
    let var = per_match.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let t = &mut targets["targets"][format!("trk.dfl.{band}")];
    assert!(t.is_object(), "the reference file has no trk.dfl.{band}");
    t["value"] = json!(mean);
    t["se"] = json!((var / n).sqrt());
    t["per_match"] = json!(per_match);
}

/// The team-shape bands gate and the figures report: each shape band sits in the gating rows
/// and never in `calib.reported`; in a measures build it has a word, and a tracked-match
/// target planted in the reference file gives its real column and gap; a build without the
/// measures says so. The figures sit in `calib.figures`, each beside its earlier build's value
/// with the change, and without a value in a build without the measures. Runs in both builds.
#[test]
fn the_shape_bands_gate_and_the_figures_report_with_their_real_values() {
    let tmp = tempfile::TempDir::new().unwrap();
    let data = tmp.path().to_path_buf();
    let content = edited_content(&data, "tuning.json", |_| {});
    // The reference file beside the content folder, with two planted tracked targets:
    // centre-back depth in possession 10 to 12 metres (mean 11), calm 0.8 to 0.9 (mean 0.85).
    let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gate/real-targets.json");
    let mut targets: Value =
        serde_json::from_str(&std::fs::read_to_string(shipped).unwrap()).unwrap();
    plant_tracked(
        &mut targets,
        "cb_depth_in",
        [10.0, 12.0, 10.0, 12.0, 10.0, 12.0, 11.0],
    );
    plant_tracked(
        &mut targets,
        "dead_ball_calm_share",
        [0.8, 0.9, 0.8, 0.9, 0.8, 0.9, 0.85],
    );
    std::fs::create_dir_all(data.join("gate")).unwrap();
    std::fs::write(
        data.join("gate/real-targets.json"),
        serde_json::to_string_pretty(&targets).unwrap(),
    )
    .unwrap();
    let real = data.join("real-data");
    std::fs::create_dir_all(&real).unwrap();
    let run = data.join("run");
    let real_arg = real.display().to_string();
    let out = calibrate(
        &data,
        &content,
        &run,
        &["--matches", "4", "--real-data", &real_arg],
    );
    let text = stderr(&out);
    let r = report(&out, &run);
    let measured = cfg!(feature = "measures");

    // The 101 team-shape bands and their 101 PFF twins, which read the same figures.
    let bands = shape_bands();
    assert_eq!(bands.len(), 202);
    let gating: Vec<&str> = rows(&r, "calib.bands")
        .iter()
        .map(|b| b["band"].as_str().unwrap())
        .collect();
    assert!(
        bands.iter().all(|(_, m)| *m),
        "a shape band outside the shared report"
    );
    for (band, needs_measures) in &bands {
        assert!(
            row(&r, "calib.reported", band).is_none(),
            "{band} only reports"
        );
        assert!(gating.contains(&band.as_str()), "{band} not in calib.bands");
        if measured || !needs_measures {
            let verdict =
                row(&r, "calib.verdicts", band).unwrap_or_else(|| panic!("no {band}: {r}"));
            assert!(verdict["word"].is_string(), "{verdict}");
        }
    }
    assert_eq!(
        text.contains("this build has no measures"),
        !measured,
        "{text}"
    );
    if measured {
        for (band, real) in [("cb_depth_in", 11.0), ("dead_ball_calm_share", 0.85)] {
            let reported = row(&r, "calib.verdicts", band).unwrap();
            assert!(
                (reported["real"].as_f64().unwrap() - real).abs() < 1e-9,
                "{reported}"
            );
            let value = reported["value"]
                .as_f64()
                .unwrap_or_else(|| panic!("{reported}"));
            let gap = reported["distance"].as_f64().unwrap();
            assert!((gap - (value - real)).abs() < 1e-3, "{reported}");
        }
    }

    let figures = rows(&r, "calib.figures");
    let names: Vec<&str> = figures
        .iter()
        .map(|f| f["figure"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "top_two_receiver_share",
            "back_line_link_share",
            "engine_passes_per_team"
        ]
    );
    for f in figures {
        assert_eq!(f["suite"], "equal");
        assert_eq!(f["build"], "6aa7e09");
        let today = f["today"].as_f64().unwrap();
        // The pass count is a summary field, so every build gives it.
        if measured || f["figure"] == "engine_passes_per_team" {
            let value = f["value"].as_f64().unwrap_or_else(|| panic!("{f}"));
            let change = f["change"].as_f64().unwrap();
            assert!((change - (value - today)).abs() < 1e-3, "{f}");
        } else {
            assert!(f.get("value").is_none() && f.get("change").is_none(), "{f}");
        }
    }
    assert!(
        text.contains("figures (beside an earlier build; never judged):"),
        "{text}"
    );
}

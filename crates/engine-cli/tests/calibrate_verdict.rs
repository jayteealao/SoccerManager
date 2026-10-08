//! The verdict of `engine-cli calibrate`: bands from the registry in the content folder, a
//! change run sized by power and judged by one paired max-t test, a band judged again from
//! the stored rows when the registry changes, the time and memory of each stage, and the
//! rules stage.

mod common;

use std::path::{Path, PathBuf};
use std::process::Output;

use common::{RecordSchemas, bin, edited_content, read_rows, record, temp};
use serde_json::{Value, json};

const FLAG: &str = "card_storm";

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
    assert_eq!(report["schema.version"], "2");
    report
}

/// The verdict row of `band` in the report.
fn row<'a>(report: &'a Value, band: &str) -> &'a Value {
    report["calib.verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["band"] == band)
        .unwrap_or_else(|| panic!("no verdict for {band}: {report}"))
}

/// The shipped content with a flag that makes every foul a booked foul.
fn flagged(data: &Path) -> PathBuf {
    edited_content(data, "tuning.json", |tuning| {
        tuning["flags"] = json!({
            FLAG: {
                "owner": "engine team",
                "hypothesis": "more fouls, each booked, raise the yellow cards per team",
                "removal_condition": "a test flag; never shipped",
                "state": "off",
                "overrides": { "engine.foul_base": 1.0, "engine.yellow_base": 1.0 }
            }
        });
    })
}

/// The ledger lines of a run folder.
fn ledger_lines(run: &Path) -> usize {
    std::fs::read_dir(run.join("ledger"))
        .unwrap()
        .map(|e| {
            std::fs::read_to_string(e.unwrap().path())
                .unwrap()
                .lines()
                .count()
        })
        .sum()
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
        .unwrap();
    edit(band);
    std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
}

/// A change run whose cap is too small for power reports every band
/// without power as not sure or fail, never pass; its report and its console hold the time
/// and memory of each stage; and its rules stage checks zero rules without error.
#[test]
fn a_change_run_without_power_never_passes_and_reports_its_stages_and_rules() {
    let data = temp("engine-cli-verdict", "power");
    let content = edited_content(&data, "tuning.json", |_| {});
    let run = data.join("run");
    let out = calibrate(
        &data,
        &content,
        &run,
        &["--matches", "4", "--base-binary", exe()],
    );
    let text = stderr(&out);
    let report = report(&out, &run);
    assert_eq!(
        out.status.code(),
        Some(2),
        "a run that is not a pass exits 2"
    );
    let rows = report["calib.verdicts"].as_array().unwrap();
    assert_eq!(rows.len(), 15, "every band of the equal suite: {rows:?}");
    let unpowered: Vec<&Value> = rows.iter().filter(|r| r["power"] == false).collect();
    assert!(!unpowered.is_empty(), "4 matches give some band no power");
    for r in &unpowered {
        assert_ne!(r["word"], "pass", "{r}");
        assert!(r["word"] == "not sure" || r["word"] == "fail", "{r}");
    }
    assert!(rows.iter().any(|r| r["word"] == "not sure"), "{rows:?}");
    assert_eq!(report["calib.power"]["equal"]["cap"], 4);
    assert_eq!(report["calib.power"]["equal"]["target"], 4);
    assert_eq!(report["calib.power"]["equal"]["reached"], false);
    assert_ne!(report["calib.joint"]["word"], "pass");
    assert_eq!(report["calib.joint"]["pairs"], 4);
    assert!(text.contains("joint verdict: "), "{text}");
    assert!(text.contains("not sure"), "{text}");

    // Every stage with its time and memory, in the report and on the console.
    for stage in [
        "play",
        "checks",
        "commentary",
        "writing",
        "disk",
        "judge",
        "total",
    ] {
        let cost = &report["calib.stages"][stage];
        assert!(cost["ms"].is_u64(), "{stage}: {cost}");
        assert!(cost.get("peak_mb").is_some(), "{stage}: {cost}");
        assert!(
            text.lines().any(|l| l.starts_with(stage)),
            "no {stage} line in the stage table: {text}"
        );
    }
    assert!(report["calib.stages"]["total"]["ms"].as_u64().unwrap() > 0);
    assert!(
        text.contains("stage            time ms    peak MB"),
        "{text}"
    );

    // The rules stage runs, with no rule yet.
    assert_eq!(
        report["calib.rules"],
        json!({ "checked": 0, "levels": 0, "failed": 0 })
    );
    assert!(
        text.contains("rules stage: 0 touched sensitivity rules checked at 0 levels, 0 failed"),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&data);
}

/// A flag that moves one measure by more than its band's smallest shift is reported
/// as a move of that band, and the joint verdict fails. The flag is first shown to move the
/// measure in two plain runs, and the same change run without the flag shows no move.
#[test]
fn a_flag_that_shifts_a_measure_fails_its_band_and_the_joint_verdict() {
    let data = temp("engine-cli-verdict", "shift");
    let content = flagged(&data);
    let plain = |name: &str, args: &[&str]| {
        let run = data.join(name);
        let out = calibrate(
            &data,
            &content,
            &run,
            &[&["--matches", "12"], args].concat(),
        );
        row(&report(&out, &run), "yellow_cards_per_team")["value"]
            .as_f64()
            .unwrap()
    };
    let (off, on) = (plain("off", &[]), plain("on", &["--flag", "card_storm=on"]));
    assert!(
        on > off + 0.35,
        "the flag moves yellow cards: {off} -> {on}"
    );

    let change = |name: &str, args: &[&str]| {
        let run = data.join(name);
        let out = calibrate(
            &data,
            &content,
            &run,
            &[&["--matches", "30", "--base-binary", exe()], args].concat(),
        );
        (report(&out, &run), out.status.code())
    };
    let (flagged, code) = change("change", &["--flag", "card_storm=on"]);
    let yellow = row(&flagged, "yellow_cards_per_team");
    let critical = flagged["calib.joint"]["critical"].as_f64().unwrap();
    let (diff, se) = (
        yellow["diff"].as_f64().unwrap(),
        yellow["se"].as_f64().unwrap(),
    );
    assert!(
        diff > 0.35,
        "the change is above the smallest shift: {yellow}"
    );
    assert!(
        diff > critical * se,
        "the move is beyond the threshold: {yellow}"
    );
    assert_eq!(yellow["word"], "fail");
    assert_eq!(flagged["calib.joint"]["word"], "fail");
    assert_eq!(code, Some(2));

    let (same, _) = change("same", &[]);
    assert_eq!(row(&same, "yellow_cards_per_team")["diff"], 0.0);
    let _ = std::fs::remove_dir_all(&data);
}

/// After a finished run, a changed range in the registry is judged again from the
/// stored rows: no match plays, the console names the band, and the report holds the new
/// range.
#[test]
fn a_changed_range_is_judged_again_from_the_stored_rows() {
    let data = temp("engine-cli-verdict", "rejudge");
    let content = edited_content(&data, "tuning.json", |_| {});
    let run = data.join("run");
    let first = calibrate(&data, &content, &run, &["--matches", "8"]);
    let before = report(&first, &run);
    let lines = ledger_lines(&run);
    edit_band(&content, "corners_per_team", |b| {
        b["lo"] = 0.0.into();
        b["hi"] = 1.0.into();
    });
    let again = calibrate(&data, &content, &run, &["--matches", "8"]);
    let text = stderr(&again);
    let after = report(&again, &run);
    assert!(
        text.contains("judged again from 8 stored rows; 0 matches played"),
        "{text}"
    );
    assert!(
        text.contains("bands changed since the last judgement: corners_per_team"),
        "{text}"
    );
    assert_eq!(ledger_lines(&run), lines, "no new ledger line");
    assert_eq!(after["calib.units"]["played"], 0);
    assert_eq!(
        after["calib.results_digest"],
        before["calib.results_digest"]
    );
    let corners = row(&after, "corners_per_team");
    assert_eq!(
        (corners["lo"].as_f64(), corners["hi"].as_f64()),
        (Some(0.0), Some(1.0))
    );
    assert_ne!(
        after["calib.registry"]["digest"],
        before["calib.registry"]["digest"]
    );
    let _ = std::fs::remove_dir_all(&data);
}

/// A band added only in the content file is judged and reported with no code change,
/// and a version 2 bands file still loads with the same bands.
#[test]
fn a_new_band_in_the_content_file_is_judged_and_a_version_two_file_loads() {
    let data = temp("engine-cli-verdict", "registry");
    let content = edited_content(&data, "realism-bands.json", |doc| {
        doc["bands"].as_array_mut().unwrap().push(json!({
            "band": "fouls_per_team",
            "suites": ["equal"],
            "measure": { "kind": "mean", "per": "team", "of": ["fouls.{side}"] },
            "lo": 0.0, "hi": 30.0, "smallest_shift": 2.5
        }));
    });
    let run = data.join("run");
    let out = calibrate(&data, &content, &run, &["--matches", "4"]);
    let r = report(&out, &run);
    let check = r["calib.bands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["band"] == "fouls_per_team")
        .expect("the new band is checked");
    assert!(check["value"].as_f64().unwrap() >= 0.0);
    assert!(check["pass"].is_boolean());
    assert!(row(&r, "fouls_per_team")["word"].is_string());

    // The version 2 file as it shipped before the registry.
    let v2 = json!({
        "schema_version": 2,
        "sample_size": 1000,
        "goals_per_match": { "lo": 2.4, "hi": 3.2 },
        "shots_per_team": { "lo": 8.0, "hi": 16.0 },
        "possession_pct": { "lo": 35.0, "hi": 65.0 },
        "stronger_team": { "attribute_boost": 1.15, "min_win_rate": 0.5 },
        "wall_minutes_per_sample": 30.0,
        "ten_plus_goals_share": { "lo": 0.0, "hi": 0.005 },
        "sending_off_share": { "lo": 0.08, "hi": 0.22 },
        "yellow_cards_per_team": { "lo": 1.2, "hi": 2.6 },
        "shots_on_target_share": { "lo": 0.3, "hi": 0.42 },
        "goals_per_xg": { "lo": 0.85, "hi": 1.15 },
        "passes_per_team": { "lo": 350.0, "hi": 550.0 },
        "pass_accuracy_pct": { "lo": 75.0, "hi": 88.0 },
        "corners_per_team": { "lo": 3.5, "hi": 6.5 },
        "throw_ins_per_match": { "lo": 35.0, "hi": 55.0 },
        "goal_kicks_per_match": { "lo": 12.0, "hi": 22.0 },
        "goalless_share": { "lo": 0.04, "hi": 0.12 }
    });
    std::fs::write(content.join("realism-bands.json"), v2.to_string()).unwrap();
    let old_run = data.join("v2");
    let out = calibrate(&data, &content, &old_run, &["--matches", "4"]);
    let old = report(&out, &old_run);
    assert_eq!(old["calib.registry"]["migrated_from"], 2);
    let shipped = record(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/realism-bands.json"),
        )
        .unwrap(),
    );
    let bands = old["calib.bands"].as_array().unwrap();
    for b in shipped["bands"].as_array().unwrap() {
        if !b["suites"].as_array().unwrap().contains(&json!("equal")) {
            continue;
        }
        let check = bands
            .iter()
            .find(|c| c["band"] == b["band"])
            .unwrap_or_else(|| panic!("no {}", b["band"]));
        assert_eq!(
            (&check["lo"], &check["hi"]),
            (&b["lo"], &b["hi"]),
            "{}",
            b["band"]
        );
    }
    let _ = std::fs::remove_dir_all(&data);
}

/// The same matches give the same result digest and the same verdicts in every
/// observation mode: a sample of full recordings, or every match recorded.
#[test]
fn every_observation_mode_gives_the_same_digest_and_verdicts() {
    let data = temp("engine-cli-verdict", "modes");
    let content = edited_content(&data, "tuning.json", |_| {});
    let mode = |keep: &str| {
        let run = data.join(keep);
        let out = calibrate(
            &data,
            &content,
            &run,
            &["--matches", "6", "--keep-events", keep],
        );
        report(&out, &run)
    };
    let (sampled, all) = (mode("outliers"), mode("all"));
    assert_eq!(sampled["calib.results_digest"], all["calib.results_digest"]);
    assert_eq!(sampled["calib.verdicts"], all["calib.verdicts"]);
    assert_ne!(
        sampled["calib.rows"]["recorded"]["matches"],
        all["calib.rows"]["recorded"]["matches"]
    );
    let _ = std::fs::remove_dir_all(&data);
}

/// A tuning change from start to verdict: it runs as a change run against the
/// old engine; it is stopped and resumed with no fixture played twice; its pilot sets the
/// power target and the run grows to it; it ends with a word per band, the joint verdict,
/// and the stage costs; and a narrowed range is judged again from the stored rows with no
/// match played by either engine.
#[test]
fn a_tuning_change_runs_from_start_to_verdict() {
    let data = temp("engine-cli-verdict", "whole");
    let content = flagged(&data);
    let run = data.join("run");
    let args = [
        "--matches",
        "24",
        "--pilot",
        "8",
        "--flag",
        "card_storm=on",
        "--base-binary",
        exe(),
    ];
    // Steps 2 and 3, stopped part-way through the pilot.
    let stopped = calibrate(
        &data,
        &content,
        &run,
        &[&args[..], &["--stop-after-units", "1"]].concat(),
    );
    assert_eq!(stopped.status.code(), Some(1), "{}", stderr(&stopped));
    assert!(stderr(&stopped).contains("stopped after 1 work units"));

    // Step 4: the same command resumes; steps 5: the verdict with the stage costs.
    let resumed = calibrate(&data, &content, &run, &args);
    let text = stderr(&resumed);
    let r = report(&resumed, &run);
    assert!(text.contains("resuming run"), "{text}");
    let power = &r["calib.power"]["equal"];
    assert_eq!(power["pilot"], 8);
    let target = power["target"].as_u64().unwrap();
    assert!((8..=24).contains(&target), "{power}");
    // The run grows past its pilot only when the pilot's spread asks for more matches.
    assert_eq!(
        text.contains("pilot of 8 matches judged"),
        target > 8,
        "{text}"
    );
    let keys: Vec<String> = read_rows(&run).iter().map(|row| row.key()).collect();
    let mut unique = keys.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), keys.len(), "no fixture played twice");
    assert_eq!(keys.len() as u64, target);
    assert!(text.contains("joint verdict: fail"), "{text}");
    assert_eq!(row(&r, "yellow_cards_per_team")["word"], "fail");
    assert!(
        text.contains("stage            time ms    peak MB"),
        "{text}"
    );
    let lines = ledger_lines(&run);

    // Step 6: a narrowed range, judged again from the stored rows.
    edit_band(&content, "goals_per_match", |b| b["hi"] = 3.0.into());
    let again = calibrate(&data, &content, &run, &args);
    let text = stderr(&again);
    let after = report(&again, &run);
    assert!(
        text.contains(&format!(
            "judged again from {target} stored rows; 0 matches played"
        )),
        "{text}"
    );
    assert!(
        text.contains(": 0 played") || text.contains("results from the cache, 0 played"),
        "{text}"
    );
    assert!(text.contains("goals_per_match"), "{text}");
    assert_eq!(ledger_lines(&run), lines);
    assert_eq!(after["calib.base"]["cache"]["played"], 0);
    assert_eq!(row(&after, "goals_per_match")["hi"], 3.0);
    let _ = std::fs::remove_dir_all(&data);
}

/// Two pairings of the formations suite: enough for a pooled band over several pairings,
/// short enough for the default test run.
const PAIRINGS: [&str; 2] = ["4-4-2 v 4-3-3", "4-4-2 v 4-2-3-1"];

/// The bands the shipped registry judges in the formations suite.
const FORMATIONS_BANDS: [&str; 3] = ["goalless_share", "goals_per_match", "ten_plus_goals_share"];

/// A short calibrate run of the formations suite on two pairings into `out`.
fn formations(data: &Path, content: &Path, out: &Path, args: &[&str]) -> Output {
    let mut cmd = bin(data);
    cmd.arg("--content-dir")
        .arg(content)
        .arg("calibrate")
        .args(["--seed", "9", "--suite", "formations", "--minutes", "5"])
        .args(["--jobs", "4"]);
    for p in PAIRINGS {
        cmd.args(["--pairing", p]);
    }
    cmd.args(args).arg("--out").arg(out).output().unwrap()
}

/// The formations rows of `calib.bands`, without the time budget.
fn formations_checks(report: &Value) -> Vec<&Value> {
    report["calib.bands"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["suite"] == "formations" && b["band"] != "wall_ms")
        .collect()
}

/// A change run judges and powers each formations band once, pooled over every pairing:
/// one verdict row per band with no pairing, a power target named with the band that set
/// it, and per-pairing rows reported as information only. A per-pairing row outside its
/// range with the pooled band inside does not fail the joint verdict.
#[test]
fn a_change_run_judges_the_formations_suite_pooled_and_reports_pairings_as_information() {
    let data = temp("engine-cli-verdict", "pooled");
    let content = edited_content(&data, "tuning.json", |_| {});
    // Five-minute matches score far below the shipped goals ranges, and a band that far
    // outside its range sets no power target; widened, the goals bands size the run.
    edit_band(&content, "goals_per_match", |b| {
        b["lo"] = 0.0.into();
        b["hi"] = 10.0.into();
    });
    edit_band(&content, "goalless_share", |b| {
        b["lo"] = 0.0.into();
        b["hi"] = 1.0.into();
    });
    let run = data.join("run");
    let args = ["--matches", "8", "--pilot", "4", "--base-binary", exe()];
    let out = formations(&data, &content, &run, &args);
    let r = report(&out, &run);

    // One pooled verdict per formations band.
    let verdicts = r["calib.verdicts"].as_array().unwrap();
    for band in FORMATIONS_BANDS {
        let rows: Vec<&Value> = verdicts.iter().filter(|v| v["band"] == band).collect();
        assert_eq!(rows.len(), 1, "{band}: {verdicts:?}");
        assert_eq!(rows[0]["suite"], "formations");
        assert!(rows[0].get("pairing").is_none(), "{}", rows[0]);
    }
    assert_eq!(verdicts.len(), FORMATIONS_BANDS.len(), "{verdicts:?}");

    // Every per-pairing row is information; the suite's time budget is not a pairing row.
    let checks = formations_checks(&r);
    assert_eq!(checks.len(), FORMATIONS_BANDS.len() * PAIRINGS.len());
    for c in &checks {
        assert!(c["pairing"].is_string(), "{c}");
        assert_eq!(c["informational"], true, "{c}");
    }
    let budget = r["calib.bands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["band"] == "wall_ms" && b["suite"] == "formations")
        .unwrap();
    assert!(budget.get("informational").is_none(), "{budget}");

    // The power target is the pooled bands', and it names the band that set it.
    let power = &r["calib.power"]["formations"];
    assert_eq!(power["pilot"], 4);
    let target = power["target"].as_u64().unwrap();
    assert!((4..=8).contains(&target), "{power}");
    let driver = power["driver"].as_str().expect("a driver above the pilot");
    assert!(FORMATIONS_BANDS.contains(&driver), "{power}");
    let kept: Value =
        serde_json::from_str(&std::fs::read_to_string(run.join("target.json")).unwrap()).unwrap();
    assert_eq!(kept["drivers"]["formations"], driver);

    // Plant a per-pairing range miss with the pooled band inside: widen the bands the short
    // matches miss, and set the goals band's floor between the lower pairing and the pool.
    let pooled = row(&r, "goals_per_match")["value"].as_f64().unwrap();
    let low = checks
        .iter()
        .filter(|c| c["band"] == "goals_per_match")
        .map(|c| c["value"].as_f64().unwrap())
        .fold(f64::INFINITY, f64::min);
    assert!(
        low < pooled,
        "the pairings differ on seed 9: {low} {pooled}"
    );
    edit_band(&content, "goals_per_match", |b| {
        b["lo"] = ((low + pooled) / 2.0).into();
        b["hi"] = 10.0.into();
    });
    edit_band(&content, "goalless_share", |b| {
        b["lo"] = 0.0.into();
        b["hi"] = 1.0.into();
    });
    edit_band(&content, "ten_plus_goals_share", |b| b["hi"] = 1.0.into());
    let again = formations(&data, &content, &run, &args);
    let text = stderr(&again);
    let after = report(&again, &run);
    assert!(text.contains("0 matches played"), "{text}");
    let missed: Vec<&Value> = formations_checks(&after)
        .into_iter()
        .filter(|c| c["band"] == "goals_per_match" && c["pass"] == false)
        .collect();
    assert_eq!(missed.len(), 1, "one pairing below the floor: {after}");
    assert_eq!(missed[0]["informational"], true);
    assert!(
        after["calib.verdicts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["word"] != "fail"),
        "{after}"
    );
    assert!(after["calib.joint"].get("guards").is_none(), "no guard");
    assert_ne!(
        after["calib.joint"]["word"], "fail",
        "a per-pairing miss does not fail the joint verdict"
    );
    assert_eq!(after["calib.power"]["formations"]["driver"], driver);
    let _ = std::fs::remove_dir_all(&data);
}

/// A flag that moves goals per match by more than the band's smallest shift fails the
/// pooled formations band and the joint verdict; without the flag the band does not move.
/// The band's range is widened so that only the move can fail it.
#[test]
fn a_flag_that_shifts_goals_fails_the_pooled_formations_band_and_the_joint_verdict() {
    let data = temp("engine-cli-verdict", "pooled-shift");
    let content = edited_content(&data, "tuning.json", |tuning| {
        tuning["flags"] = json!({
            "open_goal": {
                "owner": "engine team",
                "hypothesis": "keepers save nothing, so more shots become goals",
                "removal_condition": "a test flag; never shipped",
                "state": "off",
                "overrides": {
                    "engine.keeper_catch_chance": 0.0,
                    "engine.shots.save_high": 0.0,
                    "engine.shots.save_low": 0.0
                }
            }
        });
    });
    edit_band(&content, "goals_per_match", |b| {
        b["lo"] = 0.0.into();
        b["hi"] = 10.0.into();
    });
    let change = |name: &str, args: &[&str]| {
        let run = data.join(name);
        let all = [
            &[
                "--matches",
                "24",
                "--band",
                "goals_per_match",
                "--base-binary",
                exe(),
            ],
            args,
        ]
        .concat();
        let out = formations(&data, &content, &run, &all);
        (report(&out, &run), out.status.code())
    };
    let (flagged, code) = change("flagged", &["--flag", "open_goal=on"]);
    let goals = row(&flagged, "goals_per_match");
    assert_eq!(goals["suite"], "formations");
    assert!(goals.get("pairing").is_none(), "pooled: {goals}");
    let critical = flagged["calib.joint"]["critical"].as_f64().unwrap();
    let (diff, se) = (
        goals["diff"].as_f64().unwrap(),
        goals["se"].as_f64().unwrap(),
    );
    assert!(diff > 0.2, "above the smallest shift: {goals}");
    assert!(diff > critical * se, "beyond the threshold: {goals}");
    assert!(
        goals["value"].as_f64().unwrap() <= 10.0,
        "inside the widened range: {goals}"
    );
    assert_eq!(goals["word"], "fail");
    assert_eq!(flagged["calib.joint"]["word"], "fail");
    assert!(flagged["calib.joint"].get("guards").is_none(), "no guard");
    assert_eq!(code, Some(2));

    let (same, _) = change("same", &[]);
    let goals = row(&same, "goals_per_match");
    assert_eq!(goals["diff"], 0.0);
    assert_ne!(goals["word"], "fail");
    let _ = std::fs::remove_dir_all(&data);
}

/// A plain run (no old engine) keeps one range check per formations pairing, exactly as
/// before the change run pooled the suite: the rows below are the ones the program gave on
/// this command before that change, and none carries the information mark.
#[test]
fn a_plain_run_keeps_the_formations_checks_per_pairing_unchanged() {
    let data = temp("engine-cli-verdict", "plain-pairings");
    let content = edited_content(&data, "tuning.json", |_| {});
    let run = data.join("run");
    let out = formations(&data, &content, &run, &["--matches", "8"]);
    let r = report(&out, &run);
    let expected = json!([
        {"band": "goalless_share", "pairing": "4-4-2 v 4-2-3-1", "value": 0.75, "lo": 0.04, "hi": 0.12, "pass": false, "se": 0.15309},
        {"band": "goalless_share", "pairing": "4-4-2 v 4-3-3", "value": 0.875, "lo": 0.04, "hi": 0.12, "pass": false, "se": 0.11693},
        {"band": "goals_per_match", "pairing": "4-4-2 v 4-2-3-1", "value": 0.25, "lo": 2.4, "hi": 3.2, "pass": false, "se": 0.15309},
        {"band": "goals_per_match", "pairing": "4-4-2 v 4-3-3", "value": 0.125, "lo": 2.4, "hi": 3.2, "pass": false, "se": 0.11693},
        {"band": "ten_plus_goals_share", "pairing": "4-4-2 v 4-2-3-1", "value": 0.0, "lo": 0.0, "hi": 0.005, "pass": true, "se": 0.0},
        {"band": "ten_plus_goals_share", "pairing": "4-4-2 v 4-3-3", "value": 0.0, "lo": 0.0, "hi": 0.005, "pass": true, "se": 0.0}
    ]);
    let rows: Vec<Value> = formations_checks(&r)
        .into_iter()
        .map(|c| {
            assert!(c.get("informational").is_none(), "{c}");
            json!({
                "band": c["band"], "pairing": c["pairing"], "value": c["value"],
                "lo": c["lo"], "hi": c["hi"], "pass": c["pass"], "se": c["se"]
            })
        })
        .collect();
    assert_eq!(Value::from(rows), expected);
    assert!(r.get("calib.power").is_none());
    // The change run's distance and no-target marks never reach a plain run's verdicts.
    for v in r["calib.verdicts"].as_array().unwrap() {
        assert!(v.get("outside_by").is_none(), "{v}");
        assert!(v.get("no_target").is_none(), "{v}");
    }
    let _ = std::fs::remove_dir_all(&data);
}

/// A band already outside its range by more than its noise at the pilot sets no power
/// target: it is judged (fail), reported with its distance and the mark, kept in
/// `target.json`, and kept when the run is judged again from its stored rows; the other
/// bands set the target. The same band inside its range with the same tiny smallest shift
/// sets the target at the cap.
#[test]
fn a_band_far_outside_its_range_sets_no_power_target() {
    let data = temp("engine-cli-verdict", "far-outside");
    let content = edited_content(&data, "tuning.json", |_| {});
    let band = "passes_per_team";
    let args = ["--pilot", "4", "--matches", "8", "--base-binary", exe()];
    let kept = |run: &Path| -> Value {
        serde_json::from_str(&std::fs::read_to_string(run.join("target.json")).unwrap()).unwrap()
    };
    let listed = |v: &Value| v.as_array().is_some_and(|l| l.iter().any(|x| x == band));

    // Far: a range no five-minute match comes near, and a smallest shift only the cap
    // could power.
    edit_band(&content, band, |b| {
        b["lo"] = 5000.0.into();
        b["hi"] = 6000.0.into();
        b["smallest_shift"] = 0.001.into();
    });
    let far = data.join("far");
    let out = calibrate(&data, &content, &far, &args);
    let text = stderr(&out);
    let r = report(&out, &far);
    let power = &r["calib.power"]["equal"];
    assert!(listed(&power["no_target"]), "{power}");
    assert_ne!(power["driver"], band, "{power}");
    let passes = row(&r, band);
    assert_eq!(passes["word"], "fail", "{passes}");
    assert_eq!(passes["no_target"], true, "{passes}");
    assert!(passes["outside_by"].as_f64().unwrap() > 4000.0, "{passes}");
    assert!(
        text.contains("beyond its noise at the pilot: set no power target"),
        "{text}"
    );
    assert_eq!(kept(&far)["no_target"]["equal"], power["no_target"]);

    // The same command again: judged from the stored rows, the marks stay.
    let again = calibrate(&data, &content, &far, &args);
    let text = stderr(&again);
    let after = report(&again, &far);
    assert!(text.contains("0 matches played"), "{text}");
    assert_eq!(
        after["calib.power"]["equal"]["no_target"],
        power["no_target"]
    );
    assert_eq!(row(&after, band)["no_target"], true);
    assert_eq!(kept(&far)["no_target"]["equal"], power["no_target"]);

    // Control: inside a wide range, the tiny shift drives the target to the cap.
    edit_band(&content, band, |b| {
        b["lo"] = 0.0.into();
        b["hi"] = 1000.0.into();
    });
    let control = data.join("control");
    let out = calibrate(&data, &content, &control, &args);
    let r = report(&out, &control);
    let power = &r["calib.power"]["equal"];
    assert_eq!(power["driver"], band, "{power}");
    assert_eq!(power["target"], 8, "{power}");
    assert!(!listed(&power["no_target"]), "{power}");
    let passes = row(&r, band);
    assert!(passes.get("no_target").is_none(), "{passes}");
    assert!(passes.get("outside_by").is_none(), "{passes}");
    let _ = std::fs::remove_dir_all(&data);
}

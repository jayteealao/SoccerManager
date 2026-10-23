//! The verdict of `engine-cli calibrate`: bands from the registry in the content folder, a
//! change run of its kind's fixed count judged by one paired max-t test, a band judged again
//! from the stored rows when the registry changes, the time and memory of each stage, and the
//! rules stage.

mod common;

use std::path::{Path, PathBuf};
use std::process::Output;

use common::{
    RecordSchemas, bin, edited_content, equal_gating_bands, hold_floor, read_rows, record,
    set_counts, set_design, temp, test_band,
};
use serde_json::{Value, json};

const FLAG: &str = "card_storm";

fn exe() -> &'static str {
    env!("CARGO_BIN_EXE_engine-cli")
}

/// A short calibrate run of the equal suite into `out`, on `content`.
fn calibrate(data: &Path, content: &Path, out: &Path, args: &[&str]) -> Output {
    calibrate_on("9", data, content, out, args)
}

/// [`calibrate`] from `seed`.
fn calibrate_on(seed: &str, data: &Path, content: &Path, out: &Path, args: &[&str]) -> Output {
    holding_floor(content, || {
        bin(data)
            .arg("--content-dir")
            .arg(content)
            .arg("calibrate")
            .args(["--seed", seed, "--suite", "equal", "--minutes", "5"])
            .args(["--jobs", "4"])
            .args(common::change_intent(data, args))
            .args(args)
            .arg("--out")
            .arg(out)
            .output()
            .unwrap()
    })
}

/// `run`, again after widening the margins a test's band edits put under their floor.
fn holding_floor(content: &Path, run: impl Fn() -> Output) -> Output {
    let mut out = run();
    for _ in 0..3 {
        if !hold_floor(content, &String::from_utf8_lossy(&out.stderr)) {
            break;
        }
        out = run();
    }
    out
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

/// Sets `band` to a design target at the middle of `lo` to `hi` with half that width as its
/// margin: the version 6 form of an earlier design range.
fn set_range(band: &mut Value, lo: f64, hi: f64) {
    set_design(band, (lo + hi) / 2.0, (hi - lo) / 2.0);
}

/// Edits the band `name` of the registry in `content`: `edit` gets the band.
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

/// A change run of a short fixed count plays exactly that count, never passes, gives every
/// band a word against the old engine or the reason it has none, and prints its headline;
/// its report and its console hold the time and memory of each stage; and, in a build
/// without the sensitivity rules, its rules stage checks no rule and says how to build them
/// in.
#[test]
fn a_short_change_run_never_passes_and_reports_its_stages_and_rules() {
    let data = temp("engine-cli-verdict", "short");
    let content = edited_content(&data, "tuning.json", |_| {});
    set_counts(&content, 4);
    let run = data.join("run");
    let out = calibrate(&data, &content, &run, &["--base-binary", exe()]);
    let text = stderr(&out);
    let report = report(&out, &run);
    assert_eq!(
        out.status.code(),
        Some(2),
        "a run that is not a pass exits 2"
    );
    let rows = report["calib.verdicts"].as_array().unwrap();
    assert_eq!(
        rows.len(),
        equal_gating_bands(),
        "every band of the equal suite: {rows:?}"
    );
    for r in rows {
        assert!(
            r["old_word"].is_string() || r["old_reason"].is_string(),
            "a word against the old engine or why none: {r}"
        );
    }
    assert!(rows.iter().any(|r| r["word"] == "not sure"), "{rows:?}");
    assert_eq!(report["calib.kind"], "change");
    assert_eq!(report["calib.count"]["equal"], 4);
    assert_eq!(read_rows(&run).len(), 4, "exactly the fixed count");
    assert_eq!(
        report["calib.dropped"],
        json!({"planned": 4, "kept": 4, "dropped": 0, "changed": 0, "old": 0, "both": 0})
    );
    let headline = report["calib.headline"].as_str().unwrap();
    assert!(
        headline.starts_with("kind change: fixed count equal 4, strength 4,"),
        "{headline}"
    );
    assert!(
        headline.ends_with("accepted: 0; dropped pairs: 0; run finished"),
        "{headline}"
    );
    assert!(text.contains(headline), "{text}");
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

    // The rules stage runs; this build has no rule (see `calibrate_rules.rs` for the rows). A
    // ready old engine with no revision has nothing to compare with, so every rule counts as
    // touched.
    assert_eq!(
        report["calib.rules"],
        json!({
            "kind": "change", "matches": 0,
            "touched": {
                "reason": "no-base", "why": "a ready old engine with no revision named",
                "settings": [], "unruled": [], "rules": [], "setting_rules": [], "all": true
            },
            "checked": 0, "levels": 0, "failed": 0, "not_sure": 0,
            "built": false, "ms": 0, "rows": []
        })
    );
    assert!(
        text.contains(
            "rules stage (kind change): no revision to compare with: a ready old engine with no \
             revision named; this build has no sensitivity rules; build engine-cli with \
             --features sensitivity to check them"
        ),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&data);
}

/// A flag that moves one measure by more than its band's smallest shift is reported
/// as a move of that band against the old engine, and the run does not pass. The flag is first shown to move the
/// measure in two plain runs, and the same change run without the flag shows no move. The runs
/// start from seed 18, the lowest whose 12 short matches and 60-match change run show the move
/// (seed 14 until clearances, shots and misreads were scored by their cost; seed 8 until
/// the computer manager grouped its substitutions and a player going for a loose ball stopped being pushed off it; seed 14 until a kick stayed on its way from its kicker along the line he sent it; seed 1 until a passer read the press his receiver would meet;
/// seed 9 until the player record's mood and personality came in, when its 12 matches moved
/// 0.29 cards).
#[test]
fn a_flag_that_shifts_a_measure_moves_its_band_against_the_old_engine() {
    let data = temp("engine-cli-verdict", "shift");
    let content = flagged(&data);
    edit_band(&content, "yellow_cards_per_team", |b| {
        set_range(b, 1.2, 2.6)
    });
    let plain = |name: &str, args: &[&str]| {
        let run = data.join(name);
        let out = calibrate_on(
            "18",
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

    set_counts(&content, 60);
    let change = |name: &str, args: &[&str]| {
        let run = data.join(name);
        let out = calibrate_on(
            "18",
            &data,
            &content,
            &run,
            &[&["--base-binary", exe()], args].concat(),
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
    assert_eq!(yellow["moved"], "up", "{yellow}");
    assert!(
        yellow["old_word"] == "better" || yellow["old_word"] == "worse",
        "a move is better or worse: {yellow}"
    );
    assert_ne!(flagged["calib.joint"]["word"], "pass");
    assert_eq!(code, Some(2));

    let (same, _) = change("same", &[]);
    let yellow = row(&same, "yellow_cards_per_team");
    assert_eq!(yellow["diff"], 0.0);
    assert!(yellow.get("moved").is_none(), "{yellow}");
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
        set_range(b, 0.0, 1.0);
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
        corners["pass_rule"]["margin"].as_f64(),
        Some(0.5),
        "{corners}"
    );
    assert_ne!(
        after["calib.registry"]["digest"],
        before["calib.registry"]["digest"]
    );
    let _ = std::fs::remove_dir_all(&data);
}

/// A band added only in the content file is judged and reported with no code change, and a
/// band file of an earlier version is refused, naming version 6.
#[test]
fn a_new_band_in_the_content_file_is_judged_and_an_earlier_file_is_refused() {
    let data = temp("engine-cli-verdict", "registry");
    let content = edited_content(&data, "realism-bands.json", |doc| {
        let measure = json!({ "kind": "mean", "per": "team", "of": ["fouls.{side}"] });
        doc["bands"].as_array_mut().unwrap().push(test_band(
            "probe_fouls_per_team",
            &["equal"],
            measure,
            15.0,
            15.0,
        ));
    });
    let run = data.join("run");
    let out = calibrate(&data, &content, &run, &["--matches", "4"]);
    let r = report(&out, &run);
    let check = r["calib.bands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["band"] == "probe_fouls_per_team")
        .expect("the new band is checked");
    assert!(check["value"].as_f64().unwrap() >= 0.0);
    assert!(check["pass"].is_boolean());
    assert!(row(&r, "probe_fouls_per_team")["word"].is_string());

    // The version 2 file as it shipped before the registry.
    let v2 = json!({
        "schema_version": 2,
        "sample_size": 1000,
        "goals_per_match": { "lo": 2.4, "hi": 3.2 },
        "stronger_team": { "attribute_boost": 1.15, "min_win_rate": 0.5 },
        "wall_minutes_per_sample": 30.0
    });
    std::fs::write(content.join("realism-bands.json"), v2.to_string()).unwrap();
    let old_run = data.join("v2");
    let out = calibrate(&data, &content, &old_run, &["--matches", "4"]);
    assert_eq!(out.status.code(), Some(1));
    let text = stderr(&out);
    assert!(text.contains("schema_version 2"), "{text}");
    assert!(text.contains("this build reads 6"), "{text}");
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
/// old engine; it is stopped and resumed with no fixture played twice; it plays its kind's
/// fixed count; it ends with a word per band, the joint verdict, and the stage costs; and a
/// narrowed range is judged again from the stored rows with no match played by either
/// engine.
#[test]
fn a_tuning_change_runs_from_start_to_verdict() {
    let data = temp("engine-cli-verdict", "whole");
    let content = flagged(&data);
    // A run here has no real data: one band is judged against a design target no engine
    // meets, so the run ends with a failed band and a failed joint verdict.
    edit_band(&content, "passes_per_team", |b| {
        set_range(b, 1000.0, 1100.0)
    });
    set_counts(&content, 12);
    let run = data.join("run");
    let args = ["--flag", "card_storm=on", "--base-binary", exe()];
    // Steps 2 and 3, stopped part-way through.
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
    let target = 12;
    assert_eq!(r["calib.count"]["equal"], target);
    let keys: Vec<String> = read_rows(&run).iter().map(|row| row.key()).collect();
    let mut unique = keys.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), keys.len(), "no fixture played twice");
    assert_eq!(keys.len() as u64, target);
    assert!(text.contains("joint verdict: fail"), "{text}");
    assert_eq!(row(&r, "passes_per_team")["word"], "fail");
    assert!(
        text.contains("stage            time ms    peak MB"),
        "{text}"
    );
    let lines = ledger_lines(&run);

    // Step 6: a narrowed range, judged again from the stored rows.
    edit_band(&content, "goals_per_match", |b| {
        set_range(b, 0.0, 3.0);
    });
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
    assert_eq!(row(&after, "goals_per_match")["pass_rule"]["margin"], 1.5);
    let _ = std::fs::remove_dir_all(&data);
}

/// Two pairings of the formations suite: enough for a pooled band over several pairings,
/// short enough for the default test run.
const PAIRINGS: [&str; 2] = ["4-4-2 v 4-3-3", "4-4-2 v 4-2-3-1"];

/// The bands the shipped registry judges in the formations suite.
const FORMATIONS_BANDS: [&str; 3] = ["goalless_share", "goals_per_match", "ten_plus_goals_share"];

/// A short calibrate run of the formations suite on two pairings into `out`.
fn formations(data: &Path, content: &Path, out: &Path, args: &[&str]) -> Output {
    formations_on("9", data, content, out, args)
}

/// [`formations`] on `seed`.
fn formations_on(seed: &str, data: &Path, content: &Path, out: &Path, args: &[&str]) -> Output {
    holding_floor(content, || {
        let mut cmd = bin(data);
        cmd.arg("--content-dir")
            .arg(content)
            .arg("calibrate")
            .args(["--seed", seed, "--suite", "formations", "--minutes", "5"])
            .args(["--jobs", "4"]);
        for p in PAIRINGS {
            cmd.args(["--pairing", p]);
        }
        cmd.args(common::change_intent(data, args))
            .args(args)
            .arg("--out")
            .arg(out)
            .output()
            .unwrap()
    })
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

/// A change run judges each formations band once, pooled over every pairing: one verdict
/// row per band with no pairing, its count per pairing, and per-pairing rows reported as
/// information only. A per-pairing row outside its
/// range with the pooled band inside does not fail the joint verdict. The runs play seed 0,
/// the lowest whose two pairings score differently (seed 1 until the computer manager's substitutions read the
/// threat a player's actions add, when both pairings scored 0.125; seed 3 until clearances, shots and
/// misreads were scored by their cost; seed 2 until a passer read the press his receiver would meet; seed 9 until tempo became patience and
/// the mentality an appetite for risk, when both pairings scored 0.375).
#[test]
fn a_change_run_judges_the_formations_suite_pooled_and_reports_pairings_as_information() {
    let data = temp("engine-cli-verdict", "pooled");
    let content = edited_content(&data, "tuning.json", |_| {});
    // Five-minute matches score far below the shipped goals ranges; widened, the goals
    // bands read the short matches.
    edit_band(&content, "goals_per_match", |b| {
        set_range(b, 0.0, 10.0);
    });
    edit_band(&content, "goalless_share", |b| {
        set_range(b, 0.0, 1.0);
    });
    set_counts(&content, 8);
    let run = data.join("run");
    let args = ["--base-binary", exe()];
    let out = formations_on("0", &data, &content, &run, &args);
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

    // The fixed count is per pairing: each pairing played it.
    assert_eq!(r["calib.count"]["formations"], 8);
    assert_eq!(read_rows(&run).len(), 8 * PAIRINGS.len());
    let kept: Value =
        serde_json::from_str(&std::fs::read_to_string(run.join("target.json")).unwrap()).unwrap();
    assert_eq!(kept["kind"], "change");
    assert_eq!(kept["target"]["formations"], 8);

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
        "the pairings differ on seed 0: {low} {pooled}"
    );
    edit_band(&content, "goals_per_match", |b| {
        set_range(b, (low + pooled) / 2.0, 10.0);
    });
    edit_band(&content, "goalless_share", |b| {
        set_range(b, 0.0, 1.0);
    });
    edit_band(&content, "ten_plus_goals_share", |b| {
        set_range(b, 0.0, 1.0);
    });
    let again = formations_on("0", &data, &content, &run, &args);
    let text = stderr(&again);
    let after = report(&again, &run);
    assert!(text.contains("0 matches played"), "{text}");
    let missed: Vec<&Value> = formations_checks(&after)
        .into_iter()
        .filter(|c| c["band"] == "goals_per_match" && c["pass"] == false)
        .collect();
    // The lower pairing misses; a pairing whose interval crosses the floor is not sure, which
    // is no pass either.
    assert!(
        missed.iter().any(|c| c["value"].as_f64() == Some(low)),
        "the lower pairing misses: {after}"
    );
    assert!(missed.iter().all(|c| c["informational"] == true), "{after}");
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
    let _ = std::fs::remove_dir_all(&data);
}

/// A flag that moves goals per match by more than the band's smallest shift is seen by the
/// paired test of the pooled formations band; with the band's target at the engine's own
/// level and a margin smaller than the move, it fails the band and the joint verdict, and
/// without the flag the band does not fail. The runs play seed 0, the
/// lowest whose 24 short matches show the move (seed 1 until every option was valued by the
/// real threat of its place, when its matches moved goals by 0.15; seed 9 until the AI sides
/// started in the style their staff picks, when its matches moved goals by 0.19).
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
        set_range(b, 0.0, 10.0);
    });
    set_counts(&content, 24);
    let change = |name: &str, args: &[&str]| {
        let run = data.join(name);
        let all = [
            &["--band", "goals_per_match", "--base-binary", exe()][..],
            args,
        ]
        .concat();
        let out = formations_on("0", &data, &content, &run, &all);
        (report(&out, &run), out.status.code())
    };
    let (flagged, _) = change("flagged", &["--flag", "open_goal=on"]);
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
    assert_eq!(goals["moved"], "up", "{goals}");
    let moved = goals["value"].as_f64().unwrap();
    let (same, _) = change("same", &[]);
    let goals = row(&same, "goals_per_match");
    assert_eq!(goals["diff"], 0.0);
    let level = goals["value"].as_f64().unwrap();
    // The report rounds a value to 3 decimals and a difference to 5, so the two runs' values
    // differ from the difference by at most the two roundings.
    assert!((moved - level - diff).abs() <= 1.1e-3, "{goals}");

    // A band passes only inside its margin around its target: with the target at the
    // engine's own level and a margin of a quarter of the move, the moved engine fails it
    // and the joint verdict fails; the same engine does not fail it. Both runs are judged
    // again from their stored rows.
    edit_band(&content, "goals_per_match", |b| {
        set_design(b, level, diff / 4.0)
    });
    let (flagged, code) = change("flagged", &["--flag", "open_goal=on"]);
    assert_eq!(flagged["calib.units"]["played"], 0);
    let goals = row(&flagged, "goals_per_match");
    assert_eq!(goals["word"], "fail", "{goals}");
    assert_eq!(flagged["calib.joint"]["word"], "fail");
    assert!(flagged["calib.joint"].get("guards").is_none(), "no guard");
    assert_eq!(code, Some(2));
    let (same, _) = change("same", &[]);
    assert_ne!(row(&same, "goals_per_match")["word"], "fail");
    let _ = std::fs::remove_dir_all(&data);
}

/// A plain run (no old engine) keeps one check per formations pairing, exactly as before the
/// change run pooled the suite: the rows below are the ones the program gives on this
/// command with the shipped engine, and none carries the information mark. With no real
/// value here no row passes.
#[test]
fn a_plain_run_keeps_the_formations_checks_per_pairing_unchanged() {
    let data = temp("engine-cli-verdict", "plain-pairings");
    let content = edited_content(&data, "tuning.json", |_| {});
    let run = data.join("run");
    let out = formations(&data, &content, &run, &["--matches", "8"]);
    let r = report(&out, &run);
    let expected = json!([
        {"band": "goalless_share", "pairing": "4-4-2 v 4-2-3-1", "value": 0.75, "pass": false, "se": 0.15309},
        {"band": "goalless_share", "pairing": "4-4-2 v 4-3-3", "value": 0.75, "pass": false, "se": 0.15309},
        {"band": "goals_per_match", "pairing": "4-4-2 v 4-2-3-1", "value": 0.25, "pass": false, "se": 0.15309},
        {"band": "goals_per_match", "pairing": "4-4-2 v 4-3-3", "value": 0.25, "pass": false, "se": 0.15309},
        {"band": "ten_plus_goals_share", "pairing": "4-4-2 v 4-2-3-1", "value": 0.0, "pass": false, "se": 0.0},
        {"band": "ten_plus_goals_share", "pairing": "4-4-2 v 4-3-3", "value": 0.0, "pass": false, "se": 0.0}
    ]);
    let rows: Vec<Value> = formations_checks(&r)
        .into_iter()
        .map(|c| {
            assert!(c.get("informational").is_none(), "{c}");
            json!({
                "band": c["band"], "pairing": c["pairing"], "value": c["value"],
                "pass": c["pass"], "se": c["se"]
            })
        })
        .collect();
    assert_eq!(Value::from(rows), expected);
    assert!(r.get("calib.power").is_none());
    // The change run's distance and old-engine words never reach a plain run's verdicts.
    for v in r["calib.verdicts"].as_array().unwrap() {
        assert!(v.get("outside_by").is_none(), "{v}");
        assert!(v.get("old_word").is_none(), "{v}");
    }
    assert!(r.get("calib.count").is_none());
    let _ = std::fs::remove_dir_all(&data);
}

/// The broken copies of `content/gate-proof.json`.
fn broken_copies() -> Vec<Value> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/gate-proof.json");
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    file["copies"].as_array().unwrap().clone()
}

/// The shipped content with each broken copy of the gate proof declared as a flag, off.
fn proof_content(data: &Path) -> PathBuf {
    edited_content(data, "tuning.json", |tuning| {
        for copy in broken_copies() {
            let mut body = copy["flag"].clone();
            body["state"] = "off".into();
            tuning["flags"][copy["name"].as_str().unwrap()] = body;
        }
    })
}

/// The value of `band` in a plain run's gating or reported checks.
fn band_value(report: &Value, band: &str) -> f64 {
    report["calib.bands"]
        .as_array()
        .unwrap()
        .iter()
        .chain(report["calib.reported"].as_array().unwrap())
        .find(|c| c["band"] == band && c.get("pairing").is_none())
        .and_then(|c| c["value"].as_f64())
        .unwrap_or_else(|| panic!("no value for {band}: {report}"))
}

/// Each broken copy of the gate proof, as a content flag, moves the measure it names in one
/// short run of whole matches: the same fixtures and seeds with the flag off and on.
#[test]
fn each_broken_copy_moves_the_measure_it_names() {
    let data = temp("engine-cli-verdict", "copies");
    let content = proof_content(&data);
    let value = |copy: &str, state: &str, band: &str| {
        let run = data.join(format!("{copy}-{state}"));
        let out = bin(&data)
            .arg("--content-dir")
            .arg(&content)
            .arg("calibrate")
            .args(["--seed", "9", "--suite", "equal", "--matches", "40"])
            .args(["--jobs", "4", "--flag", &format!("{copy}={state}")])
            .arg("--out")
            .arg(&run)
            .output()
            .unwrap();
        band_value(&report(&out, &run), band)
    };
    let copies = broken_copies();
    let names: Vec<&str> = copies.iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "shots_up",
            "fouls_halved",
            "card_storm",
            "pass_completion_down"
        ]
    );
    for copy in &copies {
        let (name, band) = (
            copy["name"].as_str().unwrap(),
            copy["measure"].as_str().unwrap(),
        );
        let (off, on) = (value(name, "off", band), value(name, "on", band));
        let moved = match name {
            "shots_up" => (1.05..=1.15).contains(&(on / off)),
            "fouls_halved" => (0.35..=0.65).contains(&(on / off)),
            "card_storm" => on - off > 0.35,
            "pass_completion_down" => (-7.0..=-3.0).contains(&(on - off)),
            other => panic!("an unknown copy {other}"),
        };
        assert!(moved, "{name} moved {band} from {off} to {on}");
    }
    let _ = std::fs::remove_dir_all(&data);
}

/// Writes a proof of the gate `(registry, measures, verdict_code)` into `content`.
fn write_proof(content: &Path, registry: &str, measures: u64, verdict_code: &str) {
    let path = content.join("gate-proof.json");
    let mut file: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    file["proof"] = json!({
        "gate": { "registry": registry, "measures": measures, "verdict_code": verdict_code },
        "made_at": "2026-10-09T00:00:00Z",
        "build": { "hash": "test", "measures": true },
        "seed": 1, "matches": 1, "minutes": 5,
        "self": {
            "runs": [], "pass_rate": 0.0, "fail_rate": 0.0, "not_sure_rate": 0.0,
            "false_alarm_rate": 0.0, "move_rate": 0.0
        },
        "copies": [],
        "distribution_bands": [],
        "counts": { "runs": 0, "total_matches": 0, "pairs": 0, "observations": {} }
    });
    std::fs::write(&path, serde_json::to_string_pretty(&file).unwrap()).unwrap();
}

/// The current value of `part` from its line `gate changed since the last proof: <part> <was>
/// -> <now>`.
fn current_part(text: &str, part: &str) -> String {
    let head = format!("gate changed since the last proof: {part} ");
    let line = text
        .lines()
        .find(|l| l.starts_with(&head))
        .unwrap_or_else(|| panic!("no {part} line: {text}"));
    line.rsplit(" -> ").next().unwrap().trim().to_string()
}

/// A proof file of another gate makes every run say so, part by part; a proof of this gate
/// makes it say nothing; a file without a proof and a folder without the file each say so.
#[test]
fn a_changed_gate_prints_the_warning_and_a_matching_one_prints_nothing() {
    let data = temp("engine-cli-verdict", "gate");
    let content = edited_content(&data, "tuning.json", |_| {});
    let run = |name: &str| {
        let out = calibrate(&data, &content, &data.join(name), &["--matches", "2"]);
        report(&out, &data.join(name));
        stderr(&out)
    };
    // The shipped file before its first proof, or a proof of this gate.
    write_proof(&content, "000000000000", 999, "000000000000");
    let path = content.join("gate-proof.json");
    let mut file: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    file["proof"] = Value::Null;
    std::fs::write(&path, serde_json::to_string_pretty(&file).unwrap()).unwrap();
    let text = run("none");
    assert!(
        text.contains("gate changed since the last proof: no proof recorded in gate-proof.json"),
        "{text}"
    );

    write_proof(&content, "000000000000", 999, "000000000000");
    let text = run("other");
    let registry = current_part(&text, "registry");
    let measures = current_part(&text, "measures");
    let code = current_part(&text, "verdict code");
    assert!(
        text.contains(&format!(
            "gate changed since the last proof: registry 000000000000 -> {registry}"
        )),
        "{text}"
    );
    assert_eq!(registry.len(), 12, "{text}");
    assert_eq!(code.len(), 12, "{text}");

    write_proof(&content, &registry, measures.parse().unwrap(), &code);
    let text = run("same");
    assert!(!text.contains("gate changed"), "{text}");
    assert!(!text.contains("no gate proof"), "{text}");

    std::fs::remove_file(&path).unwrap();
    let text = run("gone");
    assert!(
        text.contains("no gate proof in this content folder"),
        "{text}"
    );
    assert!(!text.contains("gate changed"), "{text}");
    let _ = std::fs::remove_dir_all(&data);
}

/// A tiny proof: one self run and one run of each copy. It writes the proof of the current
/// gate with its rates, the bands each copy moved and a line for every whole-distribution
/// band; the self run is not degenerate; and a run after it prints no warning.
#[cfg(feature = "measures")]
#[test]
fn a_tiny_proof_writes_the_file_and_silences_the_warning() {
    let data = temp("engine-cli-verdict", "proof");
    let content = edited_content(&data, "tuning.json", |_| {});
    set_counts(&content, 8);
    let runs = data.join("proof");
    let out = bin(&data)
        .arg("--content-dir")
        .arg(&content)
        .args(["calibrate", "proof", "--seed", "3"])
        .args(["--minutes", "5", "--self-runs", "1", "--copy-runs", "1"])
        .arg("--out")
        .arg(&runs)
        .output()
        .unwrap();
    let text = stderr(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.contains("gate proof run 5 of 5"), "{text}");
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(content.join("gate-proof.json")).unwrap())
            .unwrap();
    let proof = &file["proof"];
    assert_eq!(file["copies"].as_array().unwrap().len(), 4);
    let selfs = proof["self"]["runs"].as_array().unwrap();
    assert_eq!(selfs.len(), 1);
    assert_eq!(selfs[0]["reseed"], 1);
    assert!(selfs[0]["word"].is_string(), "{proof}");
    for key in ["pass_rate", "fail_rate", "not_sure_rate"] {
        assert!(proof["self"][key].is_f64(), "{key}: {proof}");
    }
    let copies = proof["copies"].as_array().unwrap();
    assert_eq!(copies.len(), 4);
    for c in copies {
        assert_eq!(c["runs"].as_array().unwrap().len(), 1, "{c}");
        assert!(c["fail_rate"].is_f64(), "{c}");
        assert!(c["moved"].is_object(), "{c}");
    }
    let bands: Vec<&str> = proof["distribution_bands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["band"].as_str().unwrap())
        .collect();
    assert_eq!(
        bands,
        [
            "goals_distribution",
            "scoreline_cells",
            "passes_per_team_distribution",
            "shot_distance_distribution",
            "chain_passes_distribution",
            "chain_seconds_distribution"
        ]
    );
    assert_eq!(proof["counts"]["runs"], 5);
    assert!(proof["counts"]["observations"]["changed"].as_u64().unwrap() > 0);
    assert_eq!(proof["build"]["measures"], true);

    // The reseeded self run: the two engines played other matches on the same fixtures.
    let self_report =
        record(&std::fs::read_to_string(runs.join("self-0").join("report.json")).unwrap());
    let moved = self_report["calib.verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["diff"].as_f64().is_some_and(|d| d != 0.0))
        .count();
    assert!(
        moved > 0,
        "a reseeded self run is not degenerate: {self_report}"
    );
    // A copy run is not reseeded: its old engine's matches are the self run's, from the cache.
    let copy_text = std::fs::read_to_string(runs.join("card_storm-0.log")).unwrap();
    assert!(copy_text.contains("0 played"), "{copy_text}");

    // The written proof measured this gate: the next run prints no warning.
    let check = calibrate(&data, &content, &data.join("check"), &["--matches", "2"]);
    let check_text = stderr(&check);
    report(&check, &data.join("check"));
    assert!(!check_text.contains("gate changed"), "{check_text}");
    let _ = std::fs::remove_dir_all(&data);
}

/// A build without the measures cannot prove the gate: it cannot judge every
/// whole-distribution band.
#[cfg(not(feature = "measures"))]
#[test]
fn a_proof_needs_the_measures_build() {
    let data = temp("engine-cli-verdict", "proof-plain");
    let out = bin(&data)
        .args(["calibrate", "proof", "--seed", "3"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let text = stderr(&out);
    assert!(text.contains("this build has no measures"), "{text}");
    assert!(
        text.contains("calibrate proof needs a build with the measures feature"),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&data);
}

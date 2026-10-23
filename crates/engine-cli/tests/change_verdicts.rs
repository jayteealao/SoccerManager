//! The words of a change run on a small fixture band file: the run plays exactly its kind's
//! fixed count of paired matches and refuses `--matches`; a kind refuses a seed of another
//! kind's set, and the release kind plays its own count with the normal rare-band margins;
//! a band worse against the old engine that passes against real football leaves the joint
//! word a pass; the headline names the count and what stayed open; and a count edited
//! without its CHANGELOG line fails the count check; and the shipped counts are the
//! computed ones, each with its CHANGELOG line.

mod common;

use std::path::{Path, PathBuf};
use std::process::Output;

use common::{RecordSchemas, bin, content_with_registry, read_rows, record, set_design};
use serde_json::{Value, json};

/// The fixture's bands: four means with wide margins, all of the equal suite.
const BANDS: [&str; 4] = [
    "goals_per_match",
    "corners_per_team",
    "fouls_per_team",
    "yellow_cards_per_team",
];

/// The fixture's rare share, beside [`BANDS`] when a test asks for it.
const RARE: &str = "ten_plus_goals_share";
/// The rare band's margin in the band file; a change run doubles it.
const RARE_MARGIN: f64 = 0.25;
/// The rare band's minimum exposure: above every count here, so it reads not sure, and high
/// enough for its margin to hold the floor.
const RARE_EXPOSURE: f64 = 40.0;

const FLAG: &str = "open_goal";

fn exe() -> &'static str {
    env!("CARGO_BIN_EXE_engine-cli")
}

/// The shipped content with the fixture band file: the bands of [`BANDS`] only, and [`RARE`]
/// when `rare`, each with a design target its five-minute matches meet, the change kind at
/// `change` matches per suite unit and the other kinds at `others`; and a flag with which
/// keepers save nothing.
fn fixture(change: u32, others: u32, rare: bool) -> (tempfile::TempDir, PathBuf) {
    let tmp = content_with_registry(|doc| {
        doc["bands"]
            .as_array_mut()
            .unwrap()
            .retain(|b| BANDS.iter().any(|n| b["band"] == *n) || (rare && b["band"] == RARE));
        for b in doc["bands"].as_array_mut().unwrap() {
            b["suites"] = json!(["equal"]);
            if b["band"] == RARE {
                set_design(b, 0.0, RARE_MARGIN);
                b["min_exposure"]["value"] = json!(RARE_EXPOSURE);
            } else {
                set_design(b, 0.0, 50.0);
            }
            if b["band"] == "goals_per_match" {
                // A small noticeable error: a move of the goals is a move, never equivalent.
                b["margin"]["noticeable_error"] = json!(0.01);
                b["margin"]["league_spread"] = json!(49.99);
            }
        }
        doc["report_only"] = json!([]);
        doc["figures"] = json!([]);
        let counts = |n: u32| json!({"equal": n, "strength": n, "formations": n, "red-card": n});
        doc["kinds"]["change"]["count"] = counts(change);
        doc["kinds"]["milestone"]["count"] = counts(others);
        doc["kinds"]["release"]["count"] = counts(others);
        // Counts set by hand, not computed: the computation goes with them.
        for kind in ["change", "milestone", "release"] {
            doc["kinds"][kind]
                .as_object_mut()
                .unwrap()
                .remove("computed");
        }
    });
    let content = tmp.path().join("content");
    let path = content.join("tuning.json");
    let mut tuning: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    tuning["flags"] = json!({
        FLAG: {
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
    std::fs::write(&path, serde_json::to_string_pretty(&tuning).unwrap()).unwrap();
    (tmp, content)
}

/// A change run of the equal suite on `content` from `seed`, into `out`, against this build.
fn change_run(data: &Path, content: &Path, out: &Path, seed: &str, args: &[&str]) -> Output {
    bin(data)
        .arg("--content-dir")
        .arg(content)
        .arg("calibrate")
        .args(["--seed", seed, "--suite", "equal", "--minutes", "5"])
        .args(["--jobs", "4", "--base-binary", exe()])
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
    report
}

/// The verdict row of `band`.
fn row<'a>(report: &'a Value, band: &str) -> &'a Value {
    report["calib.verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["band"] == band)
        .unwrap_or_else(|| panic!("no verdict for {band}: {report}"))
}

#[test]
fn a_change_run_plays_exactly_its_count_and_refuses_matches() {
    let (_tmp, content) = fixture(6, 3, true);
    let data = tempfile::TempDir::new().unwrap();
    let run = data.path().join("run");
    let out = change_run(data.path(), &content, &run, "9", &[]);
    let text = stderr(&out);
    let r = report(&out, &run);
    assert_eq!(r["calib.kind"], "change");
    assert_eq!(r["calib.count"], json!({"equal": 6}));
    assert_eq!(read_rows(&run).len(), 6, "exactly the change count");
    assert_eq!(r["calib.joint"]["pairs"], 6);
    assert_eq!(r["calib.dropped"]["planned"], 6);
    assert_eq!(r["calib.dropped"]["dropped"], 0);
    // The headline: the kind's counts, the not-sure gating bands, accepted and dropped.
    let not_sure = r["calib.verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["word"] == "not sure")
        .count();
    let headline = format!(
        "kind change: fixed count equal 6, strength 6, formations 6 per pairing, red-card 6 \
         per arm; not sure: {not_sure} of {} gating bands; accepted: 0; dropped pairs: 0",
        BANDS.len() + 1
    );
    assert!(not_sure >= 1, "the rare band is under its exposure: {r}");
    assert_eq!(r["calib.headline"], headline.as_str());
    assert!(text.contains(&headline), "{text}");
    // Every band has its word against the old engine; the same engine is never moved.
    for band in BANDS.iter().chain([&RARE]) {
        let v = row(&r, band);
        assert!(v["old_word"].is_string(), "{v}");
        assert!(v.get("moved").is_none(), "{v}");
    }
    assert!(r["calib.pair_agreement"]["median"].is_number(), "{r}");

    // A change run plays its kind's count: --matches is refused before any match.
    let refused = data.path().join("refused");
    let out = change_run(data.path(), &content, &refused, "9", &["--matches", "6"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("--matches: a change run plays the fixed count of its kind"),
        "{}",
        stderr(&out)
    );
    assert!(!refused.join("report.json").exists());
}

#[test]
fn a_kind_refuses_another_kinds_seed_and_the_release_kind_plays_its_own_count() {
    let (_tmp, content) = fixture(6, 3, true);
    let data = tempfile::TempDir::new().unwrap();
    let run = data.path().join("milestone");
    let out = change_run(data.path(), &content, &run, "9", &["--kind", "milestone"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("belongs to the change kind"),
        "{}",
        stderr(&out)
    );
    assert!(!run.join("report.json").exists());

    let release = data.path().join("release");
    let out = change_run(
        data.path(),
        &content,
        &release,
        "2000000",
        &["--kind", "release"],
    );
    let r = report(&out, &release);
    assert_eq!(r["calib.kind"], "release");
    assert_eq!(r["calib.count"], json!({"equal": 3}));
    assert_eq!(read_rows(&release).len(), 3);
    let margin = |r: &Value| row(r, RARE)["pass_rule"]["margin"].as_f64();
    assert_eq!(margin(&r), Some(RARE_MARGIN), "the normal margin");

    let change = data.path().join("change");
    let r = report(
        &change_run(data.path(), &content, &change, "9", &[]),
        &change,
    );
    assert_eq!(
        margin(&r),
        Some(2.0 * RARE_MARGIN),
        "a change run doubles it"
    );
}

/// A flag with which keepers save nothing moves the goals away from their target: worse
/// against the old engine, still inside the band's wide margin against real football. Only
/// the word against real football decides, so the joint word stays a pass.
#[test]
fn a_band_worse_against_the_old_engine_but_passing_keeps_the_joint_pass() {
    // Enough pairs for the five-minute matches' goals to show the move beyond the threshold.
    let (_tmp, content) = fixture(200, 3, false);
    let data = tempfile::TempDir::new().unwrap();
    let run = data.path().join("run");
    let flag = format!("{FLAG}=on");
    let out = change_run(data.path(), &content, &run, "9", &["--flag", &flag]);
    let r = report(&out, &run);
    let goals = row(&r, "goals_per_match");
    assert_eq!(goals["old_word"], "worse", "{goals}");
    assert_eq!(goals["moved"], "up", "{goals}");
    assert_eq!(goals["word"], "pass", "{goals}");
    for band in BANDS {
        assert_eq!(row(&r, band)["word"], "pass", "{}", row(&r, band));
    }
    assert_eq!(r["calib.joint"]["word"], "pass", "{}", r["calib.joint"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
}

/// The CHANGELOG lines that the counts of every kind in the band file `bands` need and
/// `changelog` lacks: one line per kind naming its count for each suite.
fn missing_count_lines(bands: &Path, changelog: &Path) -> Vec<String> {
    let bands: Value = serde_json::from_str(&std::fs::read_to_string(bands).unwrap()).unwrap();
    let changelog = std::fs::read_to_string(changelog).unwrap();
    ["change", "milestone", "release"]
        .iter()
        .map(|kind| {
            let count = &bands["kinds"][kind]["count"];
            format!(
                "fixed count {kind}: equal {}, strength {}, formations {}, red-card {}",
                count["equal"], count["strength"], count["formations"], count["red-card"]
            )
        })
        .filter(|line| !changelog.contains(line.as_str()))
        .collect()
}

/// The count check passes on a band file and a CHANGELOG that agree, and fails on a copy
/// whose count was edited without its line.
#[test]
fn a_count_edited_without_its_changelog_line_fails_the_check() {
    let (tmp, content) = fixture(6, 3, false);
    let bands = content.join("realism-bands.json");
    let changelog = tmp.path().join("CHANGELOG.md");
    std::fs::write(
        &changelog,
        "- fixed count change: equal 6, strength 6, formations 6, red-card 6\n\
         - fixed count milestone: equal 3, strength 3, formations 3, red-card 3\n\
         - fixed count release: equal 3, strength 3, formations 3, red-card 3\n",
    )
    .unwrap();
    assert_eq!(
        missing_count_lines(&bands, &changelog),
        Vec::<String>::new()
    );

    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&bands).unwrap()).unwrap();
    doc["kinds"]["release"]["count"]["equal"] = json!(4);
    std::fs::write(&bands, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
    assert_eq!(
        missing_count_lines(&bands, &changelog),
        vec!["fixed count release: equal 4, strength 3, formations 3, red-card 3".to_string()]
    );
}

/// Every kind's count in the shipped band file has its CHANGELOG line, so a count edited
/// without its entry fails here.
#[test]
fn every_kinds_shipped_count_has_its_changelog_line() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(
        missing_count_lines(
            &root.join("content/realism-bands.json"),
            &root.join("CHANGELOG.md")
        ),
        Vec::<String>::new()
    );
}

/// The shipped counts equal what `calibrate count` computes from the band file.
#[test]
fn the_shipped_counts_are_the_computed_counts() {
    let data = tempfile::TempDir::new().unwrap();
    let out = bin(data.path())
        .args(["calibrate", "count"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}{}", stderr(&out));
    assert!(!stdout.contains("the band file holds"), "{stdout}");
}

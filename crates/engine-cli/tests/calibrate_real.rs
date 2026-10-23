//! The real values of the realism bands, read in place by `engine-cli calibrate`: from a
//! fixture folder named by `--real-data`, each band's real value comes from its file and the
//! report records the folder and each band's source, sample size and date; at an absent
//! folder the run completes, every real column reads "no real data here", and the report and
//! the console headline state the missing path and the count.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::{RecordSchemas, bin, copy_tree, edited_content, hold_floor, record, set_design, temp};
use serde_json::{Value, json};

/// The bands the test points at fixture files, with the hand-checked value of each.
const READ: [(&str, f64); 5] = [
    // (2 + 1) + (0 + 0) + (3 + 3) over the three matches of competition A.
    ("goals_per_match", 3.0),
    // The mean of 20 and 26 shots per match, halved for one team.
    ("shots_per_team", 11.5),
    // 400, 500, 300 and 600 passes.
    ("passes_per_team", 450.0),
    ("shots_on_target_share", 0.4),
    // Matches 1 and 4 of the four hold tag 1701 or 1703.
    ("sending_off_share", 0.5),
];

/// The equal-suite band the test turns into a band with no real counterpart.
const NO_COUNTERPART: &str = "corners_per_team";

/// The fixture folder, copied to `data`, with the compressed events file written now.
fn fixture(data: &Path) -> PathBuf {
    let dir = data.join("real-data");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/real-data"),
        &dir,
    );
    let rows = "match,tags\n1,701 1802\n1,1701\n2,1801\n2,\n3,5\n4,1703\n";
    let file = std::fs::File::create(dir.join("wy2_events.csv.gz")).unwrap();
    let mut gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    gz.write_all(rows.as_bytes()).unwrap();
    gz.finish().unwrap();
    dir
}

fn pointer(file: &str, key: &str, extra: Value) -> Value {
    let mut p = json!({ "dataset": "fixture", "file": file, "key": key });
    for (k, v) in extra.as_object().unwrap() {
        p[k] = v.clone();
    }
    p
}

/// The shipped content with a band file whose pointers name the fixture's files: five bands
/// read a fixture value, one equal-suite band has no real counterpart, and every other band
/// with a counterpart points at a file the fixture lacks.
fn content(data: &Path) -> PathBuf {
    edited_content(data, "realism-bands.json", |doc| {
        for b in doc["bands"].as_array_mut().unwrap() {
            let name = b["band"].as_str().unwrap().to_string();
            let source = match name.as_str() {
                "goals_per_match" => pointer(
                    "wy2_matches.csv",
                    "hg+ag",
                    json!({ "reduce": "mean", "where": { "comp": ["A"] } }),
                ),
                "shots_per_team" => {
                    pointer("wyscout_rates.json", "*.shots", json!({ "scale": 0.5 }))
                }
                "passes_per_team" => pointer(
                    "wy2_team_matches.csv",
                    "passes",
                    json!({ "reduce": "mean" }),
                ),
                "shots_on_target_share" => pointer(
                    "sb_action_summary.json",
                    "shot.on_target",
                    json!({ "reduce": "share" }),
                ),
                "sending_off_share" => pointer(
                    "wy2_events.csv.gz",
                    "tags",
                    json!({ "reduce": "group_share", "group_by": "match", "tokens": ["1701", "1703"] }),
                ),
                NO_COUNTERPART => {
                    // A band with no real counterpart is judged against a design target.
                    set_design(b, 5.0, 2.0);
                    b["real"]["sample"] = 0.into();
                    b["confidence"] = json!({
                        "measurement": { "count": 0 },
                        "generalisation": { "competitions": 0, "seasons": 0, "clubs": 0 }
                    });
                    json!({ "none": "a test band: no real match repeats it" })
                }
                _ if b["real"]["source"].get("none").is_some() => continue,
                // A band read from the reference file keeps its target pointer.
                _ if b["real"]["source"].get("target").is_some() => continue,
                _ => pointer("not-in-the-fixture.csv", "x", json!({})),
            };
            b["real"]["source"] = source;
        }
    })
}

fn calibrate(data: &Path, content: &Path, out: &Path, real: &Path) -> Output {
    let run = || {
        bin(data)
            .arg("--content-dir")
            .arg(content)
            .arg("calibrate")
            .args(["--seed", "9", "--suite", "equal", "--minutes", "5"])
            .args(["--jobs", "4", "--matches", "4"])
            .arg("--real-data")
            .arg(real)
            .arg("--out")
            .arg(out)
            .output()
            .unwrap()
    };
    // The fixture's small real samples put some margins under their floor: widen them.
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
    assert_eq!(report["schema.version"], "11");
    report
}

fn verdict<'a>(report: &'a Value, band: &str) -> &'a Value {
    report["calib.verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["band"] == band)
        .unwrap_or_else(|| panic!("no verdict for {band}: {report}"))
}

fn entry<'a>(report: &'a Value, band: &str) -> &'a Value {
    report["calib.real"]["bands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["band"] == band)
        .unwrap_or_else(|| panic!("no real entry for {band}: {report}"))
}

/// The console line of `band` in the verdict table.
fn line<'a>(text: &'a str, band: &str) -> &'a str {
    text.lines()
        .find(|l| l.starts_with(&format!("{band} ")))
        .unwrap_or_else(|| panic!("no table line for {band}: {text}"))
}

/// The shipped file's bands: the 157 that gate and the 4 rare bands that only report.
const BANDS: u64 = 161;

/// The shipped bands with no real counterpart: the engine experiments and design values.
const SHIPPED_NO_COUNTERPART: u64 = 8;

/// Bands with a real source: the shipped file's, less those with no real counterpart and the
/// test's own band with none.
const POINTER_BANDS: u64 = BANDS - SHIPPED_NO_COUNTERPART - 1;

/// The bands of `content`'s band file that gate in the equal suite with calibration evidence.
fn calibration_bands(content: &Path) -> u64 {
    let path = content.join("realism-bands.json");
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    file["bands"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["gates"] != false && b["evidence"] == "calibration")
        .filter(|b| b["suites"].as_array().unwrap().iter().any(|s| s == "equal"))
        .count() as u64
}

#[test]
fn the_real_values_come_from_the_named_folder_and_an_absent_folder_is_stated() {
    let data = temp("engine-cli-real", "fixture");
    let real = fixture(&data);
    let content = content(&data);

    // The fixture folder: every pointed band reads its value, and the report records the
    // folder and every band's record.
    let run = data.join("run");
    let out = calibrate(&data, &content, &run, &real);
    let text = stderr(&out);
    let r = report(&out, &run);
    let info = &r["calib.real"];
    assert_eq!(info["folder"], real.display().to_string());
    assert_eq!(info["from"], "flag");
    assert_eq!(info["present"], true);
    assert!(info.get("missing_path").is_none(), "{info}");
    assert_eq!(info["no_counterpart"], SHIPPED_NO_COUNTERPART + 1, "{info}");
    assert_eq!(
        info["missing"].as_u64(),
        Some(POINTER_BANDS - READ.len() as u64),
        "{info}"
    );
    for (band, value) in READ {
        let e = entry(&r, band);
        assert_eq!(e["status"], "read", "{e}");
        assert!((e["value"].as_f64().unwrap() - value).abs() < 1e-9, "{e}");
        assert!(e["se"].as_f64().is_some(), "{band}: {e}");
        let v = verdict(&r, band);
        assert!((v["real"].as_f64().unwrap() - value).abs() < 1e-9, "{v}");
        let distance = v["value"].as_f64().unwrap() - value;
        assert!(
            (v["distance"].as_f64().unwrap() - distance).abs() < 1e-4,
            "{v}"
        );
    }
    // The group share over the compressed events: two of four matches, with its error.
    let sending = entry(&r, "sending_off_share");
    assert!((sending["se"].as_f64().unwrap() - (0.25f64 / 4.0).sqrt()).abs() < 1e-6);
    // Every band names its source, sample size and date.
    for e in info["bands"].as_array().unwrap() {
        assert!(e["source"].is_object(), "{e}");
        assert!(e["sample"].is_u64(), "{e}");
        assert!(e["date"].is_string(), "{e}");
        assert!(e["evidence"].is_string(), "{e}");
    }
    let none = entry(&r, NO_COUNTERPART);
    assert_eq!(none["status"], "no_counterpart");
    assert_eq!(none["generalisation"]["word"], "n/a");
    assert!(verdict(&r, NO_COUNTERPART).get("real").is_none());
    assert!(
        line(&text, NO_COUNTERPART).contains("no real counterpart"),
        "{text}"
    );
    assert!(
        text.contains(&format!(
            "real data: {} (flag); {} of {BANDS} bands without a real value; {} with no real counterpart",
            real.display(),
            POINTER_BANDS - READ.len() as u64,
            SHIPPED_NO_COUNTERPART + 1
        )),
        "{text}"
    );
    assert!(text.contains("realism claim: none"), "{text}");
    assert_eq!(
        r["calib.realism"]["calibration_bands"],
        calibration_bands(&content),
        "{}",
        r["calib.realism"]
    );

    // An absent folder: the run completes with the range verdict's exit code, every real
    // column reads "no real data here", and the report and the headline state the path and
    // the count.
    let absent = data.join("absent");
    let run2 = data.join("run-absent");
    let out2 = calibrate(&data, &content, &run2, &absent);
    let text2 = stderr(&out2);
    let r2 = report(&out2, &run2);
    assert_eq!(out2.status.code(), out.status.code(), "{text2}");
    assert_eq!(r2["calib.pass"], r["calib.pass"]);
    let info2 = &r2["calib.real"];
    assert_eq!(info2["present"], false);
    assert_eq!(info2["missing_path"], absent.display().to_string());
    assert_eq!(info2["missing"].as_u64(), Some(POINTER_BANDS), "{info2}");
    assert_eq!(info2["no_counterpart"], SHIPPED_NO_COUNTERPART + 1);
    assert!(
        text2.contains(&format!(
            "{POINTER_BANDS} of {BANDS} bands without a real value"
        )),
        "{text2}"
    );
    assert!(
        text2.contains(&format!(
            "no real data here: {} does not exist",
            absent.display()
        )),
        "{text2}"
    );
    for v in r2["calib.verdicts"].as_array().unwrap() {
        let band = v["band"].as_str().unwrap();
        assert!(
            v.get("real").is_none() && v.get("distance").is_none(),
            "{v}"
        );
        let expect = if band == NO_COUNTERPART {
            "no real counterpart"
        } else {
            "no real data here"
        };
        assert!(line(&text2, band).contains(expect), "{band}: {text2}");
    }
    // The verdicts are the same: the real data never changes a word.
    let words = |r: &Value| -> Vec<Value> {
        r["calib.verdicts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| json!([v["band"], v["word"]]))
            .collect()
    };
    assert_eq!(words(&r), words(&r2));
    let _ = std::fs::remove_dir_all(&data);
}

//! The open-data targets on invented files: StatsBomb targets carry their source label and
//! never read the three refused competition-seasons; the second event source's targets go
//! only to the local side file; Germany's strictness reads the two neighbour seasons and
//! never the held-out one; two builds give the same bytes.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let dest = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &dest);
        } else {
            std::fs::copy(e.path(), dest).unwrap();
        }
    }
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// A data folder: the reference fixture with the open-data fixture laid over it.
fn data(tmp: &Path) -> PathBuf {
    let d = tmp.join("data");
    copy_dir(&fixtures().join("real-targets"), &d);
    copy_dir(&fixtures().join("open-data"), &d);
    d
}

/// Runs the builder on `data` into `out/real-targets.json` and the side folder `side`.
fn run(data: &Path, out: &Path, side: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engine-cli"))
        .env("SM_LOG", "warn")
        .env("SM_BAND_HISTORY", "off")
        .env("SM_CONTENT_DIR", repo().join("content"))
        .args(["real-targets", "build", "--data"])
        .arg(data)
        .arg("--out")
        .arg(out.join("real-targets.json"))
        .arg("--side-dir")
        .arg(side)
        .output()
        .unwrap()
}

struct Build {
    file: Value,
    text: String,
    side: PathBuf,
    printed: String,
}

fn build(tmp: &Path, name: &str) -> Build {
    let out = tmp.join(name);
    let side = out.join(".scratch").join("real-targets");
    let o = run(&data(tmp), &out, &side);
    assert!(o.status.success(), "{}", text(&o));
    let path = out.join("real-targets.json");
    let file_text = std::fs::read_to_string(&path).unwrap();
    Build {
        file: serde_json::from_str(&file_text).unwrap(),
        text: file_text,
        side,
        printed: text(&o),
    }
}

fn value(t: &Value, key: &str) -> f64 {
    t[key]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{key}: {}", t[key]))
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 2e-6
}

const REFUSED: [&str; 3] = ["43_3", "11_1", "9_281"];

#[test]
fn statsbomb_targets_carry_their_label_and_never_read_a_refused_season() {
    let tmp = TempDir::new().unwrap();
    let b = build(tmp.path(), "a");
    let t = b.file["targets"].as_object().unwrap();
    let sb: Vec<(&String, &Value)> = t.iter().filter(|(k, _)| k.starts_with("sb.")).collect();
    assert!(!sb.is_empty());
    for (key, target) in &sb {
        assert_eq!(target["provider"], "StatsBomb", "{key}");
        assert!(
            target["source_label"]
                .as_str()
                .unwrap()
                .starts_with("StatsBomb open data, "),
            "{key}"
        );
        assert!(target["context"].as_str().is_some(), "{key}");
    }
    // No target, of any source, reads a refused season.
    for (key, target) in t {
        for s in target["seasons"].as_array().unwrap() {
            assert!(!REFUSED.contains(&s.as_str().unwrap()), "{key} reads {s}");
        }
        for id in REFUSED {
            assert!(!key.starts_with(&format!("sb.{id}.")), "{key}");
        }
    }
    let read: Vec<&str> = b.file["built_from"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    for rel in [
        "sb/matches/43_3.json",
        "sb/matches/11_1.json",
        "sb/events/3005.json",
    ] {
        assert!(!read.contains(&rel), "{rel} was read");
    }
    assert!(read.contains(&"sb/matches/2_27.json") && read.contains(&"sb/events/2_27"));
    // The planted decoy values (xG 9.99 and 7.77, a 999 s carry) reach no target: the
    // pool's figures are the hand-checked ones of its three matches alone.
    assert!(close(
        value(&b.file["targets"], "sb.men.shots_on_target_share"),
        0.875
    ));
    assert!(close(value(&b.file["targets"], "sb.men.goals_per_xg"), 2.5));
    assert!(close(
        value(&b.file["targets"], "sb.men.shot_distance_mean"),
        14.4375
    ));
    for (band, num) in [("own", 5.5), ("middle", 9.0), ("final", 5.0)] {
        let v = value(
            &b.file["targets"],
            &format!("sb.men.possession_{band}_third_share"),
        );
        assert!(close(v, num / 19.5), "{band}: {v}");
    }
    for origin in ["open_play", "corner", "free_kick", "penalty", "throw_in"] {
        let v = value(&b.file["targets"], &format!("sb.men.goals_{origin}_share"));
        assert!(close(v, 0.2), "{origin}: {v}");
    }
    let shares = |key: &str| -> Vec<f64> {
        b.file["targets"][key]["shares"]
            .as_array()
            .unwrap_or_else(|| panic!("{key}"))
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    };
    let d = shares("sb.men.dist.shot_distance");
    assert_eq!(d.len(), 41);
    for (bin, share) in [
        (7, 0.25),
        (10, 0.125),
        (14, 0.375),
        (21, 0.125),
        (28, 0.125),
    ] {
        assert!(close(d[bin], share), "bin {bin}: {}", d[bin]);
    }
    let p = shares("sb.men.dist.chain_passes");
    assert!(close(p[0], 2.0 / 9.0) && close(p[1], 6.0 / 9.0) && close(p[2], 1.0 / 9.0));
    let s = shares("sb.men.dist.chain_seconds");
    assert!(close(s[0], 3.0 / 9.0) && close(s[1], 4.0 / 9.0) && close(s[2], 2.0 / 9.0));
    let pool = &b.file["targets"]["sb.men.goals_per_xg"];
    assert_eq!(pool["n"], 3);
    assert_eq!(pool["seasons"], serde_json::json!(["2_27", "55_43"]));
    assert_eq!(pool["layer"], "pooled");
    // The women's season has its labelled targets and stays out of the pool.
    assert_eq!(
        b.file["targets"]["sb.37_90.goals_per_xg"]["context"],
        "women's club"
    );
    assert!(close(
        value(&b.file["targets"], "sb.37_90.goals_per_xg"),
        1.0 / 0.9
    ));
    for id in REFUSED {
        assert!(
            b.printed.contains(&format!("refused {id}")),
            "{}",
            b.printed
        );
    }
    // The per-match histograms of the pool go to the side folder.
    let hist = std::fs::read_to_string(b.side.join("statsbomb-match-histograms.csv")).unwrap();
    assert!(hist.starts_with("label,match,kind,bin,count\n"));
    assert!(hist.contains("2_27,3001,shot_distance,7,2\n"), "{hist}");
    for line in hist.lines().skip(1) {
        assert!(
            line.starts_with("2_27,") || line.starts_with("55_43,"),
            "{line}"
        );
    }
}

#[test]
fn the_second_event_source_goes_only_to_the_local_side_file() {
    let tmp = TempDir::new().unwrap();
    let b = build(tmp.path(), "a");
    let side: Value =
        serde_json::from_str(&std::fs::read_to_string(b.side.join("impect-targets.json")).unwrap())
            .unwrap();
    assert_eq!(side["side"], "impect");
    let targets = side["targets"].as_object().unwrap();
    assert!(targets.keys().all(|k| k.starts_with("imp.bl2324.")));
    assert!(close(
        value(&side["targets"], "imp.bl2324.goals_per_match"),
        0.5
    ));
    assert!(close(
        value(&side["targets"], "imp.bl2324.shots_per_team"),
        0.75
    ));
    assert!(close(
        value(&side["targets"], "imp.bl2324.pass_accuracy_pct"),
        75.0
    ));
    assert_eq!(
        targets["imp.bl2324.passes_per_team"]["engine_count"]["band"],
        "passes_per_team"
    );
    // The reference file holds none of it: no key, no credit, no file, no word.
    assert!(
        b.file["targets"]
            .as_object()
            .unwrap()
            .keys()
            .all(|k| !k.starts_with("imp."))
    );
    assert!(!b.text.to_lowercase().contains("impect"));
    assert!(b.printed.contains("local side file:"), "{}", b.printed);
}

#[test]
fn germany_strictness_reads_the_two_neighbour_seasons_only() {
    let tmp = TempDir::new().unwrap();
    let b = build(tmp.path(), "a");
    let s = &b.file["targets"]["strict.D1.fouls_per_team"];
    // 2016/17: (22 + 30 + 18 + 26) / 8 = 12; 2018/19: (26 + 34 + 22 + 30) / 8 = 14.
    assert!(close(s["value"].as_f64().unwrap(), 13.0), "{s}");
    assert_eq!(s["seasons"], serde_json::json!(["1617", "1819"]));
    let read: Vec<&str> = b.file["built_from"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert!(read.contains(&"football-data/1617/D1.csv"));
    assert!(read.contains(&"football-data/1819/D1.csv"));
    assert!(read.iter().all(|p| !p.contains("1718/")), "{read:?}");
    assert!(
        b.printed.contains("germany strictness: 13.0000"),
        "{}",
        b.printed
    );
    // The scan-rate target reads the chosen study row.
    let scan = &b.file["targets"]["lit.jordet2020.scans_per_second"];
    assert!(close(scan["value"].as_f64().unwrap(), 0.5), "{scan}");
    assert_eq!(scan["engine_count"]["band"], "scan_rate");
    assert!(b.file["credits"]["Jordet et al. 2020"].is_object());
}

#[test]
fn two_builds_give_the_same_bytes_for_every_output() {
    let tmp = TempDir::new().unwrap();
    let a = build(tmp.path(), "a");
    let b = build(tmp.path(), "b");
    assert_eq!(a.text, b.text);
    for f in ["impect-targets.json", "statsbomb-match-histograms.csv"] {
        assert_eq!(
            std::fs::read(a.side.join(f)).unwrap(),
            std::fs::read(b.side.join(f)).unwrap(),
            "{f}"
        );
    }
}

#[test]
fn a_side_folder_in_a_tracked_folder_is_refused() {
    let tmp = TempDir::new().unwrap();
    let side = repo().join("gate").join("odt-refused-side");
    let o = run(&data(tmp.path()), tmp.path(), &side);
    assert!(!o.status.success());
    assert!(text(&o).contains(".scratch"), "{}", text(&o));
    assert!(!side.exists());
}

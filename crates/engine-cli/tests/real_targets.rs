//! The reference file of real targets: the data check, the builder on invented fixture data
//! in the formats of the real sources, and the committed file's schema and full record.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/real-targets")
}

fn bin() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_engine-cli"));
    cmd.env("SM_LOG", "warn");
    cmd.env("SM_BAND_HISTORY", "off");
    cmd.env("SM_CONTENT_DIR", repo().join("content"));
    cmd
}

fn run(args: &[&str], extra: &[&Path]) -> Output {
    let mut cmd = bin();
    cmd.args(args);
    for p in extra {
        cmd.arg(p);
    }
    cmd.output().unwrap()
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
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

/// Builds the reference file of `data` into `out/real-targets.json`, with its side folder
/// in `out` too, so no test writes to the checkout's own side folder.
fn build(data: &Path, out: &Path) -> (Value, Output) {
    let file = out.join("real-targets.json");
    let mut cmd = bin();
    cmd.args(["real-targets", "build", "--data"])
        .arg(data)
        .arg("--out")
        .arg(&file)
        .arg("--side-dir")
        .arg(out.join("side"));
    let o = cmd.output().unwrap();
    assert!(o.status.success(), "{}", text(&o));
    let v = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    (v, o)
}

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

const DIVISIONS: [&str; 22] = [
    "E0", "E1", "E2", "E3", "EC", "SC0", "SC1", "SC2", "SC3", "D1", "D2", "I1", "I2", "SP1", "SP2",
    "F1", "F2", "N1", "B1", "P1", "T1", "G1",
];
const SEASONS: [&str; 5] = ["2122", "2223", "2324", "2425", "2526"];
const FOTMOB: [&str; 5] = [
    "premier-league",
    "laliga",
    "bundesliga",
    "serie-a",
    "ligue-1",
];

/// A data folder with every file the check expects, each recorded in its manifest.
fn full_data(dir: &Path) {
    let write = |rel: &str, bytes: &[u8]| {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    };
    let mut fd = Vec::new();
    for s in SEASONS {
        for d in DIVISIONS {
            let body = format!("Div,Date\n{d},01/09/20{}\n", &s[..2]);
            write(&format!("football-data/{s}/{d}.csv"), body.as_bytes());
            fd.push(
                serde_json::json!({"file": format!("{s}/{d}.csv"), "sha256": sha(body.as_bytes())}),
            );
        }
    }
    write(
        "football-data/manifest.json",
        serde_json::json!({ "files": fd }).to_string().as_bytes(),
    );
    let mut fm = Vec::new();
    for l in FOTMOB {
        for s in SEASONS {
            let body = format!("\"team\"\n\"{l} {s}\"\n");
            write(&format!("fotmob/{l}-{s}.csv"), body.as_bytes());
            fm.push(
                serde_json::json!({"file": format!("{l}-{s}.csv"), "sha256": sha(body.as_bytes())}),
            );
        }
    }
    write(
        "fotmob/manifest.json",
        serde_json::json!({ "files": fm }).to_string().as_bytes(),
    );
    let mut st = Vec::new();
    for i in 0..20 {
        let name = format!(
            "1000{:02}_{}.json",
            i / 2,
            if i % 2 == 0 { "1st" } else { "2nd" }
        );
        write(&format!("soccertrack-v2/{name}"), b"{}\n");
        st.push(serde_json::json!({"file": name, "bytes": 3}));
    }
    write(
        "soccertrack-v2/download-status.json",
        serde_json::json!({ "files": st }).to_string().as_bytes(),
    );
    write("clubelo/clubelo-2026-10-11.csv", b"Rank,Club\n");
    // The open data: two StatsBomb seasons, the two neighbour foul files, the scan table.
    write(
        "sb/competitions.json",
        serde_json::json!([
            {"competition_id": 2, "season_id": 27, "competition_name": "P",
             "season_name": "2015/2016", "competition_gender": "male"},
            {"competition_id": 43, "season_id": 3, "competition_name": "W",
             "season_name": "2018", "competition_gender": "male"}
        ])
        .to_string()
        .as_bytes(),
    );
    for rel in [
        "sb/matches/2_27.json",
        "sb/matches/43_3.json",
        "football-data/1617/D1.csv",
        "football-data/1819/D1.csv",
        "scanning-studies/scan-rates.csv",
    ] {
        write(rel, b"x\n");
    }
}

/// A group's line of the check: its name padded as the check prints it.
fn group_line(name: &str, found: usize, expected: usize) -> String {
    format!("{name:<16} {found:>3} of {expected:>3}")
}

#[test]
fn the_data_check_counts_every_group_and_names_each_missing_file() {
    let tmp = TempDir::new().unwrap();
    let data = tmp.path().join("data");
    full_data(&data);
    let o = run(&["real-targets", "check-data", "--data"], &[&data]);
    let out = text(&o);
    assert_eq!(o.status.code(), Some(0), "{out}");
    for line in [
        "football-data    110 of 110",
        "fotmob            25 of  25",
        "soccertrack-v2    20 of  20",
        "clubelo            1 of   1",
        "every file is there",
    ] {
        assert!(out.contains(line), "{line} not in {out}");
    }
    assert!(out.contains(&group_line("open-data", 6, 6)), "{out}");
    // The local side file's source is counted, not required.
    assert!(
        out.contains(&format!(
            "{}  (not required: the IMPECT side file will not be built)",
            group_line("impect", 0, 1)
        )),
        "{out}"
    );

    // One file gone from each group, and one changed.
    for rel in [
        "football-data/2324/T1.csv",
        "fotmob/laliga-2122.csv",
        "soccertrack-v2/100003_2nd.json",
        "clubelo/clubelo-2026-10-11.csv",
        "scanning-studies/scan-rates.csv",
    ] {
        std::fs::remove_file(data.join(rel)).unwrap();
    }
    std::fs::write(data.join("football-data/2526/E0.csv"), "changed\n").unwrap();
    let o = run(&["real-targets", "check-data", "--data"], &[&data]);
    let out = text(&o);
    assert_eq!(o.status.code(), Some(1), "{out}");
    for line in [
        "football-data    109 of 110",
        "fotmob            24 of  25",
        "soccertrack-v2    19 of  20",
        "clubelo            0 of   1",
        "missing: football-data/2324/T1.csv",
        "missing: fotmob/laliga-2122.csv",
        "missing: soccertrack-v2/100003_2nd.json",
        "missing: clubelo/clubelo-<date>.csv",
        "not as recorded: football-data/2526/E0.csv",
        "missing: scanning-studies/scan-rates.csv",
    ] {
        assert!(out.contains(line), "{line} not in {out}");
    }
    assert!(out.contains(&group_line("open-data", 5, 6)), "{out}");
}

/// The matches of one fixture result file: (home, away, home goals, away goals).
fn results(div: &str, season: &str) -> Vec<(String, String, u32, u32)> {
    let path = fixture().join(format!("football-data/{season}/{div}.csv"));
    let text = std::fs::read_to_string(path).unwrap();
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    let head: Vec<&str> = lines.next().unwrap().split(',').collect();
    let col = |n: &str| head.iter().position(|h| *h == n).unwrap();
    lines
        .filter_map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            Some((
                f[col("HomeTeam")].to_string(),
                f[col("AwayTeam")].to_string(),
                f[col("FTHG")].parse().ok()?,
                f[col("FTAG")].parse().ok()?,
            ))
        })
        .collect()
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn the_builder_gives_league_tier_and_group_targets_tables_and_duel_contests() {
    let tmp = TempDir::new().unwrap();
    let (v, o) = build(&fixture(), tmp.path());
    let t = &v["targets"];

    // Each league with data: targets from five seasons, the hand-computed mean.
    for div in ["E0", "E1", "D1", "D2", "SP1"] {
        let g = &t[format!("fd.{div}.goals_per_match")];
        assert_eq!(g["seasons"], serde_json::json!(SEASONS), "{div}");
        let all: Vec<f64> = SEASONS
            .iter()
            .flat_map(|s| results(div, s))
            .map(|(_, _, h, a)| f64::from(h + a))
            .collect();
        let mean = all.iter().sum::<f64>() / all.len() as f64;
        assert!(
            close(g["value"].as_f64().unwrap(), mean, 1e-6),
            "{div}: {g}"
        );
        assert_eq!(g["n"].as_u64().unwrap(), all.len() as u64);
        assert!(g["se"].as_f64().unwrap() > 0.0);
        assert!(g["between_season_sd"].as_f64().is_some());
    }
    // A league whose files fill goals only: its shot target is absent with the reason.
    assert!(t.get("fd.SP1.shots_per_team").is_none());
    assert!(
        v["absent"]["fd.SP1.shots_per_team"]
            .as_str()
            .unwrap()
            .contains("do not fill")
    );

    // Each tier has a target set over its leagues, with equal weight.
    let t1 = &t["tier.1.goals_per_match"];
    assert_eq!(t1["leagues"], serde_json::json!(["D1", "E0", "SP1"]));
    let league_mean = ["D1", "E0", "SP1"]
        .iter()
        .map(|d| {
            t[format!("fd.{d}.goals_per_match")]["value"]
                .as_f64()
                .unwrap()
        })
        .sum::<f64>()
        / 3.0;
    assert!(
        close(t1["value"].as_f64().unwrap(), league_mean, 1e-5),
        "{t1}"
    );
    assert!(t1["between_league_sd"].as_f64().is_some());
    assert_eq!(
        t["tier.2.goals_per_match"]["leagues"],
        serde_json::json!(["D2", "E1"])
    );
    assert!(t.get("tier.2.dist.scorelines").is_some());

    // The leagues without data get banding-group targets from the fitted formula.
    let g = &t["group.t1.e1550.goals_per_match"];
    assert_eq!(g["layer"], "group");
    assert_eq!(g["leagues"], serde_json::json!(["XAA-1"]));
    let f = &g["formula"];
    assert_eq!(
        f["fitted_on"],
        serde_json::json!(["D1", "D2", "E0", "E1", "SP1"])
    );
    let predicted = f["intercept"].as_f64().unwrap()
        + f["tier"].as_f64().unwrap()
        + f["strength"].as_f64().unwrap() * 1575.0;
    assert!(close(g["value"].as_f64().unwrap(), predicted, 1e-4), "{g}");
    assert!(g["se"].as_f64().unwrap() > f["residual_sd"].as_f64().unwrap());
    assert_eq!(g["extrapolated"], false);
    // XBB-2's clubs sit far below every fitted league.
    assert_eq!(t["group.t2.e1200.goals_per_match"]["extrapolated"], true);

    // Tables: the champion's points, by hand from the results.
    let mut champions = Vec::new();
    for s in SEASONS {
        let mut pts: BTreeMap<String, u32> = BTreeMap::new();
        for (h, a, hg, ag) in results("E0", s) {
            let (ph, pa) = match hg.cmp(&ag) {
                std::cmp::Ordering::Greater => (3, 0),
                std::cmp::Ordering::Equal => (1, 1),
                std::cmp::Ordering::Less => (0, 3),
            };
            *pts.entry(h).or_default() += ph;
            *pts.entry(a).or_default() += pa;
        }
        champions.push(f64::from(*pts.values().max().unwrap()));
    }
    let champ = champions.iter().sum::<f64>() / 5.0;
    assert!(close(
        t["fd.E0.table.champion_points"]["value"].as_f64().unwrap(),
        champ,
        1e-6
    ));
    for stat in ["relegation_points", "ppg_sd", "title_margin"] {
        assert!(t.get(format!("fd.E0.table.{stat}")).is_some(), "{stat}");
    }

    // Duels: 3 paired contests and 1 or 2 lone rows per match, so 4 and 5 contests.
    let d = &t["wy.England.duel_contests_per_match"];
    assert!(close(d["value"].as_f64().unwrap(), 4.5, 1e-9), "{d}");
    assert!(text(&o).contains("wyscout England: 3 unpaired duel rows"));

    // The held-out sets are never opened; the four leagues are.
    let read: BTreeSet<&str> = v["built_from"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    for held in ["Germany", "World_Cup", "European_Championship"] {
        assert!(!read.contains(format!("wyscout/events_{held}.json").as_str()));
    }
    for league in ["England", "France", "Italy", "Spain"] {
        assert!(read.contains(format!("wyscout/events_{league}.json").as_str()));
    }

    // Shots: the cross-check found blocked shots counted, so shot targets exist.
    assert_eq!(v["definitions"]["shots_include_blocked"], true);
    assert!(t.get("fd.E0.shots_per_team").is_some());
    assert!(text(&o).contains("answer: the file's shots count blocked shots"));

    // Pass completion by Opta's count: (Σ accurate) / (Σ accurate / share).
    let p = &t["fotmob.E0.pass_completion"];
    assert_eq!(p["provider"], "Opta via FotMob");
    assert!(p["value"].as_f64().unwrap() > 75.0 && p["value"].as_f64().unwrap() < 80.0);

    // Every target carries the full record.
    for (key, target) in t.as_object().unwrap() {
        for field in [
            "provider",
            "event_definition",
            "denominator",
            "unit_of_observation",
            "label",
            "source_label",
        ] {
            assert!(
                target[field].as_str().is_some_and(|s| !s.is_empty()),
                "{key} has no {field}"
            );
        }
        assert!(target["engine_count"]["text"].as_str().is_some(), "{key}");
    }
}

#[test]
fn two_builds_of_the_same_files_give_the_same_bytes() {
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    build(&fixture(), a.path());
    build(&fixture(), b.path());
    for rel in ["real-targets.json", "real-names/football-data.json"] {
        assert_eq!(
            std::fs::read(a.path().join(rel)).unwrap(),
            std::fs::read(b.path().join(rel)).unwrap(),
            "{rel}"
        );
    }
    let names: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(a.path().join("real-names/football-data.json")).unwrap(),
    )
    .unwrap();
    assert!(names.contains(&"Leverkusen".to_string()) && names.len() == 20);
}

#[test]
fn shots_that_leave_out_blocked_shots_refuse_every_shot_target() {
    let tmp = TempDir::new().unwrap();
    let data = tmp.path().join("data");
    copy_dir(&fixture(), &data);
    // StatsBomb now sees as many extra blocked shots as the file counts in all: the file's
    // count equals the total without blocked shots.
    for e in std::fs::read_dir(data.join("sb/events")).unwrap() {
        let path = e.unwrap().path();
        let mut events: Vec<Value> =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let blocked: Vec<Value> = events
            .iter()
            .filter(|e| e["type"]["name"] == "Shot")
            .map(|e| {
                let mut b = e.clone();
                b["shot"]["outcome"]["name"] = "Blocked".into();
                b
            })
            .collect();
        for e in &mut events {
            if e["type"]["name"] == "Shot" {
                e["shot"]["outcome"]["name"] = "Off T".into();
            }
        }
        events.extend(blocked);
        std::fs::write(&path, serde_json::to_string(&events).unwrap()).unwrap();
    }
    let (v, o) = build(&data, &tmp.path().join("out"));
    assert_eq!(v["definitions"]["shots_include_blocked"], false);
    assert!(text(&o).contains("answer: the file's shots do not count blocked shots"));
    for key in [
        "fd.E0.shots_per_team",
        "fd.E0.shots_on_target_per_team",
        "fd.E0.dist.shots",
    ] {
        assert!(v["targets"].get(key).is_none(), "{key}");
        assert!(
            v["absent"][key]
                .as_str()
                .unwrap()
                .contains("shots cross-check"),
            "{key}"
        );
    }
    assert!(v["targets"].get("fd.E0.goals_per_match").is_some());
}

/// The committed reference file validates against its schema, every target carries the
/// full record, and the credits name every provider a target names.
#[test]
fn the_committed_reference_file_carries_every_record_and_credit() {
    let schema: Value = serde_json::from_str(
        &std::fs::read_to_string(repo().join("schemas/real-targets.schema.json")).unwrap(),
    )
    .unwrap();
    let file: Value = serde_json::from_str(
        &std::fs::read_to_string(repo().join("gate/real-targets.json")).unwrap(),
    )
    .unwrap();
    let mut compiler = boon::Compiler::new();
    let mut schemas = boon::Schemas::new();
    let id = schema["$id"].as_str().unwrap().to_string();
    compiler.add_resource(&id, schema).unwrap();
    let index = compiler.compile(&id, &mut schemas).unwrap();
    if let Err(e) = schemas.validate(&file, index) {
        panic!("gate/real-targets.json does not validate: {e:#}");
    }
    let targets = file["targets"].as_object().unwrap();
    assert!(targets.len() > 500, "{} targets", targets.len());
    let credits = file["credits"].as_object().unwrap();
    for (key, t) in targets {
        for field in [
            "provider",
            "event_definition",
            "denominator",
            "unit_of_observation",
        ] {
            assert!(
                t[field].as_str().is_some_and(|s| !s.is_empty()),
                "{key}: {field}"
            );
        }
        assert!(
            matches!(t["label"].as_str(), Some("calibration" | "validation")),
            "{key}"
        );
        assert!(t["engine_count"]["text"].as_str().is_some(), "{key}");
        let provider = t["provider"].as_str().unwrap();
        assert!(
            credits.contains_key(provider),
            "{key}: no credit for {provider}"
        );
    }
    // Every league with data has its five seasons; every tier has a set.
    for div in DIVISIONS {
        let g = &targets[&format!("fd.{div}.goals_per_match")];
        assert_eq!(g["seasons"].as_array().unwrap().len(), 5, "{div}");
    }
    for tier in 1..=5 {
        assert!(
            targets.contains_key(&format!("tier.{tier}.goals_per_match")),
            "{tier}"
        );
    }
    assert!(targets.keys().any(|k| k.starts_with("group.")));
    assert_eq!(file["definitions"]["shots_include_blocked"], true);
    assert!(targets.contains_key("wy.pooled.duel_contests_per_match"));
    assert!(targets.contains_key("fotmob.pooled.pass_completion"));
}

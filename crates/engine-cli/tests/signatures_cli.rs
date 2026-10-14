//! `engine-cli signatures` writes the real-team signature table from the fit data alone: two
//! runs give the same bytes, the table names only the fit competitions, a held-out set asked
//! for by name or reached through a match of the fit files stops the run with a clear error,
//! and the PPDA row of ten teams holds the quintiles and medians worked out by hand.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};

use common::{bin, temp};

/// A Wyscout-shaped real-data folder in `dir`: ten English teams, team `i` with a season PPDA
/// of `i` and one possession of one pass, and a German team the table must skip. With
/// `planted`, an event row claims the German match for England.
fn fixture(dir: &Path, planted: bool) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let mut matches = String::from("comp,match\n");
    let mut team_matches = String::from("match,team,opp_build_vs,def_act\n");
    let mut events =
        String::from("comp,match,id,period,sec,team,player,role,ev,sub,x0,y0,x1,y1,tags\n");
    for i in 1..=10 {
        matches.push_str(&format!("England,m{i}\n"));
        team_matches.push_str(&format!("m{i},{i},{},10\n", 10 * i));
        events.push_str(&format!(
            "England,m{i},{i},1H,1.0,{i},1,MD,Pass,Simple pass,30,50,40,50,1801\n"
        ));
        // The in-possession rows: a keeper's 42 m pass from his own third, a cross from the
        // left wing into the final third (high for an even team), a take-on, a shot that
        // starts further up for a higher team, and a goal kick (35 + i) % of the length long.
        let high = if i % 2 == 0 { "801" } else { "802" };
        events.push_str(&format!(
            "England,m{i},{},1H,1.2,{i},2,GK,Pass,Simple pass,5,50,45,50,1801\n",
            200 + i
        ));
        events.push_str(&format!(
            "England,m{i},{},1H,1.5,{i},1,MD,Pass,Cross,60,10,95,50,{high} 1801\n",
            300 + i
        ));
        events.push_str(&format!(
            "England,m{i},{},1H,2.0,{i},1,FW,Duel,Ground attacking duel,70,30,72,30,503 1801\n",
            400 + i
        ));
        events.push_str(&format!(
            "England,m{i},{},1H,2.5,{i},1,FW,Shot,Shot,{},50,100,50,\n",
            500 + i,
            75 + 2 * i
        ));
        events.push_str(&format!(
            "England,m{i},{},1H,3.0,{i},2,GK,Free Kick,Goal kick,5,50,{},50,\n",
            600 + i,
            40 + i
        ));
        events.push_str(&format!(
            "England,m{i},{},1H,4.0,{i},1,MD,Interruption,Ball out of the field,40,50,,,\n",
            100 + i
        ));
    }
    matches.push_str("Germany,g1\n");
    team_matches.push_str("g1,99,500,1\n");
    events.push_str("Germany,g1,900,1H,1.0,99,1,MD,Pass,Simple pass,30,50,40,50,1801\n");
    if planted {
        events.push_str("England,g1,901,1H,2.0,99,1,MD,Pass,Simple pass,30,50,40,50,1801\n");
    }
    std::fs::write(dir.join("wy2_matches.csv"), matches).unwrap();
    std::fs::write(dir.join("wy2_team_matches.csv"), team_matches).unwrap();
    let file = std::fs::File::create(dir.join("wy2_events.csv.gz")).unwrap();
    let mut gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    gz.write_all(events.as_bytes()).unwrap();
    gz.finish().unwrap();
    dir.to_path_buf()
}

/// Runs the command on `real` with `extra` arguments; the table text on success, the error
/// text on failure.
fn run(data: &Path, real: &Path, out: &str, extra: &[&str]) -> Result<String, String> {
    let out = data.join(out);
    let run = bin(data)
        .arg("signatures")
        .arg("--real-data")
        .arg(real)
        .arg("--out")
        .arg(&out)
        .args(extra)
        .output()
        .unwrap();
    if run.status.success() {
        Ok(std::fs::read_to_string(out).unwrap())
    } else {
        Err(String::from_utf8_lossy(&run.stderr).into_owned())
    }
}

#[test]
fn two_runs_write_the_same_bytes_and_name_only_the_fit_competitions() {
    let data = temp("engine-cli-signatures", "bytes");
    let real = fixture(&data.join("real"), false);
    let first = run(&data, &real, "first.json", &[]).unwrap();
    let second = run(&data, &real, "second.json", &[]).unwrap();
    assert_eq!(first, second, "two runs differ");
    let table: serde_json::Value = serde_json::from_str(&first).unwrap();
    for row in table["rows"].as_array().unwrap() {
        assert_eq!(
            row["competitions"],
            serde_json::json!(["England", "France", "Italy", "Spain"])
        );
        assert_eq!(row["teams"], 10, "the German team is skipped: {row}");
        assert_eq!(row["matches"], 10);
    }
    assert!(!first.contains("\"Germany\""), "{first}");
    std::fs::remove_dir_all(&data).unwrap();
}

#[test]
fn a_held_out_set_is_refused_by_name_and_through_a_fit_file() {
    let data = temp("engine-cli-signatures", "held-out");
    let real = fixture(&data.join("real"), false);
    let asked = run(
        &data,
        &real,
        "asked.json",
        &["--competitions", "England,Germany"],
    )
    .expect_err("a held-out competition asked for by name");
    assert!(
        asked.contains("held-out set: Germany is never read"),
        "{asked}"
    );
    let planted = fixture(&data.join("planted"), true);
    let reached = run(&data, &planted, "reached.json", &[])
        .expect_err("an event of a German match filed under England");
    assert!(
        reached.contains("held-out set: Germany is never read"),
        "{reached}"
    );
    assert!(!data.join("asked.json").exists() && !data.join("reached.json").exists());
    std::fs::remove_dir_all(&data).unwrap();
}

/// PPDA 1 to 10 across ten teams: the edges at 20, 40, 60 and 80 percent by linear
/// interpolation, and each quintile's median of its two teams, the fit points of the five
/// pressing levels.
#[test]
fn the_ppda_row_holds_the_quintiles_of_the_fit_teams() {
    let data = temp("engine-cli-signatures", "ppda");
    let real = fixture(&data.join("real"), false);
    let table: serde_json::Value =
        serde_json::from_str(&run(&data, &real, "table.json", &[]).unwrap()).unwrap();
    let row = table["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["stat"] == "ppda")
        .expect("a PPDA row");
    assert_eq!(row["primitive"], "pressing");
    assert_eq!(
        row["quintile_edges"],
        serde_json::json!([2.8, 4.6, 6.4, 8.2])
    );
    assert_eq!(row["medians"], serde_json::json!([1.5, 3.5, 5.5, 7.5, 9.5]));
    assert_eq!(row["min"], 1.0);
    assert_eq!(row["max"], 10.0);
    std::fs::remove_dir_all(&data).unwrap();
}

/// The row of `stat` in `table`.
fn row<'a>(table: &'a serde_json::Value, stat: &str) -> &'a serde_json::Value {
    table["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["stat"] == stat)
        .unwrap_or_else(|| panic!("a {stat} row"))
}

/// The in-possession rows of the ten fixture teams, worked out by hand from the events.
#[test]
fn the_in_possession_rows_follow_the_shared_definitions() {
    let data = temp("engine-cli-signatures", "possession");
    let real = fixture(&data.join("real"), false);
    let table: serde_json::Value =
        serde_json::from_str(&run(&data, &real, "table.json", &[]).unwrap()).unwrap();
    let flat = |stat: &str, v: f64| {
        let r = row(&table, stat);
        assert_eq!(
            (r["min"].as_f64(), r["max"].as_f64()),
            (Some(v), Some(v)),
            "{stat}: {r}"
        );
    };
    flat("keeper_long_share", 1.0);
    flat("takeons_per_90", 1.0);
    flat("entries_centre_share", 1.0);
    flat("crosses_per_final_third_possession", 1.0);
    let r = row(&table, "goal_kick_m");
    assert!(
        (r["min"].as_f64().unwrap() - 36.0 * 1.05).abs() < 1e-6,
        "{r}"
    );
    assert!(
        (r["max"].as_f64().unwrap() - 45.0 * 1.05).abs() < 1e-6,
        "{r}"
    );
    let r = row(&table, "outside_box_shot_share");
    assert_eq!(
        (r["min"].as_f64(), r["max"].as_f64()),
        (Some(0.0), Some(1.0)),
        "{r}"
    );
    let r = row(&table, "high_cross_share");
    assert_eq!(
        (r["min"].as_f64(), r["max"].as_f64()),
        (Some(0.0), Some(1.0)),
        "{r}"
    );
    std::fs::remove_dir_all(&data).unwrap();
}

/// Writes `text` gzipped to `path`.
fn gzip(path: &std::path::Path, text: &str) {
    let file = std::fs::File::create(path).unwrap();
    let mut gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    gz.write_all(text.as_bytes()).unwrap();
    gz.finish().unwrap();
}

/// The runs of three teams: five targets spread evenly between the lowest and the highest
/// team, marked thin-sample. The full-backs' runs come from SkillCorner's run events; the
/// forwards' runs come from tracking data, classified by the engine's own run kinds: the World
/// Cup teams give the rows (marked as national teams) and a club source gives a check.
#[test]
fn the_runs_rows_are_thin_sample_and_spread_evenly() {
    let data = temp("engine-cli-signatures", "runs");
    let real = fixture(&data.join("real"), false);
    let mut sc = String::from("match_id,team_shortname,event_type,event_subtype,player_position\n");
    let mut add = |m: &str, team: &str, kind: &str, sub: &str, pos: &str, n: usize| {
        for _ in 0..n {
            sc.push_str(&format!("{m},{team},{kind},{sub},{pos}\n"));
        }
    };
    // Full-back runs a match: A 2, B 1, C 3.
    add("m1", "A", "off_ball_run", "overlap", "LB", 2);
    add("m1", "B", "off_ball_run", "underlap", "RWB", 1);
    add("m2", "C", "off_ball_run", "overlap", "RB", 3);
    gzip(&real.join("sc_dynamic_events.csv.gz"), &sc);

    let mut runs = String::from(
        "source,competition,national,match,team,player,position,period,t0,t1,depth0,depth1,\
         line0,line1,ball0,ball1,balld0,balld1\n",
    );
    let mut run = |source: &str, national: u8, team: &str, kind: &str, n: usize| {
        // depth0, depth1, line, ball depth, ball distance at the start and at the end
        let facts = match kind {
            "behind" => "10,20,20,20,0,0,10,20",
            "short" => "20,14,25,25,0,0,20,14",
            _ => "5,8,30,30,0,0,5,8",
        };
        let competition = if national == 1 { "World Cup" } else { "League" };
        for k in 0..n {
            runs.push_str(&format!(
                "{source},{competition},{national},m1,{team},p{k},CF,1,0,1,{facts}\n"
            ));
        }
    };
    // X: 4 runs, 1 in behind, 1 coming short. Y: 2, both in behind. Z: 5, 1 coming short.
    run("pff_wc2022", 1, "X", "behind", 1);
    run("pff_wc2022", 1, "X", "short", 1);
    run("pff_wc2022", 1, "X", "other", 2);
    run("pff_wc2022", 1, "Y", "behind", 2);
    run("pff_wc2022", 1, "Z", "short", 1);
    run("pff_wc2022", 1, "Z", "other", 4);
    // A club team of another source: 2 runs, 1 in behind.
    run("skillcorner_open", 0, "W", "behind", 1);
    run("skillcorner_open", 0, "W", "other", 1);
    gzip(&real.join("tracking_runs.csv.gz"), &runs);

    let table: serde_json::Value =
        serde_json::from_str(&self::run(&data, &real, "table.json", &[]).unwrap()).unwrap();
    let behind = row(&table, "runs_in_behind_share");
    assert_eq!(behind["thin_sample"], true, "{behind}");
    assert_eq!(behind["national_teams"], true, "{behind}");
    assert_eq!(behind["source"], "pff_wc2022");
    assert_eq!(behind["teams"], 3);
    assert_eq!(
        behind["medians"],
        serde_json::json!([0.0, 0.25, 0.5, 0.75, 1.0])
    );
    let short = row(&table, "coming_short_share");
    assert_eq!(
        short["medians"],
        serde_json::json!([0.0, 0.0625, 0.125, 0.1875, 0.25])
    );
    let fullback = row(&table, "fullback_runs_per_match");
    assert_eq!(fullback["source"], "skillcorner_open");
    assert_eq!(
        fullback["medians"],
        serde_json::json!([1.0, 1.5, 2.0, 2.5, 3.0])
    );
    assert!(fullback.get("national_teams").is_none(), "{fullback}");
    let check = table["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["stat"] == "runs_in_behind_share" && c["source"] == "skillcorner_open")
        .expect("a club check of the runs in behind");
    assert_eq!(check["median"], 0.5);
    assert!(row(&table, "ppda").get("thin_sample").is_none());
    std::fs::remove_dir_all(&data).unwrap();
}

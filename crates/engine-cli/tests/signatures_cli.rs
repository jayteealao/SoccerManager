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

//! Real tracked matches: the converter on the invented DFL fixture match (ticks, slots,
//! off-pitch spells, play direction, ball status and possession, restart bits), the refusal
//! of a match whose direction does not change at half time, and event placement.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use engine::pitch::Pitch;
use engine::record::{TickRecord, read_ticks};
use serde_json::Value;
use tempfile::TempDir;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracking")
}

fn bin() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_engine-cli"));
    cmd.env("SM_LOG", "warn");
    cmd.env("SM_CONTENT_DIR", repo().join("content"));
    cmd
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

const FIX: &str = "DFL-MAT-FIX001";

/// Converts the fixture match `id` of `folder` into `out`.
fn convert(folder: &str, id: &str, out: &Path) -> Output {
    bin()
        .args(["tracking", "convert", "--match", id, "--data"])
        .arg(fixtures().join(folder))
        .arg("--out")
        .arg(out)
        .output()
        .unwrap()
}

/// The fixture match converted: its records and its side file.
fn converted() -> (TempDir, Vec<TickRecord>, Value) {
    let dir = TempDir::new().unwrap();
    let out = convert("dfl", FIX, dir.path());
    assert!(out.status.success(), "{}", text(&out));
    let ticks = read_ticks(&dir.path().join(format!("{FIX}.ticks"))).unwrap();
    let side: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join(format!("{FIX}.side.json"))).unwrap(),
    )
    .unwrap();
    (dir, ticks.records, side)
}

fn record(records: &[TickRecord], tick: u32) -> &TickRecord {
    records
        .iter()
        .find(|r| r.tick == tick)
        .unwrap_or_else(|| panic!("no record at tick {tick}"))
}

fn near(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4
}

fn spot(team: usize, slot: usize) -> [f32; 2] {
    let p = Pitch::new(104.0, 67.0).unwrap().parking_spot(team, slot);
    [p.x as f32, p.y as f32]
}

/// Record place of a slot: team 0's slots first.
fn place(team: usize, slot: usize) -> usize {
    team * 11 + slot
}

#[test]
fn tick_two_k_holds_frame_k_from_each_halfs_first_ball_frame_and_odd_ticks_are_absent() {
    let (dir, records, side) = converted();
    // Half 1 uses frames 10002 to 10059 (its ball set starts two frames after the
    // players'), half 2 frames 100000 to 100059.
    let ticks: Vec<u32> = records.iter().map(|r| r.tick).collect();
    let mut want: Vec<u32> = (0..58).map(|k| 2 * k).collect();
    want.extend((0..60).map(|k| 116 + 2 * k));
    assert_eq!(
        ticks, want,
        "half 2's first record is half 1's last tick plus 2"
    );
    assert!(ticks.iter().all(|t| t % 2 == 0), "no record on an odd tick");
    assert_eq!(side["odd_ticks"], "absent");
    assert_eq!(
        (side["frame_rate"].as_u64(), side["tick_step"].as_u64()),
        (Some(25), Some(2))
    );
    let halves = side["halves"].as_array().unwrap();
    assert_eq!(halves[0]["first_frame"], 10002);
    assert_eq!(
        (
            halves[0]["start_tick"].as_u64(),
            halves[0]["end_tick"].as_u64()
        ),
        (Some(0), Some(114))
    );
    assert_eq!(
        (
            halves[1]["start_tick"].as_u64(),
            halves[1]["end_tick"].as_u64()
        ),
        (Some(116), Some(234))
    );
    // Tick 20 holds frame 10012: the centre-back H03 (slot 2) at frame index 12 of his
    // set, turned by 180 degrees.
    assert!(near(
        record(&records, 20).players[place(0, 2)],
        [-35.0, -7.52]
    ));
    // Tick 116 holds frame 100000.
    assert!(near(
        record(&records, 116).players[place(0, 2)],
        [35.0, 8.0]
    ));
    // The file is the engine's own format, header schema 4.
    let bytes = std::fs::read(dir.path().join(format!("{FIX}.ticks"))).unwrap();
    assert_eq!(&bytes[0..4], b"SMTK");
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 4);
}

#[test]
fn team_zero_attacks_one_way_in_half_one_and_the_other_in_half_two() {
    let (_dir, records, side) = converted();
    // The home keeper defended the positive end in half 1 of the raw data: the match is
    // turned so team 0 attacks towards positive x first, as in the engine.
    assert_eq!(side["rotated"], true);
    let halves = side["halves"].as_array().unwrap();
    assert_eq!(halves[0]["team0_attack"], 1.0);
    assert_eq!(halves[1]["team0_attack"], -1.0);
    let keeper = place(0, 0);
    assert!(
        records
            .iter()
            .filter(|r| r.tick <= 114)
            .all(|r| r.players[keeper][0] < 0.0)
    );
    assert!(
        records
            .iter()
            .filter(|r| r.tick >= 116)
            .all(|r| r.players[keeper][0] > 0.0)
    );
    let away_keeper = place(1, 0);
    assert!(
        records
            .iter()
            .filter(|r| r.tick <= 114)
            .all(|r| r.players[away_keeper][0] > 0.0)
    );
}

#[test]
fn a_match_whose_direction_does_not_change_at_half_time_is_refused() {
    let dir = TempDir::new().unwrap();
    let out = convert("dfl-no-flip", "DFL-MAT-FIX002", dir.path());
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(
        text(&out).contains("does not change at half time"),
        "{}",
        text(&out)
    );
    assert!(!dir.path().join("DFL-MAT-FIX002.ticks").exists());
}

#[test]
fn the_substitute_holds_the_replaced_players_slot_from_the_substitution_tick() {
    let (_dir, records, side) = converted();
    // H09 (slot 8) goes off at frame 100020, tick 156; H12 comes on, tracked from 100024.
    let spells: Vec<(String, u64, u64)> = side["slots"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["team"] == 0 && s["slot"] == 8)
        .map(|s| {
            (
                s["person"].as_str().unwrap().to_string(),
                s["from_tick"].as_u64().unwrap(),
                s["to_tick"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        spells,
        vec![
            ("DFL-OBJ-FIXH09".to_string(), 0, 154),
            ("DFL-OBJ-FIXH12".to_string(), 156, 234),
        ]
    );
    let slot = place(0, 8);
    // From the substitution until his first tracked frame he is not on the pitch.
    assert!(near(record(&records, 156).players[slot], spot(0, 8)));
    assert!(near(record(&records, 162).players[slot], spot(0, 8)));
    assert!(near(record(&records, 164).players[slot], [-10.0, 24.04]));
    let group = side["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["person"] == "DFL-OBJ-FIXH12")
        .map(|s| s["group"].clone());
    assert_eq!(group, Some("w".into()));
}

#[test]
fn the_sent_off_player_and_the_gap_stand_on_their_parking_spots() {
    let (_dir, records, side) = converted();
    let spells: Vec<(u64, u64, String, u64, u64, String)> = side["off_pitch"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["team"].as_u64().unwrap(),
                s["slot"].as_u64().unwrap(),
                s["person"].as_str().unwrap_or("").to_string(),
                s["from_tick"].as_u64().unwrap(),
                s["to_tick"].as_u64().unwrap(),
                s["reason"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let has = |team, slot, person: &str, from, to, reason: &str| {
        spells.contains(&(team, slot, person.into(), from, to, reason.into()))
    };
    // H07 (slot 6) is not tracked in frames 10020 to 10024: ticks 36 to 44.
    assert!(
        has(0, 6, "DFL-OBJ-FIXH07", 36, 44, "not tracked"),
        "{spells:?}"
    );
    // A04 (away slot 3) is sent off at frame 100040, tick 196, though still tracked
    // walking off.
    assert!(
        has(1, 3, "DFL-OBJ-FIXA04", 196, 234, "sent off"),
        "{spells:?}"
    );
    assert!(
        has(0, 8, "DFL-OBJ-FIXH12", 156, 162, "not tracked"),
        "{spells:?}"
    );
    assert_eq!(spells.len(), 3, "{spells:?}");
    for t in [36, 40, 44] {
        assert!(near(record(&records, t).players[place(0, 6)], spot(0, 6)));
    }
    assert!(!near(record(&records, 46).players[place(0, 6)], spot(0, 6)));
    for t in [196, 200, 234] {
        assert!(near(record(&records, t).players[place(1, 3)], spot(1, 3)));
    }
    assert!(!near(
        record(&records, 194).players[place(1, 3)],
        spot(1, 3)
    ));
    // The spots are those of the stadium's pitch, 104 by 67 m, beside the touchline.
    assert!((spot(0, 6)[1] + 33.5 + 3.0).abs() < 1e-4);
}

#[test]
fn the_side_file_holds_the_fixtures_ball_status_and_possession_and_restarts_set_their_bit() {
    let (_dir, records, side) = converted();
    let runs: Vec<(u64, u64, Value, Value)> = side["ball"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["from_tick"].as_u64().unwrap(),
                r["to_tick"].as_u64().unwrap(),
                r["live"].clone(),
                r["possession"].clone(),
            )
        })
        .collect();
    let run = |from, to, live: bool, possession: u64| (from, to, live.into(), possession.into());
    assert_eq!(
        runs,
        vec![
            run(0, 34, true, 0),
            run(36, 84, true, 1),
            run(86, 100, false, 1),
            run(102, 114, true, 1),
            run(116, 174, true, 0),
            run(176, 186, false, 0),
            run(188, 216, true, 0),
            run(218, 230, false, 0),
            run(232, 234, true, 1),
        ]
    );
    let restarts: Vec<u32> = records
        .iter()
        .filter(|r| r.restart)
        .map(|r| r.tick)
        .collect();
    // Kick-off, throw-in, kick-off, free kick, kick-off after the goal.
    assert_eq!(restarts, vec![0, 102, 116, 188, 232]);
}

/// Each event goes to the frame nearest its event time, and the side file names how; the
/// fixture's passes give the planted clock offset of 0.48 s; its one event with neither an
/// event time nor a game clock is counted, not placed.
#[test]
fn every_timed_event_is_placed_on_its_nearest_frame_and_the_offset_is_fitted() {
    let (_dir, _records, side) = converted();
    let events = side["events"].as_array().unwrap();
    assert!(events.iter().all(|e| e["placed"] == "event-time"));
    let at = |kind: &str| -> Vec<u64> {
        events
            .iter()
            .filter(|e| e["kind"] == kind)
            .map(|e| e["tick"].as_u64().unwrap())
            .collect()
    };
    // The kick-offs at frames 10002, 100000 and 100058; the pass at 10020 (tick 36); the
    // goal at 100050; the throw-in at 10053.
    assert_eq!(at("kick_off"), vec![0, 116, 232]);
    assert_eq!(at("pass"), vec![36, 132]);
    assert_eq!(at("goal"), vec![216]);
    assert_eq!(at("throw_in"), vec![102]);
    assert_eq!(at("sending_off"), vec![196]);
    assert_eq!(side["unplaced"], 1);
    assert_eq!(side["clock_offset"]["ms"], 480.0);
    assert_eq!(side["clock_offset"]["passes"], 2);
}

/// `tracking measure` converts and measures every match of the folder into one tracking
/// file: every figure of the shared report home and away, a frame per record, the
/// ball-in-play minutes of each half and the converter's checks; the line it prints names
/// the frames.
#[test]
fn measure_writes_every_figure_of_every_match() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("matches.json");
    let out = bin()
        .args(["tracking", "measure", "--data"])
        .arg(fixtures().join("dfl"))
        .arg("--out")
        .arg(&file)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains(&format!("{FIX}: 118 frames")),
        "{}",
        text(&out)
    );
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(v["schema"], 1);
    assert_eq!(v["source"], "DFL");
    let m = &v["matches"][0];
    assert_eq!(m["match_id"], FIX);
    assert_eq!(m["frames"], 118);
    assert_eq!(m["live_minutes"].as_array().unwrap().len(), 2);
    assert_eq!(m["checks"]["off_pitch_spells"], 3);
    let figures = m["figures"].as_object().unwrap();
    assert_eq!(figures.len(), 173);
    assert_eq!(figures["team.frames"], serde_json::json!([118.0, 118.0]));
    // A folder without a DFL match is refused.
    let empty = TempDir::new().unwrap();
    let out = bin()
        .args(["tracking", "measure", "--data"])
        .arg(empty.path())
        .arg("--out")
        .arg(dir.path().join("none.json"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
}

/// While the committed live-time check does not pass, no band with a Wyscout real value
/// takes it per live minute: no such band's measure reads a ball-in-play field, and no
/// Wyscout target it names counts per live minute.
#[test]
fn wyscout_rates_stay_per_match_while_the_live_time_rule_fails() {
    let read = |p: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(repo().join(p)).unwrap()).unwrap()
    };
    let check = read("gate/tracking/live-time-check.json");
    assert_eq!(check["matches"].as_array().unwrap().len(), 7);
    let pass = check["verdict"] == "pass";
    assert_eq!(
        check["wyscout_rates"],
        if pass { "per live minute" } else { "per match" }
    );
    if pass {
        return;
    }
    let targets = read("gate/real-targets.json");
    let registry = read("content/realism-bands.json");
    let live =
        |s: &str| s.contains("ball_in_play") || s.contains("live_min") || s.contains("live minute");
    let mut wyscout = 0;
    for band in registry["bands"].as_array().unwrap() {
        let source = &band["real"]["source"];
        let from_target = source["target"].as_str().map(|k| &targets["targets"][k]);
        let is_wyscout = source["dataset"]
            .as_str()
            .is_some_and(|d| d.contains("Wyscout"))
            || from_target.is_some_and(|t| t["provider"] == "Wyscout");
        if !is_wyscout {
            continue;
        }
        wyscout += 1;
        let name = band["band"].as_str().unwrap();
        assert!(
            !live(&band["measure"].to_string()),
            "{name} reads live time"
        );
        assert!(
            !live(&band["estimator"].to_string()),
            "{name} is per live minute"
        );
        if let Some(t) = from_target {
            assert!(!live(&t["denominator"].to_string()), "{name}'s target");
        }
    }
    assert!(wyscout > 0, "the registry has Wyscout bands");
}

/// `tracking live-check` prints a line per match and writes the check with its verdict.
#[test]
fn live_check_writes_the_rule_against_the_tracked_minutes() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("check.json");
    let out = bin()
        .args(["tracking", "live-check", "--data"])
        .arg(fixtures().join("dfl"))
        .arg("--out")
        .arg(&file)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains(FIX), "{}", text(&out));
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let m = &v["matches"][0];
    let (rule, tracked) = (
        m["rule_minutes"].as_f64().unwrap(),
        m["tracked_minutes"].as_f64().unwrap(),
    );
    assert!((m["difference"].as_f64().unwrap() - (rule - tracked)).abs() < 2e-3);
    let within = (rule - tracked).abs() <= 2.0;
    assert_eq!(v["verdict"], if within { "pass" } else { "fail" });
}

/// `tracking contrast` prints the amateur and elite table and writes the contrast; an
/// output under `gate/` is refused.
#[test]
fn contrast_writes_amateur_and_elite_and_never_under_gate() {
    let dir = TempDir::new().unwrap();
    let tracking = dir.path().join("matches.json");
    let out = bin()
        .args(["tracking", "measure", "--data"])
        .arg(fixtures().join("dfl"))
        .arg("--out")
        .arg(&tracking)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let file = dir.path().join("contrast.json");
    let run = |out: &Path| {
        bin()
            .args(["tracking", "contrast", "--soccertrack"])
            .arg(fixtures().join("soccertrack"))
            .arg("--tracking")
            .arg(&tracking)
            .arg("--out")
            .arg(out)
            .output()
            .unwrap()
    };
    let o = run(&file);
    assert!(o.status.success(), "{}", text(&o));
    assert!(
        text(&o).contains("amateur") && text(&o).contains("elite"),
        "{}",
        text(&o)
    );
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(v["source"], "SoccerTrack");
    assert_eq!(v["amateur"]["mean"].as_array().unwrap().len(), 2);
    let gate = repo().join("gate/tracking/contrast-refused.json");
    let o = run(&gate);
    assert_eq!(o.status.code(), Some(1), "{}", text(&o));
    assert!(text(&o).contains("never writes under"), "{}", text(&o));
    assert!(!gate.exists());
}

const PFF: &str = "9001";

/// The PFF fixture match `id` of `data` converted into `out`.
fn convert_pff(data: &Path, id: &str, out: &Path) -> Output {
    bin()
        .args([
            "tracking", "convert", "--source", "pff", "--match", id, "--data",
        ])
        .arg(data)
        .arg("--out")
        .arg(out)
        .output()
        .unwrap()
}

/// The PFF fixture match converted: its records and its side file.
fn converted_pff() -> (TempDir, Vec<TickRecord>, Value) {
    let dir = TempDir::new().unwrap();
    let out = convert_pff(&fixtures().join("pff"), PFF, dir.path());
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("182 frame lines read"),
        "{}",
        text(&out)
    );
    let ticks = read_ticks(&dir.path().join(format!("{PFF}.ticks"))).unwrap();
    let side: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join(format!("{PFF}.side.json"))).unwrap(),
    )
    .unwrap();
    (dir, ticks.records, side)
}

/// Half 1 of the PFF fixture starts at its first ball frame, 100,066.733 ms of video time.
const PFF_T0: f64 = 100_066.733;

/// The PFF fixture's native frame times as its file writes them, half 1 from 100 s.
fn pff_frame_ms(k: u32) -> f64 {
    ((100_000.0 + f64::from(k) * 1000.0 / 29.97) * 1000.0).round() / 1000.0
}

/// The tick nearest native frame `k` of half 1 of the PFF fixture, the earlier on a tie.
fn pff_nearest_tick(k: u32) -> u64 {
    let ms = pff_frame_ms(k) - PFF_T0;
    let below = (ms / 20.0).floor();
    let pick = if ms - 20.0 * below <= 20.0 * (below + 1.0) - ms {
        below
    } else {
        below + 1.0
    };
    pick as u64
}

/// Every tick of each half holds a frame resampled to 50 a second from the half's first
/// ball frame: no tick is missing; the player running at 5 m/s sits on his line at each
/// tick; the file is the engine's own format.
#[test]
fn a_pff_match_fills_every_tick_with_a_frame_resampled_to_fifty_a_second() {
    let (dir, records, side) = converted_pff();
    let ticks: Vec<u32> = records.iter().map(|r| r.tick).collect();
    assert_eq!(ticks, (0..=294).collect::<Vec<u32>>());
    let halves = side["halves"].as_array().unwrap();
    assert_eq!(
        (
            halves[0]["start_tick"].as_u64(),
            halves[0]["end_tick"].as_u64(),
            halves[1]["start_tick"].as_u64(),
            halves[1]["end_tick"].as_u64()
        ),
        (Some(0), Some(145), Some(146), Some(294))
    );
    assert_eq!(halves[0]["first_frame"], 3002);
    // Home shirt 9 (slot 8) runs at 5 m/s along x in half 1.
    for r in records.iter().filter(|r| r.tick <= 145) {
        let t = PFF_T0 + 20.0 * f64::from(r.tick);
        let want = -12.0 + 5.0 * (t - 100_000.0) / 1000.0;
        let got = r.players[place(0, 8)];
        assert!(
            (f64::from(got[0]) - want).abs() < 1e-4 && (got[1] - 5.0).abs() < 1e-4,
            "tick {}: {got:?}, want x {want}",
            r.tick
        );
    }
    let bytes = std::fs::read(dir.path().join(format!("{PFF}.ticks"))).unwrap();
    assert_eq!(&bytes[0..4], b"SMTK");
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 4);
}

/// The home team starts on the negative-x end, as the match information says, so the match
/// is not turned; it attacks the other way after half time.
#[test]
fn a_pff_match_flips_its_play_direction_at_half_time() {
    let (_dir, records, side) = converted_pff();
    assert_eq!(side["rotated"], false);
    let halves = side["halves"].as_array().unwrap();
    assert_eq!(halves[0]["team0_attack"], 1.0);
    assert_eq!(halves[1]["team0_attack"], -1.0);
    let keeper = place(0, 0);
    assert!(
        records
            .iter()
            .filter(|r| r.tick <= 145)
            .all(|r| r.players[keeper][0] < 0.0)
    );
    assert!(
        records
            .iter()
            .filter(|r| r.tick >= 146)
            .all(|r| r.players[keeper][0] > 0.0)
    );
}

/// A match whose match information names the other starting end than its keepers show is
/// refused, naming the match.
#[test]
fn a_pff_match_whose_starting_end_disagrees_with_its_keepers_is_refused() {
    let data = TempDir::new().unwrap();
    let from = fixtures().join("pff/pff-wc2022");
    let to = data.path().join("pff-wc2022");
    for (dir, name) in [
        ("Metadata", "9001.json"),
        ("Rosters", "9001.json"),
        ("Event Data", "9001.json"),
        ("Tracking Data", "9001.jsonl"),
    ] {
        std::fs::create_dir_all(to.join(dir)).unwrap();
        let mut text = std::fs::read_to_string(from.join(dir).join(name)).unwrap();
        if dir == "Metadata" {
            let mut v: Value = serde_json::from_str(&text).unwrap();
            v[0]["homeTeamStartLeft"] = false.into();
            v[0]["id"] = "9003".into();
            text = v.to_string();
        }
        std::fs::write(to.join(dir).join(name.replace("9001", "9003")), text).unwrap();
    }
    let out_dir = TempDir::new().unwrap();
    let out = convert_pff(data.path(), "9003", out_dir.path());
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(
        text(&out).contains("match 9003") && text(&out).contains("starts on the positive-x end"),
        "{}",
        text(&out)
    );
    assert!(!out_dir.path().join("9003.ticks").exists());
}

/// The side file holds the ball status from the events, dead from the OUT's tick to the
/// throw-in's, and the possession runs; it names the resampling and what was left out.
#[test]
fn a_pff_side_file_holds_ball_status_possession_and_the_resampling() {
    let (_dir, _records, side) = converted_pff();
    assert_eq!(side["odd_ticks"], "resampled");
    assert_eq!(
        (side["frame_rate"].as_u64(), side["tick_step"].as_u64()),
        (Some(25), Some(2))
    );
    assert_eq!(side["resampled_from"], 29.97);
    assert_eq!(side["ball_status_from"], "events");
    assert_eq!(side["estimated_share"], 3.0 / 3914.0);
    assert_eq!(
        side["left_out"],
        serde_json::json!([{"period": "0", "frames": 1}, {"period": "3", "frames": 1}])
    );
    assert!(side["credit"].as_str().unwrap().starts_with("PFF FC"));
    let events = side["events"].as_array().unwrap();
    let at = |kind: &str| -> Vec<u64> {
        events
            .iter()
            .filter(|e| e["kind"] == kind)
            .map(|e| e["tick"].as_u64().unwrap())
            .collect()
    };
    // The OUT at native frame 40 and the throw-in at frame 55, each on its nearest tick.
    let (out, throw_in) = (at("out")[0], at("throw_in")[0]);
    assert_eq!(
        (out, throw_in),
        (pff_nearest_tick(40), pff_nearest_tick(55))
    );
    let runs = side["ball"].as_array().unwrap();
    let run_at = |t: u64| {
        runs.iter()
            .find(|r| r["from_tick"].as_u64().unwrap() <= t && t <= r["to_tick"].as_u64().unwrap())
            .unwrap()
    };
    let dead = run_at(out);
    assert_eq!(dead["live"], false);
    assert_eq!(dead["from_tick"].as_u64(), Some(out));
    assert_eq!(dead["to_tick"].as_u64(), Some(throw_in - 1));
    assert_eq!(run_at(throw_in)["live"], true);
    assert_eq!(run_at(out - 1)["live"], true);
    // Possession: home from the kick-off, away from the away pass, home from the throw-in.
    assert_eq!(run_at(0)["possession"], 0);
    assert_eq!(run_at(out)["possession"], 1);
    assert_eq!(run_at(throw_in)["possession"], 0);
    assert_eq!(at("sending_off").len(), 1);
    assert_eq!(at("kick_off"), vec![0, 146]);
}

/// Every restart of the converted PFF fixture counts once in the report, the half
/// openings left out, one of them on an odd tick between two report frames.
#[test]
fn every_pff_restart_counts_once_also_between_two_report_frames() {
    let (_dir, _records, side) = converted_pff();
    let starts: Vec<u64> = side["halves"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["start_tick"].as_u64().unwrap())
        .collect();
    let words = [
        "throw_in",
        "free_kick",
        "goal_kick",
        "corner",
        "kick_off",
        "penalty",
    ];
    let restarts: Vec<u64> = side["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| words.contains(&e["kind"].as_str().unwrap()) && !e["team"].is_null())
        .map(|e| e["tick"].as_u64().unwrap())
        .filter(|t| !starts.contains(t))
        .collect();
    let odd = |t: u64| {
        let start = starts.iter().filter(|s| **s <= t).max().unwrap();
        (t - start) % 2 == 1
    };
    assert!(restarts.iter().any(|t| odd(*t)), "{restarts:?}");
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("pff.json");
    let out = bin()
        .args(["tracking", "measure", "--source", "pff", "--data"])
        .arg(fixtures().join("pff"))
        .arg("--out")
        .arg(&file)
        .output()
        .unwrap();
    assert!(file.exists(), "{}", text(&out));
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let m = v["matches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["match_id"] == PFF)
        .unwrap();
    let counted: f64 = words
        .iter()
        .map(|w| {
            let pair = &m["figures"][format!("team.stops_{w}")];
            pair[0].as_f64().unwrap() + pair[1].as_f64().unwrap()
        })
        .sum();
    assert_eq!(counted, restarts.len() as f64, "{restarts:?}");
}

/// A DFL match converts byte for byte as before the resampling was added.
#[test]
fn a_dfl_match_converts_byte_for_byte_as_before() {
    use sha2::{Digest, Sha256};
    let (dir, _records, _side) = converted();
    let hash = |name: &str| {
        let bytes = std::fs::read(dir.path().join(name)).unwrap();
        format!("{:x}", Sha256::digest(&bytes))
    };
    assert_eq!(
        hash(&format!("{FIX}.ticks")),
        "14375d5224ff7efed40d6ca2113755ced4a78c986bcd788c3a550aad92c87f23"
    );
    assert_eq!(
        hash(&format!("{FIX}.side.json")),
        "5a49b91c6bccafb590e9866821442c9e1cdf5b65e1bd393a1975a538c404e8d8"
    );
}

/// The help of the tracking commands names both providers within 80 columns.
#[test]
fn the_tracking_help_names_both_providers_within_eighty_columns() {
    for action in ["convert", "measure"] {
        let out = bin().args(["tracking", action, "--help"]).output().unwrap();
        assert!(out.status.success(), "{}", text(&out));
        let help = String::from_utf8_lossy(&out.stdout);
        assert!(help.contains("PFF") && help.contains("DFL"), "{help}");
        assert!(help.contains("--source"), "{help}");
        for line in help.lines() {
            assert!(line.len() <= 80, "{action}: {} columns: {line}", line.len());
        }
    }
}

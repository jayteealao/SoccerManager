//! Each club's home ground has its own length and width within the Laws, and the match plays
//! on the home team's ground. Full matches on two grounds of different sizes, each team at
//! home, read the size from the ground: the restart spots sit on that ground's lines, every
//! player stays on that ground, and the event validator accepts the match. A source scan
//! keeps fixed 105 by 68 values out of the engine's match code. Each check runs once against
//! a planted fault, so it is proven able to fail.

mod common;

use std::path::Path;

use engine::data::TeamFile;
use engine::data::team::Ground;
use engine::math::DVec2;
use engine::pitch::{CORNER_ARC, GOAL_AREA_DEPTH, PENALTY_MARK, Pitch};
use engine::{
    EngineError, EngineEvent, EngineEventKind, MatchConfig, Simulation, TickRecord, Validator,
    VecSink,
};

/// `file` with its club's ground set to `length` by `width` metres.
fn on_ground(file: &TeamFile, length: f64, width: f64) -> TeamFile {
    let mut out = file.clone();
    out.club.ground = Ground { length, width };
    out
}

/// A played match: its configuration, its events, its tick records, its team shapes, and the
/// restarts the engine's restart position check judged.
struct Played {
    config: MatchConfig,
    events: Vec<EngineEvent>,
    records: Vec<TickRecord>,
    timeline: Vec<(u32, [engine::team::Team; 2])>,
    restarts_checked: u32,
}

/// Plays `minutes` of the seed-42 match with `home` at home on its ground against `away`.
fn play(home: &TeamFile, away: &TeamFile, minutes: u32) -> Played {
    let content = common::content();
    let config = MatchConfig::new(common::SEED, minutes, &content, [home, away]).unwrap();
    let mut sim = Simulation::new(config.clone()).unwrap();
    let mut sink = VecSink::default();
    sim.run(&mut sink).unwrap();
    Played {
        config,
        events: sim.take_events(),
        records: sink.records,
        timeline: sim.team_timeline().to_vec(),
        restarts_checked: sim.restarts_checked(),
    }
}

/// The restart spot check: every restart spot lies on `pitch`, and each line restart on its
/// line of that ground. One line per failure, naming the tick, the restart, and the position.
/// Where the players stand at each restart is the engine's own restart position check, which
/// judges every restart of every match in these tests and fails loudly on a fault.
fn restart_position_faults(pitch: &Pitch, events: &[EngineEvent]) -> Vec<String> {
    let (hl, hw) = (pitch.half_length(), pitch.half_width());
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let mut faults = Vec::new();
    for e in events {
        let Some(spot) = e.spot else { continue };
        let kind = e.kind;
        let on_line = match kind {
            EngineEventKind::ThrowIn => near(spot.y.abs(), hw),
            EngineEventKind::Corner => {
                (DVec2::new(hl * spot.x.signum(), hw * spot.y.signum()) - spot).length()
                    <= CORNER_ARC
            }
            EngineEventKind::GoalKick => near(spot.x.abs(), hl - GOAL_AREA_DEPTH),
            EngineEventKind::Penalty => near(spot.x.abs(), hl - PENALTY_MARK) && spot.y == 0.0,
            EngineEventKind::KickOff => spot == DVec2::ZERO,
            _ => true,
        };
        if !pitch.contains(spot) || !on_line {
            faults.push(format!(
                "tick {}: {kind:?} spot {spot} is not on the {} by {} ground's line",
                e.tick,
                pitch.length(),
                pitch.width()
            ));
        }
    }
    faults
}

/// The checks of one full match on a ground of `length` by `width`.
fn full_match_on(length: f64, width: f64, home_index: usize) {
    let content = common::content();
    let teams = common::default_teams(&content);
    let home = on_ground(&teams[home_index], length, width);
    let away = &teams[1 - home_index];
    let played = play(&home, away, 90);
    let pitch = played.config.pitch;
    // The engine reads the size from the ground data.
    assert_eq!((pitch.length(), pitch.width()), (length, width));
    for team in &played.config.teams {
        assert_eq!(team.pitch, pitch);
    }
    // The throw-ins sit on this ground's touchlines, not on 34 m.
    let throw_ins: Vec<_> = played
        .events
        .iter()
        .filter(|e| e.kind == EngineEventKind::ThrowIn)
        .collect();
    assert!(throw_ins.len() >= 5, "only {} throw-ins", throw_ins.len());
    for e in &throw_ins {
        assert_eq!(e.spot.unwrap().y.abs(), width / 2.0, "tick {}", e.tick);
    }
    let faults = restart_position_faults(&pitch, &played.events);
    assert!(faults.is_empty(), "{}", faults.join("\n"));
    // The engine judged every restart on this ground and found every player where the Laws
    // ask; a fault would have stopped the match.
    assert!(
        played.restarts_checked > 10,
        "{} restarts",
        played.restarts_checked
    );
    let validator = Validator::for_match(
        played.config.tuning.clone(),
        &played.timeline,
        &played.events,
    );
    let violations = validator.check(&played.records);
    assert!(
        violations.is_empty(),
        "{} violations; first: {:?}",
        violations.len(),
        violations.iter().take(5).collect::<Vec<_>>()
    );
    assert!(
        played
            .events
            .iter()
            .any(|e| e.kind == EngineEventKind::FullTime)
    );
}

#[test]
fn a_full_match_on_100_by_64_with_the_first_team_at_home() {
    full_match_on(100.0, 64.0, 0);
}

#[test]
fn a_full_match_on_100_by_64_with_the_second_team_at_home() {
    full_match_on(100.0, 64.0, 1);
}

#[test]
fn a_full_match_on_120_by_90_with_the_first_team_at_home() {
    full_match_on(120.0, 90.0, 0);
}

#[test]
fn a_full_match_on_120_by_90_with_the_second_team_at_home() {
    full_match_on(120.0, 90.0, 1);
}

#[test]
fn a_wide_ground_puts_players_and_the_ball_beyond_the_default_lines() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let played = play(&on_ground(&a, 120.0, 90.0), &b, 30);
    let beyond =
        |x: f32, y: f32| f64::from(x.abs()) > 52.5 + 1.0 || f64::from(y.abs()) > 34.0 + 1.0;
    assert!(
        played
            .records
            .iter()
            .any(|r| r.players.iter().any(|p| beyond(p[0], p[1])) || beyond(r.ball[0], r.ball[1])),
        "nobody went past the default ground's lines on a 120 by 90 ground"
    );
}

#[test]
fn the_four_edge_grounds_play_and_keep_their_restarts_on_their_lines() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    for (length, width) in [(90.0, 68.0), (120.0, 68.0), (105.0, 45.0), (105.0, 90.0)] {
        let played = play(&on_ground(&a, length, width), &b, 20);
        let pitch = played.config.pitch;
        assert_eq!((pitch.length(), pitch.width()), (length, width));
        let faults = restart_position_faults(&pitch, &played.events);
        assert!(
            faults.is_empty(),
            "{length} by {width}: {}",
            faults.join("\n")
        );
        assert!(played.restarts_checked > 0, "{length} by {width}");
        let validator = Validator::for_match(
            played.config.tuning.clone(),
            &played.timeline,
            &played.events,
        );
        let violations = validator.check(&played.records);
        assert!(
            violations.is_empty(),
            "{length} by {width}: {} violations; first: {:?}",
            violations.len(),
            violations.iter().take(5).collect::<Vec<_>>()
        );
    }
}

#[test]
fn the_restart_position_check_refuses_a_spot_one_metre_off_the_ground() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let played = play(&on_ground(&a, 100.0, 64.0), &b, 20);
    let pitch = played.config.pitch;
    assert!(restart_position_faults(&pitch, &played.events).is_empty());
    let mut events = played.events.clone();
    let throw_in = events
        .iter_mut()
        .find(|e| e.kind == EngineEventKind::ThrowIn)
        .expect("a throw-in in 20 minutes");
    let spot = throw_in.spot.unwrap();
    throw_in.spot = Some(DVec2::new(spot.x, spot.y + spot.y.signum()));
    let faults = restart_position_faults(&pitch, &events);
    assert_eq!(faults.len(), 1, "{faults:?}");
    assert!(
        faults[0].contains("ThrowIn") && faults[0].contains("100 by 64"),
        "{}",
        faults[0]
    );
    // The same check on the default ground refuses the 100 by 64 match's throw-ins: the
    // check reads the ground it is given.
    assert!(!restart_position_faults(&Pitch::DEFAULT, &played.events).is_empty());
}

#[test]
fn a_match_refuses_a_home_ground_outside_the_laws() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    for (length, width) in [(89.0, 68.0), (121.0, 68.0), (105.0, 44.0), (105.0, 91.0)] {
        let err =
            MatchConfig::new(1, 90, &content, [&on_ground(&a, length, width), &b]).unwrap_err();
        assert!(matches!(err, EngineError::InvalidConfig(_)), "{err}");
        assert!(err.to_string().contains("the Laws allow"), "{err}");
    }
    // The away team's ground does not matter: the match plays on the home ground.
    let config = MatchConfig::new(1, 90, &content, [&a, &on_ground(&b, 100.0, 64.0)]).unwrap();
    assert_eq!(config.pitch, Pitch::DEFAULT);
}

/// Source files the scan allows to name the default ground: the pitch itself, the team
/// file's default, and a team's placeholder before a match gives it its ground.
const DEFAULT_GROUND_FILES: &[&str] = &["pitch.rs", "data/team.rs", "team.rs"];
/// Source files the scan allows to carry the size literals: the pitch, and the tuning file's
/// load limits (behaviour distances, checked when the content loads).
const LITERAL_FILES: &[&str] = &["pitch.rs", "tuning.rs"];

/// Every place in `files` (path relative to `src`, and the text) where match code reads a
/// fixed 105 by 68 value: a default-size name or one of the literals 105.0, 68.0, 52.5 and
/// 34.0. Test modules and comments are not match code.
fn fixed_size_reads(files: &[(String, String)]) -> Vec<String> {
    let names = ["HALF_LENGTH", "HALF_WIDTH", "pitch::LENGTH", "pitch::WIDTH"];
    let literals = ["105.0", "68.0", "52.5", "34.0"];
    let mut found = Vec::new();
    for (path, text) in files {
        for (n, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("#[cfg(test)]") {
                break;
            }
            let code = line.split("//").next().unwrap_or_default();
            let mut hit = |what: &str| found.push(format!("{path}:{}: {what}", n + 1));
            for name in names {
                if code.contains(name) {
                    hit(name);
                }
            }
            if code.contains("Pitch::DEFAULT") && !DEFAULT_GROUND_FILES.contains(&path.as_str()) {
                hit("Pitch::DEFAULT");
            }
            if !LITERAL_FILES.contains(&path.as_str()) {
                for lit in literals {
                    let bytes = code.as_bytes();
                    let mut from = 0;
                    while let Some(at) = code[from..].find(lit) {
                        let start = from + at;
                        let end = start + lit.len();
                        let before = start.checked_sub(1).map(|i| bytes[i]);
                        let after = bytes.get(end).copied();
                        let digitish = |c: Option<u8>| {
                            c.is_some_and(|c| c.is_ascii_digit() || c == b'.' || c == b'_')
                        };
                        if !digitish(before) && !digitish(after) {
                            hit(lit);
                        }
                        from = end;
                    }
                }
            }
        }
    }
    found
}

/// Every `.rs` file under `dir`, with its path relative to `root`.
fn sources(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            sources(root, &path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, std::fs::read_to_string(&path).unwrap()));
        }
    }
}

#[test]
fn no_match_code_reads_a_fixed_105_by_68_value() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &root, &mut files);
    assert!(files.iter().any(|(p, _)| p == "rules/restart.rs"));
    assert!(files.len() > 40, "only {} source files", files.len());
    let found = fixed_size_reads(&files);
    assert!(
        found.is_empty(),
        "fixed ground sizes in match code:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_scan_refuses_a_planted_fixed_size() {
    let planted = vec![
        ("decision.rs".to_string(), "fn f() -> f64 {\n    52.5 * 2.0\n}\n".to_string()),
        ("steering.rs".to_string(), "fn g() -> f64 {\n    pitch::HALF_WIDTH\n}\n".to_string()),
        ("ball.rs".to_string(), "fn h(p: DVec2) -> DVec2 {\n    Pitch::DEFAULT.clamp(p, 0.2)\n}\n".to_string()),
        // Not match code: a comment, a test module, and a longer number.
        (
            "shot.rs".to_string(),
            "// 105.0 by 68.0\nfn k() -> f64 { 152.55 }\n#[cfg(test)]\nmod tests { const X: f64 = 34.0; }\n".to_string(),
        ),
    ];
    let found = fixed_size_reads(&planted);
    assert_eq!(
        found,
        vec![
            "decision.rs:2: 52.5".to_string(),
            "steering.rs:2: HALF_WIDTH".to_string(),
            "ball.rs:2: Pitch::DEFAULT".to_string(),
        ]
    );
}

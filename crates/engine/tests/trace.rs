//! The debug trace: every random draw of a gate match is recorded with its stream, key,
//! index, value, and, at a chance draw, the probability it is tested against.
//! Controls prove the checker fails on a dropped draw and on a missing probability.
#![cfg(feature = "debug-trace")]

mod common;

use std::sync::OnceLock;

use engine::data::TeamFile;
use engine::gate::{self, Fixture, Inputs, Played};
use engine::trace::{self, TraceRecord};
use engine::{Content, EngineEvent};

fn loaded() -> &'static (Content, [TeamFile; 2]) {
    static LOADED: OnceLock<(Content, [TeamFile; 2])> = OnceLock::new();
    LOADED.get_or_init(|| {
        let content = common::content();
        let teams = common::default_teams(&content);
        (content, teams)
    })
}

fn inputs() -> Inputs<'static> {
    let (content, [a, b]) = loaded();
    Inputs {
        content,
        teams: [a, b],
        pack: None,
    }
}

/// The seed-42 gate match played traced: its result, its records, its events, and the
/// registry's draw count. Played once per test binary.
fn traced() -> &'static (Played, Vec<TraceRecord>, Vec<EngineEvent>, u64) {
    static TRACED: OnceLock<(Played, Vec<TraceRecord>, Vec<EngineEvent>, u64)> = OnceLock::new();
    TRACED.get_or_init(|| {
        let mut records = Vec::new();
        let mut events = Vec::new();
        let (played, draws) = gate::play_traced(&Fixture::seed(42), &inputs(), |r, e| {
            records.extend_from_slice(r);
            events.extend_from_slice(e);
        })
        .unwrap();
        (played, records, events, draws)
    })
}

#[test]
fn every_draw_of_a_gate_match_is_recorded_with_its_fields() {
    let (_, records, _, registry) = traced();
    assert!(
        *registry > 100_000,
        "a 90-minute match takes {registry} draws"
    );
    trace::check_draws(records, *registry).unwrap();
    let draws: Vec<_> = records
        .iter()
        .filter(|r| matches!(r, TraceRecord::Draw(_)))
        .collect();
    assert_eq!(draws.len() as u64, *registry);
    // The file form of every draw has the fields a draw record names.
    let mut chance = 0;
    for r in &draws {
        let v = r.to_json();
        assert_eq!(v["k"], "draw");
        for field in [
            "t",
            "subsystem",
            "stream_id",
            "key",
            "index",
            "value",
            "scripted",
        ] {
            assert!(!v[field].is_null(), "{field} missing in {v}");
        }
        assert!(v["stream_id"].as_str().unwrap().starts_with("0x"));
        if !v["p"].is_null() {
            chance += 1;
            assert!(
                v["p"]
                    .as_array()
                    .is_some_and(|p| (1..=2).contains(&p.len()))
            );
        }
    }
    assert!(chance > 0, "the match took chance draws");
    // Ticks never go back.
    assert!(records.windows(2).all(|w| w[0].tick() <= w[1].tick()));
}

#[test]
fn the_traced_match_keeps_the_untraced_hashes() {
    let (played, _, _, _) = traced();
    let plain = gate::play_fixture(&Fixture::seed(42), &inputs()).unwrap();
    assert_eq!(played.hashes, plain.hashes);
}

#[test]
fn the_draw_checker_fails_on_a_dropped_draw() {
    let (_, records, _, registry) = traced();
    let at = records
        .iter()
        .position(|r| matches!(r, TraceRecord::Draw(_)))
        .unwrap()
        + 1000;
    let mut dropped = records.clone();
    // The 1,000th record after the first draw, or the next draw after it.
    let k = (at..dropped.len())
        .find(|&k| matches!(dropped[k], TraceRecord::Draw(_)))
        .unwrap();
    dropped.remove(k);
    let err = trace::check_draws(&dropped, *registry).unwrap_err();
    assert!(
        err.contains("index") || err.contains("draws recorded"),
        "{err}"
    );
    // The last draw leaves no index gap: the count alone catches it.
    let mut last = records.clone();
    let k = last
        .iter()
        .rposition(|r| matches!(r, TraceRecord::Draw(_)))
        .unwrap();
    last.remove(k);
    let err = trace::check_draws(&last, *registry).unwrap_err();
    assert!(err.contains("draws recorded"), "{err}");
}

#[test]
fn the_draw_checker_fails_on_a_missing_probability() {
    let (_, records, _, registry) = traced();
    let mut stripped = records.clone();
    let k = stripped
        .iter()
        .position(|r| matches!(r, TraceRecord::Draw(d) if !d.thresholds.is_empty()))
        .unwrap();
    if let TraceRecord::Draw(d) = &mut stripped[k] {
        d.thresholds.clear();
    }
    let err = trace::check_draws(&stripped, *registry).unwrap_err();
    assert!(err.contains("has no probability"), "{err}");
}

#[test]
fn an_untraced_match_records_nothing() {
    let config = common::short_match(2);
    let mut sim = engine::Simulation::new(config).unwrap();
    assert!(!sim.debug_trace_on());
    while !sim.is_over() {
        sim.step();
        assert!(sim.take_trace().is_empty());
    }
}

#[test]
fn the_header_names_the_maths_library_in_the_lock_file() {
    let lock = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock"),
    )
    .unwrap();
    let version = trace::MATHS.strip_prefix("libm ").unwrap();
    assert!(
        lock.contains(&format!("name = \"libm\"\nversion = \"{version}\"")),
        "Cargo.lock has another libm than {}",
        trace::MATHS
    );
    let header = trace::TraceHeader { seed: 7, scheme: 1 }.to_json();
    assert_eq!(header["trace_version"], 1);
    assert_eq!(header["seed"], 7);
    assert_eq!(header["maths"], trace::MATHS);
    assert_eq!(header["engine"], engine::build_hash());
}

// --- Coverage list: the anchors, the checkers' controls, and the scene-only points.

use engine::math::{DVec2, DVec3};
use engine::scenario::Scene;
use engine::streams::Action;
use engine::trace::Point;
use engine::{EngineEventKind, Simulation};

#[test]
fn every_action_and_event_kind_maps_to_a_point() {
    for action in Action::ALL {
        assert!(!trace::points_of(action).is_empty(), "{action:?}");
    }
    for kind in EngineEventKind::ALL {
        assert!(!trace::points_of_event(kind).is_empty(), "{kind:?}");
    }
    assert_eq!(Point::ALL.len(), 37);
    assert_eq!(Point::ALL.iter().filter(|p| p.is_decision()).count(), 10);
    let names: std::collections::BTreeSet<_> = Point::ALL.iter().map(|p| p.name()).collect();
    assert_eq!(names.len(), 37, "every point has its own name");
    assert_eq!(
        Action::ALL.iter().filter(|a| trace::is_chance(**a)).count(),
        20
    );
}

#[test]
fn every_tick_of_the_traced_match_has_the_points_of_its_draws_and_events() {
    let (_, records, events, _) = traced();
    let mut from = 0;
    let mut e = 0;
    while from < records.len() {
        let tick = records[from].tick();
        let to = from
            + records[from..]
                .iter()
                .take_while(|r| r.tick() == tick)
                .count();
        let start = e;
        while e < events.len() && events[e].tick <= tick {
            e += 1;
        }
        trace::check_tick(&records[from..to], &events[start..e]).unwrap();
        from = to;
    }
    assert_eq!(e, events.len(), "every event was checked");
}

#[test]
fn the_tick_checker_fails_on_a_draw_without_its_point() {
    let (_, records, events, _) = traced();
    let at = records
        .iter()
        .position(|r| matches!(r, TraceRecord::Point(p) if p.point == Point::Tackle))
        .expect("the match has a tackle");
    let tick = records[at].tick();
    let same: Vec<_> = records
        .iter()
        .filter(|r| r.tick() == tick)
        .cloned()
        .collect();
    let tick_events: Vec<_> = events.iter().filter(|e| e.tick == tick).copied().collect();
    trace::check_tick(&same, &tick_events).unwrap();
    let cut: Vec<_> = same
        .into_iter()
        .filter(|r| !matches!(r, TraceRecord::Point(p) if p.point == Point::Tackle))
        .collect();
    let err = trace::check_tick(&cut, &tick_events).unwrap_err();
    assert!(err.contains("laws.tackle"), "{err}");
}

#[test]
fn the_tick_checker_fails_on_an_event_without_its_point() {
    let (_, records, events, _) = traced();
    let kick_off = events
        .iter()
        .find(|e| e.kind == EngineEventKind::KickOff)
        .unwrap();
    let same: Vec<_> = records
        .iter()
        .filter(|r| r.tick() == kick_off.tick)
        .filter(|r| {
            !matches!(r, TraceRecord::Point(p) if matches!(p.point, Point::KickOff | Point::DeadBall))
        })
        .cloned()
        .collect();
    let err = trace::check_tick(&same, std::slice::from_ref(kick_off)).unwrap_err();
    assert!(err.contains("event kick-off"), "{err}");
}

#[test]
fn the_partition_checker_names_a_missing_point() {
    let all: std::collections::BTreeSet<Point> = Point::ALL
        .iter()
        .copied()
        .filter(|p| !trace::SCENE_ONLY.contains(p))
        .collect();
    trace::check_partition(&all).unwrap();
    let mut missing = all.clone();
    missing.remove(&Point::ShootoutDecided);
    let err = trace::check_partition(&missing).unwrap_err();
    assert!(err.contains("shootout_decided"), "{err}");
    let mut extra = all;
    extra.insert(Point::Abandoned);
    assert!(trace::check_partition(&extra).is_err());
}

/// The points of `records`.
fn points(records: &[TraceRecord]) -> Vec<&engine::trace::PointRecord> {
    records
        .iter()
        .filter_map(|r| match r {
            TraceRecord::Point(p) => Some(p),
            TraceRecord::Draw(_) => None,
        })
        .collect()
}

/// Scene-only point `abandoned` (with `send_off`, `card`, and `tackle`): a red card that
/// leaves the away side with six players ends the match.
#[test]
fn a_red_card_below_the_minimum_records_the_send_off_and_the_abandonment() {
    const CARRIER: usize = 5;
    const TACKLER: usize = 16;
    let config = common::quiet_match(90);
    let plain = Simulation::new(config.clone()).unwrap();
    let tackler = plain.skills(TACKLER);
    let carrier = plain.skills(CARRIER);
    let p_win = engine::rules::fouls::win_chance(tackler, carrier, &config.tuning);
    let p_foul = engine::rules::fouls::foul_chance(tackler, 0, &config.tuning);
    let at = DVec2::new(0.0, 10.0);
    let scene = [12, 13, 14, 15].into_iter().fold(
        common::spread(Scene::new(config), -30.0, 30.0),
        |scene, i| scene.sent_off(i),
    );
    let mut sim = scene
        .traced()
        .place(CARRIER, at)
        .place(TACKLER, at + DVec2::new(0.3, 0.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(CARRIER))
        .tick(1_001)
        .rolls(&[p_win + 0.01 * p_foul, 0.0])
        .build();
    sim.step();
    assert!(sim.abandoned());
    let records = sim.take_trace();
    let events = sim.take_events();
    trace::check_tick(&records, &events).unwrap();
    trace::check_draws(&records, sim.draws()).unwrap();
    let got = points(&records);
    let find = |point: Point| {
        *got.iter()
            .find(|p| p.point == point)
            .unwrap_or_else(|| panic!("no {} record in {got:?}", point.name()))
    };
    assert_eq!(find(Point::Tackle).detail["outcome"], "foul_ball_lost");
    assert_eq!(find(Point::Card).detail["card"], "red");
    assert_eq!(find(Point::SendOff).detail["player"], TACKLER);
    let abandoned = find(Point::Abandoned);
    assert_eq!(abandoned.detail["short_team"], 1);
    assert_eq!(abandoned.detail["reason"], "below_minimum");
}

/// Scene-only point `cross_clear`: the shipped tuning never clears a cross (its chance is
/// 0), so a scene with the chance at 1 forces one.
#[test]
fn a_cleared_cross_records_the_defender_and_the_clearance() {
    const DEFENDER: usize = 13;
    const AWAY_KEEPER: usize = 11;
    let mut config = common::calm_match(90);
    config.tuning.clearances.cross_chance = 1.0;
    config.tuning.clearances.wide_chance = 0.5;
    let mut sim = common::spread(Scene::new(config), -30.0, -30.0)
        .traced()
        .place(common::index(0, 9), DVec2::new(30.0, 0.0))
        .place(DEFENDER, DVec2::new(40.0, 0.5))
        .place(AWAY_KEEPER, DVec2::new(30.0, -25.0))
        .ball(DVec3::new(30.0, 0.0, 0.0))
        .carrier(Some(common::index(0, 9)))
        .tick(1_001)
        .kick(DVec2::X, 20.0, 0.0)
        .build();
    let mut all = Vec::new();
    for _ in 0..300 {
        sim.step();
        let records = sim.take_trace();
        let events = sim.take_events();
        trace::check_tick(&records, &events).unwrap();
        all.extend(records);
        if sim.summary().clearances[1] > 0 {
            break;
        }
    }
    assert_eq!(sim.summary().clearances, [0, 1]);
    let clear = points(&all)
        .into_iter()
        .find(|p| p.point == Point::CrossClear)
        .expect("a cross_clear record");
    assert_eq!(clear.detail["defender"], DEFENDER);
    assert_eq!(clear.detail["cleared"], true);
    assert!(clear.detail["wide"].is_boolean());
    let json = TraceRecord::Point(clear.clone()).to_json();
    assert_eq!(json["k"], "rule");
    assert_eq!(json["point"], "cross_clear");
}

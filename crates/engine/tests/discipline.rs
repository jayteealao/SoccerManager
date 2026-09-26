//! Discipline. A player who fouls makes no tackle attempt until the tuned cooldown has
//! passed, advantage or not; a player holds at most one card during a spell of advantage, the
//! most severe; a stopping foul by a player with a held card shows one card; and nobody is
//! shown two cards on one tick.
//!
//! The criterion test is slow: `cargo test --release -p engine --all-features -- --ignored`.
//! Over 200 matches with the shipped content, second yellow cards are at most 0.10 per match,
//! at most a quarter of matches have a sending-off, and no player is shown two cards on the
//! same tick.

mod common;

use common::{calm_match, content, default_teams, run_many, spread};
use engine::math::{DVec2, DVec3};
use engine::record::NullSink;
use engine::rules::fouls::foul_chance;
use engine::scenario::Scene;
use engine::{Card, EngineEvent, EngineEventKind, MatchConfig, Simulation};

/// The home carrier, and the two away players beside the ball.
const CARRIER: usize = 5;
const TACKLER: usize = 16;
const OTHER: usize = 17;

/// No foul, and no card: above every band.
const MISS: f64 = 0.99;

/// The win and foul chances of player `i` tackling the carrier, unbooked.
fn bands(config: &MatchConfig, i: usize) -> (f64, f64) {
    let sim = Simulation::new(config.clone()).unwrap();
    let tackler = sim.players()[i].derived;
    let carrier = sim.players()[CARRIER].derived;
    let p_win = engine::rules::fouls::win_chance(&tackler, &carrier, &config.tuning);
    (p_win, foul_chance(&tackler, 0, &config.tuning))
}

/// A foul draw after which the fouled team keeps the ball: advantage.
fn advantage((w, f): (f64, f64)) -> f64 {
    w + 0.9 * f
}

/// A foul draw after which the fouled team loses the ball: play stops.
fn stopping((w, f): (f64, f64)) -> f64 {
    w + 0.1 * f
}

/// The carrier holds the ball in midfield with the tackler, and `other` when given, beside
/// it; `rolls` are the referee's next draws.
fn scene(config: MatchConfig, other: bool, rolls: &[f64]) -> Simulation {
    let at = DVec2::new(0.0, 10.0);
    let mut scene = spread(Scene::new(config), -30.0, 30.0)
        .place(CARRIER, at)
        .place(TACKLER, at + DVec2::new(0.3, 0.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(CARRIER))
        .tick(1_001)
        .rolls(rolls);
    if other {
        scene = scene.place(OTHER, at + DVec2::new(-0.3, 0.3));
    }
    scene.build()
}

fn play(sim: &mut Simulation, ticks: usize) -> Vec<EngineEvent> {
    let mut events = Vec::new();
    for _ in 0..ticks {
        sim.step();
        events.extend(sim.take_events());
    }
    events
}

fn cards_for(events: &[EngineEvent], i: usize) -> Vec<Card> {
    events
        .iter()
        .filter(|e| e.kind == EngineEventKind::Card && e.player == Some(i))
        .filter_map(|e| e.card)
        .collect()
}

#[test]
fn a_tackler_who_fouled_with_advantage_makes_no_attempt_on_the_next_tick() {
    let config = calm_match(90);
    let b = bands(&config, TACKLER);
    // The second tackle draw would be a foul too, if the tackler drew again.
    let mut sim = scene(config, false, &[advantage(b), MISS, advantage(b), MISS]);
    let events = play(&mut sim, 2);
    let fouls = events
        .iter()
        .filter(|e| e.kind == EngineEventKind::Foul)
        .count();
    assert_eq!(fouls, 1, "{events:?}");
    let ready = sim.players()[TACKLER].foul_ready;
    assert_eq!(ready, 1_001 + sim.tuning().foul_cooldown_ticks);
}

#[test]
fn two_advantage_fouls_by_one_player_hold_one_card_the_more_severe() {
    let mut config = calm_match(90);
    config.tuning.foul_cooldown_ticks = 0;
    let a = bands(&config, TACKLER);
    let o = bands(&config, OTHER);
    let yellow = config.tuning.red_base + 0.001;
    let red = 0.0;
    // Tick 1: an advantage foul with a caution held. Tick 2: another with a red held. Tick 3:
    // the tackler misses and the other player stops play, which shows the held card.
    let rolls = [
        advantage(a),
        yellow,
        advantage(a),
        red,
        MISS,
        stopping(o),
        MISS,
    ];
    let mut sim = scene(config, true, &rolls);
    let events = play(&mut sim, 3);
    assert_eq!(cards_for(&events, TACKLER), vec![Card::Red], "{events:?}");
}

#[test]
fn a_stopping_foul_by_a_player_holding_a_card_shows_one_card() {
    let mut config = calm_match(90);
    config.tuning.foul_cooldown_ticks = 0;
    let a = bands(&config, TACKLER);
    let yellow = config.tuning.red_base + 0.001;
    let mut sim = scene(config, false, &[advantage(a), yellow, stopping(a), yellow]);
    let events = play(&mut sim, 2);
    assert_eq!(
        cards_for(&events, TACKLER),
        vec![Card::Yellow],
        "{events:?}"
    );
    assert_eq!(sim.players()[TACKLER].yellow, 1);
    assert!(sim.players()[TACKLER].active());
}

#[test]
#[ignore = "slow: cargo test --release -p engine --all-features -- --ignored"]
fn discipline_is_realistic() {
    let content = content();
    let [a, b] = default_teams(&content);
    let rows = run_many(1..=200, |seed| {
        let config = MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap();
        let mut sim = Simulation::new(config).unwrap();
        sim.run(&mut NullSink).unwrap();
        let events = sim.take_events();
        let cards: Vec<(u32, usize, Card)> = events
            .iter()
            .filter(|e| e.kind == EngineEventKind::Card)
            .filter_map(|e| Some((e.tick, e.player?, e.card?)))
            .collect();
        let second = cards.iter().filter(|c| c.2 == Card::SecondYellow).count();
        let mut twice = 0;
        for (k, c) in cards.iter().enumerate() {
            twice += cards[k + 1..]
                .iter()
                .filter(|d| d.0 == c.0 && d.1 == c.1)
                .count();
        }
        let s = sim.summary();
        (second, s.red[0] + s.red[1] > 0, twice)
    });
    let n = rows.len() as f64;
    let second = rows.iter().map(|r| r.0 as f64).sum::<f64>() / n;
    let sent_off = rows.iter().filter(|r| r.1).count() as f64 / n;
    let twice: usize = rows.iter().map(|r| r.2).sum();
    eprintln!(
        "second yellows per match {second:.3}, matches with a sending-off {:.1}%, same-tick pairs {twice}",
        sent_off * 100.0
    );
    assert!(second <= 0.10, "second yellows per match {second:.3}");
    assert!(
        sent_off <= 0.25,
        "matches with a sending-off {:.1}%",
        sent_off * 100.0
    );
    assert_eq!(twice, 0, "a player was shown two cards on one tick");
}

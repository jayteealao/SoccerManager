//! AC-c: a second yellow card sends the player off. The player parks off the pitch, the team
//! plays with ten, and the rest of the player's line spreads across the line's width. A
//! team reduced below the rule pack's minimum ends the match.

mod common;

use common::{quiet_match, spread};
use engine::math::{DVec2, DVec3};
use engine::player::Status;
use engine::rules::discipline::on_pitch_count;
use engine::rules::fouls::foul_chance;
use engine::scenario::Scene;
use engine::{Card, EngineEventKind, Simulation};

const CARRIER: usize = 5;
const TACKLER: usize = 16;

/// A foul by the tackler with the ball lost, judged with `card_draw`, in a scene `arrange`
/// prepares.
fn foul(card_draw: impl Fn(&Simulation) -> f64, arrange: impl Fn(Scene) -> Scene) -> Simulation {
    let config = quiet_match(90);
    let plain = Simulation::new(config.clone()).unwrap();
    let tackler = plain.players()[TACKLER].derived;
    let carrier = plain.players()[CARRIER].derived;
    let p_win = engine::rules::fouls::win_chance(&tackler, &carrier, &config.tuning);
    let p_foul = foul_chance(&tackler, 0, &config.tuning);
    let at = DVec2::new(0.0, 10.0);
    let scene = spread(Scene::new(config), -30.0, 30.0)
        .place(CARRIER, at)
        .place(TACKLER, at + DVec2::new(0.3, 0.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(CARRIER))
        .tick(1_001)
        // Deep in the band of a foul with the ball lost, for a booked tackler too (whose foul
        // chance is the booked factor times this one).
        .rolls(&[p_win + 0.01 * p_foul, card_draw(&plain)]);
    let mut sim = arrange(scene).build();
    sim.step();
    sim
}

#[test]
fn a_second_yellow_sends_the_player_off_and_the_line_spreads() {
    // Inside the yellow band and above the red one.
    let mut sim = foul(
        |sim| sim.tuning().red_base + 0.001,
        |scene| scene.yellow(TACKLER, 1),
    );
    let before = Simulation::new(quiet_match(90)).unwrap().teams()[1].clone();
    let events = sim.take_events();
    let card = events
        .iter()
        .find(|e| e.kind == EngineEventKind::Card)
        .expect("a card event");
    assert_eq!(card.card, Some(Card::SecondYellow));
    assert_eq!(card.player, Some(TACKLER));
    assert_eq!(card.team, Some(1));

    let player = sim.players()[TACKLER];
    assert_eq!(player.status, Status::SentOff);
    assert!(engine::pitch::is_parking_spot(player.pos), "{}", player.pos);
    assert_eq!(on_pitch_count(sim.players(), 1), 10);
    assert_eq!(sim.summary().red, [0, 1]);

    let after = &sim.teams()[1];
    assert!(!after.active[5]);
    let depth = before.formation[5].0;
    let line: Vec<usize> = (0..11)
        .filter(|&s| s != 5 && before.formation[s].0 == depth)
        .collect();
    assert!(!line.is_empty(), "slot 5 has team-mates on its line");
    let (lo, hi) = line
        .iter()
        .chain([5].iter())
        .map(|&s| before.formation[s].1)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), y| {
            (lo.min(y), hi.max(y))
        });
    let ys: Vec<f64> = line.iter().map(|&s| after.formation[s].1).collect();
    let spread_lo = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let spread_hi = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if line.len() > 1 {
        assert!(
            (spread_lo - lo).abs() < 1e-9 && (spread_hi - hi).abs() < 1e-9,
            "{ys:?}"
        );
    }
    assert_ne!(
        line.iter()
            .map(|&s| before.formation[s].1)
            .collect::<Vec<_>>(),
        ys,
        "the line re-spreads"
    );

    // The sent-off player takes no further part: play restarts and runs on without the
    // player leaving the parking spot.
    for _ in 0..2_000 {
        sim.step();
    }
    assert!(engine::pitch::is_parking_spot(sim.players()[TACKLER].pos));
}

#[test]
fn a_fifth_send_off_abandons_the_match() {
    let mut sim = foul(
        |_| 0.0,
        |scene| {
            [12, 13, 14, 15]
                .into_iter()
                .fold(scene, |scene, i| scene.sent_off(i))
        },
    );
    let events = sim.take_events();
    let card = events
        .iter()
        .find(|e| e.kind == EngineEventKind::Card)
        .expect("a card event");
    assert_eq!(card.card, Some(Card::Red));
    assert_eq!(on_pitch_count(sim.players(), 1), 6);
    assert!(sim.abandoned());
    assert!(sim.is_over());
    let tick = sim.tick();
    sim.step();
    assert_eq!(sim.tick(), tick, "an abandoned match does not advance");
}

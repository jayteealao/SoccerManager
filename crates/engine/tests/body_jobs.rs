//! The body fields and the condition inputs have their jobs in play (shape AC-15): height
//! adds standing reach and costs a little turning, and a taller player takes a ball in the
//! air that a shorter one cannot reach; age speeds the drain after minute 60 and changes nothing before it; days of rest
//! set the energy at kick-off and, when short, raise the injury chance; adaptation lowers the
//! mental ratings (nationality acts through it); and the build word derives from strength
//! and balance. Each check compares directions on fixed seeds of generated top-flight clubs;
//! the statistical proof of each job belongs to the sensitivity rules.

mod common;

use engine::contract::body::Body;
use engine::contract::states::GROUP_COUNT;
use engine::data::Group;
use engine::data::team::{Condition, TeamFile};
use engine::rules::clock::TICKS_PER_MINUTE;
use engine::tuning::Tuning;
use engine::{MatchConfig, Simulation};

const MENTAL: usize = 1;
const SEEDS: std::ops::RangeInclusive<u64> = 1..=6;

/// Two generated top-flight clubs of league seed `seed`, the home club changed by `home`.
fn clubs(seed: u64, home: impl Fn(&mut TeamFile)) -> [TeamFile; 2] {
    let content = common::content();
    let mut league = engine::data::generate_league_in_tier(seed, 2, 0, &content);
    let away = league.pop().unwrap();
    let mut home_file = league.pop().unwrap();
    home(&mut home_file);
    [home_file, away]
}

fn sim(seed: u64, home: impl Fn(&mut TeamFile)) -> Simulation {
    let content = common::content();
    let [a, b] = clubs(seed, home);
    Simulation::new(MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap()).unwrap()
}

fn heights(cm: u8) -> impl Fn(&mut TeamFile) {
    move |f| f.players.iter_mut().for_each(|p| p.height = Some(cm))
}

fn ages(years: u8) -> impl Fn(&mut TeamFile) {
    move |f| f.players.iter_mut().for_each(|p| p.age = Some(years))
}

fn condition(c: Condition) -> impl Fn(&mut TeamFile) {
    move |f| f.players.iter_mut().for_each(|p| p.condition = Some(c))
}

/// Whether home player `slot`, alone near a still ball `z` metres up, takes it on the next
/// tick, the home side `cm` tall.
fn takes_ball_at(seed: u64, cm: u8, slot: usize, z: f64) -> bool {
    use engine::math::{DVec2, DVec3};
    let content = common::content();
    let [a, b] = clubs(seed, heights(cm));
    let mut config = MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap();
    config.tuning.injury_per_minute = 0.0;
    config.tuning.injury_per_tackle = 0.0;
    let player = common::index(0, slot);
    let mut sim = common::spread(engine::scenario::Scene::new(config), -40.0, 40.0)
        .place(player, DVec2::new(0.0, 0.0))
        .ball(DVec3::new(0.3, 0.0, z))
        .carrier(None)
        .tick(1_000)
        .build();
    sim.step();
    sim.carrier() == Some(player)
}

#[test]
fn height_adds_reach_costs_turning_and_the_taller_player_reaches_a_higher_ball() {
    let t = Tuning::default();
    let short = sim(1, heights(170));
    let tall = sim(1, heights(195));
    for i in 0..11 {
        let (s, l) = (&short.players()[i].derived, &tall.players()[i].derived);
        let gap = l.reach_m - s.reach_m;
        assert!(
            (gap - 25.0 * t.contract.body.height.reach_per_cm).abs() < 1e-9,
            "player {i}: reach gap {gap}"
        );
        assert!(l.turn < s.turn, "player {i}: the taller turns slower");
    }
    // A ball between the two reaches: the tall player takes it, the short one cannot.
    for (seed, slot) in SEEDS.zip([5, 6, 7, 8, 9, 10]) {
        let (s, l) = (
            sim(seed, heights(170)).players()[slot].derived.reach_m,
            sim(seed, heights(195)).players()[slot].derived.reach_m,
        );
        let z = (s + l) / 2.0;
        assert!(
            takes_ball_at(seed, 195, slot, z),
            "seed {seed}: tall misses {z:.2} m"
        );
        assert!(
            !takes_ball_at(seed, 170, slot, z),
            "seed {seed}: short takes {z:.2} m"
        );
        // Both take a ball below the short reach.
        assert!(
            takes_ball_at(seed, 170, slot, s - 0.3),
            "seed {seed}: short misses low"
        );
    }
}

/// Energy the home side loses from minute 60 to the end: the sum of every fall in a home
/// player's energy from one tick to the next (a substitute coming on is a rise, not counted).
fn home_energies(sim: &Simulation) -> Vec<f64> {
    sim.players()
        .iter()
        .filter(|p| p.team == 0)
        .map(|p| p.energy)
        .collect()
}

/// The home side's energies at minute 60, and the energy it loses from there to the end.
fn late_drain(mut sim: Simulation) -> (Vec<f64>, f64) {
    let late = 60 * TICKS_PER_MINUTE;
    while sim.tick() < late {
        sim.step();
    }
    let at_60 = home_energies(&sim);
    let mut lost = 0.0;
    let mut before = at_60.clone();
    while !sim.is_over() {
        sim.step();
        let now = home_energies(&sim);
        lost += before
            .iter()
            .zip(&now)
            .map(|(b, n)| (b - n).max(0.0))
            .sum::<f64>();
        before = now;
    }
    (at_60, lost)
}

#[test]
fn an_older_side_drains_faster_after_minute_60_and_the_same_before_it() {
    let runs: Vec<[(Vec<f64>, f64); 2]> = common::run_many(SEEDS, |seed| {
        [
            late_drain(sim(seed, ages(22))),
            late_drain(sim(seed, ages(34))),
        ]
    });
    let (mut young, mut old) = (0.0, 0.0);
    for (seed, [y, o]) in SEEDS.zip(&runs) {
        assert_eq!(y.0, o.0, "seed {seed}: energies at minute 60 differ");
        young += y.1;
        old += o.1;
    }
    assert!(
        old > young,
        "energy lost after minute 60: old {old}, young {young}"
    );
}

#[test]
fn short_rest_lowers_the_kick_off_energy_and_raises_the_injury_chance() {
    let rest = |days| {
        condition(Condition {
            rest_days: Some(days),
            ..Condition::default()
        })
    };
    let tired = sim(1, rest(2));
    let rested = sim(1, rest(7));
    for i in 0..11 {
        let (t, r) = (tired.players()[i].energy, rested.players()[i].energy);
        assert!(
            t < r,
            "player {i}: kick-off energy {t} after 2 days, {r} after 7"
        );
        assert_eq!(
            tired.players()[11 + i].energy,
            1.0,
            "the away side gives no rest days"
        );
    }
    let jobs = &Tuning::default().contract.body;
    let body = Body {
        height_cm: None,
        age: Some(28),
    };
    assert!(jobs.congestion(Some(2), body) > 1.0);
    assert_eq!(jobs.congestion(Some(7), body), 1.0);
    let older = Body {
        age: Some(34),
        ..body
    };
    assert!(jobs.congestion(Some(2), older) > jobs.congestion(Some(2), body));
}

#[test]
fn adaptation_lowers_only_the_mental_ratings() {
    let content = common::content();
    let schema = &content.attributes;
    let adapting = sim(
        1,
        condition(Condition {
            adaptation: Some(30),
            ..Condition::default()
        }),
    );
    let settled = sim(
        1,
        condition(Condition {
            adaptation: Some(100),
            ..Condition::default()
        }),
    );
    for i in 0..11 {
        let mut want = [0i8; GROUP_COUNT];
        want[MENTAL] = -7;
        assert_eq!(adapting.players()[i].deltas, want, "player {i}");
        assert_eq!(settled.players()[i].deltas, [0; GROUP_COUNT], "player {i}");
        let (a, s) = (adapting.effective_ratings(i), settled.effective_ratings(i));
        for (k, def) in schema.attributes.iter().enumerate() {
            let (at, st) = (a.get(k).tenths(), s.get(k).tenths());
            if def.group == Group::Mental {
                assert_eq!(at, st.saturating_sub(7).max(10), "player {i} {}", def.name);
            } else {
                assert_eq!(at, st, "player {i} {}", def.name);
            }
        }
        assert_ne!(
            adapting.skills(i).stages,
            settled.skills(i).stages,
            "player {i}: his stage values follow"
        );
    }
}

#[test]
fn the_build_word_derives_from_strength_and_balance() {
    let jobs = &Tuning::default().contract.body;
    assert_eq!(jobs.build_word(6.0, 12.0), "slight");
    assert_eq!(jobs.build_word(12.0, 10.0), "athletic");
    assert_eq!(jobs.build_word(16.0, 8.0), "powerful");
}

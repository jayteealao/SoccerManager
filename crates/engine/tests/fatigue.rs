//! AC-4: above the fatigue threshold a player's effective values equal the base; below it the
//! fatigue curve lowers his ratings, physical first, technical and goalkeeping by their
//! weights, within the body cap, and his effective values are derived from those ratings; a
//! 90-minute match ends with every player's energy below 1.0 and above 0.0. With the fatigue
//! modifier off they stay at base.

mod common;

use common::{calm_match, full_match, index};
use engine::Simulation;
use engine::contract::states::{self, GROUP_COUNT};
use engine::data::FatigueTuning;
use engine::fatigue::{delta, multiplier};
use engine::modules::modifier::FAMILY_COUNT;
use engine::modules::{REGISTRY, SlotEntry, SlotFile, resolve};
use engine::player::Derived;
use engine::record::NullSink;
use engine::scenario::Scene;
use engine::tuning::Tuning;

const PLAYER: usize = 7;
/// The mental group's index.
const MENTAL: usize = 1;
/// The physical group's index.
const PHYSICAL: usize = 2;

fn at_energy(energy: f64) -> Simulation {
    Scene::new(calm_match(90))
        .energy(index(0, PLAYER), energy)
        .build()
}

/// The deltas fatigue alone gives at `energy`, in tenths.
fn fatigue_deltas(energy: f64, f: &FatigueTuning, t: &Tuning) -> [i8; GROUP_COUNT] {
    let mut per_family = [[0.0; GROUP_COUNT]; FAMILY_COUNT];
    per_family[0] = f
        .group_weights
        .by_group()
        .map(|w| delta(energy, f, t.contract.curve.width, w));
    states::capped(per_family, &t.contract.states)
}

/// The effective values of player `i` at `deltas`, derived from his effective ratings.
fn derived_at(sim: &Simulation, i: usize, deltas: [i8; GROUP_COUNT]) -> Derived {
    let p = sim.players()[i];
    let schema = &sim.config().attributes;
    let ratings = states::effective(&p.attributes, schema, deltas);
    Derived::from_attributes(&ratings, schema, &sim.config().tuning).0
}

#[test]
fn above_the_threshold_the_effective_values_equal_the_base() {
    for energy in [1.0, 0.9, 0.7] {
        let sim = at_energy(energy);
        let p = sim.players()[index(0, PLAYER)];
        assert_eq!(p.deltas, [0; GROUP_COUNT], "energy {energy}");
        assert_eq!(&p.derived, sim.base(index(0, PLAYER)), "energy {energy}");
    }
}

#[test]
fn below_the_threshold_the_curve_lowers_the_ratings_physical_first() {
    let config = calm_match(90);
    let (f, t) = (config.fatigue.clone(), config.tuning.clone());
    // The shipped curve passes through these points.
    for (energy, expected) in [(0.5, 0.92), (0.3, 0.82), (0.0, 0.65)] {
        let m = multiplier(energy, &f);
        assert!((m - expected).abs() < 1e-12, "energy {energy}: {m}");
        let sim = at_energy(energy);
        let i = index(0, PLAYER);
        let p = sim.players()[i];
        let deltas = fatigue_deltas(energy, &f, &t);
        assert_eq!(p.deltas, deltas, "energy {energy}");
        assert!(deltas[PHYSICAL] < 0, "energy {energy}: {deltas:?}");
        assert!(
            deltas.iter().all(|&d| d >= deltas[PHYSICAL]),
            "physical first: {deltas:?}"
        );
        assert_eq!(deltas[MENTAL], 0, "the body family leaves the mental group");
        assert!(
            f64::from(deltas[PHYSICAL]) >= t.contract.states.caps.body[0] * 10.0,
            "within the body cap: {deltas:?}"
        );
        assert_eq!(p.derived, derived_at(&sim, i, deltas), "energy {energy}");
        assert!(p.max_speed() < sim.base(i).max_speed);
    }
}

#[test]
fn a_full_match_leaves_every_player_tired_but_able() {
    let mut sim = Simulation::new(full_match()).unwrap();
    sim.run(&mut NullSink).unwrap();
    for (i, p) in sim.players().iter().enumerate() {
        assert!(
            p.energy > 0.0 && p.energy < 1.0,
            "player {i} ended at energy {}",
            p.energy
        );
    }
    let mean = sim.fatigue_mean_pct();
    assert!(mean > 0.0 && mean < 100.0, "{mean}");
}

/// Fatigue in extra time: a level knockout match from minute 89 to minute 106. Energy only ever
/// falls, across minute 90 and both breaks too, and no per-tick step at minute 90 is larger
/// than the largest one elsewhere; at minute 105 the effective values follow the same curve.
#[test]
fn fatigue_runs_on_through_extra_time_without_a_step_at_ninety_minutes() {
    use engine::fatigue::REFRESH_TICKS;
    use engine::rules::clock::TICKS_PER_MINUTE;

    let i = index(0, PLAYER);
    let mut sim = Scene::new(calm_match(90).with_knockout())
        .manager(0, engine::Manager::Human)
        .manager(1, engine::Manager::Human)
        .at_minute(89)
        .build();
    let mut energy = vec![sim.players()[i].energy];
    let mut at_ninety = None;
    let mut checked_105 = false;
    while sim.minute().0 < 106 {
        let before = sim.minute();
        sim.step();
        energy.push(sim.players()[i].energy);
        if before.0 < 90 && sim.minute().0 >= 90 && at_ninety.is_none() {
            at_ninety = Some(energy.len() - 1);
        }
        let (minute, added) = sim.minute();
        if minute == 105 && added.is_none() && sim.tick().is_multiple_of(REFRESH_TICKS) {
            let p = sim.players()[i];
            let config = sim.config();
            let deltas = fatigue_deltas(p.energy, &config.fatigue, &config.tuning);
            assert_eq!(p.deltas, deltas);
            assert_eq!(p.derived, derived_at(&sim, i, deltas));
            checked_105 = true;
        }
        assert!(sim.tick() < 200 * TICKS_PER_MINUTE, "minute 106 never came");
    }
    assert!(checked_105, "no refreshed tick at minute 105");
    assert!(sim.summary().extra_time, "the match went to extra time");
    let steps: Vec<f64> = energy.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(
        steps.iter().all(|&d| d <= 0.0),
        "energy rose: {:?}",
        steps.iter().copied().fold(f64::MIN, f64::max)
    );
    let ninety = at_ninety.expect("the clock passed minute 90") - 1;
    let largest_elsewhere = steps
        .iter()
        .enumerate()
        .filter(|(k, _)| *k != ninety)
        .map(|(_, d)| d.abs())
        .fold(0.0, f64::max);
    assert!(
        steps[ninety].abs() <= largest_elsewhere,
        "a step of {} at minute 90 against at most {largest_elsewhere} elsewhere",
        steps[ninety].abs()
    );
    assert!(sim.players()[i].energy < energy[0]);
}

#[test]
fn effective_values_through_the_modifiers_follow_the_fatigue_curve_bit_for_bit() {
    let config = calm_match(90);
    for energy in [1.0, 0.9, 0.7, 0.6, 0.5, 0.45, 0.3, 0.1, 0.0] {
        let sim = at_energy(energy);
        let i = index(0, PLAYER);
        let p = sim.players()[i];
        let deltas = fatigue_deltas(energy, &config.fatigue, &config.tuning);
        let expected = derived_at(&sim, i, deltas);
        let bits = |d: &Derived| {
            [d.max_speed, d.max_accel, d.turn, d.reach_m]
                .into_iter()
                .map(f64::to_bits)
                .collect::<Vec<_>>()
        };
        assert_eq!(bits(&p.derived), bits(&expected), "energy {energy}");
    }
}

#[test]
fn fatigue_modifier_off_leaves_the_base_values() {
    let mut config = calm_match(90);
    let mut slots = SlotFile::builtin_default();
    slots.slots.insert(
        "engine.modifier.fatigue".to_string(),
        SlotEntry {
            module: "off".to_string(),
            version: None,
        },
    );
    config.modules = resolve(&slots, REGISTRY).unwrap();
    let sim = Scene::new(config).energy(index(0, PLAYER), 0.3).build();
    let p = sim.players()[index(0, PLAYER)];
    assert_eq!(p.energy, 0.3);
    assert_eq!(p.deltas, [0; GROUP_COUNT]);
    assert_eq!(&p.derived, sim.base(index(0, PLAYER)));
}

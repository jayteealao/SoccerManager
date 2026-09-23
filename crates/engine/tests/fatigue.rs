//! AC-4: above the fatigue threshold a player's effective values equal the base; at energy
//! 0.5 and 0.3 the effective top speed and decisions are the base times the curve's
//! multiplier; a 90-minute match ends with every player's energy below 1.0 and above 0.0.

mod common;

use common::{calm_match, full_match, index};
use engine::Simulation;
use engine::fatigue::multiplier;
use engine::record::NullSink;
use engine::scenario::Scene;

const PLAYER: usize = 7;

fn at_energy(energy: f64) -> Simulation {
    Scene::new(calm_match(90))
        .energy(index(0, PLAYER), energy)
        .build()
}

#[test]
fn above_the_threshold_the_effective_values_equal_the_base() {
    for energy in [1.0, 0.9, 0.7] {
        let sim = at_energy(energy);
        let p = sim.players()[index(0, PLAYER)];
        assert_eq!(p.derived, p.base, "energy {energy}");
    }
}

#[test]
fn below_the_threshold_the_curve_scales_the_effective_values() {
    let f = calm_match(90).fatigue;
    // The shipped curve passes through these points.
    for (energy, expected) in [(0.5, 0.92), (0.3, 0.82)] {
        let m = multiplier(energy, &f);
        assert!((m - expected).abs() < 1e-12, "energy {energy}: {m}");
        let sim = at_energy(energy);
        let p = sim.players()[index(0, PLAYER)];
        for (name, effective, base) in [
            ("max_speed", p.derived.max_speed, p.base.max_speed),
            ("decisions", p.derived.decisions, p.base.decisions),
            ("passing", p.derived.passing, p.base.passing),
            ("finishing", p.derived.finishing, p.base.finishing),
        ] {
            assert!(
                (effective - base * m).abs() < 1e-9,
                "energy {energy} {name}: {effective} against {base} x {m}"
            );
        }
        assert_eq!(
            p.derived.tackling, p.base.tackling,
            "tackling is not scaled"
        );
        assert!(p.max_speed() < p.base.max_speed);
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
            let m = multiplier(p.energy, &sim.config().fatigue);
            assert!((p.derived.max_speed - p.base.max_speed * m).abs() < 1e-9);
            assert!((p.derived.decisions - p.base.decisions * m).abs() < 1e-9);
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

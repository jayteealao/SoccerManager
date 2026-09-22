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

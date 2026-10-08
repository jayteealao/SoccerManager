//! States move effective ability only within their caps (shape AC-14): each family moves only
//! the attribute groups it owns, stays within its cap and the total, and every effective
//! rating stays inside 1.0 to 20.0; a planted state past its cap is clamped; a fresh player
//! with no inputs plays from his base values bit for bit.

mod common;

use engine::contract::states::{self, GROUP_COUNT, GROUPS, StatesTuning};
use engine::data::Group;
use engine::data::team::Condition;
use engine::modules::modifier::{FAMILY_COUNT, Family};
use engine::{MatchConfig, Simulation};

const TECHNICAL: usize = 0;
const MENTAL: usize = 1;
const PHYSICAL: usize = 2;
const GOALKEEPING: usize = 3;
/// One check a second.
const EVERY: u32 = 50;

/// The seed-42 match of the default teams, the home side given no sharpness and no
/// adaptation.
fn unsharp_unadapted_home() -> Simulation {
    let content = common::content();
    let [mut a, b] = common::default_teams(&content);
    for p in &mut a.players {
        p.condition = Some(Condition {
            sharpness: Some(0),
            adaptation: Some(0),
            ..Condition::default()
        });
    }
    Simulation::new(MatchConfig::new(common::SEED, 90, &content, [&a, &b]).unwrap()).unwrap()
}

#[test]
fn every_state_stays_within_its_cap_and_its_groups_for_a_whole_match() {
    let mut sim = unsharp_unadapted_home();
    let caps = &sim.tuning().contract.states.caps;
    let (body, mind, total) = (caps.body, caps.mind, caps.total);
    let tenths = |x: f64| (x * 10.0).round() as i8;
    let mut lowest_technical = 0i8;
    let mut lowest_physical = 0i8;
    let mut checks = 0;
    while !sim.is_over() {
        if sim.tick().is_multiple_of(EVERY) {
            checks += 1;
            for (i, p) in sim.players().iter().enumerate() {
                let d = p.deltas;
                for &x in &d {
                    assert!(
                        (tenths(total[0])..=tenths(total[1])).contains(&x),
                        "player {i} tick {}: {d:?} past the total",
                        sim.tick()
                    );
                }
                // The body family (fatigue, sharpness) owns technical, physical and
                // goalkeeping; nothing in this match raises a rating, and no body delta
                // passes the body cap even with fatigue and sharpness pushing together.
                for g in [TECHNICAL, PHYSICAL, GOALKEEPING] {
                    assert!(
                        (tenths(body[0])..=0).contains(&d[g]),
                        "player {i} tick {}: group {g} {d:?} past the body cap",
                        sim.tick()
                    );
                }
                if p.team == 0 {
                    // Adaptation 0 is the mind family at its full drop, and only it moves
                    // the mental group.
                    assert_eq!(d[MENTAL], tenths((-1.0f64).max(mind[0])), "player {i}");
                    // Sharpness 0 alone drops technical by 1.5; fatigue adds to it.
                    assert!(d[TECHNICAL] <= -15, "player {i} tick {}: {d:?}", sim.tick());
                    lowest_technical = lowest_technical.min(d[TECHNICAL]);
                } else {
                    assert_eq!(d[MENTAL], 0, "player {i}: fatigue does not move mental");
                }
                lowest_physical = lowest_physical.min(d[PHYSICAL]);
                let ratings = sim.effective_ratings(i);
                for r in &ratings.values[..usize::from(ratings.len)] {
                    assert!((10..=200).contains(&r.tenths()), "player {i}: {r}");
                }
            }
        }
        sim.step();
    }
    assert!(checks > 5000, "only {checks} checks");
    // Fatigue moved the physical group, and added to sharpness on the technical group.
    assert!(lowest_physical < 0, "fatigue never moved a physical rating");
    assert!(
        lowest_technical < -15,
        "fatigue never added to sharpness: {lowest_technical}"
    );
}

#[test]
fn a_fresh_player_with_no_inputs_plays_from_his_base_values_bit_for_bit() {
    // No inputs: no states and no form offset.
    let sim = Simulation::new(common::steady(common::full_match())).unwrap();
    for i in 0..sim.players().len() {
        let p = sim.players()[i];
        assert_eq!(p.deltas, [0; GROUP_COUNT], "player {i}");
        assert_eq!(&p.derived, sim.base(i), "player {i}");
        assert_eq!(sim.skills(i).stages, sim.base_stages(i), "player {i}");
        assert_eq!(
            sim.effective_ratings(i).values,
            p.attributes.values,
            "player {i}"
        );
    }
}

#[test]
fn a_planted_state_past_its_cap_is_clamped_and_moves_only_its_groups() {
    let t = StatesTuning::default();
    let families = [
        Family::Body,
        Family::Mind,
        Family::Familiarity,
        Family::Surroundings,
    ];
    for family in families {
        let mut per_family = [[0.0; GROUP_COUNT]; FAMILY_COUNT];
        per_family[family as usize] = [-9.0; GROUP_COUNT];
        let d = states::capped(per_family, &t);
        let lo = (t.caps.of(family)[0] * 10.0).round() as i8;
        for (g, group) in GROUPS.iter().enumerate() {
            let expect = if t.family_groups.owns(family, *group) {
                lo
            } else {
                0
            };
            assert_eq!(d[g], expect, "{family:?} {group:?}");
        }
    }
    // Every family at its lowest: no group below the total cap.
    let d = states::capped([[-9.0; GROUP_COUNT]; FAMILY_COUNT], &t);
    assert!(
        d.iter().all(|&x| x >= (t.caps.total[0] * 10.0) as i8),
        "{d:?}"
    );
    // The shipped groups: sharpness (body) never reaches mental, adaptation (mind) never
    // reaches the others.
    assert!(!t.family_groups.owns(Family::Body, Group::Mental));
    assert!(t.family_groups.owns(Family::Body, Group::Technical));
    assert!(t.family_groups.owns(Family::Mind, Group::Mental));
    assert!(!t.family_groups.owns(Family::Mind, Group::Physical));
}

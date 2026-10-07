//! An AI-managed side started in a formation fields the lineup its pre-match module picks
//! for that formation's slots, and the default formation keeps every pick and every tick.

mod common;

use engine::ai::{PreMatchOff, pre_match, pre_match_for};
use engine::data::generator::generate_league;
use engine::data::team::Position;
use engine::modules::PreMatchModule;
use engine::team::{PLAYERS_PER_TEAM, Team};
use engine::{MatchConfig, Simulation, Tactics};

/// The first generated club of the seed-42 league, as a team.
fn generated_team(content: &engine::Content) -> Team {
    let league = generate_league(42, 20, content);
    let (team, _) =
        Team::from_file(0, &league[0], &content.attributes, &content.tuning.engine).unwrap();
    team
}

/// The default tactics switched to `formation`, as a side started in it holds them.
fn tactics_in(formation: usize, content: &engine::Content) -> Tactics {
    let mut tactics = Tactics::defaults(&content.tactics);
    tactics.set_formation(formation as u8, &content.tactics);
    tactics
}

#[test]
fn every_formation_slot_gets_a_suited_player_when_the_squad_holds_one() {
    let content = common::content();
    let schema = &content.tactics;
    let team = generated_team(&content);
    for (f, formation) in schema.formations.iter().enumerate() {
        let tactics = tactics_in(f, &content);
        let setup = pre_match_for(&team, tactics, schema, &content.attributes);
        let mut used: Vec<usize> = Vec::new();
        for slot in 0..PLAYERS_PER_TEAM {
            let suits = &schema.roles[usize::from(setup.tactics.roles[slot].role)].positions;
            let picked = setup.lineup[slot];
            // A player of a suited position was free when the slot was filled.
            let free_suited = (0..team.squad.len())
                .any(|s| !used.contains(&s) && suits.contains(&team.squad[s].position));
            if free_suited {
                assert!(
                    suits.contains(&team.squad[picked].position),
                    "{} slot {slot} ({:?}) got a {:?}",
                    formation.name,
                    formation.slots[slot].position,
                    team.squad[picked].position
                );
            }
            used.push(picked);
        }
        let mut distinct = setup.lineup.to_vec();
        distinct.extend(&setup.bench);
        let n = distinct.len();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            n,
            "{}: a player is named twice",
            formation.name
        );
        assert_eq!(setup.tactics, tactics, "the tactics are the ones given");
    }
}

#[test]
fn a_back_three_puts_a_full_back_at_wing_back_not_a_winger() {
    let content = common::content();
    let schema = &content.tactics;
    let team = generated_team(&content);
    let f = schema.formation_index("3-5-2").expect("3-5-2 ships");
    let setup = pre_match_for(&team, tactics_in(f, &content), schema, &content.attributes);
    // Slot 8 of 3-5-2 is the right-back slot; the 4-4-2 lineup put the right winger there.
    assert_eq!(schema.formations[f].slots[8].position, Position::RB);
    assert_eq!(team.squad[setup.lineup[8]].position, Position::RB);
}

#[test]
fn the_default_tactics_give_the_same_setup_as_the_default_pick() {
    let content = common::content();
    let team = generated_team(&content);
    for file in common::default_teams(&content) {
        let (default_team, _) =
            Team::from_file(0, &file, &content.attributes, &content.tuning.engine).unwrap();
        for t in [&team, &default_team] {
            assert_eq!(
                pre_match_for(
                    t,
                    Tactics::defaults(&content.tactics),
                    &content.tactics,
                    &content.attributes
                ),
                pre_match(t, &content.tactics, &content.attributes)
            );
        }
    }
}

#[test]
fn an_ai_side_started_in_the_default_tactics_plays_the_same_match_tick_for_tick() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let plain = MatchConfig::new(common::SEED, 90, &content, [&a, &b]).unwrap();
    let started = MatchConfig::new(common::SEED, 90, &content, [&a, &b])
        .unwrap()
        .with_ai_tactics(0, Tactics::defaults(&content.tactics))
        .with_ai_tactics(1, Tactics::defaults(&content.tactics));
    let mut one = Simulation::new(plain).unwrap();
    let mut two = Simulation::new(started).unwrap();
    for tick in 0..3_000 {
        one.step();
        two.step();
        assert_eq!(one.record(), two.record(), "tick {tick} differs");
    }
}

#[test]
fn the_off_version_keeps_file_order_with_the_given_tactics() {
    let content = common::content();
    let team = generated_team(&content);
    let f = content
        .tactics
        .formation_index("4-3-3")
        .expect("4-3-3 ships");
    let tactics = tactics_in(f, &content);
    let off = PreMatchOff.setup_for(&team, tactics, &content.tactics, &content.attributes);
    assert_eq!(off.lineup, core::array::from_fn(|slot| slot));
    assert_eq!(off.tactics, tactics);
    assert!(off.bench.iter().all(|s| !off.lineup.contains(s)));
}

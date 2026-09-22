//! AC-5: an injury on a tackle takes the player out of play on that tick, with an injury
//! event and an injury stoppage; the AI manager queues a substitution that applies at that
//! stoppage. With the substitution limit used, the team plays on with ten and the rejection
//! names the reason.

mod common;

use common::{index, kinds, quiet_match, spread};
use engine::math::{DVec2, DVec3};
use engine::player::Status;
use engine::scenario::Scene;
use engine::{
    AiCode, EngineEvent, EngineEventKind, EventDetail, InjurySource, Manager, MatchConfig,
    RejectReason, Simulation, StoppageKind,
};

/// The home carrier, and the away player who tackles.
const CARRIER: usize = 5;
const TACKLER: usize = 16;

/// A draw inside the clean-win band of the tackle in these scenes.
fn clean_win(config: &MatchConfig) -> f64 {
    let sim = Simulation::new(config.clone()).unwrap();
    let tackler = sim.players()[TACKLER].derived;
    let carrier = sim.players()[CARRIER].derived;
    0.5 * 0.05 * tackler.tackling / (tackler.tackling + carrier.dribbling)
}

/// The away player wins the ball cleanly from the home carrier in open play, and the first
/// injury draw hurts the carrier. The home side is managed by the AI with `used`
/// substitutions made over `windows` windows.
fn tackle(used: u8, windows: u8) -> (Simulation, Vec<EngineEvent>) {
    let mut config = quiet_match(90);
    config.tuning.injury_per_minute = 0.0;
    let at = DVec2::new(0.0, 10.0);
    let mut sim = spread(Scene::new(config.clone()), -30.0, 30.0)
        .manager(0, Manager::Ai)
        .manager(1, Manager::Human)
        .place(CARRIER, at)
        .place(TACKLER, at + DVec2::new(0.3, 0.0))
        .ball(DVec3::new(at.x, at.y, 0.0))
        .carrier(Some(CARRIER))
        .tick(1_001)
        .rolls(&[clean_win(&config)])
        .injury_rolls(&[0.0])
        .subs_used(0, used, windows)
        .build();
    sim.step();
    let events = sim.take_events();
    (sim, events)
}

#[test]
fn an_injured_player_leaves_play_and_the_ai_replaces_him_at_that_stoppage() {
    let (off, on) = {
        let sim = Simulation::new(quiet_match(90)).unwrap();
        (sim.teams()[0].lineup[CARRIER], sim.teams()[0].bench.clone())
    };
    let (sim, events) = tackle(0, 0);
    let injury = events
        .iter()
        .find(|e| e.kind == EngineEventKind::Injury)
        .unwrap_or_else(|| panic!("an injury event: {:?}", kinds(&events)));
    assert_eq!(injury.team, Some(0));
    assert_eq!(injury.player, Some(CARRIER));
    assert_eq!(
        injury.detail,
        Some(EventDetail::Injury {
            source: InjurySource::Tackle
        })
    );
    let stoppage = sim.stoppage().expect("the injury stops play on its tick");
    assert_eq!(stoppage.kind, StoppageKind::Injury);
    assert_eq!(sim.dead_ball().unwrap().kind, StoppageKind::Injury);
    // The AI chose a substitute, and it applied at the same stoppage.
    let ai = events
        .iter()
        .find(|e| e.kind == EngineEventKind::AiDecision)
        .expect("the AI manager decided");
    assert_eq!(
        ai.detail,
        Some(EventDetail::Ai {
            code: AiCode::SubInjury
        })
    );
    let applied = events
        .iter()
        .find(|e| e.kind == EngineEventKind::ChangeApplied)
        .unwrap_or_else(|| panic!("the substitution applied: {:?}", kinds(&events)));
    assert_eq!(applied.tick, injury.tick, "applied at the injury stoppage");
    let sub = events
        .iter()
        .find(|e| e.kind == EngineEventKind::Substitution)
        .expect("a substitution event");
    let Some(EventDetail::Substitution {
        off: left,
        on: came,
    }) = sub.detail
    else {
        panic!("{sub:?}");
    };
    assert_eq!(left, off);
    assert!(on.contains(&came), "the substitute came from the bench");
    let p = sim.players()[index(0, CARRIER)];
    assert_eq!(p.squad, came);
    assert_eq!(p.status, Status::OnPitch);
    let on_pitch = (0..11)
        .filter(|&s| sim.players()[index(0, s)].active())
        .count();
    assert_eq!(on_pitch, 11, "the team is back to eleven");
    assert_eq!(sim.summary().injuries, [1, 0]);
    assert!(sim.pending_changes().is_empty());
}

#[test]
fn with_the_limit_used_the_team_plays_on_with_ten() {
    let (sim, events) = tackle(5, 3);
    assert!(
        events.iter().any(|e| e.kind == EngineEventKind::Injury),
        "{:?}",
        kinds(&events)
    );
    let rejected = events
        .iter()
        .find(|e| e.kind == EngineEventKind::ChangeRejected)
        .unwrap_or_else(|| panic!("a rejection: {:?}", kinds(&events)));
    let Some(EventDetail::Change {
        reason: Some(reason),
        ..
    }) = rejected.detail
    else {
        panic!("{rejected:?}");
    };
    assert_eq!(reason, RejectReason::LimitReached { limit: 5 });
    assert_eq!(
        reason.text(&sim.teams()[0].player_ids),
        "substitution limit reached (5 of 5)"
    );
    assert_eq!(sim.players()[index(0, CARRIER)].status, Status::Injured);
    let on_pitch = (0..11)
        .filter(|&s| sim.players()[index(0, s)].active())
        .count();
    assert_eq!(on_pitch, 10, "the team plays on with ten");
    assert!(!sim.abandoned());
}

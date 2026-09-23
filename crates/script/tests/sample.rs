//! The shipped sample pack: each of its three hooks does what its comments say.

use std::path::Path;

use engine::Card;
use engine::plugin::{DecisionContext, FoulContext, HookOutcome, LineContext, OptionOffsets};
use script::LoadedPack;

fn sample() -> engine::Plugins {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/scripts/sample");
    LoadedPack::load(&dir).unwrap().plugins()
}

#[test]
fn a_trailing_carrier_near_goal_under_pressure_shoots_more_and_dribbles_less() {
    let mut plugins = sample();
    let hook = plugins.decision.as_mut().unwrap();
    let mut ctx = DecisionContext {
        tick: 210_000,
        minute: 70,
        team: 0,
        slot: 9,
        goals_for: 0,
        goals_against: 1,
        goal_distance: 20.0,
        nearest_opponent: 2.0,
        progress: 0.6,
    };
    assert_eq!(
        hook.adjust(&ctx),
        HookOutcome::Value(OptionOffsets {
            shoot: 0.4,
            dribble: -0.3,
            ..OptionOffsets::default()
        })
    );
    ctx.goals_against = 0;
    ctx.nearest_opponent = 8.0;
    assert_eq!(
        hook.adjust(&ctx),
        HookOutcome::Value(OptionOffsets::default())
    );
}

#[test]
fn an_early_first_caution_becomes_no_card_and_later_cards_stand() {
    let mut plugins = sample();
    let hook = plugins.rule.as_mut().unwrap();
    let mut ctx = FoulContext {
        tick: 12_000,
        minute: 4,
        team: 1,
        slot: 4,
        yellows: 0,
        aggression: 0.6,
        advantage: false,
        penalty: false,
    };
    assert_eq!(
        hook.card(&ctx, Some(Card::Yellow)),
        HookOutcome::Value(None)
    );
    assert_eq!(hook.card(&ctx, Some(Card::Red)), HookOutcome::Keep);
    ctx.minute = 30;
    assert_eq!(hook.card(&ctx, Some(Card::Yellow)), HookOutcome::Keep);
}

#[test]
fn a_goal_line_gains_its_time_of_the_match() {
    let mut plugins = sample();
    let hook = plugins.commentary.as_mut().unwrap();
    let mut ctx = LineContext {
        tick: 240_000,
        minute: 80,
        kind: "goal",
        team: Some(0),
        scores: [1, 0],
    };
    assert_eq!(
        hook.line(&ctx, "Kane scores."),
        HookOutcome::Value("Late goal! Kane scores.".into())
    );
    ctx.kind = "corner";
    assert_eq!(hook.line(&ctx, "A corner."), HookOutcome::Keep);
}

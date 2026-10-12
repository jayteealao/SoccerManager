//! The named styles and the AI's pick of one before a match.
//!
//! - AC-33: the shipped file holds eight styles, each with a mentality, instruction levels,
//!   duty hints, a non-empty formation list and 2 or 3 identity statistics; each style has a
//!   signature attribute it weighs more than any other style does.
//! - AC-36: a squad built for each style is picked for it in more than 25 of 50 seeds.
//! - AC-37: with the tactics set by hand, two matches whose advice differs play identically,
//!   tick for tick: play never reads an estimate.
//! - A person's side keeps the default tactics; content without styles, or with the style pick
//!   flag off as shipped, plays them for both.

mod common;

use std::hash::{DefaultHasher, Hasher};

use engine::advice::{advise, style_estimates};
use engine::data::Position;
use engine::data::styles::Styles;
use engine::gate::StateWriter;
use engine::team::Team;
use engine::{Content, Manager, MatchConfig, Simulation, Tactics};

fn styles(content: &Content) -> &Styles {
    content
        .styles
        .as_ref()
        .expect("the shipped content has styles")
}

/// The attributes `style` weighs more than every other style does.
fn signature(all: &Styles, style: usize) -> Vec<usize> {
    let weight = |s: usize, k: usize| {
        all.styles[s]
            .fit
            .iter()
            .find(|(a, _)| *a == k)
            .map_or(0.0, |(_, w)| *w)
    };
    all.styles[style]
        .fit
        .iter()
        .map(|(k, _)| *k)
        .filter(|&k| (0..all.styles.len()).all(|t| t == style || weight(style, k) > weight(t, k)))
        .collect()
}

#[test]
fn the_shipped_file_holds_eight_complete_styles_each_with_a_signature() {
    let content = common::content();
    let all = styles(&content);
    assert_eq!(all.styles.len(), 8);
    let names: Vec<&str> = all.styles.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "possession",
            "low_block",
            "high_press",
            "counter_attack",
            "direct",
            "wing_play",
            "gegenpress",
            "park_the_bus"
        ]
    );
    for (k, s) in all.styles.iter().enumerate() {
        assert!(usize::from(s.mentality) < content.tactics.mentalities.len());
        assert!(s.instructions.iter().any(Option::is_some), "{}", s.name);
        assert!(s.duties.iter().any(Option::is_some), "{}", s.name);
        assert!(!s.formations.is_empty(), "{}", s.name);
        assert!((2..=3).contains(&s.identity.len()), "{}", s.name);
        assert!(
            !signature(all, k).is_empty(),
            "{} has no signature attribute",
            s.name
        );
    }
}

/// `default-a` with each signature attribute of `style` raised by 5 for every player listed at
/// one of the positions the style reads.
fn built_for(content: &Content, style: usize) -> Team {
    let all = styles(content);
    let [mut file, _] = common::default_teams(content);
    let s = &all.styles[style];
    let raised: Vec<&str> = signature(all, style)
        .into_iter()
        .map(|k| content.attributes.attributes[k].name.as_str())
        .collect();
    for p in &mut file.players {
        if !s.fit_positions.contains(&p.position) {
            continue;
        }
        for name in &raised {
            let v = p.attributes.get_mut(*name).expect("an outfield attribute");
            *v = engine::Rating::from_tenths((v.tenths() + 50).min(200));
        }
    }
    Team::from_file(0, &file, &content.attributes, &content.tuning.engine)
        .unwrap()
        .0
}

/// AC-36: the AI picks the style a squad was built for in more than 25 of 50 seeds.
#[test]
fn a_squad_built_for_a_style_is_picked_for_it_more_often_than_not() {
    let content = common::content();
    let all = styles(&content);
    let mut report = Vec::new();
    for (k, style) in all.styles.iter().enumerate() {
        let team = built_for(&content, k);
        let picked = (1..=50)
            .filter(|&seed| {
                let t = advise(&team, all, &content.tactics, &content.attributes, seed, 0)
                    .expect("a style is picked");
                t == style.tactics(t.formation, &content.tactics)
                    && style.formations.contains(&t.formation)
            })
            .count();
        report.push(format!("{} {picked}/50", style.name));
        assert!(
            picked > 25,
            "{}: picked {picked} of 50; {report:?}",
            style.name
        );
    }
    println!("{report:?}");
}

/// Every tick's gate state bytes and the match's events, hashed.
fn played(config: MatchConfig) -> (u64, String) {
    let mut sim = Simulation::new(config).unwrap();
    let mut writer = StateWriter::new(false);
    let mut hasher = DefaultHasher::new();
    let mut events = Vec::new();
    while !sim.is_over() {
        sim.step();
        let tick = sim.take_events();
        hasher.write(writer.tick(&sim, &tick));
        events.extend(tick);
    }
    (hasher.finish(), format!("{events:?}"))
}

/// AC-37: two contents that differ only in the advice's error give different estimates; with
/// both sides' tactics set by hand the two matches are identical tick for tick.
#[test]
fn play_never_reads_the_estimate() {
    let content = common::content();
    let mut noisy = content.clone();
    noisy.styles.as_mut().unwrap().advice_noise = 3.0;
    let [a, b] = common::default_teams(&content);
    let (team, _) = Team::from_file(0, &a, &content.attributes, &content.tuning.engine).unwrap();
    let quiet = style_estimates(
        &team,
        styles(&content),
        &content.tactics,
        &content.attributes,
        7,
        0,
    );
    let loud = style_estimates(
        &team,
        styles(&noisy),
        &noisy.tactics,
        &noisy.attributes,
        7,
        0,
    );
    assert_ne!(quiet, loud, "the estimates differ");
    let all = styles(&content);
    let home = all.styles[2].tactics(all.styles[2].formations[0], &content.tactics);
    let away = all.styles[5].tactics(all.styles[5].formations[0], &content.tactics);
    let by_ai = |c: &Content| {
        MatchConfig::new(7, 30, c, [&a, &b])
            .unwrap()
            .with_ai_tactics(0, home)
            .with_ai_tactics(1, away)
    };
    assert_eq!(played(by_ai(&content)), played(by_ai(&noisy)));
    let by_hand = |c: &Content| {
        let config = MatchConfig::new(7, 30, c, [&a, &b]).unwrap();
        let lineups = [config.teams[0].lineup, config.teams[1].lineup];
        let benches = [config.teams[0].bench.clone(), config.teams[1].bench.clone()];
        config
            .with_setup(0, lineups[0], benches[0].clone())
            .with_setup(1, lineups[1], benches[1].clone())
            .with_tactics(0, home)
            .with_tactics(1, away)
    };
    assert_eq!(played(by_hand(&content)), played(by_hand(&noisy)));
}

/// An AI side starts in the style the advice picks; a person's side keeps the default tactics;
/// content without styles plays the default tactics for both.
#[test]
fn the_ai_side_starts_in_its_style_and_a_person_keeps_the_defaults() {
    let content = common::content_with_style_pick();
    let [a, b] = common::default_teams(&content);
    let defaults = Tactics::defaults(&content.tactics);
    let config = MatchConfig::new(11, 90, &content, [&a, &b])
        .unwrap()
        .with_manager(0, Manager::Human);
    assert!(!config.takes_advice(0));
    assert!(config.takes_advice(1));
    let sim = Simulation::new(config.clone()).unwrap();
    let teams = sim.teams();
    assert_eq!(teams[0].tactics, defaults);
    let picked = advise(
        &config.teams[1],
        styles(&content),
        &content.tactics,
        &content.attributes,
        11,
        1,
    )
    .unwrap();
    assert_eq!(teams[1].tactics, picked);
    let keeper = &teams[1].squad[teams[1].lineup[teams[1].keeper_slot()]];
    assert_eq!(keeper.position, Position::GK);
    // Without styles, or with the flag off, both sides play the defaults.
    let mut plain = content.clone();
    plain.styles = None;
    for c in [plain, common::content_without_style_pick()] {
        let config = MatchConfig::new(11, 90, &c, [&a, &b]).unwrap();
        assert!(!config.takes_advice(0) && !config.takes_advice(1));
        for team in Simulation::new(config).unwrap().teams() {
            assert_eq!(team.tactics, defaults);
        }
    }
}

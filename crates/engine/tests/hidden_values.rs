//! The hidden values in play: consistency spreads each player's play around his ratings, more
//! for a less consistent player and by nothing on average across a side; injury proneness
//! raises the injury chance; and each hidden value reaches a reader only as a word and a
//! confidence from the matches the player has seen at the club.

mod common;

use engine::contract::hidden::{Confidence, hidden_word};
use engine::data::TeamFile;
use engine::rules::clock::TICKS_PER_MINUTE;
use engine::{EngineEventKind, MatchConfig, Rating, Simulation};

/// Squad indices of the two home players whose consistency the narrowing test sets.
const STEADY: usize = 4;
const ERRATIC: usize = 7;

fn set_all(file: &mut TeamFile, name: &str, decimal: f64) {
    for p in &mut file.players {
        p.attributes
            .insert(name.into(), Rating::from_decimal(decimal).unwrap());
    }
}

/// Every active player's form, by roster index, sampled just after each period redraw of a
/// whole match, home side only.
fn forms_by_period(config: MatchConfig) -> Vec<Vec<(usize, i8)>> {
    let period = config.tuning.contract.consistency.period_minutes * TICKS_PER_MINUTE;
    let mut sim = Simulation::new(config).unwrap();
    let mut out = vec![home_forms(&sim)];
    while !sim.is_over() {
        sim.step();
        if sim.tick().is_multiple_of(period) {
            out.push(home_forms(&sim));
        }
    }
    out
}

fn home_forms(sim: &Simulation) -> Vec<(usize, i8)> {
    sim.players()
        .iter()
        .filter(|p| p.team == 0 && p.active())
        .map(|p| (p.squad, p.form))
        .collect()
}

fn spread_of(values: &[f64]) -> f64 {
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    (values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n).sqrt()
}

/// (a) A player of consistency 18 swings less from period to period and match to match than
/// one of consistency 4 on the same side; a side wholly at 20.0 never moves.
#[test]
fn consistency_narrows_a_players_form() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let mut home = a.clone();
    home.players[STEADY]
        .attributes
        .insert("consistency".into(), Rating::from_decimal(18.0).unwrap());
    home.players[ERRATIC]
        .attributes
        .insert("consistency".into(), Rating::from_decimal(4.0).unwrap());
    let runs = common::run_many(1..=30, |seed| {
        forms_by_period(MatchConfig::new(seed, 90, &content, [&home, &b]).unwrap())
    });
    let mut steady = Vec::new();
    let mut erratic = Vec::new();
    for periods in &runs {
        for forms in periods {
            for &(squad, form) in forms {
                if squad == STEADY {
                    steady.push(f64::from(form));
                } else if squad == ERRATIC {
                    erratic.push(f64::from(form));
                }
            }
        }
    }
    assert!(steady.len() > 100 && erratic.len() > 100);
    let (s, e) = (spread_of(&steady), spread_of(&erratic));
    assert!(
        s < e,
        "consistency 18 spreads {s:.2} tenths, consistency 4 {e:.2}"
    );

    let mut flawless = a.clone();
    set_all(&mut flawless, "consistency", 20.0);
    for periods in forms_by_period(MatchConfig::new(3, 90, &content, [&flawless, &b]).unwrap()) {
        assert!(periods.iter().all(|&(_, form)| form == 0), "{periods:?}");
    }
}

/// (b) Each part of the form is centred over the side, so the side's mean offset stays within
/// 0.05 of a rating point (half a tenth, from rounding each player's offset) at every period
/// while the side is the one that kicked off. A substitute draws his match part alone as he
/// comes on, so after a change the mean moves by his part over the side.
#[test]
fn a_sides_mean_form_stays_at_zero() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    for seed in [1, 2, 3] {
        let config = MatchConfig::new(seed, 90, &content, [&a, &b]).unwrap();
        let periods = forms_by_period(config);
        let eleven: Vec<usize> = periods[0].iter().map(|&(s, _)| s).collect();
        let unchanged: Vec<_> = periods
            .iter()
            .take_while(|forms| forms.iter().map(|&(s, _)| s).eq(eleven.iter().copied()))
            .collect();
        assert!(
            unchanged.len() >= 3,
            "seed {seed}: {} periods",
            unchanged.len()
        );
        for forms in unchanged {
            let mean = forms.iter().map(|&(_, f)| f64::from(f)).sum::<f64>() / forms.len() as f64;
            assert!(
                mean.abs() <= 0.5 + 1e-9,
                "seed {seed}: mean {mean:.2} tenths"
            );
            assert!(
                forms.iter().any(|&(_, f)| f != 0),
                "seed {seed}: nobody moved"
            );
        }
    }
}

/// (c) Over seeded matches, a side of injury-prone players (16.0) is injured no less often than
/// a side of rarely injured ones (4.0).
#[test]
fn injury_prone_players_are_injured_no_less_often() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let injuries = |proneness: f64| {
        let mut home = a.clone();
        let mut away = b.clone();
        set_all(&mut home, "injury_proneness", proneness);
        set_all(&mut away, "injury_proneness", proneness);
        common::run_many(1..=24, |seed| {
            let mut config = MatchConfig::new(seed, 90, &content, [&home, &away]).unwrap();
            // Raised rates, so a batch of matches holds enough injuries to compare.
            config.tuning.injury_per_minute *= 20.0;
            config.tuning.injury_per_tackle *= 20.0;
            let mut sim = Simulation::new(config).unwrap();
            let mut n = 0u32;
            while !sim.is_over() {
                sim.step();
                n += sim
                    .take_events()
                    .iter()
                    .filter(|e| e.kind == EngineEventKind::Injury)
                    .count() as u32;
            }
            n
        })
        .into_iter()
        .sum::<u32>()
    };
    let (prone, hardy) = (injuries(16.0), injuries(4.0));
    assert!(hardy > 0, "no injury at all");
    assert!(prone >= hardy, "prone {prone} injuries, hardy {hardy}");
}

/// (d) Through a team file: a player with no matches at the club shows no word, 5 matches a
/// tentative word, 40 a firm one; the word follows the rating's band.
#[test]
fn the_words_follow_the_matches_seen_at_the_club() {
    let content = common::content();
    let bytes = std::fs::read(common::fixture_path("teams/condition-good.json")).unwrap();
    let mut raw: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for (i, (matches, consistency, proneness)) in
        [(0u16, 16.0, 3.0), (5, 12.0, 8.0), (40, 6.5, 15.0)]
            .into_iter()
            .enumerate()
    {
        let p = &mut raw["players"][i];
        p["condition"] = serde_json::json!({ "matches_at_club": matches });
        p["attributes"]["consistency"] = consistency.into();
        p["attributes"]["injury_proneness"] = proneness.into();
    }
    let file = content
        .team_from_bytes(&serde_json::to_vec(&raw).unwrap(), "words.json")
        .unwrap()
        .value;
    let t = &content.tuning.hidden;
    let say = |i: usize, name: &str| {
        let p = &file.players[i];
        let matches = p.condition.as_ref().and_then(|c| c.matches_at_club);
        hidden_word(name, p.attributes[name].decimal(), matches, t)
    };
    let none = say(0, "consistency");
    assert_eq!(
        (none.word, none.confidence),
        (None, Confidence::NotYetKnown)
    );
    assert_eq!(say(0, "injury_proneness").word, None);

    let some = say(1, "consistency");
    assert_eq!(some.confidence, Confidence::Tentative);
    assert_eq!(some.word.as_deref(), Some("steady"));
    assert_eq!(
        say(1, "injury_proneness").word.as_deref(),
        Some("rarely_injured")
    );

    let many = say(2, "consistency");
    assert_eq!(many.confidence, Confidence::Firm);
    assert_eq!(many.word.as_deref(), Some("erratic"));
    assert_eq!(
        say(2, "injury_proneness").word.as_deref(),
        Some("injury_prone")
    );
}

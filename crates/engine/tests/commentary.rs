//! Commentary criteria: every event kind gets a line naming its player and club, five
//! same-kind events in ten minutes give at least three distinct lines, a late equaliser and a
//! rout read as such, and a full seeded match leaves no placeholder unfilled. Also the
//! loader's refusals.

mod common;

use engine::commentary::context::Length;
use engine::commentary::templates::{MinuteBand, ScoreState, commented};
use engine::commentary::{Commentary, Commentator, MatchNames};
use engine::data::COMMENTARY_FILE;
use engine::{AiCode, Card, EngineEvent, EngineEventKind, EventDetail, Simulation};

const NINETY: Length = Length {
    minutes: 90,
    halves: 2,
};

fn shipped() -> Commentary {
    Commentary::load(&common::content_dir()).expect("the shipped commentary loads")
}

fn names() -> MatchNames {
    MatchNames::of(&Simulation::new(common::full_match()).unwrap())
}

/// An event of `kind` for the home club in `minute`, naming roster slot 3.
fn scripted(kind: EngineEventKind, minute: u32) -> EngineEvent {
    let whistle = matches!(kind, EngineEventKind::HalfTime | EngineEventKind::FullTime);
    EngineEvent {
        tick: minute * 3_000 + 1,
        kind,
        team: (!whistle).then_some(0),
        scores: [0, 0],
        minute,
        minute_added: None,
        player: (!whistle && kind != EngineEventKind::AiDecision).then_some(3),
        secondary: (kind == EngineEventKind::Foul).then_some(14),
        card: (kind == EngineEventKind::Card).then_some(Card::Yellow),
        advantage: (kind == EngineEventKind::Foul).then_some(false),
        added_time_s: None,
        spot: None,
        detail: match kind {
            EngineEventKind::Substitution => Some(EventDetail::Substitution { off: 3, on: 15 }),
            EngineEventKind::AiDecision => Some(EventDetail::Ai {
                code: AiCode::SubFatigue,
            }),
            _ => None,
        },
        period: None,
        shootout_round: None,
        shootout_scored: None,
        shootout_scores: None,
        decided_by: None,
    }
}

/// Checks that `line` names the player and the club `event` names.
fn names_its_player_and_club(
    line: &str,
    event: &EngineEvent,
    names: &MatchNames,
    roster: &[String],
) {
    match event.team {
        Some(team) => assert!(
            line.contains(&names.clubs[team]),
            "{line:?} does not name {} ({:?})",
            names.clubs[team],
            event.kind
        ),
        None => {
            for club in &names.clubs {
                assert!(
                    line.contains(club.as_str()),
                    "{line:?} does not name {club}"
                );
            }
        }
    }
    let player = match (event.detail, event.team, event.player) {
        (Some(EventDetail::Substitution { off, .. }), Some(team), _) => {
            Some(names.squads[team][off].clone())
        }
        (_, _, Some(p)) => Some(roster[p].clone()),
        _ => None,
    };
    if let Some(player) = player {
        assert!(
            line.contains(&player),
            "{line:?} does not name {player} ({:?})",
            event.kind
        );
    }
}

#[test]
fn every_event_kind_gets_a_line_naming_its_player_and_club() {
    let commentary = shipped();
    let names = names();
    for kind in engine::EngineEventKind::ALL {
        let mut c = Commentator::new(&commentary, names.clone(), NINETY, common::SEED);
        let event = scripted(kind, 30);
        let Some(line) = c.line(&event) else {
            assert!(!commented(kind), "{kind:?} got no line");
            continue;
        };
        names_its_player_and_club(&line, &event, &names, &names.roster);
        assert_eq!(c.fallbacks(), 0, "{kind:?}");
    }
    // Every card and every AI choice.
    for card in [Card::Yellow, Card::SecondYellow, Card::Red] {
        let mut c = Commentator::new(&commentary, names.clone(), NINETY, common::SEED);
        let mut event = scripted(EngineEventKind::Card, 30);
        event.card = Some(card);
        let line = c.line(&event).unwrap();
        names_its_player_and_club(&line, &event, &names, &names.roster);
    }
    for code in [
        AiCode::MentalityUpTrailing,
        AiCode::MentalityDownLeading,
        AiCode::SubInjury,
        AiCode::SubFatigue,
        AiCode::SubKeeper,
    ] {
        let mut c = Commentator::new(&commentary, names.clone(), NINETY, common::SEED);
        let mut event = scripted(EngineEventKind::AiDecision, 75);
        event.detail = Some(EventDetail::Ai { code });
        let line = c.line(&event).unwrap();
        names_its_player_and_club(&line, &event, &names, &names.roster);
    }
}

#[test]
fn every_shipped_line_names_the_player_and_the_club() {
    for set in shipped().sets {
        let need: &[&str] = match set.kind {
            EngineEventKind::HalfTime | EngineEventKind::FullTime => &["{home}", "{away}"],
            EngineEventKind::AiDecision => &["{team}"],
            _ => &["{player}", "{team}"],
        };
        for line in &set.lines {
            for name in need {
                assert!(line.contains(name), "{:?}: {line:?} lacks {name}", set.kind);
            }
        }
    }
}

#[test]
fn five_events_of_one_kind_in_ten_minutes_give_three_distinct_lines() {
    let commentary = shipped();
    for kind in engine::EngineEventKind::ALL {
        if !commented(kind) {
            continue;
        }
        let mut c = Commentator::new(&commentary, names(), NINETY, common::SEED);
        let mut lines: Vec<String> = (0..5)
            .map(|i| c.line(&scripted(kind, 30 + 2 * i)).unwrap())
            .collect();
        lines.sort();
        lines.dedup();
        assert!(lines.len() >= 3, "{kind:?}: {lines:?}");
    }
}

/// The lines of every shipped set whose conditions satisfy `wanted`.
fn lines_where(
    commentary: &Commentary,
    wanted: impl Fn(&engine::commentary::templates::When) -> bool,
) -> Vec<String> {
    commentary
        .sets
        .iter()
        .filter(|s| s.kind == EngineEventKind::Goal && wanted(&s.when))
        .flat_map(|s| s.lines.iter().cloned())
        .collect()
}

#[test]
fn a_late_equaliser_and_a_rout_read_as_such() {
    let commentary = shipped();
    let names = names();

    let mut c = Commentator::new(&commentary, names.clone(), NINETY, common::SEED);
    let mut goal = scripted(EngineEventKind::Goal, 88);
    goal.team = Some(1);
    goal.player = Some(14);
    goal.scores = [1, 1];
    let line = c.line(&goal).unwrap();
    let late_equaliser = lines_where(&commentary, |w| {
        w.score == Some(ScoreState::Equaliser) && w.minute == Some(MinuteBand::Late)
    });
    assert!(!late_equaliser.is_empty());
    let filled: Vec<String> = late_equaliser
        .iter()
        .map(|l| {
            l.replace("{player}", &names.roster[14])
                .replace("{team}", &names.clubs[1])
                .replace("{opponent}", &names.clubs[0])
                .replace("{score}", "1-1")
        })
        .collect();
    assert!(
        filled.contains(&line),
        "{line:?} is not a late-equaliser line"
    );

    let mut c = Commentator::new(&commentary, names.clone(), NINETY, common::SEED);
    let mut goal = scripted(EngineEventKind::Goal, 20);
    goal.player = Some(9);
    goal.scores = [4, 0];
    let line = c.line(&goal).unwrap();
    let rout = lines_where(&commentary, |w| w.score == Some(ScoreState::Rout));
    assert!(!rout.is_empty());
    let filled: Vec<String> = rout
        .iter()
        .map(|l| {
            l.replace("{player}", &names.roster[9])
                .replace("{team}", &names.clubs[0])
                .replace("{opponent}", &names.clubs[1])
                .replace("{score}", "4-0")
        })
        .collect();
    assert!(filled.contains(&line), "{line:?} is not a rout line");
}

#[test]
fn a_full_seeded_match_fills_every_placeholder_and_names_every_player() {
    let commentary = shipped();
    let mut sim = Simulation::new(common::scoring_match()).unwrap();
    let names = MatchNames::of(&sim);
    let mut roster = names.roster.clone();
    let mut c = Commentator::for_match(&commentary, &sim);
    let mut events = sim.take_events();
    while !sim.is_over() {
        sim.step();
        events.extend(sim.take_events());
    }
    sim.finish();
    events.extend(sim.take_events());
    let mut lines = 0;
    for event in &events {
        let Some(line) = c.line(event) else {
            assert!(!commented(event.kind), "{event:?} got no line");
            continue;
        };
        lines += 1;
        assert!(!line.contains('{') && !line.contains('}'), "{line:?}");
        names_its_player_and_club(&line, event, &names, &roster);
        if let (Some(EventDetail::Substitution { on, .. }), Some(team), Some(slot)) =
            (event.detail, event.team, event.player)
        {
            roster[slot] = names.squads[team][on].clone();
        }
    }
    // The seed-9 match gives more than 70 lines (the seed-7 match gave 75, and scores no goal
    // since every sprint costs stamina; the seed-42 match gave 90 before a pressed lone
    // forward stopped dribbling into defenders, and now scores no goal); the floor only
    // guards against an empty check.
    assert!(lines > 70, "only {lines} lines");
    assert!(
        events.iter().any(|e| e.kind == EngineEventKind::Goal),
        "the match scored no goal"
    );
    assert_eq!(c.fallbacks(), 0);
}

/// The shipped file with `change` applied, written to a temp file, then loaded.
fn load_changed(name: &str, change: impl FnOnce(&mut serde_json::Value)) -> String {
    let dir = common::content_dir();
    let text = std::fs::read_to_string(dir.path(COMMENTARY_FILE)).unwrap();
    let mut doc: serde_json::Value = serde_json::from_str(&text).unwrap();
    change(&mut doc);
    let path = common::temp_path(&format!("commentary-{name}.json"));
    std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();
    let err = Commentary::load_path(&path, "commentary/x.json").unwrap_err();
    std::fs::remove_file(&path).unwrap();
    err.to_string()
}

/// The first set for `event` with no condition.
fn plain_set<'a>(doc: &'a mut serde_json::Value, event: &str) -> &'a mut serde_json::Value {
    doc["templates"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["event"] == event && s.get("when").is_none())
        .unwrap()
}

#[test]
fn a_kind_with_two_plain_lines_is_refused() {
    let err = load_changed("two-lines", |doc| {
        plain_set(doc, "corner")["lines"]
            .as_array_mut()
            .unwrap()
            .truncate(2);
    });
    assert!(
        err.starts_with("content refused: commentary commentary/x.json"),
        "{err}"
    );
    assert!(err.contains("event corner has 2 lines"), "{err}");
}

#[test]
fn an_unknown_placeholder_is_refused() {
    let err = load_changed("unknown", |doc| {
        plain_set(doc, "throw-in")["lines"]
            .as_array_mut()
            .unwrap()
            .push("{referee} waves play on for {team}.".into());
    });
    assert!(err.contains("unknown placeholder {referee}"), "{err}");
    assert!(err.contains("(throw-in)"), "{err}");
}

#[test]
fn a_placeholder_the_kind_cannot_fill_is_refused() {
    let err = load_changed("other-player", |doc| {
        plain_set(doc, "goal")["lines"]
            .as_array_mut()
            .unwrap()
            .push("{player} beats {other_player}.".into());
    });
    assert!(
        err.contains("placeholder {other_player} cannot be filled on goal"),
        "{err}"
    );
}

#[test]
fn another_schema_version_is_refused() {
    let err = load_changed("version", |doc| doc["schema_version"] = 2.into());
    assert!(
        err.contains("schema_version 2; this build reads 1"),
        "{err}"
    );
}

/// A shoot-out kick is a `penalty` event, but no penalty was awarded: neither the set-up nor
/// the outcome gets a line, while an awarded penalty still does.
#[test]
fn a_shootout_kick_gets_no_line() {
    let commentary = shipped();
    let mut c = Commentator::new(&commentary, names(), NINETY, common::SEED);
    let awarded = scripted(EngineEventKind::Penalty, 120);
    assert!(c.line(&awarded).is_some());
    let mut set_up = awarded;
    set_up.shootout_round = Some(1);
    assert_eq!(c.line(&set_up), None);
    let mut outcome = set_up;
    outcome.shootout_scored = Some(true);
    outcome.shootout_scores = Some([1, 0]);
    assert_eq!(c.line(&outcome), None);
}

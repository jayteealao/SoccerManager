//! Every player who plays is rated after the match: substitutes included, an unused
//! substitute not; the match record lists the same ratings with one decimal; a match resumed
//! from a snapshot rates its players exactly as the unbroken match; and a stronger side is
//! rated higher.

mod common;

use engine::observe::RatingEntry;
use engine::record::{NullSink, TickSink};
use engine::rules::clock::TICKS_PER_MINUTE;
use engine::sim::tally::PlayerRating;
use engine::snapshot::Snapshot;
use engine::{Change, EngineError, Manager, MatchConfig, Simulation, Stoppage};

const HOME: usize = 0;

/// The shipped teams at seed 5, the home side managed by hand.
fn config() -> MatchConfig {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    MatchConfig::new(5, 90, &content, [&a, &b])
        .unwrap()
        .with_manager(HOME, Manager::Human)
}

/// Plays `sim` to full time, the home side bringing on its first two substitutes at minutes
/// 55 and 70 for slots 9 and 10.
fn play_with_two_changes(sim: &mut Simulation) {
    let team = &sim.teams()[HOME];
    let changes = [
        (55, team.lineup[9], team.bench[0]),
        (70, team.lineup[10], team.bench[1]),
    ];
    let mut queued = 0;
    while !sim.is_over() {
        if let Some(&(minute, off, on)) = changes.get(queued)
            && sim.tick() >= minute * TICKS_PER_MINUTE
        {
            sim.queue_change(HOME, Change::Substitution { off, on });
            queued += 1;
        }
        sim.step();
    }
}

#[test]
fn every_player_who_played_is_rated_and_the_record_lists_the_same() {
    let mut sim = Simulation::new(config()).unwrap();
    let bench = sim.teams()[HOME].bench.clone();
    play_with_two_changes(&mut sim);
    let ratings = sim.match_ratings();
    let home: Vec<&PlayerRating> = ratings.iter().filter(|r| r.team == HOME).collect();
    assert_eq!(home.len(), 13, "eleven starters and two substitutes");
    for on in &bench[..2] {
        assert!(
            home.iter().any(|r| r.squad == *on),
            "substitute {on} is rated"
        );
    }
    for unused in &bench[2..] {
        assert!(
            home.iter().all(|r| r.squad != *unused),
            "unused substitute {unused} has no rating"
        );
    }
    for r in &ratings {
        assert!((10..=100).contains(&r.rating_tenths), "{r:?}");
    }
    assert!(
        ratings
            .windows(2)
            .all(|w| (w[0].team, w[0].squad) < (w[1].team, w[1].squad)),
        "home first, in squad order"
    );

    let record = RatingEntry::of_match(&sim);
    assert_eq!(record.len(), ratings.len());
    for (entry, r) in record.iter().zip(&ratings) {
        assert_eq!(entry.team, r.team);
        assert_eq!(entry.id, sim.teams()[r.team].player_ids[r.squad]);
        assert_eq!(entry.rating, f64::from(r.rating_tenths) / 10.0);
        let json = serde_json::to_string(entry).unwrap();
        let shown = json.split("\"rating\":").nth(1).unwrap();
        let decimals = shown.trim_end_matches('}').split('.').nth(1).unwrap_or("");
        assert!(decimals.len() <= 1, "{json}");
    }
}

/// Captures a snapshot at the first stoppage from minute 60.
#[derive(Default)]
struct At60 {
    snapshot: Option<Snapshot>,
}

impl TickSink for At60 {
    fn on_tick(&mut self, _: &engine::record::TickRecord) -> Result<(), EngineError> {
        Ok(())
    }

    fn on_stoppage(&mut self, _: &Stoppage, sim: &Simulation) -> Result<(), EngineError> {
        if self.snapshot.is_none() && sim.tick() >= 60 * TICKS_PER_MINUTE {
            self.snapshot = Some(Snapshot::capture(sim, [3; 16], 1_700_000_000_000));
        }
        Ok(())
    }
}

#[test]
fn a_match_resumed_at_minute_sixty_rates_its_players_as_the_unbroken_match() {
    let mut whole = Simulation::new(common::scoring_match()).unwrap();
    let mut capture = At60::default();
    whole.run(&mut capture).unwrap();
    let snapshot = capture.snapshot.expect("a stoppage after minute 60");
    let read = Snapshot::from_bytes(&snapshot.to_bytes(), "s.smsn").unwrap();
    let mut resumed = Simulation::from_snapshot(common::scoring_match(), &read).unwrap();
    assert!(resumed.tick() >= 60 * TICKS_PER_MINUTE);
    resumed.run(&mut NullSink).unwrap();
    assert_eq!(resumed.tallies(), whole.tallies());
    assert_eq!(resumed.match_ratings(), whole.match_ratings());
}

#[test]
fn a_stronger_side_is_rated_higher() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let strong = common::stronger(&a);
    let means = common::run_many(1..=20, |seed| {
        let mut sim =
            Simulation::new(MatchConfig::new(seed, 90, &content, [&strong, &b]).unwrap()).unwrap();
        sim.run(&mut NullSink).unwrap();
        let ratings = sim.match_ratings();
        let mean = |team: usize| {
            let side: Vec<f64> = ratings
                .iter()
                .filter(|r| r.team == team)
                .map(|r| r.rating())
                .collect();
            side.iter().sum::<f64>() / side.len() as f64
        };
        (mean(0), mean(1))
    });
    let n = means.len() as f64;
    let strong_mean = means.iter().map(|m| m.0).sum::<f64>() / n;
    let other_mean = means.iter().map(|m| m.1).sum::<f64>() / n;
    assert!(
        strong_mean > other_mean,
        "stronger side {strong_mean:.2}, other {other_mean:.2}"
    );
}

//! The fixed batch the fast model is fitted on and checked against: three strength levels
//! (every attribute times 1.00, 1.075 and 1.15) give nine ordered pairings, home level then
//! away level. Match `k` of every pairing plays the clubs of calibration fixture `k` from the
//! same generated leagues, each boosted by its level, so every pairing sees the same clubs
//! and every strength plays at home and away. The fit batch and the check batch play the
//! same fixtures with other engine seeds.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, bail};
use engine::data::TeamFile;
use engine::modules::fast_events::{FIRST_ADDED, SECOND_ADDED};
use engine::modules::fast_model::{self, KickOff, MINUTES};
use engine::record::NullSink;
use engine::team::PLAYERS_PER_TEAM;
use engine::{
    Card, Content, EngineEvent, EngineEventKind, MatchConfig, Simulation, StreamRules, Validator,
};
use serde::{Deserialize, Serialize};

use crate::calibrate::fixtures::{Leagues, boosted, fixtures, splitmix64};

/// The attribute boost of each strength level; 1.15 is the bands file's stronger-team boost.
pub const LEVELS: [f64; 3] = [1.0, 1.075, 1.15];
/// The seed of the generated leagues both batches play.
pub const LEAGUE_SEED: u64 = 1;
/// The ordered pairings, home level then away level.
pub const PAIRINGS: [[usize; 2]; 9] = [
    [0, 0],
    [0, 1],
    [0, 2],
    [1, 0],
    [1, 1],
    [1, 2],
    [2, 0],
    [2, 1],
    [2, 2],
];

/// A batch: how many matches per pairing, how long, and the engine seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub matches: u32,
    pub minutes: u32,
    pub seed: u64,
}

/// One full-engine match of a batch.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// The pairing's index in [`PAIRINGS`].
    pub pairing: usize,
    pub kick_off: KickOff,
    /// The final score, home first.
    pub goals: [u32; 2],
    /// Each goal's minute of regulation time, 0 to 89 (45 to 89 in the second half).
    pub goal_minutes: Vec<u32>,
    /// The match's other events, counted.
    pub tally: EventTally,
}

/// What a side's events count to in one match.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// Fouls the side committed, and those played on with advantage.
    pub fouls: u32,
    pub advantage: u32,
    pub offsides: u32,
    /// Yellow cards, a second yellow included, and straight reds.
    pub yellow: u32,
    pub red: u32,
    /// Second yellows, and the yellow cards shown while a player of the side on the pitch
    /// was already booked.
    pub second_yellow: u32,
    pub booked_cards: u32,
    /// Restarts the side took.
    pub corners: u32,
    pub throw_ins: u32,
    pub goal_kicks: u32,
    pub free_kicks: u32,
    pub penalties: u32,
    pub injuries: u32,
    /// Injuries that stopped play: those with a restart spot.
    pub stopping_injuries: u32,
    /// Substitutions, and those that replaced a player injured on that tick.
    pub substitutions: u32,
    pub injury_substitutions: u32,
}

/// The kinds whose minutes the fit shares out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timed {
    Foul,
    Offside,
    Corner,
    ThrowIn,
    GoalKick,
    Injury,
    /// A substitution not forced by an injury.
    Substitution,
}

/// A match's events, counted: per side, whether anyone was sent off, the seconds each half
/// added, and the minute bin of each timed event.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventTally {
    pub counts: [Counts; 2],
    pub sent_off: bool,
    pub added_s: [u32; 2],
    /// Per half, the goals scored in its added time.
    pub added_goals: [u32; 2],
    pub bins: Vec<(Timed, u8)>,
}

/// Counts the events of a regulation match, whatever played it: the full engine or the fast
/// model.
pub fn tally(events: &[EngineEvent]) -> EventTally {
    use EngineEventKind as K;
    let mut out = EventTally::default();
    let half = events
        .iter()
        .find(|e| e.kind == K::HalfTime)
        .map_or(u32::MAX, |e| e.tick);
    let bin = |e: &EngineEvent| -> u8 {
        let added = e.minute_added.is_some();
        let b = if e.tick < half {
            if added {
                FIRST_ADDED
            } else {
                (e.minute as usize).min(FIRST_ADDED - 1)
            }
        } else if added {
            SECOND_ADDED
        } else {
            FIRST_ADDED + 1 + (e.minute.saturating_sub(45) as usize).min(FIRST_ADDED - 1)
        };
        b as u8
    };
    // The booked players on the pitch, by player index.
    let mut booked = [false; 2 * PLAYERS_PER_TEAM];
    for e in events {
        match e.kind {
            K::HalfTime => out.added_s[0] = e.added_time_s.unwrap_or(0),
            K::FullTime => out.added_s[1] = e.added_time_s.unwrap_or(0),
            K::Goal if e.minute_added.is_some() && e.shootout_round.is_none() => {
                out.added_goals[usize::from(e.tick >= half)] += 1;
            }
            _ => {}
        }
        let Some(team) = e.team.filter(|t| *t < 2) else {
            continue;
        };
        let c = &mut out.counts[team];
        let timed = match e.kind {
            K::Foul => {
                c.fouls += 1;
                c.advantage += u32::from(e.advantage == Some(true));
                Some(Timed::Foul)
            }
            K::Offside => {
                c.offsides += 1;
                Some(Timed::Offside)
            }
            K::Card => {
                let side = team * PLAYERS_PER_TEAM..(team + 1) * PLAYERS_PER_TEAM;
                let any_booked = booked[side].iter().any(|b| *b);
                let player = e.player.filter(|p| *p < booked.len());
                match e.card {
                    Some(Card::Yellow) => {
                        c.yellow += 1;
                        c.booked_cards += u32::from(any_booked);
                        if let Some(p) = player {
                            booked[p] = true;
                        }
                    }
                    Some(Card::SecondYellow) => {
                        c.yellow += 1;
                        c.second_yellow += 1;
                        c.booked_cards += u32::from(any_booked);
                        out.sent_off = true;
                        if let Some(p) = player {
                            booked[p] = false;
                        }
                    }
                    Some(Card::Red) => {
                        c.red += 1;
                        out.sent_off = true;
                        if let Some(p) = player {
                            booked[p] = false;
                        }
                    }
                    None => {}
                }
                None
            }
            K::Corner => {
                c.corners += 1;
                Some(Timed::Corner)
            }
            K::ThrowIn => {
                c.throw_ins += 1;
                Some(Timed::ThrowIn)
            }
            K::GoalKick => {
                c.goal_kicks += 1;
                Some(Timed::GoalKick)
            }
            K::FreeKick => {
                c.free_kicks += 1;
                None
            }
            K::Penalty if e.shootout_round.is_none() => {
                c.penalties += 1;
                None
            }
            K::Injury => {
                c.injuries += 1;
                c.stopping_injuries += u32::from(e.spot.is_some());
                Some(Timed::Injury)
            }
            K::Substitution => {
                c.substitutions += 1;
                // The player who comes on in the slot is not booked.
                if let Some(p) = e.player.filter(|p| *p < booked.len()) {
                    booked[p] = false;
                }
                let forced = events.iter().any(|i| {
                    i.kind == K::Injury
                        && i.tick == e.tick
                        && i.team == e.team
                        && i.player == e.player
                });
                if forced {
                    c.injury_substitutions += 1;
                    None
                } else {
                    Some(Timed::Substitution)
                }
            }
            _ => None,
        };
        if let Some(t) = timed {
            out.bins.push((t, bin(e)));
        }
    }
    out
}

/// The pairing's name: `1.150 v 1.000`.
pub fn pairing_name(pairing: usize) -> String {
    let [h, a] = PAIRINGS[pairing];
    format!("{:.3} v {:.3}", LEVELS[h], LEVELS[a])
}

/// The engine seed of match `index` of a batch with `seed`.
pub fn match_seed(seed: u64, index: usize) -> u64 {
    splitmix64((seed << 40) ^ index as u64)
}

/// The minute of regulation time `goal` counts in, with the first half's added time in
/// minute 44 and the second half's in minute 89. `half` is the half-time tick.
pub fn goal_minute(goal: &EngineEvent, half: u32) -> u32 {
    let last = (MINUTES / 2 - 1) as u32;
    if goal.tick < half {
        goal.minute.min(last)
    } else {
        45 + goal.minute.saturating_sub(45).min(last)
    }
}

/// Plays every match of `spec` on the full engine with `jobs` threads; the rows come back in
/// batch order (pairing, then match). A match whose event stream breaks an event-stream rule
/// stops the batch.
pub fn play(content: &Content, spec: &Spec, jobs: usize) -> anyhow::Result<Vec<Row>> {
    let n = spec.matches as usize;
    if n == 0 {
        bail!("a batch needs at least one match per pairing");
    }
    let mut leagues = Leagues::new(LEAGUE_SEED, content);
    let clubs: Vec<[TeamFile; 2]> = fixtures(spec.matches)
        .iter()
        .map(|f| leagues.teams(f))
        .collect();
    let total = n * PAIRINGS.len();
    let next = AtomicUsize::new(0);
    let rows: Mutex<Vec<Option<anyhow::Result<Row>>>> =
        Mutex::new((0..total).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    if index >= total {
                        break;
                    }
                    let row = play_one(content, spec, &clubs, index);
                    rows.lock().expect("no thread panics holding the rows")[index] = Some(row);
                }
            });
        }
    });
    let rows = rows.into_inner().expect("no thread panicked");
    let out: Vec<Row> = rows
        .into_iter()
        .map(|r| r.expect("every match was played"))
        .collect::<anyhow::Result<_>>()?;
    tracing::info!(
        signal = "fast_model.batch",
        seed = spec.seed,
        matches = out.len()
    );
    Ok(out)
}

fn play_one(
    content: &Content,
    spec: &Spec,
    clubs: &[[TeamFile; 2]],
    index: usize,
) -> anyhow::Result<Row> {
    let n = spec.matches as usize;
    let pairing = index / n;
    let k = index % n;
    let [h, a] = PAIRINGS[pairing];
    let home = boosted(&clubs[k][0], LEVELS[h]);
    let away = boosted(&clubs[k][1], LEVELS[a]);
    let seed = match_seed(spec.seed, index);
    let config = MatchConfig::new(seed, spec.minutes, content, [&home, &away])?;
    let kick_off = fast_model::kick_off(&config);
    let rules = StreamRules::for_config(&config);
    let mut sim = Simulation::new(config)?;
    sim.run(&mut NullSink)?;
    let events = sim.take_events();
    let violations = Validator::check_events(&events, &rules);
    if let Some(v) = violations.first() {
        bail!(
            "match {index} (seed {seed}, {}) broke the event rule {} at tick {}",
            pairing_name(pairing),
            v.rule,
            v.tick
        );
    }
    let half = events
        .iter()
        .find(|e| e.kind == EngineEventKind::HalfTime)
        .map(|e| e.tick)
        .with_context(|| format!("match {index} (seed {seed}) has no half-time"))?;
    let goal_minutes = events
        .iter()
        .filter(|e| e.kind == EngineEventKind::Goal)
        .map(|e| goal_minute(e, half))
        .collect();
    Ok(Row {
        pairing,
        kick_off,
        goals: sim.summary().goals,
        goal_minutes,
        tally: tally(&events),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_ordered_pairings_play_every_level_at_home_and_away() {
        for level in 0..LEVELS.len() {
            assert_eq!(PAIRINGS.iter().filter(|p| p[0] == level).count(), 3);
            assert_eq!(PAIRINGS.iter().filter(|p| p[1] == level).count(), 3);
        }
        assert_eq!(pairing_name(6), "1.150 v 1.000");
    }

    #[test]
    fn the_two_batches_seed_every_match_differently() {
        let fit: Vec<u64> = (0..9_000).map(|i| match_seed(1, i)).collect();
        let check: Vec<u64> = (0..9_000).map(|i| match_seed(2, i)).collect();
        let all: std::collections::BTreeSet<u64> = fit.iter().chain(&check).copied().collect();
        assert_eq!(all.len(), 18_000);
    }
}

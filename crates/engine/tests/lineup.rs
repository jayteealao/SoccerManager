//! The AI lineup by position rating and the bench that covers every line.
//!
//! - A player clearly better at another slot's position than the player listed there starts
//!   at that slot; a close one does not, and the listed player starts.
//! - The bench of the shipped squads covers the goal, every line of the formation and both
//!   kinds of wide position, in every formation of the tactics file.
//! - `substitution_table` (ignored, release): substitutions a match per side, the 0 to 5
//!   counts, the mean minute, the substitute's minutes, and the substitutes out of position,
//!   both as the listed position against the slot's and as a rating at the slot below his
//!   natural rating; with the floor no bench can go under (substitutions at a position beyond
//!   the spares listed there).

mod common;

use std::collections::BTreeMap;

use engine::ai::pre_match_for;
use engine::data::{Position, TeamFile};
use engine::rules::clock::TICKS_PER_MINUTE;
use engine::team::{PLAYERS_PER_TEAM, Team};
use engine::{
    Content, EngineEventKind, EventDetail, MatchConfig, NullSink, Rating, Simulation, Tactics,
};

/// One side's substitutions over a batch.
#[derive(Default)]
struct Subs {
    matches: u32,
    counts: [u32; 6],
    minutes: Vec<f64>,
    played: Vec<f64>,
    listed_out: u32,
    rated_out: u32,
    /// Substitutions at a position beyond the squad's spares listed there at kick-off.
    floor: u32,
    /// Substitutions at a position beyond the bench players listed there at kick-off.
    bench_floor: u32,
    /// Slot position, then the substitute's listed position, for each one out of position.
    pairs: BTreeMap<String, u32>,
}

impl Subs {
    fn print(&self, name: &str) {
        let n = self.minutes.len() as f64;
        let m = f64::from(self.matches);
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
        println!(
            "{name}: {:.2} a match; counts 0..5 {:?}; mean minute {:.1}; substitute minutes {:.1}; \
             out of listed position {:.2} ({:.2} a match); rated below natural {:.2} ({:.2} a match); \
             floor {:.2} a match with any bench, {:.2} with this bench",
            n / m,
            self.counts,
            mean(&self.minutes),
            mean(&self.played),
            f64::from(self.listed_out) / n.max(1.0),
            f64::from(self.listed_out) / m,
            f64::from(self.rated_out) / n.max(1.0),
            f64::from(self.rated_out) / m,
            f64::from(self.floor) / m,
            f64::from(self.bench_floor) / m,
        );
        println!("{name}: out of position, slot<-listed: {:?}", self.pairs);
    }

    fn add(&mut self, from: Subs) {
        self.matches += from.matches;
        for (a, b) in self.counts.iter_mut().zip(from.counts) {
            *a += b;
        }
        self.minutes.extend(from.minutes);
        self.played.extend(from.played);
        self.listed_out += from.listed_out;
        self.rated_out += from.rated_out;
        self.floor += from.floor;
        self.bench_floor += from.bench_floor;
        for (k, v) in from.pairs {
            *self.pairs.entry(k).or_default() += v;
        }
    }
}

/// Plays `[home, away]` with `seed` and returns each side's substitutions, home first.
fn play(content: &Content, files: [&TeamFile; 2], seed: u64) -> [Subs; 2] {
    let mut sim = Simulation::new(MatchConfig::new(seed, 90, content, files).unwrap()).unwrap();
    let start = sim.teams();
    sim.run(&mut NullSink).unwrap();
    let teams = sim.teams();
    let tallies = sim.tallies();
    let mut out = [Subs::default(), Subs::default()];
    let mut at_position: [BTreeMap<Position, u32>; 2] = Default::default();
    for e in sim.events() {
        let (EngineEventKind::Substitution, Some(EventDetail::Substitution { on, .. })) =
            (e.kind, e.detail)
        else {
            continue;
        };
        let (Some(team), Some(roster)) = (e.team, e.player) else {
            continue;
        };
        let at = sim.slot_position(team, roster - team * PLAYERS_PER_TEAM);
        let sub = &teams[team].squad[on];
        let side = &mut out[team];
        side.minutes.push(f64::from(e.minute));
        side.played
            .push(f64::from(tallies[team][on].ticks_played) / f64::from(TICKS_PER_MINUTE));
        side.listed_out += u32::from(sub.position != at);
        let rated_out = sub.positions[at as usize] < sub.positions[sub.position as usize];
        side.rated_out += u32::from(rated_out);
        if rated_out {
            *side
                .pairs
                .entry(format!("{}<-{}", at.code(), sub.position.code()))
                .or_default() += 1;
        }
        *at_position[team].entry(at).or_default() += 1;
    }
    for (team, side) in out.iter_mut().enumerate() {
        let t = &start[team];
        let listed = |p: Position, among: &mut dyn Iterator<Item = usize>| {
            among.filter(|&s| t.squad[s].position == p).count() as u32
        };
        for (&p, &n) in &at_position[team] {
            let spares = listed(p, &mut (0..t.squad.len()).filter(|s| !t.lineup.contains(s)));
            let bench = listed(p, &mut t.bench.iter().copied());
            side.floor += n.saturating_sub(spares);
            side.bench_floor += n.saturating_sub(bench);
        }
        side.matches = 1;
        side.counts[side.minutes.len().min(5)] += 1;
    }
    out
}

/// The substitution table of the default teams over seeds 1 to 80, the 15 percent stronger
/// side against the other, in both home orders, as the stronger-side rating test plays them.
#[test]
#[ignore = "release run: cargo test --release -p engine --test lineup -- --ignored --nocapture"]
fn substitution_table() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let strong = common::stronger(&a);
    let mut sides = [Subs::default(), Subs::default()];
    for strong_home in [true, false] {
        let files = if strong_home {
            [&strong, &b]
        } else {
            [&b, &strong]
        };
        let runs = common::run_many(1..=80, |seed| play(&content, files, seed));
        for [home, away] in runs {
            let (s, o) = if strong_home {
                (home, away)
            } else {
                (away, home)
            };
            sides[0].add(s);
            sides[1].add(o);
        }
    }
    sides[0].print("stronger side");
    sides[1].print("other side");
    let per_match = |s: &Subs, v: u32| f64::from(v) / f64::from(s.matches);
    println!(
        "both sides a match: out of position by rating {:.2}; floor with any bench {:.2}; floor with this bench {:.2}",
        per_match(&sides[0], sides[0].rated_out) + per_match(&sides[1], sides[1].rated_out),
        per_match(&sides[0], sides[0].floor) + per_match(&sides[1], sides[1].floor),
        per_match(&sides[0], sides[0].bench_floor) + per_match(&sides[1], sides[1].bench_floor),
    );
}

/// `default-a` with every player rated 15.0 at his listed position and 5.0 elsewhere, and
/// its spare attacking midfielder (squad index 17) rated `at_left_wing` at left wing.
fn rated_squad(content: &Content, at_left_wing: f64) -> Team {
    let [mut file, _] = common::default_teams(content);
    for (i, p) in file.players.iter_mut().enumerate() {
        let mut positions: BTreeMap<Position, Rating> = Position::ALL
            .into_iter()
            .map(|q| {
                (
                    q,
                    Rating::from_tenths(if q == p.position { 150 } else { 50 }),
                )
            })
            .collect();
        if i == 17 {
            assert_eq!(p.position, Position::AM);
            positions.insert(Position::AM, Rating::from_tenths(180));
            positions.insert(
                Position::LW,
                Rating::from_tenths((at_left_wing * 10.0) as u8),
            );
        }
        p.positions = Some(positions);
    }
    Team::from_file(0, &file, &content.attributes, &content.tuning.engine)
        .unwrap()
        .0
}

/// The steering criterion: an AI team plays a player out of his listed position when his
/// rating there is clearly higher (by more than the tactics file's margin, 1.5), and keeps the
/// listed player when the ratings are close.
#[test]
fn a_player_clearly_better_at_another_slot_plays_there_and_a_close_one_does_not() {
    let content = common::content();
    assert_eq!(content.tactics.ai.out_of_position_margin, 1.5);
    let tactics = Tactics::defaults(&content.tactics);
    let slots = &content.tactics.formations[usize::from(tactics.formation)].slots;
    let left_wing = slots
        .iter()
        .position(|s| s.position == Position::LW)
        .unwrap();
    // 17.0 at left wing against the listed left winger's 15.0: clearly better.
    let team = rated_squad(&content, 17.0);
    let setup = pre_match_for(&team, tactics, &content.tactics, &content.attributes);
    assert_eq!(
        setup.lineup[left_wing], 17,
        "the clearly better player plays left wing"
    );
    assert_eq!(team.squad[17].position, Position::AM);
    // 16.0 against 15.0: close, so the listed left winger keeps his place.
    let team = rated_squad(&content, 16.0);
    let setup = pre_match_for(&team, tactics, &content.tactics, &content.attributes);
    assert_eq!(team.squad[setup.lineup[left_wing]].position, Position::LW);
    assert_ne!(setup.lineup[left_wing], 17);
}

/// The bench of each shipped squad, in every formation of the tactics file: a goalkeeper, a
/// player listed in each line the formation plays, a full-back and a winger when it plays
/// them, no starter, nobody twice.
#[test]
fn the_bench_covers_the_goal_every_line_and_the_wide_positions() {
    let content = common::content();
    let lines = [
        [Position::CB, Position::LB, Position::RB],
        [Position::DM, Position::CM, Position::AM],
        [Position::LW, Position::RW, Position::ST],
    ];
    for (i, file) in common::default_teams(&content).iter().enumerate() {
        let (team, _) =
            Team::from_file(i, file, &content.attributes, &content.tuning.engine).unwrap();
        for f in 0..content.tactics.formations.len() {
            let mut tactics = Tactics::defaults(&content.tactics);
            tactics.set_formation(f as u8, &content.tactics);
            let setup = pre_match_for(&team, tactics, &content.tactics, &content.attributes);
            let name = &content.tactics.formations[f].name;
            let plays: Vec<Position> = content.tactics.formations[f]
                .slots
                .iter()
                .map(|s| s.position)
                .collect();
            let bench: Vec<Position> = setup
                .bench
                .iter()
                .map(|&s| team.squad[s].position)
                .collect();
            assert_eq!(setup.bench.len(), 7, "{name}");
            assert!(
                setup.bench.iter().all(|s| !setup.lineup.contains(s)),
                "{name}"
            );
            let mut all = setup.bench.clone();
            all.sort_unstable();
            all.dedup();
            assert_eq!(all.len(), 7, "{name}: a player is named twice");
            assert_eq!(bench[0], Position::GK, "{name}");
            for line in lines {
                if plays.iter().any(|p| line.contains(p)) {
                    assert!(
                        bench.iter().any(|p| line.contains(p)),
                        "{name}: {line:?} uncovered"
                    );
                }
            }
            for wide in [[Position::LB, Position::RB], [Position::LW, Position::RW]] {
                if plays.iter().any(|p| wide.contains(p)) {
                    assert!(
                        bench.iter().any(|p| wide.contains(p)),
                        "{name}: {wide:?} uncovered"
                    );
                }
            }
        }
    }
}

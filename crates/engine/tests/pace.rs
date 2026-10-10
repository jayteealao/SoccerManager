//! The pace selector over seeded full matches: no player sprints outside an URGENT job; work
//! rate changes how often a squad sprints but not how far it runs; and two players with the
//! same job and grade sprint differently by their work rate and personality. Each check reads
//! the players' public state after every tick that started in open play.

mod common;

use engine::data::TeamFile;
use engine::data::team::Position;
use engine::pace::{Grade, Level};
use engine::{MatchConfig, Rating, Simulation};

/// Speed above which a player sprints, in metres per second, and the ticks he must hold it
/// for one sprint to count (the 7 m/s for 1 s rule).
const SPRINT_MS: f64 = 7.0;
const SPRINT_TICKS: u32 = 50;
/// Ticks per minute.
const MINUTE: f64 = 3000.0;

const SEEDS: [u64; 4] = [1, 2, 3, 4];

/// `file` with every outfield player's `name` rating set by `rating(k)`, `k` counting the
/// outfield players in file order.
fn with_ratings(file: &TeamFile, name: &str, rating: impl Fn(usize) -> f64) -> TeamFile {
    let mut out = file.clone();
    let mut k = 0;
    for p in &mut out.players {
        if p.position == Position::GK {
            continue;
        }
        let value = rating(k);
        assert!((1.0..=20.0).contains(&value), "{value} is a rating");
        let tenths = (value * 10.0).round() as u8;
        p.attributes
            .insert(name.to_string(), Rating::from_tenths(tenths));
        k += 1;
    }
    out
}

/// Plays `config` to the end and calls `each` after every tick that started in open play.
fn play(config: MatchConfig, mut each: impl FnMut(&Simulation)) {
    let mut sim = Simulation::new(config).unwrap();
    while !sim.is_over() {
        let live = sim.dead_ball().is_none()
            && matches!(sim.phase_name(), engine::rules::phases::PhaseName::OpenPlay);
        sim.step();
        if live {
            each(&sim);
        }
    }
}

/// One team's running over a match: sprints by the 7 m/s for 1 s rule, metres run, and the
/// ball-in-play minutes, for the outfield players of team `team`.
#[derive(Debug, Default, Clone, Copy)]
struct Running {
    sprints: u32,
    metres: f64,
    minutes: f64,
}

impl Running {
    fn sprints_per_minute(&self) -> f64 {
        f64::from(self.sprints) / self.minutes
    }
    fn metres_per_minute(&self) -> f64 {
        self.metres / self.minutes
    }
    fn add(&mut self, o: Running) {
        self.sprints += o.sprints;
        self.metres += o.metres;
        self.minutes += o.minutes;
    }
}

fn running(config: MatchConfig, team: usize) -> Running {
    let keeper_slot = config.teams[team].keeper_slot();
    let mut out = Running::default();
    let mut fast = [0u32; 22];
    let mut last: [Option<engine::math::DVec2>; 22] = [None; 22];
    play(config, |sim| {
        out.minutes += 1.0 / MINUTE;
        for (i, p) in sim.players().iter().enumerate() {
            if p.team != team || p.slot == keeper_slot || !p.active() {
                fast[i] = 0;
                last[i] = None;
                continue;
            }
            if let Some(prev) = last[i] {
                out.metres += (p.pos - prev).length();
            }
            last[i] = Some(p.pos);
            if p.vel.length() > SPRINT_MS {
                fast[i] += 1;
                if fast[i] == SPRINT_TICKS {
                    out.sprints += 1;
                }
            } else {
                fast[i] = 0;
            }
        }
    });
    out
}

/// AC-11: on four seeded full matches, no player moves faster than 7 m/s on a tick of open
/// play while his job is not URGENT.
#[test]
fn nobody_sprints_outside_an_urgent_job() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let found = common::run_many(0..=SEEDS.len() as u64 - 1, |k| {
        let config = MatchConfig::new(SEEDS[k as usize], 90, &content, [&a, &b]).unwrap();
        let mut bad = Vec::new();
        let mut fast = 0u64;
        play(config, |sim| {
            for (i, p) in sim.players().iter().enumerate() {
                if !p.active() || p.vel.length() <= SPRINT_MS + 1e-9 {
                    continue;
                }
                fast += 1;
                if p.pace.grade != Grade::Urgent {
                    bad.push((sim.tick(), i, p.vel.length(), p.pace));
                }
            }
        });
        (fast, bad)
    });
    let fast: u64 = found.iter().map(|(f, _)| f).sum();
    assert!(fast > 0, "some player sprinted: the check can fail");
    for (k, (_, bad)) in found.iter().enumerate() {
        assert!(
            bad.is_empty(),
            "seed {}: {} sprint ticks outside an urgent job, first {:?}",
            SEEDS[k],
            bad.len(),
            bad.first()
        );
    }
}

/// The running of `file` at home against `away` over `seeds`.
fn squad_running(file: &TeamFile, away: &TeamFile, seeds: &[u64]) -> Running {
    let content = common::content();
    let each = common::run_many(0..=seeds.len() as u64 - 1, |k| {
        let config = MatchConfig::new(seeds[k as usize], 90, &content, [file, away]).unwrap();
        running(config, 0)
    });
    let mut total = Running::default();
    for r in each {
        total.add(r);
    }
    total
}

/// AC-12: two squads that differ only in work rate (6 against 16 for every outfield player)
/// play the same opponent on the same seeds. The high squad sprints at least 15 percent more
/// often per ball-in-play minute; the distance per ball-in-play minute differs by less than 5
/// percent. Control: the same squad on two seed sets differs by less than the 15 percent.
#[test]
fn work_rate_changes_how_often_a_squad_sprints_not_how_far_it_runs() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    let low = squad_running(&with_ratings(&a, "work_rate", |_| 6.0), &b, &SEEDS);
    let high = squad_running(&with_ratings(&a, "work_rate", |_| 16.0), &b, &SEEDS);
    let (ls, hs) = (low.sprints_per_minute(), high.sprints_per_minute());
    let (lm, hm) = (low.metres_per_minute(), high.metres_per_minute());
    eprintln!(
        "work rate 6: {ls:.4} sprints and {lm:.1} m per ball-in-play minute; \
         work rate 16: {hs:.4} sprints and {hm:.1} m"
    );
    assert!(ls > 0.0, "the low squad sprints at all");
    assert!(hs >= ls * 1.15, "sprints {hs:.4} against {ls:.4}");
    let distance_gap = (hm - lm).abs() / lm;
    assert!(distance_gap < 0.05, "distance differs by {distance_gap:.3}");
    // Control: the same squad on another seed set.
    let other = squad_running(&a, &b, &[5, 6, 7, 8]);
    let same = squad_running(&a, &b, &SEEDS);
    let (os, ss) = (other.sprints_per_minute(), same.sprints_per_minute());
    let control_gap = (os - ss).abs() / os.min(ss);
    eprintln!("control: {ss:.4} against {os:.4} sprints per minute ({control_gap:.3})");
    assert!(
        control_gap < 0.15,
        "seed noise alone gives {control_gap:.3}"
    );
}

/// AC-16: in one squad whose outfield players spread from 5 to 18 in work rate,
/// determination and professionalism, players with the same job and grade sprint
/// differently: the share of their URGENT ticks at sprint pace spreads, and the highest
/// work rate's share is above the lowest's.
#[test]
fn players_with_the_same_job_and_grade_differ_by_work_rate_and_personality() {
    let content = common::content();
    let [a, b] = common::default_teams(&content);
    // The outfield players of the squad, in file order, spread evenly from 5 to 18, the same
    // order in all three values.
    let outfield = a
        .players
        .iter()
        .filter(|p| p.position != Position::GK)
        .count();
    let level = |k: usize| 5.0 + 13.0 * k as f64 / (outfield - 1) as f64;
    let mut spread = with_ratings(&a, "work_rate", level);
    spread = with_ratings(&spread, "determination", level);
    spread = with_ratings(&spread, "professionalism", level);
    let per_seed = common::run_many(0..=SEEDS.len() as u64 - 1, |k| {
        let config = MatchConfig::new(SEEDS[k as usize], 90, &content, [&spread, &b]).unwrap();
        // Urgent ticks and sprint ticks per squad index of the home team.
        let mut counts = vec![(0u64, 0u64); spread.players.len()];
        play(config, |sim| {
            for p in sim.players().iter().filter(|p| p.team == 0 && p.active()) {
                if p.pace.grade == Grade::Urgent && p.pace.level >= Level::HighSpeed {
                    counts[p.squad].0 += 1;
                    if p.pace.level == Level::Sprint {
                        counts[p.squad].1 += 1;
                    }
                }
            }
        });
        counts
    });
    let mut totals = vec![(0u64, 0u64); spread.players.len()];
    for counts in per_seed {
        for (t, c) in totals.iter_mut().zip(counts) {
            t.0 += c.0;
            t.1 += c.1;
        }
    }
    // Outfield squad indices in work-rate order, with their sprint share.
    let mut shares = Vec::new();
    for (s, p) in spread.players.iter().enumerate() {
        if p.position == Position::GK || totals[s].0 < 200 {
            continue;
        }
        let wr = p.attributes["work_rate"].tenths();
        shares.push((wr, totals[s].1 as f64 / totals[s].0 as f64));
    }
    shares.sort_by_key(|&(wr, _)| wr);
    eprintln!("work rate (tenths) and sprint share of urgent running: {shares:?}");
    assert!(shares.len() >= 4, "enough players ran urgently: {shares:?}");
    let (lo, hi) = shares
        .iter()
        .fold((f64::INFINITY, 0.0f64), |(lo, hi), &(_, s)| {
            (lo.min(s), hi.max(s))
        });
    assert!(hi - lo > 0.0, "the shares spread");
    let (first, last) = (shares[0].1, shares[shares.len() - 1].1);
    assert!(
        last > first,
        "the highest work rate sprints more of his urgent running: {last:.3} against {first:.3}"
    );
}

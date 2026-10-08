//! The five-match rule: a top player looks clearly better than an average player in the same
//! role in about two of three runs of five matches. For each role, the home player of club A
//! in that role is replaced by a copy with every visible attribute at the top level or at the
//! average level; the copy keeps the original's body fields and hidden values, and starts in
//! the original's place. A run plays five matches against club B on five seeds, once with each
//! copy; it counts for the top copy when his mean match rating over the five is higher (a tie
//! counts half). The share of runs, pooled over the roles, is judged against the target.

use serde::Serialize;

use crate::data::Content;
use crate::data::team::{Position, TeamFile};
use crate::rating::Rating;
use crate::rng::EngineRng;
use crate::sensitivity::design::{Clubs, parallel};
use crate::sensitivity::rules::FiveMatch;
use crate::sensitivity::words::{Interval, Word};
use crate::sim::{MatchConfig, Simulation};

/// One role's share of runs the top copy won.
#[derive(Debug, Clone, Serialize)]
pub struct RoleShare {
    pub role: Position,
    pub runs: usize,
    pub share: f64,
}

/// The five-match rule's result.
#[derive(Debug, Clone, Serialize)]
pub struct FiveMatchResult {
    pub roles: Vec<RoleShare>,
    pub pooled: Interval,
    pub target: f64,
    pub tolerance: f64,
    pub word: Word,
}

/// The word of the pooled share interval: pass inside the target band, fail wholly outside it.
pub fn word(pooled: Interval, target: f64, tolerance: f64) -> Word {
    let (lo, hi) = (target - tolerance, target + tolerance);
    if pooled.lo >= lo && pooled.hi <= hi {
        Word::Pass
    } else if pooled.hi < lo || pooled.lo > hi {
        Word::Fail
    } else {
        Word::NotSure
    }
}

/// The share of `outcomes` (1 a top win, 0.5 a tie, 0 a loss) and its 95 percent percentile
/// interval over `resamples` resamples from `seed`.
pub fn share(outcomes: &[f64], resamples: u32, seed: u64) -> Interval {
    let n = outcomes.len();
    let est = outcomes.iter().sum::<f64>() / n as f64;
    let mut rng = EngineRng::from_seed(seed);
    let mut means: Vec<f64> = (0..resamples)
        .map(|_| (0..n).map(|_| outcomes[rng.range_usize(n)]).sum::<f64>() / n as f64)
        .collect();
    means.sort_by(f64::total_cmp);
    let at = |q: f64| means[((means.len() - 1) as f64 * q).round() as usize];
    Interval {
        est,
        lo: at(0.025),
        hi: at(0.975),
    }
}

/// `file` with squad player `squad`'s visible attributes all at `rating`.
fn copy_at(file: &TeamFile, content: &Content, squad: usize, rating: f64) -> TeamFile {
    let mut out = file.clone();
    let tenths = (rating * 10.0).round() as u8;
    for def in content.attributes.attributes.iter().filter(|d| !d.hidden) {
        out.players[squad]
            .attributes
            .insert(def.name.clone(), Rating::from_tenths(tenths));
    }
    out
}

/// The squad index of the starter of club A in `role`, and the lineup and bench that keep him
/// in his place.
fn starter(content: &Content, clubs: &Clubs, role: Position) -> (usize, [usize; 11], Vec<usize>) {
    let config = MatchConfig::new(1, 90, content, [&clubs.measured, &clubs.opponent])
        .expect("the default clubs build a match");
    let team = &config.teams[0];
    let squad = team
        .lineup
        .iter()
        .copied()
        .find(|&q| team.squad[q].position == role)
        .unwrap_or_else(|| panic!("club A starts no {role:?}"));
    (squad, team.lineup, team.bench.clone())
}

/// The match rating of squad player `squad` of the home side in one match.
fn rating(
    content: &Content,
    file: &TeamFile,
    clubs: &Clubs,
    lineup: [usize; 11],
    bench: &[usize],
    squad: usize,
    seed: u64,
) -> f64 {
    let config = MatchConfig::new(seed, 90, content, [file, &clubs.opponent])
        .expect("the copy builds a match")
        .with_setup(0, lineup, bench.to_vec());
    let mut sim = Simulation::new(config).expect("the match starts");
    while !sim.is_over() {
        sim.step();
        sim.events.clear();
    }
    sim.finish();
    sim.match_ratings()
        .into_iter()
        .find(|r| r.team == 0 && r.squad == squad)
        .map_or(0.0, |r| r.rating())
}

/// Runs the five-match rule of `fm` from `seed`.
pub fn run(
    content: &Content,
    clubs: &Clubs,
    fm: &FiveMatch,
    seed: u64,
    resamples: u32,
) -> FiveMatchResult {
    let roles: Vec<_> = fm
        .roles
        .iter()
        .map(|&role| {
            let (squad, lineup, bench) = starter(content, clubs, role);
            let top = copy_at(&clubs.measured, content, squad, fm.top);
            let average = copy_at(&clubs.measured, content, squad, fm.average);
            (role, squad, lineup, bench, top, average)
        })
        .collect();
    let runs = fm.runs as usize;
    let per = fm.matches_per_run as usize;
    let outcomes = parallel(roles.len() * runs, |u| {
        let r = u / runs;
        let (_, squad, lineup, bench, top, average) = &roles[r];
        let mean = |file: &TeamFile| {
            (0..per)
                .map(|m| {
                    let s = seed.wrapping_mul(7_919).wrapping_add((u * per + m) as u64);
                    rating(content, file, clubs, *lineup, bench, *squad, s)
                })
                .sum::<f64>()
                / per as f64
        };
        let (t, a) = (mean(top), mean(average));
        if t > a {
            1.0
        } else if t == a {
            0.5
        } else {
            0.0
        }
    });
    let shares = roles
        .iter()
        .enumerate()
        .map(|(r, (role, ..))| {
            let part = &outcomes[r * runs..(r + 1) * runs];
            RoleShare {
                role: *role,
                runs,
                share: part.iter().sum::<f64>() / runs as f64,
            }
        })
        .collect();
    let pooled = share(&outcomes, resamples, seed ^ 0x5EED);
    FiveMatchResult {
        roles: shares,
        pooled,
        target: fm.target,
        tolerance: fm.tolerance,
        word: word(pooled, fm.target, fm.tolerance),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_share_counts_wins_and_half_ties() {
        let outcomes = [1.0, 1.0, 0.0, 0.5];
        let s = share(&outcomes, 199, 3);
        assert_eq!(s.est, 0.625);
        assert!(s.lo <= s.est && s.est <= s.hi);
    }

    #[test]
    fn the_word_follows_the_interval() {
        let i = |lo, hi| Interval {
            est: (lo + hi) / 2.0,
            lo,
            hi,
        };
        assert_eq!(word(i(0.62, 0.70), 0.6667, 0.07), Word::Pass);
        assert_eq!(word(i(0.80, 0.90), 0.6667, 0.07), Word::Fail);
        assert_eq!(word(i(0.40, 0.55), 0.6667, 0.07), Word::Fail);
        assert_eq!(word(i(0.55, 0.70), 0.6667, 0.07), Word::NotSure);
    }

    #[test]
    fn hand_made_runs_give_the_share_and_the_word() {
        let mut outcomes = vec![1.0; 268];
        outcomes.extend(vec![0.0; 132]);
        let s = share(&outcomes, 999, 11);
        assert!((s.est - 0.67).abs() < 1e-9);
        assert_eq!(word(s, 0.6667, 0.07), Word::Pass);
    }
}

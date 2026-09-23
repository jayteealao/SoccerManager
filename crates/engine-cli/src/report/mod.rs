//! The run-scoped builder of a calibration run: it folds every `match-stats` record of a
//! suite into the aggregate figures, checks them against the accepted realism bands, sums
//! the dark paths, and names the outlier matches whose event files are kept.

pub mod bands;

use std::collections::BTreeMap;

use engine::observe::{MatchStats, Record, round_to};
use serde::Serialize;

use bands::Bands;

/// Matches slower than this, in milliseconds, are outliers (the contract's slow threshold).
pub const SLOW_MATCH_MS: u64 = 2000;

/// One suite of a calibration run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Suite {
    /// Clubs of the same generated strength.
    Equal,
    /// One club of each match has every attribute raised by the bands' boost.
    Strength,
}

impl Suite {
    pub const ALL: [Suite; 2] = [Suite::Equal, Suite::Strength];

    pub fn code(self) -> &'static str {
        match self {
            Suite::Equal => "equal",
            Suite::Strength => "strength",
        }
    }

    /// The suite's number in match seeds.
    pub fn number(self) -> u64 {
        match self {
            Suite::Equal => 0,
            Suite::Strength => 1,
        }
    }
}

/// The stronger club's results in the strength suite.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct StrongerRecord {
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    /// Wins over matches played; a draw is not a win.
    pub win_rate: f64,
}

/// The aggregate figures of one suite. Means are over the matches that played to the end.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SuiteFigures {
    /// Matches the suite planned.
    pub matches: u32,
    /// Statistics records found, failed matches included.
    pub recorded: u32,
    pub failures: u32,
    pub outliers: u32,
    pub goals_per_match_mean: f64,
    pub goals_per_match_sd: f64,
    pub shots_per_team_mean: f64,
    pub shots_on_target_per_team_mean: f64,
    pub xg_per_match_mean: f64,
    pub passes_per_team_mean: f64,
    pub pass_accuracy_pct_mean: f64,
    pub possession_home_mean: f64,
    pub possession_away_mean: f64,
    /// The share of matches with both possessions inside the band.
    pub possession_in_band_share: f64,
    pub fouls_per_team_mean: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stronger: Option<StrongerRecord>,
}

/// One band judged: `pass` when `value` lies in `lo` to `hi` (the win rate must exceed
/// `lo`, and the wall time stay below `hi`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BandCheck {
    pub band: String,
    pub suite: String,
    pub value: f64,
    pub lo: f64,
    pub hi: f64,
    pub pass: bool,
}

/// Every record of one suite, as the builder keeps it.
#[derive(Debug, Default)]
struct SuiteAcc {
    matches: u32,
    records: Vec<MatchStats>,
    /// For each record of the strength suite: the side of the boosted club.
    boosted: Vec<Option<usize>>,
    outliers: u32,
}

/// The run-scoped builder.
#[derive(Debug)]
pub struct RunBuilder {
    bands: Bands,
    suites: BTreeMap<Suite, SuiteAcc>,
    /// Statistics records that never appeared (`darkpath.match_without_stats`).
    pub missing: u32,
}

impl RunBuilder {
    pub fn new(bands: Bands) -> Self {
        Self {
            bands,
            suites: BTreeMap::new(),
            missing: 0,
        }
    }

    /// `suite` planned `matches` matches.
    pub fn plan(&mut self, suite: Suite, matches: u32) {
        self.suites.entry(suite).or_default().matches = matches;
    }

    /// Folds one record in. `boosted` names the stronger club's side in the strength suite.
    /// Returns `true` when the match is an outlier.
    pub fn add(&mut self, suite: Suite, record: MatchStats, boosted: Option<usize>) -> bool {
        let outlier = self.outlier(&record);
        let acc = self.suites.entry(suite).or_default();
        acc.outliers += u32::from(outlier);
        acc.records.push(record);
        acc.boosted.push(boosted);
        outlier
    }

    /// A planned record that never appeared.
    pub fn add_missing(&mut self) {
        self.missing += 1;
    }

    /// `true` for a failed match, a match with validator violations, a slow match, a
    /// dark-path hit, and a match with a possession or a team's shots outside its band.
    pub fn outlier(&self, s: &MatchStats) -> bool {
        let b = &self.bands;
        s.outcome != "success"
            || s.validate_violations > 0
            || s.duration_ms > SLOW_MATCH_MS
            || s.tactics.change_never_applied > 0
            || s.figures
                .possession_pct
                .iter()
                .any(|&p| !b.possession_pct.contains(p))
            || s.tactics
                .shots
                .iter()
                .any(|&n| !b.shots_per_team.contains(f64::from(n)))
    }

    /// The aggregate figures of every planned suite.
    pub fn figures(&self) -> BTreeMap<Suite, SuiteFigures> {
        self.suites
            .iter()
            .map(|(&suite, acc)| (suite, self.suite_figures(acc)))
            .collect()
    }

    fn suite_figures(&self, acc: &SuiteAcc) -> SuiteFigures {
        let played: Vec<(&MatchStats, Option<usize>)> = acc
            .records
            .iter()
            .zip(acc.boosted.iter().copied())
            .filter(|(r, _)| r.outcome == "success")
            .collect();
        let per_match = |f: &dyn Fn(&MatchStats) -> f64| -> Vec<f64> {
            played.iter().map(|(r, _)| f(r)).collect()
        };
        let goals = per_match(&|r| f64::from(r.goals[0] + r.goals[1]));
        let per_team = |f: &dyn Fn(&MatchStats, usize) -> f64| -> f64 {
            mean(
                &played
                    .iter()
                    .flat_map(|(r, _)| [f(r, 0), f(r, 1)])
                    .collect::<Vec<f64>>(),
            )
        };
        let band = self.bands.possession_pct;
        let in_band = per_match(&|r| {
            f64::from(u8::from(
                r.figures.possession_pct.iter().all(|&p| band.contains(p)),
            ))
        });
        let stronger = acc.boosted.iter().any(Option::is_some).then(|| {
            let mut rec = StrongerRecord::default();
            for (r, side) in &played {
                let Some(side) = *side else { continue };
                let (mine, theirs) = (r.goals[side], r.goals[1 - side]);
                match mine.cmp(&theirs) {
                    std::cmp::Ordering::Greater => rec.wins += 1,
                    std::cmp::Ordering::Equal => rec.draws += 1,
                    std::cmp::Ordering::Less => rec.losses += 1,
                }
            }
            let n = rec.wins + rec.draws + rec.losses;
            rec.win_rate = if n == 0 {
                0.0
            } else {
                round_to(f64::from(rec.wins) / f64::from(n), 4)
            };
            rec
        });
        SuiteFigures {
            matches: acc.matches,
            // A suite holds at most 100 000 matches.
            recorded: acc.records.len() as u32,
            failures: acc
                .records
                .iter()
                .filter(|r| r.outcome != "success")
                .count() as u32,
            outliers: acc.outliers,
            goals_per_match_mean: round_to(mean(&goals), 3),
            goals_per_match_sd: round_to(sd(&goals), 3),
            shots_per_team_mean: round_to(per_team(&|r, t| f64::from(r.tactics.shots[t])), 3),
            shots_on_target_per_team_mean: round_to(
                per_team(&|r, t| f64::from(r.figures.shots_on_target[t])),
                3,
            ),
            xg_per_match_mean: round_to(
                mean(&per_match(&|r| r.figures.xg[0] + r.figures.xg[1])),
                3,
            ),
            passes_per_team_mean: round_to(per_team(&|r, t| f64::from(r.figures.passes[t])), 3),
            pass_accuracy_pct_mean: round_to(per_team(&|r, t| r.figures.pass_accuracy_pct[t]), 3),
            possession_home_mean: round_to(mean(&per_match(&|r| r.figures.possession_pct[0])), 3),
            possession_away_mean: round_to(mean(&per_match(&|r| r.figures.possession_pct[1])), 3),
            possession_in_band_share: round_to(mean(&in_band), 4),
            fouls_per_team_mean: round_to(per_team(&|r, t| f64::from(r.laws.fouls[t])), 3),
            stronger,
        }
    }

    /// The band checks of the planned suites, given each suite's wall time.
    pub fn checks(
        &self,
        figures: &BTreeMap<Suite, SuiteFigures>,
        wall_ms: &BTreeMap<Suite, u64>,
    ) -> Vec<BandCheck> {
        let b = &self.bands;
        let mut out = Vec::new();
        let mut check = |band: &str, suite: Suite, value: f64, lo: f64, hi: f64, pass: bool| {
            out.push(BandCheck {
                band: band.to_string(),
                suite: suite.code().to_string(),
                value,
                lo,
                hi,
                pass,
            });
        };
        if let Some(f) = figures.get(&Suite::Equal) {
            let within = |band: bands::Band, v: f64| band.contains(v);
            let g = b.goals_per_match;
            check(
                "goals_per_match",
                Suite::Equal,
                f.goals_per_match_mean,
                g.lo,
                g.hi,
                within(g, f.goals_per_match_mean),
            );
            let s = b.shots_per_team;
            check(
                "shots_per_team",
                Suite::Equal,
                f.shots_per_team_mean,
                s.lo,
                s.hi,
                within(s, f.shots_per_team_mean),
            );
            let p = b.possession_pct;
            check(
                "possession_home_pct",
                Suite::Equal,
                f.possession_home_mean,
                p.lo,
                p.hi,
                within(p, f.possession_home_mean),
            );
            check(
                "possession_away_pct",
                Suite::Equal,
                f.possession_away_mean,
                p.lo,
                p.hi,
                within(p, f.possession_away_mean),
            );
        }
        if let Some(rec) = figures.get(&Suite::Strength).and_then(|f| f.stronger) {
            let lo = b.stronger_team.min_win_rate;
            check(
                "stronger_team_win_rate",
                Suite::Strength,
                rec.win_rate,
                lo,
                1.0,
                rec.win_rate > lo,
            );
        }
        for (&suite, &ms) in wall_ms {
            let matches = figures.get(&suite).map_or(0, |f| f.matches);
            let budget = b.wall_budget_ms(matches) as f64;
            check(
                "wall_ms",
                suite,
                ms as f64,
                0.0,
                budget,
                (ms as f64) < budget,
            );
        }
        out
    }

    /// The sum of `darkpath.change_never_applied` over every record.
    pub fn change_never_applied(&self) -> u32 {
        self.records().map(|r| r.tactics.change_never_applied).sum()
    }

    /// The sum of `change.expired_at_full_time` over every record.
    pub fn change_expired_at_full_time(&self) -> u32 {
        self.records()
            .map(|r| r.tactics.change_expired_at_full_time)
            .sum()
    }

    /// The sum of validator violations over every record.
    pub fn violations(&self) -> usize {
        self.records().map(|r| r.validate_violations).sum()
    }

    fn records(&self) -> impl Iterator<Item = &MatchStats> {
        self.suites.values().flat_map(|a| a.records.iter())
    }
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        0.0
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}

/// The population standard deviation.
fn sd(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let m = mean(xs);
    (xs.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / xs.len() as f64).sqrt()
}

/// One `run-report` record of a calibration run.
#[derive(Debug, Serialize)]
pub struct CalibrationReport {
    #[serde(rename = "owner.id")]
    pub owner_id: String,
    #[serde(rename = "run.id")]
    pub run_id: String,
    pub seed: u64,
    #[serde(rename = "content.hash")]
    pub content_hash: String,
    pub outcome: &'static str,
    pub duration_ms: u64,
    /// Matches per suite.
    #[serde(rename = "calib.matches")]
    pub matches: u32,
    #[serde(rename = "calib.minutes")]
    pub minutes: u32,
    #[serde(rename = "calib.jobs")]
    pub jobs: u32,
    #[serde(rename = "calib.suites")]
    pub suites: BTreeMap<String, SuiteFigures>,
    /// Wall time of each suite and of the whole run (`total`), in milliseconds.
    #[serde(rename = "calib.wall_ms")]
    pub wall_ms: BTreeMap<String, u64>,
    #[serde(rename = "calib.bands")]
    pub bands: Vec<BandCheck>,
    /// Every band, both dark paths, and the time budget pass.
    #[serde(rename = "calib.pass")]
    pub pass: bool,
    #[serde(rename = "bench.matches")]
    pub bench_matches: u32,
    /// The single-thread median wall time of one match on the default teams.
    #[serde(rename = "bench.match_wall_ms")]
    pub match_wall_ms: u64,
    #[serde(rename = "bench.ticks_per_match")]
    pub ticks_per_match: u32,
    #[serde(rename = "bench.cpu_us_per_tick")]
    pub cpu_us_per_tick: Option<f64>,
    #[serde(rename = "bench.cpu_ms")]
    pub cpu_ms: Option<u64>,
    #[serde(rename = "bench.peak_mem_mb")]
    pub peak_mem_mb: Option<f64>,
    #[serde(rename = "darkpath.change_never_applied")]
    pub change_never_applied: u32,
    /// Changes still waiting at full time with no admitting stoppage after them. Reported,
    /// with no zero rule.
    #[serde(rename = "change.expired_at_full_time")]
    pub change_expired_at_full_time: u32,
    #[serde(rename = "darkpath.match_without_stats")]
    pub match_without_stats: u32,
    #[serde(rename = "validate.violations")]
    pub violations: usize,
    #[serde(rename = "events.files_written")]
    pub events_written: u32,
    #[serde(rename = "events.files_kept")]
    pub events_kept: u32,
    #[serde(rename = "machine.hash")]
    pub machine_hash: String,
    #[serde(rename = "machine.cpu_model")]
    pub cpu_model: String,
}

impl Record for CalibrationReport {
    fn kind(&self) -> &'static str {
        "run-report"
    }
    fn operation(&self) -> &'static str {
        "calibrate"
    }
    fn owner_id(&self) -> &str {
        &self.owner_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::observe::{LawStats, MatchFigures, TacticsStats, TeamRef};
    use std::path::Path;

    fn bands() -> Bands {
        let dir =
            engine::ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"));
        Bands::load(&dir).unwrap()
    }

    fn record(goals: [u32; 2], shots: [u32; 2], possession: [f64; 2]) -> MatchStats {
        MatchStats {
            owner_id: "0123456789abcdef0123456789abcdef".into(),
            match_id: "0000000000000001-1".into(),
            seed: 1,
            content_hash: "abcdef012345".into(),
            teams: [
                TeamRef {
                    id: "a".into(),
                    name: "A".into(),
                },
                TeamRef {
                    id: "b".into(),
                    name: "B".into(),
                },
            ],
            duration_ms: 400,
            outcome: "success".into(),
            ticks_per_s: 1.0,
            ticks_written: 1,
            validate_ran: true,
            validate_violations: 0,
            possession_changes: 0,
            ball_max_speed: 0.0,
            ball_idle_ticks: 0,
            goals,
            laws: LawStats::default(),
            tactics: TacticsStats {
                shots,
                ..TacticsStats::default()
            },
            figures: MatchFigures {
                goals,
                possession_pct: possession,
                ..MatchFigures::default()
            },
        }
    }

    #[test]
    fn the_equal_suite_aggregates_and_checks_every_band() {
        let mut b = RunBuilder::new(bands());
        b.plan(Suite::Equal, 3);
        assert!(!b.add(Suite::Equal, record([2, 1], [12, 10], [52.0, 48.0]), None));
        assert!(!b.add(Suite::Equal, record([1, 2], [9, 14], [45.0, 55.0]), None));
        b.add_missing();
        let figures = b.figures();
        let f = &figures[&Suite::Equal];
        assert_eq!(f.matches, 3);
        assert_eq!(f.recorded, 2);
        assert_eq!(f.goals_per_match_mean, 3.0);
        assert_eq!(f.goals_per_match_sd, 0.0);
        assert_eq!(f.shots_per_team_mean, 11.25);
        assert_eq!(f.possession_home_mean, 48.5);
        assert_eq!(f.possession_in_band_share, 1.0);
        assert_eq!(b.missing, 1);
        let wall = BTreeMap::from([(Suite::Equal, 1_000)]);
        let checks = b.checks(&figures, &wall);
        let names: Vec<&str> = checks.iter().map(|c| c.band.as_str()).collect();
        assert_eq!(
            names,
            [
                "goals_per_match",
                "shots_per_team",
                "possession_home_pct",
                "possession_away_pct",
                "wall_ms"
            ]
        );
        assert!(checks.iter().all(|c| c.pass), "{checks:?}");
    }

    #[test]
    fn a_band_miss_and_an_outlier_are_named() {
        let mut b = RunBuilder::new(bands());
        b.plan(Suite::Equal, 1);
        // Six shots and 70 percent possession: an outlier, and both bands miss.
        assert!(b.add(Suite::Equal, record([5, 1], [6, 6], [70.0, 30.0]), None));
        let figures = b.figures();
        let checks = b.checks(&figures, &BTreeMap::new());
        let failed: Vec<&str> = checks
            .iter()
            .filter(|c| !c.pass)
            .map(|c| c.band.as_str())
            .collect();
        assert_eq!(
            failed,
            [
                "goals_per_match",
                "shots_per_team",
                "possession_home_pct",
                "possession_away_pct"
            ]
        );
        assert_eq!(figures[&Suite::Equal].outliers, 1);
    }

    #[test]
    fn the_strength_suite_counts_the_stronger_side_and_skips_failures() {
        let mut b = RunBuilder::new(bands());
        b.plan(Suite::Strength, 4);
        b.add(
            Suite::Strength,
            record([2, 0], [10, 10], [50.0, 50.0]),
            Some(0),
        );
        b.add(
            Suite::Strength,
            record([2, 0], [10, 10], [50.0, 50.0]),
            Some(1),
        );
        b.add(
            Suite::Strength,
            record([1, 1], [10, 10], [50.0, 50.0]),
            Some(0),
        );
        let mut failed = record([0, 0], [0, 0], [0.0, 0.0]);
        failed.outcome = "failure".into();
        assert!(b.add(Suite::Strength, failed, Some(1)));
        let figures = b.figures();
        let rec = figures[&Suite::Strength].stronger.unwrap();
        assert_eq!((rec.wins, rec.draws, rec.losses), (1, 1, 1));
        assert_eq!(rec.win_rate, 0.3333);
        assert_eq!(figures[&Suite::Strength].failures, 1);
        let checks = b.checks(&figures, &BTreeMap::new());
        assert_eq!(checks.len(), 1);
        assert!(!checks[0].pass);
    }

    #[test]
    fn a_suite_slower_than_its_budget_fails_the_wall_band() {
        let mut b = RunBuilder::new(bands());
        b.plan(Suite::Equal, 1000);
        let figures = b.figures();
        let wall = BTreeMap::from([(Suite::Equal, 1_800_001)]);
        let checks = b.checks(&figures, &wall);
        let time = checks.iter().find(|c| c.band == "wall_ms").unwrap();
        assert_eq!(time.hi, 1_800_000.0);
        assert!(!time.pass);
    }
}

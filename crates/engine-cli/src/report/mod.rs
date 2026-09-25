//! The run-scoped builder of a calibration run: it folds every `match-stats` record of a
//! suite into the aggregate figures, checks them against the accepted realism bands, sums
//! the dark paths, and names the outlier matches whose event files are kept. The formations
//! suite is also folded per formation pairing, and the red-card suite per arm. Every band
//! check carries its sampling error.

pub mod bands;
pub mod baseline;
pub mod compare;

use std::collections::BTreeMap;

use engine::observe::{MatchStats, Record, round_to};
use serde::{Deserialize, Serialize};

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
    /// Every formation pairing, with the clubs of the equal suite.
    Formations,
    /// The controlled sending-off experiment on the default clubs, cards otherwise off. It
    /// is not part of [`Suite::ALL`].
    RedCard,
}

impl Suite {
    /// The suites `--suite all` plays. The red-card suite runs only when named.
    pub const ALL: [Suite; 3] = [Suite::Equal, Suite::Strength, Suite::Formations];

    pub fn code(self) -> &'static str {
        match self {
            Suite::Equal => "equal",
            Suite::Strength => "strength",
            Suite::Formations => "formations",
            Suite::RedCard => "red-card",
        }
    }

    /// The suite's number in match seeds and match identifiers.
    pub fn number(self) -> u64 {
        match self {
            Suite::Equal => 0,
            Suite::Strength => 1,
            Suite::Formations => 2,
            Suite::RedCard => 3,
        }
    }
}

/// The arms of the red-card suite: the name, and the away player sent off at kick-off
/// (none for the control). The home side is the full side.
pub const RED_CARD_ARMS: [(&str, Option<usize>); 4] = [
    ("control", None),
    ("keeper", Some(11)),
    ("centre-back", Some(13)),
    ("striker", Some(21)),
];

/// The full side may score at most this multiple of the control's home mean.
pub const RED_CARD_LIMIT: f64 = 1.6;

/// The lowest value a red-card `reduced_minus_full` row shows; only its top (0) is judged.
const RED_CARD_FLOOR: f64 = -10.0;

/// The suites that check `band`, or `None` for a band no suite checks. `--band` narrows a
/// run to these suites.
pub fn band_suites(band: &str) -> Option<&'static [Suite]> {
    const EQUAL: &[Suite] = &[Suite::Equal];
    const GOALS: &[Suite] = &[Suite::Equal, Suite::Formations];
    Some(match band {
        "goals_per_match" | "ten_plus_goals_share" | "goalless_share" => GOALS,
        "shots_per_team"
        | "possession_home_pct"
        | "possession_away_pct"
        | "sending_off_share"
        | "yellow_cards_per_team"
        | "shots_on_target_share"
        | "goals_per_xg"
        | "passes_per_team"
        | "pass_accuracy_pct"
        | "corners_per_team"
        | "throw_ins_per_match"
        | "goal_kicks_per_match" => EQUAL,
        "stronger_team_win_rate" => &[Suite::Strength],
        "reduced_minus_full" | "full_over_control" => &[Suite::RedCard],
        _ => return None,
    })
}

/// Every band name `--band` accepts.
pub const BAND_NAMES: [&str; 18] = [
    "goals_per_match",
    "ten_plus_goals_share",
    "goalless_share",
    "shots_per_team",
    "possession_home_pct",
    "possession_away_pct",
    "sending_off_share",
    "yellow_cards_per_team",
    "shots_on_target_share",
    "goals_per_xg",
    "passes_per_team",
    "pass_accuracy_pct",
    "corners_per_team",
    "throw_ins_per_match",
    "goal_kicks_per_match",
    "stronger_team_win_rate",
    "reduced_minus_full",
    "full_over_control",
];

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
    /// The share of matches with 10 or more goals.
    pub ten_plus_goals_share: f64,
    /// The share of matches that end 0-0.
    pub goalless_share: f64,
    /// The share of matches with at least one player sent off.
    pub sending_off_share: f64,
    /// Yellow cards per team, a second yellow included.
    pub yellow_cards_per_team_mean: f64,
    /// Shots on target over all shots, pooled over the suite.
    pub shots_on_target_share: f64,
    /// Goals over expected goals, pooled over the suite.
    pub goals_per_xg: f64,
    pub corners_per_team_mean: f64,
    pub throw_ins_per_match_mean: f64,
    pub goal_kicks_per_match_mean: f64,
    /// Minutes with the ball in play per 90 minutes of match time: each match's
    /// `stats.ball_in_play_s` over 60, times 90 over the match length. A figure, not a band.
    pub ball_in_play_min_per_90_mean: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stronger: Option<StrongerRecord>,
}

/// The figures of one formation pairing of the formations suite. `pairing` names the two
/// formations; `goals_for_mean` is the goals each one scored per match, in the same order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PairingFigures {
    pub pairing: [String; 2],
    /// Matches that played to the end.
    pub matches: u32,
    pub goals_per_match_mean: f64,
    pub goals_for_mean: [f64; 2],
    pub ten_plus_goals_share: f64,
    pub goalless_share: f64,
}

impl PairingFigures {
    /// The label of the pairing in a band check, for example `4-3-3 v 4-4-2`.
    pub fn label(&self) -> String {
        format!("{} v {}", self.pairing[0], self.pairing[1])
    }
}

/// One band judged: `pass` when `value` lies in `lo` to `hi` (the win rate must exceed
/// `lo`, the wall time stay below `hi`, and a red-card row stay at or below `hi`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BandCheck {
    pub band: String,
    pub suite: String,
    /// The formation pairing, for a check of the formations suite, or the arm, for a check
    /// of the red-card suite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pairing: Option<String>,
    pub value: f64,
    pub lo: f64,
    pub hi: f64,
    pub pass: bool,
    /// The sampling error of `value`: the standard error of a mean, share, or ratio over
    /// the matches judged. 0 for the time budget.
    pub se: f64,
}

/// One arm of the red-card suite: mean goals per match of the full (home) side and the
/// reduced (away) side.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RedCardArm {
    pub arm: String,
    /// The roster number of the away player sent off at kick-off.
    pub player: usize,
    /// Matches that played to the end.
    pub matches: u32,
    pub full: f64,
    pub reduced: f64,
    /// The reduced side does not outscore the full side, and the full side stays within the
    /// limit.
    pub pass: bool,
}

/// The figures of the red-card suite, rounded to 4 decimals, and the criterion verdict.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RedCardFigures {
    /// Matches planned per arm.
    pub matches: u32,
    /// The control's mean goals per match, home then away.
    pub control: [f64; 2],
    /// Control matches that played to the end.
    pub control_matches: u32,
    pub arms: Vec<RedCardArm>,
    /// [`RED_CARD_LIMIT`] times the control's home mean.
    pub limit: f64,
    pub pass: bool,
}

/// Every record of one suite, as the builder keeps it.
#[derive(Debug, Default)]
struct SuiteAcc {
    matches: u32,
    records: Vec<MatchStats>,
    /// For each record of the strength suite: the side of the boosted club.
    boosted: Vec<Option<usize>>,
    /// For each record of the formations suite: the pairing, and the side of its first
    /// formation.
    pairings: Vec<Option<(usize, usize)>>,
    /// For each record of the red-card suite: the arm's number in [`RED_CARD_ARMS`].
    arms: Vec<Option<usize>>,
    outliers: u32,
}

/// The run-scoped builder.
#[derive(Debug)]
pub struct RunBuilder {
    bands: Bands,
    suites: BTreeMap<Suite, SuiteAcc>,
    /// The formation names of each pairing of the formations suite.
    pairing_names: Vec<[String; 2]>,
    /// Statistics records that never appeared (`darkpath.match_without_stats`).
    pub missing: u32,
    /// The length of every match, in minutes.
    pub minutes: u32,
}

impl RunBuilder {
    pub fn new(bands: Bands) -> Self {
        Self {
            bands,
            suites: BTreeMap::new(),
            pairing_names: Vec::new(),
            missing: 0,
            minutes: 90,
        }
    }

    /// The formations suite plays `names.len()` pairings of `matches` matches each.
    pub fn plan_pairings(&mut self, matches: u32, names: Vec<[String; 2]>) {
        // At most 16 formations give 136 pairings.
        self.plan(Suite::Formations, matches * names.len() as u32);
        self.pairing_names = names;
    }

    /// `suite` planned `matches` matches.
    pub fn plan(&mut self, suite: Suite, matches: u32) {
        self.suites.entry(suite).or_default().matches = matches;
    }

    /// Folds one record in. `boosted` names the stronger club's side in the strength suite.
    /// Returns `true` when the match is an outlier.
    pub fn add(&mut self, suite: Suite, record: MatchStats, boosted: Option<usize>) -> bool {
        self.push(suite, record, boosted, None, None)
    }

    /// Folds one record of the red-card suite in, for arm `arm` of [`RED_CARD_ARMS`].
    /// Returns `true` when the match is an outlier.
    pub fn add_red_card(&mut self, record: MatchStats, arm: usize) -> bool {
        self.push(Suite::RedCard, record, None, None, Some(arm))
    }

    /// Folds one record of the formations suite in: pairing `pairing`, with its first
    /// formation on side `first_side`. Returns `true` when the match is an outlier.
    pub fn add_pairing(&mut self, record: MatchStats, pairing: usize, first_side: usize) -> bool {
        self.push(
            Suite::Formations,
            record,
            None,
            Some((pairing, first_side)),
            None,
        )
    }

    fn push(
        &mut self,
        suite: Suite,
        record: MatchStats,
        boosted: Option<usize>,
        pairing: Option<(usize, usize)>,
        arm: Option<usize>,
    ) -> bool {
        let outlier = self.outlier(&record);
        let acc = self.suites.entry(suite).or_default();
        acc.outliers += u32::from(outlier);
        acc.records.push(record);
        acc.boosted.push(boosted);
        acc.pairings.push(pairing);
        acc.arms.push(arm);
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
        let share = |f: &dyn Fn(&MatchStats) -> bool| -> f64 {
            round_to(mean(&per_match(&|r| f64::from(u8::from(f(r))))), 4)
        };
        let total = |f: &dyn Fn(&MatchStats) -> f64| -> f64 { per_match(f).iter().sum() };
        let both = |a: [u32; 2]| f64::from(a[0] + a[1]);
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
            ten_plus_goals_share: share(&|r| r.goals[0] + r.goals[1] >= 10),
            goalless_share: share(&|r| r.goals == [0, 0]),
            sending_off_share: share(&|r| r.laws.red[0] + r.laws.red[1] > 0),
            yellow_cards_per_team_mean: round_to(per_team(&|r, t| f64::from(r.laws.yellow[t])), 3),
            shots_on_target_share: round_to(
                ratio(
                    total(&|r| both(r.figures.shots_on_target)),
                    total(&|r| both(r.tactics.shots)),
                ),
                4,
            ),
            goals_per_xg: round_to(
                ratio(
                    total(&|r| both(r.goals)),
                    total(&|r| r.figures.xg[0] + r.figures.xg[1]),
                ),
                4,
            ),
            corners_per_team_mean: round_to(per_team(&|r, t| f64::from(r.laws.corners[t])), 3),
            throw_ins_per_match_mean: round_to(mean(&per_match(&|r| both(r.laws.throw_ins))), 3),
            goal_kicks_per_match_mean: round_to(mean(&per_match(&|r| both(r.laws.goal_kicks))), 3),
            ball_in_play_min_per_90_mean: round_to(
                mean(&per_match(&|r| {
                    f64::from(r.figures.ball_in_play_s) / 60.0 * 90.0
                        / f64::from(self.minutes.max(1))
                })),
                3,
            ),
            stronger,
        }
    }

    /// The figures of every pairing of the formations suite, in pairing order. A pairing
    /// with no finished match has zero figures.
    pub fn pairing_figures(&self) -> Vec<PairingFigures> {
        let Some(acc) = self.suites.get(&Suite::Formations) else {
            return Vec::new();
        };
        self.pairing_names
            .iter()
            .enumerate()
            .map(|(i, names)| {
                let played = pairing_records(acc, i);
                let per_match = |f: &dyn Fn(&MatchStats, usize) -> f64| -> f64 {
                    let xs: Vec<f64> = played.iter().map(|(r, side)| f(r, *side)).collect();
                    mean(&xs)
                };
                let share = |f: &dyn Fn(&MatchStats) -> bool| -> f64 {
                    round_to(per_match(&|r, _| f64::from(u8::from(f(r)))), 4)
                };
                PairingFigures {
                    pairing: names.clone(),
                    // A pairing holds at most 100 000 matches.
                    matches: played.len() as u32,
                    goals_per_match_mean: round_to(
                        per_match(&|r, _| f64::from(r.goals[0] + r.goals[1])),
                        3,
                    ),
                    goals_for_mean: [
                        round_to(per_match(&|r, side| f64::from(r.goals[side])), 3),
                        round_to(per_match(&|r, side| f64::from(r.goals[1 - side])), 3),
                    ],
                    ten_plus_goals_share: share(&|r| r.goals[0] + r.goals[1] >= 10),
                    goalless_share: share(&|r| r.goals == [0, 0]),
                }
            })
            .collect()
    }

    /// The finished records of each red-card arm, as goals per match, home then away.
    fn red_card_goals(&self) -> Option<[Vec<[f64; 2]>; 4]> {
        let acc = self.suites.get(&Suite::RedCard)?;
        Some(std::array::from_fn(|arm| {
            acc.records
                .iter()
                .zip(&acc.arms)
                .filter(|(r, a)| **a == Some(arm) && r.outcome == "success")
                .map(|(r, _)| r.goals.map(f64::from))
                .collect()
        }))
    }

    /// The figures of the red-card suite, or `None` when it did not run. The arithmetic is
    /// the slow test's: mean goals per match over the finished matches.
    pub fn red_card_figures(&self) -> Option<RedCardFigures> {
        let planned = self.suites.get(&Suite::RedCard)?.matches;
        let goals = self.red_card_goals()?;
        let side = |g: &[[f64; 2]], t: usize| mean(&g.iter().map(|m| m[t]).collect::<Vec<_>>());
        let control_home = side(&goals[0], 0);
        let limit = RED_CARD_LIMIT * control_home;
        let arms: Vec<RedCardArm> = RED_CARD_ARMS
            .iter()
            .zip(&goals)
            .filter_map(|((name, player), g)| {
                let player = (*player)?;
                let (full, reduced) = (side(g, 0), side(g, 1));
                Some(RedCardArm {
                    arm: (*name).to_string(),
                    player,
                    // An arm holds at most 100 000 matches.
                    matches: g.len() as u32,
                    full: round_to(full, 4),
                    reduced: round_to(reduced, 4),
                    pass: reduced <= full && full <= limit,
                })
            })
            .collect();
        Some(RedCardFigures {
            matches: planned / RED_CARD_ARMS.len() as u32,
            control: [round_to(control_home, 4), round_to(side(&goals[0], 1), 4)],
            control_matches: goals[0].len() as u32,
            pass: arms.iter().all(|a| a.pass),
            arms,
            limit: round_to(limit, 4),
        })
    }

    /// The two rows of each red-card arm: `reduced_minus_full` (at most 0) and
    /// `full_over_control` (at most [`RED_CARD_LIMIT`]), each with its sampling error.
    fn red_card_checks(&self) -> Vec<BandCheck> {
        let Some(goals) = self.red_card_goals() else {
            return Vec::new();
        };
        let control: Vec<f64> = goals[0].iter().map(|g| g[0]).collect();
        let (c, se_c) = (mean(&control), se_mean(&control));
        let mut out = Vec::new();
        for ((name, player), g) in RED_CARD_ARMS.iter().zip(&goals) {
            if player.is_none() {
                continue;
            }
            let full: Vec<f64> = g.iter().map(|m| m[0]).collect();
            let diff: Vec<f64> = g.iter().map(|m| m[1] - m[0]).collect();
            let (f, se_f) = (mean(&full), se_mean(&full));
            let d = mean(&diff);
            let row = |band: &str, value: f64, lo: f64, hi: f64, pass: bool, se: f64| BandCheck {
                band: band.to_string(),
                suite: Suite::RedCard.code().to_string(),
                pairing: Some((*name).to_string()),
                value: round_to(value, 4),
                lo,
                hi,
                pass,
                se: round_to(se, 5),
            };
            out.push(row(
                "reduced_minus_full",
                d,
                RED_CARD_FLOOR,
                0.0,
                d <= 0.0,
                se_mean(&diff),
            ));
            let r = ratio(f, c);
            // The delta-method error of a ratio of two means.
            let se_r = if f > 0.0 && c > 0.0 {
                r * ((se_f / f).powi(2) + (se_c / c).powi(2)).sqrt()
            } else {
                0.0
            };
            out.push(row(
                "full_over_control",
                r,
                0.0,
                RED_CARD_LIMIT,
                f <= RED_CARD_LIMIT * c,
                se_r,
            ));
        }
        out
    }

    /// The finished records of `suite`.
    fn played(&self, suite: Suite) -> Vec<&MatchStats> {
        self.suites.get(&suite).map_or_else(Vec::new, |acc| {
            acc.records
                .iter()
                .filter(|r| r.outcome == "success")
                .collect()
        })
    }

    /// The sampling error of each band of the equal suite.
    fn equal_errors(&self) -> BTreeMap<&'static str, f64> {
        let played = self.played(Suite::Equal);
        let n = played.len();
        let per_match = |f: &dyn Fn(&MatchStats) -> f64| -> f64 {
            se_mean(&played.iter().map(|r| f(r)).collect::<Vec<_>>())
        };
        let per_team = |f: &dyn Fn(&MatchStats, usize) -> f64| -> f64 {
            per_match(&|r| (f(r, 0) + f(r, 1)) / 2.0)
        };
        let share = |f: &dyn Fn(&MatchStats) -> bool| -> f64 {
            let hits: Vec<f64> = played.iter().map(|r| f64::from(u8::from(f(r)))).collect();
            se_share(mean(&hits), n)
        };
        let both = |a: [u32; 2]| f64::from(a[0] + a[1]);
        let pooled = |y: &dyn Fn(&MatchStats) -> f64, x: &dyn Fn(&MatchStats) -> f64| -> f64 {
            let ys: Vec<f64> = played.iter().map(|r| y(r)).collect();
            let xs: Vec<f64> = played.iter().map(|r| x(r)).collect();
            se_ratio(&ys, &xs)
        };
        BTreeMap::from([
            (
                "goals_per_match",
                per_match(&|r| f64::from(r.goals[0] + r.goals[1])),
            ),
            (
                "shots_per_team",
                per_team(&|r, t| f64::from(r.tactics.shots[t])),
            ),
            (
                "possession_home_pct",
                per_match(&|r| r.figures.possession_pct[0]),
            ),
            (
                "possession_away_pct",
                per_match(&|r| r.figures.possession_pct[1]),
            ),
            (
                "ten_plus_goals_share",
                share(&|r| r.goals[0] + r.goals[1] >= 10),
            ),
            (
                "sending_off_share",
                share(&|r| r.laws.red[0] + r.laws.red[1] > 0),
            ),
            (
                "yellow_cards_per_team",
                per_team(&|r, t| f64::from(r.laws.yellow[t])),
            ),
            (
                "shots_on_target_share",
                pooled(&|r| both(r.figures.shots_on_target), &|r| {
                    both(r.tactics.shots)
                }),
            ),
            (
                "goals_per_xg",
                pooled(&|r| both(r.goals), &|r| r.figures.xg[0] + r.figures.xg[1]),
            ),
            (
                "passes_per_team",
                per_team(&|r, t| f64::from(r.figures.passes[t])),
            ),
            // The figure is a mean of each team's percentage, so its error is a mean's.
            (
                "pass_accuracy_pct",
                per_team(&|r, t| r.figures.pass_accuracy_pct[t]),
            ),
            (
                "corners_per_team",
                per_team(&|r, t| f64::from(r.laws.corners[t])),
            ),
            (
                "throw_ins_per_match",
                per_match(&|r| both(r.laws.throw_ins)),
            ),
            (
                "goal_kicks_per_match",
                per_match(&|r| both(r.laws.goal_kicks)),
            ),
            ("goalless_share", share(&|r| r.goals == [0, 0])),
        ])
    }

    /// The band checks of the planned suites, given each suite's wall time.
    pub fn checks(
        &self,
        figures: &BTreeMap<Suite, SuiteFigures>,
        pairings: &[PairingFigures],
        wall_ms: &BTreeMap<Suite, u64>,
    ) -> Vec<BandCheck> {
        let b = &self.bands;
        let errors = self.equal_errors();
        let mut out = Vec::new();
        let mut check = |band: &str, suite: Suite, value: f64, lo: f64, hi: f64, pass: bool| {
            let se = match suite {
                Suite::Equal => errors.get(band).copied().unwrap_or(0.0),
                _ => 0.0,
            };
            out.push(BandCheck {
                band: band.to_string(),
                suite: suite.code().to_string(),
                pairing: None,
                value,
                lo,
                hi,
                pass,
                se: round_to(se, 5),
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
            // The bands of version 2, in the order of the bands file.
            for (name, band, value) in [
                (
                    "ten_plus_goals_share",
                    b.ten_plus_goals_share,
                    f.ten_plus_goals_share,
                ),
                (
                    "sending_off_share",
                    b.sending_off_share,
                    f.sending_off_share,
                ),
                (
                    "yellow_cards_per_team",
                    b.yellow_cards_per_team,
                    f.yellow_cards_per_team_mean,
                ),
                (
                    "shots_on_target_share",
                    b.shots_on_target_share,
                    f.shots_on_target_share,
                ),
                ("goals_per_xg", b.goals_per_xg, f.goals_per_xg),
                ("passes_per_team", b.passes_per_team, f.passes_per_team_mean),
                (
                    "pass_accuracy_pct",
                    b.pass_accuracy_pct,
                    f.pass_accuracy_pct_mean,
                ),
                (
                    "corners_per_team",
                    b.corners_per_team,
                    f.corners_per_team_mean,
                ),
                (
                    "throw_ins_per_match",
                    b.throw_ins_per_match,
                    f.throw_ins_per_match_mean,
                ),
                (
                    "goal_kicks_per_match",
                    b.goal_kicks_per_match,
                    f.goal_kicks_per_match_mean,
                ),
                ("goalless_share", b.goalless_share, f.goalless_share),
            ] {
                check(
                    name,
                    Suite::Equal,
                    value,
                    band.lo,
                    band.hi,
                    within(band, value),
                );
            }
        }
        let mut paired = Vec::new();
        if let Some(rec) = figures.get(&Suite::Strength).and_then(|f| f.stronger) {
            let lo = b.stronger_team.min_win_rate;
            let n = (rec.wins + rec.draws + rec.losses) as usize;
            // After the time budgets; the parent sorts every row by suite, band and pairing.
            paired.push(BandCheck {
                band: "stronger_team_win_rate".to_string(),
                suite: Suite::Strength.code().to_string(),
                pairing: None,
                value: rec.win_rate,
                lo,
                hi: 1.0,
                pass: rec.win_rate > lo,
                se: round_to(se_share(rec.win_rate, n), 5),
            });
        }
        let formations = self.suites.get(&Suite::Formations);
        for (i, p) in pairings.iter().enumerate() {
            let label = p.label();
            let played = formations.map_or_else(Vec::new, |acc| pairing_records(acc, i));
            let n = played.len();
            let goals: Vec<f64> = played
                .iter()
                .map(|(r, _)| f64::from(r.goals[0] + r.goals[1]))
                .collect();
            for (name, band, value, se) in [
                (
                    "goals_per_match",
                    b.goals_per_match,
                    p.goals_per_match_mean,
                    se_mean(&goals),
                ),
                (
                    "ten_plus_goals_share",
                    b.ten_plus_goals_share,
                    p.ten_plus_goals_share,
                    se_share(p.ten_plus_goals_share, n),
                ),
                (
                    "goalless_share",
                    b.goalless_share,
                    p.goalless_share,
                    se_share(p.goalless_share, n),
                ),
            ] {
                paired.push(BandCheck {
                    band: name.to_string(),
                    suite: Suite::Formations.code().to_string(),
                    pairing: Some(label.clone()),
                    value,
                    lo: band.lo,
                    hi: band.hi,
                    pass: band.contains(value),
                    se: round_to(se, 5),
                });
            }
        }
        paired.extend(self.red_card_checks());
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
        out.extend(paired);
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

/// The finished records of pairing `i` of the formations suite, each with the side of its
/// first formation.
fn pairing_records(acc: &SuiteAcc, i: usize) -> Vec<(&MatchStats, usize)> {
    acc.records
        .iter()
        .zip(&acc.pairings)
        .filter_map(|(r, p)| match p {
            Some((pairing, side)) if *pairing == i && r.outcome == "success" => Some((r, *side)),
            _ => None,
        })
        .collect()
}

/// The standard error of the mean of `xs`: sd / sqrt(n). 0 below two values.
pub fn se_mean(xs: &[f64]) -> f64 {
    if xs.len() < 2 {
        0.0
    } else {
        sd(xs) / (xs.len() as f64).sqrt()
    }
}

/// The standard error of a share `p` over `n` matches: sqrt(p (1 - p) / n).
pub fn se_share(p: f64, n: usize) -> f64 {
    if n == 0 {
        0.0
    } else {
        (p * (1.0 - p) / n as f64).max(0.0).sqrt()
    }
}

/// The standard error of the pooled ratio sum(y) / sum(x), by the ratio estimator:
/// sqrt(sum((y - R x)^2) / (n (n - 1))) / mean(x). 0 below two values or with no `x`.
pub fn se_ratio(ys: &[f64], xs: &[f64]) -> f64 {
    let n = ys.len().min(xs.len());
    let mx = mean(xs);
    if n < 2 || mx <= 0.0 {
        return 0.0;
    }
    let r = ys.iter().sum::<f64>() / xs.iter().sum::<f64>();
    let ss: f64 = ys.iter().zip(xs).map(|(y, x)| (y - r * x).powi(2)).sum();
    (ss / (n as f64 * (n as f64 - 1.0))).sqrt() / mx
}

/// `num` over `den`, or 0 when `den` is not positive, so a figure is never `NaN` (which
/// the record would write as `null`).
fn ratio(num: f64, den: f64) -> f64 {
    if den > 0.0 { num / den } else { 0.0 }
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
    /// SHA-256 over the inputs that decide the fixtures: the attributes, rules and tactics
    /// content, the generator block, and the two default clubs. A baseline must share it.
    #[serde(rename = "fixtures.hash")]
    pub fixtures_hash: String,
    pub outcome: &'static str,
    /// Present only when `outcome` is `error`.
    #[serde(rename = "error.type", skip_serializing_if = "Option::is_none")]
    pub error_type: Option<&'static str>,
    #[serde(rename = "error.code", skip_serializing_if = "Option::is_none")]
    pub error_code: Option<&'static str>,
    #[serde(rename = "error.retriable", skip_serializing_if = "Option::is_none")]
    pub error_retriable: Option<bool>,
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
    /// The figures of every formation pairing; absent when the formations suite did not run.
    #[serde(rename = "calib.formations", skip_serializing_if = "Vec::is_empty")]
    pub formations: Vec<PairingFigures>,
    /// The suites, pairings, and bands the run selected.
    #[serde(rename = "calib.selection")]
    pub selection: Selection,
    /// The figures of the red-card suite; absent when it did not run.
    #[serde(rename = "calib.red_card", skip_serializing_if = "Option::is_none")]
    pub red_card: Option<RedCardFigures>,
    /// The report the run was compared with; absent without `--baseline`.
    #[serde(rename = "calib.baseline", skip_serializing_if = "Option::is_none")]
    pub baseline: Option<baseline::BaselineInfo>,
    /// One row per band both runs judged, with its change and sampling error.
    #[serde(rename = "calib.diff", skip_serializing_if = "Option::is_none")]
    pub diff: Option<baseline::Diff>,
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
    /// Every declared feature flag with its state for this run. Absent when the tuning file
    /// declares none.
    #[serde(rename = "calib.flags", skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<FlagEntry>,
    /// A paired run: the flag compared. The top-level figures above describe the off arm.
    #[serde(rename = "calib.pair", skip_serializing_if = "Option::is_none")]
    pub pair: Option<PairInfo>,
    /// A paired run: the figures of each arm, under `off` and `on`.
    #[serde(rename = "calib.arms", skip_serializing_if = "Option::is_none")]
    pub arms: Option<BTreeMap<String, ArmReport>>,
    /// A paired run: one row per suite and realism band, both arms side by side.
    #[serde(rename = "calib.compare", skip_serializing_if = "Option::is_none")]
    pub compare: Option<Vec<compare::CompareRow>>,
    #[serde(rename = "calib.verdict", skip_serializing_if = "Option::is_none")]
    pub verdict: Option<compare::Verdict>,
}

/// What a calibration run selected: the suites played, the formation pairings of the
/// formations suite (empty when every pairing played), and the bands judged (empty when
/// every band was judged).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Selection {
    pub suites: Vec<String>,
    pub pairings: Vec<String>,
    pub bands: Vec<String>,
}

/// One declared feature flag in a run report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlagEntry {
    pub name: String,
    pub owner: String,
    /// `on`, `off`, or `paired` for the flag a paired run compares.
    pub state: &'static str,
    /// `file` when the tuning file set the state, `cli` when the command line did.
    pub source: &'static str,
}

/// The flag a paired run compares, with the reasons it exists.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PairInfo {
    pub flag: String,
    pub owner: String,
    pub hypothesis: String,
    pub removal_condition: String,
    /// The other flags the command line set, with their states; both arms use them.
    pub pinned: BTreeMap<String, &'static str>,
}

/// The figures of one arm of a paired run.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ArmReport {
    #[serde(rename = "content.hash")]
    pub content_hash: String,
    #[serde(rename = "tuning.flags_on")]
    pub flags_on: Vec<String>,
    #[serde(rename = "calib.suites")]
    pub suites: BTreeMap<String, SuiteFigures>,
    #[serde(rename = "calib.wall_ms")]
    pub wall_ms: BTreeMap<String, u64>,
    #[serde(rename = "calib.bands")]
    pub bands: Vec<BandCheck>,
    #[serde(rename = "calib.formations", skip_serializing_if = "Vec::is_empty")]
    pub formations: Vec<PairingFigures>,
    #[serde(rename = "calib.pass")]
    pub pass: bool,
    #[serde(rename = "darkpath.change_never_applied")]
    pub change_never_applied: u32,
    #[serde(rename = "change.expired_at_full_time")]
    pub change_expired_at_full_time: u32,
    #[serde(rename = "darkpath.match_without_stats")]
    pub match_without_stats: u32,
    #[serde(rename = "validate.violations")]
    pub violations: usize,
    #[serde(rename = "calib.workers_failed")]
    pub workers_failed: u32,
    #[serde(rename = "bench.match_wall_ms")]
    pub match_wall_ms: u64,
    #[serde(rename = "bench.ticks_per_match")]
    pub ticks_per_match: u32,
    #[serde(rename = "bench.cpu_us_per_tick")]
    pub cpu_us_per_tick: Option<f64>,
    #[serde(rename = "events.files_written")]
    pub events_written: u32,
    #[serde(rename = "events.files_kept")]
    pub events_kept: u32,
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

    /// A finished match with these goals, shots, and possessions. Every other figure is a
    /// realistic value inside its band: 36 percent of shots on target, one expected goal per
    /// goal, 430 passes at 82 percent, 2 yellow cards, 5 corners, 22 throw-ins, and 8 goal
    /// kicks per team, and no card red.
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
            flags_on: Vec::new(),
            laws: LawStats {
                yellow: [2, 2],
                corners: [5, 5],
                throw_ins: [22, 22],
                goal_kicks: [8, 8],
                ..LawStats::default()
            },
            tactics: TacticsStats {
                shots,
                ..TacticsStats::default()
            },
            figures: MatchFigures {
                goals,
                shots_on_target: shots.map(|n| n * 36 / 100),
                xg: goals.map(f64::from),
                passes: [430, 430],
                pass_accuracy_pct: [82.0, 82.0],
                possession_pct: possession,
                ..MatchFigures::default()
            },
            script: Default::default(),
        }
    }

    /// The version-2 bands of the equal suite, in the order of the bands file.
    const V2: [&str; 11] = [
        "ten_plus_goals_share",
        "sending_off_share",
        "yellow_cards_per_team",
        "shots_on_target_share",
        "goals_per_xg",
        "passes_per_team",
        "pass_accuracy_pct",
        "corners_per_team",
        "throw_ins_per_match",
        "goal_kicks_per_match",
        "goalless_share",
    ];

    #[test]
    fn the_equal_suite_aggregates_and_checks_every_band() {
        let mut b = RunBuilder::new(bands());
        b.plan(Suite::Equal, 11);
        // Ten matches: one goalless, one with a sending-off, 30 goals in all.
        let scores = [
            [0, 0],
            [2, 1],
            [1, 2],
            [2, 1],
            [1, 2],
            [3, 0],
            [0, 3],
            [2, 2],
            [3, 1],
            [1, 3],
        ];
        for (i, goals) in scores.into_iter().enumerate() {
            let (shots, possession) = if i % 2 == 0 {
                ([12, 10], [52.0, 48.0])
            } else {
                ([9, 14], [45.0, 55.0])
            };
            let mut r = record(goals, shots, possession);
            if i == 1 {
                r.laws.red = [1, 0];
            }
            assert!(!b.add(Suite::Equal, r, None));
        }
        b.add_missing();
        let figures = b.figures();
        let f = &figures[&Suite::Equal];
        assert_eq!(f.matches, 11);
        assert_eq!(f.recorded, 10);
        assert_eq!(f.goals_per_match_mean, 3.0);
        assert_eq!(f.shots_per_team_mean, 11.25);
        assert_eq!(f.possession_home_mean, 48.5);
        assert_eq!(f.possession_in_band_share, 1.0);
        assert_eq!(f.ten_plus_goals_share, 0.0);
        assert_eq!(f.goalless_share, 0.1);
        assert_eq!(f.sending_off_share, 0.1);
        assert_eq!(f.yellow_cards_per_team_mean, 2.0);
        // 12, 10, 9, and 14 shots give 4, 3, 3, and 5 on target: 75 of 225.
        assert_eq!(f.shots_on_target_share, 0.3333);
        assert_eq!(f.goals_per_xg, 1.0);
        assert_eq!(f.corners_per_team_mean, 5.0);
        assert_eq!(f.throw_ins_per_match_mean, 44.0);
        assert_eq!(f.goal_kicks_per_match_mean, 16.0);
        assert_eq!(b.missing, 1);
        let wall = BTreeMap::from([(Suite::Equal, 1_000)]);
        let checks = b.checks(&figures, &[], &wall);
        let names: Vec<&str> = checks.iter().map(|c| c.band.as_str()).collect();
        let mut expected = vec![
            "goals_per_match",
            "shots_per_team",
            "possession_home_pct",
            "possession_away_pct",
        ];
        expected.extend(V2);
        expected.push("wall_ms");
        assert_eq!(names, expected);
        assert!(checks.iter().all(|c| c.pass), "{checks:?}");
        assert!(checks.iter().all(|c| c.pairing.is_none()));
    }

    #[test]
    fn a_ratio_over_no_shots_is_zero_not_a_null() {
        let mut b = RunBuilder::new(bands());
        b.plan(Suite::Equal, 1);
        b.add(Suite::Equal, record([0, 0], [0, 0], [50.0, 50.0]), None);
        let f = &b.figures()[&Suite::Equal];
        assert_eq!(f.shots_on_target_share, 0.0);
        assert_eq!(f.goals_per_xg, 0.0);
        let json = serde_json::to_string(f).unwrap();
        assert!(!json.contains("null"), "{json}");
    }

    #[test]
    fn the_formations_suite_checks_three_goal_bands_per_pairing() {
        let mut b = RunBuilder::new(bands());
        let names = vec![
            ["4-4-2".to_string(), "4-4-2".to_string()],
            ["4-4-2".to_string(), "4-3-3".to_string()],
        ];
        b.plan_pairings(2, names);
        // Pairing 0: 3-0 and 2-1. Pairing 1: the 4-4-2 at home wins 6-5, then away loses 0-0.
        b.add_pairing(record([3, 0], [12, 10], [50.0, 50.0]), 0, 0);
        b.add_pairing(record([2, 1], [12, 10], [50.0, 50.0]), 0, 1);
        b.add_pairing(record([6, 5], [12, 10], [50.0, 50.0]), 1, 0);
        b.add_pairing(record([0, 0], [12, 10], [50.0, 50.0]), 1, 1);
        let figures = b.figures();
        assert_eq!(figures[&Suite::Formations].matches, 4);
        let pairings = b.pairing_figures();
        assert_eq!(pairings.len(), 2);
        assert_eq!(pairings[0].label(), "4-4-2 v 4-4-2");
        assert_eq!(pairings[0].matches, 2);
        assert_eq!(pairings[0].goals_per_match_mean, 3.0);
        // The first formation scored 3 at home, then 1 away.
        assert_eq!(pairings[0].goals_for_mean, [2.0, 1.0]);
        assert_eq!(pairings[1].goals_per_match_mean, 5.5);
        assert_eq!(pairings[1].ten_plus_goals_share, 0.5);
        assert_eq!(pairings[1].goalless_share, 0.5);
        let checks = b.checks(&figures, &pairings, &BTreeMap::new());
        let rows: Vec<(&str, Option<&str>, bool)> = checks
            .iter()
            .map(|c| (c.band.as_str(), c.pairing.as_deref(), c.pass))
            .collect();
        assert_eq!(
            rows,
            [
                ("goals_per_match", Some("4-4-2 v 4-4-2"), true),
                ("ten_plus_goals_share", Some("4-4-2 v 4-4-2"), true),
                ("goalless_share", Some("4-4-2 v 4-4-2"), false),
                ("goals_per_match", Some("4-4-2 v 4-3-3"), false),
                ("ten_plus_goals_share", Some("4-4-2 v 4-3-3"), false),
                ("goalless_share", Some("4-4-2 v 4-3-3"), false),
            ]
        );
        assert!(checks.iter().all(|c| c.suite == "formations"));
    }

    #[test]
    fn a_band_miss_and_an_outlier_are_named() {
        let mut b = RunBuilder::new(bands());
        b.plan(Suite::Equal, 1);
        // Six shots and 70 percent possession: an outlier, and both bands miss.
        assert!(b.add(Suite::Equal, record([5, 1], [6, 6], [70.0, 30.0]), None));
        let figures = b.figures();
        let checks = b.checks(&figures, &[], &BTreeMap::new());
        let failed: Vec<&str> = checks
            .iter()
            .filter(|c| !c.pass)
            .map(|c| c.band.as_str())
            .collect();
        // One match with no card red and a score: the two share floors miss as well.
        assert_eq!(
            failed,
            [
                "goals_per_match",
                "shots_per_team",
                "possession_home_pct",
                "possession_away_pct",
                "sending_off_share",
                "goalless_share"
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
        failed.outcome = "error".into();
        assert!(b.add(Suite::Strength, failed, Some(1)));
        let figures = b.figures();
        let rec = figures[&Suite::Strength].stronger.unwrap();
        assert_eq!((rec.wins, rec.draws, rec.losses), (1, 1, 1));
        assert_eq!(rec.win_rate, 0.3333);
        assert_eq!(figures[&Suite::Strength].failures, 1);
        let checks = b.checks(&figures, &[], &BTreeMap::new());
        assert_eq!(checks.len(), 1);
        assert!(!checks[0].pass);
    }

    #[test]
    fn a_suite_slower_than_its_budget_fails_the_wall_band() {
        let mut b = RunBuilder::new(bands());
        b.plan(Suite::Equal, 1000);
        let figures = b.figures();
        let wall = BTreeMap::from([(Suite::Equal, 1_800_001)]);
        let checks = b.checks(&figures, &[], &wall);
        let time = checks.iter().find(|c| c.band == "wall_ms").unwrap();
        assert_eq!(time.hi, 1_800_000.0);
        assert!(!time.pass);
    }

    #[test]
    fn the_sampling_errors_match_hand_worked_values() {
        // Population sd of 1, 2, 3, 4 is sqrt(1.25); over sqrt(4).
        assert!((se_mean(&[1.0, 2.0, 3.0, 4.0]) - 1.25f64.sqrt() / 2.0).abs() < 1e-12);
        assert_eq!(se_mean(&[3.0]), 0.0);
        assert!((se_share(0.1, 100) - 0.03).abs() < 1e-12);
        assert_eq!(se_share(0.0, 100), 0.0);
        // R = 3 / 4; residuals -0.5 and 0.5; sqrt(0.5 / 2) / mean(x) = 0.5 / 2.
        assert!((se_ratio(&[1.0, 2.0], &[2.0, 2.0]) - 0.25).abs() < 1e-12);
        assert_eq!(se_ratio(&[1.0], &[2.0]), 0.0);
    }

    #[test]
    fn every_band_row_carries_its_sampling_error() {
        let mut b = RunBuilder::new(bands());
        b.plan(Suite::Equal, 4);
        for goals in [[1, 1], [2, 1], [0, 0], [3, 3]] {
            b.add(Suite::Equal, record(goals, [10, 10], [50.0, 50.0]), None);
        }
        let figures = b.figures();
        let wall = BTreeMap::from([(Suite::Equal, 1_000)]);
        let checks = b.checks(&figures, &[], &wall);
        let se = |band: &str| checks.iter().find(|c| c.band == band).unwrap().se;
        // Goals 2, 3, 0, 6: mean 2.75, population variance 4.6875.
        assert_eq!(se("goals_per_match"), round_to(4.6875f64.sqrt() / 2.0, 5));
        // One goalless match in four.
        assert_eq!(
            se("goalless_share"),
            round_to((0.25f64 * 0.75 / 4.0).sqrt(), 5)
        );
        // Every match has the same possession: no spread.
        assert_eq!(se("possession_home_pct"), 0.0);
        assert_eq!(se("wall_ms"), 0.0);
    }

    #[test]
    fn the_red_card_suite_folds_each_arm_against_the_control() {
        let mut b = RunBuilder::new(bands());
        b.plan(Suite::RedCard, 8);
        // Control 2-1 and 0-1: home mean 1. Keeper arm 2-0 and 1-0; centre-back 1-1 and
        // 1-0; striker 1-0 and 3-2 (full side 2.0, above 1.6 x 1).
        let arms = [
            [[2, 1], [0, 1]],
            [[2, 0], [1, 0]],
            [[1, 1], [1, 0]],
            [[1, 0], [3, 2]],
        ];
        for (arm, games) in arms.iter().enumerate() {
            for goals in games {
                b.add_red_card(record(*goals, [10, 10], [50.0, 50.0]), arm);
            }
        }
        let f = b.red_card_figures().unwrap();
        assert_eq!(f.matches, 2);
        assert_eq!(f.control, [1.0, 1.0]);
        assert_eq!(f.limit, 1.6);
        let arm = |name: &str| f.arms.iter().find(|a| a.arm == name).unwrap();
        assert_eq!((arm("keeper").full, arm("keeper").reduced), (1.5, 0.0));
        assert!(arm("keeper").pass && arm("centre-back").pass);
        assert!(!arm("striker").pass, "the full side scored 2.0 against 1.6");
        assert!(!f.pass);
        let checks = b.checks(&b.figures(), &[], &BTreeMap::new());
        let rows: Vec<(&str, Option<&str>, f64, bool)> = checks
            .iter()
            .filter(|c| c.suite == "red-card")
            .map(|c| (c.band.as_str(), c.pairing.as_deref(), c.value, c.pass))
            .collect();
        assert_eq!(
            rows,
            [
                ("reduced_minus_full", Some("keeper"), -1.5, true),
                ("full_over_control", Some("keeper"), 1.5, true),
                ("reduced_minus_full", Some("centre-back"), -0.5, true),
                ("full_over_control", Some("centre-back"), 1.0, true),
                ("reduced_minus_full", Some("striker"), -1.0, true),
                ("full_over_control", Some("striker"), 2.0, false),
            ]
        );
    }

    #[test]
    fn every_band_belongs_to_a_suite_and_the_red_card_suite_is_not_in_all() {
        for band in BAND_NAMES {
            assert!(band_suites(band).is_some(), "{band}");
        }
        assert!(band_suites("no_such_band").is_none());
        assert!(!Suite::ALL.contains(&Suite::RedCard));
    }
}

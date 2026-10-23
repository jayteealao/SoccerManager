//! The run-scoped builder of a calibration run: it folds every `match-stats` record of a
//! suite into the aggregate figures, checks them against the accepted realism bands, sums
//! the dark paths, and names the outlier matches whose event files are kept. The formations
//! suite is also folded per formation pairing, and the red-card suite per arm. Every band
//! check carries its sampling error.

pub mod accepted;
pub mod bands;
pub mod baseline;
pub mod compare;
pub mod distribution;
pub mod history;
pub mod kinds;
pub mod measures;
pub mod nash;
pub mod proof;
pub mod real;
pub mod styles;
pub mod substitutions;
pub mod verdict;

use std::collections::BTreeMap;

use engine::observe::{MatchStats, Record, round_to};
use serde::{Deserialize, Serialize};

use bands::{BandDef, Registry, Rule, Target};
use measures::{Extras, Obs, Scope};
use pass_rule::interval::{Interval, RareKind, Real, Unit};

/// Matches slower than this, in milliseconds, are outliers (the contract's slow threshold).
pub const SLOW_MATCH_MS: u64 = 2000;

/// A match whose possession of either side lies outside this range is an outlier, and its
/// files are kept. The screen picks matches to keep; it judges no band (the band file's
/// version 2 range, kept fixed since bands moved to margins around real football).
pub const OUTLIER_POSSESSION_PCT: std::ops::RangeInclusive<f64> = 35.0..=65.0;

/// A match in which either side's shots lie outside this range is an outlier.
pub const OUTLIER_SHOTS_PER_TEAM: std::ops::RangeInclusive<u32> = 8..=16;

/// The version of the measure definitions: how a match's figures are counted. A change to a
/// measure raises it, so a run folder made under the old definitions is not resumed.
/// Version 2 adds the realism measures of a `measures` build to the rows: shot distance,
/// pass kind and length, goal phase, possession by third, possession chains, and each
/// player's running. Version 3 adds the team-shape sums: each outfield group's depth, width,
/// pace and sprints, and each team's slide, far-player response, dead-ball calm, stoppages by
/// kind and completed links.
pub const MEASURES_VERSION: u32 = 3;

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

    /// The suite a code names.
    pub fn parse(code: &str) -> Option<Self> {
        [
            Suite::Equal,
            Suite::Strength,
            Suite::Formations,
            Suite::RedCard,
        ]
        .into_iter()
        .find(|s| s.code() == code)
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

/// The full side may score at most this multiple of the control's home mean: the version 2
/// migration's `full_over_control` top.
pub const RED_CARD_LIMIT: f64 = 1.6;

/// The rows a run judges for each band: one per suite that judges it, one per formation
/// pairing in the formations suite (or one over every pairing, `pooled`), and one per arm
/// in the red-card suite; in [`Suite`] order, then pairing or arm, then registry order.
#[derive(Debug, Clone)]
pub struct RowSpec<'a> {
    pub def: &'a BandDef,
    pub suite: Suite,
    pub scope: Scope,
    /// The pairing or arm, as a band check names it.
    pub label: Option<String>,
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

/// The formations of the formations suite, the share of the matches between each two that
/// the first won, and the fewest finished matches of a pairing.
pub type WinTable = (Vec<String>, Vec<Vec<f64>>, u32);

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
    #[cfg(test)]
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
    /// `true` for a row a change run reports as information only: a formations row of one
    /// pairing, whose band the change run judges and powers pooled over every pairing. It
    /// sets no power target and does not enter the joint verdict. Absent in a plain run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub informational: Option<bool>,
    /// Why the row has no value, when it has none; its word is then not sure. Shown in the
    /// verdict row's note, never in `calib.bands`.
    #[serde(skip)]
    pub why_none: Option<String>,
    /// The band reads measures this run's matches do not carry: its word is not sure.
    #[serde(skip)]
    pub unjudged: bool,
    /// A whole-distribution band's row: its word comes from its bounds, never from `pass`.
    #[serde(skip)]
    pub whole: bool,
    /// A whole-distribution band with a value: its distance from the real sample with its
    /// bounds and word.
    #[serde(skip)]
    pub dist: Option<distribution::DistCheck>,
    /// A version 6 band: its word by the pass rule, with the interval and the margin that
    /// set it.
    #[serde(skip)]
    pub rule: Option<RuleWord>,
}

/// A band's word against real football (or its design target) by the pass rule, with what
/// decided it: the interval of engine minus the target, each side at `1 - alpha`, the margin,
/// and the exposure. A whole-distribution band's interval is the bounds of its distance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleWord {
    pub word: pass_rule::rule::Word,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lo: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hi: Option<f64>,
    /// The difference (or distance) on the unresampled data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point: Option<f64>,
    pub margin: f64,
    /// `real`, or `design <value>`.
    pub target: String,
    /// A design band's most distance from the real value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap: Option<f64>,
    /// Matches or events judged, in the unit of the minimum exposure.
    pub exposure: f64,
    pub min_exposure: f64,
    /// The engine's outer units resampled: leagues, or matches for fixed clubs.
    pub units: usize,
    /// Why the word is not sure, when a reason beyond the interval holds it there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl RuleWord {
    fn set_interval(&mut self, iv: &Interval) {
        self.lo = Some(round_to(iv.lo, 6));
        self.hi = Some(round_to(iv.hi, 6));
        self.point = Some(round_to(iv.point, 6));
    }

    /// The interval and the margin in words, as the console's note shows them.
    pub fn note(&self) -> String {
        let mut out = match (self.lo, self.hi) {
            (Some(lo), Some(hi)) => format!(
                "interval {lo} to {hi} against margin {} ({})",
                self.margin, self.target
            ),
            _ => format!("margin {} ({})", self.margin, self.target),
        };
        if let Some(cap) = self.cap {
            out.push_str(&format!(", cap {cap}"));
        }
        if let Some(why) = &self.reason {
            out.push_str(&format!("; {why}"));
        }
        out
    }
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
    /// For each record: its extras, kept only in a suite that judges a band reading them.
    extras: Vec<Option<Box<Extras>>>,
    /// For each record: its generated league, when the fold knew its fixture.
    leagues: Vec<Option<u32>>,
    /// A record of the suite carries the realism measures.
    measures: bool,
    outliers: u32,
}

/// The run-scoped builder.
#[derive(Debug)]
pub struct RunBuilder {
    bands: Registry,
    suites: BTreeMap<Suite, SuiteAcc>,
    /// The formation names of each pairing of the formations suite.
    pairing_names: Vec<[String; 2]>,
    /// Statistics records that never appeared (`darkpath.match_without_stats`).
    pub missing: u32,
    /// The length of every match, in minutes.
    pub minutes: u32,
    /// The real samples of the distribution bands, read once per judgement, and the run
    /// seed their range tests draw from.
    real: std::sync::Arc<real::Samples>,
    seed: u64,
    /// The pass rule's real side of each band, read before play, by band.
    sides: std::sync::Arc<BTreeMap<String, Real>>,
}

impl RunBuilder {
    pub fn new(bands: Registry) -> Self {
        Self {
            bands,
            suites: BTreeMap::new(),
            pairing_names: Vec::new(),
            missing: 0,
            minutes: 90,
            real: std::sync::Arc::default(),
            seed: 0,
            sides: std::sync::Arc::default(),
        }
    }

    /// Gives the builder each band's real side for the pass rule, read before play.
    pub fn set_real_sides(&mut self, sides: std::sync::Arc<BTreeMap<String, Real>>) {
        self.sides = sides;
    }

    /// Gives the record of `suite` folded in last its generated league.
    pub fn set_league(&mut self, suite: Suite, league: u32) {
        if let Some(last) = self
            .suites
            .get_mut(&suite)
            .and_then(|acc| acc.leagues.last_mut())
        {
            *last = Some(league);
        }
    }

    /// Gives the builder the real samples its distribution bands are judged against, and
    /// the run seed of their range tests.
    pub fn set_real_samples(&mut self, samples: std::sync::Arc<real::Samples>, seed: u64) {
        self.real = samples;
        self.seed = seed;
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
        acc.extras.push(None);
        acc.leagues.push(None);
        outlier
    }

    /// Gives the record of `suite` folded in last its extras: its measures, its players'
    /// running and its clubs. They are kept only when a band the suite judges reads them.
    pub fn set_extras(&mut self, suite: Suite, extras: Extras) {
        let keep = self
            .bands
            .bands
            .iter()
            .any(|b| b.judged_in(suite) && b.needs_extras());
        let Some(acc) = self.suites.get_mut(&suite) else {
            return;
        };
        acc.measures |= extras.measures.present;
        if keep {
            if let Some(last) = acc.extras.last_mut() {
                *last = Some(Box::new(extras));
            }
        }
    }

    /// `true` when a record of `suite` carries the realism measures.
    pub fn has_measures(&self, suite: Suite) -> bool {
        self.suites.get(&suite).is_some_and(|a| a.measures)
    }

    /// A planned record that never appeared.
    pub fn add_missing(&mut self) {
        self.missing += 1;
    }

    /// Sets the outlier count of `suite`: the one-process runner judges outliers when it
    /// plays, by the rule of its full recording.
    pub fn set_outliers(&mut self, suite: Suite, outliers: u32) {
        self.suites.entry(suite).or_default().outliers = outliers;
    }

    /// `true` for a failed match, a match with validator violations, a slow match, a
    /// dark-path hit, and a match with a possession or a team's shots outside its band.
    pub fn outlier(&self, s: &MatchStats) -> bool {
        let possession = |v: f64| !OUTLIER_POSSESSION_PCT.contains(&v);
        s.outcome != "success"
            || s.validate_violations > 0
            || s.duration_ms > SLOW_MATCH_MS
            || s.tactics.change_never_applied > 0
            || possession(s.figures.possession_pct[0])
            || possession(s.figures.possession_pct[1])
            || s.tactics
                .shots
                .iter()
                .any(|&n| !OUTLIER_SHOTS_PER_TEAM.contains(&n))
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
        let inside = |band: &str, v: f64| self.bands.range(band).is_none_or(|r| r.contains(v));
        let in_band = per_match(&|r| {
            f64::from(u8::from(
                inside("possession_home_pct", r.figures.possession_pct[0])
                    && inside("possession_away_pct", r.figures.possession_pct[1]),
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

    /// The win-share table of the formations suite: the formations in the tactics file's
    /// order (the order `pairings` gives), `W[i][j]` the share of the finished matches
    /// between `i` and `j` that `i` won (a draw is not a win; a mirror pairing counts the
    /// wins of either side over twice its matches), and the fewest finished matches of a
    /// pairing. `Err((played, total))` when a pairing has no finished match or is missing.
    pub fn formation_table(&self) -> Result<WinTable, (usize, usize)> {
        let mut names: Vec<String> = Vec::new();
        for name in self.pairing_names.iter().flatten() {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        let n = names.len();
        let total = n * (n + 1) / 2;
        let at = |name: &String| names.iter().position(|x| x == name).unwrap_or(0);
        let mut wins = vec![vec![0.0; n]; n];
        let mut seen = vec![vec![false; n]; n];
        let mut fewest = u32::MAX;
        let acc = self.suites.get(&Suite::Formations);
        for (k, pair) in self.pairing_names.iter().enumerate() {
            let played = acc.map(|acc| pairing_records(acc, k)).unwrap_or_default();
            if played.is_empty() {
                continue;
            }
            let (a, b) = (at(&pair[0]), at(&pair[1]));
            let won = |first: bool| {
                played
                    .iter()
                    .filter(|(r, side)| {
                        let (us, them) = if first {
                            (*side, 1 - side)
                        } else {
                            (1 - side, *side)
                        };
                        r.goals[us] > r.goals[them]
                    })
                    .count() as f64
            };
            let m = played.len() as f64;
            if a == b {
                wins[a][a] = (won(true) + won(false)) / (2.0 * m);
            } else {
                wins[a][b] = won(true) / m;
                wins[b][a] = won(false) / m;
            }
            seen[a][b] = true;
            seen[b][a] = true;
            // A pairing holds at most 100 000 matches.
            fewest = fewest.min(played.len() as u32);
        }
        let played = (0..n).map(|a| (a..n).filter(|&b| seen[a][b]).count()).sum();
        if n == 0 || played < total {
            return Err((played, total.max(self.pairing_names.len())));
        }
        Ok((names, wins, fewest))
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
        let top = self
            .bands
            .band("full_over_control")
            .map_or(RED_CARD_LIMIT, |b| b.hi);
        let limit = top * control_home;
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

    /// Every match of `suite` as the measures see it, in the order they were folded in.
    pub fn observations(&self, suite: Suite) -> Vec<Obs<'_>> {
        let Some(acc) = self.suites.get(&suite) else {
            return Vec::new();
        };
        (0..acc.records.len())
            .map(|i| Obs {
                stats: &acc.records[i],
                boosted: acc.boosted[i],
                pairing: acc.pairings[i].map(|(p, _)| p),
                arm: acc.arms[i],
                extras: acc.extras[i].as_deref(),
                league: acc.leagues[i],
            })
            .collect()
    }

    /// The rows the planned suites judge, in report order; `pooled` gives the formations
    /// suite one row per band over every pairing instead of one per pairing.
    pub fn row_specs(&self, pooled: bool) -> Vec<RowSpec<'_>> {
        let mut out = Vec::new();
        for &suite in self.suites.keys() {
            let defs = || self.bands.bands.iter().filter(move |d| d.judged_in(suite));
            let mut push = |def, scope, label| {
                out.push(RowSpec {
                    def,
                    suite,
                    scope,
                    label,
                });
            };
            match suite {
                Suite::Formations if !pooled => {
                    for (i, names) in self.pairing_names.iter().enumerate() {
                        let label = format!("{} v {}", names[0], names[1]);
                        for def in defs() {
                            push(def, Scope::Pairing(i), Some(label.clone()));
                        }
                    }
                }
                Suite::RedCard => {
                    for (arm, (name, player)) in RED_CARD_ARMS.iter().enumerate() {
                        if player.is_none() {
                            continue;
                        }
                        for def in defs() {
                            push(def, Scope::Arm(arm), Some((*name).to_string()));
                        }
                    }
                }
                _ => {
                    for def in defs() {
                        push(def, Scope::All, None);
                    }
                }
            }
        }
        out
    }

    /// One band row judged by its range: its value, rounded as the figures are, and its
    /// sampling error. A band that reads measures the suite's matches do not carry is
    /// unjudged: it does not pass.
    pub fn check(&self, spec: &RowSpec<'_>) -> BandCheck {
        let obs = self.observations(spec.suite);
        if spec.def.is_distribution() {
            return self.distribution_check(spec, &obs);
        }
        let terms = measures::terms(spec.def, &obs, spec.scope);
        let value = round_to(
            terms.value().unwrap_or(0.0),
            measures::places(&spec.def.measure),
        );
        let unjudged = spec.def.needs_measures() && !self.has_measures(spec.suite);
        let rule = spec
            .def
            .rule
            .as_ref()
            .map(|rule| self.rule_word(spec.def, rule, &obs, &terms, unjudged));
        BandCheck {
            band: spec.def.band.clone(),
            suite: spec.suite.code().to_string(),
            pairing: spec.label.clone(),
            value,
            lo: spec.def.lo,
            hi: spec.def.hi,
            pass: match &rule {
                Some(r) => r.word == pass_rule::rule::Word::Pass,
                None => !unjudged && spec.def.range().contains(value),
            },
            se: round_to(terms.se(), 5),
            informational: None,
            why_none: terms.why_none().map(str::to_string),
            unjudged,
            whole: false,
            dist: None,
            rule,
        }
    }

    /// A version 6 band's word against real football (or its design target) by the pass
    /// rule: the interval of the engine minus the target, resampled over whole leagues (or
    /// whole matches for a band of fixed clubs) and over the real side read before play,
    /// against the band's margin. A rare band takes exact count bounds; a pass below the
    /// band's minimum exposure, or a band with no real value here, is not sure.
    fn rule_word(
        &self,
        def: &BandDef,
        rule: &Rule,
        obs: &[Obs<'_>],
        terms: &measures::Terms,
        unjudged: bool,
    ) -> RuleWord {
        let alpha = self.bands.pass_rule.map_or(0.05, |p| p.alpha);
        let [a, b, c, d] = terms.per_match();
        let flags = terms.covered_flags();
        let mut units: BTreeMap<u64, Unit> = BTreeMap::new();
        let mut events = 0.0;
        for (i, &covered) in flags.iter().enumerate() {
            if !covered {
                continue;
            }
            let key = match rule.resampling.engine {
                bands::EngineUnit::League => u64::from(obs[i].league.unwrap_or(0)),
                bands::EngineUnit::Match => i as u64,
            };
            let u = units.entry(key).or_insert(Unit {
                num: 0.0,
                den: 0.0,
                scale_num: 0.0,
                scale_den: 0.0,
            });
            u.num += a[i];
            u.den += b[i];
            u.scale_num += d[i];
            u.scale_den += c[i];
            events += a[i];
        }
        let matches = terms.covered() as f64;
        let exposure = match rule.min_exposure.unit {
            bands::ExposureUnit::Matches => matches,
            bands::ExposureUnit::Events => events,
        };
        let units: Vec<Unit> = units.into_values().collect();
        let mut out = RuleWord {
            word: pass_rule::rule::Word::NotSure,
            lo: None,
            hi: None,
            point: None,
            margin: rule.margin.value,
            target: match rule.target {
                Target::Real => "real".into(),
                Target::Design { value, .. } => format!("design {value}"),
            },
            cap: match rule.target {
                Target::Design { cap, .. } => cap,
                Target::Real => None,
            },
            exposure,
            min_exposure: rule.min_exposure.value,
            units: units.len(),
            reason: None,
        };
        let real = self.sides.get(&def.band);
        let why = if unjudged {
            Some(verdict::NO_MEASURES_NOTE.to_string())
        } else if let Some(w) = terms.why_none().filter(|_| terms.value().is_none()) {
            Some(format!("no value: {w}"))
        } else if matches!(def.measure, bands::Measure::ClubRange { .. }) {
            Some("a per-club spread has no resampling units; judged when it has them".into())
        } else {
            None
        };
        if let Some(why) = why {
            out.reason = Some(why);
            return out;
        }
        let mut rng = engine::rng::EngineRng::from_seed(distribution::stream(self.seed, &def.band));
        let mut draw = || rng.next_f64();
        let interval = |to: &Real, draw: &mut dyn FnMut() -> f64| -> Result<Interval, String> {
            if rule.rare {
                let kind = match def.measure {
                    bands::Measure::Share { .. } | bands::Measure::Scoreline { .. } => {
                        RareKind::Share
                    }
                    _ => RareKind::Rate,
                };
                let (value, se) = to.estimate();
                let bounds = pass_rule::interval::real_bounds(value, se, alpha);
                Ok(pass_rule::interval::rare_interval(
                    kind,
                    events.round().max(0.0) as u64,
                    matches,
                    bounds,
                    alpha,
                ))
            } else {
                pass_rule::interval::interval(&units, to, alpha, verdict::RESAMPLES, draw)
            }
        };
        let judged = match rule.target {
            Target::Real => match real {
                None => Err("no real value here; a band with no real value is never a pass".into()),
                Some(side) => interval(side, &mut draw).map(|iv| {
                    out.set_interval(&iv);
                    pass_rule::rule::judge(&iv, rule.margin.value)
                }),
            },
            Target::Design { value, cap } => {
                interval(&Real::Fixed(value), &mut draw).and_then(|iv| {
                    out.set_interval(&iv);
                    match (cap, real) {
                        (None, _) => {
                            Ok(pass_rule::rule::judge_design(&iv, rule.margin.value, None))
                        }
                        (Some(cap), Some(side)) => {
                            let to_real = interval(side, &mut draw)?;
                            Ok(pass_rule::rule::judge_design(
                                &iv,
                                rule.margin.value,
                                Some((&to_real, cap)),
                            ))
                        }
                        // A cap with no real value here: never a pass.
                        (Some(_), None) => {
                            let w = pass_rule::rule::judge_design(&iv, rule.margin.value, None);
                            Ok(if w == pass_rule::rule::Word::Pass {
                                pass_rule::rule::Word::NotSure
                            } else {
                                w
                            })
                        }
                    }
                })
            }
        };
        match judged {
            Ok(word) => {
                let (word, why) =
                    pass_rule::rule::gate_exposure(word, exposure, rule.min_exposure.value);
                out.word = word;
                out.reason = why;
            }
            Err(why) => out.reason = Some(why),
        }
        out
    }

    /// A distribution band's row over the matches `obs`: its distance from the real sample,
    /// rounded to 5 places, in the range 0 to its margin, with its bootstrap error, bounds,
    /// noise floor and word. Without a real sample, or with fewer than two matches, it has
    /// no value and says why.
    pub fn distribution_check(&self, spec: &RowSpec<'_>, obs: &[Obs<'_>]) -> BandCheck {
        let def = spec.def;
        let unjudged = def.needs_measures() && !self.has_measures(spec.suite);
        let mut c = BandCheck {
            band: def.band.clone(),
            suite: spec.suite.code().to_string(),
            pairing: spec.label.clone(),
            value: 0.0,
            lo: def.lo,
            hi: def.hi,
            pass: false,
            se: 0.0,
            informational: None,
            why_none: None,
            unjudged,
            whole: true,
            dist: None,
            rule: None,
        };
        let margin = def.rule.as_ref().map_or(def.hi, |r| r.margin.value);
        let alpha = match (&def.rule, self.bands.pass_rule) {
            (Some(_), Some(p)) => p.alpha,
            _ => verdict::FALSE_ALARM,
        };
        let rule_word = |dist: Option<&distribution::DistCheck>, why: Option<String>| {
            def.rule.as_ref().map(|r| RuleWord {
                word: match dist {
                    Some(d) if margin > d.floor => {
                        pass_rule::rule::judge_distance(d.lower, d.upper, margin)
                    }
                    _ => pass_rule::rule::Word::NotSure,
                },
                lo: dist.map(|d| round_to(d.lower, 5)),
                hi: dist.map(|d| round_to(d.upper, 5)),
                point: dist.map(|d| round_to(d.distance, 5)),
                margin,
                target: "real".into(),
                cap: None,
                exposure: 0.0,
                min_exposure: r.min_exposure.value,
                units: 0,
                reason: why.or_else(|| {
                    dist.filter(|d| margin <= d.floor).map(|d| {
                        format!("margin below noise floor (floor {})", round_to(d.floor, 5))
                    })
                }),
            })
        };
        if unjudged {
            c.rule = rule_word(None, Some(verdict::NO_MEASURES_NOTE.into()));
            return c;
        }
        let real = match self.real.get(&def.band) {
            Some(Ok(sample)) => sample,
            Some(Err(why)) => {
                c.why_none = Some(format!("no real data here: {why}"));
                c.rule = rule_word(None, c.why_none.clone());
                return c;
            }
            None => {
                c.why_none = Some("no real data here: the real sample was not read".into());
                c.rule = rule_word(None, c.why_none.clone());
                return c;
            }
        };
        let Some((width, bins)) = measures::hist_bins(&def.measure) else {
            return c;
        };
        let engine = distribution::Sample {
            per_match: measures::histograms(def, obs, spec.scope)
                .into_iter()
                .flatten()
                .collect(),
            bins,
            width,
        };
        let seed = distribution::stream(self.seed, &def.band);
        match distribution::judge_to_real_at(
            &engine,
            real,
            measures::metric(&def.measure),
            margin,
            seed,
            verdict::RESAMPLES,
            alpha,
        ) {
            Ok(d) => {
                c.value = round_to(d.distance, 5);
                c.rule = rule_word(Some(&d), None);
                c.pass = match &c.rule {
                    Some(r) => r.word == pass_rule::rule::Word::Pass,
                    None => def.range().contains(c.value),
                };
                c.se = round_to(d.se, 5);
                c.dist = Some(d);
            }
            Err(why) => {
                c.rule = rule_word(None, Some(why.to_string()));
                c.why_none = Some(why);
            }
        }
        c
    }

    /// The figures of the registry, each over every planned suite that reports it: its value
    /// beside the value an earlier build gave. A figure that reads measures the suite's
    /// matches do not carry has no value.
    pub fn figure_rows(&self) -> Vec<FigureRow> {
        let mut out = Vec::new();
        for f in &self.bands.figures {
            let def = f.as_band();
            for &suite in self.suites.keys().filter(|s| def.judged_in(**s)) {
                let measured = !def.needs_measures() || self.has_measures(suite);
                let terms =
                    measured.then(|| measures::terms(&def, &self.observations(suite), Scope::All));
                let value = terms
                    .as_ref()
                    .and_then(measures::Terms::value)
                    .map(|v| round_to(v, 4));
                out.push(FigureRow {
                    figure: f.figure.clone(),
                    suite: suite.code().to_string(),
                    value,
                    se: terms.as_ref().map_or(0.0, |t| round_to(t.se(), 5)),
                    today: f.today.value,
                    build: f.today.build.clone(),
                    change: value.map(|v| round_to(v - f.today.value, 4)),
                });
            }
        }
        out
    }

    /// The checks of the bands that only report: every row of a band with a reason in
    /// `report_only`, in report order. A band that reads measures the suite's matches do not
    /// carry is left out: a build without the measures judges the bands of the file only.
    pub fn reported_checks(&self) -> Vec<BandCheck> {
        self.row_specs(false)
            .iter()
            .filter(|s| s.def.report_only.is_some())
            .filter(|s| !s.def.needs_measures() || self.has_measures(s.suite))
            .map(|s| self.check(s))
            .collect()
    }

    /// The band checks of the planned suites, from the registry, given each suite's wall
    /// time: the equal suite's bands, each suite's time budget, then the other suites'.
    /// Only the bands that gate: a band that only reports is in [`Self::reported_checks`].
    pub fn checks(&self, wall_ms: &BTreeMap<Suite, u64>) -> Vec<BandCheck> {
        let (equal, rest): (Vec<_>, Vec<_>) = self
            .row_specs(false)
            .into_iter()
            .filter(|s| s.def.report_only.is_none())
            .partition(|s| s.suite == Suite::Equal);
        let mut out: Vec<BandCheck> = equal.iter().map(|s| self.check(s)).collect();
        for (&suite, &ms) in wall_ms {
            let matches = self.suites.get(&suite).map_or(0, |a| a.matches);
            let budget = self.bands.wall_budget_ms(matches) as f64;
            out.push(BandCheck {
                band: "wall_ms".to_string(),
                suite: suite.code().to_string(),
                pairing: None,
                value: ms as f64,
                lo: 0.0,
                hi: budget,
                pass: (ms as f64) < budget,
                se: 0.0,
                informational: None,
                why_none: None,
                unjudged: false,
                whole: false,
                dist: None,
                rule: None,
            });
        }
        out.extend(rest.iter().map(|s| self.check(s)));
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
    /// The scheme the fixtures' keys and engine seeds are made by.
    #[serde(rename = "fixtures.scheme")]
    pub fixtures_scheme: &'static str,
    /// Everything that makes the run's results what they are, as the run folder holds it.
    #[serde(rename = "run.identity")]
    pub identity: serde_json::Value,
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
    /// Each formation's field rating against the maximum-entropy Nash equilibrium beside
    /// its win rate, and the flag; report only. Absent unless every pairing of the
    /// formations suite finished a match.
    #[serde(rename = "calib.nash", skip_serializing_if = "Option::is_none")]
    pub nash: Option<nash::NashReport>,
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
    /// The run's answer, as the exit code gives it: a change run's joint word is `pass`;
    /// any other run's bands, dark paths, and time budget pass.
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
    /// Matches that panicked; the run caught each one and recorded it as failed.
    #[serde(rename = "darkpath.match_panicked")]
    pub match_panicked: u32,
    #[serde(rename = "validate.violations")]
    pub violations: usize,
    #[serde(rename = "events.files_written")]
    pub events_written: u32,
    #[serde(rename = "events.files_kept")]
    pub events_kept: u32,
    /// `threads`: the one-process runner. Reports of earlier builds can say `processes`,
    /// the worker-process runner they also had.
    #[serde(rename = "calib.runner")]
    pub runner: &'static str,
    /// The compact rows of every arm; absent in reports of the earlier worker processes.
    #[serde(rename = "calib.rows", skip_serializing_if = "Option::is_none")]
    pub rows: Option<RowsInfo>,
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
    /// The fixtures of the run, those an earlier session finished, and those played now.
    #[serde(rename = "calib.units")]
    pub units: Units,
    /// SHA-256 over the compact rows' result columns in key order. Reports of the earlier
    /// worker processes hashed every match's statistics, identifiers and timing left out.
    #[serde(rename = "calib.results_digest")]
    pub results_digest: String,
    /// The old engine's results on the same fixtures; absent without an old engine.
    #[serde(rename = "calib.base", skip_serializing_if = "Option::is_none")]
    pub base: Option<BaseReport>,
    /// Each band's word: in a change run from the paired max-t test, otherwise from its
    /// range. Absent in a paired flag run, which has `calib.verdict`.
    #[serde(rename = "calib.verdicts", skip_serializing_if = "Vec::is_empty")]
    pub verdicts: Vec<verdict::VerdictRow>,
    /// A change run's joint verdict.
    #[serde(rename = "calib.joint", skip_serializing_if = "Option::is_none")]
    pub joint: Option<verdict::Joint>,
    /// The run's kind: change, milestone or release.
    #[serde(rename = "calib.kind")]
    pub kind: &'static str,
    /// A change run: the fixed count of its kind it played, per suite unit, by suite code.
    #[serde(rename = "calib.count", skip_serializing_if = "BTreeMap::is_empty")]
    pub count: BTreeMap<String, u32>,
    /// The bands that only report: each row's word, judged as a gating band's is, but never
    /// in the joint word, `calib.pass` or `calib.bands`. Absent when no band only reports.
    #[serde(rename = "calib.reported", skip_serializing_if = "Vec::is_empty")]
    pub reported: Vec<verdict::VerdictRow>,
    /// The registry's figures, each beside the value an earlier build gave; never judged.
    /// Absent when the registry has none.
    #[serde(
        rename = "calib.figures",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub figures: Vec<FigureRow>,
    /// The developer's acceptances of not-sure bands in the run folder; absent without an
    /// `accepted.json`.
    #[serde(rename = "calib.accepted", skip_serializing_if = "Option::is_none")]
    pub accepted: Option<accepted::AcceptedInfo>,
    /// The matches the run judged: in total, in pairs, and per engine.
    #[serde(rename = "calib.counts")]
    pub counts: Counts,
    /// The band registry the run was judged with.
    #[serde(rename = "calib.registry")]
    pub registry: RegistryInfo,
    /// The real-data folder, the bands without a real value, and every band's real value
    /// and record; absent in the old engine's own quiet run.
    #[serde(rename = "calib.real", skip_serializing_if = "Option::is_none")]
    pub real: Option<real::RealInfo>,
    /// The realism claim, from validation bands with a real counterpart only.
    #[serde(rename = "calib.realism", skip_serializing_if = "Option::is_none")]
    pub realism: Option<real::Realism>,
    /// The margins widened provisionally to hold their floor, until the floor is set again
    /// on more leagues: their count and each band.
    #[serde(rename = "calib.provisional_margins")]
    pub provisional_margins: ProvisionalMargins,
    /// Each band of a named report-only group against the floor it would need if it gated,
    /// with whether it would hold now (with the real errors read here); absent in the old
    /// engine's own quiet run.
    #[serde(
        rename = "calib.report_only_floor",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub report_only_floor: Vec<bands::ReportOnlyFloor>,
    /// The time and memory of each stage, and the run's total.
    /// The band history this run added to and the bands whose history drifts; absent when
    /// the run keeps no history.
    #[serde(rename = "calib.drift", skip_serializing_if = "Option::is_none")]
    pub drift: Option<history::DriftInfo>,
    #[serde(rename = "calib.stages")]
    pub stages: BTreeMap<String, StageCost>,
    /// A change run's rules stage.
    #[serde(rename = "calib.rules", skip_serializing_if = "Option::is_none")]
    pub rules: Option<RulesReport>,
    /// A change run's sample of the styles suite, one match per ordered pairing of styles on
    /// the changed engine; report only. Absent in other runs and without a styles file.
    #[serde(rename = "calib.styles", skip_serializing_if = "Option::is_none")]
    pub styles: Option<styles::StylesReport>,
    /// The substitutions of the first arm's matches against the real era of the rule pack's
    /// limit; absent in the old engine's own quiet run and when no row holds a match.
    #[serde(
        rename = "calib.substitutions",
        skip_serializing_if = "Option::is_none"
    )]
    pub substitutions: Option<substitutions::SubstitutionReport>,
}

/// `calib.provisional_margins`: how many margins hold a provisional widening, and each one.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProvisionalMargins {
    pub count: u32,
    pub bands: Vec<bands::Provisional>,
}

impl ProvisionalMargins {
    pub fn of(registry: &bands::Registry) -> Self {
        let bands = registry.provisional();
        ProvisionalMargins {
            count: u32::try_from(bands.len()).unwrap_or(u32::MAX),
            bands,
        }
    }

    /// The console line, or nothing when no margin is provisional.
    pub fn line(&self) -> String {
        if self.count == 0 {
            return String::new();
        }
        format!(
            "margins: {} provisional, widened to hold the 97 percent floor on today's 3 \
             generated leagues; the floor is set again on 30 leagues\n",
            self.count
        )
    }
}

/// The console lines of the report-only bands against their floor.
pub fn render_report_only_floor(rows: &[bands::ReportOnlyFloor]) -> String {
    let mut out = String::new();
    for r in rows {
        out.push_str(&format!(
            "report-only band {} (group {}): margin {} against a floor of {}; would hold now: {}\n",
            r.band,
            r.group,
            round_to(r.margin, 6),
            round_to(r.needed, 6),
            if r.holds { "yes" } else { "no" }
        ));
    }
    out
}

/// One figure's row: its value over a suite beside the value an earlier build gave.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FigureRow {
    pub figure: String,
    pub suite: String,
    /// `None` when the suite's matches do not carry the measures the figure reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    pub se: f64,
    /// The earlier build's value, and the build.
    pub today: f64,
    pub build: String,
    /// The value minus the earlier build's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<f64>,
}

/// The figures as a table: each one's value beside the earlier build's; empty when there
/// are none.
pub fn render_figures(rows: &[FigureRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let mut out = String::from("figures (beside an earlier build; never judged):\n");
    out.push_str(&format!(
        "{:<28} {:<10} {:>10} {:>9} {:>10} {:>10}  {}\n",
        "figure", "suite", "value", "se", "today", "change", "build"
    ));
    for r in rows {
        let num = |v: Option<f64>| v.map_or_else(|| "no measures".to_string(), |v| format!("{v}"));
        out.push_str(&format!(
            "{:<28} {:<10} {:>10} {:>9} {:>10} {:>10}  {}\n",
            r.figure,
            r.suite,
            num(r.value),
            r.se,
            r.today,
            num(r.change),
            r.build
        ));
    }
    out
}

/// One stage's cost in a report.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct StageCost {
    pub ms: u64,
    /// The largest heap growth of one thread in the stage, in mebibytes; for `total`, the
    /// process's peak working set where the platform gives it.
    pub peak_mb: Option<f64>,
}

/// What a change run's rules stage did.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct RulesReport {
    /// Touched rules checked.
    pub checked: u32,
    /// Levels checked, over every rule.
    pub levels: u32,
    /// Rules judged fail.
    pub failed: u32,
    /// Rules judged not sure.
    pub not_sure: u32,
    /// Whether this build carries the sensitivity rules (the `sensitivity` feature).
    pub built: bool,
    /// The stage's wall time.
    pub ms: u64,
    /// One row per rule, in the rules file's order, then the five-match rule.
    pub rows: Vec<RuleRow>,
}

/// One rule of the rules stage.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RuleRow {
    /// The job, or `five-match` for the five-match rule.
    pub job: String,
    pub statistic: String,
    /// The low and the high level: ratings, centimetres, years or percent of adaptation;
    /// for the five-match rule, the average and the top copy's rating.
    pub levels: [f64; 2],
    /// The statistic at the low and at the high level; absent for the five-match rule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<f64>,
    /// The relative move from low to high in the job's direction; absent for the five-match
    /// rule, or when it is not finite.
    #[serde(rename = "move", skip_serializing_if = "Option::is_none")]
    pub mv: Option<RuleInterval>,
    /// The job's share of the outcome move; for the five-match rule, the pooled share of
    /// runs the top copy won. Absent when no arm was played, or when the arms' outcome did
    /// not move, so the share has no finite value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share: Option<RuleInterval>,
    /// The five-match rule's share of runs per role.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<BTreeMap<String, f64>>,
    /// `pass`, `fail` or `not_sure`.
    pub word: &'static str,
    /// The rule runs and reports: its word is information, and a full sensitivity run does
    /// not fail on it. Written only when set.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub report_only: bool,
}

/// An estimate and its 95 percent interval.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RuleInterval {
    pub est: f64,
    pub lo: f64,
    pub hi: f64,
}

/// The schema version of the calibrate run report: 2 since the registry, the verdicts and
/// the stage costs; 3 since the rules stage's rows; 4 since the real values of the bands
/// (`calib.real`, `calib.realism`, and the real columns of each verdict); 5 since the bands
/// that only report (`calib.reported`), the developer's acceptances (`calib.accepted`), the
/// match counts (`calib.counts`), and how each verdict row was judged (`judged`, `note`,
/// `accepted`); 6 since the whole-distribution bands (`upper` and `noise_floor` on a
/// verdict row); 7 since the formation field rating (`calib.nash`); 8 since the registry's
/// figures (`calib.figures`); 9 since a change run's styles sample (`calib.styles`). Every
/// key of an older version stays.
pub const REPORT_SCHEMA_VERSION: &str = "10";

/// The matches a run judged: every engine's rows, the fixtures both engines of a change run
/// played, and the matches of each engine.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub total_matches: u32,
    pub pairs: u32,
    pub observations: Vec<Observations>,
}

/// One engine's matches in a run: `changed` and `old` in a change run, `this` otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Observations {
    pub engine: &'static str,
    /// The engine's build: the first 16 hex characters of its executable's SHA-256.
    pub build: String,
    pub matches: u32,
}

impl Counts {
    /// The console line: `matches: T in total; P pairs; observations: ...`.
    pub fn line(&self) -> String {
        let per: Vec<String> = self
            .observations
            .iter()
            .map(|o| format!("{} engine {}", o.engine, o.matches))
            .collect();
        format!(
            "matches: {} in total; {} pairs; observations: {}\n",
            self.total_matches,
            self.pairs,
            per.join(", ")
        )
    }
}

/// The band registry a run was judged with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegistryInfo {
    pub version: u32,
    /// SHA-256 of the bands, 12 hex characters.
    pub digest: String,
}

/// The compact rows of a run, over every arm: the row format, the row files, the rows the
/// ledger counts, and the matches with a full recording, by reason. A match can have more
/// than one reason.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct RowsInfo {
    pub format: u32,
    /// `true` when this build records the realism measures (the `measures` feature).
    pub measures: bool,
    pub files: u32,
    pub rows: u32,
    pub recorded: Recorded,
}

/// Matches with a full recording (a statistics file and an event file), and why.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Recorded {
    pub matches: u32,
    /// About 1 in 16, chosen by fixture key.
    pub sample: u32,
    /// Failed, panicked, or a dark-path hit.
    pub error: u32,
    pub violation: u32,
    /// A measure outside the 1st to 99th percentile of its suite so far.
    pub extreme: u32,
    /// `--keep-events all`.
    pub all: u32,
}

/// The fixtures of a run, counted over every arm and suite.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Units {
    pub total: u32,
    /// Fixtures an earlier session of the run had finished.
    pub finished_before: u32,
    /// Fixtures this session played to the end.
    pub played: u32,
}

/// The old engine of a change run and its results on the run's fixtures.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BaseReport {
    /// `rev` for a built revision, `binary` for a ready executable.
    pub source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(rename = "build.id")]
    pub build_id: String,
    #[serde(rename = "content.hash")]
    pub content_hash: String,
    pub cache: BaseCache,
    #[serde(rename = "calib.bands")]
    pub bands: Vec<BandCheck>,
}

/// Where the old engine's results came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BaseCache {
    /// Fixtures whose result the cache held.
    pub hits: u32,
    /// Fixtures the old engine played now.
    pub played: u32,
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
    #[serde(rename = "darkpath.match_panicked")]
    pub match_panicked: u32,
    #[serde(rename = "validate.violations")]
    pub violations: usize,
    /// Always 0 since the worker processes are gone; kept for readers of earlier reports.
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
    fn schema_version(&self) -> &'static str {
        REPORT_SCHEMA_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::observe::{LawStats, MatchFigures, TacticsStats, TeamRef};
    use std::path::Path;

    fn bands() -> Registry {
        let dir =
            engine::ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"));
        Registry::load(&dir).unwrap()
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
            ratings: Vec::new(),
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
        let checks = b.checks(&wall);
        let names: Vec<&str> = checks.iter().map(|c| c.band.as_str()).collect();
        // Every band of the equal suite, in the band file's order, then the time budget.
        let registry = bands();
        let mut expected: Vec<&str> = registry
            .names()
            .into_iter()
            .filter(|b| registry.suites_of(b).unwrap().contains(&Suite::Equal))
            .filter(|b| registry.report_only(b).is_none())
            .collect();
        expected.push("wall_ms");
        assert_eq!(names, expected);
        assert!(names.starts_with(&[
            "goals_per_match",
            "shots_per_team",
            "possession_home_pct",
            "possession_away_pct"
        ]));
        // No real value is read in a unit test, so no band passes; each has its rule word.
        for c in checks.iter().filter(|c| c.band != "wall_ms") {
            assert!(!c.pass, "{c:?}");
            assert!(c.rule.is_some(), "{c:?}");
        }
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
    fn the_win_share_table_counts_wins_by_formation_and_a_mirror_from_both_sides() {
        let mut b = RunBuilder::new(bands());
        let names = vec![
            ["4-4-2".to_string(), "4-4-2".to_string()],
            ["4-4-2".to_string(), "4-3-3".to_string()],
            ["4-3-3".to_string(), "4-3-3".to_string()],
        ];
        b.plan_pairings(2, names);
        // The 4-4-2 mirror: one side wins, then a draw.
        b.add_pairing(record([3, 0], [12, 10], [50.0, 50.0]), 0, 0);
        b.add_pairing(record([1, 1], [12, 10], [50.0, 50.0]), 0, 1);
        // 4-4-2 v 4-3-3: the 4-4-2 wins at home, then the 4-3-3 wins at home (the 4-4-2
        // away on side 1), then a draw.
        b.add_pairing(record([2, 1], [12, 10], [50.0, 50.0]), 1, 0);
        b.add_pairing(record([3, 0], [12, 10], [50.0, 50.0]), 1, 1);
        b.add_pairing(record([0, 0], [12, 10], [50.0, 50.0]), 1, 0);
        // The 4-3-3 mirror: two decisive matches.
        b.add_pairing(record([1, 0], [12, 10], [50.0, 50.0]), 2, 0);
        b.add_pairing(record([0, 2], [12, 10], [50.0, 50.0]), 2, 1);
        let (names, w, fewest) = b.formation_table().unwrap();
        assert_eq!(names, ["4-4-2", "4-3-3"]);
        assert_eq!(w[0][0], 0.25);
        assert_eq!(w[0][1], 1.0 / 3.0);
        assert_eq!(w[1][0], 1.0 / 3.0);
        assert_eq!(w[1][1], 0.5);
        assert_eq!(fewest, 2);

        // A pairing with no finished match: no table.
        let mut partial = RunBuilder::new(bands());
        partial.plan_pairings(
            1,
            vec![
                ["4-4-2".to_string(), "4-4-2".to_string()],
                ["4-4-2".to_string(), "4-3-3".to_string()],
                ["4-3-3".to_string(), "4-3-3".to_string()],
            ],
        );
        partial.add_pairing(record([1, 0], [12, 10], [50.0, 50.0]), 0, 0);
        partial.add_pairing(record([1, 0], [12, 10], [50.0, 50.0]), 2, 0);
        assert_eq!(partial.formation_table(), Err((2, 3)));
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
        let checks = b.checks(&BTreeMap::new());
        let rows: Vec<(&str, Option<&str>, f64)> = checks
            .iter()
            .map(|c| (c.band.as_str(), c.pairing.as_deref(), c.value))
            .collect();
        assert_eq!(
            rows,
            [
                ("goals_per_match", Some("4-4-2 v 4-4-2"), 3.0),
                ("ten_plus_goals_share", Some("4-4-2 v 4-4-2"), 0.0),
                ("goalless_share", Some("4-4-2 v 4-4-2"), 0.0),
                ("goals_per_match", Some("4-4-2 v 4-3-3"), 5.5),
                ("ten_plus_goals_share", Some("4-4-2 v 4-3-3"), 0.5),
                ("goalless_share", Some("4-4-2 v 4-3-3"), 0.5),
            ]
        );
        // Two matches a pairing are under every band's minimum exposure: none passes.
        assert!(checks.iter().all(|c| !c.pass && c.rule.is_some()));
        assert!(checks.iter().all(|c| c.suite == "formations"));
    }

    #[test]
    fn a_band_miss_and_an_outlier_are_named() {
        let mut b = RunBuilder::new(bands());
        b.plan(Suite::Equal, 1);
        // Six shots and 70 percent possession: an outlier, and both bands miss.
        assert!(b.add(Suite::Equal, record([5, 1], [6, 6], [70.0, 30.0]), None));
        let figures = b.figures();
        let checks = b.checks(&BTreeMap::new());
        // One match is far under every band's minimum exposure: no band passes.
        assert!(
            checks
                .iter()
                .filter(|c| c.band != "wall_ms")
                .all(|c| !c.pass),
            "{checks:?}"
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
        let checks = b.checks(&BTreeMap::new());
        assert_eq!(checks.len(), 1);
        assert!(!checks[0].pass);
    }

    #[test]
    fn a_suite_slower_than_its_budget_fails_the_wall_band() {
        let mut b = RunBuilder::new(bands());
        b.plan(Suite::Equal, 1000);
        let wall = BTreeMap::from([(Suite::Equal, 1_800_001)]);
        let checks = b.checks(&wall);
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
        let wall = BTreeMap::from([(Suite::Equal, 1_000)]);
        let checks = b.checks(&wall);
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
        // 1-0; striker 1-0 and 3-2. The limit is the top of the band's pass region, design
        // 0.8 plus its margin 0.48: the keeper's full side (1.5) and the striker's (2.0) sit
        // above 1.28 x 1.
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
        assert!((f.limit - 1.28).abs() < 1e-9, "{}", f.limit);
        let arm = |name: &str| f.arms.iter().find(|a| a.arm == name).unwrap();
        assert_eq!((arm("keeper").full, arm("keeper").reduced), (1.5, 0.0));
        assert!(arm("centre-back").pass);
        assert!(!arm("keeper").pass, "the full side scored 1.5 against 1.28");
        assert!(
            !arm("striker").pass,
            "the full side scored 2.0 against 1.28"
        );
        assert!(!f.pass);
        let checks = b.checks(&BTreeMap::new());
        let rows: Vec<(&str, Option<&str>, f64)> = checks
            .iter()
            .filter(|c| c.suite == "red-card")
            .map(|c| (c.band.as_str(), c.pairing.as_deref(), c.value))
            .collect();
        assert_eq!(
            rows,
            [
                ("reduced_minus_full", Some("keeper"), -1.5),
                ("full_over_control", Some("keeper"), 1.5),
                ("reduced_minus_full", Some("centre-back"), -0.5),
                ("full_over_control", Some("centre-back"), 1.0),
                ("reduced_minus_full", Some("striker"), -1.0),
                ("full_over_control", Some("striker"), 2.0),
            ]
        );
        // Two matches an arm are under the minimum exposure: the rule word is not a pass.
        assert!(
            checks
                .iter()
                .filter(|c| c.suite == "red-card")
                .all(|c| !c.pass && c.rule.is_some())
        );
    }

    #[test]
    fn every_band_belongs_to_a_suite_and_the_red_card_suite_is_not_in_all() {
        let r = bands();
        // A scenario band is judged by its slow test and plays in no suite.
        for band in r.names() {
            let scenario = matches!(
                r.band(band).unwrap().measure,
                bands::Measure::Scenario { .. }
            );
            assert_eq!(r.suites_of(band).unwrap().is_empty(), scenario, "{band}");
        }
        assert!(r.suites_of("no_such_band").is_none());
        assert!(!Suite::ALL.contains(&Suite::RedCard));
        assert_eq!(
            r.suites_of("goals_per_match").unwrap(),
            [Suite::Equal, Suite::Formations]
        );
    }

    /// Varied matches: goals, shots, possession, cards, passes and set pieces all change
    /// from match to match, and one match fails.
    fn varied(n: u32) -> Vec<MatchStats> {
        (0..n)
            .map(|i| {
                let mut r = record(
                    [i % 4, (i * 7 + 1) % 3],
                    [8 + i % 9, 6 + (i * 5) % 11],
                    [
                        40.0 + f64::from(i % 21) * 0.7,
                        60.0 - f64::from(i % 21) * 0.7,
                    ],
                );
                r.laws.yellow = [i % 5, (i + 2) % 4];
                r.laws.red = [u32::from(i % 13 == 0), 0];
                r.laws.corners = [3 + i % 6, 2 + (i * 3) % 7];
                r.laws.throw_ins = [18 + i % 9, 20 + i % 5];
                r.laws.goal_kicks = [6 + i % 7, 5 + i % 4];
                r.figures.passes = [380 + (i * 13) % 90, 350 + (i * 17) % 120];
                r.figures.pass_accuracy_pct = [
                    76.3 + f64::from(i % 11) * 0.9,
                    81.7 - f64::from(i % 7) * 1.1,
                ];
                r.figures.xg = [0.37 * f64::from(i % 5), 1.13 + 0.21 * f64::from(i % 3)];
                if i == 5 {
                    r.outcome = "error".into();
                }
                r
            })
            .collect()
    }

    /// The registry's checks equal the hard-coded figures they replaced, value for value,
    /// on the same matches: the equal suite's figures, each pairing's, the stronger club's
    /// win rate, and the red-card arms.
    #[test]
    fn the_registry_checks_equal_the_hard_coded_figures() {
        let mut b = RunBuilder::new(bands());
        b.plan(Suite::Equal, 60);
        b.plan(Suite::Strength, 60);
        b.plan_pairings(
            30,
            vec![
                ["4-4-2".into(), "4-3-3".into()],
                ["4-3-3".into(), "3-5-2".into()],
            ],
        );
        b.plan(Suite::RedCard, 40);
        for (i, r) in varied(60).into_iter().enumerate() {
            b.add(Suite::Equal, r.clone(), None);
            b.add(Suite::Strength, r.clone(), Some(i % 2));
            b.add_pairing(r.clone(), i % 2, (i / 2) % 2);
            if i < 40 {
                b.add_red_card(r, (i / 3) % 4);
            }
        }
        let figures = b.figures();
        let checks = b.checks(&BTreeMap::new());
        let value = |band: &str, suite: &str, pairing: Option<&str>| {
            checks
                .iter()
                .find(|c| c.band == band && c.suite == suite && c.pairing.as_deref() == pairing)
                .unwrap_or_else(|| panic!("{band} {suite} {pairing:?}"))
                .value
        };
        let f = &figures[&Suite::Equal];
        for (band, hard) in [
            ("goals_per_match", f.goals_per_match_mean),
            ("shots_per_team", f.shots_per_team_mean),
            ("possession_home_pct", f.possession_home_mean),
            ("possession_away_pct", f.possession_away_mean),
            ("ten_plus_goals_share", f.ten_plus_goals_share),
            ("sending_off_share", f.sending_off_share),
            ("yellow_cards_per_team", f.yellow_cards_per_team_mean),
            ("shots_on_target_share", f.shots_on_target_share),
            ("goals_per_xg", f.goals_per_xg),
            ("passes_per_team", f.passes_per_team_mean),
            ("pass_accuracy_pct", f.pass_accuracy_pct_mean),
            ("corners_per_team", f.corners_per_team_mean),
            ("throw_ins_per_match", f.throw_ins_per_match_mean),
            ("goal_kicks_per_match", f.goal_kicks_per_match_mean),
            ("goalless_share", f.goalless_share),
        ] {
            assert_eq!(
                value(band, "equal", None).to_bits(),
                hard.to_bits(),
                "{band}"
            );
        }
        let stronger = figures[&Suite::Strength].stronger.unwrap();
        assert_eq!(
            value("stronger_team_win_rate", "strength", None),
            stronger.win_rate
        );
        for p in b.pairing_figures() {
            let label = p.label();
            let at = Some(label.as_str());
            assert_eq!(
                value("goals_per_match", "formations", at),
                p.goals_per_match_mean
            );
            assert_eq!(
                value("ten_plus_goals_share", "formations", at),
                p.ten_plus_goals_share
            );
            assert_eq!(value("goalless_share", "formations", at), p.goalless_share);
        }
        let red = b.red_card_figures().unwrap();
        for arm in &red.arms {
            let at = Some(arm.arm.as_str());
            // The arm figures are rounded to 4 places before this arithmetic.
            let reduced = value("reduced_minus_full", "red-card", at);
            assert!(
                (reduced - (arm.reduced - arm.full)).abs() < 2e-4,
                "{reduced} {arm:?}"
            );
            let ratio = value("full_over_control", "red-card", at);
            assert!(
                (ratio - arm.full / red.control[0]).abs() < 1e-3,
                "{ratio} {arm:?}"
            );
        }
        // Every row's pass is its rule word, and its sampling error is a real one.
        for c in checks.iter().filter(|c| c.band != "wall_ms") {
            let word = c.rule.as_ref().map(|r| r.word);
            assert_eq!(c.pass, word == Some(pass_rule::rule::Word::Pass), "{c:?}");
        }
        // The record-backed bands carry a real error; a rare share with no event has none.
        let record_backed = &V2;
        assert!(
            checks
                .iter()
                .filter(|c| c.suite == "equal" && record_backed.contains(&c.band.as_str()))
                .all(|c| c.se > 0.0 || c.band == "ten_plus_goals_share")
        );
    }
}

/// Helpers the report's unit tests share.
#[cfg(test)]
pub mod tests_support {
    use engine::observe::{MatchStats, TeamRef};

    /// A finished match with every figure 0.
    pub fn blank_stats() -> MatchStats {
        MatchStats {
            owner_id: String::new(),
            match_id: String::new(),
            seed: 1,
            content_hash: String::new(),
            teams: [0, 1].map(|_| TeamRef {
                id: String::new(),
                name: String::new(),
            }),
            duration_ms: 0,
            outcome: "success".into(),
            ticks_per_s: 0.0,
            ticks_written: 0,
            validate_ran: true,
            validate_violations: 0,
            possession_changes: 0,
            ball_max_speed: 0.0,
            ball_idle_ticks: 0,
            goals: [0, 0],
            flags_on: Vec::new(),
            laws: Default::default(),
            tactics: Default::default(),
            figures: Default::default(),
            script: Default::default(),
            ratings: Vec::new(),
        }
    }
}

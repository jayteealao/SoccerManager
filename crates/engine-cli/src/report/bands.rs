//! The band registry, read from `realism-bands.json` in the content folder. The bands are
//! acceptance criteria, never tuning values: the calibration harness reads them and nothing
//! writes them.
//!
//! The file is a list of bands (version 6). Each band names its measure (a mean, a share,
//! or a pooled ratio of per-match fields from a closed catalog, one of three built-in
//! measures, a whole distribution, or a scenario that an engine slow test plays), the suites
//! that judge it, what it is judged against (the real value, or a design target with an
//! optional cap on its distance from the real value), and its margin: the spread between
//! leagues plus the smallest error a player would notice, each with its reason, and an
//! optional widening with its own reason. A provisional widening holds the margin floor until
//! the floor is set again on more leagues. Each band also states its definition version,
//! population, estimator, how each side is resampled, the exposure it needs before it may
//! pass, whether it is rare, and the engine's measured spread; and its full record: where
//! its real value is measured (a pointer into the real-data folder, a target of the
//! reference file of real targets by key, or the reason it has no real counterpart), the
//! sample size and date, its evidence label, its two confidence counts, its pitch condition,
//! and whether it gates. Every band gates, except the bands of a named report-only group.
//!
//! The loader refuses a margin under the 97 percent floor of its band, naming the margin the
//! band needs. The file holds no real value: the calibrate command reads it in place, or
//! from the reference file, at each judgement. An optional list of figures holds measures
//! reported beside the value a named build gave, never judged. A file of an earlier version
//! is refused: no migration can invent a margin or its reason.

use std::collections::BTreeMap;

use engine::EngineError;
use engine::data::{ContentDir, load_json_bytes, read_bytes};
use garde::Validate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::Suite;

/// The file name inside the content folder.
pub const BANDS_FILE: &str = "realism-bands.json";
/// The layout version this build writes and reads.
pub const REGISTRY_VERSION: u32 = 6;

const KIND: &str = "realism bands";

/// A closed range, `lo` to `hi`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Band {
    pub lo: f64,
    pub hi: f64,
}

impl Band {
    pub fn contains(&self, value: f64) -> bool {
        (self.lo..=self.hi).contains(&value)
    }
}

/// The strength suite's setting: the stronger club has every attribute times
/// `attribute_boost`. Its criterion is the band `stronger_team_win_rate`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StrongerTeam {
    #[garde(range(min = 1.0, max = 2.0))]
    pub attribute_boost: f64,
}

/// Over what a mean is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Per {
    /// The sum of the fields in each match, averaged over matches.
    Match,
    /// Each team's sum of the fields (`{side}` in a field name), averaged over teams.
    Team,
}

/// How a share compares a match's sum of fields with its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    #[serde(rename = ">=")]
    AtLeast,
    #[serde(rename = "==")]
    Equal,
    #[serde(rename = "<=")]
    AtMost,
}

impl Op {
    pub fn holds(self, sum: f64, value: f64) -> bool {
        match self {
            Op::AtLeast => sum >= value,
            Op::Equal => sum == value,
            Op::AtMost => sum <= value,
        }
    }
}

/// The measures that compare arms or sides of a suite, and so are not a sum of one match's
/// fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Builtin {
    /// The strength suite: the share of matches the boosted club wins. It passes above `lo`.
    StrongerTeamWinRate,
    /// The red-card suite, per arm: the reduced side's goals minus the full side's, per
    /// match. It passes at or below `hi`.
    ReducedMinusFull,
    /// The red-card suite, per arm: the full side's goals per match over the control's home
    /// goals per match. It passes at or below `hi`.
    FullOverControl,
}

/// A band's measure, over the per-match fields of the field catalog
/// ([`super::measures::SIDED`] and [`super::measures::MATCH`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Measure {
    /// The mean of the per-match (or per-team) sum of `of`.
    Mean {
        per: Per,
        of: Vec<String>,
    },
    /// The share of matches whose sum of `of` stands `op` `value`.
    Share {
        of: Vec<String>,
        op: Op,
        value: f64,
    },
    /// The sum of `num` over the sum of `den`, pooled over the suite.
    Ratio {
        num: Vec<String>,
        den: Vec<String>,
    },
    Builtin {
        name: Builtin,
    },
    /// The per-match mean of the sum of `of` minus the sum of `minus`, such as the home
    /// side's goals minus the away side's.
    Difference {
        of: Vec<String>,
        minus: Vec<String>,
    },
    /// The share of matches whose final score is one of `scores`, either side first.
    Scoreline {
        scores: Vec<[u32; 2]>,
    },
    /// The factorial-moment ratio `E[X(X-1)] / E[X]^2` of the per-team sum of `of` (each
    /// field as `<field>.{side}`): 1 for a Poisson count, above 1 for a count more spread.
    #[serde(rename = "factorial_ratio")]
    FactorialRatio {
        of: Vec<String>,
    },
    /// For each club with at least `window` matches, the best minus the worst per-match
    /// value of its `of` minus its opponent's `minus` (each field as `<field>.{side}`, the
    /// `minus` fields read for the other side) over its first `window` matches in fixture
    /// order; the mean over those clubs. It has no paired form, so a change run judges it
    /// on its range only.
    #[serde(rename = "club_range")]
    ClubRange {
        of: Vec<String>,
        minus: Vec<String>,
        window: u32,
    },
    /// A whole distribution, judged by its Wasserstein-1 distance from the real sample: each
    /// match's histogram of `of` over `bins` bins of `width`, the last bin holding everything
    /// above it.
    Distribution {
        of: Source,
        width: f64,
        bins: u32,
    },
    /// The scoreline table, judged by its total-variation distance from the real one: one
    /// cell per match, each side's goals capped at `max` as "max or more".
    Cells {
        max: u32,
    },
    /// A scenario an engine slow test plays, such as a penalty or a trailing side: judged by
    /// that test through the pass rule, never by a calibrate suite. `test` names it.
    Scenario {
        test: String,
    },
    /// A band the engine has no count for yet, such as the players' scans: it stays in the
    /// registry with its real target and reports only, and its row reads not sure with
    /// `reason`. It has no engine spread, and it is in no count and no floor.
    Pending {
        reason: String,
    },
}

/// What a distribution band takes from each match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// The match's goals, both sides summed: one point per match.
    Goals,
    /// Each side's passes: two points per match.
    Passes,
    /// The distance of each shot from the goal, both sides: a measures build's shot bins.
    ShotDistance,
    /// The passes of each possession chain, both sides: a measures build's chain bins.
    ChainPasses,
    /// The seconds of each possession chain, both sides: a measures build's chain bins.
    ChainSeconds,
    /// The match's shots, both sides summed, blocked shots counted: one point per match.
    Shots,
    /// The match's cards, yellow and red, both sides summed: one point per match.
    Cards,
}

impl Source {
    /// The source's name, as the band file writes it.
    pub fn code(self) -> &'static str {
        match self {
            Source::Goals => "goals",
            Source::Passes => "passes",
            Source::ShotDistance => "shot_distance",
            Source::ChainPasses => "chain_passes",
            Source::ChainSeconds => "chain_seconds",
            Source::Shots => "shots",
            Source::Cards => "cards",
        }
    }

    /// The bins of a measures build's rows, as `(width, bins)`, for the sources read from
    /// them; `None` for a source binned from the band record.
    pub fn row_bins(self) -> Option<(f64, u32)> {
        use crate::measure_rows::{CHAIN_PASS_BINS, CHAIN_SECOND_BINS, SHOT_BINS};
        // At most a few dozen bins.
        match self {
            Source::Goals | Source::Passes | Source::Shots | Source::Cards => None,
            Source::ShotDistance => Some((1.0, SHOT_BINS as u32)),
            Source::ChainPasses => Some((1.0, CHAIN_PASS_BINS as u32)),
            Source::ChainSeconds => Some((2.0, CHAIN_SECOND_BINS as u32)),
        }
    }
}

/// How a real value is reduced from its file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reduce {
    /// A CSV file: the mean over the kept rows of the sum of the key's columns.
    Mean,
    /// A CSV file: the share of kept rows whose sum stands `op` `value`. A JSON file: the
    /// value is a share from 0 to 1 over `real.sample`, which gives its standard error.
    Share,
    /// A CSV file: the share of distinct `group_by` values with at least one row whose key
    /// column, split on spaces, holds one of `tokens`.
    GroupShare,
    /// A CSV file: the share of the kept rows (by `weight`, when it names a column) whose
    /// key lies in `bins`, `lo` included and `hi` not.
    BinShare,
    /// A CSV file: the mean distance from the point the key names as `x:y` to `goal`.
    GoalDistance,
    /// A CSV file: the share of kept rows whose two key columns (`a:b`) hold one of
    /// `scores`, either way round.
    Scoreline,
    /// A CSV file: `n * sum(x(x-1)) / sum(x)^2` over the kept rows: 1 for a Poisson count.
    FactorialRatio,
    /// A CSV file: for each `group_by` value with at least `window` kept rows, the largest
    /// minus the smallest key over its first `window` rows; the mean over those groups.
    GroupRange,
    /// A CSV file: for each `group_by` value (columns joined by `+`), the largest key; the
    /// mean over the groups.
    GroupMax,
    /// A CSV file, for a distribution band: each `group_by` value is a match, and each kept
    /// row adds its key (a sum of columns, or a point `x:y` measured to `goal`) to the
    /// match's histogram.
    Histogram,
    /// A CSV file, for a cells band: each kept row is a match, its key the score `home:away`.
    Cells,
    /// A CSV file, for a distribution band of possession chains: the rows of each `group_by`
    /// value (a match) and `chain` value (a possession) are one chain, every row counted; its
    /// length is the rows whose key holds one of `tokens` (and whose `same` columns agree)
    /// when `count` is `passes`, or the seconds from its first row's `time` to its last
    /// row's `time` plus that row's key (a duration), within the first row's `period`, when
    /// `count` is `seconds`.
    Chains,
}

/// What a chains reduction measures of each chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainCount {
    Passes,
    Seconds,
}

/// Where a band's real value is measured: a file of the real-data folder and how to read it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pointer {
    /// The data set and pool, in words.
    pub dataset: String,
    /// The file name inside the real-data folder; a name ending in `.gz` is decompressed.
    pub file: String,
    /// A JSON file: the dotted path, `*` for the mean over every child. A CSV file: the
    /// column, or columns joined by `+` and summed per row.
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduce: Option<Reduce>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op: Option<Op>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// A CSV file: the rows kept, by column and the values it may hold.
    #[serde(default, rename = "where", skip_serializing_if = "Option::is_none")]
    pub filter: Option<BTreeMap<String, Vec<String>>>,
    /// A JSON file: the dotted path of a denominator; the value is the ratio.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub den: Option<String>,
    /// A factor on the value and its error, such as 0.5 for a per-match rate per team.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<Vec<String>>,
    /// A CSV file: the column a mean or a bin share is weighted by.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<String>,
    /// A CSV file: the rows kept, by numeric column and the half-open range `[lo, hi)` its
    /// value lies in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<BTreeMap<String, [f64; 2]>>,
    /// A CSV file: only the rows whose two columns hold the same text are kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same: Option<[String; 2]>,
    /// A bin share: the half-open range `[lo, hi)` of the key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bins: Option<[f64; 2]>,
    /// A goal distance: the point measured to, in the file's own units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<[f64; 2]>,
    /// A scoreline share: the scores counted, such as `5-1`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scores: Option<Vec<String>>,
    /// A group range: the rows of each group it reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<u32>,
    /// A CSV file: a kept row whose key, weight or denominator cell is blank is left out,
    /// where the file leaves a value unmeasured (such as an event without a duration).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub skip_blank: bool,
    /// A chains reduction: the column naming each row's possession.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<String>,
    /// A chains reduction of seconds: the column of each row's time, in seconds or as
    /// `HH:MM:SS.mmm`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    /// A chains reduction of seconds: the column of each row's period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<String>,
    /// A chains reduction: what it measures of each chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<ChainCount>,
}

/// A band that measures an engine experiment no real match repeats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoCounterpart {
    /// Why the band has no real counterpart.
    pub none: String,
}

/// A band whose real value is a target of the reference file of real targets, named by its
/// key; the band file holds the key and no figure (version 6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetRef {
    pub target: String,
}

/// The source of a band's real value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RealSource {
    Pointer(Box<Pointer>),
    NoCounterpart(NoCounterpart),
    Target(TargetRef),
}

/// A band's real record: its source, the sample size behind it, and the date it was
/// measured (or, with no counterpart, the date of the decision).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealRecord {
    pub source: RealSource,
    pub sample: u64,
    pub date: String,
}

impl RealRecord {
    /// The reason, when the band has no real counterpart.
    pub fn no_counterpart(&self) -> Option<&str> {
        match &self.source {
            RealSource::NoCounterpart(n) => Some(&n.none),
            RealSource::Pointer(_) | RealSource::Target(_) => None,
        }
    }

    /// The reference-file key, when the real value is a target of that file.
    pub fn target_key(&self) -> Option<&str> {
        match &self.source {
            RealSource::Target(t) => Some(&t.target),
            RealSource::Pointer(_) | RealSource::NoCounterpart(_) => None,
        }
    }
}

/// Which evidence a band's real value is: calibration evidence, which tuning may read, or
/// validation evidence, which nothing was tuned on. Only validation supports a realism claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Evidence {
    Calibration,
    Validation,
}

/// How precisely the real matches were measured: the units behind the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub count: u64,
}

/// How widely the real value generalises: the competitions, the distinct seasons and the
/// clubs behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generalisation {
    pub competitions: u32,
    pub seasons: u32,
    pub clubs: u32,
}

impl Generalisation {
    /// Competitions times seasons: the number the floor is set on.
    pub fn units(&self) -> u64 {
        u64::from(self.competitions) * u64::from(self.seasons)
    }
}

/// A band's two confidence records, as numbers; the report derives a word from each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Confidence {
    pub measurement: Measurement,
    pub generalisation: Generalisation,
}

/// The pitch and weather condition a band holds for. Only `dry` is played today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Condition {
    Dry,
    Wet,
    Worn,
    Windy,
}

/// One band of the registry, in memory. `lo` and `hi` are its design target plus and minus
/// its margin (open for a band judged against the real value), and `smallest_shift` its
/// noticeable error, for the readers that still take a range.
#[derive(Debug, Clone, PartialEq)]
pub struct BandDef {
    pub band: String,
    /// The suites that judge it: `equal`, `strength`, `formations` (per pairing), and
    /// `red-card` (per arm).
    pub suites: Vec<String>,
    pub measure: Measure,
    /// The design range: the band passes on it.
    pub lo: f64,
    pub hi: f64,
    /// The smallest change of the measure that matters: a change run plays until it can
    /// see a change this size with 80 percent power, or reports the band not sure.
    pub smallest_shift: f64,
    /// Bands that belong together, such as the two possession shares.
    pub group: Option<String>,
    /// The real record; `None` only for a figure shown as a band.
    pub real: Option<RealRecord>,
    /// `None` only for a figure shown as a band.
    pub evidence: Option<Evidence>,
    /// `None` only for a figure shown as a band.
    pub confidence: Option<Confidence>,
    pub condition: Condition,
    /// The most this band may sit from its dry band; never on a dry band.
    pub max_from_dry: Option<f64>,
    /// The band changes the verdict. A band with `gates: false` only reports.
    pub gates: bool,
    /// Why the band only reports and never changes the verdict: its named report-only group
    /// and the group's reason. Derived on load.
    pub report_only: Option<String>,
    /// The pass rule's fields: the band's target, margin and definition; `None` only for a
    /// figure shown as a band.
    pub rule: Option<Rule>,
}

/// The part of a band that decides its verdict, as the digests hash it.
#[derive(Serialize)]
struct Judgement<'a> {
    band: &'a str,
    suites: &'a [String],
    measure: &'a Measure,
    lo: f64,
    hi: f64,
    smallest_shift: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gates: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    report_only: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rule: Option<&'a Rule>,
}

impl BandDef {
    pub fn range(&self) -> Band {
        Band {
            lo: self.lo,
            hi: self.hi,
        }
    }

    /// `true` when `suite` judges the band.
    pub fn judged_in(&self, suite: Suite) -> bool {
        self.suites.iter().any(|s| s == suite.code())
    }

    /// `true` when the band reads a field only a `measures` build records.
    pub fn needs_measures(&self) -> bool {
        if let Measure::Distribution { of, .. } = &self.measure {
            return of.row_bins().is_some();
        }
        super::measures::fields_of(&self.measure)
            .into_iter()
            .any(super::measures::is_measure_field)
    }

    /// `true` when the band reads a match's extras: a measures field, or the clubs of a
    /// club range.
    pub fn needs_extras(&self) -> bool {
        self.needs_measures() || matches!(self.measure, Measure::ClubRange { .. })
    }

    /// `true` for a band judged as a whole distribution: a distribution or a cells band.
    pub fn is_distribution(&self) -> bool {
        matches!(
            self.measure,
            Measure::Distribution { .. } | Measure::Cells { .. }
        )
    }

    fn judgement(&self) -> Judgement<'_> {
        Judgement {
            band: &self.band,
            suites: &self.suites,
            measure: &self.measure,
            lo: self.lo,
            hi: self.hi,
            smallest_shift: self.smallest_shift,
            group: self.group.as_deref(),
            gates: (!self.gates).then_some(false),
            report_only: self.report_only.as_deref(),
            rule: self.rule.as_ref(),
        }
    }

    /// A short digest of everything that decides the band's verdict: its measure, suites,
    /// range and smallest shift, and whether it only reports. The real record is reported
    /// beside the verdict and never changes it, so it is left out.
    pub fn digest(&self) -> String {
        let bytes = serde_json::to_vec(&self.judgement()).unwrap_or_default();
        engine::data::hex12(&Sha256::digest(bytes))
    }
}

/// The band registry: the suite settings and the bands.
#[derive(Debug, Clone, PartialEq)]
pub struct Registry {
    pub schema_version: u32,
    /// Matches per suite the wall-time budget is set for.
    pub sample_size: u32,
    /// The wall-time budget of one suite of `sample_size` matches, in minutes.
    pub wall_minutes_per_sample: f64,
    pub stronger_team: StrongerTeam,
    /// The generalisation (competitions times seasons) under which a real value is thin: the
    /// record's word, which never changes a verdict.
    pub generalisation_floor: u32,
    pub bands: Vec<BandDef>,
    /// The figures: measures reported beside an earlier build's value, never judged.
    pub figures: Vec<FigureDef>,
    /// The pass rule's settings.
    pub pass_rule: Option<PassRuleSettings>,
    /// The named groups of bands that only report.
    pub report_only_groups: Vec<ReportOnlyGroup>,
    /// The run kinds: seed sets, rare-margin factors and fixed counts.
    pub kinds: Option<super::kinds::KindTable>,
    /// The measured agreement between paired matches the counts were computed from.
    pub pair_agreement: Vec<super::kinds::AgreementRecord>,
    /// The kind a run judges with: its rare bands' margins already carry the kind's factor.
    /// `None` for the registry as the file holds it.
    pub active_kind: Option<super::kinds::Kind>,
    /// SHA-256, as 12 hex characters, of the reference file of real targets read with the
    /// registry; `None` when no reference file was here. It joins the registry's digest, as
    /// a band that names a target is judged against that file.
    pub reference: Option<String>,
    /// The targets of the reference file that the bands name, by key, as read with the
    /// registry; empty when no reference file was here.
    pub targets: BTreeMap<String, crate::real_targets::Target>,
}

/// A figure: a measure reported beside the value an earlier build gave, with no range; it
/// never changes a verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FigureDef {
    pub figure: String,
    /// The suites that report it.
    pub suites: Vec<String>,
    pub measure: Measure,
    /// The value an earlier build gave.
    pub today: Today,
}

/// The value a figure had on an earlier build, and where it was measured.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Today {
    pub value: f64,
    /// The build, as a commit.
    pub build: String,
    /// How the value was measured, in words.
    pub source: String,
}

impl FigureDef {
    /// The figure as a band that only reports, so the band measures evaluate it: its range
    /// is empty and it never gates.
    pub fn as_band(&self) -> BandDef {
        BandDef {
            band: self.figure.clone(),
            suites: self.suites.clone(),
            measure: self.measure.clone(),
            lo: self.today.value,
            hi: self.today.value,
            smallest_shift: 1.0,
            group: None,
            real: None,
            evidence: None,
            confidence: None,
            condition: Condition::Dry,
            max_from_dry: None,
            gates: false,
            report_only: Some("a figure: reported beside an earlier build's value".into()),
            rule: None,
        }
    }
}

impl Registry {
    /// Loads the registry from the content folder, with the reference file of real targets
    /// in `gate/` when it is here. A file of another version is refused, as is a band without
    /// a required field, with an unknown field, an unknown catalog field, no fields, an
    /// unknown suite, a name used twice, a margin that is not its parts, or a margin under its
    /// floor; the refusal names the file, the band and the field, every gap of one load at
    /// once.
    pub fn load(dir: &ContentDir) -> anyhow::Result<Self> {
        let bytes = read_bytes(&dir.path(BANDS_FILE), BANDS_FILE)?;
        // A band may name a target of the reference file in gate/: the key must be in the
        // file when the file is here, and the target's error joins the margin floor.
        let reference_path = crate::real_targets::default_file(dir.root());
        let reference = reference_path
            .is_file()
            .then(|| crate::real_targets::ReferenceFile::load(&reference_path))
            .transpose()?;
        Ok(match reference {
            Some(file) => {
                let mut reg = Self::parse(&bytes, Some(&file))?;
                let reference_bytes = read_bytes(&reference_path, "gate/real-targets.json")?;
                reg.reference = Some(engine::data::hex12(&Sha256::digest(reference_bytes)));
                reg
            }
            None => Self::from_bytes(&bytes)?,
        })
    }

    /// [`Self::load`] over bytes already in memory, with no reference file: a band's margin
    /// floor is then set on the engine's error alone.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EngineError> {
        Self::parse(bytes, None)
    }

    /// The load itself, with the reference file of real targets when one is here.
    pub fn parse(
        bytes: &[u8],
        reference: Option<&crate::real_targets::ReferenceFile>,
    ) -> Result<Self, EngineError> {
        let doc = serde_json::from_slice::<Value>(bytes).ok();
        let version = doc.as_ref().and_then(|v| v["schema_version"].as_u64());
        let refuse = |(field, reason): (String, String)| EngineError::Data {
            kind: KIND,
            path: BANDS_FILE.to_string(),
            field,
            reason,
        };
        if let Some(v) = version.filter(|v| (2..REGISTRY_VERSION as u64).contains(v)) {
            return Err(refuse((
                "schema_version".into(),
                format!(
                    "schema_version {v} is an earlier layout; this build reads \
                     {REGISTRY_VERSION}, whose bands hold a target, a margin with its reasons, \
                     a definition version, population, estimator, resampling, minimum exposure \
                     and engine spread, which no migration can invent"
                ),
            )));
        }
        if version == Some(u64::from(REGISTRY_VERSION)) {
            if let Some(doc) = &doc {
                refusals(required(doc)).map_err(refuse)?;
            }
        }
        let mut registry =
            load_json_bytes::<RegistryV6>(KIND, bytes, BANDS_FILE, REGISTRY_VERSION, &())?
                .value
                .into_registry();
        registry.check().map_err(refuse)?;
        let mut list = registry.check_v6();
        if let Some(file) = reference {
            for (i, b) in registry.bands.iter().enumerate() {
                if let Some(key) = b.real.as_ref().and_then(RealRecord::target_key) {
                    match file.target(key) {
                        Some(t) => {
                            registry.targets.insert(key.to_string(), t.clone());
                        }
                        None => list.push((
                            format!("bands[{i}] ({}).real.source.target", b.band),
                            format!("{key} is not a target of {}", crate::real_targets::FILE),
                        )),
                    }
                }
            }
        }
        let target_se = |b: &BandDef| {
            let key = b.real.as_ref().and_then(RealRecord::target_key)?;
            reference?.target(key).map(|t| t.se)
        };
        for f in registry.floor_refusals(&target_se) {
            let i = registry
                .bands
                .iter()
                .position(|b| b.band == f.band)
                .unwrap_or(0);
            let field = if f.which == "cap" {
                "target.cap"
            } else {
                "margin.value"
            };
            list.push((format!("bands[{i}] ({}).{field}", f.band), f.reason()));
        }
        refusals(list).map_err(refuse)?;
        Ok(registry)
    }

    /// The registry's own rules, beyond its layout. A refusal names the field.
    fn check(&self) -> Result<(), (String, String)> {
        let mut seen = std::collections::BTreeSet::new();
        for (i, b) in self.bands.iter().enumerate() {
            let at = |field: &str| format!("bands[{i}] ({}).{field}", b.band);
            if b.band.trim().is_empty() {
                return Err((format!("bands[{i}].band"), "is empty".into()));
            }
            if !seen.insert(b.band.as_str()) {
                return Err((at("band"), "names a band already in the file".into()));
            }
            let scenario = matches!(b.measure, Measure::Scenario { .. });
            if scenario && !b.suites.is_empty() {
                return Err((
                    at("suites"),
                    "a scenario band is judged by its slow test, so it names no suite".into(),
                ));
            }
            if !scenario && b.suites.is_empty() {
                return Err((at("suites"), "names no suite".into()));
            }
            for s in &b.suites {
                if Suite::parse(s).is_none() {
                    return Err((
                        at("suites"),
                        format!("{s} is not a suite (equal, strength, formations, red-card)"),
                    ));
                }
            }
            super::measures::check(&b.measure).map_err(|why| (at("measure"), why))?;
        }
        for (i, f) in self.figures.iter().enumerate() {
            let at = |field: &str| format!("figures[{i}] ({}).{field}", f.figure);
            if f.figure.trim().is_empty() {
                return Err((format!("figures[{i}].figure"), "is empty".into()));
            }
            if !seen.insert(f.figure.as_str()) {
                return Err((
                    at("figure"),
                    "names a band or figure already in the file".into(),
                ));
            }
            if f.suites.is_empty() {
                return Err((at("suites"), "names no suite".into()));
            }
            if let Some(s) = f.suites.iter().find(|s| Suite::parse(s).is_none()) {
                return Err((
                    at("suites"),
                    format!("{s} is not a suite (equal, strength, formations, red-card)"),
                ));
            }
            if !f.today.value.is_finite() {
                return Err((at("today.value"), "is not a number".into()));
            }
            if f.today.build.trim().is_empty() || f.today.source.trim().is_empty() {
                return Err((at("today"), "needs the build and the source".into()));
            }
            if matches!(
                f.measure,
                Measure::Builtin { .. }
                    | Measure::Distribution { .. }
                    | Measure::Cells { .. }
                    | Measure::Scenario { .. }
                    | Measure::Pending { .. }
            ) {
                return Err((
                    at("measure"),
                    "a figure is a mean, a share or a ratio over the catalog".into(),
                ));
            }
            super::measures::check(&f.measure).map_err(|why| (at("measure"), why))?;
        }
        Ok(())
    }

    /// The band named `name`.
    pub fn band(&self, name: &str) -> Option<&BandDef> {
        self.bands.iter().find(|b| b.band == name)
    }

    /// The fixed range of the band `name`: its design target plus and minus its margin. A
    /// band judged against the real value has none; the pass rule sets its word.
    pub fn range(&self, name: &str) -> Option<Band> {
        let b = self.band(name)?;
        match b.rule.as_ref().map(|r| r.target) {
            Some(Target::Real) => None,
            _ => Some(b.range()),
        }
    }

    /// Every band name, in file order.
    pub fn names(&self) -> Vec<&str> {
        self.bands.iter().map(|b| b.band.as_str()).collect()
    }

    /// The suites that judge `band`, in the order of [`Suite`]; `None` for an unknown band.
    pub fn suites_of(&self, band: &str) -> Option<Vec<Suite>> {
        let def = self.band(band)?;
        let mut out: Vec<Suite> = def.suites.iter().filter_map(|s| Suite::parse(s)).collect();
        out.sort();
        out.dedup();
        Some(out)
    }

    /// Why the band `name` only reports, when it does.
    pub fn report_only(&self, name: &str) -> Option<&str> {
        self.band(name).and_then(|b| b.report_only.as_deref())
    }

    /// SHA-256, as 12 hex characters, over every band's judgement in file order: a changed
    /// target, margin, measure, suite or gating changes it; the real record does not.
    pub fn digest(&self) -> String {
        let views: Vec<Judgement<'_>> = self.bands.iter().map(BandDef::judgement).collect();
        let bytes = serde_json::to_vec(&views).unwrap_or_default();
        engine::data::hex12(&Sha256::digest(bytes))
    }

    /// The gate's registry digest: [`Self::digest`] with the reference file of real targets
    /// read with the registry. A registry read with a reference file is another gate from
    /// the same registry read without one, or with another file; without one it is
    /// [`Self::digest`].
    pub fn gate_digest(&self) -> String {
        let views: Vec<Judgement<'_>> = self.bands.iter().map(BandDef::judgement).collect();
        let mut bytes = serde_json::to_vec(&views).unwrap_or_default();
        if let Some(reference) = &self.reference {
            bytes.extend_from_slice(b"\nreference:");
            bytes.extend_from_slice(reference.as_bytes());
        }
        engine::data::hex12(&Sha256::digest(bytes))
    }

    /// The wall-time budget of a suite of `matches` matches, in milliseconds.
    pub fn wall_budget_ms(&self, matches: u32) -> u64 {
        let per_match = self.wall_minutes_per_sample * 60_000.0 / f64::from(self.sample_size);
        // At most 100 000 matches of at most 10 000 minutes: far inside u64.
        (per_match * f64::from(matches)).round() as u64
    }
}

/// The keys a real source may hold.
const SOURCE_KEYS: [&str; 25] = [
    "none",
    "target",
    "dataset",
    "file",
    "key",
    "reduce",
    "op",
    "value",
    "where",
    "den",
    "scale",
    "group_by",
    "tokens",
    "weight",
    "range",
    "same",
    "bins",
    "goal",
    "scores",
    "window",
    "skip_blank",
    "chain",
    "time",
    "period",
    "count",
];

/// The record rules of a band on the raw document, every refusal collected: a source that
/// is not one of a pointer, a reference-file target or a no-counterpart record, a blank
/// text, a sample that does not fit the source, an unknown evidence word, and a cap on a dry
/// band.
fn record_rules(b: &Value, at: &dyn Fn(&str) -> String, out: &mut Vec<(String, String)>) {
    let blank = |v: &Value| v.as_str().is_none_or(|s| s.trim().is_empty());
    let Some(source) = b["real"]["source"].as_object() else {
        if b["real"].get("source").is_some() {
            out.push((at("real.source"), "must be an object".into()));
        }
        return;
    };
    if let Some(key) = source.keys().find(|k| !SOURCE_KEYS.contains(&k.as_str())) {
        out.push((at(&format!("real.source.{key}")), "is not a field".into()));
    }
    let none = source.contains_key("none");
    let target = source.contains_key("target");
    let pointer = ["dataset", "file", "key"]
        .iter()
        .any(|k| source.contains_key(*k));
    if [none, target, pointer].iter().filter(|x| **x).count() > 1 {
        out.push((
            at("real.source"),
            "holds more than one of none, a target and a pointer; use one".into(),
        ));
    } else if none {
        if blank(&source["none"]) {
            out.push((at("real.source.none"), "needs a reason".into()));
        }
    } else if target {
        if blank(&source["target"]) {
            out.push((at("real.source.target"), "is blank".into()));
        }
    } else {
        for k in ["dataset", "file", "key"] {
            match source.get(k) {
                None => {
                    out.push((
                        at("real.source"),
                        "needs dataset, file and key, a target, or none with a reason".into(),
                    ));
                    break;
                }
                Some(v) if blank(v) => {
                    out.push((at(&format!("real.source.{k}")), "is blank".into()));
                }
                Some(_) => {}
            }
        }
    }
    match (none, b["real"]["sample"].as_u64()) {
        (_, None) if b["real"].get("sample").is_some() => {
            out.push((at("real.sample"), "must be a whole number".into()));
        }
        (false, Some(0)) => out.push((
            at("real.sample"),
            "is 0; a real value needs the sample size behind it".into(),
        )),
        (true, Some(n)) if n > 0 => out.push((
            at("real.sample"),
            "is above 0, but the band has no real counterpart".into(),
        )),
        _ => {}
    }
    if b.get("evidence").is_some()
        && !matches!(b["evidence"].as_str(), Some("calibration" | "validation"))
    {
        out.push((at("evidence"), "must be calibration or validation".into()));
    }
    if b["condition"] == "dry" && b.get("max_from_dry").is_some() {
        out.push((
            at("max_from_dry"),
            "a dry band is the reference; only another condition has a cap".into(),
        ));
    }
}

// ---------------------------------------------------------------------------------------
// The pass rule's fields.
// ---------------------------------------------------------------------------------------

/// The pass rule's settings, at file level: the error rate of each side of a band's
/// interval, and the chance a sound engine must have to pass every gating band at once. The
/// run the margin floor is set on is each run kind's count ([`super::kinds`]).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassRuleSettings {
    pub alpha: f64,
    pub sound_pass: f64,
}

/// The run a floor is checked at: matches, leagues, and the factor of a rare band's margin.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FloorAt {
    matches: f64,
    leagues: f64,
    rare_factor: f64,
}

/// A named group of bands that only report: at most `most` bands of `group`, each with
/// `gates: false`, for the stated reason. No other band may report only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportOnlyGroup {
    pub group: String,
    pub most: u32,
    pub reason: String,
}

/// What a band is judged against.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    /// The real value decides.
    Real,
    /// A design value decides; with a cap the band must also sit within `cap` of the real
    /// value.
    Design { value: f64, cap: Option<f64> },
}

/// A band's margin: the spread between leagues plus the smallest error a player would
/// notice, each with its reason, plus an optional widening with its own reason. The margin
/// is a design choice; the test carries the sampling noise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Margin {
    pub value: f64,
    pub league_spread: f64,
    pub noticeable_error: f64,
    /// Why the noticeable error is this size.
    pub reason: String,
    /// Where the league spread comes from.
    pub spread_reason: String,
    pub design_choice: bool,
    /// A part added so the margin holds its floor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widening: Option<Widening>,
}

impl Margin {
    /// The widening's value, 0 when the margin has none.
    pub fn widened_by(&self) -> f64 {
        self.widening.as_ref().map_or(0.0, |w| w.value)
    }
}

/// The reason of the provisional widenings the floor of 2026-10-22 set: the engine's spread
/// was measured at the design count on 3 generated leagues, so the floor sits again on more.
pub const PROVISIONAL_REASON: &str = "engine spread at the design count, 3 leagues";

/// A part of a margin added so it holds the 97 percent floor. A provisional one holds the
/// floor until the floor is set again on more leagues, when it may shrink or go; any other
/// widening cites a noticeable error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Widening {
    pub value: f64,
    pub reason: String,
    pub provisional: bool,
}

/// The outer unit the engine side resamples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EngineUnit {
    /// Whole generated leagues.
    League,
    /// Whole matches: a band whose suite plays fixed clubs (the strength pairs, the red-card
    /// arms, a scenario) has no leagues to resample.
    Match,
}

/// How the real side is resampled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RealUnit {
    /// Whole real matches of a file with match rows.
    Match,
    /// Whole tracked matches, one left out at a time.
    TrackedMatch,
    /// A value and its error with no rows.
    Summary,
    /// A target of the reference file of real targets.
    Target,
    /// No real side: a design target with no cap.
    None,
}

/// How each side of a band is resampled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resampling {
    pub engine: EngineUnit,
    pub real: RealUnit,
}

/// The unit of a band's minimum exposure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExposureUnit {
    Matches,
    Events,
}

/// The exposure a band needs before it may pass, with why it is this size.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinExposure {
    pub unit: ExposureUnit,
    pub value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The engine's measured spread of a band, with the run it was measured on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineSpread {
    /// The standard deviation of the band's per-match value.
    pub per_match_sd: f64,
    /// The standard deviation of the band's value between generated leagues, beyond what
    /// the matches inside them explain.
    pub between_league_sd: f64,
    /// A distribution band: the distance two engine runs of the design count sit apart by
    /// chance alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise_floor: Option<f64>,
    /// The run it was measured on, and the date.
    pub measured: String,
}

/// The pass rule's fields of one band.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rule {
    pub target: Target,
    pub margin: Margin,
    pub definition_version: u32,
    pub population: String,
    pub estimator: String,
    pub resampling: Resampling,
    pub min_exposure: MinExposure,
    pub rare: bool,
    /// The engine's measured spread; `None` only for a pending band, which has no engine
    /// count to measure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_spread: Option<EngineSpread>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// A band's target as the file writes it: `{"real": {}}`, `{"design": x}` or
/// `{"design": x, "cap": c}`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetFile {
    #[serde(default)]
    real: Option<EmptyObject>,
    #[serde(default)]
    design: Option<f64>,
    #[serde(default)]
    cap: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyObject {}

/// The layout of the file.
#[derive(Debug, Clone, PartialEq, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
struct RegistryV6 {
    #[garde(skip)]
    schema_version: u32,
    #[garde(range(min = 1, max = 100_000))]
    sample_size: u32,
    #[garde(range(min = 0.1, max = 10_000.0))]
    wall_minutes_per_sample: f64,
    #[garde(dive)]
    stronger_team: StrongerTeam,
    #[garde(range(min = 1, max = 10_000))]
    generalisation_floor: u32,
    #[garde(skip)]
    pass_rule: PassRuleSettings,
    #[garde(skip)]
    kinds: super::kinds::KindTable,
    #[serde(default)]
    #[garde(skip)]
    pair_agreement: Vec<super::kinds::AgreementRecord>,
    #[serde(default)]
    #[garde(skip)]
    report_only: Vec<ReportOnlyGroup>,
    #[garde(skip)]
    bands: Vec<BandV6>,
    #[serde(default)]
    #[garde(skip)]
    figures: Vec<FigureDef>,
}

/// One band of the file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct BandV6 {
    band: String,
    suites: Vec<String>,
    measure: Measure,
    #[serde(default)]
    group: Option<String>,
    target: TargetFile,
    margin: Margin,
    definition_version: u32,
    population: String,
    estimator: String,
    resampling: Resampling,
    min_exposure: MinExposure,
    rare: bool,
    #[serde(default)]
    engine_spread: Option<EngineSpread>,
    #[serde(default)]
    context: Option<String>,
    real: RealRecord,
    evidence: Evidence,
    confidence: Confidence,
    condition: Condition,
    #[serde(default)]
    max_from_dry: Option<f64>,
    gates: bool,
}

impl RegistryV6 {
    fn into_registry(self) -> Registry {
        let groups = self.report_only.clone();
        let bands = self
            .bands
            .into_iter()
            .map(|b| {
                // The raw rules refused a target with both or neither form.
                let target = match (b.target.real, b.target.design) {
                    (_, Some(value)) => Target::Design {
                        value,
                        cap: b.target.cap,
                    },
                    (Some(EmptyObject {}) | None, None) => Target::Real,
                };
                let m = b.margin.value;
                // The range of a band, for the readers that still show one: around its design
                // target, or open for a real target, whose word the pass rule sets.
                let (lo, hi) = match target {
                    Target::Design { value, .. } => (value - m, value + m),
                    Target::Real => (f64::MIN, f64::MAX),
                };
                let report_only = (!b.gates).then(|| {
                    groups
                        .iter()
                        .find(|g| b.group.as_deref() == Some(g.group.as_str()))
                        .map_or_else(
                            || "gates: false in the band file".to_string(),
                            |g| format!("named report-only group {}: {}", g.group, g.reason),
                        )
                });
                BandDef {
                    band: b.band,
                    suites: b.suites,
                    measure: b.measure,
                    lo,
                    hi,
                    smallest_shift: b.margin.noticeable_error,
                    group: b.group,
                    real: Some(b.real),
                    evidence: Some(b.evidence),
                    confidence: Some(b.confidence),
                    condition: b.condition,
                    max_from_dry: b.max_from_dry,
                    gates: b.gates,
                    report_only,
                    rule: Some(Rule {
                        target,
                        margin: b.margin,
                        definition_version: b.definition_version,
                        population: b.population,
                        estimator: b.estimator,
                        resampling: b.resampling,
                        min_exposure: b.min_exposure,
                        rare: b.rare,
                        engine_spread: b.engine_spread,
                        context: b.context,
                    }),
                }
            })
            .collect();
        Registry {
            schema_version: self.schema_version,
            sample_size: self.sample_size,
            wall_minutes_per_sample: self.wall_minutes_per_sample,
            stronger_team: self.stronger_team,
            generalisation_floor: self.generalisation_floor,
            bands,
            figures: self.figures,
            pass_rule: Some(self.pass_rule),
            report_only_groups: self.report_only,
            kinds: Some(self.kinds),
            pair_agreement: self.pair_agreement,
            active_kind: None,
            reference: None,
            targets: BTreeMap::new(),
        }
    }
}

/// The fields every band must hold, with no default.
pub const REQUIRED: [&str; 25] = [
    "target",
    "margin.value",
    "margin.league_spread",
    "margin.noticeable_error",
    "margin.reason",
    "margin.spread_reason",
    "margin.design_choice",
    "definition_version",
    "population",
    "estimator",
    "resampling.engine",
    "resampling.real",
    "min_exposure.unit",
    "min_exposure.value",
    "rare",
    "engine_spread.per_match_sd",
    "engine_spread.between_league_sd",
    "engine_spread.measured",
    "real.source",
    "real.sample",
    "real.date",
    "evidence",
    "confidence.measurement",
    "condition",
    "gates",
];

/// The fields of version 5 that version 6 removed, with what replaced each.
const REMOVED_IN_V6: [(&str, &str); 2] = [
    (
        "design",
        "the fixed range is gone; a band holds a target and a margin",
    ),
    (
        "real.floor",
        "a small real sample widens the interval instead of making the band report only",
    ),
];

/// The record rules of the file on the raw document, every refusal collected so that one
/// load lists every gap: a missing field, a removed field, a blank reason, a target in a
/// shape the file does not take, a real source that does not fit the target, and the record
/// rules of [`record_rules`].
fn required(doc: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for field in ["pass_rule.alpha", "pass_rule.sound_pass", "kinds"] {
        if field
            .split('.')
            .try_fold(doc, |v, k| v.get(k))
            .is_none_or(Value::is_null)
        {
            out.push((field.to_string(), "is required".to_string()));
        }
    }
    let Some(bands) = doc["bands"].as_array() else {
        return out;
    };
    for (i, b) in bands.iter().enumerate() {
        let name = b["band"].as_str().unwrap_or("?");
        let at = |field: &str| format!("bands[{i}] ({name}).{field}");
        let get = |path: &str| path.split('.').try_fold(b, |v, k| v.get(k));
        // A pending band has no engine count, so no engine spread to record.
        let pending = b["measure"]["kind"] == "pending";
        for field in REQUIRED {
            if pending && field.starts_with("engine_spread.") {
                continue;
            }
            if get(field).is_none_or(Value::is_null) {
                out.push((at(field), "is required".into()));
            }
        }
        for (field, why) in REMOVED_IN_V6 {
            if get(field).is_some() {
                out.push((at(field), format!("is not a version 6 field: {why}")));
            }
        }
        for field in [
            "margin.reason",
            "margin.spread_reason",
            "population",
            "estimator",
            "engine_spread.measured",
            "real.date",
        ] {
            if let Some(v) = get(field).filter(|v| !v.is_null()) {
                if v.as_str().is_none_or(|s| s.trim().is_empty()) {
                    out.push((at(field), "must not be blank".into()));
                }
            }
        }
        if let Some(v) = get("margin.widening.reason").filter(|v| !v.is_null()) {
            if v.as_str().is_none_or(|s| s.trim().is_empty()) {
                out.push((at("margin.widening.reason"), "must not be blank".into()));
            }
        }
        if get("margin.design_choice").is_some_and(|v| *v != Value::Bool(true)) {
            out.push((
                at("margin.design_choice"),
                "must be true: a margin is a design choice, stated as one".into(),
            ));
        }
        if let Some(t) = b.get("target").and_then(Value::as_object) {
            let (real, design) = (t.contains_key("real"), t.contains_key("design"));
            if real == design {
                out.push((
                    at("target"),
                    "must hold real ({}) or design (a value, with an optional cap)".into(),
                ));
            }
            if real && t.contains_key("cap") {
                out.push((at("target.cap"), "a cap goes with a design target".into()));
            }
        }
        let none = b["real"]["source"]
            .as_object()
            .is_some_and(|s| s.contains_key("none"));
        if none && b["target"].get("design").is_none() {
            out.push((
                at("target"),
                "a band with no real counterpart needs a design target".into(),
            ));
        }
        if b.get("gates")
            .is_some_and(|g| !g.is_boolean() && !g.is_null())
        {
            out.push((at("gates"), "must be true or false".into()));
        }
        record_rules(b, &at, &mut out);
    }
    out
}

/// One check of the margin floor: the band, the margin it holds and the margin it needs,
/// and the two errors behind the need.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FloorRefusal {
    pub band: String,
    pub margin: f64,
    pub needed: f64,
    pub sigma_e: f64,
    pub sigma_r: f64,
    /// Which margin: `margin`, or `cap` for a design band's distance from the real value.
    pub which: &'static str,
}

impl FloorRefusal {
    /// The refusal as the loader states it.
    pub fn reason(&self) -> String {
        format!(
            "{} {} is below the 97 percent floor; it needs at least {} \
             (engine error {}, real error {})",
            self.which,
            fmt_g(self.margin),
            fmt_g(self.needed),
            fmt_g(self.sigma_e),
            fmt_g(self.sigma_r)
        )
    }

    /// Whether the margin holds its floor.
    pub fn holds(&self) -> bool {
        pass_rule::floor::holds(self.margin, self.needed)
    }
}

/// A band of a named report-only group against the floor it would need if it gated: it is
/// not one of the gating bands the floor covers, so it is shown and never refused. A person
/// moves it back to gating by a band-file edit once `holds` is true.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReportOnlyFloor {
    pub band: String,
    pub group: String,
    pub margin: f64,
    pub needed: f64,
    pub sigma_e: f64,
    pub sigma_r: f64,
    /// The margin would hold its floor now.
    pub holds: bool,
}

/// A provisional widening: the band, the margin before it and the part added.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Provisional {
    pub band: String,
    pub margin: f64,
    pub widening: f64,
    pub reason: String,
}

/// A number to 4 significant figures.
fn fmt_g(x: f64) -> String {
    if x == 0.0 || !x.is_finite() {
        return format!("{x}");
    }
    let places = (3 - x.abs().log10().floor() as i32).max(0) as usize;
    format!("{x:.places$}")
}

impl Registry {
    /// The bands that gate.
    pub fn gating(&self) -> usize {
        self.bands.iter().filter(|b| b.gates).count()
    }

    /// The margins widened provisionally, in file order.
    pub fn provisional(&self) -> Vec<Provisional> {
        self.bands
            .iter()
            .filter_map(|b| {
                let m = &b.rule.as_ref()?.margin;
                let w = m.widening.as_ref().filter(|w| w.provisional)?;
                Some(Provisional {
                    band: b.band.clone(),
                    margin: m.value,
                    widening: w.value,
                    reason: w.reason.clone(),
                })
            })
            .collect()
    }

    /// The floor checks of the band `b` at `gating` gating bands, with `sigma_r` its real
    /// error: its margin, and a design band's cap. A distribution band's margin must exceed
    /// its noise floor; a rare band's margin must hold the exact interval's half-width at its
    /// minimum exposure.
    fn floor_checks(
        &self,
        b: &BandDef,
        sigma_r: f64,
        gating: usize,
        at: FloorAt,
    ) -> Vec<FloorRefusal> {
        let (Some(settings), Some(rule)) = (self.pass_rule, &b.rule) else {
            return Vec::new();
        };
        let FloorAt {
            matches, leagues, ..
        } = at;
        let margin = if rule.rare {
            rule.margin.value * at.rare_factor
        } else {
            rule.margin.value
        };
        // A pending band has no engine spread: no floor to check.
        let Some(spread) = &rule.engine_spread else {
            return Vec::new();
        };
        let one = |margin: f64, needed: f64, sigma_e: f64, sigma_r: f64, which| FloorRefusal {
            band: b.band.clone(),
            margin,
            needed,
            sigma_e,
            sigma_r,
            which,
        };
        if b.is_distribution() {
            let floor = spread.noise_floor.unwrap_or(f64::INFINITY);
            return vec![one(margin, floor, floor, 0.0, "margin")];
        }
        let units = match rule.resampling.engine {
            EngineUnit::League => leagues,
            EngineUnit::Match => matches,
        };
        let sigma_e = pass_rule::floor::engine_error(
            spread.per_match_sd,
            spread.between_league_sd,
            matches,
            units,
        );
        let (to_target_r, cap) = match rule.target {
            Target::Real => (sigma_r, None),
            Target::Design { cap, .. } => (0.0, cap),
        };
        let needed = if rule.rare {
            // Expected events per match from the measured spread (a count's variance equals
            // its mean), and the exposure in matches.
            let rate = spread.per_match_sd.powi(2);
            let exposure = match rule.min_exposure.unit {
                ExposureUnit::Matches => rule.min_exposure.value,
                ExposureUnit::Events => rule.min_exposure.value / rate.max(1e-12),
            };
            pass_rule::floor::needed_rare_margin(rate, exposure, to_target_r, settings.alpha)
        } else {
            pass_rule::floor::needed_margin(
                sigma_e,
                to_target_r,
                settings.alpha,
                settings.sound_pass,
                gating,
            )
        };
        let mut out = vec![one(margin, needed, sigma_e, to_target_r, "margin")];
        if let Some(cap) = cap {
            let needed = pass_rule::floor::needed_margin(
                sigma_e,
                sigma_r,
                settings.alpha,
                settings.sound_pass,
                gating,
            );
            out.push(one(cap, needed, sigma_e, sigma_r, "cap"));
        }
        out
    }

    /// The pass rule's floor over every gating band, with `real_se` giving each band's real
    /// error where it is known (the reference file's, or the value read in place); an
    /// unknown real error counts as 0, so the engine's error alone sets the floor until the
    /// real one is read. A band of a named report-only group is not one of the gating bands
    /// the floor covers: [`Self::report_only_floor`] shows it instead.
    pub fn floor_refusals(&self, real_se: &dyn Fn(&BandDef) -> Option<f64>) -> Vec<FloorRefusal> {
        let k = self.gating();
        let mut out: Vec<FloorRefusal> = Vec::new();
        for b in self.bands.iter().filter(|b| b.gates) {
            for at in self.floor_runs(b) {
                for f in self.floor_checks(b, real_se(b).unwrap_or(0.0), k, at) {
                    if f.holds() {
                        continue;
                    }
                    // One refusal per band and margin: the run kind that needs the most.
                    match out
                        .iter_mut()
                        .find(|o| o.band == f.band && o.which == f.which)
                    {
                        Some(o) if o.needed < f.needed => *o = f,
                        Some(_) => {}
                        None => out.push(f),
                    }
                }
            }
        }
        out
    }

    /// The runs a band's floor is checked at: the active kind's count alone once a run has
    /// picked its kind (whose factor its margins already carry), else every kind's count and
    /// margins.
    fn floor_runs(&self, b: &BandDef) -> Vec<FloorAt> {
        let Some(table) = &self.kinds else {
            return Vec::new();
        };
        let at = |k: super::kinds::Kind, factor: Option<f64>| {
            let def = table.get(k);
            FloorAt {
                matches: f64::from(def.floor_matches(b)),
                leagues: f64::from(def.leagues),
                rare_factor: factor.unwrap_or(def.rare_margin.factor),
            }
        };
        // A kind that judges no rare band checks no rare band's floor.
        let judges = |k: super::kinds::Kind| table.get(k).judges(b);
        match self.active_kind {
            Some(k) if judges(k) => vec![at(k, Some(1.0))],
            Some(_) => Vec::new(),
            None => super::kinds::Kind::ALL
                .into_iter()
                .filter(|&k| judges(k))
                .map(|k| at(k, None))
                .collect(),
        }
    }

    /// Whether the run's kind leaves the band `name` unjudged: a rare band in a run whose
    /// kind judges no rare band (the change kind).
    pub fn not_judged(&self, name: &str) -> bool {
        let (Some(k), Some(table)) = (self.active_kind, &self.kinds) else {
            return false;
        };
        self.band(name).is_some_and(|b| !table.get(k).judges(b))
    }

    /// The registry a run of `kind` judges with: each rare band's margin widened by the
    /// kind's factor, and the floor checked at the kind's count alone.
    pub fn with_kind(&self, kind: super::kinds::Kind) -> Registry {
        let mut out = self.clone();
        let Some(factor) = self.kinds.as_ref().map(|t| t.get(kind).rare_margin.factor) else {
            return out;
        };
        for b in &mut out.bands {
            if let Some(rule) = b.rule.as_mut().filter(|r| r.rare) {
                rule.margin.value *= factor;
                if let Target::Design { value, .. } = rule.target {
                    b.lo = value - rule.margin.value;
                    b.hi = value + rule.margin.value;
                }
            }
        }
        out.active_kind = Some(kind);
        out
    }

    /// Each band of a named report-only group against the floor it would need if it gated
    /// alongside today's gating bands, with `real_se` as in [`Self::floor_refusals`].
    pub fn report_only_floor(
        &self,
        real_se: &dyn Fn(&BandDef) -> Option<f64>,
    ) -> Vec<ReportOnlyFloor> {
        let k = self.gating() + 1;
        self.bands
            .iter()
            .filter(|b| !b.gates && !matches!(b.measure, Measure::Pending { .. }))
            .filter_map(|b| {
                let group = b.group.clone()?;
                let at = *self.floor_runs(b).first()?;
                let checks = self.floor_checks(b, real_se(b).unwrap_or(0.0), k, at);
                let first = checks.first()?;
                Some(ReportOnlyFloor {
                    band: b.band.clone(),
                    group,
                    margin: first.margin,
                    needed: checks.iter().map(|c| c.needed).fold(0.0, f64::max),
                    sigma_e: first.sigma_e,
                    sigma_r: first.sigma_r,
                    holds: checks.iter().all(FloorRefusal::holds),
                })
            })
            .collect()
    }

    /// The rules beyond the layout, every refusal collected.
    fn check_v6(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let Some(settings) = self.pass_rule else {
            return out;
        };
        if !(settings.alpha > 0.0 && settings.alpha < 0.5) {
            out.push((
                "pass_rule.alpha".into(),
                format!("{} is not between 0 and 0.5", settings.alpha),
            ));
        }
        if !(settings.sound_pass > 0.0 && settings.sound_pass < 1.0) {
            out.push((
                "pass_rule.sound_pass".into(),
                format!("{} is not between 0 and 1", settings.sound_pass),
            ));
        }
        if let Some(table) = &self.kinds {
            out.extend(table.check());
        }
        for (i, g) in self.report_only_groups.iter().enumerate() {
            if g.reason.trim().is_empty() {
                out.push((
                    format!("report_only[{i}] ({}).reason", g.group),
                    "needs a reason".into(),
                ));
            }
        }
        let mut per_group: BTreeMap<&str, u32> = BTreeMap::new();
        for (i, b) in self.bands.iter().enumerate() {
            let at = |field: &str| format!("bands[{i}] ({}).{field}", b.band);
            let Some(rule) = &b.rule else { continue };
            let m = &rule.margin;
            let widened = m.widened_by();
            let finite = [m.value, m.league_spread, m.noticeable_error, widened]
                .iter()
                .all(|x| x.is_finite());
            if !finite || m.noticeable_error <= 0.0 || m.league_spread < 0.0 {
                out.push((
                    at("margin"),
                    "the noticeable error must be above 0 and the league spread 0 or above".into(),
                ));
            } else if (m.value - (m.league_spread + m.noticeable_error + widened)).abs()
                > 1e-9 * m.value.abs().max(1.0)
            {
                out.push((
                    at("margin.value"),
                    format!(
                        "{} is not the league spread {} plus the noticeable error {}{}",
                        m.value,
                        m.league_spread,
                        m.noticeable_error,
                        if m.widening.is_some() {
                            format!(" plus the widening {widened}")
                        } else {
                            String::new()
                        }
                    ),
                ));
            }
            if let Some(w) = &m.widening {
                if !(w.value.is_finite() && w.value > 0.0) {
                    out.push((at("margin.widening.value"), "must be above 0".into()));
                }
                if !w.provisional && w.reason.trim() == PROVISIONAL_REASON {
                    out.push((
                        at("margin.widening.reason"),
                        "a widening that is not provisional cites a noticeable error, not the \
                         provisional reason"
                            .into(),
                    ));
                }
            }
            if rule.definition_version == 0 {
                out.push((at("definition_version"), "starts at 1".into()));
            }
            if !(rule.min_exposure.value.is_finite() && rule.min_exposure.value > 0.0) {
                out.push((at("min_exposure.value"), "must be above 0".into()));
            }
            let pending = matches!(b.measure, Measure::Pending { .. });
            match &rule.engine_spread {
                Some(spread) => {
                    if !(spread.per_match_sd >= 0.0 && spread.between_league_sd >= 0.0) {
                        out.push((at("engine_spread"), "the spreads must be 0 or above".into()));
                    }
                    if b.is_distribution() && spread.noise_floor.is_none() {
                        out.push((
                            at("engine_spread.noise_floor"),
                            "a distribution band needs its noise floor".into(),
                        ));
                    }
                }
                None if !pending => out.push((
                    at("engine_spread"),
                    "is required: only a pending band, with no engine count yet, has none".into(),
                )),
                None => {}
            }
            if pending && b.gates {
                out.push((
                    at("gates"),
                    "a pending band has no engine count yet, so it reports only: gates false \
                     in a named report-only group"
                        .into(),
                ));
            }
            if let Target::Design { value, cap } = rule.target {
                if !value.is_finite() {
                    out.push((at("target.design"), "is not a number".into()));
                }
                if cap.is_some_and(|c| !(c.is_finite() && c > 0.0)) {
                    out.push((at("target.cap"), "must be above 0".into()));
                }
            }
            let target_source = matches!(
                b.real.as_ref().map(|r| &r.source),
                Some(RealSource::Target(_))
            );
            // A reference-file target is resampled as a summary, or by its tracked matches
            // when it holds them; any other real source is never a target.
            let fits = match rule.resampling.real {
                RealUnit::Target => target_source,
                RealUnit::TrackedMatch => true,
                _ => !target_source,
            };
            if !fits {
                out.push((
                    at("resampling.real"),
                    "is target or tracked-match when the real source names a reference-file                      target, and neither target otherwise"
                        .into(),
                ));
            }
            if !b.gates {
                match self
                    .report_only_groups
                    .iter()
                    .find(|g| b.group.as_deref() == Some(g.group.as_str()))
                {
                    None => out.push((
                        at("gates"),
                        "every band gates; only a band of a named report-only group may be \
                         false"
                            .into(),
                    )),
                    Some(g) => *per_group.entry(g.group.as_str()).or_default() += 1,
                }
            }
        }
        for g in &self.report_only_groups {
            let n = per_group.get(g.group.as_str()).copied().unwrap_or(0);
            if n > g.most {
                out.push((
                    format!("report_only ({})", g.group),
                    format!("{n} bands report only; the group allows at most {}", g.most),
                ));
            }
        }
        out
    }
}

/// A refusal list as one error: the first field, and every refusal in the reason, so one
/// load names every gap.
fn refusals(list: Vec<(String, String)>) -> Result<(), (String, String)> {
    let mut it = list.into_iter();
    let Some((field, reason)) = it.next() else {
        return Ok(());
    };
    let rest: Vec<String> = it.map(|(f, r)| format!("{f}: {r}")).collect();
    if rest.is_empty() {
        Err((field, reason))
    } else {
        Err((
            field,
            format!("{reason}; and {} more: {}", rest.len(), rest.join("; ")),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn shipped() -> ContentDir {
        ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
    }

    fn shipped_text() -> String {
        std::fs::read_to_string(shipped().path(BANDS_FILE)).unwrap()
    }

    fn refusal(text: &str) -> String {
        Registry::from_bytes(text.as_bytes())
            .unwrap_err()
            .to_string()
    }

    /// The shipped file with the band `name` changed by `edit`.
    fn edited(name: &str, edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut doc: serde_json::Value = serde_json::from_str(&shipped_text()).unwrap();
        let band = doc["bands"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|b| b["band"] == name)
            .unwrap();
        edit(band);
        doc.to_string()
    }

    type Edit = Box<dyn FnOnce(&mut serde_json::Value)>;

    /// The team-shape bands: per position depth, width, pace and sprints, and per team the
    /// slide, the far-player gain, dead-ball calm, stoppages and ball in play.
    const SHAPE_BANDS: usize = 101;

    /// `true` for a team-shape band: its group names the shape.
    fn is_shape(b: &BandDef) -> bool {
        b.group.as_deref().is_some_and(|g| g.starts_with("shape_"))
    }

    /// The four rare bands of the named report-only group: their real figure is too
    /// uncertain for any match count to hold the floor.
    const RARE_REAL_ERROR: [&str; 4] = [
        "five_one_share",
        "goals_free_kick_share",
        "goals_penalty_share",
        "goals_throw_in_share",
    ];

    /// The bands the slow tests judge as scenarios, and the two whole distributions of
    /// shots and cards from the reference file.
    const ADDED: [&str; 7] = [
        "second_yellows_per_match",
        "penalty_conversion_share",
        "formation_goals_per_side",
        "ai_trailing_change_share",
        "attacking_minus_defensive_shots",
        "shots_total_distribution",
        "cards_total_distribution",
    ];

    /// The shipped file with a pending band `pending_probe` added in a group of its own
    /// (`pending_test`, at most `most` bands), cloned from goals_per_match and changed by
    /// `edit`.
    fn with_pending(most: u32, edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut doc: serde_json::Value = serde_json::from_str(&shipped_text()).unwrap();
        let mut band = doc["bands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["band"] == "goals_per_match")
            .unwrap()
            .clone();
        band["band"] = "pending_probe".into();
        band["measure"] = serde_json::json!({"kind": "pending", "reason": "no count yet"});
        band["group"] = "pending_test".into();
        band["gates"] = false.into();
        band.as_object_mut().unwrap().remove("engine_spread");
        edit(&mut band);
        doc["bands"].as_array_mut().unwrap().push(band);
        doc["report_only"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "group": "pending_test",
                "most": most,
                "reason": "a test group of bands with no engine count yet"
            }));
        doc.to_string()
    }

    #[test]
    fn a_pending_band_in_a_listed_group_loads_with_no_spread_and_no_floor() {
        let r = Registry::from_bytes(with_pending(1, |_| {}).as_bytes()).unwrap();
        let b = r.band("pending_probe").unwrap();
        assert!(matches!(b.measure, Measure::Pending { .. }));
        assert!(b.rule.as_ref().unwrap().engine_spread.is_none());
        assert!(
            b.report_only
                .as_deref()
                .is_some_and(|w| w.starts_with("named report-only group pending_test"))
        );
        assert!(
            r.report_only_floor(&|_| None)
                .iter()
                .all(|f| f.band != "pending_probe")
        );
        assert!(
            r.floor_refusals(&|_| None)
                .iter()
                .all(|f| f.band != "pending_probe")
        );
    }

    #[test]
    fn a_pending_band_that_gates_is_refused() {
        let e = refusal(&with_pending(1, |b| b["gates"] = true.into()));
        assert!(
            e.contains("pending_probe") && e.contains("reports only"),
            "{e}"
        );
        let blank = refusal(&with_pending(1, |b| {
            b["measure"] = serde_json::json!({"kind": "pending", "reason": " "});
        }));
        assert!(blank.contains("pending band says why"), "{blank}");
    }

    #[test]
    fn a_band_that_is_not_pending_needs_its_engine_spread() {
        let text = edited("goals_per_match", |b| {
            b.as_object_mut().unwrap().remove("engine_spread");
        });
        let e = refusal(&text);
        assert!(
            e.contains("goals_per_match") && e.contains("engine_spread"),
            "{e}"
        );
    }

    #[test]
    fn a_group_of_at_most_one_refuses_a_second_band() {
        let mut doc: serde_json::Value = serde_json::from_str(&with_pending(1, |_| {})).unwrap();
        let mut second = doc["bands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["band"] == "pending_probe")
            .unwrap()
            .clone();
        second["band"] = "pending_probe_two".into();
        doc["bands"].as_array_mut().unwrap().push(second);
        let e = refusal(&doc.to_string());
        assert!(e.contains("pending_test") && e.contains("at most 1"), "{e}");
    }

    #[test]
    fn the_shipped_registry_is_version_6_and_every_band_gates_but_the_named_groups() {
        let r = Registry::load(&shipped()).unwrap();
        assert_eq!(r.schema_version, REGISTRY_VERSION);
        assert_eq!(r.bands.len(), 154 + ADDED.len());
        assert_eq!(r.bands.iter().filter(|b| is_shape(b)).count(), SHAPE_BANDS);
        assert!(r.bands.iter().all(|b| b.rule.is_some()));
        for name in ADDED {
            assert!(r.band(name).is_some(), "{name}");
        }
        // The named list: the facing pair (none on this branch) and the four rare bands.
        let groups: Vec<(&str, u32)> = r
            .report_only_groups
            .iter()
            .map(|g| (g.group.as_str(), g.most))
            .collect();
        assert_eq!(groups, [("facing", 2), ("rare_real_error", 4)]);
        let mut off: Vec<&str> = r
            .bands
            .iter()
            .filter(|b| !b.gates)
            .map(|b| b.band.as_str())
            .collect();
        off.sort_unstable();
        assert_eq!(off, RARE_REAL_ERROR);
        for name in RARE_REAL_ERROR {
            let b = r.band(name).unwrap();
            assert_eq!(b.group.as_deref(), Some("rare_real_error"), "{name}");
            assert!(
                b.report_only
                    .as_deref()
                    .is_some_and(|w| w.starts_with("named report-only group rare_real_error")),
                "{name}"
            );
        }
        assert_eq!(r.gating(), r.bands.len() - RARE_REAL_ERROR.len());
        assert_eq!(
            r.band("possession_home_pct").unwrap().group.as_deref(),
            Some("possession_pct")
        );
        assert_eq!(r.figures.len(), 3);
        assert_eq!(r.sample_size, 1000);
        assert_eq!(r.stronger_team.attribute_boost, 1.15);
        assert_eq!(r.wall_budget_ms(1000), 1_800_000);
    }

    /// The built-in measures carry their old pass region: a design target at its centre and
    /// a noticeable error of its half-width. `full_over_control` keeps the centre of its old
    /// region 0 to 1.6, and its margin is the floor's needed value, 0.48, marked provisional.
    #[test]
    fn the_built_in_bands_keep_their_old_pass_region() {
        let r = Registry::load(&shipped()).unwrap();
        for (name, lo, hi) in [
            ("stronger_team_win_rate", 0.5, 1.0),
            ("reduced_minus_full", -10.0, 0.0),
            ("full_over_control", 0.32, 1.28),
        ] {
            let b = r.band(name).unwrap();
            assert!(
                (b.lo - lo).abs() < 1e-9 && (b.hi - hi).abs() < 1e-9,
                "{name}"
            );
            assert_eq!(r.range(name), Some(b.range()), "{name}");
            assert!(matches!(
                b.rule.as_ref().unwrap().target,
                Target::Design { cap: None, .. }
            ));
        }
        // A band judged against the real value has no fixed range.
        assert_eq!(r.range("goals_per_match"), None);
    }

    #[test]
    fn every_shipped_band_carries_its_full_record() {
        let r = Registry::load(&shipped()).unwrap();
        assert_eq!(r.generalisation_floor, 2);
        let experiments = [
            "stronger_team_win_rate",
            "reduced_minus_full",
            "full_over_control",
        ];
        for b in &r.bands {
            let real = b.real.as_ref().unwrap_or_else(|| panic!("{}", b.band));
            assert!(!real.date.trim().is_empty(), "{}", b.band);
            assert!(b.evidence.is_some(), "{}", b.band);
            let c = b.confidence.unwrap_or_else(|| panic!("{}", b.band));
            assert_eq!(b.condition, Condition::Dry, "{}", b.band);
            let rule = b.rule.as_ref().unwrap();
            assert!(rule.margin.design_choice, "{}", b.band);
            assert!(!rule.margin.reason.trim().is_empty(), "{}", b.band);
            assert!(!rule.population.trim().is_empty(), "{}", b.band);
            assert!(!rule.estimator.trim().is_empty(), "{}", b.band);
            match &rule.engine_spread {
                Some(s) => assert!(!s.measured.trim().is_empty(), "{}", b.band),
                None => assert!(matches!(b.measure, Measure::Pending { .. }), "{}", b.band),
            }
            match &real.source {
                RealSource::NoCounterpart(n) => {
                    let scenario = matches!(b.measure, Measure::Scenario { .. });
                    assert!(
                        scenario || experiments.contains(&b.band.as_str()),
                        "{}",
                        b.band
                    );
                    assert!(n.none.contains("no real match repeats it"), "{}", b.band);
                    assert_eq!(real.sample, 0, "{}", b.band);
                    assert_eq!(c.measurement.count, 0, "{}", b.band);
                    assert!(matches!(rule.target, Target::Design { .. }), "{}", b.band);
                }
                RealSource::Target(t) if t.target.starts_with("trk.dfl.") => {
                    // A tracked band reads the tracked matches' own target of its name.
                    assert_eq!(t.target, format!("trk.dfl.{}", b.band));
                    assert_eq!(rule.resampling.real, RealUnit::TrackedMatch, "{}", b.band);
                    assert_eq!(real.sample, 7, "{}", b.band);
                }
                RealSource::Target(t) => {
                    assert!(t.target.starts_with("tier.1.dist."), "{}", b.band);
                    assert_eq!(rule.resampling.real, RealUnit::Target, "{}", b.band);
                }
                RealSource::Pointer(p) => {
                    assert!(!experiments.contains(&b.band.as_str()), "{}", b.band);
                    for text in [&p.dataset, &p.file, &p.key] {
                        assert!(!text.trim().is_empty(), "{}", b.band);
                    }
                    assert!(real.sample > 0, "{}", b.band);
                    assert!(c.measurement.count > 0, "{}", b.band);
                    assert_eq!(rule.target, Target::Real, "{}", b.band);
                }
            }
        }
    }

    /// The reason of the provisional widenings the tracked bands took on 2026-10-25: their
    /// real error is the jackknife error over 7 tracked matches, so a margin may narrow back
    /// when more tracked matches exist.
    const TRACKED_REASON: &str = "real error from 7 tracked matches";

    /// The floor answers of 2026-10-22 and 2026-10-25: every margin the floor refused is
    /// widened by a provisional part with the stated reason, and the noticeable error keeps
    /// its design value. The 100 tracked bands the second floor refused cite their 7-match
    /// real error (10 of them were among the 36 widened on 2026-10-22); a tracked band that
    /// holds its floor keeps its earlier widening and reason.
    #[test]
    fn the_widened_margins_are_provisional_and_hold_their_parts() {
        let r = Registry::load(&shipped()).unwrap();
        let provisional = r.provisional();
        assert_eq!(provisional.len(), 126);
        let mut tracked = 0;
        for p in &provisional {
            let band = r.band(&p.band).unwrap();
            let is_tracked = band
                .real
                .as_ref()
                .and_then(|real| real.target_key())
                .is_some_and(|k| k.starts_with("trk.dfl."));
            // The one engine experiment among them has no real source to lower its floor.
            let reason = if p.band == "full_over_control" {
                "engine spread at the design count; engine experiment, no real source"
            } else if is_tracked && p.reason == TRACKED_REASON {
                tracked += 1;
                TRACKED_REASON
            } else {
                PROVISIONAL_REASON
            };
            assert_eq!(p.reason, reason, "{}", p.band);
            let m = &r.band(&p.band).unwrap().rule.as_ref().unwrap().margin;
            assert!(p.widening > 0.0, "{}", p.band);
            let parts = m.league_spread + m.noticeable_error + p.widening;
            assert!((m.value - parts).abs() < 1e-9, "{}", p.band);
        }
        assert_eq!(tracked, 100);
        for name in RARE_REAL_ERROR {
            assert!(provisional.iter().any(|p| p.band == name), "{name}");
        }
        // The report-only bands are shown against their floor, and none holds it today.
        let shown = r.report_only_floor(&|_| None);
        assert_eq!(shown.len(), RARE_REAL_ERROR.len());
        assert!(shown.iter().all(|s| s.group == "rare_real_error"));
    }

    #[test]
    fn an_earlier_version_is_refused_naming_what_version_6_adds() {
        for v in [2, 3, 4, 5] {
            let text = shipped_text().replacen(
                "\"schema_version\": 6",
                &format!("\"schema_version\": {v}"),
                1,
            );
            let err = refusal(&text);
            assert!(err.contains(BANDS_FILE), "{err}");
            assert!(
                err.contains(&format!("schema_version {v} is an earlier layout")),
                "{err}"
            );
            assert!(err.contains("a margin with its reasons"), "{err}");
        }
        let err = refusal("{\"schema_version\": 9}");
        assert!(err.contains(BANDS_FILE) && err.contains('9'), "{err}");
    }

    #[test]
    fn each_record_rule_is_refused_by_name() {
        let cases: [(&str, &str, Edit, &str); 9] = [
            (
                "corners_per_team",
                "a blank reason",
                Box::new(|b| b["real"]["source"] = serde_json::json!({"none": " "})),
                "(corners_per_team).real.source.none: needs a reason",
            ),
            (
                "corners_per_team",
                "a pointer with sample 0",
                Box::new(|b| b["real"]["sample"] = 0.into()),
                "(corners_per_team).real.sample: is 0",
            ),
            (
                "corners_per_team",
                "both none and a pointer",
                Box::new(|b| b["real"]["source"]["none"] = "why".into()),
                "(corners_per_team).real.source: holds more than one",
            ),
            (
                "corners_per_team",
                "a pointer without its key",
                Box::new(|b| {
                    b["real"]["source"].as_object_mut().unwrap().remove("key");
                }),
                "(corners_per_team).real.source: needs dataset, file and key",
            ),
            (
                "full_over_control",
                "no counterpart with a sample",
                Box::new(|b| b["real"]["sample"] = 5.into()),
                "(full_over_control).real.sample: is above 0",
            ),
            (
                "full_over_control",
                "a real floor",
                Box::new(|b| b["real"]["floor"] = 5.into()),
                "(full_over_control).real.floor: is not a version 6 field",
            ),
            (
                "corners_per_team",
                "gates as text",
                Box::new(|b| b["gates"] = "yes".into()),
                "(corners_per_team).gates: must be true or false",
            ),
            (
                "corners_per_team",
                "an unknown evidence word",
                Box::new(|b| b["evidence"] = "hunch".into()),
                "(corners_per_team).evidence",
            ),
            (
                "corners_per_team",
                "a cap on a dry band",
                Box::new(|b| b["max_from_dry"] = 0.5.into()),
                "(corners_per_team).max_from_dry",
            ),
        ];
        for (band, what, edit, named) in cases {
            let err = refusal(&edited(band, edit));
            assert!(err.contains(named), "{what}: {err}");
        }
        // An unknown pointer field is refused by name.
        let err = refusal(&edited("fouls_per_team", |b| {
            b["real"]["source"]["axes"] = serde_json::json!([1.0, 1.0]);
        }));
        assert!(err.contains("(fouls_per_team).real.source.axes"), "{err}");
    }

    #[test]
    fn the_measure_kinds_parse_and_a_malformed_one_is_refused_by_name() {
        let r = Registry::load(&shipped()).unwrap();
        let kind = |name: &str| r.band(name).unwrap().measure.clone();
        assert!(matches!(
            kind("home_goal_difference"),
            Measure::Difference { .. }
        ));
        assert_eq!(
            kind("five_one_share"),
            Measure::Scoreline {
                scores: vec![[5, 1]]
            }
        );
        assert!(matches!(
            kind("goals_poisson_ratio"),
            Measure::FactorialRatio { .. }
        ));
        assert!(matches!(
            kind("worst_to_best_spread"),
            Measure::ClubRange { window: 34, .. }
        ));
        assert_eq!(
            kind("goals_distribution"),
            Measure::Distribution {
                of: Source::Goals,
                width: 1.0,
                bins: 11
            }
        );
        assert_eq!(
            kind("shots_total_distribution"),
            Measure::Distribution {
                of: Source::Shots,
                width: 1.0,
                bins: 51
            }
        );
        assert_eq!(
            kind("cards_total_distribution"),
            Measure::Distribution {
                of: Source::Cards,
                width: 1.0,
                bins: 16
            }
        );
        assert_eq!(kind("scoreline_cells"), Measure::Cells { max: 5 });
        assert!(matches!(
            kind("penalty_conversion_share"),
            Measure::Scenario { .. }
        ));
        // Which bands read the measures, and which read a match's extras at all.
        assert!(!r.band("worst_to_best_spread").unwrap().needs_measures());
        assert!(r.band("worst_to_best_spread").unwrap().needs_extras());
        assert!(r.band("shot_distance_mean").unwrap().needs_measures());
        assert!(!r.band("fouls_per_team").unwrap().needs_extras());
        let needs: Vec<&str> = r
            .bands
            .iter()
            .filter(|b| b.is_distribution() && b.needs_measures())
            .map(|b| b.band.as_str())
            .collect();
        assert_eq!(
            needs,
            [
                "shot_distance_distribution",
                "chain_passes_distribution",
                "chain_seconds_distribution"
            ]
        );
        let cases: [(&str, Value, &str); 9] = [
            (
                "five_one_share",
                serde_json::json!({"kind": "scoreline", "scores": [[21, 1]]}),
                "score 21-1",
            ),
            (
                "five_one_share",
                serde_json::json!({"kind": "scoreline", "scores": []}),
                "scores names no score",
            ),
            (
                "worst_to_best_spread",
                serde_json::json!({"kind": "club_range", "of": ["goals.{side}"], "minus": ["goals.{side}"], "window": 1}),
                "window 1",
            ),
            (
                "goals_poisson_ratio",
                serde_json::json!({"kind": "factorial_ratio", "of": ["goals.home"]}),
                "{side}",
            ),
            (
                "home_goal_difference",
                serde_json::json!({"kind": "difference", "of": ["goals.home"], "minus": ["dribbles.away"]}),
                "dribbles.away",
            ),
            (
                "goals_distribution",
                serde_json::json!({"kind": "distribution", "of": "goals", "width": 0.0, "bins": 11}),
                "width 0",
            ),
            (
                "shot_distance_distribution",
                serde_json::json!({"kind": "distribution", "of": "shot_distance", "width": 2.0, "bins": 41}),
                "the rows hold shot_distance in 41 bins of 1",
            ),
            (
                "scoreline_cells",
                serde_json::json!({"kind": "cells", "max": 0}),
                "max 0",
            ),
            (
                "penalty_conversion_share",
                serde_json::json!({"kind": "scenario", "test": " "}),
                "names the slow test",
            ),
        ];
        for (band, measure, named) in cases {
            let err = refusal(&edited(band, |b| b["measure"] = measure));
            assert!(err.contains(&format!("({band}).measure")), "{err}");
            assert!(err.contains(named), "{named}: {err}");
        }
        // A scenario names no suite, and every other band names one.
        let err = refusal(&edited("penalty_conversion_share", |b| {
            b["suites"] = serde_json::json!(["equal"]);
        }));
        assert!(
            err.contains("(penalty_conversion_share).suites: a scenario band"),
            "{err}"
        );
        let err = refusal(&edited("goals_per_match", |b| {
            b["suites"] = serde_json::json!([]);
        }));
        assert!(
            err.contains("(goals_per_match).suites: names no suite"),
            "{err}"
        );
    }

    #[test]
    fn each_refusal_names_its_band_and_field() {
        let cases: [(&str, Edit, &str); 5] = [
            (
                "unknown catalog field",
                Box::new(|b| b["measure"]["of"][0] = "goals.middle".into()),
                "goals.middle",
            ),
            (
                "empty field list",
                Box::new(|b| b["measure"]["of"] = serde_json::json!([])),
                "goals_per_match).measure",
            ),
            (
                "unknown suite",
                Box::new(|b| b["suites"] = serde_json::json!(["cup"])),
                "cup",
            ),
            (
                "unknown key",
                Box::new(|b| b["colour"] = "red".into()),
                "colour",
            ),
            (
                "a side field in a per-match mean",
                Box::new(|b| b["measure"]["of"] = serde_json::json!(["goals.{side}"])),
                "{side}",
            ),
        ];
        for (what, edit, named) in cases {
            let err = refusal(&edited("goals_per_match", edit));
            assert!(err.contains(BANDS_FILE), "{what}: {err}");
            assert!(err.contains(named), "{what}: {err}");
        }
        let mut doc: serde_json::Value = serde_json::from_str(&shipped_text()).unwrap();
        let first = doc["bands"][0].clone();
        doc["bands"].as_array_mut().unwrap().push(first);
        let err = refusal(&doc.to_string());
        assert!(err.contains("already"), "{err}");
    }

    #[test]
    fn the_digest_follows_a_margin() {
        let base = Registry::from_bytes(shipped_text().as_bytes()).unwrap();
        let wider = Registry::from_bytes(
            edited("corners_per_team", |b| {
                let m = b["margin"]["value"].as_f64().unwrap();
                let e = b["margin"]["noticeable_error"].as_f64().unwrap();
                b["margin"]["value"] = (m + 0.5).into();
                b["margin"]["noticeable_error"] = (e + 0.5).into();
            })
            .as_bytes(),
        )
        .unwrap();
        assert_ne!(base.digest(), wider.digest());
        assert_ne!(
            base.band("corners_per_team").unwrap().digest(),
            wider.band("corners_per_team").unwrap().digest()
        );
        assert_eq!(
            base.band("goals_per_match").unwrap().digest(),
            wider.band("goals_per_match").unwrap().digest()
        );
    }

    // ---- Version 6 -------------------------------------------------------------------

    /// A version 6 band with every field, judged against `target`.
    fn v6_band(name: &str, margin: f64, target: Value, source: Value) -> Value {
        serde_json::json!({
            "band": name,
            "suites": ["equal"],
            "measure": {"kind": "mean", "per": "match", "of": ["goals.home", "goals.away"]},
            "target": target,
            "margin": {
                "value": margin,
                "league_spread": 0.0,
                "noticeable_error": margin,
                "reason": "a player notices this many goals a match",
                "spread_reason": "one population",
                "design_choice": true
            },
            "definition_version": 1,
            "population": "generated leagues of the equal suite",
            "estimator": "mean per match",
            "resampling": {"engine": "league", "real": "match"},
            "min_exposure": {"unit": "matches", "value": 100.0},
            "rare": false,
            "engine_spread": {"per_match_sd": 1.7, "between_league_sd": 0.1, "measured": "run 9, 2026-10-21"},
            "real": {"source": source, "sample": 1826, "date": "2026-10-21"},
            "evidence": "calibration",
            "confidence": {"measurement": {"count": 1826}, "generalisation": {"competitions": 5, "seasons": 1, "clubs": 98}},
            "condition": "dry",
            "gates": true
        })
    }

    fn pointer() -> Value {
        serde_json::json!({"dataset": "a fixture", "file": "wy2_matches.csv", "key": "hg+ag", "reduce": "mean"})
    }

    /// A version 6 file: a real band, a design band with no counterpart, a design band with
    /// a cap, and a band of the named report-only group.
    fn v6_doc() -> Value {
        let mut design = v6_band(
            "stronger_team_win_rate",
            0.2,
            serde_json::json!({"design": 0.75}),
            serde_json::json!({"none": "engine experiment; no real match repeats it"}),
        );
        design["suites"] = serde_json::json!(["strength"]);
        design["measure"] =
            serde_json::json!({"kind": "builtin", "name": "stronger_team_win_rate"});
        design["resampling"] = serde_json::json!({"engine": "match", "real": "none"});
        design["engine_spread"]["per_match_sd"] = 0.5.into();
        design["real"]["sample"] = 0.into();
        design["confidence"] = serde_json::json!({"measurement": {"count": 0}, "generalisation": {"competitions": 0, "seasons": 0, "clubs": 0}});
        let capped = v6_band(
            "capped_goals",
            0.5,
            serde_json::json!({"design": 2.6, "cap": 0.6}),
            pointer(),
        );
        let mut facing = v6_band(
            "facing_ball_share",
            0.3,
            serde_json::json!({"real": {}}),
            pointer(),
        );
        facing["group"] = "facing".into();
        facing["gates"] = false.into();
        serde_json::json!({
            "schema_version": 6,
            "sample_size": 1000,
            "wall_minutes_per_sample": 30.0,
            "stronger_team": {"attribute_boost": 1.15},
            "generalisation_floor": 2,
            "pass_rule": {"alpha": 0.05, "sound_pass": 0.97},
            "kinds": super::super::kinds::design_kinds(),
            "report_only": [{"group": "facing", "most": 2, "reason": "body facing is measured on two pose matches only"}],
            "bands": [
                v6_band("goals_per_match", 0.4, serde_json::json!({"real": {}}), pointer()),
                design,
                capped,
                facing
            ]
        })
    }

    fn v6_edited(edit: impl FnOnce(&mut Value)) -> String {
        let mut doc = v6_doc();
        edit(&mut doc);
        doc.to_string()
    }

    #[test]
    fn a_version_6_file_loads_with_its_rule_and_its_named_report_only_group() {
        let r = Registry::from_bytes(v6_doc().to_string().as_bytes()).unwrap();
        assert_eq!(r.schema_version, 6);
        assert_eq!(r.pass_rule.unwrap().alpha, 0.05);
        assert_eq!(r.gating(), 3);
        let goals = r.band("goals_per_match").unwrap();
        let rule = goals.rule.as_ref().unwrap();
        assert_eq!(rule.target, Target::Real);
        assert_eq!(rule.margin.value, 0.4);
        assert_eq!(goals.smallest_shift, 0.4);
        assert_eq!(goals.report_only, None);
        let design = r.band("stronger_team_win_rate").unwrap();
        assert_eq!(
            design.rule.as_ref().unwrap().target,
            Target::Design {
                value: 0.75,
                cap: None
            }
        );
        assert_eq!((design.lo, design.hi), (0.55, 0.95));
        let facing = r.band("facing_ball_share").unwrap();
        assert!(!facing.gates);
        assert!(
            facing
                .report_only
                .as_deref()
                .unwrap()
                .contains("named report-only group facing"),
            "{:?}",
            facing.report_only
        );
        // The rule joins the band's digest: a changed margin moves it.
        let wider = Registry::from_bytes(
            v6_edited(|d| {
                d["bands"][0]["margin"]["value"] = 0.5.into();
                d["bands"][0]["margin"]["noticeable_error"] = 0.5.into();
            })
            .as_bytes(),
        )
        .unwrap();
        assert_ne!(
            wider.band("goals_per_match").unwrap().digest(),
            goals.digest()
        );
    }

    #[test]
    fn a_version_6_band_without_a_required_field_is_refused_naming_band_and_field() {
        for field in REQUIRED {
            let text = v6_edited(|d| {
                let band = &mut d["bands"][0];
                let mut path: Vec<&str> = field.split('.').collect();
                let last = path.pop().unwrap();
                let parent = path.iter().fold(band, |v, k| &mut v[*k]);
                parent.as_object_mut().unwrap().remove(last);
            });
            let err = refusal(&text);
            assert!(
                err.contains(&format!("(goals_per_match).{field}: is required")),
                "{field}: {err}"
            );
        }
        // Every gap of one load is named at once.
        let err = refusal(&v6_edited(|d| {
            d["bands"][0].as_object_mut().unwrap().remove("population");
            d["bands"][2].as_object_mut().unwrap().remove("estimator");
        }));
        assert!(
            err.contains("(goals_per_match).population: is required"),
            "{err}"
        );
        assert!(
            err.contains("(capped_goals).estimator: is required"),
            "{err}"
        );
    }

    #[test]
    fn each_version_6_rule_is_refused_by_name() {
        let cases: [(&str, Edit, &str); 9] = [
            (
                "a margin not marked a design choice",
                Box::new(|d| d["bands"][0]["margin"]["design_choice"] = false.into()),
                "(goals_per_match).margin.design_choice: must be true",
            ),
            (
                "a blank reason",
                Box::new(|d| d["bands"][0]["margin"]["reason"] = " ".into()),
                "(goals_per_match).margin.reason: must not be blank",
            ),
            (
                "a margin that is not its parts",
                Box::new(|d| d["bands"][0]["margin"]["value"] = 0.9.into()),
                "(goals_per_match).margin.value: 0.9 is not the league spread",
            ),
            (
                "the old range",
                Box::new(|d| d["bands"][0]["design"] = serde_json::json!({"lo": 1, "hi": 2})),
                "(goals_per_match).design: is not a version 6 field",
            ),
            (
                "a real floor",
                Box::new(|d| d["bands"][0]["real"]["floor"] = 30.into()),
                "(goals_per_match).real.floor: is not a version 6 field",
            ),
            (
                "a band that does not gate outside the named group",
                Box::new(|d| d["bands"][0]["gates"] = false.into()),
                "(goals_per_match).gates: every band gates",
            ),
            (
                "a named group over its cap",
                Box::new(|d| d["report_only"][0]["most"] = 0.into()),
                "report_only (facing): 1 bands report only; the group allows at most 0",
            ),
            (
                "no counterpart and a real target",
                Box::new(|d| d["bands"][1]["target"] = serde_json::json!({"real": {}})),
                "(stronger_team_win_rate).target: a band with no real counterpart",
            ),
            (
                "both target forms",
                Box::new(|d| {
                    d["bands"][0]["target"] = serde_json::json!({"real": {}, "design": 1.0})
                }),
                "(goals_per_match).target: must hold real",
            ),
        ];
        for (what, edit, named) in cases {
            let err = refusal(&v6_edited(edit));
            assert!(err.contains(named), "{what}: {err}");
        }
    }

    #[test]
    fn a_kind_count_under_a_band_floor_is_refused_naming_the_band() {
        // At 100 matches per suite the engine error of goals_per_match (1.7 per match) needs
        // more than its margin of 0.4, in the change kind.
        let err = refusal(&v6_edited(|d| {
            d["kinds"]["change"]["count"]["equal"] = 100.into();
        }));
        assert!(
            err.contains(
                "(goals_per_match).margin.value: margin 0.4000 is below the 97 percent floor"
            ),
            "{err}"
        );
        let err = refusal(&v6_edited(|d| {
            d["kinds"]["milestone"]["seeds"]["from"] = 5.into();
        }));
        assert!(
            err.contains("kinds.milestone.seeds: overlaps the change seeds"),
            "{err}"
        );
    }

    #[test]
    fn a_rare_band_judges_on_the_kind_factor_times_its_margin() {
        let mut doc = v6_doc();
        doc["bands"][0]["rare"] = true.into();
        doc["bands"][0]["engine_spread"]["per_match_sd"] = 0.2.into();
        doc["bands"][0]["min_exposure"]["value"] = 1000.0.into();
        let r = Registry::from_bytes(doc.to_string().as_bytes()).unwrap();
        let own = r
            .band("goals_per_match")
            .unwrap()
            .rule
            .as_ref()
            .unwrap()
            .margin
            .value;
        let margin = |k| {
            r.with_kind(k)
                .band("goals_per_match")
                .unwrap()
                .rule
                .as_ref()
                .unwrap()
                .margin
                .value
        };
        assert_eq!(margin(super::super::kinds::Kind::Change), 2.0 * own);
        assert_eq!(margin(super::super::kinds::Kind::Milestone), own);
        assert_eq!(margin(super::super::kinds::Kind::Release), own);
        // The other bands keep their own margins in every kind.
        let design = |k| {
            r.with_kind(k)
                .band("stronger_team_win_rate")
                .unwrap()
                .rule
                .as_ref()
                .unwrap()
                .margin
                .value
        };
        assert_eq!(
            design(super::super::kinds::Kind::Change),
            design(super::super::kinds::Kind::Release)
        );
    }

    #[test]
    fn a_change_kind_that_judges_no_rare_band_leaves_it_out_of_its_count_and_floor() {
        use super::super::kinds::{Kind, compute, units};
        let mut doc = v6_doc();
        doc["bands"][0]["rare"] = true.into();
        doc["bands"][0]["engine_spread"]["per_match_sd"] = 0.2.into();
        // An exposure far beyond every count: a kind that judges the band needs it.
        doc["bands"][0]["min_exposure"]["value"] = 50_000.0.into();
        for k in ["change", "milestone", "release"] {
            doc["kinds"][k]["count"]["equal"] = 50_000.into();
        }
        doc["kinds"]["change"]["judges_rare"] = false.into();
        let r = Registry::from_bytes(doc.to_string().as_bytes()).unwrap();
        assert!(r.with_kind(Kind::Change).not_judged("goals_per_match"));
        assert!(!r.with_kind(Kind::Milestone).not_judged("goals_per_match"));
        assert!(
            !r.with_kind(Kind::Change)
                .not_judged("stronger_team_win_rate")
        );
        assert!(!r.not_judged("goals_per_match"), "no kind picked yet");
        let counts = compute(&r, &units(1), &|_| None).unwrap();
        let equal = |k: Kind| counts[&k]["equal"].real.clone();
        assert_eq!(equal(Kind::Milestone).matches, 50_000);
        assert_eq!(equal(Kind::Milestone).band, "goals_per_match");
        assert!(
            equal(Kind::Change).matches < 50_000,
            "{:?}",
            equal(Kind::Change)
        );
        assert_ne!(equal(Kind::Change).band, "goals_per_match");
    }

    #[test]
    fn a_margin_under_the_floor_is_refused_naming_the_margin_it_needs_and_one_at_it_loads() {
        let doc = v6_doc();
        let r = Registry::from_bytes(doc.to_string().as_bytes()).unwrap();
        // The floor of goals_per_match from its engine spread: 1.7 per match over 1000
        // matches, 0.1 between leagues over 30, with 3 gating bands.
        let sigma_e = pass_rule::floor::engine_error(1.7, 0.1, 1000.0, 30.0);
        let needed = pass_rule::floor::needed_margin(sigma_e, 0.0, 0.05, 0.97, r.gating());
        let at = |m: f64| {
            v6_edited(|d| {
                d["bands"][0]["margin"]["value"] = m.into();
                d["bands"][0]["margin"]["noticeable_error"] = m.into();
            })
        };
        let err = refusal(&at(needed * 0.99));
        assert!(err.contains("(goals_per_match).margin.value"), "{err}");
        assert!(
            err.contains("is below the 97 percent floor; it needs at least"),
            "{err}"
        );
        assert!(err.contains(&fmt_g(needed)), "{err}");
        assert!(Registry::from_bytes(at(needed).as_bytes()).is_ok());
        // A design band's cap is held to the floor as well.
        let err = refusal(&v6_edited(|d| d["bands"][2]["target"]["cap"] = 0.01.into()));
        assert!(
            err.contains("(capped_goals).target.cap: cap 0.01000 is below"),
            "{err}"
        );
        // A distribution band needs its noise floor, and a margin above it.
        let dist = |margin: f64, floor: Option<f64>| {
            v6_edited(|d| {
                let b = &mut d["bands"][0];
                b["measure"] = serde_json::json!({"kind": "distribution", "of": "goals", "width": 1.0, "bins": 11});
                b["margin"]["value"] = margin.into();
                b["margin"]["noticeable_error"] = margin.into();
                if let Some(f) = floor {
                    b["engine_spread"]["noise_floor"] = f.into();
                }
            })
        };
        assert!(refusal(&dist(0.05, None)).contains("needs its noise floor"));
        assert!(refusal(&dist(0.05, Some(0.06))).contains("(goals_per_match).margin.value"));
        assert!(Registry::from_bytes(dist(0.07, Some(0.06)).as_bytes()).is_ok());
    }

    #[test]
    fn a_target_key_the_reference_file_lacks_is_refused_naming_the_key() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gate/real-targets.json");
        let file = crate::real_targets::ReferenceFile::load(&path).unwrap();
        let named = |key: &str| {
            v6_edited(|d| {
                d["bands"][0]["real"]["source"] = serde_json::json!({"target": key});
                d["bands"][0]["resampling"]["real"] = "target".into();
                // Wide enough for any target's error.
                d["bands"][0]["margin"]["value"] = 5.0.into();
                d["bands"][0]["margin"]["noticeable_error"] = 5.0.into();
            })
        };
        let ok = Registry::parse(named("tier.1.goals_per_match").as_bytes(), Some(&file)).unwrap();
        assert_eq!(
            ok.band("goals_per_match")
                .unwrap()
                .real
                .as_ref()
                .unwrap()
                .target_key(),
            Some("tier.1.goals_per_match")
        );
        let err = Registry::parse(named("tier.9.goals_per_match").as_bytes(), Some(&file))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains(
                "(goals_per_match).real.source.target: tier.9.goals_per_match is not a target"
            ),
            "{err}"
        );
        // A target source goes with a target resampling.
        let err = refusal(&v6_edited(|d| {
            d["bands"][0]["real"]["source"] =
                serde_json::json!({"target": "tier.1.goals_per_match"});
        }));
        assert!(err.contains("(goals_per_match).resampling.real"), "{err}");
    }

    /// The fixture with the band at `i` widened by `w`, provisional or not, with `reason`.
    fn widened(i: usize, w: f64, provisional: bool, reason: &str) -> String {
        v6_edited(|d| {
            let m = &mut d["bands"][i]["margin"];
            let v = m["value"].as_f64().unwrap();
            m["value"] = (v + w).into();
            m["widening"] =
                serde_json::json!({"value": w, "reason": reason, "provisional": provisional});
        })
    }

    #[test]
    fn a_widened_margin_holds_its_parts_and_its_reason() {
        let r = Registry::from_bytes(widened(0, 0.2, true, PROVISIONAL_REASON).as_bytes()).unwrap();
        let m = &r
            .band("goals_per_match")
            .unwrap()
            .rule
            .as_ref()
            .unwrap()
            .margin;
        assert!((m.value - 0.6).abs() < 1e-12);
        assert_eq!(m.widened_by(), 0.2);
        assert_eq!(
            r.provisional(),
            [Provisional {
                band: "goals_per_match".into(),
                margin: m.value,
                widening: 0.2,
                reason: PROVISIONAL_REASON.into(),
            }]
        );
        // A widening that is not provisional cites a noticeable error.
        let ok = widened(
            0,
            0.2,
            false,
            "a player notices a fifth of a goal a match more",
        );
        let r = Registry::from_bytes(ok.as_bytes()).unwrap();
        assert!(r.provisional().is_empty());
        let cases: [(&str, String, &str); 4] = [
            (
                "a blank reason",
                widened(0, 0.2, true, " "),
                "(goals_per_match).margin.widening.reason: must not be blank",
            ),
            (
                "the provisional reason on a widening that is not provisional",
                widened(0, 0.2, false, PROVISIONAL_REASON),
                "(goals_per_match).margin.widening.reason: a widening that is not provisional",
            ),
            (
                "a widening of 0",
                widened(0, 0.0, true, PROVISIONAL_REASON),
                "(goals_per_match).margin.widening.value: must be above 0",
            ),
            (
                "a value that is not its three parts",
                v6_edited(|d| {
                    d["bands"][0]["margin"]["widening"] = serde_json::json!({"value": 0.2, "reason": PROVISIONAL_REASON, "provisional": true});
                }),
                "(goals_per_match).margin.value: 0.4 is not the league spread 0 plus the noticeable error 0.4 plus the widening 0.2",
            ),
        ];
        for (what, text, named) in cases {
            let err = refusal(&text);
            assert!(err.contains(named), "{what}: {err}");
        }
    }

    #[test]
    fn a_provisional_widening_to_the_floor_loads() {
        let r = Registry::from_bytes(v6_doc().to_string().as_bytes()).unwrap();
        let sigma_e = pass_rule::floor::engine_error(1.7, 0.1, 1000.0, 30.0);
        let needed = pass_rule::floor::needed_margin(sigma_e, 0.0, 0.05, 0.97, r.gating());
        // Narrow the noticeable error under the floor, then widen back to the floor.
        let text = |w: f64| {
            v6_edited(|d| {
                let m = &mut d["bands"][0]["margin"];
                m["noticeable_error"] = (needed / 2.0).into();
                m["value"] = (needed / 2.0 + w).into();
                if w > 0.0 {
                    m["widening"] = serde_json::json!({"value": w, "reason": PROVISIONAL_REASON, "provisional": true});
                }
            })
        };
        assert!(refusal(&text(0.0)).contains("is below the 97 percent floor"));
        assert!(Registry::from_bytes(text(needed / 2.0).as_bytes()).is_ok());
    }

    #[test]
    fn a_report_only_band_skips_the_floor_and_shows_whether_it_would_hold() {
        // The facing band under its floor: report only, so the load goes on.
        let text = v6_edited(|d| {
            d["bands"][3]["margin"]["value"] = 0.001.into();
            d["bands"][3]["margin"]["noticeable_error"] = 0.001.into();
        });
        let r = Registry::from_bytes(text.as_bytes()).unwrap();
        let shown = r.report_only_floor(&|_| None);
        assert_eq!(shown.len(), 1);
        assert_eq!(
            (shown[0].band.as_str(), shown[0].group.as_str()),
            ("facing_ball_share", "facing")
        );
        assert!(!shown[0].holds);
        assert!(shown[0].needed > 0.001);
        // At its own margin it would hold with no real error, and not with a large one.
        let r = Registry::from_bytes(v6_doc().to_string().as_bytes()).unwrap();
        assert!(r.report_only_floor(&|_| Some(0.0))[0].holds);
        assert!(!r.report_only_floor(&|_| Some(0.5))[0].holds);
        // The gating count leaves the report-only band out.
        assert_eq!(r.gating(), 3);
    }

    #[test]
    fn a_second_named_group_holds_its_most_and_refuses_one_more() {
        let doc = |n: usize| {
            v6_edited(|d| {
                d["report_only"]
                    .as_array_mut()
                    .unwrap()
                    .push(serde_json::json!({"group": "rare_real_error", "most": 4, "reason": "rare events with a real figure too uncertain to hold the floor"}));
                for k in 0..n {
                    let mut b = v6_band(
                        &format!("rare_{k}"),
                        0.4,
                        serde_json::json!({"real": {}}),
                        pointer(),
                    );
                    b["group"] = "rare_real_error".into();
                    b["gates"] = false.into();
                    d["bands"].as_array_mut().unwrap().push(b);
                }
            })
        };
        let r = Registry::from_bytes(doc(4).as_bytes()).unwrap();
        assert_eq!(r.gating(), 3);
        assert_eq!(r.report_only_floor(&|_| None).len(), 5);
        let err = refusal(&doc(5));
        assert!(
            err.contains(
                "report_only (rare_real_error): 5 bands report only; the group allows at most 4"
            ),
            "{err}"
        );
    }
}

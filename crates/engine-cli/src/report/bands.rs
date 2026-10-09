//! The band registry, read from `realism-bands.json` in the content folder. The bands are
//! acceptance criteria, never tuning values: the calibration harness reads them and nothing
//! writes them.
//!
//! The file is a list of bands (version 4). Each band names its measure (a mean, a share,
//! or a pooled ratio of per-match fields from a closed catalog, or one of three built-in
//! measures), the suites that judge it, its design range and the smallest shift that
//! matters, and its full record: where its real value is measured (a pointer into the
//! real-data folder, or the reason it has no real counterpart), the sample size and date,
//! its evidence label, its two confidence counts, its pitch condition, and whether it gates.
//! The file holds no real value: the calibrate command reads it in place at each judgement.
//!
//! A version 3 file (the range flat on the band, no record) still loads and migrates in
//! memory: every band gates, its condition is dry, and it has no real record. A version 2
//! file, one fixed field per band, migrates to version 3 first, each smallest shift a
//! quarter of its range.

use std::collections::BTreeMap;

use engine::EngineError;
use engine::data::{ContentDir, load_json_bytes, read_bytes};
use garde::Validate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{RED_CARD_FLOOR, RED_CARD_LIMIT, Suite};

/// The file name inside the content folder.
pub const BANDS_FILE: &str = "realism-bands.json";
/// The layout version this build writes and reads.
pub const REGISTRY_VERSION: u32 = 4;
/// The layout before the band record, migrated on load.
pub const V3_VERSION: u32 = 3;
/// The oldest layout this build migrates on load.
pub const LEGACY_VERSION: u32 = 2;
/// The generalisation floor of a file that names none (a migrated file): competitions
/// times seasons.
pub const DEFAULT_GENERALISATION_FLOOR: u32 = 2;

const KIND: &str = "realism bands";

/// A closed range, `lo` to `hi`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[garde(allow_unvalidated)]
pub struct Band {
    pub lo: f64,
    #[garde(custom(at_least(self.lo)))]
    pub hi: f64,
}

impl Band {
    pub fn contains(&self, value: f64) -> bool {
        (self.lo..=self.hi).contains(&value)
    }

    /// A floor band starts at 0, so only a value above `hi` is a miss.
    pub fn is_floor(&self) -> bool {
        self.lo == 0.0 && self.hi > 0.0
    }
}

fn at_least(lo: f64) -> impl FnOnce(&f64, &()) -> garde::Result {
    move |hi, _| {
        if *hi >= lo {
            Ok(())
        } else {
            Err(garde::Error::new(format!("hi {hi} is below lo {lo}")))
        }
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
        }
    }

    /// The bins of a measures build's rows, as `(width, bins)`, for the sources read from
    /// them; `None` for a source binned from the band record.
    pub fn row_bins(self) -> Option<(f64, u32)> {
        use crate::calibrate::rows::{CHAIN_PASS_BINS, CHAIN_SECOND_BINS, SHOT_BINS};
        // At most a few dozen bins.
        match self {
            Source::Goals | Source::Passes => None,
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

/// The source of a band's real value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RealSource {
    Pointer(Box<Pointer>),
    NoCounterpart(NoCounterpart),
}

/// A band's real record: its source, the sample size behind it, the date it was measured
/// (or, with no counterpart, the date of the decision), and an optional floor on the sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealRecord {
    pub source: RealSource,
    pub sample: u64,
    pub date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor: Option<u64>,
}

impl RealRecord {
    /// The reason, when the band has no real counterpart.
    pub fn no_counterpart(&self) -> Option<&str> {
        match &self.source {
            RealSource::NoCounterpart(n) => Some(&n.none),
            RealSource::Pointer(_) => None,
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

/// One band of the registry, in memory. The design range is flat (`lo`, `hi`,
/// `smallest_shift`), as every reader of a band takes it; the file nests it under `design`.
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
    /// The real record; `None` only for a band migrated from version 3.
    pub real: Option<RealRecord>,
    /// `None` only for a migrated band.
    pub evidence: Option<Evidence>,
    /// `None` only for a migrated band.
    pub confidence: Option<Confidence>,
    pub condition: Condition,
    /// The most this band may sit from its dry band; never on a dry band.
    pub max_from_dry: Option<f64>,
    /// The band changes the verdict. A band with `gates: false` only reports.
    pub gates: bool,
    /// Why the band only reports and never changes the verdict: `gates: false`, a real
    /// sample under its floor, or a generalisation under the file's floor. Derived on load.
    pub report_only: Option<String>,
}

/// The part of a band that decides its verdict, as the digests hash it. A gating band
/// without a real floor serializes to the same bytes as a version 3 band, so a version 3
/// file and its version 4 backfill judge alike.
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
    floor: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    report_only: Option<&'a str>,
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
            floor: self.real.as_ref().and_then(|r| r.floor),
            report_only: self.report_only.as_deref(),
        }
    }

    /// A short digest of everything that decides the band's verdict: its measure, suites,
    /// range and smallest shift, and whether it only reports. The real record is reported
    /// beside the verdict and never changes it, so it is left out.
    pub fn digest(&self) -> String {
        let bytes = serde_json::to_vec(&self.judgement()).unwrap_or_default();
        engine::data::hex12(&Sha256::digest(bytes))
    }

    /// A band of a migrated file: it gates, its condition is dry, and it has no record.
    fn migrated(band: BandV3) -> Self {
        BandDef {
            band: band.band,
            suites: band.suites,
            measure: band.measure,
            lo: band.lo,
            hi: band.hi,
            smallest_shift: band.smallest_shift,
            group: band.group,
            real: None,
            evidence: None,
            confidence: None,
            condition: Condition::Dry,
            max_from_dry: None,
            gates: true,
            report_only: None,
        }
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
    /// A band with a real counterpart whose generalisation (competitions times seasons) is
    /// under this floor only reports.
    pub generalisation_floor: u32,
    pub bands: Vec<BandDef>,
    /// The layout version the file was migrated from; `None` for a version 4 file.
    pub migrated_from: Option<u32>,
}

impl Registry {
    /// Loads the registry from the content folder: a version 4 file as it is, a version 3
    /// or 2 file through the migrations. Any other version is refused, as is a band without
    /// a required field of its record, with an unknown field, an unknown catalog field, no
    /// fields, an inverted range, a smallest shift that is not positive, an unknown suite,
    /// or a name used twice; the refusal names the file, the band and the field.
    pub fn load(dir: &ContentDir) -> anyhow::Result<Self> {
        let bytes = read_bytes(&dir.path(BANDS_FILE), BANDS_FILE)?;
        Ok(Self::from_bytes(&bytes)?)
    }

    /// [`Self::load`] over bytes already in memory.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EngineError> {
        let doc = serde_json::from_slice::<Value>(bytes).ok();
        let version = doc.as_ref().and_then(|v| v["schema_version"].as_u64());
        let refuse = |(field, reason): (String, String)| EngineError::Data {
            kind: KIND,
            path: BANDS_FILE.to_string(),
            field,
            reason,
        };
        let mut registry = if version == Some(u64::from(LEGACY_VERSION)) {
            load_json_bytes::<BandsV2>(KIND, bytes, BANDS_FILE, LEGACY_VERSION, &())?
                .value
                .migrate()
                .migrate()
        } else if version == Some(u64::from(V3_VERSION)) {
            load_json_bytes::<RegistryV3>(KIND, bytes, BANDS_FILE, V3_VERSION, &())?
                .value
                .migrate()
        } else {
            if let (Some(doc), Some(4)) = (&doc, version) {
                required(doc).map_err(refuse)?;
            }
            load_json_bytes::<RegistryV4>(KIND, bytes, BANDS_FILE, REGISTRY_VERSION, &())?
                .value
                .into_registry()
        };
        registry.check().map_err(refuse)?;
        let floor = u64::from(registry.generalisation_floor);
        for b in &mut registry.bands {
            b.report_only = report_only(b, floor);
        }
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
            // A version 4 file nests the range under `design`; a migrated one has it flat.
            let design = |field: &str| {
                if self.migrated_from.is_some() {
                    at(field)
                } else {
                    at(&format!("design.{field}"))
                }
            };
            if !(b.lo.is_finite() && b.hi.is_finite()) || b.hi < b.lo {
                return Err((design("hi"), format!("hi {} is below lo {}", b.hi, b.lo)));
            }
            if !(b.smallest_shift.is_finite() && b.smallest_shift > 0.0) {
                return Err((
                    design("smallest_shift"),
                    format!("{} is not above 0", b.smallest_shift),
                ));
            }
            if b.suites.is_empty() {
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
        Ok(())
    }

    /// The band named `name`.
    pub fn band(&self, name: &str) -> Option<&BandDef> {
        self.bands.iter().find(|b| b.band == name)
    }

    /// The range of the band `name`, if the registry has it.
    pub fn range(&self, name: &str) -> Option<Band> {
        self.band(name).map(BandDef::range)
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
    /// range, shift, measure, suite or gating changes it; the real record does not.
    pub fn digest(&self) -> String {
        let views: Vec<Judgement<'_>> = self.bands.iter().map(BandDef::judgement).collect();
        let bytes = serde_json::to_vec(&views).unwrap_or_default();
        engine::data::hex12(&Sha256::digest(bytes))
    }

    /// The wall-time budget of a suite of `matches` matches, in milliseconds.
    pub fn wall_budget_ms(&self, matches: u32) -> u64 {
        let per_match = self.wall_minutes_per_sample * 60_000.0 / f64::from(self.sample_size);
        // At most 100 000 matches of at most 10 000 minutes: far inside u64.
        (per_match * f64::from(matches)).round() as u64
    }
}

/// Why a band only reports, in this order: it does not gate; its real sample is under its
/// own floor, so the real data sets its uncertainty and more matches cannot shrink it; or it
/// has a real counterpart whose generalisation is under the file's floor. A band with no
/// counterpart has no sample and no generalisation to judge, so neither floor applies.
fn report_only(b: &BandDef, floor: u64) -> Option<String> {
    if !b.gates {
        return Some("gates: false in the band file".into());
    }
    let real = b.real.as_ref()?;
    if real.no_counterpart().is_some() {
        return None;
    }
    if let Some(f) = real.floor.filter(|f| real.sample < *f) {
        return Some(format!(
            "real sample too small to gate (sample {} under floor {f})",
            real.sample
        ));
    }
    let units = b.confidence?.generalisation.units();
    (units < floor).then(|| {
        format!("generalisation {units} (competitions x seasons) is under the floor {floor}")
    })
}

/// The fields every band of a version 4 file must hold, with no default.
const REQUIRED: [&str; 11] = [
    "design.lo",
    "design.hi",
    "design.smallest_shift",
    "real.source",
    "real.sample",
    "real.date",
    "evidence",
    "confidence.measurement",
    "confidence.generalisation",
    "condition",
    "gates",
];

/// The keys a real source may hold.
const SOURCE_KEYS: [&str; 24] = [
    "none",
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

/// The record rules of a version 4 file, checked on the raw document before the typed
/// parse, so that every refusal names the band and the field: a missing required field, a
/// source that is neither a pointer nor a no-counterpart record, a blank text, a sample that
/// does not fit the source, an unknown evidence word, and a cap on a dry band.
fn required(doc: &Value) -> Result<(), (String, String)> {
    let Some(bands) = doc["bands"].as_array() else {
        return Ok(());
    };
    for (i, b) in bands.iter().enumerate() {
        let name = b["band"].as_str().unwrap_or("?");
        let at = |field: &str| format!("bands[{i}] ({name}).{field}");
        let get = |path: &str| path.split('.').try_fold(b, |v, k| v.get(k));
        for field in REQUIRED {
            if get(field).is_none_or(Value::is_null) {
                return Err((at(field), "is required".into()));
            }
        }
        if !b["gates"].is_boolean() {
            return Err((at("gates"), "must be true or false".into()));
        }
        let blank = |v: &Value| v.as_str().is_none_or(|s| s.trim().is_empty());
        if blank(&b["real"]["date"]) {
            return Err((at("real.date"), "must be a date such as 2026-09-22".into()));
        }
        let Some(source) = b["real"]["source"].as_object() else {
            return Err((at("real.source"), "must be an object".into()));
        };
        if let Some(key) = source.keys().find(|k| !SOURCE_KEYS.contains(&k.as_str())) {
            return Err((at(&format!("real.source.{key}")), "is not a field".into()));
        }
        let none = source.contains_key("none");
        let pointer = ["dataset", "file", "key"]
            .iter()
            .any(|k| source.contains_key(*k));
        if none && pointer {
            return Err((
                at("real.source"),
                "holds both none and a pointer; use one".into(),
            ));
        }
        if none {
            if blank(&source["none"]) {
                return Err((at("real.source.none"), "needs a reason".into()));
            }
        } else {
            for k in ["dataset", "file", "key"] {
                match source.get(k) {
                    None => {
                        return Err((
                            at("real.source"),
                            "needs dataset, file and key, or none with a reason".into(),
                        ));
                    }
                    Some(v) if blank(v) => {
                        return Err((at(&format!("real.source.{k}")), "is blank".into()));
                    }
                    Some(_) => {}
                }
            }
        }
        match (none, b["real"]["sample"].as_u64()) {
            (_, None) => {
                return Err((at("real.sample"), "must be a whole number".into()));
            }
            (false, Some(0)) => {
                return Err((
                    at("real.sample"),
                    "is 0; a real value needs the sample size behind it".into(),
                ));
            }
            (true, Some(n)) if n > 0 => {
                return Err((
                    at("real.sample"),
                    "is above 0, but the band has no real counterpart".into(),
                ));
            }
            _ => {}
        }
        if none && b["real"].get("floor").is_some() {
            return Err((
                at("real.floor"),
                "a band with no real counterpart has no sample to set a floor on".into(),
            ));
        }
        if !matches!(b["evidence"].as_str(), Some("calibration" | "validation")) {
            return Err((at("evidence"), "must be calibration or validation".into()));
        }
        if b["condition"] == "dry" && b.get("max_from_dry").is_some() {
            return Err((
                at("max_from_dry"),
                "a dry band is the reference; only another condition has a cap".into(),
            ));
        }
    }
    Ok(())
}

/// The version 4 layout of the file.
#[derive(Debug, Clone, PartialEq, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
struct RegistryV4 {
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
    bands: Vec<BandV4>,
}

/// A band's design range in the version 4 file.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Design {
    lo: f64,
    hi: f64,
    smallest_shift: f64,
}

/// One band of the version 4 file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct BandV4 {
    band: String,
    suites: Vec<String>,
    measure: Measure,
    design: Design,
    #[serde(default)]
    group: Option<String>,
    real: RealRecord,
    evidence: Evidence,
    confidence: Confidence,
    condition: Condition,
    #[serde(default)]
    max_from_dry: Option<f64>,
    gates: bool,
}

impl RegistryV4 {
    fn into_registry(self) -> Registry {
        let bands = self
            .bands
            .into_iter()
            .map(|b| BandDef {
                band: b.band,
                suites: b.suites,
                measure: b.measure,
                lo: b.design.lo,
                hi: b.design.hi,
                smallest_shift: b.design.smallest_shift,
                group: b.group,
                real: Some(b.real),
                evidence: Some(b.evidence),
                confidence: Some(b.confidence),
                condition: b.condition,
                max_from_dry: b.max_from_dry,
                gates: b.gates,
                report_only: None,
            })
            .collect();
        Registry {
            schema_version: self.schema_version,
            sample_size: self.sample_size,
            wall_minutes_per_sample: self.wall_minutes_per_sample,
            stronger_team: self.stronger_team,
            generalisation_floor: self.generalisation_floor,
            bands,
            migrated_from: None,
        }
    }
}

/// The version 3 layout: the range flat on each band, no record.
#[derive(Debug, Clone, PartialEq, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
struct RegistryV3 {
    #[garde(skip)]
    schema_version: u32,
    #[garde(range(min = 1, max = 100_000))]
    sample_size: u32,
    #[garde(range(min = 0.1, max = 10_000.0))]
    wall_minutes_per_sample: f64,
    #[garde(dive)]
    stronger_team: StrongerTeam,
    #[garde(skip)]
    bands: Vec<BandV3>,
}

/// One band of the version 3 file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct BandV3 {
    band: String,
    suites: Vec<String>,
    measure: Measure,
    lo: f64,
    hi: f64,
    smallest_shift: f64,
    #[serde(default)]
    group: Option<String>,
}

impl RegistryV3 {
    /// The same bands in version 4 memory: each gates, its condition is dry, and it has no
    /// real record, evidence label or confidence, because a migration must not invent them.
    fn migrate(self) -> Registry {
        Registry {
            schema_version: REGISTRY_VERSION,
            sample_size: self.sample_size,
            wall_minutes_per_sample: self.wall_minutes_per_sample,
            stronger_team: self.stronger_team,
            generalisation_floor: DEFAULT_GENERALISATION_FLOOR,
            bands: self.bands.into_iter().map(BandDef::migrated).collect(),
            migrated_from: Some(self.schema_version),
        }
    }
}

/// The version 2 layout: one fixed field per band.
#[derive(Debug, Clone, PartialEq, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
struct BandsV2 {
    #[garde(skip)]
    schema_version: u32,
    #[garde(range(min = 1, max = 100_000))]
    sample_size: u32,
    #[garde(dive)]
    goals_per_match: Band,
    #[garde(dive)]
    shots_per_team: Band,
    #[garde(dive)]
    possession_pct: Band,
    #[garde(dive)]
    stronger_team: StrongerTeamV2,
    #[garde(range(min = 0.1, max = 10_000.0))]
    wall_minutes_per_sample: f64,
    #[garde(dive)]
    ten_plus_goals_share: Band,
    #[garde(dive)]
    sending_off_share: Band,
    #[garde(dive)]
    yellow_cards_per_team: Band,
    #[garde(dive)]
    shots_on_target_share: Band,
    #[garde(dive)]
    goals_per_xg: Band,
    #[garde(dive)]
    passes_per_team: Band,
    #[garde(dive)]
    pass_accuracy_pct: Band,
    #[garde(dive)]
    corners_per_team: Band,
    #[garde(dive)]
    throw_ins_per_match: Band,
    #[garde(dive)]
    goal_kicks_per_match: Band,
    #[garde(dive)]
    goalless_share: Band,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
struct StrongerTeamV2 {
    #[garde(range(min = 1.0, max = 2.0))]
    attribute_boost: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    min_win_rate: f64,
}

fn fields(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| (*s).to_string()).collect()
}

const GOALS: [&str; 2] = ["goals.home", "goals.away"];

impl BandsV2 {
    /// The same bands in the version 3 layout: the same ranges, each smallest shift a
    /// quarter of its range, possession as its home and away rows, the stronger team and the
    /// red-card pair as built-in bands. It keeps version 2 as its version, so the registry
    /// names the layout it came from.
    fn migrate(self) -> RegistryV3 {
        let def = |band: &str, suites: &[&str], measure: Measure, range: Band| BandV3 {
            band: band.to_string(),
            suites: fields(suites),
            measure,
            lo: range.lo,
            hi: range.hi,
            // Rounded to 9 places, so 0.8 / 4 reads 0.2 as the shipped file writes it.
            smallest_shift: engine::observe::round_to((range.hi - range.lo) / 4.0, 9),
            group: None,
        };
        let mean = |per: Per, of: &[&str]| Measure::Mean {
            per,
            of: fields(of),
        };
        let share = |op: Op, value: f64| Measure::Share {
            of: fields(&GOALS),
            op,
            value,
        };
        let equal = &["equal"][..];
        let goals = &["equal", "formations"][..];
        let possession = |side: &str| {
            let mut d = def(
                &format!("possession_{side}_pct"),
                equal,
                mean(Per::Match, &[&format!("possession_pct.{side}")]),
                self.possession_pct,
            );
            d.group = Some("possession_pct".into());
            d
        };
        let bands = vec![
            def(
                "goals_per_match",
                goals,
                mean(Per::Match, &GOALS),
                self.goals_per_match,
            ),
            def(
                "shots_per_team",
                equal,
                mean(Per::Team, &["shots.{side}"]),
                self.shots_per_team,
            ),
            possession("home"),
            possession("away"),
            def(
                "ten_plus_goals_share",
                goals,
                share(Op::AtLeast, 10.0),
                self.ten_plus_goals_share,
            ),
            def(
                "sending_off_share",
                equal,
                Measure::Share {
                    of: fields(&["red.home", "red.away"]),
                    op: Op::AtLeast,
                    value: 1.0,
                },
                self.sending_off_share,
            ),
            def(
                "yellow_cards_per_team",
                equal,
                mean(Per::Team, &["yellow.{side}"]),
                self.yellow_cards_per_team,
            ),
            def(
                "shots_on_target_share",
                equal,
                Measure::Ratio {
                    num: fields(&["shots_on_target.home", "shots_on_target.away"]),
                    den: fields(&["shots.home", "shots.away"]),
                },
                self.shots_on_target_share,
            ),
            def(
                "goals_per_xg",
                equal,
                Measure::Ratio {
                    num: fields(&GOALS),
                    den: fields(&["xg.home", "xg.away"]),
                },
                self.goals_per_xg,
            ),
            def(
                "passes_per_team",
                equal,
                mean(Per::Team, &["passes.{side}"]),
                self.passes_per_team,
            ),
            def(
                "pass_accuracy_pct",
                equal,
                mean(Per::Team, &["pass_accuracy_pct.{side}"]),
                self.pass_accuracy_pct,
            ),
            def(
                "corners_per_team",
                equal,
                mean(Per::Team, &["corners.{side}"]),
                self.corners_per_team,
            ),
            def(
                "throw_ins_per_match",
                equal,
                mean(Per::Match, &["throw_ins.home", "throw_ins.away"]),
                self.throw_ins_per_match,
            ),
            def(
                "goal_kicks_per_match",
                equal,
                mean(Per::Match, &["goal_kicks.home", "goal_kicks.away"]),
                self.goal_kicks_per_match,
            ),
            def(
                "goalless_share",
                goals,
                share(Op::Equal, 0.0),
                self.goalless_share,
            ),
            def(
                "stronger_team_win_rate",
                &["strength"],
                Measure::Builtin {
                    name: Builtin::StrongerTeamWinRate,
                },
                Band {
                    lo: self.stronger_team.min_win_rate,
                    hi: 1.0,
                },
            ),
            def(
                "reduced_minus_full",
                &["red-card"],
                Measure::Builtin {
                    name: Builtin::ReducedMinusFull,
                },
                Band {
                    lo: RED_CARD_FLOOR,
                    hi: 0.0,
                },
            ),
            def(
                "full_over_control",
                &["red-card"],
                Measure::Builtin {
                    name: Builtin::FullOverControl,
                },
                Band {
                    lo: 0.0,
                    hi: RED_CARD_LIMIT,
                },
            ),
        ];
        RegistryV3 {
            schema_version: self.schema_version,
            sample_size: self.sample_size,
            wall_minutes_per_sample: self.wall_minutes_per_sample,
            stronger_team: StrongerTeam {
                attribute_boost: self.stronger_team.attribute_boost,
            },
            bands,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn shipped() -> ContentDir {
        ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
    }

    /// The version 2 file as it shipped before the registry, kept as the migration's
    /// fixture.
    const V2: &str = r#"{
  "schema_version": 2,
  "sample_size": 1000,
  "goals_per_match": { "lo": 2.4, "hi": 3.2 },
  "shots_per_team": { "lo": 8.0, "hi": 16.0 },
  "possession_pct": { "lo": 35.0, "hi": 65.0 },
  "stronger_team": { "attribute_boost": 1.15, "min_win_rate": 0.5 },
  "wall_minutes_per_sample": 30.0,
  "ten_plus_goals_share": { "lo": 0.0, "hi": 0.005 },
  "sending_off_share": { "lo": 0.08, "hi": 0.22 },
  "yellow_cards_per_team": { "lo": 1.2, "hi": 2.6 },
  "shots_on_target_share": { "lo": 0.3, "hi": 0.42 },
  "goals_per_xg": { "lo": 0.85, "hi": 1.15 },
  "passes_per_team": { "lo": 350.0, "hi": 550.0 },
  "pass_accuracy_pct": { "lo": 75.0, "hi": 88.0 },
  "corners_per_team": { "lo": 3.5, "hi": 6.5 },
  "throw_ins_per_match": { "lo": 35.0, "hi": 55.0 },
  "goal_kicks_per_match": { "lo": 12.0, "hi": 22.0 },
  "goalless_share": { "lo": 0.04, "hi": 0.12 }
}"#;

    /// The 14 bands of version 2 (possession as one), their ranges, and the confirmed
    /// smallest shifts: a quarter of each range.
    const CONFIRMED: [(&str, f64, f64, f64); 15] = [
        ("goals_per_match", 2.4, 3.2, 0.2),
        ("shots_per_team", 8.0, 16.0, 2.0),
        ("possession_home_pct", 35.0, 65.0, 7.5),
        ("possession_away_pct", 35.0, 65.0, 7.5),
        ("ten_plus_goals_share", 0.0, 0.005, 0.00125),
        ("sending_off_share", 0.08, 0.22, 0.035),
        ("yellow_cards_per_team", 1.2, 2.6, 0.35),
        ("shots_on_target_share", 0.3, 0.42, 0.03),
        ("goals_per_xg", 0.85, 1.15, 0.075),
        ("passes_per_team", 350.0, 550.0, 50.0),
        ("pass_accuracy_pct", 75.0, 88.0, 3.25),
        ("corners_per_team", 3.5, 6.5, 0.75),
        ("throw_ins_per_match", 35.0, 55.0, 5.0),
        ("goal_kicks_per_match", 12.0, 22.0, 2.5),
        ("goalless_share", 0.04, 0.12, 0.02),
    ];

    fn assert_confirmed(r: &Registry) {
        for (name, lo, hi, shift) in CONFIRMED {
            let b = r.band(name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!((b.lo, b.hi), (lo, hi), "{name}");
            assert!((b.smallest_shift - shift).abs() < 1e-9, "{name}");
        }
        let stronger = r.band("stronger_team_win_rate").unwrap();
        assert_eq!((stronger.lo, stronger.hi), (0.5, 1.0));
        assert!((stronger.smallest_shift - 0.125).abs() < 1e-12);
        let reduced = r.band("reduced_minus_full").unwrap();
        assert_eq!(
            (reduced.lo, reduced.hi, reduced.smallest_shift),
            (-10.0, 0.0, 2.5)
        );
        let full = r.band("full_over_control").unwrap();
        assert_eq!((full.lo, full.hi), (0.0, 1.6));
        assert!((full.smallest_shift - 0.4).abs() < 1e-12);
        assert_eq!(r.sample_size, 1000);
        assert_eq!(r.stronger_team.attribute_boost, 1.15);
        assert_eq!(r.wall_budget_ms(1000), 1_800_000);
    }

    #[test]
    fn the_shipped_registry_holds_the_accepted_bands_and_the_confirmed_shifts() {
        let r = Registry::load(&shipped()).unwrap();
        assert_eq!(r.schema_version, REGISTRY_VERSION);
        assert_eq!(r.migrated_from, None);
        assert_confirmed(&r);
        assert_eq!(r.bands.len(), 18 + REPORTED.len());
        assert_eq!(r.bands.iter().filter(|b| b.gates).count(), 18);
        assert_eq!(
            r.band("possession_home_pct").unwrap().group.as_deref(),
            Some("possession_pct")
        );
        assert!(r.range("ten_plus_goals_share").unwrap().is_floor());
        assert!(!r.range("goalless_share").unwrap().is_floor());
    }

    /// The judgement of two registries is the same: every band's measure, suites, range,
    /// smallest shift and group, band by band, and every digest.
    fn assert_same_judgement(a: &Registry, b: &Registry) {
        assert_eq!(a.names(), b.names());
        for (x, y) in a.bands.iter().zip(&b.bands) {
            assert_eq!(
                (
                    &x.suites,
                    &x.measure,
                    x.lo,
                    x.hi,
                    x.smallest_shift,
                    &x.group
                ),
                (
                    &y.suites,
                    &y.measure,
                    y.lo,
                    y.hi,
                    y.smallest_shift,
                    &y.group
                ),
                "{}",
                x.band
            );
            assert_eq!(x.digest(), y.digest(), "{}", x.band);
        }
        assert_eq!(a.digest(), b.digest());
    }

    /// The bands of `r` that gate: the shipped file's 18, which the report-first bands
    /// added after them must leave as they were.
    fn gating(r: &Registry) -> Registry {
        let mut g = r.clone();
        g.bands.retain(|b| b.gates);
        g
    }

    /// The bands the shipped file adds with `gates: false`, in file order.
    const REPORTED: [&str; 30] = [
        "fouls_per_team",
        "offsides_per_team",
        "home_goal_difference",
        "five_one_share",
        "worst_to_best_spread",
        "goals_poisson_ratio",
        "shot_distance_mean",
        "possession_own_third_share",
        "possession_middle_third_share",
        "possession_final_third_share",
        "goals_open_play_share",
        "goals_corner_share",
        "goals_free_kick_share",
        "goals_penalty_share",
        "goals_throw_in_share",
        "pass_completion_ground_lt10",
        "pass_completion_ground_10_25",
        "pass_completion_chip_10_25",
        "pass_completion_chip_gt25",
        "pass_completion_cross_10_25",
        "pass_completion_cross_gt25",
        "distance_per_minute",
        "high_speed_share",
        "peak_speed_mean",
        "goals_distribution",
        "scoreline_cells",
        "passes_per_team_distribution",
        "shot_distance_distribution",
        "chain_passes_distribution",
        "chain_seconds_distribution",
    ];

    /// The registry digest of the 18 gating bands, as it stood before the distribution bands: the
    /// whole-distribution bands that follow them change none of their judgement.
    const GATING_DIGEST: &str = "2cdb3c309f21";

    #[test]
    fn the_whole_distribution_bands_parse_and_leave_the_gating_digest_as_it_was() {
        let r = Registry::load(&shipped()).unwrap();
        assert_eq!(gating(&r).digest(), GATING_DIGEST);
        let kind = |name: &str| r.band(name).unwrap().measure.clone();
        assert_eq!(
            kind("goals_distribution"),
            Measure::Distribution {
                of: Source::Goals,
                width: 1.0,
                bins: 11
            }
        );
        assert_eq!(kind("scoreline_cells"), Measure::Cells { max: 5 });
        for name in [
            "goals_distribution",
            "scoreline_cells",
            "passes_per_team_distribution",
            "shot_distance_distribution",
            "chain_passes_distribution",
            "chain_seconds_distribution",
        ] {
            let b = r.band(name).unwrap();
            assert!(b.is_distribution(), "{name}");
            assert!(!b.gates, "{name}");
            assert_eq!(b.lo, 0.0, "{name}");
            assert!((b.smallest_shift - b.hi / 4.0).abs() < 1e-9, "{name}");
            let RealSource::Pointer(p) = &b.real.as_ref().unwrap().source else {
                panic!("{name}");
            };
            assert!(
                matches!(
                    p.reduce,
                    Some(Reduce::Histogram | Reduce::Cells | Reduce::Chains)
                ),
                "{name}"
            );
        }
        // Which read the measures: the three read from a measures build's rows.
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
        // An unknown source is refused by the parse, naming the file and the source.
        let err = refusal(&edited("goals_distribution", |b| {
            b["measure"]["of"] = "corners".into();
        }));
        assert!(err.contains(BANDS_FILE) && err.contains("corners"), "{err}");
        let cases: [(&str, Value, &str); 4] = [
            (
                "goals_distribution",
                serde_json::json!({"kind": "distribution", "of": "goals", "width": 0.0, "bins": 11}),
                "width 0",
            ),
            (
                "goals_distribution",
                serde_json::json!({"kind": "distribution", "of": "goals", "width": 1.0, "bins": 1}),
                "bins 1",
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
        ];
        for (band, measure, named) in cases {
            let err = refusal(&edited(band, |b| b["measure"] = measure));
            assert!(err.contains(&format!("({band}).measure")), "{err}");
            assert!(err.contains(named), "{named}: {err}");
        }
        // An unknown chain count is refused (the pointer then matches no source).
        let err = refusal(&edited("chain_passes_distribution", |b| {
            b["real"]["source"]["count"] = "touches".into();
        }));
        assert!(
            err.contains(BANDS_FILE) && err.contains("RealSource"),
            "{err}"
        );
    }

    #[test]
    fn a_version_two_file_migrates_to_the_same_bands_as_the_shipped_registry() {
        let migrated = Registry::from_bytes(V2.as_bytes()).unwrap();
        assert_eq!(migrated.migrated_from, Some(2));
        assert_eq!(migrated.schema_version, REGISTRY_VERSION);
        assert_confirmed(&migrated);
        let shipped = Registry::load(&shipped()).unwrap();
        assert_same_judgement(&migrated, &gating(&shipped));
    }

    /// The version 3 file as it shipped before the band record, kept as the migration's
    /// fixture.
    const V3: &str = r#"{
  "schema_version": 3,
  "sample_size": 1000,
  "wall_minutes_per_sample": 30.0,
  "stronger_team": { "attribute_boost": 1.15 },
  "bands": [
    { "band": "goals_per_match", "suites": ["equal", "formations"], "measure": { "kind": "mean", "per": "match", "of": ["goals.home", "goals.away"] }, "lo": 2.4, "hi": 3.2, "smallest_shift": 0.2 },
    { "band": "shots_per_team", "suites": ["equal"], "measure": { "kind": "mean", "per": "team", "of": ["shots.{side}"] }, "lo": 8.0, "hi": 16.0, "smallest_shift": 2.0 },
    { "band": "possession_home_pct", "suites": ["equal"], "measure": { "kind": "mean", "per": "match", "of": ["possession_pct.home"] }, "lo": 35.0, "hi": 65.0, "smallest_shift": 7.5, "group": "possession_pct" },
    { "band": "possession_away_pct", "suites": ["equal"], "measure": { "kind": "mean", "per": "match", "of": ["possession_pct.away"] }, "lo": 35.0, "hi": 65.0, "smallest_shift": 7.5, "group": "possession_pct" },
    { "band": "ten_plus_goals_share", "suites": ["equal", "formations"], "measure": { "kind": "share", "of": ["goals.home", "goals.away"], "op": ">=", "value": 10.0 }, "lo": 0.0, "hi": 0.005, "smallest_shift": 0.00125 },
    { "band": "sending_off_share", "suites": ["equal"], "measure": { "kind": "share", "of": ["red.home", "red.away"], "op": ">=", "value": 1.0 }, "lo": 0.08, "hi": 0.22, "smallest_shift": 0.035 },
    { "band": "yellow_cards_per_team", "suites": ["equal"], "measure": { "kind": "mean", "per": "team", "of": ["yellow.{side}"] }, "lo": 1.2, "hi": 2.6, "smallest_shift": 0.35 },
    { "band": "shots_on_target_share", "suites": ["equal"], "measure": { "kind": "ratio", "num": ["shots_on_target.home", "shots_on_target.away"], "den": ["shots.home", "shots.away"] }, "lo": 0.3, "hi": 0.42, "smallest_shift": 0.03 },
    { "band": "goals_per_xg", "suites": ["equal"], "measure": { "kind": "ratio", "num": ["goals.home", "goals.away"], "den": ["xg.home", "xg.away"] }, "lo": 0.85, "hi": 1.15, "smallest_shift": 0.075 },
    { "band": "passes_per_team", "suites": ["equal"], "measure": { "kind": "mean", "per": "team", "of": ["passes.{side}"] }, "lo": 350.0, "hi": 550.0, "smallest_shift": 50.0 },
    { "band": "pass_accuracy_pct", "suites": ["equal"], "measure": { "kind": "mean", "per": "team", "of": ["pass_accuracy_pct.{side}"] }, "lo": 75.0, "hi": 88.0, "smallest_shift": 3.25 },
    { "band": "corners_per_team", "suites": ["equal"], "measure": { "kind": "mean", "per": "team", "of": ["corners.{side}"] }, "lo": 3.5, "hi": 6.5, "smallest_shift": 0.75 },
    { "band": "throw_ins_per_match", "suites": ["equal"], "measure": { "kind": "mean", "per": "match", "of": ["throw_ins.home", "throw_ins.away"] }, "lo": 35.0, "hi": 55.0, "smallest_shift": 5.0 },
    { "band": "goal_kicks_per_match", "suites": ["equal"], "measure": { "kind": "mean", "per": "match", "of": ["goal_kicks.home", "goal_kicks.away"] }, "lo": 12.0, "hi": 22.0, "smallest_shift": 2.5 },
    { "band": "goalless_share", "suites": ["equal", "formations"], "measure": { "kind": "share", "of": ["goals.home", "goals.away"], "op": "==", "value": 0.0 }, "lo": 0.04, "hi": 0.12, "smallest_shift": 0.02 },
    { "band": "stronger_team_win_rate", "suites": ["strength"], "measure": { "kind": "builtin", "name": "stronger_team_win_rate" }, "lo": 0.5, "hi": 1.0, "smallest_shift": 0.125 },
    { "band": "reduced_minus_full", "suites": ["red-card"], "measure": { "kind": "builtin", "name": "reduced_minus_full" }, "lo": -10.0, "hi": 0.0, "smallest_shift": 2.5 },
    { "band": "full_over_control", "suites": ["red-card"], "measure": { "kind": "builtin", "name": "full_over_control" }, "lo": 0.0, "hi": 1.6, "smallest_shift": 0.4 }
  ]
}"#;

    #[test]
    fn a_version_three_file_migrates_with_every_band_gating_and_judges_as_the_shipped_file() {
        let migrated = Registry::from_bytes(V3.as_bytes()).unwrap();
        assert_eq!(migrated.migrated_from, Some(3));
        assert_eq!(migrated.schema_version, REGISTRY_VERSION);
        assert_eq!(migrated.generalisation_floor, DEFAULT_GENERALISATION_FLOOR);
        assert_eq!(migrated.bands.len(), 18);
        for b in &migrated.bands {
            assert!(b.gates, "{}", b.band);
            assert_eq!(b.condition, Condition::Dry, "{}", b.band);
            assert_eq!((&b.real, b.evidence, b.confidence), (&None, None, None));
            assert_eq!(b.report_only, None, "{}", b.band);
        }
        // The same digests, registry and per band: the 18 bands judge as before, and a
        // folder judged under version 3 names no band as changed. The report-first bands
        // that follow them leave every one of their digests as it was.
        let shipped = Registry::load(&shipped()).unwrap();
        assert_same_judgement(&migrated, &gating(&shipped));
        assert_confirmed(&migrated);
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
            let reported = REPORTED.contains(&b.band.as_str());
            assert_eq!(b.gates, !reported, "{}", b.band);
            if reported {
                assert_eq!(
                    b.report_only.as_deref(),
                    Some("gates: false in the band file"),
                    "{}",
                    b.band
                );
                assert_eq!(b.evidence, Some(Evidence::Calibration), "{}", b.band);
            } else {
                assert_eq!(
                    b.report_only, None,
                    "{}: every band of the 18 gates",
                    b.band
                );
            }
            match &real.source {
                RealSource::NoCounterpart(n) => {
                    assert!(experiments.contains(&b.band.as_str()), "{}", b.band);
                    assert!(!n.none.trim().is_empty(), "{}", b.band);
                    assert!(n.none.contains("no real match repeats it"), "{}", b.band);
                    assert_eq!(real.sample, 0, "{}", b.band);
                    assert_eq!(c.measurement.count, 0, "{}", b.band);
                }
                RealSource::Pointer(p) => {
                    assert!(!experiments.contains(&b.band.as_str()), "{}", b.band);
                    for text in [&p.dataset, &p.file, &p.key] {
                        assert!(!text.trim().is_empty(), "{}", b.band);
                    }
                    assert!(real.sample > 0, "{}", b.band);
                    assert!(c.measurement.count > 0, "{}", b.band);
                    assert!(c.generalisation.units() >= 2, "{}", b.band);
                }
            }
        }
        let none = r
            .bands
            .iter()
            .filter(|b| {
                b.real
                    .as_ref()
                    .is_some_and(|x| x.no_counterpart().is_some())
            })
            .count();
        assert_eq!(none, experiments.len());
    }

    /// The shipped file with the band `name` changed by `edit` at the raw JSON.
    fn raw_edit(name: &str, edit: impl FnOnce(&mut serde_json::Map<String, Value>)) -> String {
        edited(name, |b| edit(b.as_object_mut().unwrap()))
    }

    #[test]
    fn a_band_without_a_required_field_is_refused_naming_the_band_and_the_field() {
        for field in REQUIRED {
            let text = raw_edit("corners_per_team", |b| {
                let mut path = field.split('.');
                let first = path.next().unwrap();
                match path.next() {
                    None => {
                        b.remove(first);
                    }
                    Some(second) => {
                        b[first].as_object_mut().unwrap().remove(second);
                    }
                }
            });
            let err = refusal(&text);
            assert!(err.contains(BANDS_FILE), "{field}: {err}");
            assert!(
                err.contains(&format!("(corners_per_team).{field}: is required")),
                "{field}: {err}"
            );
        }
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
                "(corners_per_team).real.source: holds both none and a pointer",
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
                "no counterpart with a floor",
                Box::new(|b| b["real"]["floor"] = 5.into()),
                "(full_over_control).real.floor",
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
    }

    #[test]
    fn a_band_under_the_generalisation_floor_or_not_gating_only_reports() {
        let thin = edited("corners_per_team", |b| {
            b["confidence"]["generalisation"] =
                serde_json::json!({"competitions": 1, "seasons": 1, "clubs": 20});
        });
        let r = Registry::from_bytes(thin.as_bytes()).unwrap();
        let why = r.report_only("corners_per_team").unwrap();
        assert!(why.contains("under the floor 2"), "{why}");
        assert_eq!(r.report_only("goals_per_match"), None);
        // A band that only reports judges differently: its digest moves.
        let shipped = Registry::load(&shipped()).unwrap();
        assert_ne!(
            r.band("corners_per_team").unwrap().digest(),
            shipped.band("corners_per_team").unwrap().digest()
        );
        // A band with no counterpart has counts of 0 and still gates.
        assert_eq!(r.report_only("stronger_team_win_rate"), None);
        let off = edited("corners_per_team", |b| b["gates"] = false.into());
        let r = Registry::from_bytes(off.as_bytes()).unwrap();
        assert!(
            r.report_only("corners_per_team")
                .unwrap()
                .contains("gates: false")
        );
    }

    #[test]
    fn a_real_sample_under_its_floor_only_reports_and_the_reasons_keep_their_order() {
        // A gating band whose real sample is under its own floor: report-only, by name.
        let floored = |extra: &dyn Fn(&mut Value)| {
            Registry::from_bytes(
                edited("corners_per_team", |b| {
                    b["real"]["floor"] = 5000.into();
                    extra(b);
                })
                .as_bytes(),
            )
            .unwrap()
        };
        let r = floored(&|_| {});
        assert_eq!(
            r.report_only("corners_per_team"),
            Some("real sample too small to gate (sample 1826 under floor 5000)")
        );
        // The sample floor comes before the generalisation floor, and gates: false first.
        let thin = floored(&|b| {
            b["confidence"]["generalisation"] =
                serde_json::json!({"competitions": 1, "seasons": 1, "clubs": 20});
        });
        assert!(
            thin.report_only("corners_per_team")
                .unwrap()
                .starts_with("real sample too small to gate")
        );
        let off = floored(&|b| b["gates"] = false.into());
        assert_eq!(
            off.report_only("corners_per_team"),
            Some("gates: false in the band file")
        );
        // A sample at its floor gates; so does a band with no counterpart and no floor.
        let at = Registry::from_bytes(
            edited("corners_per_team", |b| b["real"]["floor"] = 1826.into()).as_bytes(),
        )
        .unwrap();
        assert_eq!(at.report_only("corners_per_team"), None);
        assert_eq!(at.report_only("stronger_team_win_rate"), None);
        // The shipped running bands sit under their floor of 30 tracked matches, so they
        // would stay report-only after a promotion.
        let shipped = Registry::load(&shipped()).unwrap();
        for name in ["distance_per_minute", "high_speed_share", "peak_speed_mean"] {
            let promoted =
                Registry::from_bytes(edited(name, |b| b["gates"] = true.into()).as_bytes())
                    .unwrap();
            assert_eq!(
                promoted.report_only(name),
                Some("real sample too small to gate (sample 7 under floor 30)"),
                "{name}"
            );
            assert!(shipped.band(name).unwrap().needs_measures(), "{name}");
        }
    }

    #[test]
    fn the_new_measure_kinds_parse_and_a_malformed_one_is_refused_by_name() {
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
        // Which bands read the measures, and which read a match's extras at all.
        assert!(!r.band("worst_to_best_spread").unwrap().needs_measures());
        assert!(r.band("worst_to_best_spread").unwrap().needs_extras());
        assert!(r.band("shot_distance_mean").unwrap().needs_measures());
        assert!(!r.band("fouls_per_team").unwrap().needs_extras());
        let cases: [(&str, Value, &str); 6] = [
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
                "shot_distance_mean",
                serde_json::json!({"kind": "ratio", "num": ["pass_attempts.volley_gt25.home"], "den": ["shots.home"]}),
                "pass_attempts.volley_gt25.home",
            ),
        ];
        for (band, measure, named) in cases {
            let err = refusal(&edited(band, |b| b["measure"] = measure));
            assert!(err.contains(&format!("({band}).measure")), "{err}");
            assert!(err.contains(named), "{named}: {err}");
        }
        // An unknown pointer field is refused by name; the new ones are taken.
        let err = refusal(&edited("fouls_per_team", |b| {
            b["real"]["source"]["axes"] = serde_json::json!([1.0, 1.0]);
        }));
        assert!(err.contains("(fouls_per_team).real.source.axes"), "{err}");
    }

    fn refusal(text: &str) -> String {
        Registry::from_bytes(text.as_bytes())
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn another_version_is_refused_naming_the_file_and_the_version() {
        let err = refusal(&V2.replace("\"schema_version\": 2", "\"schema_version\": 1"));
        assert!(err.contains(BANDS_FILE), "{err}");
        assert!(err.contains("schema_version 1"), "{err}");
        assert!(err.contains("reads 4"), "{err}");
        let err = refusal("{\"schema_version\": 9}");
        assert!(err.contains(BANDS_FILE) && err.contains('9'), "{err}");
    }

    #[test]
    fn a_version_two_file_without_a_band_or_with_an_unknown_field_is_refused() {
        let text: String = V2
            .lines()
            .filter(|line| !line.contains("goals_per_xg"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(refusal(&text).contains("goals_per_xg"));
        let err = refusal(&V2.replace("\"sample_size\"", "\"sample_sizes\""));
        assert!(err.contains("sample_sizes"), "{err}");
        let err = refusal(&V2.replace("\"lo\": 2.4, \"hi\": 3.2", "\"lo\": 3.2, \"hi\": 2.4"));
        assert!(err.contains("goals_per_match"), "{err}");
    }

    fn shipped_text() -> String {
        std::fs::read_to_string(shipped().path(BANDS_FILE)).unwrap()
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

    #[test]
    fn each_refusal_names_its_band_and_field() {
        let cases: [(&str, Edit, &str); 7] = [
            (
                "inverted range",
                Box::new(|b| b["design"]["lo"] = 99.0.into()),
                "goals_per_match).design.hi",
            ),
            (
                "zero shift",
                Box::new(|b| b["design"]["smallest_shift"] = 0.0.into()),
                "goals_per_match).design.smallest_shift",
            ),
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
    fn the_digest_follows_a_range_and_a_shift() {
        let base = Registry::from_bytes(shipped_text().as_bytes()).unwrap();
        let narrowed = Registry::from_bytes(
            edited("corners_per_team", |b| b["design"]["hi"] = 6.0.into()).as_bytes(),
        )
        .unwrap();
        assert_ne!(base.digest(), narrowed.digest());
        let a = base.band("corners_per_team").unwrap().digest();
        let b = narrowed.band("corners_per_team").unwrap().digest();
        assert_ne!(a, b);
        assert_eq!(
            base.band("goals_per_match").unwrap().digest(),
            narrowed.band("goals_per_match").unwrap().digest()
        );
    }
}

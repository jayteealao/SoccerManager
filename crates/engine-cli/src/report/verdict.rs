//! The verdict of a change run: one paired max-t bootstrap over the matches both engines
//! played on the same fixtures, the power each band has to see its smallest shift, and a
//! word per band (pass, fail, not sure) with one joint word.
//!
//! For each of [`RESAMPLES`] resamples, the paired fixtures are drawn with replacement
//! within their stratum (a suite, a formation pairing, a red-card arm) from the engine's own
//! random stream, so the result is the same on every machine. Every band's change (changed
//! minus old) is computed under the resample's weights, with its delta-method standard
//! error over the resampled pairs; the statistic of a resample is the largest change over
//! every band, each divided by its own error (studentized). The joint threshold `c` is its
//! 97th percentile: one threshold for every band, set by how the bands move together, with
//! a 3 percent false-alarm limit for the whole run.
//!
//! A band has power when `(c + z) · se <= smallest shift`, with `z` the normal quantile of
//! 80 percent power. For power, `se` is the larger of the paired error and the error the
//! two engines' values would have apart: two engines that differ at all soon play
//! different matches, so the pairing of a run whose engines are still identical says
//! nothing about how precise a real change would be. A band **fails** when the changed
//! engine's value is outside its range, or its change is beyond the threshold and at least
//! its smallest shift; it **passes** when the value is inside, it did not move by its
//! smallest shift, and it has power; otherwise it is **not sure**. A band with no power
//! never passes.
//!
//! A band already outside its range at the pilot by more than its noise (`c` times the
//! changed engine's own error of its value) sets no power target: it fails while it stays
//! outside, so more matches would not change its word, and the other bands size the run.

use std::collections::BTreeMap;

use engine::observe::round_to;
use engine::rng::EngineRng;
use serde::Serialize;

use super::distribution::PairedHist;
use super::measures;
use super::{BandCheck, RuleWord, RunBuilder, Suite};

/// Resamples of the bootstrap.
pub const RESAMPLES: usize = 1999;
/// The joint false-alarm limit of a run.
pub const FALSE_ALARM: f64 = 0.03;
/// The power the paired change test sizes for.
pub const POWER: f64 = 0.8;

/// The normal quantile of 80 percent power, about 0.8416.
pub fn z_power() -> f64 {
    pass_rule::special::norm_quantile(POWER)
}

/// The two-sided normal threshold of one band at the false-alarm limit, about 2.1701: the
/// joint threshold over many bands is never below it, also when no resample varies.
pub fn one_band() -> f64 {
    pass_rule::special::norm_quantile(1.0 - FALSE_ALARM / 2.0)
}
/// Mixed into the run seed for the resampling stream, so it is apart from every match's.
const STREAM_LABEL: u64 = 0x6d61_785f_745f_6273;

/// A band's word, or the run's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Word {
    #[serde(rename = "pass")]
    Pass,
    #[serde(rename = "fail")]
    Fail,
    #[serde(rename = "not sure")]
    NotSure,
}

impl Word {
    pub fn code(self) -> &'static str {
        match self {
            Word::Pass => "pass",
            Word::Fail => "fail",
            Word::NotSure => "not sure",
        }
    }
}

/// One band row of a verdict.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VerdictRow {
    pub band: String,
    pub suite: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairing: Option<String>,
    pub word: Word,
    /// The run's value (the changed engine's in a change run).
    pub value: f64,
    pub lo: f64,
    pub hi: f64,
    /// A change run: the old engine's value on the same fixtures.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_value: Option<f64>,
    /// A change run: the change, changed minus old.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<f64>,
    /// A change run: the bootstrap standard error of the change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub se: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smallest_shift: Option<f64>,
    /// A change run: the band can see its smallest shift with 80 percent power.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power: Option<bool>,
    /// A band without power: about how many matches per suite (per pairing, per arm) would
    /// give it power.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needed: Option<u32>,
    /// A change run: how far the changed engine's value is outside its range, when it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outside_by: Option<f64>,
    /// A change run: the band was outside its range beyond its noise at the pilot, so it set
    /// no power target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_target: Option<bool>,
    /// The band's real value, read in place from the real-data folder; absent when it has
    /// none here. It never changes the word.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real: Option<f64>,
    /// The run's value minus the real value; for a whole-distribution band, the distance of
    /// the run's whole sample from the real one, which is also its value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<f64>,
    /// A whole-distribution band: the one-sided upper bound of its distance, the 97th
    /// percentile of the whole-match resampled distance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upper: Option<f64>,
    /// A whole-distribution band: the noise floor, the 97th percentile of the centred
    /// resampled distance; a margin at or under it can never pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub noise_floor: Option<f64>,
    /// Why the band only reports: its word is shown, but it is left out of the joint word
    /// and of `calib.pass`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_only: Option<String>,
    /// What the console's real columns read when there is no real value.
    #[serde(skip)]
    pub real_note: Option<&'static str>,
    /// How the row was judged: `paired`, by the paired max-t test against the old engine, or
    /// `range`, by its range on this engine alone with its own error. Absent for a gating
    /// row of a run without an old engine, whose word is its plain range check.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub judged: Option<&'static str>,
    /// Why the row was judged as it was, or has no value: `range only: base has no
    /// measures`, `range only: a per-club spread has no paired test`, `this build has no
    /// measures`, `no value: <why>`, or `acceptance no longer applies: it reads <word>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// `true` when the developer accepted this not-sure row in the run folder: it is
    /// accepted, not shown equivalent, and its word stays not sure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted: Option<bool>,
    /// A version 6 band: its word against real football by the pass rule, with the
    /// interval and the margin that set it. The row's word is this word.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pass_rule: Option<RuleWord>,
}

impl VerdictRow {
    /// The run's value is inside the design range: a change row is inside when it carries no
    /// distance outside; a row judged on its range alone when it has a value and that value
    /// lies in the range, a not-sure word near an edge included.
    pub fn inside(&self) -> bool {
        if self.power.is_some() {
            return self.outside_by.is_none();
        }
        match self.word {
            Word::Pass => true,
            Word::Fail => false,
            Word::NotSure => {
                let valueless = self
                    .note
                    .as_deref()
                    .is_some_and(|n| n.starts_with("no value") || n == NO_MEASURES_NOTE);
                !valueless && (self.lo..=self.hi).contains(&self.value)
            }
        }
    }

    /// Sets the row's real value and its distance from the run's value, or the note the
    /// console prints in their place.
    pub fn set_real(&mut self, real: Option<f64>, note: &'static str) {
        self.real = real.map(|r| round_to(r, 5));
        self.distance = real.map(|r| round_to(self.value - r, 5));
        self.real_note = real.is_none().then_some(note);
    }

    /// Marks a whole-distribution row whose real sample was read: its real column reads
    /// "whole sample", and its distance is its value when it has one (a bound).
    pub fn set_whole_sample(&mut self) {
        self.real = None;
        self.distance = self.upper.map(|_| self.value);
        self.real_note = Some("whole sample");
    }
}

/// The joint word over the rows that gate: fail on any guard or any failed row, pass when
/// every row passes, not sure otherwise. A report-only row never counts.
pub fn joint_word(rows: &[VerdictRow], guards: &[String]) -> Word {
    let gating: Vec<&VerdictRow> = rows.iter().filter(|r| r.report_only.is_none()).collect();
    if !guards.is_empty() || gating.iter().any(|r| r.word == Word::Fail) {
        Word::Fail
    } else if !gating.is_empty() && gating.iter().all(|r| r.word == Word::Pass) {
        Word::Pass
    } else {
        Word::NotSure
    }
}

/// Marks the rows of the bands that only report, with the reason `why` gives, and sets the
/// joint word again over the rows that gate.
pub fn mark_report_only<'a>(
    rows: &mut [VerdictRow],
    joint: Option<&mut Joint>,
    why: impl Fn(&str) -> Option<&'a str>,
) {
    for r in rows.iter_mut() {
        r.report_only = why(&r.band).map(str::to_string);
    }
    if let Some(j) = joint {
        j.word = joint_word(rows, &j.guards);
    }
}

/// The joint verdict of a change run.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Joint {
    pub word: Word,
    /// The max-t threshold `c`.
    pub critical: f64,
    pub resamples: usize,
    pub false_alarm: f64,
    /// Fixtures both engines played.
    pub pairs: u32,
    /// Why a fail comes from outside the bands: panics, dark paths, rule violations.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub guards: Vec<String>,
}

/// The note of a row whose band reads measures the run's matches do not carry.
pub const NO_MEASURES_NOTE: &str = "this build has no measures";
/// The note of a measures band in a change run whose old engine has no measures.
pub const BASE_WITHOUT_MEASURES: &str = "range only: base has no measures";
/// The note of a per-club spread in a change run.
pub const PER_CLUB: &str = "range only: a per-club spread has no paired test";

/// A row of `c` with `word` and nothing judged beyond it.
fn plain_row(c: &BandCheck, word: Word) -> VerdictRow {
    VerdictRow {
        band: c.band.clone(),
        suite: c.suite.clone(),
        pairing: c.pairing.clone(),
        word,
        value: c.value,
        lo: c.lo,
        hi: c.hi,
        base_value: None,
        diff: None,
        se: None,
        smallest_shift: None,
        power: None,
        needed: None,
        outside_by: None,
        no_target: None,
        real: None,
        distance: None,
        upper: c.dist.as_ref().map(|d| round_to(d.upper, 5)),
        noise_floor: c.dist.as_ref().map(|d| round_to(d.floor, 5)),
        report_only: None,
        real_note: None,
        judged: None,
        note: None,
        accepted: None,
        pass_rule: c.rule.clone(),
    }
}

/// The note a check carries of itself: no measures, or no value.
fn own_note(c: &BandCheck) -> Option<String> {
    if c.unjudged {
        Some(NO_MEASURES_NOTE.into())
    } else {
        c.why_none.as_ref().map(|w| format!("no value: {w}"))
    }
}

/// The rows of a run without an old engine: each band's word is its range check. A band
/// that reads measures the run's matches do not carry is not sure. A whole-distribution
/// band takes the word of its bounds ([`range_word`]), so it never passes without its upper
/// bound inside the margin.
pub fn range_only(checks: &[BandCheck]) -> Vec<VerdictRow> {
    checks
        .iter()
        .filter(|c| c.band != "wall_ms")
        .map(|c| {
            let word = match (c.unjudged, c.pass) {
                _ if c.whole || c.rule.is_some() => range_word(c),
                (true, _) => Word::NotSure,
                (false, true) => Word::Pass,
                (false, false) => Word::Fail,
            };
            let mut r = plain_row(c, word);
            r.note = match &c.rule {
                Some(rule) => Some(rule.note()),
                None => own_note(c).or_else(|| c.dist.as_ref().map(|d| d.note.clone())),
            };
            r
        })
        .collect()
}

/// The word of a row judged without an old engine: the pass rule's word, from the band's
/// interval against its margin. A row the pass rule did not judge is not sure, never a
/// pass: every band of the band file has the rule's fields, so no fixed range decides.
pub fn range_word(c: &BandCheck) -> Word {
    c.rule
        .as_ref()
        .map_or(Word::NotSure, |rule| rule_word(rule.word))
}

/// The pass rule's word as a verdict word.
pub fn rule_word(w: pass_rule::rule::Word) -> Word {
    match w {
        pass_rule::rule::Word::Pass => Word::Pass,
        pass_rule::rule::Word::NotSure => Word::NotSure,
        pass_rule::rule::Word::Fail => Word::Fail,
    }
}

/// The rows judged on their range alone, each with the noise-aware word of [`range_word`],
/// its own error, and `note` unless the check carries a note of its own.
pub fn range_rows(checks: &[BandCheck], note: Option<&str>) -> Vec<VerdictRow> {
    checks
        .iter()
        .filter(|c| c.band != "wall_ms")
        .map(|c| {
            let mut r = plain_row(c, range_word(c));
            r.se = Some(c.se);
            r.judged = Some("range");
            let bounds = c.dist.as_ref().map(|d| d.note.as_str());
            r.note = match &c.rule {
                Some(rule) => Some(match note {
                    Some(n) => format!("{n}; {}", rule.note()),
                    None => rule.note(),
                }),
                None => own_note(c).or_else(|| match (note, bounds) {
                    (Some(n), Some(b)) => Some(format!("{n}; {b}")),
                    (n, b) => n.or(b).map(str::to_string),
                }),
            };
            r
        })
        .collect()
}

/// One band row of a change run, over the fixtures both engines finished.
#[derive(Debug, Clone)]
pub struct PairedRow {
    pub band: String,
    pub suite: Suite,
    pub pairing: Option<String>,
    pub lo: f64,
    pub hi: f64,
    pub shift: f64,
    /// Per-match sums of the changed and the old engine, one entry per pair.
    pub changed: [Vec<f64>; 4],
    pub old: [Vec<f64>; 4],
    /// The changed engine's value, rounded as its band check shows it, and its range check.
    pub value: Option<f64>,
    pub base_value: Option<f64>,
    pub in_range: bool,
    /// Either engine's value varies from match to match: a band whose event never happened
    /// in either arm has nothing to measure a shift by.
    pub spread: bool,
    /// The error of the change if the two engines' matches were apart: the root of the sum
    /// of each engine's squared error. Power is judged on at least this.
    pub apart_se: f64,
    /// The changed engine's own sampling error of the band's value.
    pub value_se: f64,
    /// How far the changed engine's unrounded value is outside its range; 0 when its range
    /// check holds or it has no value.
    pub outside: f64,
    /// Index of the row's suite set.
    pub set: usize,
    /// A whole-distribution band: both engines' histograms per pair. Its per-match sums
    /// above are 0, and its change is the distance between the two engines.
    pub dist: Option<Box<PairedHist>>,
}

/// The pairs of one suite: each pair's stratum, and the matches per suite unit (per
/// pairing, per arm) the run played.
#[derive(Debug, Clone)]
pub struct SuiteSet {
    pub suite: Suite,
    pub strata: Vec<u32>,
    pub per_unit: u32,
}

/// The paired rows of a change run: for every suite both builders planned, the fixtures
/// both played, joined on key. `keys` give each builder's matches' fixture keys, suite by
/// suite, in the order the builder holds them; `per_unit` the matches per suite unit.
pub fn paired(
    changed: (&RunBuilder, &BTreeMap<Suite, Vec<u64>>),
    old: (&RunBuilder, &BTreeMap<Suite, Vec<u64>>),
    per_unit: &BTreeMap<Suite, u32>,
) -> (Vec<SuiteSet>, Vec<PairedRow>) {
    let (cb, ck) = changed;
    let (ob, ok) = old;
    let specs = cb.row_specs(true);
    let mut sets = Vec::new();
    let mut rows = Vec::new();
    for (&suite, keys) in ck {
        let (Some(old_keys), c_obs) = (ok.get(&suite), cb.observations(suite)) else {
            continue;
        };
        let o_obs = ob.observations(suite);
        let at: BTreeMap<u64, usize> = old_keys.iter().enumerate().map(|(i, k)| (*k, i)).collect();
        // A pair is kept only when both engines played its fixture. A failed match on
        // either side would leave the two sides measured over different fixtures.
        let played = |o: &measures::Obs<'_>| o.stats.outcome == "success";
        let pairs: Vec<(usize, usize)> = keys
            .iter()
            .enumerate()
            .filter_map(|(i, k)| at.get(k).map(|&j| (i, j)))
            .filter(|&(i, j)| i < c_obs.len() && j < o_obs.len())
            .filter(|&(i, j)| played(&c_obs[i]) && played(&o_obs[j]))
            .collect();
        let c_sub: Vec<_> = pairs.iter().map(|&(i, _)| c_obs[i]).collect();
        let o_sub: Vec<_> = pairs.iter().map(|&(_, j)| o_obs[j]).collect();
        let strata = c_sub
            .iter()
            .map(|o| o.pairing.or(o.arm).map_or(0, |v| v as u32 + 1))
            .collect();
        let set = sets.len();
        sets.push(SuiteSet {
            suite,
            strata,
            per_unit: per_unit.get(&suite).copied().unwrap_or(0),
        });
        for spec in specs.iter().filter(|s| s.suite == suite) {
            if spec.def.is_distribution() {
                rows.push(paired_distribution(cb, spec, (&c_sub, &o_sub), set));
                continue;
            }
            let tc = measures::terms(spec.def, &c_sub, spec.scope);
            let to = measures::terms(spec.def, &o_sub, spec.scope);
            let places = measures::places(&spec.def.measure);
            let value = tc.value().map(|v| round_to(v, places));
            let in_range = spec.def.range().contains(value.unwrap_or(0.0));
            let outside = match tc.value() {
                Some(v) if !in_range => (spec.def.lo - v).max(v - spec.def.hi).max(0.0),
                _ => 0.0,
            };
            rows.push(PairedRow {
                band: spec.def.band.clone(),
                suite,
                pairing: spec.label.clone(),
                lo: spec.def.lo,
                hi: spec.def.hi,
                shift: spec.def.smallest_shift,
                spread: tc.se() > 0.0 || to.se() > 0.0,
                apart_se: tc.se().hypot(to.se()),
                value_se: tc.se(),
                outside,
                changed: tc.per_match(),
                old: to.per_match(),
                value,
                base_value: to.value().map(|v| round_to(v, places)),
                in_range,
                set,
                dist: None,
            });
        }
    }
    (sets, rows)
}

/// A whole-distribution band's paired row: each engine's histogram per pair, and each
/// engine's distance from the real sample (the changed engine's is the row's value), from
/// the changed builder's real samples. A pair a band does not cover holds an empty
/// histogram on that side.
fn paired_distribution(
    cb: &RunBuilder,
    spec: &super::RowSpec<'_>,
    (c_sub, o_sub): (&[measures::Obs<'_>], &[measures::Obs<'_>]),
    set: usize,
) -> PairedRow {
    let def = spec.def;
    let cc = cb.distribution_check(spec, c_sub);
    let oc = cb.distribution_check(spec, o_sub);
    let value_of = |c: &BandCheck| c.dist.as_ref().map(|_| c.value);
    let value = value_of(&cc);
    let (width, bins) = measures::hist_bins(&def.measure).unwrap_or((1.0, 2));
    let side = |obs: &[measures::Obs<'_>]| -> Vec<super::distribution::Hist> {
        measures::histograms(def, obs, spec.scope)
            .into_iter()
            .map(Option::unwrap_or_default)
            .collect()
    };
    let hist = PairedHist {
        changed: side(c_sub),
        old: side(o_sub),
        metric: measures::metric(&def.measure),
        width,
        bins,
        upper: cc.dist.as_ref().map(|d| d.upper),
        floor: cc.dist.as_ref().map(|d| d.floor),
    };
    let n = c_sub.len();
    let zeros = || std::array::from_fn(|_| vec![0.0; n]);
    let in_range = value.is_some_and(|v| def.range().contains(v));
    PairedRow {
        band: def.band.clone(),
        suite: spec.suite,
        pairing: spec.label.clone(),
        lo: def.lo,
        hi: def.hi,
        shift: def.smallest_shift,
        changed: zeros(),
        old: zeros(),
        value,
        base_value: value_of(&oc),
        in_range,
        spread: hist.observed().is_some(),
        apart_se: cc.se.hypot(oc.se),
        value_se: cc.se,
        outside: match value {
            Some(v) if !in_range => (v - def.hi).max(def.lo - v).max(0.0),
            _ => 0.0,
        },
        set,
        dist: Some(Box::new(hist)),
    }
}

/// What the bootstrap found for one row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Found {
    /// The change with every weight 1; `None` when either engine has no value.
    pub diff: Option<f64>,
    pub se: f64,
}

/// One row's change under `weights`, and its delta-method standard error with the pairs
/// centred within their strata; `None` when either engine has no value.
///
/// Each engine's value is `(A / B) · (D / C)` over weighted sums. A pair's influence on it
/// is `value · (a/A - b/B + d/D - c/C)`; the change's influence is the changed engine's
/// minus the old engine's, so the pairing cancels what the two engines share. The squared
/// error is the weighted sum of the squared influences, centred in each stratum and scaled
/// by `m / (m - 1)` for a stratum of `m` pairs.
fn change(
    r: &PairedRow,
    weights: &[u32],
    strata: &BTreeMap<u32, Vec<usize>>,
) -> Option<(f64, f64)> {
    let sums = |t: &[Vec<f64>; 4]| -> [f64; 4] {
        let mut s = [0.0f64; 4];
        for (i, &w) in weights.iter().enumerate() {
            if w > 0 {
                let w = f64::from(w);
                for (k, sum) in s.iter_mut().enumerate() {
                    *sum += w * t[k][i];
                }
            }
        }
        s
    };
    let (sc, so) = (sums(&r.changed), sums(&r.old));
    let value = |s: [f64; 4]| (s[1] > 0.0 && s[2] > 0.0).then(|| s[0] / s[1] * (s[3] / s[2]));
    let (vc, vo) = (value(sc)?, value(so)?);
    let part = |t: &[Vec<f64>; 4], s: [f64; 4], v: f64, i: usize| {
        let term = |k: usize| if s[k] > 0.0 { t[k][i] / s[k] } else { 0.0 };
        v * (term(0) - term(1) + term(3) - term(2))
    };
    let mut var = 0.0;
    for members in strata.values() {
        let (mut n, mut sum, mut sq) = (0.0f64, 0.0f64, 0.0f64);
        for &i in members {
            let w = f64::from(weights[i]);
            if w == 0.0 {
                continue;
            }
            let psi = part(&r.changed, sc, vc, i) - part(&r.old, so, vo, i);
            n += w;
            sum += w * psi;
            sq += w * psi * psi;
        }
        if n > 1.0 {
            var += (sq - sum * sum / n) * n / (n - 1.0);
        }
    }
    Some((vc - vo, var.max(0.0).sqrt()))
}

/// The paired max-t bootstrap: each row's change and standard error, and the joint
/// threshold `c`. The statistic is studentized: each resample's change of a band is
/// divided by that resample's own standard error, which keeps the false-alarm rate at its
/// limit on the pilot's few hundred matches, where dividing by one shared error runs high.
pub fn bootstrap(
    sets: &[SuiteSet],
    rows: &[PairedRow],
    seed: u64,
    resamples: usize,
) -> (Vec<Found>, f64) {
    // Each set's strata, as the pair numbers of each stratum.
    let members: Vec<BTreeMap<u32, Vec<usize>>> = sets
        .iter()
        .map(|s| {
            let mut m: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
            for (i, &st) in s.strata.iter().enumerate() {
                m.entry(st).or_default().push(i);
            }
            m
        })
        .collect();
    let ones: Vec<Vec<u32>> = sets.iter().map(|s| vec![1; s.strata.len()]).collect();
    let observed: Vec<Option<(f64, f64)>> = rows
        .iter()
        .map(|r| match &r.dist {
            Some(d) => d.observed().map(|o| (o, 0.0)),
            None => change(r, &ones[r.set], &members[r.set]),
        })
        .collect();
    // A whole-distribution row: its observed difference of profiles, and its centred
    // replicate in each resample. It draws no random number, so the scalar rows' resamples
    // are the same with or without it.
    let diffs: Vec<Option<Vec<f64>>> = rows
        .iter()
        .map(|r| r.dist.as_ref().and_then(|d| d.diff(None)))
        .collect();
    let mut centred: Vec<Vec<f64>> = vec![Vec::new(); rows.len()];
    let mut rng = EngineRng::from_seed(seed ^ STREAM_LABEL);
    let mut stats = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        let weights: Vec<Vec<u32>> = sets
            .iter()
            .zip(&members)
            .map(|(s, strata)| {
                let mut w = vec![0u32; s.strata.len()];
                for idx in strata.values() {
                    for _ in 0..idx.len() {
                        w[idx[rng.range_usize(idx.len())]] += 1;
                    }
                }
                w
            })
            .collect();
        let t = rows
            .iter()
            .zip(&observed)
            .filter(|(r, _)| r.dist.is_none())
            .filter_map(|(r, o)| {
                let (d, _) = (*o)?;
                let (d_star, se_star) = change(r, &weights[r.set], &members[r.set])?;
                (se_star > 0.0).then(|| (d_star - d).abs() / se_star)
            })
            .fold(0.0, f64::max);
        stats.push(t);
        for (k, r) in rows.iter().enumerate() {
            if let (Some(d), Some(o)) = (&r.dist, &diffs[k]) {
                if let Some(c) = d.centred(&weights[r.set], o) {
                    centred[k].push(c);
                }
            }
        }
    }
    // Each whole-distribution row's error: the root mean square of its centred replicates.
    // Its standardized replicate joins each resample's maximum.
    let dist_se: Vec<f64> = centred
        .iter()
        .map(|c| {
            if c.is_empty() {
                0.0
            } else {
                (c.iter().map(|x| x * x).sum::<f64>() / c.len() as f64).sqrt()
            }
        })
        .collect();
    for (k, list) in centred.iter().enumerate() {
        if dist_se[k] > 0.0 && list.len() == stats.len() {
            for (t, c) in stats.iter_mut().zip(list) {
                *t = t.max(c / dist_se[k]);
            }
        }
    }
    stats.sort_by(f64::total_cmp);
    let critical = if stats.is_empty() {
        0.0
    } else {
        // The ceil((B + 1)(1 - alpha))-th smallest: exact for exchangeable resamples.
        let k = ((resamples as f64 + 1.0) * (1.0 - FALSE_ALARM)).ceil() as usize;
        stats[k.clamp(1, stats.len()) - 1]
    }
    .max(one_band());
    let found = observed
        .into_iter()
        .zip(rows)
        .zip(dist_se)
        .map(|((o, r), dse)| Found {
            diff: o.map(|(d, _)| d),
            se: if r.dist.is_some() {
                dse
            } else {
                o.map_or(0.0, |(_, se)| se)
            },
        })
        .collect();
    (found, critical)
}

/// `true` when a change of `diff` with error `se` is beyond the threshold `c`.
pub(crate) fn significant(diff: f64, se: f64, c: f64) -> bool {
    if se > 0.0 {
        diff.abs() > c * se
    } else {
        diff != 0.0
    }
}

/// Matches per suite unit that would give a band of error `se` at `per_unit` matches power
/// to see `shift`: the error falls with the square root of the matches.
pub fn needed(per_unit: u32, se: f64, shift: f64, c: f64) -> u32 {
    let scale = ((c + z_power()) * se / shift).powi(2);
    let n = (f64::from(per_unit.max(1)) * scale).ceil();
    if n >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        n as u32
    }
}

/// `true` when a row is outside its range by more than its noise: its range check fails, and
/// its value is further from the range than `critical` times its own error. Such a band
/// fails while it stays outside, so it sets no power target. A row with no error, or one
/// whose value fails a special range rule while numerically inside, still sets one.
pub fn far_outside(r: &PairedRow, critical: f64) -> bool {
    !r.in_range && r.value_se > 0.0 && r.outside > critical * r.value_se
}

/// A row's name in a suite's no-target list: the band, or `band/label` for a row of one
/// pairing or one arm.
pub fn target_label(band: &str, pairing: Option<&str>) -> String {
    pairing.map_or_else(|| band.to_string(), |p| format!("{band}/{p}"))
}

/// The words of a change run's rows and its joint word. `guards` name failures outside the
/// bands (panics, dark paths, violations); any one fails the run. `no_target` holds, by suite
/// code, the rows that set no power target at the pilot ([`target_label`]): it changes no
/// word, only the marks.
pub fn judge(
    sets: &[SuiteSet],
    rows: &[PairedRow],
    found: &[Found],
    critical: f64,
    guards: Vec<String>,
    no_target: &BTreeMap<String, Vec<String>>,
) -> (Vec<VerdictRow>, Joint) {
    let mut out = Vec::with_capacity(rows.len());
    for (r, f) in rows.iter().zip(found) {
        let per_unit = sets[r.set].per_unit;
        let enough = sets[r.set].strata.len() >= 2;
        let power_se = f.se.max(r.apart_se);
        let power =
            enough && r.spread && f.diff.is_some() && (critical + z_power()) * power_se <= r.shift;
        // A move fails a band when it is beyond the threshold and at least the smallest
        // shift that matters: a smaller move, however certain, is not a change of the band.
        let moved = f
            .diff
            .is_some_and(|d| significant(d, f.se, critical) && d.abs() >= r.shift);
        let word = match (r.value, f.diff) {
            (Some(_), _) if !r.in_range || moved => Word::Fail,
            (Some(_), Some(_)) if power => Word::Pass,
            _ => Word::NotSure,
        };
        let need = (!power && r.spread && power_se > 0.0)
            .then(|| needed(per_unit, power_se, r.shift, critical));
        let suite = r.suite.code();
        let label = target_label(&r.band, r.pairing.as_deref());
        let exempt = no_target.get(suite).is_some_and(|l| l.contains(&label));
        out.push(VerdictRow {
            band: r.band.clone(),
            suite: suite.to_string(),
            pairing: r.pairing.clone(),
            word,
            value: r.value.unwrap_or(0.0),
            lo: r.lo,
            hi: r.hi,
            base_value: r.base_value,
            diff: f.diff.map(|d| round_to(d, 5)),
            se: Some(round_to(f.se, 5)),
            smallest_shift: Some(r.shift),
            power: Some(power),
            needed: need,
            outside_by: (r.outside > 0.0).then(|| round_to(r.outside, 5)),
            no_target: exempt.then_some(true),
            real: None,
            distance: None,
            upper: r
                .dist
                .as_ref()
                .and_then(|d| d.upper)
                .map(|u| round_to(u, 5)),
            noise_floor: r
                .dist
                .as_ref()
                .and_then(|d| d.floor)
                .map(|f| round_to(f, 5)),
            report_only: None,
            real_note: None,
            judged: Some("paired"),
            note: None,
            accepted: None,
            pass_rule: None,
        });
    }
    let word = joint_word(&out, &guards);
    let pairs = sets.iter().map(|s| s.strata.len()).sum::<usize>();
    let joint = Joint {
        word,
        critical: round_to(critical, 4),
        resamples: RESAMPLES,
        false_alarm: FALSE_ALARM,
        pairs: u32::try_from(pairs).unwrap_or(u32::MAX),
        guards,
    };
    (out, joint)
}

/// Sets the word of every row of a version 6 band to its word against real football by the
/// pass rule, from the band's check on the changed engine; the paired test still fills the
/// change columns, and no longer sets the word.
pub fn set_rule_words(rows: &mut [VerdictRow], checks: &[BandCheck]) {
    for r in rows.iter_mut() {
        let found = checks
            .iter()
            .find(|c| c.band == r.band && c.suite == r.suite && c.pairing == r.pairing);
        if let Some(rule) = found.and_then(|c| c.rule.as_ref()) {
            r.word = rule_word(rule.word);
            r.note = Some(match r.note.take() {
                Some(n) => format!("{n}; {}", rule.note()),
                None => rule.note(),
            });
            r.pass_rule = Some(rule.clone());
        }
    }
}

/// One suite's power target and the band that set it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteTarget {
    /// Matches per suite unit: at least the pilot, at most the cap.
    pub matches: u32,
    /// The band that needs the most matches, when it needs more than the pilot; `None`
    /// when the pilot already gives every band power.
    pub driver: Option<String>,
    /// The rows outside their range beyond their noise ([`far_outside`]), which set no
    /// target, by [`target_label`].
    pub no_target: Vec<String>,
}

/// The matches per suite unit each suite needs for every band with a spread to have power,
/// from a judged pilot: at least `pilot`, at most `cap`, with the band that set it. The
/// target depends on the spread alone, never on the change seen. A row outside its range
/// beyond its noise sets no target and is listed instead; a suite whose every row is so
/// keeps the pilot.
pub fn targets(
    sets: &[SuiteSet],
    rows: &[PairedRow],
    found: &[Found],
    critical: f64,
    pilot: u32,
    cap: u32,
) -> BTreeMap<Suite, SuiteTarget> {
    let start = || SuiteTarget {
        matches: pilot.min(cap),
        driver: None,
        no_target: Vec::new(),
    };
    let mut out: BTreeMap<Suite, SuiteTarget> = sets.iter().map(|s| (s.suite, start())).collect();
    // The largest need of each suite so far, before the cap: the band with the largest need
    // is the driver even when several reach the cap.
    let mut most: BTreeMap<Suite, u32> = BTreeMap::new();
    for (r, f) in rows.iter().zip(found) {
        if far_outside(r, critical) {
            let t = out.entry(r.suite).or_insert_with(start);
            t.no_target
                .push(target_label(&r.band, r.pairing.as_deref()));
            continue;
        }
        let power_se = f.se.max(r.apart_se);
        if !r.spread || power_se <= 0.0 {
            continue;
        }
        let need = needed(pilot, power_se, r.shift, critical);
        let n = need.clamp(pilot, cap.max(pilot));
        let t = out.entry(r.suite).or_insert_with(start);
        t.matches = t.matches.max(n.min(cap));
        let top = most.entry(r.suite).or_insert(pilot);
        if need > *top {
            *top = need;
            t.driver = Some(r.band.clone());
        }
    }
    out
}

/// The console table of a run's verdicts, and its joint word when it has one.
pub fn render_table(rows: &[VerdictRow], joint: Option<&Joint>) -> String {
    let mut out = table(rows, joint.is_some());
    if let Some(j) = joint {
        out.push_str(&format!(
            "joint verdict: {} (max-t threshold {} over {} resamples, {} percent false-alarm \
             limit, {} paired matches)\n",
            j.word.code(),
            j.critical,
            j.resamples,
            j.false_alarm * 100.0,
            j.pairs
        ));
        for g in &j.guards {
            out.push_str(&format!("  fails: {g}\n"));
        }
    }
    out
}

/// The console table of the bands that only report, under its title, and its line of
/// counts; nothing when no band only reports. `change` gives the change run's columns.
pub fn render_reported(rows: &[VerdictRow], change: bool) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let count = |w: Word| rows.iter().filter(|r| r.word == w).count();
    let accepted = rows.iter().filter(|r| r.accepted == Some(true)).count();
    format!(
        "reported bands (they never change the verdict):\n{}reported: {} pass, {} fail, {} not \
         sure ({accepted} accepted, not shown equivalent); they never change the verdict\n",
        table(rows, change),
        count(Word::Pass),
        count(Word::Fail),
        count(Word::NotSure)
    )
}

/// The rows of a verdict table, under its header.
fn table(rows: &[VerdictRow], change: bool) -> String {
    let mut out = String::new();
    if change {
        out.push_str(&format!(
            "{:<24} {:<10} {:<14} {:>10} {:>10} {:>10} {:>9} {:>6} {:>19} {:>10}  {}\n",
            "band",
            "suite",
            "pairing",
            "value",
            "old",
            "change",
            "se",
            "power",
            "real",
            "distance",
            "verdict"
        ));
    } else {
        out.push_str(&format!(
            "{:<24} {:<10} {:<14} {:>10} {:>21} {:>19} {:>10}  {}\n",
            "band", "suite", "pairing", "value", "range", "real", "distance", "verdict"
        ));
    }
    for r in rows {
        let pairing = r.pairing.as_deref().unwrap_or("-");
        let mut need = r
            .needed
            .map(|n| format!(" (about {n} matches per suite would give it power)"))
            .unwrap_or_default();
        if r.no_target == Some(true) {
            need.push_str(&format!(
                " (outside its range by {}, beyond its noise at the pilot: set no power target)",
                r.outside_by.unwrap_or(0.0)
            ));
        }
        if let Some(why) = &r.report_only {
            need.push_str(&format!(" (report only: {why})"));
        }
        if let Some(note) = &r.note {
            need.insert_str(0, &format!(" ({note})"));
        }
        if r.accepted == Some(true) {
            need.insert_str(0, " (accepted, not shown equivalent)");
        }
        let real = r.real.map_or_else(
            || r.real_note.unwrap_or("-").to_string(),
            |v| format!("{v}"),
        );
        let distance = r
            .distance
            .map_or_else(|| "-".to_string(), |d| format!("{d}"));
        if change {
            let opt = |v: Option<f64>| v.map_or_else(|| "-".to_string(), |v| format!("{v}"));
            out.push_str(&format!(
                "{:<24} {:<10} {:<14} {:>10} {:>10} {:>10} {:>9} {:>6} {:>19} {:>10}  {}{need}\n",
                r.band,
                r.suite,
                pairing,
                r.value,
                opt(r.base_value),
                opt(r.diff),
                opt(r.se),
                if r.power == Some(true) { "yes" } else { "no" },
                real,
                distance,
                r.word.code()
            ));
        } else {
            out.push_str(&format!(
                "{:<24} {:<10} {:<14} {:>10} {:>21} {:>19} {:>10}  {}{need}\n",
                r.band,
                r.suite,
                pairing,
                r.value,
                format!("{} to {}", r.lo, r.hi),
                real,
                distance,
                r.word.code()
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A paired row from per-match values: a mean (`b` = 1).
    fn row(set: usize, changed: Vec<f64>, old: Vec<f64>, shift: f64) -> PairedRow {
        let n = changed.len();
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let spread = |v: &[f64]| v.iter().any(|x| *x != v[0]);
        PairedRow {
            band: format!("band{set}"),
            suite: Suite::Equal,
            pairing: None,
            lo: -1e9,
            hi: 1e9,
            shift,
            value: Some(mean(&changed)),
            base_value: Some(mean(&old)),
            in_range: true,
            spread: spread(&changed) || spread(&old),
            apart_se: 0.0,
            value_se: 0.0,
            outside: 0.0,
            changed: [changed, vec![1.0; n], vec![1.0; n], vec![1.0; n]],
            old: [old, vec![1.0; n], vec![1.0; n], vec![1.0; n]],
            set,
            dist: None,
        }
    }

    /// A whole-distribution paired row from each pair's draws, binned by 1 over 0 to 9.
    fn dist_row(set: usize, changed: &[Vec<f64>], old: &[Vec<f64>], shift: f64) -> PairedRow {
        use super::super::distribution::{Hist, Metric, bin_of};
        let hist = |draws: &[Vec<f64>]| -> Vec<Hist> {
            draws
                .iter()
                .map(|d| {
                    let mut h: BTreeMap<u32, f64> = BTreeMap::new();
                    for x in d {
                        *h.entry(bin_of(*x, 1.0, 10)).or_default() += 1.0;
                    }
                    h.into_iter().collect()
                })
                .collect()
        };
        let n = changed.len();
        PairedRow {
            band: format!("dist{set}"),
            suite: Suite::Equal,
            pairing: None,
            lo: 0.0,
            hi: 1e9,
            shift,
            value: Some(0.0),
            base_value: Some(0.0),
            in_range: true,
            spread: true,
            apart_se: 0.0,
            value_se: 0.0,
            outside: 0.0,
            changed: std::array::from_fn(|_| vec![0.0; n]),
            old: std::array::from_fn(|_| vec![0.0; n]),
            set,
            dist: Some(Box::new(PairedHist {
                changed: hist(changed),
                old: hist(old),
                metric: Metric::W1,
                width: 1.0,
                bins: 10,
                upper: None,
                floor: None,
            })),
        }
    }

    fn set(n: usize) -> SuiteSet {
        SuiteSet {
            suite: Suite::Equal,
            strata: vec![0; n],
            per_unit: u32::try_from(n).unwrap(),
        }
    }

    /// A deterministic normal draw (Box-Muller) from the engine's stream.
    fn normal(rng: &mut EngineRng) -> f64 {
        let u = rng.next_f64().max(1e-12);
        let v = rng.next_f64();
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }

    #[test]
    fn every_weight_one_gives_the_observed_change_and_the_same_seed_the_same_threshold() {
        let r = row(0, vec![1.0, 2.0, 4.0, 3.0], vec![1.0, 1.0, 2.0, 2.0], 1.0);
        let sets = [set(4)];
        let rows = [r];
        let (found, c) = bootstrap(&sets, &rows, 7, 199);
        assert_eq!(found[0].diff, Some(1.0));
        assert!(found[0].se > 0.0);
        let (again, c2) = bootstrap(&sets, &rows, 7, 199);
        assert_eq!((found, c), (again, c2));
        let (_, c3) = bootstrap(&sets, &rows, 8, 199);
        assert_ne!(c, c3, "another seed resamples differently");
    }

    #[test]
    fn the_word_rules_on_hand_made_inputs() {
        let sets = [set(10)];
        let mk = |in_range, spread| PairedRow {
            in_range,
            spread,
            ..row(0, vec![1.0; 10], vec![1.0; 10], 0.5)
        };
        let f = |diff, se| Found {
            diff: Some(diff),
            se,
        };
        let words = |r: PairedRow, found: Found| {
            judge(&sets, &[r], &[found], 2.0, Vec::new(), &BTreeMap::new()).0[0].word
        };
        // Inside, no move, power: (2 + 0.8416) * 0.1 <= 0.5.
        assert_eq!(words(mk(true, true), f(0.05, 0.1)), Word::Pass);
        // The same without power: never pass.
        assert_eq!(words(mk(true, true), f(0.05, 0.2)), Word::NotSure);
        // A move beyond the threshold and at least the smallest shift, or a value outside
        // the range: fail.
        assert_eq!(words(mk(true, true), f(0.6, 0.1)), Word::Fail);
        assert_eq!(words(mk(true, true), f(-0.5, 0.1)), Word::Fail);
        assert_eq!(words(mk(false, true), f(0.0, 0.1)), Word::Fail);
        // A certain move smaller than the smallest shift is no change of the band.
        assert_eq!(words(mk(true, true), f(0.25, 0.1)), Word::Pass);
        // Zero events in both engines: no spread, so not sure.
        assert_eq!(words(mk(true, false), f(0.0, 0.0)), Word::NotSure);
        // No value at all: not sure.
        let (rows, joint) = judge(
            &sets,
            &[mk(true, true)],
            &[Found {
                diff: None,
                se: 0.0,
            }],
            2.0,
            Vec::new(),
            &BTreeMap::new(),
        );
        assert_eq!(rows[0].word, Word::NotSure);
        assert_eq!(joint.word, Word::NotSure);
        // A guard fails the joint verdict even when every band passes.
        let (_, joint) = judge(
            &sets,
            &[mk(true, true)],
            &[f(0.0, 0.1)],
            2.0,
            vec!["1 match panicked".into()],
            &BTreeMap::new(),
        );
        assert_eq!(joint.word, Word::Fail);
        let (_, joint) = judge(
            &sets,
            &[mk(true, true)],
            &[f(0.0, 0.1)],
            2.0,
            Vec::new(),
            &BTreeMap::new(),
        );
        assert_eq!(joint.word, Word::Pass);
    }

    #[test]
    fn the_power_formula_on_a_hand_case() {
        // se 0.2 at 100 matches, shift 0.5, c 2: (2.8416 * 0.2 / 0.5)^2 = 1.29198... times 100.
        assert_eq!(needed(100, 0.2, 0.5, 2.0), 130);
        // Already powered: fewer than now.
        assert!(needed(100, 0.05, 0.5, 2.0) < 100);
        let sets = [set(100)];
        let rows = [row(0, vec![1.0, 2.0], vec![1.0, 2.0], 0.5)];
        let t = targets(
            &sets,
            &rows,
            &[Found {
                diff: Some(0.0),
                se: 0.2,
            }],
            2.0,
            100,
            120,
        );
        assert_eq!(t[&Suite::Equal].matches, 120, "capped");
        assert_eq!(t[&Suite::Equal].driver.as_deref(), Some("band0"));
        let t = targets(
            &sets,
            &rows,
            &[Found {
                diff: Some(0.0),
                se: 0.01,
            }],
            2.0,
            100,
            1000,
        );
        assert_eq!(t[&Suite::Equal].matches, 100, "never below the pilot");
        assert_eq!(t[&Suite::Equal].driver, None, "the pilot holds: no driver");
    }

    #[test]
    fn the_band_that_needs_the_most_matches_is_named_beside_the_target() {
        let sets = [set(100)];
        let mut tight = row(0, vec![1.0, 2.0], vec![1.0, 2.0], 0.1);
        tight.band = "tight".into();
        let mut loose = row(0, vec![1.0, 2.0], vec![1.0, 2.0], 0.5);
        loose.band = "loose".into();
        let f = Found {
            diff: Some(0.0),
            se: 0.2,
        };
        // Both reach the cap of 1000; the tight band needs more and is named.
        for rows in [[loose.clone(), tight.clone()], [tight, loose]] {
            let t = targets(&sets, &rows, &[f, f], 2.0, 100, 1000);
            assert_eq!(t[&Suite::Equal].matches, 1000);
            assert_eq!(t[&Suite::Equal].driver.as_deref(), Some("tight"));
        }
    }

    /// A row outside its range by `outside`, with its own error `value_se`.
    fn outside_row(band: &str, shift: f64, outside: f64, value_se: f64) -> PairedRow {
        let mut r = row(0, vec![1.0, 2.0], vec![1.0, 2.0], shift);
        r.band = band.into();
        r.in_range = outside == 0.0;
        r.outside = outside;
        r.value_se = value_se;
        r
    }

    #[test]
    fn a_band_far_outside_its_range_sets_no_target_and_the_others_do() {
        let sets = [set(100)];
        let f = Found {
            diff: Some(0.0),
            se: 0.2,
        };
        // The far band needs the most (shift 0.01) but is 1.0 outside with error 0.1, beyond
        // c (2) times it: it sets no target, and the other band sets the target and drives.
        let far = outside_row("far", 0.01, 1.0, 0.1);
        let other = outside_row("other", 0.5, 0.0, 0.1);
        let t = targets(
            &sets,
            &[far.clone(), other.clone()],
            &[f, f],
            2.0,
            100,
            1000,
        );
        let eq = &t[&Suite::Equal];
        assert_eq!(eq.matches, 130, "set by the other band alone");
        assert_eq!(eq.driver.as_deref(), Some("other"));
        assert_eq!(eq.no_target, vec!["far".to_string()]);
        // Outside by less than c times its error (0.15 < 2 * 0.1): it still sets the target.
        let near = outside_row("near", 0.01, 0.15, 0.1);
        let t = targets(&sets, &[near, other.clone()], &[f, f], 2.0, 100, 1000);
        assert_eq!(t[&Suite::Equal].matches, 1000);
        assert_eq!(t[&Suite::Equal].driver.as_deref(), Some("near"));
        assert!(t[&Suite::Equal].no_target.is_empty());
        // Outside with no error of its own: still sets the target.
        let blind = outside_row("blind", 0.01, 1.0, 0.0);
        let t = targets(&sets, &[blind, other], &[f, f], 2.0, 100, 1000);
        assert_eq!(t[&Suite::Equal].driver.as_deref(), Some("blind"));
        assert!(t[&Suite::Equal].no_target.is_empty());
        // A suite whose only row is far outside keeps the pilot, with no driver.
        let t = targets(&sets, &[far], &[f], 2.0, 100, 1000);
        assert_eq!(t[&Suite::Equal].matches, 100);
        assert_eq!(t[&Suite::Equal].driver, None);
        assert_eq!(t[&Suite::Equal].no_target, vec!["far".to_string()]);
        // A row of one pairing or arm is listed as band/label.
        let mut armed = outside_row("far", 0.01, 1.0, 0.1);
        armed.pairing = Some("reduced".into());
        let t = targets(&sets, &[armed], &[f], 2.0, 100, 1000);
        assert_eq!(t[&Suite::Equal].no_target, vec!["far/reduced".to_string()]);
    }

    #[test]
    fn a_no_target_band_is_judged_as_any_band_and_never_passes_without_power() {
        let sets = [set(100)];
        let no_target: BTreeMap<String, Vec<String>> =
            [("equal".to_string(), vec!["far".to_string()])].into();
        let far = outside_row("far", 0.01, 1.234_567, 0.1);
        let (rows, joint) = judge(
            &sets,
            std::slice::from_ref(&far),
            &[Found {
                diff: Some(0.0),
                se: 0.2,
            }],
            2.0,
            Vec::new(),
            &no_target,
        );
        assert_eq!(rows[0].word, Word::Fail, "outside its range: fail");
        assert_eq!(rows[0].no_target, Some(true));
        assert_eq!(rows[0].outside_by, Some(1.23457));
        assert_eq!(joint.word, Word::Fail);
        let table = render_table(&rows, Some(&joint));
        assert!(
            table.contains(
                "outside its range by 1.23457, beyond its noise at the pilot: set no power target"
            ),
            "{table}"
        );
        // Listed but back inside its range and without power: not sure, never pass.
        let inside = PairedRow {
            in_range: true,
            outside: 0.0,
            ..far
        };
        let (rows, _) = judge(
            &sets,
            &[inside],
            &[Found {
                diff: Some(0.0),
                se: 0.2,
            }],
            2.0,
            Vec::new(),
            &no_target,
        );
        assert_eq!(rows[0].word, Word::NotSure);
        assert_eq!(rows[0].no_target, Some(true));
        assert_eq!(rows[0].outside_by, None);
        // A row not listed carries no mark.
        let (rows, _) = judge(
            &sets,
            &[outside_row("other", 0.5, 0.0, 0.1)],
            &[Found {
                diff: Some(0.0),
                se: 0.01,
            }],
            2.0,
            Vec::new(),
            &no_target,
        );
        assert_eq!((rows[0].no_target, rows[0].outside_by), (None, None));
    }

    /// An A/A comparison: both arms draw from the same correlated distribution on each
    /// fixture, so every change is noise. Over 1000 replicates of 150 paired matches and six
    /// correlated measures (means, a share, and a pooled ratio), the max-t test flags at
    /// most 3 percent of replicates, and every band is still reported on its own row.
    #[test]
    fn an_a_a_comparison_flags_at_most_three_percent_of_replicates() {
        let (replicates, n, resamples) = (1000u32, 150, 199);
        let mut rng = EngineRng::from_seed(2026);
        let mut alarms = 0;
        for rep in 0..replicates {
            // Six measures with a shared match factor (correlation about 0.5), and three
            // draws of a whole distribution from the same factor.
            let mut arm = || -> Vec<[f64; 9]> {
                (0..n)
                    .map(|_| {
                        let common = normal(&mut rng);
                        let x: [f64; 9] = std::array::from_fn(|_| common + normal(&mut rng));
                        x
                    })
                    .collect()
            };
            let (a, b) = (arm(), arm());
            let sets = [set(n)];
            let mut rows = Vec::new();
            for k in 0..4 {
                rows.push(row(
                    0,
                    a.iter().map(|m| m[k] + 3.0).collect(),
                    b.iter().map(|m| m[k] + 3.0).collect(),
                    0.5,
                ));
            }
            // A share: the fifth measure above 1.
            let hit = |v: f64| f64::from(u8::from(v > 1.0));
            rows.push(row(
                0,
                a.iter().map(|m| hit(m[4])).collect(),
                b.iter().map(|m| hit(m[4])).collect(),
                0.1,
            ));
            // A pooled ratio of the sixth measure's square over 1 + its absolute value.
            let mut ratio = row(0, vec![0.0; n], vec![0.0; n], 0.1);
            ratio.changed[0] = a.iter().map(|m| m[5] * m[5]).collect();
            ratio.changed[1] = a.iter().map(|m| 1.0 + m[5].abs()).collect();
            ratio.old[0] = b.iter().map(|m| m[5] * m[5]).collect();
            ratio.old[1] = b.iter().map(|m| 1.0 + m[5].abs()).collect();
            rows.push(ratio);
            // A whole distribution: each match's three draws, about 4.5 apart from 0.
            let draws = |arm: &[[f64; 9]]| -> Vec<Vec<f64>> {
                arm.iter()
                    .map(|m| m[6..].iter().map(|x| 4.5 + 1.5 * x).collect())
                    .collect()
            };
            rows.push(dist_row(0, &draws(&a), &draws(&b), 0.5));
            let (found, c) = bootstrap(&sets, &rows, 1000 + u64::from(rep), resamples);
            let (judged, _) = judge(&sets, &rows, &found, c, Vec::new(), &BTreeMap::new());
            assert_eq!(judged.len(), 7, "every band on its own row");
            if found.iter().any(|f| significant(f.diff.unwrap(), f.se, c)) {
                alarms += 1;
            }
        }
        let rate = f64::from(alarms) / f64::from(replicates);
        eprintln!("A/A rate {rate} over {replicates} replicates of {n} pairs");
        assert!(rate <= FALSE_ALARM, "joint false-alarm rate {rate}");
    }

    /// The A/A comparison with the whole-distribution coordinate alone: at most 3 percent of
    /// replicates flag it.
    #[test]
    fn an_a_a_comparison_of_a_whole_distribution_alone_flags_at_most_three_percent() {
        let (replicates, n, resamples) = (1000u32, 150, 199);
        let mut rng = EngineRng::from_seed(2027);
        let mut alarms = 0;
        for rep in 0..replicates {
            let mut arm = || -> Vec<Vec<f64>> {
                (0..n)
                    .map(|_| {
                        let common = normal(&mut rng);
                        (0..3)
                            .map(|_| 4.5 + 1.5 * (common + normal(&mut rng)))
                            .collect()
                    })
                    .collect()
            };
            let (a, b) = (arm(), arm());
            let sets = [set(n)];
            let rows = [dist_row(0, &a, &b, 0.5)];
            let (found, c) = bootstrap(&sets, &rows, 5000 + u64::from(rep), resamples);
            assert!(found[0].se > 0.0);
            if significant(found[0].diff.unwrap(), found[0].se, c) {
                alarms += 1;
            }
        }
        let rate = f64::from(alarms) / f64::from(replicates);
        eprintln!("A/A rate of the distribution alone {rate}");
        assert!(rate <= FALSE_ALARM, "false-alarm rate {rate}");
    }

    /// A whole distribution moved by twice its smallest shift fails the joint word of a
    /// family of it and one unchanged scalar band; the coordinate draws no random number, so
    /// the scalar band's change and error are the same with it or without it, and the
    /// threshold never falls.
    #[test]
    fn a_shifted_distribution_fails_the_joint_word_and_leaves_the_scalar_draws() {
        let mut rng = EngineRng::from_seed(9);
        let n = 300;
        let base: Vec<Vec<f64>> = (0..n)
            .map(|_| (0..3).map(|_| 4.0 + 1.5 * normal(&mut rng)).collect())
            .collect();
        let moved: Vec<Vec<f64>> = base
            .iter()
            .map(|d| d.iter().map(|x| x + 1.0).collect())
            .collect();
        let scalar_c: Vec<f64> = (0..n).map(|_| 5.0 + normal(&mut rng)).collect();
        let scalar_o: Vec<f64> = scalar_c
            .iter()
            .map(|x| x + 0.01 * normal(&mut rng))
            .collect();
        let sets = [set(n)];
        let scalar = row(0, scalar_c, scalar_o, 0.5);
        let shifted = dist_row(0, &moved, &base, 0.5);
        let rows = [scalar.clone(), shifted];
        let (found, c) = bootstrap(&sets, &rows, 13, 499);
        let (judged, joint) = judge(&sets, &rows, &found, c, Vec::new(), &BTreeMap::new());
        assert!(found[1].diff.unwrap() >= 2.0 * 0.5 - 0.15, "{:?}", found[1]);
        assert_eq!(judged[1].word, Word::Fail, "{:?} c {c}", found[1]);
        assert_eq!(joint.word, Word::Fail);
        // The scalar row alone: the same change and error, and a threshold no higher.
        let (alone, c_alone) = bootstrap(&sets, std::slice::from_ref(&scalar), 13, 499);
        assert_eq!(alone[0], found[0]);
        assert!(c >= c_alone);
        // An unmoved distribution: the same draws, so its change is 0.
        let still = dist_row(0, &base, &base, 0.5);
        let (f, _) = bootstrap(&sets, &[scalar, still], 13, 499);
        assert_eq!(f[1].diff, Some(0.0));
    }

    /// A match of the equal suite with these goals; `played` false makes it a failed match.
    fn stats(goals: [u32; 2], played: bool) -> engine::observe::MatchStats {
        use engine::observe::{MatchFigures, MatchStats, TeamRef};
        let team = |id: &str| TeamRef {
            id: id.into(),
            name: id.into(),
        };
        MatchStats {
            owner_id: "0123456789abcdef0123456789abcdef".into(),
            match_id: "0000000000000001-1".into(),
            seed: 1,
            content_hash: "abcdef012345".into(),
            teams: [team("a"), team("b")],
            duration_ms: 400,
            outcome: if played { "success" } else { "error" }.into(),
            ticks_per_s: 1.0,
            ticks_written: 1,
            validate_ran: true,
            validate_violations: 0,
            possession_changes: 0,
            ball_max_speed: 0.0,
            ball_idle_ticks: 0,
            goals,
            flags_on: Vec::new(),
            laws: Default::default(),
            tactics: Default::default(),
            figures: MatchFigures {
                goals,
                ..MatchFigures::default()
            },
            script: Default::default(),
            ratings: Vec::new(),
        }
    }

    #[test]
    fn a_pair_is_kept_only_when_both_engines_played_its_fixture() {
        let dir = engine::ContentDir::at(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        );
        let registry = || super::super::bands::Registry::load(&dir).unwrap();
        // Four fixtures. The old engine failed fixture 30 and the changed engine failed
        // fixture 40: only fixtures 10 and 20 were played by both.
        let goals = [[1, 0], [2, 1], [5, 4], [0, 0]];
        let mut cb = RunBuilder::new(registry());
        let mut ob = RunBuilder::new(registry());
        cb.plan(Suite::Equal, 4);
        ob.plan(Suite::Equal, 4);
        for (i, g) in goals.iter().enumerate() {
            cb.add(Suite::Equal, stats(*g, i != 3), None);
            ob.add(Suite::Equal, stats(*g, i != 2), None);
        }
        let keys: BTreeMap<Suite, Vec<u64>> = [(Suite::Equal, vec![10, 20, 30, 40])].into();
        let per_unit: BTreeMap<Suite, u32> = [(Suite::Equal, 4)].into();
        let (sets, rows) = paired((&cb, &keys), (&ob, &keys), &per_unit);
        assert_eq!(
            sets[0].strata.len(),
            2,
            "only the fixtures both engines played"
        );
        let goals_row = rows
            .iter()
            .find(|r| r.band == "goals_per_match")
            .expect("the equal suite judges goals per match");
        // Both sides over fixtures 10 and 20: (1 + 3) / 2 goals per match.
        assert_eq!(goals_row.value, Some(2.0));
        assert_eq!(goals_row.base_value, Some(2.0));
        assert_eq!(goals_row.changed[0].len(), 2);
        assert_eq!(goals_row.old[0].len(), 2);
        let (found, _) = bootstrap(&sets, &rows, 3, 19);
        let at = rows
            .iter()
            .position(|r| r.band == "goals_per_match")
            .unwrap();
        assert_eq!(found[at].diff, Some(0.0), "the same fixtures on both sides");
        let (_, joint) = judge(&sets, &rows, &found, 2.0, Vec::new(), &BTreeMap::new());
        assert_eq!(joint.pairs, 2);
    }

    #[test]
    fn a_shift_beyond_the_smallest_is_seen_and_fails_the_joint_verdict() {
        let mut rng = EngineRng::from_seed(5);
        let n = 300;
        let base: Vec<f64> = (0..n).map(|_| 5.0 + normal(&mut rng)).collect();
        let noise: Vec<f64> = (0..n).map(|_| 0.3 * normal(&mut rng)).collect();
        let changed: Vec<f64> = base.iter().zip(&noise).map(|(b, e)| b + 0.8 + e).collect();
        let same: Vec<f64> = base.iter().zip(&noise).map(|(b, e)| b + e).collect();
        let sets = [set(n)];
        let rows = [row(0, changed, base.clone(), 0.5), row(0, same, base, 0.5)];
        let (found, c) = bootstrap(&sets, &rows, 11, 499);
        let (judged, joint) = judge(&sets, &rows, &found, c, Vec::new(), &BTreeMap::new());
        assert_eq!(judged[0].word, Word::Fail);
        assert_eq!(judged[1].word, Word::Pass);
        assert_eq!(joint.word, Word::Fail);
        // The same rows when the engines' matches would be apart: an error of 0.2 each
        // leaves no power at (c + z) * 0.28 > 0.5, so the unchanged band is not sure.
        let apart: Vec<PairedRow> = rows
            .iter()
            .map(|r| PairedRow {
                apart_se: 0.2f64.hypot(0.2),
                ..r.clone()
            })
            .collect();
        let (judged, _) = judge(&sets, &apart, &found, c, Vec::new(), &BTreeMap::new());
        assert_eq!(judged[0].word, Word::Fail, "a move is still a move");
        assert_eq!(judged[1].word, Word::NotSure);
        assert!(judged[1].needed.unwrap() > 300);
    }

    /// A band check of `value` in 1 to 3 with its own error `se`.
    fn check(value: f64, se: f64) -> BandCheck {
        BandCheck {
            band: "b".into(),
            suite: "equal".into(),
            pairing: None,
            value,
            lo: 1.0,
            hi: 3.0,
            pass: (1.0..=3.0).contains(&value),
            se,
            informational: None,
            why_none: None,
            unjudged: false,
            whole: false,
            dist: None,
            rule: None,
        }
    }

    #[test]
    fn a_row_takes_the_pass_rule_word_and_a_row_without_it_is_not_sure() {
        // No fixed range decides: inside the range is not a pass without the rule's word.
        assert_eq!(range_word(&check(2.0, 0.1)), Word::NotSure);
        assert_eq!(range_word(&check(3.1, 0.1)), Word::NotSure);
        for w in [
            pass_rule::rule::Word::Pass,
            pass_rule::rule::Word::NotSure,
            pass_rule::rule::Word::Fail,
        ] {
            let judged = BandCheck {
                rule: Some(rule(w)),
                ..check(0.0, 0.1)
            };
            assert_eq!(range_word(&judged), rule_word(w));
        }
        // No value, or no measures: not sure, with the reason as the note.
        let none = BandCheck {
            why_none: Some("no club has 34 matches in this run".into()),
            ..check(0.0, 0.0)
        };
        let rows = range_rows(&[none], Some(PER_CLUB));
        assert_eq!(rows[0].word, Word::NotSure);
        assert_eq!(
            rows[0].note.as_deref(),
            Some("no value: no club has 34 matches in this run")
        );
        let unjudged = BandCheck {
            unjudged: true,
            pass: false,
            ..check(2.0, 0.1)
        };
        assert_eq!(
            range_rows(std::slice::from_ref(&unjudged), None)[0].word,
            Word::NotSure
        );
        let plain = range_only(&[unjudged]);
        assert_eq!(plain[0].word, Word::NotSure);
        assert_eq!(plain[0].note.as_deref(), Some(NO_MEASURES_NOTE));
        // A row with its own note keeps the run's note otherwise.
        let rows = range_rows(&[check(2.0, 0.1)], Some(BASE_WITHOUT_MEASURES));
        assert_eq!(rows[0].judged, Some("range"));
        assert_eq!(rows[0].note.as_deref(), Some(BASE_WITHOUT_MEASURES));
        assert_eq!(rows[0].se, Some(0.1));
    }

    #[test]
    fn the_reported_table_counts_an_accepted_row_apart_and_never_moves_the_joint_word() {
        let sets = [set(10)];
        let wave: Vec<f64> = (0..10).map(f64::from).collect();
        let gating = row(0, wave.clone(), wave, 0.5);
        let found = [Found {
            diff: Some(0.0),
            se: 0.1,
        }];
        let (rows, joint) = judge(&sets, &[gating], &found, 2.0, Vec::new(), &BTreeMap::new());
        assert_eq!(joint.word, Word::Pass);
        // Reported rows of any word leave the gating word as it is.
        let ruled = |value: f64, w: pass_rule::rule::Word| BandCheck {
            rule: Some(rule(w)),
            ..check(value, 0.1)
        };
        let mut reported = range_rows(
            &[
                ruled(2.0, pass_rule::rule::Word::Pass),
                ruled(5.0, pass_rule::rule::Word::Fail),
                ruled(1.1, pass_rule::rule::Word::NotSure),
            ],
            None,
        );
        assert_eq!(joint_word(&rows, &joint.guards), Word::Pass);
        reported[2].accepted = Some(true);
        let text = render_reported(&reported, true);
        assert!(
            text.starts_with("reported bands (they never change the verdict):"),
            "{text}"
        );
        assert!(
            text.contains(
                "reported: 1 pass, 1 fail, 1 not sure (1 accepted, not shown equivalent); \
                 they never change the verdict"
            ),
            "{text}"
        );
        assert!(
            text.contains("not sure (accepted, not shown equivalent)"),
            "{text}"
        );
        assert_eq!(render_reported(&[], false), "");
    }

    /// A range row near an edge reads not sure and still lies inside its range; a row with
    /// no value or outside the range never does.
    #[test]
    fn a_not_sure_range_row_near_an_edge_is_inside_and_one_with_no_value_is_not() {
        let rows = range_rows(&[check(1.2, 0.1), check(3.1, 0.1), check(2.0, 0.1)], None);
        assert_eq!(rows[0].word, Word::NotSure);
        assert!(rows[0].inside());
        assert!(!rows[1].inside());
        assert!(rows[2].inside());
        let none = BandCheck {
            why_none: Some("no real data here".into()),
            ..check(2.0, 0.0)
        };
        assert!(!range_rows(&[none], None)[0].inside());
    }

    /// A pass-rule word of `word` with the interval -0.2 to 0.1 inside the margin 0.3.
    fn rule(word: pass_rule::rule::Word) -> RuleWord {
        RuleWord {
            word,
            lo: Some(-0.2),
            hi: Some(0.1),
            point: Some(-0.05),
            margin: 0.3,
            target: "real".into(),
            cap: None,
            exposure: 1000.0,
            min_exposure: 100.0,
            units: 3,
            reason: None,
        }
    }

    #[test]
    fn a_version_6_band_takes_the_pass_rule_word_over_its_range() {
        use pass_rule::rule::Word as Rule;
        // Outside its range, yet inside its margin against real football: a pass.
        let outside = BandCheck {
            rule: Some(rule(Rule::Pass)),
            ..check(3.5, 0.1)
        };
        assert_eq!(range_word(&outside), Word::Pass);
        let rows = range_only(std::slice::from_ref(&outside));
        assert_eq!(rows[0].word, Word::Pass);
        assert_eq!(rows[0].pass_rule.as_ref().map(|r| r.word), Some(Rule::Pass));
        assert!(
            rows[0]
                .note
                .as_deref()
                .unwrap()
                .contains("against margin 0.3")
        );
        // Inside its range, but not shown inside its margin: not sure, and the joint word
        // reads that.
        let inside = BandCheck {
            rule: Some(rule(Rule::NotSure)),
            ..check(2.0, 0.1)
        };
        let rows = range_only(std::slice::from_ref(&inside));
        assert_eq!(rows[0].word, Word::NotSure);
        assert_eq!(joint_word(&rows, &[]), Word::NotSure);
        // A change run's rows take the rule word of the changed engine's check.
        let passing = BandCheck {
            rule: Some(rule(Rule::Pass)),
            ..check(2.0, 0.1)
        };
        let mut rows = range_rows(&[passing], Some(PER_CLUB));
        assert_eq!(rows[0].word, Word::Pass);
        let failing = BandCheck {
            rule: Some(rule(Rule::Fail)),
            ..check(2.0, 0.1)
        };
        set_rule_words(&mut rows, &[failing]);
        assert_eq!(rows[0].word, Word::Fail);
        assert!(rows[0].pass_rule.is_some());
    }

    #[test]
    fn the_quantiles_are_the_constants_they_replace() {
        assert!((z_power() - 0.8416).abs() < 1e-4);
        assert!((one_band() - 2.1701).abs() < 1e-4);
    }
}

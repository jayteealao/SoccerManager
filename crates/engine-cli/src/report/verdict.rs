//! The verdict of a change run: one paired max-t bootstrap over the matches both engines
//! played on the same fixtures, two words per band, and one joint word.
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
//! A band's word against real football is the pass rule's, on the changed engine: pass
//! when the whole interval of engine minus real sits inside its margin, fail when wholly
//! outside, not sure otherwise. Only that word feeds the joint word. Its word against the
//! old engine ([`pass_rule::change::old_word`]) crosses the difference test, the change
//! beyond `c` times its error, with the equivalence test, the change and its one-sided
//! error inside the band's noticeable error: better, worse, equivalent or not sure. A run
//! plays one fixed count of paired matches set in the band file for its kind; nothing here
//! sizes it.

use std::collections::BTreeMap;

use engine::observe::round_to;
use engine::rng::EngineRng;
use serde::Serialize;

use super::distribution::PairedHist;
use super::measures;
use super::{BandCheck, RuleWord, RunBuilder, Suite};
pub use pass_rule::change::{Moved, OldWord};

/// Resamples of the bootstrap.
pub const RESAMPLES: usize = 1999;
/// The joint false-alarm limit of a run.
pub const FALSE_ALARM: f64 = 0.03;

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
    /// The band's noticeable error: the smallest change a player would notice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smallest_shift: Option<f64>,
    /// A change run: how far the changed engine's value is outside its range, when it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outside_by: Option<f64>,
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
    /// A change or pair run: the band's word against the old engine (better, worse,
    /// equivalent or not sure). It says what the change moved and never decides the run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_word: Option<OldWord>,
    /// Why the band has no word against the old engine, or why the word is not sure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_reason: Option<String>,
    /// A significant move against the old engine: up or down.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moved: Option<Moved>,
}

impl VerdictRow {
    /// The run's value is inside the design range: a change row is inside when it carries no
    /// distance outside; a row judged on its range alone when it has a value and that value
    /// lies in the range, a not-sure word near an edge included.
    pub fn inside(&self) -> bool {
        if self.judged == Some("paired") {
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
    /// Why the joint word is not sure although no band says so: too many pairs dropped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The share of planned pairs a run may drop and still be judged on the pairs it kept.
pub const DROPPED_LIMIT: f64 = 0.01;
/// The reason of a joint word that more dropped pairs than [`DROPPED_LIMIT`] made not sure.
pub const UNDER_COUNT: &str = "pairs under the fixed count";

/// The joint word of a run that dropped `dropped` of `planned` pairs: a pass of a run that
/// dropped more than [`DROPPED_LIMIT`] of its pairs is not sure, with [`UNDER_COUNT`] as its
/// reason; a fail stays a fail. At or under the limit the run is judged on its kept pairs.
pub fn under_count(joint: &mut Joint, planned: u32, dropped: u32) {
    if planned > 0 && f64::from(dropped) > DROPPED_LIMIT * f64::from(planned) {
        if joint.word == Word::Pass {
            joint.word = Word::NotSure;
        }
        if joint.word == Word::NotSure {
            joint.reason = Some(format!(
                "{UNDER_COUNT}: {dropped} of {planned} planned pairs dropped"
            ));
        }
    }
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
        outside_by: None,
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
        old_word: None,
        old_reason: None,
        moved: None,
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
    /// The changed engine's value, rounded as its band check shows it.
    pub value: Option<f64>,
    pub base_value: Option<f64>,
    /// How far the changed engine's unrounded value is outside its range; 0 when its range
    /// check holds or it has no value.
    pub outside: f64,
    /// Index of the row's suite set.
    pub set: usize,
    /// A whole-distribution band: both engines' histograms per pair. Its per-match sums
    /// above are 0, and its change is the distance between the two engines.
    pub dist: Option<Box<PairedHist>>,
}

/// The pairs of one suite: each pair's stratum (its formation pairing or red-card arm).
#[derive(Debug, Clone)]
pub struct SuiteSet {
    pub strata: Vec<u32>,
}

/// The paired rows of a change run: for every suite both builders planned, the fixtures
/// both played, joined on key. `keys` give each builder's matches' fixture keys, suite by
/// suite, in the order the builder holds them.
pub fn paired(
    changed: (&RunBuilder, &BTreeMap<Suite, Vec<u64>>),
    old: (&RunBuilder, &BTreeMap<Suite, Vec<u64>>),
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
        sets.push(SuiteSet { strata });
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
                outside,
                changed: tc.per_match(),
                old: to.per_match(),
                value,
                base_value: to.value().map(|v| round_to(v, places)),
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
        // A difference band's numerator sum can be negative: only a zero sum has no term.
        let term = |k: usize| if s[k] != 0.0 { t[k][i] / s[k] } else { 0.0 };
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

/// The rows of a change run with their change columns, and its joint word. Each row's word
/// is not sure here: [`set_rule_words`] sets its word against real football, and
/// [`set_old_words`] its word against the old engine. `guards` name failures outside the
/// bands (panics, dark paths, violations); any one fails the run.
pub fn judge(
    sets: &[SuiteSet],
    rows: &[PairedRow],
    found: &[Found],
    critical: f64,
    guards: Vec<String>,
) -> (Vec<VerdictRow>, Joint) {
    let mut out = Vec::with_capacity(rows.len());
    for (r, f) in rows.iter().zip(found) {
        let word = Word::NotSure;
        let suite = r.suite.code();
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
            outside_by: (r.outside > 0.0).then(|| round_to(r.outside, 5)),
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
            old_word: None,
            old_reason: None,
            moved: None,
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
        reason: None,
    };
    (out, joint)
}

/// Sets each paired row's word against the old engine ([`pass_rule::change::old_word`]):
/// the change and its error from the bootstrap, the joint threshold `critical`, the band's
/// noticeable error (its smallest shift) and the pass rule's `alpha`. Better means the
/// changed engine sits closer to the band's target, which `target` gives by band (the real
/// value read here, or the design value); a whole-distribution band is closer when its
/// distance from the real sample is smaller. The word never changes the row's word.
pub fn set_old_words(
    rows: &mut [VerdictRow],
    paired: &[PairedRow],
    found: &[Found],
    critical: f64,
    alpha: f64,
    target: &dyn Fn(&str) -> Option<f64>,
) {
    for ((r, p), f) in rows.iter_mut().zip(paired).zip(found) {
        let Some(diff) = f.diff else {
            r.old_reason = Some("no value on one of the engines".into());
            continue;
        };
        let (changed, old) = (p.value, p.base_value);
        let toward = match (&p.dist, changed, old) {
            (Some(_), Some(c), Some(o)) => pass_rule::change::toward(c, o, Some(0.0)),
            (None, Some(c), Some(o)) => pass_rule::change::toward(c, o, target(&p.band)),
            _ => None,
        };
        let (word, moved) =
            pass_rule::change::old_word(diff, f.se, critical, p.shift, alpha, toward);
        // A distribution's change is a distance, never negative: its direction is whether
        // the changed engine sits farther from the real sample than the old one.
        let moved = match (&p.dist, moved, changed, old) {
            (Some(_), Some(_), Some(c), Some(o)) => {
                Some(if c > o { Moved::Up } else { Moved::Down })
            }
            (_, m, _, _) => m,
        };
        r.old_word = Some(word);
        r.moved = moved;
        r.old_reason = match word {
            OldWord::NotSure if moved.is_some() => {
                Some("moved; no target here to say better or worse".into())
            }
            OldWord::NotSure => {
                Some("not shown inside the noticeable error, and no move seen".into())
            }
            _ => None,
        };
    }
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
        if let Some(r) = &j.reason {
            out.push_str(&format!("  not sure: {r}\n"));
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
            "{:<24} {:<10} {:<14} {:>10} {:>10} {:>10} {:>9} {:>10} {:>19} {:>10}  {}\n",
            "band",
            "suite",
            "pairing",
            "value",
            "old",
            "change",
            "se",
            "old engine",
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
        let mut need = String::new();
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
                "{:<24} {:<10} {:<14} {:>10} {:>10} {:>10} {:>9} {:>10} {:>19} {:>10}  {}{need}\n",
                r.band,
                r.suite,
                pairing,
                r.value,
                opt(r.base_value),
                opt(r.diff),
                opt(r.se),
                r.old_word.map_or("-", OldWord::as_str),
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
                range_cell(r.lo, r.hi),
                real,
                distance,
                r.word.code()
            ));
        }
    }
    out
}

/// The range column of a plain table: a band judged by its margin around a target has no
/// fixed range, and an open end is left out rather than printed as the largest number.
fn range_cell(lo: f64, hi: f64) -> String {
    let bounded = |v: f64| v.is_finite() && v.abs() < f64::MAX;
    match (bounded(lo), bounded(hi)) {
        (true, true) => format!("{lo} to {hi}"),
        (true, false) => format!("from {lo}"),
        (false, true) => format!("up to {hi}"),
        (false, false) => "-".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn more_than_one_percent_of_pairs_dropped_is_never_a_pass() {
        let joint = |word| Joint {
            word,
            critical: 2.5,
            resamples: 10,
            false_alarm: FALSE_ALARM,
            pairs: 0,
            guards: Vec::new(),
            reason: None,
        };
        // At the limit the run is judged on its kept pairs.
        let mut j = joint(Word::Pass);
        under_count(&mut j, 1000, 10);
        assert_eq!((j.word, j.reason.as_deref()), (Word::Pass, None));
        let mut j = joint(Word::Pass);
        under_count(&mut j, 1000, 11);
        assert_eq!(j.word, Word::NotSure);
        assert_eq!(
            j.reason.as_deref(),
            Some("pairs under the fixed count: 11 of 1000 planned pairs dropped")
        );
        let mut j = joint(Word::Fail);
        under_count(&mut j, 1000, 500);
        assert_eq!((j.word, j.reason), (Word::Fail, None));
    }

    #[test]
    fn an_open_range_prints_no_largest_number() {
        assert_eq!(range_cell(1.0, 2.5), "1 to 2.5");
        assert_eq!(range_cell(-f64::MAX, f64::MAX), "-");
        assert_eq!(range_cell(0.0, f64::MAX), "from 0");
        assert_eq!(range_cell(f64::NEG_INFINITY, 3.0), "up to 3");
    }

    /// A paired row from per-match values: a mean (`b` = 1).
    fn row(set: usize, changed: Vec<f64>, old: Vec<f64>, shift: f64) -> PairedRow {
        let n = changed.len();
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        PairedRow {
            band: format!("band{set}"),
            suite: Suite::Equal,
            pairing: None,
            lo: -1e9,
            hi: 1e9,
            shift,
            value: Some(mean(&changed)),
            base_value: Some(mean(&old)),
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
        SuiteSet { strata: vec![0; n] }
    }

    /// A band whose values sum below zero, as a home-minus-away difference can, has the
    /// same paired error as the same values moved above zero: its error is never zero.
    #[test]
    fn a_negative_sum_keeps_its_paired_error() {
        let changed = vec![-1.0, -3.0, 0.0, -2.0, -4.0, -1.0];
        let old = vec![-2.0, -1.0, -1.0, -2.0, -2.0, 0.0];
        let up = |v: &[f64]| v.iter().map(|x| x + 10.0).collect::<Vec<f64>>();
        let strata = BTreeMap::from([(0u32, (0..6).collect::<Vec<usize>>())]);
        let ones = vec![1u32; 6];
        let (d, se) = change(&row(0, changed.clone(), old.clone(), 1.0), &ones, &strata).unwrap();
        let (d_up, se_up) = change(&row(0, up(&changed), up(&old), 1.0), &ones, &strata).unwrap();
        assert!(se > 0.0, "{se}");
        assert!((d - d_up).abs() < 1e-12, "{d} {d_up}");
        assert!((se - se_up).abs() < 1e-12, "{se} {se_up}");
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
            let (judged, _) = judge(&sets, &rows, &found, c, Vec::new());
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

    /// A whole distribution moved by twice its smallest shift is a move against the old
    /// engine, and the unchanged scalar band beside it is equivalent; the coordinate draws no
    /// random number, so the scalar band's change and error are the same with it or without
    /// it, and the threshold never falls.
    #[test]
    fn a_shifted_distribution_moves_against_the_old_engine_and_leaves_the_scalar_draws() {
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
        let (mut judged, _) = judge(&sets, &rows, &found, c, Vec::new());
        set_old_words(&mut judged, &rows, &found, c, 0.05, &|_| None);
        assert!(found[1].diff.unwrap() >= 2.0 * 0.5 - 0.15, "{:?}", found[1]);
        assert!(judged[1].moved.is_some(), "{:?} c {c}", found[1]);
        assert_ne!(judged[1].old_word, Some(OldWord::Equivalent));
        assert_eq!(judged[0].old_word, Some(OldWord::Equivalent));
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
        let (sets, rows) = paired((&cb, &keys), (&ob, &keys));
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
        let (_, joint) = judge(&sets, &rows, &found, 2.0, Vec::new());
        assert_eq!(joint.pairs, 2);
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
        let (mut rows, joint) = judge(&sets, &[gating], &found, 2.0, Vec::new());
        rows[0].word = Word::Pass;
        assert_eq!(joint_word(&rows, &joint.guards), Word::Pass);
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
    fn a_band_worse_against_the_old_engine_leaves_the_joint_word_to_real_football() {
        let mut rng = EngineRng::from_seed(21);
        let n = 300;
        let base: Vec<f64> = (0..n).map(|_| 5.0 + normal(&mut rng)).collect();
        let noise: Vec<f64> = (0..n).map(|_| 0.2 * normal(&mut rng)).collect();
        // Moved up by 1 from the old engine, away from a target of 5.
        let worse: Vec<f64> = base.iter().zip(&noise).map(|(b, e)| b + 1.0 + e).collect();
        // Moved down by 1 from 6.5, toward the same target.
        let old_high: Vec<f64> = base.iter().map(|b| b + 1.5).collect();
        let better: Vec<f64> = base.iter().zip(&noise).map(|(b, e)| b + 0.5 + e).collect();
        let same: Vec<f64> = base.iter().zip(&noise).map(|(b, e)| b + 0.01 * e).collect();
        let sets = [set(n)];
        let rows = [
            row(0, worse, base.clone(), 0.5),
            row(0, better, old_high, 0.5),
            row(0, same, base, 0.5),
        ];
        let (found, c) = bootstrap(&sets, &rows, 11, 499);
        let (mut judged, joint) = judge(&sets, &rows, &found, c, Vec::new());
        set_old_words(&mut judged, &rows, &found, c, 0.05, &|_| Some(5.0));
        assert_eq!(judged[0].old_word, Some(OldWord::Worse));
        assert_eq!(judged[0].moved, Some(Moved::Up));
        assert_eq!(judged[1].old_word, Some(OldWord::Better));
        assert_eq!(judged[1].moved, Some(Moved::Down));
        assert_eq!(judged[2].old_word, Some(OldWord::Equivalent));
        // Every row passes against real football: the worse row does not fail the run.
        for r in &mut judged {
            r.word = Word::Pass;
        }
        assert_eq!(joint_word(&judged, &joint.guards), Word::Pass);
        // With no target here, a moved band is not sure and keeps its direction.
        set_old_words(&mut judged, &rows, &found, c, 0.05, &|_| None);
        assert_eq!(judged[0].old_word, Some(OldWord::NotSure));
        assert_eq!(judged[0].moved, Some(Moved::Up));
        assert!(
            judged[0]
                .old_reason
                .as_deref()
                .unwrap()
                .contains("no target here")
        );
    }

    #[test]
    fn the_quantiles_are_the_constants_they_replace() {
        assert!((one_band() - 2.1701).abs() < 1e-4);
    }
}

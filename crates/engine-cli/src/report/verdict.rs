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

use super::measures;
use super::{BandCheck, RunBuilder, Suite};

/// Resamples of the bootstrap.
pub const RESAMPLES: usize = 1999;
/// The joint false-alarm limit of a run.
pub const FALSE_ALARM: f64 = 0.03;
/// The normal quantile of 80 percent power.
pub const Z_POWER: f64 = 0.8416;
/// The two-sided normal threshold of one band at the false-alarm limit: the joint
/// threshold over many bands is never below it, also when no resample varies.
pub const ONE_BAND: f64 = 2.1701;
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

/// The rows of a run without an old engine: each band's word is its range check.
pub fn range_only(checks: &[BandCheck]) -> Vec<VerdictRow> {
    checks
        .iter()
        .filter(|c| c.band != "wall_ms")
        .map(|c| VerdictRow {
            band: c.band.clone(),
            suite: c.suite.clone(),
            pairing: c.pairing.clone(),
            word: if c.pass { Word::Pass } else { Word::Fail },
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
            let tc = measures::terms(spec.def, &c_sub, spec.scope);
            let to = measures::terms(spec.def, &o_sub, spec.scope);
            let places = measures::places(&spec.def.measure);
            let value = tc.value().map(|v| round_to(v, places));
            let in_range = measures::passes(spec.def, value.unwrap_or(0.0), &tc);
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
            });
        }
    }
    (sets, rows)
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
        .map(|r| change(r, &ones[r.set], &members[r.set]))
        .collect();
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
            .filter_map(|(r, o)| {
                let (d, _) = (*o)?;
                let (d_star, se_star) = change(r, &weights[r.set], &members[r.set])?;
                (se_star > 0.0).then(|| (d_star - d).abs() / se_star)
            })
            .fold(0.0, f64::max);
        stats.push(t);
    }
    stats.sort_by(f64::total_cmp);
    let critical = if stats.is_empty() {
        0.0
    } else {
        // The ceil((B + 1)(1 - alpha))-th smallest: exact for exchangeable resamples.
        let k = ((resamples as f64 + 1.0) * (1.0 - FALSE_ALARM)).ceil() as usize;
        stats[k.clamp(1, stats.len()) - 1]
    }
    .max(ONE_BAND);
    let found = observed
        .into_iter()
        .map(|o| Found {
            diff: o.map(|(d, _)| d),
            se: o.map_or(0.0, |(_, se)| se),
        })
        .collect();
    (found, critical)
}

/// `true` when a change of `diff` with error `se` is beyond the threshold `c`.
fn significant(diff: f64, se: f64, c: f64) -> bool {
    if se > 0.0 {
        diff.abs() > c * se
    } else {
        diff != 0.0
    }
}

/// Matches per suite unit that would give a band of error `se` at `per_unit` matches power
/// to see `shift`: the error falls with the square root of the matches.
pub fn needed(per_unit: u32, se: f64, shift: f64, c: f64) -> u32 {
    let scale = ((c + Z_POWER) * se / shift).powi(2);
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
            enough && r.spread && f.diff.is_some() && (critical + Z_POWER) * power_se <= r.shift;
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
        });
    }
    let word = if !guards.is_empty() || out.iter().any(|r| r.word == Word::Fail) {
        Word::Fail
    } else if !out.is_empty() && out.iter().all(|r| r.word == Word::Pass) {
        Word::Pass
    } else {
        Word::NotSure
    };
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
    let mut out = String::new();
    let change = joint.is_some();
    if change {
        out.push_str(&format!(
            "{:<24} {:<10} {:<14} {:>10} {:>10} {:>10} {:>9} {:>6}  {}\n",
            "band", "suite", "pairing", "value", "old", "change", "se", "power", "verdict"
        ));
    } else {
        out.push_str(&format!(
            "{:<24} {:<10} {:<14} {:>10} {:>21}  {}\n",
            "band", "suite", "pairing", "value", "range", "verdict"
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
        if change {
            let opt = |v: Option<f64>| v.map_or_else(|| "-".to_string(), |v| format!("{v}"));
            out.push_str(&format!(
                "{:<24} {:<10} {:<14} {:>10} {:>10} {:>10} {:>9} {:>6}  {}{need}\n",
                r.band,
                r.suite,
                pairing,
                r.value,
                opt(r.base_value),
                opt(r.diff),
                opt(r.se),
                if r.power == Some(true) { "yes" } else { "no" },
                r.word.code()
            ));
        } else {
            out.push_str(&format!(
                "{:<24} {:<10} {:<14} {:>10} {:>21}  {}\n",
                r.band,
                r.suite,
                pairing,
                r.value,
                format!("{} to {}", r.lo, r.hi),
                r.word.code()
            ));
        }
    }
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
            // Six measures with a shared match factor (correlation about 0.5).
            let mut arm = || -> Vec<[f64; 6]> {
                (0..n)
                    .map(|_| {
                        let common = normal(&mut rng);
                        let x: [f64; 6] = std::array::from_fn(|_| common + normal(&mut rng));
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
            let (found, c) = bootstrap(&sets, &rows, 1000 + u64::from(rep), resamples);
            let (judged, _) = judge(&sets, &rows, &found, c, Vec::new(), &BTreeMap::new());
            assert_eq!(judged.len(), 6, "every band on its own row");
            if found.iter().any(|f| significant(f.diff.unwrap(), f.se, c)) {
                alarms += 1;
            }
        }
        let rate = f64::from(alarms) / f64::from(replicates);
        eprintln!("A/A rate {rate} over {replicates} replicates of {n} pairs");
        assert!(rate <= FALSE_ALARM, "joint false-alarm rate {rate}");
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
}

//! The measures of the band registry: a closed catalog of per-match fields, read from each
//! match's band record, and the evaluator that turns a band's measure into a value over a
//! suite's matches, with or without resampling weights.
//!
//! Every measure is a ratio of weighted sums over the matches, `A / B`, times `D / C` for
//! the measures that divide by a second sum (`full_over_control`, which compares two arms,
//! and the factorial-moment ratio): a mean is the sum of the values over the count, a share
//! the count of hits over the count, a pooled ratio the sum of numerators over the sum of
//! denominators. With every weight 1 the value is computed in the order the hard-coded
//! figures were, so it equals them exactly. The one measure that is not a sum over matches,
//! a club range, has its own value and error and no paired form.
//!
//! A measures build adds the realism measures of each match to the catalog: they are read
//! from the match's extras (its compact row's measures and players' running), and a match
//! without them does not count toward a band that reads them.

use std::collections::BTreeMap;

use engine::observe::MatchStats;

use super::bands::{BandDef, Builtin, Measure, Per, Source};
use super::distribution::{Hist, Metric, bin_of};
use super::{se_mean, se_ratio, se_share};
use crate::measure_rows::{
    Measures, PASS_CELL_NAMES, PHASES, PLAYER_SLOTS, PlayerRow, SHAPE_FIELDS, ZONES, shape_place,
};

/// The per-team fields of the catalog. A band names one as `<field>.home`, `<field>.away`,
/// or, in a per-team mean, `<field>.{side}`.
pub const SIDED: [&str; 16] = [
    "goals",
    "shots",
    "shots_on_target",
    "xg",
    "passes",
    "passes_completed",
    "pass_accuracy_pct",
    "possession_pct",
    "fouls",
    "offsides",
    "corners",
    "throw_ins",
    "goal_kicks",
    "yellow",
    "red",
    "substitutions",
];

/// The per-match fields of the catalog: `ball_in_play_min` is `ball_in_play_s` in minutes.
pub const MATCH: [&str; 3] = ["injuries", "ball_in_play_s", "ball_in_play_min"];

/// The per-team fields a `measures` build adds, read from a match's extras. Five name a
/// family: `pass_attempts.<cell>` and `pass_completed.<cell>` for each pass cell,
/// `goals_phase.<phase>` for each goal phase, `third_ticks.<third>` for each third, and
/// `zone_ticks.<zone>` for each speed zone. The running fields sum over the players of the
/// side who played: `players_used`, `minutes_on`, `distance_m`, the sum of each player's
/// peak speed `peak_mps`, and the ball-in-play ticks in each zone. A sixth family,
/// `shape.<sum>`, names each team-shape sum ([`crate::measure_rows::shape_name`]), such as
/// `shape.cb.depth_in_sum` or `shape.slide_x_in_sxy`, and `shape.<group>.live_min`, the
/// group's ball-in-play player-minutes.
pub const MEASURE_FIELDS: [&str; 14] = [
    "shot_distance_m",
    "pass_attempts",
    "pass_completed",
    "goals_phase",
    "third_ticks",
    "chains",
    "chain_passes",
    "chain_seconds",
    "players_used",
    "minutes_on",
    "distance_m",
    "peak_mps",
    "zone_ticks",
    "shape",
];

/// The thirds of the pitch, from a side's own goal line.
pub const THIRDS: [&str; 3] = ["own", "middle", "final"];

/// What a match's compact row holds beyond its band record: the realism measures, each
/// player's running, the team-shape sums, and the two clubs as (league, club), home first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extras {
    pub measures: Measures,
    pub physical: [[PlayerRow; PLAYER_SLOTS]; 2],
    pub shape: [[f64; SHAPE_FIELDS]; 2],
    pub clubs: [(u32, u32); 2],
}

/// The family member `name` of a field family, by its place.
fn member(family: &[&str], name: &str) -> Option<usize> {
    family.iter().position(|f| *f == name)
}

/// `true` when `base` names a measures field: a plain one, or a family with a known member.
fn measure_base(base: &str) -> bool {
    match base.split_once('.') {
        Some(("pass_attempts" | "pass_completed", cell)) => {
            member(&PASS_CELL_NAMES, cell).is_some()
        }
        Some(("goals_phase", phase)) => member(&PHASES, phase).is_some(),
        Some(("third_ticks", third)) => member(&THIRDS, third).is_some(),
        Some(("zone_ticks", zone)) => member(&ZONES, zone).is_some(),
        Some(("shape", sum)) => shape_sum(&[[0.0; SHAPE_FIELDS]; 2], sum, 0).is_some(),
        Some(_) => false,
        None => {
            MEASURE_FIELDS.contains(&base)
                && !matches!(
                    base,
                    "pass_attempts"
                        | "pass_completed"
                        | "goals_phase"
                        | "third_ticks"
                        | "zone_ticks"
                        | "shape"
                )
        }
    }
}

/// `true` when the field `name` (`<base>.home`, `.away` or `.{side}`) is a measures field.
pub fn is_measure_field(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(base, _)| measure_base(base))
}

/// Every field a measure names.
pub fn fields_of(m: &Measure) -> Vec<&str> {
    fn all<'a>(lists: &[&'a Vec<String>]) -> Vec<&'a str> {
        lists
            .iter()
            .flat_map(|l| l.iter().map(String::as_str))
            .collect()
    }
    match m {
        Measure::Mean { of, .. } | Measure::Share { of, .. } | Measure::FactorialRatio { of } => {
            all(&[of])
        }
        Measure::Ratio { num, den } => all(&[num, den]),
        Measure::Difference { of, minus } | Measure::ClubRange { of, minus, .. } => {
            all(&[of, minus])
        }
        Measure::Scoreline { .. }
        | Measure::Builtin { .. }
        | Measure::Distribution { .. }
        | Measure::Cells { .. } => Vec::new(),
    }
}

/// The value of the measures field `base` for `side` in a match's extras; `None` when the
/// match was not played by a measures build.
fn extra(e: &Extras, base: &str, side: usize) -> Option<f64> {
    if !e.measures.present {
        return None;
    }
    let m = &e.measures;
    let ticks_per_s = f64::from(engine::TICKS_PER_SECOND);
    let players = || e.physical[side].iter().filter(|p| p.ticks_on > 0);
    Some(match base.split_once('.') {
        Some(("pass_attempts", cell)) => {
            f64::from(m.pass_attempts[side][member(&PASS_CELL_NAMES, cell)?])
        }
        Some(("pass_completed", cell)) => {
            f64::from(m.pass_completed[side][member(&PASS_CELL_NAMES, cell)?])
        }
        Some(("goals_phase", phase)) => f64::from(m.goals_by_phase[side][member(&PHASES, phase)?]),
        Some(("third_ticks", third)) => f64::from(m.third_ticks[side][member(&THIRDS, third)?]),
        Some(("zone_ticks", zone)) => {
            let z = member(&ZONES, zone)?;
            players().map(|p| f64::from(p.zone_ticks[z])).sum()
        }
        Some(("shape", sum)) => shape_sum(&e.shape, sum, side)?,
        Some(_) => return None,
        None => match base {
            "shot_distance_m" => m.shot_distance_m[side],
            "chains" => f64::from(m.chains[side]),
            "chain_passes" => f64::from(m.chain_passes[side]),
            // At most 2^53 ticks: exact in an f64.
            "chain_seconds" => m.chain_ticks[side] as f64 / ticks_per_s,
            // At most 16 slots.
            "players_used" => players().count() as f64,
            "minutes_on" => {
                players().map(|p| f64::from(p.ticks_on)).sum::<f64>() / (ticks_per_s * 60.0)
            }
            "distance_m" => players().map(|p| p.distance_m).sum(),
            "peak_mps" => players().map(|p| p.peak_mps).sum(),
            _ => return None,
        },
    })
}

/// The team-shape sum `sum` of `side`: a kept sum by its name, or `<group>.live_min`, the
/// group's ball-in-play ticks in player-minutes.
fn shape_sum(shape: &[[f64; SHAPE_FIELDS]; 2], sum: &str, side: usize) -> Option<f64> {
    if let Some(group) = sum.strip_suffix(".live_min") {
        let ticks = shape[side][shape_place(&format!("{group}.live_ticks"))?];
        return Some(ticks / (f64::from(engine::TICKS_PER_SECOND) * 60.0));
    }
    Some(shape[side][shape_place(sum)?])
}

/// The value of the per-team field `name` for `side` (0 home, 1 away).
fn sided(s: &MatchStats, name: &str, side: usize) -> Option<f64> {
    let u = |a: [u32; 2]| f64::from(a[side]);
    Some(match name {
        "goals" => u(s.goals),
        "shots" => u(s.tactics.shots),
        "shots_on_target" => u(s.figures.shots_on_target),
        "xg" => s.figures.xg[side],
        "passes" => u(s.figures.passes),
        "passes_completed" => u(s.figures.passes_completed),
        "pass_accuracy_pct" => s.figures.pass_accuracy_pct[side],
        "possession_pct" => s.figures.possession_pct[side],
        "fouls" => u(s.laws.fouls),
        "offsides" => u(s.laws.offsides),
        "corners" => u(s.laws.corners),
        "throw_ins" => u(s.laws.throw_ins),
        "goal_kicks" => u(s.laws.goal_kicks),
        "yellow" => u(s.laws.yellow),
        "red" => u(s.laws.red),
        "substitutions" => u(s.tactics.substitutions),
        _ => return None,
    })
}

/// The value of the catalog field `name` in one match; `side` replaces `{side}`.
pub fn field(s: &MatchStats, name: &str, side: Option<usize>) -> Option<f64> {
    match name {
        "injuries" => return Some(f64::from(s.tactics.injury_count)),
        "ball_in_play_s" => return Some(f64::from(s.figures.ball_in_play_s)),
        "ball_in_play_min" => return Some(f64::from(s.figures.ball_in_play_s) / 60.0),
        _ => {}
    }
    let (base, which) = name.rsplit_once('.')?;
    let side = match which {
        "home" => 0,
        "away" => 1,
        "{side}" => side?,
        _ => return None,
    };
    sided(s, base, side)
}

/// The value of the catalog field `name` in the match `o`: from its band record, or for a
/// measures field from its extras; `side` replaces `{side}`.
pub fn field_of(o: &Obs<'_>, name: &str, side: Option<usize>) -> Option<f64> {
    if !is_measure_field(name) {
        return field(o.stats, name, side);
    }
    let (base, which) = name.rsplit_once('.')?;
    let side = match which {
        "home" => 0,
        "away" => 1,
        "{side}" => side?,
        _ => return None,
    };
    extra(o.extras?, base, side)
}

/// Why a field name is not in the catalog, or `None` when it is. `per_team` allows
/// `{side}`, and requires it.
fn field_error(name: &str, per_team: bool) -> Option<String> {
    let known = |base: &str| SIDED.contains(&base) || measure_base(base);
    let fine = match name.rsplit_once('.') {
        Some((base, "{side}")) if known(base) => per_team,
        Some((base, "home" | "away")) if known(base) => !per_team,
        _ => MATCH.contains(&name) && !per_team,
    };
    if fine {
        return None;
    }
    let catalog = || {
        let sided: Vec<String> = SIDED.iter().map(|f| format!("{f}.home|away")).collect();
        format!(
            "{}, {}; a measures build adds {}",
            sided.join(", "),
            MATCH.join(", "),
            MEASURE_FIELDS.join(", ")
        )
    };
    Some(if per_team {
        format!(
            "{name}: a per-team mean names each field as <field>.{{side}}; the fields are {}",
            SIDED.join(", ")
        )
    } else if name.ends_with(".{side}") {
        format!("{name}: {{side}} belongs to a per-team mean only")
    } else {
        format!("{name} is not a field of the catalog ({})", catalog())
    })
}

/// Checks a measure against the catalog; the error names the field.
pub fn check(m: &Measure) -> Result<(), String> {
    let list = |of: &[String], per_team: bool, what: &str| -> Result<(), String> {
        if of.is_empty() {
            return Err(format!("{what} names no field"));
        }
        match of.iter().find_map(|f| field_error(f, per_team)) {
            Some(why) => Err(why),
            None => Ok(()),
        }
    };
    match m {
        Measure::Mean { per, of } => list(of, *per == Per::Team, "of"),
        Measure::Share { of, value, .. } => {
            if !value.is_finite() {
                return Err(format!("value {value} is not a number"));
            }
            list(of, false, "of")
        }
        Measure::Ratio { num, den } => {
            list(num, false, "num")?;
            list(den, false, "den")
        }
        Measure::Builtin { .. } => Ok(()),
        Measure::Difference { of, minus } => {
            list(of, false, "of")?;
            list(minus, false, "minus")
        }
        Measure::Scoreline { scores } => {
            if scores.is_empty() {
                return Err("scores names no score".into());
            }
            match scores.iter().find(|s| s.iter().any(|g| *g > MAX_SCORE)) {
                Some(s) => Err(format!(
                    "score {}-{}: each side's goals must be 0 to {MAX_SCORE}",
                    s[0], s[1]
                )),
                None => Ok(()),
            }
        }
        Measure::FactorialRatio { of } => list(of, true, "of"),
        Measure::ClubRange { of, minus, window } => {
            list(of, true, "of")?;
            list(minus, true, "minus")?;
            if *window < 2 {
                return Err(format!("window {window}: a range needs at least 2 matches"));
            }
            Ok(())
        }
        Measure::Distribution { of, width, bins } => {
            if !(width.is_finite() && *width > 0.0) {
                return Err(format!("width {width} is not above 0"));
            }
            if !(2..=MAX_BINS).contains(bins) {
                return Err(format!("bins {bins}: a histogram has 2 to {MAX_BINS} bins"));
            }
            match of.row_bins() {
                Some((w, b)) if (w, b) != (*width, *bins) => Err(format!(
                    "width {width} and bins {bins}: the rows hold {} in {b} bins of {w}",
                    of.code()
                )),
                _ => Ok(()),
            }
        }
        Measure::Cells { max } => {
            if !(1..=MAX_SCORE).contains(max) {
                return Err(format!(
                    "max {max}: each side's cap must be 1 to {MAX_SCORE}"
                ));
            }
            Ok(())
        }
    }
}

/// The most bins a distribution band may name.
const MAX_BINS: u32 = 100_000;

/// The most goals one side of a scoreline band may name.
const MAX_SCORE: u32 = 20;

/// One match of a suite as the measures see it: its band record and its place in the
/// suite's design.
#[derive(Debug, Clone, Copy)]
pub struct Obs<'a> {
    pub stats: &'a MatchStats,
    /// The strength suite: the side of the boosted club.
    pub boosted: Option<usize>,
    /// The formations suite: the pairing's place in the run.
    pub pairing: Option<usize>,
    /// The red-card suite: the arm's number.
    pub arm: Option<usize>,
    /// The match's extras: its measures, its players' running and its clubs; `None` for a
    /// match folded without them.
    pub extras: Option<&'a Extras>,
    /// The generated league the match belongs to, the outer unit the pass rule resamples;
    /// `None` for a match folded without its fixture.
    pub league: Option<u32>,
}

impl Obs<'_> {
    fn played(&self) -> bool {
        self.stats.outcome == "success"
    }
}

/// The matches of a suite a band row covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    All,
    /// One formation pairing, by its place in the run.
    Pairing(usize),
    /// One red-card arm, by its number.
    Arm(usize),
}

impl Scope {
    fn holds(self, o: &Obs<'_>) -> bool {
        match self {
            Scope::All => true,
            Scope::Pairing(p) => o.pairing == Some(p),
            Scope::Arm(a) => o.arm == Some(a),
        }
    }
}

/// How a row's sampling error is computed: as the hard-coded figures did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Error {
    /// The standard error of the mean of the per-match values.
    Mean,
    /// sqrt(p (1 - p) / n).
    Share,
    /// The ratio estimator's error over every covered match.
    Ratio,
    /// The delta-method error of one arm's mean over the control's.
    Delta,
    /// The delta-method error of `(A / B) · (D / C)` from each covered match's influence.
    Influence,
    /// A value that is not a sum over matches (a club range): its own error, in `fixed_se`.
    Fixed,
}

/// The per-match terms of one band row. `a` and `b` hold `stride` entries per match (two
/// for a per-team mean, home then away); `c` and `d` hold one, and are 1 for every measure
/// but `full_over_control`. A match the row does not cover, or that did not finish, has
/// every term 0 and is not in `covered`.
#[derive(Debug, Clone, PartialEq)]
pub struct Terms {
    stride: usize,
    a: Vec<f64>,
    b: Vec<f64>,
    c: Vec<f64>,
    d: Vec<f64>,
    covered: Vec<bool>,
    error: Error,
    /// The measure's value with every weight 1, before rounding; `None` for an empty
    /// denominator.
    value: Option<f64>,
    /// A club range: the standard error of its value over the clubs.
    fixed_se: f64,
    /// Why the row has no value, when the measure can say.
    why_none: Option<String>,
}

impl Terms {
    /// Matches the row covers that finished.
    pub fn covered(&self) -> usize {
        self.covered.iter().filter(|c| **c).count()
    }

    /// For each match, whether the row covers it and it finished.
    pub fn covered_flags(&self) -> &[bool] {
        &self.covered
    }

    /// The value with every weight 1, before rounding; `None` when a denominator is 0.
    pub fn value(&self) -> Option<f64> {
        self.value
    }

    /// Why the row has no value, when the measure says.
    pub fn why_none(&self) -> Option<&str> {
        self.why_none.as_deref()
    }

    /// The per-match sums `(a, b, c, d)`, one entry per match.
    pub fn per_match(&self) -> [Vec<f64>; 4] {
        let collapse = |v: &[f64], s: usize| -> Vec<f64> {
            v.chunks(s.max(1)).map(|c| c.iter().sum()).collect()
        };
        [
            collapse(&self.a, self.stride),
            collapse(&self.b, self.stride),
            self.c.clone(),
            self.d.clone(),
        ]
    }

    /// The sampling error of the value, as the hard-coded figures computed it; 0 below two
    /// covered matches.
    pub fn se(&self) -> f64 {
        let [a, b, c, d] = self.per_match();
        let covered = |x: &[f64], y: &[f64]| -> Vec<f64> {
            x.iter()
                .zip(y)
                .zip(&self.covered)
                .filter(|(_, c)| **c)
                .map(|((x, y), _)| x / y)
                .collect()
        };
        match self.error {
            Error::Mean => se_mean(&covered(&a, &b)),
            Error::Share => {
                let n = self.covered();
                let hits: f64 = a.iter().sum();
                se_share(if n == 0 { 0.0 } else { hits / n as f64 }, n)
            }
            Error::Ratio => {
                let pick = |v: &[f64]| -> Vec<f64> {
                    v.iter()
                        .zip(&self.covered)
                        .filter(|(_, c)| **c)
                        .map(|(x, _)| *x)
                        .collect()
                };
                se_ratio(&pick(&a), &pick(&b))
            }
            Error::Delta => {
                let full: Vec<f64> = a
                    .iter()
                    .zip(&b)
                    .filter(|(_, b)| **b > 0.0)
                    .map(|(a, b)| a / b)
                    .collect();
                let control: Vec<f64> = c
                    .iter()
                    .zip(&d)
                    .filter(|(_, d)| **d > 0.0)
                    .map(|(c, d)| c / d)
                    .collect();
                let (f, se_f) = (mean(&full), se_mean(&full));
                let (k, se_k) = (mean(&control), se_mean(&control));
                if f > 0.0 && k > 0.0 {
                    (f / k) * ((se_f / f).powi(2) + (se_k / k).powi(2)).sqrt()
                } else {
                    0.0
                }
            }
            Error::Influence => influence_se([&a, &b, &c, &d], &self.covered),
            Error::Fixed => self.fixed_se,
        }
    }
}

/// The delta-method standard error of `(A / B) · (D / C)` over the covered matches: each
/// match's influence is `v · (a/A - b/B + d/D - c/C)`, and the squared error is the sum of
/// the centred squared influences times `n / (n - 1)`. 0 below two covered matches.
fn influence_se(t: [&[f64]; 4], covered: &[bool]) -> f64 {
    let at: Vec<usize> = (0..covered.len()).filter(|&i| covered[i]).collect();
    let n = at.len();
    let s: [f64; 4] = std::array::from_fn(|k| at.iter().map(|&i| t[k][i]).sum());
    if n < 2 || s[1] <= 0.0 || s[2] <= 0.0 {
        return 0.0;
    }
    let v = s[0] / s[1] * (s[3] / s[2]);
    let term = |k: usize, i: usize| if s[k] > 0.0 { t[k][i] / s[k] } else { 0.0 };
    let psi: Vec<f64> = at
        .iter()
        .map(|&i| v * (term(0, i) - term(1, i) + term(3, i) - term(2, i)))
        .collect();
    let m = mean(&psi);
    let ss: f64 = psi.iter().map(|p| (p - m) * (p - m)).sum();
    (ss * n as f64 / (n as f64 - 1.0)).sqrt()
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        0.0
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}

/// The value of per-match sums under `weights` (one per match): `(Σ w a / Σ w b) ·
/// (Σ w d / Σ w c)`, or `None` when a denominator is 0.
#[cfg(test)]
pub fn weighted(terms: &[Vec<f64>; 4], weights: &[u32]) -> Option<f64> {
    let mut s = [0.0f64; 4];
    for (i, &w) in weights.iter().enumerate() {
        if w == 0 {
            continue;
        }
        let w = f64::from(w);
        for (k, sum) in s.iter_mut().enumerate() {
            *sum += w * terms[k][i];
        }
    }
    let [a, b, c, d] = s;
    if b > 0.0 && c > 0.0 {
        Some(a / b * (d / c))
    } else {
        None
    }
}

/// The sum of `of` in one match, `side` replacing `{side}`. Every name is in the catalog
/// (the registry checked it on load); an unknown one counts 0.
fn sum(o: &Obs<'_>, of: &[String], side: Option<usize>) -> f64 {
    of.iter()
        .map(|f| field_of(o, f, side).unwrap_or(0.0))
        .fold(0.0, |acc, v| acc + v)
}

/// A club range over the matches `take` keeps: for each club, the per-match value of its
/// `of` minus its opponent's `minus` over its first `window` matches; the mean of the best
/// minus the worst over the clubs with `window` matches, and its standard error over them.
fn club_range(
    obs: &[Obs<'_>],
    take: impl Fn(&Obs<'_>) -> bool,
    (of, minus, window): (&[String], &[String], u32),
) -> Result<(f64, f64), String> {
    let window = window as usize;
    let mut by: BTreeMap<(u32, u32), Vec<f64>> = BTreeMap::new();
    for o in obs.iter().filter(|o| take(o)) {
        let Some(e) = o.extras else { continue };
        for side in 0..2 {
            let list = by.entry(e.clubs[side]).or_default();
            if list.len() < window {
                list.push(sum(o, of, Some(side)) - sum(o, minus, Some(1 - side)));
            }
        }
    }
    let ranges: Vec<f64> = by
        .values()
        .filter(|l| l.len() >= window)
        .map(|l| {
            let max = l.iter().copied().fold(f64::MIN, f64::max);
            let min = l.iter().copied().fold(f64::MAX, f64::min);
            max - min
        })
        .collect();
    if ranges.is_empty() {
        return Err(format!("no club has {window} matches in this run"));
    }
    Ok((mean(&ranges), se_mean(&ranges)))
}

/// The terms of `def`'s measure over `obs`, a suite's matches in fixture order, for the
/// matches `scope` covers.
pub fn terms(def: &BandDef, obs: &[Obs<'_>], scope: Scope) -> Terms {
    let n = obs.len();
    let mut t = Terms {
        stride: 1,
        a: Vec::with_capacity(n),
        b: Vec::with_capacity(n),
        c: vec![1.0; n],
        d: vec![1.0; n],
        covered: Vec::with_capacity(n),
        error: Error::Mean,
        value: None,
        fixed_se: 0.0,
        why_none: None,
    };
    // The scope test comes first: a match outside the scope adds zeros without reading
    // its fields, so a per-pairing row costs its own matches, not the whole suite.
    fn push<const N: usize>(
        t: &mut Terms,
        covered: bool,
        of: impl FnOnce() -> ([f64; N], [f64; N]),
    ) {
        t.covered.push(covered);
        let (a, b) = if covered { of() } else { ([0.0; N], [0.0; N]) };
        t.a.extend_from_slice(&a);
        t.b.extend_from_slice(&b);
    }
    // A band that reads the measures counts only the matches a measures build played.
    let needs = def.needs_measures();
    let take = |o: &Obs<'_>| {
        o.played() && scope.holds(o) && (!needs || o.extras.is_some_and(|e| e.measures.present))
    };
    match &def.measure {
        Measure::Mean {
            per: Per::Match,
            of,
        } => {
            for o in obs {
                push(&mut t, take(o), || ([sum(o, of, None)], [1.0]));
            }
        }
        Measure::Mean { per: Per::Team, of } => {
            t.stride = 2;
            for o in obs {
                push(&mut t, take(o), || {
                    let home = sum(o, of, Some(0));
                    let away = sum(o, of, Some(1));
                    ([home, away], [1.0, 1.0])
                });
            }
        }
        Measure::Share { of, op, value } => {
            t.error = Error::Share;
            for o in obs {
                push(&mut t, take(o), || {
                    let hit = op.holds(sum(o, of, None), *value);
                    ([f64::from(u8::from(hit))], [1.0])
                });
            }
        }
        Measure::Ratio { num, den } => {
            t.error = Error::Ratio;
            for o in obs {
                push(&mut t, take(o), || {
                    ([sum(o, num, None)], [sum(o, den, None)])
                });
            }
        }
        Measure::Difference { of, minus } => {
            for o in obs {
                push(&mut t, take(o), || {
                    ([sum(o, of, None) - sum(o, minus, None)], [1.0])
                });
            }
        }
        Measure::Scoreline { scores } => {
            t.error = Error::Share;
            for o in obs {
                push(&mut t, take(o), || {
                    let [h, a] = o.stats.goals;
                    let hit = scores.iter().any(|s| *s == [h, a] || *s == [a, h]);
                    ([f64::from(u8::from(hit))], [1.0])
                });
            }
        }
        Measure::FactorialRatio { of } => {
            t.error = Error::Influence;
            for (i, o) in obs.iter().enumerate() {
                let covered = take(o);
                let x = if covered {
                    [sum(o, of, Some(0)), sum(o, of, Some(1))]
                } else {
                    [0.0; 2]
                };
                push(&mut t, covered, || {
                    ([x[0] * (x[0] - 1.0) + x[1] * (x[1] - 1.0)], [x[0] + x[1]])
                });
                t.c[i] = x[0] + x[1];
                t.d[i] = if covered { 2.0 } else { 0.0 };
            }
        }
        Measure::ClubRange { of, minus, window } => {
            t.error = Error::Fixed;
            for _ in obs {
                push(&mut t, false, || ([0.0], [0.0]));
            }
            match club_range(obs, take, (of, minus, *window)) {
                Ok((value, se)) => {
                    t.value = Some(value);
                    t.fixed_se = se;
                }
                Err(why) => t.why_none = Some(why),
            }
            return t;
        }
        // A whole distribution has no per-match terms: it is judged by its distance, from
        // [`histograms`].
        Measure::Distribution { .. } | Measure::Cells { .. } => {
            t.error = Error::Fixed;
            for _ in obs {
                push(&mut t, false, || ([0.0], [0.0]));
            }
            t.why_none = Some("a whole distribution is judged by its distance".into());
            return t;
        }
        Measure::Builtin {
            name: Builtin::StrongerTeamWinRate,
        } => {
            t.error = Error::Share;
            for o in obs {
                let covered = take(o) && o.boosted.is_some();
                push(&mut t, covered, || {
                    let side = o.boosted.unwrap_or(0);
                    let win = o.stats.goals[side] > o.stats.goals[1 - side];
                    ([f64::from(u8::from(win))], [1.0])
                });
            }
        }
        Measure::Builtin {
            name: Builtin::ReducedMinusFull,
        } => {
            for o in obs {
                push(&mut t, take(o), || {
                    let g = o.stats.goals.map(f64::from);
                    ([g[1] - g[0]], [1.0])
                });
            }
        }
        Measure::Builtin {
            name: Builtin::FullOverControl,
        } => {
            t.error = Error::Delta;
            for (i, o) in obs.iter().enumerate() {
                let home = f64::from(o.stats.goals[0]);
                push(&mut t, take(o), || ([home], [1.0]));
                let control = o.played() && o.arm == Some(0);
                t.c[i] = if control { home } else { 0.0 };
                t.d[i] = f64::from(u8::from(control));
            }
        }
    }
    // The value in the hard-coded figures' order: each entry in turn, then one division.
    let total = |v: &[f64]| v.iter().fold(0.0, |acc, x| acc + x);
    let (a, b, c, d) = (total(&t.a), total(&t.b), total(&t.c), total(&t.d));
    t.value = match &def.measure {
        Measure::Builtin {
            name: Builtin::FullOverControl,
        } => {
            let (full, control) = (a / b.max(1.0), c / d.max(1.0));
            (b > 0.0 && d > 0.0 && control > 0.0).then(|| full / control)
        }
        Measure::FactorialRatio { .. } => (b > 0.0 && c > 0.0).then(|| a / b * (d / c)),
        _ => (b > 0.0).then(|| a / b),
    };
    if t.value.is_none() {
        t.why_none = Some(if needs && t.covered() == 0 {
            "no match of this run carries the measures".into()
        } else {
            "no match of this run gives it a value".into()
        });
    }
    t
}

/// The places a value is shown to: a mean to 3, a share or a ratio to 4, a distance to 5.
pub fn places(m: &Measure) -> i32 {
    match m {
        Measure::Mean { .. } | Measure::Difference { .. } | Measure::ClubRange { .. } => 3,
        Measure::Distribution { .. } | Measure::Cells { .. } => 5,
        _ => 4,
    }
}

/// The metric of a distribution band: total variation for cells, Wasserstein-1 otherwise.
pub fn metric(m: &Measure) -> Metric {
    match m {
        Measure::Cells { .. } => Metric::Tv,
        _ => Metric::W1,
    }
}

/// The `(width, bins)` of a distribution band's histograms; `None` for any other band.
pub fn hist_bins(m: &Measure) -> Option<(f64, u32)> {
    match m {
        Measure::Distribution { width, bins, .. } => Some((*width, *bins)),
        Measure::Cells { max } => Some((1.0, (max + 1) * (max + 1))),
        _ => None,
    }
}

/// The cell of a score with each side capped at `max`: home times `max + 1` plus away.
pub fn cell(home: u32, away: u32, max: u32) -> u32 {
    home.min(max) * (max + 1) + away.min(max)
}

/// The histogram of `def`'s distribution in each match of `obs`, in their order: `None` for
/// a match the band does not cover (outside `scope`, not played, or without the measures a
/// measures source needs), so the list stays aligned with the matches. A measures source
/// copies the rows' bins as they are; goals and passes are binned by `width`.
pub fn histograms(def: &BandDef, obs: &[Obs<'_>], scope: Scope) -> Vec<Option<Hist>> {
    let needs = def.needs_measures();
    let Some((width, bins)) = hist_bins(&def.measure) else {
        return vec![None; obs.len()];
    };
    let sparse = |dense: &mut dyn Iterator<Item = (usize, u32)>| -> Hist {
        let mut h: BTreeMap<u32, f64> = BTreeMap::new();
        for (k, n) in dense {
            if n > 0 {
                // At most a few dozen bins.
                *h.entry(k as u32).or_default() += f64::from(n);
            }
        }
        h.into_iter().collect()
    };
    obs.iter()
        .map(|o| {
            let measures = o.extras.map(|e| &e.measures).filter(|m| m.present);
            if !(o.played() && scope.holds(o)) || (needs && measures.is_none()) {
                return None;
            }
            let pair = |a: &[u32], b: &[u32]| -> Hist {
                sparse(&mut a.iter().zip(b).map(|(x, y)| x + y).enumerate())
            };
            let points = |xs: &[f64]| -> Hist {
                let mut h: BTreeMap<u32, f64> = BTreeMap::new();
                for x in xs {
                    *h.entry(bin_of(*x, width, bins)).or_default() += 1.0;
                }
                h.into_iter().collect()
            };
            let s = o.stats;
            Some(match &def.measure {
                Measure::Cells { max } => vec![(cell(s.goals[0], s.goals[1], *max), 1.0)],
                Measure::Distribution { of, .. } => match of {
                    Source::Goals => points(&[f64::from(s.goals[0] + s.goals[1])]),
                    Source::Passes => points(&s.figures.passes.map(f64::from)),
                    Source::Shots => points(&[f64::from(s.tactics.shots[0] + s.tactics.shots[1])]),
                    Source::Cards => {
                        let l = &s.laws;
                        points(&[f64::from(l.yellow[0] + l.yellow[1] + l.red[0] + l.red[1])])
                    }
                    Source::ShotDistance => {
                        let m = measures?;
                        pair(&m.shot_bins[0], &m.shot_bins[1])
                    }
                    Source::ChainPasses => {
                        let m = measures?;
                        pair(&m.chain_pass_bins[0], &m.chain_pass_bins[1])
                    }
                    Source::ChainSeconds => {
                        let m = measures?;
                        pair(&m.chain_second_bins[0], &m.chain_second_bins[1])
                    }
                },
                _ => return None,
            })
        })
        .collect()
}

/// The row's range check, as the hard-coded checks made it: the rounded value inside the
/// range; the stronger team's win rate above `lo`; a red-card row at or below `hi`, judged
/// on the unrounded means.
pub fn passes(def: &BandDef, rounded: f64, terms: &Terms) -> bool {
    match &def.measure {
        Measure::Builtin {
            name: Builtin::StrongerTeamWinRate,
        } => rounded > def.lo,
        Measure::Builtin {
            name: Builtin::ReducedMinusFull,
        } => terms.value.unwrap_or(0.0) <= def.hi,
        Measure::Builtin {
            name: Builtin::FullOverControl,
        } => {
            let [a, b, c, d] = terms.per_match();
            let mean = |x: &[f64], y: &[f64]| {
                let n: f64 = y.iter().sum();
                if n > 0.0 {
                    x.iter().sum::<f64>() / n
                } else {
                    0.0
                }
            };
            mean(&a, &b) <= def.hi * mean(&c, &d)
        }
        _ => def.range().contains(rounded),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::bands::Op;
    use engine::observe::{LawStats, MatchFigures, TacticsStats};

    fn stats(goals: [u32; 2], shots: [u32; 2], on_target: [u32; 2]) -> MatchStats {
        MatchStats {
            outcome: "success".into(),
            goals,
            laws: LawStats::default(),
            tactics: TacticsStats {
                shots,
                ..TacticsStats::default()
            },
            figures: MatchFigures {
                goals,
                shots_on_target: on_target,
                ..MatchFigures::default()
            },
            ..crate::report::tests_support::blank_stats()
        }
    }

    fn obs(s: &[MatchStats]) -> Vec<Obs<'_>> {
        s.iter()
            .map(|stats| Obs {
                stats,
                boosted: None,
                pairing: None,
                arm: None,
                extras: None,
                league: None,
            })
            .collect()
    }

    fn band(measure: Measure) -> BandDef {
        BandDef {
            band: "b".into(),
            suites: vec!["equal".into()],
            measure,
            lo: 0.0,
            hi: 1.0,
            smallest_shift: 0.1,
            group: None,
            real: None,
            evidence: None,
            confidence: None,
            condition: crate::report::bands::Condition::Dry,
            max_from_dry: None,
            gates: true,
            report_only: None,
            rule: None,
        }
    }

    fn of(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn zero_events_give_zero_and_a_missing_denominator_gives_no_value() {
        let matches = vec![stats([0, 0], [0, 0], [0, 0]); 3];
        let o = obs(&matches);
        let share = terms(
            &band(Measure::Share {
                of: of(&["red.home", "red.away"]),
                op: Op::AtLeast,
                value: 1.0,
            }),
            &o,
            Scope::All,
        );
        assert_eq!(share.value(), Some(0.0));
        assert_eq!(share.se(), 0.0);
        let mean = terms(
            &band(Measure::Mean {
                per: Per::Team,
                of: of(&["yellow.{side}"]),
            }),
            &o,
            Scope::All,
        );
        assert_eq!(mean.value(), Some(0.0));
        // No shot in any match: no conversion rate at all, not a rate of 0.
        let ratio = terms(
            &band(Measure::Ratio {
                num: of(&["shots_on_target.home", "shots_on_target.away"]),
                den: of(&["shots.home", "shots.away"]),
            }),
            &o,
            Scope::All,
        );
        assert_eq!(ratio.value(), None);
        assert_eq!(weighted(&ratio.per_match(), &[1, 1, 1]), None);
        // No match at all: no value for any measure.
        assert_eq!(
            terms(
                &band(Measure::Mean {
                    per: Per::Match,
                    of: of(&["goals.home"])
                }),
                &[],
                Scope::All
            )
            .value(),
            None
        );
    }

    #[test]
    fn hand_values_one_match_and_equal_weights() {
        // Shots 10 and 6 with 4 and 3 on target; goals 2-1 and 0-0.
        let matches = vec![stats([2, 1], [6, 4], [3, 1]), stats([0, 0], [3, 3], [2, 1])];
        let o = obs(&matches);
        let goals = band(Measure::Mean {
            per: Per::Match,
            of: of(&["goals.home", "goals.away"]),
        });
        let t = terms(&goals, &o, Scope::All);
        assert_eq!(t.value(), Some(1.5));
        let per_team = terms(
            &band(Measure::Mean {
                per: Per::Team,
                of: of(&["shots.{side}"]),
            }),
            &o,
            Scope::All,
        );
        assert_eq!(per_team.value(), Some(4.0));
        let ratio = terms(
            &band(Measure::Ratio {
                num: of(&["shots_on_target.home", "shots_on_target.away"]),
                den: of(&["shots.home", "shots.away"]),
            }),
            &o,
            Scope::All,
        );
        assert_eq!(ratio.value(), Some(7.0 / 16.0));
        // Equal weights give the unweighted value; weights 2 and 0 give the first match.
        for t in [&t, &per_team, &ratio] {
            assert_eq!(weighted(&t.per_match(), &[3, 3]), t.value());
        }
        assert_eq!(weighted(&t.per_match(), &[2, 0]), Some(3.0));
        assert_eq!(weighted(&ratio.per_match(), &[2, 0]), Some(0.4));
        // One match: its own value, and no sampling error.
        let one = terms(&goals, &o[..1], Scope::All);
        assert_eq!(one.value(), Some(3.0));
        assert_eq!(one.se(), 0.0);
        let goalless = terms(
            &band(Measure::Share {
                of: of(&["goals.home", "goals.away"]),
                op: Op::Equal,
                value: 0.0,
            }),
            &o,
            Scope::All,
        );
        assert_eq!(goalless.value(), Some(0.5));
        assert_eq!(goalless.se(), (0.25f64 / 2.0).sqrt());
    }

    #[test]
    fn a_failed_match_and_a_match_out_of_scope_count_nothing() {
        let mut failed = stats([9, 9], [9, 9], [9, 9]);
        failed.outcome = "error".into();
        let matches = vec![stats([1, 0], [2, 2], [1, 1]), failed];
        let mut o = obs(&matches);
        o[0].pairing = Some(1);
        let goals = band(Measure::Mean {
            per: Per::Match,
            of: of(&["goals.home", "goals.away"]),
        });
        assert_eq!(terms(&goals, &o, Scope::All).value(), Some(1.0));
        assert_eq!(terms(&goals, &o, Scope::Pairing(1)).covered(), 1);
        assert_eq!(terms(&goals, &o, Scope::Pairing(0)).value(), None);
    }

    #[test]
    fn the_catalog_refuses_unknown_and_misplaced_fields() {
        let mean = |per, names: &[&str]| Measure::Mean { per, of: of(names) };
        assert!(check(&mean(Per::Match, &["goals.home", "injuries"])).is_ok());
        assert!(check(&mean(Per::Team, &["fouls.{side}"])).is_ok());
        assert!(
            check(&mean(Per::Team, &["fouls.home"]))
                .unwrap_err()
                .contains("{side}")
        );
        assert!(
            check(&mean(Per::Match, &["fouls.{side}"]))
                .unwrap_err()
                .contains("per-team")
        );
        assert!(
            check(&mean(Per::Match, &["dribbles.home"]))
                .unwrap_err()
                .contains("dribbles.home")
        );
        assert!(
            check(&mean(Per::Match, &[]))
                .unwrap_err()
                .contains("no field")
        );
        let s = MatchStats {
            laws: LawStats {
                offsides: [3, 4],
                ..LawStats::default()
            },
            ..stats([0, 0], [0, 0], [0, 0])
        };
        assert_eq!(field(&s, "offsides.away", None), Some(4.0));
        assert_eq!(field(&s, "offsides.{side}", Some(0)), Some(3.0));
        assert_eq!(field(&s, "offsides.{side}", None), None);
    }

    /// Extras with the measures present: shot distances 30 m and 12 m, ground passes under
    /// 10 m 8 of 10 (home), one corner goal and one open-play goal (home), thirds 100, 300,
    /// 600 ticks (home), and two home players who played: 3000 and 1500 ticks on, 6000 m
    /// and 2500 m, peaks 8 and 9 m/s, 50 and 10 ticks at sprint.
    fn extras(clubs: [(u32, u32); 2]) -> Extras {
        let mut m = Measures {
            present: true,
            ..Measures::default()
        };
        m.shot_distance_m = [30.0, 12.0];
        m.pass_attempts[0][0] = 10;
        m.pass_completed[0][0] = 8;
        m.goals_by_phase[0][0] = 1;
        m.goals_by_phase[0][1] = 1;
        m.third_ticks[0] = [100, 300, 600];
        m.chains = [4, 2];
        m.chain_ticks = [500, 0];
        let mut physical = [[PlayerRow::default(); PLAYER_SLOTS]; 2];
        physical[0][0] = PlayerRow {
            squad: 0,
            ticks_on: 3000,
            distance_m: 6000.0,
            zone_ticks: [10, 20, 30, 40, 50],
            peak_mps: 8.0,
        };
        physical[0][1] = PlayerRow {
            squad: 12,
            ticks_on: 1500,
            distance_m: 2500.0,
            zone_ticks: [0, 0, 0, 0, 10],
            peak_mps: 9.0,
        };
        let mut shape = [[0.0; SHAPE_FIELDS]; 2];
        shape[0][shape_place("cb.depth_in_sum").unwrap()] = 120.0;
        shape[0][shape_place("cb.depth_in_n").unwrap()] = 12.0;
        shape[1][shape_place("stops_corner").unwrap()] = 3.0;
        Extras {
            measures: m,
            physical,
            shape,
            clubs,
        }
    }

    fn with<'a>(stats: &'a MatchStats, e: &'a Extras) -> Obs<'a> {
        Obs {
            stats,
            boosted: None,
            pairing: None,
            arm: None,
            extras: Some(e),
            league: None,
        }
    }

    #[test]
    fn each_measures_field_reads_its_extras_and_none_without_them() {
        let s = stats([2, 0], [3, 1], [1, 0]);
        let e = extras([(0, 1), (0, 2)]);
        let o = with(&s, &e);
        let f = |name: &str| field_of(&o, name, None);
        assert_eq!(f("shot_distance_m.home"), Some(30.0));
        assert_eq!(f("pass_attempts.ground_lt10.home"), Some(10.0));
        assert_eq!(f("pass_completed.ground_lt10.home"), Some(8.0));
        assert_eq!(f("goals_phase.corner.home"), Some(1.0));
        assert_eq!(f("goals_phase.open_play.home"), Some(1.0));
        assert_eq!(f("third_ticks.final.home"), Some(600.0));
        assert_eq!(f("chains.home"), Some(4.0));
        assert_eq!(f("chain_seconds.home"), Some(10.0));
        assert_eq!(f("players_used.home"), Some(2.0));
        assert_eq!(f("minutes_on.home"), Some(1.5));
        assert_eq!(f("distance_m.home"), Some(8500.0));
        assert_eq!(f("peak_mps.home"), Some(17.0));
        assert_eq!(f("zone_ticks.sprint.home"), Some(60.0));
        assert_eq!(f("players_used.away"), Some(0.0));
        assert_eq!(f("shape.cb.depth_in_sum.home"), Some(120.0));
        assert_eq!(f("shape.cb.depth_in_n.home"), Some(12.0));
        assert_eq!(f("shape.stops_corner.away"), Some(3.0));
        assert_eq!(f("shape.stops_corner.home"), Some(0.0));
        assert!(is_measure_field("shape.st.sprints_fwd.{side}"));
        assert!(!is_measure_field("shape.gk.depth_in_sum.home"));
        assert!(!is_measure_field("shape.home"));
        assert!(is_measure_field("shape.w.live_min.away"));
        assert!(!is_measure_field("shape.slide.live_min.away"));
        // A catalog field still reads the band record.
        assert_eq!(f("goals.home"), Some(2.0));
        // Without extras, or with extras of a build without the measures: no value.
        let bare = obs(std::slice::from_ref(&s));
        assert_eq!(field_of(&bare[0], "shot_distance_m.home", None), None);
        let absent = Extras {
            measures: Measures::default(),
            ..e
        };
        assert_eq!(field_of(&with(&s, &absent), "chains.home", None), None);
        assert!(is_measure_field("zone_ticks.hi.{side}"));
        assert!(!is_measure_field("zone_ticks.fast.home"));
        assert!(!is_measure_field("goals.home"));
        // A band that reads the measures covers only the matches that carry them.
        let ratio = band(Measure::Ratio {
            num: of(&["shot_distance_m.home", "shot_distance_m.away"]),
            den: of(&["shots.home", "shots.away"]),
        });
        let mixed = [with(&s, &e), bare[0]];
        let t = terms(&ratio, &mixed, Scope::All);
        assert_eq!(t.covered(), 1);
        assert_eq!(t.value(), Some(42.0 / 4.0));
        let none = terms(&ratio, &bare, Scope::All);
        assert_eq!(none.value(), None);
        assert_eq!(
            none.why_none(),
            Some("no match of this run carries the measures")
        );
    }

    #[test]
    fn a_difference_a_scoreline_and_a_factorial_ratio_on_hand_values() {
        let matches = vec![
            stats([5, 1], [0, 0], [0, 0]),
            stats([1, 5], [0, 0], [0, 0]),
            stats([4, 1], [0, 0], [0, 0]),
            stats([0, 0], [0, 0], [0, 0]),
        ];
        let o = obs(&matches);
        let diff = terms(
            &band(Measure::Difference {
                of: of(&["goals.home"]),
                minus: of(&["goals.away"]),
            }),
            &o,
            Scope::All,
        );
        // 4, -4, 3 and 0 over four matches.
        assert_eq!(diff.value(), Some(0.75));
        assert_eq!(diff.se(), se_mean(&[4.0, -4.0, 3.0, 0.0]));
        let five_one = terms(
            &band(Measure::Scoreline {
                scores: vec![[5, 1]],
            }),
            &o,
            Scope::All,
        );
        assert_eq!(five_one.value(), Some(0.5), "5-1 and 1-5 hit, 4-1 misses");
        // Goals per team 5, 1, 1, 5, 4, 1, 0, 0: sum 17, sum x(x-1) 20 + 20 + 12 = 52, over
        // 8 team-matches: 8 * 52 / 17^2.
        let ratio = band(Measure::FactorialRatio {
            of: of(&["goals.{side}"]),
        });
        let t = terms(&ratio, &o, Scope::All);
        assert!((t.value().unwrap() - 8.0 * 52.0 / 289.0).abs() < 1e-12);
        assert!(t.se() > 0.0);
        // Every team scoring the same count: less spread than a Poisson count, below 1.
        let even = vec![stats([2, 2], [0, 0], [0, 0]); 5];
        let t = terms(&ratio, &obs(&even), Scope::All);
        assert_eq!(t.value(), Some(0.5));
        // The paired form gives the same value with every weight 1.
        assert_eq!(weighted(&t.per_match(), &[1; 5]), t.value());
        assert_eq!(places(&ratio.measure), 4);
    }

    #[test]
    fn each_distribution_source_gives_each_match_its_histogram() {
        use crate::report::bands::{Measure, Source};
        let dist = |of, width, bins| band(Measure::Distribution { of, width, bins });
        let mut a = stats([2, 1], [0, 0], [0, 0]);
        a.figures.passes = [400, 500];
        let b = stats([0, 0], [0, 0], [0, 0]);
        let pair = [a.clone(), b];
        let o = obs(&pair);
        let h = histograms(&dist(Source::Goals, 1.0, 11), &o, Scope::All);
        assert_eq!(h, vec![Some(vec![(3, 1.0)]), Some(vec![(0, 1.0)])]);
        let h = histograms(&dist(Source::Passes, 1.0, 1500), &o, Scope::All);
        assert_eq!(h[0], Some(vec![(400, 1.0), (500, 1.0)]));
        // 10 goals or more is the last bin; the cells cap each side.
        let mut many = stats([7, 6], [0, 0], [0, 0]);
        many.figures.passes = [2000, 0];
        let one = [many.clone()];
        let h = histograms(&dist(Source::Goals, 1.0, 11), &obs(&one), Scope::All);
        assert_eq!(h[0], Some(vec![(10, 1.0)]));
        let cells = band(Measure::Cells { max: 5 });
        let two = [a.clone(), many];
        let h = histograms(&cells, &obs(&two), Scope::All);
        assert_eq!(h, vec![Some(vec![(13, 1.0)]), Some(vec![(35, 1.0)])]);
        // A measures source copies the rows' bins, both sides summed.
        let mut e = extras([(0, 1), (0, 2)]);
        e.measures.shot_bins[0][11] = 2;
        e.measures.shot_bins[1][11] = 1;
        e.measures.shot_bins[1][40] = 1;
        e.measures.chain_second_bins[0][3] = 4;
        let with_measures = [with(&a, &e), obs(std::slice::from_ref(&a))[0]];
        let shots = dist(Source::ShotDistance, 1.0, 41);
        let h = histograms(&shots, &with_measures, Scope::All);
        assert_eq!(h[0], Some(vec![(11, 3.0), (40, 1.0)]));
        assert_eq!(h[1], None, "a match without the measures is left out");
        let h = histograms(
            &dist(Source::ChainSeconds, 2.0, 31),
            &with_measures,
            Scope::All,
        );
        assert_eq!(h[0], Some(vec![(3, 4.0)]));
        // Out of scope or not played: none.
        let h = histograms(&shots, &with_measures, Scope::Arm(1));
        assert_eq!(h, vec![None, None]);
        assert!(
            check(&Measure::Distribution {
                of: Source::Goals,
                width: 1.0,
                bins: 11
            })
            .is_ok()
        );
        assert_eq!(places(&cells.measure), 5);
        assert_eq!(metric(&cells.measure), Metric::Tv);
    }

    #[test]
    fn a_club_range_takes_each_club_over_its_window_and_leaves_short_clubs_out() {
        // Club A (0, 1) and club B (0, 2) play the first three matches; club C (0, 3) the
        // last one, against A.
        let matches = vec![
            stats([3, 1], [0, 0], [0, 0]),
            stats([0, 1], [0, 0], [0, 0]),
            stats([3, 0], [0, 0], [0, 0]),
            stats([1, 1], [0, 0], [0, 0]),
        ];
        let es = [
            extras([(0, 1), (0, 2)]),
            extras([(0, 2), (0, 1)]),
            extras([(0, 1), (0, 2)]),
            extras([(0, 3), (0, 1)]),
        ];
        let o: Vec<Obs<'_>> = matches.iter().zip(&es).map(|(s, e)| with(s, e)).collect();
        let range = band(Measure::ClubRange {
            of: of(&["goals.{side}"]),
            minus: of(&["goals.{side}"]),
            window: 3,
        });
        let t = terms(&range, &o, Scope::All);
        // A: +2, +1 (away, won 1-0), +3: range 2. B: -2, -1, -3: range 2. C has one match,
        // and A's fourth is past its window.
        assert_eq!(t.value(), Some(2.0));
        assert_eq!(t.se(), 0.0);
        assert_eq!(t.covered(), 0, "no per-match terms");
        let long = band(Measure::ClubRange {
            of: of(&["goals.{side}"]),
            minus: of(&["goals.{side}"]),
            window: 5,
        });
        let t = terms(&long, &o, Scope::All);
        assert_eq!(t.value(), None);
        assert_eq!(t.why_none(), Some("no club has 5 matches in this run"));
        // No extras: no club at all.
        let bare = obs(&matches);
        assert_eq!(terms(&range, &bare, Scope::All).value(), None);
    }
}

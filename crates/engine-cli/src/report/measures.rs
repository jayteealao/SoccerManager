//! The measures of the band registry: a closed catalog of per-match fields, read from each
//! match's band record, and the evaluator that turns a band's measure into a value over a
//! suite's matches, with or without resampling weights.
//!
//! Every measure is a ratio of weighted sums over the matches, `A / B`, times `D / C` for
//! the one measure that compares two arms (`full_over_control`): a mean is the sum of the
//! values over the count, a share the count of hits over the count, a pooled ratio the sum
//! of numerators over the sum of denominators. With every weight 1 the value is computed in
//! the order the hard-coded figures were, so it equals them exactly.

use engine::observe::MatchStats;

use super::bands::{BandDef, Builtin, Measure, Per};
use super::{se_mean, se_ratio, se_share};

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

/// The per-match fields of the catalog.
pub const MATCH: [&str; 2] = ["injuries", "ball_in_play_s"];

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

/// Why a field name is not in the catalog, or `None` when it is. `per_team` allows
/// `{side}`, and requires it.
fn field_error(name: &str, per_team: bool) -> Option<String> {
    let known = |base: &str| SIDED.contains(&base);
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
        format!("{}, {}", sided.join(", "), MATCH.join(", "))
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
    }
}

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
}

impl Terms {
    /// Matches the row covers that finished.
    pub fn covered(&self) -> usize {
        self.covered.iter().filter(|c| **c).count()
    }

    /// The value with every weight 1, before rounding; `None` when a denominator is 0.
    pub fn value(&self) -> Option<f64> {
        self.value
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
        }
    }
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
fn sum(s: &MatchStats, of: &[String], side: Option<usize>) -> f64 {
    of.iter()
        .map(|f| field(s, f, side).unwrap_or(0.0))
        .fold(0.0, |acc, v| acc + v)
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
    let take = |o: &Obs<'_>| o.played() && scope.holds(o);
    match &def.measure {
        Measure::Mean {
            per: Per::Match,
            of,
        } => {
            for o in obs {
                push(&mut t, take(o), || ([sum(o.stats, of, None)], [1.0]));
            }
        }
        Measure::Mean { per: Per::Team, of } => {
            t.stride = 2;
            for o in obs {
                push(&mut t, take(o), || {
                    let home = sum(o.stats, of, Some(0));
                    let away = sum(o.stats, of, Some(1));
                    ([home, away], [1.0, 1.0])
                });
            }
        }
        Measure::Share { of, op, value } => {
            t.error = Error::Share;
            for o in obs {
                push(&mut t, take(o), || {
                    let hit = op.holds(sum(o.stats, of, None), *value);
                    ([f64::from(u8::from(hit))], [1.0])
                });
            }
        }
        Measure::Ratio { num, den } => {
            t.error = Error::Ratio;
            for o in obs {
                push(&mut t, take(o), || {
                    ([sum(o.stats, num, None)], [sum(o.stats, den, None)])
                });
            }
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
        _ => (b > 0.0).then(|| a / b),
    };
    t
}

/// The places a value is shown to: a mean to 3, a share or a ratio to 4.
pub fn places(m: &Measure) -> i32 {
    match m {
        Measure::Mean { .. } => 3,
        _ => 4,
    }
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
}

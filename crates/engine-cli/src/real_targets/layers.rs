//! The tier and banding-group layers above the per-league targets.
//!
//! A tier target pools the leagues of that tier with equal weight. A banding group holds the
//! leagues without data: ClubElo places their clubs at a country and level, and the group is
//! that level (the tier) and a 50-point bin of the mean ClubElo rating of their clubs (the
//! strength). Each group target is the prediction of a least-squares formula
//! `stat = a + b tier + c strength`, fitted on the leagues with data, at the bin's centre;
//! its error is the formula's prediction error, so the formula's own uncertainty enters
//! the target.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::csv::Table;
use super::data_check::DIVISIONS;
use super::football_data::mean_sd;
use super::{Formula, Layer, Target, group_key, tier_key};

/// The width of a strength bin, in ClubElo points.
pub const ELO_BIN: f64 = 50.0;

/// The mean ClubElo rating of the clubs at each (country, level), and what was left out.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Strength {
    /// (country, level) → (mean rating, clubs).
    pub leagues: BTreeMap<(String, u32), (f64, u32)>,
    /// Rows left out: provisional, or without a country or a level.
    pub left_out: u32,
}

/// Reads a ClubElo snapshot.
pub fn strength(path: &Path) -> anyhow::Result<Strength> {
    let t = Table::read(path)?;
    let mut sums: BTreeMap<(String, u32), (f64, u32)> = BTreeMap::new();
    let mut left_out = 0;
    for row in &t.rows {
        let country = t.get(row, "Country");
        let level = t.get(row, "Level").parse::<u32>().ok();
        let elo = t.num(row, "Elo");
        let provisional = t.get(row, "Provisional").eq_ignore_ascii_case("true");
        match (country.is_empty(), level, elo, provisional) {
            (false, Some(level), Some(elo), false) => {
                let e = sums.entry((country.to_string(), level)).or_default();
                e.0 += elo;
                e.1 += 1;
            }
            _ => left_out += 1,
        }
    }
    Ok(Strength {
        leagues: sums
            .into_iter()
            .map(|(k, (s, n))| (k, (s / f64::from(n), n)))
            .collect(),
        left_out,
    })
}

/// The lower edge of the strength bin of `strength`.
pub fn elo_bin(strength: f64) -> i64 {
    ((strength / ELO_BIN).floor() * ELO_BIN) as i64
}

/// The key prefix of the banding group of a league at `tier` with mean rating `strength`:
/// `group.t<tier>.e<bin>`; a target adds `.<stat>`.
pub fn group_of(tier: u32, strength: f64) -> String {
    let key = group_key(tier, elo_bin(strength), "");
    key.trim_end_matches('.').to_string()
}

/// A least-squares fit of `y = a + b tier + c strength`.
#[derive(Debug, Clone, PartialEq)]
pub struct Fit {
    pub coef: [f64; 3],
    /// `(X'X)^-1`.
    inverse: [[f64; 3]; 3],
    pub residual_sd: f64,
    pub tiers: (f64, f64),
    pub strengths: (f64, f64),
}

/// The inverse of a 3 x 3 matrix; `None` when it is singular.
fn invert(m: [[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let c =
        |r1: usize, c1: usize, r2: usize, c2: usize| m[r1][c1] * m[r2][c2] - m[r1][c2] * m[r2][c1];
    let cof = [
        [c(1, 1, 2, 2), -c(1, 0, 2, 2), c(1, 0, 2, 1)],
        [-c(0, 1, 2, 2), c(0, 0, 2, 2), -c(0, 0, 2, 1)],
        [c(0, 1, 1, 2), -c(0, 0, 1, 2), c(0, 0, 1, 1)],
    ];
    let det = m[0][0] * cof[0][0] + m[0][1] * cof[0][1] + m[0][2] * cof[0][2];
    // Singular relative to the diagonal's own size, so the test does not depend on the
    // scale of the ratings.
    if det.abs() <= 1e-12 * (m[0][0] * m[1][1] * m[2][2]).abs() {
        return None;
    }
    let mut inv = [[0.0; 3]; 3];
    for (i, row) in inv.iter_mut().enumerate() {
        for (j, x) in row.iter_mut().enumerate() {
            // The adjugate is the transposed cofactor matrix.
            *x = cof[j][i] / det;
        }
    }
    Some(inv)
}

/// Fits `points` of `(tier, strength, y)`; `None` with fewer than four points or a design
/// that cannot separate the three terms.
pub fn fit(points: &[(f64, f64, f64)]) -> Option<Fit> {
    if points.len() < 4 {
        return None;
    }
    let mut xtx = [[0.0; 3]; 3];
    let mut xty = [0.0; 3];
    for &(t, e, y) in points {
        let x = [1.0, t, e];
        for i in 0..3 {
            xty[i] += x[i] * y;
            for j in 0..3 {
                xtx[i][j] += x[i] * x[j];
            }
        }
    }
    let inverse = invert(xtx)?;
    let coef: [f64; 3] =
        std::array::from_fn(|i| (0..3).map(|j| inverse[i][j] * xty[j]).sum::<f64>());
    let rss: f64 = points
        .iter()
        .map(|&(t, e, y)| (y - (coef[0] + coef[1] * t + coef[2] * e)).powi(2))
        .sum();
    let span = |f: fn(&(f64, f64, f64)) -> f64| {
        points
            .iter()
            .map(f)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            })
    };
    Some(Fit {
        coef,
        inverse,
        residual_sd: (rss / (points.len() as f64 - 3.0)).sqrt(),
        tiers: span(|p| p.0),
        strengths: span(|p| p.1),
    })
}

impl Fit {
    /// The prediction at `(tier, strength)` and its prediction error,
    /// `s sqrt(1 + x0' (X'X)^-1 x0)`.
    pub fn predict(&self, tier: f64, strength: f64) -> (f64, f64) {
        let x = [1.0, tier, strength];
        let value = (0..3).map(|i| self.coef[i] * x[i]).sum();
        let lever: f64 = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| x[i] * self.inverse[i][j] * x[j])
                    .sum::<f64>()
            })
            .sum();
        (value, self.residual_sd * (1.0 + lever).sqrt())
    }

    /// `true` when `(tier, strength)` lies outside the range the fit saw.
    pub fn extrapolates(&self, tier: f64, strength: f64) -> bool {
        tier < self.tiers.0
            || tier > self.tiers.1
            || strength < self.strengths.0
            || strength > self.strengths.1
    }
}

/// The statistic of a league key `fd.<div>.<stat>`.
fn split_league(key: &str) -> Option<(&str, &str)> {
    key.strip_prefix("fd.")?.split_once('.')
}

/// The tier targets: for each statistic, the leagues of each tier with equal weight. A
/// scalar's error combines the leagues' errors; its spread between leagues is kept beside
/// it. A distribution averages the leagues' shares.
pub fn tier_targets(targets: &mut BTreeMap<String, Target>) {
    let tiers: BTreeMap<&str, u32> = DIVISIONS.iter().map(|(d, _, t)| (*d, *t)).collect();
    let mut groups: BTreeMap<(u32, String), Vec<(String, Target)>> = BTreeMap::new();
    for (key, t) in targets.iter() {
        let Some((div, stat)) = split_league(key) else {
            continue;
        };
        let Some(&tier) = tiers.get(div) else {
            continue;
        };
        groups
            .entry((tier, stat.to_string()))
            .or_default()
            .push((div.to_string(), t.clone()));
    }
    for ((tier, stat), list) in groups {
        let k = list.len() as f64;
        let mut t = list[0].1.clone();
        t.layer = Layer::Tier;
        t.leagues = Some(list.iter().map(|(d, _)| d.clone()).collect());
        t.n = list.iter().map(|(_, t)| t.n).sum();
        t.seasons = list
            .iter()
            .flat_map(|(_, t)| t.seasons.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        t.between_season_sd = None;
        if let Some(first) = &list[0].1.shares {
            let mut shares = vec![0.0; first.len()];
            let mut var = vec![0.0; first.len()];
            for (_, lt) in &list {
                let s = lt.shares.as_deref().unwrap_or(&[]);
                for (i, p) in s.iter().enumerate().take(shares.len()) {
                    shares[i] += p / k;
                    var[i] += p * (1.0 - p) / lt.n as f64 / (k * k);
                }
            }
            t.se = var.iter().fold(0.0, |a, v| a.max(v.sqrt()));
            t.shares = Some(shares);
        } else {
            let values: Vec<f64> = list.iter().filter_map(|(_, t)| t.value).collect();
            t.value = Some(values.iter().sum::<f64>() / k);
            t.se = list.iter().map(|(_, t)| t.se * t.se).sum::<f64>().sqrt() / k;
            t.between_league_sd = mean_sd(&values).map(|(_, s)| s);
        }
        targets.insert(tier_key(tier, &stat), t);
    }
}

/// The banding-group targets of every scalar statistic, from the formula fitted on the
/// divisions with data and a strength. Returns the leagues each group holds.
pub fn group_targets(
    targets: &mut BTreeMap<String, Target>,
    absent: &mut BTreeMap<String, String>,
    strength: &Strength,
    division_strength: &BTreeMap<String, f64>,
) -> BTreeMap<String, Vec<String>> {
    let tiers: BTreeMap<&str, u32> = DIVISIONS.iter().map(|(d, _, t)| (*d, *t)).collect();
    let covered: BTreeSet<(String, u32)> = DIVISIONS
        .iter()
        .map(|(_, c, t)| ((*c).to_string(), *t))
        .collect();
    // The leagues without data, grouped by tier and strength bin.
    let mut groups: BTreeMap<(u32, i64), Vec<String>> = BTreeMap::new();
    for ((country, level), (mean, _)) in &strength.leagues {
        if !covered.contains(&(country.clone(), *level)) {
            groups
                .entry((*level, elo_bin(*mean)))
                .or_default()
                .push(format!("{country}-{level}"));
        }
    }
    // The scalar statistics with league targets.
    let mut by_stat: BTreeMap<String, Vec<(String, Target)>> = BTreeMap::new();
    for (key, t) in targets.iter() {
        if let Some((div, stat)) = split_league(key) {
            if t.value.is_some() {
                by_stat
                    .entry(stat.to_string())
                    .or_default()
                    .push((div.to_string(), t.clone()));
            }
        }
    }
    let mut out = BTreeMap::new();
    for (stat, list) in by_stat {
        let fitted: Vec<(String, (f64, f64, f64))> = list
            .iter()
            .filter_map(|(d, t)| {
                let e = division_strength.get(d)?;
                Some((
                    d.clone(),
                    (f64::from(*tiers.get(d.as_str())?), *e, t.value?),
                ))
            })
            .collect();
        let points: Vec<(f64, f64, f64)> = fitted.iter().map(|(_, p)| *p).collect();
        let Some(f) = fit(&points) else {
            for (tier, bin) in groups.keys() {
                absent.insert(
                    group_key(*tier, *bin, &stat),
                    format!(
                        "too few leagues with data and a strength to fit the formula ({})",
                        points.len()
                    ),
                );
            }
            continue;
        };
        let template = &list[0].1;
        for ((tier, bin), leagues) in &groups {
            let centre = *bin as f64 + ELO_BIN / 2.0;
            let (value, se) = f.predict(f64::from(*tier), centre);
            let mut t = template.clone();
            t.value = Some(value);
            t.se = se;
            t.n = fitted.len() as u64;
            t.layer = Layer::Group;
            t.leagues = Some(leagues.clone());
            t.between_season_sd = None;
            t.between_league_sd = None;
            t.extrapolated = Some(f.extrapolates(f64::from(*tier), centre));
            t.formula = Some(Formula {
                intercept: f.coef[0],
                tier: f.coef[1],
                strength: f.coef[2],
                residual_sd: f.residual_sd,
                fitted_on: fitted.iter().map(|(d, _)| d.clone()).collect(),
            });
            let key = group_key(*tier, *bin, &stat);
            out.insert(group_of(*tier, *bin as f64), leagues.clone());
            targets.insert(key, t);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_exact_plane_is_fitted_exactly() {
        let plane = |t: f64, e: f64| 4.0 - 0.3 * t + 0.002 * e;
        let points: Vec<_> = [(1.0, 1500.0), (2.0, 1300.0), (1.0, 1700.0), (3.0, 1250.0)]
            .iter()
            .map(|&(t, e)| (t, e, plane(t, e)))
            .collect();
        let f = fit(&points).unwrap();
        for (got, want) in f.coef.iter().zip([4.0, -0.3, 0.002]) {
            assert!((got - want).abs() < 1e-9, "{:?}", f.coef);
        }
        assert!(f.residual_sd < 1e-9);
    }

    #[test]
    fn the_prediction_error_grows_away_from_the_data_and_far_points_extrapolate() {
        let points = [
            (1.0, 1500.0, 2.9),
            (2.0, 1350.0, 2.6),
            (1.0, 1650.0, 2.8),
            (2.0, 1400.0, 2.5),
            (3.0, 1300.0, 2.7),
        ];
        let f = fit(&points).unwrap();
        let (_, near) = f.predict(2.0, 1440.0);
        let (_, far) = f.predict(4.0, 1000.0);
        assert!(far > near && near > 0.0, "{near} {far}");
        assert!(!f.extrapolates(2.0, 1440.0));
        assert!(f.extrapolates(4.0, 1000.0));
    }

    #[test]
    fn three_points_cannot_be_fitted_and_groups_name_their_bin() {
        assert!(fit(&[(1.0, 1.0, 1.0), (2.0, 2.0, 2.0), (3.0, 1.0, 3.0)]).is_none());
        assert_eq!(group_of(2, 1449.9), "group.t2.e1400");
        assert_eq!(group_of(1, 1450.0), "group.t1.e1450");
    }
}

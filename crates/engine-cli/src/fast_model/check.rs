//! The equality check: the fast model against the full engine on the held-out check batch.
//!
//! For each check-batch match the fast model plays `draws` seeded matches from the same
//! kick-off. Compared, full engine against fast model: per pairing the home win, draw and
//! away win shares and the home and away goals per match (9 x 5 = 45 figures); over the
//! equal-strength pairings the season figures goals per match, the goalless share and the
//! share of matches with ten or more goals; and over 1.15 v 1.00 in both orders the
//! stronger side's win rate (4 figures). A figure passes when the two differ by at most
//! max(floor, z x the combined standard error), each standard error from that side's own
//! distribution; z = 3.7 bounds the chance that a fast model equal to the full engine fails
//! any of the 49 figures at about 1 percent.

use engine::modules::fast_model::{FastFit, FastModel};
use serde::{Deserialize, Serialize};

use super::batch::{PAIRINGS, Row, pairing_name};
use crate::calibrate::fixtures::splitmix64;

/// The z of every tolerance.
pub const Z: f64 = 3.7;
/// The smallest tolerance of a share.
pub const SHARE_FLOOR: f64 = 0.01;
/// The smallest tolerance of goals per match.
pub const MEAN_FLOOR: f64 = 0.03;
/// The figures the check compares.
pub const FIGURES: usize = PAIRINGS.len() * 5 + 4;

/// A realism band's range, shown beside a season figure for information only.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Range {
    pub lo: f64,
    pub hi: f64,
}

/// One compared figure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Figure {
    /// `1.150 v 1.000` for a pairing, `season` for a season figure.
    pub group: String,
    pub name: String,
    pub full: f64,
    pub fast: f64,
    pub tolerance: f64,
    pub pass: bool,
    /// The realism band's range, for information; the check never requires it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub band: Option<Range>,
}

/// The fast model's scores for every row: `draws` per row, in row order.
pub fn play_fast(
    model: &dyn FastModel,
    fit: &FastFit,
    rows: &[Row],
    draws: u32,
    seed: u64,
) -> anyhow::Result<Vec<Vec<[u32; 2]>>> {
    rows.iter()
        .enumerate()
        .map(|(i, r)| {
            (0..draws)
                .map(|d| {
                    let s = splitmix64(
                        (0xfa57 << 48)
                            ^ (seed << 40)
                            ^ (i as u64 * u64::from(draws) + u64::from(d)),
                    );
                    Ok(model.play(fit, &r.kick_off, s)?.scores)
                })
                .collect()
        })
        .collect()
}

/// A sample of values: its mean and the standard error of the mean.
fn stats(values: &[f64]) -> (f64, f64) {
    let n = values.len() as f64;
    if n == 0.0 {
        return (0.0, 0.0);
    }
    let mean = values.iter().sum::<f64>() / n;
    let var = if n > 1.0 {
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0)
    } else {
        0.0
    };
    (mean, (var / n).sqrt())
}

/// A share's standard error is sqrt(p (1 - p) / n); a mean's is s / sqrt(n).
fn figure(group: String, name: &str, full: &[f64], fast: &[f64], share: bool) -> Figure {
    let (f, se_f) = if share {
        share_stats(full)
    } else {
        stats(full)
    };
    let (m, se_m) = if share {
        share_stats(fast)
    } else {
        stats(fast)
    };
    let floor = if share { SHARE_FLOOR } else { MEAN_FLOOR };
    let tolerance = floor.max(Z * (se_f * se_f + se_m * se_m).sqrt());
    Figure {
        group,
        name: name.to_string(),
        full: f,
        fast: m,
        tolerance,
        pass: (f - m).abs() <= tolerance,
        band: None,
    }
}

fn share_stats(values: &[f64]) -> (f64, f64) {
    let n = values.len() as f64;
    if n == 0.0 {
        return (0.0, 0.0);
    }
    let p = values.iter().sum::<f64>() / n;
    (p, (p * (1.0 - p) / n).sqrt())
}

/// The season figures' band ranges, by name, when the bands file is at hand.
pub type Bands = [(&'static str, Option<Range>); 4];

/// A per-pairing figure: its name, its value for one score, and whether it is a share.
type ScoreRule<'a> = (&'static str, &'a dyn Fn(&[u32; 2]) -> f64, bool);
/// A season figure: its name, the pairings it pools, its value for one score of a pairing,
/// and whether it is a share.
type SeasonRule<'a> = (
    &'static str,
    &'a dyn Fn(usize) -> bool,
    &'a dyn Fn(usize, &[u32; 2]) -> f64,
    bool,
);

/// Compares the full engine's `rows` with the fast model's `fast` scores (`fast[i]` for
/// `rows[i]`).
pub fn compare(rows: &[Row], fast: &[Vec<[u32; 2]>], bands: &Bands) -> Vec<Figure> {
    let mut out = Vec::with_capacity(FIGURES);
    let flag = |b: bool| if b { 1.0 } else { 0.0 };
    for pairing in 0..PAIRINGS.len() {
        let full: Vec<[u32; 2]> = rows
            .iter()
            .filter(|r| r.pairing == pairing)
            .map(|r| r.goals)
            .collect();
        let quick: Vec<[u32; 2]> = rows
            .iter()
            .zip(fast)
            .filter(|(r, _)| r.pairing == pairing)
            .flat_map(|(_, f)| f.iter().copied())
            .collect();
        let name = pairing_name(pairing);
        let per = |f: &dyn Fn(&[u32; 2]) -> f64, list: &[[u32; 2]]| -> Vec<f64> {
            list.iter().map(f).collect()
        };
        let rules: [ScoreRule; 5] = [
            ("home_win_share", &|g| flag(g[0] > g[1]), true),
            ("draw_share", &|g| flag(g[0] == g[1]), true),
            ("away_win_share", &|g| flag(g[0] < g[1]), true),
            ("home_goals_per_match", &|g| f64::from(g[0]), false),
            ("away_goals_per_match", &|g| f64::from(g[1]), false),
        ];
        for (figure_name, f, share) in rules {
            out.push(figure(
                name.clone(),
                figure_name,
                &per(f, &full),
                &per(f, &quick),
                share,
            ));
        }
    }

    let equal = |p: usize| PAIRINGS[p][0] == PAIRINGS[p][1];
    let pick = |keep: &dyn Fn(usize) -> bool, f: &dyn Fn(usize, &[u32; 2]) -> f64| {
        let full: Vec<f64> = rows
            .iter()
            .filter(|r| keep(r.pairing))
            .map(|r| f(r.pairing, &r.goals))
            .collect();
        let quick: Vec<f64> = rows
            .iter()
            .zip(fast)
            .filter(|(r, _)| keep(r.pairing))
            .flat_map(|(r, list)| list.iter().map(|g| f(r.pairing, g)))
            .collect();
        (full, quick)
    };
    let top = PAIRINGS
        .iter()
        .map(|p| p[0].max(p[1]))
        .max()
        .unwrap_or_default();
    let stronger = |p: usize| {
        let [h, a] = PAIRINGS[p];
        (h == top && a == 0) || (h == 0 && a == top)
    };
    let season: [SeasonRule; 4] = [
        (
            "goals_per_match",
            &equal,
            &|_, g| f64::from(g[0] + g[1]),
            false,
        ),
        ("goalless_share", &equal, &|_, g| flag(*g == [0, 0]), true),
        (
            "ten_plus_goals_share",
            &equal,
            &|_, g| flag(g[0] + g[1] >= 10),
            true,
        ),
        (
            "stronger_team_win_rate",
            &stronger,
            &|p, g| {
                let side = usize::from(PAIRINGS[p][1] > PAIRINGS[p][0]);
                flag(g[side] > g[1 - side])
            },
            true,
        ),
    ];
    for (name, keep, f, share) in season {
        let (full, quick) = pick(keep, f);
        let mut fig = figure("season".into(), name, &full, &quick, share);
        fig.band = bands.iter().find(|(n, _)| *n == name).and_then(|(_, r)| *r);
        out.push(fig);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::modules::fast_model::KickOff;

    fn rows_from(scores: &dyn Fn(usize, usize) -> [u32; 2], per: usize) -> Vec<Row> {
        (0..PAIRINGS.len())
            .flat_map(|p| {
                (0..per).map(move |k| Row {
                    pairing: p,
                    kick_off: KickOff::even([50.0, 50.0]),
                    goals: scores(p, k),
                    goal_minutes: Vec::new(),
                })
            })
            .collect()
    }

    fn base(_: usize, k: usize) -> [u32; 2] {
        match k % 10 {
            0..=3 => [1, 0],
            4..=6 => [1, 1],
            7 => [0, 0],
            _ => [0, 2],
        }
    }

    const NO_BANDS: Bands = [
        ("goals_per_match", None),
        ("goalless_share", None),
        ("ten_plus_goals_share", None),
        ("stronger_team_win_rate", None),
    ];

    #[test]
    fn equal_tables_pass_every_figure() {
        let rows = rows_from(&base, 1000);
        let fast: Vec<Vec<[u32; 2]>> = rows.iter().map(|r| vec![r.goals; 20]).collect();
        let figures = compare(&rows, &fast, &NO_BANDS);
        assert_eq!(figures.len(), FIGURES);
        assert!(figures.iter().all(|f| f.pass), "{figures:?}");
    }

    /// A fast model whose draw share is 0.1 lower in one pairing fails, and the failing
    /// figure names the pairing.
    #[test]
    fn a_planted_draw_share_shift_fails_and_names_the_pairing() {
        let rows = rows_from(&base, 1000);
        let fast: Vec<Vec<[u32; 2]>> = rows
            .iter()
            .enumerate()
            .map(|(i, r)| {
                // In pairing 6, one draw in ten turns into a home win.
                if r.pairing == 6 && i % 10 == 4 {
                    vec![[1, 0]; 20]
                } else {
                    vec![r.goals; 20]
                }
            })
            .collect();
        let figures = compare(&rows, &fast, &NO_BANDS);
        let failed: Vec<&Figure> = figures.iter().filter(|f| !f.pass).collect();
        assert!(
            failed
                .iter()
                .any(|f| f.group == "1.150 v 1.000" && f.name == "draw_share"),
            "{failed:?}"
        );
        assert!(
            failed
                .iter()
                .all(|f| f.group == "1.150 v 1.000" || f.group == "season")
        );
    }
}

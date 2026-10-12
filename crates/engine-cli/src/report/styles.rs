//! The report of the styles suite: every ordered pairing of two distinct named styles,
//! played with identical squads. Per style it gives the points per match, the share of all
//! points, the styles it loses to clearly, the dominance flag and the identity flags; per
//! ordered pairing the points of each side. The report only describes: no band, verdict or
//! exit code reads it.
//!
//! A style loses clearly to another when the 95 percent interval of the mean points
//! difference over their matches, in both home orders, lies wholly below zero. A style is
//! flagged as dominant when it loses clearly to no other style, or when it takes more than
//! [`SHARE_CAP`] of all the points. An identity statistic is flagged when the style's sides
//! do not move it in the style's direction by more than 1.96 standard errors against the
//! mean over every side.

use engine::data::styles::{Direction, IDENTITY_STATS, IdentityEntry};
use serde::Serialize;

/// A style that takes more than this share of all the points is flagged.
pub const SHARE_CAP: f64 = 0.20;

/// The two-sided 95 percent normal quantile.
pub const Z95: f64 = 1.96;

/// One match of the suite, as the report reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleMatch {
    /// Each side's style, by its place in the styles file, home first.
    pub styles: [usize; 2],
    /// Each side's formation name, home first.
    pub formations: [String; 2],
    pub goals: [u32; 2],
    /// Each side's value of every statistic in [`IDENTITY_STATS`] order; `NaN` where a
    /// side has none (the on-target share of a side without a shot).
    pub stats: [[f64; IDENTITY_STATS.len()]; 2],
}

impl StyleMatch {
    /// Points for each side: 3 for a win, 1 for a draw.
    pub fn points(&self) -> [f64; 2] {
        match self.goals[0].cmp(&self.goals[1]) {
            std::cmp::Ordering::Greater => [3.0, 0.0],
            std::cmp::Ordering::Less => [0.0, 3.0],
            std::cmp::Ordering::Equal => [1.0, 1.0],
        }
    }
}

/// One style's identity statistic: its sides' mean beside every side's mean.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IdentityRow {
    pub stat: String,
    /// The other side's figure is read.
    pub against: bool,
    pub direction: &'static str,
    pub style_mean: f64,
    pub all_mean: f64,
    /// The standard error of the difference.
    pub se: f64,
    /// The style does not move the statistic in its direction by more than 1.96 errors.
    pub flagged: bool,
}

/// One style's row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StyleRow {
    pub style: String,
    pub matches: u32,
    pub points_per_match: f64,
    pub se: f64,
    /// The style's points over all the points of the suite.
    pub share: f64,
    /// The styles this one loses to clearly.
    pub clear_losses: Vec<String>,
    /// No clear loss, or a share above the cap.
    pub dominant: bool,
    pub identity: Vec<IdentityRow>,
}

/// One ordered pairing's row: the first style at home.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PairingRow {
    pub home: String,
    pub away: String,
    pub matches: u32,
    pub home_points_per_match: f64,
    pub away_points_per_match: f64,
    pub goals_per_match: f64,
    pub shots_per_match: f64,
}

/// The suite's report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StylesReport {
    /// Matches planned per ordered pairing.
    pub matches_per_pairing: u32,
    /// Matches planned, and those that played to full time.
    pub matches: u32,
    pub played: u32,
    /// Rule violations the running checker found over the suite.
    pub violations: usize,
    pub styles: Vec<StyleRow>,
    pub pairings: Vec<PairingRow>,
    /// The styles flagged as dominant.
    pub dominant: Vec<String>,
    /// Each flagged identity statistic, as `style: stat`.
    pub identity_flags: Vec<String>,
    /// The suite's wall time, in milliseconds.
    pub wall_ms: u64,
}

/// The mean of `xs`, 0 when empty.
fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        0.0
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}

/// The standard error of the mean of `xs` with the sample standard deviation; 0 below two.
fn se(xs: &[f64]) -> f64 {
    let n = xs.len();
    if n < 2 {
        return 0.0;
    }
    let m = mean(xs);
    let var = xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n as f64 - 1.0);
    (var / n as f64).sqrt()
}

fn round(x: f64, places: i32) -> f64 {
    let f = 10f64.powi(places);
    (x * f).round() / f
}

/// Whether `a` loses clearly to `b`: over every match between them, in both home orders, at
/// least two matches and the 95 percent interval of `a`'s points minus `b`'s wholly below 0.
pub fn clear_loss(matches: &[StyleMatch], a: usize, b: usize) -> bool {
    let diffs: Vec<f64> = matches
        .iter()
        .filter_map(|m| {
            let p = m.points();
            match m.styles {
                [h, w] if h == a && w == b => Some(p[0] - p[1]),
                [h, w] if h == b && w == a => Some(p[1] - p[0]),
                _ => None,
            }
        })
        .collect();
    diffs.len() >= 2 && mean(&diffs) + Z95 * se(&diffs) < 0.0
}

/// Per style: its share of all points and the styles it loses to clearly; and whether it is
/// dominant (no clear loss, or a share above [`SHARE_CAP`]).
pub fn dominance(styles: usize, matches: &[StyleMatch]) -> Vec<(f64, Vec<usize>, bool)> {
    let mut points = vec![0.0; styles];
    for m in matches {
        let p = m.points();
        for side in 0..2 {
            points[m.styles[side]] += p[side];
        }
    }
    let total: f64 = points.iter().sum();
    (0..styles)
        .map(|a| {
            let share = if total > 0.0 { points[a] / total } else { 0.0 };
            let losses: Vec<usize> = (0..styles)
                .filter(|&b| b != a && clear_loss(matches, a, b))
                .collect();
            let dominant = losses.is_empty() || share > SHARE_CAP;
            (share, losses, dominant)
        })
        .collect()
}

/// The identity row of `entry` for `style`.
fn identity_row(matches: &[StyleMatch], style: usize, entry: &IdentityEntry) -> IdentityRow {
    let k = IDENTITY_STATS
        .iter()
        .position(|s| *s == entry.stat)
        .unwrap_or(0);
    let read = |m: &StyleMatch, side: usize| {
        let from = if entry.against { 1 - side } else { side };
        m.stats[from][k]
    };
    let mut own = Vec::new();
    let mut all = Vec::new();
    for m in matches {
        for side in 0..2 {
            let v = read(m, side);
            if v.is_nan() {
                continue;
            }
            all.push(v);
            if m.styles[side] == style {
                own.push(v);
            }
        }
    }
    let (style_mean, all_mean) = (mean(&own), mean(&all));
    let error = (se(&own).powi(2) + se(&all).powi(2)).sqrt();
    let moved = match entry.direction {
        Direction::Up => style_mean - all_mean,
        Direction::Down => all_mean - style_mean,
    };
    IdentityRow {
        stat: entry.stat.clone(),
        against: entry.against,
        direction: match entry.direction {
            Direction::Up => "up",
            Direction::Down => "down",
        },
        style_mean: round(style_mean, 3),
        all_mean: round(all_mean, 3),
        se: round(error, 3),
        flagged: own.len() < 2 || moved <= Z95 * error,
    }
}

/// The report over `matches`, with the styles' names and identity lists in file order.
pub fn build(
    names: &[String],
    identities: &[Vec<IdentityEntry>],
    matches_per_pairing: u32,
    planned: u32,
    matches: &[StyleMatch],
    violations: usize,
    wall_ms: u64,
) -> StylesReport {
    let dom = dominance(names.len(), matches);
    let styles: Vec<StyleRow> = names
        .iter()
        .enumerate()
        .map(|(s, name)| {
            let pts: Vec<f64> = matches
                .iter()
                .flat_map(|m| {
                    let p = m.points();
                    (0..2)
                        .filter(move |&side| m.styles[side] == s)
                        .map(move |side| p[side])
                })
                .collect();
            let (share, losses, dominant) = &dom[s];
            StyleRow {
                style: name.clone(),
                matches: u32::try_from(pts.len()).unwrap_or(u32::MAX),
                points_per_match: round(mean(&pts), 3),
                se: round(se(&pts), 3),
                share: round(*share, 4),
                clear_losses: losses.iter().map(|&b| names[b].clone()).collect(),
                dominant: *dominant,
                identity: identities[s]
                    .iter()
                    .map(|e| identity_row(matches, s, e))
                    .collect(),
            }
        })
        .collect();
    let shots = IDENTITY_STATS
        .iter()
        .position(|s| *s == "shots")
        .unwrap_or(0);
    let mut pairings = Vec::new();
    for (h, home) in names.iter().enumerate() {
        for (w, away) in names.iter().enumerate() {
            if h == w {
                continue;
            }
            let these: Vec<&StyleMatch> = matches.iter().filter(|m| m.styles == [h, w]).collect();
            let col = |f: &dyn Fn(&StyleMatch) -> f64| {
                round(mean(&these.iter().map(|m| f(m)).collect::<Vec<_>>()), 3)
            };
            pairings.push(PairingRow {
                home: home.clone(),
                away: away.clone(),
                matches: u32::try_from(these.len()).unwrap_or(u32::MAX),
                home_points_per_match: col(&|m| m.points()[0]),
                away_points_per_match: col(&|m| m.points()[1]),
                goals_per_match: col(&|m| f64::from(m.goals[0] + m.goals[1])),
                shots_per_match: col(&|m| m.stats[0][shots] + m.stats[1][shots]),
            });
        }
    }
    let dominant = styles
        .iter()
        .filter(|r| r.dominant)
        .map(|r| r.style.clone())
        .collect();
    let identity_flags = styles
        .iter()
        .flat_map(|r| {
            r.identity.iter().filter(|i| i.flagged).map(move |i| {
                let against = if i.against { " against" } else { "" };
                format!("{}: {}{against}", r.style, i.stat)
            })
        })
        .collect();
    StylesReport {
        matches_per_pairing,
        matches: planned,
        played: u32::try_from(matches.len()).unwrap_or(u32::MAX),
        violations,
        styles,
        pairings,
        dominant,
        identity_flags,
        wall_ms,
    }
}

/// The report as a console table: one line per style, then the flags.
pub fn render(r: &StylesReport) -> String {
    let mut out = format!(
        "styles suite: {} of {} matches played, {} per ordered pairing, {} rule violations\n\
         style            pts/match     se   share  clear losses\n",
        r.played, r.matches, r.matches_per_pairing, r.violations
    );
    for s in &r.styles {
        let losses = if s.clear_losses.is_empty() {
            "none".to_string()
        } else {
            s.clear_losses.join(", ")
        };
        let flag = if s.dominant { "  DOMINANT" } else { "" };
        out.push_str(&format!(
            "{:<16} {:>9.3} {:>6.3} {:>7.3}  {losses}{flag}\n",
            s.style, s.points_per_match, s.se, s.share
        ));
    }
    let list = |v: &[String]| {
        if v.is_empty() {
            "none".to_string()
        } else {
            v.join("; ")
        }
    };
    out.push_str(&format!("dominant: {}\n", list(&r.dominant)));
    out.push_str(&format!("identity flags: {}\n", list(&r.identity_flags)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const STYLES: usize = 8;

    fn game(styles: [usize; 2], goals: [u32; 2]) -> StyleMatch {
        StyleMatch {
            styles,
            formations: ["4-4-2".into(), "4-4-2".into()],
            goals,
            stats: [[0.0; IDENTITY_STATS.len()]; 2],
        }
    }

    /// Every ordered pairing `per` times, the result given by `score(home, away)`.
    fn table(per: u32, score: impl Fn(usize, usize) -> [u32; 2]) -> Vec<StyleMatch> {
        let mut out = Vec::new();
        for h in 0..STYLES {
            for w in 0..STYLES {
                if h != w {
                    for _ in 0..per {
                        out.push(game([h, w], score(h, w)));
                    }
                }
            }
        }
        out
    }

    /// AC-35, guarded failure: a style that beats everyone loses clearly to no one and takes
    /// a third of the points; the check must flag it, and only it, when the rest draw.
    #[test]
    fn a_planted_dominant_style_is_flagged() {
        let planted = table(5, |h, w| match (h, w) {
            (0, _) => [2, 0],
            (_, 0) => [0, 2],
            _ => [1, 1],
        });
        let dom = dominance(STYLES, &planted);
        assert!(dom[0].2, "the planted style is flagged");
        assert!(dom[0].1.is_empty(), "it loses clearly to no one");
        assert!(dom[0].0 > SHARE_CAP, "share {}", dom[0].0);
        for (s, (_, losses, _)) in dom.iter().enumerate().skip(1) {
            assert_eq!(
                losses,
                &vec![0],
                "style {s} loses clearly to the planted one"
            );
        }
    }

    /// AC-35: a balanced table, where each style beats the next one round the circle and
    /// draws the rest, flags none: every style has a clear loss and an equal share.
    #[test]
    fn a_balanced_table_flags_none() {
        let balanced = table(5, |h, w| {
            if (h + 1) % STYLES == w {
                [2, 1]
            } else if (w + 1) % STYLES == h {
                [1, 2]
            } else {
                [0, 0]
            }
        });
        let dom = dominance(STYLES, &balanced);
        for (s, (share, losses, dominant)) in dom.iter().enumerate() {
            assert!(
                !dominant,
                "style {s} flagged: share {share}, losses {losses:?}"
            );
            assert_eq!(losses, &vec![(s + STYLES - 1) % STYLES]);
            assert!((share - 1.0 / STYLES as f64).abs() < 1e-9);
        }
    }

    /// A share above the cap flags a style even when it has a clear loss.
    #[test]
    fn a_share_above_the_cap_flags_a_style_with_a_weakness() {
        let skewed = table(5, |h, w| match (h, w) {
            (0, 1) | (1, 0) => {
                if h == 1 {
                    [3, 0]
                } else {
                    [0, 3]
                }
            }
            (0, _) => [2, 0],
            (_, 0) => [0, 2],
            _ => [1, 1],
        });
        let dom = dominance(STYLES, &skewed);
        assert_eq!(dom[0].1, vec![1], "style 0 loses clearly to style 1");
        assert!(dom[0].0 > SHARE_CAP && dom[0].2);
    }

    /// One match per pairing never makes a clear loss: an interval needs two matches.
    #[test]
    fn a_single_match_is_no_clear_loss() {
        let one = vec![game([0, 1], [0, 3])];
        assert!(!clear_loss(&one, 0, 1));
        assert!(!clear_loss(&one, 1, 0));
    }

    /// An identity statistic moved in the style's direction is not flagged; one that is not
    /// moved is.
    #[test]
    fn identity_flags_follow_the_direction() {
        let k = IDENTITY_STATS
            .iter()
            .position(|s| *s == "possession_pct")
            .unwrap();
        let mut matches = table(10, |_, _| [1, 1]);
        for (i, m) in matches.iter_mut().enumerate() {
            let jitter = (i % 3) as f64;
            for side in 0..2 {
                m.stats[side][k] = if m.styles[side] == 0 {
                    60.0 + jitter
                } else {
                    48.0 + jitter
                };
            }
        }
        let up = IdentityEntry {
            stat: "possession_pct".into(),
            direction: Direction::Up,
            against: false,
        };
        let down = IdentityEntry {
            direction: Direction::Down,
            ..up.clone()
        };
        assert!(!identity_row(&matches, 0, &up).flagged);
        assert!(identity_row(&matches, 0, &down).flagged);
        assert!(identity_row(&matches, 1, &up).flagged);
    }
}

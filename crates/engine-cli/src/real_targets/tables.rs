//! League tables from results: 3 points a win, 1 a draw, no deduction applied, as the
//! engine's tables are built. Each league-season gives the champion's points, the points of
//! the first direct relegation place, the spread of points per game across clubs, and the
//! title margin; the five seasons give each target's mean and error.

use std::collections::BTreeMap;

use super::football_data::{Desc, MatchRow, mean_sd, record};
use super::{Layer, Target, league_key};

/// One league-season's table figures.
#[derive(Debug, Clone, PartialEq)]
pub struct Figures {
    pub champion_points: f64,
    /// `None` when the division's direct relegation places are not established.
    pub relegation_points: Option<f64>,
    pub ppg_sd: f64,
    pub title_margin: f64,
}

/// The table figures of one league-season's matches; `relegated` is the number of direct
/// relegation places. `None` with fewer than two clubs.
pub fn figures(matches: &[&MatchRow], relegated: Option<u32>) -> Option<Figures> {
    let mut clubs: BTreeMap<&str, (f64, f64)> = BTreeMap::new();
    for m in matches {
        let (h, a) = match m.goals[0].cmp(&m.goals[1]) {
            std::cmp::Ordering::Greater => (3.0, 0.0),
            std::cmp::Ordering::Equal => (1.0, 1.0),
            std::cmp::Ordering::Less => (0.0, 3.0),
        };
        let home = clubs.entry(&m.home).or_default();
        home.0 += h;
        home.1 += 1.0;
        let away = clubs.entry(&m.away).or_default();
        away.0 += a;
        away.1 += 1.0;
    }
    let mut points: Vec<f64> = clubs.values().map(|(p, _)| *p).collect();
    points.sort_by(|a, b| b.total_cmp(a));
    if points.len() < 2 {
        return None;
    }
    let ppg: Vec<f64> = clubs.values().map(|(p, g)| p / g).collect();
    let relegation_points = relegated
        .filter(|&r| r >= 1 && (r as usize) < points.len())
        .map(|r| points[points.len() - r as usize]);
    Some(Figures {
        champion_points: points[0],
        relegation_points,
        ppg_sd: mean_sd(&ppg).map_or(0.0, |(_, s)| s),
        title_margin: points[0] - points[1],
    })
}

/// How the engine counts every table statistic.
const ENGINE_TABLE: &str =
    "the table of each generated league's double round-robin, built the same way";

/// The table statistics: name, definition, and how a figure is read.
const TABLE_STATS: [(&str, &str); 4] = [
    (
        "champion_points",
        "points of the club first in the table (3 a win, 1 a draw, no deduction)",
    ),
    (
        "relegation_points",
        "points of the club in the first direct relegation place",
    ),
    (
        "ppg_sd",
        "standard deviation across clubs of points per game",
    ),
    (
        "title_margin",
        "points of the champion less the points of the runner-up",
    ),
];

fn pick(f: &Figures, stat: &str) -> Option<f64> {
    match stat {
        "champion_points" => Some(f.champion_points),
        "relegation_points" => f.relegation_points,
        "ppg_sd" => Some(f.ppg_sd),
        _ => Some(f.title_margin),
    }
}

/// The table targets of every division of `rows`, `fd.<div>.table.<stat>`; `relegation`
/// gives each division's direct relegation places.
pub fn league_targets(
    rows: &[MatchRow],
    relegation: &BTreeMap<String, Option<u32>>,
    targets: &mut BTreeMap<String, Target>,
    absent: &mut BTreeMap<String, String>,
) {
    let mut by: BTreeMap<(&str, &str), Vec<&MatchRow>> = BTreeMap::new();
    for r in rows {
        by.entry((&r.div, &r.season)).or_default().push(r);
    }
    let mut seasons: BTreeMap<&str, Vec<(&str, Figures)>> = BTreeMap::new();
    for ((div, season), matches) in &by {
        let places = relegation.get(*div).copied().flatten();
        if let Some(f) = figures(matches, places) {
            seasons.entry(div).or_default().push((season, f));
        }
    }
    for (div, list) in &seasons {
        for (stat, definition) in TABLE_STATS {
            let key = league_key(div, &format!("table.{stat}"));
            let values: Vec<f64> = list.iter().filter_map(|(_, f)| pick(f, stat)).collect();
            let Some((mean, sd)) = mean_sd(&values) else {
                absent.insert(
                    key,
                    if stat == "relegation_points" {
                        format!("the direct relegation places of {div} are not established")
                    } else {
                        format!("{div} has fewer than two seasons with a table")
                    },
                );
                continue;
            };
            let mut t = record(
                Some(mean),
                sd / (values.len() as f64).sqrt(),
                values.len() as u64,
                list.iter().map(|(s, _)| (*s).to_string()).collect(),
                Layer::League,
                &Desc {
                    event_definition: &format!(
                        "{definition}; from every result of the season file"
                    ),
                    denominator: "league-seasons",
                    unit: "league-season",
                    engine: ENGINE_TABLE,
                    band: None,
                },
            );
            t.between_season_sd = Some(sd);
            targets.insert(key, t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(home: &str, away: &str, h: u32, a: u32) -> MatchRow {
        MatchRow {
            div: "X1".into(),
            season: "2425".into(),
            date: "01/01/2025".into(),
            home: home.into(),
            away: away.into(),
            goals: [h, a],
            half_time: None,
            shots: None,
            on_target: None,
            fouls: None,
            corners: None,
            yellows: None,
            reds: None,
        }
    }

    #[test]
    fn a_four_club_season_gives_its_known_figures() {
        // A beats everyone home and away (18); B draws C twice and beats D twice (8);
        // C draws B twice and D twice (4); D draws C twice (2).
        let rows = vec![
            m("A", "B", 2, 0),
            m("B", "A", 0, 1),
            m("A", "C", 3, 1),
            m("C", "A", 0, 2),
            m("A", "D", 1, 0),
            m("D", "A", 0, 4),
            m("B", "C", 1, 1),
            m("C", "B", 0, 0),
            m("B", "D", 2, 1),
            m("D", "B", 0, 3),
            m("C", "D", 2, 2),
            m("D", "C", 1, 1),
        ];
        let refs: Vec<&MatchRow> = rows.iter().collect();
        let f = figures(&refs, Some(1)).unwrap();
        assert_eq!(f.champion_points, 18.0);
        assert_eq!(f.title_margin, 10.0);
        assert_eq!(f.relegation_points, Some(2.0));
        // Points per game 3, 4/3, 2/3, 1/3.
        let ppg = [3.0, 4.0 / 3.0, 2.0 / 3.0, 1.0 / 3.0];
        let mean = ppg.iter().sum::<f64>() / 4.0;
        let sd = (ppg.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / 3.0).sqrt();
        assert!((f.ppg_sd - sd).abs() < 1e-12);
        assert_eq!(figures(&refs, None).unwrap().relegation_points, None);
    }
}

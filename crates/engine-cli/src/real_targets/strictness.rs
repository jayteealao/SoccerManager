//! Germany's referee strictness: the fouls per team per match of the Bundesliga's two
//! neighbour seasons of the held-out 2017/18, each season counting once.
//!
//! The reader opens exactly the two files below and records each path it opens, so a test
//! can show that the held-out season is never read.

use std::path::Path;

use anyhow::Context;

use super::csv::Table;
use super::football_data::{PROVIDER, mean_sd};
use super::{EngineCount, Label, Layer, Target};

/// The key of the target.
pub const KEY: &str = "strict.D1.fouls_per_team";
/// The two neighbour seasons, as football-data.co.uk names their folders, and their files.
pub const SEASONS: [&str; 2] = ["1617", "1819"];
pub const FILES: [&str; 2] = ["football-data/1617/D1.csv", "football-data/1819/D1.csv"];
/// The source label of the target.
pub const SOURCE: &str = "football-data.co.uk Bundesliga fouls, 2016/17 and 2018/19";

/// One season: its mean fouls per team per match, the error of that mean, and its matches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeasonMean {
    pub mean: f64,
    pub se: f64,
    pub matches: usize,
}

/// The target, each season's mean, and the files opened (relative to the data folder).
#[derive(Debug, Clone, PartialEq)]
pub struct Built {
    pub target: Target,
    pub seasons: Vec<SeasonMean>,
    pub opened: Vec<String>,
}

fn season(data: &Path, rel: &str, opened: &mut Vec<String>) -> anyhow::Result<SeasonMean> {
    opened.push(rel.to_string());
    let t = Table::read(&data.join(rel))?;
    let per_match: Vec<f64> = t
        .rows
        .iter()
        .filter_map(|r| Some((t.num(r, "HF")? + t.num(r, "AF")?) / 2.0))
        .collect();
    let (mean, sd) =
        mean_sd(&per_match).with_context(|| format!("{rel} holds fewer than two foul rows"))?;
    Ok(SeasonMean {
        mean,
        se: sd / (per_match.len() as f64).sqrt(),
        matches: per_match.len(),
    })
}

/// Germany's strictness from the two neighbour files of `data`; `None` when one of them is
/// not there.
pub fn germany(data: &Path) -> anyhow::Result<Option<Built>> {
    if FILES.iter().any(|f| !data.join(f).is_file()) {
        return Ok(None);
    }
    let mut opened = Vec::new();
    let seasons = FILES
        .iter()
        .map(|f| season(data, f, &mut opened))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let means: Vec<f64> = seasons.iter().map(|s| s.mean).collect();
    let value = means.iter().sum::<f64>() / means.len() as f64;
    let se = seasons.iter().map(|s| s.se * s.se).sum::<f64>().sqrt() / seasons.len() as f64;
    let target = Target {
        value: Some(value),
        shares: None,
        bins: None,
        width: None,
        se,
        n: seasons.iter().map(|s| s.matches as u64).sum(),
        seasons: SEASONS.iter().map(|s| (*s).to_string()).collect(),
        between_season_sd: mean_sd(&means).map(|(_, sd)| sd),
        between_league_sd: None,
        layer: Layer::League,
        leagues: None,
        provider: PROVIDER.into(),
        source_label: SOURCE.into(),
        event_definition: "fouls committed per side (HF and AF), the mean of the two \
                           seasons' means, each season counting once"
            .into(),
        denominator: "team-matches".into(),
        unit_of_observation: "match".into(),
        label: Label::Calibration,
        engine_count: EngineCount {
            text: "mean per team of fouls.{side}".into(),
            band: None,
            measure: None,
        },
        context: Some("neighbour seasons of the held-out 2017/18".into()),
        extrapolated: None,
        formula: None,
        per_match: None,
    };
    Ok(Some(Built {
        target,
        seasons,
        opened,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(dir: &Path, rel: &str, fouls: &[(u32, u32)]) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        let mut text = String::from("Div,Date,HomeTeam,AwayTeam,HF,AF\n");
        for (i, (h, a)) in fouls.iter().enumerate() {
            text.push_str(&format!("D1,01/09/16,H{i},A{i},{h},{a}\n"));
        }
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn the_value_is_the_mean_of_the_two_season_means_and_only_they_are_opened() {
        let tmp = TempDir::new().unwrap();
        // 2016/17: 12 a side; 2018/19: 14 (two matches over 13 and 15); 2017/18 planted.
        write(tmp.path(), FILES[0], &[(10, 14), (12, 12)]);
        write(tmp.path(), FILES[1], &[(13, 13), (15, 15)]);
        write(
            tmp.path(),
            "football-data/1718/D1.csv",
            &[(99, 99), (99, 99)],
        );
        let b = germany(tmp.path()).unwrap().unwrap();
        assert!((b.target.value.unwrap() - 13.0).abs() < 1e-12);
        assert_eq!(b.opened, FILES);
        assert_eq!(b.target.seasons, ["1617", "1819"]);
        assert_eq!(b.target.n, 4);
        // Season 2018/19 per-match 13 and 15: sd sqrt(2), se 1; 2016/17 has no spread.
        assert!((b.target.se - 0.5).abs() < 1e-12, "{}", b.target.se);
        assert!((b.target.between_season_sd.unwrap() - 2f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn a_missing_neighbour_file_builds_nothing() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), FILES[0], &[(10, 14), (12, 12)]);
        assert!(germany(tmp.path()).unwrap().is_none());
    }
}

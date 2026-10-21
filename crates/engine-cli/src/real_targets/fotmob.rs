//! Pass completion by Opta's count, from the FotMob team tables of the five top leagues.
//! Each table gives each team's accuracy (percent) and accurate passes per match; the
//! attempts per match are the accurate passes over the accuracy share. A league-season's
//! completion is its summed accurate passes over its summed attempts; the five seasons give
//! the target's mean and error. Other leagues get no pass-completion target here.

use std::collections::BTreeMap;
use std::path::Path;

use super::csv::Table;
use super::data_check::{FOTMOB_LEAGUES, SEASONS};
use super::football_data::mean_sd;
use super::{EngineCount, Label, Layer, Target};

/// The provider's name.
pub const PROVIDER: &str = "Opta via FotMob";
/// The source label.
pub const SOURCE: &str = "FotMob team accurate-pass tables, 2021/22 to 2025/26";

/// The completion of one league-season's table, in percent; `None` when no row reads.
pub fn completion(t: &Table) -> Option<f64> {
    let (mut accurate, mut attempts) = (0.0, 0.0);
    for row in &t.rows {
        let (Some(pct), Some(acc)) = (
            t.num(row, "pass_accuracy_percent"),
            t.num(row, "accurate_passes_per_match"),
        ) else {
            continue;
        };
        if pct > 0.0 {
            accurate += acc;
            attempts += acc / (pct / 100.0);
        }
    }
    (attempts > 0.0).then(|| 100.0 * accurate / attempts)
}

fn target(
    values: &[f64],
    seasons: Vec<String>,
    layer: Layer,
    leagues: Option<Vec<String>>,
) -> Option<Target> {
    let (mean, sd) = mean_sd(values)?;
    Some(Target {
        value: Some(mean),
        shares: None,
        bins: None,
        width: None,
        se: sd / (values.len() as f64).sqrt(),
        n: values.len() as u64,
        seasons,
        between_season_sd: (layer == Layer::League).then_some(sd),
        between_league_sd: None,
        layer,
        leagues,
        provider: PROVIDER.into(),
        source_label: SOURCE.into(),
        event_definition: "completed passes as a percentage of passes attempted, by Opta's \
                           count; attempts per team are accurate passes per match over the \
                           accuracy share"
            .into(),
        denominator: "passes attempted (Opta)".into(),
        unit_of_observation: "league-season".into(),
        label: Label::Calibration,
        engine_count: EngineCount {
            text: "completed passes as a percentage of passes of each side per match".into(),
            band: Some("pass_accuracy_pct".into()),
            measure: Some("mean per team of pass_accuracy_pct.{side}".into()),
        },
        context: None,
        extrapolated: None,
        formula: None,
    })
}

/// The pass-completion targets `fotmob.<div>.pass_completion` and
/// `fotmob.pooled.pass_completion`; returns the files read.
pub fn targets(
    data: &Path,
    targets: &mut BTreeMap<String, Target>,
    absent: &mut BTreeMap<String, String>,
) -> anyhow::Result<Vec<String>> {
    let mut files = Vec::new();
    let mut pooled = Vec::new();
    for (league, div) in FOTMOB_LEAGUES {
        let mut values = Vec::new();
        let mut seasons = Vec::new();
        for season in SEASONS {
            let rel = format!("fotmob/{league}-{season}.csv");
            if !data.join(&rel).is_file() {
                continue;
            }
            let t = Table::read(&data.join(&rel))?;
            files.push(rel);
            if let Some(c) = completion(&t) {
                values.push(c);
                seasons.push(season.to_string());
            }
        }
        let key = format!("fotmob.{div}.pass_completion");
        match target(&values, seasons, Layer::League, None) {
            Some(t) => {
                targets.insert(key, t);
            }
            None => {
                absent.insert(key, format!("fewer than two {league} seasons read"));
            }
        }
        pooled.extend(values);
    }
    if let Some(t) = target(
        &pooled,
        SEASONS.iter().map(|s| (*s).to_string()).collect(),
        Layer::Pooled,
        Some(
            FOTMOB_LEAGUES
                .iter()
                .map(|(_, d)| (*d).to_string())
                .collect(),
        ),
    ) {
        targets.insert("fotmob.pooled.pass_completion".into(), t);
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_teams_give_their_summed_completion() {
        // 400 accurate at 80 percent is 500 attempts; 300 at 75 percent is 400.
        let t = Table::parse(
            "\"team\",\"pass_accuracy_percent\",\"accurate_passes\",\"total_passes\",\
             \"accurate_passes_per_match\"\n\"A\",\"80\",\"\",\"\",\"400\"\n\"B\",\"75\",\"\",\"\",\"300\"\n",
        )
        .unwrap();
        let c = completion(&t).unwrap();
        assert!((c - 100.0 * 700.0 / 900.0).abs() < 1e-9, "{c}");
    }
}

//! Figures from published studies: the scan rate of elite outfield players, from the
//! scan-rate study table on this PC.
//!
//! The row is chosen by study, year, position group, condition and measure, so another row
//! of the same study (a position, a pressure condition, scans per window) is never taken.

use std::path::Path;

use anyhow::Context;

use super::csv::Table;
use super::{EngineCount, Label, Layer, Target};

/// The table, relative to the data folder.
pub const FILE: &str = "scanning-studies/scan-rates.csv";
/// The key of the scan-rate target.
pub const KEY: &str = "lit.jordet2020.scans_per_second";
/// The provider and its credit.
pub const PROVIDER: &str = "Jordet et al. 2020";
pub const SOURCE: &str = "Jordet, Aksum, Pedersen et al. (2020), Scanning, contextual factors \
                          and performance in English Premier League footballers, Frontiers \
                          in Psychology 11:553813";
pub const LICENCE: &str = "CC BY 4.0";

/// The row the target reads: (column, value), the study matched by its first word.
const STUDY: &str = "Jordet";
const YEAR: &str = "2020";
const POSITION: &str = "all outfield";
const CONDITION: &str = "all";
const MEASURE: &str = "scan frequency";

/// The leading number of a cell such as `SD 0.30` or `9574 possessions`.
fn number(text: &str) -> Option<f64> {
    text.split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .find(|s| !s.is_empty() && s.parse::<f64>().is_ok())
        .and_then(|s| s.parse().ok())
}

/// The scan-rate target of `data`; `None` when the table is not there.
pub fn scan_rate(data: &Path) -> anyhow::Result<Option<Target>> {
    if !data.join(FILE).is_file() {
        return Ok(None);
    }
    let t = Table::read(&data.join(FILE))?;
    let row = t
        .rows
        .iter()
        .find(|r| {
            t.get(r, "study").starts_with(STUDY)
                && t.get(r, "year") == YEAR
                && t.get(r, "position_group") == POSITION
                && t.get(r, "condition") == CONDITION
                && t.get(r, "measure") == MEASURE
        })
        .with_context(|| format!("{FILE} has no {STUDY} {YEAR} {POSITION} {MEASURE} row"))?;
    let value = t
        .num(row, "value")
        .with_context(|| format!("{FILE}: the chosen row has no value"))?;
    let sd = number(t.get(row, "sd_or_ci"))
        .with_context(|| format!("{FILE}: the chosen row has no SD"))?;
    let n = number(t.get(row, "n")).with_context(|| format!("{FILE}: the chosen row has no n"))?;
    Ok(Some(Target {
        value: Some(value),
        shares: None,
        bins: None,
        width: None,
        se: sd / n.sqrt(),
        n: n as u64,
        seasons: vec!["1718".into()],
        between_season_sd: None,
        between_league_sd: None,
        layer: Layer::League,
        leagues: None,
        provider: PROVIDER.into(),
        source_label: "Jordet et al. 2020, scans per second of elite outfield players".into(),
        event_definition: "active head turns away from the ball to see team-mates and \
                           opponents, in the window before a player receives, per second"
            .into(),
        denominator: "seconds of the window before each possession".into(),
        unit_of_observation: "possession".into(),
        label: Label::Calibration,
        engine_count: EngineCount {
            text: "no engine count yet: the engine counts no scans".into(),
            band: Some("scan_rate".into()),
            measure: Some("pending".into()),
        },
        context: Some("one Premier League club, 2017/18, coded from video".into()),
        extrapolated: None,
        formula: None,
        per_match: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn the_chosen_row_is_the_studys_outfield_scan_frequency() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join(FILE);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(
            &p,
            "study,year,position_group,condition,measure,value,sd_or_ci,n\n\
             \"Jordet, A et al.\",2020,all outfield,all,scans in window,3.0,SD 2.1,100 possessions\n\
             \"Jordet, A et al.\",2020,CM (central midfielder),all,scan frequency,0.52,SD 0.3,not reported\n\
             \"Jordet, A et al.\",2020,all outfield,pressure 2 m,scan frequency,0.40,SD 0.3,not reported\n\
             \"Jordet, A et al.\",2020,all outfield,all,scan frequency,0.5,SD 0.2,400 possessions\n\
             \"Other, B\",2020,all outfield,all,scan frequency,0.9,SD 0.1,10 possessions\n",
        )
        .unwrap();
        let t = scan_rate(tmp.path()).unwrap().unwrap();
        assert_eq!(t.value, Some(0.5));
        assert_eq!(t.n, 400);
        assert!((t.se - 0.01).abs() < 1e-12);
        assert_eq!(t.engine_count.band.as_deref(), Some("scan_rate"));
        assert_eq!(t.unit_of_observation, "possession");
    }

    #[test]
    fn numbers_are_read_from_their_cells() {
        assert_eq!(number("SD 0.30"), Some(0.30));
        assert_eq!(number("9574 possessions"), Some(9574.0));
        assert_eq!(number("not reported"), None);
    }
}

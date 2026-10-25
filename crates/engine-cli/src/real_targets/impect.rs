//! IMPECT open data, 1. Bundesliga 2023/24: event targets for the local side file only.
//!
//! IMPECT's terms forbid redistribution and commercial use, so these targets never enter
//! the committed reference file and no band reads them: they are cross-source comparisons
//! in the calibration report, on the machines that hold the side file. Each match gives its
//! counts per side; a per-team target is the mean over matches of the two sides' mean, a
//! per-match target the mean of their sum, each with its error over whole matches.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

use super::football_data::mean_sd;
use super::{EngineCount, Label, Layer, Target};
use crate::report::distribution::bin_of;
use crate::report::se_ratio;

/// The provider every target names.
pub const PROVIDER: &str = "IMPECT";
/// The source label of every target.
pub const SOURCE: &str = "IMPECT open data, 1. Bundesliga 2023/24";
/// The context of every target.
pub const CONTEXT: &str = "men's club, Germany, 2023/24";
/// The matches file of the season, relative to the data folder.
pub const MATCHES: &str = "impect-open-data/data/matches/matches_743.json";
/// The folder of the event files, relative to the data folder.
pub const EVENTS: &str = "impect-open-data/data/events";
/// The licence note the side file carries.
pub const LICENCE: &str = "IMPECT open data licence: no editing or redistribution, no \
                           commercial use, credit IMPECT";

#[derive(Deserialize)]
struct MatchEntry {
    id: u64,
}

#[derive(Deserialize)]
struct ShotInfo {
    #[serde(default)]
    distance: Option<f64>,
}

/// One event, the fields the terms read.
#[derive(Deserialize)]
pub struct Event {
    #[serde(rename = "squadId", default)]
    squad: Option<u64>,
    #[serde(rename = "actionType")]
    kind: String,
    #[serde(default)]
    result: Option<String>,
    #[serde(rename = "periodId", default)]
    period: Option<u32>,
    #[serde(default)]
    shot: Option<ShotInfo>,
}

/// One match's counts, each side by its squad id order (first seen first).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MatchTerms {
    pub goals: f64,
    pub shots: f64,
    pub distance: f64,
    pub shot_bins: Vec<u32>,
    /// Passes of any result, completed (SUCCESS) and judged (SUCCESS or FAIL), per side.
    pub passes: [f64; 2],
    pub completed: [f64; 2],
    pub judged: [f64; 2],
    pub fouls: f64,
    pub corners: f64,
    pub throw_ins: f64,
    pub goal_kicks: f64,
    pub offsides: f64,
}

/// The counts of a match's events; a shoot-out (period 5) counts nowhere. NEUTRAL passes
/// are counted as passes and left out of completion.
pub fn match_terms(events: &[Event]) -> MatchTerms {
    let mut t = MatchTerms::default();
    let mut sides: Vec<u64> = Vec::new();
    for e in events.iter().filter(|e| e.period.unwrap_or(1) <= 4) {
        let side = e.squad.map(|s| {
            if let Some(i) = sides.iter().position(|&x| x == s) {
                i
            } else {
                sides.push(s);
                sides.len() - 1
            }
        });
        match e.kind.as_str() {
            "GOAL" => t.goals += 1.0,
            "SHOT" => {
                t.shots += 1.0;
                if let Some(d) = e.shot.as_ref().and_then(|s| s.distance) {
                    t.distance += d;
                    t.shot_bins.push(bin_of(d, 1.0, 41));
                }
            }
            "PASS" => {
                if let Some(i) = side.filter(|&i| i < 2) {
                    t.passes[i] += 1.0;
                    match e.result.as_deref() {
                        Some("SUCCESS") => {
                            t.completed[i] += 1.0;
                            t.judged[i] += 1.0;
                        }
                        Some("FAIL") => t.judged[i] += 1.0,
                        _ => {}
                    }
                }
            }
            "FOUL" => t.fouls += 1.0,
            "CORNER" => t.corners += 1.0,
            "THROW_IN" => t.throw_ins += 1.0,
            "GOAL_KICK" => t.goal_kicks += 1.0,
            "OFFSIDE" => t.offsides += 1.0,
            _ => {}
        }
    }
    t
}

/// The counts of an event file's text.
pub fn parse_terms(text: &str) -> anyhow::Result<MatchTerms> {
    let events: Vec<Event> = serde_json::from_str(text)?;
    Ok(match_terms(&events))
}

/// How a statistic reduces over matches.
enum Reduce {
    /// The mean over matches of the per-match value.
    Mean(fn(&MatchTerms) -> Option<f64>),
    /// The summed numerator over the summed denominator.
    Ratio(fn(&MatchTerms) -> f64, fn(&MatchTerms) -> f64),
}

/// One statistic, with the band that counts the same thing.
struct Stat {
    name: &'static str,
    reduce: Reduce,
    event_definition: &'static str,
    denominator: &'static str,
    band: &'static str,
    engine: &'static str,
}

fn pct(t: &MatchTerms) -> Option<f64> {
    let sides: Vec<f64> = (0..2)
        .filter(|&i| t.judged[i] > 0.0)
        .map(|i| 100.0 * t.completed[i] / t.judged[i])
        .collect();
    (!sides.is_empty()).then(|| sides.iter().sum::<f64>() / sides.len() as f64)
}

const STATS: [Stat; 10] = [
    Stat {
        name: "goals_per_match",
        reduce: Reduce::Mean(|t| Some(t.goals)),
        event_definition: "goals, both sides",
        denominator: "matches",
        band: "goals_per_match",
        engine: "mean per match of goals",
    },
    Stat {
        name: "shots_per_team",
        reduce: Reduce::Mean(|t| Some(t.shots / 2.0)),
        event_definition: "shots per side, blocked shots counted",
        denominator: "team-matches",
        band: "shots_per_team",
        engine: "mean per team of shots.{side}",
    },
    Stat {
        name: "shot_distance_mean",
        reduce: Reduce::Ratio(|t| t.distance, |t| t.shot_bins.len() as f64),
        event_definition: "IMPECT's metres from each shot to the goal",
        denominator: "shots with a distance",
        band: "shot_distance_mean",
        engine: "ratio of shot_distance_m over shots",
    },
    Stat {
        name: "passes_per_team",
        reduce: Reduce::Mean(|t| Some((t.passes[0] + t.passes[1]) / 2.0)),
        event_definition: "passes per side of any result, NEUTRAL counted",
        denominator: "team-matches",
        band: "passes_per_team",
        engine: "mean per team of passes.{side}",
    },
    Stat {
        name: "pass_accuracy_pct",
        reduce: Reduce::Mean(pct),
        event_definition: "per side, passes with result SUCCESS over SUCCESS plus FAIL, in \
                           percent; NEUTRAL passes left out",
        denominator: "team-matches",
        band: "pass_accuracy_pct",
        engine: "mean per team of pass_accuracy_pct.{side}",
    },
    Stat {
        name: "fouls_per_team",
        reduce: Reduce::Mean(|t| Some(t.fouls / 2.0)),
        event_definition: "fouls committed per side",
        denominator: "team-matches",
        band: "fouls_per_team",
        engine: "mean per team of fouls.{side}",
    },
    Stat {
        name: "corners_per_team",
        reduce: Reduce::Mean(|t| Some(t.corners / 2.0)),
        event_definition: "corners taken per side",
        denominator: "team-matches",
        band: "corners_per_team",
        engine: "mean per team of corners.{side}",
    },
    Stat {
        name: "throw_ins_per_match",
        reduce: Reduce::Mean(|t| Some(t.throw_ins)),
        event_definition: "throw-ins, both sides",
        denominator: "matches",
        band: "throw_ins_per_match",
        engine: "mean per match of throw_ins",
    },
    Stat {
        name: "goal_kicks_per_match",
        reduce: Reduce::Mean(|t| Some(t.goal_kicks)),
        event_definition: "goal kicks, both sides",
        denominator: "matches",
        band: "goal_kicks_per_match",
        engine: "mean per match of goal_kicks",
    },
    Stat {
        name: "offsides_per_team",
        reduce: Reduce::Mean(|t| Some(t.offsides / 2.0)),
        event_definition: "offsides per side",
        denominator: "team-matches",
        band: "offsides_per_team",
        engine: "mean per team of offsides.{side}",
    },
];

/// The key of an IMPECT target.
pub fn key(stat: &str) -> String {
    format!("imp.bl2324.{stat}")
}

fn record(value: Option<f64>, se: f64, n: u64, text: (&str, &str, &str, &str)) -> Target {
    let (event_definition, denominator, band, engine) = text;
    Target {
        value,
        shares: None,
        bins: None,
        width: None,
        se,
        n,
        seasons: vec!["2324".into()],
        between_season_sd: None,
        between_league_sd: None,
        layer: Layer::League,
        leagues: None,
        provider: PROVIDER.into(),
        source_label: SOURCE.into(),
        event_definition: event_definition.into(),
        denominator: denominator.into(),
        unit_of_observation: "match".into(),
        label: Label::Calibration,
        engine_count: EngineCount {
            text: engine.into(),
            band: Some(band.into()),
            measure: Some(engine.into()),
        },
        context: Some(CONTEXT.into()),
        extrapolated: None,
        formula: None,
        per_match: None,
    }
}

/// The targets of `matches`.
pub fn targets(matches: &[MatchTerms]) -> BTreeMap<String, Target> {
    let mut out = BTreeMap::new();
    let n = matches.len() as u64;
    for s in &STATS {
        let got = match s.reduce {
            Reduce::Mean(f) => {
                let xs: Vec<f64> = matches.iter().filter_map(f).collect();
                mean_sd(&xs).map(|(m, sd)| (m, sd / (xs.len() as f64).sqrt()))
            }
            Reduce::Ratio(num, den) => {
                let ys: Vec<f64> = matches.iter().map(num).collect();
                let xs: Vec<f64> = matches.iter().map(den).collect();
                let d: f64 = xs.iter().sum();
                (d > 0.0).then(|| (ys.iter().sum::<f64>() / d, se_ratio(&ys, &xs)))
            }
        };
        if let Some((value, se)) = got {
            out.insert(
                key(s.name),
                record(
                    Some(value),
                    se,
                    n,
                    (s.event_definition, s.denominator, s.band, s.engine),
                ),
            );
        }
    }
    let mut counts = vec![0.0; 41];
    for t in matches {
        for &b in &t.shot_bins {
            counts[b as usize] += 1.0;
        }
    }
    let total: f64 = counts.iter().sum();
    if total > 0.0 {
        let shares: Vec<f64> = counts.iter().map(|c| c / total).collect();
        let top = (0..41).fold(0, |best, i| if shares[i] > shares[best] { i } else { best });
        let ys: Vec<f64> = matches
            .iter()
            .map(|t| t.shot_bins.iter().filter(|&&b| b as usize == top).count() as f64)
            .collect();
        let xs: Vec<f64> = matches.iter().map(|t| t.shot_bins.len() as f64).collect();
        let mut t = record(
            None,
            se_ratio(&ys, &xs),
            n,
            (
                "IMPECT's metres from each shot to the goal, 1 m bins, 40 m and over as one",
                "matches",
                "shot_distance_distribution",
                "distribution of shot_distance",
            ),
        );
        t.shares = Some(shares);
        t.bins = Some(41);
        t.width = Some(1.0);
        out.insert(key("dist.shot_distance"), t);
    }
    out
}

/// What the IMPECT step read and built.
#[derive(Debug, Clone)]
pub struct Built {
    pub matches: usize,
    pub targets: BTreeMap<String, Target>,
    /// Files read, relative to the data folder.
    pub files: Vec<String>,
}

/// Reads the season's matches file and each match's event file; `None` when the data
/// folder holds no IMPECT matches file.
pub fn build(data: &Path) -> anyhow::Result<Option<Built>> {
    if !data.join(MATCHES).is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(data.join(MATCHES))
        .with_context(|| format!("cannot read {MATCHES}"))?;
    let list: Vec<MatchEntry> =
        serde_json::from_str(&text).with_context(|| format!("{MATCHES} cannot be read"))?;
    let mut ids: Vec<u64> = list.into_iter().map(|m| m.id).collect();
    ids.sort_unstable();
    ids.dedup();
    let mut files = vec![MATCHES.to_string()];
    let mut matches = Vec::with_capacity(ids.len());
    for id in ids {
        let rel = format!("{EVENTS}/events_{id}.json");
        let text = std::fs::read_to_string(data.join(&rel))
            .with_context(|| format!("cannot read {rel}"))?;
        matches.push(parse_terms(&text).with_context(|| format!("{rel} cannot be read"))?);
        files.push(rel);
    }
    Ok(Some(Built {
        matches: matches.len(),
        targets: targets(&matches),
        files,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A completed, a failed and a neutral pass, a 12 m shot and its goal, a foul.
    const THREE: &str = r#"[
      {"squadId": 1, "actionType": "PASS", "result": "SUCCESS", "periodId": 1},
      {"squadId": 1, "actionType": "PASS", "result": "NEUTRAL", "periodId": 1},
      {"squadId": 2, "actionType": "PASS", "result": "FAIL", "periodId": 1},
      {"squadId": 1, "actionType": "SHOT", "result": "SUCCESS", "periodId": 2,
       "shot": {"distance": 12.0, "angle": 10.0}},
      {"squadId": 1, "actionType": "GOAL", "result": null, "periodId": 2},
      {"squadId": 2, "actionType": "FOUL", "result": null, "periodId": 2}
    ]"#;

    #[test]
    fn neutral_passes_are_counted_and_left_out_of_completion() {
        let t = parse_terms(THREE).unwrap();
        assert_eq!(t.passes, [2.0, 1.0]);
        assert_eq!(t.completed, [1.0, 0.0]);
        assert_eq!(t.judged, [1.0, 1.0]);
        assert_eq!((t.goals, t.shots, t.fouls), (1.0, 1.0, 1.0));
        assert_eq!(t.shot_bins, vec![12]);
        // Side 1 completes 1 of 1 judged, side 2 none of 1: 50 percent on average.
        assert_eq!(pct(&t), Some(50.0));
    }

    #[test]
    fn each_target_names_its_band_and_the_impect_label() {
        let a = parse_terms(THREE).unwrap();
        let mut b = a.clone();
        b.goals = 3.0;
        let t = targets(&[a, b]);
        let g = &t["imp.bl2324.goals_per_match"];
        assert_eq!(g.value, Some(2.0));
        assert_eq!(g.n, 2);
        assert_eq!(g.source_label, SOURCE);
        assert_eq!(g.provider, PROVIDER);
        assert_eq!(g.engine_count.band.as_deref(), Some("goals_per_match"));
        assert_eq!(
            t["imp.bl2324.passes_per_team"].value,
            Some(1.5),
            "three passes, two sides"
        );
        assert!(t.contains_key("imp.bl2324.dist.shot_distance"));
        assert!(t.keys().all(|k| k.starts_with("imp.bl2324.")));
    }
}

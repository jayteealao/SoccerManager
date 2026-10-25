//! The football-data.co.uk result files: whole-match rows read in place, and the per-league
//! targets over five seasons, scalar and whole distributions.
//!
//! The shot columns (`HS`, `AS`) count blocked shots: the builder checks it on a known
//! competition before it builds any shot target ([`super::shots_check`]).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::csv::Table;
use super::{EngineCount, Label, Layer, Target, league_key};
use crate::report::distribution::bin_of;
use crate::report::measures::cell;

/// The provider's name in every football-data target.
pub const PROVIDER: &str = "football-data.co.uk";
/// The source label of the result files.
pub const SOURCE: &str = "football-data.co.uk results and match statistics, 2021/22 to 2025/26";

/// One match of a result file. A column the file does not fill reads `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchRow {
    pub div: String,
    pub season: String,
    /// `dd/mm/yyyy`, as the file writes it.
    pub date: String,
    pub home: String,
    pub away: String,
    pub goals: [u32; 2],
    pub half_time: Option<[u32; 2]>,
    pub shots: Option<[u32; 2]>,
    pub on_target: Option<[u32; 2]>,
    pub fouls: Option<[u32; 2]>,
    pub corners: Option<[u32; 2]>,
    pub yellows: Option<[u32; 2]>,
    pub reds: Option<[u32; 2]>,
}

/// What reading the result files gave: the matches, and the rows left out (no result yet,
/// or no teams) per file.
#[derive(Debug, Default)]
pub struct Read {
    pub rows: Vec<MatchRow>,
    pub skipped: BTreeMap<String, u32>,
    /// Each file read, relative to the data folder.
    pub files: Vec<String>,
    /// Each file not there, relative to the data folder.
    pub missing: Vec<String>,
}

fn pair(t: &Table, row: &[String], home: &str, away: &str) -> Option<[u32; 2]> {
    let n = |c: &str| {
        t.num(row, c)
            .filter(|v| *v >= 0.0 && v.fract() == 0.0)
            .map(|v| v as u32)
    };
    Some([n(home)?, n(away)?])
}

/// Every played match of `divisions` in `seasons`, read in place from
/// `data/football-data/<season>/<div>.csv`, in file order. A file not there is listed in
/// `missing` (the data check names it before a build); a file that cannot be read is an
/// error that names it.
pub fn match_rows(data: &Path, divisions: &[&str], seasons: &[&str]) -> anyhow::Result<Read> {
    let mut out = Read::default();
    for season in seasons {
        for div in divisions {
            let rel = format!("football-data/{season}/{div}.csv");
            if !data.join(&rel).is_file() {
                out.missing.push(rel);
                continue;
            }
            let t = Table::read(&data.join(&rel))?;
            for row in &t.rows {
                let home = t.get(row, "HomeTeam");
                let away = t.get(row, "AwayTeam");
                let goals = pair(&t, row, "FTHG", "FTAG");
                let (Some(goals), false, false) = (goals, home.is_empty(), away.is_empty()) else {
                    *out.skipped.entry(rel.clone()).or_default() += 1;
                    continue;
                };
                out.rows.push(MatchRow {
                    div: (*div).to_string(),
                    season: (*season).to_string(),
                    date: t.get(row, "Date").to_string(),
                    home: home.to_string(),
                    away: away.to_string(),
                    goals,
                    half_time: pair(&t, row, "HTHG", "HTAG"),
                    shots: pair(&t, row, "HS", "AS"),
                    on_target: pair(&t, row, "HST", "AST"),
                    fouls: pair(&t, row, "HF", "AF"),
                    corners: pair(&t, row, "HC", "AC"),
                    yellows: pair(&t, row, "HY", "AY"),
                    reds: pair(&t, row, "HR", "AR"),
                });
            }
            out.files.push(rel);
        }
    }
    Ok(out)
}

/// The mean and the sample standard deviation of `xs`; `None` below two values.
pub fn mean_sd(xs: &[f64]) -> Option<(f64, f64)> {
    if xs.len() < 2 {
        return None;
    }
    let n = xs.len() as f64;
    let mean = xs.iter().sum::<f64>() / n;
    let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    Some((mean, var.sqrt()))
}

/// One scalar statistic of a match.
pub struct Stat {
    pub name: &'static str,
    pub of: fn(&MatchRow) -> Option<f64>,
    pub event_definition: &'static str,
    pub denominator: &'static str,
    pub unit: &'static str,
    pub engine: &'static str,
    /// The registry band that counts the same thing, and its measure.
    pub band: Option<(&'static str, &'static str)>,
    /// `true` when the stat counts shots, so it waits for the shots answer.
    pub shots: bool,
}

fn per_team(x: Option<[u32; 2]>) -> Option<f64> {
    x.map(|[h, a]| f64::from(h + a) / 2.0)
}

fn flag(b: bool) -> Option<f64> {
    Some(if b { 1.0 } else { 0.0 })
}

/// The scalar statistics of every league.
pub const STATS: [Stat; 15] = [
    Stat {
        name: "goals_per_match",
        of: |m| Some(f64::from(m.goals[0] + m.goals[1])),
        event_definition: "full-time goals of both teams",
        denominator: "matches",
        unit: "match",
        engine: "goals of both sides per match",
        band: Some((
            "goals_per_match",
            "mean per match of goals.home + goals.away",
        )),
        shots: false,
    },
    Stat {
        name: "home_goals_per_match",
        of: |m| Some(f64::from(m.goals[0])),
        event_definition: "full-time goals of the home team",
        denominator: "matches",
        unit: "match",
        engine: "goals of the home side per match",
        band: None,
        shots: false,
    },
    Stat {
        name: "away_goals_per_match",
        of: |m| Some(f64::from(m.goals[1])),
        event_definition: "full-time goals of the away team",
        denominator: "matches",
        unit: "match",
        engine: "goals of the away side per match",
        band: None,
        shots: false,
    },
    Stat {
        name: "home_win_share",
        of: |m| flag(m.goals[0] > m.goals[1]),
        event_definition: "matches the home team won at full time",
        denominator: "matches",
        unit: "match",
        engine: "share of matches the home side wins",
        band: None,
        shots: false,
    },
    Stat {
        name: "draw_share",
        of: |m| flag(m.goals[0] == m.goals[1]),
        event_definition: "matches level at full time",
        denominator: "matches",
        unit: "match",
        engine: "share of drawn matches",
        band: None,
        shots: false,
    },
    Stat {
        name: "away_win_share",
        of: |m| flag(m.goals[0] < m.goals[1]),
        event_definition: "matches the away team won at full time",
        denominator: "matches",
        unit: "match",
        engine: "share of matches the away side wins",
        band: None,
        shots: false,
    },
    Stat {
        name: "goalless_share",
        of: |m| flag(m.goals == [0, 0]),
        event_definition: "matches ending 0-0",
        denominator: "matches",
        unit: "match",
        engine: "share of matches with no goal",
        band: Some((
            "goalless_share",
            "share of matches with goals.home + goals.away == 0",
        )),
        shots: false,
    },
    Stat {
        name: "sending_off_share",
        of: |m| m.reds.map(|[h, a]| if h + a >= 1 { 1.0 } else { 0.0 }),
        event_definition: "matches with at least one red card (HR + AR)",
        denominator: "matches",
        unit: "match",
        engine: "share of matches with at least one sending-off",
        band: Some((
            "sending_off_share",
            "share of matches with red.home + red.away >= 1",
        )),
        shots: false,
    },
    Stat {
        name: "shots_per_team",
        of: |m| per_team(m.shots),
        event_definition: "shots of each team, blocked shots counted (HS, AS)",
        denominator: "team-matches",
        unit: "match",
        engine: "shots of each side per match, blocked shots counted",
        band: Some(("shots_per_team", "mean per team of shots.{side}")),
        shots: true,
    },
    Stat {
        name: "shots_on_target_per_team",
        of: |m| per_team(m.on_target),
        event_definition: "shots on target of each team (HST, AST)",
        denominator: "team-matches",
        unit: "match",
        engine: "shots on target of each side per match",
        band: None,
        shots: true,
    },
    Stat {
        name: "fouls_per_team",
        of: |m| per_team(m.fouls),
        event_definition: "fouls committed by each team (HF, AF)",
        denominator: "team-matches",
        unit: "match",
        engine: "fouls of each side per match",
        band: Some(("fouls_per_team", "mean per team of fouls.{side}")),
        shots: false,
    },
    Stat {
        name: "corners_per_team",
        of: |m| per_team(m.corners),
        event_definition: "corners won by each team (HC, AC)",
        denominator: "team-matches",
        unit: "match",
        engine: "corners of each side per match",
        band: Some(("corners_per_team", "mean per team of corners.{side}")),
        shots: false,
    },
    Stat {
        name: "yellow_cards_per_team",
        of: |m| per_team(m.yellows),
        event_definition: "yellow cards shown to each team (HY, AY)",
        denominator: "team-matches",
        unit: "match",
        engine: "yellow cards of each side per match",
        band: Some(("yellow_cards_per_team", "mean per team of yellow.{side}")),
        shots: false,
    },
    Stat {
        name: "red_cards_per_team",
        of: |m| per_team(m.reds),
        event_definition: "red cards shown to each team (HR, AR)",
        denominator: "team-matches",
        unit: "match",
        engine: "red cards of each side per match",
        band: None,
        shots: false,
    },
    Stat {
        name: "cards_per_team",
        of: |m| match (m.yellows, m.reds) {
            (Some([hy, ay]), Some([hr, ar])) => Some(f64::from(hy + ay + hr + ar) / 2.0),
            _ => None,
        },
        event_definition: "yellow and red cards shown to each team (HY + HR, AY + AR)",
        denominator: "team-matches",
        unit: "match",
        engine: "yellow and red cards of each side per match",
        band: None,
        shots: false,
    },
];

/// One whole distribution of a match.
pub struct Dist {
    pub name: &'static str,
    pub bins: u32,
    pub width: f64,
    pub bin: fn(&MatchRow, u32, f64) -> Option<u32>,
    pub event_definition: &'static str,
    pub engine: &'static str,
    pub band: Option<(&'static str, &'static str)>,
    pub shots: bool,
}

/// The highest goals per side of a scoreline cell: the registry's `scoreline_cells` band.
pub const SCORE_MAX: u32 = 5;

/// The whole distributions of every league.
pub const DISTS: [Dist; 4] = [
    Dist {
        name: "scorelines",
        bins: (SCORE_MAX + 1) * (SCORE_MAX + 1),
        width: 1.0,
        bin: |m, _, _| Some(cell(m.goals[0], m.goals[1], SCORE_MAX)),
        event_definition: "full-time scoreline, each side capped at 5 goals \
                           (cell = home x 6 + away)",
        engine: "each match's scoreline cell, each side capped at 5",
        band: Some(("scoreline_cells", "cells max 5")),
        shots: false,
    },
    Dist {
        name: "goals",
        bins: 11,
        width: 1.0,
        bin: |m, b, w| Some(bin_of(f64::from(m.goals[0] + m.goals[1]), w, b)),
        event_definition: "full-time goals of both teams; the last bin holds 10 and more",
        engine: "goals of both sides per match",
        band: Some((
            "goals_distribution",
            "distribution of goals, width 1, 11 bins",
        )),
        shots: false,
    },
    Dist {
        name: "shots",
        bins: 51,
        width: 1.0,
        bin: |m, b, w| m.shots.map(|[h, a]| bin_of(f64::from(h + a), w, b)),
        event_definition: "shots of both teams, blocked shots counted (HS + AS); \
                           the last bin holds 50 and more",
        engine: "shots of both sides per match, blocked shots counted",
        band: None,
        shots: true,
    },
    Dist {
        name: "cards",
        bins: 16,
        width: 1.0,
        bin: |m, b, w| match (m.yellows, m.reds) {
            (Some([hy, ay]), Some([hr, ar])) => Some(bin_of(f64::from(hy + ay + hr + ar), w, b)),
            _ => None,
        },
        event_definition: "yellow and red cards of both teams (HY + AY + HR + AR); \
                           the last bin holds 15 and more",
        engine: "cards of both sides per match",
        band: None,
        shots: false,
    },
];

/// What a target counts: its definition, denominator and unit, and how the engine counts
/// the same thing (a text, and the registry band and its measure when one counts it).
pub struct Desc<'a> {
    pub event_definition: &'a str,
    pub denominator: &'a str,
    pub unit: &'a str,
    pub engine: &'a str,
    pub band: Option<(&'a str, &'a str)>,
}

/// The record fields a football-data target shares.
pub fn record(
    value: Option<f64>,
    se: f64,
    n: u64,
    seasons: Vec<String>,
    layer: Layer,
    desc: &Desc<'_>,
) -> Target {
    let Desc {
        event_definition,
        denominator,
        unit,
        engine,
        band,
    } = *desc;
    Target {
        value,
        shares: None,
        bins: None,
        width: None,
        se,
        n,
        seasons,
        between_season_sd: None,
        between_league_sd: None,
        layer,
        leagues: None,
        provider: PROVIDER.into(),
        source_label: SOURCE.into(),
        event_definition: event_definition.into(),
        denominator: denominator.into(),
        unit_of_observation: unit.into(),
        label: Label::Calibration,
        engine_count: EngineCount {
            text: engine.into(),
            band: band.map(|(b, _)| b.to_string()),
            measure: band.map(|(_, m)| m.to_string()),
        },
        context: None,
        extrapolated: None,
        formula: None,
        per_match: None,
    }
}

/// The per-league targets of `rows`: every scalar statistic and every distribution, each
/// division over its seasons. A statistic a division's files do not fill is absent with
/// the reason; shot targets are absent when `shots_ok` is false.
pub fn league_targets(
    rows: &[MatchRow],
    shots_ok: bool,
    targets: &mut BTreeMap<String, Target>,
    absent: &mut BTreeMap<String, String>,
) {
    let mut by_div: BTreeMap<&str, Vec<&MatchRow>> = BTreeMap::new();
    for r in rows {
        by_div.entry(&r.div).or_default().push(r);
    }
    for (div, matches) in &by_div {
        for stat in &STATS {
            let key = league_key(div, stat.name);
            if stat.shots && !shots_ok {
                absent.insert(key, shots_refused());
                continue;
            }
            let mut per_season: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
            let mut all = Vec::new();
            for m in matches {
                if let Some(v) = (stat.of)(m) {
                    per_season.entry(&m.season).or_default().push(v);
                    all.push(v);
                }
            }
            let Some((mean, sd)) = mean_sd(&all) else {
                absent.insert(key, format!("the {div} files do not fill this column"));
                continue;
            };
            let season_means: Vec<f64> = per_season
                .values()
                .map(|v| v.iter().sum::<f64>() / v.len() as f64)
                .collect();
            let mut t = record(
                Some(mean),
                sd / (all.len() as f64).sqrt(),
                all.len() as u64,
                per_season.keys().map(|s| (*s).to_string()).collect(),
                Layer::League,
                &Desc {
                    event_definition: stat.event_definition,
                    denominator: stat.denominator,
                    unit: stat.unit,
                    engine: stat.engine,
                    band: stat.band,
                },
            );
            t.between_season_sd = mean_sd(&season_means).map(|(_, s)| s);
            targets.insert(key, t);
        }
        for dist in &DISTS {
            let key = league_key(div, &format!("dist.{}", dist.name));
            if dist.shots && !shots_ok {
                absent.insert(key, shots_refused());
                continue;
            }
            let mut counts = vec![0u64; dist.bins as usize];
            let mut seasons = BTreeSet::new();
            for m in matches {
                if let Some(b) = (dist.bin)(m, dist.bins, dist.width) {
                    counts[b as usize] += 1;
                    seasons.insert(m.season.clone());
                }
            }
            let n: u64 = counts.iter().sum();
            if n < 2 {
                absent.insert(key, format!("the {div} files do not fill this column"));
                continue;
            }
            targets.insert(
                key,
                dist_target(&counts, dist, seasons.into_iter().collect(), Layer::League),
            );
        }
    }
}

/// A distribution target from bin counts.
pub fn dist_target(counts: &[u64], dist: &Dist, seasons: Vec<String>, layer: Layer) -> Target {
    let n: u64 = counts.iter().sum();
    let shares: Vec<f64> = counts.iter().map(|&c| c as f64 / n as f64).collect();
    let se = shares
        .iter()
        .map(|p| (p * (1.0 - p) / n as f64).sqrt())
        .fold(0.0, f64::max);
    let mut t = record(
        None,
        se,
        n,
        seasons,
        layer,
        &Desc {
            event_definition: dist.event_definition,
            denominator: "matches",
            unit: "match",
            engine: dist.engine,
            band: dist.band,
        },
    );
    t.shares = Some(shares);
    t.bins = Some(dist.bins);
    t.width = Some(dist.width);
    t
}

fn shots_refused() -> String {
    "the shots cross-check did not find that HS and AS count blocked shots, so no shot \
     target is built"
        .into()
}

/// The sorted unique team names of `rows`.
pub fn team_names(rows: &[MatchRow]) -> Vec<String> {
    let names: BTreeSet<&str> = rows
        .iter()
        .flat_map(|r| [r.home.as_str(), r.away.as_str()])
        .collect();
    names.into_iter().map(str::to_string).collect()
}

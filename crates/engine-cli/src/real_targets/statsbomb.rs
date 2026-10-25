//! StatsBomb open data: event targets per competition-season and one pool of the men's
//! senior seasons, which the registry's StatsBomb bands read.
//!
//! The builder reads `sb/competitions.json`, every kept season's matches file and each
//! match's event file once. Three competition-seasons are refused by id and never read:
//! World Cup 2018 (held out) and two seasons another source already covers. Each match
//! gives its terms under the definitions the bands state (shots and shots on target, goals
//! and xG, shot distance, the possession team's pass and carry time by third, goals by the
//! play pattern of their shot, possession chains); a target sums them over its matches, so
//! every match weighs by its own events, and its error is over whole matches.
//!
//! Coordinates are StatsBomb's 120 by 80 yards, each event seen from the acting team
//! attacking the far goal, scaled to 105 by 68 metres. A shoot-out (period 5) is not play
//! and counts nowhere.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Context;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::football_data::mean_sd;
use super::{EngineCount, InputFile, Label, Layer, Target};
use crate::report::distribution::bin_of;
use crate::report::se_ratio;

/// The provider every target names; its credit is in the reference file.
pub const PROVIDER: &str = "StatsBomb";
/// The competition list, relative to the data folder.
pub const COMPETITIONS: &str = "sb/competitions.json";
/// The key prefix of the men's senior pool.
pub const POOL: &str = "men";

/// The competition-seasons never read, by `<competition>_<season>` id, with the reason.
pub const REFUSED: [(&str, &str); 3] = [
    (
        "43_3",
        "FIFA World Cup 2018 is held out: it checks the engine and never sets a target",
    ),
    (
        "11_1",
        "La Liga 2017/18 repeats the Wyscout Spain season the duel targets read",
    ),
    (
        "9_281",
        "1. Bundesliga 2023/24 repeats the IMPECT season of the local side file",
    ),
];

/// The bins of the three distributions: (kind, width, bins), as the bands bin them.
pub const SHOT_DISTANCE: (&str, f64, u32) = ("shot_distance", 1.0, 41);
pub const CHAIN_PASSES: (&str, f64, u32) = ("chain_passes", 1.0, 16);
pub const CHAIN_SECONDS: (&str, f64, u32) = ("chain_seconds", 2.0, 31);

const X_SCALE: f64 = 105.0 / 120.0;
const Y_SCALE: f64 = 68.0 / 80.0;

/// One competition-season of the list.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Competition {
    pub competition_id: u32,
    pub season_id: u32,
    pub competition_name: String,
    pub season_name: String,
    pub competition_gender: String,
    #[serde(default)]
    pub competition_youth: bool,
    #[serde(default)]
    pub competition_international: bool,
}

impl Competition {
    /// The id `<competition>_<season>`, as the matches file is named.
    pub fn label(&self) -> String {
        format!("{}_{}", self.competition_id, self.season_id)
    }

    /// Men's senior football, the engine's: the pool's seasons.
    pub fn men_senior(&self) -> bool {
        self.competition_gender == "male" && !self.competition_youth
    }

    /// Who plays: men's or women's, youth, club or international.
    pub fn context(&self) -> String {
        let who = if self.competition_gender == "male" {
            "men's"
        } else {
            "women's"
        };
        let youth = if self.competition_youth { " youth" } else { "" };
        let kind = if self.competition_international {
            "international"
        } else {
            "club"
        };
        format!("{who}{youth} {kind}")
    }

    /// The target's source label.
    pub fn source_label(&self) -> String {
        format!(
            "StatsBomb open data, {} {}",
            self.competition_name, self.season_name
        )
    }
}

/// Why `label` is never read, when it is refused.
pub fn refused(label: &str) -> Option<&'static str> {
    REFUSED.iter().find(|(id, _)| *id == label).map(|(_, r)| *r)
}

/// Refuses a read of a refused competition-season, naming it and the reason.
pub fn guard(label: &str) -> anyhow::Result<()> {
    match refused(label) {
        Some(why) => anyhow::bail!("StatsBomb competition-season {label} is not read: {why}"),
        None => Ok(()),
    }
}

/// The competition list of `data`.
pub fn competitions(data: &Path) -> anyhow::Result<Vec<Competition>> {
    let text = std::fs::read_to_string(data.join(COMPETITIONS))
        .with_context(|| format!("cannot read {COMPETITIONS}"))?;
    serde_json::from_str(&text).with_context(|| format!("{COMPETITIONS} cannot be read"))
}

/// The kept competition-seasons of `all`, in the list's order, the refused ones left out.
pub fn select(all: &[Competition]) -> Vec<Competition> {
    all.iter()
        .filter(|c| refused(&c.label()).is_none())
        .cloned()
        .collect()
}

#[derive(Deserialize)]
struct Name {
    name: String,
}

#[derive(Deserialize)]
struct Shot {
    outcome: Name,
    #[serde(default)]
    statsbomb_xg: Option<f64>,
    #[serde(rename = "type", default)]
    kind: Option<Name>,
}

/// One event, the fields the terms read; every other field is skipped.
#[derive(Deserialize)]
pub struct Event {
    period: u32,
    timestamp: String,
    #[serde(rename = "type")]
    kind: Name,
    team: Name,
    possession: u32,
    possession_team: Name,
    play_pattern: Name,
    #[serde(default)]
    location: Option<Vec<f64>>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    shot: Option<Shot>,
}

#[derive(Deserialize)]
struct MatchEntry {
    match_id: u64,
}

/// The terms of one match, both teams together.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MatchTerms {
    pub shots: f64,
    pub on_target: f64,
    /// Goals from shots (own goals are not shots).
    pub goals: f64,
    pub xg: f64,
    /// The summed metres from each shot to the centre of the goal.
    pub distance: f64,
    /// The possession team's pass and carry seconds by the third where each starts: own,
    /// middle, final.
    pub thirds: [f64; 3],
    /// Goals by origin: open play, corner, free kick, penalty, throw-in.
    pub origins: [f64; 5],
    /// The bin of each shot's distance, each possession's passes and its seconds.
    pub shot_bins: Vec<u32>,
    pub chain_pass_bins: Vec<u32>,
    pub chain_second_bins: Vec<u32>,
}

/// A time `HH:MM:SS.mmm` in seconds.
fn seconds(text: &str) -> f64 {
    text.split(':')
        .try_fold(0.0, |acc, part| {
            Some(acc * 60.0 + part.parse::<f64>().ok()?)
        })
        .unwrap_or(0.0)
}

/// The origin index of a goal: its shot's type first (a penalty, a direct free kick), else
/// the play pattern of its possession; `None` for a goal kick, keeper or kick-off origin.
fn origin(pattern: &str, shot_type: Option<&str>) -> Option<usize> {
    match shot_type {
        Some("Penalty") => return Some(3),
        Some("Free Kick") => return Some(2),
        _ => {}
    }
    match pattern {
        "Regular Play" | "From Counter" | "Other" => Some(0),
        "From Corner" => Some(1),
        "From Free Kick" => Some(2),
        "From Throw In" => Some(4),
        _ => None,
    }
}

/// The terms of a match's events, in file order.
pub fn match_terms(events: &[Event]) -> MatchTerms {
    let mut t = MatchTerms::default();
    // Each possession: passes, first time and period, and the end of its last event there.
    let mut chains: BTreeMap<u32, (u32, f64, u32, f64)> = BTreeMap::new();
    for e in events.iter().filter(|e| e.period <= 4) {
        let at = seconds(&e.timestamp);
        let own = e.team.name == e.possession_team.name;
        let chain = chains.entry(e.possession).or_insert((0, at, e.period, at));
        if e.period == chain.2 {
            chain.3 = at + e.duration.unwrap_or(0.0);
        }
        let point = e
            .location
            .as_deref()
            .filter(|l| l.len() >= 2)
            .map(|l| (l[0] * X_SCALE, l[1] * Y_SCALE));
        match e.kind.name.as_str() {
            "Pass" | "Carry" if own => {
                if e.kind.name == "Pass" {
                    chain.0 += 1;
                }
                if let (Some((x, _)), Some(d)) = (point, e.duration) {
                    let third = if x < 35.0 {
                        0
                    } else if x < 70.0 {
                        1
                    } else {
                        2
                    };
                    t.thirds[third] += d;
                }
            }
            "Shot" => {
                let Some(shot) = &e.shot else { continue };
                t.shots += 1.0;
                t.xg += shot.statsbomb_xg.unwrap_or(0.0);
                let outcome = shot.outcome.name.as_str();
                if matches!(outcome, "Goal" | "Saved" | "Saved to Post") {
                    t.on_target += 1.0;
                }
                if outcome == "Goal" {
                    t.goals += 1.0;
                    let kind = shot.kind.as_ref().map(|k| k.name.as_str());
                    if let Some(i) = origin(&e.play_pattern.name, kind) {
                        t.origins[i] += 1.0;
                    }
                }
                if let Some((x, y)) = point {
                    let d = (105.0 - x).hypot(34.0 - y);
                    t.distance += d;
                    t.shot_bins
                        .push(bin_of(d, SHOT_DISTANCE.1, SHOT_DISTANCE.2));
                }
            }
            _ => {}
        }
    }
    for (passes, start, _, end) in chains.into_values() {
        t.chain_pass_bins
            .push(bin_of(f64::from(passes), CHAIN_PASSES.1, CHAIN_PASSES.2));
        t.chain_second_bins.push(bin_of(
            (end - start).max(0.0),
            CHAIN_SECONDS.1,
            CHAIN_SECONDS.2,
        ));
    }
    t
}

/// The terms of an event file's text.
pub fn parse_terms(text: &str) -> anyhow::Result<MatchTerms> {
    let events: Vec<Event> = serde_json::from_str(text)?;
    Ok(match_terms(&events))
}

/// One scalar statistic: the summed `num` over the summed `den` of its matches.
pub struct Scalar {
    pub band: &'static str,
    pub num: fn(&MatchTerms) -> f64,
    pub den: fn(&MatchTerms) -> f64,
    pub event_definition: &'static str,
    pub denominator: &'static str,
    pub engine: &'static str,
}

fn third_total(t: &MatchTerms) -> f64 {
    t.thirds.iter().sum()
}

/// The eleven scalar statistics, by the band that counts each.
pub const SCALARS: [Scalar; 11] = [
    Scalar {
        band: "shots_on_target_share",
        num: |t| t.on_target,
        den: |t| t.shots,
        event_definition: "shots whose outcome is a goal, a save or a save onto the post, \
                           periods 1 to 4",
        denominator: "shots, blocked shots counted",
        engine: "ratio of shots_on_target over shots",
    },
    Scalar {
        band: "goals_per_xg",
        num: |t| t.goals,
        den: |t| t.xg,
        event_definition: "goals from shots over StatsBomb's summed xG, periods 1 to 4",
        denominator: "summed StatsBomb xG",
        engine: "ratio of goals over xg",
    },
    Scalar {
        band: "shot_distance_mean",
        num: |t| t.distance,
        den: |t| t.shots,
        event_definition: "metres from each shot to the centre of the goal on a 105 by 68 \
                           pitch",
        denominator: "shots",
        engine: "ratio of shot_distance_m over shots",
    },
    Scalar {
        band: "possession_own_third_share",
        num: |t| t.thirds[0],
        den: third_total,
        event_definition: "seconds of the possession team's passes and carries that start \
                           in its own third (0 to 35 m)",
        denominator: "seconds of the possession team's passes and carries",
        engine: "ratio of third_ticks.own over all third_ticks",
    },
    Scalar {
        band: "possession_middle_third_share",
        num: |t| t.thirds[1],
        den: third_total,
        event_definition: "seconds of the possession team's passes and carries that start \
                           in the middle third (35 to 70 m)",
        denominator: "seconds of the possession team's passes and carries",
        engine: "ratio of third_ticks.middle over all third_ticks",
    },
    Scalar {
        band: "possession_final_third_share",
        num: |t| t.thirds[2],
        den: third_total,
        event_definition: "seconds of the possession team's passes and carries that start \
                           in the final third (70 to 105 m)",
        denominator: "seconds of the possession team's passes and carries",
        engine: "ratio of third_ticks.final over all third_ticks",
    },
    Scalar {
        band: "goals_open_play_share",
        num: |t| t.origins[0],
        den: |t| t.goals,
        event_definition: "goals whose possession started in regular play, a counter or \
                           another open-play pattern",
        denominator: "goals from shots",
        engine: "ratio of goals_phase.open_play over goals",
    },
    Scalar {
        band: "goals_corner_share",
        num: |t| t.origins[1],
        den: |t| t.goals,
        event_definition: "goals whose possession started from a corner",
        denominator: "goals from shots",
        engine: "ratio of goals_phase.corner over goals",
    },
    Scalar {
        band: "goals_free_kick_share",
        num: |t| t.origins[2],
        den: |t| t.goals,
        event_definition: "goals from a direct free kick or a possession started from a free \
                           kick",
        denominator: "goals from shots",
        engine: "ratio of goals_phase.free_kick over goals",
    },
    Scalar {
        band: "goals_penalty_share",
        num: |t| t.origins[3],
        den: |t| t.goals,
        event_definition: "goals from penalties in play (no shoot-out)",
        denominator: "goals from shots",
        engine: "ratio of goals_phase.penalty over goals",
    },
    Scalar {
        band: "goals_throw_in_share",
        num: |t| t.origins[4],
        den: |t| t.goals,
        event_definition: "goals whose possession started from a throw-in",
        denominator: "goals from shots",
        engine: "ratio of goals_phase.throw_in over goals",
    },
];

/// One distribution: its bins and the per-match bin list it reads.
pub struct Dist {
    pub kind: (&'static str, f64, u32),
    pub band: &'static str,
    pub of: fn(&MatchTerms) -> &Vec<u32>,
    pub event_definition: &'static str,
    pub engine: &'static str,
}

/// The three distributions.
pub const DISTS: [Dist; 3] = [
    Dist {
        kind: SHOT_DISTANCE,
        band: "shot_distance_distribution",
        of: |t| &t.shot_bins,
        event_definition: "metres from each shot to the centre of the goal, 1 m bins, 40 m \
                           and over as one",
        engine: "distribution of shot_distance",
    },
    Dist {
        kind: CHAIN_PASSES,
        band: "chain_passes_distribution",
        of: |t| &t.chain_pass_bins,
        event_definition: "each StatsBomb possession as a chain, its passes by the team in \
                           possession, 15 or more as one bin",
        engine: "distribution of chain_passes",
    },
    Dist {
        kind: CHAIN_SECONDS,
        band: "chain_seconds_distribution",
        of: |t| &t.chain_second_bins,
        event_definition: "each StatsBomb possession as a chain, its seconds from its first \
                           event to the end of its last event within one period, 2-second \
                           bins, 60 s and over as one",
        engine: "distribution of chain_seconds",
    },
];

/// The key of a StatsBomb target: `sb.<label>.<stat>` or `sb.<label>.dist.<kind>`.
pub fn key(label: &str, stat: &str) -> String {
    format!("sb.{label}.{stat}")
}

/// The matches of one kept competition-season, with their terms.
#[derive(Debug, Clone)]
pub struct Season {
    pub competition: Competition,
    pub matches: Vec<(u64, MatchTerms)>,
}

/// Who a set of targets speaks for: one competition-season or the pool.
struct Scope<'a> {
    label: String,
    source_label: String,
    context: String,
    layer: Layer,
    seasons: Vec<String>,
    leagues: Option<Vec<String>>,
    matches: Vec<&'a MatchTerms>,
    /// The pool's spread between its seasons, per statistic.
    between: BTreeMap<String, f64>,
}

fn record(
    scope: &Scope<'_>,
    value: Option<f64>,
    se: f64,
    text: (&str, &str, &str),
    band: &str,
) -> Target {
    let (event_definition, denominator, engine) = text;
    Target {
        value,
        shares: None,
        bins: None,
        width: None,
        se,
        n: scope.matches.len() as u64,
        seasons: scope.seasons.clone(),
        between_season_sd: scope.between.get(band).copied(),
        between_league_sd: None,
        layer: scope.layer,
        leagues: scope.leagues.clone(),
        provider: PROVIDER.into(),
        source_label: scope.source_label.clone(),
        event_definition: event_definition.into(),
        denominator: denominator.into(),
        unit_of_observation: "match".into(),
        label: Label::Calibration,
        engine_count: EngineCount {
            text: engine.into(),
            band: Some(band.into()),
            measure: Some(engine.into()),
        },
        context: Some(scope.context.clone()),
        extrapolated: None,
        formula: None,
        per_match: None,
    }
}

/// The scalar value of `s` over `matches`, and its error over whole matches; `None` when
/// the matches hold no denominator.
pub fn scalar(s: &Scalar, matches: &[&MatchTerms]) -> Option<(f64, f64)> {
    let ys: Vec<f64> = matches.iter().map(|t| (s.num)(t)).collect();
    let xs: Vec<f64> = matches.iter().map(|t| (s.den)(t)).collect();
    let den: f64 = xs.iter().sum();
    (den > 0.0).then(|| (ys.iter().sum::<f64>() / den, se_ratio(&ys, &xs)))
}

/// The shares of `d` over `matches`, and the error of the largest share over whole
/// matches; `None` when the matches hold no point.
pub fn shares(d: &Dist, matches: &[&MatchTerms]) -> Option<(Vec<f64>, f64)> {
    let bins = d.kind.2 as usize;
    let mut counts = vec![0.0; bins];
    let mut per_match = Vec::with_capacity(matches.len());
    for t in matches {
        let mut c = vec![0.0; bins];
        for &b in (d.of)(t) {
            c[b as usize] += 1.0;
        }
        for (all, one) in counts.iter_mut().zip(&c) {
            *all += one;
        }
        per_match.push(c);
    }
    let total: f64 = counts.iter().sum();
    if total <= 0.0 {
        return None;
    }
    let shares: Vec<f64> = counts.iter().map(|c| c / total).collect();
    let top = (0..bins).fold(0, |best, i| if shares[i] > shares[best] { i } else { best });
    let ys: Vec<f64> = per_match.iter().map(|c| c[top]).collect();
    let xs: Vec<f64> = per_match.iter().map(|c| c.iter().sum()).collect();
    Some((shares, se_ratio(&ys, &xs)))
}

fn targets_of(scope: &Scope<'_>, out: &mut BTreeMap<String, Target>) {
    for s in &SCALARS {
        if let Some((value, se)) = scalar(s, &scope.matches) {
            out.insert(
                key(&scope.label, s.band),
                record(
                    scope,
                    Some(value),
                    se,
                    (s.event_definition, s.denominator, s.engine),
                    s.band,
                ),
            );
        }
    }
    for d in &DISTS {
        if let Some((shares, se)) = shares(d, &scope.matches) {
            let mut t = record(
                scope,
                None,
                se,
                (d.event_definition, "matches", d.engine),
                d.band,
            );
            t.shares = Some(shares);
            t.bins = Some(d.kind.2);
            t.width = Some(d.kind.1);
            out.insert(key(&scope.label, &format!("dist.{}", d.kind.0)), t);
        }
    }
}

/// The targets of `seasons`: one set per competition-season, and the men's senior pool
/// (`sb.men.<stat>`, `sb.men.dist.<kind>`) over every match of its seasons, with the
/// spread between those seasons.
pub fn targets(seasons: &[Season]) -> BTreeMap<String, Target> {
    let mut out = BTreeMap::new();
    for s in seasons {
        let label = s.competition.label();
        targets_of(
            &Scope {
                label: label.clone(),
                source_label: s.competition.source_label(),
                context: s.competition.context(),
                layer: Layer::League,
                seasons: vec![label],
                leagues: None,
                matches: s.matches.iter().map(|(_, t)| t).collect(),
                between: BTreeMap::new(),
            },
            &mut out,
        );
    }
    let pool: Vec<&Season> = seasons
        .iter()
        .filter(|s| s.competition.men_senior() && !s.matches.is_empty())
        .collect();
    if pool.is_empty() {
        return out;
    }
    let labels: Vec<String> = pool.iter().map(|s| s.competition.label()).collect();
    let mut between = BTreeMap::new();
    for s in &SCALARS {
        let per_season: Vec<f64> = pool
            .iter()
            .filter_map(|p| {
                let m: Vec<&MatchTerms> = p.matches.iter().map(|(_, t)| t).collect();
                scalar(s, &m).map(|(v, _)| v)
            })
            .collect();
        if let Some((_, sd)) = mean_sd(&per_season) {
            between.insert(s.band.to_string(), sd);
        }
    }
    targets_of(
        &Scope {
            label: POOL.into(),
            source_label: format!(
                "StatsBomb open data, men's senior matches, {} competition-seasons",
                pool.len()
            ),
            context: "men's senior club and international".into(),
            layer: Layer::Pooled,
            seasons: labels.clone(),
            leagues: Some(labels),
            matches: pool
                .iter()
                .flat_map(|s| s.matches.iter().map(|(_, t)| t))
                .collect(),
            between,
        },
        &mut out,
    );
    out
}

/// One row of the per-match histogram file: a match's count in one bin of one kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistRow {
    pub label: String,
    pub match_id: u64,
    pub kind: &'static str,
    pub bin: u32,
    pub count: u32,
}

/// The per-match histograms of the pool's matches, each non-empty bin one row, in season,
/// match, kind and bin order.
pub fn histogram_rows(seasons: &[Season]) -> Vec<HistRow> {
    let mut out = Vec::new();
    for s in seasons.iter().filter(|s| s.competition.men_senior()) {
        let label = s.competition.label();
        let mut matches: Vec<&(u64, MatchTerms)> = s.matches.iter().collect();
        matches.sort_by_key(|(id, _)| *id);
        for (id, t) in matches {
            for d in &DISTS {
                let mut counts: BTreeMap<u32, u32> = BTreeMap::new();
                for &b in (d.of)(t) {
                    *counts.entry(b).or_default() += 1;
                }
                for (bin, count) in counts {
                    out.push(HistRow {
                        label: label.clone(),
                        match_id: *id,
                        kind: d.kind.0,
                        bin,
                        count,
                    });
                }
            }
        }
    }
    out
}

/// The histogram rows as CSV text.
pub fn histogram_csv(rows: &[HistRow]) -> String {
    let mut out = String::from("label,match,kind,bin,count\n");
    for r in rows {
        let _ = writeln!(
            out,
            "{},{},{},{},{}",
            r.label, r.match_id, r.kind, r.bin, r.count
        );
    }
    out
}

/// What the StatsBomb step read and built.
#[derive(Debug, Clone)]
pub struct Built {
    pub seasons: Vec<Season>,
    pub targets: BTreeMap<String, Target>,
    /// Files read whole, relative to the data folder, for `built_from`.
    pub files: Vec<String>,
    /// One record per kept competition-season's event files.
    pub event_groups: Vec<InputFile>,
}

impl Built {
    /// The summary lines: the kept and refused seasons, the matches, and the pool.
    pub fn summary(&self) -> String {
        let matches: usize = self.seasons.iter().map(|s| s.matches.len()).sum();
        let mut out = format!(
            "statsbomb: {} competition-seasons, {matches} matches\n",
            self.seasons.len()
        );
        for (id, why) in REFUSED {
            let _ = writeln!(out, "  refused {id}: {why}");
        }
        let pool: Vec<&Season> = self
            .seasons
            .iter()
            .filter(|s| s.competition.men_senior())
            .collect();
        let _ = writeln!(
            out,
            "  men's senior pool: {} matches in {} competition-seasons",
            pool.iter().map(|s| s.matches.len()).sum::<usize>(),
            pool.len()
        );
        let mut by_context: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for s in &self.seasons {
            let e = by_context.entry(s.competition.context()).or_default();
            e.0 += 1;
            e.1 += s.matches.len();
        }
        for (context, (labels, n)) in by_context {
            let _ = writeln!(
                out,
                "  {context}: {labels} competition-seasons, {n} matches"
            );
        }
        out
    }
}

/// One match file read: its season index, match id, terms and sha256.
type ReadMatch = (usize, u64, MatchTerms, String);

/// Reads the kept seasons of `data`, their matches files and each match's event file
/// (several at once), and builds their targets; `None` when the data folder holds no
/// competition list.
pub fn build(data: &Path) -> anyhow::Result<Option<Built>> {
    if !data.join(COMPETITIONS).is_file() {
        return Ok(None);
    }
    let all = competitions(data)?;
    let kept = select(&all);
    let mut files = vec![COMPETITIONS.to_string()];
    // Every (season index, match id) to read.
    let mut jobs: Vec<(usize, u64)> = Vec::new();
    for (i, c) in kept.iter().enumerate() {
        let label = c.label();
        guard(&label)?;
        let rel = format!("sb/matches/{label}.json");
        let text = std::fs::read_to_string(data.join(&rel))
            .with_context(|| format!("cannot read {rel}"))?;
        let list: Vec<MatchEntry> =
            serde_json::from_str(&text).with_context(|| format!("{rel} cannot be read"))?;
        files.push(rel);
        let ids: BTreeSet<u64> = list.into_iter().map(|m| m.match_id).collect();
        jobs.extend(ids.into_iter().map(|id| (i, id)));
    }
    let next = AtomicUsize::new(0);
    let done: Mutex<Vec<anyhow::Result<ReadMatch>>> = Mutex::new(Vec::with_capacity(jobs.len()));
    let workers = std::thread::available_parallelism()
        .map_or(4, std::num::NonZero::get)
        .min(8);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(season, id)) = jobs.get(k) else {
                        break;
                    };
                    let rel = format!("sb/events/{id}.json");
                    let got = std::fs::read(data.join(&rel))
                        .with_context(|| format!("cannot read {rel}"))
                        .and_then(|bytes| {
                            let digest = stream::record::hex(&Sha256::digest(&bytes));
                            let text = std::str::from_utf8(&bytes)
                                .with_context(|| format!("{rel} is not UTF-8"))?;
                            let terms = parse_terms(text)
                                .with_context(|| format!("{rel} cannot be read"))?;
                            Ok((season, id, terms, digest))
                        });
                    done.lock().expect("no worker panics").push(got);
                }
            });
        }
    });
    let mut per_season: Vec<Vec<(u64, MatchTerms, String)>> = vec![Vec::new(); kept.len()];
    for got in done.into_inner().expect("no worker panics") {
        let (season, id, terms, digest) = got?;
        per_season[season].push((id, terms, digest));
    }
    let mut event_groups = Vec::new();
    let mut seasons = Vec::new();
    for (c, mut matches) in kept.into_iter().zip(per_season) {
        matches.sort_by_key(|(id, _, _)| *id);
        let label = c.label();
        let mut hasher = Sha256::new();
        let mut bytes = 0u64;
        for (id, _, digest) in &matches {
            hasher.update(digest.as_bytes());
            bytes +=
                std::fs::metadata(data.join(format!("sb/events/{id}.json"))).map_or(0, |m| m.len());
        }
        event_groups.push(InputFile {
            path: format!("sb/events/{label}"),
            bytes,
            sha256: stream::record::hex(&hasher.finalize()),
        });
        seasons.push(Season {
            competition: c,
            matches: matches.into_iter().map(|(id, t, _)| (id, t)).collect(),
        });
    }
    let targets = targets(&seasons);
    Ok(Some(Built {
        seasons,
        targets,
        files,
        event_groups,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comp(cid: u32, sid: u32, gender: &str, youth: bool, intl: bool) -> Competition {
        Competition {
            competition_id: cid,
            season_id: sid,
            competition_name: format!("Comp {cid}"),
            season_name: format!("Season {sid}"),
            competition_gender: gender.into(),
            competition_youth: youth,
            competition_international: intl,
        }
    }

    fn terms(text: &str) -> MatchTerms {
        parse_terms(text).unwrap()
    }

    /// A pass and a goal from 7 m by the team in possession, then a shoot-out kick.
    const TWO: &str = r#"[
      {"period": 1, "timestamp": "00:00:01.000", "type": {"name": "Pass"},
       "team": {"name": "A"}, "possession": 2, "possession_team": {"name": "A"},
       "play_pattern": {"name": "From Corner"}, "location": [100.0, 40.0], "duration": 1.5},
      {"period": 1, "timestamp": "00:00:03.000", "type": {"name": "Shot"},
       "team": {"name": "A"}, "possession": 2, "possession_team": {"name": "A"},
       "play_pattern": {"name": "From Corner"}, "location": [112.0, 40.0], "duration": 0.5,
       "shot": {"outcome": {"name": "Goal"}, "statsbomb_xg": 0.25, "type": {"name": "Open Play"}}},
      {"period": 5, "timestamp": "00:00:00.000", "type": {"name": "Shot"},
       "team": {"name": "B"}, "possession": 3, "possession_team": {"name": "B"},
       "play_pattern": {"name": "Other"}, "location": [108.0, 40.0], "duration": 0.5,
       "shot": {"outcome": {"name": "Goal"}, "statsbomb_xg": 0.78, "type": {"name": "Penalty"}}}
    ]"#;

    #[test]
    fn each_term_of_a_two_event_match() {
        let t = terms(TWO);
        assert_eq!((t.shots, t.on_target, t.goals), (1.0, 1.0, 1.0));
        assert!((t.xg - 0.25).abs() < 1e-12);
        // (112 x 105/120, 40 x 68/80) = (98, 34): 7 m from the goal centre.
        assert!((t.distance - 7.0).abs() < 1e-9, "{}", t.distance);
        assert_eq!(t.shot_bins, vec![7]);
        // The pass starts at 87.5 m: the final third.
        assert_eq!(t.thirds, [0.0, 0.0, 1.5]);
        assert_eq!(t.origins, [0.0, 1.0, 0.0, 0.0, 0.0]);
        // One chain of one pass, from 1 s to 3.5 s; the shoot-out is no chain.
        assert_eq!(t.chain_pass_bins, vec![1]);
        assert_eq!(t.chain_second_bins, vec![1]);
    }

    #[test]
    fn a_penalty_and_a_direct_free_kick_go_by_their_shot_type() {
        assert_eq!(origin("Other", Some("Penalty")), Some(3));
        assert_eq!(origin("Regular Play", Some("Free Kick")), Some(2));
        assert_eq!(origin("From Counter", Some("Open Play")), Some(0));
        assert_eq!(origin("From Throw In", None), Some(4));
        assert_eq!(origin("From Goal Kick", None), None);
    }

    #[test]
    fn each_refused_id_is_left_out_and_a_read_of_it_errors_naming_it() {
        let all = vec![
            comp(2, 27, "male", false, false),
            comp(43, 3, "male", false, true),
            comp(11, 1, "male", false, false),
            comp(9, 281, "male", false, false),
            comp(43, 106, "male", false, true),
        ];
        let kept: Vec<String> = select(&all).iter().map(Competition::label).collect();
        assert_eq!(kept, ["2_27", "43_106"]);
        for (id, why) in [
            ("43_3", "held out"),
            ("11_1", "Wyscout"),
            ("9_281", "IMPECT"),
        ] {
            let e = format!("{:#}", guard(id).unwrap_err());
            assert!(e.contains(id) && e.contains(why), "{e}");
        }
        assert!(guard("2_27").is_ok());
    }

    fn season(c: Competition, matches: &[&str]) -> Season {
        Season {
            competition: c,
            matches: matches
                .iter()
                .enumerate()
                .map(|(i, m)| (i as u64, terms(m)))
                .collect(),
        }
    }

    #[test]
    fn the_pool_leaves_out_womens_and_youth_seasons() {
        let seasons = vec![
            season(comp(2, 27, "male", false, false), &[TWO]),
            season(comp(55, 43, "male", false, true), &[TWO]),
            season(comp(37, 90, "female", false, false), &[TWO]),
            season(comp(1470, 274, "male", true, true), &[TWO]),
        ];
        let t = targets(&seasons);
        let pool = &t["sb.men.goals_per_xg"];
        assert_eq!(pool.seasons, ["2_27", "55_43"]);
        assert_eq!(pool.n, 2);
        assert_eq!(pool.layer, Layer::Pooled);
        assert!(pool.source_label.contains("2 competition-seasons"));
        // Every season keeps its own labelled targets.
        assert_eq!(
            t["sb.37_90.goals_per_xg"].context.as_deref(),
            Some("women's club")
        );
        assert_eq!(
            t["sb.1470_274.goals_per_xg"].context.as_deref(),
            Some("men's youth international")
        );
        assert!(
            t["sb.2_27.shots_on_target_share"]
                .source_label
                .starts_with("StatsBomb open data, Comp 2")
        );
        let rows = histogram_rows(&seasons);
        assert!(rows.iter().all(|r| r.label == "2_27" || r.label == "55_43"));
    }

    #[test]
    fn a_share_and_a_ratio_and_their_errors_match_a_hand_calculation() {
        let a = MatchTerms {
            shots: 3.0,
            on_target: 3.0,
            ..MatchTerms::default()
        };
        let b = MatchTerms {
            shots: 4.0,
            on_target: 4.0,
            ..MatchTerms::default()
        };
        let c = MatchTerms {
            shots: 1.0,
            on_target: 0.0,
            ..MatchTerms::default()
        };
        let (v, se) = scalar(&SCALARS[0], &[&a, &b, &c]).unwrap();
        assert!((v - 0.875).abs() < 1e-12);
        // Residuals 0.375, 0.5, -0.875: sqrt(1.15625 / 6) / (8 / 3).
        let hand = (1.15625_f64 / 6.0).sqrt() / (8.0 / 3.0);
        assert!((se - hand).abs() < 1e-12, "{se} {hand}");

        // Shot distance bins 7, 7 | 14 | 7: shares of bin 7 three quarters.
        let d = |bins: Vec<u32>| MatchTerms {
            shot_bins: bins,
            ..MatchTerms::default()
        };
        let (m1, m2, m3) = (d(vec![7, 7]), d(vec![14]), d(vec![7]));
        let (s, se) = shares(&DISTS[0], &[&m1, &m2, &m3]).unwrap();
        assert_eq!(s.len(), 41);
        assert!((s[7] - 0.75).abs() < 1e-12 && (s[14] - 0.25).abs() < 1e-12);
        // Bin 7 per match 2, 0, 1 over totals 2, 1, 1: the ratio error by hand.
        let hand = se_ratio(&[2.0, 0.0, 1.0], &[2.0, 1.0, 1.0]);
        assert!((se - hand).abs() < 1e-12 && se > 0.0);
    }
}

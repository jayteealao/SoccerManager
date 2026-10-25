//! Assembles the reference file from every source, with its credits and the record of
//! each raw file read, and writes it with sorted keys so two builds of the same files give
//! the same bytes. Each source has its own step, so a later source adds its targets
//! without a change of format.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::Context;
use serde::Deserialize;

use super::data_check::{DIVISIONS, SEASONS, clubelo_snapshots, sha256_file};
use super::{
    Builder, Credit, Definitions, InputFile, Layer, NAMES_DIR, ReferenceFile, SCHEMA_VERSION,
    Target, duels, football_data, fotmob, layers, shots_check, tables,
};
use crate::stats::round_to;

/// The places every figure of the file keeps.
const PLACES: i32 = 6;

/// A built reference file and what the build printed.
#[derive(Debug)]
pub struct Built {
    pub file: ReferenceFile,
    /// Real club names of the result files, for the packaging scan.
    pub names: Vec<String>,
    pub shots_table: String,
    /// Wyscout duel rows left without a partner, per league.
    pub unpaired: BTreeMap<String, u32>,
    /// Result rows left out (no result or no teams), per file.
    pub skipped: BTreeMap<String, u32>,
    pub banding_groups: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct Structure {
    associations: Vec<Association>,
}

#[derive(Deserialize)]
struct Association {
    code: String,
    #[serde(default)]
    movement: BTreeMap<String, Movement>,
}

#[derive(Deserialize)]
struct Movement {
    #[serde(default)]
    relegated_direct: Option<u32>,
}

/// The direct relegation places of each division from `competitions/domestic-structure.json`
/// (the move from its tier to the next); `None` where the file does not establish it.
fn relegation(data: &Path) -> anyhow::Result<BTreeMap<String, Option<u32>>> {
    let rel = "competitions/domestic-structure.json";
    let text =
        std::fs::read_to_string(data.join(rel)).with_context(|| format!("cannot read {rel}"))?;
    let s: Structure =
        serde_json::from_str(&text).with_context(|| format!("{rel} cannot be read"))?;
    Ok(DIVISIONS
        .iter()
        .map(|(div, country, tier)| {
            let places = s
                .associations
                .iter()
                .find(|a| a.code == *country)
                .and_then(|a| a.movement.get(&format!("{tier}-{}", tier + 1)))
                .and_then(|m| m.relegated_direct);
            ((*div).to_string(), places)
        })
        .collect())
}

fn credits() -> BTreeMap<String, Credit> {
    let c = |name: &str, source: &str, licence: &str, use_note: &str| {
        (
            name.to_string(),
            Credit {
                name: name.into(),
                source: source.into(),
                licence: licence.into(),
                use_note: use_note.into(),
            },
        )
    };
    [
        c(
            football_data::PROVIDER,
            "https://www.football-data.co.uk, season result files, downloaded by hand",
            "free for private individuals; no automated or AI fetching",
            "measuring only, not commercial; the game ships none of the data",
        ),
        c(
            fotmob::PROVIDER,
            "https://www.fotmob.com team accurate-pass tables (Opta data), read by hand",
            "FotMob terms: no scraping, bulk retrieval or commercial use without consent",
            "measuring only, not commercial; the game ships none of the data",
        ),
        c(
            duels::PROVIDER,
            "Pappalardo et al. (2019), A public data set of spatio-temporal match events in \
             soccer competitions, Scientific Data 6:236",
            "CC BY 4.0",
            "derived figures with credit",
        ),
        c(
            "StatsBomb",
            "StatsBomb open data, https://github.com/statsbomb/open-data",
            "StatsBomb open data user agreement, with credit",
            "used here only for the blocked-shot cross-check",
        ),
        c(
            "ClubElo",
            "http://clubelo.com, Lars Schiefler; ranking snapshot",
            "not stated; cited",
            "club strength for the banding groups, measuring only",
        ),
        c(
            "Wikipedia",
            "English Wikipedia league pages, facts only (domestic-structure.json)",
            "CC BY-SA 4.0",
            "relegation places, facts only",
        ),
    ]
    .into_iter()
    .collect()
}

fn round(t: &mut Target) {
    let r = |x: f64| round_to(x, PLACES);
    t.value = t.value.map(r);
    t.se = r(t.se);
    t.between_season_sd = t.between_season_sd.map(r);
    t.between_league_sd = t.between_league_sd.map(r);
    if let Some(s) = &mut t.shares {
        for p in s.iter_mut() {
            *p = r(*p);
        }
    }
    if let Some(f) = &mut t.formula {
        // The strength coefficient is per rating point: keep more places.
        f.intercept = r(f.intercept);
        f.tier = r(f.tier);
        f.strength = round_to(f.strength, PLACES + 4);
        f.residual_sd = r(f.residual_sd);
    }
}

/// Builds the reference file from the raw files of `data`; `git_head` names the builder's
/// commit.
pub fn build(data: &Path, git_head: &str) -> anyhow::Result<Built> {
    let mut targets = BTreeMap::new();
    let mut absent = BTreeMap::new();
    let mut read: Vec<String> = Vec::new();

    // Shots: decide first, so the shot targets follow the answer.
    let (lines, shots, files) = shots_check::run(data, &shots_check::BUNDESLIGA_2324)?;
    let shots_table = shots_check::table(&lines, &shots);
    read.extend(files);

    let divisions: Vec<&str> = DIVISIONS.iter().map(|(d, _, _)| *d).collect();
    let rows = football_data::match_rows(data, &divisions, &SEASONS)?;
    read.extend(rows.files.iter().cloned());
    for rel in &rows.missing {
        absent.insert(
            format!("file.{rel}"),
            "the raw file is not in the data folder".into(),
        );
    }
    football_data::league_targets(&rows.rows, shots.include_blocked, &mut targets, &mut absent);

    let relegation_places = relegation(data)?;
    read.push("competitions/domestic-structure.json".into());
    tables::league_targets(&rows.rows, &relegation_places, &mut targets, &mut absent);

    let snapshot = clubelo_snapshots(data)
        .pop()
        .context("no ClubElo snapshot clubelo/clubelo-<date>.csv in the data folder")?;
    let strength = layers::strength(&snapshot)?;
    read.push(format!(
        "clubelo/{}",
        snapshot
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    ));
    let division_strength: BTreeMap<String, f64> = DIVISIONS
        .iter()
        .filter_map(|(d, c, t)| {
            strength
                .leagues
                .get(&((*c).to_string(), *t))
                .map(|(e, _)| ((*d).to_string(), round_to(*e, 3)))
        })
        .collect();
    layers::tier_targets(&mut targets);
    let banding_groups =
        layers::group_targets(&mut targets, &mut absent, &strength, &division_strength);

    let (unpaired, opened) = duels::targets(data, &mut targets, &mut absent)?;
    read.extend(opened);
    read.extend(fotmob::targets(data, &mut targets, &mut absent)?);

    for t in targets.values_mut() {
        round(t);
    }
    read.sort();
    read.dedup();
    let built_from = read
        .iter()
        .map(|rel| {
            let (sha256, bytes) =
                sha256_file(&data.join(rel)).with_context(|| format!("cannot hash {rel}"))?;
            Ok(InputFile {
                path: rel.clone(),
                bytes,
                sha256,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let file = ReferenceFile {
        schema_version: SCHEMA_VERSION,
        builder: Builder {
            version: env!("CARGO_PKG_VERSION").into(),
            git_head: git_head.into(),
        },
        built_from,
        credits: credits(),
        definitions: Definitions {
            shots_include_blocked: shots.include_blocked,
            shots_check: shots,
            tiers: DIVISIONS
                .iter()
                .map(|(d, _, t)| ((*d).to_string(), *t))
                .collect(),
            relegation_places,
            strength: division_strength,
            seasons: SEASONS.iter().map(|s| (*s).to_string()).collect(),
        },
        targets,
        absent,
    };
    Ok(Built {
        file,
        names: football_data::team_names(&rows.rows),
        shots_table,
        unpaired,
        skipped: rows.skipped,
        banding_groups,
    })
}

/// Writes the reference file to `out` and the real-name list beside it in
/// `real-names/football-data.json`.
pub fn write(built: &Built, out: &Path) -> anyhow::Result<()> {
    let dir = out.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir.join(NAMES_DIR))
        .with_context(|| format!("cannot create {}", dir.join(NAMES_DIR).display()))?;
    let text = serde_json::to_string_pretty(&built.file)? + "\n";
    std::fs::write(out, text).with_context(|| format!("cannot write {}", out.display()))?;
    let names_path = dir.join(NAMES_DIR).join("football-data.json");
    let names = serde_json::to_string_pretty(&built.names)? + "\n";
    std::fs::write(&names_path, names)
        .with_context(|| format!("cannot write {}", names_path.display()))?;
    Ok(())
}

impl Built {
    /// The printed summary: the shots check, the targets per layer and the absent ones.
    pub fn summary(&self) -> String {
        let mut out = self.shots_table.clone();
        let mut layers: BTreeMap<&str, u32> = BTreeMap::new();
        for t in self.file.targets.values() {
            let name = match t.layer {
                Layer::League => "league",
                Layer::Tier => "tier",
                Layer::Group => "group",
                Layer::Pooled => "pooled",
            };
            *layers.entry(name).or_default() += 1;
        }
        let _ = writeln!(out, "targets: {}", self.file.targets.len());
        for (layer, n) in &layers {
            let _ = writeln!(out, "  {layer:<7} {n}");
        }
        for key in [
            "tier.1.goals_per_match",
            "tier.1.shots_per_team",
            "wy.pooled.duel_contests_per_match",
            "fotmob.pooled.pass_completion",
        ] {
            if let Some(v) = self.file.target(key).and_then(|t| t.value) {
                let _ = writeln!(out, "  {key} = {v:.3}");
            }
        }
        let _ = writeln!(out, "banding groups: {}", self.banding_groups.len());
        for (league, n) in &self.unpaired {
            let _ = writeln!(
                out,
                "wyscout {league}: {n} unpaired duel rows, each one contest"
            );
        }
        for (file, n) in &self.skipped {
            let _ = writeln!(out, "{file}: {n} rows without a result left out");
        }
        let _ = writeln!(out, "absent targets: {}", self.file.absent.len());
        for (key, why) in &self.file.absent {
            let _ = writeln!(out, "  {key}: {why}");
        }
        let _ = writeln!(out, "real club names: {}", self.names.len());
        out
    }
}

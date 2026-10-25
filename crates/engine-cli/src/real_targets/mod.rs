//! The reference file of real targets: every real figure the harness checks the engine
//! against, with its full record (provider, event definition, denominator, unit of
//! observation, calibration or validation label, and how the engine counts the same thing)
//! and the credit of its source.
//!
//! The file is built once by `engine-cli real-targets build` from the raw files read in
//! place on this PC (the main checkout's `.scratch/data`), and committed in `gate/`, which
//! never ships with the game. Only derived per-target figures enter it; the raw match rows
//! stay on this PC, where [`football_data::match_rows`] reads them for whole-match
//! resampling.
//!
//! A target is named by key:
//! - `fd.<div>.<stat>`: one league (a football-data.co.uk division) over five seasons;
//! - `tier.<n>.<stat>`: the leagues of one tier, each with equal weight;
//! - `group.t<n>.e<elo>.<stat>`: a banding group of leagues without data, by tier and a
//!   50-point ClubElo strength bin, from a formula fitted on the leagues with data;
//! - `fd.<div>.table.<stat>`, `tier.<n>.table.<stat>`, `group.….table.<stat>`: the
//!   league table;
//! - `fd.<div>.dist.<scorelines|goals|shots|cards>`, `tier.<n>.dist.…`: whole
//!   distributions;
//! - `wy.<league>.duel_contests_per_match`, `wy.pooled.duel_contests_per_match`;
//! - `fotmob.<div>.pass_completion`, `fotmob.pooled.pass_completion`;
//! - `sb.<competition>_<season>.<stat>`, `sb.<…>.dist.<kind>`: one StatsBomb
//!   competition-season; `sb.men.<stat>`, `sb.men.dist.<kind>`: the men's senior pool.

pub mod build;
pub mod calibration;
pub mod csv;
pub mod data_check;
pub mod duels;
pub mod football_data;
pub mod fotmob;
pub mod impect;
pub mod layers;
pub mod shots_check;
pub mod statsbomb;
pub mod tables;
pub mod tracked;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::cli::{RealTargetsAction, RealTargetsOpts};

/// The version of the reference file's format this program writes and reads.
pub const SCHEMA_VERSION: u32 = 1;
/// The reference file's name in `gate/`.
pub const FILE: &str = "real-targets.json";
/// The tracking file of the tracked-match targets, in `gate/`.
pub const TRACKING_FILE: &str = "tracking/dfl-matches.json";
/// The folder of real-name lists in `gate/`; the packaging test scans the shipped content
/// for every name of every list file there.
pub const NAMES_DIR: &str = "real-names";

/// The reference file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceFile {
    pub schema_version: u32,
    pub builder: Builder,
    /// Each raw file read, relative to the data folder.
    pub built_from: Vec<InputFile>,
    /// One credit per provider a target names.
    pub credits: BTreeMap<String, Credit>,
    pub definitions: Definitions,
    pub targets: BTreeMap<String, Target>,
    /// Targets that could not be built, by key, with the reason.
    pub absent: BTreeMap<String, String>,
}

/// The program that built the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Builder {
    pub version: String,
    /// The commit the builder was built from, or "unknown".
    pub git_head: String,
}

/// One raw file the builder read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputFile {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

/// The credit of one provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credit {
    pub name: String,
    pub source: String,
    pub licence: String,
    /// How the figures may be used here.
    pub use_note: String,
}

/// The definitions the targets were built under.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definitions {
    /// Whether football-data.co.uk's shot columns count blocked shots, by the cross-check.
    pub shots_include_blocked: bool,
    pub shots_check: shots_check::Summary,
    /// The tier of each division.
    pub tiers: BTreeMap<String, u32>,
    /// The direct relegation places of each division, `None` when not established.
    pub relegation_places: BTreeMap<String, Option<u32>>,
    /// The mean ClubElo rating of each division's clubs, when ClubElo places clubs there.
    pub strength: BTreeMap<String, f64>,
    /// The seasons every league target covers.
    pub seasons: Vec<String>,
}

/// Which layer a target belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    League,
    Tier,
    Group,
    /// Several leagues pooled match by match.
    Pooled,
}

/// Whether a target tunes the engine or checks it on data tuning never saw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Label {
    Calibration,
    Validation,
}

/// How the engine counts the same thing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineCount {
    pub text: String,
    /// The registry band that counts it, when one does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub band: Option<String>,
    /// The band's measure, as the registry writes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<String>,
}

/// The fitted formula of a banding-group target: `a + b tier + c strength`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Formula {
    pub intercept: f64,
    pub tier: f64,
    pub strength: f64,
    pub residual_sd: f64,
    /// The divisions the formula was fitted on.
    pub fitted_on: Vec<String>,
}

/// One real target with its full record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// A scalar target's value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// A distribution target's share per bin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shares: Option<Vec<f64>>,
    /// A distribution target's bin count and width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bins: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    /// The standard error: of the mean over whole units of observation for a scalar; the
    /// largest share's for a distribution; the prediction error for a group target.
    pub se: f64,
    /// How many units of observation the target rests on.
    pub n: u64,
    pub seasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub between_season_sd: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub between_league_sd: Option<f64>,
    pub layer: Layer,
    /// The leagues behind a tier, group or pooled target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leagues: Option<Vec<String>>,
    pub provider: String,
    /// The source within the provider, as its credit names it.
    pub source_label: String,
    pub event_definition: String,
    pub denominator: String,
    pub unit_of_observation: String,
    pub label: Label,
    pub engine_count: EngineCount,
    /// The context of the data when it is not a club league (for example "tournament,
    /// national teams").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// A group target outside the range the formula was fitted on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrapolated: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<Formula>,
    /// A tracked-match target's value in each match, in match order, so a band may
    /// resample the matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_match: Option<Vec<f64>>,
}

impl ReferenceFile {
    /// Reads the reference file at `path`, refusing another format version.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read the reference file {}", path.display()))?;
        let file: Self = serde_json::from_str(&text)
            .with_context(|| format!("the reference file {} cannot be read", path.display()))?;
        anyhow::ensure!(
            file.schema_version == SCHEMA_VERSION,
            "the reference file {} has format version {}; this program reads version {}",
            path.display(),
            file.schema_version,
            SCHEMA_VERSION
        );
        Ok(file)
    }

    /// The target named `key`.
    pub fn target(&self, key: &str) -> Option<&Target> {
        self.targets.get(key)
    }
}

/// The key of a league target.
pub fn league_key(div: &str, stat: &str) -> String {
    format!("fd.{div}.{stat}")
}

/// The key of a tier target.
pub fn tier_key(tier: u32, stat: &str) -> String {
    format!("tier.{tier}.{stat}")
}

/// The key of a banding-group target: tier and the lower edge of its 50-point strength
/// bin.
pub fn group_key(tier: u32, elo_bin: i64, stat: &str) -> String {
    format!("group.t{tier}.e{elo_bin}.{stat}")
}

/// The default reference file: `gate/real-targets.json` beside the content folder.
pub fn default_file(content_root: &Path) -> PathBuf {
    gate_dir(content_root).join(FILE)
}

/// The `gate/` folder of the checkout holding the content folder.
pub fn gate_dir(content_root: &Path) -> PathBuf {
    let root = std::path::absolute(content_root).unwrap_or_else(|_| content_root.into());
    root.parent()
        .map_or_else(|| PathBuf::from("gate"), |p| p.join("gate"))
}

/// The default raw-data folder: the main checkout's `.scratch/data`.
pub fn default_data(content_root: &Path) -> Option<PathBuf> {
    crate::report::real::resolve(None, content_root)
        .path
        .and_then(|out| out.parent().map(|p| p.join("data")))
}

/// The commit of the checkout holding `dir`, or "unknown" without git.
fn git_head(dir: &Path) -> String {
    std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

/// Runs `engine-cli real-targets`.
pub fn run(content_dir: Option<&Path>, opts: &RealTargetsOpts) -> anyhow::Result<i32> {
    let dir = engine::ContentDir::resolve(content_dir)?;
    let data = |flag: &Option<PathBuf>| -> anyhow::Result<PathBuf> {
        match flag {
            Some(d) => Ok(d.clone()),
            None => default_data(dir.root()).context(
                "no data folder: pass --data, or run inside a checkout of this repository \
                 whose main checkout holds .scratch/data",
            ),
        }
    };
    match &opts.action {
        RealTargetsAction::CheckData { data: flag } => {
            let folder = data(flag)?;
            let report = data_check::check(&folder)?;
            print!("{}", report.text());
            Ok(i32::from(!report.complete()))
        }
        RealTargetsAction::Build { data: flag, out } => {
            let folder = data(flag)?;
            let out = out.clone().unwrap_or_else(|| default_file(dir.root()));
            let tracking = gate_dir(dir.root()).join(TRACKING_FILE);
            let tracked = if tracking.exists() {
                Some(build::TrackedInput::read(
                    &tracking,
                    &format!("gate/{TRACKING_FILE}"),
                    &dir.path("realism-bands.json"),
                )?)
            } else {
                None
            };
            let built = build::build(&folder, tracked.as_ref(), &git_head(dir.root()))?;
            build::write(&built, &out)?;
            print!("{}", built.summary());
            println!("wrote {}", out.display());
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_follow_the_scheme() {
        assert_eq!(league_key("E0", "goals_per_match"), "fd.E0.goals_per_match");
        assert_eq!(
            tier_key(2, "table.title_margin"),
            "tier.2.table.title_margin"
        );
        assert_eq!(
            group_key(1, 1450, "goals_per_match"),
            "group.t1.e1450.goals_per_match"
        );
    }

    #[test]
    fn the_gate_folder_sits_beside_the_content_folder() {
        let gate = gate_dir(Path::new("/repo/content"));
        assert!(
            gate.ends_with("repo/gate") || gate.ends_with("repo\\gate"),
            "{gate:?}"
        );
    }
}

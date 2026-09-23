//! Shared loading for the subcommands: the content folder, the content files, the commentary
//! lines, the two team files (the shipped defaults when no flag names a file), and the script
//! pack when a flag names one.

use std::path::Path;

use engine::data::{TEAM_A_FILE, TEAM_B_FILE, TeamFile};
use engine::{Commentary, Content, ContentDir, MatchConfig, Simulation};
use script::LoadedPack;

/// Everything loaded from disk before a match is built.
pub struct Loaded {
    pub content: Content,
    pub commentary: Commentary,
    pub teams: [TeamFile; 2],
    /// The script pack, when `--script-pack` names one.
    pub script: Option<LoadedPack>,
}

impl Loaded {
    /// Folds the script pack's hash into the match's content hash, so a snapshot of a
    /// scripted match resumes only with the same pack. Without a pack it changes nothing.
    pub fn fold(&self, config: &mut MatchConfig) {
        if let Some(pack) = &self.script {
            config.fold_pack_hash(pack.sha());
        }
    }

    /// Attaches fresh script hooks to `sim`. Without a pack it changes nothing.
    pub fn attach(&self, sim: &mut Simulation) {
        if let Some(pack) = &self.script {
            sim.set_plugins(pack.plugins());
        }
    }
}

/// Resolves the content folder and loads the content, two teams, and the script pack. A
/// refused pack is an error that names the file, the field, and the reason.
pub fn load(
    content_dir: Option<&Path>,
    team_a: Option<&Path>,
    team_b: Option<&Path>,
    script_pack: Option<&Path>,
) -> anyhow::Result<Loaded> {
    let dir = ContentDir::resolve(content_dir)?;
    let content = Content::load(&dir)?;
    let commentary = Commentary::load(&dir)?;
    let a = team_a.map_or_else(|| dir.path(TEAM_A_FILE), Path::to_path_buf);
    let b = team_b.map_or_else(|| dir.path(TEAM_B_FILE), Path::to_path_buf);
    let team_a = content.load_team(&dir, &a)?.value;
    let team_b = content.load_team(&dir, &b)?.value;
    let script = match script_pack {
        Some(path) => Some(LoadedPack::load(path).inspect_err(|err| {
            tracing::error!(signal = "script.refused", reason = %err);
        })?),
        None => None,
    };
    Ok(Loaded {
        content,
        commentary,
        teams: [team_a, team_b],
        script,
    })
}

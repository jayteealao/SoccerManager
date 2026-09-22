//! Shared loading for the subcommands: the content folder, the content files, the commentary
//! lines, and the two team files (the shipped defaults when no flag names a file).

use std::path::Path;

use engine::data::{TEAM_A_FILE, TEAM_B_FILE, TeamFile};
use engine::{Commentary, Content, ContentDir};

/// Everything loaded from disk before a match is built.
pub struct Loaded {
    pub content: Content,
    pub commentary: Commentary,
    pub teams: [TeamFile; 2],
}

/// Resolves the content folder and loads the content and two teams.
pub fn load(
    content_dir: Option<&Path>,
    team_a: Option<&Path>,
    team_b: Option<&Path>,
) -> anyhow::Result<Loaded> {
    let dir = ContentDir::resolve(content_dir)?;
    let content = Content::load(&dir)?;
    let commentary = Commentary::load(&dir)?;
    let a = team_a.map_or_else(|| dir.path(TEAM_A_FILE), Path::to_path_buf);
    let b = team_b.map_or_else(|| dir.path(TEAM_B_FILE), Path::to_path_buf);
    let team_a = content.load_team(&dir, &a)?.value;
    let team_b = content.load_team(&dir, &b)?.value;
    Ok(Loaded {
        content,
        commentary,
        teams: [team_a, team_b],
    })
}

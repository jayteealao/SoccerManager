//! `engine-cli generate`: write fictional clubs as team files, one file per club.

use std::path::Path;

use anyhow::Context;
use engine::data::generate_league;
use engine::{Content, ContentDir};

use crate::cli::GenerateOpts;

pub fn run(content_dir: Option<&Path>, opts: &GenerateOpts) -> anyhow::Result<i32> {
    if opts.clubs == 0 {
        anyhow::bail!("--clubs must be at least 1");
    }
    let dir = ContentDir::resolve(content_dir)?;
    let content = Content::load(&dir)?;
    let teams = generate_league(opts.seed, opts.clubs, &content);
    std::fs::create_dir_all(&opts.out)
        .with_context(|| format!("cannot create {}", opts.out.display()))?;
    for team in &teams {
        let path = opts.out.join(format!("{}.json", team.club.id));
        if path.exists() && !opts.force {
            anyhow::bail!(
                "{} already exists; pass --force to overwrite",
                path.display()
            );
        }
        let json = serde_json::to_string_pretty(team)?;
        std::fs::write(&path, format!("{json}\n"))
            .with_context(|| format!("cannot write {}", path.display()))?;
    }
    println!("wrote {} team files to {}", teams.len(), opts.out.display());
    Ok(0)
}

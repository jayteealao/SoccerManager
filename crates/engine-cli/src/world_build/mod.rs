//! `engine-cli world-database build`: the world database from local snapshots only.
//!
//! It reads the openfootball, ClubElo, Wikidata and competitions snapshots of the data folder,
//! the hand-fill file and the real-name lists of `gate/real-names/`, and writes
//! `content/world.json` (facsimile names only, it ships) and, in `gate/` (never shipped), the
//! real-name list `real-names/world.json` for the packaging check, `world/name-map.json`,
//! `world/gaps.csv` and `world/coverage.json`. Two builds of the same inputs write the same
//! bytes. No network request is made.

pub mod assemble;
pub mod facsimile;
pub mod output;
pub mod sources;
pub mod strength;

use std::path::Path;

use anyhow::Context;
use engine::data::world::WORLD_FILE;

use crate::cli::{WorldDatabaseAction, WorldDatabaseOpts};
use sources::Snapshot;

/// Runs `engine-cli world-database`.
pub fn run(content_dir: Option<&Path>, opts: &WorldDatabaseOpts) -> anyhow::Result<i32> {
    let dir = engine::ContentDir::resolve(content_dir)?;
    match &opts.action {
        WorldDatabaseAction::Build {
            data,
            out,
            gate,
            hand_fills,
        } => {
            let data = match data {
                Some(d) => d.clone(),
                None => crate::real_targets::default_data(dir.root()).context(
                    "no data folder: pass --data, or run inside a checkout of this repository \
                     whose main checkout holds .scratch/data",
                )?,
            };
            let gate = gate
                .clone()
                .unwrap_or_else(|| crate::real_targets::gate_dir(dir.root()));
            let out = out.clone().unwrap_or_else(|| dir.root().join(WORLD_FILE));
            let fills_path = hand_fills
                .clone()
                .unwrap_or_else(|| gate.join("world").join("hand-fills.json"));
            let data = Snapshot::new(&data, "data");
            let gate = Snapshot::new(&gate, "gate");
            let fills_dir = fills_path.parent().unwrap_or(Path::new("."));
            let fills_name = fills_path
                .file_name()
                .context("--hand-fills names no file")?
                .to_string_lossy()
                .to_string();
            let fills_snap = Snapshot::new(fills_dir, "hand-fills");
            let fills = if fills_path.is_file() {
                serde_json::from_str(&fills_snap.text(&fills_name)?)
                    .with_context(|| format!("{} is not a hand-fill file", fills_path.display()))?
            } else if hand_fills.is_some() {
                anyhow::bail!("cannot read {}", fills_path.display());
            } else {
                assemble::HandFills::default()
            };
            let built = assemble::build(&data, &gate, &fills)?;
            let mut opened = data.opened();
            opened.extend(gate.opened());
            opened.extend(fills_snap.opened());
            let written = output::write(built, &out, &gate, opened)?;
            print!("{}", written.summary);
            for p in &written.paths {
                println!("wrote {}", p.display());
            }
            Ok(0)
        }
    }
}

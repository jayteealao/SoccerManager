//! Writing a build: the world database in the content folder (facsimile names only) and, in
//! `gate/`, the real-name list for the packaging check, the name map, the gap list and the
//! coverage record. Before anything is written, the world file is loaded back and scanned
//! with the packaging check's rule, so the builder never writes a file that check refuses.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use engine::data::world::{StrengthSource, load_world};
use serde_json::json;

use super::assemble::{Built, Gap, strings};
use super::sources::Snapshot;

/// The real-name list this builder writes in `gate/real-names/`.
pub const LIST_FILE: &str = "real-names/world.json";

/// What a build wrote, for the printed summary.
pub struct Written {
    pub paths: Vec<PathBuf>,
    pub summary: String,
}

fn pretty(v: &impl serde::Serialize) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(v)? + "\n")
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The gap list as CSV, one row per gap, in the sorted order of `gaps`.
pub fn gaps_csv(gaps: &[Gap]) -> String {
    let mut out = String::from("kind,entity,id,real_name,country,missing,reason,sources_tried\n");
    for g in gaps {
        let row = [
            g.kind,
            g.entity,
            &g.id,
            &g.real_name,
            &g.country,
            g.missing,
            &g.reason,
            g.sources_tried,
        ];
        out.push_str(&row.map(csv_field).join(","));
        out.push('\n');
    }
    out
}

/// Checks and writes `built`: the world file to `out`, the gate files under `gate`.
/// `opened` lists every input read, with its SHA-256.
pub fn write(
    built: Built,
    out: &Path,
    gate: &Snapshot,
    mut opened: BTreeMap<String, String>,
) -> anyhow::Result<Written> {
    let world_text = pretty(&built.world)?;
    load_world(world_text.as_bytes(), "world.json").context("the built world file is refused")?;
    let mut all = Vec::new();
    strings(&serde_json::to_value(&built.world)?, &mut all);
    let structural: BTreeSet<String> = all.iter().map(|s| s.trim().to_lowercase()).collect();

    // The list for the packaging check: a real name equal to a string of the world file (a
    // code, a region, a stage kind) is left out and recorded.
    let mut list: BTreeSet<String> = BTreeSet::new();
    let mut exceptions: BTreeSet<String> = BTreeSet::new();
    for n in &built.scan_names {
        let n = n.trim().to_string();
        if structural.contains(&n.to_lowercase()) {
            exceptions.insert(n);
        } else {
            list.insert(n);
        }
    }

    // The packaging rule: no string of the world file equals a name of any gate list.
    let mut lists: Vec<(String, BTreeSet<String>)> = vec![(
        LIST_FILE.to_string(),
        list.iter().map(|n| n.to_lowercase()).collect(),
    )];
    for f in gate.files("real-names")? {
        if f.ends_with(".json") && f != LIST_FILE {
            let mut names = Vec::new();
            strings(&gate.json(&f)?, &mut names);
            opened.extend(gate.opened());
            lists.push((f, names.iter().map(|s| s.trim().to_lowercase()).collect()));
        }
    }
    let mut hits = Vec::new();
    for s in &structural {
        for (f, names) in &lists {
            if names.contains(s) {
                hits.push(format!("{s:?} equals a name of {f}"));
            }
        }
    }
    if !hits.is_empty() {
        bail!(
            "the world file would hold a real name; nothing is written:\n  {}",
            hits.join("\n  ")
        );
    }

    let mut coverage = built.coverage;
    coverage["inputs"] = json!(
        opened
            .iter()
            .map(|(p, h)| json!({"path": p, "sha256": h}))
            .collect::<Vec<_>>()
    );
    coverage["scan_exceptions"] = json!(exceptions);

    let root = gate.root();
    let files: Vec<(PathBuf, String)> = vec![
        (out.to_path_buf(), world_text),
        (root.join(LIST_FILE), pretty(&list)?),
        (root.join("world/name-map.json"), pretty(&built.name_map)?),
        (root.join("world/gaps.csv"), gaps_csv(&built.gaps)),
        (root.join("world/coverage.json"), pretty(&coverage)?),
    ];
    for (path, text) in &files {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("cannot create {}", dir.display()))?;
        }
        std::fs::write(path, text).with_context(|| format!("cannot write {}", path.display()))?;
    }

    let w = &built.world;
    let mut by_source: BTreeMap<&str, usize> = BTreeMap::new();
    for c in w.clubs.values() {
        let key = match c.strength_source {
            StrengthSource::Clubelo => "clubelo",
            StrengthSource::LeagueMedian => "league-median",
            StrengthSource::TierStep => "tier-step",
            StrengthSource::Floor => "floor",
        };
        *by_source.entry(key).or_default() += 1;
    }
    let mut by_kind: BTreeMap<&str, usize> = BTreeMap::new();
    for g in &built.gaps {
        *by_kind.entry(g.kind).or_default() += 1;
    }
    let line = |m: &BTreeMap<&str, usize>| {
        m.iter()
            .map(|(k, n)| format!("{k} {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let summary = format!(
        "world database: {} countries, {} leagues, {} clubs ({}), {} competitions\n\
         gap list: {} rows ({})\n\
         real names listed for the packaging check: {}; scan exceptions: {}\n\
         world file: {} bytes\n",
        w.countries.len(),
        w.leagues.len(),
        w.clubs.len(),
        line(&by_source),
        w.competitions.len(),
        built.gaps.len(),
        line(&by_kind),
        list.len(),
        coverage["scan_exceptions"].as_array().map_or(0, Vec::len),
        files[0].1.len(),
    );
    Ok(Written {
        paths: files.into_iter().map(|(p, _)| p).collect(),
        summary,
    })
}

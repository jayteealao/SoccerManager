//! The local side folder: files the builder derives from data whose terms keep it off the
//! committed tree (IMPECT's targets), and per-match rows too many for `gate/` (StatsBomb's
//! match histograms). Both stay on this PC.
//!
//! The default folder is `.scratch/real-targets` of the main checkout, beside the raw data
//! in `.scratch/data`, which git ignores; a judging run finds it there from any worktree.
//! A write refuses every path inside a checkout that is not under its `.scratch/`, so a
//! side file can never land in a tracked folder.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use super::{InputFile, Target};

/// The folder's name beside `.scratch/data`.
pub const DIR: &str = "real-targets";
/// The IMPECT targets.
pub const IMPECT_FILE: &str = "impect-targets.json";
/// The StatsBomb per-match histograms of the men's senior pool.
pub const HISTOGRAM_FILE: &str = "statsbomb-match-histograms.csv";

/// The IMPECT side file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImpectFile {
    /// Always `impect`.
    pub side: String,
    pub licence: String,
    pub credit: String,
    pub built_from: Vec<InputFile>,
    pub targets: BTreeMap<String, Target>,
}

/// The main checkout holding `content_root`, by the same search as the real-data folder.
fn main_checkout(content_root: &Path) -> Option<PathBuf> {
    crate::report::real::resolve(None, content_root)
        .path
        .and_then(|out| out.parent()?.parent().map(Path::to_path_buf))
}

/// The default side folder: `.scratch/real-targets` of the main checkout holding
/// `content_root`; `None` outside a checkout.
pub fn default_dir(content_root: &Path) -> Option<PathBuf> {
    main_checkout(content_root).map(|m| m.join(".scratch").join(DIR))
}

/// The checkouts a side file must stay out of (outside their `.scratch/`): the one holding
/// the content folder and the main checkout.
pub fn checkouts(content_root: &Path) -> Vec<PathBuf> {
    let root = std::path::absolute(content_root).unwrap_or_else(|_| content_root.into());
    let mut out: Vec<PathBuf> = root.parent().map(Path::to_path_buf).into_iter().collect();
    out.extend(main_checkout(content_root));
    out
}

/// `path` resolved through its longest existing ancestor, so a folder not yet made compares
/// with canonical checkout paths; `..` parts below that ancestor are taken by name.
fn resolved(path: &Path) -> PathBuf {
    let abs = std::path::absolute(path).unwrap_or_else(|_| path.into());
    let mut rest = Vec::new();
    let mut at = abs.as_path();
    loop {
        if let Ok(c) = at.canonicalize() {
            let mut out = c;
            for part in rest.iter().rev() {
                match part {
                    Component::ParentDir => {
                        out.pop();
                    }
                    Component::CurDir => {}
                    other => out.push(other),
                }
            }
            return out;
        }
        match (at.parent(), at.components().next_back()) {
            (Some(parent), Some(last)) => {
                rest.push(last);
                at = parent;
            }
            _ => return abs,
        }
    }
}

/// Refuses `dir` when it lies inside one of `checkouts` and not under that checkout's
/// `.scratch/`.
pub fn guard(dir: &Path, checkouts: &[PathBuf]) -> anyhow::Result<()> {
    let at = resolved(dir);
    for root in checkouts {
        let root = resolved(root);
        if at.starts_with(&root) && !at.starts_with(root.join(".scratch")) {
            anyhow::bail!(
                "the side folder {} is inside the checkout {} but not under its .scratch/: \
                 side files never go where git tracks them",
                dir.display(),
                root.display()
            );
        }
    }
    Ok(())
}

/// Writes `text` to `dir/name` after the guard; returns the path written.
pub fn write(dir: &Path, name: &str, text: &str, checkouts: &[PathBuf]) -> anyhow::Result<PathBuf> {
    guard(dir, checkouts)?;
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let path = dir.join(name);
    std::fs::write(&path, text).with_context(|| format!("cannot write {}", path.display()))?;
    Ok(path)
}

/// The text of `dir/name`; `None` when it is not there.
pub fn read(dir: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(dir.join(name)).ok()
}

/// The IMPECT side file of `dir`; `Ok(None)` when it is not there.
pub fn read_impect(dir: &Path) -> anyhow::Result<Option<ImpectFile>> {
    let Some(text) = read(dir, IMPECT_FILE) else {
        return Ok(None);
    };
    let file: ImpectFile = serde_json::from_str(&text)
        .with_context(|| format!("{} cannot be read", dir.join(IMPECT_FILE).display()))?;
    anyhow::ensure!(
        file.side == "impect",
        "{} is not an IMPECT side file",
        dir.join(IMPECT_FILE).display()
    );
    Ok(Some(file))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A checkout in a temporary folder: `.git`, `content/`, `gate/`.
    fn checkout() -> TempDir {
        let tmp = TempDir::new().unwrap();
        for d in [".git", "content", "gate"] {
            std::fs::create_dir_all(tmp.path().join(d)).unwrap();
        }
        tmp
    }

    #[test]
    fn the_default_folder_is_the_checkouts_scratch_real_targets() {
        let tmp = checkout();
        let dir = default_dir(&tmp.path().join("content")).unwrap();
        assert!(dir.ends_with(Path::new(".scratch").join(DIR)), "{dir:?}");
        assert!(resolved(&dir).starts_with(resolved(tmp.path())));
    }

    #[test]
    fn a_tracked_folder_is_refused_and_scratch_or_outside_is_written() {
        let tmp = checkout();
        let roots = checkouts(&tmp.path().join("content"));
        let gate = tmp.path().join("gate").join("side");
        let e = format!("{:#}", write(&gate, IMPECT_FILE, "{}", &roots).unwrap_err());
        assert!(e.contains("side") && e.contains(".scratch"), "{e}");
        assert!(!gate.exists());
        // A path that climbs out of .scratch back into the checkout is refused too.
        let sneaky = tmp.path().join(".scratch").join("..").join("content");
        assert!(write(&sneaky, IMPECT_FILE, "{}", &roots).is_err());

        let scratch = default_dir(&tmp.path().join("content")).unwrap();
        let p = write(&scratch, IMPECT_FILE, "{}", &roots).unwrap();
        assert_eq!(read(&scratch, IMPECT_FILE).as_deref(), Some("{}"));
        assert!(p.is_file());

        let outside = TempDir::new().unwrap();
        write(outside.path(), HISTOGRAM_FILE, "label\n", &roots).unwrap();
        assert!(read(outside.path(), HISTOGRAM_FILE).is_some());
    }

    #[test]
    fn an_absent_side_file_reads_none() {
        let tmp = TempDir::new().unwrap();
        assert!(read(tmp.path(), IMPECT_FILE).is_none());
        assert!(read_impect(tmp.path()).unwrap().is_none());
    }
}

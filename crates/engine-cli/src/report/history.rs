//! The band history and its drift check.
//!
//! Each judged run adds one JSON line per band row to `gate/bands/history.jsonl` of the
//! checkout that holds the content folder: the run, its date, build and content, the band,
//! its definition version, value, error, word and matches. Judging a run folder again
//! replaces that run's lines. `SM_BAND_HISTORY` names another file, and `off` writes none.
//!
//! The drift section reads the whole history: each band's points of one definition version
//! go through a two-sided cumulative sum against the mean of its first points (see
//! `pass_rule::shift`). A flag names the band and the run where the rise began; it never
//! changes a word or blocks a change.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use pass_rule::shift::{self, Point};
use serde::{Deserialize, Serialize};

use super::BandCheck;
use super::bands::Registry;

/// The variable that names the history file; `off` keeps no history.
pub const HISTORY_ENV: &str = "SM_BAND_HISTORY";

/// The history file, relative to the checkout.
pub const HISTORY_FILE: &str = "gate/bands/history.jsonl";

/// One band row of one run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryPoint {
    pub run: String,
    pub date: String,
    pub build: String,
    pub content: String,
    pub band: String,
    pub suite: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pairing: Option<String>,
    pub definition_version: u32,
    pub value: f64,
    pub se: f64,
    pub word: String,
    pub matches: u64,
}

/// The run a history's points come from.
#[derive(Debug, Clone)]
pub struct RunStamp<'a> {
    pub run: &'a str,
    pub date: String,
    pub build: &'a str,
    pub content: &'a str,
}

/// A band whose history drifts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Drift {
    pub band: String,
    pub suite: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairing: Option<String>,
    pub definition_version: u32,
    /// `up` or `down`.
    pub direction: &'static str,
    /// The run where the sum began to rise.
    pub since_run: String,
    /// The run where the sum passed the threshold.
    pub flagged_run: String,
    /// The band's points of this definition version.
    pub points: usize,
}

/// `calib.drift`: the history file, the points this run added, and the drifting bands.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DriftInfo {
    pub history: String,
    pub added: usize,
    pub drifts: Vec<Drift>,
}

/// The history file of a run whose content folder is `content_root`: the file
/// `SM_BAND_HISTORY` names, none when it is `off`, else `gate/bands/history.jsonl` of the
/// first ancestor of the content folder that holds `.git` (a folder or a worktree's file).
/// A content folder outside any checkout keeps no history.
pub fn path(content_root: &Path) -> Option<PathBuf> {
    path_from(std::env::var(HISTORY_ENV).ok().as_deref(), content_root)
}

/// The history file under a checkout, joined part by part so the path reads in one style.
fn in_checkout(checkout: &Path) -> PathBuf {
    HISTORY_FILE
        .split('/')
        .fold(checkout.to_path_buf(), |at, part| at.join(part))
}

fn path_from(setting: Option<&str>, content_root: &Path) -> Option<PathBuf> {
    match setting.map(str::trim).filter(|s| !s.is_empty()) {
        Some("off") => None,
        Some(file) => Some(PathBuf::from(file)),
        None => {
            let start =
                std::path::absolute(content_root).unwrap_or_else(|_| content_root.to_path_buf());
            start
                .ancestors()
                .find(|at| at.join(".git").exists())
                .map(in_checkout)
        }
    }
}

/// The points of `checks`: one per judged row with a value, named by band, suite and
/// pairing. A row with no rule word reads not sure; its matches are the run's.
pub fn points(
    stamp: &RunStamp<'_>,
    registry: &Registry,
    checks: &[BandCheck],
    matches: u32,
) -> Vec<HistoryPoint> {
    checks
        .iter()
        .filter(|c| c.band != "wall_ms" && c.value.is_finite() && c.why_none.is_none())
        .filter_map(|c| {
            let def = registry.band(&c.band)?;
            Some(HistoryPoint {
                run: stamp.run.to_string(),
                date: stamp.date.clone(),
                build: stamp.build.to_string(),
                content: stamp.content.to_string(),
                band: c.band.clone(),
                suite: c.suite.clone(),
                pairing: c.pairing.clone(),
                definition_version: def.rule.as_ref().map_or(1, |r| r.definition_version),
                value: c.value,
                se: c.se,
                word: c
                    .rule
                    .as_ref()
                    .map_or("not sure", |r| r.word.as_str())
                    .to_string(),
                matches: c
                    .rule
                    .as_ref()
                    .map_or(u64::from(matches), |r| r.exposure.round().max(0.0) as u64),
            })
        })
        .collect()
}

/// Reads the history; a missing file is an empty history. A line that does not read is
/// refused with its number, so a damaged history is never judged in part.
pub fn read(path: &Path) -> anyhow::Result<Vec<HistoryPoint>> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("cannot read {}", path.display())),
    };
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(n, l)| {
            serde_json::from_str(l)
                .with_context(|| format!("{} line {} does not read", path.display(), n + 1))
        })
        .collect()
}

/// Adds `points` to the history at `path`, first removing the lines of their run, so a run
/// judged again keeps one point per band. Returns the whole history after the write.
pub fn record(path: &Path, points: &[HistoryPoint]) -> anyhow::Result<Vec<HistoryPoint>> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    // Two runs that share one history take turns, so neither loses the other's points.
    let _lock = Lock::take(&path.with_extension("jsonl.lock"))?;
    let mut all = read(path)?;
    if let Some(run) = points.first().map(|p| p.run.clone()) {
        all.retain(|p| p.run != run);
    }
    all.extend(points.iter().cloned());
    let mut text = Vec::new();
    for p in &all {
        serde_json::to_writer(&mut text, p)?;
        text.push(b'\n');
    }
    let partial = path.with_extension(format!("jsonl.{}.partial", std::process::id()));
    {
        let mut file = std::fs::File::create(&partial)
            .with_context(|| format!("cannot write {}", partial.display()))?;
        file.write_all(&text)?;
        file.sync_all()?;
    }
    std::fs::rename(&partial, path)
        .with_context(|| format!("cannot replace {}", path.display()))?;
    Ok(all)
}

/// A lock file held while one run rewrites the history; removed when dropped. A lock older
/// than [`Lock::STALE`] was left by a run that stopped, and is taken over.
struct Lock(std::path::PathBuf);

impl Lock {
    const STALE: std::time::Duration = std::time::Duration::from_secs(120);
    const WAIT: std::time::Duration = std::time::Duration::from_secs(60);

    fn take(path: &Path) -> anyhow::Result<Lock> {
        let start = std::time::Instant::now();
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
            {
                Ok(_) => return Ok(Lock(path.to_path_buf())),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let stale = std::fs::metadata(path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.elapsed().ok())
                        .is_some_and(|age| age > Self::STALE);
                    if stale {
                        let _ = std::fs::remove_file(path);
                        continue;
                    }
                    anyhow::ensure!(
                        start.elapsed() < Self::WAIT,
                        "another run holds {}; remove it if no run is writing the history",
                        path.display()
                    );
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => {
                    return Err(e).with_context(|| format!("cannot create {}", path.display()));
                }
            }
        }
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// The drifting bands of a history, in band order: each band, suite, pairing and definition
/// version is one series, in file order.
pub fn drifts(history: &[HistoryPoint]) -> Vec<Drift> {
    type Key = (String, String, Option<String>, u32);
    let mut series: BTreeMap<Key, Vec<&HistoryPoint>> = BTreeMap::new();
    for p in history {
        series
            .entry((
                p.band.clone(),
                p.suite.clone(),
                p.pairing.clone(),
                p.definition_version,
            ))
            .or_default()
            .push(p);
    }
    series
        .into_iter()
        .filter_map(|((band, suite, pairing, definition_version), list)| {
            let usable: Vec<&HistoryPoint> = list
                .into_iter()
                .filter(|p| p.value.is_finite() && p.se.is_finite() && p.se > 0.0)
                .collect();
            let points: Vec<Point> = usable
                .iter()
                .map(|p| Point {
                    value: p.value,
                    se: p.se,
                })
                .collect();
            let flag = shift::cusum(&points, shift::K, shift::H)?;
            Some(Drift {
                band,
                suite,
                pairing,
                definition_version,
                direction: if flag.up { "up" } else { "down" },
                since_run: usable[flag.start].run.clone(),
                flagged_run: usable[flag.at].run.clone(),
                points: points.len(),
            })
        })
        .collect()
}

/// The console lines of the drift section.
pub fn render(info: &DriftInfo) -> String {
    let mut out = format!(
        "band history: {} points added to {}; {} drifting\n",
        info.added,
        info.history,
        info.drifts.len()
    );
    for d in &info.drifts {
        let pairing = d
            .pairing
            .as_deref()
            .map(|p| format!(" ({p})"))
            .unwrap_or_default();
        out.push_str(&format!(
            "  drift {}: {}{} in {}, rising since run {}, flagged at run {}\n",
            d.direction, d.band, pairing, d.suite, d.since_run, d.flagged_run
        ));
    }
    out
}

/// The UTC date of `millis` after 1970, as `YYYY-MM-DD`.
pub fn date_of(millis: u64) -> String {
    super::real::civil_date(i64::try_from(millis / 86_400_000).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn point(run: &str, band: &str, value: f64) -> HistoryPoint {
        HistoryPoint {
            run: run.to_string(),
            date: "2026-10-22".to_string(),
            build: "abc".to_string(),
            content: "def".to_string(),
            band: band.to_string(),
            suite: "equal".to_string(),
            pairing: None,
            definition_version: 1,
            value,
            se: 0.1,
            word: "pass".to_string(),
            matches: 1000,
        }
    }

    #[test]
    fn the_setting_names_the_file_or_turns_it_off() {
        let dir = TempDir::new().unwrap();
        assert_eq!(path_from(Some("off"), dir.path()), None);
        assert_eq!(
            path_from(Some("x/h.jsonl"), dir.path()),
            Some(PathBuf::from("x/h.jsonl"))
        );
        // A content folder outside any checkout keeps no history.
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        assert_eq!(path_from(None, &content), None);
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        assert_eq!(
            path_from(None, &content),
            Some(in_checkout(&std::path::absolute(dir.path()).unwrap()))
        );
    }

    #[test]
    fn a_run_judged_again_replaces_its_own_points() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("bands").join("history.jsonl");
        record(&file, &[point("r1", "a", 1.0), point("r1", "b", 2.0)]).unwrap();
        record(&file, &[point("r2", "a", 1.1), point("r2", "b", 2.1)]).unwrap();
        let all = record(&file, &[point("r1", "a", 1.2), point("r1", "b", 2.2)]).unwrap();
        assert_eq!(all.len(), 4);
        assert_eq!(read(&file).unwrap(), all);
        let r1: Vec<f64> = all
            .iter()
            .filter(|p| p.run == "r1")
            .map(|p| p.value)
            .collect();
        assert_eq!(r1, vec![1.2, 2.2]);
    }

    #[test]
    fn two_runs_writing_at_once_keep_every_point() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("history.jsonl");
        std::thread::scope(|s| {
            for run in ["r1", "r2", "r3", "r4"] {
                let file = &file;
                s.spawn(move || {
                    for k in 0..5 {
                        record(file, &[point(run, &format!("b{k}"), 1.0)]).unwrap();
                    }
                });
            }
        });
        let all = read(&file).unwrap();
        assert_eq!(all.len(), 4, "each run keeps its last write: {all:?}");
        assert!(!file.with_extension("jsonl.lock").exists());
    }

    #[test]
    fn a_damaged_line_is_refused_with_its_number() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("history.jsonl");
        std::fs::write(&file, "{\"run\": 1}\n").unwrap();
        let err = format!("{:#}", read(&file).unwrap_err());
        assert!(err.contains("line 1"), "{err}");
    }

    #[test]
    fn a_step_in_one_band_is_flagged_and_a_steady_band_is_not() {
        let mut history = Vec::new();
        for i in 0..30 {
            let run = format!("r{i:02}");
            history.push(point(&run, "steady", 1.0 + 0.05 * f64::from(i % 3) - 0.05));
            let stepped = if i >= 15 { 1.3 } else { 1.0 };
            history.push(point(&run, "stepped", stepped));
        }
        let found = drifts(&history);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].band, "stepped");
        assert_eq!(found[0].direction, "up");
        assert_eq!(found[0].since_run, "r15");
        // A new definition version starts a new series.
        for p in history.iter_mut().filter(|p| p.run.as_str() >= "r15") {
            p.definition_version = 2;
        }
        assert!(drifts(&history).is_empty());
    }
}

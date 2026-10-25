//! The calibration report: the engine against the real targets, band by band, from a
//! finished run folder's stored rows. It only reports: it writes `calibration.json` in the
//! run folder and never `report.json`, so the run's verdict cannot change.
//!
//! A band appears when a target names it in `engine_count.band`. The engine side is the
//! band's own measure over the equal suite's matches (clubs of the same strength, the suite
//! the real leagues compare with): a mean or share with its error, or a whole distribution
//! with its distance from each real one (total variation for scoreline cells, Wasserstein-1
//! otherwise). Every other band is listed as having no target.
//!
//! Each line names its target's source label. The targets of the local side file (a source
//! whose figures may not be shared) join the bands they name when the side folder holds the
//! file, and the report says when it does not.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Context;
use engine::ContentDir;
use engine::observe::MatchStats;
use serde::Serialize;

use super::{Layer, ReferenceFile, Target, default_file, side};
use crate::calibrate::fixtures::FixtureKey;
use crate::calibrate::rows;
use crate::calibrate::run_folder::{self, UnitLine};
use crate::report::Suite;
use crate::report::bands::Registry;
use crate::report::distribution::Metric;
use crate::report::measures::{self, Obs, Scope};

/// The file the report writes in the run folder.
pub const FILE: &str = "calibration.json";

/// One target against the engine.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Line {
    pub target: String,
    pub layer: Layer,
    pub source_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real_se: Option<f64>,
    /// A scalar: the engine's value less the real one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difference: Option<f64>,
    /// A distribution: the distance of the engine's distribution from the real one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real_shares: Option<Vec<f64>>,
}

/// One band: the engine's side and every target that names it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BandLine {
    pub band: String,
    pub matches: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_se: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_shares: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<&'static str>,
    pub targets: Vec<Line>,
}

/// The whole report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub run: String,
    pub targets_file: String,
    pub suite: &'static str,
    pub matches: usize,
    pub bands: Vec<BandLine>,
    /// Registry bands no target names.
    pub without_target: Vec<String>,
    /// Targets naming a band the registry does not hold.
    pub unknown_bands: Vec<String>,
    /// The local side file: where it was looked for, and how many targets it gave.
    pub side: SideLine,
}

/// Where the local side file was looked for and what it gave; `targets` is `None` when it
/// is not there.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SideLine {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<usize>,
}

/// A StatsBomb competition-season target (`sb.<competition>_<season>.…`), which the table
/// counts and leaves to `calibration.json`, as it does banding-group targets.
fn per_season(key: &str) -> bool {
    key.strip_prefix("sb.")
        .and_then(|rest| rest.split('.').next())
        .is_some_and(|label| {
            label.contains('_') && label.chars().all(|c| c.is_ascii_digit() || c == '_')
        })
}

/// The suite of every finished fixture, from the ledger lines of `dir`.
fn suites(dir: &Path) -> anyhow::Result<BTreeMap<FixtureKey, String>> {
    let mut out = BTreeMap::new();
    let ledger = dir.join("ledger");
    let Ok(entries) = std::fs::read_dir(&ledger) else {
        return Ok(out);
    };
    for path in entries.filter_map(Result::ok).map(|e| e.path()) {
        if path.extension().is_none_or(|e| e != "jsonl") {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let complete = text.rfind('\n').map_or("", |end| &text[..end]);
        for line in complete.lines().filter(|l| !l.trim().is_empty()) {
            if let Ok(unit) = serde_json::from_str::<UnitLine>(line) {
                for done in unit.fixtures {
                    out.insert(done.key, unit.suite.clone());
                }
            }
        }
    }
    Ok(out)
}

/// The band records of the equal suite's finished matches in `dir`.
fn equal_records(dir: &Path) -> anyhow::Result<Vec<MatchStats>> {
    let done = run_folder::finished(dir, None)?;
    let read = rows::read_rows(dir, &done)?;
    let suites = suites(dir)?;
    Ok(read
        .rows
        .iter()
        .filter(|(k, _)| suites.get(*k).is_some_and(|s| s == Suite::Equal.code()))
        .map(|(_, row)| row.band_record())
        .collect())
}

/// The distance between two share vectors over the same bins of `width`: total variation,
/// half the summed absolute difference; or Wasserstein-1, `width` times the summed absolute
/// difference of the cumulative shares. The judging code's own forms are test-only, and
/// this report must not edit a judging file.
fn distance(metric: Metric, a: &[f64], b: &[f64], width: f64) -> f64 {
    match metric {
        Metric::Tv => 0.5 * a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f64>(),
        Metric::W1 => {
            let (mut ca, mut cb, mut sum) = (0.0, 0.0, 0.0);
            for (x, y) in a.iter().zip(b) {
                ca += x;
                cb += y;
                sum += (ca - cb).abs();
            }
            width * sum
        }
    }
}

fn shares_of(hists: &[Option<Vec<(u32, f64)>>], bins: u32) -> Option<Vec<f64>> {
    let mut counts = vec![0.0; bins as usize];
    for h in hists.iter().flatten() {
        for (b, n) in h {
            if let Some(c) = counts.get_mut(*b as usize) {
                *c += n;
            }
        }
    }
    let total: f64 = counts.iter().sum();
    (total > 0.0).then(|| counts.iter().map(|c| c / total).collect())
}

/// Builds the report of the run folder `run` against `file` and, when given, the side
/// file's targets (`side_targets`) found at `side_path`.
pub fn report(
    run: &Path,
    registry: &Registry,
    file: &ReferenceFile,
    targets_path: &Path,
    side_targets: Option<&BTreeMap<String, Target>>,
    side_path: &Path,
) -> anyhow::Result<Report> {
    let records = equal_records(run)?;
    anyhow::ensure!(
        !records.is_empty(),
        "{} holds no finished match of the equal suite; run calibrate with --suite equal \
         (or all) into it first",
        run.display()
    );
    let obs: Vec<Obs<'_>> = records
        .iter()
        .map(|stats| Obs {
            stats,
            boosted: None,
            pairing: None,
            arm: None,
            extras: None,
            league: None,
        })
        .collect();
    let mut linked: BTreeMap<&str, Vec<(&String, &Target)>> = BTreeMap::new();
    for (key, t) in file
        .targets
        .iter()
        .chain(side_targets.into_iter().flatten())
    {
        if let Some(band) = &t.engine_count.band {
            linked.entry(band).or_default().push((key, t));
        }
    }
    let mut bands = Vec::new();
    let mut unknown = Vec::new();
    for (band, list) in &linked {
        let Some(def) = registry.band(band) else {
            unknown.extend(list.iter().map(|(k, _)| (*k).clone()));
            continue;
        };
        let mut line = BandLine {
            band: (*band).to_string(),
            matches: obs.len(),
            engine: None,
            engine_se: None,
            engine_shares: None,
            metric: None,
            targets: Vec::new(),
        };
        if let Some((width, bins)) = measures::hist_bins(&def.measure) {
            let metric = measures::metric(&def.measure);
            line.metric = Some(match metric {
                Metric::Tv => "total variation",
                Metric::W1 => "wasserstein-1",
            });
            let engine = shares_of(&measures::histograms(def, &obs, Scope::All), bins);
            for (key, t) in list {
                let distance = match (&engine, &t.shares) {
                    (Some(e), Some(r)) if e.len() == r.len() => Some(distance(metric, e, r, width)),
                    _ => None,
                };
                line.targets.push(Line {
                    target: (*key).clone(),
                    layer: t.layer,
                    source_label: t.source_label.clone(),
                    context: t.context.clone(),
                    real: None,
                    real_se: Some(t.se),
                    difference: None,
                    distance,
                    real_shares: t.shares.clone(),
                });
            }
            line.engine_shares = engine;
        } else {
            let terms = measures::terms(def, &obs, Scope::All);
            line.engine = terms.value();
            line.engine_se = terms.value().map(|_| terms.se());
            for (key, t) in list {
                line.targets.push(Line {
                    target: (*key).clone(),
                    layer: t.layer,
                    source_label: t.source_label.clone(),
                    context: t.context.clone(),
                    real: t.value,
                    real_se: Some(t.se),
                    difference: line.engine.zip(t.value).map(|(e, r)| e - r),
                    distance: None,
                    real_shares: None,
                });
            }
        }
        bands.push(line);
    }
    let named: BTreeSet<&str> = linked.keys().copied().collect();
    let without_target = registry
        .names()
        .into_iter()
        .filter(|n| !named.contains(n))
        .map(str::to_string)
        .collect();
    Ok(Report {
        run: run.display().to_string(),
        targets_file: targets_path.display().to_string(),
        suite: Suite::Equal.code(),
        matches: obs.len(),
        bands,
        without_target,
        unknown_bands: unknown,
        side: SideLine {
            path: side_path.display().to_string(),
            targets: side_targets.map(BTreeMap::len),
        },
    })
}

impl Report {
    /// The printed table. Banding-group targets are left to `calibration.json`.
    pub fn text(&self) -> String {
        let mut out = format!(
            "calibration report: engine against real, {} matches of the {} suite\n\
             report only: the run's verdict is not changed\n",
            self.matches, self.suite
        );
        let f = |x: Option<f64>| x.map_or_else(|| "-".to_string(), |v| format!("{v:.4}"));
        for b in &self.bands {
            match &b.metric {
                Some(metric) => {
                    let _ = writeln!(out, "\n{} (whole distribution, {metric})", b.band);
                    if let Some(e) = &b.engine_shares {
                        let _ = writeln!(out, "  engine shares: {}", shares_text(e));
                    }
                }
                None => {
                    let _ = writeln!(
                        out,
                        "\n{}: engine {} (se {})",
                        b.band,
                        f(b.engine),
                        f(b.engine_se)
                    );
                }
            }
            let mut groups = 0;
            let mut seasons = 0;
            for t in &b.targets {
                if t.layer == Layer::Group {
                    groups += 1;
                    continue;
                }
                if per_season(&t.target) {
                    seasons += 1;
                    continue;
                }
                match &b.metric {
                    Some(_) => {
                        let _ = writeln!(
                            out,
                            "  {:<36} distance {}  real shares: {}\n    {}",
                            t.target,
                            f(t.distance),
                            t.real_shares
                                .as_deref()
                                .map_or_else(String::new, shares_text),
                            t.source_label
                        );
                    }
                    None => {
                        let _ = writeln!(
                            out,
                            "  {:<36} real {} (se {})  engine less real {}\n    {}",
                            t.target,
                            f(t.real),
                            f(t.real_se),
                            f(t.difference),
                            t.source_label
                        );
                    }
                }
            }
            if groups > 0 {
                let _ = writeln!(out, "  and {groups} banding-group targets in {FILE}");
            }
            if seasons > 0 {
                let _ = writeln!(
                    out,
                    "  and {seasons} StatsBomb competition-season targets in {FILE}"
                );
            }
        }
        if !self.without_target.is_empty() {
            let _ = writeln!(
                out,
                "\nbands without a real target here: {}",
                self.without_target.join(", ")
            );
        }
        if !self.unknown_bands.is_empty() {
            let _ = writeln!(
                out,
                "targets naming a band the registry does not hold: {}",
                self.unknown_bands.join(", ")
            );
        }
        match self.side.targets {
            Some(n) => {
                let _ = writeln!(
                    out,
                    "\nIMPECT side file: read from {} ({n} targets)",
                    self.side.path
                );
            }
            None => {
                let _ = writeln!(
                    out,
                    "\nIMPECT side file absent at {}: IMPECT-derived targets not shown",
                    self.side.path
                );
            }
        }
        out
    }
}

fn shares_text(s: &[f64]) -> String {
    let last = s.iter().rposition(|p| *p > 0.0).map_or(0, |i| i + 1);
    s[..last]
        .iter()
        .map(|p| format!("{p:.3}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Runs `engine-cli calibrate report`.
pub fn run(
    content_dir: Option<&Path>,
    run: &Path,
    targets: Option<&PathBuf>,
    side_dir: Option<&PathBuf>,
) -> anyhow::Result<i32> {
    let dir = ContentDir::resolve(content_dir)?;
    let registry = Registry::load(&dir)?;
    let path = targets.cloned().unwrap_or_else(|| default_file(dir.root()));
    let file = ReferenceFile::load(&path)?;
    let side_folder = side_dir
        .cloned()
        .or_else(|| side::default_dir(dir.root()))
        .unwrap_or_else(|| PathBuf::from(".scratch").join(side::DIR));
    let side_file = side::read_impect(&side_folder)?;
    let report = report(
        run,
        &registry,
        &file,
        &path,
        side_file.as_ref().map(|f| &f.targets),
        &side_folder.join(side::IMPECT_FILE),
    )?;
    print!("{}", report.text());
    let out = run.join(FILE);
    std::fs::write(&out, serde_json::to_string_pretty(&report)? + "\n")
        .with_context(|| format!("cannot write {}", out.display()))?;
    println!("wrote {}", out.display());
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_distances_match_their_definitions() {
        let a = [0.5, 0.5, 0.0];
        let b = [0.0, 0.5, 0.5];
        assert!((distance(Metric::Tv, &a, &b, 1.0) - 0.5).abs() < 1e-12);
        // Cumulative 0.5, 1, 1 against 0, 0.5, 1: |0.5| + |0.5| + 0, times width 2.
        assert!((distance(Metric::W1, &a, &b, 2.0) - 2.0).abs() < 1e-12);
    }
}

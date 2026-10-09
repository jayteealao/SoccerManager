//! The real values of the realism bands, read in place from the real-data folder at each
//! judgement. The band file holds only a pointer, the sample size and the date; no figure
//! measured from real matches enters the repository.
//!
//! The folder is `--real-data <DIR>`, or by default the main checkout's `.scratch/out`,
//! found from the content folder through git's own files (a `.git` folder, or in a linked
//! worktree a `.git` file naming its `gitdir`, whose `commondir` names the shared folder).
//! When the folder, a file or a key is not there, the band reads "no real data here" and
//! the run goes on: the real value is reported beside the verdict and never changes it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use engine::observe::round_to;
use serde::Serialize;
use serde_json::Value;

use super::bands::{BandDef, Evidence, Pointer, RealSource, Reduce, Registry};
use super::{se_mean, se_share};

/// Where the real-data folder came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// `--real-data`.
    Flag,
    /// The main checkout's `.scratch/out`.
    Default,
    /// No flag, and the content folder is in no git checkout.
    NotFound,
}

impl Origin {
    pub fn code(self) -> &'static str {
        match self {
            Origin::Flag => "flag",
            Origin::Default => "default",
            Origin::NotFound => "not_found",
        }
    }
}

/// The real-data folder of a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealFolder {
    pub path: Option<PathBuf>,
    pub from: Origin,
}

/// The folder `--real-data` names; otherwise the main checkout's `.scratch/out`, found from
/// the content folder's ancestors without running git.
pub fn resolve(flag: Option<&Path>, content_root: &Path) -> RealFolder {
    if let Some(dir) = flag {
        return RealFolder {
            path: Some(dir.to_path_buf()),
            from: Origin::Flag,
        };
    }
    let start = std::path::absolute(content_root).unwrap_or_else(|_| content_root.into());
    match main_checkout(&normal(&start)) {
        Some(main) => RealFolder {
            path: Some(main.join(".scratch").join("out")),
            from: Origin::Default,
        },
        None => RealFolder {
            path: None,
            from: Origin::NotFound,
        },
    }
}

/// The main checkout of the git checkout holding `dir`: the first ancestor with `.git`; a
/// `.git` folder makes it the main checkout, and a `.git` file (a linked worktree) names its
/// `gitdir`, whose `commondir` names the shared git folder inside the main checkout.
fn main_checkout(dir: &Path) -> Option<PathBuf> {
    for at in dir.ancestors() {
        let git = at.join(".git");
        if git.is_dir() {
            return Some(at.to_path_buf());
        }
        if git.is_file() {
            let text = std::fs::read_to_string(&git).ok()?;
            let gitdir = text.lines().find_map(|l| l.strip_prefix("gitdir:"))?.trim();
            let gitdir = at.join(gitdir);
            let common = match std::fs::read_to_string(gitdir.join("commondir")) {
                Ok(c) => gitdir.join(c.trim()),
                // A `.git` file without a shared folder (a submodule): its own checkout.
                Err(_) => return Some(at.to_path_buf()),
            };
            return normal(&common).parent().map(Path::to_path_buf);
        }
    }
    None
}

/// `path` with its `.` and `..` parts resolved by name, as a person would write it; no
/// link is followed, and no `\\?\` prefix is added on Windows.
fn normal(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// What reading one band's real value gave.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The value, its standard error when the source gives one, and the file's date when it
    /// is later than the band's `real.date`.
    Read {
        value: f64,
        se: Option<f64>,
        file_date: Option<String>,
    },
    /// The folder, the file or the key is not there, or not in the shape the pointer reads.
    NotHere { path: String, reason: String },
    /// The band measures an engine experiment; the folder is not read.
    NoCounterpart { reason: String },
    /// A band migrated from version 3: it has no real record.
    NoRecord,
}

/// The real value of `def` from `folder`.
pub fn read(folder: &RealFolder, def: &BandDef) -> Outcome {
    let Some(real) = &def.real else {
        return Outcome::NoRecord;
    };
    let pointer = match &real.source {
        RealSource::NoCounterpart(n) => {
            return Outcome::NoCounterpart {
                reason: n.none.clone(),
            };
        }
        RealSource::Pointer(p) => p,
    };
    let Some(dir) = &folder.path else {
        return Outcome::NotHere {
            path: String::new(),
            reason: "no real-data folder: the content folder is in no git checkout; \
                     pass --real-data"
                .into(),
        };
    };
    if !dir.is_dir() {
        return Outcome::NotHere {
            path: dir.display().to_string(),
            reason: "the folder does not exist".into(),
        };
    }
    let path = dir.join(&pointer.file);
    let not_here = |reason: String| Outcome::NotHere {
        path: path.display().to_string(),
        reason,
    };
    if !path.is_file() {
        return not_here("the file does not exist".into());
    }
    let read = if is_json(&pointer.file) {
        read_json(&path, pointer, real.sample)
    } else {
        read_csv(&path, pointer)
    };
    match read {
        Ok((value, se)) => {
            let scale = pointer.scale.unwrap_or(1.0);
            let file_date = modified_date(&path).filter(|d| *d > real.date);
            Outcome::Read {
                value: value * scale,
                se: se.map(|s| s * scale.abs()),
                file_date,
            }
        }
        Err(reason) => not_here(reason),
    }
}

fn is_json(file: &str) -> bool {
    file.ends_with(".json") || file.ends_with(".json.gz")
}

/// The lines of a file, decompressed when its name ends in `.gz`, streamed so a large
/// events file is never held in memory.
fn lines(path: &Path) -> Result<Box<dyn BufRead>, String> {
    let file = File::open(path).map_err(|e| format!("cannot open: {e}"))?;
    let gz = path.extension().is_some_and(|e| e == "gz");
    Ok(if gz {
        Box::new(BufReader::new(flate2::read::MultiGzDecoder::new(file)))
    } else {
        Box::new(BufReader::new(file))
    })
}

/// A JSON file: the value at the dotted key (`*` is the mean over every child, whose spread
/// gives the error), over `den` when it names one (no error), and a share over `sample`.
fn read_json(path: &Path, p: &Pointer, sample: u64) -> Result<(f64, Option<f64>), String> {
    let mut text = String::new();
    lines(path)?
        .read_to_string(&mut text)
        .map_err(|e| format!("cannot read: {e}"))?;
    let doc: Value = serde_json::from_str(&text).map_err(|e| format!("not JSON: {e}"))?;
    let num = values(&doc, &p.key)?;
    let value = mean(&num);
    let spread = p.key.split('.').any(|s| s == "*");
    if let Some(den) = &p.den {
        let d = mean(&values(&doc, den)?);
        if d == 0.0 {
            return Err(format!("{den} is 0"));
        }
        return Ok((value / d, None));
    }
    let se = if spread {
        Some(se_mean(&num))
    } else if p.reduce == Some(Reduce::Share) {
        if !(0.0..=1.0).contains(&value) {
            return Err(format!("{} is {value}, not a share from 0 to 1", p.key));
        }
        Some(se_share(
            value,
            usize::try_from(sample).unwrap_or(usize::MAX),
        ))
    } else {
        None
    };
    Ok((value, se))
}

/// The numbers at a dotted key, a `*` segment taking every child of an object.
fn values(doc: &Value, key: &str) -> Result<Vec<f64>, String> {
    let mut at = vec![doc];
    for seg in key.split('.') {
        let mut next = Vec::new();
        for v in at {
            if seg == "*" {
                match v.as_object() {
                    Some(o) => next.extend(o.values()),
                    None => return Err(format!("no value at {key}: {seg} needs an object")),
                }
            } else {
                match v.get(seg) {
                    Some(child) => next.push(child),
                    None => return Err(format!("no value at {key}")),
                }
            }
        }
        at = next;
    }
    if at.is_empty() {
        return Err(format!("no value at {key}"));
    }
    at.into_iter()
        .map(|v| {
            v.as_f64()
                .ok_or_else(|| format!("{key} holds {v}, not a number"))
        })
        .collect()
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        0.0
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}

/// A CSV file without quoted fields: a mean, a share or a group share over the kept rows.
fn read_csv(path: &Path, p: &Pointer) -> Result<(f64, Option<f64>), String> {
    let mut rows = lines(path)?.lines();
    let header = match rows.next() {
        Some(line) => line.map_err(|e| format!("cannot read: {e}"))?,
        None => return Err("the file is empty".into()),
    };
    let names: Vec<&str> = header.trim_end_matches('\r').split(',').collect();
    let column = |name: &str| {
        names
            .iter()
            .position(|n| *n == name)
            .ok_or_else(|| format!("no column {name}"))
    };
    let reduce = p.reduce.unwrap_or(Reduce::Mean);
    let filter: Vec<(usize, &Vec<String>)> = p
        .filter
        .iter()
        .flatten()
        .map(|(c, vals)| column(c).map(|i| (i, vals)))
        .collect::<Result<_, _>>()?;
    let (sum_cols, tokens, group) = if reduce == Reduce::GroupShare {
        let group = p
            .group_by
            .as_deref()
            .ok_or("a group share needs group_by")?;
        let tokens: BTreeSet<&str> = p.tokens.iter().flatten().map(String::as_str).collect();
        if tokens.is_empty() {
            return Err("a group share needs tokens".into());
        }
        (vec![column(&p.key)?], tokens, Some(column(group)?))
    } else {
        let cols = p.key.split('+').map(column).collect::<Result<_, _>>()?;
        (cols, BTreeSet::new(), None)
    };
    let (op, target) = (p.op, p.value);
    if reduce == Reduce::Share && (op.is_none() || target.is_none()) {
        return Err("a share needs op and value".into());
    }
    let mut sums = Vec::new();
    let mut groups: BTreeMap<String, bool> = BTreeMap::new();
    for (n, line) in rows.enumerate() {
        let line = line.map_err(|e| format!("cannot read line {}: {e}", n + 2))?;
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if line.contains('"') {
            return Err(format!(
                "line {} holds a quoted field; this reader takes none",
                n + 2
            ));
        }
        let cells: Vec<&str> = line.split(',').collect();
        let cell = |i: usize| {
            cells
                .get(i)
                .copied()
                .ok_or_else(|| format!("line {} has no column {}", n + 2, names[i]))
        };
        let mut kept = true;
        for (i, vals) in &filter {
            kept &= vals.iter().any(|v| v == cell(*i).unwrap_or(""));
        }
        if !kept {
            continue;
        }
        if let Some(g) = group {
            let hit = cell(sum_cols[0])?
                .split_whitespace()
                .any(|t| tokens.contains(t));
            *groups.entry(cell(g)?.to_string()).or_default() |= hit;
            continue;
        }
        let mut sum = 0.0;
        for &i in &sum_cols {
            let text = cell(i)?;
            sum += text.trim().parse::<f64>().map_err(|_| {
                format!("line {} column {}: {text} is not a number", n + 2, names[i])
            })?;
        }
        sums.push(sum);
    }
    match reduce {
        Reduce::GroupShare => {
            if groups.is_empty() {
                return Err("no row kept".into());
            }
            let hit = groups.values().filter(|h| **h).count();
            let p = hit as f64 / groups.len() as f64;
            Ok((p, Some(se_share(p, groups.len()))))
        }
        _ if sums.is_empty() => Err("no row kept".into()),
        Reduce::Mean => Ok((mean(&sums), Some(se_mean(&sums)))),
        Reduce::Share => {
            let (op, target) = (
                op.unwrap_or(super::bands::Op::AtLeast),
                target.unwrap_or(0.0),
            );
            let hit = sums.iter().filter(|s| op.holds(**s, target)).count();
            let p = hit as f64 / sums.len() as f64;
            Ok((p, Some(se_share(p, sums.len()))))
        }
    }
}

/// The file's modified date, as `YYYY-MM-DD` in UTC.
fn modified_date(path: &Path) -> Option<String> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let secs = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(civil_date(i64::try_from(secs / 86_400).ok()?))
}

/// The calendar date of a day count from 1970-01-01 (Howard Hinnant's `civil_from_days`).
fn civil_date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// The measurement word: how the real value's standard error compares with the band's
/// smallest shift. `precise` at a quarter of it or less, `fair` up to it, `rough` above,
/// `unknown` without an error, `n/a` with no real counterpart.
pub fn measurement_word(se: Option<f64>, shift: f64, no_counterpart: bool) -> &'static str {
    match se {
        _ if no_counterpart => "n/a",
        None => "unknown",
        Some(se) if se <= shift / 4.0 => "precise",
        Some(se) if se <= shift => "fair",
        Some(_) => "rough",
    }
}

/// The generalisation word: competitions times seasons against the floor. `broad` at twice
/// the floor or more, `narrow` at the floor or more, `thin` under it (the band only
/// reports), `n/a` with no real counterpart, `unknown` without a record.
pub fn generalisation_word(units: Option<u64>, floor: u64, no_counterpart: bool) -> &'static str {
    match units {
        _ if no_counterpart => "n/a",
        None => "unknown",
        Some(u) if u >= 2 * floor => "broad",
        Some(u) if u >= floor => "narrow",
        Some(_) => "thin",
    }
}

/// `calib.real`: the folder, the counts, and every band's real value and record.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RealInfo {
    /// The folder read; absent when none was found.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    pub from: &'static str,
    /// The folder exists.
    pub present: bool,
    /// Bands without a real value: not read here, or migrated without a record. A band with
    /// no real counterpart is never counted here.
    pub missing: u32,
    /// Bands that measure an engine experiment with no real counterpart.
    pub no_counterpart: u32,
    /// The folder, when it does not exist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_path: Option<String>,
    pub bands: Vec<RealBand>,
}

/// One band's real value and record in `calib.real`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RealBand {
    pub band: String,
    /// `read`, `not_here`, `no_counterpart` or `no_record`.
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub se: Option<f64>,
    /// The real file's date, when it is later than `date`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_date: Option<String>,
    /// Why there is no value: the missing path and reason, or the no-counterpart reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<RealSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Evidence>,
    pub measurement: MeasurementInfo,
    pub generalisation: GeneralisationInfo,
    /// Why the band only reports, when it does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_only: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MeasurementInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub se: Option<f64>,
    pub word: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GeneralisationInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub competitions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seasons: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clubs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<u64>,
    pub word: &'static str,
}

/// Reads every band's real value from `folder`.
pub fn read_all(folder: &RealFolder, registry: &Registry) -> RealInfo {
    let floor = u64::from(registry.generalisation_floor);
    let present = folder.path.as_ref().is_some_and(|p| p.is_dir());
    let mut info = RealInfo {
        folder: folder.path.as_ref().map(|p| p.display().to_string()),
        from: folder.from.code(),
        present,
        missing: 0,
        no_counterpart: 0,
        missing_path: folder
            .path
            .as_ref()
            .filter(|_| !present)
            .map(|p| p.display().to_string()),
        bands: Vec::with_capacity(registry.bands.len()),
    };
    for def in &registry.bands {
        let outcome = read(folder, def);
        let nc = matches!(outcome, Outcome::NoCounterpart { .. });
        let (status, value, se, file_date, reason) = match outcome {
            Outcome::Read {
                value,
                se,
                file_date,
            } => ("read", Some(value), se, file_date, None),
            Outcome::NotHere { path, reason } => {
                let why = if path.is_empty() {
                    reason
                } else {
                    format!("{path}: {reason}")
                };
                ("not_here", None, None, None, Some(why))
            }
            Outcome::NoCounterpart { reason } => ("no_counterpart", None, None, None, Some(reason)),
            Outcome::NoRecord => (
                "no_record",
                None,
                None,
                None,
                Some("no real record (migrated from an older band file)".into()),
            ),
        };
        match status {
            "not_here" | "no_record" => info.missing += 1,
            "no_counterpart" => info.no_counterpart += 1,
            _ => {}
        }
        let reach = def.confidence.map(|c| c.generalisation);
        let units = reach.map(|g| g.units());
        info.bands.push(RealBand {
            band: def.band.clone(),
            status,
            value: value.map(|v| round_to(v, 6)),
            se: se.map(|s| round_to(s, 6)),
            file_date,
            reason,
            source: def.real.as_ref().map(|r| r.source.clone()),
            sample: def.real.as_ref().map(|r| r.sample),
            date: def.real.as_ref().map(|r| r.date.clone()),
            evidence: def.evidence,
            measurement: MeasurementInfo {
                count: def.confidence.map(|c| c.measurement.count),
                se: se.map(|s| round_to(s, 6)),
                word: measurement_word(se, def.smallest_shift, nc),
            },
            generalisation: GeneralisationInfo {
                competitions: reach.map(|g| g.competitions),
                seasons: reach.map(|g| g.seasons),
                clubs: reach.map(|g| g.clubs),
                units,
                word: generalisation_word(units, floor, nc),
            },
            report_only: def.report_only.clone(),
        });
    }
    info
}

impl RealInfo {
    /// The band `name`'s entry.
    pub fn band(&self, name: &str) -> Option<&RealBand> {
        self.bands.iter().find(|b| b.band == name)
    }

    /// The console headline: the folder and where it came from, the bands without a real
    /// value, and the bands with no real counterpart; and, when the folder is absent, a line
    /// that says so.
    pub fn headline(&self) -> String {
        let total = self.bands.len();
        let folder = match (&self.folder, self.from) {
            (Some(f), from) => format!("{f} ({})", from.replace('_', " ")),
            (None, _) => {
                "no folder (the content folder is in no git checkout; pass --real-data)".to_string()
            }
        };
        let mut out = format!(
            "real data: {folder}; {} of {total} bands without a real value; {} with no real \
             counterpart\n",
            self.missing, self.no_counterpart
        );
        if let Some(p) = &self.missing_path {
            out.push_str(&format!("no real data here: {p} does not exist\n"));
        }
        out
    }
}

/// `calib.realism`: the realism claim, from validation bands with a real counterpart only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Realism {
    /// Validation bands with a real counterpart that the run judged and that gate.
    pub validation_bands: u32,
    /// Those whose value is inside their design range in this run.
    pub validation_inside: u32,
    /// Calibration bands the run judged: tuning may have read them, so they are not counted.
    pub calibration_bands: u32,
    pub claim: String,
}

/// The realism claim of a run over its verdict rows: only a gating validation band with a
/// real counterpart counts, and it counts as inside when every row of it is inside its
/// design range.
pub fn realism(registry: &Registry, rows: &[super::verdict::VerdictRow]) -> Realism {
    let mut validation = 0u32;
    let mut inside = 0u32;
    let mut calibration = 0u32;
    for def in &registry.bands {
        let mine: Vec<_> = rows.iter().filter(|r| r.band == def.band).collect();
        if mine.is_empty() {
            continue;
        }
        let counterpart = def
            .real
            .as_ref()
            .is_some_and(|r| r.no_counterpart().is_none());
        match def.evidence {
            Some(Evidence::Validation) if counterpart && def.report_only.is_none() => {
                validation += 1;
                if mine.iter().all(|r| r.inside()) {
                    inside += 1;
                }
            }
            Some(Evidence::Calibration) => calibration += 1,
            _ => {}
        }
    }
    let claim = if validation == 0 {
        format!(
            "realism claim: none; no band of this run is validation evidence with a real \
             counterpart ({calibration} calibration bands are not counted: tuning may have \
             read them)"
        )
    } else {
        format!(
            "realism claim: {inside} of {validation} validation bands inside their design \
             range ({calibration} calibration bands are not counted: tuning may have read them)"
        )
    };
    Realism {
        validation_bands: validation,
        validation_inside: inside,
        calibration_bands: calibration,
        claim,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::bands::{BandDef, Condition, Confidence, Generalisation, Measurement};
    use std::io::Write;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("engine-cli-real-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn pointer(file: &str, key: &str) -> Pointer {
        Pointer {
            dataset: "test".into(),
            file: file.into(),
            key: key.into(),
            reduce: None,
            op: None,
            value: None,
            filter: None,
            den: None,
            scale: None,
            group_by: None,
            tokens: None,
        }
    }

    fn band(source: RealSource, sample: u64, shift: f64) -> BandDef {
        BandDef {
            band: "b".into(),
            suites: vec!["equal".into()],
            measure: crate::report::bands::Measure::Builtin {
                name: crate::report::bands::Builtin::StrongerTeamWinRate,
            },
            lo: 0.0,
            hi: 1.0,
            smallest_shift: shift,
            group: None,
            real: Some(crate::report::bands::RealRecord {
                source,
                sample,
                date: "2026-09-22".into(),
                floor: None,
            }),
            evidence: Some(Evidence::Calibration),
            confidence: Some(Confidence {
                measurement: Measurement { count: sample },
                generalisation: Generalisation {
                    competitions: 5,
                    seasons: 1,
                    clubs: 98,
                },
            }),
            condition: Condition::Dry,
            max_from_dry: None,
            gates: true,
            report_only: None,
        }
    }

    fn at(dir: &Path) -> RealFolder {
        RealFolder {
            path: Some(dir.to_path_buf()),
            from: Origin::Flag,
        }
    }

    fn value(o: Outcome) -> (f64, Option<f64>) {
        match o {
            Outcome::Read { value, se, .. } => (value, se),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_default_folder_is_the_main_checkout_of_a_worktree_and_of_a_plain_checkout() {
        let root = temp("layout");
        // A plain checkout: a `.git` folder.
        let main = root.join("main");
        std::fs::create_dir_all(main.join(".git/worktrees/wt")).unwrap();
        std::fs::create_dir_all(main.join("content")).unwrap();
        let got = resolve(None, &main.join("content"));
        assert_eq!(got.from, Origin::Default);
        let expect = normal(&std::path::absolute(&main).unwrap())
            .join(".scratch")
            .join("out");
        assert_eq!(got.path.as_deref(), Some(expect.as_path()));
        // A linked worktree far from the main checkout: a `.git` file naming its gitdir,
        // whose commondir names the shared folder.
        let wt = root.join("deep/wt");
        std::fs::create_dir_all(wt.join("content")).unwrap();
        let gitdir = main.join(".git/worktrees/wt");
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", gitdir.display())).unwrap();
        std::fs::write(gitdir.join("commondir"), "../..\n").unwrap();
        let got = resolve(None, &wt.join("content"));
        assert_eq!(got.from, Origin::Default);
        assert_eq!(got.path.as_deref(), Some(expect.as_path()));
        // The flag wins.
        let flag = root.join("elsewhere");
        let got = resolve(Some(&flag), &wt.join("content"));
        assert_eq!((got.path, got.from), (Some(flag), Origin::Flag));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn each_pointer_kind_reads_its_value_and_error() {
        let dir = temp("kinds");
        std::fs::write(
            dir.join("rates.json"),
            r#"{"A": {"shots": 20.0}, "B": {"shots": 24.0}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("sb.json"),
            r#"{"s": {"on": 0.25, "g": 3.0, "xg": 2.0}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("m.csv"),
            "comp,hg,ag,home\nX,1,2,True\nX,0,0,False\nY,5,5,True\nX,2,2,True\n",
        )
        .unwrap();
        let f = at(&dir);
        // A `*` mean: the mean of the children, the spread of the children as its error.
        let mut p = pointer("rates.json", "*.shots");
        p.scale = Some(0.5);
        let (v, se) = value(read(&f, &band(RealSource::Pointer(p), 10, 1.0)));
        assert_eq!(v, 11.0);
        assert!((se.unwrap() - se_mean(&[20.0, 24.0]) * 0.5).abs() < 1e-12);
        // A ratio: no error.
        let mut p = pointer("sb.json", "s.g");
        p.den = Some("s.xg".into());
        assert_eq!(
            value(read(&f, &band(RealSource::Pointer(p), 10, 1.0))),
            (1.5, None)
        );
        // A JSON share: its error over the sample.
        let mut p = pointer("sb.json", "s.on");
        p.reduce = Some(Reduce::Share);
        let (v, se) = value(read(&f, &band(RealSource::Pointer(p), 300, 1.0)));
        assert_eq!((v, se), (0.25, Some(se_share(0.25, 300))));
        // A CSV mean of summed columns over the kept rows.
        let mut p = pointer("m.csv", "hg+ag");
        p.reduce = Some(Reduce::Mean);
        p.filter = Some(BTreeMap::from([("comp".into(), vec!["X".into()])]));
        let (v, se) = value(read(&f, &band(RealSource::Pointer(p), 3, 1.0)));
        assert_eq!(v, 7.0 / 3.0);
        assert_eq!(se, Some(se_mean(&[3.0, 0.0, 4.0])));
        // A CSV share.
        let mut p = pointer("m.csv", "hg+ag");
        p.reduce = Some(Reduce::Share);
        p.op = Some(crate::report::bands::Op::Equal);
        p.value = Some(0.0);
        let (v, se) = value(read(&f, &band(RealSource::Pointer(p), 4, 1.0)));
        assert_eq!((v, se), (0.25, Some(se_share(0.25, 4))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_group_share_over_a_gzip_stream_counts_distinct_groups() {
        let dir = temp("gz");
        let rows = "match,tags\n1,701 1802\n1,1701\n2,1801\n2,\n3,1703 9\n3,5\n";
        let mut enc = flate2::write::GzEncoder::new(
            File::create(dir.join("e.csv.gz")).unwrap(),
            flate2::Compression::default(),
        );
        enc.write_all(rows.as_bytes()).unwrap();
        enc.finish().unwrap();
        let mut p = pointer("e.csv.gz", "tags");
        p.reduce = Some(Reduce::GroupShare);
        p.group_by = Some("match".into());
        p.tokens = Some(vec!["1701".into(), "1703".into()]);
        let (v, se) = value(read(&at(&dir), &band(RealSource::Pointer(p), 3, 1.0)));
        assert_eq!(v, 2.0 / 3.0);
        assert_eq!(se, Some(se_share(2.0 / 3.0, 3)));
        // A corrupt stream is not here, by name.
        std::fs::write(dir.join("bad.csv.gz"), b"not gzip at all").unwrap();
        let mut p = pointer("bad.csv.gz", "tags");
        p.reduce = Some(Reduce::GroupShare);
        p.group_by = Some("match".into());
        p.tokens = Some(vec!["1701".into()]);
        assert!(matches!(
            read(&at(&dir), &band(RealSource::Pointer(p), 3, 1.0)),
            Outcome::NotHere { .. }
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn each_missing_piece_reads_not_here_and_no_counterpart_never_reads_the_folder() {
        let dir = temp("missing");
        std::fs::write(dir.join("r.json"), r#"{"a": {"b": "text"}}"#).unwrap();
        std::fs::write(dir.join("q.csv"), "a,b\n\"1\",2\n").unwrap();
        let f = at(&dir);
        let not_here =
            |p: Pointer, f: &RealFolder| match read(f, &band(RealSource::Pointer(p), 5, 1.0)) {
                Outcome::NotHere { path, reason } => format!("{path}: {reason}"),
                other => panic!("{other:?}"),
            };
        assert!(not_here(pointer("r.json", "a.b"), &f).contains("not a number"));
        assert!(not_here(pointer("r.json", "a.c"), &f).contains("no value at a.c"));
        assert!(not_here(pointer("gone.json", "a"), &f).contains("does not exist"));
        assert!(not_here(pointer("q.csv", "a"), &f).contains("quoted field"));
        assert!(not_here(pointer("q.csv", "z"), &f).contains("no column z"));
        let absent = at(&dir.join("absent"));
        assert!(not_here(pointer("r.json", "a.b"), &absent).contains("folder does not exist"));
        let none = RealSource::NoCounterpart(crate::report::bands::NoCounterpart {
            none: "engine experiment".into(),
        });
        assert_eq!(
            read(&absent, &band(none, 0, 1.0)),
            Outcome::NoCounterpart {
                reason: "engine experiment".into()
            }
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_words_follow_their_fixed_rules() {
        assert_eq!(measurement_word(Some(0.25), 1.0, false), "precise");
        assert_eq!(measurement_word(Some(0.26), 1.0, false), "fair");
        assert_eq!(measurement_word(Some(1.0), 1.0, false), "fair");
        assert_eq!(measurement_word(Some(1.01), 1.0, false), "rough");
        assert_eq!(measurement_word(None, 1.0, false), "unknown");
        assert_eq!(measurement_word(None, 1.0, true), "n/a");
        assert_eq!(generalisation_word(Some(4), 2, false), "broad");
        assert_eq!(generalisation_word(Some(3), 2, false), "narrow");
        assert_eq!(generalisation_word(Some(2), 2, false), "narrow");
        assert_eq!(generalisation_word(Some(1), 2, false), "thin");
        assert_eq!(generalisation_word(Some(0), 2, true), "n/a");
        assert_eq!(generalisation_word(None, 2, false), "unknown");
    }

    #[test]
    fn a_day_count_gives_its_calendar_date() {
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(20_718), "2026-09-22");
        assert_eq!(civil_date(11_016), "2000-02-29");
    }

    use crate::report::verdict::{Joint, VerdictRow, Word, mark_report_only};

    /// The shipped band file with edits to named bands, as a registry.
    fn registry(edits: &[(&str, &str, Value)]) -> Registry {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/realism-bands.json");
        let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        for (band, field, value) in edits {
            let b = doc["bands"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|b| b["band"] == *band)
                .unwrap();
            let mut at = b;
            let parts: Vec<&str> = field.split('.').collect();
            for part in &parts[..parts.len() - 1] {
                at = &mut at[*part];
            }
            at[parts[parts.len() - 1]] = value.clone();
        }
        Registry::from_bytes(doc.to_string().as_bytes()).unwrap()
    }

    /// A plain run's row of `band` with `word`.
    fn row(band: &str, word: Word) -> VerdictRow {
        VerdictRow {
            band: band.into(),
            suite: "equal".into(),
            pairing: None,
            word,
            value: 1.0,
            lo: 0.0,
            hi: 2.0,
            base_value: None,
            diff: None,
            se: None,
            smallest_shift: None,
            power: None,
            needed: None,
            outside_by: None,
            no_target: None,
            real: None,
            distance: None,
            report_only: None,
            real_note: None,
        }
    }

    #[test]
    fn the_realism_claim_counts_only_validation_bands_with_a_real_counterpart() {
        let r = registry(&[
            ("goals_per_match", "evidence", "validation".into()),
            ("stronger_team_win_rate", "evidence", "validation".into()),
        ]);
        let rows = [
            row("goals_per_match", Word::Pass),
            row("corners_per_team", Word::Fail),
            row("stronger_team_win_rate", Word::Pass),
        ];
        let claim = realism(&r, &rows);
        assert_eq!(claim.validation_bands, 1, "{claim:?}");
        assert_eq!(claim.validation_inside, 1);
        assert_eq!(claim.calibration_bands, 1, "only corners: {claim:?}");
        assert!(
            claim.claim.contains("1 of 1 validation bands"),
            "{}",
            claim.claim
        );
        assert!(
            claim.claim.contains("calibration bands are not counted"),
            "{}",
            claim.claim
        );
        // With no validation band, the claim says there is none.
        let none = realism(&registry(&[]), &rows);
        assert_eq!(none.validation_bands, 0);
        assert!(
            none.claim.starts_with("realism claim: none"),
            "{}",
            none.claim
        );
    }

    #[test]
    fn a_band_under_its_floor_prints_report_only_and_leaves_the_joint_word() {
        let thin = serde_json::json!({"competitions": 1, "seasons": 1, "clubs": 20});
        let r = registry(&[("corners_per_team", "confidence.generalisation", thin)]);
        // The record shows the counts, the error and the words.
        let info = read_all(
            &at(&std::env::temp_dir().join("engine-cli-real-absent")),
            &r,
        );
        let corners = info.band("corners_per_team").unwrap();
        assert_eq!(corners.generalisation.units, Some(1));
        assert_eq!(corners.generalisation.word, "thin");
        assert_eq!(corners.measurement.count, Some(1826));
        assert_eq!(corners.measurement.word, "unknown");
        assert!(
            corners
                .report_only
                .as_deref()
                .unwrap()
                .contains("under the floor")
        );
        let goals = info.band("goals_per_match").unwrap();
        assert_eq!(
            (goals.generalisation.word, goals.report_only.as_deref()),
            ("broad", None)
        );
        // A band with no counterpart: counts of 0, words n/a, and it still gates.
        let strong = info.band("stronger_team_win_rate").unwrap();
        assert_eq!(strong.status, "no_counterpart");
        assert_eq!(
            (strong.measurement.word, strong.generalisation.word),
            ("n/a", "n/a")
        );
        assert_eq!(strong.report_only, None);
        assert_eq!((info.missing, info.no_counterpart), (15, 3));

        let mut joint = Joint {
            word: Word::Fail,
            critical: 2.5,
            resamples: 1999,
            false_alarm: 0.03,
            pairs: 10,
            guards: Vec::new(),
        };
        let mut rows = vec![
            row("goals_per_match", Word::Pass),
            row("corners_per_team", Word::Fail),
            row("stronger_team_win_rate", Word::Pass),
        ];
        mark_report_only(&mut rows, Some(&mut joint), |b| r.report_only(b));
        assert!(rows[1].report_only.is_some());
        assert_eq!(rows[1].word, Word::Fail, "its word is still shown");
        assert_eq!(joint.word, Word::Pass, "the report-only fail is left out");
        assert!(rows[2].report_only.is_none());
        rows[2].word = Word::Fail;
        mark_report_only(&mut rows, Some(&mut joint), |b| r.report_only(b));
        assert_eq!(
            joint.word,
            Word::Fail,
            "a no-counterpart band stays in the word"
        );
    }
}

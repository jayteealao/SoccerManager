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

use pass_rule::interval::{Real, Unit};

use super::bands::{BandDef, ChainCount, Evidence, Measure, Pointer, RealSource, Reduce, Registry};
use super::distribution::{Sample, bin_of};
use super::{se_mean, se_ratio, se_share};
use crate::real_targets::ReferenceFile;

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

/// The real value of `def` from `folder`, the file read for this band alone.
#[cfg(test)]
pub fn read(folder: &RealFolder, def: &BandDef) -> Outcome {
    read_with(folder, def, None, None).0
}

/// One reduction of a file: the value, its standard error when the reduction gives one, and
/// the pass rule's real side (per-unit terms, tracked values, or the value and its error).
#[derive(Debug, Clone, PartialEq)]
pub struct Reduced {
    pub value: f64,
    pub se: Option<f64>,
    pub side: Real,
}

/// The real value of `def` from `folder`, and its real side for the pass rule; `csv` is the
/// reduction of its CSV file when one pass over the file already read it for several bands,
/// and `reference` the reference file of real targets for a band that names one.
fn read_with(
    folder: &RealFolder,
    def: &BandDef,
    csv: Option<Result<Reduced, String>>,
    reference: Option<&ReferenceFile>,
) -> (Outcome, Option<Real>) {
    let Some(real) = &def.real else {
        return (Outcome::NoRecord, None);
    };
    let pointer = match &real.source {
        RealSource::NoCounterpart(n) => {
            return (
                Outcome::NoCounterpart {
                    reason: n.none.clone(),
                },
                None,
            );
        }
        RealSource::Pointer(p) => p,
        RealSource::Target(t) => {
            let found = reference.and_then(|f| f.target(&t.target));
            return match found.and_then(|x| x.value.map(|v| (v, x.se, x.per_match.clone()))) {
                // A tracked-match target carries its matches: the band resamples them, one
                // left out at a time.
                Some((value, se, Some(values))) if !values.is_empty() => (
                    Outcome::Read {
                        value,
                        se: Some(se),
                        file_date: None,
                    },
                    Some(Real::Tracked(values)),
                ),
                Some((value, se, _)) => (
                    Outcome::Read {
                        value,
                        se: Some(se),
                        file_date: None,
                    },
                    Some(Real::Summary { value, se }),
                ),
                None => (
                    Outcome::NotHere {
                        path: t.target.clone(),
                        reason: if reference.is_some() {
                            "the reference file of real targets has no scalar target by this key"
                                .into()
                        } else {
                            "no reference file of real targets in gate/".into()
                        },
                    },
                    None,
                ),
            };
        }
    };
    let Some(dir) = &folder.path else {
        return (
            Outcome::NotHere {
                path: String::new(),
                reason: "no real-data folder: the content folder is in no git checkout; \
                         pass --real-data"
                    .into(),
            },
            None,
        );
    };
    if !dir.is_dir() {
        return (
            Outcome::NotHere {
                path: dir.display().to_string(),
                reason: "the folder does not exist".into(),
            },
            None,
        );
    }
    let path = dir.join(&pointer.file);
    let not_here = |reason: String| {
        (
            Outcome::NotHere {
                path: path.display().to_string(),
                reason,
            },
            None,
        )
    };
    if !path.is_file() {
        return not_here("the file does not exist".into());
    }
    let read = if is_json(&pointer.file) {
        read_json(&path, pointer, real.sample)
    } else {
        csv.unwrap_or_else(|| read_csv(&path, pointer))
    };
    match read {
        Ok(Reduced { value, se, side }) => {
            let scale = pointer.scale.unwrap_or(1.0);
            let file_date = modified_date(&path).filter(|d| *d > real.date);
            (
                Outcome::Read {
                    value: value * scale,
                    se: se.map(|s| s * scale.abs()),
                    file_date,
                },
                Some(scaled(side, scale)),
            )
        }
        Err(reason) => not_here(reason),
    }
}

/// A real side with every value times `scale`.
fn scaled(side: Real, scale: f64) -> Real {
    if scale == 1.0 {
        return side;
    }
    match side {
        Real::Matches(strata) => Real::Matches(
            strata
                .into_iter()
                .map(|units| {
                    units
                        .into_iter()
                        .map(|u| Unit {
                            num: u.num * scale,
                            ..u
                        })
                        .collect()
                })
                .collect(),
        ),
        Real::Summary { value, se } => Real::Summary {
            value: value * scale,
            se: se * scale.abs(),
        },
        Real::Tracked(values) => Real::Tracked(values.into_iter().map(|v| v * scale).collect()),
        Real::Fixed(v) => Real::Fixed(v * scale),
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
/// gives the error; keys joined by `+` are summed), over `den` when it names one (no error),
/// and a share over `sample`.
fn read_json(path: &Path, p: &Pointer, sample: u64) -> Result<Reduced, String> {
    let mut text = String::new();
    lines(path)?
        .read_to_string(&mut text)
        .map_err(|e| format!("cannot read: {e}"))?;
    let doc: Value = serde_json::from_str(&text).map_err(|e| format!("not JSON: {e}"))?;
    // Terms joined by `+` are summed: each term's value (its mean, for a `*` term) added.
    let terms = p
        .key
        .split('+')
        .map(|k| values(&doc, k))
        .collect::<Result<Vec<_>, _>>()?;
    let num: Vec<f64> = match terms.as_slice() {
        [one] => one.clone(),
        many => vec![many.iter().map(|t| mean(t)).sum()],
    };
    let value = mean(&num);
    let spread = terms.len() == 1 && p.key.split('.').any(|s| s == "*");
    if let Some(den) = &p.den {
        let d = mean(&values(&doc, den)?);
        if d == 0.0 {
            return Err(format!("{den} is 0"));
        }
        // A ratio of two summaries has no error the file gives: its real side is exact.
        return Ok(Reduced {
            value: value / d,
            se: None,
            side: Real::Summary {
                value: value / d,
                se: 0.0,
            },
        });
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
    // A `*` term holds one value per tracked match: the pass rule leaves one match out at a
    // time. Any other summary is drawn from its value and error.
    let side = if spread {
        Real::Tracked(num)
    } else {
        Real::Summary {
            value,
            se: se.unwrap_or(0.0),
        }
    };
    Ok(Reduced { value, se, side })
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

/// The columns of a key written as columns joined by `+` or `-`, each with its sign.
fn signed_columns(
    key: &str,
    column: &dyn Fn(&str) -> Result<usize, String>,
) -> Result<Vec<(usize, f64)>, String> {
    let mut out = Vec::new();
    let mut sign = 1.0;
    let mut start = 0;
    for (i, ch) in key.char_indices().chain([(key.len(), '+')]) {
        if ch == '+' || ch == '-' {
            out.push((column(&key[start..i])?, sign));
            sign = if ch == '-' { -1.0 } else { 1.0 };
            start = i + 1;
        }
    }
    Ok(out)
}

/// A score such as `5-1`.
fn score(text: &str) -> Result<[f64; 2], String> {
    let parse = |s: &str| s.trim().parse::<f64>().ok();
    match text.split_once('-').map(|(a, b)| (parse(a), parse(b))) {
        Some((Some(a), Some(b))) => Ok([a, b]),
        _ => Err(format!("score {text} is not two numbers joined by -")),
    }
}

/// What a CSV pointer reads from each kept row.
enum Key {
    /// The signed sum of columns.
    Sum(Vec<(usize, f64)>),
    /// Two columns: a point (`x:y`) or a score (`home:away`).
    Pair(usize, usize),
    /// One column of tokens split on spaces.
    Tokens(usize),
}

/// A CSV file without quoted fields, streamed line by line: the kept rows (by `where`,
/// `range`, `same` and `skip_blank`), reduced as the pointer says. A mean is weighted by `weight` or
/// divided by the sum of `den` when the pointer names one.
fn read_csv(path: &Path, p: &Pointer) -> Result<Reduced, String> {
    read_csv_all(path, &[p])
        .pop()
        .unwrap_or_else(|| Err("no pointer".into()))
}

/// Every pointer of `pointers` over one CSV file, in one pass over its lines: each pointer's
/// reduction, in their order, as [`read_csv`] gives it alone. A file read once serves every
/// band that points into it, so a large compressed file is inflated once per judgement.
fn read_csv_all(path: &Path, pointers: &[&Pointer]) -> Vec<Result<Reduced, String>> {
    let fail = |e: String| pointers.iter().map(|_| Err(e.clone())).collect();
    let mut rows = match lines(path) {
        Ok(r) => r.lines(),
        Err(e) => return fail(e),
    };
    let header = match rows.next() {
        Some(Ok(line)) => line,
        Some(Err(e)) => return fail(format!("cannot read: {e}")),
        None => return fail("the file is empty".into()),
    };
    let names: Vec<&str> = header.trim_end_matches('\r').split(',').collect();
    let mut accs: Vec<Result<CsvAcc<'_>, String>> =
        pointers.iter().map(|p| CsvAcc::new(&names, p)).collect();
    // A reason that ends every pointer still reading: the line cannot be read or holds a
    // quoted field.
    let stop_all = |accs: &mut Vec<Result<CsvAcc<'_>, String>>, why: String| {
        for a in accs.iter_mut().filter(|a| a.is_ok()) {
            *a = Err(why.clone());
        }
    };
    for (n, line) in rows.enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                stop_all(&mut accs, format!("cannot read line {}: {e}", n + 2));
                break;
            }
        };
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if line.contains('"') {
            stop_all(
                &mut accs,
                format!(
                    "line {} holds a quoted field; this reader takes none",
                    n + 2
                ),
            );
            break;
        }
        let cells: Vec<&str> = line.split(',').collect();
        let mut reading = false;
        for a in &mut accs {
            let fed = match a {
                Ok(acc) => acc.feed(n, &names, &cells),
                Err(_) => continue,
            };
            match fed {
                Ok(()) => reading = true,
                Err(e) => *a = Err(e),
            }
        }
        if !reading {
            break;
        }
    }
    accs.into_iter()
        .map(|a| a.and_then(CsvAcc::finish))
        .collect()
}

/// One CSV pointer's reading: its columns, found from the header, and the kept rows' values,
/// weights, denominators and groups so far.
struct CsvAcc<'a> {
    p: &'a Pointer,
    reduce: Reduce,
    filter: Vec<(usize, &'a Vec<String>)>,
    ranges: Vec<(usize, [f64; 2])>,
    same: Option<(usize, usize)>,
    weight: Option<usize>,
    den: Option<Vec<(usize, f64)>>,
    group: Option<Vec<usize>>,
    key: Key,
    tokens: BTreeSet<&'a str>,
    scores: Vec<[f64; 2]>,
    xs: Vec<f64>,
    ws: Vec<f64>,
    ds: Vec<f64>,
    groups: BTreeMap<String, Vec<f64>>,
    hits: BTreeMap<String, bool>,
}

impl<'a> CsvAcc<'a> {
    /// The reading of `p` over a file of the columns `names`, or why it cannot read it.
    fn new(names: &[&str], p: &'a Pointer) -> Result<Self, String> {
        let column = |name: &str| {
            names
                .iter()
                .position(|n| *n == name)
                .ok_or_else(|| format!("no column {name}"))
        };
        let reduce = p.reduce.unwrap_or(Reduce::Mean);
        if matches!(reduce, Reduce::Histogram | Reduce::Cells | Reduce::Chains) {
            return Err(WHOLE_SAMPLE.into());
        }
        let filter: Vec<(usize, &Vec<String>)> = p
            .filter
            .iter()
            .flatten()
            .map(|(c, vals)| column(c).map(|i| (i, vals)))
            .collect::<Result<_, _>>()?;
        let ranges: Vec<(usize, [f64; 2])> = p
            .range
            .iter()
            .flatten()
            .map(|(c, r)| column(c).map(|i| (i, *r)))
            .collect::<Result<_, _>>()?;
        let same = match &p.same {
            Some([a, b]) => Some((column(a)?, column(b)?)),
            None => None,
        };
        let weight = p.weight.as_deref().map(column).transpose()?;
        let den = match &p.den {
            Some(d) => Some(signed_columns(d, &column)?),
            None => None,
        };
        let group: Option<Vec<usize>> = match &p.group_by {
            Some(g) => Some(g.split('+').map(column).collect::<Result<_, _>>()?),
            None => None,
        };
        let need = |what: &str| format!("a {} needs {what}", reduce_name(reduce));
        let key = match reduce {
            Reduce::GroupShare => Key::Tokens(column(&p.key)?),
            Reduce::GoalDistance | Reduce::Scoreline => {
                let (a, b) = p
                    .key
                    .split_once(':')
                    .ok_or_else(|| need("its key as two columns joined by :"))?;
                Key::Pair(column(a)?, column(b)?)
            }
            _ => Key::Sum(signed_columns(&p.key, &column)?),
        };
        let tokens: BTreeSet<&str> = p.tokens.iter().flatten().map(String::as_str).collect();
        let scores: Vec<[f64; 2]> = p
            .scores
            .iter()
            .flatten()
            .map(|s| score(s))
            .collect::<Result<_, _>>()?;
        match reduce {
            Reduce::GroupShare if tokens.is_empty() => return Err(need("tokens")),
            Reduce::Share if p.op.is_none() || p.value.is_none() => {
                return Err(need("op and value"));
            }
            Reduce::BinShare if p.bins.is_none() => return Err(need("bins")),
            Reduce::GoalDistance if p.goal.is_none() => return Err(need("goal")),
            Reduce::Scoreline if scores.is_empty() => return Err(need("scores")),
            Reduce::GroupRange if p.window.is_none_or(|w| w < 2) => {
                return Err(need("a window of at least 2"));
            }
            Reduce::GroupShare | Reduce::GroupRange | Reduce::GroupMax if group.is_none() => {
                return Err(need("group_by"));
            }
            _ => {}
        }
        Ok(Self {
            p,
            reduce,
            filter,
            ranges,
            same,
            weight,
            den,
            group,
            key,
            tokens,
            scores,
            xs: Vec::new(),
            ws: Vec::new(),
            ds: Vec::new(),
            groups: BTreeMap::new(),
            hits: BTreeMap::new(),
        })
    }

    /// Takes line `n` (0 for the first after the header), split into `cells`, when the
    /// pointer keeps it.
    fn feed(&mut self, n: usize, names: &[&str], cells: &[&str]) -> Result<(), String> {
        let cell = |i: usize| {
            cells
                .get(i)
                .copied()
                .ok_or_else(|| format!("line {} has no column {}", n + 2, names[i]))
        };
        let number = |i: usize| -> Result<f64, String> {
            let text = cell(i)?;
            text.trim()
                .parse::<f64>()
                .map_err(|_| format!("line {} column {}: {text} is not a number", n + 2, names[i]))
        };
        let sum = |cols: &[(usize, f64)]| -> Result<f64, String> {
            cols.iter()
                .try_fold(0.0, |acc, &(i, s)| Ok(acc + s * number(i)?))
        };
        let mut kept = true;
        for (i, vals) in &self.filter {
            kept &= vals.iter().any(|v| v == cell(*i).unwrap_or(""));
        }
        if let Some((a, b)) = self.same {
            kept &= cell(a)? == cell(b)?;
        }
        for &(i, [lo, hi]) in &self.ranges {
            if !kept {
                break;
            }
            let v = number(i)?;
            kept &= lo <= v && v < hi;
        }
        if !kept {
            return Ok(());
        }
        if self.p.skip_blank {
            let used = match &self.key {
                Key::Sum(cols) => cols.iter().map(|c| c.0).collect(),
                Key::Pair(a, b) => vec![*a, *b],
                Key::Tokens(i) => vec![*i],
            };
            let blank = used
                .into_iter()
                .chain(self.weight)
                .chain(self.den.iter().flatten().map(|c| c.0))
                .any(|i| cell(i).is_ok_and(|t| t.trim().is_empty()));
            if blank {
                return Ok(());
            }
        }
        let name = |cols: &[usize]| -> Result<String, String> {
            Ok(cols
                .iter()
                .map(|&i| cell(i))
                .collect::<Result<Vec<_>, _>>()?
                .join("+"))
        };
        let x = match &self.key {
            Key::Tokens(i) => {
                let hit = cell(*i)?
                    .split_whitespace()
                    .any(|t| self.tokens.contains(t));
                *self
                    .hits
                    .entry(name(self.group.as_deref().unwrap_or(&[]))?)
                    .or_default() |= hit;
                return Ok(());
            }
            Key::Pair(a, b) => {
                let (u, v) = (number(*a)?, number(*b)?);
                match (self.reduce, self.p.goal) {
                    (Reduce::GoalDistance, Some([gx, gy])) => (gx - u).hypot(gy - v),
                    _ => f64::from(u8::from(
                        self.scores.iter().any(|s| *s == [u, v] || *s == [v, u]),
                    )),
                }
            }
            Key::Sum(cols) => sum(cols)?,
        };
        if let Some(g) = &self.group {
            self.groups.entry(name(g)?).or_default().push(x);
            return Ok(());
        }
        self.xs.push(match (self.reduce, self.p.bins) {
            (Reduce::BinShare, Some([lo, hi])) => f64::from(u8::from(lo <= x && x < hi)),
            _ => x,
        });
        self.ws.push(self.weight.map_or(Ok(1.0), number)?);
        if let Some(d) = &self.den {
            self.ds.push(sum(d)?);
        }
        Ok(())
    }

    /// The reduction of the kept rows, with its standard error when the reduction gives one,
    /// and the per-unit terms the pass rule resamples: one unit per kept row, or per group
    /// for a reduction over groups.
    fn finish(self) -> Result<Reduced, String> {
        let Self {
            p,
            reduce,
            weight,
            xs,
            ws,
            ds,
            groups,
            hits,
            ..
        } = self;
        let read = |value: f64, se: Option<f64>, units: Vec<Unit>| -> Result<Reduced, String> {
            Ok(Reduced {
                value,
                se,
                side: Real::Matches(vec![units]),
            })
        };
        let ones = |v: &[f64]| -> Vec<Unit> { v.iter().map(|x| Unit::ratio(*x, 1.0)).collect() };
        let per_group = |f: &dyn Fn(&[f64]) -> Option<f64>| -> Result<Reduced, String> {
            let values: Vec<f64> = groups.values().filter_map(|v| f(v)).collect();
            if values.is_empty() {
                return Err("no group has enough rows".into());
            }
            read(mean(&values), Some(se_mean(&values)), ones(&values))
        };
        let largest = |v: &[f64]| v.iter().copied().fold(f64::MIN, f64::max);
        let smallest = |v: &[f64]| v.iter().copied().fold(f64::MAX, f64::min);
        match reduce {
            Reduce::GroupShare => {
                if hits.is_empty() {
                    return Err("no row kept".into());
                }
                let flags: Vec<f64> = hits.values().map(|h| f64::from(u8::from(*h))).collect();
                let p = mean(&flags);
                read(p, Some(se_share(p, hits.len())), ones(&flags))
            }
            Reduce::GroupRange => {
                let window = p.window.unwrap_or(2) as usize;
                per_group(&|v| {
                    let first = &v[..window.min(v.len())];
                    (v.len() >= window).then(|| largest(first) - smallest(first))
                })
            }
            Reduce::GroupMax => per_group(&|v| (!v.is_empty()).then(|| largest(v))),
            _ if xs.is_empty() => Err("no row kept".into()),
            Reduce::Mean if !ds.is_empty() => {
                let d: f64 = ds.iter().sum();
                if d == 0.0 {
                    return Err(format!("{} sums to 0", p.den.as_deref().unwrap_or("den")));
                }
                let units = xs
                    .iter()
                    .zip(&ds)
                    .map(|(x, d)| Unit::ratio(*x, *d))
                    .collect();
                read(xs.iter().sum::<f64>() / d, None, units)
            }
            Reduce::Mean | Reduce::BinShare if weight.is_some() => {
                let w: f64 = ws.iter().sum();
                if w <= 0.0 {
                    return Err("the weights sum to 0".into());
                }
                let wx: Vec<f64> = xs.iter().zip(&ws).map(|(x, w)| x * w).collect();
                let units = wx
                    .iter()
                    .zip(&ws)
                    .map(|(a, b)| Unit::ratio(*a, *b))
                    .collect();
                read(wx.iter().sum::<f64>() / w, Some(se_ratio(&wx, &ws)), units)
            }
            Reduce::Mean | Reduce::GoalDistance => read(mean(&xs), Some(se_mean(&xs)), ones(&xs)),
            Reduce::BinShare | Reduce::Scoreline => {
                let p = mean(&xs);
                read(p, Some(se_share(p, xs.len())), ones(&xs))
            }
            Reduce::Share => {
                let (op, target) = (
                    p.op.unwrap_or(super::bands::Op::AtLeast),
                    p.value.unwrap_or(0.0),
                );
                let flags: Vec<f64> = xs
                    .iter()
                    .map(|s| f64::from(u8::from(op.holds(*s, target))))
                    .collect();
                let p = mean(&flags);
                read(p, Some(se_share(p, xs.len())), ones(&flags))
            }
            Reduce::FactorialRatio => {
                let s: f64 = xs.iter().sum();
                if s <= 0.0 {
                    return Err(format!("{} sums to 0", p.key));
                }
                let f: f64 = xs.iter().map(|x| x * (x - 1.0)).sum();
                // n sum(x(x-1)) / sum(x)^2 = (sum x(x-1) / sum x) (n / sum x).
                let units = xs
                    .iter()
                    .map(|x| Unit {
                        num: x * (x - 1.0),
                        den: *x,
                        scale_num: 1.0,
                        scale_den: *x,
                    })
                    .collect();
                read(xs.len() as f64 * f / (s * s), None, units)
            }
            Reduce::Histogram | Reduce::Cells | Reduce::Chains => Err(WHOLE_SAMPLE.into()),
        }
    }
}

/// The reductions of every band with a single value read from a CSV file of `folder`, by
/// band: each file is streamed once for all the bands that point into it.
fn csv_values<'r>(
    folder: &RealFolder,
    registry: &'r Registry,
) -> BTreeMap<&'r str, Result<Reduced, String>> {
    let mut by_file: BTreeMap<PathBuf, Vec<(&'r str, &'r Pointer)>> = BTreeMap::new();
    if let Some(dir) = folder.path.as_ref().filter(|d| d.is_dir()) {
        for def in registry.bands.iter().filter(|d| !d.is_distribution()) {
            if let Some(RealSource::Pointer(p)) = def.real.as_ref().map(|r| &r.source) {
                let path = dir.join(&p.file);
                if !is_json(&p.file) && path.is_file() {
                    by_file
                        .entry(path)
                        .or_default()
                        .push((def.band.as_str(), &**p));
                }
            }
        }
    }
    let mut out = BTreeMap::new();
    for (path, bands) in &by_file {
        let pointers: Vec<&Pointer> = bands.iter().map(|(_, p)| *p).collect();
        for ((band, _), value) in bands.iter().zip(read_csv_all(path, &pointers)) {
            out.insert(*band, value);
        }
    }
    out
}

/// Why a whole-sample reduction gives no single value.
const WHOLE_SAMPLE: &str = "a whole sample is read by the distribution test, not as one value";

/// The real samples of the distribution bands, by band: the whole sample, or why it is not
/// here (the path and the reason).
pub type Samples = BTreeMap<String, Result<Sample, String>>;

/// What one distribution band's reader does with each row of its file.
enum Take {
    /// A histogram of the key: the signed sum of columns, or a point measured to `goal`.
    Histogram(Key, Option<[f64; 2]>),
    /// A cells table of the score in two columns, each side capped at `max`.
    Cells(usize, usize, u32),
    /// Possession chains of the `chain` column: passes counted by the key's tokens, or
    /// seconds from the `time` column with the key as the duration, within one `period`.
    Chains {
        chain: usize,
        count: ChainCount,
        key: usize,
        tokens: BTreeSet<String>,
        time: Option<usize>,
        period: Option<usize>,
    },
}

/// One chain as its rows arrive: its first time and period, the end of its last row in
/// that period, and its counted rows.
#[derive(Default)]
struct ChainAcc {
    start: f64,
    period: String,
    end: f64,
    passes: u32,
}

/// One distribution band reading its file: its rules, and what it has read so far.
struct Reader {
    band: String,
    width: f64,
    bins: u32,
    take: Take,
    filter: Vec<(usize, Vec<String>)>,
    ranges: Vec<(usize, [f64; 2])>,
    same: Option<(usize, usize)>,
    group: Vec<usize>,
    skip_blank: bool,
    /// Each match's histogram, by its group value.
    hists: BTreeMap<String, BTreeMap<u32, f64>>,
    chains: BTreeMap<(String, String), ChainAcc>,
    error: Option<String>,
}

/// A time cell: seconds, or `HH:MM:SS.mmm`.
fn seconds(text: &str) -> Option<f64> {
    let text = text.trim();
    if let Ok(v) = text.parse::<f64>() {
        return Some(v);
    }
    text.split(':').try_fold(0.0, |acc, part| {
        Some(acc * 60.0 + part.parse::<f64>().ok()?)
    })
}

impl Reader {
    /// A reader of `def` over a file with the columns `names`; `Err` names what the pointer
    /// lacks or the file does not hold.
    fn new(def: &BandDef, p: &Pointer, names: &[&str]) -> Result<Self, String> {
        let column = |name: &str| {
            names
                .iter()
                .position(|n| *n == name)
                .ok_or_else(|| format!("no column {name}"))
        };
        let (width, bins) = super::measures::hist_bins(&def.measure)
            .ok_or_else(|| "the band is not a whole distribution".to_string())?;
        let reduce = p.reduce.ok_or_else(|| {
            "a whole distribution needs reduce histogram, cells or chains".to_string()
        })?;
        let need = |what: &str| format!("a {} needs {what}", reduce_name(reduce));
        let pair = || -> Result<(usize, usize), String> {
            let (a, b) = p
                .key
                .split_once(':')
                .ok_or_else(|| need("its key as two columns joined by :"))?;
            Ok((column(a)?, column(b)?))
        };
        let take = match (reduce, &def.measure) {
            (Reduce::Histogram, Measure::Distribution { .. }) => {
                if p.key.contains(':') {
                    let (a, b) = pair()?;
                    Take::Histogram(Key::Pair(a, b), Some(p.goal.ok_or_else(|| need("goal"))?))
                } else {
                    Take::Histogram(Key::Sum(signed_columns(&p.key, &column)?), None)
                }
            }
            (Reduce::Cells, Measure::Cells { max }) => {
                let (a, b) = pair()?;
                Take::Cells(a, b, *max)
            }
            (Reduce::Chains, Measure::Distribution { .. }) => {
                let count = p.count.ok_or_else(|| need("count (passes or seconds)"))?;
                let chain = column(p.chain.as_deref().ok_or_else(|| need("chain"))?)?;
                let (time, period) = match count {
                    ChainCount::Passes => (None, None),
                    ChainCount::Seconds => (
                        Some(column(p.time.as_deref().ok_or_else(|| need("time"))?)?),
                        Some(column(p.period.as_deref().ok_or_else(|| need("period"))?)?),
                    ),
                };
                let tokens: BTreeSet<String> = p.tokens.iter().flatten().cloned().collect();
                if count == ChainCount::Passes && tokens.is_empty() {
                    return Err(need("tokens"));
                }
                Take::Chains {
                    chain,
                    count,
                    key: column(&p.key)?,
                    tokens,
                    time,
                    period,
                }
            }
            (r, _) => {
                return Err(format!(
                    "reduce {} does not read this band's kind",
                    reduce_name(r)
                ));
            }
        };
        let group = match &p.group_by {
            Some(g) => g.split('+').map(column).collect::<Result<_, _>>()?,
            None => return Err(need("group_by")),
        };
        Ok(Reader {
            band: def.band.clone(),
            width,
            bins,
            take,
            filter: p
                .filter
                .iter()
                .flatten()
                .map(|(c, vals)| column(c).map(|i| (i, vals.clone())))
                .collect::<Result<_, _>>()?,
            ranges: p
                .range
                .iter()
                .flatten()
                .map(|(c, r)| column(c).map(|i| (i, *r)))
                .collect::<Result<_, _>>()?,
            same: match &p.same {
                Some([a, b]) => Some((column(a)?, column(b)?)),
                None => None,
            },
            group,
            skip_blank: p.skip_blank,
            hists: BTreeMap::new(),
            chains: BTreeMap::new(),
            error: None,
        })
    }

    /// Reads one row, `n` its line number.
    fn row(&mut self, cells: &[&str], names: &[&str], n: usize) -> Result<(), String> {
        let cell = |i: usize| {
            cells
                .get(i)
                .copied()
                .ok_or_else(|| format!("line {n} has no column {}", names[i]))
        };
        let number = |i: usize| -> Result<f64, String> {
            let text = cell(i)?;
            text.trim()
                .parse::<f64>()
                .map_err(|_| format!("line {n} column {}: {text} is not a number", names[i]))
        };
        let group = self
            .group
            .iter()
            .map(|&i| cell(i))
            .collect::<Result<Vec<_>, _>>()?
            .join("+");
        // A chain is every row of its possession; the filters choose the rows it counts.
        let mut kept = true;
        for (i, vals) in &self.filter {
            kept &= vals.iter().any(|v| v == cell(*i).unwrap_or(""));
        }
        if let Some((a, b)) = self.same {
            kept &= cell(a)? == cell(b)?;
        }
        for &(i, [lo, hi]) in &self.ranges {
            if !kept {
                break;
            }
            let v = number(i)?;
            kept &= lo <= v && v < hi;
        }
        let (width, bins) = (self.width, self.bins);
        match &self.take {
            Take::Chains {
                chain,
                count,
                key,
                tokens,
                time,
                period,
            } => {
                let id = (group, cell(*chain)?.to_string());
                let fresh = !self.chains.contains_key(&id);
                let acc = self.chains.entry(id).or_default();
                match (count, time, period) {
                    (ChainCount::Seconds, Some(t), Some(p)) => {
                        let at = seconds(cell(*t)?)
                            .ok_or_else(|| format!("line {n} column {}: not a time", names[*t]))?;
                        let dur_text = cell(*key)?.trim();
                        let dur = if dur_text.is_empty() {
                            0.0
                        } else {
                            number(*key)?
                        };
                        let per = cell(*p)?;
                        if fresh {
                            acc.start = at;
                            acc.period = per.to_string();
                        }
                        if per == acc.period {
                            acc.end = at + dur;
                        }
                    }
                    _ => {
                        let hit = cell(*key)?.split_whitespace().any(|t| tokens.contains(t));
                        acc.passes += u32::from(kept && hit);
                    }
                }
                return Ok(());
            }
            _ if !kept => return Ok(()),
            Take::Cells(a, b, max) => {
                let (h, w) = (number(*a)?, number(*b)?);
                if h < 0.0 || w < 0.0 {
                    return Err(format!("line {n}: a score below 0"));
                }
                // A score of whole goals, capped at `max`.
                let k = super::measures::cell(h as u32, w as u32, *max);
                *self.hists.entry(group).or_default().entry(k).or_default() += 1.0;
            }
            Take::Histogram(key, goal) => {
                if self.skip_blank {
                    let used = match key {
                        Key::Sum(cols) => cols.iter().map(|c| c.0).collect(),
                        Key::Pair(a, b) => vec![*a, *b],
                        Key::Tokens(i) => vec![*i],
                    };
                    if used
                        .into_iter()
                        .any(|i| cell(i).is_ok_and(|t| t.trim().is_empty()))
                    {
                        return Ok(());
                    }
                }
                let x = match (key, goal) {
                    (Key::Pair(a, b), Some([gx, gy])) => (gx - number(*a)?).hypot(gy - number(*b)?),
                    (Key::Sum(cols), _) => cols
                        .iter()
                        .try_fold(0.0, |acc, &(i, s)| Ok::<_, String>(acc + s * number(i)?))?,
                    _ => return Err("a histogram key is columns or a point".into()),
                };
                let k = bin_of(x, width, bins);
                *self.hists.entry(group).or_default().entry(k).or_default() += 1.0;
            }
        }
        Ok(())
    }

    /// The whole sample: one histogram per match, in the order of their group values.
    fn finish(mut self) -> Result<Sample, String> {
        if let Some(e) = self.error {
            return Err(e);
        }
        if let Take::Chains { count, .. } = &self.take {
            for ((group, _), acc) in std::mem::take(&mut self.chains) {
                let v = match count {
                    ChainCount::Passes => f64::from(acc.passes),
                    ChainCount::Seconds => (acc.end - acc.start).max(0.0),
                };
                let k = bin_of(v, self.width, self.bins);
                *self.hists.entry(group).or_default().entry(k).or_default() += 1.0;
            }
        }
        if self.hists.is_empty() {
            return Err("no row kept".into());
        }
        Ok(Sample {
            per_match: self
                .hists
                .into_values()
                .map(|h| h.into_iter().collect())
                .collect(),
            bins: self.bins,
            width: self.width,
        })
    }
}

/// Reads the real sample of every distribution band of `registry` from `folder`: each file
/// once, streamed line by line, even when several bands point into it. A band whose folder,
/// file, columns or rows are not there is `Err` with the path and the reason, and the others
/// read on.
pub fn samples(folder: &RealFolder, registry: &Registry) -> Samples {
    let mut out = Samples::new();
    let mut by_file: BTreeMap<&str, Vec<(&BandDef, &Pointer)>> = BTreeMap::new();
    for def in registry.bands.iter().filter(|d| d.is_distribution()) {
        match def.real.as_ref().map(|r| &r.source) {
            Some(RealSource::Pointer(p)) => by_file.entry(&p.file).or_default().push((def, p)),
            Some(RealSource::NoCounterpart(n)) => {
                out.insert(
                    def.band.clone(),
                    Err(format!("no real counterpart: {}", n.none)),
                );
            }
            Some(RealSource::Target(t)) => {
                let got = match registry.targets.get(&t.target) {
                    Some(target) => target_sample(folder, def, &t.target, target),
                    None => Err(format!(
                        "{} is not a target of the reference file read with the band file",
                        t.target
                    )),
                };
                out.insert(def.band.clone(), got);
            }
            None => {
                out.insert(def.band.clone(), Err("no real record".into()));
            }
        }
    }
    for (file, bands) in by_file {
        let read = read_file(folder, file, &bands);
        for (def, _) in &bands {
            let got = match &read {
                Ok(map) => map
                    .get(&def.band)
                    .cloned()
                    .unwrap_or_else(|| Err("not read".into())),
                Err(e) => Err(e.clone()),
            };
            out.insert(def.band.clone(), got);
        }
    }
    out
}

/// The raw-data folder beside a real-data folder: the folder itself when it holds the
/// football-data files, otherwise its sibling `data` (the main checkout's `.scratch/data`
/// beside `.scratch/out`).
fn data_folder(folder: &RealFolder) -> Option<PathBuf> {
    let dir = folder.path.as_ref()?;
    if dir.join("football-data").is_dir() {
        return Some(dir.clone());
    }
    dir.parent().map(|p| p.join("data"))
}

/// The whole real sample of a distribution band whose real side is a distribution target
/// of the reference file (`<layer>.dist.<kind>`): every match of the target's leagues and
/// seasons read in place from the football-data files, binned as the target was built. Each
/// league weighs the same, as the target pools its leagues, so the sample's pooled shares
/// are the target's: each match counts `n / (leagues x matches of its league)`.
fn target_sample(
    folder: &RealFolder,
    def: &BandDef,
    key: &str,
    target: &crate::real_targets::Target,
) -> Result<Sample, String> {
    use crate::real_targets::football_data::{DISTS, match_rows};
    let kind = key
        .rsplit_once(".dist.")
        .map(|(_, k)| k)
        .ok_or_else(|| format!("{key} is not a distribution target"))?;
    let (width, bins) = super::measures::hist_bins(&def.measure)
        .ok_or_else(|| "the band is not a whole distribution".to_string())?;
    if target.bins != Some(bins) || target.width != Some(width) {
        return Err(format!(
            "{key} holds {:?} bins of {:?}; the band has {bins} of {width}",
            target.bins, target.width
        ));
    }
    if key.starts_with("sb.") {
        return statsbomb_sample(folder, key, target, kind, (width, bins));
    }
    let dist = DISTS
        .iter()
        .find(|d| d.name == kind)
        .ok_or_else(|| format!("{key}: no reader of the distribution {kind}"))?;
    let data = data_folder(folder).ok_or_else(|| {
        "no real-data folder: the content folder is in no git checkout; pass --real-data"
            .to_string()
    })?;
    let leagues: Vec<&str> = target
        .leagues
        .iter()
        .flatten()
        .map(String::as_str)
        .collect();
    let seasons: Vec<&str> = target.seasons.iter().map(String::as_str).collect();
    let read = match_rows(&data, &leagues, &seasons).map_err(|e| format!("{e:#}"))?;
    if !read.missing.is_empty() {
        return Err(format!(
            "{}: {} result files are not here, such as {}",
            data.display(),
            read.missing.len(),
            read.missing[0]
        ));
    }
    let mut per_league: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
    for row in &read.rows {
        if let Some(k) = (dist.bin)(row, bins, width) {
            per_league.entry(row.div.as_str()).or_default().push(k);
        }
    }
    let total: usize = per_league.values().map(Vec::len).sum();
    if per_league.is_empty() || total < 2 {
        return Err(format!("{key}: no match of its leagues holds the columns"));
    }
    let groups = per_league.len() as f64;
    let mut per_match = Vec::with_capacity(total);
    for matches in per_league.values() {
        let weight = total as f64 / (groups * matches.len() as f64);
        per_match.extend(matches.iter().map(|&k| vec![(k, weight)]));
    }
    Ok(Sample {
        per_match,
        bins,
        width,
    })
}

/// The local side folder beside a real-data folder: `.scratch/real-targets` beside the raw
/// data in `.scratch/data`, where `engine-cli real-targets build` writes it.
pub fn side_folder(folder: &RealFolder) -> Option<PathBuf> {
    data_folder(folder)?
        .parent()
        .map(|p| p.join(crate::real_targets::side::DIR))
}

/// The whole real sample of a StatsBomb distribution target (`sb.<label>.dist.<kind>`):
/// each match of the target's competition-seasons one histogram, read from the per-match
/// histogram file of the side folder, each match weighing by its own events as the target
/// pools them. The file must hold every season of the target and give the target's shares;
/// otherwise the band has no real sample and the reason says to rebuild.
fn statsbomb_sample(
    folder: &RealFolder,
    key: &str,
    target: &crate::real_targets::Target,
    kind: &str,
    (width, bins): (f64, u32),
) -> Result<Sample, String> {
    use crate::real_targets::side::HISTOGRAM_FILE;
    let rebuild = "run engine-cli real-targets build";
    let dir = side_folder(folder).ok_or_else(|| {
        format!(
            "per-match StatsBomb histograms absent: the content folder is in no git checkout; \
             {rebuild}"
        )
    })?;
    let path = dir.join(HISTOGRAM_FILE);
    let text = std::fs::read_to_string(&path).map_err(|_| {
        format!(
            "per-match StatsBomb histograms absent at {}: {rebuild}",
            path.display()
        )
    })?;
    let labels: BTreeSet<&str> = target.seasons.iter().map(String::as_str).collect();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut per: BTreeMap<(String, u64), BTreeMap<u32, f64>> = BTreeMap::new();
    for (n, line) in text.lines().enumerate().skip(1) {
        let cells: Vec<&str> = line.split(',').collect();
        let [label, id, k, bin, count] = cells[..] else {
            return Err(format!("{} line {}: not five cells", path.display(), n + 1));
        };
        if k != kind || !labels.contains(label) {
            continue;
        }
        let bad = || format!("{} line {}: not a number", path.display(), n + 1);
        let id: u64 = id.parse().map_err(|_| bad())?;
        let bin: u32 = bin.parse().map_err(|_| bad())?;
        let count: f64 = count.parse().map_err(|_| bad())?;
        if bin >= bins {
            return Err(format!(
                "{} line {}: bin {bin} of {kind} is beyond the band's {bins} bins",
                path.display(),
                n + 1
            ));
        }
        seen.insert(label.to_string());
        *per.entry((label.to_string(), id))
            .or_default()
            .entry(bin)
            .or_default() += count;
    }
    if seen.len() < labels.len() || per.len() as u64 > target.n {
        return Err(format!(
            "{} is short for {key}: it holds {} of the target's {} competition-seasons and {} \
             of its {} matches; {rebuild}",
            path.display(),
            seen.len(),
            labels.len(),
            per.len(),
            target.n
        ));
    }
    let mut pooled = vec![0.0; bins as usize];
    for h in per.values() {
        for (b, c) in h {
            pooled[*b as usize] += c;
        }
    }
    let total: f64 = pooled.iter().sum();
    let agrees = target.shares.as_ref().is_some_and(|want| {
        total > 0.0
            && want.len() == pooled.len()
            && want
                .iter()
                .zip(&pooled)
                .all(|(w, c)| (w - c / total).abs() < 1e-5)
    });
    if !agrees {
        return Err(format!(
            "{} does not give the shares of {key}: the reference file and the side folder \
             were built apart; {rebuild}",
            path.display()
        ));
    }
    Ok(Sample {
        per_match: per.into_values().map(|h| h.into_iter().collect()).collect(),
        bins,
        width,
    })
}

/// The samples of `bands` from one file of `folder`; `Err` when the folder or the file is
/// not there, or its header cannot be read.
fn read_file(
    folder: &RealFolder,
    file: &str,
    bands: &[(&BandDef, &Pointer)],
) -> Result<BTreeMap<String, Result<Sample, String>>, String> {
    let Some(dir) = &folder.path else {
        return Err(
            "no real-data folder: the content folder is in no git checkout; pass --real-data"
                .into(),
        );
    };
    if !dir.is_dir() {
        return Err(format!("{}: the folder does not exist", dir.display()));
    }
    let path = dir.join(file);
    let at = |why: String| format!("{}: {why}", path.display());
    if !path.is_file() {
        return Err(at("the file does not exist".into()));
    }
    let mut rows = lines(&path).map_err(at)?.lines();
    let header = match rows.next() {
        Some(line) => line.map_err(|e| at(format!("cannot read: {e}")))?,
        None => return Err(at("the file is empty".into())),
    };
    let names: Vec<&str> = header.trim_end_matches('\r').split(',').collect();
    let mut out = BTreeMap::new();
    let mut readers = Vec::new();
    for (def, p) in bands {
        match Reader::new(def, p, &names) {
            Ok(r) => readers.push(r),
            Err(why) => {
                out.insert(def.band.clone(), Err(at(why)));
            }
        }
    }
    for (n, line) in rows.enumerate() {
        let line = line.map_err(|e| at(format!("cannot read line {}: {e}", n + 2)))?;
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if line.contains('"') {
            return Err(at(format!(
                "line {} holds a quoted field; this reader takes none",
                n + 2
            )));
        }
        let cells: Vec<&str> = line.split(',').collect();
        for r in readers.iter_mut().filter(|r| r.error.is_none()) {
            if let Err(e) = r.row(&cells, &names, n + 2) {
                r.error = Some(e);
            }
        }
    }
    for r in readers {
        let band = r.band.clone();
        out.insert(band, r.finish().map_err(at));
    }
    Ok(out)
}

/// The value `calib.real` shows for a whole sample, which has no single value: the mean of
/// its points (each bin at `k · width`; for a cells table, home plus away goals as capped)
/// and its standard error over matches, as a pooled ratio.
pub fn sample_mean(def: &BandDef, s: &Sample) -> (f64, f64) {
    let point = |k: u32| match &def.measure {
        Measure::Cells { max } => f64::from(k / (max + 1) + k % (max + 1)),
        _ => f64::from(k) * s.width,
    };
    let (ys, xs): (Vec<f64>, Vec<f64>) = s
        .per_match
        .iter()
        .map(|h| {
            h.iter()
                .fold((0.0, 0.0), |(y, x), &(k, n)| (y + n * point(k), x + n))
        })
        .unzip();
    let total: f64 = xs.iter().sum();
    let mean = if total > 0.0 {
        ys.iter().sum::<f64>() / total
    } else {
        0.0
    };
    (mean, se_ratio(&ys, &xs))
}

/// The name of a reduction, as the band file writes it.
fn reduce_name(r: Reduce) -> String {
    serde_json::to_value(r)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.replace('_', " ")))
        .unwrap_or_default()
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
pub(crate) fn civil_date(days: i64) -> String {
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
    /// The pass rule's real side: per-unit terms, tracked values, or the value and its
    /// error. Judging reads it; the record does not hold it.
    #[serde(skip)]
    pub side: Option<Real>,
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

/// Reads every band's real value from `folder`; a distribution band's comes from its whole
/// sample in `samples`, read once for the judgement.
#[cfg(test)]
pub fn read_all(folder: &RealFolder, registry: &Registry, samples: &Samples) -> RealInfo {
    read_all_with(folder, registry, samples, None)
}

/// [`read_all`], with the reference file of real targets for the bands that name a target.
/// Each band's entry also carries its real side for the pass rule.
pub fn read_all_with(
    folder: &RealFolder,
    registry: &Registry,
    samples: &Samples,
    reference: Option<&ReferenceFile>,
) -> RealInfo {
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
    let mut csv = csv_values(folder, registry);
    for def in &registry.bands {
        let whole = def.is_distribution().then(|| samples.get(&def.band));
        let mut side = None;
        let outcome = match whole {
            Some(Some(Ok(sample))) => {
                let (value, se) = sample_mean(def, sample);
                Outcome::Read {
                    value,
                    se: Some(se),
                    file_date: def.real.as_ref().and_then(|r| match &r.source {
                        RealSource::Pointer(p) => folder
                            .path
                            .as_ref()
                            .and_then(|d| modified_date(&d.join(&p.file)))
                            .filter(|d| *d > r.date),
                        RealSource::NoCounterpart(_) | RealSource::Target(_) => None,
                    }),
                }
            }
            Some(Some(Err(why)))
                if def
                    .real
                    .as_ref()
                    .is_some_and(|r| r.no_counterpart().is_none()) =>
            {
                Outcome::NotHere {
                    path: String::new(),
                    reason: why.clone(),
                }
            }
            Some(None) => Outcome::NotHere {
                path: String::new(),
                reason: "the whole sample was not read".into(),
            },
            _ => {
                let (outcome, s) = read_with(folder, def, csv.remove(def.band.as_str()), reference);
                side = s;
                outcome
            }
        };
        let matches = match whole {
            Some(Some(Ok(sample))) => Some(sample.per_match.len() as u64),
            _ => None,
        };
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
                count: matches.or(def.confidence.map(|c| c.measurement.count)),
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
            side,
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
    use crate::report::distribution::Hist;
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
            weight: None,
            range: None,
            same: None,
            bins: None,
            goal: None,
            scores: None,
            window: None,
            skip_blank: false,
            chain: None,
            time: None,
            period: None,
            count: None,
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
            rule: None,
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

    /// One pass over a file reads every pointer into it as each reads alone, a pointer that
    /// does not fit the file included.
    #[test]
    fn one_pass_reads_every_pointer_into_a_file_as_each_alone() {
        let dir = temp("one-pass");
        let path = dir.join("m.csv");
        std::fs::write(
            &path,
            "comp,hg,ag
A,1,0
B,3,2
A,0,0
",
        )
        .unwrap();
        let mean = pointer("m.csv", "hg+ag");
        let share = Pointer {
            reduce: Some(Reduce::Share),
            op: Some(super::super::bands::Op::AtLeast),
            value: Some(1.0),
            ..pointer("m.csv", "hg")
        };
        let only_a = Pointer {
            filter: Some(BTreeMap::from([(
                "comp".to_string(),
                vec!["A".to_string()],
            )])),
            ..pointer("m.csv", "hg")
        };
        let broken = pointer("m.csv", "no_such_column");
        let all = read_csv_all(&path, &[&mean, &share, &only_a, &broken]);
        for (p, got) in [&mean, &share, &only_a, &broken].into_iter().zip(&all) {
            assert_eq!(*got, read_csv(&path, p));
        }
        assert_eq!(all[0].as_ref().unwrap().value, 2.0);
        assert!(all[3].as_ref().unwrap_err().contains("no_such_column"));
        let _ = std::fs::remove_dir_all(&dir);
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
        let (v, se) = value(read(&f, &band(RealSource::Pointer(Box::new(p)), 10, 1.0)));
        assert_eq!(v, 11.0);
        assert!((se.unwrap() - se_mean(&[20.0, 24.0]) * 0.5).abs() < 1e-12);
        // A ratio: no error.
        let mut p = pointer("sb.json", "s.g");
        p.den = Some("s.xg".into());
        assert_eq!(
            value(read(&f, &band(RealSource::Pointer(Box::new(p)), 10, 1.0))),
            (1.5, None)
        );
        // A JSON share: its error over the sample.
        let mut p = pointer("sb.json", "s.on");
        p.reduce = Some(Reduce::Share);
        let (v, se) = value(read(&f, &band(RealSource::Pointer(Box::new(p)), 300, 1.0)));
        assert_eq!((v, se), (0.25, Some(se_share(0.25, 300))));
        // A CSV mean of summed columns over the kept rows.
        let mut p = pointer("m.csv", "hg+ag");
        p.reduce = Some(Reduce::Mean);
        p.filter = Some(BTreeMap::from([("comp".into(), vec!["X".into()])]));
        let (v, se) = value(read(&f, &band(RealSource::Pointer(Box::new(p)), 3, 1.0)));
        assert_eq!(v, 7.0 / 3.0);
        assert_eq!(se, Some(se_mean(&[3.0, 0.0, 4.0])));
        // A CSV share.
        let mut p = pointer("m.csv", "hg+ag");
        p.reduce = Some(Reduce::Share);
        p.op = Some(crate::report::bands::Op::Equal);
        p.value = Some(0.0);
        let (v, se) = value(read(&f, &band(RealSource::Pointer(Box::new(p)), 4, 1.0)));
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
        let (v, se) = value(read(
            &at(&dir),
            &band(RealSource::Pointer(Box::new(p)), 3, 1.0),
        ));
        assert_eq!(v, 2.0 / 3.0);
        assert_eq!(se, Some(se_share(2.0 / 3.0, 3)));
        // A corrupt stream is not here, by name.
        std::fs::write(dir.join("bad.csv.gz"), b"not gzip at all").unwrap();
        let mut p = pointer("bad.csv.gz", "tags");
        p.reduce = Some(Reduce::GroupShare);
        p.group_by = Some("match".into());
        p.tokens = Some(vec!["1701".into()]);
        assert!(matches!(
            read(&at(&dir), &band(RealSource::Pointer(Box::new(p)), 3, 1.0)),
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
        let not_here = |p: Pointer, f: &RealFolder| match read(
            f,
            &band(RealSource::Pointer(Box::new(p)), 5, 1.0),
        ) {
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
            outside_by: None,
            real: None,
            distance: None,
            upper: None,
            noise_floor: None,
            report_only: None,
            real_note: None,
            judged: None,
            note: None,
            accepted: None,
            pass_rule: None,
            old_word: None,
            old_reason: None,
            moved: None,
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
    fn a_band_of_a_report_only_group_prints_report_only_and_leaves_the_joint_word() {
        let thin = serde_json::json!({"competitions": 1, "seasons": 1, "clubs": 20});
        // A thin real sample alone no longer makes a band report only: a band reports only
        // when it belongs to a group the band file names.
        let r = registry(&[
            ("corners_per_team", "confidence.generalisation", thin),
            ("corners_per_team", "gates", serde_json::json!(false)),
            ("corners_per_team", "group", serde_json::json!("facing")),
        ]);
        // The record shows the counts, the error and the words.
        let info = read_all(
            &at(&std::env::temp_dir().join("engine-cli-real-absent")),
            &r,
            &Samples::new(),
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
                .contains("body facing"),
            "{:?}",
            corners.report_only
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
        assert_eq!(
            (info.missing, info.no_counterpart),
            // The 101 team-shape bands read a file this folder lacks, and the shots and cards
            // distributions a reference file it lacks; the three experiment bands and the five
            // bands only the slow tests judge have no real counterpart.
            (15 + 35 + 101 + 2, 3 + 5)
        );

        let mut joint = Joint {
            word: Word::Fail,
            critical: 2.5,
            resamples: 1999,
            false_alarm: 0.03,
            pairs: 10,
            guards: Vec::new(),
            reason: None,
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

    /// The value and error `p` reads from `dir`.
    fn read_at(dir: &Path, p: Pointer) -> (f64, Option<f64>) {
        value(read(
            &at(dir),
            &band(RealSource::Pointer(Box::new(p)), 10, 1.0),
        ))
    }

    #[test]
    fn each_new_csv_reduction_reads_its_hand_value() {
        let dir = temp("reductions");
        std::fs::write(
            dir.join("m.csv"),
            "comp,hg,ag\nX,5,1\nX,1,5\nX,4,1\nX,0,0\nY,5,1\n",
        )
        .unwrap();
        // Signed key terms: 4, -4, 3, 0 over competition X.
        let mut p = pointer("m.csv", "hg-ag");
        p.filter = Some(BTreeMap::from([("comp".into(), vec!["X".into()])]));
        assert_eq!(
            read_at(&dir, p.clone()),
            (0.75, Some(se_mean(&[4.0, -4.0, 3.0, 0.0])))
        );
        // A scoreline share, either way round.
        p.key = "hg:ag".into();
        p.reduce = Some(Reduce::Scoreline);
        p.scores = Some(vec!["5-1".into()]);
        assert_eq!(read_at(&dir, p.clone()), (0.5, Some(se_share(0.5, 4))));
        // A factorial ratio over the home goals 5, 1, 4, 0: 4 * (20 + 0 + 12 + 0) / 10^2.
        let mut p = pointer("m.csv", "hg");
        p.filter = Some(BTreeMap::from([("comp".into(), vec!["X".into()])]));
        p.reduce = Some(Reduce::FactorialRatio);
        assert_eq!(read_at(&dir, p), (1.28, None));

        std::fs::write(
            dir.join("e.csv"),
            "type,team,poss,x,y,dur,len\nPass,A,A,10,34,2,5\nPass,A,B,90,34,4,12\n\
             Carry,A,A,50,34,6,30\nPass,B,B,80,30,,8\nShot,A,A,95,34,,\n",
        )
        .unwrap();
        // A bin share weighted by duration, of the possession team's rows: 2 of 2 + 6 in
        // the own third; the blank duration is skipped.
        let mut p = pointer("e.csv", "x");
        p.reduce = Some(Reduce::BinShare);
        p.bins = Some([0.0, 35.0]);
        p.weight = Some("dur".into());
        p.same = Some(["team".into(), "poss".into()]);
        p.filter = Some(BTreeMap::from([(
            "type".into(),
            vec!["Pass".into(), "Carry".into()],
        )]));
        p.skip_blank = true;
        let (v, se) = read_at(&dir, p.clone());
        assert_eq!(v, 0.25);
        assert_eq!(se, Some(se_ratio(&[2.0, 0.0], &[2.0, 6.0])));
        // Without skip_blank, the blank weight is not a number, by line.
        p.skip_blank = false;
        let why = match read(&at(&dir), &band(RealSource::Pointer(Box::new(p)), 10, 1.0)) {
            Outcome::NotHere { reason, .. } => reason,
            other => panic!("{other:?}"),
        };
        assert!(why.contains("line 5 column dur"), "{why}");
        // A goal distance from the shot at (95, 34) to (105, 34).
        let mut p = pointer("e.csv", "x:y");
        p.reduce = Some(Reduce::GoalDistance);
        p.goal = Some([105.0, 34.0]);
        p.filter = Some(BTreeMap::from([("type".into(), vec!["Shot".into()])]));
        assert_eq!(read_at(&dir, p), (10.0, Some(0.0)));
        // A range on a numeric column, half-open: lengths 10 to 25 keep the 12 m pass only.
        let mut p = pointer("e.csv", "x");
        p.range = Some(BTreeMap::from([("len".into(), [10.0, 25.0])]));
        p.filter = Some(BTreeMap::from([(
            "type".into(),
            vec!["Pass".into(), "Carry".into()],
        )]));
        assert_eq!(read_at(&dir, p).0, 90.0);
        // A weighted mean and a ratio of sums.
        let mut p = pointer("e.csv", "x");
        p.filter = Some(BTreeMap::from([("team".into(), vec!["A".into()])]));
        p.filter
            .as_mut()
            .unwrap()
            .insert("type".into(), vec!["Pass".into(), "Carry".into()]);
        p.weight = Some("dur".into());
        assert_eq!(read_at(&dir, p.clone()).0, (20.0 + 360.0 + 300.0) / 12.0);
        p.weight = None;
        p.den = Some("dur".into());
        assert_eq!(read_at(&dir, p), ((10.0 + 90.0 + 50.0) / 12.0, None));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_group_range_and_a_group_max_reduce_per_group() {
        let dir = temp("groups");
        std::fs::write(
            dir.join("t.csv"),
            "team,goals,conceded,mid,name,top\nA,3,1,m1,p,8.5\nB,1,3,m1,q,7.0\nA,0,1,m1,p,9.0\n\
             B,1,0,m2,q,7.5\nA,3,0,m2,p,8.0\nB,0,3,m2,r,6.0\nA,0,0,m3,p,1.0\n",
        )
        .unwrap();
        // The first 3 goal differences of A: +2, -1, +3, range 4; of B: -2, +1, -3, range 4.
        let mut p = pointer("t.csv", "goals-conceded");
        p.reduce = Some(Reduce::GroupRange);
        p.group_by = Some("team".into());
        p.window = Some(3);
        assert_eq!(read_at(&dir, p.clone()), (4.0, Some(0.0)));
        p.window = Some(5);
        let why = match read(&at(&dir), &band(RealSource::Pointer(Box::new(p)), 10, 1.0)) {
            Outcome::NotHere { reason, .. } => reason,
            other => panic!("{other:?}"),
        };
        assert!(why.contains("no group has enough rows"), "{why}");
        // The largest top speed per match and player: m1+p 9, m1+q 7, m2+q 7.5, m2+p 8,
        // m2+r 6, m3+p 1.
        let mut p = pointer("t.csv", "top");
        p.reduce = Some(Reduce::GroupMax);
        p.group_by = Some("mid+name".into());
        let maxes = [9.0, 7.0, 8.0, 7.5, 6.0, 1.0];
        let (v, se) = read_at(&dir, p);
        assert!((v - maxes.iter().sum::<f64>() / 6.0).abs() < 1e-12);
        assert!((se.unwrap() - se_mean(&maxes)).abs() < 1e-12);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_keys_joined_by_plus_are_summed() {
        let dir = temp("json-sum");
        std::fs::write(
            dir.join("g.json"),
            r#"{"goal_origin": {"Regular Play": 0.3, "From Counter": 0.06, "Other": 0.01, "Penalty": 0.08}}"#,
        )
        .unwrap();
        let mut p = pointer(
            "g.json",
            "goal_origin.Regular Play+goal_origin.From Counter+goal_origin.Other",
        );
        p.reduce = Some(Reduce::Share);
        let (v, se) = value(read(
            &at(&dir),
            &band(RealSource::Pointer(Box::new(p)), 600, 1.0),
        ));
        assert!((v - 0.37).abs() < 1e-12);
        assert!((se.unwrap() - se_share(v, 600)).abs() < 1e-12);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn gz(path: &Path, text: &str) {
        let file = File::create(path).unwrap();
        let mut gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        gz.write_all(text.as_bytes()).unwrap();
        gz.finish().unwrap();
    }

    /// The shipped distribution bands' pointers over small synthetic files: histograms
    /// grouped by match, cells from a score, shot distances from a compressed file, and
    /// possession chains in passes and seconds from one compressed file read once.
    #[test]
    fn each_whole_sample_reads_one_histogram_per_match() {
        let dir = temp("samples");
        std::fs::write(
            dir.join("wy2_matches.csv"),
            "comp,match,hg,ag\nEngland,1,2,1\nFrance,2,0,0\nGermany,3,6,6\nOther,4,9,9\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("wy2_team_matches.csv"),
            "match,team,passes\n1,10,400\n1,11,500\n2,12,1600\n",
        )
        .unwrap();
        gz(
            &dir.join("sb_events.csv.gz"),
            "mid,type,x,y\n7,Shot,93,34\n7,Pass,10,10\n8,Shot,105,0\n8,Shot,50,34\n",
        );
        gz(
            &dir.join("sb2_events.csv.gz"),
            "mid,period,ts,type,team,poss,poss_team,dur\n\
             5,1,00:00:00.000,Starting XI,A,1,A,0.0\n\
             5,1,00:00:01.000,Pass,A,1,A,1.5\n\
             5,1,00:00:03.000,Pass,A,1,A,2.0\n\
             5,1,00:00:05.500,Pass,B,1,A,1.0\n\
             5,1,00:00:06.000,Pass,A,1,A,0.5\n\
             5,1,00:00:10.000,Carry,B,2,B,4.0\n\
             6,1,00:44:50.000,Pass,A,1,A,1.0\n\
             6,2,00:00:05.000,Pass,A,1,A,\n",
        );
        let r = registry(&[]);
        let got = samples(&at(&dir), &r);
        // The shots and cards distributions read reference-file targets, which this
        // registry, read without the reference file, does not hold.
        assert_eq!(got.len(), 8, "{got:?}");
        for band in ["shots_total_distribution", "cards_total_distribution"] {
            let why = got[band].as_ref().unwrap_err();
            assert!(why.contains("is not a target"), "{band}: {why}");
        }
        let hist = |band: &str| -> Vec<Hist> {
            got[band]
                .as_ref()
                .unwrap_or_else(|e| panic!("{band}: {e}"))
                .per_match
                .clone()
        };
        // Matches 1, 2 and 3 of the five leagues: 3 goals, 0, and 12 as "10 or more".
        assert_eq!(
            hist("goals_distribution"),
            [vec![(3, 1.0)], vec![(0, 1.0)], vec![(10, 1.0)]]
        );
        // 2-1 is cell 2 * 6 + 1; 6-6 is capped to 5-5, cell 35.
        assert_eq!(
            hist("scoreline_cells"),
            [vec![(13, 1.0)], vec![(0, 1.0)], vec![(35, 1.0)]]
        );
        assert_eq!(
            hist("passes_per_team_distribution"),
            [vec![(400, 1.0), (500, 1.0)], vec![(1499, 1.0)]]
        );
        // Shots 12 m, 34 m and 55 m from the goal; the pass is not a shot.
        assert_eq!(
            hist("shot_distance_distribution"),
            [vec![(12, 1.0)], vec![(34, 1.0), (40, 1.0)]]
        );
        // Match 5: a chain of 3 passes by the team in possession (the other team's pass
        // is not one of them), from 0 s to 6.5 s; and a chain of a carry, 4 s. Match 6: one
        // chain of 2 passes whose seconds stay in its first period: 1 s.
        assert_eq!(
            hist("chain_passes_distribution"),
            [vec![(0, 1.0), (3, 1.0)], vec![(2, 1.0)]]
        );
        assert_eq!(
            hist("chain_seconds_distribution"),
            [vec![(2, 1.0), (3, 1.0)], vec![(0, 1.0)]]
        );
        // calib.real shows a whole sample's mean and its error over matches.
        let info = read_all(&at(&dir), &r, &got);
        let goals = info.band("goals_distribution").unwrap();
        assert_eq!(goals.status, "read");
        assert_eq!(goals.value, Some(round_to(13.0 / 3.0, 6)));
        assert_eq!(goals.measurement.count, Some(3));
        let cells = info.band("scoreline_cells").unwrap();
        assert_eq!(cells.value, Some(round_to(13.0 / 3.0, 6)));
        // A missing file: not here, by path, and the other bands read on.
        std::fs::remove_file(dir.join("wy2_team_matches.csv")).unwrap();
        let got = samples(&at(&dir), &r);
        let why = got["passes_per_team_distribution"].as_ref().unwrap_err();
        assert!(
            why.ends_with("wy2_team_matches.csv: the file does not exist"),
            "{why}"
        );
        assert!(got["goals_distribution"].is_ok());
        let info = read_all(&at(&dir), &r, &got);
        let passes = info.band("passes_per_team_distribution").unwrap();
        assert_eq!(passes.status, "not_here");
        // An absent folder: every band not here.
        let none = samples(&at(&dir.join("absent")), &r);
        let target_backed = ["shots_total_distribution", "cards_total_distribution"];
        assert!(
            none.iter()
                .filter(|(b, _)| !target_backed.contains(&b.as_str()))
                .all(|(_, s)| {
                    s.as_ref()
                        .unwrap_err()
                        .ends_with("the folder does not exist")
                })
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- The pass rule's real side ---------------------------------------------------

    /// The real side of one band read alone from `dir`.
    fn side_of(dir: &Path, source: RealSource) -> (Outcome, Option<Real>) {
        read_with(&at(dir), &band(source, 10, 1.0), None, None)
    }

    #[test]
    fn a_csv_reduction_gives_one_unit_per_kept_row_that_reproduces_its_value() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("m.csv"),
            "comp,hg,ag\nA,1,0\nA,3,2\nB,0,0\nA,2,2\n",
        )
        .unwrap();
        let cases: [(Pointer, usize); 4] = [
            (pointer("m.csv", "hg+ag"), 4),
            (
                Pointer {
                    reduce: Some(Reduce::Share),
                    op: Some(super::super::bands::Op::AtLeast),
                    value: Some(3.0),
                    ..pointer("m.csv", "hg+ag")
                },
                4,
            ),
            (
                Pointer {
                    reduce: Some(Reduce::FactorialRatio),
                    ..pointer("m.csv", "hg")
                },
                4,
            ),
            (
                Pointer {
                    filter: Some(BTreeMap::from([(
                        "comp".to_string(),
                        vec!["A".to_string()],
                    )])),
                    ..pointer("m.csv", "hg")
                },
                3,
            ),
        ];
        for (p, rows) in cases {
            let (outcome, side) = side_of(dir.path(), RealSource::Pointer(Box::new(p.clone())));
            let (value, _) = value(outcome);
            let Some(Real::Matches(strata)) = side else {
                panic!("{}: {side:?}", p.key);
            };
            assert_eq!(strata.len(), 1);
            assert_eq!(strata[0].len(), rows, "{}", p.key);
            let (estimate, se) = pass_rule::interval::ratio_and_se(&strata[0]);
            assert!(
                (estimate - value).abs() < 1e-12,
                "{}: {estimate} {value}",
                p.key
            );
            assert!(se > 0.0, "{}", p.key);
        }
    }

    #[test]
    fn a_json_key_over_every_match_is_a_tracked_side_and_a_summary_its_value_and_error() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("t.json"),
            r#"{"per_match": {"a": {"x": 1.0}, "b": {"x": 2.0}, "c": {"x": 4.0}}, "rate": 0.5}"#,
        )
        .unwrap();
        let (_, side) = side_of(
            dir.path(),
            RealSource::Pointer(Box::new(pointer("t.json", "per_match.*.x"))),
        );
        assert_eq!(side, Some(Real::Tracked(vec![1.0, 2.0, 4.0])));
        let share = Pointer {
            reduce: Some(Reduce::Share),
            ..pointer("t.json", "rate")
        };
        let (outcome, side) = side_of(dir.path(), RealSource::Pointer(Box::new(share)));
        let (v, se) = value(outcome);
        assert_eq!(
            side,
            Some(Real::Summary {
                value: v,
                se: se.unwrap()
            })
        );
        // A scale applies to the side as to the value.
        let scaled = Pointer {
            scale: Some(0.5),
            ..pointer("t.json", "per_match.*.x")
        };
        let (_, side) = side_of(dir.path(), RealSource::Pointer(Box::new(scaled)));
        assert_eq!(side, Some(Real::Tracked(vec![0.5, 1.0, 2.0])));
    }

    #[test]
    fn a_reference_file_target_reads_its_value_and_error() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gate/real-targets.json");
        let file = ReferenceFile::load(&path).unwrap();
        let key = "tier.1.goals_per_match";
        let want = file.target(key).unwrap();
        let def = band(
            RealSource::Target(super::super::bands::TargetRef { target: key.into() }),
            10,
            1.0,
        );
        let folder = at(Path::new("."));
        let (outcome, side) = read_with(&folder, &def, None, Some(&file));
        assert_eq!(value(outcome), (want.value.unwrap(), Some(want.se)));
        assert_eq!(
            side,
            Some(Real::Summary {
                value: want.value.unwrap(),
                se: want.se
            })
        );
        // Without the file, or with a key it lacks, the band reads not here.
        assert!(matches!(
            read_with(&folder, &def, None, None).0,
            Outcome::NotHere { .. }
        ));
        let missing = band(
            RealSource::Target(super::super::bands::TargetRef {
                target: "tier.9.goals_per_match".into(),
            }),
            10,
            1.0,
        );
        assert!(matches!(
            read_with(&folder, &missing, None, Some(&file)).0,
            Outcome::NotHere { .. }
        ));
    }

    /// A StatsBomb distribution target: a band of 41 shot-distance bins over two seasons.
    fn sb_target(shares: Vec<f64>, n: u64) -> crate::real_targets::Target {
        serde_json::from_value(serde_json::json!({
            "shares": shares, "bins": 41, "width": 1.0,
            "se": 0.1, "n": n, "seasons": ["2_27", "55_43"], "layer": "pooled",
            "provider": "StatsBomb", "source_label": "StatsBomb open data, test",
            "event_definition": "e", "denominator": "d", "unit_of_observation": "match",
            "label": "calibration", "engine_count": {"text": "t"}
        }))
        .unwrap()
    }

    fn sb_band() -> BandDef {
        let mut def = band(
            RealSource::Target(super::super::bands::TargetRef {
                target: "sb.men.dist.shot_distance".into(),
            }),
            3,
            0.1,
        );
        def.measure = crate::report::bands::Measure::Distribution {
            of: crate::report::bands::Source::ShotDistance,
            width: 1.0,
            bins: 41,
        };
        def
    }

    #[test]
    fn statsbomb_match_histograms_reach_the_sample_one_histogram_per_match() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let side = tmp.path().join("real-targets");
        std::fs::create_dir_all(&side).unwrap();
        // Three matches: bins 7 7 | 14 | 7 21; another kind and a season outside the target.
        std::fs::write(
            side.join("statsbomb-match-histograms.csv"),
            "label,match,kind,bin,count\n\
             2_27,1,shot_distance,7,2\n\
             2_27,1,chain_passes,3,9\n\
             2_27,2,shot_distance,14,1\n\
             55_43,3,shot_distance,7,1\n\
             55_43,3,shot_distance,21,1\n\
             37_90,4,shot_distance,40,5\n",
        )
        .unwrap();
        let mut shares = vec![0.0; 41];
        shares[7] = 0.6;
        shares[14] = 0.2;
        shares[21] = 0.2;
        let target = sb_target(shares, 3);
        let def = sb_band();
        let s = target_sample(&at(&out), &def, "sb.men.dist.shot_distance", &target).unwrap();
        assert_eq!(
            s.per_match,
            vec![vec![(7, 2.0)], vec![(14, 1.0)], vec![(7, 1.0), (21, 1.0)]]
        );
        assert_eq!((s.bins, s.width), (41, 1.0));
        // The pooled shares are the target's.
        let mut pooled = vec![0.0; 41];
        for h in &s.per_match {
            for (b, c) in h {
                pooled[*b as usize] += c;
            }
        }
        let total: f64 = pooled.iter().sum();
        for (b, want) in target.shares.as_ref().unwrap().iter().enumerate() {
            assert!((pooled[b] / total - want).abs() < 1e-12, "bin {b}");
        }

        // A file built apart from the reference file gives the reason, not a sample.
        let mut other = vec![0.0; 41];
        other[7] = 1.0;
        let stale = sb_target(other, 3);
        let e = target_sample(&at(&out), &def, "sb.men.dist.shot_distance", &stale).unwrap_err();
        assert!(
            e.contains("rebuild") || e.contains("real-targets build"),
            "{e}"
        );
    }

    #[test]
    fn missing_statsbomb_histograms_give_the_reason_and_no_sample() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let target = sb_target(vec![1.0 / 41.0; 41], 3);
        let def = sb_band();
        let e = target_sample(&at(&out), &def, "sb.men.dist.shot_distance", &target).unwrap_err();
        assert!(
            e.starts_with("per-match StatsBomb histograms absent at "),
            "{e}"
        );
        assert!(e.ends_with("run engine-cli real-targets build"), "{e}");
        // A file that misses a season of the target is short.
        let side = tmp.path().join("real-targets");
        std::fs::create_dir_all(&side).unwrap();
        std::fs::write(
            side.join("statsbomb-match-histograms.csv"),
            "label,match,kind,bin,count\n2_27,1,shot_distance,7,2\n",
        )
        .unwrap();
        let e = target_sample(&at(&out), &def, "sb.men.dist.shot_distance", &target).unwrap_err();
        assert!(e.contains("is short for sb.men.dist.shot_distance"), "{e}");
    }

    /// A tracked-match target reads as its matches, resampled one left out at a time.
    #[test]
    fn a_tracked_match_target_reads_its_matches() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gate/real-targets.json");
        let file = ReferenceFile::load(&path).unwrap();
        let key = "trk.dfl.cb_depth_in";
        let want = file.target(key).unwrap();
        let def = band(
            RealSource::Target(super::super::bands::TargetRef { target: key.into() }),
            7,
            1.0,
        );
        let (outcome, side) = read_with(&at(Path::new(".")), &def, None, Some(&file));
        assert_eq!(value(outcome), (want.value.unwrap(), Some(want.se)));
        let values = want.per_match.clone().unwrap();
        assert_eq!(values.len(), 7);
        assert_eq!(side, Some(Real::Tracked(values)));
    }

    /// The slow tests read the reference file through the pass-rule crate's narrow view; it
    /// gives the same four figures as the full reader for every key of the committed file.
    #[test]
    fn the_slow_tests_view_of_the_reference_file_matches_the_full_reader() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gate/real-targets.json");
        let full = crate::real_targets::ReferenceFile::load(&path).unwrap();
        let view = pass_rule::limits::reference_targets(&path).unwrap();
        assert_eq!(view.len(), full.targets.len());
        for (key, target) in &full.targets {
            let seen = view
                .get(key)
                .unwrap_or_else(|| panic!("the view has no {key}"));
            assert_eq!(seen.value, target.value, "{key} value");
            assert_eq!(seen.se, target.se, "{key} se");
            assert_eq!(seen.n, target.n, "{key} n");
            assert_eq!(seen.between_league_sd, target.between_league_sd, "{key}");
        }
    }
}

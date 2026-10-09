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

use super::bands::{BandDef, ChainCount, Evidence, Measure, Pointer, RealSource, Reduce, Registry};
use super::distribution::{Sample, bin_of};
use super::{se_mean, se_ratio, se_share};

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
/// gives the error; keys joined by `+` are summed), over `den` when it names one (no error),
/// and a share over `sample`.
fn read_json(path: &Path, p: &Pointer, sample: u64) -> Result<(f64, Option<f64>), String> {
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
        Reduce::Share if p.op.is_none() || p.value.is_none() => return Err(need("op and value")),
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
    // The kept rows' values, weights and denominators, and each group's values in file order.
    let (mut xs, mut ws, mut ds) = (Vec::new(), Vec::new(), Vec::new());
    let mut groups: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut hits: BTreeMap<String, bool> = BTreeMap::new();
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
        for (i, vals) in &filter {
            kept &= vals.iter().any(|v| v == cell(*i).unwrap_or(""));
        }
        if let Some((a, b)) = same {
            kept &= cell(a)? == cell(b)?;
        }
        for &(i, [lo, hi]) in &ranges {
            if !kept {
                break;
            }
            let v = number(i)?;
            kept &= lo <= v && v < hi;
        }
        if !kept {
            continue;
        }
        if p.skip_blank {
            let used = match &key {
                Key::Sum(cols) => cols.iter().map(|c| c.0).collect(),
                Key::Pair(a, b) => vec![*a, *b],
                Key::Tokens(i) => vec![*i],
            };
            let blank = used
                .into_iter()
                .chain(weight)
                .chain(den.iter().flatten().map(|c| c.0))
                .any(|i| cell(i).is_ok_and(|t| t.trim().is_empty()));
            if blank {
                continue;
            }
        }
        let name = |cols: &[usize]| -> Result<String, String> {
            Ok(cols
                .iter()
                .map(|&i| cell(i))
                .collect::<Result<Vec<_>, _>>()?
                .join("+"))
        };
        let x = match &key {
            Key::Tokens(i) => {
                let hit = cell(*i)?.split_whitespace().any(|t| tokens.contains(t));
                *hits
                    .entry(name(group.as_deref().unwrap_or(&[]))?)
                    .or_default() |= hit;
                continue;
            }
            Key::Pair(a, b) => {
                let (u, v) = (number(*a)?, number(*b)?);
                match (reduce, p.goal) {
                    (Reduce::GoalDistance, Some([gx, gy])) => (gx - u).hypot(gy - v),
                    _ => f64::from(u8::from(
                        scores.iter().any(|s| *s == [u, v] || *s == [v, u]),
                    )),
                }
            }
            Key::Sum(cols) => sum(cols)?,
        };
        if let Some(g) = &group {
            groups.entry(name(g)?).or_default().push(x);
            continue;
        }
        xs.push(match (reduce, p.bins) {
            (Reduce::BinShare, Some([lo, hi])) => f64::from(u8::from(lo <= x && x < hi)),
            _ => x,
        });
        ws.push(weight.map_or(Ok(1.0), number)?);
        if let Some(d) = &den {
            ds.push(sum(d)?);
        }
    }
    let per_group = |f: &dyn Fn(&[f64]) -> Option<f64>| -> Result<(f64, Option<f64>), String> {
        let values: Vec<f64> = groups.values().filter_map(|v| f(v)).collect();
        if values.is_empty() {
            return Err("no group has enough rows".into());
        }
        Ok((mean(&values), Some(se_mean(&values))))
    };
    let largest = |v: &[f64]| v.iter().copied().fold(f64::MIN, f64::max);
    let smallest = |v: &[f64]| v.iter().copied().fold(f64::MAX, f64::min);
    match reduce {
        Reduce::GroupShare => {
            if hits.is_empty() {
                return Err("no row kept".into());
            }
            let hit = hits.values().filter(|h| **h).count();
            let p = hit as f64 / hits.len() as f64;
            Ok((p, Some(se_share(p, hits.len()))))
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
            Ok((xs.iter().sum::<f64>() / d, None))
        }
        Reduce::Mean | Reduce::BinShare if weight.is_some() => {
            let w: f64 = ws.iter().sum();
            if w <= 0.0 {
                return Err("the weights sum to 0".into());
            }
            let wx: Vec<f64> = xs.iter().zip(&ws).map(|(x, w)| x * w).collect();
            Ok((wx.iter().sum::<f64>() / w, Some(se_ratio(&wx, &ws))))
        }
        Reduce::Mean | Reduce::GoalDistance => Ok((mean(&xs), Some(se_mean(&xs)))),
        Reduce::BinShare | Reduce::Scoreline => {
            let p = mean(&xs);
            Ok((p, Some(se_share(p, xs.len()))))
        }
        Reduce::Share => {
            let (op, target) = (
                p.op.unwrap_or(super::bands::Op::AtLeast),
                p.value.unwrap_or(0.0),
            );
            let hit = xs.iter().filter(|s| op.holds(**s, target)).count();
            let p = hit as f64 / xs.len() as f64;
            Ok((p, Some(se_share(p, xs.len()))))
        }
        Reduce::FactorialRatio => {
            let s: f64 = xs.iter().sum();
            if s <= 0.0 {
                return Err(format!("{} sums to 0", p.key));
            }
            let f: f64 = xs.iter().map(|x| x * (x - 1.0)).sum();
            Ok((xs.len() as f64 * f / (s * s), None))
        }
        Reduce::Histogram | Reduce::Cells | Reduce::Chains => Err(WHOLE_SAMPLE.into()),
    }
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

/// Reads every band's real value from `folder`; a distribution band's comes from its whole
/// sample in `samples`, read once for the judgement.
pub fn read_all(folder: &RealFolder, registry: &Registry, samples: &Samples) -> RealInfo {
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
        let whole = def.is_distribution().then(|| samples.get(&def.band));
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
                        RealSource::NoCounterpart(_) => None,
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
            _ => read(folder, def),
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
            power: None,
            needed: None,
            outside_by: None,
            no_target: None,
            real: None,
            distance: None,
            upper: None,
            noise_floor: None,
            report_only: None,
            real_note: None,
            judged: None,
            note: None,
            accepted: None,
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
        assert_eq!((info.missing, info.no_counterpart), (15 + 30, 3));

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
        assert_eq!(got.len(), 6, "{got:?}");
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
        assert!(none.values().all(|s| {
            s.as_ref()
                .unwrap_err()
                .ends_with("the folder does not exist")
        }));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

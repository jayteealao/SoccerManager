//! `engine-cli bisect`: find the first tick where two engine versions differ on one replay
//! file.
//!
//! Each side is a commit, built on demand into the bisect build cache, or a ready binary.
//! Both builds re-simulate the file twice: once for the digest of the full match state
//! after every tick, and once more, up to the first tick whose digests differ, for that
//! tick's named state parts and debug trace. A side that cannot be built, lacks the
//! re-simulate options, crashes, hangs, or stops early makes the result incomplete (exit 3),
//! never "no difference".

mod cache;
mod report;
mod runs;

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use stream::read_fixture;

use crate::cli::BisectOpts;
use report::{Build, Found, Report};
use runs::{Digests, TickState};

/// Exit code: no difference.
const SAME: i32 = 0;
/// Exit code: refused or bad arguments.
const REFUSED: i32 = 1;
/// Exit code: the builds differ.
const DIFFERS: i32 = 2;
/// Exit code: a side did not give a complete result.
const INCOMPLETE: i32 = 3;

/// One side as the command line names it.
struct Side {
    name: &'static str,
    rev: Option<String>,
    binary: Option<PathBuf>,
}

impl Side {
    fn given(&self) -> String {
        match (&self.rev, &self.binary) {
            (Some(rev), _) => rev.clone(),
            (None, Some(path)) => path.display().to_string(),
            (None, None) => String::new(),
        }
    }
}

/// A side resolved to an executable.
struct Resolved {
    path: PathBuf,
    source: &'static str,
}

/// A folder for the run's files, removed when the run ends.
struct WorkDir(PathBuf);

impl WorkDir {
    fn new() -> anyhow::Result<Self> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let dir =
            std::env::temp_dir().join(format!("engine-cli-bisect-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn run(content_dir: Option<&Path>, opts: &BisectOpts) -> anyhow::Result<i32> {
    if content_dir.is_some() {
        eprintln!("error: bisect reads every input from the replay file; drop --content-dir");
        return Ok(REFUSED);
    }
    let shown = opts.fixture.display().to_string();
    match read_fixture(&opts.fixture) {
        Err(err) => {
            eprintln!("error: cannot bisect {shown}: {err}");
            return Ok(REFUSED);
        }
        Ok(fixture) if fixture.record.is_none() => {
            eprintln!(
                "error: {shown} holds no inputs: it is a version-3 replay file, which plays from \
                 its frames only; bisect needs the inputs to re-simulate it"
            );
            return Ok(REFUSED);
        }
        Ok(_) => {}
    }
    let fixture = std::path::absolute(&opts.fixture)?;
    let limit = Duration::from_secs(opts.timeout);
    // Absolute paths: cargo runs inside each checkout, where a relative cache path would
    // name a folder inside that checkout.
    let repo = std::path::absolute(&opts.repo)?;
    let cache_root = match &opts.cache {
        Some(cache) => std::path::absolute(cache)?,
        None => repo.join("target").join("bisect"),
    };
    let sides = [
        Side {
            name: "a",
            rev: opts.a.clone(),
            binary: opts.a_binary.clone(),
        },
        Side {
            name: "b",
            rev: opts.b.clone(),
            binary: opts.b_binary.clone(),
        },
    ];
    let work = WorkDir::new()?;

    // Resolve both sides to executables.
    let resolved = both(&sides, |_, side| resolve(side, opts, &repo, &cache_root));
    let [a, b] = match resolved {
        Ok(pair) => pair,
        Err(reasons) => return Ok(incomplete(opts, &shown, &sides, &reasons, None)),
    };
    let bins = [&a, &b];

    // Probe, then pass 1.
    if let Err(reasons) = both(&sides, |i, side| {
        runs::probe(&bins[i].path, &work.0, side.name, limit)
    }) {
        return Ok(incomplete(opts, &shown, &sides, &reasons, None));
    }
    let digests = both(&sides, |i, side| {
        runs::digests(&bins[i].path, &fixture, &work.0, side.name, limit)
    });
    let [da, db] = match digests {
        Ok(pair) => pair,
        Err(reasons) => return Ok(incomplete(opts, &shown, &sides, &reasons, None)),
    };
    if let Err(reason) = runs::same_inventory(&da, &db) {
        let reasons = [None, None];
        return Ok(incomplete(opts, &shown, &sides, &reasons, Some(&reason)));
    }
    let builds = [build(&sides[0], &a, &da), build(&sides[1], &b, &db)];

    // Scan, then pass 2 at the first differing tick.
    let found = match scan(&da, &db) {
        Scan::Same => Found::Same {
            ticks: da.ticks.len(),
        },
        Scan::Differs {
            tick,
            length,
            full_time,
        } => {
            let played = [da.ticks.len(), db.ticks.len()];
            let states = both(&sides, |i, side| {
                if played[i] < tick as usize {
                    // This build ended at full time before the tick.
                    return Ok(TickState {
                        fields: Vec::new(),
                        trace: Vec::new(),
                    });
                }
                runs::tick_state(&bins[i].path, &fixture, &work.0, side.name, tick, limit)
                    .map_err(|reason| format!("the states first differ at tick {tick}; {reason}"))
            });
            let [sa, sb] = match states {
                Ok(pair) => pair,
                Err(reasons) => return Ok(incomplete(opts, &shown, &sides, &reasons, None)),
            };
            Found::Differs {
                tick,
                fields: report::differing(&sa.fields, &sb.fields),
                length,
                full_time,
                traces: [sa.trace, sb.trace],
            }
        }
    };
    let code = match found {
        Found::Same { .. } => SAME,
        Found::Differs { .. } => DIFFERS,
    };
    let report = Report {
        fixture: shown,
        builds,
        found,
    };
    if opts.json {
        println!("{}", report.json());
    } else {
        print!("{}", report.text());
    }
    Ok(code)
}

/// Runs `step` on both sides, with each side's place, and returns both results, or each
/// side's reason when either side failed.
fn both<T>(
    sides: &[Side; 2],
    mut step: impl FnMut(usize, &Side) -> Result<T, String>,
) -> Result<[T; 2], [Option<String>; 2]> {
    let a = step(0, &sides[0]);
    let b = step(1, &sides[1]);
    match (a, b) {
        (Ok(a), Ok(b)) => Ok([a, b]),
        (a, b) => Err([a.err(), b.err()]),
    }
}

/// A side's executable: the cached build of its commit, or its ready binary.
fn resolve(
    side: &Side,
    opts: &BisectOpts,
    repo: &Path,
    cache_root: &Path,
) -> Result<Resolved, String> {
    if let Some(path) = &side.binary {
        if !path.is_file() {
            return Err(format!("{} does not exist", path.display()));
        }
        let path = std::path::absolute(path).map_err(|e| e.to_string())?;
        return Ok(Resolved {
            path,
            source: "ready binary",
        });
    }
    let rev = side.rev.as_deref().unwrap_or_default();
    let commit = cache::resolve_commit(repo, rev)?;
    let (toolchain, target) = cache::toolchain_of(repo, &commit, cache_root)?;
    let key = cache::CacheKey {
        commit,
        toolchain,
        target,
        profile: opts.profile.clone(),
        features: cache::features_key(&opts.features),
    };
    eprintln!(
        "bisect: {} = {rev} at commit {} ({} profile)",
        side.name, key.commit, key.profile
    );
    let builder = cache::GitWorktreeBuilder {
        repo: repo.to_path_buf(),
        cache: cache_root.to_path_buf(),
    };
    let cached = cache::lookup(cache_root, &key, &builder)?;
    Ok(Resolved {
        path: cached.path,
        source: if cached.reused { "reused" } else { "built" },
    })
}

fn build(side: &Side, resolved: &Resolved, digests: &Digests) -> Build {
    Build {
        side: side.name,
        given: side.given(),
        source: resolved.source,
        engine: digests.header["engine"].clone(),
        scheme: digests.scheme(),
    }
}

/// Where two digest lists first part.
#[derive(Debug, PartialEq, Eq)]
enum Scan {
    Same,
    Differs {
        tick: u32,
        length: Option<[usize; 2]>,
        full_time: bool,
    },
}

/// Compares two gap-free digest lists tick by tick. When every tick both played is the
/// same and one build played more, the first tick past the shorter run differs.
fn scan(a: &Digests, b: &Digests) -> Scan {
    if let Some((x, _)) = a.ticks.iter().zip(&b.ticks).find(|(x, y)| x.1 != y.1) {
        return Scan::Differs {
            tick: x.0,
            length: None,
            full_time: false,
        };
    }
    let (na, nb) = (a.ticks.len(), b.ticks.len());
    if na != nb {
        return Scan::Differs {
            tick: na.min(nb) as u32 + 1,
            length: Some([na, nb]),
            full_time: false,
        };
    }
    if a.finish != b.finish || a.full_time != b.full_time {
        return Scan::Differs {
            tick: a.ticks.last().map_or(0, |(t, _)| *t),
            length: None,
            full_time: true,
        };
    }
    Scan::Same
}

/// Prints the incomplete result: every side's reason, and a reason for the pair.
fn incomplete(
    opts: &BisectOpts,
    shown: &str,
    sides: &[Side; 2],
    reasons: &[Option<String>; 2],
    pair: Option<&str>,
) -> i32 {
    if opts.json {
        let mut out = serde_json::json!({ "verdict": "incomplete", "fixture": shown });
        for (side, reason) in sides.iter().zip(reasons) {
            out[side.name] = serde_json::json!({ "given": side.given(), "reason": reason });
        }
        if let Some(reason) = pair {
            out["reason"] = serde_json::json!(reason);
        }
        println!("{out}");
    } else {
        println!(
            "bisect: incomplete: no comparison was made; this is not a \"no difference\" result"
        );
        println!("replay file: {shown}");
        for (side, reason) in sides.iter().zip(reasons) {
            if let Some(reason) = reason {
                println!("{} ({}): {reason}", side.name, side.given());
            }
        }
        if let Some(reason) = pair {
            println!("both: {reason}");
        }
    }
    INCOMPLETE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digests(ticks: &[&str], finish: &str) -> Digests {
        Digests {
            header: serde_json::json!({ "state_digests": 1, "inventory": 1, "scheme": 1 }),
            ticks: ticks
                .iter()
                .enumerate()
                .map(|(i, d)| (i as u32 + 1, d.to_string()))
                .collect(),
            finish: finish.into(),
            full_time: true,
        }
    }

    #[test]
    fn the_scan_finds_the_first_differing_tick_a_length_or_full_time() {
        let a = digests(&["x", "y", "z"], "f");
        assert_eq!(scan(&a, &a.clone()), Scan::Same);
        assert_eq!(
            scan(&a, &digests(&["x", "q", "r"], "f")),
            Scan::Differs {
                tick: 2,
                length: None,
                full_time: false
            }
        );
        assert_eq!(
            scan(&a, &digests(&["x", "y"], "f")),
            Scan::Differs {
                tick: 3,
                length: Some([3, 2]),
                full_time: false
            }
        );
        assert_eq!(
            scan(&a, &digests(&["x", "y", "z"], "g")),
            Scan::Differs {
                tick: 3,
                length: None,
                full_time: true
            }
        );
    }
}

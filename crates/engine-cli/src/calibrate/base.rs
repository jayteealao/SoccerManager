//! The old engine of a change run, and its result cache.
//!
//! The old engine is a revision, built once through the bisect build cache, or a ready
//! binary. Its results live in an ordinary keyed run folder that the old engine's own
//! `calibrate` writes, with a compact row per match, under
//! `SM_DATA_DIR/calibrate/base/v2/<build id>/<content hash>/seed-<S>-minutes-<M>/`. A planned
//! fixture whose key that folder's ledger finished with the same engine seed is a cache hit,
//! and its row is the old engine's result. Only when a fixture misses is the old engine
//! built and run, and then it resumes its folder and plays only the missing fixtures. `v2`
//! is the cache format: `v1` folders (statistics files, no rows) are never read and can be
//! deleted.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Context;
use sha2::{Digest, Sha256};

use super::fixtures::{FIXTURE_SCHEME, FixtureKey};
use super::rows::ROWS_FORMAT;
use super::run_folder;
use crate::bisect::cache::{self, Builder, CacheKey};
use crate::report::Suite;

/// The cache format, the first folder under `calibrate/base/`.
pub const CACHE_FORMAT: &str = "v2";

/// Where the old engine comes from.
#[derive(Debug, Clone)]
pub enum Source {
    /// A revision of this repository, built with the release profile.
    Rev(String),
    /// A ready executable, played with the run's content.
    Binary(PathBuf),
}

/// What the old engine plays.
pub struct Request<'a> {
    pub source: Source,
    pub data: &'a Path,
    /// The run's content folder: the content of a ready binary.
    pub content_dir: &'a Path,
    pub seed: u64,
    pub minutes: u32,
    pub matches: u32,
    pub jobs: u32,
    pub suites: &'a [Suite],
    /// The `--pairing` names, when the run plays only some pairings.
    pub pairings: &'a [String],
    /// Every planned fixture: its key and engine seed.
    pub planned: &'a [(FixtureKey, u64)],
    /// A test seam: the build fails as a broken base branch would.
    pub fail_build: bool,
}

/// The old engine's results for a run.
#[derive(Debug, Clone)]
pub struct BaseRun {
    pub source: &'static str,
    pub rev: Option<String>,
    pub commit: Option<String>,
    pub build_id: String,
    pub content_hash: String,
    /// The cache folder: a keyed run folder of the old engine.
    pub dir: PathBuf,
    /// Planned fixtures the cache already held.
    pub hits: u32,
    /// Planned fixtures the old engine played now.
    pub played: u32,
}

/// A revision, resolved without building it.
struct Revision {
    repo: PathBuf,
    cache_root: PathBuf,
    key: CacheKey,
}

/// Finds or plays the old engine's result of every planned fixture. Any failure comes
/// before the changed engine plays a match.
pub fn run(req: &Request<'_>) -> anyhow::Result<BaseRun> {
    let (source, rev, revision, build_id, content_dir) = match &req.source {
        Source::Binary(path) => {
            let path = std::path::absolute(path)?;
            let sha = cache::file_sha256(&path)
                .with_context(|| format!("the old engine {} cannot be read", path.display()))?;
            ("binary", None, None, sha, req.content_dir.to_path_buf())
        }
        Source::Rev(rev) => {
            let revision = resolve(rev)?;
            let content = extract_content(&revision)?;
            let id = revision.key.id();
            ("rev", Some(rev.clone()), Some(revision), id, content)
        }
    };
    let build_id = build_id[..16].to_string();
    let content_hash = folder_hash(&content_dir)?;
    let dir = req
        .data
        .join("calibrate")
        .join("base")
        .join(CACHE_FORMAT)
        .join(&build_id)
        .join(&content_hash)
        .join(format!("seed-{}-minutes-{}", req.seed, req.minutes));
    let hits = hits(&dir, req.planned)?;
    let missing = req.planned.len() as u32 - hits;
    if missing > 0 {
        let exe = match (&req.source, &revision) {
            (Source::Binary(path), _) => std::path::absolute(path)?,
            (Source::Rev(rev), Some(revision)) => build(rev, revision, req.fail_build)?,
            (Source::Rev(_), None) => unreachable!("a revision is resolved"),
        };
        probe(&exe, &content_dir, &dir)?;
        for &suite in req.suites {
            play(req, &exe, &content_dir, &dir, suite)?;
        }
        let now = self::hits(&dir, req.planned)?;
        if (now as usize) < req.planned.len() {
            anyhow::bail!(
                "the old engine left {} of {} fixtures unplayed; its errors are above",
                req.planned.len() - now as usize,
                req.planned.len()
            );
        }
    }
    Ok(BaseRun {
        source,
        rev,
        commit: revision.map(|r| r.key.commit),
        build_id,
        content_hash,
        dir,
        hits,
        played: missing,
    })
}

/// Planned fixtures the folder's ledger finished with the same engine seed.
fn hits(dir: &Path, planned: &[(FixtureKey, u64)]) -> anyhow::Result<u32> {
    let done = run_folder::finished(dir, None)?;
    let n = planned
        .iter()
        .filter(|(key, seed)| done.get(key).is_some_and(|d| d.seed == *seed))
        .count();
    Ok(u32::try_from(n).unwrap_or(u32::MAX))
}

/// The cache key of `rev` in the repository this command runs in, with the release profile.
fn resolve(rev: &str) -> anyhow::Result<Revision> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .stdin(Stdio::null())
        .output()
        .context("cannot run git to find the repository of the old engine")?;
    let top = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || top.is_empty() {
        anyhow::bail!("--base {rev} needs a git checkout of this repository as the working folder");
    }
    let repo = std::path::absolute(PathBuf::from(top))?;
    let cache_root = repo.join("target").join("bisect");
    let commit = cache::resolve_commit(&repo, rev).map_err(anyhow::Error::msg)?;
    let (toolchain, target) =
        cache::toolchain_of(&repo, &commit, &cache_root).map_err(anyhow::Error::msg)?;
    Ok(Revision {
        repo,
        cache_root,
        key: CacheKey {
            commit,
            toolchain,
            target,
            profile: "release".into(),
            features: String::new(),
        },
    })
}

/// The `content/` folder of the revision, written once under the build cache.
fn extract_content(revision: &Revision) -> anyhow::Result<PathBuf> {
    let commit = &revision.key.commit;
    let dir = revision.cache_root.join("content").join(commit);
    let done = dir.join(".complete");
    if done.is_file() {
        return Ok(dir);
    }
    let git = |args: &[&str]| -> anyhow::Result<Vec<u8>> {
        let out = Command::new("git")
            .arg("-C")
            .arg(&revision.repo)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .context("cannot run git")?;
        if !out.status.success() {
            anyhow::bail!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(out.stdout)
    };
    let list = git(&["ls-tree", "-r", "--name-only", commit, "--", "content"])?;
    let _ = fs::remove_dir_all(&dir);
    for path in String::from_utf8_lossy(&list).lines() {
        let Some(relative) = path.strip_prefix("content/") else {
            continue;
        };
        let bytes = git(&["show", &format!("{commit}:{path}")])?;
        let target = dir.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, bytes).with_context(|| format!("cannot write {}", target.display()))?;
    }
    fs::create_dir_all(&dir)?;
    fs::write(&done, commit)?;
    Ok(dir)
}

/// A builder that fails as a broken base branch does: the `base-build` test seam.
struct BrokenBuild;

impl Builder for BrokenBuild {
    fn build(&self, _key: &CacheKey, _out: &Path) -> Result<(), String> {
        Err("error[E0425]: injected base build failure".into())
    }
}

/// The executable of the revision, from the build cache or built now.
fn build(rev: &str, revision: &Revision, fail: bool) -> anyhow::Result<PathBuf> {
    eprintln!(
        "old engine {rev} at commit {}: building it, or taking it from the build cache",
        revision.key.commit
    );
    let git_builder = cache::GitWorktreeBuilder {
        repo: revision.repo.clone(),
        cache: revision.cache_root.clone(),
    };
    let builder: &dyn Builder = if fail { &BrokenBuild } else { &git_builder };
    cache::lookup(&revision.cache_root, &revision.key, builder)
        .map(|cached| cached.path)
        .map_err(|why| {
            anyhow::anyhow!(
                "the old engine {rev} (commit {}) cannot be built, so no match was played:\n{why}",
                revision.key.commit
            )
        })
}

/// Plays one one-minute match with the old engine and refuses it when its report does not
/// carry this program's fixture scheme (an engine from before fixture keys plays other
/// matches, so its results cannot be joined on keys) or this program's row format (an
/// engine from before compact rows leaves no rows to read).
fn probe(exe: &Path, content: &Path, dir: &Path) -> anyhow::Result<()> {
    let probe = dir.with_file_name(format!("probe-{}", std::process::id()));
    let _ = fs::remove_dir_all(&probe);
    let status = calibrate(exe, content)
        .args([
            "--seed",
            "1",
            "--matches",
            "1",
            "--minutes",
            "1",
            "--suite",
            "equal",
        ])
        .args(["--jobs", "1", "--out"])
        .arg(&probe)
        .status()
        .with_context(|| format!("cannot start the old engine {}", exe.display()))?;
    let report = fs::read_to_string(probe.join("report.json")).ok();
    let _ = fs::remove_dir_all(&probe);
    if !matches!(status.code(), Some(0 | 2)) {
        anyhow::bail!(
            "the old engine {} failed its probe match (exit {:?}); no match was played",
            exe.display(),
            status.code()
        );
    }
    let report =
        report.and_then(|text| serde_json::from_str::<serde_json::Value>(text.trim()).ok());
    let scheme = report
        .as_ref()
        .and_then(|v| v["fixtures.scheme"].as_str().map(str::to_string));
    if scheme.as_deref() != Some(FIXTURE_SCHEME) {
        anyhow::bail!(
            "the old engine {} plays fixtures by the {} scheme, and this run uses fixture \
             keys ({FIXTURE_SCHEME}); it was built before fixture keys, so its matches cannot \
             be joined with this run's",
            exe.display(),
            scheme.as_deref().unwrap_or("old seeding")
        );
    }
    let format = report
        .as_ref()
        .and_then(|v| v["calib.rows"]["format"].as_u64());
    refuse_rows(exe, format)
}

/// Refuses an old engine whose probe report names no row format, or another one than
/// this program reads.
fn refuse_rows(exe: &Path, format: Option<u64>) -> anyhow::Result<()> {
    if format != Some(u64::from(ROWS_FORMAT)) {
        anyhow::bail!(
            "the old engine {} writes {}, and this run reads compact rows of format \
             {ROWS_FORMAT}; it was built before that row format, so its results cannot be \
             cached",
            exe.display(),
            format.map_or_else(
                || "no compact rows".to_string(),
                |f| format!("compact rows of format {f}")
            )
        );
    }
    Ok(())
}

/// Plays `suite` with the old engine into the cache folder, which it resumes.
fn play(
    req: &Request<'_>,
    exe: &Path,
    content: &Path,
    dir: &Path,
    suite: Suite,
) -> anyhow::Result<()> {
    let mut cmd = calibrate(exe, content);
    cmd.args(["--seed", &req.seed.to_string()])
        .args(["--matches", &req.matches.to_string()])
        .args(["--minutes", &req.minutes.to_string()])
        .args(["--suite", suite.code()])
        .args(["--jobs", &req.jobs.to_string()])
        .arg("--out")
        .arg(dir);
    if suite == Suite::Formations {
        for name in req.pairings {
            cmd.arg("--pairing").arg(name);
        }
    }
    let status = cmd
        .status()
        .with_context(|| format!("cannot start the old engine {}", exe.display()))?;
    if !matches!(status.code(), Some(0 | 2)) {
        anyhow::bail!(
            "the old engine stopped with exit {:?} in the {} suite",
            status.code(),
            suite.code()
        );
    }
    Ok(())
}

/// The old engine's `calibrate` command on `content`: warnings only, no standard output.
fn calibrate(exe: &Path, content: &Path) -> Command {
    let mut cmd = Command::new(exe);
    cmd.arg("--content-dir")
        .arg(content)
        .arg("calibrate")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    if std::env::var_os("SM_LOG").is_none() {
        cmd.env("SM_LOG", "warn");
    }
    cmd
}

/// SHA-256, as 12 hex characters, over every file of a folder: its path inside the folder
/// and its bytes, in path order.
pub fn folder_hash(dir: &Path) -> anyhow::Result<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) -> std::io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                walk(root, &path, out)?;
            } else {
                let relative = path.strip_prefix(root).unwrap_or(&path);
                out.push((relative.to_string_lossy().replace('\\', "/"), path));
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(dir, dir, &mut files).with_context(|| format!("cannot read {}", dir.display()))?;
    files.sort();
    let mut hasher = Sha256::new();
    for (name, path) in files {
        if name == ".complete" {
            continue;
        }
        let bytes = fs::read(&path).with_context(|| format!("cannot read {}", path.display()))?;
        hasher.update((name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    Ok(engine::data::hex12(&hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibrate::run_folder::{Done, LedgerWriter, UnitLine};

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("engine-cli-base-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn key(n: u64) -> FixtureKey {
        FixtureKey::parse(&format!("{n:016x}")).unwrap()
    }

    #[test]
    fn a_planned_fixture_is_a_hit_only_with_the_same_engine_seed() {
        let dir = temp("hits");
        let mut ledger = LedgerWriter::open(&dir, "s", "equal", 0).unwrap();
        ledger
            .append(&UnitLine {
                suite: "equal".into(),
                unit: 0,
                fixtures: vec![
                    Done {
                        key: key(1),
                        seed: 11,
                        match_id: "a".into(),
                        session: String::new(),
                    },
                    Done {
                        key: key(2),
                        seed: 22,
                        match_id: "b".into(),
                        session: String::new(),
                    },
                ],
            })
            .unwrap();
        assert_eq!(
            hits(&dir, &[(key(1), 11), (key(2), 22), (key(3), 33)]).unwrap(),
            2
        );
        assert_eq!(hits(&dir, &[(key(1), 12)]).unwrap(), 0);
        assert_eq!(hits(&dir.join("empty"), &[(key(1), 11)]).unwrap(), 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_cache_is_the_rows_format_and_an_old_engine_without_rows_is_refused() {
        assert_eq!(CACHE_FORMAT, "v2");
        let exe = Path::new("old-engine");
        let err = refuse_rows(exe, None).unwrap_err().to_string();
        assert!(err.contains("no compact rows"), "{err}");
        let err = refuse_rows(exe, Some(9)).unwrap_err().to_string();
        assert!(err.contains("compact rows of format 9"), "{err}");
        refuse_rows(exe, Some(u64::from(ROWS_FORMAT))).unwrap();
    }

    #[test]
    fn the_folder_hash_follows_names_and_bytes() {
        let dir = temp("hash");
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("a.json"), "1").unwrap();
        fs::write(dir.join("sub/b.json"), "2").unwrap();
        let first = folder_hash(&dir).unwrap();
        assert_eq!(first.len(), 12);
        fs::write(dir.join(".complete"), "x").unwrap();
        assert_eq!(
            folder_hash(&dir).unwrap(),
            first,
            "the marker is not content"
        );
        fs::write(dir.join("sub/b.json"), "3").unwrap();
        assert_ne!(folder_hash(&dir).unwrap(), first);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_broken_build_names_its_error() {
        let root = temp("broken");
        let revision = Revision {
            repo: root.clone(),
            cache_root: root.clone(),
            key: CacheKey {
                commit: "0123456789abcdef0123456789abcdef01234567".into(),
                toolchain: "1.92.0 x".into(),
                target: "x86_64-pc-windows-msvc".into(),
                profile: "release".into(),
                features: String::new(),
            },
        };
        let err = build("main", &revision, true).unwrap_err().to_string();
        assert!(
            err.contains("error[E0425]: injected base build failure"),
            "{err}"
        );
        assert!(err.contains("no match was played"), "{err}");
        let _ = fs::remove_dir_all(&root);
    }
}

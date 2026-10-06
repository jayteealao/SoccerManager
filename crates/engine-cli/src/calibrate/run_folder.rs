//! The folder of a calibration run: `run.json` holds the run's identity, and `ledger/`
//! holds one line per finished work unit, so a stopped run resumes and a bigger run grows
//! without playing a match twice or losing one.
//!
//! A run's identity is everything that changes a match's result: the build, the content,
//! the flags, the seed, the minutes, the strength boost, the random-number scheme, the
//! fixture scheme, the measure definitions, and the band registry's version. The match
//! count, the suites, the pairings, the bands judged, the workers, and the event files kept
//! only choose which work is done, so they are not part of it.
//!
//! Each worker appends to its own ledger file, `ledger/<session>-<suite>-<shard>.jsonl`,
//! after every match of a unit has its statistics file, and syncs the file. A unit cut off
//! mid-way has no line, so it plays again; a trailing partial line is ignored.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::fixtures::FixtureKey;

/// The file that holds the run's identity.
pub const RUN_FILE: &str = "run.json";
/// The version of [`RunFile`] this program writes and reads.
pub const RUN_FILE_VERSION: u32 = 1;
/// Fixtures per work unit, at most.
pub const UNIT: usize = 8;

/// The fixtures per work unit for `remaining` fixtures over `shards` workers: [`UNIT`], or
/// fewer when there are too few fixtures to give every worker a unit, so a short suite
/// still keeps every worker busy. The parent and the workers cut with the same size.
pub fn unit_size(remaining: usize, shards: u32) -> usize {
    let shards = usize::try_from(shards.max(1)).unwrap_or(usize::MAX);
    remaining.div_ceil(shards).clamp(1, UNIT)
}
/// Where a run with another identity moves the old run's files.
pub const SUPERSEDED: &str = "superseded";
/// The files and folders a run owns in its folder.
const OWNED: [&str; 6] = [RUN_FILE, "ledger", "stats", "events", "arms", "report.json"];

/// What makes a run's results what they are.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunIdentity {
    pub build: Build,
    /// SHA-256 over the content files' digest and the slot file.
    pub content: String,
    /// The fixtures hash: the inputs that decide the fixtures.
    pub fixtures: String,
    /// The flag states the command line set.
    pub flags: BTreeMap<String, String>,
    /// The flag a paired run compares.
    pub pair: Option<String>,
    pub seed: u64,
    pub minutes: u32,
    /// The strength suite's attribute boost, from the bands file.
    pub strength_boost: f64,
    /// The engine's random-number scheme.
    pub rng_scheme: u8,
    pub fixture_scheme: String,
    /// The version of the measure definitions.
    pub measures: u32,
    /// The band registry's version: the bands file's `schema_version`.
    pub registry: u32,
}

/// The program that plays the run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    /// The short commit, with `-dirty` when the working tree held changes.
    pub hash: String,
    /// The SHA-256 of the running executable: it changes with every uncommitted change.
    pub executable_sha256: String,
}

impl RunIdentity {
    /// Each part of the identity that differs from `new`, named, as `<part> <old> -> <new>`.
    pub fn differences(&self, new: &Self) -> Vec<String> {
        let mut out = Vec::new();
        let mut part = |name: &str, old: String, new: String| {
            if old != new {
                out.push(format!("{name} {old} -> {new}"));
            }
        };
        let build = |b: &Build| {
            format!(
                "{} (executable {})",
                b.hash,
                b.executable_sha256
                    .get(..12)
                    .unwrap_or(&b.executable_sha256)
            )
        };
        part("build", build(&self.build), build(&new.build));
        part("content", self.content.clone(), new.content.clone());
        part("fixtures", self.fixtures.clone(), new.fixtures.clone());
        part("flags", flags(&self.flags), flags(&new.flags));
        let pair = |p: &Option<String>| p.clone().unwrap_or_else(|| "none".into());
        part("pair", pair(&self.pair), pair(&new.pair));
        part("seed", self.seed.to_string(), new.seed.to_string());
        part("minutes", self.minutes.to_string(), new.minutes.to_string());
        part(
            "strength boost",
            self.strength_boost.to_string(),
            new.strength_boost.to_string(),
        );
        part(
            "random-number scheme",
            self.rng_scheme.to_string(),
            new.rng_scheme.to_string(),
        );
        part(
            "fixture scheme",
            self.fixture_scheme.clone(),
            new.fixture_scheme.clone(),
        );
        part(
            "measures",
            self.measures.to_string(),
            new.measures.to_string(),
        );
        part(
            "registry",
            self.registry.to_string(),
            new.registry.to_string(),
        );
        out
    }
}

fn flags(flags: &BTreeMap<String, String>) -> String {
    if flags.is_empty() {
        return "none".into();
    }
    flags
        .iter()
        .map(|(n, s)| format!("{n}={s}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// The `run.json` of a run folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunFile {
    pub version: u32,
    #[serde(rename = "run.id")]
    pub run_id: String,
    /// The run's start, in Unix milliseconds: part of every match identifier.
    pub millis: u64,
    pub identity: RunIdentity,
}

/// How the run folder was found.
#[derive(Debug, Clone, PartialEq)]
pub enum Opened {
    /// No run was there: a new run starts.
    New,
    /// The same identity: the run resumes.
    Resumed,
    /// Another identity: its files moved to `superseded/<old run id>/`, and a new run starts.
    Superseded {
        old_run_id: String,
        differences: Vec<String>,
    },
}

/// The run a folder holds after [`open`].
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub run_id: String,
    pub millis: u64,
    pub opened: Opened,
}

/// Opens the run folder `dir` for a run of `identity`: resumes the run there when its
/// identity is the same, moves it to `superseded/` when it differs, and otherwise starts
/// the run `run_id` begun at `millis`. A `run.json` this program cannot read is refused.
pub fn open(
    dir: &Path,
    identity: &RunIdentity,
    run_id: &str,
    millis: u64,
) -> anyhow::Result<Session> {
    let path = dir.join(RUN_FILE);
    let mut opened = Opened::New;
    if path.exists() {
        let old = read_run_file(&path)?;
        if old.identity == *identity {
            return Ok(Session {
                run_id: old.run_id,
                millis: old.millis,
                opened: Opened::Resumed,
            });
        }
        move_superseded(dir, &old.run_id)?;
        opened = Opened::Superseded {
            differences: old.identity.differences(identity),
            old_run_id: old.run_id,
        };
    }
    fs::create_dir_all(dir)
        .with_context(|| format!("cannot create the run folder {}", dir.display()))?;
    let file = RunFile {
        version: RUN_FILE_VERSION,
        run_id: run_id.to_string(),
        millis,
        identity: identity.clone(),
    };
    let partial = dir.join(format!("{RUN_FILE}.partial"));
    fs::write(&partial, serde_json::to_vec_pretty(&file)?)
        .and_then(|()| fs::rename(&partial, &path))
        .with_context(|| format!("cannot write {}", path.display()))?;
    Ok(Session {
        run_id: run_id.to_string(),
        millis,
        opened,
    })
}

fn read_run_file(path: &Path) -> anyhow::Result<RunFile> {
    let refuse = |why: String| {
        anyhow::anyhow!(
            "the run folder's {} cannot be read: {why}; move the folder away or choose \
             another --out folder",
            path.display()
        )
    };
    let text = fs::read_to_string(path).map_err(|e| refuse(e.to_string()))?;
    let value: Value = serde_json::from_str(&text).map_err(|e| refuse(e.to_string()))?;
    match value["version"].as_u64() {
        Some(v) if v == u64::from(RUN_FILE_VERSION) => {}
        v => {
            return Err(refuse(format!(
                "its version is {}, and this program reads version {RUN_FILE_VERSION}",
                v.map_or_else(|| "missing".into(), |v| v.to_string())
            )));
        }
    }
    serde_json::from_value(value).map_err(|e| refuse(e.to_string()))
}

/// Moves the files of the run `old_run_id` in `dir` to `dir/superseded/<old_run_id>/`.
pub fn move_superseded(dir: &Path, old_run_id: &str) -> anyhow::Result<PathBuf> {
    let to = dir.join(SUPERSEDED).join(old_run_id);
    fs::create_dir_all(&to).with_context(|| format!("cannot create {}", to.display()))?;
    for name in OWNED {
        let from = dir.join(name);
        if from.exists() {
            fs::rename(&from, to.join(name))
                .with_context(|| format!("cannot move {} to {}", from.display(), to.display()))?;
        }
    }
    Ok(to)
}

/// One finished fixture of a ledger line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Done {
    pub key: FixtureKey,
    /// The engine seed the match was played with.
    pub seed: u64,
    /// The match identifier: the name of its statistics file.
    #[serde(rename = "match")]
    pub match_id: String,
}

/// One ledger line: a finished work unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitLine {
    pub suite: String,
    /// The unit's place among the units of its suite in its session.
    pub unit: u32,
    pub fixtures: Vec<Done>,
}

/// Every finished fixture in the ledger of `dir`, by key. The ledger files of the session
/// `skip` are left out: a worker reads only what earlier sessions finished, so every
/// worker of a session cuts the same units. A trailing partial line is ignored; a key
/// finished twice is an error that names it.
pub fn finished(dir: &Path, skip: Option<&str>) -> anyhow::Result<BTreeMap<FixtureKey, Done>> {
    let ledger = dir.join("ledger");
    let mut files: Vec<PathBuf> = match fs::read_dir(&ledger) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
            .filter(|p| {
                let name = p.file_name().map(|n| n.to_string_lossy().into_owned());
                skip.is_none_or(|s| !name.is_some_and(|n| n.starts_with(&format!("{s}-"))))
            })
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e).with_context(|| format!("cannot read {}", ledger.display())),
    };
    files.sort();
    let mut out = BTreeMap::new();
    for file in files {
        let text =
            fs::read_to_string(&file).with_context(|| format!("cannot read {}", file.display()))?;
        // The text after the last newline is a line cut off mid-write.
        let complete = text.rfind('\n').map_or("", |end| &text[..end]);
        for (n, line) in complete
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
        {
            let unit: UnitLine = serde_json::from_str(line).with_context(|| {
                format!(
                    "line {} of the ledger file {} cannot be read",
                    n + 1,
                    file.display()
                )
            })?;
            for done in unit.fixtures {
                if let Some(first) = out.insert(done.key, done) {
                    anyhow::bail!(
                        "the ledger of {} finishes the fixture {} twice; the run folder is \
                         damaged, so start the run in another folder",
                        dir.display(),
                        first.key
                    );
                }
            }
        }
    }
    Ok(out)
}

/// A worker's ledger file.
pub struct LedgerWriter {
    file: fs::File,
}

impl LedgerWriter {
    /// Opens `dir/ledger/<session>-<suite>-<shard>.jsonl` for appending.
    pub fn open(dir: &Path, session: &str, suite: &str, shard: u32) -> anyhow::Result<Self> {
        let ledger = dir.join("ledger");
        fs::create_dir_all(&ledger)
            .with_context(|| format!("cannot create {}", ledger.display()))?;
        let path = ledger.join(format!("{session}-{suite}-{shard}.jsonl"));
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("cannot open {}", path.display()))?;
        Ok(Self { file })
    }

    /// Appends one finished unit as one line and syncs it to disk.
    pub fn append(&mut self, unit: &UnitLine) -> anyhow::Result<()> {
        let mut line = serde_json::to_vec(unit)?;
        line.push(b'\n');
        self.file.write_all(&line)?;
        self.file.sync_data()?;
        Ok(())
    }
}

/// The keys of a statistics record that differ between two plays of the same match: its
/// identifier, its owner, and its timing.
const VOLATILE: [&str; 4] = ["match.id", "owner.id", "duration_ms", "engine.ticks_per_s"];

/// SHA-256, as 64 hex characters, over every arm's statistics records in key order, with
/// the identifier, owner, and timing keys removed: equal results give equal digests however
/// the run was cut into sessions.
pub fn results_digest(arms: &[(String, Vec<(FixtureKey, Value)>)]) -> String {
    let mut hasher = Sha256::new();
    for (arm, records) in arms {
        hasher.update(format!("arm {arm}\n"));
        let mut sorted: Vec<&(FixtureKey, Value)> = records.iter().collect();
        sorted.sort_by_key(|(key, _)| *key);
        for (key, record) in sorted {
            let mut record = record.clone();
            if let Some(map) = record.as_object_mut() {
                for name in VOLATILE {
                    map.remove(name);
                }
            }
            hasher.update(format!("{key} {record}\n"));
        }
    }
    stream::record::hex(&hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn identity() -> RunIdentity {
        RunIdentity {
            build: Build {
                hash: "1442fad".into(),
                executable_sha256: "ab".repeat(32),
            },
            content: "c".repeat(64),
            fixtures: "f".repeat(12),
            flags: BTreeMap::new(),
            pair: None,
            seed: 42,
            minutes: 90,
            strength_boost: 1.15,
            rng_scheme: 2,
            fixture_scheme: "fixture-key-1".into(),
            measures: 1,
            registry: 2,
        }
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "engine-cli-run-folder-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn key(n: u64) -> FixtureKey {
        FixtureKey::parse(&format!("{n:016x}")).unwrap()
    }

    #[test]
    fn a_short_suite_is_cut_into_smaller_units_so_every_worker_has_one() {
        // 20 fixtures over 8 workers: units of 3 give 7 workers a unit, not 3.
        assert_eq!(unit_size(20, 8), 3);
        assert_eq!(20usize.div_ceil(unit_size(20, 8)), 7);
        // Enough fixtures: full units.
        assert_eq!(unit_size(1000, 8), UNIT);
        assert_eq!(unit_size(20, 2), UNIT);
        // Edges: no fixtures, one worker, no worker count.
        assert_eq!(unit_size(0, 8), 1);
        assert_eq!(unit_size(5, 1), 5);
        assert_eq!(unit_size(5, 0), 5);
    }

    fn line(suite: &str, unit: u32, keys: &[u64]) -> UnitLine {
        UnitLine {
            suite: suite.into(),
            unit,
            fixtures: keys
                .iter()
                .map(|&k| Done {
                    key: key(k),
                    seed: k * 10,
                    match_id: format!("{k:016x}-1"),
                })
                .collect(),
        }
    }

    #[test]
    fn equal_identities_differ_in_nothing_and_each_changed_part_is_named() {
        let a = identity();
        assert!(a.differences(&a.clone()).is_empty());
        type Change = (&'static str, Box<dyn Fn(&mut RunIdentity)>);
        let changes: [Change; 12] = [
            (
                "build",
                Box::new(|i| i.build.executable_sha256 = "cd".repeat(32)),
            ),
            ("content", Box::new(|i| i.content = "d".repeat(64))),
            ("fixtures", Box::new(|i| i.fixtures = "e".repeat(12))),
            (
                "flags",
                Box::new(|i| {
                    i.flags.insert("probe".into(), "on".into());
                }),
            ),
            ("pair", Box::new(|i| i.pair = Some("probe".into()))),
            ("seed", Box::new(|i| i.seed = 7)),
            ("minutes", Box::new(|i| i.minutes = 5)),
            ("strength boost", Box::new(|i| i.strength_boost = 1.2)),
            ("random-number scheme", Box::new(|i| i.rng_scheme = 3)),
            (
                "fixture scheme",
                Box::new(|i| i.fixture_scheme = "x".into()),
            ),
            ("measures", Box::new(|i| i.measures = 2)),
            ("registry", Box::new(|i| i.registry = 3)),
        ];
        for (name, change) in changes {
            let mut b = a.clone();
            change(&mut b);
            let named = a.differences(&b);
            assert_eq!(named.len(), 1, "{name}: {named:?}");
            assert!(
                named[0].starts_with(&format!("{name} ")),
                "{name}: {named:?}"
            );
        }
        let mut b = a.clone();
        b.minutes = 5;
        assert_eq!(a.differences(&b), ["minutes 90 -> 5"]);
    }

    #[test]
    fn a_folder_starts_resumes_and_moves_a_run_of_another_identity_aside() {
        let dir = temp("open");
        let first = open(&dir, &identity(), "calib-1", 100).unwrap();
        assert_eq!(first.opened, Opened::New);
        fs::create_dir_all(dir.join("stats")).unwrap();
        fs::write(dir.join("stats/a.json"), "{}").unwrap();

        let again = open(&dir, &identity(), "calib-2", 200).unwrap();
        assert_eq!((again.run_id.as_str(), again.millis), ("calib-1", 100));
        assert_eq!(again.opened, Opened::Resumed);

        let mut other = identity();
        other.minutes = 5;
        let moved = open(&dir, &other, "calib-3", 300).unwrap();
        assert_eq!(moved.run_id, "calib-3");
        assert_eq!(
            moved.opened,
            Opened::Superseded {
                old_run_id: "calib-1".into(),
                differences: vec!["minutes 90 -> 5".into()],
            }
        );
        assert!(dir.join("superseded/calib-1/stats/a.json").is_file());
        assert!(dir.join("superseded/calib-1/run.json").is_file());
        assert!(!dir.join("stats").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_run_file_of_an_unknown_version_is_refused() {
        let dir = temp("version");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(RUN_FILE), json!({"version": 9}).to_string()).unwrap();
        let err = open(&dir, &identity(), "calib-1", 1)
            .unwrap_err()
            .to_string();
        assert!(err.contains("version is 9"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_ledger_joins_its_files_skips_a_partial_line_and_a_session_and_refuses_a_twice_key() {
        let dir = temp("ledger");
        let mut a = LedgerWriter::open(&dir, "s1", "equal", 0).unwrap();
        a.append(&line("equal", 0, &[1, 2])).unwrap();
        let mut b = LedgerWriter::open(&dir, "s1", "equal", 1).unwrap();
        b.append(&line("equal", 1, &[3])).unwrap();
        let mut c = LedgerWriter::open(&dir, "s2", "equal", 0).unwrap();
        c.append(&line("equal", 0, &[4])).unwrap();
        // A line cut off mid-write.
        let path = dir.join("ledger/s2-equal-0.jsonl");
        let mut text = fs::read_to_string(&path).unwrap();
        text.push_str("{\"suite\":\"equal\",\"unit\":1,\"fix");
        fs::write(&path, text).unwrap();

        let all = finished(&dir, None).unwrap();
        assert_eq!(
            all.keys().copied().collect::<Vec<_>>(),
            [key(1), key(2), key(3), key(4)]
        );
        assert_eq!(all[&key(2)].seed, 20);
        let earlier = finished(&dir, Some("s2")).unwrap();
        assert_eq!(earlier.len(), 3);
        assert!(finished(&temp("none"), None).unwrap().is_empty());

        let mut d = LedgerWriter::open(&dir, "s3", "equal", 0).unwrap();
        d.append(&line("equal", 0, &[2])).unwrap();
        let err = finished(&dir, None).unwrap_err().to_string();
        assert!(err.contains(&key(2).to_string()), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_digest_ignores_identifiers_and_timing_and_follows_a_goal() {
        let record = |id: &str, ms: u64, goals: u32| {
            json!({"match.id": id, "owner.id": "o", "duration_ms": ms,
                   "engine.ticks_per_s": ms as f64, "goals": [goals, 0], "seed": 1})
        };
        let one = [(
            "".to_string(),
            vec![(key(1), record("a", 5, 1)), (key(2), record("b", 6, 0))],
        )];
        let two = [(
            "".to_string(),
            vec![(key(2), record("y", 9, 0)), (key(1), record("x", 7, 1))],
        )];
        assert_eq!(results_digest(&one), results_digest(&two));
        assert_eq!(results_digest(&one).len(), 64);
        let goal = [(
            "".to_string(),
            vec![(key(1), record("a", 5, 2)), (key(2), record("b", 6, 0))],
        )];
        assert_ne!(results_digest(&one), results_digest(&goal));
    }
}

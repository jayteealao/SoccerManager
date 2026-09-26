//! `engine-cli gate`: plays the replay-gate fixtures, prints one line per match, and exits 0
//! when every match has its golden hashes, 2 when one differs, and 1 when the golden file or
//! the content cannot be read or is not valid. Its write modes (`--bootstrap`,
//! `--regenerate`, `--add-machine-set`) check their flags and reason before any file is read
//! and write through a temporary file, so a refusal or a failed match leaves the golden file
//! byte-identical. The hashing, the compare, and the report text
//! live in `engine::gate`, which the engine's fault tests use as well.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, bail};
use engine::ContentDir;
use engine::gate::golden::{self, GoldenFile};
use engine::gate::{self, Fixture, Inputs, MatchHashes, PackInputs, Played, Verdict};
use script::LoadedPack;

use crate::cli::GateOpts;

/// What a gate run does with the golden file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Compare,
    Bootstrap,
    Regenerate,
    AddMachineSet,
}

/// The run's mode and the reason to record, checked before any file is read or match is
/// played, so a refusal leaves the golden file untouched.
fn mode(opts: &GateOpts) -> anyhow::Result<(Mode, String)> {
    let modes = [
        (opts.bootstrap, "--bootstrap", Mode::Bootstrap),
        (opts.regenerate, "--regenerate", Mode::Regenerate),
        (
            opts.add_machine_set,
            "--add-machine-set",
            Mode::AddMachineSet,
        ),
    ];
    let set: Vec<_> = modes.iter().filter(|(on, _, _)| *on).collect();
    if set.len() > 1 {
        bail!("{} and {} cannot be used together", set[0].1, set[1].1);
    }
    let Some(&&(_, flag, mode)) = set.first() else {
        if opts.reason.is_some() {
            bail!("--reason goes with --regenerate, --add-machine-set, or --bootstrap");
        }
        return Ok((Mode::Compare, String::new()));
    };
    if !opts.fixture.is_empty() {
        bail!("{flag} plays every fixture; leave out --fixture");
    }
    let reason = opts.reason.as_deref().map(str::trim).unwrap_or_default();
    if !reason.is_empty() {
        return Ok((mode, reason.to_string()));
    }
    match mode {
        Mode::Bootstrap => Ok((mode, golden::BOOTSTRAP_REASON.to_string())),
        Mode::Regenerate => bail!("--regenerate needs --reason \"<why the hashes change>\""),
        _ => bail!("--add-machine-set needs --reason \"<why this machine's set is added>\""),
    }
}

pub fn run(content_dir: Option<&Path>, opts: &GateOpts) -> anyhow::Result<i32> {
    let (mode, reason) = mode(opts)?;
    let all = gate::fixtures();
    let selected = select(&all, &opts.fixture)?;
    let path = opts
        .golden
        .clone()
        .unwrap_or_else(|| PathBuf::from(golden::DEFAULT_PATH));
    let machine = golden::machine_key();
    let mut old: Option<GoldenFile> = None;
    let expected: Option<Vec<MatchHashes>> = match mode {
        Mode::Bootstrap => {
            if path.exists() {
                return Err(golden::GoldenError::Exists {
                    path: path.display().to_string(),
                }
                .into());
            }
            None
        }
        Mode::Compare => {
            let file = golden::load(&path, &all)?;
            Some(file.set_for(&machine)?.to_vec())
        }
        // A regeneration reads the old file without the strict checks, so a gate-schema or
        // fixture change in the code can still be regenerated.
        Mode::Regenerate => {
            old = Some(golden::load_lenient(&path)?);
            None
        }
        Mode::AddMachineSet => {
            let file = golden::load(&path, &all)?;
            if file.hash_sets.contains_key(&machine) {
                return Err(golden::GoldenError::SetExists { machine }.into());
            }
            old = Some(file);
            None
        }
    };

    let loaded = crate::content::load(content_dir, None, None, None)?;
    let needs_pack = selected.iter().any(|f| f.pack.is_some());
    let pack = if needs_pack {
        let dir = ContentDir::resolve(content_dir)?;
        let pack_dir = dir.path(&format!("scripts/{}", gate::KNOCKOUT_PACK));
        // The real wall clock: a slow hook call is not stopped, it only marks the match, so a
        // busy machine plays the same gate match and prints a warning.
        Some(
            LoadedPack::load(&pack_dir)
                .with_context(|| format!("the {} script pack", gate::KNOCKOUT_PACK))?,
        )
    } else {
        None
    };
    let hooks = || pack.as_ref().map(LoadedPack::plugins).unwrap_or_default();
    let inputs = Inputs {
        content: &loaded.content,
        teams: [&loaded.teams[0], &loaded.teams[1]],
        pack: pack.as_ref().map(|p| PackInputs {
            id: &p.pack.manifest.id,
            sha: *p.sha(),
            hooks: &hooks,
        }),
    };

    let started = Instant::now();
    let mut code = 0;
    let mut written = Vec::new();
    for fixture in &selected {
        let played = match gate::play_fixture(fixture, &inputs) {
            Ok(played) => played,
            Err(err) => {
                // A NaN or an infinity in the hashed state fails the gate like a difference.
                code = 2;
                emit_error(opts.json, fixture, &err.to_string());
                continue;
            }
        };
        let verdict = match &expected {
            None => Verdict::Same,
            Some(set) => {
                let golden = set
                    .iter()
                    .find(|m| m.id == fixture.id)
                    .expect("the strict load found every fixture");
                gate::compare(golden, &played.hashes)
            }
        };
        if verdict != Verdict::Same {
            code = 2;
        }
        emit(opts.json, fixture, &played, verdict);
        // A marked match warns and is still compared; the verdict above follows the hashes.
        if let Some(warning) = gate::warning_line(fixture, &played) {
            eprintln!("{warning}");
        }
        written.push(played.hashes);
    }
    let seconds = started.elapsed().as_secs_f64();
    let played = selected.len();
    if mode != Mode::Compare && code != 0 {
        bail!("a match failed; the golden file was not written");
    }
    match (mode, old) {
        (Mode::Compare, _) => {
            let failed = if code == 0 {
                "none differ"
            } else {
                "some differ"
            };
            eprintln!("gate: {played} matches for {machine} in {seconds:.1} s; {failed}");
        }
        (Mode::Bootstrap, _) => {
            GoldenFile::first(&all, &machine, written, &reason).write_new(&path)?;
            eprintln!(
                "wrote {}: {played} matches for {machine} in {seconds:.1} s",
                path.display()
            );
        }
        (Mode::Regenerate, Some(old)) => {
            let unchanged = old.hash_sets.get(&machine) == Some(&written);
            let (file, dropped) = old.regenerated(&all, &machine, written, &reason);
            file.write_replace(&path)?;
            eprintln!(
                "wrote {}: regenerated {played} matches for {machine} in {seconds:.1} s{}",
                path.display(),
                if unchanged { "; hashes unchanged" } else { "" }
            );
            if !dropped.is_empty() {
                eprintln!(
                    "dropped the stale hash sets of {}; add each again with `engine-cli gate --add-machine-set --reason <TEXT>` on that machine",
                    dropped.join(", ")
                );
            }
        }
        (Mode::AddMachineSet, Some(old)) => {
            let file = old.with_machine_set(&machine, written, &reason)?;
            file.write_replace(&path)?;
            let differ: usize = file.set_differences.iter().map(|d| d.differ.len()).sum();
            eprintln!(
                "wrote {}: added {played} matches for {machine} in {seconds:.1} s; {differ} differ from the other sets",
                path.display()
            );
        }
        (_, None) => unreachable!("the write modes read the old file"),
    }
    Ok(code)
}

/// The fixtures `ids` names, in gate order, or all of them when `ids` is empty.
fn select<'a>(all: &'a [Fixture], ids: &[String]) -> anyhow::Result<Vec<&'a Fixture>> {
    if let Some(unknown) = ids.iter().find(|id| !all.iter().any(|f| &f.id == *id)) {
        let known: Vec<&str> = all.iter().map(|f| f.id.as_str()).collect();
        bail!(
            "no gate fixture is named {unknown}; the fixtures are: {}",
            known.join(", ")
        );
    }
    Ok(all
        .iter()
        .filter(|f| ids.is_empty() || ids.contains(&f.id))
        .collect())
}

fn emit(json: bool, fixture: &Fixture, played: &Played, verdict: Verdict) {
    if !json {
        println!("{}", gate::report_line(fixture, played, verdict));
        return;
    }
    let (word, window, detail) = match verdict {
        Verdict::Same => ("match", None, None),
        Verdict::Differs { from, to } => (
            "differs",
            Some(serde_json::json!({ "from": from, "to": to })),
            None,
        ),
        Verdict::TickCount { expected, actual } => (
            "differs",
            None,
            Some(format!(
                "the golden file has {expected} ticks, this run played {actual}"
            )),
        ),
    };
    let f = &played.facts;
    let line = serde_json::json!({
        "fixture": fixture.id,
        "verdict": word,
        "ticks": played.hashes.ticks,
        "final_hash": played.hashes.final_hash,
        "window": window,
        "detail": detail,
        "extra_time": f.extra_time,
        "shootout": f.shootout,
        "decided_by": f.decided_by.map(|d| d.code()),
        "substitutions_applied": f.substitutions,
        "tactics_changes_applied": f.tactics_changes,
        "slow_calls": f.slow_calls,
        "invalid": (f.slow_calls > 0).then_some(engine::plugin::INVALID_SLOW_SCRIPT),
    });
    println!("{line}");
}

fn emit_error(json: bool, fixture: &Fixture, detail: &str) {
    if json {
        let line = serde_json::json!({
            "fixture": fixture.id,
            "verdict": "error",
            "detail": detail,
        });
        println!("{line}");
    } else {
        println!("{:<26} error    {detail}", fixture.id);
    }
}

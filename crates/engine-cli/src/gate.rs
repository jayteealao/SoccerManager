//! `engine-cli gate`: plays the replay-gate fixtures, prints one line per match, and exits 0
//! when every match has its golden hashes, 2 when one differs, and 1 when the golden file or
//! the content cannot be read or is not valid. The hashing, the compare, and the report text
//! live in `engine::gate`, which the engine's fault tests use as well.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, bail};
use engine::ContentDir;
use engine::gate::golden::{self, GoldenFile};
use engine::gate::{self, Fixture, Inputs, MatchHashes, PackInputs, Played, Verdict};
use script::{Backstop, LoadedPack};

use crate::cli::GateOpts;

pub fn run(content_dir: Option<&Path>, opts: &GateOpts) -> anyhow::Result<i32> {
    let all = gate::fixtures();
    if opts.bootstrap && !opts.fixture.is_empty() {
        bail!("--bootstrap plays every fixture; leave out --fixture");
    }
    let selected = select(&all, &opts.fixture)?;
    let path = opts
        .golden
        .clone()
        .unwrap_or_else(|| PathBuf::from(golden::DEFAULT_PATH));
    let machine = golden::machine_key();
    let expected: Option<Vec<MatchHashes>> = if opts.bootstrap {
        if path.exists() {
            return Err(golden::GoldenError::Exists {
                path: path.display().to_string(),
            }
            .into());
        }
        None
    } else {
        let file = golden::load(&path, &all)?;
        Some(file.set_for(&machine)?.to_vec())
    };

    let loaded = crate::content::load(content_dir, None, None, None)?;
    let needs_pack = selected.iter().any(|f| f.pack.is_some());
    let pack = if needs_pack {
        let dir = ContentDir::resolve(content_dir)?;
        let pack_dir = dir.path(&format!("scripts/{}", gate::KNOCKOUT_PACK));
        // The gate clock: no hook call is stopped on time, so a busy machine cannot change a
        // gate match. The operation budget still stops a long call.
        Some(
            LoadedPack::load_with(&pack_dir, Backstop::Never)
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
        written.push(played.hashes);
    }
    let seconds = started.elapsed().as_secs_f64();
    if opts.bootstrap {
        if code != 0 {
            bail!("the bootstrap run failed a match; no golden file was written");
        }
        GoldenFile::first(&all, written).write_new(&path)?;
        eprintln!(
            "wrote {}: {} matches for {machine} in {seconds:.1} s",
            path.display(),
            selected.len()
        );
    } else {
        let failed = if code == 0 {
            "none differ"
        } else {
            "some differ"
        };
        eprintln!(
            "gate: {} matches for {machine} in {seconds:.1} s; {failed}",
            selected.len()
        );
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

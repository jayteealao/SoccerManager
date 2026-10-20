//! `engine-cli calibrate proof`: the gate's pass and fail rates, measured.
//!
//! The proof plays ordinary change runs of this executable against itself, each a child
//! process into its own run folder under `--out`: self runs on new run seeds in which the
//! changed engine plays new match seeds (`--reseed`) and the old engine the planned ones, and
//! runs of each broken copy that `gate-proof.json` declares, its flag on against the same
//! engine with it off. The copies are written into a copy of the content folder as flags,
//! state off, so the old engine plays exactly the shipped content. A stopped proof resumes:
//! each finished run is judged again from its stored rows with no match played. After the
//! last run the proof reads every report, writes the rates into `gate-proof.json` with the
//! identity of the gate it measured, and prints them. The rates never change the exit code.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Context;
use engine::ContentDir;
use engine::observe::identity::data_dir;
use serde_json::Value;

use crate::report::bands::Registry;
use crate::report::proof::{self, BrokenCopy, PROOF_FILE, Proof, ProofBuild, RunMeta};
use crate::report::real;

/// What `calibrate proof` was asked for.
#[derive(Debug, Clone)]
pub struct Opts {
    pub seed: u64,
    pub matches: u32,
    pub out: Option<PathBuf>,
    pub write: Option<PathBuf>,
    pub real_data: Option<PathBuf>,
    pub self_runs: u32,
    pub copy_runs: u32,
    pub minutes: u32,
}

pub fn run(content_dir: Option<&Path>, o: &Opts) -> anyhow::Result<i32> {
    // The proof states whether a copy moved each whole-distribution band, and only a
    // measures build judges them all.
    if !cfg!(feature = "measures") {
        eprintln!("{}", super::NO_MEASURES);
        eprintln!(
            "calibrate proof needs a build with the measures feature: only it judges every \
             whole-distribution band"
        );
        return Ok(1);
    }
    let dir = ContentDir::resolve(content_dir)?;
    let registry = Registry::load(&dir)?;
    let gate = proof::current(&registry);
    let mut file = proof::read(&dir)
        .map_err(|e| anyhow::anyhow!("{PROOF_FILE} {e:#}"))?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no {PROOF_FILE} in the content folder {}: it declares the broken copies",
                dir.root().display()
            )
        })?;
    if file.copies.is_empty() {
        anyhow::bail!("{PROOF_FILE} declares no broken copy");
    }
    let out = o.out.clone().unwrap_or_else(|| {
        data_dir()
            .join("proofs")
            .join(format!("{}-{}", gate.registry, gate.verdict_code))
    });
    let write = o.write.clone().unwrap_or_else(|| dir.path(PROOF_FILE));
    let real_folder = real::resolve(o.real_data.as_deref(), dir.root());
    let content = out.join("content");
    write_content(dir.root(), &content, &file.copies)?;
    // nosemgrep: rust.lang.security.current-exe.current-exe -- the proof's runs play this build against itself
    let exe = std::env::current_exe().context("cannot find this program")?;
    let total = o.self_runs + o.copy_runs * file.copies.len() as u32;
    eprintln!(
        "gate proof: {total} change runs into {}; run the same command again to resume",
        out.display()
    );
    let child = Child {
        exe: &exe,
        content: &content,
        out: &out,
        opts: o,
        real: real_folder.path.as_deref(),
    };
    let mut done = 0;
    let mut self_reports = Vec::new();
    for i in 0..o.self_runs {
        let meta = RunMeta {
            name: format!("self-{i}"),
            seed: o.seed + u64::from(i),
            reseed: Some(u64::from(i) + 1),
        };
        done += 1;
        let report = child.run(&meta, None, (done, total))?;
        self_reports.push((meta, report));
    }
    let mut copy_reports = Vec::new();
    for copy in &file.copies {
        for j in 0..o.copy_runs {
            let meta = RunMeta {
                name: format!("{}-{j}", copy.name),
                seed: o.seed + u64::from(j),
                reseed: None,
            };
            done += 1;
            let report = child.run(&meta, Some(&copy.name), (done, total))?;
            copy_reports.push((copy.name.clone(), meta, report));
        }
    }
    let (self_runs, copies, distribution_bands, counts) = proof::summarise(
        &self_reports,
        &copy_reports,
        &file.copies,
        &proof::distribution_bands(&registry),
    );
    let made = Proof {
        gate,
        made_at: engine::gate::golden::utc_now(),
        build: ProofBuild {
            hash: engine::build_hash().to_string(),
            measures: true,
        },
        seed: o.seed,
        matches: o.matches,
        minutes: o.minutes,
        self_runs,
        copies,
        distribution_bands,
        counts,
    };
    eprint!("{}", proof::render(&made));
    tracing::info!(
        signal = "calibrate.proof",
        registry = %made.gate.registry,
        measures = made.gate.measures,
        verdict_code = %made.gate.verdict_code,
        runs = made.counts.runs,
        self_pass_rate = made.self_runs.pass_rate,
        matches = made.counts.total_matches
    );
    file.proof = Some(made);
    let text = serde_json::to_string_pretty(&file)? + "\n";
    std::fs::write(&write, text).with_context(|| format!("cannot write {}", write.display()))?;
    eprintln!("wrote {}", write.display());
    Ok(0)
}

/// How the proof starts one of its change runs.
struct Child<'a> {
    exe: &'a Path,
    content: &'a Path,
    out: &'a Path,
    opts: &'a Opts,
    real: Option<&'a Path>,
}

impl Child<'_> {
    /// Plays (or judges again) the change run `meta`, with `flag` on, and returns its report.
    fn run(
        &self,
        meta: &RunMeta,
        flag: Option<&str>,
        (n, total): (u32, u32),
    ) -> anyhow::Result<Value> {
        let run_dir = self.out.join(&meta.name);
        let log = self.out.join(format!("{}.log", meta.name));
        let mut cmd = Command::new(self.exe);
        cmd.arg("--content-dir")
            .arg(self.content)
            .arg("calibrate")
            .args(["--seed", &meta.seed.to_string()])
            .args(["--matches", &self.opts.matches.to_string()])
            .args(["--minutes", &self.opts.minutes.to_string()])
            .arg("--base-binary")
            .arg(self.exe)
            .arg("--out")
            .arg(&run_dir);
        if let Some(salt) = meta.reseed {
            cmd.args(["--reseed", &salt.to_string()]);
        }
        if let Some(flag) = flag {
            cmd.arg("--flag").arg(format!("{flag}=on"));
        }
        if let Some(real) = self.real {
            cmd.arg("--real-data").arg(real);
        }
        let log_file = std::fs::File::create(&log)
            .with_context(|| format!("cannot write {}", log.display()))?;
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log_file);
        eprintln!(
            "gate proof run {n} of {total}: {} (seed {})",
            meta.name, meta.seed
        );
        let status = cmd
            .status()
            .with_context(|| format!("cannot start {}", self.exe.display()))?;
        if !matches!(status.code(), Some(0 | 2)) {
            anyhow::bail!(
                "the proof's run {} stopped with exit {:?}; its log is {}",
                meta.name,
                status.code(),
                log.display()
            );
        }
        let path = run_dir.join("report.json");
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        serde_json::from_str(text.trim()).with_context(|| format!("cannot read {}", path.display()))
    }
}

/// Writes the proof's content folder: the content of `from` without its proof file, with
/// each broken copy declared in `tuning.json` as a flag, state off.
fn write_content(from: &Path, to: &Path, copies: &[BrokenCopy]) -> anyhow::Result<()> {
    let _ = std::fs::remove_dir_all(to);
    copy_tree(from, to)?;
    let tuning = to.join(engine::data::TUNING_FILE);
    let text = std::fs::read_to_string(&tuning)
        .with_context(|| format!("cannot read {}", tuning.display()))?;
    let mut value: Value =
        serde_json::from_str(&text).with_context(|| format!("cannot read {}", tuning.display()))?;
    let flags = value
        .as_object_mut()
        .context("tuning.json is not an object")?
        .entry("flags")
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .context("the flags block of tuning.json is not an object")?;
    for copy in copies {
        if flags.contains_key(&copy.name) {
            anyhow::bail!(
                "the broken copy {} has the name of a flag of tuning.json",
                copy.name
            );
        }
        let mut body = copy.flag.clone();
        body.insert("state".into(), Value::from("off"));
        flags.insert(copy.name.clone(), Value::Object(body));
    }
    std::fs::write(&tuning, serde_json::to_string_pretty(&value)? + "\n")
        .with_context(|| format!("cannot write {}", tuning.display()))?;
    Ok(())
}

/// Copies every file of `from` into `to`, but any proof file.
fn copy_tree(from: &Path, to: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(to).with_context(|| format!("cannot create {}", to.display()))?;
    for entry in
        std::fs::read_dir(from).with_context(|| format!("cannot read {}", from.display()))?
    {
        let path = entry?.path();
        let Some(name) = path.file_name() else {
            continue;
        };
        let target = to.join(name);
        if path.is_dir() {
            copy_tree(&path, &target)?;
        } else if name != PROOF_FILE {
            std::fs::copy(&path, &target)
                .with_context(|| format!("cannot copy {}", path.display()))?;
        }
    }
    Ok(())
}

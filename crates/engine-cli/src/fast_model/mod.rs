//! `engine-cli fast-model`: fit the fast model from full-engine results, check it against
//! the full engine, or refuse a stale fit.
//!
//! These commands are the only callers of `fast_model::resolve`: nothing in the game plays
//! the fast model, and a source-scan test fails the build when anything else reaches it.

pub mod batch;
pub mod check;
pub mod fit;
pub mod id;

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use engine::modules::fast_events::FitRules;
use engine::modules::fast_model::{
    self, FIT_FILE, FIT_VERSION, FIT_VERSION_2, FITTED_SCORES, FastFit,
};
use engine::{Content, ContentDir};
use serde::{Deserialize, Serialize};

use crate::cli::{FastModelAction, FastModelOpts};
use crate::report::bands::Bands;
use batch::{LEAGUE_SEED, LEVELS, Spec};
use check::{Figure, Range};
use id::EngineId;

/// The seed of the fit batch's engine matches.
pub const FIT_SEED: u64 = 1;
/// The seed of the check batch's engine matches: the same fixtures, other engine seeds.
pub const CHECK_SEED: u64 = 2;

/// The fit file (`content/fast-model.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FitFile {
    pub schema_version: u32,
    /// The module the fit is for, `name@version`.
    pub model: String,
    /// The engine id of the results the fit came from.
    pub engine_id: String,
    pub engine: EngineId,
    /// The program that fitted it.
    pub fitted_by: FittedBy,
    pub fit: FastFit,
    pub batch: BatchRecord,
    pub check: CheckRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FittedBy {
    pub engine_version: String,
    pub build: String,
}

/// The fit batch: the leagues, the strength levels, and the batch spec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchRecord {
    pub league_seed: u64,
    pub levels: Vec<f64>,
    pub matches_per_pairing: u32,
    pub minutes: u32,
    pub seed: u64,
}

/// The check the fit passed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckRecord {
    pub seed: u64,
    pub draws: u32,
    /// The z of the score figures.
    pub z: f64,
    /// The z of the event figures.
    pub event_z: f64,
    pub share_floor: f64,
    pub mean_floor: f64,
    pub figures: u32,
    pub failed: u32,
    pub pass: bool,
}

impl FitFile {
    /// Reads a fit file of this build's layout, or of version 2, whose model had no
    /// favourite's tilt: it reads with a tilt of 0, which plays as it did.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read the fit file {}", path.display()))?;
        let mut value: serde_json::Value = serde_json::from_str(&text)
            .with_context(|| format!("the fit file {} is malformed", path.display()))?;
        let version = value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64);
        if version == Some(u64::from(FIT_VERSION_2)) {
            if let Some(params) = value
                .pointer_mut("/fit/params")
                .and_then(serde_json::Value::as_object_mut)
            {
                params.insert("tilt".into(), 0.0.into());
            }
            value["schema_version"] = FIT_VERSION.into();
        } else if version != Some(u64::from(FIT_VERSION)) {
            bail!(
                "the fit file {} has schema_version {}; this build reads {FIT_VERSION}; \
                 run engine-cli fast-model fit",
                path.display(),
                version.map_or_else(|| "none".to_string(), |v| v.to_string())
            );
        }
        let file: Self = serde_json::from_value(value)
            .with_context(|| format!("the fit file {} is malformed", path.display()))?;
        file.fit.check()?;
        Ok(file)
    }

    fn to_text(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).expect("a fit file serializes");
        text.push('\n');
        text
    }
}

pub fn run(content_dir: Option<&Path>, opts: &FastModelOpts) -> anyhow::Result<i32> {
    let golden = opts
        .golden
        .clone()
        .unwrap_or_else(|| PathBuf::from(engine::gate::golden::DEFAULT_PATH));
    let dir = ContentDir::resolve(content_dir)?;
    let default_fit = dir.path(FIT_FILE);
    match opts.action {
        FastModelAction::Stale => {
            let path = opts.fit.clone().unwrap_or(default_fit);
            stale(&path, &golden)?;
            Ok(0)
        }
        FastModelAction::Fit => fit_action(content_dir, &dir, &golden, opts, &default_fit),
        FastModelAction::Check => check_action(content_dir, &dir, &golden, opts, &default_fit),
    }
}

/// Refuses a fit whose engine id is not the golden results' id, naming both.
fn stale(path: &Path, golden: &Path) -> anyhow::Result<EngineId> {
    let file = FitFile::load(path)?;
    let current = id::of_path(golden)?;
    if file.engine_id != current.id {
        bail!(
            "the fast-model fit is stale: the fit records {}, the golden results are {}; \
             run engine-cli fast-model fit",
            file.engine_id,
            current.id
        );
    }
    println!(
        "the fast-model fit {} matches the golden results: {}",
        path.display(),
        current.id
    );
    Ok(current)
}

fn jobs(opts: &FastModelOpts) -> usize {
    opts.jobs.map_or_else(
        || std::thread::available_parallelism().map_or(4, |n| n.get()),
        |j| j.max(1) as usize,
    )
}

fn load_content(dir: &ContentDir) -> anyhow::Result<Content> {
    Ok(Content::load(dir)?)
}

/// The season figures' band ranges from the bands file, for information.
fn band_ranges(dir: &ContentDir) -> check::Bands {
    let bands = Bands::load(dir).ok();
    let range = |f: &dyn Fn(&Bands) -> (f64, f64)| {
        bands.as_ref().map(|b| {
            let (lo, hi) = f(b);
            Range { lo, hi }
        })
    };
    [
        (
            "goals_per_match",
            range(&|b| (b.goals_per_match.lo, b.goals_per_match.hi)),
        ),
        (
            "goalless_share",
            range(&|b| (b.goalless_share.lo, b.goalless_share.hi)),
        ),
        (
            "ten_plus_goals_share",
            range(&|b| (b.ten_plus_goals_share.lo, b.ten_plus_goals_share.hi)),
        ),
        (
            "stronger_team_win_rate",
            range(&|b| (b.stronger_team.min_win_rate, 1.0)),
        ),
        (
            "sending_off_share",
            range(&|b| (b.sending_off_share.lo, b.sending_off_share.hi)),
        ),
        (
            "yellow_cards_per_team",
            range(&|b| (b.yellow_cards_per_team.lo, b.yellow_cards_per_team.hi)),
        ),
        (
            "corners_per_team",
            range(&|b| (b.corners_per_team.lo, b.corners_per_team.hi)),
        ),
        (
            "throw_ins_per_match",
            range(&|b| (b.throw_ins_per_match.lo, b.throw_ins_per_match.hi)),
        ),
        (
            "goal_kicks_per_match",
            range(&|b| (b.goal_kicks_per_match.lo, b.goal_kicks_per_match.hi)),
        ),
    ]
}

fn fit_action(
    content_dir: Option<&Path>,
    dir: &ContentDir,
    golden: &Path,
    opts: &FastModelOpts,
    default_fit: &Path,
) -> anyhow::Result<i32> {
    if opts.draws == 0 {
        bail!("--draws must be at least 1");
    }
    let engine = id::of_path(golden)?;
    let confirmed = id::confirm_engine(content_dir, golden, &opts.gate_fixtures)?;
    eprintln!(
        "fast-model: {confirmed} gate fixtures play the golden results {}",
        engine.id
    );
    let content = load_content(dir)?;
    let jobs = jobs(opts);
    let fit_spec = Spec {
        matches: opts.matches,
        minutes: opts.minutes,
        seed: FIT_SEED,
    };
    let check_spec = Spec {
        seed: CHECK_SEED,
        ..fit_spec
    };
    let fit_rows = batch::play(&content, &fit_spec, jobs)?;
    let fitted = fit::fit(
        &fit_rows,
        FitRules::of(&content.rules, &content.tuning.engine),
    );
    let check_rows = batch::play(&content, &check_spec, jobs)?;
    let model = fast_model::resolve(&content.modules);
    let fast = check::play_fast(model, &fitted, &check_rows, opts.draws, CHECK_SEED)?;
    let figures = check::compare(&check_rows, &fast, &band_ranges(dir));
    let failed = figures.iter().filter(|f| !f.pass).count();
    let file = FitFile {
        schema_version: FIT_VERSION,
        model: format!("{FITTED_SCORES}@1"),
        engine_id: engine.id.clone(),
        engine,
        fitted_by: FittedBy {
            engine_version: engine::version().into(),
            build: engine::build_hash().into(),
        },
        fit: fitted,
        batch: BatchRecord {
            league_seed: LEAGUE_SEED,
            levels: LEVELS.to_vec(),
            matches_per_pairing: opts.matches,
            minutes: opts.minutes,
            seed: FIT_SEED,
        },
        check: CheckRecord {
            seed: CHECK_SEED,
            draws: opts.draws,
            z: check::Z,
            event_z: check::EVENT_Z,
            share_floor: check::SHARE_FLOOR,
            mean_floor: check::MEAN_FLOOR,
            figures: figures.len() as u32,
            failed: failed as u32,
            pass: failed == 0,
        },
    };
    print_report(&file, &figures);
    if let Some(report) = &opts.report {
        write_report(report, &file, &figures)?;
    }
    if failed > 0 {
        eprintln!("fast-model: the check failed; the fit file was not written");
        return Ok(2);
    }
    let out = opts
        .out
        .clone()
        .unwrap_or_else(|| default_fit.to_path_buf());
    std::fs::write(&out, file.to_text())
        .with_context(|| format!("cannot write the fit file {}", out.display()))?;
    eprintln!("fast-model: wrote {}", out.display());
    Ok(0)
}

fn check_action(
    content_dir: Option<&Path>,
    dir: &ContentDir,
    golden: &Path,
    opts: &FastModelOpts,
    default_fit: &Path,
) -> anyhow::Result<i32> {
    let path = opts
        .fit
        .clone()
        .unwrap_or_else(|| default_fit.to_path_buf());
    let engine = stale(&path, golden)?;
    let mut file = FitFile::load(&path)?;
    let confirmed = id::confirm_engine(content_dir, golden, &opts.gate_fixtures)?;
    eprintln!(
        "fast-model: {confirmed} gate fixtures play the golden results {}",
        engine.id
    );
    let content = load_content(dir)?;
    let spec = Spec {
        matches: file.batch.matches_per_pairing,
        minutes: file.batch.minutes,
        seed: file.check.seed,
    };
    let rows = batch::play(&content, &spec, jobs(opts))?;
    let model = fast_model::resolve(&content.modules);
    let fast = check::play_fast(model, &file.fit, &rows, file.check.draws, file.check.seed)?;
    let figures = check::compare(&rows, &fast, &band_ranges(dir));
    let failed = figures.iter().filter(|f| !f.pass).count();
    file.check.failed = failed as u32;
    file.check.pass = failed == 0;
    print_report(&file, &figures);
    if let Some(report) = &opts.report {
        write_report(report, &file, &figures)?;
    }
    Ok(if failed == 0 { 0 } else { 2 })
}

fn print_report(file: &FitFile, figures: &[Figure]) {
    let p = &file.fit.params;
    println!("fast-model {} for {}", file.model, file.engine_id);
    println!(
        "params: base {:.4} home {:.4} attack {:.4} curve {:.4} defence {:.4} dispersion {:.3} rho {:.4} draw {:.4} tilt {:.4}",
        p.base, p.home, p.attack, p.curve, p.defence, p.dispersion, p.rho, p.draw, p.tilt
    );
    println!(
        "{:<14} {:<30} {:>9} {:>9} {:>9} {:>9} {:>5}  verdict  band",
        "group", "figure", "full", "fast", "diff", "tol", "z"
    );
    for f in figures {
        let band = f
            .band
            .map(|b| format!("{:.3}-{:.3} (information)", b.lo, b.hi))
            .unwrap_or_default();
        let verdict = match (f.pass, f.no_events) {
            (true, true) => "no events",
            (true, false) => "pass",
            (false, _) => "FAIL",
        };
        println!(
            "{:<14} {:<30} {:>9.4} {:>9.4} {:>+9.4} {:>9.4} {:>5.2}  {verdict:<7}  {band}",
            f.group,
            f.name,
            f.full,
            f.fast,
            f.fast - f.full,
            f.tolerance,
            f.z,
        );
    }
    let failed = figures.iter().filter(|f| !f.pass).count();
    println!(
        "fast-model check: {} figures, {failed} outside tolerance; {}",
        figures.len(),
        if failed == 0 { "pass" } else { "fail" }
    );
}

fn write_report(dir: &Path, file: &FitFile, figures: &[Figure]) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let report = serde_json::json!({
        "engine_id": file.engine_id,
        "pass": figures.iter().all(|f| f.pass),
        "figures": figures,
        "fit": file,
    });
    let path = dir.join("report.json");
    let mut text = serde_json::to_string_pretty(&report)?;
    text.push('\n');
    std::fs::write(&path, text).with_context(|| format!("cannot write {}", path.display()))?;
    Ok(())
}

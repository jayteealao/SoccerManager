//! `engine-cli restart-waits`: writes the restart wait table (`content/restart-waits.json`)
//! from real restart waits minus the engine's own visible steps.
//!
//! The command plays seeded matches with a zero table (every extra 0, no quick restart) and
//! takes, per kind of stoppage, the median of the engine's visible steps: the seconds from the
//! ball going dead to the tick the taker is at the ball and every player the law moves has
//! moved. It reads the real dead spells (`idsse_dead.csv` in the real-data folder) through
//! four columns only: the kind (`typ`), the length (`dur`, seconds), the match minute and the
//! restarting team's goal margin. Per kind it writes the 21 quantiles of
//! `max(0, real length − median visible steps)`. A real spell of a team leading in the late
//! window of regulation is left out, since the engine applies the late-lead factor to those
//! itself. Ball in play is never an input: the minutes of ball in play follow from play.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use engine::data::restart_waits::{QUANTILES, RestartWaits, WaitKind};
use engine::{ContentDir, DeadBall, MatchConfig, Simulation};

use crate::cli::RestartWaitsOpts;

/// The real dead spells file inside the real-data folder.
pub const DEAD_SPELLS_FILE: &str = "idsse_dead.csv";

/// One real dead spell, as the command reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct DeadSpell {
    /// The kind: `ThrowIn`, `GoalKick`, `CornerKick`, `FreeKick`, `Penalty`, `KickOff`, or
    /// anything else for a spell the data leaves unclassified.
    pub typ: String,
    /// The spell's length, in seconds.
    pub dur: f64,
    /// The match minute it started in.
    pub minute: f64,
    /// The restarting team's goals less the other team's, when the data has it.
    pub margin: Option<f64>,
}

impl DeadSpell {
    /// The engine kind of the spell: a kick-off is one after a goal (a period start is no
    /// stoppage), and an unclassified spell stands for the dropped ball.
    pub fn kind(&self) -> WaitKind {
        match self.typ.as_str() {
            "ThrowIn" => WaitKind::ThrowIn,
            "GoalKick" => WaitKind::GoalKick,
            "CornerKick" => WaitKind::Corner,
            "FreeKick" => WaitKind::FreeKick,
            "Penalty" => WaitKind::Penalty,
            "KickOff" => WaitKind::KickOff,
            _ => WaitKind::DropBall,
        }
    }
}

/// Reads the real dead spells from the CSV text `text`: the header names the columns, and only
/// `typ`, `dur`, `minute` and `margin` are read.
pub fn read_spells(text: &str) -> anyhow::Result<Vec<DeadSpell>> {
    let mut lines = text.lines();
    let header: Vec<&str> = lines
        .next()
        .context("the dead spells file is empty")?
        .trim_end_matches('\r')
        .split(',')
        .collect();
    let column = |name: &str| {
        header
            .iter()
            .position(|h| *h == name)
            .with_context(|| format!("the dead spells file has no {name} column"))
    };
    let (typ, dur, minute, margin) = (
        column("typ")?,
        column("dur")?,
        column("minute")?,
        column("margin")?,
    );
    let mut spells = Vec::new();
    for (n, line) in lines.enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let cells: Vec<&str> = line.split(',').collect();
        let cell = |i: usize| cells.get(i).copied().unwrap_or("");
        let number = |i: usize| -> anyhow::Result<f64> {
            cell(i)
                .parse::<f64>()
                .with_context(|| format!("row {}: {} is not a number", n + 2, cell(i)))
        };
        spells.push(DeadSpell {
            typ: cell(typ).to_string(),
            dur: number(dur)?,
            minute: number(minute)?,
            margin: if cell(margin).is_empty() {
                None
            } else {
                Some(number(margin)?)
            },
        });
    }
    Ok(spells)
}

/// The value at share `p` (0 to 1) of the sorted values `v`: linear between the two nearest
/// ranks.
fn quantile(v: &[f64], p: f64) -> f64 {
    let at = p * (v.len() - 1) as f64;
    let i = at.floor() as usize;
    let j = (i + 1).min(v.len() - 1);
    v[i] + (v[j] - v[i]) * (at - i as f64)
}

/// The median of `v`, or `None` when it is empty.
pub fn median(v: &[f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    let mut sorted = v.to_vec();
    sorted.sort_by(f64::total_cmp);
    Some(quantile(&sorted, 0.5))
}

/// The table `like` with each kind's extras taken from `spells` less the kind's median
/// visible steps (`visible`, seconds; a kind the engine never measured counts 0): the 21
/// quantiles of `max(0, dur − visible)` over the kind's spells, leaving out a spell of a team
/// leading from minute `late_from` on. A kind with no spell keeps its quantiles. The steps
/// limits, the late-lead factor and the quick-restart rule stay as `like` holds them.
pub fn build_table(
    like: &RestartWaits,
    spells: &[DeadSpell],
    visible: &BTreeMap<&'static str, f64>,
    late_from: f64,
    source: Vec<String>,
) -> RestartWaits {
    let mut table = like.clone();
    for kind in WaitKind::ALL {
        let steps = visible.get(kind.code()).copied().unwrap_or(0.0);
        let mut extras: Vec<f64> = spells
            .iter()
            .filter(|s| s.kind() == kind && !leading_late(s, late_from))
            .map(|s| (s.dur - steps).max(0.0))
            .collect();
        if extras.is_empty() {
            continue;
        }
        extras.sort_by(f64::total_cmp);
        let quantiles: [f64; QUANTILES] = std::array::from_fn(|i| {
            let q = quantile(&extras, i as f64 / (QUANTILES - 1) as f64);
            // Two decimals: a hundredth of a second is below a tick.
            (q * 100.0).round() / 100.0
        });
        table.kinds.get_mut(kind).extra_s = quantiles;
    }
    table.source = source;
    table
}

/// `true` for a spell of a team leading from minute `late_from` on.
fn leading_late(s: &DeadSpell, late_from: f64) -> bool {
    s.minute >= late_from && s.margin.is_some_and(|m| m > 0.0)
}

/// Every seed `spec` names: `N` or `A-B`.
pub fn parse_seeds(spec: &str) -> anyhow::Result<Vec<u64>> {
    let bad = || format!("--seeds {spec}: give a seed or a range such as 1-8");
    match spec.split_once('-') {
        Some((a, b)) => {
            let (a, b): (u64, u64) = (
                a.trim().parse().with_context(bad)?,
                b.trim().parse().with_context(bad)?,
            );
            if a > b || b - a >= 1000 {
                bail!(bad());
            }
            Ok((a..=b).collect())
        }
        None => Ok(vec![spec.trim().parse().with_context(bad)?]),
    }
}

/// The visible steps of every stoppage of the matches `config` plays on each of `seeds`, in
/// seconds by kind: from the tick the ball went dead to the tick the restart was taken, which
/// under a zero wait table is the tick its visible steps were first done (or its hard limit).
pub fn visible_steps(
    config: &MatchConfig,
    seeds: &[u64],
) -> anyhow::Result<BTreeMap<&'static str, Vec<f64>>> {
    let mut out: BTreeMap<&'static str, Vec<f64>> = BTreeMap::new();
    let dt = config.tuning.dt;
    for &seed in seeds {
        let mut config = config.clone();
        config.seed = seed;
        let mut sim = Simulation::new(config)?;
        let mut waiting: Option<DeadBall> = None;
        while !sim.is_over() {
            sim.step();
            let now = sim.dead_ball();
            if let Some(dead) = waiting
                && now.is_none_or(|n| n.since != dead.since)
            {
                // Taken on this tick: a shoot-out kick never comes in a league match.
                out.entry(WaitKind::of(dead.kind).code())
                    .or_default()
                    .push(f64::from(sim.tick() - dead.since) * dt);
            }
            waiting = now;
        }
    }
    Ok(out)
}

pub fn run(content_dir: Option<&Path>, opts: &RestartWaitsOpts) -> anyhow::Result<i32> {
    let dir = ContentDir::resolve(content_dir)?;
    let loaded = crate::content::load(content_dir, None, None, None)?;
    let like = loaded
        .content
        .restart_waits
        .clone()
        .unwrap_or_else(RestartWaits::defaults);
    let seeds = parse_seeds(&opts.seeds)?;
    let [team_a, team_b] = &loaded.teams;
    let mut config = MatchConfig::new(seeds[0], opts.minutes, &loaded.content, [team_a, team_b])?;
    config.restart_waits = Some(RestartWaits::zero(&like));
    let measured = visible_steps(&config, &seeds)?;
    let visible: BTreeMap<&'static str, f64> = measured
        .iter()
        .filter_map(|(kind, v)| median(v).map(|m| (*kind, m)))
        .collect();

    let folder = crate::report::real::resolve(opts.real_data.as_deref(), dir.root());
    let Some(real) = folder.path.as_ref() else {
        bail!("no real-data folder: give --real-data DIR");
    };
    let path: PathBuf = real.join(DEAD_SPELLS_FILE);
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read {DEAD_SPELLS_FILE} in the real-data folder"))?;
    let spells = read_spells(&text)?;
    let rules = &loaded.content.rules;
    let late_from = f64::from(
        rules
            .regulation_minutes()
            .saturating_sub(loaded.content.tuning.engine.pace.late_minutes),
    );

    let count = |kind: WaitKind| spells.iter().filter(|s| s.kind() == kind).count();
    let dropped = spells.iter().filter(|s| leading_late(s, late_from)).count();
    let per_kind = |f: &dyn Fn(WaitKind) -> String| {
        WaitKind::ALL
            .iter()
            .map(|&k| format!("{} {}", k.code(), f(k)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let source = vec![
        format!(
            "real waits: {DEAD_SPELLS_FILE}, {} dead spells ({}); kick-offs follow goals; \
             unclassified spells stand for the dropped ball",
            spells.len(),
            per_kind(&|k| count(k).to_string())
        ),
        format!(
            "left out: {dropped} spells of a team leading from minute {late_from} on (the engine \
             applies the late-lead factor)"
        ),
        format!(
            "visible steps: median seconds over {}-minute matches of the shipped teams on seeds \
             {}, engine {} ({})",
            opts.minutes,
            opts.seeds,
            engine::version(),
            per_kind(&|k| match (visible.get(k.code()), measured.get(k.code())) {
                (Some(m), Some(v)) => format!("{m:.2} s of {}", v.len()),
                _ => "none measured".into(),
            })
        ),
        "extra: the quantiles of max(0, real length - median visible steps); written by \
         engine-cli restart-waits, never by hand"
            .to_string(),
    ];
    let table = build_table(&like, &spells, &visible, late_from, source);
    let mut text = serde_json::to_string_pretty(&table)?;
    text.push('\n');
    // The table must load as content before it is written.
    engine::data::restart_waits::load_restart_waits_bytes(text.as_bytes(), "restart-waits.json")?;
    std::fs::write(&opts.out, text)
        .with_context(|| format!("cannot write {}", opts.out.display()))?;
    let summary = serde_json::json!({
        "kind": "restart-waits",
        "out": opts.out.display().to_string(),
        "spells": spells.len(),
        "left_out": dropped,
        "visible_median_s": visible,
        "visible_count": measured.iter().map(|(k, v)| (*k, v.len())).collect::<BTreeMap<_, _>>(),
        "extra_median_s": WaitKind::ALL
            .iter()
            .map(|&k| (k.code(), table.kinds.get(k).extra_s[QUANTILES / 2]))
            .collect::<BTreeMap<_, _>>(),
    });
    println!("{summary}");
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spell(typ: &str, dur: f64, minute: f64, margin: Option<f64>) -> DeadSpell {
        DeadSpell {
            typ: typ.into(),
            dur,
            minute,
            margin,
        }
    }

    #[test]
    fn the_extras_are_the_real_lengths_less_the_median_visible_steps() {
        let spells: Vec<DeadSpell> = (0..21)
            .map(|i| spell("ThrowIn", 10.0 + f64::from(i), 20.0, Some(0.0)))
            .chain([spell("CornerKick", 3.0, 20.0, None)])
            .collect();
        let visible = BTreeMap::from([("throw_in", 12.0), ("corner", 5.0)]);
        let t = build_table(&RestartWaits::defaults(), &spells, &visible, 75.0, vec![]);
        let q = t.kinds.throw_in.extra_s;
        // 10..30 less 12, floored at 0: 0, 0, 0, 1, 2, ..., 18.
        assert_eq!(q[0], 0.0);
        assert_eq!(q[2], 0.0);
        assert_eq!(q[3], 1.0);
        assert_eq!(q[20], 18.0);
        assert_eq!(t.kinds.corner.extra_s, [0.0; QUANTILES], "never below 0");
        assert_eq!(t.kinds.penalty.extra_s, [0.0; QUANTILES], "no spell: kept");
    }

    #[test]
    fn a_spell_of_a_team_leading_late_is_left_out() {
        let spells = vec![
            spell("FreeKick", 20.0, 80.0, Some(1.0)),
            spell("FreeKick", 10.0, 80.0, Some(0.0)),
            spell("FreeKick", 10.0, 30.0, Some(2.0)),
        ];
        let t = build_table(
            &RestartWaits::defaults(),
            &spells,
            &BTreeMap::new(),
            75.0,
            vec![],
        );
        assert_eq!(t.kinds.free_kick.extra_s, [10.0; QUANTILES]);
    }

    #[test]
    fn only_the_four_columns_are_read_by_name() {
        let text =
            "mid,ball_in_play,typ,dur,minute,margin\nA,99,ThrowIn,12.5,3.2,\nA,1,none,4,7,-1\n";
        let spells = read_spells(text).unwrap();
        assert_eq!(spells[0], spell("ThrowIn", 12.5, 3.2, None));
        assert_eq!(spells[1].kind(), WaitKind::DropBall);
        assert!(
            read_spells("typ,dur\n").is_err(),
            "a missing column is refused"
        );
    }

    #[test]
    fn seeds_read_as_one_or_a_range() {
        assert_eq!(parse_seeds("1-4").unwrap(), [1, 2, 3, 4]);
        assert_eq!(parse_seeds("9").unwrap(), [9]);
        assert!(parse_seeds("5-2").is_err());
    }
}

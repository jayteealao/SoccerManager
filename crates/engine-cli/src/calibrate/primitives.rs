//! The primitives harness (a `measures` build only): the monotonic check and the fit of a
//! tactical primitive, from played matches.
//!
//! A pair is two matches with identical squads (the equal suite's home club on both sides),
//! the same engine seed, the opponent in the tactics file's defaults, and AI in-match tactic
//! changes off (the run's content copy moves their minutes past full time; substitutions go
//! on as usual). Only the tested side's setting differs between the two matches, and the
//! tested side alternates home and away from pair to pair, so home advantage cancels. The
//! harness reads each signature statistic of the tested side from the match's measures.
//!
//! - [`check`] steps a graded primitive one level up and passes when every signature
//!   statistic moves in its stated direction by more than the paired noise: the mean paired
//!   difference times the direction exceeds 1.96 standard errors. A choice primitive pairs
//!   each choice against the default choice on the choice's own statistic.
//! - [`fit`] searches the primitive's first plan input at one level by bisection until the
//!   side's mean primary statistic lies within the noise of the median of the level's real
//!   quintile in `signatures.json`, with every other primitive at its default.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, bail};
use engine::data::TeamFile;
use engine::data::primitives::{
    Direction, GRADED_LEVELS, OWN, Primitive, PrimitiveKind, Steps, held_out, input_field,
    level_value, set_level_value, settings,
};
use engine::data::tactics::{INSTRUCTIONS, TacticsSchema};
use engine::measures::MatchMeasures;
use engine::measures::runs::RunsSink;
use engine::measures::shape::BlockHeightSink;
use engine::{Content, FanoutSink, MatchConfig, Simulation, Tactics};
use serde::Serialize;

use super::fixtures::{self, Leagues};

/// The statistics the harness measures, in value order: the signature statistics, then the
/// side's possessions and shots a match, which every step reports beside them.
pub const STATS: [&str; 65] = [
    "passes_per_possession",
    "recycle_share",
    "turnovers_per_possession",
    "shot_share",
    "block_height_m",
    "hold_dribble_share",
    "forward_pass_share",
    "long_ball_share",
    "own_third_long_share",
    "goal_kick_m",
    "keeper_long_share",
    "wide_pass_share",
    "entries_left_share",
    "entries_right_share",
    "entries_centre_share",
    "entries_wide_share",
    "entries_centre_left_share",
    "entries_centre_right_share",
    "outside_box_shot_share",
    "shot_distance_m",
    "takeons_per_90",
    "crosses_per_final_third_possession",
    "high_cross_share",
    "low_cross_share",
    "cross_origin_m",
    "off_plan_share",
    "runs_in_behind_share",
    "coming_short_share",
    "fullback_runs_per_match",
    "overlaps_per_match",
    "underlaps_per_match",
    "counter_attacks_per_match",
    "regain_to_shot_s",
    "counterpress_pressures_per_loss",
    "counterpress_regain_share",
    "last_counterpress_s",
    "ppda",
    "def_action_height_m",
    "defender_action_height_m",
    "oop_width_m",
    "opp_entries_wide_share",
    "offsides_caught_per_match",
    "opp_goal_kick_m",
    "fouls_per_match",
    "yellows_per_match",
    "pressures_after_back_pass",
    "pressures_after_poor_touch",
    "pressures_after_facing_back",
    "pressures_after_wide",
    "reds_per_match",
    "traps_per_match",
    "back_passes_per_match",
    "poor_touches_per_match",
    "facing_back_per_match",
    "runs_per_match",
    "free_pass_completion",
    "pressed_pass_completion",
    "free_touch_loss",
    "pressed_touch_loss",
    "crosses",
    "entries",
    "final_third_possessions",
    "goals",
    "possessions",
    "shots",
];

/// One side's statistics of one match, in [`STATS`] order, each as its numerator and its
/// denominator, so that many matches pool the way the real table pools a team's season (sum
/// over sum) and one short match cannot swing the result.
pub type Stats = [(f64, f64); STATS.len()];

/// The index of statistic `name` in [`STATS`].
pub fn stat_index(name: &str) -> anyhow::Result<usize> {
    STATS
        .iter()
        .position(|s| *s == name)
        .with_context(|| format!("the harness does not measure {name}"))
}

fn ratio(n: f64, d: f64) -> f64 {
    if d > 0.0 { n / d } else { f64::NAN }
}

/// What one match gives the harness beside its measures: the block heights, the off-ball
/// runs, the goals, and the match's length in units of 90 minutes.
pub struct Sinks<'a> {
    pub block: &'a BlockHeightSink,
    pub runs: &'a RunsSink,
    pub goals: [u32; 2],
    /// Fouls committed, yellow and red cards shown, and offsides called, per side.
    pub fouls: [u32; 2],
    pub yellows: [u32; 2],
    pub reds: [u32; 2],
    pub offsides: [u32; 2],
    pub nineties: f64,
}

/// Side `team`'s statistics from a match's measures and sinks.
pub fn side_stats(m: &MatchMeasures, sinks: &Sinks<'_>, team: usize) -> Stats {
    let f = f64::from;
    let possessions = f(m.possessions[team]);
    let passes = f(m.possession_passes[team]);
    let choices = f(m.choices[team].iter().sum::<u32>());
    let s = &m.possessing[team];
    let e = s.entries;
    let entries = f(e.iter().sum::<u32>());
    let runs = sinks.runs;
    let fullback = f(runs.fullback_runs(team));
    let n = sinks.nineties;
    let x = &m.transitions[team];
    let other = 1 - team;
    let d = &m.defending[team];
    let oe = m.possessing[other].entries;
    let opp_entries = f(oe.iter().sum::<u32>());
    let o = &m.possessing[other];
    let trig = |k: usize| (f(d.trigger_pressures[k]), f(d.trigger_events[k]));
    [
        (passes, possessions),
        (f(m.recycle_passes[team]), f(m.open_play_passes[team])),
        (f(m.turnovers[team]), possessions),
        (f(m.shot_possessions[team]), possessions),
        sinks.block.height[team],
        (f(m.choices[team][2] + m.choices[team][3]), choices),
        (f(s.forward), f(s.passes)),
        (f(s.long), f(s.passes)),
        (f(s.own_third_long), f(s.own_third)),
        (s.goal_kick_m, f(s.goal_kicks)),
        (f(s.keeper_long), f(s.keeper_actions)),
        (f(s.wide), f(s.upfield)),
        (f(e[0]), entries),
        (f(e[2]), entries),
        (f(e[1]), entries),
        (f(e[0] + e[2]), entries),
        (f(e[0] + e[1]), entries),
        (f(e[1] + e[2]), entries),
        (f(s.outside_shots), f(s.open_shots)),
        (s.open_shot_m, f(s.open_shots)),
        (f(s.takeons), n),
        (f(s.crosses), f(s.final_third_possessions)),
        (f(s.high_crosses), f(s.crosses)),
        (f(s.crosses - s.high_crosses), f(s.crosses)),
        (s.cross_origin_m, f(s.crosses)),
        (f(s.off_plan), choices),
        (f(runs.behind[team]), f(runs.runs[team])),
        (f(runs.coming_short[team]), f(runs.runs[team])),
        (fullback, n),
        (f(runs.overlaps[team]), n),
        (f(runs.underlaps[team]), n),
        (f(x.counter_attacks), n),
        (x.regain_shot_s, f(x.regain_shots)),
        (f(x.counterpress_pressures), f(x.losses)),
        (f(x.counterpress_regains), f(x.losses)),
        (x.last_counterpress_s, f(x.last_counterpress)),
        (f(m.defending[other].build_passes), f(d.ppda_actions)),
        (d.action_m, f(d.actions)),
        (d.defender_action_m, f(d.defender_actions)),
        sinks.block.width[team],
        (f(oe[0] + oe[2]), opp_entries),
        (f(sinks.offsides[other]), n),
        (o.goal_kick_m, f(o.goal_kicks)),
        (f(sinks.fouls[team]), n),
        (f(sinks.yellows[team]), n),
        trig(0),
        trig(1),
        trig(2),
        trig(3),
        (f(sinks.reds[team]), n),
        (f(d.traps), n),
        (f(d.trigger_events[0]), n),
        (f(d.trigger_events[1]), n),
        (f(d.trigger_events[2]), n),
        (f(runs.runs[team]), n),
        (f(s.pressure_completed[0]), f(s.pressure_passes[0])),
        (f(s.pressure_completed[1]), f(s.pressure_passes[1])),
        (f(s.touches_lost[0]), f(s.touches[0])),
        (f(s.touches_lost[1]), f(s.touches[1])),
        (f(s.crosses), 1.0),
        (entries, 1.0),
        (f(s.final_third_possessions), 1.0),
        (f(sinks.goals[team]), 1.0),
        (possessions, 1.0),
        (f(m.shot_bins[team].iter().sum::<u32>()), 1.0),
    ]
}

/// One match: the two sides' statistics.
pub fn play_one(
    content: &Content,
    teams: &[TeamFile; 2],
    tactics: [Tactics; 2],
    seed: u64,
    minutes: u32,
) -> anyhow::Result<[Stats; 2]> {
    let mut config = MatchConfig::new(seed, minutes, content, [&teams[0], &teams[1]])?;
    for (team, t) in tactics.into_iter().enumerate() {
        config = config.with_ai_tactics(team, t);
    }
    let mut sim = Simulation::new(config)?;
    sim.with_measures();
    let mut sinks = FanoutSink::new(BlockHeightSink::default(), RunsSink::default());
    sim.run(&mut sinks)?;
    let (block, runs) = sinks.into_parts();
    let m = sim
        .measures()
        .context("a measures build counts the measures")?;
    let sinks = Sinks {
        block: &block,
        runs: &runs,
        goals: sim.summary().goals,
        fouls: sim.summary().fouls,
        yellows: sim.summary().yellow,
        reds: sim.summary().red,
        offsides: sim.summary().offsides,
        nineties: f64::from(minutes) / 90.0,
    };
    Ok([side_stats(m, &sinks, 0), side_stats(m, &sinks, 1)])
}

/// What one harness run needs.
#[derive(Debug, Clone)]
pub struct Setup {
    /// The equal suite's seed: its leagues and fixtures.
    pub seed: u64,
    pub pairs: u32,
    pub minutes: u32,
    pub jobs: u32,
}

/// The content of a harness run: AI in-match tactic changes off.
pub fn harness_content(content: &Content) -> Content {
    let mut c = content.clone();
    c.tactics.ai.trailing_minute = 200;
    c.tactics.ai.leading_minute = 200;
    c
}

/// The primitive `name` of `schema`.
pub fn primitive<'a>(schema: &'a TacticsSchema, name: &str) -> anyhow::Result<&'a Primitive> {
    schema
        .primitives
        .iter()
        .find(|p| p.name == name)
        .with_context(|| format!("the tactics file has no primitive {name}"))
}

/// `tactics` with primitive `p` at level or choice `level`: in the settings when `p` owns its
/// levels or choices, else in its level block.
pub fn with_level(
    mut tactics: Tactics,
    schema: &TacticsSchema,
    p: &Primitive,
    level: usize,
) -> anyhow::Result<Tactics> {
    if p.is_setting() {
        let k = settings(schema)
            .position(|o| o.name == p.name)
            .with_context(|| format!("{} is not a setting", p.name))?;
        tactics.settings[k] = u8::try_from(level).context("a level index")?;
        return Ok(tactics);
    }
    with_block_level(tactics, p.levels.as_deref().unwrap_or_default(), level)
}

/// `tactics` with the level block `block` at level `level`.
fn with_block_level(mut tactics: Tactics, block: &str, level: usize) -> anyhow::Result<Tactics> {
    let level = u8::try_from(level).context("a level index")?;
    if block == "mentalities" {
        tactics.mentality = level;
        return Ok(tactics);
    }
    let index = block
        .strip_prefix("instructions.")
        .and_then(|name| INSTRUCTIONS.iter().position(|i| *i == name))
        .with_context(|| format!("{block} is not a level block"))?;
    tactics.instructions[index] = level;
    Ok(tactics)
}

/// A side's setting as plan input values: the content with those values written into a level
/// of each input's block (or a step or choice of the primitive that owns the input) that the
/// defaults do not use, and the tactics that select it.
pub fn with_values(
    content: &Content,
    values: &BTreeMap<String, f64>,
) -> anyhow::Result<(Content, Tactics)> {
    let mut c = content.clone();
    let defaults = Tactics::defaults(&c.tactics);
    let mut tactics = defaults;
    for (input, value) in values {
        let (block, _, lo, hi) =
            input_field(input).with_context(|| format!("{input} has no level field"))?;
        if !(lo..=hi).contains(value) {
            bail!("{input} {value} lies outside its range {lo} to {hi}");
        }
        if block == OWN {
            let p = c
                .tactics
                .primitives
                .iter()
                .find(|p| p.is_setting() && p.inputs.iter().any(|i| &i.input == input))
                .cloned()
                .with_context(|| format!("no primitive owns {input}"))?;
            let default = p.default_index(&c.tactics).context("a default")?;
            let spare = if default == 0 { 1 } else { 0 };
            set_level_value(&mut c.tactics, input, spare, *value);
            tactics = with_level(tactics, &c.tactics, &p, spare)?;
            continue;
        }
        let default = if block == "mentalities" {
            usize::from(defaults.mentality)
        } else {
            let i = block
                .strip_prefix("instructions.")
                .and_then(|name| INSTRUCTIONS.iter().position(|n| *n == name))
                .with_context(|| format!("{block} is not a level block"))?;
            usize::from(defaults.instructions[i])
        };
        let spare = if default == 0 { 1 } else { 0 };
        set_level_value(&mut c.tactics, input, spare, *value);
        tactics = with_block_level(tactics, block, spare)?;
    }
    Ok((c, tactics))
}

/// One arm of a pair: the content and the tested side's tactics.
pub struct Arm {
    pub content: Content,
    pub tactics: Tactics,
}

/// Plays the `arms` (one or two) on each of `pairs` fixtures on `jobs` threads and gives,
/// per fixture, the tested side's statistics in each arm.
pub fn play_pairs(
    setup: &Setup,
    teams_of: &Content,
    arms: &[Arm],
    salt: &str,
) -> anyhow::Result<Vec<Vec<Stats>>> {
    let list = fixtures::fixtures(setup.pairs);
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(usize, Vec<Stats>)>> = Mutex::new(Vec::new());
    let first_error: Mutex<Option<anyhow::Error>> = Mutex::new(None);
    let threads = usize::try_from(setup.jobs.max(1))
        .unwrap_or(1)
        .min(list.len().max(1));
    std::thread::scope(|scope| {
        for _ in 0..threads {
            let (list, next, results, first_error) = (&list, &next, &results, &first_error);
            scope.spawn(move || {
                let mut leagues = Leagues::new(setup.seed, teams_of);
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(f) = list.get(i) else { break };
                    let [home, _] = leagues.teams(f);
                    let mut away = home.clone();
                    away.club.id = format!("{}-b", home.club.id);
                    let teams = [home, away];
                    let league = format!("league:{}", setup.seed.wrapping_add(u64::from(f.league)));
                    let club = f.clubs[0].to_string();
                    let (_, engine_seed) = fixtures::key_of(
                        "primitives",
                        &league,
                        [&club, &club],
                        [salt, salt],
                        i as u64,
                    );
                    let tested = i % 2;
                    let mut out = vec![[(0.0, 0.0); STATS.len()]; arms.len()];
                    let mut failed = None;
                    for (a, arm) in arms.iter().enumerate() {
                        let opponent = Tactics::defaults(&arm.content.tactics);
                        let mut tactics = [opponent; 2];
                        tactics[tested] = arm.tactics;
                        match play_one(&arm.content, &teams, tactics, engine_seed, setup.minutes) {
                            Ok(stats) => out[a] = stats[tested],
                            Err(e) => failed = Some(e),
                        }
                    }
                    match failed {
                        None => results
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .push((i, out)),
                        Some(e) => {
                            first_error
                                .lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .get_or_insert(e);
                        }
                    }
                }
            });
        }
    });
    if let Some(e) = first_error.into_inner().unwrap_or_else(|e| e.into_inner()) {
        return Err(e);
    }
    let mut done = results.into_inner().unwrap_or_else(|e| e.into_inner());
    done.sort_by_key(|(i, _)| *i);
    Ok(done.into_iter().map(|(_, s)| s).collect())
}

/// The pooled estimate of one statistic over `units` (the fixtures), each holding the
/// statistic's numerator and denominator in one arm or two: with one arm its pooled value (sum
/// over sum), with two the second arm's pooled value less the first's. Returns the estimate,
/// its standard error by the jackknife over the units (each left out in turn), and the units.
pub fn pooled(units: &[Vec<(f64, f64)>]) -> (f64, f64, usize) {
    let n = units.len();
    let arms = units.first().map_or(1, Vec::len);
    let mut sums = vec![(0.0, 0.0); arms];
    for u in units {
        for (s, x) in sums.iter_mut().zip(u) {
            s.0 += x.0;
            s.1 += x.1;
        }
    }
    let value = |s: &[(f64, f64)]| {
        let r = |a: usize| ratio(s[a].0, s[a].1);
        if arms == 2 { r(1) - r(0) } else { r(0) }
    };
    let all = value(&sums);
    if n < 2 {
        return (all, f64::INFINITY, n);
    }
    let left_out: Vec<f64> = units
        .iter()
        .map(|u| {
            let s: Vec<(f64, f64)> = sums
                .iter()
                .zip(u)
                .map(|(s, x)| (s.0 - x.0, s.1 - x.1))
                .collect();
            value(&s)
        })
        .collect();
    let mean = left_out.iter().sum::<f64>() / n as f64;
    let var = left_out.iter().map(|v| (v - mean).powi(2)).sum::<f64>();
    (all, ((n - 1) as f64 / n as f64 * var).sqrt(), n)
}

/// The units of statistic `stat` in `pairs`, arm by arm.
fn units(pairs: &[Vec<Stats>], stat: usize, arms: &[usize]) -> Vec<Vec<(f64, f64)>> {
    pairs
        .iter()
        .map(|p| arms.iter().map(|&a| p[a][stat]).collect())
        .collect()
}

/// One statistic's verdict on one step.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatVerdict {
    pub stat: String,
    pub direction: String,
    /// The pooled value of each arm and the pooled difference (second arm less first).
    pub before: f64,
    pub after: f64,
    pub difference: f64,
    /// 1.96 standard errors of the difference, by the jackknife over the pairs.
    pub noise: f64,
    pub pairs: usize,
    pub pass: bool,
}

/// The verdict of one step or one choice.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StepVerdict {
    pub primitive: String,
    /// `<from> -> <to>`.
    pub step: String,
    /// The primary statistic first, then the secondary ones.
    pub stats: Vec<StatVerdict>,
    /// The primary statistic's verdict: the step's.
    pub pass: bool,
    /// Every statistic the harness measures, the pooled value of each arm (lower level
    /// first).
    pub means: BTreeMap<String, [f64; 2]>,
    /// Every statistic the harness measures, its paired difference (second arm less first)
    /// and that difference's noise (1.96 standard errors), so that a statistic a step must
    /// leave alone can be read against its noise.
    pub differences: BTreeMap<String, [f64; 2]>,
}

/// The paired difference of every statistic in `pairs` and its noise.
pub fn arm_differences(pairs: &[Vec<Stats>]) -> BTreeMap<String, [f64; 2]> {
    STATS
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let (difference, se, _) = pooled(&units(pairs, i, &[0, 1]));
            (s.to_string(), [difference, 1.96 * se])
        })
        .collect()
}

/// The pooled value of every statistic in each arm of `pairs`.
pub fn arm_means(pairs: &[Vec<Stats>]) -> BTreeMap<String, [f64; 2]> {
    STATS
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let at = |a: usize| pooled(&units(pairs, i, &[a])).0;
            (s.to_string(), [at(0), at(1)])
        })
        .collect()
}

/// The verdict of the pairs on statistic `stat` moving in `direction`.
pub fn judge(pairs: &[Vec<Stats>], stat: usize, direction: Direction) -> StatVerdict {
    let (difference, se, n) = pooled(&units(pairs, stat, &[0, 1]));
    let noise = 1.96 * se;
    StatVerdict {
        stat: STATS[stat].to_string(),
        direction: match direction {
            Direction::Up => "up".into(),
            Direction::Down => "down".into(),
        },
        before: pooled(&units(pairs, stat, &[0])).0,
        after: pooled(&units(pairs, stat, &[1])).0,
        difference,
        noise,
        pairs: n,
        pass: difference * direction.sign() > noise,
    }
}

/// The monotonic check of `name`: each step of a graded primitive from level `from` (all
/// steps when `None`) one level up, or each choice of a choice primitive against its default.
/// The levels `with` (`<primitive>=<level>`) of other primitives are set in the upper arm of
/// each graded step: a combined step against both at their lower levels.
pub fn check(
    content: &Content,
    name: &str,
    from: Option<&str>,
    with: &[String],
    setup: &Setup,
) -> anyhow::Result<Vec<StepVerdict>> {
    let content = harness_content(content);
    let p = primitive(&content.tactics, name)?.clone();
    let mut extra: Vec<(Primitive, usize)> = Vec::new();
    for w in with {
        let (other, level) = w
            .split_once('=')
            .with_context(|| format!("--with {w}: give <primitive>=<level>"))?;
        let o = primitive(&content.tactics, other)?.clone();
        let at = o
            .level_names(&content.tactics)
            .iter()
            .position(|n| *n == level)
            .with_context(|| format!("{other} has no level {level}"))?;
        extra.push((o, at));
    }
    if !extra.is_empty() && p.kind != PrimitiveKind::Graded {
        bail!("{name}: --with combines graded steps only");
    }
    let names: Vec<String> = p
        .level_names(&content.tactics)
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut out = Vec::new();
    match p.kind {
        PrimitiveKind::Graded => {
            let steps: Vec<usize> = match from {
                Some(level) => vec![
                    names
                        .iter()
                        .position(|n| n == level)
                        .filter(|&i| i + 1 < names.len())
                        .with_context(|| format!("{name} has no step up from {level}"))?,
                ],
                None => (0..names.len() - 1).collect(),
            };
            let defaults = Tactics::defaults(&content.tactics);
            let d = p.default_index(&content.tactics).context("a default")?;
            for l in steps {
                let arms = [l, l + 1].map(|level| -> anyhow::Result<Arm> {
                    Ok(Arm {
                        content: content.clone(),
                        tactics: with_level(defaults, &content.tactics, &p, level)?,
                    })
                });
                let [a, b] = arms;
                let mut arms = [a?, b?];
                for (o, at) in &extra {
                    arms[1].tactics = with_level(arms[1].tactics, &content.tactics, o, *at)?;
                }
                let mut step = format!("{} -> {}", names[l], names[l + 1]);
                for w in with {
                    step.push_str(&format!(" with {w}"));
                }
                let pairs = play_pairs(setup, &content, &arms, &format!("{name}:{step}"))?;
                let mut stats = Vec::new();
                for s in p.signature.iter().filter(|s| s.steps.covers_step(l, d)) {
                    stats.push(judge(&pairs, stat_index(&s.stat)?, s.direction));
                }
                // A graded step is judged on its primary statistic, the first that judges the
                // step (a statistic may judge only the steps below or above the default); a
                // secondary statistic is reported beside it and does not decide the step.
                let pass = stats.first().is_some_and(|s| s.pass);
                out.push(StepVerdict {
                    primitive: name.into(),
                    step,
                    stats,
                    pass,
                    means: arm_means(&pairs),
                    differences: arm_differences(&pairs),
                });
            }
        }
        PrimitiveKind::Choice => {
            let default = p
                .choices
                .iter()
                .find(|c| c.name == p.default)
                .context("the default choice")?;
            // A choice whose inputs the choices own is selected in the settings; one that sets
            // a level block's input is written into a spare level of that block.
            let owned = p
                .inputs
                .iter()
                .all(|i| input_field(&i.input).is_some_and(|(b, ..)| b == OWN));
            let defaults = Tactics::defaults(&content.tactics);
            let arm_of =
                |choice: &engine::data::primitives::Choice| -> anyhow::Result<(Content, Tactics)> {
                    if owned {
                        let k = p
                            .choices
                            .iter()
                            .position(|c| c.name == choice.name)
                            .context("a choice")?;
                        Ok((
                            content.clone(),
                            with_level(defaults, &content.tactics, &p, k)?,
                        ))
                    } else {
                        with_values(&content, &choice.values)
                    }
                };
            let (dc, dt) = arm_of(default)?;
            for choice in p.choices.iter().filter(|c| c.name != p.default) {
                let (cc, ct) = arm_of(choice)?;
                let arms = [
                    Arm {
                        content: dc.clone(),
                        tactics: dt,
                    },
                    Arm {
                        content: cc,
                        tactics: ct,
                    },
                ];
                let step = format!("{} -> {}", default.name, choice.name);
                let pairs = play_pairs(setup, &content, &arms, &format!("{name}:{step}"))?;
                let stats = vec![judge(&pairs, stat_index(&choice.stat)?, Direction::Up)];
                let pass = stats[0].pass;
                out.push(StepVerdict {
                    primitive: name.into(),
                    step,
                    stats,
                    pass,
                    means: arm_means(&pairs),
                    differences: arm_differences(&pairs),
                });
            }
        }
        PrimitiveKind::OnOff => {
            // Off to on: the primitive owns its two states, off first.
            if !p.is_setting() {
                bail!("{name}: an on/off primitive is checked when it owns its two states");
            }
            let defaults = Tactics::defaults(&content.tactics);
            let arms = [
                Arm {
                    content: content.clone(),
                    tactics: with_level(defaults, &content.tactics, &p, 0)?,
                },
                Arm {
                    content: content.clone(),
                    tactics: with_level(defaults, &content.tactics, &p, 1)?,
                },
            ];
            let step = format!("{} -> {}", names[0], names[1]);
            let pairs = play_pairs(setup, &content, &arms, &format!("{name}:{step}"))?;
            let mut stats = Vec::new();
            for s in &p.signature {
                stats.push(judge(&pairs, stat_index(&s.stat)?, s.direction));
            }
            let pass = stats.first().is_some_and(|s| s.pass);
            out.push(StepVerdict {
                primitive: name.into(),
                step,
                stats,
                pass,
                means: arm_means(&pairs),
                differences: arm_differences(&pairs),
            });
        }
    }
    Ok(out)
}

/// One evaluation of the fit's search.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Probe {
    pub value: f64,
    pub mean: f64,
    pub noise: f64,
}

/// The result of a search.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Search {
    pub value: f64,
    pub achieved: f64,
    pub noise: f64,
    /// `true` when the target lies outside what the range reaches; the value is the range
    /// end nearest it.
    pub unreachable: bool,
    pub probes: Vec<Probe>,
}

/// Bisection of `eval` over `[lo, hi]` toward `target`: the two ends first, then the middle
/// of the bracket, until a value's mean lies within half its noise of the target or
/// `max_evals` evaluations are spent (the closest probe then). Half the noise, so that two
/// neighbouring levels fitted to nearby targets do not both stop at the same edge of the
/// noise and leave a step the monotonic check cannot see. `eval` gives a mean and its noise;
/// the mean is taken to move one way across the range; the target is out of reach when it
/// lies beyond both ends by more than their noise.
pub fn bisect(
    mut eval: impl FnMut(f64) -> anyhow::Result<(f64, f64)>,
    lo: f64,
    hi: f64,
    target: f64,
    max_evals: usize,
) -> anyhow::Result<Search> {
    let mut probes = Vec::new();
    let mut run = |x: f64, probes: &mut Vec<Probe>| -> anyhow::Result<Probe> {
        let (mean, noise) = eval(x)?;
        let p = Probe {
            value: x,
            mean,
            noise,
        };
        probes.push(p.clone());
        Ok(p)
    };
    let a = run(lo, &mut probes)?;
    let b = run(hi, &mut probes)?;
    let rising = b.mean >= a.mean;
    let (low, high) = if rising { (&a, &b) } else { (&b, &a) };
    if target < low.mean - low.noise || target > high.mean + high.noise {
        let end = if (target - a.mean).abs() <= (target - b.mean).abs() {
            a.clone()
        } else {
            b.clone()
        };
        return Ok(Search {
            value: end.value,
            achieved: end.mean,
            noise: end.noise,
            unreachable: true,
            probes,
        });
    }
    let close = |p: &Probe| (p.mean - target).abs() <= 0.5 * p.noise;
    let (mut x0, mut x1) = (lo, hi);
    for p in [&a, &b] {
        if close(p) {
            return Ok(Search {
                value: p.value,
                achieved: p.mean,
                noise: p.noise,
                unreachable: false,
                probes,
            });
        }
    }
    while probes.len() < max_evals {
        let mid = 0.5 * (x0 + x1);
        let p = run(mid, &mut probes)?;
        if close(&p) {
            return Ok(Search {
                value: p.value,
                achieved: p.mean,
                noise: p.noise,
                unreachable: false,
                probes,
            });
        }
        // Keep the half whose ends bracket the target.
        if (p.mean < target) == rising {
            x0 = mid;
        } else {
            x1 = mid;
        }
    }
    let best = probes
        .iter()
        .min_by(|p, q| (p.mean - target).abs().total_cmp(&(q.mean - target).abs()))
        .cloned()
        .context("a probe")?;
    Ok(Search {
        value: best.value,
        achieved: best.mean,
        noise: best.noise,
        unreachable: false,
        probes,
    })
}

/// A secondary signature statistic at a level's fit.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Secondary {
    pub stat: String,
    /// The engine's ratio of the level to the default level, and its noise.
    pub ratio: f64,
    pub noise: f64,
    /// The engine's pooled value at the level.
    pub engine: f64,
    /// The real quintile range of the level over the real median of the default level's
    /// quintile, and whether the engine's ratio lies inside it.
    pub real_range: [f64; 2],
    pub inside: bool,
    /// The real median of the level's quintile.
    pub real: f64,
}

/// The fit of one level.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FitResult {
    pub primitive: String,
    pub level: String,
    pub input: String,
    pub stat: String,
    /// The ratio the fit aims at: the real median of the level's quintile over the real median
    /// of the default level's quintile.
    pub target: f64,
    /// The real medians of the level's and of the default level's quintiles.
    pub real: f64,
    pub real_default: f64,
    /// The engine's pooled primary statistic at the default level and at the fitted value:
    /// their gap from the real medians is recorded, not fitted.
    pub engine_default: f64,
    pub engine: f64,
    pub source: String,
    pub competitions: Vec<String>,
    pub row: String,
    pub matches: u32,
    pub seed: u64,
    /// The search over the first plan input; its means are ratios to the default level.
    pub search: Search,
    /// The level's other plan inputs, at their content values.
    pub held: BTreeMap<String, f64>,
    /// The other signature statistics at the fitted value.
    pub secondary: Vec<Secondary>,
}

/// The row `row`, statistic `stat`, of the signature table.
fn table_row<'a>(
    table: &'a serde_json::Value,
    row: &str,
    stat: &str,
) -> anyhow::Result<&'a serde_json::Value> {
    table["rows"]
        .as_array()
        .context("signatures.json has no rows")?
        .iter()
        .find(|r| r["primitive"] == row && r["stat"] == stat)
        .with_context(|| format!("signatures.json has no {row} {stat} row"))
}

/// The real median of `primitive`'s statistic `stat` at quintile `q` from the table.
pub fn target(table: &serde_json::Value, row: &str, stat: &str, q: usize) -> anyhow::Result<f64> {
    table_row(table, row, stat)?["medians"][q]
        .as_f64()
        .with_context(|| format!("signatures.json {row} {stat}: no median {q}"))
}

/// The pooled ratio of the second arm to the first over `units`, and 1.96 jackknife standard
/// errors.
pub fn pooled_ratio(units: &[Vec<(f64, f64)>]) -> (f64, f64) {
    let n = units.len();
    let sums = |skip: Option<usize>| {
        let mut s = [(0.0, 0.0); 2];
        for (i, u) in units.iter().enumerate() {
            if Some(i) == skip {
                continue;
            }
            for a in 0..2 {
                s[a].0 += u[a].0;
                s[a].1 += u[a].1;
            }
        }
        ratio(s[1].0, s[1].1) / ratio(s[0].0, s[0].1)
    };
    let all = sums(None);
    if n < 2 {
        return (all, f64::INFINITY);
    }
    let left: Vec<f64> = (0..n).map(|i| sums(Some(i))).collect();
    let mean = left.iter().sum::<f64>() / n as f64;
    let var = left.iter().map(|v| (v - mean).powi(2)).sum::<f64>();
    (all, 1.96 * ((n - 1) as f64 / n as f64 * var).sqrt())
}

/// Fits level `level` of primitive `name` against `table` (`signatures.json`): the level's
/// first plan input is searched, with its other inputs at their content values and every
/// other primitive at its default, until the ratio of the side's primary statistic to the
/// default level's lies within its noise of the real ratio: the median of the level's real
/// quintile over the median of the default level's. The default level is the base of every
/// ratio and keeps its value.
pub fn fit(
    content: &Content,
    table: &serde_json::Value,
    name: &str,
    level: &str,
    setup: &Setup,
    max_evals: usize,
) -> anyhow::Result<FitResult> {
    let content = harness_content(content);
    let p = primitive(&content.tactics, name)?.clone();
    if p.kind != PrimitiveKind::Graded {
        bail!("{name}: only a graded primitive is fitted level by level");
    }
    let source = p
        .data
        .source
        .clone()
        .with_context(|| format!("{name} names no real data"))?;
    for c in &p.data.competitions {
        if held_out(&source, c) {
            bail!("held-out set: {c} of {source} is never read");
        }
    }
    let row = p.data.row.clone().context("a data row")?;
    let names = p.level_names(&content.tactics);
    let q = names
        .iter()
        .position(|n| *n == level)
        .with_context(|| format!("{name} has no level {level}"))?;
    let d = p.default_index(&content.tactics).context("a default")?;
    if q == d {
        bail!("{name}: {level} is the default level, the base of every ratio; it keeps its value");
    }
    // The level's primary statistic: the first that judges it. A statistic that falls as the
    // level rises reads the real spread from the other end: the lowest level takes the highest
    // quintile.
    let primary = p
        .signature
        .iter()
        .find(|s| s.steps.covers_level(q, d))
        .context("a signature that judges the level")?;
    let stat = primary.stat.clone();
    let at = |level: usize| match primary.direction {
        Direction::Up => level,
        Direction::Down => GRADED_LEVELS - 1 - level,
    };
    let (real, real_default) = (
        target(table, &row, &stat, at(q))?,
        target(table, &row, &stat, at(d))?,
    );
    let first = p.inputs.first().context("an input")?.input.clone();
    let (_, _, lo, hi) = input_field(&first).with_context(|| format!("{first} has no range"))?;
    // A statistic that judges only one side of the default is searched on that side of the
    // default level's value.
    let middle = level_value(&content.tactics, &first, d).context("the default's value")?;
    let (lo, hi) = match primary.steps {
        Steps::All => (lo, hi),
        Steps::BelowDefault => (lo, middle),
        Steps::AboveDefault => (middle, hi),
    };
    let held: BTreeMap<String, f64> = p
        .inputs
        .iter()
        .skip(1)
        .filter_map(|i| level_value(&content.tactics, &i.input, q).map(|v| (i.input.clone(), v)))
        .collect();
    let si = stat_index(&stat)?;
    let defaults = Tactics::defaults(&content.tactics);
    let salt = format!("{name}:fit");
    let base = play_pairs(
        setup,
        &content,
        &[Arm {
            content: content.clone(),
            tactics: defaults,
        }],
        &salt,
    )?;
    let tested = with_level(defaults, &content.tactics, &p, q)?;
    let played: std::cell::RefCell<Vec<(f64, Vec<Vec<Stats>>)>> = Default::default();
    let eval = |x: f64| -> anyhow::Result<(f64, f64)> {
        let mut c = content.clone();
        set_level_value(&mut c.tactics, &first, q, x);
        let arm = Arm {
            content: c,
            tactics: tested,
        };
        let cand = play_pairs(setup, &content, &[arm], &salt)?;
        let units: Vec<Vec<(f64, f64)>> = base
            .iter()
            .zip(&cand)
            .map(|(b, c)| vec![b[0][si], c[0][si]])
            .collect();
        played.borrow_mut().push((x, cand));
        Ok(pooled_ratio(&units))
    };
    let search = bisect(eval, lo, hi, real / real_default, max_evals)?;
    let played = played.into_inner();
    let cand = &played
        .iter()
        .find(|(x, _)| *x == search.value)
        .context("the fitted value was played")?
        .1;
    let pooled_of = |pairs: &[Vec<Stats>], s: usize| pooled(&units(pairs, s, &[0])).0;
    let mut secondary = Vec::new();
    for s in p
        .signature
        .iter()
        .filter(|s| s.stat != stat && s.steps.covers_level(q, d))
    {
        let k = stat_index(&s.stat)?;
        let units: Vec<Vec<(f64, f64)>> = base
            .iter()
            .zip(cand)
            .map(|(b, c)| vec![b[0][k], c[0][k]])
            .collect();
        let (r, noise) = pooled_ratio(&units);
        let real_row = table_row(table, &row, &s.stat).ok();
        let (real_range, real_q) = match real_row {
            Some(rr) => {
                let base = rr["medians"][d].as_f64().unwrap_or(f64::NAN);
                let lo = rr["quintile_ranges"][q][0].as_f64().unwrap_or(f64::NAN);
                let hi = rr["quintile_ranges"][q][1].as_f64().unwrap_or(f64::NAN);
                (
                    [lo / base, hi / base],
                    rr["medians"][q].as_f64().unwrap_or(f64::NAN),
                )
            }
            None => ([f64::NAN; 2], f64::NAN),
        };
        secondary.push(Secondary {
            stat: s.stat.clone(),
            ratio: r,
            noise,
            engine: pooled_of(cand, k),
            real_range,
            inside: (real_range[0]..=real_range[1]).contains(&r),
            real: real_q,
        });
    }
    Ok(FitResult {
        primitive: name.into(),
        level: level.into(),
        input: first,
        stat,
        target: real / real_default,
        real,
        real_default,
        engine_default: pooled_of(&base, si),
        engine: pooled_of(cand, si),
        source,
        competitions: p.data.competitions.clone(),
        row,
        matches: setup.pairs,
        seed: setup.seed,
        search,
        held,
        secondary,
    })
}

/// The reach of a primitive: each signature statistic, pooled, with its 1.96 jackknife
/// noise, at the two ends of its first input's range, with the level's other inputs at their
/// default level values.
pub fn reach(
    content: &Content,
    name: &str,
    setup: &Setup,
) -> anyhow::Result<BTreeMap<String, [(f64, f64); 2]>> {
    let content = harness_content(content);
    let p = primitive(&content.tactics, name)?.clone();
    let first = p.inputs.first().context("an input")?.input.clone();
    let (_, _, lo, hi) = input_field(&first).with_context(|| format!("{first} has no range"))?;
    let q = p.default_index(&content.tactics).context("a default")?;
    let mut out: BTreeMap<String, [(f64, f64); 2]> = BTreeMap::new();
    let mut ends = Vec::new();
    for x in [lo, hi] {
        let mut values: BTreeMap<String, f64> = p
            .inputs
            .iter()
            .skip(1)
            .filter_map(|i| {
                level_value(&content.tactics, &i.input, q).map(|v| (i.input.clone(), v))
            })
            .collect();
        values.insert(first.clone(), x);
        let (c, t) = with_values(&content, &values)?;
        let arms = [Arm {
            content: c,
            tactics: t,
        }];
        ends.push(play_pairs(
            setup,
            &content,
            &arms,
            &format!("{name}:reach"),
        )?);
    }
    for s in &p.signature {
        let si = stat_index(&s.stat)?;
        let at = |e: usize| {
            let (m, se, _) = pooled(&units(&ends[e], si, &[0]));
            (m, 1.96 * se)
        };
        out.insert(s.stat.clone(), [at(0), at(1)]);
    }
    Ok(out)
}

/// The jobs of a run: the number given, else the logical cores.
fn jobs(given: Option<u32>) -> u32 {
    given.unwrap_or_else(|| {
        std::thread::available_parallelism().map_or(1, |n| u32::try_from(n.get()).unwrap_or(1))
    })
}

/// Prints `value` as one JSON line and writes it pretty to `out` when given.
fn emit(value: &impl Serialize, out: Option<&std::path::Path>) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string(value)?);
    if let Some(path) = out {
        let mut text = serde_json::to_string_pretty(value)?;
        text.push('\n');
        std::fs::write(path, text).with_context(|| format!("cannot write {}", path.display()))?;
    }
    Ok(())
}

/// `engine-cli primitives`: exit 0 when every step passes or the fit ran, 2 when a step fails.
pub fn run(
    content_dir: Option<&std::path::Path>,
    opts: &crate::cli::PrimitivesOpts,
) -> anyhow::Result<i32> {
    use crate::cli::PrimitivesAction;
    let loaded = crate::content::load(content_dir, None, None, None)?;
    let content = &loaded.content;
    match &opts.action {
        PrimitivesAction::Check {
            primitive,
            step,
            with,
            pairs,
            seed,
            jobs: j,
            minutes,
            out,
        } => {
            let setup = Setup {
                seed: *seed,
                pairs: *pairs,
                minutes: *minutes,
                jobs: jobs(*j),
            };
            let verdicts = check(content, primitive, step.as_deref(), with, &setup)?;
            let pass = verdicts.iter().all(|v| v.pass);
            for v in &verdicts {
                let failed: Vec<String> = v
                    .stats
                    .iter()
                    .skip(1)
                    .filter(|s| !s.pass)
                    .map(|s| {
                        format!(
                            "{} moved {:+.4} (noise {:.4}, wanted {})",
                            s.stat, s.difference, s.noise, s.direction
                        )
                    })
                    .collect();
                if !failed.is_empty() {
                    eprintln!(
                        "{}: {} secondary miss: {}",
                        v.primitive,
                        v.step,
                        failed.join("; ")
                    );
                }
            }
            for v in verdicts.iter().filter(|v| !v.pass) {
                let failed: Vec<String> = v
                    .stats
                    .iter()
                    .take(1)
                    .filter(|s| !s.pass)
                    .map(|s| {
                        format!(
                            "{} moved {:+.4} (noise {:.4}, wanted {})",
                            s.stat, s.difference, s.noise, s.direction
                        )
                    })
                    .collect();
                eprintln!("{}: {} fails: {}", v.primitive, v.step, failed.join("; "));
            }
            emit(
                &serde_json::json!({"kind": "primitives-check", "pass": pass, "steps": verdicts}),
                out.as_deref(),
            )?;
            Ok(if pass { 0 } else { 2 })
        }
        PrimitivesAction::Fit {
            primitive,
            level,
            matches,
            seed,
            jobs: j,
            minutes,
            evals,
            table,
            out,
        } => {
            let path = match table {
                Some(p) => p.clone(),
                None => engine::ContentDir::resolve(content_dir)?
                    .root()
                    .join("signatures.json"),
            };
            let table: serde_json::Value = serde_json::from_slice(
                &std::fs::read(&path).with_context(|| format!("cannot read {}", path.display()))?,
            )?;
            let setup = Setup {
                seed: *seed,
                pairs: *matches,
                minutes: *minutes,
                jobs: jobs(*j),
            };
            let result = fit(content, &table, primitive, level, &setup, *evals)?;
            emit(&result, out.as_deref())?;
            Ok(0)
        }
        PrimitivesAction::Reach {
            primitive,
            matches,
            seed,
            jobs: j,
            minutes,
            out,
        } => {
            let setup = Setup {
                seed: *seed,
                pairs: *matches,
                minutes: *minutes,
                jobs: jobs(*j),
            };
            let ends = reach(content, primitive, &setup)?;
            emit(
                &serde_json::json!({"kind": "primitives-reach", "primitive": primitive, "ends": ends}),
                out.as_deref(),
            )?;
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bisection finds a target inside a monotone evaluator's reach within the noise, in
    /// either direction, and reports a target outside the reach as unreachable with the
    /// nearer end.
    #[test]
    fn the_bisection_finds_a_reachable_target_and_names_an_unreachable_one() {
        let rising = |x: f64| Ok((2.0 + 3.0 * x, 0.01));
        let s = bisect(rising, 0.0, 1.0, 3.5, 12).unwrap();
        assert!(!s.unreachable);
        assert!((s.achieved - 3.5).abs() <= 0.01, "{s:?}");
        assert!((s.value - 0.5).abs() < 0.01);
        let falling = |x: f64| Ok((5.0 - 4.0 * x, 0.01));
        let s = bisect(falling, 0.0, 1.0, 2.0, 12).unwrap();
        assert!(!s.unreachable);
        assert!((s.value - 0.75).abs() < 0.01, "{s:?}");
        let s = bisect(rising, 0.0, 1.0, 9.0, 12).unwrap();
        assert!(s.unreachable);
        assert_eq!(s.value, 1.0);
        assert_eq!(s.probes.len(), 2);
        let s = bisect(rising, 0.0, 1.0, 3.5001, 4).unwrap();
        assert!(s.probes.len() <= 4);
    }

    #[test]
    fn a_paired_step_passes_only_beyond_the_noise() {
        let mut pairs = Vec::new();
        for i in 0..20 {
            let mut a = [(0.0, 1.0); STATS.len()];
            let mut b = [(0.0, 1.0); STATS.len()];
            a[0] = (30.0 + f64::from(i % 3), 10.0);
            b[0] = (a[0].0 + 5.0 + f64::from(i % 2) * 0.5, 10.0);
            a[1] = (20.0, 100.0);
            b[1] = (20.0 + if i % 2 == 0 { 1.0 } else { -1.0 }, 100.0);
            pairs.push(vec![a, b]);
        }
        assert!(judge(&pairs, 0, Direction::Up).pass);
        assert!(!judge(&pairs, 0, Direction::Down).pass);
        assert!(!judge(&pairs, 1, Direction::Up).pass, "inside the noise");
    }

    /// Every statistic's paired difference comes with its noise, so a statistic a step must
    /// leave alone reads inside its noise and one the step moves reads beyond it.
    #[test]
    fn every_statistic_reports_its_paired_difference_and_noise() {
        let mut pairs = Vec::new();
        for i in 0..20 {
            let mut a = [(0.0, 1.0); STATS.len()];
            let mut b = [(0.0, 1.0); STATS.len()];
            a[0] = (30.0 + f64::from(i % 3), 10.0);
            b[0] = (a[0].0 + 5.0, 10.0);
            let held = stat_index("hold_dribble_share").unwrap();
            a[held] = (20.0, 100.0);
            b[held] = (20.0 + if i % 2 == 0 { 1.0 } else { -1.0 }, 100.0);
            pairs.push(vec![a, b]);
        }
        let d = arm_differences(&pairs);
        assert_eq!(d.len(), STATS.len());
        let [moved, noise] = d["passes_per_possession"];
        assert!(moved > noise, "{moved} {noise}");
        let [held, noise] = d["hold_dribble_share"];
        assert!(held.abs() <= noise && noise > 0.0, "{held} {noise}");
    }

    /// Pooling is a sum over a sum: one short match with a wild ratio moves the pooled value
    /// by its weight only; the ratio of two arms is their pooled values' ratio.
    #[test]
    fn the_pooled_value_weighs_each_match_by_its_denominator() {
        let units = vec![vec![(10.0, 100.0)], vec![(9.0, 1.0)]];
        let (v, _, n) = pooled(&units);
        assert_eq!(n, 2);
        assert!((v - 19.0 / 101.0).abs() < 1e-12);
        let two = vec![
            vec![(10.0, 10.0), (12.0, 10.0)],
            vec![(20.0, 10.0), (24.0, 10.0)],
        ];
        let (r, noise) = pooled_ratio(&two);
        assert!((r - 1.2).abs() < 1e-12);
        assert!(noise < 1e-9, "every pair agrees: {noise}");
    }
}

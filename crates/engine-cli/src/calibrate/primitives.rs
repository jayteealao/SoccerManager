//! The primitives harness (a `measures` build only): the monotonic check and the fit of a
//! tactical primitive, from played matches.
//!
//! A pair is two matches with identical squads (the equal suite's home club on both sides),
//! the same engine seed, and the opponent in the tactics file's defaults (the AI manager makes
//! substitutions as usual and no tactical change of its own). Only the tested side's setting differs between the two matches, and the
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
use engine::measures::players::PlayersSink;
use engine::measures::runs::RunsSink;
use engine::measures::shape::BlockHeightSink;
use engine::tactics::rules::GameState;
use engine::team::PLAYERS_PER_TEAM;
use engine::{Content, FanoutSink, MatchConfig, Simulation, Tactics};
use serde::Serialize;

use super::fixtures::{self, Leagues};

/// The statistics the harness measures, in value order: the signature statistics, then the
/// side's possessions and shots a match, which every step reports beside them.
pub const STATS: [&str; 91] = [
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
    "restart_wait_leading_late_s",
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
    "pass_completion_0_1m",
    "pass_completion_1_2m",
    "pass_completion_2_3m",
    "pass_completion_3_5m",
    "pass_completion_5m_plus",
    "touch_loss_0_1m",
    "touch_loss_1_2m",
    "touch_loss_2_3m",
    "touch_loss_3_5m",
    "touch_loss_5m_plus",
    "free_long_share",
    "pressed_long_share",
    "free_long_clear_share",
    "pressed_long_clear_share",
    "free_completion_0_10m",
    "pressed_completion_0_10m",
    "free_completion_10_20m",
    "pressed_completion_10_20m",
    "free_completion_20_30m",
    "pressed_completion_20_30m",
    "free_completion_30m_plus",
    "pressed_completion_30m_plus",
    "takeon_success_share",
    "opp_short_goal_kick_share",
    "opp_short_goal_kick_kept_share",
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

fn ratio_of(n: f64, d: f64) -> f64 {
    ratio(n, d)
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
        (s.late_lead_wait_s, f(s.late_lead_restarts)),
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
        (f(s.band_completed[0]), f(s.band_passes[0])),
        (f(s.band_completed[1]), f(s.band_passes[1])),
        (f(s.band_completed[2]), f(s.band_passes[2])),
        (f(s.band_completed[3]), f(s.band_passes[3])),
        (f(s.band_completed[4]), f(s.band_passes[4])),
        (f(s.band_lost[0]), f(s.band_touches[0])),
        (f(s.band_lost[1]), f(s.band_touches[1])),
        (f(s.band_lost[2]), f(s.band_touches[2])),
        (f(s.band_lost[3]), f(s.band_touches[3])),
        (f(s.band_lost[4]), f(s.band_touches[4])),
        (f(s.pressure_long[0]), f(s.pressure_passes[0])),
        (f(s.pressure_long[1]), f(s.pressure_passes[1])),
        (
            f(s.pressure_long[0] + s.pressure_clears[0]),
            f(s.pressure_passes[0] + s.pressure_clears[0]),
        ),
        (
            f(s.pressure_long[1] + s.pressure_clears[1]),
            f(s.pressure_passes[1] + s.pressure_clears[1]),
        ),
        (f(s.length_completed[0][0]), f(s.length_passes[0][0])),
        (f(s.length_completed[1][0]), f(s.length_passes[1][0])),
        (f(s.length_completed[0][1]), f(s.length_passes[0][1])),
        (f(s.length_completed[1][1]), f(s.length_passes[1][1])),
        (f(s.length_completed[0][2]), f(s.length_passes[0][2])),
        (f(s.length_completed[1][2]), f(s.length_passes[1][2])),
        (f(s.length_completed[0][3]), f(s.length_passes[0][3])),
        (f(s.length_completed[1][3]), f(s.length_passes[1][3])),
        (f(s.takeons), f(s.takeons + s.run_lost)),
        (f(o.short_goal_kicks), f(o.goal_kicks)),
        (f(o.short_goal_kicks_kept), f(o.short_goal_kicks)),
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
    play_pairs_by(
        setup,
        teams_of,
        arms.len(),
        salt,
        |a, teams, tested, seed| {
            let arm = &arms[a];
            let opponent = Tactics::defaults(&arm.content.tactics);
            let mut tactics = [opponent; 2];
            tactics[tested] = arm.tactics;
            Ok(play_one(&arm.content, teams, tactics, seed, setup.minutes)?[tested])
        },
    )
}

/// Plays `arms` arms on each of `pairs` fixtures on `jobs` threads: `play(arm, teams, tested,
/// seed)` plays one arm of one fixture, the tested side alternating home and away, and gives
/// what the run keeps of it. Per fixture, each arm's result in arm order.
pub fn play_pairs_by<T: Send>(
    setup: &Setup,
    teams_of: &Content,
    arms: usize,
    salt: &str,
    play: impl Fn(usize, &[TeamFile; 2], usize, u64) -> anyhow::Result<T> + Sync,
) -> anyhow::Result<Vec<Vec<T>>> {
    let list = fixtures::fixtures(setup.pairs);
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(usize, Vec<T>)>> = Mutex::new(Vec::new());
    let first_error: Mutex<Option<anyhow::Error>> = Mutex::new(None);
    let threads = usize::try_from(setup.jobs.max(1))
        .unwrap_or(1)
        .min(list.len().max(1));
    std::thread::scope(|scope| {
        for _ in 0..threads {
            let (list, next, results, first_error, play) =
                (&list, &next, &results, &first_error, &play);
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
                    let mut out = Vec::with_capacity(arms);
                    let mut failed = None;
                    for a in 0..arms {
                        match play(a, &teams, tested, engine_seed) {
                            Ok(kept) => out.push(kept),
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
/// each graded step: a combined step against both at their lower levels. The levels `hold` of
/// other primitives are set in both arms of every step, so the step is read with them there.
pub fn check(
    content: &Content,
    name: &str,
    from: Option<&str>,
    with: &[String],
    hold: &[String],
    setup: &Setup,
) -> anyhow::Result<Vec<StepVerdict>> {
    let p = primitive(&content.tactics, name)?.clone();
    let levels_of = |given: &[String], flag: &str| -> anyhow::Result<Vec<(Primitive, usize)>> {
        let mut out = Vec::new();
        for w in given {
            let (other, level) = w
                .split_once('=')
                .with_context(|| format!("--{flag} {w}: give <primitive>=<level>"))?;
            if other == name {
                bail!("--{flag} {w}: the checked primitive cannot be set beside itself");
            }
            let o = primitive(&content.tactics, other)?.clone();
            let at = o
                .level_names(&content.tactics)
                .iter()
                .position(|n| *n == level)
                .with_context(|| format!("{other} has no level {level}"))?;
            out.push((o, at));
        }
        Ok(out)
    };
    let extra = levels_of(with, "with")?;
    let held = levels_of(hold, "hold")?;
    // Both arms of a step take the held levels.
    let hold_in = |arms: &mut [Arm]| -> anyhow::Result<()> {
        for arm in arms.iter_mut() {
            for (o, at) in &held {
                arm.tactics = with_level(arm.tactics, &content.tactics, o, *at)?;
            }
        }
        Ok(())
    };
    let held_note = |step: &mut String| {
        for h in hold {
            step.push_str(&format!(" holding {h}"));
        }
    };
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
                hold_in(&mut arms)?;
                let mut step = format!("{} -> {}", names[l], names[l + 1]);
                for w in with {
                    step.push_str(&format!(" with {w}"));
                }
                held_note(&mut step);
                let pairs = play_pairs(setup, content, &arms, &format!("{name}:{step}"))?;
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
                        with_values(content, &choice.values)
                    }
                };
            let (dc, dt) = arm_of(default)?;
            for choice in p.choices.iter().filter(|c| c.name != p.default) {
                let (cc, ct) = arm_of(choice)?;
                let mut arms = [
                    Arm {
                        content: dc.clone(),
                        tactics: dt,
                    },
                    Arm {
                        content: cc,
                        tactics: ct,
                    },
                ];
                hold_in(&mut arms)?;
                let mut step = format!("{} -> {}", default.name, choice.name);
                held_note(&mut step);
                let pairs = play_pairs(setup, content, &arms, &format!("{name}:{step}"))?;
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
            // Off to on: its own two states, or its level block's two levels, off first.
            let defaults = Tactics::defaults(&content.tactics);
            let mut arms = [
                Arm {
                    content: content.clone(),
                    tactics: with_level(defaults, &content.tactics, &p, 0)?,
                },
                Arm {
                    content: content.clone(),
                    tactics: with_level(defaults, &content.tactics, &p, 1)?,
                },
            ];
            hold_in(&mut arms)?;
            let mut step = format!("{} -> {}", names[0], names[1]);
            held_note(&mut step);
            let pairs = play_pairs(setup, content, &arms, &format!("{name}:{step}"))?;
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

/// The statistics of one player that the per-player check reads, in [`PlayerStats`] order:
/// each a primitive's team signature counted for one player
/// ([`engine::measures::players`]).
pub const PLAYER_STATS: [&str; 12] = [
    "takeons_per_90",
    "crosses_per_90",
    "high_cross_share",
    "low_cross_share",
    "cross_origin_m",
    "outside_box_shot_share",
    "runs_in_behind_share",
    "width_m",
    "pressures_per_90",
    "tackles_per_90",
    "window_forward_share",
    "counterpress_pressures_per_loss",
];

/// One side's per-player statistics of one match: per statistic, the instructed player's
/// numerator and denominator, then the pooled ones of his ten team-mates.
pub type PlayerStats = [[(f64, f64); 2]; PLAYER_STATS.len()];

/// The player statistic that judges an instruction of `primitive` one level up, rising: the
/// team signature counted for the one player. A choice primitive judges each choice on its
/// own statistic, as the team check does.
pub fn player_signature(primitive: &str) -> Option<&'static str> {
    Some(match primitive {
        "dribbling" => "takeons_per_90",
        "crossing" => "crosses_per_90",
        "final_third" => "outside_box_shot_share",
        "forward_runs" => "runs_in_behind_share",
        "width" => "width_m",
        "pressing" => "pressures_per_90",
        "tackling" => "tackles_per_90",
        "on_winning_the_ball" => "window_forward_share",
        "on_losing_the_ball" => "counterpress_pressures_per_loss",
        _ => return None,
    })
}

/// Side `team`'s per-player statistics with the player in formation slot `slot` apart.
pub fn player_side(
    m: &MatchMeasures,
    sink: &PlayersSink,
    nineties: f64,
    team: usize,
    slot: usize,
) -> PlayerStats {
    let f = f64::from;
    let losses = f(m.transitions[team].losses);
    let one = |i: usize| -> [(f64, f64); PLAYER_STATS.len()] {
        let c = &m.per_player[i];
        let width = (
            sink.width_m.get(i).copied().unwrap_or(0.0),
            f(sink.width_steps.get(i).copied().unwrap_or(0)),
        );
        let runs = f(sink.runs.get(i).copied().unwrap_or(0));
        let behind = f(sink.behind.get(i).copied().unwrap_or(0));
        [
            (f(c.takeons), nineties),
            (f(c.crosses), nineties),
            (f(c.high_crosses), f(c.crosses)),
            (f(c.crosses - c.high_crosses), f(c.crosses)),
            (c.cross_origin_m, f(c.crosses)),
            (f(c.outside_shots), f(c.open_shots)),
            (behind, runs),
            width,
            (f(c.pressures), nineties),
            (f(c.tackles), nineties),
            (f(c.window_forward), f(c.window_passes)),
            (f(c.counterpress_pressures), losses),
        ]
    };
    let first = team * PLAYERS_PER_TEAM;
    let own = one(first + slot);
    let mut others = [(0.0, 0.0); PLAYER_STATS.len()];
    for s in (0..PLAYERS_PER_TEAM).filter(|&s| s != slot) {
        for (sum, x) in others.iter_mut().zip(one(first + s)) {
            sum.0 += x.0;
            sum.1 += x.1;
        }
    }
    // A rate per 90 or per loss pools the ten players' counts over the one denominator.
    for k in [0, 1, 8, 9, 11] {
        others[k].1 = own[k].1;
    }
    std::array::from_fn(|k| [own[k], others[k]])
}

/// One match of a per-player check: the tested side's per-player statistics, its player in
/// `slot` playing `state` (his instruction; empty for none).
#[allow(clippy::too_many_arguments)]
pub fn play_one_player(
    content: &Content,
    teams: &[TeamFile; 2],
    tactics: [Tactics; 2],
    state: &GameState,
    tested: usize,
    slot: usize,
    seed: u64,
    minutes: u32,
) -> anyhow::Result<PlayerStats> {
    let mut config = MatchConfig::new(seed, minutes, content, [&teams[0], &teams[1]])?;
    for (team, t) in tactics.into_iter().enumerate() {
        config = config.with_ai_tactics(team, t);
    }
    if !state.is_empty() {
        config = config.with_game_state(tested, state.clone());
    }
    let mut sim = Simulation::new(config)?;
    sim.with_measures();
    let mut sink = PlayersSink::default();
    sim.run(&mut sink)?;
    let m = sim
        .measures()
        .context("a measures build counts the measures")?;
    Ok(player_side(
        m,
        &sink,
        f64::from(minutes) / 90.0,
        tested,
        slot,
    ))
}

/// The verdict of one per-player step or choice: the instructed player's statistic must move
/// in its direction beyond the paired noise, and the same statistic pooled over his ten
/// team-mates must stay inside it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlayerVerdict {
    pub primitive: String,
    pub slot: usize,
    /// The slot's position in the side's formation.
    pub position: String,
    /// `<from> -> <to>`.
    pub step: String,
    pub own: StatVerdict,
    pub others: StatVerdict,
    pub pass: bool,
    /// Every per-player statistic, the player's paired difference and its noise.
    pub differences: BTreeMap<String, [f64; 2]>,
}

/// The verdict of `pairs` on per-player statistic `stat`, the player's (`k` 0) or the pooled
/// team-mates' (`k` 1), moving in `direction`.
fn judge_player(
    pairs: &[Vec<PlayerStats>],
    stat: usize,
    k: usize,
    direction: Direction,
) -> StatVerdict {
    let units = |arms: &[usize]| -> Vec<Vec<(f64, f64)>> {
        pairs
            .iter()
            .map(|p| arms.iter().map(|&a| p[a][stat][k]).collect())
            .collect()
    };
    let (difference, se, n) = pooled(&units(&[0, 1]));
    let noise = 1.96 * se;
    StatVerdict {
        stat: PLAYER_STATS[stat].to_string(),
        direction: match direction {
            Direction::Up => "up".into(),
            Direction::Down => "down".into(),
        },
        before: pooled(&units(&[0])).0,
        after: pooled(&units(&[1])).0,
        difference,
        noise,
        pairs: n,
        pass: if k == 0 {
            difference * direction.sign() > noise
        } else {
            difference.abs() <= noise
        },
    }
}

/// The per-player check of `name` for the player in formation slot `slot` of the tested
/// side: a graded primitive's instruction one level up from the default against no
/// instruction, or each choice of a choice primitive against no instruction. The instruction
/// is resolved as a tactic's is, so an instruction the slot cannot take is refused by name.
pub fn check_player(
    content: &Content,
    name: &str,
    slot: usize,
    setup: &Setup,
) -> anyhow::Result<Vec<PlayerVerdict>> {
    let schema = &content.tactics;
    let p = primitive(schema, name)?.clone();
    let defaults = Tactics::defaults(schema);
    let position = schema.formations[usize::from(defaults.formation)]
        .slots
        .get(slot)
        .map(|s| format!("{:?}", s.position))
        .with_context(|| format!("--player {slot}: the formation has slots 0 to 10"))?;
    let names: Vec<String> = p
        .level_names(schema)
        .iter()
        .map(|s| s.to_string())
        .collect();
    let d = p.default_index(schema).context("a default")?;
    let steps: Vec<(usize, &'static str)> = match p.kind {
        PrimitiveKind::Graded => {
            if d + 1 >= names.len() {
                bail!("{name} has no level above its default");
            }
            let stat = player_signature(name)
                .with_context(|| format!("{name} has no per-player statistic"))?;
            vec![(d + 1, stat)]
        }
        PrimitiveKind::Choice => p
            .choices
            .iter()
            .enumerate()
            .filter(|(_, c)| c.name != p.default)
            .map(|(k, c)| -> anyhow::Result<(usize, &'static str)> {
                let stat = PLAYER_STATS
                    .iter()
                    .find(|s| **s == c.stat)
                    .with_context(|| format!("{}: no per-player {}", c.name, c.stat))?;
                Ok((k, *stat))
            })
            .collect::<anyhow::Result<_>>()?,
        PrimitiveKind::OnOff => {
            bail!("{name}: the per-player check steps graded and choice primitives")
        }
    };
    let mut out = Vec::new();
    for (level, stat) in steps {
        let file = engine::data::game_state::GameStateFile {
            rules: Vec::new(),
            instructions: vec![engine::data::game_state::InstructionFile {
                slot: u8::try_from(slot).context("a slot")?,
                primitive: name.to_string(),
                level: names[level].clone(),
            }],
        };
        let state = engine::data::game_state::resolve(&file, schema, &[defaults.formation])
            .map_err(|(field, reason)| anyhow::anyhow!("{field}: {reason}"))?;
        let states = [GameState::default(), state];
        let step = format!("{} -> {}", names[d], names[level]);
        let pairs = play_pairs_by(
            setup,
            content,
            2,
            &format!("{name}:player {slot}:{step}"),
            |a, teams, tested, seed| {
                let tactics = [defaults; 2];
                play_one_player(
                    content,
                    teams,
                    tactics,
                    &states[a],
                    tested,
                    slot,
                    seed,
                    setup.minutes,
                )
            },
        )?;
        let s = PLAYER_STATS
            .iter()
            .position(|x| *x == stat)
            .context("a player statistic")?;
        let own = judge_player(&pairs, s, 0, Direction::Up);
        let others = judge_player(&pairs, s, 1, Direction::Up);
        let pass = own.pass && others.pass;
        let differences = PLAYER_STATS
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let v = judge_player(&pairs, i, 0, Direction::Up);
                (n.to_string(), [v.difference, v.noise])
            })
            .collect();
        out.push(PlayerVerdict {
            primitive: name.into(),
            slot,
            position: position.clone(),
            step,
            own,
            others,
            pass,
            differences,
        });
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

/// The search of [`bisect`] where the mean need not move one way across `[lo, hi]`: the two
/// ends and the middle first; then [`bisect`] inside the half whose ends bracket the target
/// (the half that holds `prefer` when both do). When neither half brackets it, the target is
/// out of reach and the value is the probe nearest it.
pub fn bisect_bracketed(
    mut eval: impl FnMut(f64) -> anyhow::Result<(f64, f64)>,
    lo: f64,
    hi: f64,
    prefer: f64,
    target: f64,
    max_evals: usize,
) -> anyhow::Result<Search> {
    let mut seen: Vec<Probe> = Vec::new();
    let mut run = |x: f64, seen: &mut Vec<Probe>| -> anyhow::Result<Probe> {
        if let Some(p) = seen.iter().find(|p| p.value == x) {
            return Ok(p.clone());
        }
        let (mean, noise) = eval(x)?;
        let p = Probe {
            value: x,
            mean,
            noise,
        };
        seen.push(p.clone());
        Ok(p)
    };
    let mid = 0.5 * (lo + hi);
    let ends = [
        run(lo, &mut seen)?,
        run(mid, &mut seen)?,
        run(hi, &mut seen)?,
    ];
    let brackets = |a: &Probe, b: &Probe| {
        let (low, high) = if a.mean <= b.mean { (a, b) } else { (b, a) };
        target >= low.mean - low.noise && target <= high.mean + high.noise
    };
    let halves = [(&ends[0], &ends[1]), (&ends[1], &ends[2])];
    let holds = |(a, b): &(&Probe, &Probe)| (a.value..=b.value).contains(&prefer);
    let half = halves
        .iter()
        .filter(|h| brackets(h.0, h.1))
        .max_by_key(|h| holds(h));
    let Some((a, b)) = half else {
        let best = ends
            .iter()
            .min_by(|p, q| (p.mean - target).abs().total_cmp(&(q.mean - target).abs()))
            .cloned()
            .context("a probe")?;
        return Ok(Search {
            value: best.value,
            achieved: best.mean,
            noise: best.noise,
            unreachable: true,
            probes: seen,
        });
    };
    let (x0, x1) = (a.value, b.value);
    let inner = bisect(
        |x| run(x, &mut seen).map(|p| (p.mean, p.noise)),
        x0,
        x1,
        target,
        max_evals.saturating_sub(1).max(2),
    )?;
    let mut probes = seen;
    for p in inner.probes.iter() {
        if !probes.iter().any(|q| q.value == p.value) {
            probes.push(p.clone());
        }
    }
    Ok(Search { probes, ..inner })
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
/// ratio and keeps its value. With `between_neighbours`, the search stays between the values
/// of the level's neighbours (an end level: between its neighbour and the range end), where
/// the statistic need not move one way, so the fitted levels keep their order.
pub fn fit(
    content: &Content,
    table: &serde_json::Value,
    name: &str,
    level: &str,
    setup: &Setup,
    max_evals: usize,
    between_neighbours: bool,
) -> anyhow::Result<FitResult> {
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
    let current = level_value(&content.tactics, &first, q).unwrap_or(lo);
    let (lo, hi) = if between_neighbours {
        let below = q
            .checked_sub(1)
            .and_then(|l| level_value(&content.tactics, &first, l))
            .unwrap_or(lo);
        let above = (q + 1 < GRADED_LEVELS)
            .then(|| level_value(&content.tactics, &first, q + 1))
            .flatten()
            .unwrap_or(hi);
        let (below, above) = (below.max(lo), above.min(hi));
        if below >= above {
            bail!("{name} {level}: its neighbours' values {below} and {above} leave no room");
        }
        (below, above)
    } else {
        (lo, hi)
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
        content,
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
        let cand = play_pairs(setup, content, &[arm], &salt)?;
        let units: Vec<Vec<(f64, f64)>> = base
            .iter()
            .zip(&cand)
            .map(|(b, c)| vec![b[0][si], c[0][si]])
            .collect();
        played.borrow_mut().push((x, cand));
        Ok(pooled_ratio(&units))
    };
    let search = if between_neighbours {
        bisect_bracketed(eval, lo, hi, current, real / real_default, max_evals)?
    } else {
        bisect(eval, lo, hi, real / real_default, max_evals)?
    };
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
        let (c, t) = with_values(content, &values)?;
        let arms = [Arm {
            content: c,
            tactics: t,
        }];
        ends.push(play_pairs(setup, content, &arms, &format!("{name}:reach"))?);
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

/// The fewest actions a player needs in each state before his rates enter the spread, the
/// thresholds of the real rows: pressed and free passes, pressed and free receptions, and
/// tackles faced while running.
pub const SPREAD_MIN: [u32; 5] = [40, 150, 30, 150, 15];

/// The spread of one per-player rate across players: its quantiles (10, 25, 50, 75 and 90
/// percent), its mean, its observed standard deviation, the mean sampling noise, and the
/// spread left when the noise is taken out (method of moments), over `players` players.
#[derive(Debug, Clone, Serialize)]
pub struct Spread {
    pub players: usize,
    pub quantiles: [f64; 5],
    pub mean: f64,
    pub sd_observed: f64,
    pub sd_noise: f64,
    pub sd_true: f64,
    /// The rate's least-squares slope on the player's resistance for the action, the spread
    /// of that resistance across the players, and their product: the part of the rate's
    /// spread the resistance carries.
    pub resistance_slope: f64,
    pub resistance_sd: f64,
    pub resistance_part: f64,
}

/// The spread of `rates`, each a rate with its sampling variance and the player's resistance
/// for the action.
pub fn spread_of(rates: &[(f64, f64, f64)]) -> Spread {
    let n = rates.len();
    let rm = rates.iter().map(|r| r.2).sum::<f64>() / n.max(1) as f64;
    let ym = rates.iter().map(|r| r.0).sum::<f64>() / n.max(1) as f64;
    let sxx: f64 = rates.iter().map(|r| (r.2 - rm).powi(2)).sum();
    let sxy: f64 = rates.iter().map(|r| (r.2 - rm) * (r.0 - ym)).sum();
    let slope = if sxx > 0.0 { sxy / sxx } else { 0.0 };
    let rsd = if n > 1 {
        (sxx / (n - 1) as f64).sqrt()
    } else {
        0.0
    };
    let mut xs: Vec<f64> = rates.iter().map(|r| r.0).collect();
    xs.sort_by(f64::total_cmp);
    let q = |p: f64| {
        if xs.is_empty() {
            return f64::NAN;
        }
        let k = (xs.len() - 1) as f64 * p;
        let lo = k.floor() as usize;
        let hi = (lo + 1).min(xs.len() - 1);
        xs[lo] + (xs[hi] - xs[lo]) * (k - lo as f64)
    };
    let mean = xs.iter().sum::<f64>() / n.max(1) as f64;
    let obs = if n > 1 {
        xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64
    } else {
        0.0
    };
    let noise = rates.iter().map(|r| r.1).sum::<f64>() / n.max(1) as f64;
    Spread {
        players: n,
        quantiles: [q(0.1), q(0.25), q(0.5), q(0.75), q(0.9)],
        mean,
        sd_observed: obs.sqrt(),
        sd_noise: noise.sqrt(),
        sd_true: (obs - noise).max(0.0).sqrt(),
        resistance_slope: slope,
        resistance_sd: rsd,
        resistance_part: slope * rsd,
    }
}

/// The spread of each per-player pressure rate, by name.
pub type SpreadTable = BTreeMap<&'static str, Spread>;
/// The pooled mean of each harness statistic, by name.
pub type MeanTable = BTreeMap<&'static str, f64>;
/// Each side's players' pressure counts in one match, by squad index.
type SidePlayers = [BTreeMap<usize, engine::measures::PlayerPressure>; 2];

/// The pressure spread across the engine's players: each player's pressed-against-free pass
/// completion (ratio), his pressed-minus-free touch loss, and the share of tackles he keeps
/// the ball against while running with it, over `setup.pairs` matches of the equal suite's
/// clubs at the default tactics (each club against its copy), players pooled by club and
/// squad place, with the real rows' action thresholds ([`SPREAD_MIN`]).
pub fn pressure_spread(
    content: &Content,
    setup: &Setup,
) -> anyhow::Result<(SpreadTable, MeanTable)> {
    use engine::measures::PlayerPressure;
    let list = fixtures::fixtures(setup.pairs);
    let next = AtomicUsize::new(0);
    let pooled_players: Mutex<BTreeMap<(String, usize), PlayerPressure>> =
        Mutex::new(BTreeMap::new());
    // Each player's resistance for a pass, a reception and a dribble.
    let resist: Mutex<BTreeMap<(String, usize), [f64; 3]>> = Mutex::new(BTreeMap::new());
    let totals: Mutex<Stats> = Mutex::new([(0.0, 0.0); STATS.len()]);
    let first_error: Mutex<Option<anyhow::Error>> = Mutex::new(None);
    let threads = usize::try_from(setup.jobs.max(1))
        .unwrap_or(1)
        .min(list.len().max(1));
    std::thread::scope(|scope| {
        for _ in 0..threads {
            let (list, next, pooled_players, first_error, content, totals, resist) = (
                &list,
                &next,
                &pooled_players,
                &first_error,
                &content,
                &totals,
                &resist,
            );
            scope.spawn(move || {
                let mut leagues = Leagues::new(setup.seed, content);
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(f) = list.get(i) else { break };
                    let [home, _] = leagues.teams(f);
                    let club = home.club.id.clone();
                    let mut away = home.clone();
                    away.club.id = format!("{club}-b");
                    let league = format!("league:{}", setup.seed.wrapping_add(u64::from(f.league)));
                    let (_, seed) = fixtures::key_of(
                        "primitives",
                        &league,
                        [&club, &club],
                        ["pressure:spread", "pressure:spread"],
                        i as u64,
                    );
                    let run = || -> anyhow::Result<(SidePlayers, [Stats; 2])> {
                        let defaults = Tactics::defaults(&content.tactics);
                        let config =
                            MatchConfig::new(seed, setup.minutes, content, [&home, &away])?
                                .with_ai_tactics(0, defaults)
                                .with_ai_tactics(1, defaults);
                        let mut sim = Simulation::new(config)?;
                        sim.with_measures();
                        let note = |sim: &Simulation| {
                            let mut r = resist.lock().unwrap_or_else(|e| e.into_inner());
                            for (i, p) in sim.players().iter().enumerate() {
                                let k = sim.skills(i);
                                r.entry((club.clone(), p.squad)).or_insert([
                                    engine::pressure::pass_resistance(
                                        k,
                                        engine::contract::Stage::PASS_EXECUTE,
                                        engine::contract::Stage::PASS_PRESSURE,
                                    ),
                                    engine::pressure::reception_resistance(k),
                                    engine::pressure::dribble_resistance(k),
                                ]);
                            }
                        };
                        note(&sim);
                        let mut sinks =
                            FanoutSink::new(BlockHeightSink::default(), RunsSink::default());
                        sim.run(&mut sinks)?;
                        note(&sim);
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
                            nineties: f64::from(setup.minutes) / 90.0,
                        };
                        Ok((
                            m.players.clone(),
                            [side_stats(m, &sinks, 0), side_stats(m, &sinks, 1)],
                        ))
                    };
                    match run() {
                        Ok((sides, stats)) => {
                            {
                                let mut t = totals.lock().unwrap_or_else(|e| e.into_inner());
                                for side in stats {
                                    for (acc, (n, d)) in t.iter_mut().zip(side) {
                                        if n.is_finite() && d.is_finite() {
                                            acc.0 += n;
                                            acc.1 += d;
                                        }
                                    }
                                }
                            }
                            let mut all = pooled_players.lock().unwrap_or_else(|e| e.into_inner());
                            for side in sides {
                                for (squad, p) in side {
                                    let e = all.entry((club.clone(), squad)).or_default();
                                    for k in 0..2 {
                                        e.passes[k] += p.passes[k];
                                        e.completed[k] += p.completed[k];
                                        e.touches[k] += p.touches[k];
                                        e.lost[k] += p.lost[k];
                                    }
                                    e.run_tackles += p.run_tackles;
                                    e.run_lost += p.run_lost;
                                }
                            }
                        }
                        Err(e) => {
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
    let all = pooled_players
        .into_inner()
        .unwrap_or_else(|e| e.into_inner());
    let resist = resist.into_inner().unwrap_or_else(|e| e.into_inner());
    let f = f64::from;
    let (mut ratio, mut gap, mut keep) = (Vec::new(), Vec::new(), Vec::new());
    for (key, p) in &all {
        let Some(&[rp, rr, rd]) = resist.get(key) else {
            continue;
        };
        let [np, nf] = [p.passes[1], p.passes[0]];
        if np >= SPREAD_MIN[0] && nf >= SPREAD_MIN[1] {
            let cp = f(p.completed[1]) / f(np);
            let cf = f(p.completed[0]) / f(nf);
            if cp > 0.0 && cf > 0.0 {
                let r = cp / cf;
                let var = r * r * ((1.0 - cp) / (f(np) * cp) + (1.0 - cf) / (f(nf) * cf));
                ratio.push((r, var, rp));
            }
        }
        let [tp, tf] = [p.touches[1], p.touches[0]];
        if tp >= SPREAD_MIN[2] && tf >= SPREAD_MIN[3] {
            let a = f(p.lost[1]) / f(tp);
            let b = f(p.lost[0]) / f(tf);
            gap.push((a - b, a * (1.0 - a) / f(tp) + b * (1.0 - b) / f(tf), rr));
        }
        if p.run_tackles >= SPREAD_MIN[4] {
            let k = 1.0 - f(p.run_lost) / f(p.run_tackles);
            keep.push((k, k * (1.0 - k) / f(p.run_tackles), rd));
        }
    }
    let t = totals.into_inner().unwrap_or_else(|e| e.into_inner());
    let means = STATS
        .iter()
        .zip(t)
        .map(|(name, (n, d))| (*name, ratio_of(n, d)))
        .collect();
    Ok((
        BTreeMap::from([
            ("pressed_completion_ratio", spread_of(&ratio)),
            ("touch_loss_gap", spread_of(&gap)),
            ("run_tackle_keep_share", spread_of(&keep)),
        ]),
        means,
    ))
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
            hold,
            player,
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
            if let Some(slot) = player {
                let verdicts = check_player(content, primitive, *slot, &setup)?;
                let pass = verdicts.iter().all(|v| v.pass);
                for v in verdicts.iter().filter(|v| !v.pass) {
                    let mut failed = Vec::new();
                    if !v.own.pass {
                        failed.push(format!(
                            "slot {} {} moved {:+.4} (noise {:.4}, wanted up)",
                            v.slot, v.own.stat, v.own.difference, v.own.noise
                        ));
                    }
                    if !v.others.pass {
                        failed.push(format!(
                            "the other ten's {} moved {:+.4} (noise {:.4})",
                            v.others.stat, v.others.difference, v.others.noise
                        ));
                    }
                    eprintln!("{}: {} fails: {}", v.primitive, v.step, failed.join("; "));
                }
                emit(
                    &serde_json::json!({
                        "kind": "primitives-check-player",
                        "pass": pass,
                        "steps": verdicts
                    }),
                    out.as_deref(),
                )?;
                return Ok(if pass { 0 } else { 2 });
            }
            let verdicts = check(content, primitive, step.as_deref(), with, hold, &setup)?;
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
            between_neighbours,
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
            let result = fit(
                content,
                &table,
                primitive,
                level,
                &setup,
                *evals,
                *between_neighbours,
            )?;
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
        PrimitivesAction::Spread {
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
            let (spread, means) = pressure_spread(content, &setup)?;
            emit(
                &serde_json::json!({"kind": "pressure-spread", "matches": matches, "seed": seed, "spread": spread, "means": means}),
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

    /// Over a hump the plain bisection sees only its two ends; the bracketed search finds the
    /// target on the side that holds the preferred value, and names a target above the hump
    /// out of reach with the probe nearest it, which may lie inside the range.
    #[test]
    fn the_bracketed_search_handles_a_statistic_that_does_not_move_one_way() {
        let hump = |x: f64| Ok((1.0 - (x - 0.5).powi(2), 0.001));
        let plain = bisect(hump, 0.0, 1.0, 0.9, 12).unwrap();
        assert!(plain.unreachable, "{plain:?}");
        let s = bisect_bracketed(hump, 0.0, 1.0, 0.8, 0.9, 12).unwrap();
        assert!(!s.unreachable, "{s:?}");
        assert!((s.achieved - 0.9).abs() <= 0.001, "{s:?}");
        assert!(s.value > 0.5, "the half that holds 0.8: {s:?}");
        let s = bisect_bracketed(hump, 0.0, 1.0, 0.2, 0.9, 12).unwrap();
        assert!(s.value < 0.5, "the half that holds 0.2: {s:?}");
        let s = bisect_bracketed(hump, 0.0, 1.0, 0.2, 1.5, 12).unwrap();
        assert!(s.unreachable);
        assert_eq!(s.value, 0.5);
        assert_eq!(s.probes.len(), 3);
        let s = bisect_bracketed(hump, 0.0, 1.0, 0.2, 0.9001, 5).unwrap();
        assert!(s.probes.len() <= 5, "{s:?}");
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

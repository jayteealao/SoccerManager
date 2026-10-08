//! The balanced design that measures every job from the same matches. In each match every
//! factor of the measured side is at its low or its high level; each factor is high in
//! exactly half the matches, and a seeded permutation per factor keeps the factors
//! independent of each other. A job's move is its statistic over the high matches less its
//! statistic over the low matches: the job's main effect from low to high, averaged over the
//! other factors. Two arms, every factor low and every factor high, give the outcome move of
//! all jobs together, which each job's share of the outcome is measured against.
//!
//! The measured side is default club A, every one of its players set alike, against default
//! club B; it plays at home on even match indices and away on odd ones, so neither end of the
//! pitch is favoured.

use crate::contract::level::group_mean;
use crate::data::Content;
use crate::data::attributes::Group;
use crate::data::team::{Condition, TeamFile};
use crate::rating::Rating;
use crate::rng::EngineRng;
use crate::sensitivity::probe::SideCounts;
use crate::sensitivity::rules::{JobKind, Measure, RuleDef};
use crate::sim::{EngineEventKind, MatchConfig, Simulation};

/// What a factor sets on every player of the measured side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    /// The rating of the attribute at this schema index.
    Attribute(usize),
    Height,
    Age,
    /// The adaptation input, through which nationality acts.
    Adaptation,
}

/// One factor of the design: a job, what it sets, and its two levels.
#[derive(Debug, Clone, PartialEq)]
pub struct Factor {
    pub job: String,
    pub target: Target,
    pub levels: [f64; 2],
}

impl Factor {
    /// The factor of `rule` under the content's attribute schema.
    pub fn of(rule: &RuleDef, content: &Content) -> Self {
        let target = match rule.kind {
            JobKind::Attribute => Target::Attribute(
                content
                    .attributes
                    .index(&rule.job)
                    .expect("a checked rules file names schema attributes"),
            ),
            JobKind::Body => match rule.job.as_str() {
                "height" => Target::Height,
                "age" => Target::Age,
                _ => Target::Adaptation,
            },
        };
        Self {
            job: rule.job.clone(),
            target,
            levels: rule.levels,
        }
    }

    /// The level between the two, rounded as the field is stored: held there by a design
    /// that does not vary this factor.
    pub fn middle(&self) -> f64 {
        let m = (self.levels[0] + self.levels[1]) / 2.0;
        match self.target {
            Target::Attribute(_) => (m * 10.0).round() / 10.0,
            _ => m.round(),
        }
    }
}

/// Sets `value` of `target` on every player of `file`.
pub fn set(file: &mut TeamFile, content: &Content, target: Target, value: f64) {
    for p in &mut file.players {
        match target {
            Target::Attribute(i) => {
                let name = &content.attributes.attributes[i].name;
                let tenths = (value * 10.0).round().clamp(10.0, 200.0) as u8;
                p.attributes
                    .insert(name.clone(), Rating::from_tenths(tenths));
            }
            Target::Height => p.height = Some(value.round() as u8),
            Target::Age => p.age = Some(value.round() as u8),
            Target::Adaptation => {
                let c = p.condition.get_or_insert_with(Condition::default);
                c.adaptation = Some(value.round() as u8);
            }
        }
    }
}

/// The levels of one match: each factor's value, and whether it is at its high level.
#[derive(Debug, Clone, PartialEq)]
pub struct Levels {
    pub values: Vec<f64>,
    pub high: Vec<bool>,
}

/// For `matches` matches (an even number), whether each of `factors` is high: exactly half
/// the matches per factor, by a seeded permutation of its own.
pub fn balanced(factors: usize, matches: usize, seed: u64) -> Vec<Vec<bool>> {
    let columns: Vec<Vec<bool>> = (0..factors)
        .map(|f| {
            let mut rng =
                EngineRng::from_seed(seed ^ (0x9E37_79B9_7F4A_7C15u64.wrapping_mul(f as u64 + 1)));
            let mut order: Vec<usize> = (0..matches).collect();
            for i in (1..matches).rev() {
                let j = rng.range_usize(i + 1);
                order.swap(i, j);
            }
            let mut column = vec![false; matches];
            for &m in &order[..matches / 2] {
                column[m] = true;
            }
            column
        })
        .collect();
    (0..matches)
        .map(|m| columns.iter().map(|c| c[m]).collect())
        .collect()
}

/// One match of the measured side: the statistic parts of every measure, the goal
/// difference, and each player's match rating.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchRow {
    /// Per factor of the run, whether it was high in this match.
    pub high: Vec<bool>,
    /// Per measure, in [`Measure::ALL`] order: the numerator and the denominator its
    /// statistic sums over matches.
    pub parts: Vec<[f64; 2]>,
    /// The measured side's goals less the other side's.
    pub gd: f64,
    /// The measured side's expected goals less the other side's.
    pub xgd: f64,
    /// The match rating of each measured player who played at least an hour, by squad index.
    pub ratings: Vec<(usize, f64)>,
}

/// What the run reads off the match tick by tick for the measured side.
#[derive(Debug, Default)]
struct Watch {
    top_speed: [f64; 11],
    lapse_until: [u32; 22],
    lapses: f64,
    anchor_m: f64,
    anchor_n: f64,
    half_time_gain: f64,
    half_time_n: f64,
    /// Energy at the first tick of minute 60, by roster slot, with the squad index then.
    at_sixty: Option<[(usize, f64); 11]>,
    mental: f64,
    mental_n: f64,
}

/// The ticks between two samples of the distance from the anchor.
const ANCHOR_EVERY: u32 = 10;

/// Plays one match of `files` (home first) with the measured side `side`, and reads it.
pub fn play(
    content: &Content,
    files: [&TeamFile; 2],
    side: usize,
    seed: u64,
    high: Vec<bool>,
) -> MatchRow {
    let config =
        MatchConfig::new(seed, 90, content, files).expect("the design's teams build a match");
    let mut sim = Simulation::new(config).expect("the design's match starts");
    sim.with_probe();
    let first = side * 11;
    let roster = first..first + 11;
    let mut w = Watch::default();
    for i in roster.clone() {
        let r = sim.effective_ratings(i);
        w.mental += group_mean(&r, &sim.config.attributes, Group::Mental);
        w.mental_n += 1.0;
    }
    let mut energy = [0.0; 11];
    while !sim.is_over() {
        for (e, p) in energy.iter_mut().zip(&sim.players[roster.clone()]) {
            *e = p.energy;
        }
        sim.step();
        let tick = sim.tick;
        for (k, i) in roster.clone().enumerate() {
            let p = &sim.players[i];
            if !p.active() {
                continue;
            }
            let v = p.vel.length();
            if v > w.top_speed[k] {
                w.top_speed[k] = v;
            }
            if p.lapse_until > w.lapse_until[i] {
                w.lapse_until[i] = p.lapse_until;
                w.lapses += 1.0;
            }
        }
        let ball = sim.ball.xy();
        let reach = sim.config.tuning.reach_radius;
        if let Some(probe) = sim.probe.as_deref_mut()
            && let Some(f) = probe.flight
            && f.arrived.is_none()
            && (sim.players[f.mate].pos - ball).length() <= reach
        {
            probe.arrived(tick);
        }
        let against = sim.carrier.is_some_and(|c| sim.players[c].team != side);
        if against && tick.is_multiple_of(ANCHOR_EVERY) {
            let keeper = sim.keeper(side);
            let t = std::sync::Arc::clone(&sim.tuning);
            for i in roster.clone() {
                let p = &sim.players[i];
                if i == keeper || !p.active() {
                    continue;
                }
                let anchor = sim.teams[side].anchor(p.slot, ball, false, &t);
                w.anchor_m += (p.pos - anchor).length();
                w.anchor_n += 1.0;
            }
        }
        for e in sim.take_events() {
            if e.kind == EngineEventKind::HalfTime {
                for (k, i) in roster.clone().enumerate() {
                    let p = &sim.players[i];
                    if p.active() {
                        w.half_time_gain += p.energy - energy[k];
                        w.half_time_n += 1.0;
                    }
                }
            }
        }
        if w.at_sixty.is_none() && sim.half() >= 1 && sim.minute().0 >= 60 {
            let mut at = [(0, 0.0); 11];
            for (slot, i) in at.iter_mut().zip(roster.clone()) {
                let p = &sim.players[i];
                *slot = (if p.active() { p.squad } else { usize::MAX }, p.energy);
            }
            w.at_sixty = Some(at);
        }
    }
    sim.finish();
    read(&sim, side, &w, high)
}

/// The row of a finished match.
fn read(sim: &Simulation, side: usize, w: &Watch, high: Vec<bool>) -> MatchRow {
    let other = 1 - side;
    let probe = sim.probe.as_deref().expect("the design attached the probe");
    let (s, o): (SideCounts, SideCounts) = (probe.sides[side], probe.sides[other]);
    let summary = sim.summary;
    let first = side * 11;
    let mut ft = (0.0, 0.0);
    let mut late = (0.0, 0.0);
    for (k, i) in (first..first + 11).enumerate() {
        let p = &sim.players[i];
        if !p.active() {
            continue;
        }
        ft.0 += p.energy;
        ft.1 += 1.0;
        if let Some(at) = w.at_sixty
            && at[k].0 == p.squad
        {
            late.0 += at[k].1 - p.energy;
            late.1 += 1.0;
        }
    }
    let blocks: u32 = sim.tallies[side].iter().map(|t| t.blocks).sum();
    let top: f64 = w.top_speed.iter().sum::<f64>() / 11.0;
    let parts = Measure::ALL
        .iter()
        .map(|m| match m {
            Measure::PassCompletion => [s.passes_completed, s.passes],
            Measure::TakeOnCompletion => [s.running_tackles_beaten, s.running_tackles_faced],
            Measure::ReceiptLoss => [s.arrivals - s.arrivals_kept, s.arrivals],
            Measure::BoxFinishing => [s.goals_on_target_in_box, s.on_target_in_box],
            Measure::CrossCompletion => [s.crosses_completed, s.crosses],
            Measure::HeadersWon => [s.headers_won, s.headers],
            Measure::LongShotsOnTarget => [s.on_target_outside, s.shots_outside],
            Measure::TacklesWon => [s.tackles_won, s.tackles],
            Measure::SkillAttempts => [s.chips + s.take_ons, 1.0],
            Measure::Blocks => [f64::from(blocks), 1.0],
            Measure::PassProgress => [s.progress_m, s.progress_passes],
            Measure::XgPerShot => [s.xg, s.shots],
            Measure::PressedCompletion => [s.pressed_completed, s.pressed_passes],
            Measure::LooseBallShare => [s.loose_won, s.loose_won + o.loose_won],
            Measure::TackleAttempts => [s.tackles, 1.0],
            Measure::Fouls => [f64::from(summary.fouls[side]), 1.0],
            Measure::Lapses => [w.lapses, 1.0],
            Measure::AnchorDistance => [w.anchor_m, w.anchor_n],
            Measure::TopSpeed => [top, 1.0],
            Measure::CloseLooseBalls => [s.loose_won_close, 1.0],
            Measure::FullTimeEnergy => [ft.0, ft.1],
            Measure::StandingTacklesLost => [s.standing_tackles_lost, 1.0],
            Measure::FouledKept => [s.fouled_kept, s.fouled],
            Measure::AerialBallsReached => [s.headers, 1.0],
            Measure::HalfTimeGain => [w.half_time_gain, w.half_time_n],
            Measure::Injuries => [f64::from(summary.injuries[side]), 1.0],
            Measure::SavesHeld => [s.saves_held, s.saves],
            Measure::FarSaves => [s.saved_far, s.faced_far],
            Measure::HighClaims => [s.high_claims, 1.0],
            Measure::NearSaves => [s.saved_near, s.faced_near],
            Measure::KeeperKicks => [s.keeper_kicks_completed, s.keeper_kicks],
            Measure::KeeperThrows => [s.throws_completed, s.throws],
            Measure::CrossesClaimed => [s.crosses_claimed, 1.0],
            Measure::OffsidesWon => [f64::from(summary.offsides[other]), 1.0],
            Measure::Sweeps => [s.sweeps, 1.0],
            // Read from the ratings, across matches.
            Measure::RatingSpread => [0.0, 0.0],
            Measure::LateEnergyLoss => [late.0, late.1],
            Measure::MentalRating => [w.mental, w.mental_n],
        })
        .collect();
    let hour = 60 * crate::rules::clock::TICKS_PER_MINUTE;
    let ratings = sim
        .match_ratings()
        .into_iter()
        .filter(|r| r.team == side && sim.tallies[side][r.squad].ticks_played >= hour)
        .map(|r| (r.squad, r.rating()))
        .collect();
    MatchRow {
        high,
        parts,
        gd: f64::from(summary.goals[side]) - f64::from(summary.goals[other]),
        xgd: summary.xg[side] - summary.xg[other],
        ratings,
    }
}

/// The two default clubs: the measured one (club A) and its opponent (club B).
#[derive(Debug, Clone)]
pub struct Clubs {
    pub measured: TeamFile,
    pub opponent: TeamFile,
}

/// The measured side's file for one match: every factor at its value.
fn measured_file(clubs: &Clubs, content: &Content, factors: &[Factor], values: &[f64]) -> TeamFile {
    let mut file = clubs.measured.clone();
    for (f, &v) in factors.iter().zip(values) {
        set(&mut file, content, f.target, v);
    }
    file
}

/// Plays `job(k)` for every `k` in `0..n` on all cores and returns the results in order.
pub fn parallel<T: Send>(n: usize, job: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let threads = std::thread::available_parallelism().map_or(4, |t| t.get());
    let chunk = n.div_ceil(threads).max(1);
    let job = &job;
    let ks: Vec<usize> = (0..n).collect();
    std::thread::scope(|scope| {
        let handles: Vec<_> = ks
            .chunks(chunk)
            .map(|part| scope.spawn(move || part.iter().map(|&k| job(k)).collect::<Vec<T>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("a sensitivity match thread panicked"))
            .collect()
    })
}

/// Plays the balanced design over `varied` (indices into `all`), holding every other factor of
/// `all` at its middle level: `matches` matches from `seed`. Each row's `high` lists the
/// varied factors in order.
pub fn play_design(
    content: &Content,
    clubs: &Clubs,
    all: &[Factor],
    varied: &[usize],
    matches: usize,
    seed: u64,
) -> Vec<MatchRow> {
    let plan = balanced(varied.len(), matches, seed);
    parallel(matches, |m| {
        let mut values: Vec<f64> = all.iter().map(Factor::middle).collect();
        for (k, &f) in varied.iter().enumerate() {
            values[f] = all[f].levels[usize::from(plan[m][k])];
        }
        let file = measured_file(clubs, content, all, &values);
        play_side(content, clubs, &file, m, seed, plan[m].clone())
    })
}

/// Plays one arm: every factor of `all` at its low (`high` false) or its high level,
/// `matches` matches from `seed`.
pub fn play_arm(
    content: &Content,
    clubs: &Clubs,
    all: &[Factor],
    high: bool,
    matches: usize,
    seed: u64,
) -> Vec<MatchRow> {
    let values: Vec<f64> = all.iter().map(|f| f.levels[usize::from(high)]).collect();
    let file = measured_file(clubs, content, all, &values);
    parallel(matches, |m| {
        play_side(content, clubs, &file, m, seed, Vec::new())
    })
}

/// Match `m` of a run from `seed`: the measured side at home on even `m`.
fn play_side(
    content: &Content,
    clubs: &Clubs,
    file: &TeamFile,
    m: usize,
    seed: u64,
    high: Vec<bool>,
) -> MatchRow {
    let seed = seed.wrapping_mul(1_000_003).wrapping_add(m as u64);
    if m.is_multiple_of(2) {
        play(content, [file, &clubs.opponent], 0, seed, high)
    } else {
        play(content, [&clubs.opponent, file], 1, seed, high)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_factor_is_high_in_half_the_matches_and_the_factors_are_unrelated() {
        let (factors, matches) = (40, 2000);
        let plan = balanced(factors, matches, 42);
        for f in 0..factors {
            let n = plan.iter().filter(|m| m[f]).count();
            assert_eq!(n, matches / 2, "factor {f}");
        }
        let x = |m: &Vec<bool>, f: usize| if m[f] { 1.0 } else { -1.0 };
        let mut worst: f64 = 0.0;
        for a in 0..factors {
            for b in a + 1..factors {
                let r: f64 = plan.iter().map(|m| x(m, a) * x(m, b)).sum::<f64>() / matches as f64;
                worst = worst.max(r.abs());
            }
        }
        assert!(worst < 0.1, "largest correlation {worst}");
        assert_eq!(plan, balanced(factors, matches, 42), "the plan is seeded");
    }
}

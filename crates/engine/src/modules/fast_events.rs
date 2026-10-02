//! The fast model's events: every kind the full engine emits in a regulation match, played
//! after the score from fitted rates on the same seeded generator. Each event names the team
//! and the player the full engine would name; there are no positions and no spots.
//!
//! Per side, each count is drawn at fitted minutes: fouls, offsides, corners, throw-ins and
//! goal kicks are negative binomial (variance `mean + mean^2 / dispersion`) and injuries
//! Poisson, each with the mean `exp(base + home * [home side] + strength * d)`, where `d` is
//! the side's strength less the other side's, over 10. Each foul is then played on with
//! advantage, or gives a free kick or a penalty, and may be booked or sent off, by fitted
//! shares. An offside gives the other side a free kick. Substitutions not forced by an
//! injury come from a fitted share table by strength level, grouped into the rule pack's
//! windows; an injury forces a substitution while one is left. Added time is the rule
//! pack's price of the half's stoppages and cards plus its variance, as the full engine
//! adds it. A goal is followed by the conceding side's kick-off.
//!
//! The minute shares have [`BINS`] bins: minutes 0 to 44, the first half's added time,
//! minutes 45 to 89, and the second half's added time.

use serde::{Deserialize, Serialize};

use super::RulesModule;
use super::fast_model::{FastPlayer, KickOff, Side};
use crate::ai::AiCode;
use crate::data::RulePack;
use crate::data::rules::{AddedTime, StoppageKind, Substitutions};
use crate::error::EngineError;
use crate::fatigue::InjurySource;
use crate::rng::EngineRng;
use crate::rules::clock::{TICKS_PER_MINUTE, Tally, added_seconds};
use crate::rules::fouls::Card;
use crate::rules::pack::RulePackOff;
use crate::sim::{EngineEvent, EngineEventKind, EventDetail};
use crate::tactics::change::{ChangeId, ChangeKind};
use crate::team::PLAYERS_PER_TEAM;
use crate::{TICKS_PER_SECOND, ticks_for_minutes};

/// Minute bins of the event shares: minutes 0 to 44, the first half's added time, minutes 45
/// to 89, the second half's added time.
pub const BINS: usize = 92;
/// The bin of the first half's added time.
pub const FIRST_ADDED: usize = 45;
/// The bin of the second half's added time.
pub const SECOND_ADDED: usize = 91;
/// A side whose strength is within this many tenths of the other side's is level for the
/// substitution table.
pub const LEVEL_BAND: f64 = 0.2;
/// The most minutes a substitution's decision comes before the stoppage it applies at.
const DECISION_LEAD_MINUTES: f64 = 3.0;
/// Substitutions drawn within this many minutes of each other share one stoppage.
const SAME_STOPPAGE_MINUTES: u32 = 3;
/// The largest count a side can draw of one kind.
const MAX_COUNT: u32 = 200;

/// One count per side: its log-linear mean, its dispersion, and its minute shares.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountFit {
    /// Log mean of an away side level with the other side.
    pub base: f64,
    /// Added to the home side's log mean.
    pub home: f64,
    /// Log mean per unit of `d`, the side's strength less the other side's, over 10.
    pub strength: f64,
    /// The negative binomial's shape; `None` for a Poisson count.
    pub dispersion: Option<f64>,
    /// One share per bin; they sum to 1.
    pub minute_shares: Vec<f64>,
}

impl CountFit {
    /// The mean count of `side` at kick-off `kick_off`.
    pub fn mean(&self, kick_off: &KickOff, side: usize) -> f64 {
        let x = covariates(kick_off, side);
        libm::exp(self.base * x[0] + self.home * x[1] + self.strength * x[2])
    }

    fn check(&self, name: &str) -> Result<(), EngineError> {
        let finite = [self.base, self.home, self.strength]
            .iter()
            .all(|v| v.is_finite());
        let dispersion_ok = self.dispersion.is_none_or(|k| k.is_finite() && k > 0.0);
        if !finite || !dispersion_ok {
            return Err(EngineError::InvalidConfig(format!(
                "the fast-model {name} count must be finite, with a positive dispersion"
            )));
        }
        check_shares(&self.minute_shares, BINS, &format!("{name} minute shares"))
    }
}

/// The intercept, the home flag and `d` of `side`: its strength less the other side's, over
/// 10.
pub fn covariates(kick_off: &KickOff, side: usize) -> [f64; 3] {
    let d = (kick_off.strength[side] - kick_off.strength[1 - side]) / 10.0;
    [1.0, if side == 0 { 1.0 } else { 0.0 }, d]
}

/// The strength level of `side` for the substitution table: 0 weaker, 1 level, 2 stronger.
pub fn level(kick_off: &KickOff, side: usize) -> usize {
    let d = covariates(kick_off, side)[2];
    if d < -LEVEL_BAND {
        0
    } else if d > LEVEL_BAND {
        2
    } else {
        1
    }
}

/// The rule pack's parts the fast model keeps to: the substitution limits and the price of
/// added time, as the fit's full-engine matches played them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FitRules {
    pub substitutions: Substitutions,
    pub added_time: AddedTime,
}

impl FitRules {
    /// The parts of `rules` the fast model keeps to.
    pub fn of(rules: &RulePack) -> Self {
        Self {
            substitutions: rules.substitutions.clone(),
            added_time: rules.added_time.clone(),
        }
    }

    /// The parts of the standard Laws built into the program.
    pub fn standard() -> Self {
        let laws = RulePackOff.load(&[]).expect("the built-in laws load").value;
        Self::of(&laws)
    }
}

/// What the fast model plays its events from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventFit {
    pub fouls: CountFit,
    pub offsides: CountFit,
    pub corners: CountFit,
    pub throw_ins: CountFit,
    pub goal_kicks: CountFit,
    pub injuries: CountFit,
    /// The share of fouls played on with advantage.
    pub advantage: f64,
    /// The share of fouls not played on that give a penalty.
    pub penalty: f64,
    /// The share of fouls booked: a yellow card, or a second yellow to a booked player.
    pub yellow: f64,
    /// The share of fouls sent off straight.
    pub red: f64,
    /// Substitutions not forced by an injury: the share of each count from 0 to the rule
    /// pack's limit, for a weaker, a level and a stronger side.
    pub substitutions: [Vec<f64>; 3],
    /// The minute shares of the substitutions not forced by an injury.
    pub substitution_minute_shares: Vec<f64>,
    pub rules: FitRules,
}

impl EventFit {
    /// Refuses a fit the model cannot play.
    pub fn check(&self) -> Result<(), EngineError> {
        for (name, count) in self.counts() {
            count.check(name)?;
        }
        let share = |v: f64| v.is_finite() && (0.0..=1.0).contains(&v);
        if ![self.advantage, self.penalty, self.yellow, self.red]
            .into_iter()
            .all(share)
            || self.yellow + self.red > 1.0
        {
            return Err(EngineError::InvalidConfig(
                "the fast-model foul shares must lie in 0 to 1, booked and sent off together \
                 at most 1"
                    .into(),
            ));
        }
        let limit = usize::from(self.rules.substitutions.limit);
        for table in &self.substitutions {
            check_shares(table, limit + 1, "substitution table")?;
        }
        check_shares(
            &self.substitution_minute_shares,
            BINS,
            "substitution minute shares",
        )
    }

    /// A fit with round rates near the full engine's in a level match, every minute equally
    /// likely, and each side making as many substitutions as `rules` allows. Tests play it.
    pub fn plain(rules: FitRules) -> Self {
        let count = |mean: f64, dispersion: Option<f64>| CountFit {
            base: libm::log(mean),
            home: 0.0,
            strength: 0.0,
            dispersion,
            minute_shares: vec![1.0 / BINS as f64; BINS],
        };
        let limit = usize::from(rules.substitutions.limit);
        let mut table = vec![0.0; limit + 1];
        table[limit] = 1.0;
        Self {
            fouls: count(7.0, Some(20.0)),
            offsides: count(1.5, Some(20.0)),
            corners: count(1.6, Some(20.0)),
            throw_ins: count(11.4, Some(20.0)),
            goal_kicks: count(6.8, Some(20.0)),
            injuries: count(0.15, None),
            advantage: 0.35,
            penalty: 0.02,
            yellow: 0.13,
            red: 0.015,
            substitutions: [table.clone(), table.clone(), table],
            substitution_minute_shares: vec![1.0 / BINS as f64; BINS],
            rules,
        }
    }

    /// Every count with its name, in the order the model draws them.
    pub fn counts(&self) -> [(&'static str, &CountFit); 6] {
        [
            ("fouls", &self.fouls),
            ("offsides", &self.offsides),
            ("corners", &self.corners),
            ("throw_ins", &self.throw_ins),
            ("goal_kicks", &self.goal_kicks),
            ("injuries", &self.injuries),
        ]
    }
}

fn check_shares(shares: &[f64], len: usize, what: &str) -> Result<(), EngineError> {
    let sum: f64 = shares.iter().sum();
    if shares.len() != len
        || shares.iter().any(|s| !s.is_finite() || *s < 0.0)
        || (sum - 1.0).abs() > 1e-6
    {
        return Err(EngineError::InvalidConfig(format!(
            "the fast-model {what} must be {len} values summing to 1"
        )));
    }
    Ok(())
}

/// The index whose share holds `u` on the cumulative scale.
pub fn pick_share(shares: &[f64], u: f64) -> usize {
    let mut acc = 0.0;
    for (i, s) in shares.iter().enumerate() {
        acc += s;
        if u < acc {
            return i;
        }
    }
    shares.len().saturating_sub(1)
}

/// A count with mean `mean` drawn by inversion from one uniform `u`: negative binomial with
/// shape `k`, or Poisson when `k` is `None`.
pub fn draw_count(mean: f64, k: Option<f64>, u: f64) -> u32 {
    if mean <= 0.0 {
        return 0;
    }
    let mut p = match k {
        Some(k) => libm::pow(k / (k + mean), k),
        None => libm::exp(-mean),
    };
    let mut acc = p;
    let mut y = 0;
    while u >= acc && y < MAX_COUNT {
        y += 1;
        let n = f64::from(y);
        p *= match k {
            Some(k) => (n - 1.0 + k) / n * (mean / (k + mean)),
            None => mean / n,
        };
        acc += p;
    }
    y
}

/// What a drawn happening is, before its players are picked.
#[derive(Debug, Clone, Copy, PartialEq)]
enum What {
    /// A foul: played on, or the restart it gives; and the card drawn with it.
    Foul {
        advantage: bool,
        penalty: bool,
        card: Option<Card>,
    },
    Offside,
    Corner,
    ThrowIn,
    GoalKick,
    Injury,
}

/// A happening of `team` in minute bin `bin`, `at` its place inside the bin from 0 to 1.
#[derive(Debug, Clone, Copy)]
struct Drawn {
    team: usize,
    bin: usize,
    at: f64,
    what: What,
}

/// What happens at a tick of the walk.
#[derive(Debug, Clone, Copy)]
enum Item {
    Goal(usize),
    Drawn(Drawn),
    /// A substitution not forced by an injury is decided: side, its index among the side's.
    Decide(usize, usize),
    /// The substitutions of a side's stoppage apply: side, the first and last index.
    Apply(usize, usize, usize),
}

/// A side during the walk.
struct OnPitch {
    side: Side,
    /// The player in each roster slot, `None` once the slot's player left play for good.
    slots: [Option<FastPlayer>; PLAYERS_PER_TEAM],
    /// `true` while the slot holds the player who started there.
    starter: [bool; PLAYERS_PER_TEAM],
    booked: [bool; PLAYERS_PER_TEAM],
    /// The bench players who have not come on, in bench order.
    bench: Vec<FastPlayer>,
    used: u32,
    /// The ticks of the stoppages the side substituted at.
    windows: Vec<u32>,
    /// Each decided substitution's change id.
    decided: Vec<Option<ChangeId>>,
}

impl OnPitch {
    fn new(side: &Side, decisions: usize) -> Self {
        Self {
            side: side.clone(),
            slots: side.starters.map(Some),
            starter: [true; PLAYERS_PER_TEAM],
            booked: [false; PLAYERS_PER_TEAM],
            bench: side.bench.clone(),
            used: 0,
            windows: Vec::new(),
            decided: vec![None; decisions],
        }
    }

    /// The occupied slots that `keep` accepts.
    fn slots_where(&self, keep: impl Fn(usize, &FastPlayer) -> bool) -> Vec<usize> {
        (0..PLAYERS_PER_TEAM)
            .filter(|&s| self.slots[s].as_ref().is_some_and(|p| keep(s, p)))
            .collect()
    }

    fn outfield(&self) -> Vec<usize> {
        self.slots_where(|_, p| !p.keeper)
    }

    /// A slot drawn from `slots` with weights `weight`, or uniformly when every weight is 0.
    fn weighted(&self, slots: &[usize], weight: impl Fn(&FastPlayer) -> f64, u: f64) -> usize {
        let weights: Vec<f64> = slots
            .iter()
            .map(|&s| self.slots[s].as_ref().map_or(0.0, &weight).max(0.0))
            .collect();
        let total: f64 = weights.iter().sum();
        if total <= 0.0 {
            return slots[((u * slots.len() as f64) as usize).min(slots.len() - 1)];
        }
        let shares: Vec<f64> = weights.iter().map(|w| w / total).collect();
        slots[pick_share(&shares, u)]
    }

    fn uniform(slots: &[usize], u: f64) -> Option<usize> {
        (!slots.is_empty()).then(|| slots[((u * slots.len() as f64) as usize).min(slots.len() - 1)])
    }

    /// The kick-off taker: the highest advanced slot still occupied, else any player.
    fn kick_off_taker(&self) -> Option<usize> {
        let advanced = self.slots_where(|s, p| self.side.advanced[s] && !p.keeper);
        advanced
            .last()
            .copied()
            .or_else(|| self.slots_where(|_, _| true).last().copied())
    }

    /// The keeper's slot, or an outfield slot when no keeper is on the pitch.
    fn keeper(&self) -> Option<usize> {
        self.slots_where(|_, p| p.keeper)
            .first()
            .copied()
            .or_else(|| self.outfield().first().copied())
    }

    /// `true` when a substitution at `tick` keeps the rule pack's limit and windows.
    fn may_substitute(&self, tick: u32, rules: &Substitutions) -> bool {
        self.used < u32::from(rules.limit)
            && (self.windows.contains(&tick) || self.windows.len() < usize::from(rules.windows))
    }
}

/// The match clock of a fast match: the regulation halves and the added time of each.
struct Clock {
    half: u32,
    /// The ticks each half adds.
    added: [u32; 2],
}

impl Clock {
    /// The tick the first half ends and the second starts on.
    fn half_time(&self) -> u32 {
        self.half + self.added[0]
    }

    fn full_time(&self) -> u32 {
        self.half_time() + self.half + self.added[1]
    }

    /// The tick of a place `at` (0 to 1) inside minute bin `bin`. A tick in added time stays
    /// before the period's end.
    fn tick(&self, bin: usize, at: f64) -> u32 {
        let within = |len: u32| ((at * f64::from(len)) as u32).min(len.saturating_sub(1));
        match bin {
            b if b < FIRST_ADDED => b as u32 * TICKS_PER_MINUTE + within(TICKS_PER_MINUTE),
            FIRST_ADDED => self.half + within(self.added[0]),
            b if b < SECOND_ADDED => {
                self.half_time()
                    + (b - FIRST_ADDED - 1) as u32 * TICKS_PER_MINUTE
                    + within(TICKS_PER_MINUTE)
            }
            _ => self.half_time() + self.half + within(self.added[1]),
        }
    }

    /// The minute an event at `tick` shows, and the added minute in added time.
    fn minute(&self, tick: u32) -> (u32, Option<u32>) {
        let (start, before) = if tick < self.half_time() {
            (0, 0)
        } else {
            (self.half_time(), self.half)
        };
        let elapsed = tick - start;
        if elapsed < self.half {
            ((before + elapsed) / TICKS_PER_MINUTE, None)
        } else {
            (
                (before + self.half) / TICKS_PER_MINUTE,
                Some((elapsed - self.half) / TICKS_PER_MINUTE + 1),
            )
        }
    }
}

/// Plays the events of a regulation match whose goals are `goals` (minute, team) in minute
/// order, drawing from `rng` after the score.
pub fn play(
    fit: &EventFit,
    kick_off: &KickOff,
    goals: &[(u32, usize)],
    rng: &mut EngineRng,
) -> Vec<EngineEvent> {
    // The counts, each happening's bin and place, and each foul's outcome.
    let mut drawn: Vec<Drawn> = Vec::new();
    for team in 0..2 {
        for (name, count) in fit.counts() {
            let n = draw_count(count.mean(kick_off, team), count.dispersion, rng.next_f64());
            for _ in 0..n {
                let bin = pick_share(&count.minute_shares, rng.next_f64());
                let at = rng.next_f64();
                let what = match name {
                    "fouls" => {
                        let advantage = rng.next_f64() < fit.advantage;
                        let penalty = !advantage && rng.next_f64() < fit.penalty;
                        let u = rng.next_f64();
                        let card = if u < fit.yellow {
                            Some(Card::Yellow)
                        } else if u < fit.yellow + fit.red {
                            Some(Card::Red)
                        } else {
                            None
                        };
                        What::Foul {
                            advantage,
                            penalty,
                            card,
                        }
                    }
                    "offsides" => What::Offside,
                    "corners" => What::Corner,
                    "throw_ins" => What::ThrowIn,
                    "goal_kicks" => What::GoalKick,
                    _ => What::Injury,
                };
                drawn.push(Drawn {
                    team,
                    bin,
                    at,
                    what,
                });
            }
        }
    }
    // The substitutions not forced by an injury: a count per side, a bin and place each.
    let mut subs: [Vec<(usize, f64)>; 2] = [Vec::new(), Vec::new()];
    for (team, list) in subs.iter_mut().enumerate() {
        let n = pick_share(&fit.substitutions[level(kick_off, team)], rng.next_f64());
        for _ in 0..n {
            let bin = pick_share(&fit.substitution_minute_shares, rng.next_f64());
            list.push((bin, rng.next_f64()));
        }
    }
    // Added time: the rule pack's price of each half's stoppages and cards before its own
    // time ends, plus the variance.
    let mut tallies = [Tally::default(); 2];
    for &(minute, _) in goals {
        tallies[usize::from(minute >= 45)].add(StoppageKind::Goal);
    }
    for d in &drawn {
        if d.bin == FIRST_ADDED || d.bin == SECOND_ADDED {
            continue;
        }
        let tally = &mut tallies[usize::from(d.bin > FIRST_ADDED)];
        match d.what {
            What::Foul {
                advantage,
                penalty,
                card,
            } => {
                if !advantage {
                    tally.add(if penalty {
                        StoppageKind::Penalty
                    } else {
                        StoppageKind::FreeKick
                    });
                }
                if card.is_some() {
                    tally.cards += 1;
                }
            }
            What::Offside => tally.add(StoppageKind::FreeKick),
            What::Corner => tally.add(StoppageKind::Corner),
            What::ThrowIn => tally.add(StoppageKind::ThrowIn),
            What::GoalKick => tally.add(StoppageKind::GoalKick),
            What::Injury => tally.add(StoppageKind::Injury),
        }
    }
    let added = tallies.map(|t| added_seconds(&t, &fit.rules.added_time, rng.next_f64()));
    let clock = Clock {
        half: ticks_for_minutes(45),
        added: added.map(|s| s * TICKS_PER_SECOND),
    };

    // Every happening on the clock, in tick order; at one tick, the order drawn.
    let mut items: Vec<(u32, Item)> = Vec::new();
    for &(minute, team) in goals {
        let bin = if minute < 45 {
            minute as usize
        } else {
            minute as usize + 1
        };
        items.push((clock.tick(bin, 0.5), Item::Goal(team)));
    }
    for d in &drawn {
        items.push((clock.tick(d.bin, d.at), Item::Drawn(*d)));
    }
    for (team, list) in subs.iter().enumerate() {
        let mut ticks: Vec<u32> = list.iter().map(|&(b, at)| clock.tick(b, at)).collect();
        ticks.sort_unstable();
        for (i, group) in stoppages(&ticks, usize::from(fit.rules.substitutions.windows)) {
            let lead =
                (rng.next_f64() * DECISION_LEAD_MINUTES * f64::from(TICKS_PER_MINUTE)) as u32;
            let start = if group < clock.half_time() {
                0
            } else {
                clock.half_time()
            };
            for k in i.clone() {
                items.push((group.saturating_sub(lead).max(start), Item::Decide(team, k)));
            }
            items.push((group, Item::Apply(team, i.start, i.end)));
        }
    }
    items.sort_by_key(|(tick, _)| *tick);

    let decisions = [subs[0].len(), subs[1].len()];
    let mut walk = Walk {
        fit,
        clock: &clock,
        sides: [0, 1].map(|t| OnPitch::new(&kick_off.sides[t], decisions[t])),
        scores: [0, 0],
        out: Vec::new(),
        changes: 0,
    };
    walk.kick_off(0, 0);
    let mut half_time_done = false;
    for (tick, item) in items {
        if tick >= clock.half_time() && !half_time_done {
            walk.half_time(added[0]);
            half_time_done = true;
        }
        walk.item(tick, item, rng);
    }
    if !half_time_done {
        walk.half_time(added[0]);
    }
    let mut full_time = walk.event(EngineEventKind::FullTime, clock.full_time(), None);
    full_time.added_time_s = Some(added[1]);
    walk.out.push(full_time);
    walk.out
}

/// Groups the sorted ticks of a side's substitutions into at most `windows` stoppages:
/// ticks within [`SAME_STOPPAGE_MINUTES`] of the group's first share it, then the two
/// closest groups merge until `windows` remain. Each group applies at its last tick.
fn stoppages(ticks: &[u32], windows: usize) -> Vec<(std::ops::Range<usize>, u32)> {
    let mut groups: Vec<std::ops::Range<usize>> = Vec::new();
    for i in 0..ticks.len() {
        match groups.last_mut() {
            Some(g) if ticks[i] - ticks[g.start] <= SAME_STOPPAGE_MINUTES * TICKS_PER_MINUTE => {
                g.end = i + 1;
            }
            _ => groups.push(i..i + 1),
        }
    }
    while groups.len() > windows.max(1) {
        let gap = |k: usize| ticks[groups[k + 1].start] - ticks[groups[k].end - 1];
        let k = (0..groups.len() - 1)
            .min_by_key(|&k| gap(k))
            .expect("two groups or more");
        let next = groups.remove(k + 1);
        groups[k].end = next.end;
    }
    groups
        .into_iter()
        .map(|g| {
            let at = ticks[g.end - 1];
            (g, at)
        })
        .collect()
}

/// The walk over the clock: the players on the pitch, the score and the events so far.
struct Walk<'a> {
    fit: &'a EventFit,
    clock: &'a Clock,
    sides: [OnPitch; 2],
    scores: [u32; 2],
    out: Vec<EngineEvent>,
    /// The change queue's running count, over both sides.
    changes: u32,
}

impl Walk<'_> {
    fn event(&self, kind: EngineEventKind, tick: u32, team: Option<usize>) -> EngineEvent {
        let (minute, minute_added) = self.clock.minute(tick);
        EngineEvent {
            tick,
            kind,
            team,
            scores: self.scores,
            minute,
            minute_added,
            player: None,
            secondary: None,
            card: None,
            advantage: None,
            added_time_s: None,
            spot: None,
            detail: None,
            period: None,
            shootout_round: None,
            shootout_scored: None,
            shootout_scores: None,
            decided_by: None,
        }
    }

    fn push(&mut self, kind: EngineEventKind, tick: u32, team: usize, slot: Option<usize>) {
        let mut e = self.event(kind, tick, Some(team));
        e.player = slot.map(|s| team * PLAYERS_PER_TEAM + s);
        self.out.push(e);
    }

    fn kick_off(&mut self, tick: u32, team: usize) {
        let taker = self.sides[team].kick_off_taker();
        self.push(EngineEventKind::KickOff, tick, team, taker);
    }

    fn half_time(&mut self, added_s: u32) {
        // The first half's clock stamps the break: minute 45 and the added minute it ends in.
        let tick = self.clock.half_time();
        let mut e = self.event(EngineEventKind::HalfTime, tick, None);
        e.minute = self.clock.half / TICKS_PER_MINUTE;
        e.minute_added = Some(self.clock.added[0] / TICKS_PER_MINUTE + 1);
        e.added_time_s = Some(added_s);
        self.out.push(e);
        self.kick_off(tick, 1);
    }

    fn item(&mut self, tick: u32, item: Item, rng: &mut EngineRng) {
        match item {
            Item::Goal(team) => {
                let side = &self.sides[team];
                let mut attackers = side.slots_where(|s, p| side.side.advanced[s] && !p.keeper);
                if attackers.is_empty() {
                    attackers = side.slots_where(|_, _| true);
                }
                let u = rng.next_f64();
                let scorer =
                    (!attackers.is_empty()).then(|| side.weighted(&attackers, |p| p.attack, u));
                self.scores[team] += 1;
                self.push(EngineEventKind::Goal, tick, team, scorer);
                self.kick_off(tick, 1 - team);
            }
            Item::Drawn(d) => self.drawn(tick, d, rng),
            Item::Decide(team, k) => {
                let id = ChangeId {
                    tick: tick.saturating_sub(1),
                    n: self.changes,
                };
                self.changes += 1;
                self.sides[team].decided[k] = Some(id);
                let mut e = self.event(EngineEventKind::AiDecision, tick, Some(team));
                e.detail = Some(EventDetail::Ai {
                    code: AiCode::SubFatigue,
                });
                self.out.push(e);
            }
            Item::Apply(team, first, last) => {
                for k in first..last {
                    let Some(id) = self.sides[team].decided[k] else {
                        continue;
                    };
                    let side = &self.sides[team];
                    let off = side.slots_where(|s, p| side.starter[s] && !p.keeper);
                    let Some(slot) = OnPitch::uniform(&off, rng.next_f64()) else {
                        continue;
                    };
                    self.substitute(tick, team, slot, id, false);
                }
            }
        }
    }

    /// Puts the next bench player on for the player in `slot`: a keeper for a keeper when
    /// the bench has one. Leaves the slot as it is when the rule pack or the bench allows no
    /// substitution, and says whether it substituted.
    fn substitute(
        &mut self,
        tick: u32,
        team: usize,
        slot: usize,
        id: ChangeId,
        keeper: bool,
    ) -> bool {
        let rules = &self.fit.rules.substitutions;
        let side = &mut self.sides[team];
        if !side.may_substitute(tick, rules) {
            return false;
        }
        let pick = side
            .bench
            .iter()
            .position(|p| p.keeper == keeper)
            .or_else(|| side.bench.iter().position(|p| !p.keeper));
        let (Some(at), Some(off)) = (pick, side.slots[slot]) else {
            return false;
        };
        let on = side.bench.remove(at);
        side.slots[slot] = Some(on);
        side.starter[slot] = false;
        side.booked[slot] = false;
        side.used += 1;
        if !side.windows.contains(&tick) {
            side.windows.push(tick);
        }
        let mut e = self.event(EngineEventKind::Substitution, tick, Some(team));
        e.player = Some(team * PLAYERS_PER_TEAM + slot);
        e.detail = Some(EventDetail::Substitution {
            off: off.squad,
            on: on.squad,
        });
        self.out.push(e);
        let mut e = self.event(EngineEventKind::ChangeApplied, tick, Some(team));
        e.detail = Some(EventDetail::Change {
            id,
            kind: ChangeKind::Substitution,
            reason: None,
        });
        self.out.push(e);
        true
    }

    fn drawn(&mut self, tick: u32, d: Drawn, rng: &mut EngineRng) {
        let team = d.team;
        let other = 1 - team;
        match d.what {
            What::Foul {
                advantage,
                penalty,
                card,
            } => {
                let side = &self.sides[team];
                let all = side.slots_where(|_, _| true);
                let fouled = self.sides[other].outfield();
                let (u, v) = (rng.next_f64(), rng.next_f64());
                if all.is_empty() || fouled.is_empty() {
                    return;
                }
                let offender = side.weighted(&all, |p| p.foul, u);
                let victim = OnPitch::uniform(&fouled, v).expect("an outfield player");
                let mut e = self.event(EngineEventKind::Foul, tick, Some(team));
                e.player = Some(team * PLAYERS_PER_TEAM + offender);
                e.secondary = Some(other * PLAYERS_PER_TEAM + victim);
                e.advantage = Some(advantage);
                self.out.push(e);
                if let Some(card) = card {
                    let side = &mut self.sides[team];
                    let shown = match card {
                        Card::Yellow if side.booked[offender] => Card::SecondYellow,
                        c => c,
                    };
                    if shown == Card::Yellow {
                        side.booked[offender] = true;
                    } else {
                        side.slots[offender] = None;
                    }
                    let mut e = self.event(EngineEventKind::Card, tick, Some(team));
                    e.player = Some(team * PLAYERS_PER_TEAM + offender);
                    e.card = Some(shown);
                    self.out.push(e);
                }
                if advantage {
                    return;
                }
                if penalty {
                    let side = &self.sides[other];
                    let all = side.slots_where(|_, _| true);
                    let taker = all.iter().copied().max_by(|&a, &b| {
                        let w = |s: usize| side.slots[s].map_or(0.0, |p| p.attack);
                        w(a).total_cmp(&w(b)).then(b.cmp(&a))
                    });
                    self.push(EngineEventKind::Penalty, tick, other, taker);
                } else {
                    self.push(EngineEventKind::FreeKick, tick, other, Some(victim));
                }
            }
            What::Offside => {
                let side = &self.sides[team];
                let mut front = side.slots_where(|s, p| side.side.advanced[s] && !p.keeper);
                if front.is_empty() {
                    front = side.outfield();
                }
                let (u, v) = (rng.next_f64(), rng.next_f64());
                let Some(player) = OnPitch::uniform(&front, u) else {
                    return;
                };
                self.push(EngineEventKind::Offside, tick, team, Some(player));
                let taker = OnPitch::uniform(&self.sides[other].outfield(), v);
                self.push(EngineEventKind::FreeKick, tick, other, taker);
            }
            What::Corner | What::ThrowIn => {
                let taker = OnPitch::uniform(&self.sides[team].outfield(), rng.next_f64());
                let kind = if d.what == What::Corner {
                    EngineEventKind::Corner
                } else {
                    EngineEventKind::ThrowIn
                };
                self.push(kind, tick, team, taker);
            }
            What::GoalKick => {
                let taker = self.sides[team].keeper();
                self.push(EngineEventKind::GoalKick, tick, team, taker);
            }
            What::Injury => {
                let all = self.sides[team].slots_where(|_, _| true);
                let Some(slot) = OnPitch::uniform(&all, rng.next_f64()) else {
                    return;
                };
                let mut e = self.event(EngineEventKind::Injury, tick, Some(team));
                e.player = Some(team * PLAYERS_PER_TEAM + slot);
                e.detail = Some(EventDetail::Injury {
                    source: InjurySource::Background,
                });
                self.out.push(e);
                let keeper = self.sides[team].slots[slot].is_some_and(|p| p.keeper);
                let id = ChangeId {
                    tick: tick.saturating_sub(1),
                    n: self.changes,
                };
                if self.sides[team].may_substitute(tick, &self.fit.rules.substitutions)
                    && !self.sides[team].bench.is_empty()
                {
                    self.changes += 1;
                    let mut e = self.event(EngineEventKind::AiDecision, tick, Some(team));
                    e.detail = Some(EventDetail::Ai {
                        code: AiCode::SubInjury,
                    });
                    self.out.push(e);
                    if self.substitute(tick, team, slot, id, keeper) {
                        return;
                    }
                }
                self.sides[team].slots[slot] = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 200 000 draws of each count have the closed-form mean within 1 percent, and the
    /// negative binomial its variance within 3 percent.
    #[test]
    fn the_count_draws_have_their_means() {
        let mut rng = EngineRng::from_seed(5);
        for (mean, k) in [
            (7.0, Some(12.0)),
            (11.4, Some(30.0)),
            (0.15, None),
            (1.6, None),
        ] {
            let n = 200_000;
            let draws: Vec<f64> = (0..n)
                .map(|_| f64::from(draw_count(mean, k, rng.next_f64())))
                .collect();
            let m = draws.iter().sum::<f64>() / f64::from(n);
            assert!((m - mean).abs() / mean < 0.01, "{mean} {k:?}: {m}");
            let var = draws.iter().map(|d| (d - m) * (d - m)).sum::<f64>() / f64::from(n - 1);
            let expected = mean + k.map_or(0.0, |k| mean * mean / k);
            assert!(
                (var - expected).abs() / expected < 0.03,
                "{mean} {k:?}: {var}"
            );
        }
        assert_eq!(draw_count(0.0, None, 0.5), 0);
    }

    /// Substitutions drawn close together share a stoppage, and never more stoppages than
    /// the windows.
    #[test]
    fn substitutions_group_into_the_windows() {
        let m = TICKS_PER_MINUTE;
        let ticks = [56 * m, 57 * m, 63 * m, 70 * m, 80 * m];
        let groups = stoppages(&ticks, 3);
        assert_eq!(groups.len(), 3);
        // 56 and 57 share a stoppage; 63 then joins them, the closest pair of four.
        assert_eq!(groups[0], (0..3, 63 * m));
        assert!(groups.iter().map(|(r, _)| r.len()).sum::<usize>() == 5);
        assert!(stoppages(&[], 3).is_empty());
        assert_eq!(stoppages(&ticks, 1), vec![(0..5, 80 * m)]);
    }

    /// Each bin's ticks show its minute, and added time shows the period's last minute with
    /// the added minute.
    #[test]
    fn the_clock_places_each_bin() {
        let clock = Clock {
            half: ticks_for_minutes(45),
            added: [150 * TICKS_PER_SECOND, 61 * TICKS_PER_SECOND],
        };
        assert_eq!(clock.minute(clock.tick(0, 0.0)), (0, None));
        assert_eq!(clock.minute(clock.tick(44, 0.99)), (44, None));
        assert_eq!(clock.minute(clock.tick(FIRST_ADDED, 0.0)), (45, Some(1)));
        assert_eq!(clock.minute(clock.tick(FIRST_ADDED, 0.999)), (45, Some(3)));
        assert!(clock.tick(FIRST_ADDED, 0.999) < clock.half_time());
        assert_eq!(clock.minute(clock.tick(46, 0.0)), (45, None));
        assert_eq!(clock.minute(clock.tick(90, 0.5)), (89, None));
        assert_eq!(clock.minute(clock.tick(SECOND_ADDED, 0.999)), (90, Some(2)));
        assert!(clock.tick(SECOND_ADDED, 0.999) < clock.full_time());
    }
}

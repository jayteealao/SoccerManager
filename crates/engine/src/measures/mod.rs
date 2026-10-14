//! The realism measures of one match: where shots come from, passes by kind and length and
//! whether they arrived, goals by the phase of play that began them, possession by third of
//! the pitch, and possession chains in passes and in seconds.
//!
//! A calibration run attaches [`MatchMeasures`] to a match
//! ([`crate::sim::Simulation::with_measures`]); the count sites of the match call its methods
//! and read, never write, the match, so a seed plays the same match with or without it. Only
//! a build with the `measures` feature has the module and its count sites.
//!
//! Every array is per side, home first.
//!
//! A possession chain starts when a side gains the ball, or takes a restart, and ends at an
//! opponent's touch, the ball going dead (out of play, a foul, an offside, any stoppage), a
//! goal, or full time. Its length counts the side's open-play passes and its restart kicks.
//! A goal takes the phase of the restart that began the scoring side's open chain, else
//! open play.
//!
//! The primitives' signatures count possessions the way the real-team table counts them in
//! the event data, which is not the chain above: a possession starts at a side's touch or
//! restart and passes to the other side at that side's first touch, unless the side that had
//! the ball touches it again within [`crate::decision::REGAIN_S`] with no dead ball between
//! while the other side completed no pass and took no shot (a short regain, merged into the
//! possession). A dead ball, a goal or full time ends the possession. Its passes count its
//! open-play passes and its restart kicks. An open-play pass is forward when its planned
//! target gains [`crate::decision::FORWARD_M`] or more toward the attacked goal; the open-play
//! passes of a possession before its first forward one are recycle passes (a restart kick is
//! a set play and takes no part); a possession the other side takes with no shot in it is a
//! turnover; a possession with a shot is a shot possession. The carrier's choices count by
//! kind, once each time his choice changes, not on every tick he holds it.

pub mod runs;
pub mod shape;

use crate::TICKS_PER_SECOND;
use crate::data::StoppageKind;
use crate::decision::PassKind;

/// Shot distance bins: 1 m each from 0 m to 40 m, then 40 m and over.
pub const SHOT_BINS: usize = 41;
/// Pass cells: three kinds by three length bins, keeper throw, keeper kick, and unknown.
pub const PASS_CELLS: usize = 12;
/// Goal phases, in [`PHASES`] order.
pub const PHASE_COUNT: usize = 7;
/// Chain length bins in passes: 0 to 14, then 15 and over.
pub const CHAIN_PASS_BINS: usize = 16;
/// Chain length bins in seconds: 2 s each up to 60 s, then 60 s and over.
pub const CHAIN_SECOND_BINS: usize = 31;
/// Seconds per chain length bin.
pub const CHAIN_BIN_S: u32 = 2;

/// The names of the pass cells, in cell order.
pub const PASS_CELL_NAMES: [&str; PASS_CELLS] = [
    "ground_lt10",
    "ground_10_25",
    "ground_gt25",
    "chip_lt10",
    "chip_10_25",
    "chip_gt25",
    "cross_lt10",
    "cross_10_25",
    "cross_gt25",
    "keeper_throw",
    "keeper_kick",
    "unknown",
];
/// The cell of a pass kicked without a matching plan (expected never).
pub const UNKNOWN_CELL: usize = 11;

/// The names of the goal phases, in phase order.
pub const PHASES: [&str; PHASE_COUNT] = [
    "open_play",
    "corner",
    "free_kick",
    "penalty",
    "throw_in",
    "kick_off",
    "goal_kick",
];

/// The cell of a pass of `kind` planned `d` metres long: ground pass, long pass or chip,
/// cross, each under 10 m, 10 to 25 m, or over 25 m; a keeper's throw and kick apart.
pub fn pass_cell(kind: PassKind, d: f64) -> usize {
    let length = if d < 10.0 {
        0
    } else if d <= 25.0 {
        1
    } else {
        2
    };
    match kind {
        PassKind::Pass => length,
        PassKind::Chip => 3 + length,
        PassKind::Cross => 6 + length,
        PassKind::Throw => 9,
        PassKind::Kick => 10,
    }
}

/// The phase of a goal whose chain began with `restart`.
fn phase_of(restart: Option<StoppageKind>) -> usize {
    match restart {
        Some(StoppageKind::Corner) => 1,
        Some(StoppageKind::FreeKick) => 2,
        Some(StoppageKind::Penalty) => 3,
        Some(StoppageKind::ThrowIn) => 4,
        Some(StoppageKind::KickOff) => 5,
        Some(StoppageKind::GoalKick) => 6,
        _ => 0,
    }
}

/// The third of the pitch, 0 (own) to 2 (final), of a ball at `x` for a side attacking
/// `attack_x`, on a pitch `length` metres long.
pub fn third_of(x: f64, attack_x: f64, length: f64) -> usize {
    let from_own_line = x * attack_x + 0.5 * length;
    // Truncation toward zero is the floor here: the clamp keeps a ball behind a line in
    // its end third.
    ((from_own_line * 3.0 / length).max(0.0) as usize).min(2)
}

/// The carrier's choices, in [`MatchMeasures::choices`] order.
pub const CHOICES: [&str; 5] = ["pass", "shot", "dribble", "hold", "clear"];

/// The possession chain in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Chain {
    team: usize,
    start: u32,
    passes: u32,
    restart: Option<StoppageKind>,
}

/// A possession of the primitives' count in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Possession {
    team: usize,
    passes: u32,
    open_passes: u32,
    recycle: u32,
    /// A forward pass was played in it.
    forward: bool,
    shot: bool,
    /// The ball was in the side's final third during it.
    final_third: bool,
}

impl Possession {
    fn new(team: usize) -> Self {
        Self {
            team,
            passes: 0,
            open_passes: 0,
            recycle: 0,
            forward: false,
            shot: false,
            final_third: false,
        }
    }

    /// A restart kick, or an open-play pass with its planned gain toward goal. A pass the
    /// carrier did not plan (a first-time touch on, a header) has no known target: it counts
    /// as a pass of the possession but takes no part in the recycle share.
    fn pass(&mut self, restart: bool, progress_m: Option<f64>) {
        self.passes += 1;
        let Some(gain) = progress_m.filter(|_| !restart) else {
            return;
        };
        self.open_passes += 1;
        if gain >= crate::decision::FORWARD_M {
            self.forward = true;
        } else if !self.forward {
            self.recycle += 1;
        }
    }
}

/// The other side's touches since its first touch at `start`: its possession if it keeps the
/// ball, else a short regain merged away.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Spell {
    start: u32,
    poss: Possession,
}

/// [`crate::decision::REGAIN_S`] in ticks.
fn regain_ticks() -> u32 {
    // The constant is a few seconds: the cast is exact.
    (crate::decision::REGAIN_S * f64::from(TICKS_PER_SECOND)).round() as u32
}

/// The pass the carrier planned this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Planned {
    carrier: usize,
    cell: usize,
    /// The planned target's gain toward the attacked goal, in metres.
    progress_m: f64,
    facts: Option<PassFacts>,
}

/// What the in-possession statistics read of a planned pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassFacts {
    /// From the carrier to the planned target, in metres.
    pub length_m: f64,
    /// The thirds, 0 (own) to 2 (final), of the start and of the target ([`third_of`]).
    pub start_third: usize,
    pub target_third: usize,
    /// The target's channel ([`crate::decision::channel`]): 0 left, 1 centre, 2 right.
    pub channel: usize,
    /// A cross ([`PassKind::Cross`]), and its start's distance from the attacked goal line.
    pub cross: bool,
    pub origin_m: f64,
    /// The carrier keeps goal.
    pub keeper: bool,
}

/// The in-possession counts of one side: every field is a count, or a sum of metres for an
/// exact mean.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Possessing {
    /// Open-play passes kicked as planned.
    pub passes: u32,
    /// Those gaining [`crate::decision::FORWARD_M`] or more toward goal, and those of
    /// [`crate::decision::LONG_BALL_M`] or more.
    pub forward: u32,
    pub long: u32,
    /// Those that start in the side's own third, and the long ones among them.
    pub own_third: u32,
    pub own_third_long: u32,
    /// The keeper's open-play passes and clearances, and the long passes and clearances
    /// among them.
    pub keeper_actions: u32,
    pub keeper_long: u32,
    /// Passes aimed into the middle or final third, and those aimed into a wide channel.
    pub upfield: u32,
    pub wide: u32,
    /// Completed passes from outside into the final third, by the target's channel.
    pub entries: [u32; 3],
    /// Open-play crosses, the lofted ones, and the sum of their starts' distances from the
    /// goal line.
    pub crosses: u32,
    pub high_crosses: u32,
    pub cross_origin_m: f64,
    /// Goal kicks taken and the sum of their lengths.
    pub goal_kicks: u32,
    pub goal_kick_m: f64,
    /// Open-play shots, those from outside the penalty area, and the sum of their distances.
    pub open_shots: u32,
    pub outside_shots: u32,
    pub open_shot_m: f64,
    /// Take-ons: opponents a dribbling carrier passed within the pressed distance, one each.
    pub takeons: u32,
    /// Carrier choices that were not the top-scored option before the noise.
    pub off_plan: u32,
    /// Possessions (the primitives' count) that reached the final third.
    pub final_third_possessions: u32,
}

/// The measures of one match.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchMeasures {
    /// Shots by distance from the ball at the kick to the centre of the goal attacked;
    /// every shot the summary counts, penalties included.
    pub shot_bins: [[u32; SHOT_BINS]; 2],
    /// The sum of those distances in metres, for an exact mean.
    pub shot_distance_m: [f64; 2],
    /// Open-play passes played, by [`pass_cell`]; they sum to the summary's passes.
    pub pass_attempts: [[u32; PASS_CELLS]; 2],
    /// Those whose next controlling touch was a team-mate's; they sum to the summary's
    /// completed passes.
    pub pass_completed: [[u32; PASS_CELLS]; 2],
    /// Goals by phase, in [`PHASES`] order; they sum to the summary's goals.
    pub goals_by_phase: [[u32; PHASE_COUNT]; 2],
    /// Open-play ticks credited to the side, by the ball's third counted from the side's own
    /// goal line; they sum to the summary's possession ticks.
    pub third_ticks: [[u32; 3]; 2],
    /// Possession chains ended.
    pub chains: [u32; 2],
    /// Chains by length in passes.
    pub chain_pass_bins: [[u32; CHAIN_PASS_BINS]; 2],
    /// Chains by length in seconds.
    pub chain_second_bins: [[u32; CHAIN_SECOND_BINS]; 2],
    /// The sum of the chains' passes and of their ticks, for exact means.
    pub chain_passes: [u32; 2],
    pub chain_ticks: [u64; 2],
    /// Possessions of the primitives' count ended (the module header defines them).
    pub possessions: [u32; 2],
    /// Their passes and restart kicks.
    pub possession_passes: [u32; 2],
    /// Their open-play passes with a planned target.
    pub open_play_passes: [u32; 2],
    /// The open-play passes played before their possession's first forward one.
    pub recycle_passes: [u32; 2],
    /// Possessions the other side took with no shot in them.
    pub turnovers: [u32; 2],
    /// Possessions that held a shot of their side.
    pub shot_possessions: [u32; 2],
    /// The carrier's choices by kind, in [`CHOICES`] order, each counted when it changes.
    pub choices: [[u32; 5]; 2],
    /// The in-possession counts.
    pub possessing: [Possessing; 2],
    /// The last carrier and his choice.
    last_choice: Option<(usize, usize)>,
    /// The carrier at the last decision and the opponents goal-side of him then, within the
    /// pressed distance.
    taking_on: Option<(usize, u32)>,
    planned: Option<Planned>,
    /// The cell of the pass in flight.
    in_flight: Option<usize>,
    /// The channel of the pass in flight when it enters the final third from outside.
    entry: Option<usize>,
    chain: Option<Chain>,
    possession: Option<Possession>,
    spell: Option<Spell>,
}

impl Default for MatchMeasures {
    fn default() -> Self {
        Self {
            shot_bins: [[0; SHOT_BINS]; 2],
            shot_distance_m: [0.0; 2],
            pass_attempts: [[0; PASS_CELLS]; 2],
            pass_completed: [[0; PASS_CELLS]; 2],
            goals_by_phase: [[0; PHASE_COUNT]; 2],
            third_ticks: [[0; 3]; 2],
            chains: [0; 2],
            chain_pass_bins: [[0; CHAIN_PASS_BINS]; 2],
            chain_second_bins: [[0; CHAIN_SECOND_BINS]; 2],
            chain_passes: [0; 2],
            chain_ticks: [0; 2],
            possessions: [0; 2],
            possession_passes: [0; 2],
            open_play_passes: [0; 2],
            recycle_passes: [0; 2],
            turnovers: [0; 2],
            shot_possessions: [0; 2],
            choices: [[0; 5]; 2],
            possessing: [Possessing::default(); 2],
            last_choice: None,
            taking_on: None,
            planned: None,
            in_flight: None,
            entry: None,
            chain: None,
            possession: None,
            spell: None,
        }
    }
}

impl MatchMeasures {
    /// Carrier `c` planned a pass of `cell` whose target gains `progress_m` metres toward the
    /// attacked goal.
    pub(crate) fn planned_pass(&mut self, c: usize, cell: usize, progress_m: f64) {
        self.planned = Some(Planned {
            carrier: c,
            cell,
            progress_m,
            facts: None,
        });
    }

    /// The in-possession facts of the pass carrier `c` just planned.
    pub(crate) fn pass_facts(&mut self, c: usize, facts: PassFacts) {
        if let Some(p) = self.planned.as_mut().filter(|p| p.carrier == c) {
            p.facts = Some(facts);
        }
    }

    /// The length of the pass carrier `c` planned, if he planned one with its facts.
    pub(crate) fn planned_length(&self, c: usize) -> Option<f64> {
        self.planned
            .filter(|p| p.carrier == c)
            .and_then(|p| p.facts)
            .map(|f| f.length_m)
    }

    /// Carrier `c` of `team` chose `choice`, an index into [`CHOICES`]: a choice counts when
    /// it differs from the last carrier's last one, and a counted choice `off_plan` (not the
    /// top-scored option before the noise) counts as one.
    pub(crate) fn chose(&mut self, team: usize, c: usize, choice: usize, off_plan: bool) {
        if self.last_choice == Some((c, choice)) {
            return;
        }
        self.choices[team][choice] += 1;
        self.last_choice = Some((c, choice));
        if off_plan {
            self.possessing[team].off_plan += 1;
        }
    }

    /// Carrier `c` of `team` at a decision: `ahead` and `beside` hold the roster bits of the
    /// opponents within the pressed distance of him, goal-side of him and level with or behind
    /// him. While he dribbles (`dribbling`), each opponent who was goal-side of him at his last
    /// decision and is now level or behind, still within the pressed distance, is a take-on:
    /// one per opponent he passes.
    pub(crate) fn carrying(
        &mut self,
        team: usize,
        c: usize,
        dribbling: bool,
        ahead: u32,
        beside: u32,
    ) {
        if let Some((last, was_ahead)) = self.taking_on
            && last == c
            && dribbling
        {
            self.possessing[team].takeons += (was_ahead & beside).count_ones();
        }
        self.taking_on = Some((c, ahead));
    }

    /// Carrier `c` of `team` kicked an open-play pass: the pass he planned, else an unknown
    /// one. It is in flight and adds a pass to the side's open chain.
    #[cfg(test)]
    pub(crate) fn kicked_pass(&mut self, c: usize, team: usize) {
        self.kicked_pass_lofted(c, team, 0.0);
    }

    /// [`MatchMeasures::kicked_pass`] with the kick's `loft`: a planned pass with its facts
    /// adds to the side's in-possession counts.
    pub(crate) fn kicked_pass_lofted(&mut self, c: usize, team: usize, loft: f64) {
        let planned = self.planned.take().filter(|p| p.carrier == c);
        let cell = planned.map_or(UNKNOWN_CELL, |p| p.cell);
        self.pass_attempts[team][cell] += 1;
        self.in_flight = Some(cell);
        self.entry = None;
        if let Some(p) = planned
            && let Some(f) = p.facts
        {
            let s = &mut self.possessing[team];
            let long = f.length_m >= crate::decision::LONG_BALL_M;
            s.passes += 1;
            s.forward += u32::from(p.progress_m >= crate::decision::FORWARD_M);
            s.long += u32::from(long);
            if f.start_third == 0 {
                s.own_third += 1;
                s.own_third_long += u32::from(long);
            }
            if f.keeper {
                s.keeper_actions += 1;
                s.keeper_long += u32::from(long);
            }
            if f.target_third >= 1 {
                s.upfield += 1;
                s.wide += u32::from(f.channel != 1);
            }
            if f.cross {
                s.crosses += 1;
                s.high_crosses += u32::from(loft > 0.0);
                s.cross_origin_m += f.origin_m;
            }
            if f.start_third < 2 && f.target_third == 2 {
                self.entry = Some(f.channel);
            }
        }
        self.add_pass(team, false, planned.map(|p| p.progress_m));
    }

    /// The keeper of `team` cleared the ball in open play: a long action of his.
    pub(crate) fn keeper_cleared(&mut self, team: usize) {
        let s = &mut self.possessing[team];
        s.keeper_actions += 1;
        s.keeper_long += 1;
    }

    /// `team` took a goal kick `length_m` metres long.
    pub(crate) fn goal_kick(&mut self, team: usize, length_m: f64) {
        let s = &mut self.possessing[team];
        s.goal_kicks += 1;
        s.goal_kick_m += length_m;
    }

    /// `team` shot in open play from `distance_m` metres from the goal centre, from outside
    /// the penalty area when `outside`.
    pub(crate) fn open_shot(&mut self, team: usize, distance_m: f64, outside: bool) {
        let s = &mut self.possessing[team];
        s.open_shots += 1;
        s.outside_shots += u32::from(outside);
        s.open_shot_m += distance_m;
    }

    /// `team`'s restart taker played his first kick: it adds a pass to the side's chain and
    /// possession.
    pub(crate) fn restart_kicked(&mut self, team: usize) {
        self.kicked_other();
        self.add_pass(team, true, None);
    }

    /// Any kick that is not a pass: no plan and no pass stays in flight.
    pub(crate) fn kicked_other(&mut self) {
        self.planned = None;
        self.in_flight = None;
        self.entry = None;
    }

    /// The pass in flight reached a team-mate of its passer.
    pub(crate) fn completed(&mut self, team: usize) {
        if let Some(cell) = self.in_flight.take() {
            self.pass_completed[team][cell] += 1;
            if let Some(ch) = self.entry.take() {
                self.possessing[team].entries[ch] += 1;
            }
        }
        // A completed pass of the other side: it keeps the ball.
        if self.spell.is_some_and(|s| s.poss.team == team) {
            self.take_over();
        }
    }

    /// `team` shot from `distance_m` metres from the centre of the goal it attacks.
    pub(crate) fn shot(&mut self, team: usize, distance_m: f64) {
        self.kicked_other();
        if let Some(s) = self.spell.as_mut().filter(|s| s.poss.team == team) {
            s.poss.shot = true;
            self.take_over();
        } else if let Some(p) = self.possession.as_mut().filter(|p| p.team == team) {
            p.shot = true;
        }
        // The float-to-int cast saturates; a distance is never negative.
        let bin = (distance_m.max(0.0) as usize).min(SHOT_BINS - 1);
        self.shot_bins[team][bin] += 1;
        self.shot_distance_m[team] += distance_m;
    }

    /// A player of `team` touched the ball: an opponent's touch ends the open chain.
    pub(crate) fn touched(&mut self, team: usize, tick: u32) {
        if self.chain.is_some_and(|ch| ch.team != team) {
            self.end_chain(tick);
        }
        self.spell_timeout(tick);
        match self.possession {
            None => self.possession = Some(Possession::new(team)),
            // The side with the ball touches it again: a short regain.
            Some(p) if p.team == team => self.spell = None,
            Some(_) => {
                if self.spell.is_none() {
                    self.spell = Some(Spell {
                        start: tick,
                        poss: Possession::new(team),
                    });
                }
            }
        }
    }

    /// The other side's spell has lasted longer than a short regain: it has the ball.
    fn spell_timeout(&mut self, tick: u32) {
        if self
            .spell
            .is_some_and(|s| tick.saturating_sub(s.start) > regain_ticks())
        {
            self.take_over();
        }
    }

    /// The other side's spell becomes its possession from its first touch; the possession
    /// before it ends as a turnover.
    fn take_over(&mut self) {
        if let Some(s) = self.spell.take() {
            self.close_possession(true);
            self.possession = Some(s.poss);
        }
    }

    /// Ends the open possession; `taken` when the other side took the ball. An open spell
    /// that never became a possession is merged away.
    fn close_possession(&mut self, taken: bool) {
        let Some(p) = self.possession.take() else {
            return;
        };
        let t = p.team;
        self.possessions[t] += 1;
        self.possession_passes[t] += p.passes;
        self.open_play_passes[t] += p.open_passes;
        self.recycle_passes[t] += p.recycle;
        if p.shot {
            self.shot_possessions[t] += 1;
        } else if taken {
            self.turnovers[t] += 1;
        }
        self.possessing[t].final_third_possessions += u32::from(p.final_third);
    }

    /// A dead ball, a goal or full time at `tick`: a spell longer than a short regain is the
    /// other side's possession, any other spell is merged away, and the possession ends.
    fn possession_stops(&mut self, tick: u32) {
        self.spell_timeout(tick);
        self.spell = None;
        self.close_possession(false);
    }

    /// `team` gained the ball: with no chain open, an open-play chain starts.
    pub(crate) fn gained(&mut self, team: usize, tick: u32) {
        self.in_flight = None;
        self.entry = None;
        if self.chain.is_none() {
            self.chain = Some(Chain {
                team,
                start: tick,
                passes: 0,
                restart: None,
            });
        }
    }

    /// `team` took a restart of `kind`: any open chain ends and the restart's chain starts.
    pub(crate) fn restart_taken(&mut self, kind: StoppageKind, team: usize, tick: u32) {
        self.end_chain(tick);
        self.chain = Some(Chain {
            team,
            start: tick,
            passes: 0,
            restart: Some(kind),
        });
        self.possession_stops(tick);
        self.possession = Some(Possession::new(team));
    }

    /// The ball went dead: the open chain ends and nothing stays in flight.
    pub(crate) fn dead(&mut self, tick: u32) {
        self.kicked_other();
        self.end_chain(tick);
        self.possession_stops(tick);
    }

    /// `team` scored: the goal takes the phase of the scoring side's open chain, then the
    /// chain ends.
    pub(crate) fn goal(&mut self, team: usize, tick: u32) {
        let restart = self
            .chain
            .filter(|ch| ch.team == team)
            .and_then(|ch| ch.restart);
        self.goals_by_phase[team][phase_of(restart)] += 1;
        self.end_chain(tick);
        self.possession_stops(tick);
    }

    /// One open-play tick credited to `team` with the ball in `third`.
    pub(crate) fn live_tick(&mut self, team: usize, third: usize) {
        self.third_ticks[team][third] += 1;
        if third == 2 {
            if let Some(s) = self.spell.as_mut().filter(|s| s.poss.team == team) {
                s.poss.final_third = true;
            } else if let Some(p) = self.possession.as_mut().filter(|p| p.team == team) {
                p.final_third = true;
            }
        }
    }

    /// Full time: the open chain ends.
    pub(crate) fn finish(&mut self, tick: u32) {
        self.end_chain(tick);
        self.possession_stops(tick);
    }

    /// A pass of `team`: a restart kick, or an open-play pass with its planned gain toward
    /// goal when known.
    fn add_pass(&mut self, team: usize, restart: bool, progress_m: Option<f64>) {
        if let Some(ch) = self.chain.as_mut().filter(|ch| ch.team == team) {
            ch.passes += 1;
        }
        if let Some(s) = self.spell.as_mut().filter(|s| s.poss.team == team) {
            s.poss.pass(restart, progress_m);
        } else if let Some(p) = self.possession.as_mut().filter(|p| p.team == team) {
            p.pass(restart, progress_m);
        }
    }

    fn end_chain(&mut self, tick: u32) {
        let Some(ch) = self.chain.take() else {
            return;
        };
        let ticks = tick.saturating_sub(ch.start);
        let t = ch.team;
        self.chains[t] += 1;
        self.chain_pass_bins[t][(ch.passes as usize).min(CHAIN_PASS_BINS - 1)] += 1;
        let bin = (ticks / (CHAIN_BIN_S * TICKS_PER_SECOND)) as usize;
        self.chain_second_bins[t][bin.min(CHAIN_SECOND_BINS - 1)] += 1;
        self.chain_passes[t] += ch.passes;
        self.chain_ticks[t] += u64::from(ticks);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: u32 = TICKS_PER_SECOND;
    const HOME: usize = 0;
    const AWAY: usize = 1;

    /// (a) A corner by home, a home touch, two completed passes, a shot and a goal: the goal
    /// is a corner goal and the chain holds the corner kick and the two passes.
    #[test]
    fn a_corner_goal_takes_the_corner_phase_and_its_chain() {
        let mut m = MatchMeasures::default();
        m.restart_taken(StoppageKind::Corner, HOME, 100);
        m.restart_kicked(HOME);
        m.touched(HOME, 120);
        m.gained(HOME, 120);
        for (c, t) in [(3, 150), (4, 180)] {
            m.planned_pass(c, pass_cell(PassKind::Pass, 12.0), 8.0);
            m.kicked_pass(c, HOME);
            m.touched(HOME, t);
            m.completed(HOME);
            m.touched(HOME, t + 10);
            m.gained(HOME, t + 10);
        }
        m.shot(HOME, 11.4);
        m.touched(HOME, 200);
        m.goal(HOME, 100 + 7 * S);
        m.dead(100 + 7 * S);
        assert_eq!(m.goals_by_phase[HOME], [0, 1, 0, 0, 0, 0, 0]);
        assert_eq!(m.chains, [1, 0]);
        assert_eq!(m.chain_passes[HOME], 3, "the corner kick and two passes");
        assert_eq!(m.chain_pass_bins[HOME][3], 1);
        assert_eq!(m.chain_ticks[HOME], u64::from(7 * S));
        assert_eq!(m.chain_second_bins[HOME][3], 1, "7 s is the 6 to 8 s bin");
        assert_eq!(m.pass_attempts[HOME][1], 2);
        assert_eq!(m.pass_completed[HOME][1], 2);
        assert_eq!(m.shot_bins[HOME][11], 1);
        assert!((m.shot_distance_m[HOME] - 11.4).abs() < 1e-12);
    }

    /// (b) The same corner cleared by the away side, then home regains and scores: the corner
    /// chain ends at the clearance and the goal is open play.
    #[test]
    fn a_rebound_after_a_cleared_corner_is_open_play() {
        let mut m = MatchMeasures::default();
        m.restart_taken(StoppageKind::Corner, HOME, 0);
        m.restart_kicked(HOME);
        // An away defender clears: his touch ends the home chain.
        m.touched(AWAY, 50);
        m.kicked_other();
        // Away gains the loose ball briefly, then home wins it back.
        m.touched(AWAY, 80);
        m.gained(AWAY, 80);
        m.touched(HOME, 130);
        m.gained(HOME, 130);
        m.shot(HOME, 18.0);
        m.goal(HOME, 230);
        assert_eq!(m.goals_by_phase[HOME], [1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(m.chains, [2, 1]);
        assert_eq!(m.chain_ticks, [50 + 100, 50]);
        assert_eq!(m.chain_passes, [1, 0]);
    }

    /// (c) A penalty scored is a penalty goal.
    #[test]
    fn a_penalty_goal_takes_the_penalty_phase() {
        let mut m = MatchMeasures::default();
        m.restart_taken(StoppageKind::Penalty, AWAY, 10);
        m.shot(AWAY, 11.0);
        m.goal(AWAY, 40);
        assert_eq!(m.goals_by_phase[AWAY], [0, 0, 0, 1, 0, 0, 0]);
        assert_eq!(m.shot_bins[AWAY][11], 1);
    }

    /// (d) A goal from a chain no restart began is open play.
    #[test]
    fn a_goal_from_a_won_ball_is_open_play() {
        let mut m = MatchMeasures::default();
        m.gained(HOME, 0);
        m.shot(HOME, 45.0);
        m.goal(HOME, 10);
        assert_eq!(m.goals_by_phase[HOME][0], 1);
        assert_eq!(m.shot_bins[HOME][SHOT_BINS - 1], 1, "40 m and over");
    }

    /// (e) An own goal: the open chain is the other side's, so the goal is open play even
    /// when that chain began at a corner.
    #[test]
    fn an_own_goal_is_open_play() {
        let mut m = MatchMeasures::default();
        m.restart_taken(StoppageKind::Corner, AWAY, 0);
        m.restart_kicked(AWAY);
        m.goal(HOME, 30);
        assert_eq!(m.goals_by_phase[HOME], [1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(m.goals_by_phase[AWAY], [0; PHASE_COUNT]);
        assert_eq!(m.chains, [0, 1]);
    }

    /// (f) A 16-pass chain and a 61-second chain land in the overflow bins.
    #[test]
    fn long_chains_land_in_the_overflow_bins() {
        let mut m = MatchMeasures::default();
        m.gained(HOME, 0);
        for c in 0..16 {
            m.planned_pass(c, 0, 0.0);
            m.kicked_pass(c, HOME);
            m.completed(HOME);
        }
        m.dead(10 * S);
        m.gained(AWAY, 0);
        m.dead(61 * S);
        assert_eq!(m.chain_pass_bins[HOME][CHAIN_PASS_BINS - 1], 1);
        assert_eq!(m.chain_passes[HOME], 16);
        assert_eq!(m.chain_second_bins[AWAY][CHAIN_SECOND_BINS - 1], 1);
        assert_eq!(m.chain_pass_bins[AWAY][0], 1);
    }

    /// (g) Full time closes an open chain once.
    #[test]
    fn full_time_closes_the_open_chain_once() {
        let mut m = MatchMeasures::default();
        m.gained(AWAY, 100);
        m.finish(300);
        m.finish(400);
        assert_eq!(m.chains, [0, 1]);
        assert_eq!(m.chain_ticks[AWAY], 200);
    }

    /// A pass kicked by another carrier than the one who planned it is unknown, and a pass
    /// that did not arrive counts no completion.
    #[test]
    fn an_unplanned_pass_is_unknown_and_an_interception_completes_nothing() {
        let mut m = MatchMeasures::default();
        m.planned_pass(2, 4, 0.0);
        m.kicked_pass(5, HOME);
        assert_eq!(m.pass_attempts[HOME][UNKNOWN_CELL], 1);
        m.gained(AWAY, 3);
        m.completed(AWAY);
        assert_eq!(m.pass_completed, [[0; PASS_CELLS]; 2]);
    }

    /// A back pass then a forward pass counts one recycle pass; the passes after the forward
    /// pass count none; a dead ball ends the possession with no turnover.
    #[test]
    fn passes_before_the_first_forward_pass_are_recycle_passes() {
        let mut m = MatchMeasures::default();
        m.touched(HOME, 0);
        m.gained(HOME, 0);
        for (c, gain, t) in [(2, -6.0, 30), (3, 12.0, 60), (4, -3.0, 90)] {
            m.planned_pass(c, 0, gain);
            m.kicked_pass(c, HOME);
            m.touched(HOME, t);
            m.completed(HOME);
            m.gained(HOME, t);
        }
        m.dead(100);
        assert_eq!(m.recycle_passes, [1, 0]);
        assert_eq!(m.possession_passes, [3, 0]);
        assert_eq!(m.open_play_passes, [3, 0]);
        assert_eq!(m.possessions, [1, 0]);
        assert_eq!(m.turnovers, [0, 0], "a dead ball is no turnover");
    }

    /// An interception the other side keeps past a short regain ends the possession as a
    /// turnover from its first touch; a shot then a save is a shot possession, not a turnover.
    #[test]
    fn an_interception_is_a_turnover_and_a_saved_shot_is_a_shot_possession() {
        let r = regain_ticks();
        let mut m = MatchMeasures::default();
        m.touched(HOME, 0);
        m.gained(HOME, 0);
        m.planned_pass(3, 0, 10.0);
        m.kicked_pass(3, HOME);
        m.touched(AWAY, 20);
        m.gained(AWAY, 20);
        assert_eq!(m.turnovers, [0, 0], "not yet: a short regain may follow");
        m.shot(AWAY, 16.0);
        assert_eq!(
            m.turnovers,
            [1, 0],
            "the shot shows the away side had the ball"
        );
        m.touched(HOME, 40 + r + 1);
        m.gained(HOME, 40 + r + 1);
        m.touched(AWAY, 60 + 3 * r);
        assert_eq!(m.turnovers, [1, 0]);
        assert_eq!(m.shot_possessions, [0, 1]);
        assert_eq!(m.possessions, [1, 1]);
        m.chose(HOME, 6, 3, false);
        m.chose(HOME, 6, 3, false);
        m.chose(HOME, 6, 0, false);
        m.chose(HOME, 7, 0, false);
        assert_eq!(
            m.choices[HOME],
            [2, 0, 0, 1, 0],
            "a choice counts when it changes"
        );
    }

    /// The other side's touch that the side with the ball follows within a short regain, with
    /// no completed pass and no shot, is merged: one possession, its passes kept, no turnover.
    /// A completed pass of the other side takes the ball at once. A restart kick counts as a
    /// pass of the possession but never as a recycle pass.
    #[test]
    fn a_short_regain_is_merged_and_a_completed_pass_takes_the_ball() {
        let r = regain_ticks();
        let mut m = MatchMeasures::default();
        m.restart_taken(StoppageKind::ThrowIn, HOME, 0);
        m.planned_pass(2, 0, -2.0);
        m.restart_kicked(HOME);
        m.touched(AWAY, 10);
        m.kicked_other();
        m.touched(HOME, 10 + r);
        m.gained(HOME, 10 + r);
        m.planned_pass(4, 0, 20.0);
        m.kicked_pass(4, HOME);
        m.touched(AWAY, 100);
        m.gained(AWAY, 100);
        m.planned_pass(13, 0, 6.0);
        m.kicked_pass(13, AWAY);
        m.touched(AWAY, 110);
        m.completed(AWAY);
        m.gained(AWAY, 110);
        assert_eq!(m.possessions, [1, 0]);
        assert_eq!(m.possession_passes, [2, 0]);
        assert_eq!(m.open_play_passes, [1, 0]);
        assert_eq!(m.recycle_passes, [0, 0]);
        assert_eq!(m.turnovers, [1, 0]);
        m.dead(400);
        assert_eq!(m.possessions, [1, 1]);
        assert_eq!(m.possession_passes, [2, 1]);
        assert_eq!(m.turnovers, [1, 0]);
    }

    #[test]
    fn the_pass_cells_follow_kind_and_length() {
        assert_eq!(pass_cell(PassKind::Pass, 9.99), 0);
        assert_eq!(pass_cell(PassKind::Pass, 10.0), 1);
        assert_eq!(pass_cell(PassKind::Pass, 25.0), 1);
        assert_eq!(pass_cell(PassKind::Chip, 25.1), 5);
        assert_eq!(pass_cell(PassKind::Cross, 18.0), 7);
        assert_eq!(pass_cell(PassKind::Throw, 5.0), 9);
        assert_eq!(pass_cell(PassKind::Kick, 50.0), 10);
        assert_eq!(PASS_CELL_NAMES[UNKNOWN_CELL], "unknown");
    }

    /// The thirds count from each side's own goal line, whichever way it attacks.
    #[test]
    fn the_thirds_follow_the_direction_of_attack() {
        let l = 105.0;
        assert_eq!(third_of(-50.0, 1.0, l), 0);
        assert_eq!(third_of(0.0, 1.0, l), 1);
        assert_eq!(third_of(50.0, 1.0, l), 2);
        assert_eq!(third_of(-50.0, -1.0, l), 2);
        assert_eq!(third_of(50.0, -1.0, l), 0);
        assert_eq!(third_of(-60.0, 1.0, l), 0, "behind the own line");
        assert_eq!(third_of(60.0, 1.0, l), 2, "behind the far line");
    }

    fn facts(length_m: f64, start_third: usize, target_third: usize, channel: usize) -> PassFacts {
        PassFacts {
            length_m,
            start_third,
            target_third,
            channel,
            cross: false,
            origin_m: 0.0,
            keeper: false,
        }
    }

    /// A 35 m pass from the own third counts long and own-third long; a 20 m forward pass into
    /// the middle third's right channel counts forward and wide; a keeper's long pass and his
    /// clearance are both long keeper actions.
    #[test]
    fn a_long_pass_from_the_own_third_counts_long_and_own_third_long() {
        let mut m = MatchMeasures::default();
        m.planned_pass(3, 2, 30.0);
        m.pass_facts(3, facts(35.0, 0, 1, 1));
        m.kicked_pass(3, HOME);
        m.planned_pass(4, 1, 18.0);
        m.pass_facts(4, facts(20.0, 1, 1, 2));
        m.kicked_pass(4, HOME);
        m.planned_pass(0, 10, 40.0);
        m.pass_facts(
            0,
            PassFacts {
                keeper: true,
                ..facts(40.0, 0, 1, 1)
            },
        );
        m.kicked_pass(0, HOME);
        m.keeper_cleared(HOME);
        let s = m.possessing[HOME];
        assert_eq!((s.passes, s.forward, s.long), (3, 3, 2));
        assert_eq!((s.own_third, s.own_third_long), (2, 2));
        assert_eq!((s.upfield, s.wide), (3, 1));
        assert_eq!((s.keeper_actions, s.keeper_long), (2, 2));
        assert_eq!(m.possessing[AWAY], Possessing::default());
    }

    /// A pass from the middle third into the final third counts as an entry by its channel
    /// only when a team-mate takes it; an interception counts none.
    #[test]
    fn an_entry_counts_only_when_completed() {
        let mut m = MatchMeasures::default();
        m.planned_pass(5, 1, 20.0);
        m.pass_facts(5, facts(22.0, 1, 2, 0));
        m.kicked_pass(5, HOME);
        m.gained(AWAY, 10);
        m.completed(AWAY);
        assert_eq!(m.possessing[HOME].entries, [0, 0, 0]);
        m.planned_pass(6, 1, 20.0);
        m.pass_facts(6, facts(22.0, 1, 2, 0));
        m.kicked_pass(6, HOME);
        m.completed(HOME);
        assert_eq!(m.possessing[HOME].entries, [1, 0, 0]);
        // A pass inside the final third is no entry.
        m.planned_pass(7, 1, 8.0);
        m.pass_facts(7, facts(10.0, 2, 2, 1));
        m.kicked_pass(7, HOME);
        m.completed(HOME);
        assert_eq!(m.possessing[HOME].entries, [1, 0, 0]);
    }

    /// A lofted cross counts high, a driven one low; a possession whose ball reached the final
    /// third counts once when it ends; a dribbler counts one take-on for each opponent he
    /// passes within the pressed distance, and none for an opponent he passes while not
    /// dribbling or one another carrier left behind.
    #[test]
    fn crosses_final_third_possessions_and_takeons_count_by_their_definitions() {
        let mut m = MatchMeasures::default();
        m.touched(HOME, 0);
        m.gained(HOME, 0);
        m.live_tick(HOME, 1);
        m.live_tick(HOME, 2);
        m.live_tick(HOME, 2);
        for (c, loft) in [(8, 4.0), (9, 0.0)] {
            m.planned_pass(c, 7, 6.0);
            m.pass_facts(
                c,
                PassFacts {
                    cross: true,
                    origin_m: 12.0,
                    ..facts(18.0, 2, 2, 1)
                },
            );
            m.kicked_pass_lofted(c, HOME, loft);
        }
        m.dead(50);
        let s = m.possessing[HOME];
        assert_eq!((s.crosses, s.high_crosses), (2, 1));
        assert!((s.cross_origin_m - 24.0).abs() < 1e-12);
        assert_eq!(s.final_third_possessions, 1);
        // Opponents 13 and 14 goal-side, then 13 passed, then 14 passed while holding, then
        // both goal-side again and both passed at once; a new carrier passes nobody.
        for (c, dribbling, ahead, beside) in [
            (9, true, 0b11 << 13, 0),
            (9, true, 0b10 << 13, 0b01 << 13),
            (9, false, 0, 0b10 << 13),
            (9, true, 0b11 << 13, 0),
            (9, true, 0, 0b11 << 13),
            (10, true, 0b01 << 13, 0),
            (11, true, 0, 0b01 << 13),
        ] {
            m.carrying(HOME, c, dribbling, ahead, beside);
        }
        assert_eq!(m.possessing[HOME].takeons, 3);
    }
}

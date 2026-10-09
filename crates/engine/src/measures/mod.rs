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

/// The possession chain in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Chain {
    team: usize,
    start: u32,
    passes: u32,
    restart: Option<StoppageKind>,
}

/// The pass the carrier planned this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Planned {
    carrier: usize,
    cell: usize,
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
    planned: Option<Planned>,
    /// The cell of the pass in flight.
    in_flight: Option<usize>,
    chain: Option<Chain>,
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
            planned: None,
            in_flight: None,
            chain: None,
        }
    }
}

impl MatchMeasures {
    /// Carrier `c` planned a pass of `cell`.
    pub(crate) fn planned_pass(&mut self, c: usize, cell: usize) {
        self.planned = Some(Planned { carrier: c, cell });
    }

    /// Carrier `c` of `team` kicked an open-play pass: the pass he planned, else an unknown
    /// one. It is in flight and adds a pass to the side's open chain.
    pub(crate) fn kicked_pass(&mut self, c: usize, team: usize) {
        let cell = self
            .planned
            .take()
            .filter(|p| p.carrier == c)
            .map_or(UNKNOWN_CELL, |p| p.cell);
        self.pass_attempts[team][cell] += 1;
        self.in_flight = Some(cell);
        self.add_pass(team);
    }

    /// `team`'s restart taker played his first kick: it adds a pass to the side's chain.
    pub(crate) fn restart_kicked(&mut self, team: usize) {
        self.kicked_other();
        self.add_pass(team);
    }

    /// Any kick that is not a pass: no plan and no pass stays in flight.
    pub(crate) fn kicked_other(&mut self) {
        self.planned = None;
        self.in_flight = None;
    }

    /// The pass in flight reached a team-mate of its passer.
    pub(crate) fn completed(&mut self, team: usize) {
        if let Some(cell) = self.in_flight.take() {
            self.pass_completed[team][cell] += 1;
        }
    }

    /// `team` shot from `distance_m` metres from the centre of the goal it attacks.
    pub(crate) fn shot(&mut self, team: usize, distance_m: f64) {
        self.kicked_other();
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
    }

    /// `team` gained the ball: with no chain open, an open-play chain starts.
    pub(crate) fn gained(&mut self, team: usize, tick: u32) {
        self.in_flight = None;
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
    }

    /// The ball went dead: the open chain ends and nothing stays in flight.
    pub(crate) fn dead(&mut self, tick: u32) {
        self.kicked_other();
        self.end_chain(tick);
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
    }

    /// One open-play tick credited to `team` with the ball in `third`.
    pub(crate) fn live_tick(&mut self, team: usize, third: usize) {
        self.third_ticks[team][third] += 1;
    }

    /// Full time: the open chain ends.
    pub(crate) fn finish(&mut self, tick: u32) {
        self.end_chain(tick);
    }

    fn add_pass(&mut self, team: usize) {
        if let Some(ch) = self.chain.as_mut().filter(|ch| ch.team == team) {
            ch.passes += 1;
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
            m.planned_pass(c, pass_cell(PassKind::Pass, 12.0));
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
            m.planned_pass(c, 0);
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
        m.planned_pass(2, 4);
        m.kicked_pass(5, HOME);
        assert_eq!(m.pass_attempts[HOME][UNKNOWN_CELL], 1);
        m.gained(AWAY, 3);
        m.completed(AWAY);
        assert_eq!(m.pass_completed, [[0; PASS_CELLS]; 2]);
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
}

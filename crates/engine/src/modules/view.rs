//! The read-only view a module reads the match through.

use crate::ai::AiState;
use crate::ball::Ball;
use crate::data::attributes::AttributeSchema;
use crate::data::rules::RulePack;
use crate::data::tactics::TacticsSchema;
use crate::data::tuning::FatigueTuning;
use crate::player::Player;
use crate::rules::Referee;
use crate::sim::{Simulation, Summary};
use crate::tactics::change::{ChangeQueue, SubLedger};
use crate::team::Team;
use crate::tuning::Tuning;

/// A borrowed, read-only view of a match. Its one field is private, and every accessor
/// returns a shared reference or a copy, so a module cannot write match state through it:
/// assigning through an accessor is a compile error, and so is reaching the field.
#[derive(Clone, Copy)]
pub struct MatchView<'a> {
    sim: &'a Simulation,
}

impl<'a> MatchView<'a> {
    #[inline]
    pub(crate) fn new(sim: &'a Simulation) -> Self {
        Self { sim }
    }

    /// Every player on the roster, in roster order.
    #[inline]
    pub fn players(&self) -> &'a [Player] {
        &self.sim.players
    }

    /// The player at roster index `i`.
    #[inline]
    pub fn player(&self, i: usize) -> &'a Player {
        &self.sim.players[i]
    }

    /// The current tick.
    #[inline]
    pub fn tick(&self) -> u32 {
        self.sim.tick
    }

    /// The engine tuning of the match.
    #[inline]
    pub fn tuning(&self) -> &'a Tuning {
        &self.sim.config.tuning
    }

    /// The fatigue tuning of the match.
    #[inline]
    pub fn fatigue(&self) -> &'a FatigueTuning {
        &self.sim.config.fatigue
    }

    /// The ball's position along the length of the pitch.
    #[inline]
    pub fn ball_x(&self) -> f64 {
        self.sim.ball.pos.x
    }

    /// The attack direction of `team`: +1 or -1 along x.
    #[inline]
    pub fn attack_x(&self, team: usize) -> f64 {
        self.sim.teams[team].attack_x
    }

    /// The referee's state: the phase, the clock, the tally, held cards, and the shoot-out.
    #[inline]
    pub fn referee(&self) -> &'a Referee {
        &self.sim.referee
    }

    /// Both teams.
    #[inline]
    pub fn teams(&self) -> &'a [Team; 2] {
        &self.sim.teams
    }

    /// The rule pack of the match.
    #[inline]
    pub fn rules(&self) -> &'a RulePack {
        &self.sim.config.rules
    }

    /// `true` for a knockout match, which a level score sends to extra time and penalties.
    #[inline]
    pub fn knockout(&self) -> bool {
        self.sim.config.knockout
    }

    /// Each team's goals so far.
    #[inline]
    pub fn goals(&self) -> [u32; 2] {
        self.sim.summary.goals
    }

    /// The ball.
    #[inline]
    pub fn ball(&self) -> &'a Ball {
        &self.sim.ball
    }

    /// The team that touched the ball last.
    #[inline]
    pub fn last_touch(&self) -> Option<usize> {
        self.sim.last_touch
    }

    /// The attribute schema of the match.
    #[inline]
    pub fn attributes(&self) -> &'a AttributeSchema {
        &self.sim.config.attributes
    }

    /// The roster index of the player keeping goal for `team`.
    #[inline]
    pub fn keeper(&self, team: usize) -> usize {
        self.sim.keeper(team)
    }

    /// The player controlling the ball, if any.
    #[inline]
    pub fn carrier(&self) -> Option<usize> {
        self.sim.carrier
    }

    /// The tick the carrier gained the ball.
    #[inline]
    pub fn control_since(&self) -> u32 {
        self.sim.control_since
    }

    /// `true` once a keeper failed to hold the ball in this flight.
    #[inline]
    pub fn keeper_beaten(&self) -> bool {
        self.sim.keeper_beaten
    }

    /// The team whose open-play pass is in flight and not yet resolved.
    #[inline]
    pub fn pass_in_flight(&self) -> Option<usize> {
        self.sim.pass_in_flight
    }

    /// The team whose shot is in flight and not yet resolved.
    #[inline]
    pub fn shot_in_flight(&self) -> Option<usize> {
        self.sim.shot_in_flight
    }

    /// `true` while the shot in flight heads on target.
    #[inline]
    pub fn shot_on_target(&self) -> bool {
        self.sim.shot_on_target
    }

    /// The quality of the shot in flight, for the save roll.
    #[inline]
    pub fn shot_quality(&self) -> f64 {
        self.sim.shot_quality
    }

    /// The outfield defenders (one bit per roster index) who already tried to block the shot.
    #[inline]
    pub fn blockers_tried(&self) -> u32 {
        self.sim.blockers_tried
    }

    /// The defenders (one bit per roster index) who already tried to clear the pass in flight.
    #[inline]
    pub fn clearers_tried(&self) -> u32 {
        self.sim.clearers_tried
    }

    /// The restart taker until his first kick.
    #[inline]
    pub fn restart_taker(&self) -> Option<usize> {
        self.sim.restart_taker
    }

    /// The changes waiting for a stoppage.
    #[inline]
    pub fn queue(&self) -> &'a ChangeQueue {
        &self.sim.queue
    }

    /// Each team's substitutions so far, home first.
    #[inline]
    pub fn ledgers(&self) -> [SubLedger; 2] {
        self.sim.ledgers
    }

    /// The AI manager's memory for `team`.
    #[inline]
    pub fn ai_memory(&self, team: usize) -> AiState {
        self.sim.ai[team]
    }

    /// The substitution limit and window count in force.
    #[inline]
    pub fn substitution_limits(&self) -> (u8, u8) {
        self.sim.substitution_limits()
    }

    /// The tactics file of the match.
    #[inline]
    pub fn tactics(&self) -> &'a TacticsSchema {
        &self.sim.config.tactics
    }

    /// The match summary so far.
    #[inline]
    pub fn summary(&self) -> &'a Summary {
        &self.sim.summary
    }
}

impl Simulation {
    /// The read-only view modules read this match through.
    #[inline]
    pub fn view(&self) -> MatchView<'_> {
        MatchView::new(self)
    }
}

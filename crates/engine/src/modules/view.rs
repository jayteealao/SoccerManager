//! The read-only view a module reads the match through.

use crate::ball::Ball;
use crate::data::attributes::AttributeSchema;
use crate::data::rules::RulePack;
use crate::data::tuning::FatigueTuning;
use crate::player::Player;
use crate::rules::Referee;
use crate::sim::Simulation;
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
}

impl Simulation {
    /// The read-only view modules read this match through.
    #[inline]
    pub fn view(&self) -> MatchView<'_> {
        MatchView::new(self)
    }
}

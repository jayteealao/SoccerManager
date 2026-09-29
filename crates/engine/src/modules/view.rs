//! The read-only view a module reads the match through.

use crate::data::tuning::FatigueTuning;
use crate::player::Player;
use crate::sim::Simulation;
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
}

impl Simulation {
    /// The read-only view modules read this match through.
    #[inline]
    pub fn view(&self) -> MatchView<'_> {
        MatchView::new(self)
    }
}

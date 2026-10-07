//! Modifiers: effects on a player's effective values, such as fatigue. Each modifier names
//! its family (body, mind, familiarity, surroundings) and returns one factor per effective
//! value it may scale, where 1.0 is no effect. Within a family the factors multiply; across
//! families the soft combine ([`soft_combine`]) makes each further effect in one direction
//! count for less than the one before. The loop writes the result into `p.derived`.
//!
//! With one non-neutral factor (fatigue alone, as in every match today) the combine returns
//! that factor bit for bit, so each effective value is the one multiply it always was.

mod combine;
pub mod stand_ins;

pub use combine::soft_combine;

use super::MatchView;
use super::registry::MODIFIER_COUNT;
use crate::player::Derived;

/// The number of modifier families.
pub const FAMILY_COUNT: usize = 4;

/// What kind of effect a modifier is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Body,
    Mind,
    Familiarity,
    Surroundings,
}

/// One factor per effective value a modifier may scale. 1.0 is no effect. Passing,
/// finishing, decisions, and composure each scale a group of stage values
/// ([`crate::contract::EFFECT_STAGES`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effect {
    pub max_speed: f64,
    pub max_accel: f64,
    pub passing: f64,
    pub finishing: f64,
    pub decisions: f64,
    pub composure: f64,
}

impl Effect {
    /// No effect on any value.
    pub const NEUTRAL: Effect = Effect::all(1.0);

    /// The factor `m` on every value.
    pub const fn all(m: f64) -> Self {
        Self {
            max_speed: m,
            max_accel: m,
            passing: m,
            finishing: m,
            decisions: m,
            composure: m,
        }
    }

    /// The six factors in order: top speed, acceleration, passing, finishing, decisions,
    /// composure.
    pub fn fields(&self) -> [f64; 6] {
        [
            self.max_speed,
            self.max_accel,
            self.passing,
            self.finishing,
            self.decisions,
            self.composure,
        ]
    }
}

/// A modifier. It reads the match and returns its effect on player `i`; it never writes and
/// never draws.
pub trait Modifier: Send + Sync + 'static {
    /// The family the modifier belongs to.
    fn family(&self) -> Family;
    /// Its effect on player `i` now.
    fn effect(&self, view: &MatchView<'_>, i: usize) -> Effect;
}

/// A modifier with no effect: the stand-ins that wait for their own piece, and every
/// modifier's off version.
pub struct Neutral(pub Family);

impl Modifier for Neutral {
    fn family(&self) -> Family {
        self.0
    }

    fn effect(&self, _: &MatchView<'_>, _: usize) -> Effect {
        Effect::NEUTRAL
    }
}

/// The modifiers of one match, in registry order.
#[derive(Clone, Copy)]
pub struct Modifiers([&'static dyn Modifier; MODIFIER_COUNT]);

impl Modifiers {
    pub(crate) fn new(list: [&'static dyn Modifier; MODIFIER_COUNT]) -> Self {
        Self(list)
    }

    /// Player `i`'s effective values: its base values scaled by every modifier's effect,
    /// multiplied within each family and soft-combined across families.
    pub fn effective(&self, view: &MatchView<'_>, i: usize) -> Derived {
        // One family step per field: the product of the family's factors, from 1.0.
        let mut families = [[1.0; FAMILY_COUNT]; 6];
        for m in &self.0 {
            let family = m.family() as usize;
            for (field, factor) in families.iter_mut().zip(m.effect(view, i).fields()) {
                field[family] *= factor;
            }
        }
        view.base(i).scaled(families.map(soft_combine))
    }
}

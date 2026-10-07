//! Modifiers: the states that move a player's ratings, such as fatigue. Each modifier names
//! its family (body, mind, familiarity, surroundings) and returns a delta in rating points per
//! attribute group (technical, mental, physical, goalkeeping), where 0 is no effect. Within a
//! family the deltas add; a family acts only on the groups it owns and is clamped to its cap;
//! across families the soft combine ([`soft_combine`]) makes each further effect in one
//! direction count for less than the one before; the total is clamped and rounded to a tenth
//! ([`crate::contract::states::capped`]). The refresh derives the player's effective values
//! from his ratings plus those deltas.
//!
//! With one non-zero delta in a group (fatigue alone, as in a match with no condition inputs)
//! the combine returns that delta bit for bit.

mod combine;
pub mod condition;
pub mod stand_ins;

pub use combine::soft_combine;

use super::MatchView;
use super::registry::MODIFIER_COUNT;
use crate::contract::states::{self, GROUP_COUNT};

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

/// A delta in rating points per attribute group, in [`crate::contract::states::GROUPS`]
/// order. 0 is no effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effect {
    pub groups: [f64; GROUP_COUNT],
}

impl Effect {
    /// No effect on any group.
    pub const NEUTRAL: Effect = Effect {
        groups: [0.0; GROUP_COUNT],
    };
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

    /// Player `i`'s state delta per attribute group, in tenths of a rating point: every
    /// modifier's deltas summed within each family, then capped and combined across families
    /// ([`states::capped`]).
    pub fn effective(&self, view: &MatchView<'_>, i: usize) -> [i8; GROUP_COUNT] {
        let mut families = [[0.0; GROUP_COUNT]; FAMILY_COUNT];
        for m in &self.0 {
            let family = &mut families[m.family() as usize];
            for (sum, d) in family.iter_mut().zip(m.effect(view, i).groups) {
                *sum += d;
            }
        }
        states::capped(families, &view.tuning().contract.states)
    }
}

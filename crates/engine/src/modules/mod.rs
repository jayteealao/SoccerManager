//! The module contract. Each engine part that moved onto it is a module behind a typed
//! interface, registered by name and version in one ordered table ([`registry::REGISTRY`])
//! and chosen per slot by the slot file (`content/slots.json`).
//!
//! A module reads the match through a [`MatchView`], which has private fields and read-only
//! accessors, so the compiler refuses a module that writes match state. A module that needs
//! a change returns a [`Proposal`]; only the central loop applies it. A module never draws:
//! the loop draws on the action keys the module's card lists, and the module maps the draw
//! to an outcome. So every stream id, every draw, and every draw order stay as they were.

pub mod card;
pub mod config;
pub mod proposal;
pub mod registry;
pub mod view;

use std::fmt;

use crate::rules::fouls::{Card, Tackle};
use crate::rules::offside::OffsideSet;

pub use card::{CardError, MOVED_KEYS, ModuleCard, OwnershipError, check_card, check_ownership};
pub use config::{SLOTS_VERSION, SlotEntry, SlotFile, resolve};
pub use proposal::Proposal;
pub use registry::{ModuleRef, REGISTRY, Registration, SlotDecl};
pub use view::MatchView;

/// A slot: a place in the engine that one module fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// The dotted id the slot file names, such as `engine.fouls`.
    pub id: &'static str,
    /// A required slot refuses `off`.
    pub required: bool,
}

/// The chances of one tackle attempt, computed before its one draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TackleChances {
    /// A clean win below this.
    pub p_win: f64,
    /// A foul in the next band of this width.
    pub p_foul: f64,
    /// The share of the foul band after which the fouled team loses the ball.
    pub ball_loss: f64,
}

/// Tackles, fouls, and cards. The loop draws the `Tackle` key against
/// `[p_win, p_win + p_foul]` and the `FoulCard` key against [`FoulsModule::card_thresholds`].
pub trait FoulsModule: Send + Sync + 'static {
    /// The chances of one tackle attempt by `tackler` on `carrier`.
    fn tackle_chances(&self, view: &MatchView<'_>, tackler: usize, carrier: usize)
    -> TackleChances;
    /// What the attempt did, given its draw.
    fn tackle_outcome(&self, chances: &TackleChances, draw: f64) -> Tackle;
    /// The two cumulative thresholds of the card draw for a foul by `offender`.
    fn card_thresholds(&self, view: &MatchView<'_>, offender: usize) -> [f64; 2];
    /// The card, if any, for a foul by `offender`, given the card draw.
    fn card(&self, view: &MatchView<'_>, offender: usize, draw: f64) -> Option<Card>;
}

/// Offside (IFAB Law 11). It draws nothing.
pub trait OffsideModule: Send + Sync + 'static {
    /// `passer` of `team` plays the ball in open play: the offside positions it fixes.
    fn on_kick(&self, view: &MatchView<'_>, passer: usize, team: usize) -> Proposal;
    /// `true` when `toucher` commits an offence, given the positions fixed at the kick.
    fn is_offence(&self, view: &MatchView<'_>, set: OffsideSet, toucher: usize) -> bool;
}

/// One slot's choice: the slot id, the module name, and its version (0 for `off`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Picked {
    pub slot: &'static str,
    pub module: &'static str,
    pub version: u32,
}

/// The modules one match runs, resolved once from the slot file. Modules are stateless, so
/// the references are shared and nothing about them enters a snapshot.
#[derive(Clone, Copy)]
pub struct ResolvedModules {
    pub fouls: &'static dyn FoulsModule,
    pub offside: &'static dyn OffsideModule,
    picked: [Picked; 2],
}

impl ResolvedModules {
    /// What each slot holds, in registry order.
    pub fn picked(&self) -> &[Picked] {
        &self.picked
    }

    /// The module chosen for `slot`, or `None` for an undeclared slot.
    pub fn picked_for(&self, slot: &str) -> Option<Picked> {
        self.picked.iter().copied().find(|p| p.slot == slot)
    }
}

impl fmt::Debug for ResolvedModules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_map();
        for p in &self.picked {
            list.entry(&p.slot, &format_args!("{}@{}", p.module, p.version));
        }
        list.finish()
    }
}

impl PartialEq for ResolvedModules {
    fn eq(&self, other: &Self) -> bool {
        self.picked == other.picked
    }
}

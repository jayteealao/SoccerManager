//! The game-wide stub slots (`game.world`, `game.season`, `game.people`,
//! `game.presentation`): the world (nations, clubs, grounds), the season systems, the people
//! and their minds, and what the player sees. None of these systems exists yet, so each slot
//! declares one no-op entry point, a default module that proposes no change, an off version,
//! and a card. Nothing in the engine calls them; a later system gives its slot behaviour
//! through a new module version.
//!
//! These systems live outside a match, so a stub reads no [`super::MatchView`] and draws
//! nothing, and no stub card owns an action key.

use super::card::ModuleCard;

/// A day of the game calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameDay(pub u32);

/// The changes a game-wide module proposes for one day. Empty until a system exists.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct GameChanges {}

/// The world: nations, clubs, and grounds.
pub trait WorldModule: Send + Sync + 'static {
    /// The changes the world proposes on `day`.
    fn on_day(&self, day: GameDay) -> GameChanges;
}

/// The season systems.
pub trait SeasonModule: Send + Sync + 'static {
    /// The changes the season systems propose on `day`.
    fn on_day(&self, day: GameDay) -> GameChanges;
}

/// The people and their minds.
pub trait PeopleModule: Send + Sync + 'static {
    /// The changes the people propose on `day`.
    fn on_day(&self, day: GameDay) -> GameChanges;
}

/// What the player sees.
pub trait PresentationModule: Send + Sync + 'static {
    /// The changes the presentation proposes on `day`.
    fn on_day(&self, day: GameDay) -> GameChanges;
}

/// One stub module: its default (`<name>-stub@1`) and its off version share the no-op body.
macro_rules! stub {
    ($trait:ident, $v1:ident, $off:ident) => {
        #[doc = concat!("The no-op default of the `", stringify!($trait), "` slot.")]
        pub struct $v1;

        impl $trait for $v1 {
            fn on_day(&self, _: GameDay) -> GameChanges {
                GameChanges::default()
            }
        }

        #[doc = concat!("The off version of the `", stringify!($trait), "` slot.")]
        pub struct $off;

        impl $trait for $off {
            fn on_day(&self, _: GameDay) -> GameChanges {
                GameChanges::default()
            }
        }
    };
}

stub!(WorldModule, WorldStubV1, WorldOff);
stub!(SeasonModule, SeasonStubV1, SeasonOff);
stub!(PeopleModule, PeopleStubV1, PeopleOff);
stub!(PresentationModule, PresentationStubV1, PresentationOff);

const INPUTS: &str = "The game day.";
const OUTPUTS: &str = "No change.";
const CALIBRATION: &str = "none: stub slot, no behaviour yet";
const OFF_PURPOSE: &str = "Switches the slot off: proposes no change.";

const fn stub_card(purpose: &'static str) -> ModuleCard {
    ModuleCard {
        purpose,
        inputs: INPUTS,
        outputs: OUTPUTS,
        tuning: &["none"],
        calibration: CALIBRATION,
        keys: &[],
    }
}

pub const WORLD_STUB_V1_CARD: ModuleCard =
    stub_card("Stub for the world (nations, clubs, grounds): no behaviour yet.");
pub const WORLD_OFF_CARD: ModuleCard = stub_card(OFF_PURPOSE);
pub const SEASON_STUB_V1_CARD: ModuleCard =
    stub_card("Stub for the season systems: no behaviour yet.");
pub const SEASON_OFF_CARD: ModuleCard = stub_card(OFF_PURPOSE);
pub const PEOPLE_STUB_V1_CARD: ModuleCard =
    stub_card("Stub for the people and their minds: no behaviour yet.");
pub const PEOPLE_OFF_CARD: ModuleCard = stub_card(OFF_PURPOSE);
pub const PRESENTATION_STUB_V1_CARD: ModuleCard =
    stub_card("Stub for what the player sees: no behaviour yet.");
pub const PRESENTATION_OFF_CARD: ModuleCard = stub_card(OFF_PURPOSE);

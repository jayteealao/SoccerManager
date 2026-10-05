//! The viewer's skin slot (`viewer.skin`): which look the match viewer loads. A skin is a
//! folder of token, style, and font files in the viewer project; the module only names the
//! folder. The engine never reads a skin, so a skin can never change a match: `viewer.*`
//! slots stay out of the content digest.
//!
//! The slot is optional. Its off version means "no skin chosen" and names the viewer's
//! built-in default look, the same reading as the rules slot's off version.

use super::card::ModuleCard;

/// The skin folder names the viewer project ships, in registry order. A test holds this
/// list equal to the folders under `viewer/src/skins/`.
pub const SKIN_NAMES: [&str; 2] = ["broadcast-blue", "interim-light"];

/// The look the viewer loads.
pub trait SkinModule: Send + Sync + 'static {
    /// The name of the skin folder the viewer loads.
    fn skin(&self) -> &'static str;
}

/// The default look: dark Broadcast Blue with the Saira fonts.
pub struct BroadcastBlueV1;

impl SkinModule for BroadcastBlueV1 {
    fn skin(&self) -> &'static str {
        SKIN_NAMES[0]
    }
}

/// The interim light look, kept as the test skin that proves a swap.
pub struct InterimLightV1;

impl SkinModule for InterimLightV1 {
    fn skin(&self) -> &'static str {
        SKIN_NAMES[1]
    }
}

/// The off version: no skin chosen, so the viewer shows its built-in default look.
pub struct SkinOff;

impl SkinModule for SkinOff {
    fn skin(&self) -> &'static str {
        SKIN_NAMES[0]
    }
}

const fn skin_card(purpose: &'static str) -> ModuleCard {
    ModuleCard {
        purpose,
        inputs: "None.",
        outputs: "The skin folder name the viewer loads.",
        tuning: &["none"],
        calibration: "none: viewer skin, no realism band",
        keys: &[],
    }
}

pub const BROADCAST_BLUE_V1_CARD: ModuleCard =
    skin_card("The default viewer look: dark Broadcast Blue with the Saira fonts.");
pub const INTERIM_LIGHT_V1_CARD: ModuleCard =
    skin_card("The interim light viewer look, used as the test skin that proves a swap.");
pub const SKIN_OFF_CARD: ModuleCard =
    skin_card("Switches the slot off: the viewer shows its built-in default look.");

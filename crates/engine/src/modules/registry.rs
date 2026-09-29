//! The slot registry: an explicit, ordered table of every slot and the modules registered
//! for it, by name and version. The order is fixed here, never by link order, so
//! registration can never change a match, and no registration can be dropped by the linker.

use super::card::ModuleCard;
use super::{FoulsModule, OffsideModule, Slot};
use crate::rules::{fouls, offside};

/// A registered module, typed by the slot it fills.
#[derive(Clone, Copy)]
pub enum ModuleRef {
    Fouls(&'static dyn FoulsModule),
    Offside(&'static dyn OffsideModule),
}

/// One module registered for a slot.
#[derive(Clone, Copy)]
pub struct Registration {
    pub name: &'static str,
    pub version: u32,
    pub module: ModuleRef,
    pub card: &'static ModuleCard,
}

/// One slot and what may fill it.
#[derive(Clone, Copy)]
pub struct SlotDecl {
    pub slot: Slot,
    /// The modules by name and version, in a fixed order.
    pub registrations: &'static [Registration],
    /// The off version an optional slot takes on `{"module": "off"}`; `None` for a required
    /// slot.
    pub off: Option<Registration>,
}

impl SlotDecl {
    /// The valid values for this slot, as a refusal lists them: `name@version`, then `off`.
    pub fn valid_names(&self) -> String {
        let mut names: Vec<String> = self
            .registrations
            .iter()
            .map(|r| format!("{}@{}", r.name, r.version))
            .collect();
        if self.off.is_some() {
            names.push("off".to_string());
        }
        names.join(", ")
    }
}

pub const FOULS: Slot = Slot {
    id: "engine.fouls",
    required: false,
};

pub const OFFSIDE: Slot = Slot {
    id: "engine.offside",
    required: false,
};

const FOULS_REGISTRATIONS: &[Registration] = &[Registration {
    name: "fouls",
    version: 1,
    module: ModuleRef::Fouls(&fouls::FoulsV1),
    card: &fouls::FOULS_V1_CARD,
}];

#[cfg(not(feature = "scenario"))]
const OFFSIDE_REGISTRATIONS: &[Registration] = &[OFFSIDE_V1];

/// Test builds also register a faulty offside module, which the gate tests select to prove
/// that one changed output fails the gate. A release build never contains it.
#[cfg(feature = "scenario")]
const OFFSIDE_REGISTRATIONS: &[Registration] = &[
    OFFSIDE_V1,
    Registration {
        name: "offside-faulty",
        version: 1,
        module: ModuleRef::Offside(&offside::OffsideFaulty),
        card: &offside::OFFSIDE_FAULTY_CARD,
    },
];

const OFFSIDE_V1: Registration = Registration {
    name: "offside",
    version: 1,
    module: ModuleRef::Offside(&offside::OffsideV1),
    card: &offside::OFFSIDE_V1_CARD,
};

/// Every slot, in the fixed order the engine resolves them.
pub static REGISTRY: &[SlotDecl] = &[
    SlotDecl {
        slot: FOULS,
        registrations: FOULS_REGISTRATIONS,
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Fouls(&fouls::FoulsOff),
            card: &fouls::FOULS_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: OFFSIDE,
        registrations: OFFSIDE_REGISTRATIONS,
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Offside(&offside::OffsideOff),
            card: &offside::OFFSIDE_OFF_CARD,
        }),
    },
];

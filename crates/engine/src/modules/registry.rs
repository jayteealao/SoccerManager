//! The slot registry: an explicit, ordered table of every slot and the modules registered
//! for it, by name and version. The order is fixed here, never by link order, so
//! registration can never change a match, and no registration can be dropped by the linker.

use super::card::ModuleCard;
use super::modifier::{Modifier, stand_ins};
use super::{
    BallModule, ChangesModule, ClockModule, DecisionModule, DisciplineModule, FatigueModule,
    FoulsModule, InjuriesModule, ManagerModule, OffsideModule, PossessionModule, PreMatchModule,
    RestartsModule, ShotModule, Slot, SteeringModule,
};
use crate::rules::{clock, discipline, fouls, injury, offside, restart};
use crate::tactics::verdict;
use crate::{ai, ball, decision, fatigue, possession, shot, steering};

/// A registered module, typed by the slot it fills.
#[derive(Clone, Copy)]
pub enum ModuleRef {
    Fouls(&'static dyn FoulsModule),
    Offside(&'static dyn OffsideModule),
    Shot(&'static dyn ShotModule),
    Fatigue(&'static dyn FatigueModule),
    Steering(&'static dyn SteeringModule),
    PreMatch(&'static dyn PreMatchModule),
    Modifier(&'static dyn Modifier),
    Clock(&'static dyn ClockModule),
    Restarts(&'static dyn RestartsModule),
    Discipline(&'static dyn DisciplineModule),
    Injuries(&'static dyn InjuriesModule),
    Ball(&'static dyn BallModule),
    Possession(&'static dyn PossessionModule),
    Decision(&'static dyn DecisionModule),
    Manager(&'static dyn ManagerModule),
    Changes(&'static dyn ChangesModule),
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

pub const SHOT: Slot = Slot {
    id: "engine.shot",
    required: false,
};

pub const FATIGUE: Slot = Slot {
    id: "engine.fatigue",
    required: false,
};

/// Movement and steering is on the core list, so it is required and has no off version.
pub const STEERING: Slot = Slot {
    id: "engine.steering",
    required: true,
};

pub const PRE_MATCH: Slot = Slot {
    id: "engine.pre-match",
    required: false,
};

/// The modifier slots, in family order: body, mind, then surroundings.
pub const MODIFIER_FATIGUE: Slot = Slot {
    id: "engine.modifier.fatigue",
    required: false,
};

pub const MODIFIER_PRESSURE: Slot = Slot {
    id: "engine.modifier.pressure",
    required: false,
};

pub const MODIFIER_MOMENTUM: Slot = Slot {
    id: "engine.modifier.momentum",
    required: false,
};

pub const MODIFIER_WEATHER: Slot = Slot {
    id: "engine.modifier.weather",
    required: false,
};

/// The clock and match end is on the core list, so it is required and has no off version.
pub const CLOCK: Slot = Slot {
    id: "engine.clock",
    required: true,
};

/// Restarts are on the core list, so the slot is required and has no off version.
pub const RESTARTS: Slot = Slot {
    id: "engine.restarts",
    required: true,
};

pub const DISCIPLINE: Slot = Slot {
    id: "engine.discipline",
    required: false,
};

pub const INJURIES: Slot = Slot {
    id: "engine.injuries",
    required: false,
};

/// Ball physics is on the core list, so the slot is required and has no off version.
pub const BALL: Slot = Slot {
    id: "engine.ball",
    required: true,
};

/// Possession is on the core list, so the slot is required and has no off version.
pub const POSSESSION: Slot = Slot {
    id: "engine.possession",
    required: true,
};

/// The decision maker is on the core list, so the slot is required and has no off version.
pub const DECISION: Slot = Slot {
    id: "engine.decision",
    required: true,
};

/// The AI manager's in-match checks.
pub const MANAGER: Slot = Slot {
    id: "engine.manager",
    required: false,
};

/// Tactics changes: what a stoppage admits and the verdicts on changes.
pub const CHANGES: Slot = Slot {
    id: "engine.changes",
    required: false,
};

/// The number of declared slots. `ResolvedModules` holds one typed field per slot, and one
/// `Modifiers` field for the modifier slots.
pub const SLOT_COUNT: usize = 19;

/// The number of modifier slots.
pub const MODIFIER_COUNT: usize = 4;

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

#[cfg(not(feature = "scenario"))]
const SHOT_REGISTRATIONS: &[Registration] = &[SHOT_V1];

/// Test builds also register a faulty shot module, which the gate tests select to prove
/// that one changed output fails the gate. A release build never contains it.
#[cfg(feature = "scenario")]
const SHOT_REGISTRATIONS: &[Registration] = &[
    SHOT_V1,
    Registration {
        name: "shot-faulty",
        version: 1,
        module: ModuleRef::Shot(&shot::ShotFaulty),
        card: &shot::SHOT_FAULTY_CARD,
    },
];

const SHOT_V1: Registration = Registration {
    name: "shot",
    version: 1,
    module: ModuleRef::Shot(&shot::ShotV1),
    card: &shot::SHOT_V1_CARD,
};

#[cfg(not(feature = "scenario"))]
const CLOCK_REGISTRATIONS: &[Registration] = &[CLOCK_V1];

/// Test builds also register a faulty clock, which the gate tests select to prove that one
/// changed output fails the gate. A release build never contains it.
#[cfg(feature = "scenario")]
const CLOCK_REGISTRATIONS: &[Registration] = &[
    CLOCK_V1,
    Registration {
        name: "clock-faulty",
        version: 1,
        module: ModuleRef::Clock(&clock::ClockFaulty),
        card: &clock::CLOCK_FAULTY_CARD,
    },
];

#[cfg(not(feature = "scenario"))]
const POSSESSION_REGISTRATIONS: &[Registration] = &[POSSESSION_V1];

/// Test builds also register a faulty possession module, which the gate tests select to
/// prove that one changed output fails the gate. A release build never contains it.
#[cfg(feature = "scenario")]
const POSSESSION_REGISTRATIONS: &[Registration] = &[
    POSSESSION_V1,
    Registration {
        name: "possession-faulty",
        version: 1,
        module: ModuleRef::Possession(&possession::PossessionFaulty),
        card: &possession::POSSESSION_FAULTY_CARD,
    },
];

const POSSESSION_V1: Registration = Registration {
    name: "possession",
    version: 1,
    module: ModuleRef::Possession(&possession::PossessionV1),
    card: &possession::POSSESSION_V1_CARD,
};

const CLOCK_V1: Registration = Registration {
    name: "clock",
    version: 1,
    module: ModuleRef::Clock(&clock::ClockV1),
    card: &clock::CLOCK_V1_CARD,
};

/// Every slot, in the fixed order the engine resolves them.
pub static REGISTRY: &[SlotDecl] = &DECLS;

const DECLS: [SlotDecl; SLOT_COUNT] = [
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
    SlotDecl {
        slot: SHOT,
        registrations: SHOT_REGISTRATIONS,
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Shot(&shot::ShotOff),
            card: &shot::SHOT_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: FATIGUE,
        registrations: &[Registration {
            name: "fatigue",
            version: 1,
            module: ModuleRef::Fatigue(&fatigue::FatigueV1),
            card: &fatigue::FATIGUE_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Fatigue(&fatigue::FatigueOff),
            card: &fatigue::FATIGUE_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: STEERING,
        registrations: &[Registration {
            name: "steering",
            version: 1,
            module: ModuleRef::Steering(&steering::SteeringV1),
            card: &steering::STEERING_V1_CARD,
        }],
        off: None,
    },
    SlotDecl {
        slot: PRE_MATCH,
        registrations: &[Registration {
            name: "pre-match",
            version: 1,
            module: ModuleRef::PreMatch(&ai::PreMatchV1),
            card: &ai::PRE_MATCH_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::PreMatch(&ai::PreMatchOff),
            card: &ai::PRE_MATCH_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: MODIFIER_FATIGUE,
        registrations: &[Registration {
            name: "fatigue-curve",
            version: 1,
            module: ModuleRef::Modifier(&fatigue::FatigueCurveV1),
            card: &fatigue::FATIGUE_CURVE_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Modifier(&fatigue::FATIGUE_CURVE_OFF),
            card: &fatigue::FATIGUE_CURVE_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: MODIFIER_PRESSURE,
        registrations: &[Registration {
            name: "pressure",
            version: 1,
            module: ModuleRef::Modifier(&stand_ins::PRESSURE_V1),
            card: &stand_ins::PRESSURE_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Modifier(&stand_ins::PRESSURE_OFF),
            card: &stand_ins::PRESSURE_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: MODIFIER_MOMENTUM,
        registrations: &[Registration {
            name: "momentum",
            version: 1,
            module: ModuleRef::Modifier(&stand_ins::MOMENTUM_V1),
            card: &stand_ins::MOMENTUM_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Modifier(&stand_ins::MOMENTUM_OFF),
            card: &stand_ins::MOMENTUM_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: MODIFIER_WEATHER,
        registrations: &[Registration {
            name: "weather",
            version: 1,
            module: ModuleRef::Modifier(&stand_ins::WEATHER_V1),
            card: &stand_ins::WEATHER_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Modifier(&stand_ins::WEATHER_OFF),
            card: &stand_ins::WEATHER_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: CLOCK,
        registrations: CLOCK_REGISTRATIONS,
        off: None,
    },
    SlotDecl {
        slot: RESTARTS,
        registrations: &[Registration {
            name: "restarts",
            version: 1,
            module: ModuleRef::Restarts(&restart::RestartsV1),
            card: &restart::RESTARTS_V1_CARD,
        }],
        off: None,
    },
    SlotDecl {
        slot: DISCIPLINE,
        registrations: &[Registration {
            name: "discipline",
            version: 1,
            module: ModuleRef::Discipline(&discipline::DisciplineV1),
            card: &discipline::DISCIPLINE_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Discipline(&discipline::DisciplineOff),
            card: &discipline::DISCIPLINE_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: INJURIES,
        registrations: &[Registration {
            name: "injuries",
            version: 1,
            module: ModuleRef::Injuries(&injury::InjuriesV1),
            card: &injury::INJURIES_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Injuries(&injury::InjuriesOff),
            card: &injury::INJURIES_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: BALL,
        registrations: &[Registration {
            name: "ball",
            version: 1,
            module: ModuleRef::Ball(&ball::BallV1),
            card: &ball::BALL_V1_CARD,
        }],
        off: None,
    },
    SlotDecl {
        slot: POSSESSION,
        registrations: POSSESSION_REGISTRATIONS,
        off: None,
    },
    SlotDecl {
        slot: DECISION,
        registrations: &[Registration {
            name: "decision",
            version: 1,
            module: ModuleRef::Decision(&decision::DecisionV1),
            card: &decision::DECISION_V1_CARD,
        }],
        off: None,
    },
    SlotDecl {
        slot: MANAGER,
        registrations: &[Registration {
            name: "ai-manager",
            version: 1,
            module: ModuleRef::Manager(&ai::AiManagerV1),
            card: &ai::AI_MANAGER_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Manager(&ai::ManagerOff),
            card: &ai::MANAGER_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: CHANGES,
        registrations: &[Registration {
            name: "changes",
            version: 1,
            module: ModuleRef::Changes(&verdict::ChangesV1),
            card: &verdict::CHANGES_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Changes(&verdict::ChangesOff),
            card: &verdict::CHANGES_OFF_CARD,
        }),
    },
];

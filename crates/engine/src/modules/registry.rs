//! The slot registry: an explicit, ordered table of every slot and the modules registered
//! for it, by name and version. The order is fixed here, never by link order, so
//! registration can never change a match, and no registration can be dropped by the linker.

use super::card::ModuleCard;
use super::fast_model::{self, FastModel};
use super::game::{self, PeopleModule, PresentationModule, SeasonModule, WorldModule};
use super::modifier::{Modifier, stand_ins};
use super::viewer::{self, SkinModule};
use super::{
    BallModule, ChangesModule, ClockModule, CommentaryHookModule, DecisionHookModule,
    DecisionModule, DisciplineModule, FatigueModule, FoulsModule, InjuriesModule, ManagerModule,
    OffsideModule, PossessionModule, PreMatchModule, RestartsModule, RuleHookModule, RulesModule,
    ShotModule, Slot, SteeringModule,
};
use crate::rules::{clock, discipline, fouls, injury, offside, pack, restart};
use crate::tactics::verdict;
use crate::{ai, ball, decision, fatigue, hook_slots, possession, shot, steering};

/// Every typed slot kind once: its `ModuleRef` variant, its `ResolvedModules` field, and its
/// slot trait. `ModuleRef` and the resolver's builder are both generated from this one list,
/// so a new kind is written here once, and the compiler then checks the `ResolvedModules`
/// struct against it. The modifier kind is not listed: its four slots share one list.
macro_rules! for_each_slot_kind {
    ($callback:ident) => {
        $callback! {
            Fouls fouls FoulsModule,
            Offside offside OffsideModule,
            Shot shot ShotModule,
            Fatigue fatigue FatigueModule,
            Steering steering SteeringModule,
            PreMatch pre_match PreMatchModule,
            Clock clock ClockModule,
            Restarts restarts RestartsModule,
            Discipline discipline DisciplineModule,
            Injuries injuries InjuriesModule,
            Ball ball BallModule,
            Possession possession PossessionModule,
            Decision decision DecisionModule,
            Manager manager ManagerModule,
            Changes changes ChangesModule,
            DecisionHook decision_hook DecisionHookModule,
            RuleHook rule_hook RuleHookModule,
            CommentaryHook commentary_hook CommentaryHookModule,
            Rules rule_pack RulesModule,
            World world WorldModule,
            Season season SeasonModule,
            People people PeopleModule,
            Presentation presentation PresentationModule,
            Skin skin SkinModule,
            FastModel fast_model FastModel,
        }
    };
}
pub(crate) use for_each_slot_kind;

macro_rules! module_ref_enum {
    ($($variant:ident $field:ident $kind:ident,)*) => {
        /// A registered module, typed by the slot it fills.
        #[derive(Clone, Copy)]
        pub enum ModuleRef {
            $($variant(&'static dyn $kind),)*
            Modifier(&'static dyn Modifier),
        }
    };
}
for_each_slot_kind!(module_ref_enum);

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

/// The decision hook's slot. The hooks are not on the core list, so each hook slot is
/// optional; its off version consults no hook.
pub const HOOK_DECISION: Slot = Slot {
    id: "engine.hook.decision",
    required: false,
};

/// The rule hook's slot.
pub const HOOK_RULE: Slot = Slot {
    id: "engine.hook.rule",
    required: false,
};

/// The commentary hook's slot.
pub const HOOK_COMMENTARY: Slot = Slot {
    id: "engine.hook.commentary",
    required: false,
};

/// The rule pack slot. The game-wide slots are not on the core list, so each is optional;
/// the rules slot's off version plays under the standard Laws built into the program.
pub const RULES: Slot = Slot {
    id: "game.rules",
    required: false,
};

/// The world's stub slot: nations, clubs, and grounds. No behaviour yet.
pub const WORLD: Slot = Slot {
    id: "game.world",
    required: false,
};

/// The season systems' stub slot. No behaviour yet.
pub const SEASON: Slot = Slot {
    id: "game.season",
    required: false,
};

/// The stub slot of the people and their minds. No behaviour yet.
pub const PEOPLE: Slot = Slot {
    id: "game.people",
    required: false,
};

/// The stub slot of what the player sees. No behaviour yet.
pub const PRESENTATION: Slot = Slot {
    id: "game.presentation",
    required: false,
};

/// The viewer's skin slot: which look the viewer loads. Optional; off shows the viewer's
/// built-in default look. A `viewer.*` slot never enters the content digest.
pub const SKIN: Slot = Slot {
    id: "viewer.skin",
    required: false,
};

/// The fast-model slot: a results model fitted from full-engine results. Optional; off
/// refuses to play. Only the fit and check commands reach it (`fast_model::resolve`), and it
/// never enters the content digest, because it never plays a full-engine match.
pub const FAST_MODEL: Slot = Slot {
    id: "engine.fast-model",
    required: false,
};

/// The number of declared slots, counted from the registry table. `ResolvedModules` holds
/// one typed field per slot, and one `Modifiers` field for the modifier slots.
pub const SLOT_COUNT: usize = DECLS.len();

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
pub static REGISTRY: &[SlotDecl] = DECLS;

const DECLS: &[SlotDecl] = &[
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
    SlotDecl {
        slot: HOOK_DECISION,
        registrations: &[Registration {
            name: "decision-hook",
            version: 1,
            module: ModuleRef::DecisionHook(&hook_slots::DecisionHookV1),
            card: &hook_slots::DECISION_HOOK_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::DecisionHook(&hook_slots::DecisionHookOff),
            card: &hook_slots::DECISION_HOOK_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: HOOK_RULE,
        registrations: &[Registration {
            name: "rule-hook",
            version: 1,
            module: ModuleRef::RuleHook(&hook_slots::RuleHookV1),
            card: &hook_slots::RULE_HOOK_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::RuleHook(&hook_slots::RuleHookOff),
            card: &hook_slots::RULE_HOOK_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: HOOK_COMMENTARY,
        registrations: &[Registration {
            name: "commentary-hook",
            version: 1,
            module: ModuleRef::CommentaryHook(&hook_slots::CommentaryHookV1),
            card: &hook_slots::COMMENTARY_HOOK_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::CommentaryHook(&hook_slots::CommentaryHookOff),
            card: &hook_slots::COMMENTARY_HOOK_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: RULES,
        registrations: &[Registration {
            name: "rule-pack",
            version: 1,
            module: ModuleRef::Rules(&pack::RulePackV1),
            card: &pack::RULE_PACK_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Rules(&pack::RulePackOff),
            card: &pack::RULE_PACK_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: WORLD,
        registrations: &[Registration {
            name: "world-stub",
            version: 1,
            module: ModuleRef::World(&game::WorldStubV1),
            card: &game::WORLD_STUB_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::World(&game::WorldOff),
            card: &game::WORLD_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: SEASON,
        registrations: &[Registration {
            name: "season-stub",
            version: 1,
            module: ModuleRef::Season(&game::SeasonStubV1),
            card: &game::SEASON_STUB_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Season(&game::SeasonOff),
            card: &game::SEASON_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: PEOPLE,
        registrations: &[Registration {
            name: "people-stub",
            version: 1,
            module: ModuleRef::People(&game::PeopleStubV1),
            card: &game::PEOPLE_STUB_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::People(&game::PeopleOff),
            card: &game::PEOPLE_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: PRESENTATION,
        registrations: &[Registration {
            name: "presentation-stub",
            version: 1,
            module: ModuleRef::Presentation(&game::PresentationStubV1),
            card: &game::PRESENTATION_STUB_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Presentation(&game::PresentationOff),
            card: &game::PRESENTATION_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: SKIN,
        registrations: &[
            Registration {
                name: "broadcast-blue",
                version: 1,
                module: ModuleRef::Skin(&viewer::BroadcastBlueV1),
                card: &viewer::BROADCAST_BLUE_V1_CARD,
            },
            Registration {
                name: "interim-light",
                version: 1,
                module: ModuleRef::Skin(&viewer::InterimLightV1),
                card: &viewer::INTERIM_LIGHT_V1_CARD,
            },
        ],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::Skin(&viewer::SkinOff),
            card: &viewer::SKIN_OFF_CARD,
        }),
    },
    SlotDecl {
        slot: FAST_MODEL,
        registrations: &[Registration {
            name: fast_model::FITTED_SCORES,
            version: 1,
            module: ModuleRef::FastModel(&fast_model::FittedScoresV1),
            card: &fast_model::FITTED_SCORES_V1_CARD,
        }],
        off: Some(Registration {
            name: "off",
            version: 0,
            module: ModuleRef::FastModel(&fast_model::FastModelOff),
            card: &fast_model::FAST_MODEL_OFF_CARD,
        }),
    },
];

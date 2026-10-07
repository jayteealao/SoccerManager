//! Older content versions and their converters. A version 1 team file or tactics file, a
//! version 1 attribute file, or a version 2 tuning file is read in its own shape, checked as
//! before, and lifted to the current shape on load, so the shipped version 1 files and the
//! inputs that old replays embed still play.
//!
//! The attribute and tuning converters add the attribute contract from a frozen copy of its
//! first tables (`contract/frozen-v1.json`), compiled into the build and never edited, so an
//! old replay converts the same way however the shipped tables are tuned later. A version 3
//! tuning file takes the state caps, the body jobs, and the fatigue group weights from a
//! second frozen copy (`contract/frozen-v2.json`), and a version 1 slot file takes the two
//! condition modifiers in their first versions.

use std::collections::BTreeMap;

use garde::Validate;
use serde::Deserialize;

use crate::contract::GateDefs;
use crate::data::attributes::{
    ATTRIBUTES_VERSION, ActionTables, AttributeDef, AttributeSchema, Group, Job,
};
use crate::data::tactics::{
    AiTuning, Duty, Formation, Instructions, Mentality, Offset, PreferredActions, Role,
    TacticsSchema, Teammates,
};
use crate::data::team::{Club, MAX_SQUAD, PlayerEntry, Position, TEAM_VERSION, TeamFile};
use crate::rating::Rating;

/// The team file version the converter reads.
pub const TEAM_V1: u32 = 1;
/// The tactics file version the converter reads.
pub const TACTICS_V1: u32 = 1;
/// The attribute file version the converter reads.
pub const ATTRIBUTES_V1: u32 = 1;
/// The tuning file versions the converters read.
pub const TUNING_V2: u32 = 2;
pub const TUNING_V3: u32 = 3;
/// The slot file version the converter reads.
pub const SLOTS_V1: u32 = 1;

/// The first contract tables of this build, as a version 1 attribute file and a version 2
/// tuning file convert with them. Never edited: tuning the shipped files leaves it alone.
pub const FROZEN_V1: &str = include_str!("../contract/frozen-v1.json");

/// The frozen copy's parts.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frozen {
    attributes: FrozenAttributes,
    tuning: FrozenTuning,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenAttributes {
    jobs: BTreeMap<String, Job>,
    actions: ActionTables,
    gates: GateDefs,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenTuning {
    /// Values merged into the `engine` block, each replacing the field of that name.
    engine: serde_json::Map<String, serde_json::Value>,
    /// Values merged into `engine.decision`.
    decision: serde_json::Map<String, serde_json::Value>,
    /// Values merged into the `generator` block.
    generator: serde_json::Map<String, serde_json::Value>,
}

fn frozen() -> Frozen {
    serde_json::from_str(FROZEN_V1).expect("the frozen contract copy parses; a test checks it")
}

/// One attribute in a version 1 attribute file: a name and a group.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeDefV1 {
    pub name: String,
    pub group: Group,
}

/// A version 1 attribute file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeSchemaV1 {
    pub schema_version: u32,
    pub attributes: Vec<AttributeDefV1>,
}

/// A version 1 attribute file in the current shape: each attribute takes its job, and the
/// file takes the stage tables and gates, from the frozen copy. A name the frozen copy has
/// no job for is refused, naming it. The caller validates the result as a current file.
pub fn attributes_v1_to_v2(file: AttributeSchemaV1) -> Result<AttributeSchema, String> {
    let mut frozen = frozen().attributes;
    let attributes = file
        .attributes
        .into_iter()
        .map(|a| match frozen.jobs.remove(&a.name) {
            Some(job) => Ok(AttributeDef {
                name: a.name,
                group: a.group,
                job,
            }),
            None => Err(format!(
                "attribute {} has no job in this build's first contract tables",
                a.name
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AttributeSchema {
        schema_version: ATTRIBUTES_VERSION,
        attributes,
        actions: frozen.actions,
        gates: frozen.gates,
    })
}

/// The engine fields a version 2 tuning file holds that version 3 drops: the linear speed
/// and acceleration, which the top-speed map and the sprint stage replace.
const TUNING_V2_DROPPED: [&str; 4] = ["base_speed", "pace_speed", "base_accel", "accel_bonus"];

/// A version 2 tuning file, as JSON, in the version 3 shape: the dropped speed fields and the
/// per-position distributions go; the contract block, the world spread, the foul weights
/// (which now act in log-odds per curve point), and the teamwork weight come from the frozen
/// copy. The caller parses and validates the result as a current file.
pub fn tuning_v2_to_v3(mut file: serde_json::Value) -> Result<serde_json::Value, String> {
    let frozen = frozen().tuning;
    let obj = file
        .as_object_mut()
        .ok_or_else(|| "the file is not an object".to_string())?;
    obj.insert("schema_version".into(), serde_json::Value::from(TUNING_V3));
    let engine = obj
        .get_mut("engine")
        .and_then(|e| e.as_object_mut())
        .ok_or_else(|| "engine is missing".to_string())?;
    for name in TUNING_V2_DROPPED {
        engine.remove(name);
    }
    for (k, v) in frozen.engine {
        engine.insert(k, v);
    }
    let decision = engine
        .get_mut("decision")
        .and_then(|d| d.as_object_mut())
        .ok_or_else(|| "engine.decision is missing".to_string())?;
    for (k, v) in frozen.decision {
        decision.insert(k, v);
    }
    let generator = obj
        .get_mut("generator")
        .and_then(|g| g.as_object_mut())
        .ok_or_else(|| "generator is missing".to_string())?;
    generator.remove("per_position");
    for (k, v) in frozen.generator {
        generator.insert(k, v);
    }
    Ok(file)
}

/// The state caps, body jobs, and fatigue group weights as they first shipped. Never edited:
/// tuning the shipped file leaves it alone.
pub const FROZEN_V2: &str = include_str!("../contract/frozen-v2.json");

/// The second frozen copy's parts.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenV2 {
    /// Values merged into `engine.contract`.
    contract: serde_json::Map<String, serde_json::Value>,
    /// Values merged into the `fatigue` block.
    fatigue: serde_json::Map<String, serde_json::Value>,
}

/// A version 3 tuning file, as JSON, in the version 4 shape: the state caps, the body jobs,
/// and the fatigue group weights come from the second frozen copy. The caller parses and
/// validates the result as a current file.
pub fn tuning_v3_to_v4(mut file: serde_json::Value) -> Result<serde_json::Value, String> {
    let frozen: FrozenV2 =
        serde_json::from_str(FROZEN_V2).expect("the second frozen copy parses; a test checks it");
    let obj = file
        .as_object_mut()
        .ok_or_else(|| "the file is not an object".to_string())?;
    obj.insert(
        "schema_version".into(),
        serde_json::Value::from(crate::data::TUNING_VERSION),
    );
    let contract = obj
        .get_mut("engine")
        .and_then(|e| e.get_mut("contract"))
        .and_then(|c| c.as_object_mut())
        .ok_or_else(|| "engine.contract is missing".to_string())?;
    for (k, v) in frozen.contract {
        contract.insert(k, v);
    }
    let fatigue = obj
        .get_mut("fatigue")
        .and_then(|f| f.as_object_mut())
        .ok_or_else(|| "fatigue is missing".to_string())?;
    for (k, v) in frozen.fatigue {
        fatigue.insert(k, v);
    }
    Ok(file)
}

/// The slots a version 2 slot file declares that version 1 did not, with the module and
/// version each takes when a version 1 file converts.
pub const SLOTS_V2_ADDED: [(&str, &str, u32); 2] = [
    ("engine.modifier.sharpness", "sharpness", 1),
    ("engine.modifier.adaptation", "adaptation", 1),
];

/// A version 1 slot file, as JSON, in the version 2 shape: the sharpness and adaptation
/// modifier slots take their first versions. The caller parses and resolves the result.
pub fn slots_v1_to_v2(mut file: serde_json::Value) -> Result<serde_json::Value, String> {
    let obj = file
        .as_object_mut()
        .ok_or_else(|| "the file is not an object".to_string())?;
    obj.insert(
        "schema_version".into(),
        serde_json::Value::from(crate::modules::SLOTS_VERSION),
    );
    let slots = obj
        .get_mut("slots")
        .and_then(|s| s.as_object_mut())
        .ok_or_else(|| "slots is missing".to_string())?;
    for (slot, module, version) in SLOTS_V2_ADDED {
        slots.insert(
            slot.into(),
            serde_json::json!({ "module": module, "version": version }),
        );
    }
    Ok(file)
}

/// One player in a version 1 team file: attributes are whole numbers 1 to 100.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PlayerEntryV1 {
    #[garde(length(min = 1, max = 64))]
    pub id: String,
    #[garde(length(min = 2, max = 48))]
    pub name: String,
    #[garde(range(min = 1, max = 99))]
    pub shirt: u8,
    #[garde(skip)]
    pub position: Position,
    #[garde(skip)]
    pub attributes: BTreeMap<String, u8>,
}

/// A version 1 team file.
#[derive(Debug, Clone, PartialEq, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[garde(context(AttributeSchema))]
pub struct TeamFileV1 {
    #[garde(skip)]
    pub schema_version: u32,
    #[garde(dive(()))]
    pub club: Club,
    #[garde(length(min = 11, max = MAX_SQUAD), dive(()), custom(check_squad_v1))]
    pub players: Vec<PlayerEntryV1>,
}

/// The version 1 squad check, with its messages as version 1 gave them.
fn check_squad_v1(players: &[PlayerEntryV1], schema: &AttributeSchema) -> garde::Result {
    for (i, p) in players.iter().enumerate() {
        if players[..i].iter().any(|q| q.id == p.id) {
            return Err(garde::Error::new(format!("player {} appears twice", p.id)));
        }
        if players[..i].iter().any(|q| q.shirt == p.shirt) {
            return Err(garde::Error::new(format!(
                "player {}: shirt {} is already worn",
                p.id, p.shirt
            )));
        }
        for def in &schema.attributes {
            match p.attributes.get(&def.name) {
                None => {
                    return Err(garde::Error::new(format!(
                        "player {}: attribute {} is missing",
                        p.id, def.name
                    )));
                }
                Some(&value) if !(1..=100).contains(&value) => {
                    return Err(garde::Error::new(format!(
                        "player {}: attribute {} is {value}; allowed 1 to 100",
                        p.id, def.name
                    )));
                }
                Some(_) => {}
            }
        }
        for name in p.attributes.keys() {
            if schema.index(name).is_none() {
                return Err(garde::Error::new(format!(
                    "player {}: attribute {name} is not in the schema",
                    p.id
                )));
            }
        }
    }
    Ok(())
}

/// A checked version 1 team file in the current shape: every value `v` becomes `2v`
/// tenths, at least 1.0, so 1 to 4 become 1.0; no player has body fields.
pub fn team_v1_to_v2(file: TeamFileV1) -> TeamFile {
    TeamFile {
        schema_version: TEAM_VERSION,
        club: file.club,
        players: file
            .players
            .into_iter()
            .map(|p| PlayerEntry {
                id: p.id,
                name: p.name,
                shirt: p.shirt,
                position: p.position,
                attributes: p
                    .attributes
                    .into_iter()
                    .map(|(name, v)| {
                        (
                            name,
                            Rating::from_tenths((2 * v).max(crate::rating::MIN_TENTHS)),
                        )
                    })
                    .collect(),
                height: None,
                age: None,
                nationality: None,
                condition: None,
            })
            .collect(),
    }
}

/// A role in a version 1 tactics file: the three offsets sit on the role itself.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleV1 {
    pub name: String,
    pub positions: Vec<Position>,
    pub attributes: BTreeMap<String, f64>,
    pub shoot: f64,
    pub dribble: f64,
    pub progress: f64,
}

/// A duty in a version 1 tactics file: no scale.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DutyV1 {
    pub name: String,
    pub depth: f64,
    pub risk: f64,
}

/// A version 1 tactics file. Everything but roles and duties has the current shape. It is
/// checked after conversion, as a current file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TacticsV1 {
    pub schema_version: u32,
    pub formations: Vec<Formation>,
    pub mentalities: Vec<Mentality>,
    pub instructions: Instructions,
    pub roles: Vec<RoleV1>,
    pub duties: Vec<DutyV1>,
    pub ai: AiTuning,
}

/// A version 1 tactics file in the current shape, with values that keep today's play: the
/// role's shoot, dribble, and progress offsets become its preferred actions, its positions
/// in and out of possession move nothing, teammates treat it as before, and every duty
/// scales its role by 1. The caller validates the result as a current file.
pub fn tactics_v1_to_v2(file: TacticsV1) -> TacticsSchema {
    TacticsSchema {
        schema_version: crate::data::tactics::TACTICS_VERSION,
        formations: file.formations,
        mentalities: file.mentalities,
        instructions: file.instructions,
        roles: file
            .roles
            .into_iter()
            .map(|r| Role {
                name: r.name,
                positions: r.positions,
                attributes: r.attributes,
                in_possession: Offset::default(),
                out_of_possession: Offset::default(),
                preferred_actions: PreferredActions {
                    shoot: r.shoot,
                    dribble: r.dribble,
                    progress: r.progress,
                },
                teammates: Teammates::default(),
            })
            .collect(),
        duties: file
            .duties
            .into_iter()
            .map(|d| Duty {
                name: d.name,
                depth: d.depth,
                risk: d.risk,
                scale: 1.0,
            })
            .collect(),
        ai: file.ai,
    }
}

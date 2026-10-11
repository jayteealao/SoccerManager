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
//!
//! A version 2 attribute file and a version 4 tuning file take the hidden values from a third
//! frozen copy (`contract/frozen-v3.json`): injury resistance becomes injury proneness, which
//! raises the injury chance as it rises, and consistency joins the file with its spread, its
//! words, and the match rating's weights. A version 1 team file converts each player's injury
//! resistance R to injury proneness 21.0 − R and gives him consistency 10.0.
//!
//! A version 3 attribute file takes the version 4 additions from a fourth frozen copy
//! (`contract/frozen-v4.json`): the new performance attributes and the personality values,
//! communication moved to the mental group, the new and changed stage tables, and a
//! consistency degree per action.
//!
//! A version 4 attribute file takes the version 5 changes from a fifth frozen copy
//! (`contract/frozen-v5.json`): the press stage led by positioning, the shape stage led by
//! teamwork with concentration and consistency, the new cover and recovery-run stages, work
//! rate's job moved to the recovery run, determination's job made an occasion, and the two new
//! consistency degrees.
//!
//! A version 1 or 2 team file converts to version 3 with values from related attributes
//! ([`team_v2_to_v3`]): each new performance attribute is the mean of the attributes it grows
//! from ([`V3_RELATED`]), an outfield player's communication is the mean of the old mental
//! group, every personality value is 10.0, his listed position is rated at his level and every
//! other at 1.0, and his weaker foot is 50 percent. The table is fixed in code and never edited,
//! so an old replay's team file converts the same way however the catalogue is tuned later.

use std::collections::BTreeMap;

use garde::Validate;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::contract::level::overall_level;
use crate::contract::{ActionKind, GateDefs, StageKind};
use crate::data::attributes::{
    ATTRIBUTES_VERSION, ActionTables, AttributeDef, AttributeSchema, Direction, Group, Job,
};
use crate::data::tactics::{
    AiTuning, Duty, Formation, Instructions, Mentality, Offset, PreferredActions, Role,
    TacticsSchema, Teammates,
};
use crate::data::team::{
    Club, Foot, MAX_SQUAD, PlayerEntry, Position, Side, TEAM_VERSION, TeamFile,
};
use crate::player::Attributes;
use crate::rating::{MAX_TENTHS, MIN_TENTHS, Rating};

/// The team file versions the converters read.
pub const TEAM_V1: u32 = 1;
pub const TEAM_V2: u32 = 2;
/// The tactics file version the converter reads.
pub const TACTICS_V1: u32 = 1;
/// The attribute file versions the converters read.
pub const ATTRIBUTES_V1: u32 = 1;
pub const ATTRIBUTES_V2: u32 = 2;
pub const ATTRIBUTES_V3: u32 = 3;
pub const ATTRIBUTES_V4: u32 = 4;
/// The tuning file versions the converters read.
pub const TUNING_V2: u32 = 2;
pub const TUNING_V3: u32 = 3;
pub const TUNING_V4: u32 = 4;

/// The attribute version 2 name that version 3 renames: injury resistance (high is good)
/// becomes injury proneness (high is bad), mirrored on the scale.
pub const RESISTANCE: &str = "injury_resistance";
/// The name injury resistance takes in version 3.
pub const PRONENESS: &str = "injury_proneness";
/// The hidden attribute version 3 adds.
pub const CONSISTENCY: &str = "consistency";
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

/// A version 1 attribute file in the version 2 shape: each attribute takes its job, and the
/// file takes the stage tables and gates, from the frozen copy. A name the frozen copy has
/// no job for is refused, naming it. The caller converts the result to version 3
/// ([`attributes_v2_to_v3`]) and validates it as a current file.
pub fn attributes_v1_to_v2(file: AttributeSchemaV1) -> Result<AttributeSchema, String> {
    let mut frozen = frozen().attributes;
    let attributes = file
        .attributes
        .into_iter()
        .map(|a| match frozen.jobs.remove(&a.name) {
            Some(job) => Ok(AttributeDef {
                name: a.name,
                group: a.group,
                hidden: false,
                job,
            }),
            None => Err(format!(
                "attribute {} has no job in this build's first contract tables",
                a.name
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AttributeSchema {
        schema_version: ATTRIBUTES_V2,
        attributes,
        actions: frozen.actions,
        consistency_degrees: BTreeMap::new(),
        gates: frozen.gates,
    })
}

/// The hidden values as they first shipped: the consistency definition, and the tuning blocks
/// a version 4 tuning file lacks. Never edited: tuning the shipped files leaves it alone.
pub const FROZEN_V3: &str = include_str!("../contract/frozen-v3.json");

/// The third frozen copy's parts.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenV3 {
    attributes: BTreeMap<String, AttributeDef>,
    tuning: FrozenV3Tuning,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenV3Tuning {
    /// Values merged into `engine.contract`.
    contract: serde_json::Map<String, serde_json::Value>,
    hidden: serde_json::Value,
    match_rating: serde_json::Value,
    /// Values merged into `generator.world`.
    world: serde_json::Map<String, serde_json::Value>,
}

fn frozen_v3() -> FrozenV3 {
    serde_json::from_str(FROZEN_V3).expect("the third frozen copy parses; a test checks it")
}

/// A version 2 attribute file in the current shape. Injury resistance becomes injury
/// proneness, a hidden value whose job raises injuries as it rises; the injury stage keeps it
/// as its main attribute and drops its supports, each of which lowered injuries. Consistency
/// joins from the frozen copy. The caller validates the result as a current file.
pub fn attributes_v2_to_v3(mut file: AttributeSchema) -> AttributeSchema {
    file.schema_version = ATTRIBUTES_V3;
    let mut renamed = false;
    for def in &mut file.attributes {
        if def.name == RESISTANCE {
            def.name = PRONENESS.to_string();
            def.hidden = true;
            def.job.direction = Direction::Up;
            renamed = true;
        }
    }
    if renamed {
        for stages in file.actions.values_mut() {
            for table in stages.values_mut() {
                for w in std::iter::once(&mut table.main).chain(table.supports.iter_mut()) {
                    if w.attribute == RESISTANCE {
                        w.attribute = PRONENESS.to_string();
                    }
                }
            }
        }
        if let Some(table) = file
            .actions
            .get_mut(&ActionKind::Injury)
            .and_then(|t| t.get_mut(&StageKind::Execute))
        {
            table.supports.clear();
        }
    }
    if file.index(CONSISTENCY).is_none()
        && let Some(def) = frozen_v3().attributes.remove(CONSISTENCY)
    {
        file.attributes.push(def);
    }
    file
}

/// The version 4 additions as they first shipped: the new definitions, the group moves, the
/// new and changed stage tables, and the consistency degrees. Never edited: tuning the shipped
/// file leaves it alone.
pub const FROZEN_V4: &str = include_str!("../contract/frozen-v4.json");

/// The attributes version 4 adds, in the order a converted file appends them: the new
/// performance attributes, then the personality values.
pub const V4_ADDED: [&str; 21] = [
    "marking",
    "off_ball_movement",
    "corners",
    "free_kicks",
    "penalties",
    "long_throws",
    "flair",
    "determination",
    "bravery",
    "eccentricity",
    "professionalism",
    "temperament",
    "pressure_handling",
    "sportsmanship",
    "leadership",
    "adaptability",
    "versatility",
    "ambition",
    "loyalty",
    "money_motivation",
    "controversy",
];

/// The fourth frozen copy's parts.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenV4 {
    attributes: BTreeMap<String, AttributeDef>,
    moved: BTreeMap<String, Group>,
    tables: ActionTables,
    consistency_degrees: BTreeMap<ActionKind, f64>,
}

/// A version 3 attribute file in the version 4 shape: each version 4 attribute it lacks is
/// appended in [`V4_ADDED`] order, communication moves to the mental group, each frozen table
/// replaces or adds its stage, and each missing consistency degree is added. The version 5
/// step follows ([`attributes_v4_to_v5`]); the caller validates the result as a current file.
pub fn attributes_v3_to_v4(mut file: AttributeSchema) -> AttributeSchema {
    let mut frozen: FrozenV4 =
        serde_json::from_str(FROZEN_V4).expect("the fourth frozen copy parses; a test checks it");
    file.schema_version = ATTRIBUTES_V4;
    for name in V4_ADDED {
        if file.index(name).is_none()
            && let Some(def) = frozen.attributes.remove(name)
        {
            file.attributes.push(def);
        }
    }
    for def in &mut file.attributes {
        if let Some(&group) = frozen.moved.get(&def.name) {
            def.group = group;
        }
    }
    for (action, stages) in frozen.tables {
        let tables = file.actions.entry(action).or_default();
        tables.extend(stages);
    }
    for (action, degree) in frozen.consistency_degrees {
        file.consistency_degrees.entry(action).or_insert(degree);
    }
    file
}

/// The version 5 changes as they first shipped: the re-weighted press and shape stages, the
/// cover and recovery-run stages, the jobs of work rate and determination, and the two new
/// consistency degrees. Never edited: tuning the shipped file leaves it alone.
pub const FROZEN_V5: &str = include_str!("../contract/frozen-v5.json");

/// The fifth frozen copy's parts.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenV5 {
    tables: ActionTables,
    jobs: BTreeMap<String, Job>,
    consistency_degrees: BTreeMap<ActionKind, f64>,
}

/// A version 4 attribute file in the version 5 shape: each frozen table replaces or adds its
/// stage, each frozen job replaces the job of an attribute the file holds, and each missing
/// consistency degree is added. The caller validates the result as a current file.
pub fn attributes_v4_to_v5(mut file: AttributeSchema) -> AttributeSchema {
    let frozen: FrozenV5 =
        serde_json::from_str(FROZEN_V5).expect("the fifth frozen copy parses; a test checks it");
    file.schema_version = ATTRIBUTES_VERSION;
    for (action, stages) in frozen.tables {
        file.actions.entry(action).or_default().extend(stages);
    }
    for def in &mut file.attributes {
        if let Some(job) = frozen.jobs.get(&def.name) {
            def.job = job.clone();
        }
    }
    for (action, degree) in frozen.consistency_degrees {
        file.consistency_degrees.entry(action).or_insert(degree);
    }
    file
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
    obj.insert("schema_version".into(), serde_json::Value::from(TUNING_V4));
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

/// A version 4 tuning file, as JSON, in the version 5 shape: consistency, the words of the
/// hidden values, the match rating, and the spread of the hidden values in the generator come
/// from the third frozen copy. The caller parses and validates the result as a current file.
pub fn tuning_v4_to_v5(mut file: serde_json::Value) -> Result<serde_json::Value, String> {
    let frozen = frozen_v3().tuning;
    let obj = file
        .as_object_mut()
        .ok_or_else(|| "the file is not an object".to_string())?;
    obj.insert(
        "schema_version".into(),
        serde_json::Value::from(crate::data::TUNING_VERSION),
    );
    obj.insert("hidden".into(), frozen.hidden);
    obj.insert("match_rating".into(), frozen.match_rating);
    let contract = obj
        .get_mut("engine")
        .and_then(|e| e.get_mut("contract"))
        .and_then(|c| c.as_object_mut())
        .ok_or_else(|| "engine.contract is missing".to_string())?;
    for (k, v) in frozen.contract {
        contract.insert(k, v);
    }
    let world = obj
        .get_mut("generator")
        .and_then(|g| g.get_mut("world"))
        .and_then(|w| w.as_object_mut())
        .ok_or_else(|| "generator.world is missing".to_string())?;
    for (k, v) in frozen.world {
        world.insert(k, v);
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
    obj.insert("schema_version".into(), serde_json::Value::from(SLOTS_V2));
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

/// The slot file version before the familiarity slot.
pub const SLOTS_V2: u32 = 2;

/// The slots a version 3 slot file declares that version 2 did not, with the module and
/// version each takes when a version 2 file converts.
pub const SLOTS_V3_ADDED: [(&str, &str, u32); 1] =
    [("engine.modifier.familiarity", "familiarity", 1)];

/// A version 2 slot file, as JSON, in the version 3 shape: the familiarity modifier slot
/// takes its first version. The caller parses and resolves the result.
pub fn slots_v2_to_v3(mut file: serde_json::Value) -> Result<serde_json::Value, String> {
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
    for (slot, module, version) in SLOTS_V3_ADDED {
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

/// The name `def` had in a version 1 team file: injury proneness was injury resistance, and
/// consistency and the version 4 attributes did not exist.
fn v1_name(def: &AttributeDef) -> Option<&str> {
    match def.name.as_str() {
        CONSISTENCY => None,
        name if V4_ADDED.contains(&name) => None,
        PRONENESS => Some(RESISTANCE),
        name => Some(name),
    }
}

/// The new performance attributes of a version 3 team file and the attributes each is
/// converted from: the mean of their ratings, rounded to a tenth. Never edited.
pub const V3_RELATED: [(&str, &[&str]); 10] = [
    ("marking", &["tackling", "positioning", "concentration"]),
    ("off_ball_movement", &["anticipation", "decisions"]),
    ("corners", &["crossing", "technique"]),
    ("free_kicks", &["crossing", "technique", "finishing"]),
    ("penalties", &["finishing", "composure", "technique"]),
    ("long_throws", &["strength", "technique"]),
    ("flair", &["dribbling", "technique", "vision"]),
    ("determination", &["work_rate", "composure"]),
    ("bravery", &["aggression", "heading", "strength"]),
    ("eccentricity", &["rushing_out", "aggression"]),
];

/// The mental group before version 4; a converted outfield player's communication is their
/// mean. Never edited.
pub const MENTAL_V3: [&str; 9] = [
    "positioning",
    "vision",
    "decisions",
    "composure",
    "anticipation",
    "work_rate",
    "aggression",
    "concentration",
    "teamwork",
];

/// The communication attribute, which moved from the goalkeeping group to the mental group.
pub const COMMUNICATION: &str = "communication";

/// The weaker foot of a converted player, in percent of the strong one.
pub const CONVERTED_WEAKER_FOOT: u8 = 50;

/// The mean of the ratings of `names` that `attributes` holds, rounded to a tenth (half up)
/// and kept to 1.0 to 20.0; `None` when it holds none of them.
fn mean_of(attributes: &BTreeMap<String, Rating>, names: &[&str]) -> Option<Rating> {
    let values: Vec<u32> = names
        .iter()
        .filter_map(|n| attributes.get(*n))
        .map(|r| u32::from(r.tenths()))
        .collect();
    if values.is_empty() {
        return None;
    }
    let n = values.len() as u32;
    let mean = (values.iter().sum::<u32>() + n / 2) / n;
    // The clamp keeps the mean inside 10..=200, so it fits a u8.
    let tenths = mean.clamp(u32::from(MIN_TENTHS), u32::from(MAX_TENTHS)) as u8;
    Some(Rating::from_tenths(tenths))
}

/// The strong side of a converted player, fixed by his id: left when the first byte of the
/// SHA-256 of the id is divisible by 4 (about one in four), else right.
pub fn strong_side_of(id: &str) -> Side {
    if Sha256::digest(id.as_bytes())[0] % 4 == 0 {
        Side::Left
    } else {
        Side::Right
    }
}

/// The position ratings of a player whose file gives none: his listed position at his level
/// (the overall level of his ratings there), every other position 1.0.
pub fn listed_positions_only(
    entry: &PlayerEntry,
    schema: &AttributeSchema,
) -> BTreeMap<Position, Rating> {
    let level = overall_level(
        &Attributes::from_entry(&entry.attributes, schema),
        schema,
        entry.position,
    )
    .clamp(MIN_TENTHS, MAX_TENTHS);
    Position::ALL
        .into_iter()
        .map(|code| {
            let tenths = if code == entry.position {
                level
            } else {
                MIN_TENTHS
            };
            (code, Rating::from_tenths(tenths))
        })
        .collect()
}

/// The feet of a player whose file gives none: the strong side from his id
/// ([`strong_side_of`]) and a weaker foot of 50 percent.
pub fn converted_foot(id: &str) -> Foot {
    Foot {
        strong: strong_side_of(id),
        weaker: CONVERTED_WEAKER_FOOT,
    }
}

/// A version 2 team file (or a version 1 file converted to version 2) in the version 3 shape.
/// For each player, in this order, filling only what is missing: each new performance
/// attribute from its related ones ([`V3_RELATED`]); an outfield player's communication from
/// the old mental group ([`MENTAL_V3`]), replacing his old value, while a keeper keeps his;
/// each personality value 10.0; his listed position at his level and every other at 1.0; a
/// weaker foot of 50 percent on the side his id fixes; no traits. The caller checks the result
/// as a current file.
pub fn team_v2_to_v3(mut file: TeamFile, schema: &AttributeSchema) -> TeamFile {
    file.schema_version = TEAM_VERSION;
    for p in &mut file.players {
        for (name, from) in V3_RELATED {
            if !p.attributes.contains_key(name)
                && let Some(value) = mean_of(&p.attributes, from)
            {
                p.attributes.insert(name.to_string(), value);
            }
        }
        if p.position != Position::GK
            && let Some(value) = mean_of(&p.attributes, &MENTAL_V3)
        {
            p.attributes.insert(COMMUNICATION.to_string(), value);
        }
        for def in schema.attributes.iter().filter(|d| d.is_personality()) {
            p.attributes
                .entry(def.name.clone())
                .or_insert(Rating::from_tenths(100));
        }
        if p.positions.is_none() {
            p.positions = Some(listed_positions_only(p, schema));
        }
        if p.foot.is_none() {
            p.foot = Some(converted_foot(&p.id));
        }
    }
    file
}

/// The version 1 squad check, with its messages as version 1 gave them, against the names the
/// schema had then.
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
        for name in schema.attributes.iter().filter_map(v1_name) {
            match p.attributes.get(name) {
                None => {
                    return Err(garde::Error::new(format!(
                        "player {}: attribute {name} is missing",
                        p.id
                    )));
                }
                Some(&value) if !(1..=100).contains(&value) => {
                    return Err(garde::Error::new(format!(
                        "player {}: attribute {name} is {value}; allowed 1 to 100",
                        p.id
                    )));
                }
                Some(_) => {}
            }
        }
        for name in p.attributes.keys() {
            if !schema.attributes.iter().any(|d| v1_name(d) == Some(name)) {
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
/// tenths, at least 1.0, so 1 to 4 become 1.0; injury resistance R becomes injury proneness
/// 21.0 − R, which keeps the order of players and stays inside 1.0 to 20.0; every player gets
/// consistency 10.0; no player has body fields. The caller converts the result on to version 3
/// ([`team_v2_to_v3`]).
pub fn team_v1_to_v2(file: TeamFileV1) -> TeamFile {
    TeamFile {
        schema_version: TEAM_V2,
        converted_from: None,
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
                        let t = (2 * v).max(crate::rating::MIN_TENTHS);
                        if name == RESISTANCE {
                            // 10..=200 mirrors onto 10..=200.
                            (PRONENESS.to_string(), Rating::from_tenths(210 - t))
                        } else {
                            (name, Rating::from_tenths(t))
                        }
                    })
                    .chain(std::iter::once((
                        CONSISTENCY.to_string(),
                        Rating::from_tenths(100),
                    )))
                    .collect(),
                positions: None,
                foot: None,
                traits: BTreeMap::new(),
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
                options: Default::default(),
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
        stands: Default::default(),
        stand_options: None,
        rest_defence: None,
    }
}

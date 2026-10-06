//! Older content versions and their converters. A version 1 team file or tactics file is
//! read in its own shape, checked as before, and lifted to the current shape on load, so
//! the shipped version 1 files and the inputs that old replays embed still play.

use std::collections::BTreeMap;

use garde::Validate;
use serde::Deserialize;

use crate::data::attributes::AttributeSchema;
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
/// tenths exactly, so 1 to 4 become 0.2 to 0.8 (the temporary floor only converted ratings
/// may use), and no player has body fields.
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
                    .map(|(name, v)| (name, Rating::from_tenths(2 * v)))
                    .collect(),
                height: None,
                age: None,
                nationality: None,
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

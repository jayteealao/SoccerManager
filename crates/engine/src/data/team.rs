//! The team data file: club identity, kit, and a squad with positions and attributes.
//! Validation runs with the attribute schema as context so a value of 120 is refused by
//! player and attribute name.

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::attributes::AttributeSchema;

/// Schema version this build reads.
pub const TEAM_VERSION: u32 = 1;

/// The ten position codes (product-owner choice, plan Q9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Position {
    GK,
    CB,
    LB,
    RB,
    DM,
    CM,
    AM,
    LW,
    RW,
    ST,
}

impl Position {
    /// Every code, in declaration order.
    pub const ALL: [Position; 10] = [
        Position::GK,
        Position::CB,
        Position::LB,
        Position::RB,
        Position::DM,
        Position::CM,
        Position::AM,
        Position::LW,
        Position::RW,
        Position::ST,
    ];

    /// The code as written in a data file.
    pub fn code(&self) -> &'static str {
        match self {
            Position::GK => "GK",
            Position::CB => "CB",
            Position::LB => "LB",
            Position::RB => "RB",
            Position::DM => "DM",
            Position::CM => "CM",
            Position::AM => "AM",
            Position::LW => "LW",
            Position::RW => "RW",
            Position::ST => "ST",
        }
    }
}

/// Two kit colours as `#RRGGBB`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Kit {
    #[garde(custom(hex_colour))]
    pub primary: String,
    #[garde(custom(hex_colour))]
    pub secondary: String,
}

/// The club a team file describes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Club {
    #[garde(length(min = 3, max = 64))]
    pub id: String,
    #[garde(length(min = 2, max = 48))]
    pub name: String,
    #[garde(length(min = 2, max = 4))]
    pub short_name: String,
    #[garde(dive)]
    pub kit: Kit,
}

/// One player in a team file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PlayerEntry {
    #[garde(length(min = 1, max = 64))]
    pub id: String,
    #[garde(length(min = 2, max = 48))]
    pub name: String,
    #[garde(range(min = 1, max = 99))]
    pub shirt: u8,
    #[garde(skip)]
    pub position: Position,
    /// Attribute name to value; the squad check compares it with the schema.
    #[garde(skip)]
    pub attributes: BTreeMap<String, u8>,
}

/// The team data file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[garde(context(AttributeSchema))]
pub struct TeamFile {
    #[garde(skip)]
    pub schema_version: u32,
    #[garde(dive(()))]
    pub club: Club,
    #[garde(length(min = 11, max = 40), dive(()), custom(check_squad))]
    pub players: Vec<PlayerEntry>,
}

fn hex_colour(value: &str, _ctx: &()) -> garde::Result {
    let ok = value.len() == 7
        && value.starts_with('#')
        && value[1..].chars().all(|c| c.is_ascii_hexdigit());
    if ok {
        Ok(())
    } else {
        Err(garde::Error::new(format!(
            "{value} is not a #RRGGBB colour"
        )))
    }
}

fn check_squad(players: &[PlayerEntry], schema: &AttributeSchema) -> garde::Result {
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

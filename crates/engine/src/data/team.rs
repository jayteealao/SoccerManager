//! The team data file: club identity, kit, and a squad with positions, ratings, and body
//! fields. Validation runs with the attribute schema as context so a rating of 20.5 is
//! refused by player and attribute name.
//!
//! Version 2 holds every rating in tenths of the 1 to 20 scale, written with one decimal,
//! and three body fields per player: height, age, and nationality. A version 1 file (whole
//! numbers 1 to 100, no body fields) converts on load; see [`crate::data::convert`].
//!
//! A player may also carry a `condition` block: his match sharpness, his adaptation to the
//! country, the days of rest before the match, and the matches he has played for the club.
//! These are per-match inputs; a file without the block re-serializes, and hashes, as before.

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::attributes::AttributeSchema;
use crate::pitch::Pitch;
use crate::rating::{MAX_TENTHS, MIN_TENTHS, Rating};

/// Schema version this build reads and writes. Version 1 files convert on load.
pub const TEAM_VERSION: u32 = 2;

/// Height in whole centimetres a version 2 file may give.
pub const HEIGHT_CM: std::ops::RangeInclusive<u8> = 150..=215;
/// Age in whole years a version 2 file may give.
pub const AGE_YEARS: std::ops::RangeInclusive<u8> = 15..=45;

/// The largest squad a team file may hold. The random streams size their dense key index
/// from it (`streams::table::SQUAD_MAX`), so the two can never disagree.
pub const MAX_SQUAD: usize = 40;

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

/// The club's home ground: its touchline (`length`) and goal line (`width`) in metres. A
/// team file that gives none plays on 105 by 68. The default is never written back, so a
/// file that gives it and one that gives none hash the same.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ground {
    pub length: f64,
    pub width: f64,
}

impl Default for Ground {
    fn default() -> Self {
        Self {
            length: Pitch::DEFAULT.length(),
            width: Pitch::DEFAULT.width(),
        }
    }
}

impl Ground {
    /// `true` for 105 by 68.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// The club a team file describes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
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
    /// The home ground; a match plays on the home team's.
    #[serde(default, skip_serializing_if = "Ground::is_default")]
    #[garde(custom(check_ground(&self.name)))]
    pub ground: Ground,
}

impl Club {
    /// The pitch of the club's ground. A validated file always has one the Laws allow.
    pub fn pitch(&self) -> Result<Pitch, crate::pitch::GroundError> {
        Pitch::new(self.ground.length, self.ground.width)
    }
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
    /// Attribute name to rating; the squad check compares it with the schema.
    #[garde(skip)]
    pub attributes: BTreeMap<String, Rating>,
    /// Height in whole centimetres, 150 to 215. Required in a version 2 file; a player
    /// converted from version 1 has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub height: Option<u8>,
    /// Age in whole years, 15 to 45. Required in a version 2 file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub age: Option<u8>,
    /// Nationality as three upper-case letters, for example `ENG`. Required in a version 2
    /// file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub nationality: Option<String>,
    /// The player's match condition, every field optional. Absent: fully sharp, fully
    /// adapted, rested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub condition: Option<Condition>,
}

/// A player's condition for one match. The season piece carries these between matches; until
/// then the team file or the match setup gives them. An absent field has no effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    /// Match sharpness, 0 to 100 percent: below 100 his technical ratings drop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sharpness: Option<u8>,
    /// Adaptation to the country he plays in, 0 to 100 percent: below 100 his mental
    /// ratings drop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adaptation: Option<u8>,
    /// Days of rest since his last match, 0 to 14: they set his energy at kick-off and, when
    /// short, raise his injury chance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rest_days: Option<u8>,
    /// Matches he has played for the club, 0 to 1000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matches_at_club: Option<u16>,
}

/// Sharpness and adaptation a file may give, in percent.
pub const CONDITION_PERCENT: std::ops::RangeInclusive<u8> = 0..=100;
/// Days of rest a file may give.
pub const REST_DAYS: std::ops::RangeInclusive<u8> = 0..=14;
/// Matches at the club a file may give.
pub const MATCHES_AT_CLUB: std::ops::RangeInclusive<u16> = 0..=1000;

/// The team data file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[garde(context(AttributeSchema))]
pub struct TeamFile {
    #[garde(skip)]
    pub schema_version: u32,
    #[garde(dive(()))]
    pub club: Club,
    #[garde(length(min = 11, max = MAX_SQUAD), dive(()), custom(check_squad))]
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

/// Refuses a ground outside the Laws, naming the club's ground, the value, and the limit.
fn check_ground(club: &str) -> impl FnOnce(&Ground, &()) -> garde::Result + '_ {
    move |ground, _| {
        Pitch::new(ground.length, ground.width)
            .map(|_| ())
            .map_err(|e| garde::Error::new(format!("{club}: {e}")))
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
                Some(&value) if !(MIN_TENTHS..=MAX_TENTHS).contains(&value.tenths()) => {
                    return Err(garde::Error::new(format!(
                        "player {}: attribute {} is {value}; allowed 1.0 to 20.0",
                        p.id, def.name
                    )));
                }
                Some(_) => {}
            }
        }
        check_body(p)?;
        check_condition(p)?;
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

/// Refuses a condition field out of its range, naming the player, the field, and the range.
fn check_condition(p: &PlayerEntry) -> garde::Result {
    let Some(c) = p.condition else {
        return Ok(());
    };
    let refuse = |field: &str, v: u16, range: &str| {
        Err(garde::Error::new(format!(
            "player {}: condition field {field} is {v}; allowed {range}",
            p.id
        )))
    };
    for (field, v) in [("sharpness", c.sharpness), ("adaptation", c.adaptation)] {
        if let Some(v) = v
            && !CONDITION_PERCENT.contains(&v)
        {
            return refuse(field, v.into(), "0 to 100");
        }
    }
    if let Some(v) = c.rest_days
        && !REST_DAYS.contains(&v)
    {
        return refuse("rest_days", v.into(), "0 to 14");
    }
    if let Some(v) = c.matches_at_club
        && !MATCHES_AT_CLUB.contains(&v)
    {
        return refuse("matches_at_club", v, "0 to 1000");
    }
    Ok(())
}

/// Refuses a missing or out-of-range body field, naming the player and the field.
fn check_body(p: &PlayerEntry) -> garde::Result {
    let missing = |field: &str| {
        Err(garde::Error::new(format!(
            "player {}: body field {field} is missing",
            p.id
        )))
    };
    match p.height {
        None => return missing("height"),
        Some(h) if !HEIGHT_CM.contains(&h) => {
            return Err(garde::Error::new(format!(
                "player {}: body field height is {h}; allowed 150 to 215 cm",
                p.id
            )));
        }
        Some(_) => {}
    }
    match p.age {
        None => return missing("age"),
        Some(a) if !AGE_YEARS.contains(&a) => {
            return Err(garde::Error::new(format!(
                "player {}: body field age is {a}; allowed 15 to 45 years",
                p.id
            )));
        }
        Some(_) => {}
    }
    match &p.nationality {
        None => return missing("nationality"),
        Some(n) if !(n.len() == 3 && n.bytes().all(|b| b.is_ascii_uppercase())) => {
            return Err(garde::Error::new(format!(
                "player {}: body field nationality is {n}; allowed three upper-case letters",
                p.id
            )));
        }
        Some(_) => {}
    }
    Ok(())
}

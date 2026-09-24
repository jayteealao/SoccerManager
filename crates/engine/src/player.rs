//! Player state, the attribute array, and the derived values the hot path reads.
//!
//! `Attributes` is a fixed array in schema order so `Player` stays `Copy` and no name lookup
//! happens during a tick. `Derived` is computed once at load from the required attributes;
//! fatigue lowers a player's effective values from that base as energy falls.

use std::collections::BTreeMap;

use crate::data::attributes::{AttributeSchema, MAX_ATTRIBUTES};
use crate::math::DVec2;
use crate::tuning::Tuning;

/// Attribute values on the 1 to 100 scale, in schema order.
#[derive(Debug, Clone, Copy)]
pub struct Attributes {
    pub values: [u8; MAX_ATTRIBUTES],
    pub len: u8,
}

impl Attributes {
    /// The values of a validated team-file entry, in schema order.
    pub fn from_entry(entry: &BTreeMap<String, u8>, schema: &AttributeSchema) -> Self {
        let mut values = [0u8; MAX_ATTRIBUTES];
        for (slot, def) in values.iter_mut().zip(&schema.attributes) {
            *slot = entry.get(&def.name).copied().unwrap_or(1);
        }
        Self {
            values,
            // The schema holds at most MAX_ATTRIBUTES (50) entries.
            len: schema.len() as u8,
        }
    }

    /// The value at schema index `index`.
    pub fn get(&self, index: usize) -> u8 {
        self.values[index]
    }

    /// The values in schema order.
    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        self.values[..usize::from(self.len)].iter().copied()
    }
}

/// Values the simulation reads every tick, computed once from the attributes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Derived {
    /// Maximum speed in metres per second.
    pub max_speed: f64,
    /// Maximum acceleration in metres per second squared.
    pub max_accel: f64,
    pub passing: f64,
    pub dribbling: f64,
    pub tackling: f64,
    pub positioning: f64,
    /// Aggression on a 0 to 1 scale; it raises the chance of a foul and of a card.
    pub aggression: f64,
    /// Skill values on the 1 to 100 scale the decision layer reads.
    pub finishing: f64,
    pub vision: f64,
    pub decisions: f64,
    pub composure: f64,
    /// Stamina, natural fitness, and injury resistance on a 0 to 1 scale.
    pub stamina: f64,
    pub natural_fitness: f64,
    pub injury_resistance: f64,
}

impl Derived {
    /// Derives from the required attributes of a validated schema.
    pub fn from_attributes(a: &Attributes, schema: &AttributeSchema, t: &Tuning) -> Self {
        let [
            pace,
            acceleration,
            passing,
            dribbling,
            tackling,
            positioning,
            aggression,
            finishing,
            vision,
            decisions,
            composure,
            stamina,
            natural_fitness,
            injury_resistance,
        ] = schema.required_indices().map(|i| f64::from(a.get(i)));
        Self {
            max_speed: t.base_speed + t.pace_speed * pace / 100.0,
            max_accel: t.base_accel + t.accel_bonus * acceleration / 100.0,
            passing,
            dribbling,
            tackling,
            positioning,
            aggression: aggression / 100.0,
            finishing,
            vision,
            decisions,
            composure,
            stamina: stamina / 100.0,
            natural_fitness: natural_fitness / 100.0,
            injury_resistance: injury_resistance / 100.0,
        }
    }
}

/// Whether a player takes part in play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    OnPitch,
    /// Sent off: the player stands at a fixed point beside the pitch until full time.
    SentOff,
    /// Injured: the player left play and stands beside the pitch until a substitute takes
    /// the place, or until full time.
    Injured,
}

/// One player on the pitch.
#[derive(Debug, Clone, Copy)]
pub struct Player {
    /// Roster index, 0 to 21.
    pub id: usize,
    /// Team index, 0 (home) or 1 (away).
    pub team: usize,
    /// Slot index inside the formation, 0 to 10.
    pub slot: usize,
    /// The player's place in the team file; a substitute takes the roster slot of the player
    /// it replaces and brings its own squad index.
    pub squad: usize,
    pub shirt: u8,
    pub attributes: Attributes,
    /// The effective values play reads: `base` lowered by fatigue.
    pub derived: Derived,
    /// The values the attributes give a fresh player.
    pub base: Derived,
    /// Energy from 1.0 (fresh) down to 0.0.
    pub energy: f64,
    pub pos: DVec2,
    pub vel: DVec2,
    /// Where steering drives the player this tick.
    pub target: DVec2,
    /// Unit vector of the last non-zero velocity.
    pub facing: DVec2,
    pub status: Status,
    /// Yellow cards shown to the player in this match.
    pub yellow: u8,
    /// The first tick the player may attempt a tackle again after a foul.
    pub foul_ready: u32,
}

impl Player {
    /// `true` while the player takes part in play.
    pub fn active(&self) -> bool {
        self.status == Status::OnPitch
    }

    /// Maximum speed in metres per second.
    pub fn max_speed(&self) -> f64 {
        self.derived.max_speed
    }

    /// Maximum acceleration in metres per second squared.
    pub fn max_accel(&self) -> f64 {
        self.derived.max_accel
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// A player with every attribute at `v`, for unit tests that need no schema.
    pub(crate) fn flat_player(id: usize, v: u8, t: &Tuning) -> Player {
        let attributes = Attributes {
            values: [v; MAX_ATTRIBUTES],
            len: 6,
        };
        let value = f64::from(v);
        let derived = Derived {
            max_speed: t.base_speed + t.pace_speed * value / 100.0,
            max_accel: t.base_accel + t.accel_bonus * value / 100.0,
            passing: value,
            dribbling: value,
            tackling: value,
            positioning: value,
            aggression: value / 100.0,
            finishing: value,
            vision: value,
            decisions: value,
            composure: value,
            stamina: value / 100.0,
            natural_fitness: value / 100.0,
            injury_resistance: value / 100.0,
        };
        Player {
            id,
            team: 0,
            slot: 0,
            squad: 0,
            shirt: 1,
            attributes,
            derived,
            base: derived,
            energy: 1.0,
            pos: DVec2::ZERO,
            vel: DVec2::ZERO,
            target: DVec2::ZERO,
            facing: DVec2::X,
            status: Status::OnPitch,
            yellow: 0,
            foul_ready: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::attributes::{ATTRIBUTES_VERSION, AttributeDef, Group, REQUIRED};

    fn schema() -> AttributeSchema {
        let mut names: Vec<String> = REQUIRED.iter().map(|s| s.to_string()).collect();
        for i in 0..16 {
            names.push(format!("attr_{i}"));
        }
        AttributeSchema {
            schema_version: ATTRIBUTES_VERSION,
            attributes: names
                .into_iter()
                .map(|name| AttributeDef {
                    name,
                    group: Group::Physical,
                })
                .collect(),
        }
    }

    #[test]
    fn derived_speed_scales_with_pace() {
        let t = Tuning::default();
        let s = schema();
        let mut entry: BTreeMap<String, u8> =
            s.attributes.iter().map(|a| (a.name.clone(), 1)).collect();
        let a = Attributes::from_entry(&entry, &s);
        assert_eq!(a.len, 30);
        let d = Derived::from_attributes(&a, &s, &t);
        assert_eq!(d.max_speed, t.base_speed + t.pace_speed * 0.01);
        entry.insert("pace".into(), 100);
        let d = Derived::from_attributes(&Attributes::from_entry(&entry, &s), &s, &t);
        assert_eq!(d.max_speed, t.base_speed + t.pace_speed);
    }
}

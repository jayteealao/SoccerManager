//! The tactics file: formations, mentalities, the six team instructions with their levels,
//! roles mapped to attributes, duties, and the thresholds the AI manager reads. Ranges are
//! `garde` rules; the checks that span fields or files run in [`TacticsSchema::check`], which
//! the loader calls with the attribute schema, so a role that names an unknown attribute is
//! refused naming the file, the role, and the attribute.

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::attributes::AttributeSchema;
use crate::data::team::Position;
use crate::team::PLAYERS_PER_TEAM;

/// Schema version this build reads.
pub const TACTICS_VERSION: u32 = 1;

/// The six team instructions, in the order a [`crate::tactics::Tactics`] stores their levels.
pub const INSTRUCTIONS: [&str; 6] = [
    "pressing",
    "width",
    "tempo",
    "line_height",
    "passing_directness",
    "time_wasting",
];

/// Index of each instruction in [`INSTRUCTIONS`].
pub const PRESSING: usize = 0;
pub const WIDTH: usize = 1;
pub const TEMPO: usize = 2;
pub const LINE_HEIGHT: usize = 3;
pub const DIRECTNESS: usize = 4;
pub const TIME_WASTING: usize = 5;

/// The tactics file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct TacticsSchema {
    #[garde(skip)]
    pub schema_version: u32,
    #[garde(length(min = 1, max = 16), dive)]
    pub formations: Vec<Formation>,
    /// From the most defensive to the most attacking; the AI manager steps along this order.
    #[garde(length(min = 1, max = 9), dive)]
    pub mentalities: Vec<Mentality>,
    #[garde(dive)]
    pub instructions: Instructions,
    #[garde(length(min = 1, max = 64), dive)]
    pub roles: Vec<Role>,
    #[garde(length(min = 1, max = 5), dive)]
    pub duties: Vec<Duty>,
    #[garde(dive)]
    pub ai: AiTuning,
}

/// Eleven slots in attack coordinates: `x` metres from the own goal line, `y` across.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Formation {
    #[garde(length(min = 1, max = 32))]
    pub name: String,
    #[garde(length(min = 11, max = 11), dive)]
    pub slots: Vec<FormationSlot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct FormationSlot {
    #[garde(range(min = 1.0, max = 100.0))]
    pub x: f64,
    #[garde(range(min = -33.0, max = 33.0))]
    pub y: f64,
    #[garde(skip)]
    pub position: Position,
}

/// A mentality: how far the whole block moves up (metres, negative drops it) and the offsets
/// it adds to the carrier's shoot, forward-pass, and hold scores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Mentality {
    #[garde(length(min = 1, max = 32))]
    pub name: String,
    #[garde(range(min = -20.0, max = 20.0))]
    pub block_depth: f64,
    #[garde(range(min = -2.0, max = 2.0))]
    pub shoot: f64,
    #[garde(range(min = -2.0, max = 2.0))]
    pub progress: f64,
    #[garde(range(min = -2.0, max = 2.0))]
    pub hold: f64,
}

/// The six instructions. Each has a default level and two to five levels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Instructions {
    /// Opponents who press the carrier, and a scale on the tuned press distance.
    #[garde(dive)]
    pub pressing: Instruction<PressLevel>,
    /// A scale on each slot's distance from the centre line of the pitch.
    #[garde(dive)]
    pub width: Instruction<Level>,
    /// Added to every pass score and taken from the dribble and hold scores.
    #[garde(dive)]
    pub tempo: Instruction<Level>,
    /// Metres the whole block moves up (negative drops it).
    #[garde(dive)]
    pub line_height: Instruction<Level>,
    /// Added to a pass score in proportion to its length.
    #[garde(dive)]
    pub passing_directness: Instruction<Level>,
    /// A factor on the team's restart delay while it leads.
    #[garde(dive)]
    pub time_wasting: Instruction<Level>,
}

impl Instructions {
    /// The level names of the instruction at `index` in [`INSTRUCTIONS`].
    pub fn level_names(&self, index: usize) -> Vec<&str> {
        match index {
            PRESSING => self.pressing.names(),
            WIDTH => self.width.names(),
            TEMPO => self.tempo.names(),
            LINE_HEIGHT => self.line_height.names(),
            DIRECTNESS => self.passing_directness.names(),
            _ => self.time_wasting.names(),
        }
    }

    /// The default level of every instruction, in [`INSTRUCTIONS`] order.
    pub fn defaults(&self) -> [u8; 6] {
        let mut out = [0u8; 6];
        for (i, slot) in out.iter_mut().enumerate() {
            let default = match i {
                PRESSING => &self.pressing.default,
                WIDTH => &self.width.default,
                TEMPO => &self.tempo.default,
                LINE_HEIGHT => &self.line_height.default,
                DIRECTNESS => &self.passing_directness.default,
                _ => &self.time_wasting.default,
            };
            // A checked schema names every default among at most five levels.
            *slot = self
                .level_names(i)
                .iter()
                .position(|n| n == default)
                .unwrap_or(0) as u8;
        }
        out
    }

    /// The number of levels of the instruction at `index`.
    pub fn level_count(&self, index: usize) -> usize {
        self.level_names(index).len()
    }
}

/// One instruction: its default level and its levels, in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Instruction<L: Validate<Context = ()>> {
    #[garde(length(min = 1, max = 32))]
    pub default: String,
    #[garde(length(min = 2, max = 5), dive)]
    pub levels: Vec<L>,
}

impl<L: Validate<Context = ()> + Named> Instruction<L> {
    fn names(&self) -> Vec<&str> {
        self.levels.iter().map(Named::name).collect()
    }
}

/// Anything with a name in the tactics file.
pub trait Named {
    fn name(&self) -> &str;
}

/// A level with one number; the instruction says what the number means.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Level {
    #[garde(length(min = 1, max = 32))]
    pub name: String,
    #[garde(range(min = -20.0, max = 20.0))]
    pub value: f64,
}

impl Named for Level {
    fn name(&self) -> &str {
        &self.name
    }
}

/// A pressing level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PressLevel {
    #[garde(length(min = 1, max = 32))]
    pub name: String,
    #[garde(range(min = 0, max = 4))]
    pub press_count: u8,
    #[garde(range(min = 0.1, max = 3.0))]
    pub press_distance_scale: f64,
}

impl Named for PressLevel {
    fn name(&self) -> &str {
        &self.name
    }
}

/// A role: the positions it suits, the attribute weights that measure how well a player
/// fits it, and the offsets it adds to the carrier's shoot, dribble, and forward-pass scores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Role {
    #[garde(length(min = 1, max = 32))]
    pub name: String,
    #[garde(length(min = 1, max = 10))]
    pub positions: Vec<Position>,
    /// Attribute name to weight, 0 to 10.
    #[garde(length(min = 1, max = 12), custom(weights_in_range))]
    pub attributes: BTreeMap<String, f64>,
    #[garde(range(min = -2.0, max = 2.0))]
    pub shoot: f64,
    #[garde(range(min = -2.0, max = 2.0))]
    pub dribble: f64,
    #[garde(range(min = -2.0, max = 2.0))]
    pub progress: f64,
}

/// A duty: metres the player's anchor moves up (negative drops it), and the risk offset it
/// adds to forward passes and dribbles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Duty {
    #[garde(length(min = 1, max = 32))]
    pub name: String,
    #[garde(range(min = -15.0, max = 15.0))]
    pub depth: f64,
    #[garde(range(min = -2.0, max = 2.0))]
    pub risk: f64,
}

/// What the AI manager reads: its default setup and its in-match thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct AiTuning {
    #[garde(length(min = 1, max = 32))]
    pub formation: String,
    #[garde(length(min = 1, max = 32))]
    pub mentality: String,
    #[garde(length(min = 1, max = 32))]
    pub duty: String,
    /// From this minute a trailing team raises its mentality one step.
    #[garde(range(min = 0, max = 200))]
    pub trailing_minute: u32,
    /// From this minute a leading team lowers its mentality one step and wastes time.
    #[garde(range(min = 0, max = 200))]
    pub leading_minute: u32,
    /// Simulated seconds between two checks; an injury triggers a check at once.
    #[garde(range(min = 1, max = 600))]
    pub check_interval_s: u32,
    /// A player below this energy is substituted for fatigue from `fatigue_from_minute`.
    #[garde(range(min = 0.0, max = 1.0))]
    pub fatigue_energy: f64,
    #[garde(range(min = 0, max = 200))]
    pub fatigue_from_minute: u32,
    /// Until this minute one substitution is kept back for an injury.
    #[garde(range(min = 0, max = 200))]
    pub keep_for_injury_until_minute: u32,
    /// Substitutes named before kick-off; one is a goalkeeper when the squad has one.
    #[garde(range(min = 0, max = 12))]
    pub bench_size: u8,
}

fn weights_in_range(weights: &BTreeMap<String, f64>, _ctx: &()) -> garde::Result {
    for (name, w) in weights {
        if !(0.0..=10.0).contains(w) {
            return Err(garde::Error::new(format!(
                "attribute {name}: weight {w}; allowed 0 to 10"
            )));
        }
    }
    Ok(())
}

/// A refusal from [`TacticsSchema::check`]: the field path and the reason.
pub type Refusal = (String, String);

impl TacticsSchema {
    /// The checks that span fields or the attribute schema: unique names, a goalkeeper in
    /// slot 0 and only there, a role for every position a formation uses, every default and
    /// AI name resolving, and every role attribute in the attribute schema.
    pub fn check(&self, attributes: &AttributeSchema) -> Result<(), Refusal> {
        unique(
            "formations",
            self.formations.iter().map(|f| f.name.as_str()),
        )?;
        unique(
            "mentalities",
            self.mentalities.iter().map(|m| m.name.as_str()),
        )?;
        unique("roles", self.roles.iter().map(|r| r.name.as_str()))?;
        unique("duties", self.duties.iter().map(|d| d.name.as_str()))?;
        for (i, name) in INSTRUCTIONS.iter().enumerate() {
            let names = self.instructions.level_names(i);
            unique(
                &format!("instructions.{name}.levels"),
                names.iter().copied(),
            )?;
            let default = match i {
                PRESSING => &self.instructions.pressing.default,
                WIDTH => &self.instructions.width.default,
                TEMPO => &self.instructions.tempo.default,
                LINE_HEIGHT => &self.instructions.line_height.default,
                DIRECTNESS => &self.instructions.passing_directness.default,
                _ => &self.instructions.time_wasting.default,
            };
            if !names.contains(&default.as_str()) {
                return Err((
                    format!("instructions.{name}.default"),
                    format!("{default} is not one of its levels"),
                ));
            }
        }
        for (f, formation) in self.formations.iter().enumerate() {
            for (slot, s) in formation.slots.iter().enumerate() {
                let keeper = s.position == Position::GK;
                if keeper != (slot == 0) {
                    return Err((
                        format!("formations[{f}].slots[{slot}].position"),
                        format!(
                            "formation {}: slot 0 must be the goalkeeper and only slot 0",
                            formation.name
                        ),
                    ));
                }
                if !self.roles.iter().any(|r| r.positions.contains(&s.position)) {
                    return Err((
                        format!("formations[{f}].slots[{slot}].position"),
                        format!("no role suits position {}", s.position.code()),
                    ));
                }
            }
        }
        for role in &self.roles {
            for name in role.attributes.keys() {
                if attributes.index(name).is_none() {
                    return Err((
                        format!("roles.{}.attributes", role.name),
                        format!(
                            "role {} names attribute {name}, which is not in the attribute schema",
                            role.name
                        ),
                    ));
                }
            }
        }
        if self.formation_index(&self.ai.formation).is_none() {
            return Err((
                "ai.formation".into(),
                format!("{} is not a formation", self.ai.formation),
            ));
        }
        if self.mentality_index(&self.ai.mentality).is_none() {
            return Err((
                "ai.mentality".into(),
                format!("{} is not a mentality", self.ai.mentality),
            ));
        }
        if self.duty_index(&self.ai.duty).is_none() {
            return Err(("ai.duty".into(), format!("{} is not a duty", self.ai.duty)));
        }
        Ok(())
    }

    pub fn formation_index(&self, name: &str) -> Option<usize> {
        self.formations.iter().position(|f| f.name == name)
    }

    pub fn mentality_index(&self, name: &str) -> Option<usize> {
        self.mentalities.iter().position(|m| m.name == name)
    }

    pub fn duty_index(&self, name: &str) -> Option<usize> {
        self.duties.iter().position(|d| d.name == name)
    }

    pub fn role_index(&self, name: &str) -> Option<usize> {
        self.roles.iter().position(|r| r.name == name)
    }

    /// The first role that suits `position`. A checked schema has one for every position its
    /// formations use; any other position falls back to the first role.
    pub fn default_role(&self, position: Position) -> usize {
        self.roles
            .iter()
            .position(|r| r.positions.contains(&position))
            .unwrap_or(0)
    }

    /// The formation slots as `(x, y)` pairs.
    pub fn slots(&self, formation: usize) -> [(f64, f64); PLAYERS_PER_TEAM] {
        let mut out = [(0.0, 0.0); PLAYERS_PER_TEAM];
        for (o, s) in out.iter_mut().zip(&self.formations[formation].slots) {
            *o = (s.x, s.y);
        }
        out
    }
}

fn unique<'a>(field: &str, names: impl Iterator<Item = &'a str>) -> Result<(), Refusal> {
    let names: Vec<&str> = names.collect();
    for (i, n) in names.iter().enumerate() {
        if names[..i].contains(n) {
            return Err((field.to_string(), format!("{n} appears twice")));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::data::test_support::{shipped_content, shipped_dir};
    use crate::data::{Content, TACTICS_FILE};

    #[test]
    fn the_shipped_file_loads_with_ten_formations_and_five_mentalities() {
        let content = shipped_content();
        let t = &content.tactics;
        let names: Vec<&str> = t.formations.iter().map(|f| f.name.as_str()).collect();
        // The four first formations keep their places, so saved lineups keep their shape.
        assert_eq!(
            names,
            [
                "4-4-2",
                "4-3-3",
                "4-2-3-1",
                "3-5-2",
                "4-1-4-1",
                "4-4-1-1",
                "4-1-2-1-2",
                "3-4-3",
                "5-3-2",
                "5-4-1"
            ]
        );
        assert_eq!(t.mentalities.len(), 5);
        assert_eq!(t.instructions.defaults(), [1, 1, 1, 1, 1, 0]);
        assert_eq!(t.slots(0), crate::team::FORMATION_442);
        assert_eq!(t.duties.len(), 3);
    }

    fn with_tactics(edit: impl FnOnce(&mut serde_json::Value)) -> crate::EngineError {
        let src = shipped_dir();
        let dir = std::env::temp_dir().join(format!(
            "engine-tactics-{}-{}",
            std::process::id(),
            rand_suffix()
        ));
        for rel in [
            crate::data::ATTRIBUTES_FILE,
            crate::data::TUNING_FILE,
            crate::data::RULES_FILE,
        ] {
            let to = dir.join(rel);
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(src.path(rel), &to).unwrap();
        }
        let text = std::fs::read_to_string(src.path(TACTICS_FILE)).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        edit(&mut value);
        std::fs::write(dir.join(TACTICS_FILE), value.to_string()).unwrap();
        let err = Content::load(&crate::ContentDir::at(&dir)).unwrap_err();
        std::fs::remove_dir_all(&dir).unwrap();
        err
    }

    fn rand_suffix() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        N.fetch_add(1, Ordering::Relaxed)
    }

    #[test]
    fn a_role_naming_an_unknown_attribute_is_refused_by_file_role_and_attribute() {
        let err = with_tactics(|v| {
            v["roles"][1]["attributes"]["telepathy"] = 1.0.into();
        });
        assert_eq!(
            err.to_string(),
            "content refused: tactics tactics.json: roles.central_defender.attributes: \
             role central_defender names attribute telepathy, which is not in the attribute schema"
        );
    }

    #[test]
    fn a_formation_without_a_goalkeeper_in_slot_zero_is_refused() {
        let err = with_tactics(|v| {
            v["formations"][2]["slots"][0]["position"] = "CB".into();
        });
        let text = err.to_string();
        assert!(
            text.contains("formations[2].slots[0].position")
                && text.contains("slot 0 must be the goalkeeper"),
            "{text}"
        );
    }
}

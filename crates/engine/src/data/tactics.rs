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

/// Schema version this build reads and writes. Version 1 and 2 files convert on load.
/// Version 3 makes tempo the patience of the circulation (five levels from very direct to
/// very patient), makes the mentality a risk appetite (a loss weight in place of the flat
/// shoot, progress and hold offsets), and adds the tactical primitives (`primitives`); a
/// version 2 file takes them from a copy compiled into the build
/// ([`crate::data::convert::tactics_v2_to_v3`]). Version 4 adds the primitives that own their
/// levels (`levels: "own"` with their `steps`) or their choices, whose level or choice the
/// tactics store as settings, and gives width and passing directness five levels; a version 3
/// file takes them from a copy compiled into the build
/// ([`crate::data::convert::tactics_v3_to_v4`]). Version 5 names the tempo levels by speed,
/// very fast to very slow, with the same values; a version 4 file takes the names
/// ([`crate::data::convert::tactics_v4_to_v5`]). Version 6 gives pressing and line height five
/// levels (an outer level each side of the three they had) and adds the out-of-possession
/// primitives, on/off primitives among them; a version 5 file takes them from a copy compiled
/// into the build ([`crate::data::convert::tactics_v5_to_v6`]).
pub const TACTICS_VERSION: u32 = 6;

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
    /// Where each position stands in and out of possession, as an offset from its formation
    /// slot. A file without it stands every position on its slot.
    #[serde(default)]
    #[garde(dive)]
    pub stands: Stands,
    /// The sizes of the stand options. A file without it takes no stand option.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(dive)]
    pub stand_options: Option<StandOptionSizes>,
    /// How the rest-defence guard holds players back. A file without it holds nobody back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(dive)]
    pub rest_defence: Option<RestDefence>,
    /// The tactical primitives: each setting of the tactics board, the plan inputs it sets,
    /// the hook that reads each, its signature statistics and its fit to real teams. A file
    /// without it declares none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[garde(dive)]
    pub primitives: Vec<crate::data::primitives::Primitive>,
}

/// Each position's stand per phase, as an offset from its formation slot: metres up the
/// pitch (`x`) and away from the centre line on the slot's side (`y`). The stand is never an
/// offset from the ball; the ball shift moves the whole block after it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields, default)]
pub struct Stands {
    #[garde(dive)]
    pub gk: PhaseStand,
    #[garde(dive)]
    pub cb: PhaseStand,
    #[garde(dive)]
    pub lb: PhaseStand,
    #[garde(dive)]
    pub rb: PhaseStand,
    #[garde(dive)]
    pub dm: PhaseStand,
    #[garde(dive)]
    pub cm: PhaseStand,
    #[garde(dive)]
    pub am: PhaseStand,
    #[garde(dive)]
    pub lw: PhaseStand,
    #[garde(dive)]
    pub rw: PhaseStand,
    #[garde(dive)]
    pub st: PhaseStand,
}

impl Stands {
    /// The stand of `position`.
    pub fn of(&self, position: Position) -> PhaseStand {
        match position {
            Position::GK => self.gk,
            Position::CB => self.cb,
            Position::LB => self.lb,
            Position::RB => self.rb,
            Position::DM => self.dm,
            Position::CM => self.cm,
            Position::AM => self.am,
            Position::LW => self.lw,
            Position::RW => self.rw,
            Position::ST => self.st,
        }
    }
}

/// One position's stand while its team has the ball and while the other team has it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields, default)]
pub struct PhaseStand {
    #[garde(dive)]
    pub in_possession: StandOffset,
    #[garde(dive)]
    pub out_of_possession: StandOffset,
}

/// A stand offset in metres, each part from -40 to 40.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StandOffset {
    #[garde(range(min = -40.0, max = 40.0))]
    pub x: f64,
    #[garde(range(min = -40.0, max = 40.0))]
    pub y: f64,
}

/// A stand option a role may take from its menu. None names a partner: a pairing such as a
/// full-back overlapping a winger follows from where the team-mates stand, whoever they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandOption {
    /// Come short toward the ball, to give the carrier a passing option.
    Drop,
    /// Move further from the centre line.
    Widen,
    /// Move into the channel between the centre and the wing.
    HalfSpace,
    /// Run round the outside of a team-mate ahead on the same side.
    Overlap,
    /// Run inside a team-mate ahead on the same side who holds the touchline.
    Underlap,
}

impl StandOption {
    pub fn name(self) -> &'static str {
        match self {
            StandOption::Drop => "drop",
            StandOption::Widen => "widen",
            StandOption::HalfSpace => "half_space",
            StandOption::Overlap => "overlap",
            StandOption::Underlap => "underlap",
        }
    }
}

/// The most options a menu may list for one phase.
pub const MENU_MAX: usize = 5;

/// A role's stand options per phase, in preference order.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields, default)]
pub struct OptionMenu {
    #[garde(length(max = MENU_MAX))]
    pub in_possession: Vec<StandOption>,
    #[garde(length(max = MENU_MAX))]
    pub out_of_possession: Vec<StandOption>,
}

/// The sizes the stand options read, in metres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StandOptionSizes {
    /// How far a dropping player comes short of his stand.
    #[garde(range(min = 0.0, max = 30.0))]
    pub drop_m: f64,
    /// How far behind a player the carrier must be for him to drop.
    #[garde(range(min = 0.0, max = 60.0))]
    pub drop_gap_m: f64,
    /// How far a widening player moves out.
    #[garde(range(min = 0.0, max = 20.0))]
    pub widen_m: f64,
    /// The distance of the half-space line from the centre line, drawn for a 68 m width.
    #[garde(range(min = 0.0, max = 40.0))]
    pub half_space_y: f64,
    /// How far ahead of the team-mate an overlapping player runs, and how far outside him.
    #[garde(range(min = 0.0, max = 30.0))]
    pub overlap_ahead_m: f64,
    #[garde(range(min = 0.0, max = 20.0))]
    pub overlap_out_m: f64,
    /// How far ahead of the team-mate an underlapping player runs, and how far inside him.
    #[garde(range(min = 0.0, max = 30.0))]
    pub underlap_ahead_m: f64,
    #[garde(range(min = 0.0, max = 20.0))]
    pub underlap_in_m: f64,
    /// The space a team-mate takes up: another player this close to a spot fills it.
    #[garde(range(min = 0.0, max = 20.0))]
    pub space_m: f64,
    /// How near the ball a team-mate must be for a player to overlap or underlap him.
    #[garde(range(min = 0.0, max = 40.0))]
    pub near_ball_m: f64,
}

/// How the rest-defence guard holds players back while the team has the ball.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct RestDefence {
    /// Metres goal-side of the ball a held player stands at the least.
    #[garde(range(min = 0.0, max = 20.0))]
    pub hold_margin_m: f64,
    /// Above the minimum, a support or defend duty goes forward only with a team-mate
    /// goal-side within this many metres across.
    #[garde(range(min = 0.0, max = 100.0))]
    pub lane_m: f64,
    /// The familiarity, in rating points, a team-mate needs to stay and cover a full-back
    /// who went forward.
    #[garde(range(min = -12.7, max = 12.7))]
    pub cover_familiarity_min: f64,
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

/// A mentality: the team's appetite for risk and nothing else. It sets how much a lost
/// ball's danger counts against an option's gain (the loss weight, -5 to 5; below 0 the side
/// seeks risk) and the rest-defence minimum. Where the block stands belongs to the line
/// height, not to the mentality.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Mentality {
    #[garde(length(min = 1, max = 32))]
    pub name: String,
    /// Each option of the carrier loses `loss_weight x failure chance x danger`; the danger
    /// of losing the ball at a place is read in `decision.rs` (hook `option.loss_danger`).
    #[garde(range(min = -5.0, max = 5.0))]
    pub loss_weight: f64,
    /// The fewest outfield players who stay goal-side of the ball while the team has it.
    #[serde(default)]
    #[garde(range(min = 0, max = 10))]
    pub rest_defence: u8,
}

/// The six instructions. Each has a default level and two to five levels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Instructions {
    /// Opponents who press the carrier, and a scale on the tuned press distance.
    #[garde(dive)]
    pub pressing: Instruction<PressLevel>,
    /// A scale on each slot's distance from the centre line of the pitch while the side has
    /// the ball, and the carrier's preference for a team-mate in a wide channel.
    #[garde(dive)]
    pub width: Instruction<WidthLevel>,
    /// The patience of the circulation: added to the score of a short pass that does not go
    /// forward, so a patient side recycles the ball before it plays forward and a direct side
    /// plays forward sooner.
    #[garde(dive)]
    pub tempo: Instruction<Level>,
    /// Metres the whole block moves up (negative drops it).
    #[garde(dive)]
    pub line_height: Instruction<Level>,
    /// Added to a pass score in proportion to its length.
    #[garde(dive)]
    pub passing_directness: Instruction<Level>,
    /// A factor on the team's drawn restart extra while it leads (on its fixed restart delay
    /// when the content has no restart wait table).
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

/// A width level: the scale on each slot's distance from the centre line while the side has
/// the ball (`value`), and the weight the carrier gives a pass to a team-mate in a wide channel
/// of the middle or final third (`wide_target`, 0 by default).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct WidthLevel {
    #[garde(length(min = 1, max = 32))]
    pub name: String,
    #[garde(range(min = -20.0, max = 20.0))]
    pub value: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[garde(range(min = -2.0, max = 2.0))]
    pub wide_target: f64,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

impl Named for WidthLevel {
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
    /// The willingness a presser of this level adds to his tackle decision (hook
    /// `tackle.decide`, its second input); 0 when absent.
    #[serde(default)]
    #[garde(range(min = -2.0, max = 2.0))]
    pub press_willingness: f64,
}

impl Named for PressLevel {
    fn name(&self) -> &str {
        &self.name
    }
}

/// A role: the positions it suits, the attribute weights that measure how well a player
/// fits it, and its three behaviours: where the player stands in and out of possession,
/// which actions the player prefers, and how teammates treat the player.
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
    /// Metres the player's anchor moves while the team has the ball: `x` up the pitch, `y`
    /// away from the centre line on the slot's side.
    #[garde(dive)]
    pub in_possession: Offset,
    /// Metres the anchor moves while the other team has the ball.
    #[garde(dive)]
    pub out_of_possession: Offset,
    #[garde(dive)]
    pub preferred_actions: PreferredActions,
    #[garde(dive)]
    pub teammates: Teammates,
    /// The stand options the player may take, per phase, in preference order.
    #[serde(default)]
    #[garde(dive)]
    pub options: OptionMenu,
}

/// An anchor offset in metres, each part from -15 to 15.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Offset {
    #[garde(range(min = -15.0, max = 15.0))]
    pub x: f64,
    #[garde(range(min = -15.0, max = 15.0))]
    pub y: f64,
}

/// The offsets a role adds to the carrier's shoot, dribble, and forward-pass scores, each
/// -2 to 2.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PreferredActions {
    #[garde(range(min = -2.0, max = 2.0))]
    pub shoot: f64,
    #[garde(range(min = -2.0, max = 2.0))]
    pub dribble: f64,
    #[garde(range(min = -2.0, max = 2.0))]
    pub progress: f64,
}

/// How teammates treat the player.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Teammates {
    /// Added to a teammate's pass to this player in proportion to the pass length, 0 to 2:
    /// a target forward draws long balls.
    #[garde(range(min = 0.0, max = 2.0))]
    pub long_ball_target: f64,
}

/// A duty: metres the player's anchor moves up (negative drops it), the risk offset it
/// adds to forward passes and dribbles, and the scale it puts on the role's preferred
/// actions (0.5 to 1.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Duty {
    #[garde(length(min = 1, max = 32))]
    pub name: String,
    #[garde(range(min = -15.0, max = 15.0))]
    pub depth: f64,
    #[garde(range(min = -2.0, max = 2.0))]
    pub risk: f64,
    #[garde(range(min = 0.5, max = 1.5))]
    pub scale: f64,
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
    /// The rating points a player listed at a slot's position gets over another player when
    /// the lineup is picked: another player starts there only when his rating at the position
    /// is higher by more than this. 0 when the file gives none.
    #[serde(default)]
    #[garde(range(min = 0.0, max = 10.0))]
    pub out_of_position_margin: f64,
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
            for (phase, menu) in [
                ("in_possession", &role.options.in_possession),
                ("out_of_possession", &role.options.out_of_possession),
            ] {
                if let Some(k) = (1..menu.len()).find(|&k| menu[..k].contains(&menu[k])) {
                    return Err((
                        format!("roles.{}.options.{phase}", role.name),
                        format!("{} appears twice", menu[k].name()),
                    ));
                }
            }
            for name in role.attributes.keys() {
                let Some(idx) = attributes.index(name) else {
                    return Err((
                        format!("roles.{}.attributes", role.name),
                        format!(
                            "role {} names attribute {name}, which is not in the attribute schema",
                            role.name
                        ),
                    ));
                };
                // A role's fit reaches the page as a number, and a hidden value never does.
                if attributes.attributes[idx].hidden {
                    return Err((
                        format!("roles.{}.attributes", role.name),
                        format!(
                            "role {} names attribute {name}, which is hidden; a role may name only visible attributes",
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
        crate::data::primitives::check(self)
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
        // Pressing, width, tempo, line height and passing directness have five levels, so their
        // middle level is 2.
        assert_eq!(t.instructions.defaults(), [2, 2, 2, 2, 2, 0]);
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
            crate::data::TRAITS_FILE,
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
    fn a_role_naming_a_hidden_attribute_is_refused() {
        let err = with_tactics(|v| {
            v["roles"][1]["attributes"]["injury_proneness"] = 1.0.into();
        });
        assert_eq!(
            err.to_string(),
            "content refused: tactics tactics.json: roles.central_defender.attributes: \
             role central_defender names attribute injury_proneness, which is hidden; a role may \
             name only visible attributes"
        );
    }

    /// Shape AC-6, content part: every role's menu holds only the five stand options, and no
    /// option names a partner; a menu entry that names one, or an option listed twice, is
    /// refused.
    #[test]
    fn menus_hold_only_stand_options_and_no_partner() {
        use super::StandOption;
        let content = shipped_content();
        let t = &content.tactics;
        let kinds = [
            StandOption::Drop,
            StandOption::Widen,
            StandOption::HalfSpace,
            StandOption::Overlap,
            StandOption::Underlap,
        ];
        let mut listed = 0;
        for role in &t.roles {
            for o in role
                .options
                .in_possession
                .iter()
                .chain(&role.options.out_of_possession)
            {
                assert!(kinds.contains(o), "role {}", role.name);
                listed += 1;
            }
        }
        assert!(listed > 0);
        assert_eq!(
            t.mentalities
                .iter()
                .map(|m| m.rest_defence)
                .collect::<Vec<_>>(),
            [6, 5, 4, 3, 2]
        );
        assert!(t.stand_options.is_some() && t.rest_defence.is_some());
        // A menu entry naming a partner does not load.
        let err = with_tactics(|v| {
            v["roles"][3]["options"]["in_possession"][0] = serde_json::json!({"overlap": "winger"});
        });
        assert!(err.to_string().contains("tactics"), "{err}");
        let err = with_tactics(|v| {
            v["roles"][3]["options"]["partner"] = "winger".into();
        });
        assert!(err.to_string().contains("tactics"), "{err}");
        // An option listed twice is refused by role and phase.
        let err = with_tactics(|v| {
            v["roles"][3]["options"]["in_possession"] = serde_json::json!(["widen", "widen"]);
        });
        assert_eq!(
            err.to_string(),
            "content refused: tactics tactics.json: roles.full_back.options.in_possession: \
             widen appears twice"
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

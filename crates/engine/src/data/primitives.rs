//! The tactical primitives of the tactics file: each setting a manager can choose on the
//! tactics board, as data. A primitive is graded (5 ordered levels), a choice (a list of
//! choices) or on/off (two states). It sets one or more plan inputs, each read at one hook of
//! the hook list ([`crate::tactics::hooks::HOOKS`]); it moves a signature statistic in a
//! stated direction; and its levels are fitted to the spread of that statistic across real
//! teams, or it names why no real data fits it.
//!
//! The levels of a graded primitive live in a level block of the tactics file (an
//! instruction's levels, or the mentality list), so the tactics a match stores keep their
//! shape: a level is an index into its block. A graded primitive may instead own its levels
//! (`levels: "own"`, with its five `steps` and each step's input values), as a choice
//! primitive owns its choices; the tactics then store its level or choice in
//! [`crate::tactics::Tactics::settings`], one byte per owning primitive in the file's order.
//! [`check`] refuses a bad entry by name.

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::tactics::{INSTRUCTIONS, Refusal, TacticsSchema};
use crate::tactics::hooks;

/// The number of levels of a graded primitive.
pub const GRADED_LEVELS: usize = 5;

/// The statistics a primitive can name as its signature. The real-team table
/// (`signatures.json`) and the engine measure each one by the same definition.
pub const SIGNATURE_STATS: [&str; 51] = [
    "passes_per_possession",
    "recycle_share",
    "turnovers_per_possession",
    "shot_share",
    "block_height_m",
    "hold_dribble_share",
    "ppda",
    "forward_pass_share",
    "long_ball_share",
    "own_third_long_share",
    "goal_kick_m",
    "keeper_long_share",
    "wide_pass_share",
    "entries_left_share",
    "entries_right_share",
    "entries_centre_share",
    "entries_wide_share",
    "entries_centre_left_share",
    "entries_centre_right_share",
    "outside_box_shot_share",
    "shot_distance_m",
    "takeons_per_90",
    "crosses_per_final_third_possession",
    "high_cross_share",
    "low_cross_share",
    "cross_origin_m",
    "off_plan_share",
    "runs_in_behind_share",
    "coming_short_share",
    "fullback_runs_per_match",
    "overlaps_per_match",
    "underlaps_per_match",
    "counter_attacks_per_match",
    "regain_to_shot_s",
    "counterpress_pressures_per_loss",
    "counterpress_regain_share",
    "last_counterpress_s",
    "def_action_height_m",
    "defender_action_height_m",
    "oop_width_m",
    "opp_entries_wide_share",
    "offsides_caught_per_match",
    "opp_goal_kick_m",
    "opp_short_goal_kick_share",
    "opp_short_goal_kick_kept_share",
    "fouls_per_match",
    "yellows_per_match",
    "pressures_after_back_pass",
    "pressures_after_poor_touch",
    "pressures_after_facing_back",
    "pressures_after_wide",
];

/// The real data sources a fit may read. `pff_wc2022` is national-team data (the 2022 World
/// Cup's tracking and events), not a held-out set.
pub const SOURCES: [&str; 5] = [
    "wyscout_2017_18",
    "statsbomb_open",
    "skillcorner_open",
    "dfl_open",
    "pff_wc2022",
];

/// The held-out sets, by source and competition: no fit reads them, and an entry that names
/// one is refused.
pub const HELD_OUT: [(&str, &str); 3] = [
    ("wyscout_2017_18", "Germany"),
    ("wyscout_2017_18", "World_Cup"),
    ("wyscout_2017_18", "European_Championship"),
];

/// `true` when `competition` of `source` is a held-out set.
pub fn held_out(source: &str, competition: &str) -> bool {
    HELD_OUT
        .iter()
        .any(|(s, c)| *s == source && c.eq_ignore_ascii_case(competition))
}

/// The level block of a primitive that owns its levels (its `steps`) or its choices.
pub const OWN: &str = "own";

/// The most primitives that own their levels or choices: the bytes of
/// [`crate::tactics::Tactics::settings`].
pub const SETTINGS_MAX: usize = 32;

/// Where each plan input of a primitive is written: the block (a level block of the tactics
/// file, or [`OWN`] for an input the primitive's own steps or choices hold), the field of each
/// level, and the range the field allows.
pub const INPUT_FIELDS: [(&str, &str, &str, f64, f64); 40] = [
    ("patience", "instructions.tempo", "value", -2.0, 2.0),
    ("loss_weight", "mentalities", "loss_weight", -5.0, 5.0),
    ("rest_defence", "mentalities", "rest_defence", 0.0, 10.0),
    ("width", "instructions.width", "value", 0.5, 1.5),
    (
        "wide_target",
        "instructions.width",
        "wide_target",
        -2.0,
        2.0,
    ),
    (
        "directness",
        "instructions.passing_directness",
        "value",
        -1.0,
        1.0,
    ),
    ("build_up", OWN, "build_up", -2.0, 2.0),
    ("channel_left", OWN, "channel_left", -2.0, 2.0),
    ("channel_centre", OWN, "channel_centre", -2.0, 2.0),
    ("channel_right", OWN, "channel_right", -2.0, 2.0),
    ("long_shot", OWN, "long_shot", -2.0, 2.0),
    ("shot_reach_m", OWN, "shot_reach_m", 0.0, 15.0),
    ("dribble", OWN, "dribble", -2.0, 2.0),
    ("crossing", OWN, "crossing", -2.0, 2.0),
    ("early_cross", OWN, "early_cross", -2.0, 2.0),
    ("cross_loft", OWN, "cross_loft", -1.0, 1.0),
    ("freedom", OWN, "freedom", 0.0, 3.0),
    ("run_start_m", OWN, "run_start_m", -30.0, 30.0),
    ("fullback_run", OWN, "fullback_run", -1.0, 1.0),
    ("fullback_reach", OWN, "fullback_reach", 0.25, 3.0),
    ("counter", OWN, "counter", -2.0, 2.0),
    ("pace_lift", OWN, "pace_lift", 0.0, 1.0),
    ("counter_press", OWN, "counter_press", -1.0, 3.0),
    ("counter_press_s", OWN, "counter_press_s", 1.0, 12.0),
    (
        "press_distance",
        "instructions.pressing",
        "press_distance_scale",
        0.1,
        3.0,
    ),
    (
        "press_count",
        "instructions.pressing",
        "press_count",
        0.0,
        4.0,
    ),
    (
        "press_willingness",
        "instructions.pressing",
        "press_willingness",
        -2.0,
        2.0,
    ),
    (
        "block_depth",
        "instructions.line_height",
        "value",
        -20.0,
        20.0,
    ),
    ("engage_depth_m", OWN, "engage_depth_m", -20.0, 20.0),
    ("engage_line", OWN, "engage_line", 0.2, 1.0),
    ("defensive_width", OWN, "defensive_width", 0.5, 1.5),
    ("force", OWN, "force", -1.0, 1.0),
    ("tackle_willingness", OWN, "tackle_willingness", -1.0, 1.0),
    ("tackle_risk", OWN, "tackle_risk", -2.0, 2.0),
    ("offside_trap", OWN, "offside_trap", 0.0, 1.0),
    ("screen_short", OWN, "screen_short", 0.0, 1.0),
    ("trigger_back_pass", OWN, "trigger_back_pass", 0.0, 1.0),
    ("trigger_poor_touch", OWN, "trigger_poor_touch", 0.0, 1.0),
    ("trigger_facing_back", OWN, "trigger_facing_back", 0.0, 1.0),
    ("trigger_wide", OWN, "trigger_wide", 0.0, 1.0),
];

/// The block, field and range of plan input `input`.
pub fn input_field(input: &str) -> Option<(&'static str, &'static str, f64, f64)> {
    INPUT_FIELDS
        .iter()
        .find(|(i, ..)| *i == input)
        .map(|&(_, block, field, lo, hi)| (block, field, lo, hi))
}

/// The kind of a primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveKind {
    Graded,
    Choice,
    OnOff,
}

/// The way a signature statistic moves as the level rises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Up,
    Down,
}

impl Direction {
    /// +1 for up, -1 for down.
    pub fn sign(self) -> f64 {
        match self {
            Direction::Up => 1.0,
            Direction::Down => -1.0,
        }
    }
}

/// A plan input a primitive sets, and the hook that reads it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct InputHook {
    #[garde(length(min = 1, max = 32))]
    pub input: String,
    #[garde(length(min = 1, max = 48))]
    pub hook: String,
}

/// The steps of a graded primitive a signature statistic judges: every step, or only those
/// below or above the default level (a primitive whose lower levels do one thing and whose
/// upper levels another, each with its own statistic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Steps {
    #[default]
    All,
    BelowDefault,
    AboveDefault,
}

impl Steps {
    fn is_all(&self) -> bool {
        *self == Steps::All
    }

    /// `true` when the step from level `from` one level up lies in this scope, the default
    /// being level `default`.
    pub fn covers_step(self, from: usize, default: usize) -> bool {
        match self {
            Steps::All => true,
            Steps::BelowDefault => from < default,
            Steps::AboveDefault => from >= default,
        }
    }

    /// `true` when level `level` lies in this scope, the default being level `default`.
    pub fn covers_level(self, level: usize, default: usize) -> bool {
        match self {
            Steps::All => true,
            Steps::BelowDefault => level < default,
            Steps::AboveDefault => level > default,
        }
    }
}

/// A signature statistic and its direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Signature {
    #[garde(length(min = 1, max = 48))]
    pub stat: String,
    #[garde(skip)]
    pub direction: Direction,
    /// The steps this statistic judges; every step when absent.
    #[serde(default, skip_serializing_if = "Steps::is_all")]
    #[garde(skip)]
    pub steps: Steps,
}

/// One choice of a choice primitive: its name, the value it gives each of the primitive's
/// inputs, and the statistic whose share it raises.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    #[garde(length(min = 1, max = 32))]
    pub name: String,
    #[garde(skip)]
    pub values: BTreeMap<String, f64>,
    #[garde(length(min = 1, max = 48))]
    pub stat: String,
}

/// One level of a graded primitive that owns its levels: its name and the value it gives
/// each of the primitive's inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Step {
    #[garde(length(min = 1, max = 32))]
    pub name: String,
    #[garde(skip)]
    pub values: BTreeMap<String, f64>,
}

/// The real data a primitive's levels are fitted to, or why none fits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct DataSource {
    /// One of [`SOURCES`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub source: Option<String>,
    /// The competitions of the source the fit reads.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[garde(skip)]
    pub competitions: Vec<String>,
    /// The row of `signatures.json` that holds the real spread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub row: Option<String>,
    /// Why no real data fits the primitive; then no source is named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub none: Option<String>,
    /// What of a primitive with a data source is not fitted or judged yet, and why: its
    /// levels or choices keep their seeded values, and its checks do not gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub deferred: Option<String>,
    /// The levels the `deferred` reason covers, when it covers only some: those keep their
    /// seeded values and the other levels but the default hold fits. Empty: the whole
    /// primitive is deferred.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[garde(skip)]
    pub deferred_levels: Vec<String>,
}

/// The fit of one level: the plan input values it found and what the search measured.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Fit {
    #[garde(length(min = 1, max = 32))]
    pub level: String,
    /// The fitted value of each plan input of the level.
    #[garde(skip)]
    pub values: BTreeMap<String, f64>,
    /// The primary statistic: the real quintile median, the engine's mean at the fitted
    /// value, and the paired noise (1.96 standard errors).
    #[garde(skip)]
    pub target: f64,
    #[garde(skip)]
    pub achieved: f64,
    #[garde(skip)]
    pub noise: f64,
    #[garde(range(min = 1, max = 100_000))]
    pub matches: u32,
    #[garde(skip)]
    pub seed: u64,
    /// `true` when the target lies outside what the input's range reaches; the value is then
    /// the probe nearest the target (a range end, or a value between the neighbours' values
    /// when the search stayed between them).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[garde(skip)]
    pub unreachable: bool,
}

/// One primitive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Primitive {
    #[garde(length(min = 1, max = 32))]
    pub name: String,
    #[garde(skip)]
    pub kind: PrimitiveKind,
    /// A graded primitive's level block: `instructions.<instruction>`, `mentalities`, or
    /// [`OWN`] for its own `steps`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub levels: Option<String>,
    /// The levels of a graded primitive that owns them, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[garde(dive)]
    pub steps: Vec<Step>,
    /// A choice primitive's choices.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[garde(dive)]
    pub choices: Vec<Choice>,
    /// An on/off primitive's two states, off first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[garde(skip)]
    pub states: Vec<String>,
    #[garde(length(min = 1, max = 32))]
    pub default: String,
    #[garde(dive)]
    pub inputs: Vec<InputHook>,
    #[garde(dive)]
    pub signature: Vec<Signature>,
    #[garde(dive)]
    pub data: DataSource,
    /// The fit of each level, when the levels were fitted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[garde(dive)]
    pub fits: Vec<Fit>,
}

impl Primitive {
    /// The names of the primitive's levels, choices or states, in order.
    pub fn level_names<'a>(&'a self, schema: &'a TacticsSchema) -> Vec<&'a str> {
        if self.owns_levels() {
            return self.steps.iter().map(|s| s.name.as_str()).collect();
        }
        match self.kind {
            PrimitiveKind::Graded => self
                .levels
                .as_deref()
                .map(|block| block_names(schema, block))
                .unwrap_or_default(),
            PrimitiveKind::Choice => self.choices.iter().map(|c| c.name.as_str()).collect(),
            PrimitiveKind::OnOff => self.states.iter().map(String::as_str).collect(),
        }
    }

    /// The index of the default among [`Primitive::level_names`].
    pub fn default_index(&self, schema: &TacticsSchema) -> Option<usize> {
        self.level_names(schema)
            .iter()
            .position(|n| *n == self.default)
    }

    /// `true` for a graded primitive whose levels are its own `steps`, and for an on/off
    /// primitive whose two states are its own `steps` (off first).
    pub fn owns_levels(&self) -> bool {
        matches!(self.kind, PrimitiveKind::Graded | PrimitiveKind::OnOff)
            && self.levels.as_deref() == Some(OWN)
    }

    /// The number of steps a primitive that owns its levels lists: five for a graded one, two
    /// for an on/off one.
    pub fn step_count(&self) -> usize {
        if self.kind == PrimitiveKind::OnOff {
            2
        } else {
            GRADED_LEVELS
        }
    }

    /// `true` when the tactics store this primitive's level, choice or state in
    /// [`crate::tactics::Tactics::settings`]: it owns its levels or states, or it is a choice.
    pub fn is_setting(&self) -> bool {
        self.owns_levels() || self.kind == PrimitiveKind::Choice
    }

    /// The value level or choice `level` gives plan input `input`, when the primitive owns
    /// it. A choice that does not name an input leaves it at its neutral value: `None`.
    pub fn own_value(&self, input: &str, level: usize) -> Option<f64> {
        if self.owns_levels() {
            self.steps.get(level)?.values.get(input).copied()
        } else if self.kind == PrimitiveKind::Choice {
            self.choices.get(level)?.values.get(input).copied()
        } else {
            None
        }
    }
}

/// The primitives whose level or choice the tactics store in
/// [`crate::tactics::Tactics::settings`], in the file's order: the settings' byte order.
pub fn settings(schema: &TacticsSchema) -> impl Iterator<Item = &Primitive> {
    schema.primitives.iter().filter(|p| p.is_setting())
}

/// The primitive that sets plan input `input`.
fn owner<'a>(schema: &'a TacticsSchema, input: &str) -> Option<&'a Primitive> {
    schema
        .primitives
        .iter()
        .find(|p| p.inputs.iter().any(|i| i.input == input))
}

/// The level names of block `block`, or none when the tactics file has no such block.
pub fn block_names<'a>(schema: &'a TacticsSchema, block: &str) -> Vec<&'a str> {
    if block == "mentalities" {
        return schema.mentalities.iter().map(|m| m.name.as_str()).collect();
    }
    block
        .strip_prefix("instructions.")
        .and_then(|name| INSTRUCTIONS.iter().position(|i| *i == name))
        .map(|index| schema.instructions.level_names(index))
        .unwrap_or_default()
}

/// The value of plan input `input` at level `level` of its block, or of the steps or choices
/// of the primitive that owns it.
pub fn level_value(schema: &TacticsSchema, input: &str, level: usize) -> Option<f64> {
    match input {
        "patience" => schema.instructions.tempo.levels.get(level).map(|l| l.value),
        "width" => schema.instructions.width.levels.get(level).map(|l| l.value),
        "wide_target" => schema
            .instructions
            .width
            .levels
            .get(level)
            .map(|l| l.wide_target),
        "directness" => schema
            .instructions
            .passing_directness
            .levels
            .get(level)
            .map(|l| l.value),
        "press_distance" => schema
            .instructions
            .pressing
            .levels
            .get(level)
            .map(|l| l.press_distance_scale),
        "press_count" => schema
            .instructions
            .pressing
            .levels
            .get(level)
            .map(|l| f64::from(l.press_count)),
        "press_willingness" => schema
            .instructions
            .pressing
            .levels
            .get(level)
            .map(|l| l.press_willingness),
        "block_depth" => schema
            .instructions
            .line_height
            .levels
            .get(level)
            .map(|l| l.value),
        "loss_weight" => schema.mentalities.get(level).map(|m| m.loss_weight),
        "rest_defence" => schema
            .mentalities
            .get(level)
            .map(|m| f64::from(m.rest_defence)),
        _ => owner(schema, input)?.own_value(input, level),
    }
}

/// Sets plan input `input` at level `level` of its block, or of its primitive's steps or
/// choices, to `value`; `false` when the input or the level does not exist.
pub fn set_level_value(schema: &mut TacticsSchema, input: &str, level: usize, value: f64) -> bool {
    match input {
        "patience" => schema
            .instructions
            .tempo
            .levels
            .get_mut(level)
            .map(|l| l.value = value)
            .is_some(),
        "width" => schema
            .instructions
            .width
            .levels
            .get_mut(level)
            .map(|l| l.value = value)
            .is_some(),
        "wide_target" => schema
            .instructions
            .width
            .levels
            .get_mut(level)
            .map(|l| l.wide_target = value)
            .is_some(),
        "directness" => schema
            .instructions
            .passing_directness
            .levels
            .get_mut(level)
            .map(|l| l.value = value)
            .is_some(),
        "press_distance" => schema
            .instructions
            .pressing
            .levels
            .get_mut(level)
            .map(|l| l.press_distance_scale = value)
            .is_some(),
        // A count: the nearest whole number inside its range.
        "press_count" => schema
            .instructions
            .pressing
            .levels
            .get_mut(level)
            .map(|l| l.press_count = value.round().clamp(0.0, 4.0) as u8)
            .is_some(),
        "press_willingness" => schema
            .instructions
            .pressing
            .levels
            .get_mut(level)
            .map(|l| l.press_willingness = value)
            .is_some(),
        "block_depth" => schema
            .instructions
            .line_height
            .levels
            .get_mut(level)
            .map(|l| l.value = value)
            .is_some(),
        "loss_weight" => schema
            .mentalities
            .get_mut(level)
            .map(|m| m.loss_weight = value)
            .is_some(),
        // A count: the nearest whole number inside its range.
        "rest_defence" => schema
            .mentalities
            .get_mut(level)
            .map(|m| m.rest_defence = value.round().clamp(0.0, 10.0) as u8)
            .is_some(),
        _ => {
            let Some(p) = schema
                .primitives
                .iter_mut()
                .find(|p| p.inputs.iter().any(|i| i.input == input))
            else {
                return false;
            };
            let values = if p.owns_levels() {
                p.steps.get_mut(level).map(|s| &mut s.values)
            } else if p.kind == PrimitiveKind::Choice {
                p.choices.get_mut(level).map(|c| &mut c.values)
            } else {
                None
            };
            values.map(|v| v.insert(input.to_string(), value)).is_some()
        }
    }
}

fn refuse(p: &str, what: &str, reason: String) -> Refusal {
    (format!("primitives.{p}.{what}"), reason)
}

/// The checks of every primitive: the kind's levels, choices or states; the default; each
/// input's hook; the signature; the data source and the held-out sets; and each fit record.
pub fn check(schema: &TacticsSchema) -> Result<(), Refusal> {
    for (i, p) in schema.primitives.iter().enumerate() {
        if schema.primitives[..i].iter().any(|o| o.name == p.name) {
            return Err(("primitives".into(), format!("{} appears twice", p.name)));
        }
        check_one(schema, p)?;
    }
    let owning = settings(schema).count();
    if owning > SETTINGS_MAX {
        return Err((
            "primitives".into(),
            format!(
                "{owning} primitives own their levels or choices; the tactics store at most \
                 {SETTINGS_MAX}"
            ),
        ));
    }
    Ok(())
}

/// The checks of a primitive's own steps: exactly [`GRADED_LEVELS`] of them for a graded
/// primitive and two for an on/off one, each name once, and each step giving every input of
/// the primitive a value inside the input's range.
fn check_steps(p: &Primitive) -> Result<(), Refusal> {
    let n = &p.name;
    let want = p.step_count();
    if p.steps.len() != want {
        let what = if p.kind == PrimitiveKind::OnOff {
            format!(
                "an on/off primitive has exactly two states; it has {}",
                p.steps.len()
            )
        } else {
            format!(
                "a graded primitive has exactly {GRADED_LEVELS} levels; it has {}",
                p.steps.len()
            )
        };
        return Err(refuse(n, "steps", what));
    }
    for (i, s) in p.steps.iter().enumerate() {
        if p.steps[..i].iter().any(|o| o.name == s.name) {
            return Err(refuse(n, "steps", format!("{} appears twice", s.name)));
        }
        for ih in &p.inputs {
            if !s.values.contains_key(&ih.input) {
                return Err(refuse(
                    n,
                    "steps",
                    format!("step {} gives no value to {}", s.name, ih.input),
                ));
            }
        }
        check_values(n, "steps", &s.name, &s.values, p)?;
    }
    Ok(())
}

/// Each value of a step or a choice sets an input of the primitive, inside the input's range.
fn check_values(
    n: &str,
    what: &str,
    level: &str,
    values: &BTreeMap<String, f64>,
    p: &Primitive,
) -> Result<(), Refusal> {
    for (k, v) in values {
        if !p.inputs.iter().any(|i| &i.input == k) {
            return Err(refuse(
                n,
                what,
                format!("{level} sets {k}, which is not an input of the primitive"),
            ));
        }
        if let Some((_, _, lo, hi)) = input_field(k)
            && !(lo..=hi).contains(v)
        {
            return Err(refuse(
                n,
                what,
                format!("{level} gives {k} {v}, outside its range {lo} to {hi}"),
            ));
        }
    }
    Ok(())
}

fn check_one(schema: &TacticsSchema, p: &Primitive) -> Result<(), Refusal> {
    let n = &p.name;
    if !p.steps.is_empty() && !p.owns_levels() {
        return Err(refuse(
            n,
            "steps",
            format!("only a graded or on/off primitive whose levels are \"{OWN}\" lists steps"),
        ));
    }
    match p.kind {
        PrimitiveKind::Graded | PrimitiveKind::OnOff if p.owns_levels() => check_steps(p)?,
        PrimitiveKind::Graded => {
            let Some(block) = p.levels.as_deref() else {
                return Err(refuse(
                    n,
                    "levels",
                    "a graded primitive names its level block".into(),
                ));
            };
            let names = block_names(schema, block);
            if names.is_empty() {
                return Err(refuse(
                    n,
                    "levels",
                    format!("{block} is not a level block of the tactics file"),
                ));
            }
            if names.len() != GRADED_LEVELS {
                return Err(refuse(
                    n,
                    "levels",
                    format!(
                        "a graded primitive has exactly {GRADED_LEVELS} levels; {block} has {}",
                        names.len()
                    ),
                ));
            }
        }
        PrimitiveKind::Choice => {
            if p.choices.is_empty() {
                return Err(refuse(
                    n,
                    "choices",
                    "a choice primitive lists its choices".into(),
                ));
            }
            for (i, c) in p.choices.iter().enumerate() {
                if p.choices[..i].iter().any(|o| o.name == c.name) {
                    return Err(refuse(n, "choices", format!("{} appears twice", c.name)));
                }
                check_values(n, "choices", &format!("choice {}", c.name), &c.values, p)?;
                if !SIGNATURE_STATS.contains(&c.stat.as_str()) {
                    return Err(refuse(
                        n,
                        "choices",
                        format!("choice {}: {} is not a signature statistic", c.name, c.stat),
                    ));
                }
            }
        }
        PrimitiveKind::OnOff => {
            if p.states.len() != 2 {
                return Err(refuse(
                    n,
                    "states",
                    format!(
                        "an on/off primitive has exactly two states; it has {}",
                        p.states.len()
                    ),
                ));
            }
        }
    }
    if p.default_index(schema).is_none() {
        return Err(refuse(
            n,
            "default",
            format!("{} is not one of its levels", p.default),
        ));
    }
    if p.inputs.is_empty() {
        return Err(refuse(
            n,
            "inputs",
            "a primitive sets at least one plan input".into(),
        ));
    }
    for ih in &p.inputs {
        let Some(hook) = hooks::hook(&ih.hook) else {
            return Err(refuse(n, "inputs", format!("{} is not a hook", ih.hook)));
        };
        let reads = std::iter::once(hook.input).chain(
            hooks::SECOND_INPUTS
                .iter()
                .filter(|(h, _)| *h == hook.name)
                .map(|(_, i)| *i),
        );
        let reads: Vec<&str> = reads.collect();
        if !reads.contains(&ih.input.as_str()) {
            return Err(refuse(
                n,
                "inputs",
                format!(
                    "hook {} reads {}, not {}",
                    ih.hook,
                    reads.join(", "),
                    ih.input
                ),
            ));
        }
        if p.kind == PrimitiveKind::Graded || p.owns_levels() {
            let block = p.levels.as_deref().unwrap_or_default();
            if input_field(&ih.input).is_none_or(|(b, ..)| b != block) {
                return Err(refuse(
                    n,
                    "inputs",
                    format!("{} is not a field of the levels of {block}", ih.input),
                ));
            }
        }
    }
    if p.signature.is_empty() {
        return Err(refuse(
            n,
            "signature",
            "a primitive names at least one signature statistic".into(),
        ));
    }
    if let Some(s) = p
        .signature
        .iter()
        .find(|s| !SIGNATURE_STATS.contains(&s.stat.as_str()))
    {
        return Err(refuse(
            n,
            "signature",
            format!("{} is not a signature statistic", s.stat),
        ));
    }
    if p.signature.iter().any(|s| s.steps != Steps::All) {
        if p.kind != PrimitiveKind::Graded {
            return Err(refuse(
                n,
                "signature",
                "only a graded primitive judges its steps below and above the default apart".into(),
            ));
        }
        let d = p.default_index(schema).unwrap_or_default();
        if let Some(step) =
            (0..GRADED_LEVELS - 1).find(|&l| !p.signature.iter().any(|s| s.steps.covers_step(l, d)))
        {
            return Err(refuse(
                n,
                "signature",
                format!("no statistic judges the step up from level {step}"),
            ));
        }
    }
    let d = &p.data;
    match (&d.source, &d.none) {
        (None, None) => {
            return Err(refuse(
                n,
                "data",
                "name a data source, or the reason no real data fits".into(),
            ));
        }
        (Some(_), Some(_)) => {
            return Err(refuse(
                n,
                "data",
                "a data source and a no-data reason exclude each other".into(),
            ));
        }
        (Some(source), None) => {
            if !SOURCES.contains(&source.as_str()) {
                return Err(refuse(n, "data", format!("{source} is not a data source")));
            }
            if let Some(c) = d.competitions.iter().find(|c| held_out(source, c)) {
                return Err(refuse(
                    n,
                    "data",
                    format!("held-out set: {c} of {source} is never read"),
                ));
            }
            if d.row.is_none() {
                return Err(refuse(
                    n,
                    "data",
                    "a data source names its row of the signature table".into(),
                ));
            }
            if d.deferred.as_deref().is_some_and(|r| {
                r.trim().is_empty() || (d.deferred_levels.is_empty() && !p.fits.is_empty())
            }) {
                return Err(refuse(
                    n,
                    "data",
                    "a deferred primitive gives its reason and has no fit".into(),
                ));
            }
            if !d.deferred_levels.is_empty() {
                let names = p.level_names(schema);
                let default = p.default.as_str();
                if d.deferred.is_none()
                    || d.deferred_levels
                        .iter()
                        .any(|l| l == default || !names.contains(&l.as_str()))
                    || p.fits.iter().any(|f| d.deferred_levels.contains(&f.level))
                {
                    return Err(refuse(
                        n,
                        "data",
                        "deferred levels are levels other than the default, under a deferred \
                         reason, with no fit"
                            .into(),
                    ));
                }
            }
        }
        (None, Some(reason)) => {
            if reason.trim().is_empty()
                || !p.fits.is_empty()
                || d.deferred.is_some()
                || !d.deferred_levels.is_empty()
            {
                return Err(refuse(
                    n,
                    "data",
                    "a primitive with no real data gives its reason, has no fit and defers \
                     no fit"
                        .into(),
                ));
            }
        }
    }
    let names = p.level_names(schema);
    for fit in &p.fits {
        let Some(level) = names.iter().position(|l| *l == fit.level) else {
            return Err(refuse(
                n,
                "fits",
                format!("{} is not one of its levels", fit.level),
            ));
        };
        for (input, value) in &fit.values {
            let actual = match p.kind {
                PrimitiveKind::Graded | PrimitiveKind::OnOff => level_value(schema, input, level),
                PrimitiveKind::Choice => p.choices[level].values.get(input).copied(),
            };
            if actual.is_none_or(|a| (a - value).abs() > 1e-9) {
                return Err(refuse(
                    n,
                    "fits",
                    format!(
                        "the fit of {} gives {input} {value}; the level holds {}",
                        fit.level,
                        actual.map_or("nothing".to_string(), |a| a.to_string())
                    ),
                ));
            }
        }
    }
    Ok(())
}

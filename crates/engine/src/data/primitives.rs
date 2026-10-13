//! The tactical primitives of the tactics file: each setting a manager can choose on the
//! tactics board, as data. A primitive is graded (5 ordered levels), a choice (a list of
//! choices) or on/off (two states). It sets one or more plan inputs, each read at one hook of
//! the hook list ([`crate::tactics::hooks::HOOKS`]); it moves a signature statistic in a
//! stated direction; and its levels are fitted to the spread of that statistic across real
//! teams, or it names why no real data fits it.
//!
//! The levels of a graded primitive live in a level block of the tactics file (an
//! instruction's levels, or the mentality list), so the tactics a match stores keep their
//! shape: a level is an index into its block. [`check`] refuses a bad entry by name.

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::tactics::{INSTRUCTIONS, Refusal, TacticsSchema};
use crate::tactics::hooks;

/// The number of levels of a graded primitive.
pub const GRADED_LEVELS: usize = 5;

/// The statistics a primitive can name as its signature. The real-team table
/// (`signatures.json`) and the engine measure each one by the same definition.
pub const SIGNATURE_STATS: [&str; 7] = [
    "passes_per_possession",
    "recycle_share",
    "turnovers_per_possession",
    "shot_share",
    "block_height_m",
    "hold_dribble_share",
    "ppda",
];

/// The real data sources a fit may read.
pub const SOURCES: [&str; 4] = [
    "wyscout_2017_18",
    "statsbomb_open",
    "skillcorner_open",
    "dfl_open",
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

/// Where each plan input of a graded primitive is written in its level block: the block, the
/// field of each level, and the range the field allows.
pub const INPUT_FIELDS: [(&str, &str, &str, f64, f64); 3] = [
    ("patience", "instructions.tempo", "value", -2.0, 2.0),
    ("loss_weight", "mentalities", "loss_weight", -5.0, 5.0),
    ("rest_defence", "mentalities", "rest_defence", 0.0, 10.0),
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

/// A signature statistic and its direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Signature {
    #[garde(length(min = 1, max = 48))]
    pub stat: String,
    #[garde(skip)]
    pub direction: Direction,
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
    /// the range end nearest the target.
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
    /// A graded primitive's level block: `instructions.<instruction>` or `mentalities`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub levels: Option<String>,
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

/// The value of plan input `input` at level `level` of its block.
pub fn level_value(schema: &TacticsSchema, input: &str, level: usize) -> Option<f64> {
    match input {
        "patience" => schema.instructions.tempo.levels.get(level).map(|l| l.value),
        "loss_weight" => schema.mentalities.get(level).map(|m| m.loss_weight),
        "rest_defence" => schema
            .mentalities
            .get(level)
            .map(|m| f64::from(m.rest_defence)),
        _ => None,
    }
}

/// Sets plan input `input` at level `level` of its block to `value`; `false` when the input
/// or the level does not exist.
pub fn set_level_value(schema: &mut TacticsSchema, input: &str, level: usize, value: f64) -> bool {
    match input {
        "patience" => schema
            .instructions
            .tempo
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
        _ => false,
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
    Ok(())
}

fn check_one(schema: &TacticsSchema, p: &Primitive) -> Result<(), Refusal> {
    let n = &p.name;
    match p.kind {
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
            for c in &p.choices {
                if let Some(k) = c
                    .values
                    .keys()
                    .find(|k| !p.inputs.iter().any(|i| &i.input == *k))
                {
                    return Err(refuse(
                        n,
                        "choices",
                        format!(
                            "choice {} sets {k}, which is not an input of the primitive",
                            c.name
                        ),
                    ));
                }
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
        if p.kind == PrimitiveKind::Graded {
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
        }
        (None, Some(reason)) => {
            if reason.trim().is_empty() || !p.fits.is_empty() {
                return Err(refuse(
                    n,
                    "data",
                    "a primitive with no real data gives its reason and has no fit".into(),
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
                PrimitiveKind::Graded => level_value(schema, input, level),
                PrimitiveKind::Choice => p.choices[level].values.get(input).copied(),
                PrimitiveKind::OnOff => None,
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

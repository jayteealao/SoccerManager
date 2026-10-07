//! The attribute schema file: 30 to 50 named attributes in four groups, each rated 1.0 to
//! 20.0 in tenths, with the job each one does in play, the stage tables that blend them for
//! every action ([`crate::contract`]), and the skill gates.
//!
//! Version 2 adds the jobs, the stage tables, and the gates; a version 1 file converts with
//! the first tables of this build ([`crate::data::convert::attributes_v1_to_v2`]). Version 3
//! adds hidden attributes: consistency, whose job is the spread of every action, and injury
//! proneness, which replaces injury resistance; a version 2 file converts
//! ([`crate::data::convert::attributes_v2_to_v3`]).

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::contract::{ActionKind, GateDefs, STAGES, StageKind, StageTable};

/// Schema version this build reads.
pub const ATTRIBUTES_VERSION: u32 = 3;
/// Upper bound on the attribute count; `Attributes` on a player is a fixed array of this size.
pub const MAX_ATTRIBUTES: usize = 50;
/// Lower bound on the attribute count.
pub const MIN_ATTRIBUTES: usize = 30;
/// Attributes play reads by name rather than through a stage table: pace through the
/// top-speed map, technique and agility through the skill gates, and consistency through the
/// spread of every action ([`crate::contract::consistency`]).
pub const REQUIRED: [&str; 4] = ["pace", "technique", "agility", "consistency"];

/// The four attribute groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    Technical,
    Mental,
    Physical,
    Goalkeeping,
}

/// Which way a job moves its statistic as the rating rises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Up,
    Down,
}

/// Where a job acts: one of the four stages of its action, the top-speed map, or the spread
/// of every action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStage {
    See,
    Choose,
    Execute,
    Pressure,
    /// The top-speed map, which only pace feeds.
    TopSpeed,
    /// The spread of a player's play around his ratings within and between matches, which
    /// only a hidden attribute (consistency) feeds. It names no action: it acts on all.
    Spread,
}

impl JobStage {
    /// The stage kind, or `None` for the top-speed map and the spread.
    pub fn stage(self) -> Option<StageKind> {
        match self {
            JobStage::See => Some(StageKind::See),
            JobStage::Choose => Some(StageKind::Choose),
            JobStage::Execute => Some(StageKind::Execute),
            JobStage::Pressure => Some(StageKind::Pressure),
            JobStage::TopSpeed | JobStage::Spread => None,
        }
    }
}

/// An attribute's job in play: the action and stage it acts in, and the statistic of a match
/// it moves, in which direction. The sensitivity rules read the statistic. Every job names an
/// action except the spread, which acts on every action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Job {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub action: Option<ActionKind>,
    #[garde(skip)]
    pub stage: JobStage,
    #[garde(length(min = 3, max = 120))]
    pub statistic: String,
    #[garde(skip)]
    pub direction: Direction,
}

/// One attribute definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct AttributeDef {
    #[garde(length(min = 2, max = 32))]
    pub name: String,
    #[garde(skip)]
    pub group: Group,
    /// A hidden value: it plays, but nothing that leaves the engine for a page carries it as
    /// a number, only as a word with a confidence ([`crate::contract::hidden`]).
    #[serde(default, skip_serializing_if = "is_false")]
    #[garde(skip)]
    pub hidden: bool,
    #[garde(dive)]
    pub job: Job,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// The stage tables of every action, keyed by action and stage.
pub type ActionTables = BTreeMap<ActionKind, BTreeMap<StageKind, StageTable>>;

/// The attribute schema file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct AttributeSchema {
    #[garde(skip)]
    pub schema_version: u32,
    #[garde(length(min = 30, max = 50), dive, custom(check_names))]
    pub attributes: Vec<AttributeDef>,
    /// One table per stage play reads; see [`crate::contract::STAGES`].
    #[garde(dive, custom(check_tables(&self.attributes)))]
    pub actions: ActionTables,
    #[garde(dive)]
    pub gates: GateDefs,
}

impl AttributeSchema {
    /// The number of attributes.
    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    /// True when the schema holds no attribute.
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
    }

    /// The index of `name` in schema order, or `None`.
    pub fn index(&self, name: &str) -> Option<usize> {
        self.attributes.iter().position(|a| a.name == name)
    }

    /// The indices of the required attributes, in `REQUIRED` order.
    pub fn required_indices(&self) -> [usize; REQUIRED.len()] {
        let mut out = [0; REQUIRED.len()];
        for (slot, name) in out.iter_mut().zip(REQUIRED) {
            *slot = self
                .index(name)
                .expect("validated schema holds every required name");
        }
        out
    }

    /// The names of the hidden attributes, in schema order.
    pub fn hidden_names(&self) -> impl Iterator<Item = &str> {
        self.attributes
            .iter()
            .filter(|a| a.hidden)
            .map(|a| a.name.as_str())
    }

    /// `true` when `attribute` is the main attribute or a support of `action`'s `stage`.
    pub fn in_stage(&self, attribute: &str, action: ActionKind, stage: StageKind) -> bool {
        self.actions
            .get(&action)
            .and_then(|t| t.get(&stage))
            .is_some_and(|t| {
                t.main.attribute == attribute || t.supports.iter().any(|s| s.attribute == attribute)
            })
    }
}

fn check_names(attributes: &[AttributeDef], _ctx: &()) -> garde::Result {
    for (i, a) in attributes.iter().enumerate() {
        if attributes[..i].iter().any(|b| b.name == a.name) {
            return Err(garde::Error::new(format!(
                "attribute {} appears twice",
                a.name
            )));
        }
    }
    for name in REQUIRED {
        if !attributes.iter().any(|a| a.name == name) {
            return Err(garde::Error::new(format!(
                "required attribute {name} is missing"
            )));
        }
    }
    Ok(())
}

/// Every stage play reads has a table and no other stage does; each table names attributes of
/// the file, each once, with positive weights and the main weight the largest; every
/// attribute other than pace and one whose job is the spread is in some stage; every job acts
/// where the tables put its attribute; only a hidden attribute has the spread for its job, and
/// a hidden attribute's job is the spread or an execute stage.
fn check_tables(
    attributes: &[AttributeDef],
) -> impl FnOnce(&ActionTables, &()) -> garde::Result + '_ {
    move |tables, _| {
        let known = |name: &str| attributes.iter().any(|a| a.name == name);
        for &(action, stage) in &STAGES {
            if tables.get(&action).and_then(|t| t.get(&stage)).is_none() {
                return Err(garde::Error::new(format!(
                    "{}.{} has no table",
                    action.name(),
                    stage.name()
                )));
            }
        }
        for (action, stages) in tables {
            for (stage, table) in stages {
                let at = format!("{}.{}", action.name(), stage.name());
                if !STAGES.contains(&(*action, *stage)) {
                    return Err(garde::Error::new(format!(
                        "{at} is not a stage play reads; remove it"
                    )));
                }
                let all: Vec<_> = std::iter::once(&table.main)
                    .chain(&table.supports)
                    .collect();
                for (i, w) in all.iter().enumerate() {
                    if !known(&w.attribute) {
                        return Err(garde::Error::new(format!(
                            "{at}: attribute {} is not in the file",
                            w.attribute
                        )));
                    }
                    if all[..i].iter().any(|v| v.attribute == w.attribute) {
                        return Err(garde::Error::new(format!(
                            "{at}: attribute {} appears twice",
                            w.attribute
                        )));
                    }
                    if w.weight <= 0.0 {
                        return Err(garde::Error::new(format!(
                            "{at}: attribute {} has weight {}; must be above 0",
                            w.attribute, w.weight
                        )));
                    }
                }
                if let Some(s) = table
                    .supports
                    .iter()
                    .find(|s| s.weight >= table.main.weight)
                {
                    return Err(garde::Error::new(format!(
                        "{at}: support {} weighs {}, not below the main weight {}",
                        s.attribute, s.weight, table.main.weight
                    )));
                }
            }
        }
        let in_some = |name: &str| {
            tables
                .values()
                .flat_map(|s| s.values())
                .any(|t| t.main.attribute == name || t.supports.iter().any(|s| s.attribute == name))
        };
        for def in attributes {
            let job = &def.job;
            let action_name = job.action.map_or("(none)", ActionKind::name);
            if job.stage == JobStage::Spread {
                if !def.hidden {
                    return Err(garde::Error::new(format!(
                        "attribute {}: only a hidden attribute may have the spread for its job",
                        def.name
                    )));
                }
                if job.action.is_some() {
                    return Err(garde::Error::new(format!(
                        "attribute {}: the spread acts on every action; remove its action {}",
                        def.name, action_name
                    )));
                }
                continue;
            }
            let Some(action) = job.action else {
                return Err(garde::Error::new(format!(
                    "attribute {}: its job at {:?} names no action",
                    def.name, job.stage
                )));
            };
            if def.hidden && job.stage != JobStage::Execute {
                return Err(garde::Error::new(format!(
                    "attribute {}: a hidden attribute's job is the spread or an execute stage, \
                     not {:?}",
                    def.name, job.stage
                )));
            }
            let fits = match job.stage.stage() {
                Some(stage) => tables
                    .get(&action)
                    .and_then(|t| t.get(&stage))
                    .is_some_and(|t| {
                        t.main.attribute == def.name
                            || t.supports.iter().any(|s| s.attribute == def.name)
                    }),
                None => def.name == "pace" && action == ActionKind::Sprint,
            };
            if !fits {
                return Err(garde::Error::new(format!(
                    "attribute {}: its job names {}.{:?}, where the tables do not put it",
                    def.name, action_name, job.stage
                )));
            }
            if def.name != "pace" && !in_some(&def.name) {
                return Err(garde::Error::new(format!(
                    "attribute {} is in no stage table",
                    def.name
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_content;

    fn shipped() -> AttributeSchema {
        shipped_content().attributes
    }

    fn refusal(schema: &AttributeSchema) -> String {
        schema.validate().unwrap_err().to_string()
    }

    #[test]
    fn the_shipped_schema_validates_and_names_the_required_four() {
        let s = shipped();
        assert!(s.validate().is_ok());
        let [pace, technique, agility, consistency] = s.required_indices();
        assert_eq!(s.attributes[pace].name, "pace");
        assert_eq!(s.attributes[technique].name, "technique");
        assert_eq!(s.attributes[agility].name, "agility");
        assert_eq!(s.attributes[consistency].name, "consistency");
        let hidden: Vec<_> = s.hidden_names().collect();
        assert_eq!(hidden, ["injury_proneness", "consistency"]);
    }

    #[test]
    fn the_spread_belongs_to_a_hidden_attribute_only() {
        let mut s = shipped();
        let i = s.index("consistency").unwrap();
        s.attributes[i].hidden = false;
        let text = refusal(&s);
        assert!(
            text.contains("only a hidden attribute may have the spread"),
            "{text}"
        );
    }

    #[test]
    fn a_job_with_no_action_outside_the_spread_is_refused() {
        let mut s = shipped();
        let i = s.index("passing").unwrap();
        s.attributes[i].job.action = None;
        let text = refusal(&s);
        assert!(
            text.contains("attribute passing: its job at Execute names no action"),
            "{text}"
        );
    }

    #[test]
    fn a_schema_without_agility_is_refused_by_name() {
        let mut s = shipped();
        s.attributes.retain(|a| a.name != "agility");
        let text = refusal(&s);
        assert!(
            text.contains("attributes: required attribute agility is missing"),
            "{text}"
        );
    }

    #[test]
    fn a_missing_stage_table_is_refused_by_name() {
        let mut s = shipped();
        s.actions
            .get_mut(&ActionKind::Tackle)
            .unwrap()
            .remove(&StageKind::Choose);
        let text = refusal(&s);
        assert!(text.contains("tackle.choose has no table"), "{text}");
    }

    #[test]
    fn a_stage_play_does_not_read_is_refused() {
        let mut s = shipped();
        let extra = s.actions[&ActionKind::Tackle][&StageKind::Choose].clone();
        s.actions
            .get_mut(&ActionKind::Tackle)
            .unwrap()
            .insert(StageKind::See, extra);
        let text = refusal(&s);
        assert!(
            text.contains("tackle.see is not a stage play reads"),
            "{text}"
        );
    }

    #[test]
    fn a_support_as_heavy_as_the_main_attribute_is_refused() {
        let mut s = shipped();
        let t = s
            .actions
            .get_mut(&ActionKind::Pass)
            .unwrap()
            .get_mut(&StageKind::Execute)
            .unwrap();
        t.supports[0].weight = t.main.weight;
        let text = refusal(&s);
        assert!(text.contains("pass.execute: support"), "{text}");
    }

    #[test]
    fn an_attribute_in_no_stage_is_refused() {
        let mut s = shipped();
        for stages in s.actions.values_mut() {
            for t in stages.values_mut() {
                t.supports.retain(|w| w.attribute != "kicking");
                if t.main.attribute == "kicking" {
                    t.main.attribute = "throwing".into();
                }
            }
        }
        let text = refusal(&s);
        assert!(text.contains("attribute kicking"), "{text}");
    }

    #[test]
    fn too_few_attributes_are_refused() {
        let mut s = shipped();
        s.attributes.truncate(10);
        assert!(refusal(&s).contains("length is lower than 30"));
    }
}

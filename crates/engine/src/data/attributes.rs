//! The attribute schema file: 30 to 50 visible and hidden attributes in four groups and up to
//! 14 personality values in a fifth, each rated 1.0 to 20.0 in tenths, with the job each one
//! does in play, the stage tables that blend them for every action ([`crate::contract`]), the
//! consistency degree of each action, and the skill gates.
//!
//! Version 2 adds the jobs, the stage tables, and the gates; a version 1 file converts with
//! the first tables of this build ([`crate::data::convert::attributes_v1_to_v2`]). Version 3
//! adds hidden attributes: consistency, whose job is the spread of every action, and injury
//! proneness, which replaces injury resistance; a version 2 file converts
//! ([`crate::data::convert::attributes_v2_to_v3`]). Version 4 adds the new performance
//! attributes, moves communication to the mental group, adds the personality group (hidden
//! values whose jobs act on occasions in a match, or on nothing in a match), the small weights
//! of a stage, and a consistency degree per action; a version 3 file converts
//! ([`crate::data::convert::attributes_v3_to_v4`]). Version 5 adds the cover and recovery-run
//! stages, re-weights the press and shape stages, moves work rate's job to the recovery run
//! and makes determination's an occasion; a version 4 file converts
//! ([`crate::data::convert::attributes_v4_to_v5`]).

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::contract::{ActionKind, GateDefs, STAGES, StageKind, StageTable};

/// Schema version this build reads.
pub const ATTRIBUTES_VERSION: u32 = 5;
/// The slots of a player's rating array: `Attributes` on a player is a fixed array of this
/// size, holding every visible, hidden, and personality value.
pub const MAX_ATTRIBUTES: usize = 64;
/// Upper bound on the visible and hidden attributes together; personality values do not count.
pub const MAX_PERFORMANCE: usize = 50;
/// Upper bound on the personality values.
pub const MAX_PERSONALITY: usize = 14;
/// Lower bound on the visible and hidden attributes together.
pub const MIN_ATTRIBUTES: usize = 30;
/// The statistic of a personality value that acts on nothing in a match.
pub const INERT_STATISTIC: &str = "inert in a match";
/// The heaviest a consistency degree may be.
pub const MAX_DEGREE: f64 = 2.0;
/// Attributes play reads by name rather than through a stage table: pace through the
/// top-speed map, technique and agility through the skill gates, and consistency through the
/// spread of every action ([`crate::contract::consistency`]).
pub const REQUIRED: [&str; 4] = ["pace", "technique", "agility", "consistency"];

/// The attribute groups: the four the states act on, and personality, which no state moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    Technical,
    Mental,
    Physical,
    Goalkeeping,
    /// Hidden character values. A catalogue group but not a state group: no state family
    /// owns it and no delta moves it.
    Personality,
}

impl Group {
    /// The group's index in the state deltas, or `None` for personality, which has none.
    pub fn state_index(self) -> Option<usize> {
        match self {
            Group::Technical => Some(0),
            Group::Mental => Some(1),
            Group::Physical => Some(2),
            Group::Goalkeeping => Some(3),
            Group::Personality => None,
        }
    }
}

/// Which way a job moves its statistic as the rating rises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Up,
    Down,
}

/// Where a job acts: one of the four stages of its action, the top-speed map, the spread of
/// every action, an occasion in a match (a personality value), or nothing in a match.
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
    /// An occasion or a choice in a match that a personality value acts on, such as a
    /// reaction to a foul or the weight in team mood. It names no action.
    Occasion,
    /// Nothing in a match: a personality value that acts off the pitch. Its statistic is
    /// "inert in a match".
    Inert,
}

impl JobStage {
    /// The stage kind, or `None` for the top-speed map, the spread, an occasion, and inert.
    pub fn stage(self) -> Option<StageKind> {
        match self {
            JobStage::See => Some(StageKind::See),
            JobStage::Choose => Some(StageKind::Choose),
            JobStage::Execute => Some(StageKind::Execute),
            JobStage::Pressure => Some(StageKind::Pressure),
            JobStage::TopSpeed | JobStage::Spread | JobStage::Occasion | JobStage::Inert => None,
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

impl AttributeDef {
    /// `true` for a personality value.
    pub fn is_personality(&self) -> bool {
        self.group == Group::Personality
    }
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
    #[garde(dive, custom(check_counts), custom(check_names))]
    pub attributes: Vec<AttributeDef>,
    /// One table per stage play reads; see [`crate::contract::STAGES`].
    #[garde(dive, custom(check_tables(&self.attributes)))]
    pub actions: ActionTables,
    /// How far consistency narrows or widens each action's spread: one degree per action with
    /// a table, 0.0 to 2.0, 1.0 the uniform spread.
    #[serde(default)]
    #[garde(custom(check_degrees))]
    pub consistency_degrees: BTreeMap<ActionKind, f64>,
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

    /// The names of the hidden attributes, personality values included, in schema order.
    pub fn hidden_names(&self) -> impl Iterator<Item = &str> {
        self.attributes
            .iter()
            .filter(|a| a.hidden)
            .map(|a| a.name.as_str())
    }

    /// `true` when `attribute` is the main attribute or a shown support of `action`'s `stage`.
    /// A small weight does not count: the screens never list it.
    pub fn in_stage(&self, attribute: &str, action: ActionKind, stage: StageKind) -> bool {
        self.actions
            .get(&action)
            .and_then(|t| t.get(&stage))
            .is_some_and(|t| {
                t.main.attribute == attribute || t.supports.iter().any(|s| s.attribute == attribute)
            })
    }
}

/// Visible and hidden attributes together 30 to 50, and personality values at most 14, so
/// personality never takes a performance attribute's place.
fn check_counts(attributes: &[AttributeDef], _ctx: &()) -> garde::Result {
    let personality = attributes.iter().filter(|a| a.is_personality()).count();
    let performance = attributes.len() - personality;
    if performance > MAX_PERFORMANCE {
        return Err(garde::Error::new(format!(
            "{performance} visible and hidden attributes; at most {MAX_PERFORMANCE}"
        )));
    }
    if performance < MIN_ATTRIBUTES {
        return Err(garde::Error::new(format!(
            "{performance} visible and hidden attributes; at least {MIN_ATTRIBUTES}"
        )));
    }
    if personality > MAX_PERSONALITY {
        return Err(garde::Error::new(format!(
            "{personality} personality values; at most {MAX_PERSONALITY}"
        )));
    }
    Ok(())
}

/// One degree for every action play reads a stage of, each 0.0 to 2.0, and none for another.
fn check_degrees(degrees: &BTreeMap<ActionKind, f64>, _ctx: &()) -> garde::Result {
    for action in ActionKind::ALL {
        let has_stage = STAGES.iter().any(|(a, _)| *a == action);
        match degrees.get(&action) {
            None if has_stage => {
                return Err(garde::Error::new(format!(
                    "action {} has no consistency degree",
                    action.name()
                )));
            }
            Some(_) if !has_stage => {
                return Err(garde::Error::new(format!(
                    "action {} has no stage; remove its consistency degree",
                    action.name()
                )));
            }
            Some(&d) if !(0.0..=MAX_DEGREE).contains(&d) => {
                return Err(garde::Error::new(format!(
                    "action {}: consistency degree {d}; allowed 0.0 to {MAX_DEGREE}",
                    action.name()
                )));
            }
            _ => {}
        }
    }
    Ok(())
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
/// the file, each once, with positive weights and the main weight the largest, and each small
/// weight lighter than the lightest shown support (or the main, without a support); every
/// attribute other than pace, one whose job is the spread, and a personality value is in some
/// stage; every job acts where the tables show its attribute (main or shown support, never a
/// small weight); only a hidden attribute has the spread for its job, and a hidden
/// attribute's job is the spread or an execute stage. A personality value is hidden, has an
/// occasion or inert job that names no action, and is in no table; a visible performance
/// attribute may have an occasion job that names no action, and then it may sit in tables or
/// in none; no other attribute has an occasion or inert job; an inert job's statistic is
/// "inert in a match".
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
                    .chain(&table.minor)
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
                let lightest = table
                    .supports
                    .iter()
                    .map(|s| s.weight)
                    .fold(table.main.weight, f64::min);
                if let Some(m) = table.minor.iter().find(|m| m.weight >= lightest) {
                    return Err(garde::Error::new(format!(
                        "{at}: small weight {} weighs {}, not below the lightest shown weight {}",
                        m.attribute, m.weight, lightest
                    )));
                }
            }
        }
        let in_some = |name: &str| {
            tables.values().flat_map(|s| s.values()).any(|t| {
                t.main.attribute == name
                    || t.supports.iter().any(|s| s.attribute == name)
                    || t.minor.iter().any(|s| s.attribute == name)
            })
        };
        for def in attributes {
            let job = &def.job;
            let action_name = job.action.map_or("(none)", ActionKind::name);
            let occasion = matches!(job.stage, JobStage::Occasion | JobStage::Inert);
            if def.is_personality() {
                let refuse = |why: String| {
                    Err(garde::Error::new(format!(
                        "personality value {}: {why}",
                        def.name
                    )))
                };
                if !def.hidden {
                    return refuse("a personality value is hidden; set hidden".into());
                }
                if !occasion {
                    return refuse(format!(
                        "its job is an occasion or inert in a match, not {:?}",
                        job.stage
                    ));
                }
                if job.action.is_some() {
                    return refuse(format!(
                        "it acts on an occasion, not an action; remove its action {action_name}"
                    ));
                }
                if in_some(&def.name) {
                    return refuse("it is in a stage table; a personality value is in none".into());
                }
                if job.stage == JobStage::Inert && job.statistic != INERT_STATISTIC {
                    return refuse(format!(
                        "an inert job's statistic is \"{INERT_STATISTIC}\", not \"{}\"",
                        job.statistic
                    ));
                }
                continue;
            }
            // A visible performance attribute may act on an occasion that no stage of its own
            // reads (determination widens the press while his side trails); such a job names
            // no action, and the attribute may sit in tables or in none. Only a personality
            // value is inert in a match.
            if job.stage == JobStage::Occasion && !def.hidden {
                if job.action.is_some() {
                    return Err(garde::Error::new(format!(
                        "attribute {}: an occasion job names no action; remove its action \
                         {action_name}",
                        def.name
                    )));
                }
                continue;
            }
            if occasion {
                return Err(garde::Error::new(format!(
                    "attribute {}: only a personality value or a visible attribute may have an \
                     occasion job, and only a personality value an inert one, not {:?}",
                    def.name, job.stage
                )));
            }
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
        let hidden: Vec<_> = s
            .hidden_names()
            .filter(|n| !s.attributes[s.index(n).unwrap()].is_personality())
            .collect();
        assert_eq!(hidden, ["injury_proneness", "consistency"]);
        assert_eq!(s.hidden_names().count(), 2 + 11);
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
        let text = refusal(&s);
        assert!(
            text.contains("10 visible and hidden attributes; at least 30"),
            "{text}"
        );
    }

    #[test]
    fn a_fifteenth_personality_value_is_refused_with_the_count() {
        let mut s = shipped();
        let first = s
            .attributes
            .iter()
            .position(|a| a.is_personality())
            .unwrap();
        for i in 0..4 {
            let mut extra = s.attributes[first].clone();
            extra.name = format!("character_{i}");
            s.attributes.push(extra);
        }
        let text = refusal(&s);
        assert!(text.contains("15 personality values; at most 14"), "{text}");
    }

    #[test]
    fn a_small_weight_as_heavy_as_a_shown_support_is_refused() {
        let mut s = shipped();
        let t = s
            .actions
            .get_mut(&ActionKind::Penalty)
            .unwrap()
            .get_mut(&StageKind::Execute)
            .unwrap();
        let lightest = t.supports.iter().map(|w| w.weight).fold(f64::MAX, f64::min);
        t.minor[0].weight = lightest;
        let text = refusal(&s);
        assert!(
            text.contains("penalty.execute: small weight technique weighs"),
            "{text}"
        );
    }

    #[test]
    fn a_job_on_a_small_weight_is_refused() {
        let mut s = shipped();
        let i = s.index("technique").unwrap();
        s.attributes[i].job.action = Some(ActionKind::Penalty);
        s.attributes[i].job.stage = JobStage::Execute;
        let text = refusal(&s);
        assert!(
            text.contains("attribute technique: its job names penalty.Execute"),
            "{text}"
        );
    }

    #[test]
    fn an_inert_job_outside_personality_is_refused() {
        let mut s = shipped();
        let i = s.index("vision").unwrap();
        s.attributes[i].job.stage = JobStage::Inert;
        s.attributes[i].job.action = None;
        let text = refusal(&s);
        assert!(
            text.contains("attribute vision: only a personality value or a visible attribute"),
            "{text}"
        );
        let mut s = shipped();
        let i = s.index("consistency").unwrap();
        s.attributes[i].job.stage = JobStage::Occasion;
        let text = refusal(&s);
        assert!(text.contains("attribute consistency: only"), "{text}");
    }

    #[test]
    fn determination_acts_on_an_occasion_and_an_occasion_job_names_no_action() {
        let s = shipped();
        let job = &s.attributes[s.index("determination").unwrap()].job;
        assert_eq!((job.stage, job.action), (JobStage::Occasion, None));
        assert!(s.validate().is_ok());
        let mut s = shipped();
        let i = s.index("determination").unwrap();
        s.attributes[i].job.action = Some(ActionKind::Press);
        let text = refusal(&s);
        assert!(
            text.contains("attribute determination: an occasion job names no action"),
            "{text}"
        );
    }

    #[test]
    fn an_action_without_a_consistency_degree_is_refused() {
        let mut s = shipped();
        s.consistency_degrees.remove(&ActionKind::Corner);
        let text = refusal(&s);
        assert!(
            text.contains("action corner has no consistency degree"),
            "{text}"
        );
        let mut s = shipped();
        s.consistency_degrees.insert(ActionKind::Shot, 2.5);
        let text = refusal(&s);
        assert!(
            text.contains("action shot: consistency degree 2.5"),
            "{text}"
        );
    }
}

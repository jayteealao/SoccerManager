//! The named styles file (`content/styles.json`): each style is a mentality, instruction
//! levels, duty hints by position, the formations it plays, the two or three statistics that
//! show its identity, and the attributes its key jobs read. The AI manager's pre-match advice
//! weighs a squad against each style ([`crate::ai::style_estimates`]); the styles suite plays
//! them against each other. The file is optional: content without it keeps the tactics file's
//! default AI tactics.
//!
//! Loading resolves every name against the tactics file and the attribute schema; an unknown
//! formation, mentality, instruction, level, duty, position or attribute is refused by name,
//! as is an empty formation list, an identity list outside 2 to 3, an unknown identity
//! statistic, or fit weights that sum to 0.

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::attributes::{AttributeSchema, Group};
use crate::data::tactics::{INSTRUCTIONS, TacticsSchema};
use crate::data::team::Position;
use crate::data::{Loaded, load_json_bytes};
use crate::error::EngineError;
use crate::tactics::{RoleDuty, Tactics};
use crate::team::PLAYERS_PER_TEAM;

/// Schema version this build reads.
pub const STYLES_VERSION: u32 = 1;

/// The statistics a style's identity may name: per-team match figures the styles report reads.
/// `against` reads the other side's figure.
pub const IDENTITY_STATS: [&str; 9] = [
    "possession_pct",
    "passes",
    "pass_accuracy_pct",
    "shots",
    "shots_on_target_share",
    "fouls",
    "corners",
    "goals",
    "xg",
];

/// The styles file as written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StylesFile {
    #[garde(skip)]
    pub schema_version: u32,
    /// The spread of the advice's error on each style's estimate, on the 1 to 20 scale.
    #[garde(range(min = 0.0, max = 5.0))]
    pub advice_noise: f64,
    #[garde(length(min = 1, max = 16), dive)]
    pub styles: Vec<StyleEntry>,
}

/// One style as written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StyleEntry {
    #[garde(length(min = 1, max = 32))]
    pub name: String,
    #[garde(length(min = 1, max = 32))]
    pub mentality: String,
    /// Instruction name to level name; an instruction left out stays at its default.
    #[serde(default)]
    #[garde(skip)]
    pub instructions: BTreeMap<String, String>,
    /// Position to duty name, one or more; a position left out takes the AI's default duty.
    #[garde(length(min = 1, max = 10))]
    pub duties: BTreeMap<Position, String>,
    /// The formations the style plays, in preference order.
    #[garde(length(min = 1, max = 16))]
    pub formations: Vec<String>,
    /// The statistics that show the style, each with the direction it moves them.
    #[garde(length(min = 2, max = 3), dive)]
    pub identity: Vec<IdentityEntry>,
    /// The attributes the style's key jobs read, by weight.
    #[garde(length(min = 1, max = 12))]
    pub fit: BTreeMap<String, f64>,
    /// The positions whose players the fit reads; every outfield position when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub fit_positions: Option<Vec<Position>>,
    /// Game-state rules the preset plays by, in priority order, in the form of
    /// [`crate::data::game_state`]; none when absent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[garde(skip)]
    pub rules: Vec<crate::data::game_state::RuleFile>,
    /// Player instructions of the preset, by formation slot; none when absent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[garde(skip)]
    pub player_instructions: Vec<crate::data::game_state::InstructionFile>,
}

/// The direction a style moves an identity statistic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
}

/// One identity statistic as written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct IdentityEntry {
    #[garde(length(min = 1, max = 32))]
    pub stat: String,
    #[garde(skip)]
    pub direction: Direction,
    /// Read the other side's figure.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[garde(skip)]
    pub against: bool,
}

/// The styles with every name resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct Styles {
    pub advice_noise: f64,
    pub styles: Vec<Style>,
}

/// One resolved style.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    pub name: String,
    pub mentality: u8,
    /// A level per instruction in [`INSTRUCTIONS`] order; `None` keeps the default.
    pub instructions: [Option<u8>; 6],
    /// A duty per position in [`Position::ALL`] order; `None` takes the AI's default duty.
    pub duties: [Option<u8>; 10],
    /// Formation indices, in the file's order.
    pub formations: Vec<u8>,
    pub identity: Vec<IdentityEntry>,
    /// Attribute index and weight.
    pub fit: Vec<(usize, f64)>,
    /// The positions whose players the fit reads.
    pub fit_positions: Vec<Position>,
    /// The preset's game-state rules and player instructions.
    pub game_state: crate::tactics::rules::GameState,
}

impl Style {
    /// The style's tactics in `formation` (an index into the tactics file): its mentality and
    /// instruction levels over the defaults, each slot's first suitable role, and the duty hint
    /// of the slot's position (the AI's default duty where the style gives none).
    pub fn tactics(&self, formation: u8, schema: &TacticsSchema) -> Tactics {
        let mut t = Tactics::defaults(schema);
        let default_duty = schema.duty_index(&schema.ai.duty).unwrap_or(0) as u8;
        t.formation = formation;
        t.mentality = self.mentality;
        for (level, set) in t.instructions.iter_mut().zip(self.instructions) {
            if let Some(l) = set {
                *level = l;
            }
        }
        let slots = &schema.formations[usize::from(formation)].slots;
        let mut roles = [RoleDuty::default(); PLAYERS_PER_TEAM];
        for (rd, slot) in roles.iter_mut().zip(slots) {
            *rd = RoleDuty {
                role: schema.default_role(slot.position) as u8,
                duty: self.duties[slot.position as usize].unwrap_or(default_duty),
            };
        }
        t.roles = roles;
        t
    }
}

/// Loads and resolves a styles file's bytes against the tactics file and the attribute
/// schema. Every refusal names the field and the reason.
pub fn load_styles_bytes(
    bytes: &[u8],
    shown: &str,
    tactics: &TacticsSchema,
    attrs: &AttributeSchema,
) -> Result<Loaded<Styles>, EngineError> {
    const KIND: &str = "styles";
    let file = load_json_bytes::<StylesFile>(KIND, bytes, shown, STYLES_VERSION, &())?;
    let value = resolve(&file.value, tactics, attrs).map_err(|(field, reason)| {
        tracing::error!(signal = "content.refused", kind = KIND, path = shown, field = %field, reason = %reason);
        EngineError::Data {
            kind: KIND,
            path: shown.to_string(),
            field,
            reason,
        }
    })?;
    Ok(Loaded {
        value,
        digest: file.digest,
        converted_from: None,
    })
}

/// Resolves every name of `file`; a refusal is the field path and the reason.
pub fn resolve(
    file: &StylesFile,
    tactics: &TacticsSchema,
    attrs: &AttributeSchema,
) -> Result<Styles, (String, String)> {
    let mut styles = Vec::with_capacity(file.styles.len());
    for (i, s) in file.styles.iter().enumerate() {
        let at = |field: &str| format!("styles[{i}].{field}");
        if file.styles[..i].iter().any(|o| o.name == s.name) {
            return Err((at("name"), format!("{} appears twice", s.name)));
        }
        let mentality = tactics.mentality_index(&s.mentality).ok_or_else(|| {
            (
                at("mentality"),
                format!("{} is not a mentality", s.mentality),
            )
        })?;
        let mut instructions = [None; 6];
        for (name, level) in &s.instructions {
            let k = INSTRUCTIONS.iter().position(|n| n == name).ok_or_else(|| {
                (
                    at(&format!("instructions.{name}")),
                    format!("{name} is not an instruction"),
                )
            })?;
            let l = tactics
                .instructions
                .level_names(k)
                .iter()
                .position(|n| n == level)
                .ok_or_else(|| {
                    (
                        at(&format!("instructions.{name}")),
                        format!("{level} is not a level of {name}"),
                    )
                })?;
            instructions[k] = Some(l as u8);
        }
        let mut duties = [None; 10];
        for (position, duty) in &s.duties {
            let d = tactics.duty_index(duty).ok_or_else(|| {
                (
                    at(&format!("duties.{}", position.code())),
                    format!("{duty} is not a duty"),
                )
            })?;
            duties[*position as usize] = Some(d as u8);
        }
        if s.formations.is_empty() {
            return Err((at("formations"), "the formation list is empty".into()));
        }
        let mut formations = Vec::with_capacity(s.formations.len());
        for name in &s.formations {
            let f = tactics
                .formation_index(name)
                .ok_or_else(|| (at("formations"), format!("{name} is not a formation")))?;
            formations.push(f as u8);
        }
        if !(2..=3).contains(&s.identity.len()) {
            return Err((
                at("identity"),
                format!("{} statistics; allowed 2 or 3", s.identity.len()),
            ));
        }
        for id in &s.identity {
            if !IDENTITY_STATS.contains(&id.stat.as_str()) {
                return Err((
                    at("identity"),
                    format!("{} is not an identity statistic", id.stat),
                ));
            }
        }
        let mut fit = Vec::with_capacity(s.fit.len());
        for (name, w) in &s.fit {
            let field = at(&format!("fit.{name}"));
            let k = attrs
                .index(name)
                .ok_or_else(|| (field.clone(), format!("{name} is not an attribute")))?;
            let def = &attrs.attributes[k];
            if def.hidden
                || !matches!(
                    def.group,
                    Group::Technical | Group::Mental | Group::Physical
                )
            {
                return Err((field, format!("{name} is not a visible outfield attribute")));
            }
            if !(0.0..=1.0).contains(w) {
                return Err((field, format!("weight {w}; allowed 0 to 1")));
            }
            fit.push((k, *w));
        }
        if fit.iter().map(|(_, w)| w).sum::<f64>() <= 0.0 {
            return Err((at("fit"), "the weights sum to 0".into()));
        }
        let fit_positions = match &s.fit_positions {
            Some(list) if list.is_empty() || list.contains(&Position::GK) => {
                return Err((
                    at("fit_positions"),
                    "list one or more outfield positions".into(),
                ));
            }
            Some(list) => list.clone(),
            None => Position::ALL[1..].to_vec(),
        };
        let written = crate::data::game_state::GameStateFile {
            rules: s.rules.clone(),
            instructions: s.player_instructions.clone(),
        };
        let game_state = crate::data::game_state::resolve(&written, tactics, &formations)
            .map_err(|(field, reason)| (at(&field), reason))?;
        styles.push(Style {
            name: s.name.clone(),
            mentality: mentality as u8,
            instructions,
            duties,
            formations,
            identity: s.identity.clone(),
            fit,
            fit_positions,
            game_state,
        });
    }
    Ok(Styles {
        advice_noise: file.advice_noise,
        styles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_content;

    fn shipped_bytes() -> Vec<u8> {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/styles.json");
        std::fs::read(path).unwrap()
    }

    fn refusal(edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let content = shipped_content();
        let mut v: serde_json::Value = serde_json::from_slice(&shipped_bytes()).unwrap();
        edit(&mut v);
        load_styles_bytes(
            &serde_json::to_vec(&v).unwrap(),
            "styles.json",
            &content.tactics,
            &content.attributes,
        )
        .unwrap_err()
        .to_string()
    }

    #[test]
    fn the_shipped_styles_load_and_resolve() {
        let content = shipped_content();
        let loaded = load_styles_bytes(
            &shipped_bytes(),
            "styles.json",
            &content.tactics,
            &content.attributes,
        )
        .unwrap();
        assert_eq!(loaded.value.styles.len(), 8);
        for s in &loaded.value.styles {
            let t = s.tactics(s.formations[0], &content.tactics);
            assert_eq!(t.formation, s.formations[0]);
            assert_eq!(t.mentality, s.mentality);
        }
    }

    #[test]
    fn an_empty_formation_list_is_refused() {
        let err = refusal(|v| v["styles"][0]["formations"] = serde_json::json!([]));
        assert!(err.contains("formations"), "{err}");
    }

    #[test]
    fn an_unknown_formation_is_refused_by_name() {
        let err = refusal(|v| v["styles"][1]["formations"][0] = "2-2-6".into());
        assert!(err.contains("2-2-6 is not a formation"), "{err}");
    }

    #[test]
    fn an_unknown_mentality_level_duty_or_attribute_is_refused_by_name() {
        let err = refusal(|v| v["styles"][0]["mentality"] = "reckless".into());
        assert!(err.contains("reckless is not a mentality"), "{err}");
        let err = refusal(|v| v["styles"][0]["instructions"]["tempo"] = "frantic".into());
        assert!(err.contains("frantic is not a level of tempo"), "{err}");
        let err = refusal(|v| v["styles"][0]["instructions"]["swagger"] = "high".into());
        assert!(err.contains("swagger is not an instruction"), "{err}");
        let err = refusal(|v| v["styles"][0]["duties"]["ST"] = "loiter".into());
        assert!(err.contains("loiter is not a duty"), "{err}");
        let err = refusal(|v| v["styles"][0]["duties"]["XX"] = "attack".into());
        assert!(err.contains("XX"), "{err}");
        let err = refusal(|v| v["styles"][0]["fit"]["charm"] = 0.5.into());
        assert!(err.contains("charm is not an attribute"), "{err}");
        let err = refusal(|v| v["styles"][0]["fit"]["consistency"] = 0.5.into());
        assert!(err.contains("not a visible outfield attribute"), "{err}");
    }

    #[test]
    fn an_identity_list_outside_two_to_three_or_an_unknown_statistic_is_refused() {
        let err = refusal(|v| {
            let first = v["styles"][0]["identity"][0].clone();
            v["styles"][0]["identity"] = serde_json::json!([first]);
        });
        assert!(err.contains("identity"), "{err}");
        let err = refusal(|v| {
            let first = v["styles"][0]["identity"][0].clone();
            v["styles"][0]["identity"] = serde_json::json!([first, first, first, first]);
        });
        assert!(err.contains("identity"), "{err}");
        let err = refusal(|v| v["styles"][0]["identity"][0]["stat"] = "style_points".into());
        assert!(
            err.contains("style_points is not an identity statistic"),
            "{err}"
        );
    }

    #[test]
    fn bad_fit_weights_and_a_duplicate_name_are_refused() {
        let err = refusal(|v| v["styles"][0]["fit"] = serde_json::json!({"passing": 0.0}));
        assert!(err.contains("sum to 0"), "{err}");
        let err = refusal(|v| v["styles"][0]["fit"] = serde_json::json!({"passing": -0.5}));
        assert!(err.contains("allowed 0 to 1"), "{err}");
        let err = refusal(|v| v["styles"][1]["name"] = v["styles"][0]["name"].clone());
        assert!(err.contains("appears twice"), "{err}");
        let err = refusal(|v| v["styles"] = serde_json::json!([]));
        assert!(err.contains("styles"), "{err}");
        let err = refusal(|v| v["styles"][0]["fit_positions"] = serde_json::json!(["GK"]));
        assert!(err.contains("outfield"), "{err}");
        let err = refusal(|v| v["styles"][0]["duties"] = serde_json::json!({}));
        assert!(err.contains("duties"), "{err}");
    }
}

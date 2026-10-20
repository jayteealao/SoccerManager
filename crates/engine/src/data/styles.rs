//! The named styles file (`content/styles.json`): each style is a preset of tactics
//! primitives (a list of primitive settings in the form of a rule action, the mentality
//! among them), duty hints by position, the formations it plays, the two or three statistics
//! that show its identity, the attributes its key jobs read, and its own game-state rules.
//! The AI manager's pre-match advice weighs a squad against each style
//! ([`crate::ai::style_estimates`]); the styles suite plays them against each other. The file
//! is optional: content without it keeps the tactics file's default AI tactics.
//!
//! Loading resolves every name against the tactics file and the attribute schema; an unknown
//! formation, primitive, level, choice, state, duty, position or attribute is refused by name,
//! as is a primitive set twice, a preset without a mentality setting, a preset with no rule or
//! more rules than the tactics file allows, an empty formation list, an identity list outside
//! 2 to 3, an unknown identity statistic, or fit weights that sum to 0.
//!
//! A version 1 file (a mentality and an instruction map in place of the settings list, as
//! older replay files carry it) converts on load: the mentality and each instruction become
//! settings, so it plays the same tactics.

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::attributes::{AttributeSchema, Group};
use crate::data::tactics::TacticsSchema;
use crate::data::team::Position;
use crate::data::{Loaded, load_json_bytes};
use crate::error::EngineError;
use crate::tactics::{RoleDuty, Tactics};
use crate::team::PLAYERS_PER_TEAM;

/// Schema version this build reads.
pub const STYLES_VERSION: u32 = 2;

/// The version 1 form, which loads by conversion.
pub const STYLES_V1: u32 = 1;

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
    /// The preset's primitive settings, each a primitive and its level, choice or state (the
    /// form of a rule action); the mentality is one of them and must be set. A primitive left
    /// out stays at its default.
    #[garde(length(min = 1, max = 32), dive)]
    pub primitives: Vec<Setting>,
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
    /// [`crate::data::game_state`]: one or more, up to the tactics file's limit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[garde(skip)]
    pub rules: Vec<crate::data::game_state::RuleFile>,
    /// Player instructions of the preset, by formation slot; none when absent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[garde(skip)]
    pub player_instructions: Vec<crate::data::game_state::InstructionFile>,
}

/// One primitive setting of a preset as written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Setting {
    #[garde(length(min = 1, max = 32))]
    pub primitive: String,
    #[garde(length(min = 1, max = 32))]
    pub level: String,
}

/// The version 1 styles file: a style had a mentality and an instruction map in place of
/// the settings list.
#[derive(Debug, Clone, PartialEq, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StylesFileV1 {
    #[garde(skip)]
    pub schema_version: u32,
    #[garde(range(min = 0.0, max = 5.0))]
    pub advice_noise: f64,
    #[garde(length(min = 1, max = 16), dive)]
    pub styles: Vec<StyleEntryV1>,
}

/// One version 1 style.
#[derive(Debug, Clone, PartialEq, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StyleEntryV1 {
    #[garde(length(min = 1, max = 32))]
    pub name: String,
    #[garde(length(min = 1, max = 32))]
    pub mentality: String,
    /// Instruction name to level name; an instruction left out stays at its default.
    #[serde(default)]
    #[garde(skip)]
    pub instructions: BTreeMap<String, String>,
    #[garde(length(min = 1, max = 10))]
    pub duties: BTreeMap<Position, String>,
    #[garde(length(min = 1, max = 16))]
    pub formations: Vec<String>,
    #[garde(length(min = 2, max = 3), dive)]
    pub identity: Vec<IdentityEntry>,
    #[garde(length(min = 1, max = 12))]
    pub fit: BTreeMap<String, f64>,
    #[serde(default)]
    #[garde(skip)]
    pub fit_positions: Option<Vec<Position>>,
    #[serde(default)]
    #[garde(skip)]
    pub rules: Vec<crate::data::game_state::RuleFile>,
    #[serde(default)]
    #[garde(skip)]
    pub player_instructions: Vec<crate::data::game_state::InstructionFile>,
}

/// A version 1 file in the current form: the mentality first, then each instruction as the
/// setting of the primitive that owns the instruction's levels. An instruction no primitive
/// owns is refused by name.
pub fn styles_v1_to_v2(
    v1: StylesFileV1,
    tactics: &TacticsSchema,
) -> Result<StylesFile, (String, String)> {
    let mut styles = Vec::with_capacity(v1.styles.len());
    let mentality = tactics
        .primitives
        .iter()
        .find(|p| p.levels.as_deref() == Some(MENTALITY_BLOCK))
        .map(|p| p.name.clone())
        .ok_or_else(|| {
            (
                "styles".to_string(),
                "the tactics file has no mentality primitive".to_string(),
            )
        })?;
    for (i, s) in v1.styles.into_iter().enumerate() {
        let mut primitives = vec![Setting {
            primitive: mentality.clone(),
            level: s.mentality,
        }];
        for (name, level) in s.instructions {
            let block = format!("instructions.{name}");
            let p = tactics
                .primitives
                .iter()
                .find(|p| p.levels.as_deref() == Some(block.as_str()))
                .ok_or_else(|| {
                    (
                        format!("styles[{i}].instructions.{name}"),
                        format!("{name} is not an instruction"),
                    )
                })?;
            primitives.push(Setting {
                primitive: p.name.clone(),
                level,
            });
        }
        styles.push(StyleEntry {
            name: s.name,
            primitives,
            duties: s.duties,
            formations: s.formations,
            identity: s.identity,
            fit: s.fit,
            fit_positions: s.fit_positions,
            rules: s.rules,
            player_instructions: s.player_instructions,
        });
    }
    Ok(StylesFile {
        schema_version: STYLES_VERSION,
        advice_noise: v1.advice_noise,
        styles,
    })
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

/// The level block of the mentality primitive.
const MENTALITY_BLOCK: &str = "mentalities";

/// One resolved style.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    pub name: String,
    /// The preset's settings in the file's order: a primitive index into the tactics file
    /// and its level, choice or state. The mentality is among them.
    pub primitives: Vec<(u8, u8)>,
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
    /// The style's tactics in `formation` (an index into the tactics file): its primitive
    /// settings over the defaults, each set as a rule action sets it, each slot's first
    /// suitable role, and the duty hint of the slot's position (the AI's default duty where
    /// the style gives none).
    pub fn tactics(&self, formation: u8, schema: &TacticsSchema) -> Tactics {
        let mut t = Tactics::defaults(schema);
        for &(primitive, level) in &self.primitives {
            t = t.with_level(usize::from(primitive), level, schema);
        }
        let default_duty = schema.duty_index(&schema.ai.duty).unwrap_or(0) as u8;
        t.formation = formation;
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
/// schema. Every refusal names the field and the reason. A version 1 file converts first
/// ([`styles_v1_to_v2`]) and loads with `converted_from` 1; it may hold no rule, as files
/// written before presets carried rules do. The digest is that of the bytes read.
pub fn load_styles_bytes(
    bytes: &[u8],
    shown: &str,
    tactics: &TacticsSchema,
    attrs: &AttributeSchema,
) -> Result<Loaded<Styles>, EngineError> {
    const KIND: &str = "styles";
    let refuse = |(field, reason): (String, String)| {
        tracing::error!(signal = "content.refused", kind = KIND, path = shown, field = %field, reason = %reason);
        EngineError::Data {
            kind: KIND,
            path: shown.to_string(),
            field,
            reason,
        }
    };
    if super::peek_version(KIND, bytes, shown)? == STYLES_V1 {
        let v1: StylesFileV1 = super::parse_checked(KIND, bytes, shown, &())?;
        let file = styles_v1_to_v2(v1, tactics).map_err(refuse)?;
        let value = resolve_with(&file, tactics, attrs, false).map_err(refuse)?;
        let loaded = super::loaded(KIND, bytes, shown, value, STYLES_V1, Some(STYLES_V1));
        return Ok(loaded);
    }
    let file = load_json_bytes::<StylesFile>(KIND, bytes, shown, STYLES_VERSION, &())?;
    let value = resolve(&file.value, tactics, attrs).map_err(refuse)?;
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
    resolve_with(file, tactics, attrs, true)
}

/// [`resolve`], with the rule list required to be non-empty or not (a converted version 1
/// file may hold none).
fn resolve_with(
    file: &StylesFile,
    tactics: &TacticsSchema,
    attrs: &AttributeSchema,
    rules_required: bool,
) -> Result<Styles, (String, String)> {
    let mut styles = Vec::with_capacity(file.styles.len());
    for (i, s) in file.styles.iter().enumerate() {
        let at = |field: &str| format!("styles[{i}].{field}");
        if file.styles[..i].iter().any(|o| o.name == s.name) {
            return Err((at("name"), format!("{} appears twice", s.name)));
        }
        let mut primitives = Vec::with_capacity(s.primitives.len());
        for (k, setting) in s.primitives.iter().enumerate() {
            let field = at(&format!("primitives[{k}]"));
            if s.primitives[..k]
                .iter()
                .any(|o| o.primitive == setting.primitive)
            {
                return Err((
                    field,
                    format!("{} is set twice in the preset", setting.primitive),
                ));
            }
            primitives.push(crate::data::game_state::level_of(
                tactics,
                &setting.primitive,
                &setting.level,
                &field,
            )?);
        }
        let sets_mentality = primitives.iter().any(|&(p, _)| {
            tactics.primitives[usize::from(p)].levels.as_deref() == Some(MENTALITY_BLOCK)
        });
        if !sets_mentality {
            return Err((
                at("primitives"),
                "a preset sets the mentality; add a mentality setting".into(),
            ));
        }
        if rules_required && s.rules.is_empty() {
            return Err((
                at("rules"),
                "a preset holds one or more game-state rules".into(),
            ));
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
            primitives,
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
        assert_eq!(loaded.converted_from, None);
        for s in &loaded.value.styles {
            let t = s.tactics(s.formations[0], &content.tactics);
            assert_eq!(t.formation, s.formations[0]);
            for &(p, l) in &s.primitives {
                let again = t.with_level(usize::from(p), l, &content.tactics);
                assert_eq!(again, t, "{}: setting {p} is applied", s.name);
            }
        }
    }

    fn fixture(name: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data")
            .join(name);
        std::fs::read(path).unwrap()
    }

    /// A version 1 file (the styles of the last version 1 content) converts, loads with
    /// `converted_from` 1, and gives every style the same tactics in every formation as its
    /// version 2 rewrite.
    #[test]
    fn a_version_1_file_converts_to_the_same_tactics() {
        let content = shipped_content();
        let load = |bytes: &[u8]| {
            load_styles_bytes(bytes, "styles.json", &content.tactics, &content.attributes).unwrap()
        };
        let old = load(&fixture("styles-v1.json"));
        let new = load(&fixture("styles-v1-as-v2.json"));
        assert_eq!(old.converted_from, Some(STYLES_V1));
        assert_eq!(new.converted_from, None);
        assert_eq!(old.value.styles.len(), new.value.styles.len());
        for (a, b) in old.value.styles.iter().zip(&new.value.styles) {
            // The instruction map of version 1 has no order; the settings agree as a set.
            let sorted = |s: &Style| {
                let mut s = s.clone();
                s.primitives.sort_unstable();
                s
            };
            assert_eq!(sorted(a), sorted(b), "{}", a.name);
            for f in 0..content.tactics.formations.len() as u8 {
                assert_eq!(
                    a.tactics(f, &content.tactics),
                    b.tactics(f, &content.tactics),
                    "{} in formation {f}",
                    a.name
                );
            }
        }
        // A version 1 file without rules still loads; a version 2 preset needs one.
        let mut v: serde_json::Value = serde_json::from_slice(&fixture("styles-v1.json")).unwrap();
        for s in v["styles"].as_array_mut().unwrap() {
            s.as_object_mut().unwrap().remove("rules");
        }
        assert!(
            load(&serde_json::to_vec(&v).unwrap()).value.styles[0]
                .game_state
                .rules
                .is_empty()
        );
        v["styles"][0]["instructions"]["swagger"] = "high".into();
        let err = load_styles_bytes(
            &serde_json::to_vec(&v).unwrap(),
            "styles.json",
            &content.tactics,
            &content.attributes,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("swagger is not an instruction"), "{err}");
    }

    #[test]
    fn a_bad_preset_setting_is_refused_by_name() {
        let set = |v: &mut serde_json::Value, k: usize, p: &str, l: &str| {
            v["styles"][0]["primitives"][k] = serde_json::json!({ "primitive": p, "level": l });
        };
        let err = refusal(|v| set(v, 0, "mentality", "reckless"));
        assert!(
            err.contains("reckless is not a level of mentality"),
            "{err}"
        );
        assert!(err.contains("styles[0].primitives[0].level"), "{err}");
        let err = refusal(|v| set(v, 1, "tempo", "frantic"));
        assert!(err.contains("frantic is not a level of tempo"), "{err}");
        let err = refusal(|v| set(v, 1, "swagger", "high"));
        assert!(err.contains("swagger is not a primitive"), "{err}");
        let err = refusal(|v| set(v, 1, "attacking_channel", "through_the_middle"));
        assert!(
            err.contains("through_the_middle is not a level of attacking_channel"),
            "{err}"
        );
        let err = refusal(|v| set(v, 1, "offside_trap", "sometimes"));
        assert!(
            err.contains("sometimes is not a level of offside_trap"),
            "{err}"
        );
        let err = refusal(|v| set(v, 1, "mentality", "balanced"));
        assert!(err.contains("mentality is set twice"), "{err}");
        let err = refusal(|v| set(v, 0, "tackling", "careful"));
        assert!(err.contains("a preset sets the mentality"), "{err}");
        let err = refusal(|v| {
            v["styles"][0]["primitives"][1] = serde_json::json!({ "formation": "4-4-2" });
        });
        assert!(err.contains("unknown field `formation`"), "{err}");
        let err = refusal(|v| v["styles"][0]["primitives"] = serde_json::json!([]));
        assert!(err.contains("primitives"), "{err}");
    }

    #[test]
    fn a_preset_with_no_rule_too_many_rules_or_a_bad_condition_is_refused() {
        let err = refusal(|v| v["styles"][0]["rules"] = serde_json::json!([]));
        assert!(err.contains("one or more game-state rules"), "{err}");
        let err = refusal(|v| {
            v["styles"][0].as_object_mut().unwrap().remove("rules");
        });
        assert!(err.contains("styles[0].rules"), "{err}");
        let err = refusal(|v| {
            let rule = v["styles"][0]["rules"][0].clone();
            v["styles"][0]["rules"] = serde_json::Value::Array(vec![rule; 7]);
        });
        assert!(err.contains("7 rules; a tactic holds at most 6"), "{err}");
        let err = refusal(|v| v["styles"][0]["rules"][0]["when"][0]["kind"] = "weather".into());
        assert!(err.contains("weather is not a condition kind"), "{err}");
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
    fn an_unknown_duty_position_or_attribute_is_refused_by_name() {
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

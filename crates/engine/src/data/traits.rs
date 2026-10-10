//! The trait catalogue (`content/traits.json`): the traits of play a player may carry, such as
//! cutting inside or shooting from distance. Each trait names the engine choice it acts on (its
//! hook) and, when it has one, its opposite, which no player may carry beside it. A player
//! carries a trait at a strength: moderate or strong.
//!
//! This build loads and checks the catalogue; the team files and play read it in later work.

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::EngineError;
use crate::data::{Loaded, load_json_bytes};

/// Schema version this build reads.
pub const TRAITS_VERSION: u32 = 1;

/// How strongly a player carries a trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraitStrength {
    Moderate,
    Strong,
}

/// One trait of play.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct TraitDef {
    #[garde(length(min = 2, max = 40))]
    pub name: String,
    /// The trait no player may carry beside this one; it names this one back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(skip)]
    pub opposite: Option<String>,
    /// The engine choice the trait acts on, in snake case.
    #[garde(skip)]
    pub hook: String,
}

/// The trait catalogue file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct TraitCatalogue {
    #[garde(skip)]
    pub schema_version: u32,
    /// The strengths a player may carry a trait at.
    #[garde(length(min = 1, max = 2), custom(check_strengths))]
    pub strengths: Vec<TraitStrength>,
    #[garde(length(min = 1, max = 64), dive, custom(check_traits))]
    pub traits: Vec<TraitDef>,
}

impl TraitCatalogue {
    /// The trait named `name`, or `None`.
    pub fn get(&self, name: &str) -> Option<&TraitDef> {
        self.traits.iter().find(|t| t.name == name)
    }
}

fn check_strengths(strengths: &[TraitStrength], _ctx: &()) -> garde::Result {
    for (i, s) in strengths.iter().enumerate() {
        if strengths[..i].contains(s) {
            return Err(garde::Error::new(
                format!("{s:?} appears twice").to_lowercase(),
            ));
        }
    }
    Ok(())
}

/// `true` for a non-empty snake-case word: lower-case letters and digits joined by single
/// underscores.
fn is_snake_case(s: &str) -> bool {
    !s.is_empty()
        && s.split('_').all(|w| {
            !w.is_empty()
                && w.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// Each trait once, with a snake-case hook; an opposite names another trait of the file, which
/// names this one back.
fn check_traits(traits: &[TraitDef], _ctx: &()) -> garde::Result {
    let refuse = |name: &str, why: String| Err(garde::Error::new(format!("trait {name}: {why}")));
    for (i, t) in traits.iter().enumerate() {
        if traits[..i].iter().any(|u| u.name == t.name) {
            return refuse(&t.name, "appears twice".into());
        }
        if !is_snake_case(&t.hook) {
            return refuse(
                &t.name,
                format!("hook \"{}\" must be a snake-case engine choice", t.hook),
            );
        }
        let Some(opposite) = &t.opposite else {
            continue;
        };
        if *opposite == t.name {
            return refuse(&t.name, "is its own opposite".into());
        }
        let Some(other) = traits.iter().find(|u| u.name == *opposite) else {
            return refuse(&t.name, format!("its opposite {opposite} is not a trait"));
        };
        if other.opposite.as_deref() != Some(t.name.as_str()) {
            return refuse(
                &t.name,
                format!("its opposite {opposite} does not name it back"),
            );
        }
    }
    Ok(())
}

/// Loads and checks the trait catalogue's bytes. A refusal names the trait and the rule.
pub fn load_traits_bytes(bytes: &[u8], shown: &str) -> Result<Loaded<TraitCatalogue>, EngineError> {
    load_json_bytes::<TraitCatalogue>("traits", bytes, shown, TRAITS_VERSION, &())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> serde_json::Value {
        let path = crate::data::test_support::shipped_dir().path("traits.json");
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    fn refusal(edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut v = shipped();
        edit(&mut v);
        load_traits_bytes(&serde_json::to_vec(&v).unwrap(), "traits.json")
            .unwrap_err()
            .to_string()
    }

    fn trait_of<'a>(v: &'a mut serde_json::Value, name: &str) -> &'a mut serde_json::Value {
        v["traits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|t| t["name"] == name)
            .unwrap()
    }

    #[test]
    fn the_shipped_catalogue_loads_with_thirty_one_traits_and_nine_opposite_pairs() {
        let c = load_traits_bytes(&serde_json::to_vec(&shipped()).unwrap(), "traits.json")
            .unwrap()
            .value;
        assert_eq!(c.traits.len(), 31);
        let with_opposite = c.traits.iter().filter(|t| t.opposite.is_some()).count();
        assert_eq!(with_opposite, 18, "nine pairs, each named from both sides");
        assert_eq!(
            c.strengths,
            [TraitStrength::Moderate, TraitStrength::Strong]
        );
        assert_eq!(
            c.get("cuts_inside").unwrap().hook,
            "dribble_direction_from_wide"
        );
    }

    #[test]
    fn a_duplicate_trait_is_refused_by_name() {
        let text = refusal(|v| {
            let first = v["traits"][0].clone();
            v["traits"].as_array_mut().unwrap().push(first);
        });
        assert!(text.contains("trait cuts_inside: appears twice"), "{text}");
    }

    #[test]
    fn an_unknown_opposite_is_refused_by_name() {
        let text = refusal(|v| trait_of(v, "switches_play")["opposite"] = "plays_it_safe".into());
        assert!(
            text.contains("trait switches_play: its opposite plays_it_safe is not a trait"),
            "{text}"
        );
    }

    #[test]
    fn a_one_sided_opposite_is_refused_by_name() {
        let text = refusal(|v| {
            trait_of(v, "hugs_touchline")
                .as_object_mut()
                .unwrap()
                .remove("opposite");
        });
        assert!(
            text.contains("trait cuts_inside: its opposite hugs_touchline does not name it back"),
            "{text}"
        );
    }

    #[test]
    fn a_trait_that_is_its_own_opposite_is_refused_by_name() {
        let text = refusal(|v| trait_of(v, "crosses_early")["opposite"] = "crosses_early".into());
        assert!(
            text.contains("trait crosses_early: is its own opposite"),
            "{text}"
        );
    }

    #[test]
    fn an_empty_or_badly_formed_hook_is_refused_by_name() {
        let text = refusal(|v| trait_of(v, "holds_up_ball")["hook"] = "".into());
        assert!(text.contains("trait holds_up_ball: hook"), "{text}");
        let text = refusal(|v| trait_of(v, "holds_up_ball")["hook"] = "Hold Choice".into());
        assert!(text.contains("trait holds_up_ball: hook"), "{text}");
    }

    #[test]
    fn another_version_is_refused() {
        let text = refusal(|v| v["schema_version"] = 2.into());
        assert!(text.contains('2'), "{text}");
    }
}

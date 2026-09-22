//! The attribute schema file: 30 to 50 named attributes in four groups, each on the 1 to 100
//! scale. Seven names are required because the engine reads them by index after load.

use garde::Validate;
use serde::{Deserialize, Serialize};

/// Schema version this build reads.
pub const ATTRIBUTES_VERSION: u32 = 1;
/// Upper bound on the attribute count; `Attributes` on a player is a fixed array of this size.
pub const MAX_ATTRIBUTES: usize = 50;
/// Lower bound on the attribute count.
pub const MIN_ATTRIBUTES: usize = 30;
/// Attributes the engine reads by name at load time, in the order `Derived` consumes them.
pub const REQUIRED: [&str; 7] = [
    "pace",
    "acceleration",
    "passing",
    "dribbling",
    "tackling",
    "positioning",
    "aggression",
];

/// The four attribute groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    Technical,
    Mental,
    Physical,
    Goalkeeping,
}

/// One attribute definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct AttributeDef {
    #[garde(length(min = 2, max = 32))]
    pub name: String,
    #[garde(skip)]
    pub group: Group,
}

/// The attribute schema file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct AttributeSchema {
    #[garde(skip)]
    pub schema_version: u32,
    #[garde(length(min = 30, max = 50), dive, custom(check_names))]
    pub attributes: Vec<AttributeDef>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(names: &[&str]) -> AttributeSchema {
        AttributeSchema {
            schema_version: ATTRIBUTES_VERSION,
            attributes: names
                .iter()
                .map(|n| AttributeDef {
                    name: (*n).to_string(),
                    group: Group::Technical,
                })
                .collect(),
        }
    }

    #[test]
    fn thirty_names_with_the_required_seven_validate() {
        let mut names: Vec<String> = REQUIRED.iter().map(|s| s.to_string()).collect();
        for i in 0..23 {
            names.push(format!("attr_{i}"));
        }
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let s = schema(&refs);
        assert!(s.validate().is_ok());
        assert_eq!(s.required_indices(), [0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn a_schema_without_aggression_is_refused_by_name() {
        let mut names: Vec<String> = REQUIRED[..6].iter().map(|s| s.to_string()).collect();
        for i in 0..24 {
            names.push(format!("attr_{i}"));
        }
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let text = schema(&refs).validate().unwrap_err().to_string();
        assert!(
            text.contains("attributes: required attribute aggression is missing"),
            "{text}"
        );
    }

    #[test]
    fn a_missing_required_name_is_refused_by_name() {
        let names: Vec<String> = (0..30).map(|i| format!("attr_{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let report = schema(&refs).validate().unwrap_err();
        let text = report.to_string();
        assert!(
            text.contains("attributes: required attribute pace is missing"),
            "{text}"
        );
    }

    #[test]
    fn too_few_attributes_are_refused() {
        let report = schema(&REQUIRED).validate().unwrap_err();
        assert!(
            report.to_string().contains("length is lower than 30"),
            "{report}"
        );
    }
}

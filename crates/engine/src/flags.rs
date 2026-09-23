//! Feature flags in the tuning file. A flag switches between a current model and a
//! candidate, so that a paired calibration run can compare the two on the realism bands.
//!
//! A flag is declared in the `flags` block of `tuning.json` with an owner, a hypothesis,
//! and a removal condition. It has an effect in one of two ways: `overrides` patch tuning
//! values while it is on, or its name is registered in [`CODE_FLAGS`] and engine code reads
//! it through [`ActiveFlags::is_on`]. Flags resolve once, when the content loads; nothing
//! reads a flag's state during a tick.

use std::collections::BTreeMap;
use std::str::FromStr;

use garde::Validate;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::data::TUNING_FILE;
use crate::data::tuning::TuningFile;
use crate::error::EngineError;

/// Flags that engine code reads through [`ActiveFlags::is_on`]. A candidate model adds its
/// flag here and declares it in the shipped `tuning.json` in the same change; removing the
/// model removes the name.
pub const CODE_FLAGS: &[&str] = &[];

/// The longest flag name, in characters.
pub const MAX_FLAG_NAME: usize = 40;

/// Whether a flag is on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlagState {
    #[default]
    Off,
    On,
}

impl FlagState {
    pub fn code(self) -> &'static str {
        match self {
            FlagState::Off => "off",
            FlagState::On => "on",
        }
    }
}

impl FromStr for FlagState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "on" => Ok(FlagState::On),
            "off" => Ok(FlagState::Off),
            other => Err(format!("state {other} is not on or off")),
        }
    }
}

/// One flag in the `flags` block. The three text fields are required: a flag without an
/// owner, a hypothesis, or a removal condition is refused by name when the file loads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct FlagDef {
    /// Who decides the flag's fate.
    #[serde(default)]
    #[garde(custom(required_text))]
    pub owner: String,
    /// What the candidate is expected to change, and by how much.
    #[serde(default)]
    #[garde(custom(required_text))]
    pub hypothesis: String,
    /// When the flag is removed, whatever the result.
    #[serde(default)]
    #[garde(custom(required_text))]
    pub removal_condition: String,
    /// The state the file sets; a command-line state takes precedence.
    #[serde(default)]
    #[garde(skip)]
    pub state: FlagState,
    /// Tuning values set while the flag is on, by dotted path (`engine.shot_range`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[garde(skip)]
    pub overrides: BTreeMap<String, Value>,
}

fn required_text(value: &str, _ctx: &()) -> garde::Result {
    if value.trim().is_empty() {
        return Err(garde::Error::new("is required"));
    }
    Ok(())
}

/// The rule on the whole `flags` block: every name is lower snake case of at most 40
/// characters, and every flag switches something.
pub(crate) fn each_flag_switches_something(
    flags: &BTreeMap<String, FlagDef>,
    _ctx: &(),
) -> garde::Result {
    for (name, def) in flags {
        let snake = !name.is_empty()
            && name.len() <= MAX_FLAG_NAME
            && name.starts_with(|c: char| c.is_ascii_lowercase())
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if !snake {
            return Err(garde::Error::new(format!(
                "flag {name}: the name must be lower snake case, at most {MAX_FLAG_NAME} characters"
            )));
        }
        if def.overrides.is_empty() && !CODE_FLAGS.contains(&name.as_str()) {
            return Err(garde::Error::new(format!(
                "flag {name} switches nothing: add overrides or register it in code"
            )));
        }
    }
    Ok(())
}

/// Flag states given on the command line, by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlagStates(pub BTreeMap<String, FlagState>);

impl FlagStates {
    /// The states of `settings`, each `name=on` or `name=off`; a later setting of the same
    /// name wins.
    pub fn from_settings(settings: &[FlagSetting]) -> Self {
        Self(settings.iter().map(|s| (s.name.clone(), s.state)).collect())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// One `name=on|off` setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagSetting {
    pub name: String,
    pub state: FlagState,
}

impl FromStr for FlagSetting {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, state) = s
            .split_once('=')
            .ok_or_else(|| format!("{s} is not NAME=on or NAME=off"))?;
        if name.is_empty() {
            return Err(format!("{s} names no flag"));
        }
        Ok(Self {
            name: name.to_string(),
            state: state.parse()?,
        })
    }
}

impl std::fmt::Display for FlagSetting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.name, self.state.code())
    }
}

/// The flags that are on for one match, sorted by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveFlags(Vec<String>);

impl ActiveFlags {
    /// Whether the code flag `name` is on. Only a name in [`CODE_FLAGS`] may be asked.
    pub fn is_on(&self, name: &str) -> bool {
        debug_assert!(CODE_FLAGS.contains(&name), "{name} is not a code flag");
        self.0.binary_search_by(|n| n.as_str().cmp(name)).is_ok()
    }

    /// The names that are on, sorted.
    pub fn names(&self) -> &[String] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Where a flag's effective state came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateSource {
    File,
    Cli,
}

impl StateSource {
    pub fn code(self) -> &'static str {
        match self {
            StateSource::File => "file",
            StateSource::Cli => "cli",
        }
    }
}

/// The effective state of every declared flag, by name.
pub fn effective(
    file: &TuningFile,
    states: &FlagStates,
) -> Result<BTreeMap<String, (FlagState, StateSource)>, EngineError> {
    if let Some(name) = states.0.keys().find(|n| !file.flags.contains_key(*n)) {
        return Err(refused(
            format!("flags.{name}"),
            "not declared in tuning.json".into(),
        ));
    }
    Ok(file
        .flags
        .iter()
        .map(|(name, def)| {
            let resolved = match states.0.get(name) {
                Some(&s) => (s, StateSource::Cli),
                None => (def.state, StateSource::File),
            };
            (name.clone(), resolved)
        })
        .collect())
}

/// The tuning file with every flag that is on applied, and the list of those flags.
///
/// The effective state of a flag is its command-line state when one is given, otherwise its
/// file state. Every override of a flag that is on is written into a copy of the file, and
/// the copy is parsed and validated again, so a flagged value meets the same bound as a
/// written one. Each declared flag is first tried alone, whatever its state, so a bad
/// override is refused when the file loads and not only when the flag is switched on. A
/// state for an undeclared name, an override path that does not exist or that points at
/// `schema_version` or `flags`, a value of the wrong JSON type, a value out of its bound,
/// and two flags that are on and set the same path are refused by name.
pub fn apply(
    file: &TuningFile,
    states: &FlagStates,
) -> Result<(TuningFile, ActiveFlags), EngineError> {
    let resolved = effective(file, states)?;
    if file.flags.is_empty() {
        return Ok((file.clone(), ActiveFlags::default()));
    }
    let on: Vec<String> = resolved
        .iter()
        .filter(|(_, (s, _))| *s == FlagState::On)
        .map(|(n, _)| n.clone())
        .collect();
    let base = serde_json::to_value(file).map_err(|e| EngineError::Format(e.to_string()))?;
    for name in file.flags.keys() {
        patch(&base, file, &resolved, std::slice::from_ref(name))?;
    }
    let patched = patch(&base, file, &resolved, &on)?;
    Ok((patched, ActiveFlags(on)))
}

/// `base` with the overrides of the flags `names` written in and every flag's state set to
/// its effective state, parsed and validated as a tuning file.
fn patch(
    base: &Value,
    file: &TuningFile,
    resolved: &BTreeMap<String, (FlagState, StateSource)>,
    names: &[String],
) -> Result<TuningFile, EngineError> {
    let mut tree = base.clone();
    let mut owner_of: BTreeMap<&str, &str> = BTreeMap::new();
    for name in names {
        for (path, value) in &file.flags[name].overrides {
            let field = format!("flags.{name}.overrides.{path}");
            if let Some(other) = owner_of.insert(path, name) {
                return Err(refused(
                    field,
                    format!("flags {other} and {name} both set {path}"),
                ));
            }
            let first = path.split('.').next().unwrap_or_default();
            if first == "schema_version" || first == "flags" {
                return Err(refused(field, "a flag cannot set this path".into()));
            }
            let slot = locate(&mut tree, path)
                .ok_or_else(|| refused(field.clone(), "no such tuning value".into()))?;
            if json_kind(slot) != json_kind(value) {
                return Err(refused(
                    field,
                    format!(
                        "a {} where the file holds a {}",
                        json_kind(value),
                        json_kind(slot)
                    ),
                ));
            }
            *slot = value.clone();
        }
    }
    for (name, (state, _)) in resolved {
        tree["flags"][name]["state"] = Value::String(state.code().to_string());
    }
    // The flag whose override the error path points into, else every flag applied.
    let who = |path: &str| -> String {
        owner_of
            .iter()
            .find(|(p, _)| {
                path == **p
                    || path.starts_with(&format!("{p}."))
                    || p.starts_with(&format!("{path}."))
            })
            .map_or_else(|| names.join(", "), |(_, n)| (*n).to_string())
    };
    let patched: TuningFile = serde_json::from_value(tree)
        .map_err(|e| refused(format!("flags.{}", names.join(", ")), e.to_string()))?;
    if let Err(report) = patched.validate() {
        let (path, message) = report
            .iter()
            .next()
            .map(|(p, e)| (p.to_string(), e.message().to_string()))
            .unwrap_or_default();
        return Err(refused(
            format!("flags.{}", who(&path)),
            format!("{path}: {message}"),
        ));
    }
    Ok(patched)
}

/// The value at a dotted path: object keys, or array indexes as numbers.
fn locate<'a>(tree: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    path.split('.').try_fold(tree, |node, key| match node {
        Value::Object(map) => map.get_mut(key),
        Value::Array(items) => key.parse::<usize>().ok().and_then(|i| items.get_mut(i)),
        _ => None,
    })
}

fn json_kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn refused(field: String, reason: String) -> EngineError {
    tracing::error!(signal = "content.refused", kind = "tuning", path = TUNING_FILE, field = %field, reason = %reason);
    EngineError::Data {
        kind: "tuning",
        path: TUNING_FILE.to_string(),
        field,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_setting_parses_name_and_state() {
        let s: FlagSetting = "short_range=on".parse().unwrap();
        assert_eq!(s.name, "short_range");
        assert_eq!(s.state, FlagState::On);
        assert_eq!(s.to_string(), "short_range=on");
        let err = "short_range=maybe".parse::<FlagSetting>().unwrap_err();
        assert!(err.contains("maybe"), "{err}");
        assert!("short_range".parse::<FlagSetting>().is_err());
        assert!("=on".parse::<FlagSetting>().is_err());
    }

    #[test]
    fn active_flags_answer_by_name() {
        let flags = ActiveFlags(vec!["a".into(), "c".into()]);
        assert_eq!(flags.names(), ["a", "c"]);
        assert!(!ActiveFlags::default().names().iter().any(|n| n == "a"));
    }
}

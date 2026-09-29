//! The slot file (`content/slots.json`): which module and version fills each slot. A bad
//! entry refuses start-up and names the slot, the bad value, and the valid names.
//!
//! The built-in default selection leaves the content digest as it was, so every hash the
//! replay gate records holds. Any other selection (an optional slot switched off, say) is
//! folded into the digest by `Content::with_slots`, and a replay file carries the slot file
//! as one of its inputs, so a match played with it is never taken for a default one.

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

use super::registry::{ModuleRef, Registration, SlotDecl};
use super::{Picked, ResolvedModules};
use crate::error::EngineError;

/// The slot file version this build reads.
pub const SLOTS_VERSION: u32 = 1;

/// The value `module` takes to switch an optional slot off.
pub const OFF: &str = "off";

/// The slot file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[garde(allow_unvalidated)]
pub struct SlotFile {
    pub schema_version: u32,
    /// Slot id to the module that fills it.
    pub slots: BTreeMap<String, SlotEntry>,
}

/// One slot's module: a name and a version, or `{"module": "off"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotEntry {
    pub module: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
}

impl SlotFile {
    /// The selection the engine uses when no slot file is read (a replay's inputs). It
    /// equals the shipped `content/slots.json`.
    pub fn builtin_default() -> Self {
        let entry = |module: &str| SlotEntry {
            module: module.to_string(),
            version: Some(1),
        };
        Self {
            schema_version: SLOTS_VERSION,
            slots: BTreeMap::from([
                ("engine.fouls".to_string(), entry("fouls")),
                ("engine.offside".to_string(), entry("offside")),
            ]),
        }
    }
}

impl ResolvedModules {
    /// The modules of the built-in default selection.
    pub fn builtin_default() -> Self {
        resolve(&SlotFile::builtin_default(), super::REGISTRY)
            .expect("the built-in slot selection resolves")
    }
}

/// Resolves every slot of `registry` from `file`. Refuses an undeclared slot id, a declared
/// slot missing from the file, an empty module name, an unknown module, a version that is
/// missing or not built, and `off` on a required slot or with a version.
pub fn resolve(file: &SlotFile, registry: &[SlotDecl]) -> Result<ResolvedModules, EngineError> {
    let declared = || {
        registry
            .iter()
            .map(|d| d.slot.id)
            .collect::<Vec<_>>()
            .join(", ")
    };
    if let Some(id) = file
        .slots
        .keys()
        .find(|id| !registry.iter().any(|d| d.slot.id == id.as_str()))
    {
        return Err(refused(
            id,
            id,
            format!("{id:?} is not a declared slot"),
            declared(),
        ));
    }
    let mut chosen: Vec<(&SlotDecl, Registration)> = Vec::with_capacity(registry.len());
    for decl in registry {
        let id = decl.slot.id;
        let Some(entry) = file.slots.get(id) else {
            return Err(refused(
                id,
                "",
                "the slot is missing from slots.json".to_string(),
                decl.valid_names(),
            ));
        };
        chosen.push((decl, pick(decl, entry)?));
    }
    let mut fouls = None;
    let mut offside = None;
    let mut picked = Vec::with_capacity(chosen.len());
    for (decl, reg) in &chosen {
        match reg.module {
            ModuleRef::Fouls(m) => fouls = Some(m),
            ModuleRef::Offside(m) => offside = Some(m),
        }
        picked.push(Picked {
            slot: decl.slot.id,
            module: reg.name,
            version: reg.version,
        });
    }
    let (Some(fouls), Some(offside), Ok(picked)) = (fouls, offside, picked.try_into()) else {
        return Err(EngineError::InvalidConfig(
            "the slot registry does not declare the fouls and offside slots once each".into(),
        ));
    };
    Ok(ResolvedModules {
        fouls,
        offside,
        picked,
    })
}

/// The registration `entry` names for `decl`, or the refusal.
fn pick(decl: &SlotDecl, entry: &SlotEntry) -> Result<Registration, EngineError> {
    let id = decl.slot.id;
    let name = entry.module.trim();
    let bad = |problem: String| refused(id, &entry.module, problem, decl.valid_names());
    if name.is_empty() {
        return Err(bad(format!("the module name {:?} is empty", entry.module)));
    }
    if name == OFF {
        if decl.slot.required {
            return Err(bad("off is not allowed: the slot is required".into()));
        }
        if let Some(version) = entry.version {
            return Err(bad(format!("off takes no version, got {version}")));
        }
        return decl
            .off
            .ok_or_else(|| bad("the slot has no off version".into()));
    }
    let named: Vec<&Registration> = decl
        .registrations
        .iter()
        .filter(|r| r.name == name)
        .collect();
    if named.is_empty() {
        return Err(bad(format!("module {name:?} is not registered")));
    }
    let Some(version) = entry.version else {
        return Err(bad(format!("module {name:?} has no version")));
    };
    named
        .into_iter()
        .find(|r| r.version == version)
        .copied()
        .ok_or_else(|| bad(format!("{name} version {version} is not built")))
}

fn refused(slot: &str, value: &str, problem: String, valid: String) -> EngineError {
    tracing::error!(
        signal = "content.refused",
        kind = "slots",
        path = crate::data::SLOTS_FILE,
        slot,
        value,
        reason = %problem
    );
    EngineError::SlotRefused {
        slot: slot.to_string(),
        value: value.to_string(),
        problem,
        valid,
    }
}

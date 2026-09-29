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

use super::registry::{ModuleRef, Registration, SLOT_COUNT, SlotDecl};
use super::{
    FatigueModule, FoulsModule, OffsideModule, Picked, PreMatchModule, ResolvedModules, ShotModule,
    SteeringModule,
};
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
                ("engine.shot".to_string(), entry("shot")),
                ("engine.fatigue".to_string(), entry("fatigue")),
                ("engine.steering".to_string(), entry("steering")),
                ("engine.pre-match".to_string(), entry("pre-match")),
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
    let mut built = Builder::default();
    let mut picked = Vec::with_capacity(chosen.len());
    for (decl, reg) in &chosen {
        built.fill(reg.module)?;
        picked.push(Picked {
            slot: decl.slot.id,
            module: reg.name,
            version: reg.version,
        });
    }
    built.finish(picked)
}

/// The typed modules of one resolution, filled once per slot.
#[derive(Default)]
struct Builder {
    fouls: Option<&'static dyn FoulsModule>,
    offside: Option<&'static dyn OffsideModule>,
    shot: Option<&'static dyn ShotModule>,
    fatigue: Option<&'static dyn FatigueModule>,
    steering: Option<&'static dyn SteeringModule>,
    pre_match: Option<&'static dyn PreMatchModule>,
}

impl Builder {
    /// Fills the field `module` belongs to; a field filled twice is a registry defect.
    fn fill(&mut self, module: ModuleRef) -> Result<(), EngineError> {
        fn set<T: ?Sized>(field: &mut Option<&'static T>, m: &'static T) -> bool {
            field.replace(m).is_none()
        }
        let once = match module {
            ModuleRef::Fouls(m) => set(&mut self.fouls, m),
            ModuleRef::Offside(m) => set(&mut self.offside, m),
            ModuleRef::Shot(m) => set(&mut self.shot, m),
            ModuleRef::Fatigue(m) => set(&mut self.fatigue, m),
            ModuleRef::Steering(m) => set(&mut self.steering, m),
            ModuleRef::PreMatch(m) => set(&mut self.pre_match, m),
        };
        if once { Ok(()) } else { Err(Self::defect()) }
    }

    fn finish(self, picked: Vec<Picked>) -> Result<ResolvedModules, EngineError> {
        let picked: [Picked; SLOT_COUNT] = picked.try_into().map_err(|_| Self::defect())?;
        let (
            Some(fouls),
            Some(offside),
            Some(shot),
            Some(fatigue),
            Some(steering),
            Some(pre_match),
        ) = (
            self.fouls,
            self.offside,
            self.shot,
            self.fatigue,
            self.steering,
            self.pre_match,
        )
        else {
            return Err(Self::defect());
        };
        Ok(ResolvedModules {
            fouls,
            offside,
            shot,
            fatigue,
            steering,
            pre_match,
            picked,
        })
    }

    fn defect() -> EngineError {
        EngineError::InvalidConfig("the slot registry does not declare every slot once".into())
    }
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

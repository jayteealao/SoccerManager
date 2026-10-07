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

use super::fast_model::FastModel;
use super::modifier::{Modifier, Modifiers};
use super::registry::{
    MODIFIER_COUNT, ModuleRef, Registration, SLOT_COUNT, SlotDecl, for_each_slot_kind,
};
use super::{
    BallModule, ChangesModule, ClockModule, CommentaryHookModule, DecisionHookModule,
    DecisionModule, DisciplineModule, FatigueModule, FoulsModule, InjuriesModule, ManagerModule,
    OffsideModule, PeopleModule, Picked, PossessionModule, PreMatchModule, PresentationModule,
    ResolvedModules, RestartsModule, RuleHookModule, RulesModule, SeasonModule, ShotModule,
    SkinModule, SteeringModule, WorldModule,
};
use crate::error::EngineError;

/// The slot file version this build reads. Version 2 adds the sharpness and adaptation
/// modifier slots; a version 1 file converts ([`crate::data::convert::slots_v1_to_v2`]).
pub const SLOTS_VERSION: u32 = 2;

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
    /// The selection the engine uses when no slot file is read (a replay's inputs): the
    /// first registration of every slot, in registry order. It equals the shipped
    /// `content/slots.json`, which a test holds.
    pub fn builtin_default() -> Self {
        Self {
            schema_version: SLOTS_VERSION,
            slots: super::REGISTRY
                .iter()
                .map(|decl| {
                    let default = &decl.registrations[0];
                    let entry = SlotEntry {
                        module: default.name.to_string(),
                        version: Some(default.version),
                    };
                    (decl.slot.id.to_string(), entry)
                })
                .collect(),
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

macro_rules! builder {
    ($($variant:ident $field:ident $kind:ident,)*) => {
        /// The typed modules of one resolution, filled once per slot.
        #[derive(Default)]
        struct Builder {
            $($field: Option<&'static dyn $kind>,)*
            /// The modifier slots, in registry order.
            modifiers: Vec<&'static dyn Modifier>,
        }

        impl Builder {
            /// Fills the field `module` belongs to; a field filled twice is a registry
            /// defect. A modifier joins the modifier list.
            fn fill(&mut self, module: ModuleRef) -> Result<(), EngineError> {
                let once = match module {
                    $(ModuleRef::$variant(m) => self.$field.replace(m).is_none(),)*
                    ModuleRef::Modifier(m) => {
                        self.modifiers.push(m);
                        true
                    }
                };
                if once { Ok(()) } else { Err(Self::defect()) }
            }

            fn finish(self, picked: Vec<Picked>) -> Result<ResolvedModules, EngineError> {
                let picked: [Picked; SLOT_COUNT] =
                    picked.try_into().map_err(|_| Self::defect())?;
                let modifiers: [&'static dyn Modifier; MODIFIER_COUNT] =
                    self.modifiers.try_into().map_err(|_| Self::defect())?;
                Ok(ResolvedModules {
                    $($field: self.$field.ok_or_else(Self::defect)?,)*
                    modifiers: Modifiers::new(modifiers),
                    picked,
                })
            }
        }
    };
}
for_each_slot_kind!(builder);

impl Builder {
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

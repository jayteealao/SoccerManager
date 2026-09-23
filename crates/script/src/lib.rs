//! Script packs for the football match engine. A pack is a folder with `pack.json` and one
//! Rhai script; the script can adjust the ball carrier's option scores, review the referee's
//! card for a foul, and rewrite commentary lines, through the engine's plugin interface
//! (`engine::plugin`). The script runs in a sandbox with an operation budget, a 2 ms
//! wall-clock backstop, size limits, no import, no `eval`, and no file or network function
//! (see [`sandbox`]). A hook that fails never stops a match: the engine keeps its own choice
//! and records the failure.

pub mod hooks;
pub mod pack;
pub mod sandbox;

use std::path::Path;
use std::sync::Arc;

use engine::EngineError;
use engine::plugin::{HookPoint, Plugins};

pub use pack::{Manifest, PACK_VERSION, Pack};
pub use sandbox::Sandbox;

use hooks::{
    CARD_FN, DECIDE_FN, LINE_FN, ScriptCommentaryHook, ScriptDecisionHook, ScriptRuleHook,
};

/// Why a pack was refused.
#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    /// `pack.json` failed to read, parse, or validate.
    #[error("{0}")]
    Manifest(EngineError),
    #[error("script pack refused: {path}: {field}: {reason}")]
    Refused {
        path: String,
        field: String,
        reason: String,
    },
}

impl ScriptError {
    pub(crate) fn refused(path: &str, field: &str, reason: impl Into<String>) -> Self {
        Self::Refused {
            path: path.to_string(),
            field: field.to_string(),
            reason: reason.into(),
        }
    }

    pub(crate) fn io(path: &str, field: &str, err: std::io::Error) -> Self {
        Self::refused(path, field, err.to_string())
    }
}

/// A pack that loaded: its files, its identity, and its compiled script.
#[derive(Debug, Clone)]
pub struct LoadedPack {
    pub pack: Pack,
    sandbox: Arc<Sandbox>,
}

impl LoadedPack {
    /// Reads, checks, and compiles the pack in `dir`. A listed hook whose function the script
    /// does not define is refused, and so is a script that does not compile, uses `eval`, or
    /// fails in its top-level statements.
    pub fn load(dir: &Path) -> Result<Self, ScriptError> {
        let pack = Pack::read(dir)?;
        let shown = pack.manifest_shown();
        let identity = pack.identity();
        let sandbox = Sandbox::new(&pack.source, pack.manifest.limits.max_operations, &identity)
            .map_err(|reason| ScriptError::refused(&shown, "entry", reason))?;
        for (hook, (name, params)) in [
            (HookPoint::Decision, DECIDE_FN),
            (HookPoint::Rule, CARD_FN),
            (HookPoint::Commentary, LINE_FN),
        ] {
            if pack.has_hook(hook) && !sandbox.defines(name, params) {
                return Err(ScriptError::refused(
                    &shown,
                    "hooks",
                    format!(
                        "lists {} but the script defines no function {name} with {params} parameter{}",
                        hook.code(),
                        if params == 1 { "" } else { "s" }
                    ),
                ));
            }
        }
        tracing::info!(
            signal = "script.loaded",
            pack = %identity,
            hooks = %pack.manifest.hooks.join(","),
            max_operations = pack.manifest.limits.max_operations,
            refresh_ticks = pack.manifest.decision.refresh_ticks
        );
        Ok(Self {
            pack,
            sandbox: Arc::new(sandbox),
        })
    }

    /// The pack identity, `id@version+sha12`.
    pub fn identity(&self) -> String {
        self.pack.identity()
    }

    /// SHA-256 over `pack.json` and the script, to fold into the match's content hash.
    pub fn sha(&self) -> &[u8; 32] {
        &self.pack.sha
    }

    /// Fresh hooks for one match, with counters at zero. The compiled script is shared.
    pub fn plugins(&self) -> Plugins {
        let mut plugins = Plugins::new(self.identity());
        plugins.refresh_ticks = self.pack.manifest.decision.refresh_ticks;
        if self.pack.has_hook(HookPoint::Decision) {
            plugins.decision = Some(Box::new(ScriptDecisionHook(Arc::clone(&self.sandbox))));
        }
        if self.pack.has_hook(HookPoint::Rule) {
            plugins.rule = Some(Box::new(ScriptRuleHook(Arc::clone(&self.sandbox))));
        }
        if self.pack.has_hook(HookPoint::Commentary) {
            plugins.commentary = Some(Box::new(ScriptCommentaryHook(Arc::clone(&self.sandbox))));
        }
        plugins
    }
}

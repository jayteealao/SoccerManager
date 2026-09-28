//! The script pack format: a folder holding `pack.json` (schema version 1) and one `.rhai`
//! entry file. The loader refuses a pack that names an unknown field, targets another plugin
//! interface version, names no hook, sets a limit out of range, or points its entry outside
//! the folder. Every refusal names the file, the field, and the reason.

use std::path::{Path, PathBuf};

use engine::data::{hex12, load_json_bytes};
use engine::plugin::{DEFAULT_REFRESH_TICKS, HookPoint, PLUGIN_API_VERSION};
use garde::Validate;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::ScriptError;

/// The `pack.json` schema version this build reads.
pub const PACK_VERSION: u32 = 1;

/// The manifest file inside a pack folder.
pub const MANIFEST_FILE: &str = "pack.json";

/// The operation budget a pack gets when it names none.
pub const DEFAULT_MAX_OPERATIONS: u64 = 10_000;

/// `pack.json`.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    #[garde(skip)]
    pub schema_version: u32,
    /// Lower-case letters, digits, and hyphens, 1 to 40 characters.
    #[garde(length(min = 1, max = 40), custom(kebab))]
    pub id: String,
    /// Free text such as `1.0.0`, 1 to 20 characters, no `+` or `@`.
    #[garde(length(min = 1, max = 20), custom(plain_version))]
    pub version: String,
    #[garde(length(min = 1, max = 80))]
    pub name: String,
    /// The plugin interface version the pack was written for.
    #[garde(custom(api_version))]
    pub plugin_api: u32,
    /// The script file, a `.rhai` file name inside the pack folder.
    #[garde(custom(entry_name))]
    pub entry: String,
    /// The hooks the script defines, at least one.
    #[garde(length(min = 1, max = 3), custom(known_hooks))]
    pub hooks: Vec<String>,
    #[serde(default)]
    #[garde(dive)]
    pub decision: DecisionSettings,
    #[serde(default)]
    #[garde(dive)]
    pub limits: Limits,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct DecisionSettings {
    /// Ticks the decision hook's offsets stay in force before it is asked again.
    #[garde(range(min = 5, max = 250))]
    pub refresh_ticks: u32,
}

impl Default for DecisionSettings {
    fn default() -> Self {
        Self {
            refresh_ticks: DEFAULT_REFRESH_TICKS,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    /// Script operations one hook call may run.
    #[garde(range(min = 1_000, max = 50_000))]
    pub max_operations: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_operations: DEFAULT_MAX_OPERATIONS,
        }
    }
}

fn kebab(id: &str, _: &()) -> garde::Result {
    let ok = id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !id.starts_with('-');
    if ok {
        Ok(())
    } else {
        Err(garde::Error::new(
            "use lower-case letters, digits, and hyphens, not starting with a hyphen",
        ))
    }
}

fn plain_version(v: &str, _: &()) -> garde::Result {
    if v.chars().any(|c| c == '+' || c == '@' || c.is_whitespace()) {
        Err(garde::Error::new("must not contain '+', '@', or spaces"))
    } else {
        Ok(())
    }
}

fn api_version(v: &u32, _: &()) -> garde::Result {
    if *v == PLUGIN_API_VERSION {
        Ok(())
    } else {
        Err(garde::Error::new(format!(
            "found {v}; this build offers plugin interface {PLUGIN_API_VERSION}"
        )))
    }
}

fn entry_name(entry: &str, _: &()) -> garde::Result {
    let plain = !entry.is_empty()
        && !entry.contains(['/', '\\', ':'])
        && !entry.contains("..")
        && entry.ends_with(".rhai");
    if plain {
        Ok(())
    } else {
        Err(garde::Error::new(
            "must be a .rhai file name inside the pack folder, with no path separator or '..'",
        ))
    }
}

fn known_hooks(hooks: &[String], _: &()) -> garde::Result {
    for (i, hook) in hooks.iter().enumerate() {
        if !HookPoint::ALL.iter().any(|h| h.code() == hook) {
            return Err(garde::Error::new(format!(
                "unknown hook '{hook}'; use decision, rule, or commentary"
            )));
        }
        if hooks[..i].contains(hook) {
            return Err(garde::Error::new(format!("hook '{hook}' is listed twice")));
        }
    }
    Ok(())
}

/// Checks `pack.json`'s bytes; `shown` is the path refusals name.
fn parse_manifest(bytes: &[u8], shown: &str) -> Result<Manifest, ScriptError> {
    load_json_bytes::<Manifest>("script pack", bytes, shown, PACK_VERSION, &())
        .map(|loaded| loaded.value)
        .map_err(ScriptError::Manifest)
}

/// A pack read from disk and checked, with its script source and identity.
#[derive(Debug, Clone)]
pub struct Pack {
    pub manifest: Manifest,
    pub source: String,
    /// SHA-256 over the `pack.json` bytes followed by the entry file's bytes.
    pub sha: [u8; 32],
    /// The folder the pack was read from.
    pub dir: PathBuf,
}

impl Pack {
    /// Reads and checks the pack in `dir`.
    pub fn read(dir: &Path) -> Result<Self, ScriptError> {
        let manifest_path = dir.join(MANIFEST_FILE);
        let shown = manifest_path.display().to_string();
        let manifest_bytes =
            engine::data::read_bytes(&manifest_path, &shown).map_err(ScriptError::Manifest)?;
        let manifest = parse_manifest(&manifest_bytes, &shown)?;
        let entry_path = dir.join(&manifest.entry);
        // The entry name has no separator, so it cannot leave the folder; a link could.
        let folder = dir
            .canonicalize()
            .map_err(|e| ScriptError::io(&shown, "entry", e))?;
        let resolved = entry_path
            .canonicalize()
            .map_err(|e| ScriptError::io(&shown, "entry", e))?;
        if !resolved.starts_with(&folder) {
            return Err(ScriptError::refused(
                &shown,
                "entry",
                "resolves outside the pack folder",
            ));
        }
        let source_bytes =
            std::fs::read(&resolved).map_err(|e| ScriptError::io(&shown, "entry", e))?;
        Self::from_bytes(&manifest_bytes, &source_bytes, dir)
    }

    /// Checks a pack from the bytes of its `pack.json` and its entry file, as a replay file
    /// stores them. `dir` only names the pack in refusals; nothing is read from it.
    pub fn from_bytes(
        manifest_bytes: &[u8],
        source_bytes: &[u8],
        dir: &Path,
    ) -> Result<Self, ScriptError> {
        let shown = dir.join(MANIFEST_FILE).display().to_string();
        let manifest = parse_manifest(manifest_bytes, &shown)?;
        let source = String::from_utf8(source_bytes.to_vec()).map_err(|_| {
            ScriptError::refused(&shown, "entry", "the script file is not UTF-8 text")
        })?;
        let mut hasher = Sha256::new();
        hasher.update(manifest_bytes);
        hasher.update(source_bytes);
        Ok(Self {
            manifest,
            source,
            sha: hasher.finalize().into(),
            dir: dir.to_path_buf(),
        })
    }

    /// The pack identity, `id@version+sha12`.
    pub fn identity(&self) -> String {
        format!(
            "{}@{}+{}",
            self.manifest.id,
            self.manifest.version,
            hex12(&self.sha)
        )
    }

    /// `true` when the pack lists `hook`.
    pub fn has_hook(&self, hook: HookPoint) -> bool {
        self.manifest.hooks.iter().any(|h| h == hook.code())
    }

    /// The path of `pack.json`, as refusals name it.
    pub fn manifest_shown(&self) -> String {
        self.dir.join(MANIFEST_FILE).display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes a pack with `manifest` and a one-hook script into a fresh temp folder.
    fn pack(name: &str, manifest: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("script-pack-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(MANIFEST_FILE), manifest).unwrap();
        std::fs::write(dir.join("main.rhai"), "fn decide(ctx) { #{} }").unwrap();
        dir
    }

    const GOOD: &str = r#"{"schema_version":1,"id":"good","version":"1.0.0","name":"Good","plugin_api":1,"entry":"main.rhai","hooks":["decision"]}"#;

    fn refusal(name: &str, manifest: &str) -> String {
        Pack::read(&pack(name, manifest)).unwrap_err().to_string()
    }

    #[test]
    fn a_valid_pack_gives_the_same_identity_twice() {
        let dir = pack("valid", GOOD);
        let a = Pack::read(&dir).unwrap();
        let b = Pack::read(&dir).unwrap();
        assert_eq!(a.identity(), b.identity());
        assert!(a.identity().starts_with("good@1.0.0+"), "{}", a.identity());
        assert_eq!(a.manifest.decision.refresh_ticks, 25);
        assert_eq!(a.manifest.limits.max_operations, 10_000);
    }

    #[test]
    fn a_bad_manifest_is_refused_naming_the_field() {
        let err = refusal(
            "unknown",
            &GOOD.replace(r#""hooks""#, r#""colour":"red","hooks""#),
        );
        assert!(err.contains("unknown field `colour`"), "{err}");
        let err = refusal(
            "api",
            &GOOD.replace(r#""plugin_api":1"#, r#""plugin_api":2"#),
        );
        assert!(
            err.contains("plugin_api") && err.contains("found 2"),
            "{err}"
        );
        let err = refusal(
            "escape",
            &GOOD.replace(r#""main.rhai""#, r#""../main.rhai""#),
        );
        assert!(
            err.contains("entry") && err.contains("path separator"),
            "{err}"
        );
        let err = refusal("sep", &GOOD.replace(r#""main.rhai""#, r#""a/main.rhai""#));
        assert!(err.contains("entry"), "{err}");
        let err = refusal("nohooks", &GOOD.replace(r#"["decision"]"#, "[]"));
        assert!(err.contains("hooks"), "{err}");
        let err = refusal(
            "refresh",
            &GOOD.replace(
                r#""hooks":["decision"]"#,
                r#""hooks":["decision"],"decision":{"refresh_ticks":4}"#,
            ),
        );
        assert!(err.contains("decision.refresh_ticks"), "{err}");
        let err = refusal(
            "version",
            &GOOD.replace(r#""schema_version":1"#, r#""schema_version":2"#),
        );
        assert!(err.contains("schema_version 2"), "{err}");
    }
}

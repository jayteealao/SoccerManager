//! Content files and the fail-closed loader (named mechanism). Every file carries an
//! integer `schema_version` checked before deserialization; every refusal names the kind,
//! the relative path, the field path, and the reason.

pub mod attributes;
pub mod generator;
pub mod names;
pub mod rules;
pub mod tactics;
pub mod team;
pub mod tuning;

use std::path::{Path, PathBuf};

use garde::Validate;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

use crate::error::EngineError;

pub use attributes::{ATTRIBUTES_VERSION, AttributeSchema, Group, MAX_ATTRIBUTES};
pub use generator::generate_league;
pub use rules::{AddedTime, ExtraTime, RULES_VERSION, RulePack, Shootout, StoppageKind};
pub use tactics::{TACTICS_VERSION, TacticsSchema};
pub use team::{Club, Kit, PlayerEntry, Position, TEAM_VERSION, TeamFile};
pub use tuning::{
    Dist, FatigueTuning, GeneratorTuning, GroupDist, StreamTuning, TUNING_VERSION, TuningFile,
};

/// Relative path of each shipped file inside the content folder.
pub const ATTRIBUTES_FILE: &str = "attributes.json";
pub const TUNING_FILE: &str = "tuning.json";
pub const RULES_FILE: &str = "rules/default.json";
pub const TACTICS_FILE: &str = "tactics.json";
pub const TEAM_A_FILE: &str = "teams/default-a.json";
pub const TEAM_B_FILE: &str = "teams/default-b.json";
/// The English commentary lines. The file stays out of `Content` and its digest: editing a
/// line changes no content hash and never makes a snapshot refuse to resume.
pub const COMMENTARY_FILE: &str = "commentary/en.json";
/// Environment variable that names the content folder.
pub const CONTENT_DIR_ENV: &str = "SM_CONTENT_DIR";

/// The content folder.
#[derive(Debug, Clone)]
pub struct ContentDir {
    root: PathBuf,
}

impl ContentDir {
    /// Resolves the folder. When the `--content-dir` flag is given, only that folder is used.
    /// Otherwise, when `SM_CONTENT_DIR` is set, only that folder is used. Only when neither is
    /// set does resolution fall through to `./content`, then the `content` folder beside the
    /// running binary. The error lists every path tried.
    pub fn resolve(flag: Option<&Path>) -> Result<Self, EngineError> {
        if let Some(p) = flag {
            return Self::resolve_exact(p);
        }
        if let Some(p) = std::env::var_os(CONTENT_DIR_ENV) {
            return Self::resolve_exact(&PathBuf::from(p));
        }
        let mut tried: Vec<PathBuf> = Vec::new();
        tried.push(PathBuf::from("content"));
        if let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
        {
            tried.push(dir.join("content"));
        }
        for p in &tried {
            if p.join(ATTRIBUTES_FILE).is_file() {
                return Ok(Self { root: p.clone() });
            }
        }
        let list: Vec<String> = tried.iter().map(|p| p.display().to_string()).collect();
        Err(EngineError::Read {
            path: format!("content folder (tried {})", list.join(", ")),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no folder holds {ATTRIBUTES_FILE}"),
            ),
        })
    }

    /// Resolves a single, explicitly-named candidate folder (the flag or the env var), with
    /// no fall-through to other candidates.
    fn resolve_exact(p: &Path) -> Result<Self, EngineError> {
        if p.join(ATTRIBUTES_FILE).is_file() {
            return Ok(Self {
                root: p.to_path_buf(),
            });
        }
        Err(EngineError::Read {
            path: format!("content folder (tried {})", p.display()),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no folder holds {ATTRIBUTES_FILE}"),
            ),
        })
    }

    /// A folder at a known path, with no probing.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The path of a file inside the folder.
    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    /// A path for signals and messages: relative to the folder when inside it, otherwise the
    /// file name alone. An absolute path never enters a record.
    pub fn relative(&self, path: &Path) -> String {
        let shown = path
            .strip_prefix(&self.root)
            .ok()
            .or_else(|| path.file_name().map(Path::new))
            .unwrap_or(path);
        shown.to_string_lossy().replace('\\', "/")
    }
}

#[derive(Deserialize)]
struct VersionOnly {
    schema_version: u32,
}

/// One loaded file: the value and the SHA-256 of its bytes.
#[derive(Debug, Clone)]
pub struct Loaded<T> {
    pub value: T,
    pub digest: [u8; 32],
}

/// Reads, version-checks, deserializes, and validates one JSON content file.
/// `shown` is the path as signals and errors name it.
pub fn load_json<T>(
    kind: &'static str,
    path: &Path,
    shown: &str,
    expected_version: u32,
    ctx: &T::Context,
) -> Result<Loaded<T>, EngineError>
where
    T: DeserializeOwned + Validate,
{
    let bytes = std::fs::read(path).map_err(|source| EngineError::Read {
        path: shown.to_string(),
        source,
    })?;
    let refused = |field: String, reason: String| {
        tracing::error!(signal = "content.refused", kind, path = shown, field = %field, reason = %reason);
        EngineError::Data {
            kind,
            path: shown.to_string(),
            field,
            reason,
        }
    };
    let peek: VersionOnly = serde_json::from_slice(&bytes)
        .map_err(|e| refused("schema_version".into(), e.to_string()))?;
    if peek.schema_version != expected_version {
        tracing::error!(
            signal = "content.refused",
            kind,
            path = shown,
            field = "schema_version",
            reason = %format!("found {}; this build reads {expected_version}", peek.schema_version)
        );
        return Err(EngineError::Version {
            kind,
            path: shown.to_string(),
            found: peek.schema_version,
            expected: expected_version,
        });
    }
    let value: T = serde_json::from_slice(&bytes).map_err(|e| {
        refused(
            format!("line {} column {}", e.line(), e.column()),
            e.to_string(),
        )
    })?;
    if let Err(report) = value.validate_with(ctx) {
        let (field, reason) = report
            .iter()
            .next()
            .map(|(path, error)| (path.to_string(), error.message().to_string()))
            .unwrap_or_default();
        return Err(refused(field, reason));
    }
    let digest: [u8; 32] = Sha256::digest(&bytes).into();
    tracing::info!(
        signal = "content.loaded",
        kind,
        path = shown,
        schema_version = expected_version,
        bytes = bytes.len(),
        hash = %hex12(&digest)
    );
    Ok(Loaded { value, digest })
}

/// The first twelve hex characters of a digest.
pub fn hex12(digest: &[u8]) -> String {
    digest.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

/// The four files every match needs, plus a digest over their bytes.
#[derive(Debug, Clone)]
pub struct Content {
    pub attributes: AttributeSchema,
    pub tuning: TuningFile,
    pub rules: RulePack,
    pub tactics: TacticsSchema,
    /// SHA-256 over the four file digests in order: attributes, tuning, rules, tactics.
    pub digest: [u8; 32],
}

impl Content {
    /// Loads the four shipped files from `dir`. The tactics file is checked against the
    /// attribute schema, so a role naming an unknown attribute is refused by name.
    pub fn load(dir: &ContentDir) -> Result<Self, EngineError> {
        let attributes = load_json::<AttributeSchema>(
            "attributes",
            &dir.path(ATTRIBUTES_FILE),
            ATTRIBUTES_FILE,
            ATTRIBUTES_VERSION,
            &(),
        )?;
        let tuning = load_json::<TuningFile>(
            "tuning",
            &dir.path(TUNING_FILE),
            TUNING_FILE,
            TUNING_VERSION,
            &(),
        )?;
        let rules = load_json::<RulePack>(
            "rules",
            &dir.path(RULES_FILE),
            RULES_FILE,
            RULES_VERSION,
            &(),
        )?;
        let tactics = load_json::<TacticsSchema>(
            "tactics",
            &dir.path(TACTICS_FILE),
            TACTICS_FILE,
            TACTICS_VERSION,
            &(),
        )?;
        if let Err((field, reason)) = tactics.value.check(&attributes.value) {
            tracing::error!(signal = "content.refused", kind = "tactics", path = TACTICS_FILE, field = %field, reason = %reason);
            return Err(EngineError::Data {
                kind: "tactics",
                path: TACTICS_FILE.to_string(),
                field,
                reason,
            });
        }
        let mut hasher = Sha256::new();
        hasher.update(attributes.digest);
        hasher.update(tuning.digest);
        hasher.update(rules.digest);
        hasher.update(tactics.digest);
        Ok(Self {
            attributes: attributes.value,
            tuning: tuning.value,
            rules: rules.value,
            tactics: tactics.value,
            digest: hasher.finalize().into(),
        })
    }

    /// Loads and validates one team file against the attribute schema of this content.
    pub fn load_team(
        &self,
        dir: &ContentDir,
        path: &Path,
    ) -> Result<Loaded<TeamFile>, EngineError> {
        let shown = dir.relative(path);
        load_json::<TeamFile>("team", path, &shown, TEAM_VERSION, &self.attributes)
    }

    /// Twelve hex characters identifying the four content files.
    pub fn hash(&self) -> String {
        hex12(&self.digest)
    }
}

/// Unit-test access to the shipped content folder at the workspace root.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::sim::MatchConfig;

    pub(crate) fn shipped_dir() -> ContentDir {
        ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
    }

    pub(crate) fn shipped_content() -> Content {
        Content::load(&shipped_dir()).expect("shipped content loads")
    }

    pub(crate) fn default_teams(content: &Content) -> [TeamFile; 2] {
        let dir = shipped_dir();
        let a = content
            .load_team(&dir, &dir.path(TEAM_A_FILE))
            .expect("team a loads");
        let b = content
            .load_team(&dir, &dir.path(TEAM_B_FILE))
            .expect("team b loads");
        [a.value, b.value]
    }

    pub(crate) fn shipped_config(seed: u64, minutes: u32) -> Result<MatchConfig, EngineError> {
        let content = shipped_content();
        let [a, b] = default_teams(&content);
        MatchConfig::new(seed, minutes, &content, [&a, &b])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str, body: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("engine-data-{}-{name}.json", std::process::id()));
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn a_version_mismatch_names_both_versions() {
        let path = temp("version", r#"{"schema_version": 7, "halves": 2}"#);
        let err =
            load_json::<RulePack>("rules", &path, "rules/x.json", RULES_VERSION, &()).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            err.to_string(),
            "content refused: rules rules/x.json: schema_version 7; this build reads 4"
        );
    }

    #[test]
    fn an_unknown_key_is_refused_with_its_name() {
        let path = temp(
            "unknown",
            r#"{"schema_version": 4, "halves": 2, "half_minutes": 45, "extra": 1,
                "substitutions": {"limit": 5, "windows": 3, "windows_exempt": []},
                "stoppages": []}"#,
        );
        let err =
            load_json::<RulePack>("rules", &path, "rules/x.json", RULES_VERSION, &()).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        let text = err.to_string();
        assert!(
            text.starts_with("content refused: rules rules/x.json: line "),
            "{text}"
        );
        assert!(text.contains("unknown field `extra`"), "{text}");
    }

    #[test]
    fn a_range_violation_names_the_field_path() {
        let path = temp(
            "range",
            r#"{"schema_version": 4, "halves": 3, "half_minutes": 45,
                "substitutions": {"limit": 5, "windows": 3, "windows_exempt": []},
                "stoppages": [],
                "added_time": {"per_kind": {"kick_off": 0, "throw_in": 0, "corner": 0,
                    "goal_kick": 0, "free_kick": 0, "penalty": 0, "goal": 0,
                    "half_time": 0, "injury": 0},
                    "card_s": 0, "variance_s": 0, "min_s": 0, "max_s": 0},
                "min_players": 7,
                "extra_time": {"periods": 2, "period_minutes": 15, "added_max_s": 300,
                    "extra_substitutions": 1, "extra_windows": 1},
                "shootout": {"kicks": 5, "allowance_rounds": 10}}"#,
        );
        let err =
            load_json::<RulePack>("rules", &path, "rules/x.json", RULES_VERSION, &()).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            err.to_string(),
            "content refused: rules rules/x.json: halves: greater than 2"
        );
    }

    #[test]
    fn a_missing_file_names_its_path() {
        let path = std::env::temp_dir().join("engine-data-does-not-exist.json");
        let err = load_json::<RulePack>("rules", &path, "rules/gone.json", RULES_VERSION, &())
            .unwrap_err();
        assert_eq!(err.to_string(), "cannot read rules/gone.json");
    }

    #[test]
    fn relative_paths_never_leak_the_root() {
        let dir = ContentDir::at("C:\\somewhere\\content");
        assert_eq!(
            dir.relative(Path::new("C:\\somewhere\\content\\teams\\a.json")),
            "teams/a.json"
        );
        assert_eq!(dir.relative(Path::new("D:\\elsewhere\\b.json")), "b.json");
    }
}

//! Content files and the fail-closed loader (named mechanism). Every file carries an
//! integer `schema_version` checked before deserialization; every refusal names the kind,
//! the relative path, the field path, and the reason.

pub mod attributes;
pub mod convert;
pub mod generator;
pub mod names;
pub mod primitives;
pub mod restart_waits;
pub mod rules;
pub mod styles;
pub mod tactics;
pub mod team;
pub mod traits;
pub mod tuning;

use std::path::{Path, PathBuf};

use garde::Validate;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

use crate::error::EngineError;
use crate::flags::{ActiveFlags, FlagState, FlagStates, TEAM_STANDS};
use crate::modules::{Picked, REGISTRY, ResolvedModules, SLOTS_VERSION, SlotFile};
use tactics::Stands;

pub use attributes::{ATTRIBUTES_VERSION, AttributeSchema, Group, MAX_ATTRIBUTES};
pub use generator::{generate_league, generate_league_in_tier};
pub use rules::{AddedTime, ExtraTime, RULES_VERSION, RulePack, Shootout, StoppageKind};
pub use tactics::{TACTICS_VERSION, TacticsSchema};
pub use team::{Club, Kit, PlayerEntry, Position, TEAM_VERSION, TeamFile};
pub use tuning::{
    FatigueTuning, GeneratorTuning, GroupOffsets, StreamTuning, TUNING_VERSION, TierDist,
    TuningFile, WorldTuning,
};

/// Relative path of each shipped file inside the content folder.
pub const ATTRIBUTES_FILE: &str = "attributes.json";
pub const TUNING_FILE: &str = "tuning.json";
pub const RULES_FILE: &str = "rules/default.json";
pub const TACTICS_FILE: &str = "tactics.json";
/// The trait catalogue: the traits of play a team file may give a player.
pub const TRAITS_FILE: &str = "traits.json";
/// The restart wait table: how long each kind of stoppage waits beyond its visible steps. It is
/// optional; content without it plays the tuning file's fixed restart delays.
pub const RESTART_WAITS_FILE: &str = "restart-waits.json";
/// The named styles the AI manager picks from before a match. It is optional; content without
/// it keeps the tactics file's default AI tactics.
pub const STYLES_FILE: &str = "styles.json";
pub const TEAM_A_FILE: &str = "teams/default-a.json";
pub const TEAM_B_FILE: &str = "teams/default-b.json";
/// The English commentary lines. The file stays out of `Content` and its digest: editing a
/// line changes no content hash and never makes a snapshot refuse to resume.
pub const COMMENTARY_FILE: &str = "commentary/en.json";
/// The slot file: which module fills each engine slot. It stays out of the content digest
/// while it names the built-in default selection; any other selection is folded in (see
/// [`Content::with_slots`]), so a match played with it is never taken for a default one.
pub const SLOTS_FILE: &str = "slots.json";
/// Slots whose id starts with this choose how the viewer looks; they never enter the digest.
pub const VIEWER_SLOT_PREFIX: &str = "viewer.";
/// The fast-model slot never plays a full-engine match, so it never enters the digest either.
pub const FAST_MODEL_SLOT: &str = "engine.fast-model";
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
        // nosemgrep: rust.lang.security.current-exe.current-exe -- only finds the content folder next to the program; not a security decision
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
    /// The version the file was written in, when it was older and converted on load.
    pub converted_from: Option<u32>,
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
    let bytes = read_bytes(path, shown)?;
    load_json_bytes(kind, &bytes, shown, expected_version, ctx)
}

/// Reads one file whole; a failure names the file as `shown`.
pub fn read_bytes(path: &Path, shown: &str) -> Result<Vec<u8>, EngineError> {
    std::fs::read(path).map_err(|source| EngineError::Read {
        path: shown.to_string(),
        source,
    })
}

/// [`load_json`] over bytes already in memory: version-checks, deserializes, and validates
/// them, and computes their SHA-256. A replay file's inputs load through this, so a
/// re-simulated match reads no file on disk.
pub fn load_json_bytes<T>(
    kind: &'static str,
    bytes: &[u8],
    shown: &str,
    expected_version: u32,
    ctx: &T::Context,
) -> Result<Loaded<T>, EngineError>
where
    T: DeserializeOwned + Validate,
{
    let version = peek_version(kind, bytes, shown)?;
    if version != expected_version {
        return Err(version_refused(kind, shown, version, expected_version));
    }
    let value: T = parse_checked(kind, bytes, shown, ctx)?;
    Ok(loaded(kind, bytes, shown, value, version, None))
}

/// A refusal of `kind` at `shown`, logged as `content.refused`.
fn refused(kind: &'static str, shown: &str, field: String, reason: String) -> EngineError {
    tracing::error!(signal = "content.refused", kind, path = shown, field = %field, reason = %reason);
    EngineError::Data {
        kind,
        path: shown.to_string(),
        field,
        reason,
    }
}

/// The `schema_version` of a content file's bytes.
fn peek_version(kind: &'static str, bytes: &[u8], shown: &str) -> Result<u32, EngineError> {
    let peek: VersionOnly = serde_json::from_slice(bytes)
        .map_err(|e| refused(kind, shown, "schema_version".into(), e.to_string()))?;
    Ok(peek.schema_version)
}

/// The refusal of a version this build does not read, naming both versions.
fn version_refused(kind: &'static str, shown: &str, found: u32, expected: u32) -> EngineError {
    tracing::error!(
        signal = "content.refused",
        kind,
        path = shown,
        field = "schema_version",
        reason = %format!("found {found}; this build reads {expected}")
    );
    EngineError::Version {
        kind,
        path: shown.to_string(),
        found,
        expected,
    }
}

/// Deserializes and validates `bytes` as `T`.
fn parse_checked<T>(
    kind: &'static str,
    bytes: &[u8],
    shown: &str,
    ctx: &T::Context,
) -> Result<T, EngineError>
where
    T: DeserializeOwned + Validate,
{
    checked(kind, shown, parse(kind, bytes, shown)?, ctx)
}

/// Deserializes `bytes` as `T`, refusing with the line and column of the fault.
fn parse<T: DeserializeOwned>(
    kind: &'static str,
    bytes: &[u8],
    shown: &str,
) -> Result<T, EngineError> {
    serde_json::from_slice(bytes).map_err(|e| {
        refused(
            kind,
            shown,
            format!("line {} column {}", e.line(), e.column()),
            e.to_string(),
        )
    })
}

/// `value` when it validates, or the refusal of its first fault.
fn checked<T: Validate>(
    kind: &'static str,
    shown: &str,
    value: T,
    ctx: &T::Context,
) -> Result<T, EngineError> {
    if let Err(report) = value.validate_with(ctx) {
        let (field, reason) = report
            .iter()
            .next()
            .map(|(path, error)| (path.to_string(), error.message().to_string()))
            .unwrap_or_default();
        return Err(refused(kind, shown, field, reason));
    }
    Ok(value)
}

/// A loaded value with the SHA-256 of the bytes read, logged as `content.loaded`.
fn loaded<T>(
    kind: &'static str,
    bytes: &[u8],
    shown: &str,
    value: T,
    version: u32,
    converted_from: Option<u32>,
) -> Loaded<T> {
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    tracing::info!(
        signal = "content.loaded",
        kind,
        path = shown,
        schema_version = version,
        bytes = bytes.len(),
        hash = %hex12(&digest)
    );
    Loaded {
        value,
        digest,
        converted_from,
    }
}

/// Loads a team file's bytes in any version this build reads. A version 3 or 2 file has its
/// ratings (and a version 3 file its position ratings) checked first, so a value off the tenth
/// grid or outside 1.0 to 20.0 is refused naming the player and the field; a version 3 file's
/// traits are checked against `traits`. A version 2 file converts to version 3
/// ([`convert::team_v2_to_v3`]) and is then checked as a current file; a version 1 file is
/// checked as version 1 and converted through version 2. The digest is that of the bytes read.
pub fn load_team_bytes(
    bytes: &[u8],
    shown: &str,
    schema: &AttributeSchema,
    traits: &traits::TraitCatalogue,
) -> Result<Loaded<TeamFile>, EngineError> {
    const KIND: &str = "team";
    match peek_version(KIND, bytes, shown)? {
        TEAM_VERSION => {
            check_ratings(bytes, shown)?;
            let value: TeamFile = parse(KIND, bytes, shown)?;
            let value = checked(KIND, shown, value, schema)?;
            team::check_traits(&value, traits)
                .map_err(|reason| refused(KIND, shown, "players".into(), reason))?;
            Ok(loaded(KIND, bytes, shown, value, TEAM_VERSION, None))
        }
        convert::TEAM_V2 => {
            check_ratings(bytes, shown)?;
            let value: TeamFile = parse(KIND, bytes, shown)?;
            let value = checked(KIND, shown, convert::team_v2_to_v3(value, schema), schema)?;
            Ok(loaded(
                KIND,
                bytes,
                shown,
                value,
                convert::TEAM_V2,
                Some(convert::TEAM_V2),
            ))
        }
        convert::TEAM_V1 => {
            let v1 = parse_checked::<convert::TeamFileV1>(KIND, bytes, shown, schema)?;
            let value = convert::team_v2_to_v3(convert::team_v1_to_v2(v1), schema);
            Ok(loaded(
                KIND,
                bytes,
                shown,
                value,
                convert::TEAM_V1,
                Some(convert::TEAM_V1),
            ))
        }
        found => Err(version_refused(KIND, shown, found, TEAM_VERSION)),
    }
}

/// Refuses the first rating of a version 2 or 3 team file that is off the tenth grid or outside
/// 1.0 to 20.0, naming the player and the attribute or position. Anything else malformed is
/// left to the full parse.
fn check_ratings(bytes: &[u8], shown: &str) -> Result<(), EngineError> {
    let Ok(doc) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return Ok(());
    };
    let Some(players) = doc.get("players").and_then(|p| p.as_array()) else {
        return Ok(());
    };
    for p in players {
        let id = p.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        for (key, label) in [
            ("attributes", "attribute"),
            ("positions", "position rating"),
        ] {
            let Some(values) = p.get(key).and_then(|a| a.as_object()) else {
                continue;
            };
            for (name, value) in values {
                if let Some(x) = value.as_f64()
                    && let Err(e) = crate::rating::Rating::from_decimal(x)
                {
                    return Err(refused(
                        "team",
                        shown,
                        "players".into(),
                        format!("player {id}: {label} {name} {e}"),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Loads an attribute file's bytes in any version this build reads; a version 1 file is
/// converted with the frozen contract tables ([`convert::attributes_v1_to_v2`]), a version 2
/// file takes the hidden values ([`convert::attributes_v2_to_v3`]), a version 3 file takes
/// the version 4 additions ([`convert::attributes_v3_to_v4`]), a version 4 file takes the
/// version 5 changes ([`convert::attributes_v4_to_v5`]), and the result is checked once as a
/// current file. The digest is that of the bytes read.
pub fn load_attributes_bytes(
    bytes: &[u8],
    shown: &str,
) -> Result<Loaded<AttributeSchema>, EngineError> {
    const KIND: &str = "attributes";
    match peek_version(KIND, bytes, shown)? {
        ATTRIBUTES_VERSION => {
            let value = parse_checked::<AttributeSchema>(KIND, bytes, shown, &())?;
            Ok(loaded(KIND, bytes, shown, value, ATTRIBUTES_VERSION, None))
        }
        found @ (convert::ATTRIBUTES_V1
        | convert::ATTRIBUTES_V2
        | convert::ATTRIBUTES_V3
        | convert::ATTRIBUTES_V4) => {
            let v4 = match found {
                convert::ATTRIBUTES_V1 => {
                    let v1: convert::AttributeSchemaV1 = parse(KIND, bytes, shown)?;
                    let v2 = convert::attributes_v1_to_v2(v1)
                        .map_err(|reason| refused(KIND, shown, "attributes".into(), reason))?;
                    convert::attributes_v3_to_v4(convert::attributes_v2_to_v3(v2))
                }
                convert::ATTRIBUTES_V2 => {
                    convert::attributes_v3_to_v4(convert::attributes_v2_to_v3(parse::<
                        AttributeSchema,
                    >(
                        KIND, bytes, shown,
                    )?))
                }
                convert::ATTRIBUTES_V3 => {
                    convert::attributes_v3_to_v4(parse::<AttributeSchema>(KIND, bytes, shown)?)
                }
                _ => parse::<AttributeSchema>(KIND, bytes, shown)?,
            };
            let value = checked(KIND, shown, convert::attributes_v4_to_v5(v4), &())?;
            Ok(loaded(KIND, bytes, shown, value, found, Some(found)))
        }
        found => Err(version_refused(KIND, shown, found, ATTRIBUTES_VERSION)),
    }
}

/// Loads a tuning file's bytes in any version this build reads; a version 2 file is
/// converted to version 3 ([`convert::tuning_v2_to_v3`]), a version 3 file to version 4
/// ([`convert::tuning_v3_to_v4`]), a version 4 file to version 5
/// ([`convert::tuning_v4_to_v5`]), a version 5 file to version 6
/// ([`convert::tuning_v5_to_v6`]), and the result is checked as a current file.
pub fn load_tuning_bytes(bytes: &[u8], shown: &str) -> Result<Loaded<TuningFile>, EngineError> {
    const KIND: &str = "tuning";
    match peek_version(KIND, bytes, shown)? {
        TUNING_VERSION => {
            let value = parse_checked::<TuningFile>(KIND, bytes, shown, &())?;
            Ok(loaded(KIND, bytes, shown, value, TUNING_VERSION, None))
        }
        found @ (convert::TUNING_V2
        | convert::TUNING_V3
        | convert::TUNING_V4
        | convert::TUNING_V5) => {
            let mut json: serde_json::Value = parse(KIND, bytes, shown)?;
            if found == convert::TUNING_V2 {
                json = convert::tuning_v2_to_v3(json)
                    .map_err(|reason| refused(KIND, shown, "engine".into(), reason))?;
            }
            if found <= convert::TUNING_V3 {
                json = convert::tuning_v3_to_v4(json)
                    .map_err(|reason| refused(KIND, shown, "engine".into(), reason))?;
            }
            if found <= convert::TUNING_V4 {
                json = convert::tuning_v4_to_v5(json)
                    .map_err(|reason| refused(KIND, shown, "engine".into(), reason))?;
            }
            let v6 = convert::tuning_v5_to_v6(json)
                .map_err(|reason| refused(KIND, shown, "engine".into(), reason))?;
            let value: TuningFile = serde_json::from_value(v6)
                .map_err(|e| refused(KIND, shown, "converted file".into(), e.to_string()))?;
            let value = checked(KIND, shown, value, &())?;
            Ok(loaded(KIND, bytes, shown, value, found, Some(found)))
        }
        found => Err(version_refused(KIND, shown, found, TUNING_VERSION)),
    }
}

/// Loads a tactics file's bytes in any version this build reads; a version 1 file is
/// converted to version 2 ([`convert::tactics_v1_to_v2`]), a version 2 file to version 3
/// ([`convert::tactics_v2_to_v3`]), a version 3 file to version 4
/// ([`convert::tactics_v3_to_v4`]), a version 4 file to version 5
/// ([`convert::tactics_v4_to_v5`]), and the result is checked as a current file.
pub fn load_tactics_bytes(bytes: &[u8], shown: &str) -> Result<Loaded<TacticsSchema>, EngineError> {
    const KIND: &str = "tactics";
    match peek_version(KIND, bytes, shown)? {
        TACTICS_VERSION => {
            let value = parse_checked::<TacticsSchema>(KIND, bytes, shown, &())?;
            Ok(loaded(KIND, bytes, shown, value, TACTICS_VERSION, None))
        }
        found @ (convert::TACTICS_V1
        | convert::TACTICS_V2
        | convert::TACTICS_V3
        | convert::TACTICS_V4) => {
            let v4 = if found == convert::TACTICS_V4 {
                parse(KIND, bytes, shown)?
            } else {
                let v3 = if found == convert::TACTICS_V3 {
                    parse(KIND, bytes, shown)?
                } else {
                    let v2 = if found == convert::TACTICS_V1 {
                        let v1: convert::TacticsV1 = parse(KIND, bytes, shown)?;
                        convert::tactics_v1_to_v2(v1)
                    } else {
                        parse(KIND, bytes, shown)?
                    };
                    convert::tactics_v2_to_v3(v2)
                        .map_err(|reason| refused(KIND, shown, "tactics".into(), reason))?
                };
                convert::tactics_v3_to_v4(v3)
                    .map_err(|reason| refused(KIND, shown, "tactics".into(), reason))?
            };
            let v5 = convert::tactics_v4_to_v5(v4)
                .map_err(|reason| refused(KIND, shown, "tactics".into(), reason))?;
            let value: TacticsSchema = serde_json::from_value(v5)
                .map_err(|e| refused(KIND, shown, "converted file".into(), e.to_string()))?;
            let value = checked(KIND, shown, value, &())?;
            Ok(loaded(KIND, bytes, shown, value, found, Some(found)))
        }
        found => Err(version_refused(KIND, shown, found, TACTICS_VERSION)),
    }
}

/// Loads a slot file's bytes in any version this build reads; a version 1 file is converted
/// to version 2 ([`convert::slots_v1_to_v2`]) and a version 2 file to version 3
/// ([`convert::slots_v2_to_v3`]). The digest is that of the bytes read.
pub fn load_slots_bytes(bytes: &[u8]) -> Result<Loaded<SlotFile>, EngineError> {
    const KIND: &str = "slots";
    let found = peek_version(KIND, bytes, SLOTS_FILE)?;
    match found {
        SLOTS_VERSION => load_json_bytes::<SlotFile>(KIND, bytes, SLOTS_FILE, SLOTS_VERSION, &()),
        convert::SLOTS_V1 | convert::SLOTS_V2 => {
            let mut file: serde_json::Value = parse(KIND, bytes, SLOTS_FILE)?;
            let bad = |reason| refused(KIND, SLOTS_FILE, "slots".into(), reason);
            if found == convert::SLOTS_V1 {
                file = convert::slots_v1_to_v2(file).map_err(bad)?;
            }
            let v3 = convert::slots_v2_to_v3(file).map_err(bad)?;
            let value: SlotFile = serde_json::from_value(v3)
                .map_err(|e| refused(KIND, SLOTS_FILE, "converted file".into(), e.to_string()))?;
            let value = checked(KIND, SLOTS_FILE, value, &())?;
            Ok(loaded(KIND, bytes, SLOTS_FILE, value, found, Some(found)))
        }
        found => Err(version_refused(KIND, SLOTS_FILE, found, SLOTS_VERSION)),
    }
}

/// The first twelve hex characters of a digest.
pub fn hex12(digest: &[u8]) -> String {
    digest.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

/// The bytes of the five content files, and of the restart wait table when the content has
/// one, as read from a content folder or a replay file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentFiles {
    pub attributes: Vec<u8>,
    pub tuning: Vec<u8>,
    pub rules: Vec<u8>,
    pub tactics: Vec<u8>,
    pub traits: Vec<u8>,
    /// The restart wait table, `None` when the content has none.
    pub restart_waits: Option<Vec<u8>>,
    /// The named styles, `None` when the content has none.
    pub styles: Option<Vec<u8>>,
}

impl ContentFiles {
    /// Reads the five files from `dir`, each once, and the restart wait table when the folder
    /// holds one.
    pub fn read(dir: &ContentDir) -> Result<Self, EngineError> {
        let read = |rel: &str| read_bytes(&dir.path(rel), rel);
        let waits = dir.path(RESTART_WAITS_FILE);
        let styles = dir.path(STYLES_FILE);
        Ok(Self {
            attributes: read(ATTRIBUTES_FILE)?,
            tuning: read(TUNING_FILE)?,
            rules: read(RULES_FILE)?,
            tactics: read(TACTICS_FILE)?,
            traits: read(TRAITS_FILE)?,
            restart_waits: if waits.is_file() {
                Some(read(RESTART_WAITS_FILE)?)
            } else {
                None
            },
            styles: if styles.is_file() {
                Some(read(STYLES_FILE)?)
            } else {
                None
            },
        })
    }
}

/// The five files every match needs, plus a digest over their bytes.
#[derive(Debug, Clone)]
pub struct Content {
    pub attributes: AttributeSchema,
    pub tuning: TuningFile,
    pub rules: RulePack,
    pub tactics: TacticsSchema,
    /// The trait catalogue: the traits of play a team file may give a player.
    pub traits: traits::TraitCatalogue,
    /// The restart wait table, `None` when the content has none: the fixed delays play then.
    pub restart_waits: Option<restart_waits::RestartWaits>,
    /// The named styles, `None` when the content has none: the AI keeps its default tactics.
    pub styles: Option<styles::Styles>,
    /// SHA-256 over the five file digests in order: attributes, tuning, rules, tactics, traits,
    /// then, when the content has a restart wait table, a tag and its digest, and the same for a
    /// styles file.
    /// When a flag state differs from the file's, the list of flags that are on is hashed
    /// in as well.
    pub digest: [u8; 32],
    /// The flags that are on; `tuning` already holds their overrides.
    pub flags: ActiveFlags,
    /// The module in each slot. [`Content::load`] resolves `slots.json`; content built from
    /// the four files' bytes alone takes the built-in default selection.
    pub modules: ResolvedModules,
    /// The tuning file as written, and the digest over the files as written.
    written_tuning: TuningFile,
    written_digest: [u8; 32],
    /// The rule file as written. The rules slot's module turns it into `rules`.
    written_rules: Vec<u8>,
    /// The tactics file's stands as written. `tactics` holds them only while the
    /// `team_stands` flag is on; off, every stand in `tactics` reads 0.
    written_stands: Stands,
}

impl Content {
    /// Loads the four shipped files from `dir`, then the slot file. The tactics file is
    /// checked against the attribute schema, so a role naming an unknown attribute is
    /// refused by name; a bad slot entry is refused with the slot, the value, and the valid
    /// names.
    pub fn load(dir: &ContentDir) -> Result<Self, EngineError> {
        let content = Self::from_files(&ContentFiles::read(dir)?)?;
        content.with_slots(&read_bytes(&dir.path(SLOTS_FILE), SLOTS_FILE)?)
    }

    /// This content with the modules the slot file `bytes` selects. A bad entry is refused
    /// with the slot, the value, and the valid names.
    ///
    /// A selection other than the built-in default (`viewer.*` and the fast-model slot aside)
    /// is folded into the digest, the way a changed flag state is: a match played with a
    /// module off never shares a content hash with a default one, so a snapshot refuses to
    /// resume under another selection, and the default selection leaves every hash as it
    /// was. Call it once, on content that [`Content::from_files`] built.
    pub fn with_slots(&self, bytes: &[u8]) -> Result<Self, EngineError> {
        let slots = load_slots_bytes(bytes)?;
        let modules = crate::modules::resolve(&slots.value, REGISTRY)?;
        let mut next = Self {
            modules,
            ..self.clone()
        };
        // The rule pack loads through the rules slot: a selection that changes the rules
        // module reloads it from the rule file as written.
        let rules_slot = crate::modules::registry::RULES.id;
        if modules.picked_for(rules_slot) != self.modules.picked_for(rules_slot) {
            next.rules = modules.rule_pack.load(&self.written_rules)?.value;
        }
        // A `viewer.*` slot chooses how a match looks, never what happens in it, and the
        // fast-model slot never plays a full-engine match, so both stay out of the digest: a
        // save or a replay does not split on either.
        let in_match = |picked: &[Picked]| -> Vec<Picked> {
            picked
                .iter()
                .copied()
                .filter(|p| !p.slot.starts_with(VIEWER_SLOT_PREFIX) && p.slot != FAST_MODEL_SLOT)
                .collect()
        };
        let in_match_picked = in_match(modules.picked());
        if in_match_picked != in_match(ResolvedModules::builtin_default().picked()) {
            let fold = |digest: [u8; 32]| -> [u8; 32] {
                let mut hasher = Sha256::new();
                hasher.update(digest);
                hasher.update(b"slots:");
                for p in &in_match_picked {
                    hasher.update(format!("{}={}@{},", p.slot, p.module, p.version).as_bytes());
                }
                hasher.finalize().into()
            };
            let flagged = self.digest != self.written_digest;
            next.written_digest = fold(self.written_digest);
            next.digest = if flagged {
                fold(self.digest)
            } else {
                next.written_digest
            };
        }
        Ok(next)
    }

    /// The content from the four files' bytes, with the same checks, digest, and flag
    /// states as [`Content::load`].
    pub fn from_files(files: &ContentFiles) -> Result<Self, EngineError> {
        let attributes = load_attributes_bytes(&files.attributes, ATTRIBUTES_FILE)?;
        let tuning = load_tuning_bytes(&files.tuning, TUNING_FILE)?;
        let modules = ResolvedModules::builtin_default();
        let rules = modules.rule_pack.load(&files.rules)?;
        let tactics = load_tactics_bytes(&files.tactics, TACTICS_FILE)?;
        if let Err((field, reason)) = tactics.value.check(&attributes.value) {
            tracing::error!(signal = "content.refused", kind = "tactics", path = TACTICS_FILE, field = %field, reason = %reason);
            return Err(EngineError::Data {
                kind: "tactics",
                path: TACTICS_FILE.to_string(),
                field,
                reason,
            });
        }
        // A hidden value reaches a page only as a word, so each one needs its word bands. A
        // personality value reaches a page through the summary-word rule of the panel instead.
        if let Some(name) = attributes
            .value
            .attributes
            .iter()
            .filter(|d| d.hidden && !d.is_personality())
            .map(|d| d.name.as_str())
            .find(|n| !tuning.value.hidden.words.contains_key(*n))
        {
            let (field, reason) = (
                "hidden.words".to_string(),
                format!("hidden attribute {name} has no word bands"),
            );
            tracing::error!(signal = "content.refused", kind = "tuning", path = TUNING_FILE, field = %field, reason = %reason);
            return Err(EngineError::Data {
                kind: "tuning",
                path: TUNING_FILE.to_string(),
                field,
                reason,
            });
        }
        let traits = traits::load_traits_bytes(&files.traits, TRAITS_FILE)?;
        let waits = files
            .restart_waits
            .as_deref()
            .map(|bytes| restart_waits::load_restart_waits_bytes(bytes, RESTART_WAITS_FILE))
            .transpose()?;
        let styles = files
            .styles
            .as_deref()
            .map(|bytes| {
                styles::load_styles_bytes(bytes, STYLES_FILE, &tactics.value, &attributes.value)
            })
            .transpose()?;
        // The generator gives a keeper only the keeper traits, so each must be a trait.
        if let Some(name) = tuning.value.generator.person.as_ref().and_then(|p| {
            p.traits
                .keeper_only
                .iter()
                .find(|n| traits.value.get(n).is_none())
        }) {
            let (field, reason) = (
                "generator.person.traits.keeper_only".to_string(),
                format!("trait {name} is not in the trait catalogue"),
            );
            tracing::error!(signal = "content.refused", kind = "tuning", path = TUNING_FILE, field = %field, reason = %reason);
            return Err(EngineError::Data {
                kind: "tuning",
                path: TUNING_FILE.to_string(),
                field,
                reason,
            });
        }
        let mut hasher = Sha256::new();
        hasher.update(attributes.digest);
        hasher.update(tuning.digest);
        hasher.update(rules.digest);
        hasher.update(tactics.digest);
        hasher.update(traits.digest);
        // Content without a wait table keeps the digest it had before the table existed.
        if let Some(waits) = &waits {
            hasher.update(b"restart_waits:");
            hasher.update(waits.digest);
        }
        // Content without a styles file keeps the digest it had before the file existed.
        if let Some(styles) = &styles {
            hasher.update(b"styles:");
            hasher.update(styles.digest);
        }
        let digest: [u8; 32] = hasher.finalize().into();
        let written_stands = tactics.value.stands;
        let written = Self {
            attributes: attributes.value,
            tuning: tuning.value.clone(),
            rules: rules.value,
            tactics: tactics.value,
            traits: traits.value,
            restart_waits: waits.map(|w| w.value),
            styles: styles.map(|s| s.value),
            digest,
            flags: ActiveFlags::default(),
            modules,
            written_tuning: tuning.value,
            written_digest: digest,
            written_rules: files.rules.clone(),
            written_stands,
        };
        written.with_flags(&FlagStates::default())
    }

    /// This content with the flag states in `states` applied over the file's own states.
    /// A flag that is on has its overrides written into `tuning`, and each one is logged as
    /// `flag.active`. The tactics file's stands enter `tactics` only while the `team_stands`
    /// flag is on; off, every stand reads 0. When a state differs from the file's, the list of flags that are on
    /// enters the digest, so a match played with it is never taken for one played without.
    pub fn with_flags(&self, states: &FlagStates) -> Result<Self, EngineError> {
        let file = &self.written_tuning;
        let resolved = crate::flags::effective(file, states)?;
        let (tuning, flags) = crate::flags::apply(file, states)?;
        for (name, (state, source)) in &resolved {
            if *state == FlagState::On {
                tracing::info!(
                    signal = "flag.active",
                    flag = %name,
                    owner = %file.flags[name].owner,
                    source = source.code()
                );
            }
        }
        let differs = resolved
            .iter()
            .any(|(name, (state, _))| *state != file.flags[name].state);
        let digest = if differs {
            let mut hasher = Sha256::new();
            hasher.update(self.written_digest);
            hasher.update(b"flags:");
            hasher.update(flags.names().join(",").as_bytes());
            hasher.finalize().into()
        } else {
            self.written_digest
        };
        let mut tactics = self.tactics.clone();
        tactics.stands = if flags.is_on(TEAM_STANDS) {
            self.written_stands
        } else {
            Stands::default()
        };
        Ok(Self {
            tuning,
            tactics,
            digest,
            flags,
            ..self.clone()
        })
    }

    /// Loads and validates one team file against the attribute schema and the trait catalogue
    /// of this content. A version 1 or 2 file converts; its `converted_from` is its version.
    pub fn load_team(
        &self,
        dir: &ContentDir,
        path: &Path,
    ) -> Result<Loaded<TeamFile>, EngineError> {
        let shown = dir.relative(path);
        load_team_bytes(
            &read_bytes(path, &shown)?,
            &shown,
            &self.attributes,
            &self.traits,
        )
    }

    /// Loads and validates one team file's bytes against the attribute schema and the trait
    /// catalogue of this content; `shown` is the name errors give the file.
    pub fn team_from_bytes(
        &self,
        bytes: &[u8],
        shown: &str,
    ) -> Result<Loaded<TeamFile>, EngineError> {
        load_team_bytes(bytes, shown, &self.attributes, &self.traits)
    }

    /// The tuning file as written, before any flag state is applied.
    pub fn written_tuning(&self) -> &TuningFile {
        &self.written_tuning
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

    /// The shipped content with the `team_stands` flag on: the tactics file's stands apply.
    pub(crate) fn shipped_content_with_stands() -> Content {
        let on = FlagStates([(TEAM_STANDS.to_string(), FlagState::On)].into());
        shipped_content()
            .with_flags(&on)
            .expect("the stands flag turns on")
    }

    /// [`shipped_config`] with the team stands on.
    pub(crate) fn shipped_config_with_stands(
        seed: u64,
        minutes: u32,
    ) -> Result<MatchConfig, EngineError> {
        let content = shipped_content_with_stands();
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
    #[cfg(windows)]
    fn relative_paths_never_leak_the_root() {
        let dir = ContentDir::at("C:\\somewhere\\content");
        assert_eq!(
            dir.relative(Path::new("C:\\somewhere\\content\\teams\\a.json")),
            "teams/a.json"
        );
        assert_eq!(dir.relative(Path::new("D:\\elsewhere\\b.json")), "b.json");
    }

    #[test]
    fn content_from_bytes_matches_content_from_the_folder() {
        let dir = test_support::shipped_dir();
        let from_dir = Content::load(&dir).unwrap();
        let from_bytes = Content::from_files(&ContentFiles::read(&dir).unwrap()).unwrap();
        assert_eq!(from_bytes.digest, from_dir.digest);
        assert_eq!(from_bytes.hash(), from_dir.hash());
        assert_eq!(from_bytes.flags, from_dir.flags);
        let bytes = std::fs::read(dir.path(TEAM_A_FILE)).unwrap();
        let team = from_bytes.team_from_bytes(&bytes, TEAM_A_FILE).unwrap();
        let loaded = from_dir.load_team(&dir, &dir.path(TEAM_A_FILE)).unwrap();
        assert_eq!(team.digest, loaded.digest);
    }
}

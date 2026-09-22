//! Identity: the runtime data folder, the owner identifier created once per machine, and
//! the match identifier (RIM-2, RIM-4; contract Block C).

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use sha2::{Digest, Sha256};

use crate::error::EngineError;

/// Environment variable that names the runtime data folder.
pub const DATA_DIR_ENV: &str = "SM_DATA_DIR";
/// File inside the data folder that holds the owner identifier.
pub const OWNER_FILE: &str = "owner.id";

/// The runtime data folder: `SM_DATA_DIR`, else `%LOCALAPPDATA%\SoccerManager` on Windows,
/// else `$HOME/.local/share/SoccerManager`, else `./SoccerManager`.
pub fn data_dir() -> PathBuf {
    if let Some(p) = std::env::var_os(DATA_DIR_ENV) {
        return PathBuf::from(p);
    }
    if let Some(p) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(p).join("SoccerManager");
    }
    if let Some(p) = std::env::var_os("HOME") {
        return PathBuf::from(p).join(".local/share/SoccerManager");
    }
    PathBuf::from("SoccerManager")
}

/// Reads `owner.id` from `dir`, or creates it once. The value is 32 lower-case hex
/// characters: half of a SHA-256 over the machine name, the time, and the process id.
pub fn load_or_create_owner_id(dir: &Path) -> Result<String, EngineError> {
    let path = dir.join(OWNER_FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let id = text.trim().to_string();
            owner_bytes(&id)?;
            Ok(id)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(dir)?;
            let name = std::env::var("COMPUTERNAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .unwrap_or_else(|_| "unknown".into());
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let mut hasher = Sha256::new();
            hasher.update(name.as_bytes());
            hasher.update(nanos.to_le_bytes());
            hasher.update(std::process::id().to_le_bytes());
            let digest = hasher.finalize();
            let id = owner_hex(digest[..16].try_into().expect("16 bytes"));
            std::fs::write(&path, format!("{id}\n")).map_err(|source| EngineError::Read {
                path: OWNER_FILE.into(),
                source,
            })?;
            tracing::info!(signal = "identity.owner_created", path = OWNER_FILE);
            Ok(id)
        }
        Err(source) => Err(EngineError::Read {
            path: OWNER_FILE.into(),
            source,
        }),
    }
}

/// The 16 bytes of a 32-character hex owner identifier.
pub fn owner_bytes(id: &str) -> Result<[u8; 16], EngineError> {
    let bad = || EngineError::Format(format!("{OWNER_FILE} must hold 32 hex characters"));
    if id.len() != 32 || !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(bad());
    }
    let mut out = [0u8; 16];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&id[2 * i..2 * i + 2], 16).map_err(|_| bad())?;
    }
    Ok(out)
}

/// The 32-character hex form of an owner identifier.
pub fn owner_hex(bytes: &[u8; 16]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A match identifier: the seed and the start time, `{seed:016x}-{millis}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchId {
    pub seed: u64,
    pub millis: u64,
}

impl MatchId {
    /// A new identifier for `seed` stamped with the current time.
    pub fn now(seed: u64) -> Self {
        Self {
            seed,
            millis: u64::try_from(super::unix_millis()).unwrap_or(u64::MAX),
        }
    }
}

impl fmt::Display for MatchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}-{}", self.seed, self.millis)
    }
}

impl FromStr for MatchId {
    type Err = EngineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = || EngineError::Format(format!("match id {s} is not <seed:016x>-<millis>"));
        let (seed, millis) = s.split_once('-').ok_or_else(bad)?;
        if seed.len() != 16 {
            return Err(bad());
        }
        Ok(Self {
            seed: u64::from_str_radix(seed, 16).map_err(|_| bad())?,
            millis: millis.parse().map_err(|_| bad())?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_owner_id_is_created_once_and_reused() {
        let dir = std::env::temp_dir().join(format!("engine-identity-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let first = load_or_create_owner_id(&dir).unwrap();
        let second = load_or_create_owner_id(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 32);
        assert_eq!(owner_hex(&owner_bytes(&first).unwrap()), first);
    }

    #[test]
    fn a_corrupt_owner_file_is_refused() {
        let dir = std::env::temp_dir().join(format!("engine-identity-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(OWNER_FILE), "not-hex\n").unwrap();
        let err = load_or_create_owner_id(&dir).unwrap_err();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(err.to_string().contains("32 hex characters"), "{err}");
    }

    #[test]
    fn match_id_round_trips_through_text() {
        let id = MatchId {
            seed: 42,
            millis: 1_700_000_000_123,
        };
        let text = id.to_string();
        assert_eq!(text, "000000000000002a-1700000000123");
        assert_eq!(text.parse::<MatchId>().unwrap(), id);
        assert!("2a-1".parse::<MatchId>().is_err());
    }
}

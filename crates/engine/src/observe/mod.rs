//! Observability records: `match-stats` and `run-report` with the canonical dotted keys,
//! the owner identity, the content hash, the hashed machine identity, the build hash, and
//! process measurements. `match-stats` is also saved under the runtime data folder.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::EngineError;

pub mod identity;
pub mod process;

/// Schema version of every record.
pub const SCHEMA_VERSION: &str = "1";
/// Service name carried by every record this crate's binaries emit.
pub const SERVICE: &str = "engine-cli";

/// Milliseconds since the Unix epoch.
pub fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// SHA-256 of the computer name, first 12 hex characters (contract Block C).
pub fn machine_hash() -> String {
    let name = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".into());
    let digest = Sha256::digest(name.as_bytes());
    digest.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

/// The environment name (`SM_ENV`, default `dev`).
pub fn env_name() -> String {
    std::env::var("SM_ENV").unwrap_or_else(|_| "dev".into())
}

/// A record kind and operation, for the envelope keys.
pub trait Record: Serialize {
    fn kind(&self) -> &'static str;
    fn operation(&self) -> &'static str;
    fn owner_id(&self) -> &str;
}

/// One club as `match-stats` names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamRef {
    #[serde(rename = "team.id")]
    pub id: String,
    #[serde(rename = "team.name")]
    pub name: String,
}

/// One `match-stats` record: the wide event of one simulate run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchStats {
    #[serde(rename = "owner.id")]
    pub owner_id: String,
    #[serde(rename = "match.id")]
    pub match_id: String,
    pub seed: u64,
    #[serde(rename = "content.hash")]
    pub content_hash: String,
    /// The two clubs, home first. An additive extra to the contract vocabulary.
    pub teams: [TeamRef; 2],
    pub duration_ms: u64,
    pub outcome: String,
    #[serde(rename = "engine.ticks_per_s")]
    pub ticks_per_s: f64,
    #[serde(rename = "ticks.written")]
    pub ticks_written: u32,
    #[serde(rename = "validate.ran")]
    pub validate_ran: bool,
    #[serde(rename = "validate.violations")]
    pub validate_violations: usize,
    #[serde(rename = "possession.changes")]
    pub possession_changes: u32,
    #[serde(rename = "ball.max_speed")]
    pub ball_max_speed: f64,
    #[serde(rename = "ball.idle_ticks")]
    pub ball_idle_ticks: u32,
    pub goals: [u32; 2],
}

impl Record for MatchStats {
    fn kind(&self) -> &'static str {
        "match-stats"
    }
    fn operation(&self) -> &'static str {
        "simulate"
    }
    fn owner_id(&self) -> &str {
        &self.owner_id
    }
}

/// One `run-report` record: the benchmark result.
#[derive(Debug, Serialize)]
pub struct RunReport {
    #[serde(rename = "owner.id")]
    pub owner_id: String,
    #[serde(rename = "run.id")]
    pub run_id: String,
    pub seed: u64,
    #[serde(rename = "content.hash")]
    pub content_hash: String,
    pub outcome: &'static str,
    #[serde(rename = "bench.matches")]
    pub matches: u32,
    #[serde(rename = "bench.match_wall_ms")]
    pub match_wall_ms: u64,
    #[serde(rename = "bench.cpu_ms")]
    pub cpu_ms: Option<u64>,
    #[serde(rename = "bench.peak_mem_mb")]
    pub peak_mem_mb: Option<f64>,
    #[serde(rename = "engine.ticks_per_s")]
    pub ticks_per_s: f64,
    #[serde(rename = "bench.cpu_wall_ratio")]
    pub cpu_wall_ratio: Option<f64>,
    /// Ticks delivered per second over the socket to an unthrottled client.
    #[serde(
        rename = "bench.stream_ticks_per_s",
        skip_serializing_if = "Option::is_none"
    )]
    pub stream_ticks_per_s: Option<f64>,
    /// Times the producer paused at the buffer bound during the streamed match.
    #[serde(
        rename = "bench.stream_pauses",
        skip_serializing_if = "Option::is_none"
    )]
    pub stream_pauses: Option<u32>,
    #[serde(rename = "machine.hash")]
    pub machine_hash: String,
    #[serde(rename = "machine.cpu_model")]
    pub cpu_model: String,
    #[serde(rename = "machine.power_plan")]
    pub power_plan: String,
    #[serde(rename = "budget.pass")]
    pub budget_pass: bool,
}

impl Record for RunReport {
    fn kind(&self) -> &'static str {
        "run-report"
    }
    fn operation(&self) -> &'static str {
        "benchmark"
    }
    fn owner_id(&self) -> &str {
        &self.owner_id
    }
}

/// The record as one JSON object with the envelope keys first.
pub fn to_json<R: Record>(record: &R) -> Result<String, EngineError> {
    let mut map = serde_json::Map::new();
    map.insert("record.kind".into(), record.kind().into());
    map.insert("schema.version".into(), SCHEMA_VERSION.into());
    map.insert("owner.id".into(), record.owner_id().into());
    map.insert("service".into(), SERVICE.into());
    map.insert("version".into(), crate::version().into());
    map.insert("build.hash".into(), crate::build_hash().into());
    map.insert("env".into(), env_name().into());
    map.insert("operation".into(), record.operation().into());
    let body = serde_json::to_value(record).map_err(|e| EngineError::Format(e.to_string()))?;
    if let serde_json::Value::Object(fields) = body {
        map.extend(fields);
    }
    serde_json::to_string(&serde_json::Value::Object(map))
        .map_err(|e| EngineError::Format(e.to_string()))
}

/// Writes the record as one JSON line on stdout.
pub fn emit_line<R: Record>(record: &R) -> Result<(), EngineError> {
    let line = to_json(record)?;
    let mut out = std::io::stdout().lock();
    out.write_all(line.as_bytes())?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

/// Saves the record as `matches/<match.id>/stats.json` under `data_dir` and returns the path.
pub fn write_stats(data_dir: &Path, stats: &MatchStats) -> Result<PathBuf, EngineError> {
    let dir = data_dir.join("matches").join(&stats.match_id);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("stats.json");
    let line = to_json(stats)?;
    std::fs::write(&path, format!("{line}\n")).map_err(|source| EngineError::Read {
        path: format!("matches/{}/stats.json", stats.match_id),
        source,
    })?;
    Ok(path)
}

/// Reads a saved `stats.json`. Refuses any record kind or schema version this build does
/// not write.
pub fn read_stats(path: &Path) -> Result<MatchStats, EngineError> {
    let text = std::fs::read_to_string(path).map_err(|source| EngineError::Read {
        path: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        source,
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| EngineError::Format(e.to_string()))?;
    let kind = value["record.kind"].as_str().unwrap_or("");
    if kind != "match-stats" {
        return Err(EngineError::Format(format!(
            "stats.json holds record.kind {kind:?}; expected \"match-stats\""
        )));
    }
    let version = value["schema.version"].as_str().unwrap_or("");
    if version != SCHEMA_VERSION {
        return Err(EngineError::Format(format!(
            "stats.json holds schema.version {version:?}; this build reads {SCHEMA_VERSION:?}"
        )));
    }
    serde_json::from_value(value).map_err(|e| EngineError::Format(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats() -> MatchStats {
        MatchStats {
            owner_id: "0123456789abcdef0123456789abcdef".into(),
            match_id: identity::MatchId::now(42).to_string(),
            seed: 42,
            content_hash: "abcdef012345".into(),
            teams: [
                TeamRef {
                    id: "club-a".into(),
                    name: "A".into(),
                },
                TeamRef {
                    id: "club-b".into(),
                    name: "B".into(),
                },
            ],
            duration_ms: 1,
            outcome: "success".into(),
            ticks_per_s: 1.0,
            ticks_written: 1,
            validate_ran: true,
            validate_violations: 0,
            possession_changes: 0,
            ball_max_speed: 0.0,
            ball_idle_ticks: 0,
            goals: [0, 0],
        }
    }

    #[test]
    fn both_records_carry_every_contract_key() {
        let json = to_json(&stats()).unwrap();
        for key in [
            "\"record.kind\":\"match-stats\"",
            "\"schema.version\":\"1\"",
            "\"match.id\"",
            "\"owner.id\":\"0123456789abcdef0123456789abcdef\"",
            "\"service\":\"engine-cli\"",
            "\"build.hash\"",
            "\"operation\":\"simulate\"",
            "\"engine.ticks_per_s\"",
            "\"validate.ran\":true",
            "\"content.hash\":\"abcdef012345\"",
            "\"team.id\":\"club-a\"",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
        let report = RunReport {
            owner_id: "0123456789abcdef0123456789abcdef".into(),
            run_id: "bench-1".into(),
            seed: 42,
            content_hash: "abcdef012345".into(),
            outcome: "success",
            matches: 1,
            match_wall_ms: 1,
            cpu_ms: None,
            peak_mem_mb: None,
            ticks_per_s: 1.0,
            cpu_wall_ratio: None,
            stream_ticks_per_s: None,
            stream_pauses: None,
            machine_hash: machine_hash(),
            cpu_model: "x".into(),
            power_plan: "y".into(),
            budget_pass: true,
        };
        let json = to_json(&report).unwrap();
        for key in [
            "\"record.kind\":\"run-report\"",
            "\"bench.match_wall_ms\"",
            "\"machine.hash\"",
            "\"budget.pass\":true",
            "\"content.hash\"",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
    }

    #[test]
    fn stats_round_trip_through_the_data_folder() {
        let dir = std::env::temp_dir().join(format!("engine-stats-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let written = stats();
        let path = write_stats(&dir, &written).unwrap();
        assert!(
            path.ends_with(
                Path::new("matches")
                    .join(&written.match_id)
                    .join("stats.json")
            )
        );
        let read = read_stats(&path).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(read, written);
    }

    #[test]
    fn an_unknown_stats_version_is_refused() {
        let path =
            std::env::temp_dir().join(format!("engine-stats-v9-{}.json", std::process::id()));
        std::fs::write(
            &path,
            "{\"record.kind\":\"match-stats\",\"schema.version\":\"9\"}\n",
        )
        .unwrap();
        let err = read_stats(&path).unwrap_err();
        std::fs::remove_file(&path).unwrap();
        assert!(err.to_string().contains("schema.version \"9\""), "{err}");
    }

    #[test]
    fn machine_hash_is_twelve_hex_characters() {
        let h = machine_hash();
        assert_eq!(h.len(), 12);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

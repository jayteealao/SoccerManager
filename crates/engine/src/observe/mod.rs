//! Observability records: `match-stats` and `run-report` with the canonical dotted keys,
//! the hashed machine identity, the build hash, and process measurements.

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::EngineError;

pub mod process;

/// Schema version of every record.
pub const SCHEMA_VERSION: &str = "1";
/// Service name carried by every record this crate's binaries emit.
pub const SERVICE: &str = "engine-cli";
/// Owner identifier for headless runs.
pub const OWNER_ID: &str = "local-cli";

/// Milliseconds since the Unix epoch.
pub fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// A match identifier: the seed plus the start time. It never enters the tick file.
pub fn match_id(seed: u64) -> String {
    format!("{seed:016x}-{}", unix_millis())
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
}

/// One `match-stats` record: the wide event of one simulate run.
#[derive(Debug, Serialize)]
pub struct MatchStats {
    #[serde(rename = "match.id")]
    pub match_id: String,
    pub seed: u64,
    pub duration_ms: u64,
    pub outcome: &'static str,
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
}

/// One `run-report` record: the benchmark result.
#[derive(Debug, Serialize)]
pub struct RunReport {
    #[serde(rename = "run.id")]
    pub run_id: String,
    pub seed: u64,
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
}

/// The record as one JSON object with the envelope keys first.
pub fn to_json<R: Record>(record: &R) -> Result<String, EngineError> {
    let mut map = serde_json::Map::new();
    map.insert("record.kind".into(), record.kind().into());
    map.insert("schema.version".into(), SCHEMA_VERSION.into());
    map.insert("owner.id".into(), OWNER_ID.into());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_records_carry_every_contract_key() {
        let stats = MatchStats {
            match_id: match_id(42),
            seed: 42,
            duration_ms: 1,
            outcome: "success",
            ticks_per_s: 1.0,
            ticks_written: 1,
            validate_ran: true,
            validate_violations: 0,
            possession_changes: 0,
            ball_max_speed: 0.0,
            ball_idle_ticks: 0,
            goals: [0, 0],
        };
        let json = to_json(&stats).unwrap();
        for key in [
            "\"record.kind\":\"match-stats\"",
            "\"schema.version\":\"1\"",
            "\"match.id\"",
            "\"owner.id\":\"local-cli\"",
            "\"service\":\"engine-cli\"",
            "\"build.hash\"",
            "\"operation\":\"simulate\"",
            "\"engine.ticks_per_s\"",
            "\"validate.ran\":true",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
        let report = RunReport {
            run_id: "bench-1".into(),
            seed: 42,
            outcome: "success",
            matches: 1,
            match_wall_ms: 1,
            cpu_ms: None,
            peak_mem_mb: None,
            ticks_per_s: 1.0,
            cpu_wall_ratio: None,
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
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
    }

    #[test]
    fn machine_hash_is_twelve_hex_characters() {
        let h = machine_hash();
        assert_eq!(h.len(), 12);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

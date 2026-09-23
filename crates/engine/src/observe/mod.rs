//! Observability records: `match-stats` and `run-report` with the canonical dotted keys,
//! the owner identity, the content hash, the hashed machine identity, the build hash, and
//! process measurements. `match-stats` is also saved under the runtime data folder, and it
//! carries the law counts of the match and its tactics counts: shots, injuries, fatigue,
//! substitutions, AI choices, and the verdicts on queued changes.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::EngineError;
use crate::sim::Summary;

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
    #[serde(flatten, default)]
    pub laws: LawStats,
    #[serde(flatten, default)]
    pub tactics: TacticsStats,
    #[serde(flatten, default)]
    pub figures: MatchFigures,
}

/// The match figures the realism bands read. Per-team arrays are home first. Every key is
/// a contract key; `passes.completed` is an additive extra, and `error.type` appears only
/// on a failed match.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MatchFigures {
    #[serde(rename = "stats.goals")]
    pub goals: [u32; 2],
    #[serde(rename = "stats.shots_on_target")]
    pub shots_on_target: [u32; 2],
    /// Expected goals, two decimals.
    #[serde(rename = "stats.xg")]
    pub xg: [f64; 2],
    #[serde(rename = "stats.passes")]
    pub passes: [u32; 2],
    #[serde(rename = "passes.completed")]
    pub passes_completed: [u32; 2],
    /// Completed passes over passes played, one decimal; 0 for a team that played none.
    #[serde(rename = "stats.pass_accuracy_pct")]
    pub pass_accuracy_pct: [f64; 2],
    /// Each team's share of the open-play ticks, one decimal.
    #[serde(rename = "stats.possession_pct")]
    pub possession_pct: [f64; 2],
    /// `human` or `ai` for each team.
    #[serde(rename = "manager.kind")]
    pub manager_kind: [String; 2],
    #[serde(rename = "error.type", skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
}

impl MatchFigures {
    /// The figures of a match from its summary and its managers.
    pub fn new(s: &Summary, managers: [crate::Manager; 2]) -> Self {
        let possession: u32 = s.possession_ticks[0] + s.possession_ticks[1];
        let share = |part: u32, whole: u32| {
            if whole == 0 {
                0.0
            } else {
                round_to(100.0 * f64::from(part) / f64::from(whole), 1)
            }
        };
        Self {
            goals: s.goals,
            shots_on_target: s.shots_on_target,
            xg: s.xg.map(|x| round_to(x, 2)),
            passes: s.passes,
            passes_completed: s.passes_completed,
            pass_accuracy_pct: [0, 1].map(|t| share(s.passes_completed[t], s.passes[t])),
            possession_pct: [0, 1].map(|t| share(s.possession_ticks[t], possession)),
            manager_kind: managers.map(|m| {
                match m {
                    crate::Manager::Ai => "ai",
                    crate::Manager::Human => "human",
                }
                .to_string()
            }),
            error_type: None,
        }
    }
}

/// `x` rounded to `places` decimals.
pub fn round_to(x: f64, places: i32) -> f64 {
    let scale = 10f64.powi(places);
    (x * scale).round() / scale
}

/// The tactics counts of one match. Per-team arrays are home first. `stats.shots`,
/// `injury.count`, `fatigue.mean_pct`, `darkpath.change_never_applied`, and
/// `change.expired_at_full_time` are contract keys; the rest are additive extras.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TacticsStats {
    #[serde(rename = "stats.shots")]
    pub shots: [u32; 2],
    /// Injuries in the match, both teams.
    #[serde(rename = "injury.count")]
    pub injury_count: u32,
    /// The mean energy of the players on the pitch at the end, 0 to 100.
    #[serde(rename = "fatigue.mean_pct")]
    pub fatigue_mean_pct: f64,
    /// Changes still waiting at the end although a stoppage that admits their kind opened
    /// after they were queued; 0 in a healthy match.
    #[serde(rename = "darkpath.change_never_applied")]
    pub change_never_applied: u32,
    /// Changes still waiting at the end because no stoppage that admits their kind opened
    /// after they were queued. Expected late in a match; no zero rule.
    #[serde(rename = "change.expired_at_full_time")]
    pub change_expired_at_full_time: u32,
    #[serde(rename = "changes.queued")]
    pub changes_queued: u32,
    #[serde(rename = "changes.applied")]
    pub changes_applied: u32,
    #[serde(rename = "changes.rejected")]
    pub changes_rejected: u32,
    pub substitutions: [u32; 2],
    #[serde(rename = "ai.decisions")]
    pub ai_decisions: u32,
}

impl TacticsStats {
    /// The tactics counts of a finished match.
    pub fn new(sim: &crate::Simulation) -> Self {
        let s = sim.summary();
        let unapplied = sim.unapplied_changes();
        Self {
            shots: s.shots,
            injury_count: s.injuries[0] + s.injuries[1],
            fatigue_mean_pct: sim.fatigue_mean_pct(),
            change_never_applied: unapplied.never_applied,
            change_expired_at_full_time: unapplied.expired,
            changes_queued: s.changes_queued,
            changes_applied: s.changes_applied,
            changes_rejected: s.changes_rejected,
            substitutions: s.substitutions,
            ai_decisions: s.ai_decisions,
        }
    }
}

/// The law counts of one match. Per-team arrays are home first. `stats.fouls`,
/// `stats.offsides`, `stats.corners`, and `rules.pack_version` are contract keys; the rest are
/// additive extras.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LawStats {
    #[serde(rename = "stats.fouls")]
    pub fouls: [u32; 2],
    #[serde(rename = "stats.offsides")]
    pub offsides: [u32; 2],
    #[serde(rename = "stats.corners")]
    pub corners: [u32; 2],
    #[serde(rename = "rules.pack_version")]
    pub pack_version: u32,
    #[serde(rename = "cards.yellow")]
    pub yellow: [u32; 2],
    #[serde(rename = "cards.red")]
    pub red: [u32; 2],
    /// Seconds added to each half.
    #[serde(rename = "added_time.s")]
    pub added_s: [u32; 2],
    #[serde(rename = "ticks.played")]
    pub ticks_played: u32,
    /// Stoppages announced through the stoppage hook.
    #[serde(rename = "rules.stoppages")]
    pub stoppages: u32,
    #[serde(rename = "rules.dead_ball_ticks")]
    pub dead_ball_ticks: u32,
    #[serde(rename = "rules.offside_checks")]
    pub offside_checks: u32,
    #[serde(rename = "snapshot.writes")]
    pub snapshot_writes: u32,
}

impl LawStats {
    /// The law counts of a match that played `ticks_played` ticks under rule pack version
    /// `pack_version` and wrote `snapshot_writes` snapshots.
    pub fn new(s: &Summary, pack_version: u32, ticks_played: u32, snapshot_writes: u32) -> Self {
        Self {
            fouls: s.fouls,
            offsides: s.offsides,
            corners: s.corners,
            pack_version,
            yellow: s.yellow,
            red: s.red,
            added_s: s.added_s,
            ticks_played,
            stoppages: s.stoppages,
            dead_ball_ticks: s.dead_ball_ticks,
            offside_checks: s.offside_checks,
            snapshot_writes,
        }
    }
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
    /// Ticks in the median match: added time makes the length vary.
    #[serde(rename = "bench.ticks_per_match")]
    pub ticks_per_match: u32,
    /// Processor time per simulated tick over the timed matches, in microseconds.
    #[serde(rename = "bench.cpu_us_per_tick")]
    pub cpu_us_per_tick: Option<f64>,
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
    let rel = format!("matches/{}/stats.json", stats.match_id);
    write_record_at(&data_dir.join(&rel), &rel, stats)
}

/// Saves the record as `<dir>/<match.id>.json`, the layout of a run folder's `stats/`, and
/// returns the path.
pub fn write_stats_at(dir: &Path, stats: &MatchStats) -> Result<PathBuf, EngineError> {
    let name = format!("{}.json", stats.match_id);
    write_record_at(&dir.join(&name), &format!("stats/{name}"), stats)
}

/// Writes one record as a JSON line at `path`, creating its folder. `shown` is the path an
/// error names, relative to the data folder.
pub fn write_record_at<R: Record>(
    path: &Path,
    shown: &str,
    record: &R,
) -> Result<PathBuf, EngineError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let line = to_json(record)?;
    std::fs::write(path, format!("{line}\n")).map_err(|source| EngineError::Read {
        path: shown.to_string(),
        source,
    })?;
    Ok(path.to_path_buf())
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
            laws: LawStats {
                fouls: [3, 4],
                pack_version: 2,
                ticks_played: 280_000,
                ..LawStats::default()
            },
            tactics: TacticsStats {
                shots: [11, 7],
                injury_count: 1,
                fatigue_mean_pct: 61.25,
                ..TacticsStats::default()
            },
            figures: MatchFigures {
                goals: [2, 1],
                xg: [1.37, 0.52],
                possession_pct: [54.2, 45.8],
                manager_kind: ["ai".into(), "ai".into()],
                ..MatchFigures::default()
            },
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
            "\"stats.fouls\":[3,4]",
            "\"stats.offsides\"",
            "\"stats.corners\"",
            "\"rules.pack_version\":2",
            "\"cards.yellow\"",
            "\"cards.red\"",
            "\"added_time.s\"",
            "\"ticks.played\":280000",
            "\"rules.stoppages\"",
            "\"rules.dead_ball_ticks\"",
            "\"rules.offside_checks\"",
            "\"snapshot.writes\"",
            "\"stats.shots\":[11,7]",
            "\"injury.count\":1",
            "\"fatigue.mean_pct\":61.25",
            "\"darkpath.change_never_applied\":0",
            "\"change.expired_at_full_time\"",
            "\"changes.queued\"",
            "\"changes.applied\"",
            "\"changes.rejected\"",
            "\"substitutions\"",
            "\"ai.decisions\"",
            "\"stats.goals\":[2,1]",
            "\"stats.shots_on_target\"",
            "\"stats.xg\":[1.37,0.52]",
            "\"stats.passes\"",
            "\"stats.pass_accuracy_pct\"",
            "\"stats.possession_pct\":[54.2,45.8]",
            "\"manager.kind\":[\"ai\",\"ai\"]",
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
            ticks_per_match: 276_000,
            cpu_us_per_tick: Some(1.4336),
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
            "\"bench.ticks_per_match\":276000",
            "\"bench.cpu_us_per_tick\":1.4336",
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

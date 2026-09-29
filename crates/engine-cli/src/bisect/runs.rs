//! Running one engine build on the replay file: the probe for the re-simulate options, the
//! state digests of every tick (pass 1), and the state fields and debug trace of one tick
//! (pass 2). Each run has a wall-clock limit. A run counts only when it exits with 0 or 2,
//! prints its verdict line, and writes whole files; anything else is incomplete with the
//! reason, never an equal result.

use std::ffi::OsString;
use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::state_files::{Digests, Field, parse_digests, parse_fields};

/// The re-simulate options bisect needs, as `resimulate --help` names them.
pub const OPTIONS: [&str; 4] = [
    "--state-digests",
    "--state-fields",
    "--at-tick",
    "--debug-trace",
];

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ended {
    Code(i32),
    /// Ended with no exit code (a signal).
    NoCode,
    TimedOut,
}

/// A finished run: how it ended and its two output streams.
struct Ran {
    ended: Ended,
    stdout: String,
    stderr: String,
}

/// Runs `bin` with `args`, its output streams in files under `dir` named after `name`, and
/// kills it after `limit`.
fn run(
    bin: &Path,
    args: &[OsString],
    dir: &Path,
    name: &str,
    limit: Duration,
) -> Result<Ran, String> {
    let out_path = dir.join(format!("{name}.stdout"));
    let err_path = dir.join(format!("{name}.stderr"));
    let open = |path: &Path| {
        File::create(path).map_err(|e| format!("cannot create {}: {e}", path.display()))
    };
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(open(&out_path)?)
        .stderr(open(&err_path)?)
        .spawn()
        .map_err(|e| format!("{} cannot be started: {e}", bin.display()))?;
    let started = Instant::now();
    let ended = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().map_or(Ended::NoCode, Ended::Code),
            Ok(None) if started.elapsed() >= limit => {
                let _ = child.kill();
                let _ = child.wait();
                break Ended::TimedOut;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(format!("cannot wait for {}: {e}", bin.display())),
        }
    };
    let read = |path: &Path| {
        String::from_utf8_lossy(&std::fs::read(path).unwrap_or_default()).into_owned()
    };
    Ok(Ran {
        ended,
        stdout: read(&out_path),
        stderr: read(&err_path),
    })
}

/// The last non-empty line of `text`, or `(nothing)`.
fn last_line(text: &str) -> &str {
    text.lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("(nothing)")
}

/// Checks that `bin` has the re-simulate command and all four options.
pub fn probe(bin: &Path, dir: &Path, name: &str, limit: Duration) -> Result<(), String> {
    let args = ["resimulate".into(), "--help".into()];
    let ran = run(bin, &args, dir, &format!("{name}.probe"), limit)?;
    match ran.ended {
        Ended::Code(0) => {
            let missing: Vec<&str> = OPTIONS
                .into_iter()
                .filter(|o| !ran.stdout.contains(o))
                .collect();
            if missing.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "lacks the state-digest options (added with bisect): resimulate --help does not name {}",
                    missing.join(", ")
                ))
            }
        }
        Ended::TimedOut => Err(format!(
            "timed out after {} s answering resimulate --help",
            limit.as_secs()
        )),
        Ended::Code(code) => Err(format!(
            "lacks the resimulate command: resimulate --help exited with code {code}: {}",
            last_line(&ran.stderr)
        )),
        Ended::NoCode => Err(format!(
            "lacks the resimulate command: resimulate --help ended with no exit code: {}",
            last_line(&ran.stderr)
        )),
    }
}

/// Checks that a re-simulation finished: exit code 0 or 2 and a JSON verdict line last.
fn completed(ran: &Ran, limit: Duration) -> Result<(), String> {
    match ran.ended {
        Ended::TimedOut => {
            return Err(format!(
                "timed out after {} s and was stopped",
                limit.as_secs()
            ));
        }
        Ended::NoCode => {
            return Err(format!(
                "crashed: the run ended with no exit code: {}",
                last_line(&ran.stderr)
            ));
        }
        Ended::Code(0 | 2) => {}
        Ended::Code(code) => {
            return Err(format!(
                "crashed or failed: the run exited with code {code}: {}",
                last_line(&ran.stderr)
            ));
        }
    }
    let verdict: Option<serde_json::Value> = serde_json::from_str(last_line(&ran.stdout)).ok();
    if verdict.as_ref().and_then(|v| v.get("verdict")).is_none() {
        return Err(format!(
            "ends early: the run printed no verdict line (last output line: {})",
            last_line(&ran.stdout)
        ));
    }
    Ok(())
}

/// Pass 1: the state digest of every tick.
pub fn digests(
    bin: &Path,
    fixture: &Path,
    dir: &Path,
    name: &str,
    limit: Duration,
) -> Result<Digests, String> {
    let path = dir.join(format!("{name}.digests"));
    let args: Vec<OsString> = vec![
        "resimulate".into(),
        "--fixture".into(),
        fixture.into(),
        "--compare".into(),
        "--state-digests".into(),
        path.clone().into(),
    ];
    let ran = run(bin, &args, dir, &format!("{name}.pass1"), limit)?;
    completed(&ran, limit)?;
    let text = std::fs::read_to_string(&path)
        .map_err(|_| "ends early: the run wrote no state digest file".to_string())?;
    parse_digests(&text)
}

/// The state of one tick: its named parts and its debug trace records.
#[derive(Debug, Clone, PartialEq)]
pub struct TickState {
    pub fields: Vec<Field>,
    pub trace: Vec<serde_json::Value>,
}

/// Pass 2: the state fields and the debug trace records of `tick`.
pub fn tick_state(
    bin: &Path,
    fixture: &Path,
    dir: &Path,
    name: &str,
    tick: u32,
    limit: Duration,
) -> Result<TickState, String> {
    let fields_path = dir.join(format!("{name}.fields.json"));
    let trace_path = dir.join(format!("{name}.trace.jsonl"));
    let args: Vec<OsString> = vec![
        "resimulate".into(),
        "--fixture".into(),
        fixture.into(),
        "--compare".into(),
        "--state-fields".into(),
        fields_path.clone().into(),
        "--at-tick".into(),
        tick.to_string().into(),
        "--debug-trace".into(),
        trace_path.clone().into(),
    ];
    let ran = run(bin, &args, dir, &format!("{name}.pass2"), limit)?;
    completed(&ran, limit)?;
    let written: serde_json::Value = std::fs::read(&fields_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .ok_or("ends early: the run wrote no state fields file")?;
    let fields = parse_fields(&written)?;
    let trace = crate::trace_file::tick_records(&trace_path, tick)?;
    Ok(TickState { fields, trace })
}

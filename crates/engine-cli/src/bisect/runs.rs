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

use engine::gate::{FieldKind, STATE_DIGEST_FORMAT};

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

/// A whole state digest file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digests {
    pub header: serde_json::Value,
    /// `(tick, digest)` for every tick, from tick 1 with no gap.
    pub ticks: Vec<(u32, String)>,
    /// The digest of the state after full time.
    pub finish: String,
    /// The `end` line's full-time flag.
    pub full_time: bool,
}

impl Digests {
    /// The stream scheme the build played.
    pub fn scheme(&self) -> u64 {
        self.header["scheme"].as_u64().unwrap_or(0)
    }
}

/// Reads a state digest file. A file with no `end` line, a gap in its ticks, or a line
/// count unlike the `end` line's is refused with the reason.
pub fn parse_digests(text: &str) -> Result<Digests, String> {
    let mut lines = text.lines();
    let header: serde_json::Value = lines
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .ok_or("ends early: the state digest file has no header line")?;
    if header["state_digests"].as_u64() != Some(u64::from(STATE_DIGEST_FORMAT)) {
        return Err(format!(
            "the state digest file is format {}, and this bisect reads format {STATE_DIGEST_FORMAT}",
            header["state_digests"]
        ));
    }
    let mut ticks: Vec<(u32, String)> = Vec::new();
    let mut finish = None;
    let mut end = None;
    for line in lines {
        if end.is_some() {
            return Err("the state digest file has lines after its end line".into());
        }
        let mut parts = line.split(' ');
        match (parts.next(), parts.next(), parts.next()) {
            (Some("end"), Some(count), Some(full_time)) => {
                let count: usize = count
                    .parse()
                    .map_err(|_| format!("a bad end line: {line}"))?;
                end = Some((count, full_time == "true"));
            }
            (Some("finish"), Some(digest), None) => finish = Some(digest.to_string()),
            (Some(tick), Some(digest), None) if finish.is_none() => {
                let tick: u32 = tick
                    .parse()
                    .map_err(|_| format!("a bad state digest line: {line}"))?;
                let expected = ticks.last().map_or(1, |(t, _)| t + 1);
                if tick != expected {
                    return Err(format!(
                        "a gap in the state digests: tick {tick} follows tick {}",
                        expected - 1
                    ));
                }
                ticks.push((tick, digest.to_string()));
            }
            _ => return Err(format!("a bad state digest line: {line}")),
        }
    }
    let Some((count, full_time)) = end else {
        return Err(format!(
            "ends early: the state digest file stops after tick {} with no end line",
            ticks.last().map_or(0, |(t, _)| *t)
        ));
    };
    if count != ticks.len() {
        return Err(format!(
            "the state digest file has {} tick lines but its end line counts {count}",
            ticks.len()
        ));
    }
    let finish = finish.ok_or("ends early: the state digest file has no finish line")?;
    Ok(Digests {
        header,
        ticks,
        finish,
        full_time,
    })
}

/// Checks that two builds hash the same state inventory, so their digests compare.
pub fn same_inventory(a: &Digests, b: &Digests) -> Result<(), String> {
    for name in ["state_digests", "inventory"] {
        if a.header[name] != b.header[name] {
            return Err(format!(
                "the two builds hash different state inventories ({name} {} and {})",
                a.header[name], b.header[name]
            ));
        }
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

/// One named part of a tick's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub kind: FieldKind,
    pub bytes: Vec<u8>,
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
    let text = std::fs::read_to_string(&trace_path)
        .map_err(|_| "ends early: the run wrote no debug trace file".to_string())?;
    let trace = text
        .lines()
        .skip(1)
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|r| r["t"].as_u64() == Some(u64::from(tick)))
        .collect();
    Ok(TickState { fields, trace })
}

/// The fields of a state fields file.
fn parse_fields(written: &serde_json::Value) -> Result<Vec<Field>, String> {
    let bad = || "the state fields file is not in the expected form".to_string();
    written["fields"]
        .as_array()
        .ok_or_else(bad)?
        .iter()
        .map(|f| {
            let name = f["name"].as_str().ok_or_else(bad)?.to_string();
            let kind = f["kind"]
                .as_str()
                .and_then(FieldKind::from_name)
                .ok_or_else(bad)?;
            let bytes = unhex(f["hex"].as_str().ok_or_else(bad)?).ok_or_else(bad)?;
            Ok(Field { name, kind, bytes })
        })
        .collect()
}

/// The bytes of lowercase or uppercase hex text.
fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str =
        r#"{"state_digests":1,"inventory":1,"gate_schema":1,"scheme":1,"engine":{}}"#;

    fn file(body: &str) -> String {
        format!("{HEADER}\n{body}")
    }

    #[test]
    fn a_whole_file_is_read() {
        let d = parse_digests(&file("1 aa\n2 bb\nfinish cc\nend 2 true\n")).unwrap();
        assert_eq!(d.ticks, [(1, "aa".to_string()), (2, "bb".to_string())]);
        assert_eq!(d.finish, "cc");
        assert!(d.full_time);
        assert_eq!(d.scheme(), 1);
    }

    #[test]
    fn a_missing_end_line_is_an_early_end() {
        let err = parse_digests(&file("1 aa\n2 bb\nfinish cc\n")).unwrap_err();
        assert!(err.starts_with("ends early"), "{err}");
        assert!(err.contains("after tick 2"), "{err}");
    }

    #[test]
    fn a_gap_or_a_late_start_is_refused() {
        let err = parse_digests(&file("1 aa\n3 bb\nfinish cc\nend 2 true\n")).unwrap_err();
        assert!(err.contains("gap"), "{err}");
        let err = parse_digests(&file("2 aa\nfinish cc\nend 1 true\n")).unwrap_err();
        assert!(err.contains("gap"), "{err}");
    }

    #[test]
    fn a_count_unlike_the_end_line_is_refused() {
        let err = parse_digests(&file("1 aa\n2 bb\nfinish cc\nend 3 true\n")).unwrap_err();
        assert!(err.contains("end line counts 3"), "{err}");
    }

    #[test]
    fn another_format_or_inventory_is_refused() {
        let other = "{\"state_digests\":2}\nend 0 true\n";
        assert!(parse_digests(other).unwrap_err().contains("format 2"));
        let a = parse_digests(&file("finish cc\nend 0 true\n")).unwrap();
        let mut b = a.clone();
        b.header["inventory"] = serde_json::json!(2);
        let err = same_inventory(&a, &b).unwrap_err();
        assert!(err.contains("different state inventories"), "{err}");
        assert_eq!(same_inventory(&a, &a.clone()), Ok(()));
    }

    #[test]
    fn hex_reads_back() {
        assert_eq!(unhex("00ff10"), Some(vec![0, 255, 16]));
        assert_eq!(unhex("0"), None);
        assert_eq!(unhex("zz"), None);
    }
}

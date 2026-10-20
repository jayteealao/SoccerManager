//! A developer's acceptance of a not-sure band, kept in the run folder as `accepted.json`
//! by `engine-cli calibrate accept <band> --run <dir>`.
//!
//! Each judgement of that folder reads the file: a band it names that reads not sure is
//! marked "accepted, not shown equivalent". Its word stays not sure, so the acceptance never
//! counts as a pass, never joins the pass count, the joint word or `calib.pass`. The file
//! names the run and a digest of the run's identity; a file of another run or identity, or
//! one that cannot be read, is ignored and the report says why.

use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The file name inside a run folder.
pub const ACCEPTED_FILE: &str = "accepted.json";
/// The layout version this build writes and reads.
pub const ACCEPTED_VERSION: u32 = 1;

/// The file's layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    schema_version: u32,
    run_id: String,
    /// The first 12 hex characters of the SHA-256 of the run's identity, as `report.json`
    /// holds it under `run.identity`.
    identity: String,
    accepted: Vec<Entry>,
}

/// One accepted band.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    band: String,
    /// When it was accepted, in Unix milliseconds.
    at: u64,
    /// The word it read when it was accepted: always `not sure`.
    word: String,
}

/// `calib.accepted`: the file, the bands it accepts, and why it was ignored when it was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AcceptedInfo {
    pub file: String,
    /// The bands the file accepts; empty when it was ignored.
    pub bands: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignored: Option<String>,
}

/// The digest of a run identity, as `accepted.json` names it.
pub fn identity_digest(identity: &Value) -> String {
    engine::data::hex12(&Sha256::digest(
        serde_json::to_vec(identity).unwrap_or_default(),
    ))
}

/// The acceptances of the run folder `run_dir` for the run `run_id` of identity digest
/// `identity`; `None` without a file.
pub fn read(run_dir: &Path, run_id: &str, identity: &str) -> Option<AcceptedInfo> {
    let path = run_dir.join(ACCEPTED_FILE);
    if !path.is_file() {
        return None;
    }
    let ignored = |why: String| AcceptedInfo {
        file: path.display().to_string(),
        bands: Vec::new(),
        ignored: Some(why),
    };
    let file = match std::fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|b| serde_json::from_slice::<File>(&b).map_err(|e| e.to_string()))
    {
        Ok(f) => f,
        Err(e) => return Some(ignored(format!("{ACCEPTED_FILE} cannot be read: {e}"))),
    };
    if file.schema_version != ACCEPTED_VERSION {
        return Some(ignored(format!(
            "{ACCEPTED_FILE} is version {}; this build reads {ACCEPTED_VERSION}",
            file.schema_version
        )));
    }
    if file.run_id != run_id || file.identity != identity {
        return Some(ignored(format!(
            "{ACCEPTED_FILE} names run {} (identity {}); this folder holds run {run_id} \
             (identity {identity})",
            file.run_id, file.identity
        )));
    }
    Some(AcceptedInfo {
        file: path.display().to_string(),
        bands: file.accepted.into_iter().map(|e| e.band).collect(),
        ignored: None,
    })
}

/// `engine-cli calibrate accept <band> --run <dir>`: records that the developer accepts the
/// band, which reads not sure in the folder's last judgement. A band with no row, or whose
/// rows read pass or fail, is refused by name. The next judgement of the folder shows the
/// mark; this command changes no report.
pub fn accept(band: &str, run_dir: &Path) -> anyhow::Result<i32> {
    let report_path = run_dir.join("report.json");
    let text = std::fs::read_to_string(&report_path).with_context(|| {
        format!(
            "cannot read {}: run calibrate into this folder first",
            report_path.display()
        )
    })?;
    let report: Value = serde_json::from_str(text.trim())
        .with_context(|| format!("{} is not JSON", report_path.display()))?;
    let run_id = report["run.id"]
        .as_str()
        .with_context(|| format!("{} names no run.id", report_path.display()))?
        .to_string();
    let identity = identity_digest(&report["run.identity"]);
    let rows: Vec<&Value> = ["calib.verdicts", "calib.reported"]
        .iter()
        .flat_map(|k| report[*k].as_array().into_iter().flatten())
        .filter(|r| r["band"] == band)
        .collect();
    if rows.is_empty() {
        anyhow::bail!("{band} has no verdict row in run {run_id}; name a band the run judged");
    }
    if !rows.iter().any(|r| r["word"] == "not sure") {
        let mut words: Vec<&str> = rows.iter().filter_map(|r| r["word"].as_str()).collect();
        words.dedup();
        anyhow::bail!(
            "{band} reads {} in run {run_id}; only a not sure band can be accepted",
            words.join(" and ")
        );
    }
    let path = run_dir.join(ACCEPTED_FILE);
    // A file of the same run keeps its acceptances; any other is replaced.
    let mut file = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice::<File>(&b).ok())
        .filter(|f| {
            f.schema_version == ACCEPTED_VERSION && f.run_id == run_id && f.identity == identity
        })
        .unwrap_or(File {
            schema_version: ACCEPTED_VERSION,
            run_id: run_id.clone(),
            identity,
            accepted: Vec::new(),
        });
    if !file.accepted.iter().any(|e| e.band == band) {
        file.accepted.push(Entry {
            band: band.to_string(),
            at: u64::try_from(engine::observe::unix_millis()).unwrap_or(u64::MAX),
            word: "not sure".into(),
        });
    }
    let partial = run_dir.join(format!("{ACCEPTED_FILE}.partial"));
    std::fs::write(&partial, serde_json::to_vec_pretty(&file)?)
        .and_then(|()| std::fs::rename(&partial, &path))
        .with_context(|| format!("cannot write {}", path.display()))?;
    eprintln!(
        "accepted {band} in run {run_id}; run the same calibrate command again to see it \
         marked \"accepted, not shown equivalent\""
    );
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn folder(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("engine-cli-accepted-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn report(dir: &Path) -> Value {
        let report = json!({
            "run.id": "calib-1",
            "run.identity": { "seed": 1, "content": "abc" },
            "calib.verdicts": [
                { "band": "goals_per_match", "word": "pass" },
                { "band": "corners_per_team", "word": "not sure" }
            ],
            "calib.reported": [
                { "band": "fouls_per_team", "word": "not sure" },
                { "band": "five_one_share", "word": "fail" }
            ]
        });
        std::fs::write(dir.join("report.json"), format!("{report}\n")).unwrap();
        report
    }

    #[test]
    fn a_not_sure_band_is_written_appended_once_and_read_back() {
        let dir = folder("write");
        let r = report(&dir);
        let identity = identity_digest(&r["run.identity"]);
        assert_eq!(read(&dir, "calib-1", &identity), None, "no file yet");
        accept("fouls_per_team", &dir).unwrap();
        accept("corners_per_team", &dir).unwrap();
        accept("fouls_per_team", &dir).unwrap();
        let info = read(&dir, "calib-1", &identity).unwrap();
        assert_eq!(info.ignored, None);
        assert_eq!(info.bands, ["fouls_per_team", "corners_per_team"]);
        let file: Value =
            serde_json::from_slice(&std::fs::read(dir.join(ACCEPTED_FILE)).unwrap()).unwrap();
        assert_eq!(file["schema_version"], 1);
        assert_eq!(file["identity"], identity);
        assert_eq!(file["accepted"][0]["word"], "not sure");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_band_that_reads_pass_or_fail_or_has_no_row_is_refused_by_name() {
        let dir = folder("refuse");
        report(&dir);
        let err = accept("five_one_share", &dir).unwrap_err().to_string();
        assert!(
            err.contains("five_one_share reads fail in run calib-1"),
            "{err}"
        );
        assert!(err.contains("only a not sure band"), "{err}");
        let err = accept("goals_per_match", &dir).unwrap_err().to_string();
        assert!(err.contains("reads pass"), "{err}");
        let err = accept("no_such_band", &dir).unwrap_err().to_string();
        assert!(err.contains("no verdict row"), "{err}");
        assert!(!dir.join(ACCEPTED_FILE).exists());
        let err = accept("x", &dir.join("absent")).unwrap_err().to_string();
        assert!(
            err.contains("run calibrate into this folder first"),
            "{err}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_of_another_run_or_identity_or_unreadable_is_ignored_with_its_reason() {
        let dir = folder("ignore");
        let r = report(&dir);
        let identity = identity_digest(&r["run.identity"]);
        accept("fouls_per_team", &dir).unwrap();
        let other = read(&dir, "calib-2", &identity).unwrap();
        assert!(other.bands.is_empty());
        let why = other.ignored.unwrap();
        assert!(why.contains("names run calib-1"), "{why}");
        assert!(why.contains("this folder holds run calib-2"), "{why}");
        let moved = read(&dir, "calib-1", "000000000000").unwrap();
        assert!(moved.ignored.unwrap().contains("identity 000000000000"));
        std::fs::write(dir.join(ACCEPTED_FILE), b"{ not json").unwrap();
        let broken = read(&dir, "calib-1", &identity).unwrap();
        assert!(broken.ignored.unwrap().contains("cannot be read"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

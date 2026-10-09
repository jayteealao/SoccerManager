//! The gate proof: `gate-proof.json` in the content folder records the broken copies of the
//! engine that the proof plays, and the pass and fail rates the gate showed on the engine
//! against itself and on each copy, with the identity of the gate it measured.
//!
//! The gate's identity has three parts: the band registry's digest (what decides each band's
//! verdict, including `gates`), the measures version, and a digest of the judging source
//! files. Every calibrate run compares it with the proof's and says when they differ; the
//! comparison never changes a verdict, a count or an exit code.

use std::collections::{BTreeMap, BTreeSet};

use engine::ContentDir;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::MEASURES_VERSION;
use super::bands::Registry;

/// The proof file's name in the content folder.
pub const PROOF_FILE: &str = "gate-proof.json";
/// The layout version of the proof file.
pub const PROOF_VERSION: u32 = 1;

/// The source files that judge a band, in the order they are hashed.
const JUDGING: [&str; 4] = [
    include_str!("verdict.rs"),
    include_str!("distribution.rs"),
    include_str!("measures.rs"),
    include_str!("bands.rs"),
];

/// What the gate is: the parts a proof's rates hold for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateIdentity {
    /// The band registry's digest.
    pub registry: String,
    /// The version of the measure definitions.
    pub measures: u32,
    /// SHA-256, as 12 hex characters, over the judging source files.
    pub verdict_code: String,
}

/// The identity of the gate this build runs with `reg`.
pub fn current(reg: &Registry) -> GateIdentity {
    GateIdentity {
        registry: reg.digest(),
        measures: MEASURES_VERSION,
        verdict_code: verdict_code(),
    }
}

/// The digest of the judging source files of this build.
pub fn verdict_code() -> String {
    source_digest(&JUDGING)
}

/// SHA-256, as 12 hex characters, over each text with its carriage returns removed and its
/// length first, in order: a checkout with other line endings gives the same digest.
fn source_digest(texts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for text in texts {
        let text = text.replace('\r', "");
        hasher.update((text.len() as u64).to_le_bytes());
        hasher.update(text.as_bytes());
    }
    engine::data::hex12(&hasher.finalize())
}

/// The proof file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProofFile {
    pub schema_version: u32,
    /// The broken copies the proof plays.
    pub copies: Vec<BrokenCopy>,
    /// The last proof; `null` until one runs.
    pub proof: Option<Proof>,
}

/// A deliberately broken copy of the engine: a content flag that moves one measure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrokenCopy {
    /// The flag's name in the proof's content folder.
    pub name: String,
    /// The band whose measure the copy moves.
    pub measure: String,
    /// The expected direction and size of the move, in words.
    #[serde(rename = "move")]
    pub moves: String,
    /// The flag body: owner, hypothesis, removal condition and overrides.
    pub flag: serde_json::Map<String, Value>,
}

/// The pass and fail rates of one proof, and the gate it measured.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proof {
    pub gate: GateIdentity,
    pub made_at: String,
    pub build: ProofBuild,
    /// The first run seed: self run `i` plays `seed + i`, copy run `j` plays `seed + j`.
    pub seed: u64,
    /// The cap of each change run.
    pub matches: u32,
    pub minutes: u32,
    #[serde(rename = "self")]
    pub self_runs: SelfRuns,
    pub copies: Vec<CopyProof>,
    /// For each whole-distribution band, the copies that moved it in any run.
    pub distribution_bands: Vec<DistributionBand>,
    pub counts: ProofCounts,
}

/// The build that played the proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProofBuild {
    pub hash: String,
    pub measures: bool,
}

/// The engine against itself, with new match seeds on the changed side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfRuns {
    pub runs: Vec<SelfRun>,
    /// Runs whose joint word is pass, over the runs; a not sure is not a pass.
    pub pass_rate: f64,
    pub fail_rate: f64,
    pub not_sure_rate: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfRun {
    pub name: String,
    pub seed: u64,
    pub reseed: u64,
    /// The run's joint word.
    pub word: String,
    /// Matches of the run, both engines.
    pub matches: u32,
    /// Bands that read fail, in either table.
    pub failed: Vec<String>,
    /// Bands that read not sure, in either table.
    pub not_sure: Vec<String>,
}

/// One broken copy's runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopyProof {
    pub name: String,
    pub runs: Vec<CopyRun>,
    /// Runs whose joint word is fail, over the runs.
    pub fail_rate: f64,
    /// Runs in which no band read fail: missed detections.
    pub missed: u32,
    /// Each band that read fail, with the number of runs it did.
    pub moved: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopyRun {
    pub seed: u64,
    pub word: String,
    pub matches: u32,
    /// Bands that read fail, in either table; empty for a missed detection.
    pub moved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistributionBand {
    pub band: String,
    /// The copies that moved it; empty when no copy did.
    pub moved_by: Vec<String>,
}

/// Matches over every run of the proof, as each run's report counts them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProofCounts {
    pub runs: u32,
    pub total_matches: u64,
    pub pairs: u64,
    /// Matches of each engine role: `changed` and `old`.
    pub observations: BTreeMap<String, u64>,
}

/// How the proof file of a content folder stands against the current gate.
#[derive(Debug, Clone, PartialEq)]
pub enum Check {
    Matches,
    /// Each part that differs: its name, the proof's value and the current one.
    Changed(Vec<(&'static str, String, String)>),
    NoProof,
    NoFile,
    Unreadable(String),
}

impl Check {
    /// The state's name for the trace event.
    pub fn state(&self) -> &'static str {
        match self {
            Check::Matches => "matches",
            Check::Changed(_) => "changed",
            Check::NoProof => "no_proof",
            Check::NoFile => "no_file",
            Check::Unreadable(_) => "unreadable",
        }
    }

    /// The console lines; none when the proof measured this gate.
    pub fn lines(&self) -> Option<String> {
        const CHANGED: &str = "gate changed since the last proof";
        match self {
            Check::Matches => None,
            Check::Changed(parts) => Some(
                parts
                    .iter()
                    .map(|(part, was, now)| format!("{CHANGED}: {part} {was} -> {now}\n"))
                    .collect(),
            ),
            Check::NoProof => Some(format!("{CHANGED}: no proof recorded in {PROOF_FILE}\n")),
            Check::NoFile => Some("no gate proof in this content folder\n".to_string()),
            Check::Unreadable(why) => Some(format!("{CHANGED}: {PROOF_FILE} {why}\n")),
        }
    }
}

/// Reads the proof file of a content folder.
pub fn read(dir: &ContentDir) -> anyhow::Result<Option<ProofFile>> {
    let path = dir.path(PROOF_FILE);
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)?;
    parse(&text).map(Some)
}

/// The proof file from its text; an unknown field or another version is refused by name.
pub fn parse(text: &str) -> anyhow::Result<ProofFile> {
    let file: ProofFile =
        serde_json::from_str(text).map_err(|e| anyhow::anyhow!("cannot be read: {e}"))?;
    if file.schema_version != PROOF_VERSION {
        anyhow::bail!(
            "has schema_version {}; this build reads {PROOF_VERSION}",
            file.schema_version
        );
    }
    Ok(file)
}

/// Compares the proof file of `dir` with the gate `now`.
pub fn check(dir: &ContentDir, now: &GateIdentity) -> Check {
    match read(dir) {
        Ok(None) => Check::NoFile,
        Err(e) => Check::Unreadable(format!("{e:#}")),
        Ok(Some(file)) => compare(file.proof.as_ref().map(|p| &p.gate), now),
    }
}

fn compare(was: Option<&GateIdentity>, now: &GateIdentity) -> Check {
    let Some(was) = was else {
        return Check::NoProof;
    };
    let mut parts = Vec::new();
    if was.registry != now.registry {
        parts.push(("registry", was.registry.clone(), now.registry.clone()));
    }
    if was.measures != now.measures {
        parts.push((
            "measures",
            was.measures.to_string(),
            now.measures.to_string(),
        ));
    }
    if was.verdict_code != now.verdict_code {
        parts.push((
            "verdict code",
            was.verdict_code.clone(),
            now.verdict_code.clone(),
        ));
    }
    if parts.is_empty() {
        Check::Matches
    } else {
        Check::Changed(parts)
    }
}

/// The whole-distribution bands of a registry, in its order.
pub fn distribution_bands(reg: &Registry) -> Vec<String> {
    reg.bands
        .iter()
        .filter(|b| b.is_distribution())
        .map(|b| b.band.clone())
        .collect()
}

/// One finished change run of the proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunMeta {
    pub name: String,
    pub seed: u64,
    pub reseed: Option<u64>,
}

/// What a run's report says: its joint word, its bands by word, its matches.
struct Read {
    word: String,
    failed: Vec<String>,
    not_sure: Vec<String>,
    total: u32,
    pairs: u32,
    observations: Vec<(String, u32)>,
}

fn read_report(report: &Value) -> Read {
    let word = report["calib.joint"]["word"]
        .as_str()
        .unwrap_or("none")
        .to_string();
    let mut failed = BTreeSet::new();
    let mut not_sure = BTreeSet::new();
    for table in ["calib.verdicts", "calib.reported"] {
        for row in report[table].as_array().into_iter().flatten() {
            let band = row["band"].as_str().unwrap_or_default().to_string();
            match row["word"].as_str() {
                Some("fail") => {
                    failed.insert(band);
                }
                Some("not sure") => {
                    not_sure.insert(band);
                }
                _ => {}
            }
        }
    }
    let counts = &report["calib.counts"];
    let n = |v: &Value| u32::try_from(v.as_u64().unwrap_or(0)).unwrap_or(u32::MAX);
    Read {
        word,
        failed: failed.into_iter().collect(),
        not_sure: not_sure.into_iter().collect(),
        total: n(&counts["total_matches"]),
        pairs: n(&counts["pairs"]),
        observations: counts["observations"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|o| {
                (
                    o["engine"].as_str().unwrap_or("?").to_string(),
                    n(&o["matches"]),
                )
            })
            .collect(),
    }
}

/// `part` over `whole`, rounded to 4 places; 0 for no runs.
fn rate(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    (part as f64 / whole as f64 * 1e4).round() / 1e4
}

/// The proof's rates and moves from the finished reports of its runs: `self_runs` holds the
/// self runs, `copy_runs` each copy run with its copy's name.
pub fn summarise(
    self_runs: &[(RunMeta, Value)],
    copy_runs: &[(String, RunMeta, Value)],
    copies: &[BrokenCopy],
    dist_bands: &[String],
) -> (SelfRuns, Vec<CopyProof>, Vec<DistributionBand>, ProofCounts) {
    let mut counts = ProofCounts::default();
    let mut count = |r: &Read| {
        counts.runs += 1;
        counts.total_matches += u64::from(r.total);
        counts.pairs += u64::from(r.pairs);
        for (engine, n) in &r.observations {
            *counts.observations.entry(engine.clone()).or_default() += u64::from(*n);
        }
    };
    let runs: Vec<SelfRun> = self_runs
        .iter()
        .map(|(meta, report)| {
            let r = read_report(report);
            count(&r);
            SelfRun {
                name: meta.name.clone(),
                seed: meta.seed,
                reseed: meta.reseed.unwrap_or(0),
                word: r.word,
                matches: r.total,
                failed: r.failed,
                not_sure: r.not_sure,
            }
        })
        .collect();
    let words = |w: &str| runs.iter().filter(|r| r.word == w).count();
    let self_proof = SelfRuns {
        pass_rate: rate(words("pass"), runs.len()),
        fail_rate: rate(words("fail"), runs.len()),
        not_sure_rate: rate(words("not sure"), runs.len()),
        runs,
    };
    let copy_proofs: Vec<CopyProof> = copies
        .iter()
        .map(|copy| {
            let runs: Vec<CopyRun> = copy_runs
                .iter()
                .filter(|(name, _, _)| *name == copy.name)
                .map(|(_, meta, report)| {
                    let r = read_report(report);
                    count(&r);
                    CopyRun {
                        seed: meta.seed,
                        word: r.word,
                        matches: r.total,
                        moved: r.failed,
                    }
                })
                .collect();
            let mut moved: BTreeMap<String, u32> = BTreeMap::new();
            for band in runs.iter().flat_map(|r| &r.moved) {
                *moved.entry(band.clone()).or_default() += 1;
            }
            CopyProof {
                name: copy.name.clone(),
                fail_rate: rate(runs.iter().filter(|r| r.word == "fail").count(), runs.len()),
                missed: u32::try_from(runs.iter().filter(|r| r.moved.is_empty()).count())
                    .unwrap_or(u32::MAX),
                moved,
                runs,
            }
        })
        .collect();
    let distribution = dist_bands
        .iter()
        .map(|band| DistributionBand {
            band: band.clone(),
            moved_by: copy_proofs
                .iter()
                .filter(|c| c.moved.contains_key(band))
                .map(|c| c.name.clone())
                .collect(),
        })
        .collect();
    (self_proof, copy_proofs, distribution, counts)
}

/// The console summary of a proof: one line per copy, the self runs, the distribution
/// bands, and the counts.
pub fn render(p: &Proof) -> String {
    let mut out = format!(
        "gate proof of registry {}, measures {}, verdict code {}\n",
        p.gate.registry, p.gate.measures, p.gate.verdict_code
    );
    out.push_str(&format!(
        "self runs: {} runs, pass rate {:.2}, fail rate {:.2}, not sure rate {:.2}\n",
        p.self_runs.runs.len(),
        p.self_runs.pass_rate,
        p.self_runs.fail_rate,
        p.self_runs.not_sure_rate
    ));
    for c in &p.copies {
        let moved: Vec<String> = c.moved.iter().map(|(b, n)| format!("{b} {n}")).collect();
        out.push_str(&format!(
            "copy {}: fail rate {:.2} over {} runs; {} missed; bands moved: {}\n",
            c.name,
            c.fail_rate,
            c.runs.len(),
            c.missed,
            if moved.is_empty() {
                "none".to_string()
            } else {
                moved.join(", ")
            }
        ));
    }
    for d in &p.distribution_bands {
        out.push_str(&format!(
            "distribution band {}: {}\n",
            d.band,
            if d.moved_by.is_empty() {
                "no copy moved it".to_string()
            } else {
                format!("moved by {}", d.moved_by.join(", "))
            }
        ));
    }
    let per: Vec<String> = p
        .counts
        .observations
        .iter()
        .map(|(e, n)| format!("{e} engine {n}"))
        .collect();
    out.push_str(&format!(
        "matches: {} in total over {} runs; {} pairs; observations: {}\n",
        p.counts.total_matches,
        p.counts.runs,
        p.counts.pairs,
        per.join(", ")
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn gate() -> GateIdentity {
        GateIdentity {
            registry: "aaaaaaaaaaaa".into(),
            measures: 2,
            verdict_code: "bbbbbbbbbbbb".into(),
        }
    }

    #[test]
    fn a_matching_gate_prints_nothing_and_a_changed_one_names_each_part() {
        assert_eq!(compare(Some(&gate()), &gate()), Check::Matches);
        assert_eq!(Check::Matches.lines(), None);
        let now = GateIdentity {
            registry: "cccccccccccc".into(),
            ..gate()
        };
        let check = compare(Some(&gate()), &now);
        assert_eq!(check.state(), "changed");
        assert_eq!(
            check.lines().unwrap(),
            "gate changed since the last proof: registry aaaaaaaaaaaa -> cccccccccccc\n"
        );
        let all = GateIdentity {
            registry: "cccccccccccc".into(),
            measures: 3,
            verdict_code: "dddddddddddd".into(),
        };
        let lines = compare(Some(&gate()), &all).lines().unwrap();
        assert_eq!(lines.lines().count(), 3, "{lines}");
        assert!(lines.contains("measures 2 -> 3"), "{lines}");
        assert!(
            lines.contains("verdict code bbbbbbbbbbbb -> dddddddddddd"),
            "{lines}"
        );
    }

    #[test]
    fn no_proof_and_no_file_each_print_their_own_line() {
        assert_eq!(
            compare(None, &gate()).lines().unwrap(),
            "gate changed since the last proof: no proof recorded in gate-proof.json\n"
        );
        assert_eq!(
            Check::NoFile.lines().unwrap(),
            "no gate proof in this content folder\n"
        );
    }

    #[test]
    fn the_verdict_code_is_12_hex_and_ignores_line_endings() {
        let code = verdict_code();
        assert_eq!(code.len(), 12);
        assert!(code.chars().all(|c| c.is_ascii_hexdigit()), "{code}");
        assert_eq!(
            source_digest(&["a\r\nb\r\n", "c"]),
            source_digest(&["a\nb\n", "c"])
        );
        assert_ne!(source_digest(&["ab", "c"]), source_digest(&["a", "bc"]));
    }

    #[test]
    fn the_proof_file_refuses_an_unknown_field_by_name() {
        let ok = json!({ "schema_version": 1, "copies": [], "proof": null }).to_string();
        assert_eq!(parse(&ok).unwrap().proof, None);
        let bad = json!({ "schema_version": 1, "copies": [], "proof": null, "extra": 1 });
        let err = parse(&bad.to_string()).unwrap_err().to_string();
        assert!(err.contains("extra"), "{err}");
        let old = json!({ "schema_version": 9, "copies": [], "proof": null }).to_string();
        assert!(parse(&old).unwrap_err().to_string().contains("9"));
    }

    #[test]
    fn the_shipped_proof_file_reads_and_names_four_copies() {
        let text = include_str!("../../../../content/gate-proof.json");
        let file = parse(text).unwrap();
        let names: Vec<&str> = file.copies.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "shots_up",
                "fouls_halved",
                "card_storm",
                "pass_completion_down"
            ]
        );
        for c in &file.copies {
            for key in ["owner", "hypothesis", "removal_condition", "overrides"] {
                assert!(c.flag.contains_key(key), "{} lacks {key}", c.name);
            }
        }
    }

    fn report(word: &str, verdicts: &[(&str, &str)], reported: &[(&str, &str)]) -> Value {
        let rows = |r: &[(&str, &str)]| -> Vec<Value> {
            r.iter()
                .map(|(b, w)| json!({ "band": b, "word": w }))
                .collect()
        };
        json!({
            "calib.joint": { "word": word },
            "calib.verdicts": rows(verdicts),
            "calib.reported": rows(reported),
            "calib.counts": {
                "total_matches": 20, "pairs": 10,
                "observations": [
                    { "engine": "changed", "build": "x", "matches": 10 },
                    { "engine": "old", "build": "y", "matches": 10 }
                ]
            }
        })
    }

    fn meta(i: u64) -> RunMeta {
        RunMeta {
            name: format!("self-{i}"),
            seed: 100 + i,
            reseed: Some(i + 1),
        }
    }

    fn copy(name: &str) -> BrokenCopy {
        BrokenCopy {
            name: name.into(),
            measure: "shots_per_team".into(),
            moves: "up".into(),
            flag: serde_json::Map::new(),
        }
    }

    #[test]
    fn a_not_sure_self_run_is_never_a_pass() {
        let mut selfs: Vec<(RunMeta, Value)> = (0..9)
            .map(|i| (meta(i), report("pass", &[("goals", "pass")], &[])))
            .collect();
        selfs.push((
            meta(9),
            report("not sure", &[("goals", "not sure")], &[("x", "pass")]),
        ));
        let (s, _, _, counts) = summarise(&selfs, &[], &[], &[]);
        assert_eq!((s.pass_rate, s.not_sure_rate, s.fail_rate), (0.9, 0.1, 0.0));
        assert_eq!(s.runs[9].not_sure, ["goals"]);
        assert_eq!(s.runs[9].reseed, 10);
        assert_eq!(
            (counts.runs, counts.total_matches, counts.pairs),
            (10, 200, 100)
        );
        assert_eq!(counts.observations["changed"], 100);
    }

    #[test]
    fn a_copy_counts_its_fails_its_moves_from_both_tables_and_its_misses() {
        let runs: Vec<(String, RunMeta, Value)> = (0..5)
            .map(|j| {
                let r = if j < 4 {
                    report(
                        "fail",
                        &[("shots_per_team", "fail"), ("goals", "pass")],
                        &[("goals_distribution", "fail")],
                    )
                } else {
                    report("not sure", &[("shots_per_team", "not sure")], &[])
                };
                ("shots_up".to_string(), meta(j), r)
            })
            .collect();
        let dist = [
            "goals_distribution".to_string(),
            "scoreline_cells".to_string(),
        ];
        let (_, copies, bands, counts) =
            summarise(&[], &runs, &[copy("shots_up"), copy("idle")], &dist);
        let c = &copies[0];
        assert_eq!(c.fail_rate, 0.8);
        assert_eq!(c.missed, 1);
        assert!(c.runs[4].moved.is_empty());
        assert_eq!(c.moved["shots_per_team"], 4);
        assert_eq!(c.moved["goals_distribution"], 4);
        assert_eq!(copies[1].runs.len(), 0);
        assert_eq!(copies[1].fail_rate, 0.0);
        assert_eq!(bands[0].moved_by, ["shots_up"]);
        assert!(bands[1].moved_by.is_empty());
        assert_eq!(counts.runs, 5);
    }
}

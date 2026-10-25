//! The data check: counts the raw files the reference file is built from and names each
//! missing one. football-data.co.uk and FotMob files are also checked against the sha256
//! of their manifest; the SoccerTrack v2 annotation files (about 56 GB) by size only.

use std::fmt::Write as _;
use std::io::Read;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The 22 football-data.co.uk divisions, each with its country (ClubElo's code) and tier.
pub const DIVISIONS: [(&str, &str, u32); 22] = [
    ("E0", "ENG", 1),
    ("E1", "ENG", 2),
    ("E2", "ENG", 3),
    ("E3", "ENG", 4),
    ("EC", "ENG", 5),
    ("SC0", "SCO", 1),
    ("SC1", "SCO", 2),
    ("SC2", "SCO", 3),
    ("SC3", "SCO", 4),
    ("D1", "GER", 1),
    ("D2", "GER", 2),
    ("I1", "ITA", 1),
    ("I2", "ITA", 2),
    ("SP1", "ESP", 1),
    ("SP2", "ESP", 2),
    ("F1", "FRA", 1),
    ("F2", "FRA", 2),
    ("N1", "NED", 1),
    ("B1", "BEL", 1),
    ("P1", "POR", 1),
    ("T1", "TUR", 1),
    ("G1", "GRE", 1),
];

/// The five seasons, as football-data.co.uk names their folders.
pub const SEASONS: [&str; 5] = ["2122", "2223", "2324", "2425", "2526"];

/// The five FotMob leagues and their football-data.co.uk division.
pub const FOTMOB_LEAGUES: [(&str, &str); 5] = [
    ("premier-league", "E0"),
    ("laliga", "SP1"),
    ("bundesliga", "D1"),
    ("serie-a", "I1"),
    ("ligue-1", "F1"),
];

/// The SoccerTrack v2 annotation files this check expects.
pub const SOCCERTRACK_FILES: usize = 20;

/// One group of files: how many were expected, how many are there, and what is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: &'static str,
    pub expected: usize,
    pub found: usize,
    /// Files missing, each as a path relative to the data folder.
    pub missing: Vec<String>,
    /// Files there but not as their manifest records them.
    pub wrong: Vec<String>,
}

/// What the check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub folder: String,
    pub groups: Vec<Group>,
}

impl Report {
    /// `true` when every expected file is there and as recorded.
    pub fn complete(&self) -> bool {
        self.groups
            .iter()
            .all(|g| g.found == g.expected && g.missing.is_empty() && g.wrong.is_empty())
    }

    /// The printed report: one line per group, then each missing or wrong file.
    pub fn text(&self) -> String {
        let mut out = format!("data folder: {}\n", self.folder);
        for g in &self.groups {
            let _ = writeln!(out, "{:<16} {:>3} of {:>3}", g.name, g.found, g.expected);
        }
        for g in &self.groups {
            for m in &g.missing {
                let _ = writeln!(out, "missing: {m}");
            }
            for w in &g.wrong {
                let _ = writeln!(out, "not as recorded: {w}");
            }
        }
        out.push_str(if self.complete() {
            "every file is there\n"
        } else {
            "files are missing or not as recorded\n"
        });
        out
    }
}

#[derive(Deserialize)]
struct Manifest {
    files: Vec<ManifestFile>,
}

#[derive(Deserialize)]
struct ManifestFile {
    file: String,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    bytes: Option<u64>,
}

/// The manifest entries of `path` by file name; empty when the manifest cannot be read.
fn manifest(path: &Path) -> std::collections::BTreeMap<String, ManifestFile> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<Manifest>(&t).ok())
        .map(|m| m.files.into_iter().map(|f| (f.file.clone(), f)).collect())
        .unwrap_or_default()
}

/// The sha256 of the file at `path`, as 64 hex characters, and its size.
pub fn sha256_file(path: &Path) -> std::io::Result<(String, u64)> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    let mut bytes = 0u64;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        hasher.update(&buf[..n]);
    }
    Ok((stream::record::hex(&hasher.finalize()), bytes))
}

/// Checks the files named in `expected` (relative to `base`, shown under `prefix`) against
/// the manifest at `manifest_path`.
fn hashed_group(
    name: &'static str,
    base: &Path,
    prefix: &str,
    expected: &[String],
    manifest_path: &Path,
) -> Group {
    let recorded = manifest(manifest_path);
    let mut group = Group {
        name,
        expected: expected.len(),
        found: 0,
        missing: Vec::new(),
        wrong: Vec::new(),
    };
    for file in expected {
        let path = base.join(file);
        if !path.is_file() {
            group.missing.push(format!("{prefix}/{file}"));
            continue;
        }
        group.found += 1;
        let want = recorded.get(file).and_then(|m| m.sha256.as_deref());
        match (want, sha256_file(&path)) {
            (Some(want), Ok((got, _))) if want != got => {
                group.wrong.push(format!(
                    "{prefix}/{file} (sha256 differs from the manifest)"
                ));
            }
            (None, _) => group
                .wrong
                .push(format!("{prefix}/{file} (not in the manifest)")),
            (_, Err(e)) => group.wrong.push(format!("{prefix}/{file} ({e})")),
            _ => {}
        }
    }
    group
}

/// Counts the files of `data` and names each missing one.
pub fn check(data: &Path) -> anyhow::Result<Report> {
    let fd: Vec<String> = SEASONS
        .iter()
        .flat_map(|s| {
            DIVISIONS
                .iter()
                .map(move |(d, _, _)| format!("{s}/{d}.csv"))
        })
        .collect();
    let fotmob: Vec<String> = FOTMOB_LEAGUES
        .iter()
        .flat_map(|(l, _)| SEASONS.iter().map(move |s| format!("{l}-{s}.csv")))
        .collect();
    let mut groups = vec![
        hashed_group(
            "football-data",
            &data.join("football-data"),
            "football-data",
            &fd,
            &data.join("football-data").join("manifest.json"),
        ),
        hashed_group(
            "fotmob",
            &data.join("fotmob"),
            "fotmob",
            &fotmob,
            &data.join("fotmob").join("manifest.json"),
        ),
    ];

    // SoccerTrack v2: the names and sizes its download record gives, by size only.
    let st = data.join("soccertrack-v2");
    let listed = manifest(&st.join("download-status.json"));
    let mut group = Group {
        name: "soccertrack-v2",
        expected: SOCCERTRACK_FILES,
        found: 0,
        missing: Vec::new(),
        wrong: Vec::new(),
    };
    if listed.is_empty() {
        group
            .missing
            .push("soccertrack-v2/download-status.json (it names the annotation files)".into());
    }
    for (file, entry) in &listed {
        let path = st.join(file);
        match std::fs::metadata(&path) {
            Ok(meta) if meta.is_file() => {
                group.found += 1;
                if entry.bytes.is_some_and(|b| b != meta.len()) {
                    group.wrong.push(format!(
                        "soccertrack-v2/{file} (size differs from its record)"
                    ));
                }
            }
            _ => group.missing.push(format!("soccertrack-v2/{file}")),
        }
    }
    if !listed.is_empty() && listed.len() != SOCCERTRACK_FILES {
        group.wrong.push(format!(
            "soccertrack-v2/download-status.json names {} files, not {SOCCERTRACK_FILES}",
            listed.len()
        ));
    }
    groups.push(group);

    // ClubElo: one snapshot, clubelo/clubelo-<date>.csv.
    let snapshots = clubelo_snapshots(data);
    groups.push(Group {
        name: "clubelo",
        expected: 1,
        found: snapshots.len().min(1),
        missing: if snapshots.is_empty() {
            vec!["clubelo/clubelo-<date>.csv (a ClubElo snapshot)".into()]
        } else {
            Vec::new()
        },
        wrong: Vec::new(),
    });
    Ok(Report {
        folder: data.display().to_string(),
        groups,
    })
}

/// The ClubElo snapshot files `clubelo/clubelo-<YYYY-MM-DD>.csv`, latest last.
pub fn clubelo_snapshots(data: &Path) -> Vec<std::path::PathBuf> {
    let mut out: Vec<_> = std::fs::read_dir(data.join("clubelo"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .and_then(|n| n.strip_prefix("clubelo-"))
                        .and_then(|n| n.strip_suffix(".csv"))
                        .is_some_and(|d| {
                            d.len() == 10 && d.chars().all(|c| c.is_ascii_digit() || c == '-')
                        })
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

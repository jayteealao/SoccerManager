//! Which engine program finishes a saved match. A snapshot resumes only on the build that
//! wrote it, so a release ships the previous release's engine beside its own, as
//! `previous/engine-cli` with that release's own `previous/content/`. The resolver reads who
//! wrote a save ([`Snapshot::identify`]) and picks a program by release version: this
//! program for its own version, the previous program for the previous release's version,
//! and a refusal that names the save's version for anything else. The snapshot format never
//! decides the program.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use engine::{Snapshot, SnapshotIdentity};

/// The previous release this program ships beside itself, pinned in
/// `packaging/previous-engine.json`.
const PIN: &str = include_str!("../../../packaging/previous-engine.json");

/// The release version of the previous engine this program ships beside itself.
pub fn previous_version() -> String {
    serde_json::from_str::<serde_json::Value>(PIN)
        .ok()
        .and_then(|pin| pin["version"].as_str().map(str::to_string))
        .expect("packaging/previous-engine.json names a version")
}

/// The previous engine program and the content folder it plays with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviousEngine {
    pub program: PathBuf,
    pub content: PathBuf,
}

impl PreviousEngine {
    /// `--previous-engine`, then `SM_PREVIOUS_ENGINE_PATH`, then `previous/engine-cli` beside
    /// this program. Its content folder is the `content` folder beside it.
    pub fn locate(flag: Option<&Path>) -> Self {
        let program = flag
            .map(Path::to_path_buf)
            .or_else(|| {
                std::env::var_os("SM_PREVIOUS_ENGINE_PATH")
                    .filter(|p| !p.is_empty())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| {
                // nosemgrep: rust.lang.security.current-exe.current-exe -- only used to find the engine shipped beside this program
                let here = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("engine-cli"));
                let dir = here.parent().map(Path::to_path_buf).unwrap_or_default();
                dir.join("previous").join(program_name())
            });
        let content = program
            .parent()
            .map(|dir| dir.join("content"))
            .unwrap_or_else(|| PathBuf::from("content"));
        Self { program, content }
    }

    /// The version the program prints with `--version`, or `None` when it does not run.
    pub fn version(&self) -> Option<String> {
        let out = Command::new(&self.program)
            .arg("--version")
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .nth(1)
            .map(str::to_string)
    }
}

fn program_name() -> &'static str {
    if cfg!(windows) {
        "engine-cli.exe"
    } else {
        "engine-cli"
    }
}

/// Why no shipped program can finish a save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalKind {
    /// Saved by a release older than the previous one: two or more versions back.
    Older,
    /// Saved by a newer release than this one.
    Newer,
    /// Saved by a version between the two this release carries, or beside them.
    Other,
    /// Saved by a build no release shipped.
    Unreleased,
    /// Saved by the previous release, whose engine is missing from this copy.
    PreviousMissing,
}

impl RefusalKind {
    pub fn word(self) -> &'static str {
        match self {
            RefusalKind::Older => "older",
            RefusalKind::Newer => "newer",
            RefusalKind::Other => "other",
            RefusalKind::Unreleased => "unreleased",
            RefusalKind::PreviousMissing => "previous-missing",
        }
    }
}

/// A save no shipped program can finish, with the words the page and the console show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub kind: RefusalKind,
    pub identity: SnapshotIdentity,
    /// The plain reason, which names the save's version.
    pub reason: String,
}

/// The program that finishes a save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    /// This program: the save names this program's version (the exact build check still
    /// runs when the snapshot is read).
    Current,
    /// The previous release's program, with its own content folder.
    Previous {
        engine: PreviousEngine,
        version: String,
    },
    /// No shipped program can finish it.
    Refused(Refusal),
}

/// Picks the program for the save at `path`. A file that does not identify (missing,
/// damaged, not a snapshot) goes to this program, whose strict reader refuses it by name.
pub fn resolve(path: &Path, previous: &PreviousEngine) -> (Choice, Option<SnapshotIdentity>) {
    let Ok(bytes) = std::fs::read(path) else {
        return (Choice::Current, None);
    };
    let Ok(identity) = Snapshot::identify(&bytes) else {
        return (Choice::Current, None);
    };
    let choice = choose(&identity, engine::version(), &previous_version(), || {
        previous.version()
    })
    .map_or(Choice::Current, |found| match found {
        Ok(version) => Choice::Previous {
            engine: previous.clone(),
            version,
        },
        Err(refusal) => Choice::Refused(refusal),
    });
    (choice, Some(identity))
}

/// The pure half of [`resolve`]: `None` for this program, `Some(Ok(version))` for the
/// previous program, `Some(Err(_))` for a refusal. `found` reads the previous program's
/// version only when the save needs it.
pub fn choose(
    identity: &SnapshotIdentity,
    current: &str,
    previous: &str,
    found: impl FnOnce() -> Option<String>,
) -> Option<Result<String, Refusal>> {
    let refuse = |kind: RefusalKind, reason: String| {
        Some(Err(Refusal {
            kind,
            identity: identity.clone(),
            reason,
        }))
    };
    let Some(saved) = identity.engine_version.as_deref() else {
        return refuse(
            RefusalKind::Unreleased,
            format!(
                "this match was saved by {}, which no release shipped; this version finishes \
                 matches saved by Touchline {current} and {previous}",
                identity.writer()
            ),
        );
    };
    if saved == current {
        return None;
    }
    if saved == previous {
        return match found() {
            Some(version) if version == previous => Some(Ok(version)),
            _ => refuse(
                RefusalKind::PreviousMissing,
                format!(
                    "this match was saved by Touchline {saved}, and the engine of {saved} is \
                     missing from this copy of the game"
                ),
            ),
        };
    }
    let (kind, words) = match (
        compare_versions(saved, previous),
        compare_versions(saved, current),
    ) {
        (Some(Ordering::Less), _) => (RefusalKind::Older, "two or more versions back"),
        (_, Some(Ordering::Greater)) => (RefusalKind::Newer, "newer than this version"),
        _ => (RefusalKind::Other, "not one this version carries"),
    };
    refuse(
        kind,
        format!(
            "this match was saved by Touchline {saved}, {words}; this version finishes \
             matches saved by Touchline {current} and {previous}"
        ),
    )
}

/// Semantic-version precedence of two `major.minor.patch[-pre]` versions; `None` when either
/// does not parse. Build metadata (`+...`) is ignored.
pub fn compare_versions(a: &str, b: &str) -> Option<Ordering> {
    let (a_core, a_pre) = split(a)?;
    let (b_core, b_pre) = split(b)?;
    Some(a_core.cmp(&b_core).then_with(|| match (a_pre, b_pre) {
        (None, None) => Ordering::Equal,
        // A pre-release sorts before its release.
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (Some(a), Some(b)) => compare_pre(a, b),
    }))
}

fn split(version: &str) -> Option<([u64; 3], Option<&str>)> {
    let version = version.split('+').next()?;
    let (core, pre) = match version.split_once('-') {
        Some((core, pre)) if !pre.is_empty() => (core, Some(pre)),
        Some(_) => return None,
        None => (version, None),
    };
    let mut parts = core.split('.');
    let mut out = [0u64; 3];
    for slot in &mut out {
        *slot = parts.next()?.parse().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    Some((out, pre))
}

/// Pre-release precedence: dot-separated identifiers, numbers below words, numbers by value,
/// words by ASCII order, and a shorter list first when every shared identifier is equal.
fn compare_pre(a: &str, b: &str) -> Ordering {
    let mut a = a.split('.');
    let mut b = b.split('.');
    loop {
        match (a.next(), b.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let order = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(x), Ok(y)) => x.cmp(&y),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => x.cmp(y),
                };
                if order != Ordering::Equal {
                    return order;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(version: Option<&str>) -> SnapshotIdentity {
        SnapshotIdentity {
            format: 8,
            build_hash: "abc1234".into(),
            engine_version: version.map(str::to_string),
            tick: Some(156_500),
            teams: None,
            score: None,
        }
    }

    #[test]
    fn versions_follow_semantic_precedence() {
        use Ordering::*;
        for (a, b, order) in [
            ("0.1.0", "0.2.0-beta.1", Less),
            ("0.2.0-beta.1", "0.2.0-dev", Less),
            ("0.2.0-dev", "0.2.0", Less),
            ("0.2.0", "0.3.0", Less),
            ("0.2.0-beta.1", "0.2.0-beta.2", Less),
            ("0.2.0-beta.2", "0.2.0-beta.11", Less),
            ("0.2.0-alpha", "0.2.0-alpha.1", Less),
            ("0.2.0-1", "0.2.0-alpha", Less),
            ("1.0.0", "0.9.9", Greater),
            ("0.2.0+abc", "0.2.0", Equal),
        ] {
            assert_eq!(compare_versions(a, b), Some(order), "{a} vs {b}");
            assert_eq!(compare_versions(b, a), Some(order.reverse()), "{b} vs {a}");
        }
        for bad in ["", "1", "1.2", "1.2.3.4", "a.b.c", "1.2.3-"] {
            assert_eq!(compare_versions(bad, "1.0.0"), None, "{bad}");
        }
    }

    #[test]
    fn the_program_is_chosen_by_version() {
        let none = || -> Option<String> { panic!("the previous program is not asked") };
        assert_eq!(
            choose(&saved(Some("0.2.0-dev")), "0.2.0-dev", "0.2.0-beta.1", none),
            None
        );
        assert_eq!(
            choose(
                &saved(Some("0.2.0-beta.1")),
                "0.2.0-dev",
                "0.2.0-beta.1",
                || { Some("0.2.0-beta.1".into()) }
            ),
            Some(Ok("0.2.0-beta.1".to_string()))
        );
    }

    #[test]
    fn every_other_save_is_refused_naming_its_version() {
        let none = || -> Option<String> { None };
        let refused =
            |version: Option<&str>| match choose(&saved(version), "0.3.0", "0.2.0-beta.1", none) {
                Some(Err(refusal)) => refusal,
                other => panic!("not refused: {other:?}"),
            };
        let older = refused(Some("0.1.0"));
        assert_eq!(older.kind, RefusalKind::Older);
        assert_eq!(
            older.reason,
            "this match was saved by Touchline 0.1.0, two or more versions back; this version \
             finishes matches saved by Touchline 0.3.0 and 0.2.0-beta.1"
        );
        assert_eq!(refused(Some("0.4.0")).kind, RefusalKind::Newer);
        assert_eq!(refused(Some("0.2.0-beta.2")).kind, RefusalKind::Other);
        assert_eq!(refused(Some("junk")).kind, RefusalKind::Other);
        let unreleased = refused(None);
        assert_eq!(unreleased.kind, RefusalKind::Unreleased);
        assert!(unreleased.reason.contains("an unreleased build abc1234"));
        let missing = refused(Some("0.2.0-beta.1"));
        assert_eq!(missing.kind, RefusalKind::PreviousMissing);
        assert!(missing.reason.contains("Touchline 0.2.0-beta.1"));
        // A program that prints another version is not the previous engine.
        assert!(matches!(
            choose(
                &saved(Some("0.2.0-beta.1")),
                "0.3.0",
                "0.2.0-beta.1",
                || Some("0.1.0".into())
            ),
            Some(Err(Refusal {
                kind: RefusalKind::PreviousMissing,
                ..
            }))
        ));
    }

    #[test]
    fn the_pin_names_the_previous_release() {
        assert_eq!(previous_version(), "0.2.0-beta.1");
    }
}

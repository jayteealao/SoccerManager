//! The bug report of a background match that failed.
//!
//! A background match should never fail. When a defect makes one fail anyway, the player's
//! match plays on, the fixture shows "result unavailable", and this report keeps what a
//! developer needs to play the match again: its seed and the engine that played it.

use std::path::{Path, PathBuf};

/// The layout version of the bug report.
pub const SCHEMA_VERSION: u32 = 1;

/// Why a background match has no result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// The match panicked while it played.
    Panic,
    /// The match was still running when the wait after the player's full time ran out.
    DidNotFinish,
    /// A club file is missing or changed since the match was saved, or does not make a match.
    TeamFile,
}

impl Failure {
    /// The name the report and the signal give it.
    pub fn code(self) -> &'static str {
        match self {
            Failure::Panic => "panic",
            Failure::DidNotFinish => "did-not-finish",
            Failure::TeamFile => "team-file",
        }
    }
}

/// One failed background match.
#[derive(Debug, Clone)]
pub struct BugReport<'a> {
    pub match_id: &'a str,
    pub fixture: u32,
    pub seed: u64,
    pub club_ids: &'a [String; 2],
    /// The tick the match reached before it stopped.
    pub tick: u32,
    pub failure: Failure,
    pub message: &'a str,
}

impl BugReport<'_> {
    /// The report's path inside the data folder.
    pub fn relative_path(&self) -> String {
        format!(
            "matches/{}/matchday-bug-{}.json",
            self.match_id, self.fixture
        )
    }

    /// Writes the report under `data_dir` so a reader never sees half of it, and returns its
    /// path inside the data folder.
    pub fn write(&self, data_dir: &Path) -> std::io::Result<String> {
        let relative = self.relative_path();
        let path = data_dir.join(&relative);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::json!({
            "schema.version": SCHEMA_VERSION,
            "match.id": self.match_id,
            "fixture": self.fixture,
            "seed": self.seed,
            "engine.version": engine::version(),
            "build.hash": engine::build_hash(),
            "home.team.id": self.club_ids[0],
            "away.team.id": self.club_ids[1],
            "tick": self.tick,
            "error.type": self.failure.code(),
            "error.message": without_paths(self.message),
        });
        let temporary: PathBuf = path.with_extension("json.tmp");
        std::fs::write(&temporary, format!("{json}\n"))?;
        std::fs::rename(&temporary, &path)?;
        Ok(relative)
    }
}

/// `text` with every absolute path cut to its file name, so a user name in a home folder
/// never reaches a report or a log. A path may hold spaces (`C:\Users\Jo Smith\x.json`): a
/// quoted path runs to its closing quote, and an unquoted one takes in each following word
/// that still holds a path separator.
pub fn without_paths(text: &str) -> String {
    const QUOTES: [char; 3] = ['\'', '"', '`'];
    let words: Vec<&str> = text.split(' ').collect();
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    let mut i = 0;
    while i < words.len() {
        let word = words[i];
        let bare = word.trim_matches(|c| matches!(c, '\'' | '"' | '(' | ')' | ',' | '`'));
        if bare.is_empty() || !is_absolute(bare) {
            out.push(word.to_string());
            i += 1;
            continue;
        }
        let quote = word.chars().find(|c| QUOTES.contains(c));
        let mut j = i;
        match quote {
            // A quoted path ends at the word that closes the quote.
            Some(q) if word.matches(q).count() < 2 => {
                while j + 1 < words.len() && !words[j].ends_with(q) && !words[j + 1].is_empty() {
                    j += 1;
                    if words[j].contains(q) {
                        break;
                    }
                }
            }
            Some(_) => {}
            None => {
                while j + 1 < words.len() && words[j + 1].contains(['\\', '/']) {
                    j += 1;
                }
            }
        }
        let span = words[i..=j].join(" ");
        let path = span.trim_matches(|c| matches!(c, '\'' | '"' | '(' | ')' | ',' | '`'));
        let name = path
            .rsplit(['/', '\\'])
            .next()
            .filter(|n| !n.is_empty())
            .unwrap_or("");
        out.push(span.replace(path, name));
        i = j + 1;
    }
    out.join(" ")
}

/// Whether `word` starts an absolute path: a root, a drive, or a share.
fn is_absolute(word: &str) -> bool {
    Path::new(word).is_absolute()
        || word.starts_with('/')
        || word.starts_with("\\\\")
        || word.get(1..3) == Some(":\\")
        || word.get(1..3) == Some(":/")
}

/// The words of a panic payload.
pub fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a panic with no message".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_paths_are_cut_to_their_file_names() {
        assert_eq!(
            without_paths(r"cannot read C:\Users\someone\content\teams\x.json: denied"),
            "cannot read x.json: denied"
        );
        assert_eq!(
            without_paths("panicked at '/home/someone/src/sim.rs' line 3"),
            "panicked at 'sim.rs' line 3"
        );
        assert_eq!(
            without_paths("induced fault at tick 102000"),
            "induced fault at tick 102000"
        );
        // A home folder with a space in its name: no part of the user name is left.
        assert_eq!(
            without_paths(r"cannot read C:\Users\Jo Smith\content\x.json: denied"),
            "cannot read x.json: denied"
        );
        assert_eq!(
            without_paths(r"panicked at 'C:\Users\Jo Smith\src\sim.rs' line 3"),
            "panicked at 'sim.rs' line 3"
        );
        assert_eq!(
            without_paths("panicked at \"/home/jo smith/src/sim.rs\" line 3"),
            "panicked at \"sim.rs\" line 3"
        );
    }

    #[test]
    fn a_report_names_the_seed_and_the_engine() {
        let dir = std::env::temp_dir().join(format!("engine-cli-bug-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let clubs = ["club-a".to_string(), "club-b".to_string()];
        let report = BugReport {
            match_id: "000000000000002a-1",
            fixture: 1,
            seed: 77,
            club_ids: &clubs,
            tick: 102_000,
            failure: Failure::Panic,
            message: "induced fault",
        };
        let relative = report.write(&dir).unwrap();
        assert_eq!(relative, "matches/000000000000002a-1/matchday-bug-1.json");
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join(&relative)).unwrap()).unwrap();
        assert_eq!(json["seed"], 77);
        assert_eq!(json["engine.version"], engine::version());
        assert_eq!(json["build.hash"], engine::build_hash());
        assert_eq!(json["error.type"], "panic");
        assert_eq!(json["tick"], 102_000);
        assert_eq!(json["home.team.id"], "club-a");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

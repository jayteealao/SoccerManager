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
/// never reaches a report or a log.
pub fn without_paths(text: &str) -> String {
    text.split(' ')
        .map(|word| {
            let bare = word.trim_matches(|c| matches!(c, '\'' | '"' | '(' | ')' | ',' | '`'));
            let absolute = Path::new(bare).is_absolute()
                || bare.starts_with('/')
                || bare.get(1..3) == Some(":\\")
                || bare.get(1..3) == Some(":/");
            if absolute && !bare.is_empty() {
                let name = bare
                    .rsplit(['/', '\\'])
                    .next()
                    .filter(|n| !n.is_empty())
                    .unwrap_or("");
                word.replace(bare, name)
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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

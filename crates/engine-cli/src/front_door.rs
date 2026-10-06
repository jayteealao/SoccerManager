//! What the launcher's start screen reads: the sample teams match setup offers, the saved
//! match Resume continues, and the player's three settings.
//!
//! The teams are the club files in the content folder's `teams/`, loaded through the same
//! loader as the matchday round, so the setup list and the round never disagree. A saved match
//! is the newest match folder that holds a snapshot and no `stats.json`, because `stats.json`
//! is written exactly at full time. The settings live in `settings.json` in the data folder:
//! the page's port, and with it the browser's own storage, changes at every launch.

use std::path::{Path, PathBuf};

use engine::data::TeamFile;
use engine::{Content, ContentDir, MatchConfig, Simulation, Snapshot};
use serde::{Deserialize, Serialize};

use crate::matchday::round;

/// The settings file in the data folder.
pub const SETTINGS_FILE: &str = "settings.json";
/// The settings file layout this build reads and writes.
const SETTINGS_SCHEMA: u32 = 1;
/// The playback speeds a player can choose as the default.
const SPEEDS: [u8; 4] = [1, 2, 4, 8];

/// One club match setup offers.
#[derive(Debug, Clone)]
pub struct SampleTeam {
    pub file: TeamFile,
    /// The team file, as the worker's `--team-a` or `--team-b` receives it.
    pub path: PathBuf,
    /// The mean attribute of the file's first eleven players, from 0 to 100.
    pub strength: f64,
}

impl SampleTeam {
    pub fn id(&self) -> &str {
        &self.file.club.id
    }

    pub fn name(&self) -> &str {
        &self.file.club.name
    }

    /// The team as `/engine.json` lists it.
    pub fn json(&self) -> serde_json::Value {
        let club = &self.file.club;
        serde_json::json!({
            "id": club.id,
            "name": club.name,
            "short_name": club.short_name,
            "kit": [club.kit.primary, club.kit.secondary],
            "ground": [club.ground.length, club.ground.width],
            "strength": (self.strength * 10.0).round() / 10.0,
        })
    }
}

/// Every club file in `teams/`, ordered by club id. Two files that share a club id or a club
/// name are refused, because match setup and Resume find a team by them.
pub fn sample_teams(dir: &ContentDir, content: &Content) -> anyhow::Result<Vec<SampleTeam>> {
    let mut teams: Vec<SampleTeam> = Vec::new();
    for club in round::club_files(dir, content) {
        let (id, name) = (&club.file.club.id, &club.file.club.name);
        if let Some(twin) = teams.iter().find(|t| t.id() == id || t.name() == name) {
            let shared = if twin.id() == id {
                format!("the club id {id}")
            } else {
                format!("the club name {name}")
            };
            anyhow::bail!(
                "two team files share {shared}: {} and {}",
                dir.relative(&twin.path),
                dir.relative(&club.path)
            );
        }
        teams.push(SampleTeam {
            strength: strength(&club.file),
            path: club.path,
            file: club.file,
        });
    }
    teams.sort_by(|a, b| a.id().cmp(b.id()));
    Ok(teams)
}

/// The mean rating of the first eleven players in the file, on the 1 to 20 scale.
fn strength(file: &TeamFile) -> f64 {
    let (sum, count) = file
        .players
        .iter()
        .take(11)
        .flat_map(|p| p.attributes.values())
        .fold((0u64, 0u64), |(s, n), v| (s + u64::from(v.tenths()), n + 1));
    if count == 0 {
        0.0
    } else {
        sum as f64 / count as f64 / 10.0
    }
}

/// The unfinished match Resume continues: the newest snapshot under `matches/` whose folder
/// holds no `stats.json`.
#[derive(Debug, Clone)]
pub struct SavedMatch {
    pub path: PathBuf,
    pub identity: engine::SnapshotIdentity,
    /// The players' places at the saved tick, for a save of this release.
    pub positions: Option<Positions>,
}

/// The players' places on the saved pitch, in metres from the top-left corner.
#[derive(Debug, Clone)]
pub struct Positions {
    pub pitch: [f64; 2],
    /// `(team, x, y)`, home team 0.
    pub players: Vec<(usize, f64, f64)>,
}

impl SavedMatch {
    /// The newest unfinished save under `data`, or `None`.
    pub fn newest(data: &Path) -> Option<PathBuf> {
        let entries = std::fs::read_dir(data.join("matches")).ok()?;
        entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|dir| !dir.join("stats.json").exists())
            .map(|dir| dir.join(engine::snapshot::FILE_NAME))
            .filter_map(|file| {
                let modified = std::fs::metadata(&file).ok()?.modified().ok()?;
                Some((modified, file))
            })
            .max_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
            .map(|(_, file)| file)
    }

    /// Reads the save at `path`. A file that does not identify is no save.
    pub fn read(path: &Path, teams: &[SampleTeam], content: &Content) -> Option<SavedMatch> {
        let bytes = std::fs::read(path).ok()?;
        let identity = Snapshot::identify(&bytes).ok()?;
        let positions = if identity.engine_version.as_deref() == Some(engine::version()) {
            positions(path, &identity, teams, content)
        } else {
            None
        };
        Some(SavedMatch {
            path: path.to_path_buf(),
            identity,
            positions,
        })
    }

    /// `current` for a save of this release, `previous` for the previous release's, `other`
    /// for any version this release cannot finish.
    pub fn kind(&self) -> &'static str {
        match self.identity.engine_version.as_deref() {
            Some(v) if v == engine::version() => "current",
            Some(v) if v == crate::engines::previous_version() => "previous",
            _ => "other",
        }
    }

    /// The two team files the saved club names name, home first; `None` when either club is
    /// not among the sample teams.
    pub fn team_files(&self, teams: &[SampleTeam]) -> Option<[PathBuf; 2]> {
        let names = self.identity.teams.as_ref()?;
        // The snapshot header keeps a name to its first 44 bytes, so a club is compared by
        // its name cut the same way.
        let find = |name: &str| {
            teams
                .iter()
                .find(|t| as_saved(t.name()) == name)
                .map(|t| t.path.clone())
        };
        Some([find(&names[0])?, find(&names[1])?])
    }

    /// The save as `/engine.json` lists it.
    pub fn json(&self) -> serde_json::Value {
        let id = &self.identity;
        serde_json::json!({
            "kind": self.kind(),
            "version": id.engine_version,
            "tick": id.tick,
            "teams": id.teams,
            "score": id.score,
            "millis": id.match_millis,
            "positions": self.positions.as_ref().map(|p| serde_json::json!({
                "pitch": p.pitch,
                "players": p.players.iter().map(|(team, x, y)| serde_json::json!([
                    team,
                    (x * 10.0).round() / 10.0,
                    (y * 10.0).round() / 10.0,
                ])).collect::<Vec<_>>(),
            })),
        })
    }
}

/// The players' places in a save of this release, rebuilt the way a resume rebuilds the
/// match. Any failure (a team file changed, a scripted match) leaves the pitch empty.
fn positions(
    path: &Path,
    identity: &engine::SnapshotIdentity,
    teams: &[SampleTeam],
    content: &Content,
) -> Option<Positions> {
    let names = identity.teams.as_ref()?;
    let find = |name: &str| teams.iter().find(|t| t.name() == name);
    let (home, away) = (find(&names[0])?, find(&names[1])?);
    let snapshot = Snapshot::read(path, &path.display().to_string()).ok()?;
    let mut config = MatchConfig::new(
        snapshot.seed(),
        snapshot.minutes(),
        content,
        [&home.file, &away.file],
    )
    .ok()?;
    if snapshot.knockout() {
        config = config.with_knockout();
    }
    let sim = Simulation::from_snapshot(config, &snapshot).ok()?;
    let ground = &home.file.club.ground;
    Some(Positions {
        pitch: [ground.length, ground.width],
        players: sim
            .players()
            .iter()
            // The simulation measures from the centre spot; the page draws from the corner.
            .map(|p| {
                (
                    p.team,
                    p.pos.x + ground.length / 2.0,
                    p.pos.y + ground.width / 2.0,
                )
            })
            .collect(),
    })
}

/// How the viewer moves: as the operating system asks, always reduced, or always full.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Motion {
    Follow,
    Reduce,
    Full,
}

/// The player's three settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[serde(default = "schema")]
    pub schema_version: u32,
    /// The playback speed a match starts at: 1, 2, 4 or 8.
    pub speed: u8,
    pub motion: Motion,
    pub commentary: bool,
}

fn schema() -> u32 {
    SETTINGS_SCHEMA
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA,
            speed: 1,
            motion: Motion::Follow,
            commentary: true,
        }
    }
}

impl Settings {
    /// Reads `body` and checks every value, naming the first that is wrong.
    pub fn parse(body: &str) -> Result<Settings, String> {
        let settings: Settings =
            serde_json::from_str(body).map_err(|e| format!("the settings do not read: {e}"))?;
        if settings.schema_version != SETTINGS_SCHEMA {
            return Err(format!(
                "settings schema_version {} is not {SETTINGS_SCHEMA}",
                settings.schema_version
            ));
        }
        if !SPEEDS.contains(&settings.speed) {
            return Err(format!(
                "speed {} is not one of 1, 2, 4 or 8",
                settings.speed
            ));
        }
        Ok(settings)
    }

    /// The settings in `data`. A missing file gives the defaults; a file that does not read
    /// gives the defaults and says so in a signal.
    pub fn load(data: &Path) -> Settings {
        let path = data.join(SETTINGS_FILE);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Settings::default();
        };
        Settings::parse(&text).unwrap_or_else(|reason| {
            tracing::warn!(signal = "launch.settings_refused", reason = %reason);
            Settings::default()
        })
    }

    /// Writes the settings to `data` through a temporary file, so a crash never leaves half
    /// a file.
    pub fn save(&self, data: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(data)?;
        let path = data.join(SETTINGS_FILE);
        let temporary = data.join(format!("{SETTINGS_FILE}.tmp"));
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&temporary, text + "\n")?;
        std::fs::rename(&temporary, &path)
    }

    pub fn json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("settings always serialize")
    }
}

/// `name` as a snapshot header keeps it: cut to the header's width at a character boundary.
fn as_saved(name: &str) -> &str {
    let mut end = name.len().min(engine::snapshot::TEAM_BYTES);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    &name[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A long club name is matched as the header keeps it: its first 44 bytes, never cut
    /// inside a character.
    #[test]
    fn a_long_club_name_is_matched_as_the_header_keeps_it() {
        let long = "Sporting Club of the Very Long Name Hills Athletic";
        assert_eq!(as_saved(long).len(), engine::snapshot::TEAM_BYTES);
        assert!(long.starts_with(as_saved(long)));
        assert_eq!(as_saved("Rovers"), "Rovers");
        let accented = "é".repeat(30);
        assert_eq!(as_saved(&accented).len(), 44);
    }

    fn shipped() -> ContentDir {
        ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "engine-cli-front-door-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn match_setup_offers_every_shipped_club_with_its_file_and_strength() {
        let dir = shipped();
        let content = Content::load(&dir).unwrap();
        let teams = sample_teams(&dir, &content).unwrap();
        assert_eq!(teams.len(), 10);
        assert!(teams.iter().any(|t| t.name() == "Oakmere Rangers"));
        for team in &teams {
            assert!(team.path.is_file(), "{}", team.path.display());
            assert!((1.0..100.0).contains(&team.strength), "{}", team.strength);
        }
        let ids: Vec<&str> = teams.iter().map(SampleTeam::id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "the clubs are in club-id order");
    }

    #[test]
    fn the_settings_default_save_and_read_back() {
        let data = temp("settings");
        assert_eq!(Settings::load(&data), Settings::default());
        let chosen = Settings {
            speed: 4,
            motion: Motion::Reduce,
            commentary: false,
            ..Settings::default()
        };
        chosen.save(&data).unwrap();
        assert_eq!(Settings::load(&data), chosen);
        std::fs::write(data.join(SETTINGS_FILE), "{ not json").unwrap();
        assert_eq!(Settings::load(&data), Settings::default());
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn a_bad_setting_is_refused_by_name() {
        assert!(
            Settings::parse(r#"{"speed":3,"motion":"full","commentary":true}"#)
                .unwrap_err()
                .contains("speed 3")
        );
        assert!(Settings::parse(r#"{"speed":2,"motion":"slow","commentary":true}"#).is_err());
        assert!(
            Settings::parse(r#"{"speed":2,"motion":"full","commentary":true,"skin":"x"}"#).is_err()
        );
        assert_eq!(
            Settings::parse(r#"{"speed":8,"motion":"full","commentary":true}"#)
                .unwrap()
                .speed,
            8
        );
    }

    #[test]
    fn a_finished_match_is_no_save() {
        let data = temp("saves");
        assert!(SavedMatch::newest(&data).is_none());
        let done = data.join("matches").join("a");
        std::fs::create_dir_all(&done).unwrap();
        std::fs::write(done.join(engine::snapshot::FILE_NAME), b"x").unwrap();
        std::fs::write(done.join("stats.json"), b"{}").unwrap();
        assert!(SavedMatch::newest(&data).is_none());
        let open = data.join("matches").join("b");
        std::fs::create_dir_all(&open).unwrap();
        std::fs::write(open.join(engine::snapshot::FILE_NAME), b"x").unwrap();
        assert_eq!(
            SavedMatch::newest(&data),
            Some(open.join(engine::snapshot::FILE_NAME))
        );
        let _ = std::fs::remove_dir_all(&data);
    }
}

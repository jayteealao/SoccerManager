//! What the launcher's start screen reads: the sample teams match setup offers, the saved
//! match Resume continues, and the player's three settings.
//!
//! The teams are the club files in the content folder's `teams/`, loaded through the same
//! loader as the matchday round, so the setup list and the round never disagree. A saved match
//! is the newest match folder that holds a snapshot and no `stats.json`, because `stats.json`
//! is written exactly at full time. The settings live in `settings.json` in the data folder:
//! the page's port, and with it the browser's own storage, changes at every launch.

use std::collections::BTreeMap;
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
    /// The mean visible rating of the file's first eleven players, on the 1 to 20 scale.
    pub strength: f64,
    /// The version the file was written in, when it was older and converted on load: setup
    /// names the club in a notice.
    pub converted_from: Option<u32>,
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
            "converted": self.converted_from.is_some(),
        })
    }
}

/// Every club file in `teams/`, ordered by club id. Two files that share a club id or a club
/// name are refused, because match setup and Resume find a team by them.
pub fn sample_teams(dir: &ContentDir, content: &Content) -> anyhow::Result<Vec<SampleTeam>> {
    let mut teams: Vec<SampleTeam> = Vec::new();
    let hidden: Vec<&str> = content.attributes.hidden_names().collect();
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
            strength: strength(&club.file, &hidden),
            converted_from: club.converted_from,
            path: club.path,
            file: club.file,
        });
    }
    teams.sort_by(|a, b| a.id().cmp(b.id()));
    Ok(teams)
}

/// The mean visible rating of the first eleven players in the file, on the 1 to 20 scale. A
/// hidden value never moves a figure the page shows, so consistency and injury proneness are
/// left out.
fn strength(file: &TeamFile, hidden: &[&str]) -> f64 {
    let (sum, count) = file
        .players
        .iter()
        .take(11)
        .flat_map(|p| p.attributes.iter())
        .filter(|(name, _)| !hidden.contains(&name.as_str()))
        .map(|(_, v)| v)
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

/// The squad screen's store in the data folder: named views and match ratings, per club.
pub const VIEWS_FILE: &str = "views.json";
/// The views file layout this build reads and writes.
const VIEWS_SCHEMA: u32 = 1;
/// The most named views a club keeps.
pub const MAX_VIEWS: usize = 20;
/// The most columns one view lists.
pub const MAX_COLUMNS: usize = 64;
/// The longest view name and column id, in characters.
const MAX_NAME_CHARS: usize = 40;
/// The longest club id the page may save views for, in characters.
const MAX_CLUB_CHARS: usize = 64;
/// The most clubs the store keeps views for.
pub const MAX_CLUBS: usize = 500;
/// The match ratings kept per player: the newest ten.
pub const KEPT_RATINGS: usize = 10;

/// Which way a view sorts its column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
}

/// The column a view sorts by, and which way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sort {
    pub column: String,
    pub direction: Direction,
}

/// One named view: the columns shown, in order, and the sort. The column ids are the page's;
/// the store keeps them as text, and the page drops an id it does not know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub name: String,
    pub columns: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<Sort>,
}

/// One match rating of a player: the match it was earned in and the rating, 1.0 to 10.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rated {
    #[serde(rename = "match")]
    pub match_id: String,
    pub rating: f64,
}

/// One club's part of the store: its views, the active one, and its players' newest match
/// ratings, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClubViews {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
    #[serde(default)]
    pub views: Vec<View>,
    #[serde(default)]
    pub ratings: BTreeMap<String, Vec<Rated>>,
}

/// The squad screen's store, `views.json` in the data folder. A stand-in for the saved game,
/// which does not exist yet: when careers exist, it moves into the saved game. Local only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Views {
    pub schema_version: u32,
    #[serde(default)]
    pub clubs: BTreeMap<String, ClubViews>,
}

impl Default for Views {
    fn default() -> Self {
        Self {
            schema_version: VIEWS_SCHEMA,
            clubs: BTreeMap::new(),
        }
    }
}

/// What the page sends to save one club's views.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveViews {
    club: String,
    #[serde(default)]
    active: Option<String>,
    views: Vec<View>,
}

/// The first thing wrong with one club's `views` and `active` view, named.
fn check_views(club: &str, active: Option<&String>, views: &[View]) -> Result<(), String> {
    if views.len() > MAX_VIEWS {
        return Err(format!(
            "club {club}: {} views; at most {MAX_VIEWS}",
            views.len()
        ));
    }
    for (i, view) in views.iter().enumerate() {
        if view.name.trim().is_empty() {
            return Err(format!("club {club}: view {} has no name", i + 1));
        }
        if view.name.chars().count() > MAX_NAME_CHARS {
            return Err(format!(
                "club {club}: the name of view {} is longer than {MAX_NAME_CHARS} characters",
                i + 1
            ));
        }
        if view.columns.is_empty() {
            return Err(format!("club {club}: view {:?} has no columns", view.name));
        }
        if view.columns.len() > MAX_COLUMNS {
            return Err(format!(
                "club {club}: view {:?} has {} columns; at most {MAX_COLUMNS}",
                view.name,
                view.columns.len()
            ));
        }
        if let Some(bad) = view
            .columns
            .iter()
            .find(|c| c.is_empty() || c.chars().count() > MAX_NAME_CHARS)
        {
            return Err(format!(
                "club {club}: view {:?} has the column id {bad:?}",
                view.name
            ));
        }
    }
    if let Some(active) = active
        && !views.iter().any(|v| &v.name == active)
    {
        return Err(format!(
            "club {club}: the active view {active:?} is not one of its views"
        ));
    }
    Ok(())
}

impl Views {
    /// Reads `body` and checks every value, naming the first that is wrong.
    pub fn parse(body: &str) -> Result<Views, String> {
        let views: Views =
            serde_json::from_str(body).map_err(|e| format!("the views do not read: {e}"))?;
        if views.schema_version != VIEWS_SCHEMA {
            return Err(format!(
                "views schema_version {} is not {VIEWS_SCHEMA}",
                views.schema_version
            ));
        }
        for (club, part) in &views.clubs {
            check_views(club, part.active.as_ref(), &part.views)?;
            for (player, list) in &part.ratings {
                if list.len() > KEPT_RATINGS {
                    return Err(format!(
                        "club {club}: player {player} has {} ratings; at most {KEPT_RATINGS}",
                        list.len()
                    ));
                }
                if let Some(r) = list.iter().find(|r| !(1.0..=10.0).contains(&r.rating)) {
                    return Err(format!(
                        "club {club}: player {player} has the rating {} in match {}; allowed 1.0 to 10.0",
                        r.rating, r.match_id
                    ));
                }
            }
        }
        Ok(views)
    }

    /// The store in `data`. A missing file gives an empty store; a file that does not read
    /// gives an empty store and says so in a signal. Never fails the launch.
    pub fn load(data: &Path) -> Views {
        let path = data.join(VIEWS_FILE);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Views::default();
        };
        Views::parse(&text).unwrap_or_else(|reason| {
            tracing::warn!(signal = "launch.views_refused", reason = %reason);
            Views::default()
        })
    }

    /// Writes the store to `data` through a temporary file, so a crash never leaves half a
    /// file.
    pub fn save(&self, data: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(data)?;
        let path = data.join(VIEWS_FILE);
        let temporary = data.join(format!("{VIEWS_FILE}.tmp"));
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&temporary, text + "\n")?;
        std::fs::rename(&temporary, &path)
    }

    /// Replaces one club's views and active view from the page's `body`
    /// (`{"club", "active", "views"}`), keeping its ratings. Returns the club id.
    pub fn set_views(&mut self, body: &str) -> Result<String, String> {
        let save: SaveViews =
            serde_json::from_str(body).map_err(|e| format!("the views do not read: {e}"))?;
        if save.club.trim().is_empty() {
            return Err("the views name no club".into());
        }
        if save.club.chars().count() > MAX_CLUB_CHARS {
            return Err(format!(
                "the club id is longer than {MAX_CLUB_CHARS} characters"
            ));
        }
        if !self.clubs.contains_key(&save.club) && self.clubs.len() >= MAX_CLUBS {
            return Err(format!("the views already hold {MAX_CLUBS} clubs"));
        }
        check_views(&save.club, save.active.as_ref(), &save.views)?;
        let part = self.clubs.entry(save.club.clone()).or_default();
        part.views = save.views;
        part.active = save.active;
        Ok(save.club)
    }

    /// Adds the home side's match ratings of one finished match to the home club's ratings:
    /// a match it already holds is skipped, and each player keeps his newest ten. Returns how
    /// many ratings were added.
    pub fn ingest(&mut self, stats: &engine::observe::MatchStats) -> usize {
        let club = &stats.teams[0].id;
        let part = self.clubs.entry(club.clone()).or_default();
        let mut added = 0;
        for entry in stats.ratings.iter().filter(|r| r.team == 0) {
            let list = part.ratings.entry(entry.id.clone()).or_default();
            if list.iter().any(|r| r.match_id == stats.match_id) {
                continue;
            }
            list.push(Rated {
                match_id: stats.match_id.clone(),
                rating: entry.rating.clamp(1.0, 10.0),
            });
            if list.len() > KEPT_RATINGS {
                list.drain(..list.len() - KEPT_RATINGS);
            }
            added += 1;
        }
        added
    }

    /// One club's part as the page reads it, `{"active", "views", "ratings"}`; empty when the
    /// store holds nothing for the club.
    pub fn club_json(&self, club: &str) -> serde_json::Value {
        let part = self.clubs.get(club).cloned().unwrap_or_default();
        serde_json::to_value(part).expect("views always serialize")
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
pub(crate) mod tests {
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

    /// The shipped team files are version 1: each lists as converted, so setup names the
    /// club in its notice; a file of this version lists as not converted.
    #[test]
    fn a_converted_team_file_lists_as_converted_and_a_current_one_does_not() {
        let dir = shipped();
        let content = Content::load(&dir).unwrap();
        let teams = sample_teams(&dir, &content).unwrap();
        for team in &teams {
            assert_eq!(team.converted_from, Some(1), "{}", team.name());
            assert_eq!(team.json()["converted"], true);
        }
        let current = SampleTeam {
            converted_from: None,
            ..teams[0].clone()
        };
        assert_eq!(current.json()["converted"], false);
    }

    fn view(name: &str, columns: &[&str]) -> View {
        View {
            name: name.into(),
            columns: columns.iter().map(|c| (*c).to_string()).collect(),
            sort: Some(Sort {
                column: columns[0].into(),
                direction: Direction::Down,
            }),
        }
    }

    pub(crate) fn stats(
        match_id: &str,
        ratings: &[(&str, usize, f64)],
    ) -> engine::observe::MatchStats {
        use engine::observe::{
            LawStats, MatchFigures, MatchStats, RatingEntry, ScriptFigures, TacticsStats, TeamRef,
        };
        let team = |id: &str| TeamRef {
            id: id.into(),
            name: id.into(),
        };
        MatchStats {
            owner_id: "0123456789abcdef0123456789abcdef".into(),
            match_id: match_id.into(),
            seed: 42,
            content_hash: "abcdef012345".into(),
            teams: [team("club-a"), team("club-b")],
            duration_ms: 1,
            outcome: "success".into(),
            ticks_per_s: 1.0,
            ticks_written: 1,
            validate_ran: false,
            validate_violations: 0,
            possession_changes: 0,
            ball_max_speed: 0.0,
            ball_idle_ticks: 0,
            goals: [0, 0],
            flags_on: Vec::new(),
            laws: LawStats::default(),
            tactics: TacticsStats::default(),
            figures: MatchFigures::default(),
            script: ScriptFigures::default(),
            ratings: ratings
                .iter()
                .map(|(id, team, rating)| RatingEntry {
                    id: (*id).into(),
                    team: *team,
                    rating: *rating,
                })
                .collect(),
        }
    }

    #[test]
    fn the_views_save_and_read_back_and_a_damaged_file_starts_empty() {
        let data = temp("views");
        assert_eq!(Views::load(&data), Views::default());
        let mut views = Views::default();
        let body = serde_json::json!({
            "club": "club-a",
            "active": "Before Kelder",
            "views": [view("Before Kelder", &["age", "height", "attr:pace"])],
        })
        .to_string();
        assert_eq!(views.set_views(&body).unwrap(), "club-a");
        views.ingest(&stats("m-1", &[("a-1", 0, 7.4)]));
        views.save(&data).unwrap();
        assert!(
            !data.join(format!("{VIEWS_FILE}.tmp")).exists(),
            "the temporary file is renamed"
        );
        let back = Views::load(&data);
        assert_eq!(back, views);
        assert_eq!(back.club_json("club-a")["active"], "Before Kelder");
        assert_eq!(
            back.club_json("club-a")["views"][0]["columns"][2],
            "attr:pace"
        );
        assert_eq!(back.club_json("club-a")["ratings"]["a-1"][0]["rating"], 7.4);
        assert_eq!(
            back.club_json("club-nowhere"),
            serde_json::json!({"views": [], "ratings": {}})
        );
        // Saving views again keeps the ratings.
        let mut again = back.clone();
        again
            .set_views(
                &serde_json::json!({"club": "club-a", "views": [view("Wide", &["age"])]})
                    .to_string(),
            )
            .unwrap();
        assert_eq!(again.clubs["club-a"].ratings, back.clubs["club-a"].ratings);
        assert_eq!(again.clubs["club-a"].active, None);
        std::fs::write(data.join(VIEWS_FILE), "{ not json").unwrap();
        assert_eq!(Views::load(&data), Views::default());
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn a_bad_view_store_is_refused_by_its_first_wrong_value() {
        let refused = |body: serde_json::Value| Views::parse(&body.to_string()).unwrap_err();
        assert!(refused(serde_json::json!({"schema_version": 2})).contains("schema_version 2"));
        assert!(
            refused(serde_json::json!({"schema_version": 1, "clubs": {}, "theme": 1}))
                .contains("theme")
        );
        let club = |part: serde_json::Value| serde_json::json!({"schema_version": 1, "clubs": {"club-a": part}});
        assert!(
            refused(club(
                serde_json::json!({"views": [{"name": " ", "columns": ["age"]}]})
            ))
            .contains("view 1 has no name")
        );
        assert!(
            refused(club(
                serde_json::json!({"views": [{"name": "A", "columns": []}]})
            ))
            .contains("\"A\" has no columns")
        );
        let many: Vec<serde_json::Value> = (0..=MAX_VIEWS)
            .map(|i| serde_json::json!({"name": format!("V{i}"), "columns": ["age"]}))
            .collect();
        assert!(refused(club(serde_json::json!({"views": many}))).contains("21 views"));
        assert!(
            refused(club(serde_json::json!({
                "active": "B",
                "views": [{"name": "A", "columns": ["age"]}]
            })))
            .contains("active view \"B\"")
        );
        assert!(
            refused(club(serde_json::json!({
                "ratings": {"a-1": [{"match": "m-1", "rating": 11.0}]}
            })))
            .contains("player a-1 has the rating 11")
        );
        let kept = Views::parse(
            &club(serde_json::json!({
                "active": "A",
                "views": [{"name": "A", "columns": ["age", "a-column-a-later-build-retired"]}],
                "ratings": {"a-1": [{"match": "m-1", "rating": 6.5}]}
            }))
            .to_string(),
        )
        .unwrap();
        assert_eq!(
            kept.clubs["club-a"].views[0].columns.len(),
            2,
            "unknown ids are the page's to drop"
        );
        let mut store = Views::default();
        assert!(
            store
                .set_views(r#"{"club":"","views":[]}"#)
                .unwrap_err()
                .contains("no club")
        );
        assert!(
            store
                .set_views(r#"{"club":"club-a","views":[],"x":1}"#)
                .is_err()
        );
        let long = format!(r#"{{"club":"{}","views":[]}}"#, "c".repeat(65));
        assert!(
            store
                .set_views(&long)
                .unwrap_err()
                .contains("longer than 64")
        );
        for i in 0..MAX_CLUBS {
            store
                .set_views(&format!(r#"{{"club":"club-{i}","views":[]}}"#))
                .unwrap();
        }
        assert!(
            store
                .set_views(r#"{"club":"one-more","views":[]}"#)
                .unwrap_err()
                .contains("500 clubs")
        );
        assert!(
            store.set_views(r#"{"club":"club-0","views":[]}"#).is_ok(),
            "a club the store holds may still save"
        );
    }

    #[test]
    fn the_ingest_keeps_ten_home_ratings_and_skips_a_match_it_holds() {
        let mut views = Views::default();
        let first = stats("m-0", &[("a-1", 0, 6.8), ("b-1", 1, 8.0)]);
        assert_eq!(views.ingest(&first), 1, "the away side is not the club's");
        assert_eq!(views.ingest(&first), 0, "a match already held is skipped");
        for m in 1..=11 {
            views.ingest(&stats(
                &format!("m-{m}"),
                &[("a-1", 0, 5.0 + f64::from(m) / 10.0)],
            ));
        }
        let kept = &views.clubs["club-a"].ratings["a-1"];
        assert_eq!(kept.len(), KEPT_RATINGS);
        assert_eq!(kept[0].match_id, "m-2", "the oldest go first");
        assert_eq!(kept.last().unwrap().match_id, "m-11");
        assert!(!views.clubs.contains_key("club-b"));
    }
}

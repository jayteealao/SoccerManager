//! The season record: each player's season across the matches the launcher plays, his one
//! season goal, and the two figures his kick-off mood reads from it.
//!
//! `season.json` in the data folder keeps, per club, the matches it played and per player his
//! starts, minutes, goals, assists, clean sheets, red cards and every match rating, with the
//! goal he was given when he first entered the record. Both sides of every finished launcher
//! match are added, once per match id. A missing file starts empty; a damaged one loads as
//! empty with a signal and never fails the launch.
//!
//! Before a club's next match the launcher writes a copy of its team file into the new match
//! folder with each recorded player's recent form (the mean of his last five ratings) and his
//! progress toward his goal in the player's `condition` block, and the worker plays from the
//! copy, so a replay keeps the inputs. The engine turns the two into his kick-off mood,
//! weighted by his ambition. A stand-in for the season layer, which does not exist yet: when
//! it does, it takes this file over.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use engine::data::attributes::AttributeSchema;
use engine::data::team::{CONVERTED_FROM, PlayerEntry, Position, TEAM_VERSION, TeamFile};
use engine::observe::MatchStats;
use serde::{Deserialize, Serialize};

use crate::front_door::Rated;

/// The season record in the data folder.
pub const SEASON_FILE: &str = "season.json";
/// The season file layout this build reads and writes.
const SEASON_SCHEMA: u32 = 1;
/// The matches a season goal's target is set for.
pub const SEASON_MATCHES: u32 = 38;
/// The newest ratings his recent form is the mean of.
pub const FORM_MATCHES: usize = 5;
/// The most clubs the record keeps.
pub const MAX_CLUBS: usize = 500;
/// The most matches the record keeps per club.
pub const MAX_MATCHES: usize = 200;
/// The most players the record keeps per club.
pub const MAX_PLAYERS: usize = 1000;
/// The largest season file the launcher reads, in bytes.
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
/// The level, in tenths, from which a player's goal is the high band's.
const HIGH_FROM: u8 = 140;
/// The level, in tenths, from which a player's goal is the middle band's.
const MIDDLE_FROM: u8 = 100;
/// The team file names of the copies the launcher writes into a match folder, home first.
pub const COPY_FILES: [&str; 2] = ["team-a.json", "team-b.json"];

/// What a season goal counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalKind {
    /// Score N goals.
    Goals,
    /// Make N assists.
    Assists,
    /// Keep N clean sheets.
    CleanSheets,
    /// Start N matches.
    Starts,
    /// Average a match rating of at least X.
    AverageRating,
    /// Miss no match through his own fault: no red card.
    NoOwnFaultAbsence,
}

/// One player's season goal: its kind and its target for a season of [`SEASON_MATCHES`]
/// (a count, or a rating to a tenth; 0 for no own-fault absence).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Goal {
    pub kind: GoalKind,
    pub target: f64,
}

/// One player's season so far.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerSeason {
    #[serde(default)]
    pub starts: u32,
    #[serde(default)]
    pub minutes: u32,
    #[serde(default)]
    pub goals: u32,
    #[serde(default)]
    pub assists: u32,
    #[serde(default)]
    pub clean_sheets: u32,
    #[serde(default)]
    pub red_cards: u32,
    /// Every match rating, oldest first.
    #[serde(default)]
    pub ratings: Vec<Rated>,
    pub goal: Goal,
}

impl PlayerSeason {
    /// A season with nothing played yet, aiming at `goal`.
    pub fn new(goal: Goal) -> Self {
        Self {
            starts: 0,
            minutes: 0,
            goals: 0,
            assists: 0,
            clean_sheets: 0,
            red_cards: 0,
            ratings: Vec::new(),
            goal,
        }
    }
}

/// One club's part of the record: the matches it played, oldest first, and its players.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClubSeason {
    #[serde(default)]
    pub matches: Vec<String>,
    #[serde(default)]
    pub players: BTreeMap<String, PlayerSeason>,
}

/// The season record, `season.json` in the data folder. Local only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Season {
    pub schema_version: u32,
    #[serde(default)]
    pub clubs: BTreeMap<String, ClubSeason>,
}

impl Default for Season {
    fn default() -> Self {
        Self {
            schema_version: SEASON_SCHEMA,
            clubs: BTreeMap::new(),
        }
    }
}

/// The level band of a level in tenths: 0 high, 1 middle, 2 low.
fn band(level: u8) -> usize {
    if level >= HIGH_FROM {
        0
    } else if level >= MIDDLE_FROM {
        1
    } else {
        2
    }
}

/// The goal kind for a player listed at `position` in level band `band` (0 high, 1 middle,
/// 2 low): a keeper aims for clean sheets unless his level is low, and every low-level player
/// aims to play.
pub fn goal_kind(position: Position, band: usize) -> GoalKind {
    use GoalKind::{Assists, AverageRating, CleanSheets, Goals, NoOwnFaultAbsence, Starts};
    let [high, middle, low] = match position {
        Position::GK => [CleanSheets, CleanSheets, Starts],
        Position::CB => [CleanSheets, NoOwnFaultAbsence, Starts],
        Position::LB | Position::RB => [CleanSheets, Assists, Starts],
        Position::DM => [AverageRating, NoOwnFaultAbsence, Starts],
        Position::CM => [AverageRating, Assists, Starts],
        Position::AM => [Assists, Assists, Starts],
        Position::LW | Position::RW => [Goals, Assists, Starts],
        Position::ST => [Goals, Goals, Starts],
    };
    [high, middle, low][band.min(2)]
}

/// The target of `kind` for a player listed at `position` at level `level` (tenths): the
/// value at level 10 and at level 20, joined by a straight line and kept to that range,
/// rounded to a whole number (an average rating to a tenth).
pub fn goal_target(kind: GoalKind, position: Position, level: u8) -> f64 {
    let (at_10, at_20) = match kind {
        GoalKind::Goals if position == Position::ST => (8.0, 24.0),
        GoalKind::Goals => (5.0, 14.0),
        GoalKind::Assists if position == Position::AM => (5.0, 14.0),
        GoalKind::Assists => (3.0, 9.0),
        GoalKind::CleanSheets => (8.0, 18.0),
        GoalKind::Starts => (15.0, 34.0),
        GoalKind::AverageRating => (6.4, 7.4),
        GoalKind::NoOwnFaultAbsence => (0.0, 0.0),
    };
    let t = (f64::from(level).clamp(100.0, 200.0) - 100.0) / 100.0;
    let v = at_10 + t * (at_20 - at_10);
    if kind == GoalKind::AverageRating {
        (v * 10.0).round() / 10.0
    } else {
        v.round()
    }
}

/// The one season goal of the team-file player `entry`: its kind by his listed position and
/// his level there in three bands (high from 14.0, middle from 10.0), its target by his level.
pub fn assign_goal(entry: &PlayerEntry, schema: &AttributeSchema) -> Goal {
    let ratings = engine::player::Attributes::from_entry(&entry.attributes, schema);
    let level = engine::contract::level::overall_level(&ratings, schema, entry.position);
    let kind = goal_kind(entry.position, band(level));
    Goal {
        kind,
        target: goal_target(kind, entry.position, level),
    }
}

/// His recent form: the mean of his newest [`FORM_MATCHES`] ratings, in tenths (10 to 100);
/// `None` before his first rating.
pub fn recent_rating(p: &PlayerSeason) -> Option<u8> {
    let newest = &p.ratings[p.ratings.len().saturating_sub(FORM_MATCHES)..];
    if newest.is_empty() {
        return None;
    }
    let mean = newest.iter().map(|r| r.rating).sum::<f64>() / newest.len() as f64;
    Some((mean * 10.0).round().clamp(10.0, 100.0) as u8)
}

/// His progress toward his goal after his club's `club_matches` matches, in percent of the
/// pace (100 on track), 0 to 200: a count against `target · club_matches / 38`; an average
/// rating against its target once he has a rating; no own-fault absence 100 without a red
/// card and 0 with one. `None` before his club's first match, and for an average rating
/// before his first rating.
pub fn goal_progress(p: &PlayerSeason, club_matches: usize) -> Option<u8> {
    if club_matches == 0 {
        return None;
    }
    let count = match p.goal.kind {
        GoalKind::Goals => p.goals,
        GoalKind::Assists => p.assists,
        GoalKind::CleanSheets => p.clean_sheets,
        GoalKind::Starts => p.starts,
        GoalKind::AverageRating => {
            if p.ratings.is_empty() {
                return None;
            }
            let mean = p.ratings.iter().map(|r| r.rating).sum::<f64>() / p.ratings.len() as f64;
            return Some(percent(100.0 * mean / p.goal.target.max(1.0)));
        }
        GoalKind::NoOwnFaultAbsence => return Some(if p.red_cards == 0 { 100 } else { 0 }),
    };
    let pace = p.goal.target * club_matches as f64 / f64::from(SEASON_MATCHES);
    Some(percent(100.0 * f64::from(count) / pace.max(1e-9)))
}

/// `v` rounded and kept to 0 to 200.
fn percent(v: f64) -> u8 {
    v.round().clamp(0.0, 200.0) as u8
}

impl Season {
    /// Reads `body` and checks every value, naming the first that is wrong.
    pub fn parse(body: &str) -> Result<Season, String> {
        let season: Season =
            serde_json::from_str(body).map_err(|e| format!("the season does not read: {e}"))?;
        if season.schema_version != SEASON_SCHEMA {
            return Err(format!(
                "season schema_version {} is not {SEASON_SCHEMA}",
                season.schema_version
            ));
        }
        if season.clubs.len() > MAX_CLUBS {
            return Err(format!(
                "the season holds {} clubs; at most {MAX_CLUBS}",
                season.clubs.len()
            ));
        }
        for (club, part) in &season.clubs {
            if part.matches.len() > MAX_MATCHES {
                return Err(format!(
                    "club {club} has {} matches; at most {MAX_MATCHES}",
                    part.matches.len()
                ));
            }
            if part.players.len() > MAX_PLAYERS {
                return Err(format!(
                    "club {club} has {} players; at most {MAX_PLAYERS}",
                    part.players.len()
                ));
            }
            for (player, p) in &part.players {
                if p.ratings.len() > MAX_MATCHES {
                    return Err(format!(
                        "club {club}: player {player} has {} ratings; at most {MAX_MATCHES}",
                        p.ratings.len()
                    ));
                }
                if let Some(r) = p.ratings.iter().find(|r| !(1.0..=10.0).contains(&r.rating)) {
                    return Err(format!(
                        "club {club}: player {player} has the rating {} in match {}; allowed 1.0 to 10.0",
                        r.rating, r.match_id
                    ));
                }
            }
        }
        Ok(season)
    }

    /// The record in `data`. A missing file gives an empty record; a file that does not read,
    /// or is larger than [`MAX_FILE_BYTES`], gives an empty record and says so in a signal.
    /// A refused file is first moved aside (see [`set_aside`]), so the next save of the empty
    /// record never overwrites the person's matches. Never fails the launch.
    pub fn load(data: &Path) -> Season {
        Season::load_within(data, MAX_FILE_BYTES)
    }

    /// [`Season::load`] with a file larger than `max_bytes` refused.
    fn load_within(data: &Path, max_bytes: u64) -> Season {
        let path = data.join(SEASON_FILE);
        if let Ok(meta) = std::fs::metadata(&path)
            && meta.len() > max_bytes
        {
            refuse(
                &path,
                &format!(
                    "the season file is {} bytes; at most {max_bytes}",
                    meta.len()
                ),
            );
            return Season::default();
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Season::default();
            }
            Err(error) => {
                refuse(&path, &format!("the season file does not read: {error}"));
                return Season::default();
            }
        };
        Season::parse(&text).unwrap_or_else(|reason| {
            refuse(&path, &reason);
            Season::default()
        })
    }

    /// Writes the record to `data` through a temporary file, so a crash never leaves half a
    /// file.
    pub fn save(&self, data: &Path) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        write_through_temporary(&data.join(SEASON_FILE), &(text + "\n"))
    }

    /// Adds both sides of one finished match, each to its club, unless the club already holds
    /// the match or the record is full (a full club says so in a signal, and a club at the
    /// player cap takes no new player); `files` are the two
    /// sides' team files, home first, from which a player entering the record gets his goal.
    /// Returns how many clubs recorded the match: 0 when the record did not change.
    pub fn ingest(
        &mut self,
        stats: &MatchStats,
        files: [&TeamFile; 2],
        schema: &AttributeSchema,
    ) -> usize {
        let mut recorded = 0;
        for (side, file) in files.iter().enumerate() {
            let club = &stats.teams[side].id;
            if !self.clubs.contains_key(club) && self.clubs.len() >= MAX_CLUBS {
                continue;
            }
            let part = self.clubs.entry(club.clone()).or_default();
            if part.matches.contains(&stats.match_id) {
                continue;
            }
            if part.matches.len() >= MAX_MATCHES {
                tracing::warn!(
                    signal = "launch.season_full",
                    club = %club,
                    match_id = %stats.match_id,
                    matches = MAX_MATCHES
                );
                continue;
            }
            part.matches.push(stats.match_id.clone());
            recorded += 1;
            for entry in stats.ratings.iter().filter(|r| r.team == side) {
                let Some(player) = file.players.iter().find(|p| p.id == entry.id) else {
                    continue;
                };
                // A club at the player cap takes no new player, so the file stays one
                // [`Season::parse`] reads; its players already in the record keep counting.
                if !part.players.contains_key(&entry.id) && part.players.len() >= MAX_PLAYERS {
                    tracing::warn!(
                        signal = "launch.season_full",
                        club = %club,
                        match_id = %stats.match_id,
                        player = %entry.id,
                        players = MAX_PLAYERS
                    );
                    continue;
                }
                let p = part
                    .players
                    .entry(entry.id.clone())
                    .or_insert_with(|| PlayerSeason::new(assign_goal(player, schema)));
                p.starts = p.starts.saturating_add(u32::from(entry.started));
                p.minutes = p.minutes.saturating_add(entry.minutes);
                p.goals = p.goals.saturating_add(entry.goals);
                p.assists = p.assists.saturating_add(entry.assists);
                p.clean_sheets = p.clean_sheets.saturating_add(u32::from(entry.clean_sheet));
                p.red_cards = p.red_cards.saturating_add(entry.red);
                p.ratings.push(Rated {
                    match_id: stats.match_id.clone(),
                    rating: entry.rating.clamp(1.0, 10.0),
                });
            }
        }
        recorded
    }

    /// A copy of `file` at the current team-file version with each recorded player's recent
    /// form and goal progress in his condition (his other condition fields kept); `None` when
    /// the club has no match in the record. A file converted from version 1 has players with no
    /// body fields, and its copy says so (`converted_from`), so it loads as the converted file
    /// played.
    pub fn conditioned(&self, file: &TeamFile) -> Option<TeamFile> {
        let part = self.clubs.get(&file.club.id)?;
        if part.matches.is_empty() {
            return None;
        }
        let mut copy = file.clone();
        copy.schema_version = TEAM_VERSION;
        if copy
            .players
            .iter()
            .any(|p| p.height.is_none() || p.age.is_none() || p.nationality.is_none())
        {
            copy.converted_from = Some(CONVERTED_FROM);
        }
        for entry in &mut copy.players {
            let Some(p) = part.players.get(&entry.id) else {
                continue;
            };
            let mut c = entry.condition.unwrap_or_default();
            c.recent_rating = recent_rating(p);
            c.goal_progress = goal_progress(p, part.matches.len());
            entry.condition = Some(c);
        }
        Some(copy)
    }

    /// One club's part as the page reads it: `{"matches", "players"}`, each player with his
    /// record, his goal, his recent form (`form`, 1.0 to 10.0) and his goal progress
    /// (`progress`, percent of the pace). With the club's team file, every squad player is
    /// listed, those with no match yet at nothing played and the goal he will be given.
    pub fn club_json(
        &self,
        club: &str,
        squad: Option<(&TeamFile, &AttributeSchema)>,
    ) -> serde_json::Value {
        let part = self.clubs.get(club).cloned().unwrap_or_default();
        let mut players = part.players.clone();
        if let Some((file, schema)) = squad {
            for entry in &file.players {
                players
                    .entry(entry.id.clone())
                    .or_insert_with(|| PlayerSeason::new(assign_goal(entry, schema)));
            }
        }
        let players: serde_json::Map<String, serde_json::Value> = players
            .iter()
            .map(|(id, p)| {
                let mut row = serde_json::to_value(p).expect("a season always serializes");
                row["form"] = recent_rating(p).map(|t| f64::from(t) / 10.0).into();
                row["progress"] = goal_progress(p, part.matches.len()).into();
                (id.clone(), row)
            })
            .collect();
        serde_json::json!({ "matches": part.matches, "players": players })
    }
}

/// Says in a signal why the season file at `path` is refused, and moves it aside first, so
/// the empty record that replaces it in this launch never overwrites it on the next save.
fn refuse(path: &Path, reason: &str) {
    match set_aside(path) {
        Ok(kept) => tracing::warn!(
            signal = "launch.season_refused",
            reason = %reason,
            kept = %kept.display()
        ),
        Err(error) => tracing::warn!(
            signal = "launch.season_refused",
            reason = %reason,
            kept = %format!("not kept: {error}")
        ),
    }
}

/// Renames the file at `path` to the first free name of `<name>.refused`, `<name>.refused.1`,
/// `<name>.refused.2` and so on, beside it, and returns the new path. An earlier refused copy
/// is never replaced.
fn set_aside(path: &Path) -> std::io::Result<PathBuf> {
    let base = {
        let mut name = path.as_os_str().to_os_string();
        name.push(".refused");
        PathBuf::from(name)
    };
    let mut kept = base.clone();
    let mut n = 0u32;
    while kept.exists() {
        n += 1;
        let mut name = base.as_os_str().to_os_string();
        name.push(format!(".{n}"));
        kept = PathBuf::from(name);
    }
    std::fs::rename(path, &kept)?;
    Ok(kept)
}

/// Writes `text` at `path` through a temporary file beside it, creating the folder.
fn write_through_temporary(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut temporary = path.as_os_str().to_os_string();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    std::fs::write(&temporary, text)?;
    std::fs::rename(&temporary, path)
}

/// Writes `file` as the team-file copy `name` in the match folder `folder`, through a
/// temporary file. Returns its path.
pub fn write_copy(folder: &Path, name: &str, file: &TeamFile) -> std::io::Result<PathBuf> {
    let path = folder.join(name);
    let text = serde_json::to_string_pretty(file).map_err(std::io::Error::other)?;
    write_through_temporary(&path, &(text + "\n"))?;
    Ok(path)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use engine::observe::RatingEntry;
    use engine::rating::Rating;
    use engine::{Content, ContentDir, MatchConfig, Simulation};

    fn shipped() -> (ContentDir, Content) {
        let dir = ContentDir::at(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"));
        let content = Content::load(&dir).unwrap();
        (dir, content)
    }

    fn sample(content: &Content, dir: &ContentDir) -> Vec<crate::front_door::SampleTeam> {
        crate::front_door::sample_teams(dir, content).unwrap()
    }

    /// A temporary data folder, deleted when the test ends, also when it fails.
    pub(crate) struct Temp(pub(crate) PathBuf);

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    pub(crate) fn temp(name: &str) -> Temp {
        let dir = std::env::temp_dir().join(format!("season_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Temp(dir)
    }

    /// A record of match `id` between `files` with `ratings`.
    fn record(id: &str, files: [&TeamFile; 2], ratings: Vec<RatingEntry>) -> MatchStats {
        let mut stats = crate::front_door::tests::stats(id, &[]);
        for (side, file) in files.iter().enumerate() {
            stats.teams[side].id = file.club.id.clone();
        }
        stats.ratings = ratings;
        stats
    }

    /// Plays `files` for `minutes` on `seed` to its end.
    fn play(content: &Content, files: [&TeamFile; 2], seed: u64, minutes: u32) -> Simulation {
        let config = MatchConfig::new(seed, minutes, content, files).unwrap();
        let mut sim = Simulation::new(config).unwrap();
        while !sim.is_over() {
            sim.step();
        }
        sim.finish();
        sim
    }

    /// Two shipped clubs play three full seeded matches; each player's season
    /// starts, minutes, goals, assists, clean sheets, red cards and ratings are the sums over
    /// the three simulations' tallies; ingesting a match again adds nothing.
    #[test]
    fn three_matches_add_up_in_each_players_season() {
        let (dir, content) = shipped();
        let teams = sample(&content, &dir);
        let files = [&teams[0].file, &teams[1].file];
        let schema = &content.attributes;
        let minute = engine::rules::clock::TICKS_PER_MINUTE;
        let min_minutes = content.tuning.match_rating.clean_sheet.min_minutes;
        let mut season = Season::default();
        // Per side and player id: starts, minutes, goals, assists, clean sheets, red; ratings.
        type Sums = BTreeMap<String, ([u32; 6], Vec<f64>)>;
        let mut want: [Sums; 2] = Default::default();
        let mut kept = Vec::new();
        for seed in 1..=3 {
            let sim = play(&content, files, seed, 90);
            let ratings = RatingEntry::of_match(&sim);
            for r in &ratings {
                let squad = files[r.team]
                    .players
                    .iter()
                    .position(|p| p.id == r.id)
                    .unwrap();
                let t = sim.tallies()[r.team][squad];
                let minutes = t.ticks_played / minute;
                let clean = t.conceded == 0 && minutes >= min_minutes;
                let row = want[r.team].entry(r.id.clone()).or_default();
                let add = [
                    u32::from(t.started),
                    minutes,
                    t.goals,
                    t.assists,
                    u32::from(clean),
                    t.red,
                ];
                for (k, v) in add.into_iter().enumerate() {
                    row.0[k] += v;
                }
                row.1.push(r.rating);
            }
            let stats = record(&format!("m-{seed}"), files, ratings);
            assert!(season.ingest(&stats, files, schema) > 0);
            kept.push(stats);
        }
        let mut scored = 0;
        for (side, file) in files.iter().enumerate() {
            let part = &season.clubs[&file.club.id];
            assert_eq!(part.matches, ["m-1", "m-2", "m-3"]);
            assert_eq!(part.players.len(), want[side].len());
            for (id, (sums, ratings)) in &want[side] {
                let p = &part.players[id];
                let got = [
                    p.starts,
                    p.minutes,
                    p.goals,
                    p.assists,
                    p.clean_sheets,
                    p.red_cards,
                ];
                assert_eq!(&got, sums, "{id}");
                let list: Vec<f64> = p.ratings.iter().map(|r| r.rating).collect();
                assert_eq!(&list, ratings, "{id}");
                scored += p.goals + p.assists;
            }
        }
        assert!(scored > 0, "three matches with no goal and no assist");
        let before = season.clone();
        for stats in &kept {
            assert_eq!(season.ingest(stats, files, schema), 0);
        }
        assert_eq!(season, before, "a match ingested again adds nothing");
    }

    /// Every player of each of the ten shipped squads has exactly one goal, the
    /// table's kind for his listed position and level band.
    #[test]
    fn every_shipped_player_gets_the_tables_goal() {
        let (dir, content) = shipped();
        let schema = &content.attributes;
        let teams = sample(&content, &dir);
        assert_eq!(teams.len(), 10);
        for team in &teams {
            for entry in &team.file.players {
                let ratings = engine::player::Attributes::from_entry(&entry.attributes, schema);
                let level =
                    engine::contract::level::overall_level(&ratings, schema, entry.position);
                let goal = assign_goal(entry, schema);
                assert_eq!(
                    goal.kind,
                    goal_kind(entry.position, band(level)),
                    "{}",
                    entry.id
                );
                assert_eq!(goal.target, goal_target(goal.kind, entry.position, level));
            }
        }
    }

    /// A planted squad with every position at a high, a middle and a low level yields all six
    /// kinds; a keeper of middle or high level aims for clean sheets; targets rise with level.
    #[test]
    fn a_planted_squad_covers_all_six_kinds_and_targets_rise_with_level() {
        let (dir, content) = shipped();
        let schema = &content.attributes;
        let base = sample(&content, &dir)[0].file.players[0].clone();
        let at = |position: Position, tenths: u8| {
            let mut e = base.clone();
            e.position = position;
            for v in e.attributes.values_mut() {
                *v = Rating::from_tenths(tenths);
            }
            assign_goal(&e, schema)
        };
        let mut kinds = std::collections::BTreeSet::new();
        for position in Position::ALL {
            for tenths in [160, 120, 80] {
                kinds.insert(at(position, tenths).kind);
            }
        }
        assert_eq!(kinds.len(), 6, "{kinds:?}");
        assert_eq!(at(Position::GK, 160).kind, GoalKind::CleanSheets);
        assert_eq!(at(Position::GK, 120).kind, GoalKind::CleanSheets);
        assert_eq!(at(Position::GK, 80).kind, GoalKind::Starts);
        for (position, low, high) in [
            (Position::ST, 100, 180),
            (Position::ST, 140, 200),
            (Position::AM, 140, 190),
            (Position::CM, 100, 130),
            (Position::GK, 140, 200),
            (Position::DM, 140, 200),
        ] {
            let (a, b) = (at(position, low), at(position, high));
            assert_eq!(a.kind, b.kind);
            assert!(b.target > a.target, "{position:?}: {a:?} then {b:?}");
        }
        assert_eq!(goal_target(GoalKind::Goals, Position::ST, 100), 8.0);
        assert_eq!(goal_target(GoalKind::Goals, Position::ST, 200), 24.0);
        assert_eq!(goal_target(GoalKind::AverageRating, Position::CM, 150), 6.9);
    }

    fn season_of(kind: GoalKind, target: f64, ratings: &[f64]) -> PlayerSeason {
        let mut p = PlayerSeason::new(Goal { kind, target });
        p.ratings = ratings
            .iter()
            .enumerate()
            .map(|(k, &rating)| Rated {
                match_id: format!("m-{k}"),
                rating,
            })
            .collect();
        p
    }

    /// Progress against the pace, and form from the newest five ratings.
    #[test]
    fn progress_follows_the_pace_and_form_reads_the_newest_five() {
        let mut p = season_of(GoalKind::Goals, 19.0, &[]);
        assert_eq!(goal_progress(&p, 0), None, "no club match yet");
        p.goals = 5;
        assert_eq!(goal_progress(&p, 10), Some(100), "on track");
        p.goals = 8;
        assert!(goal_progress(&p, 10).unwrap() > 100, "ahead");
        p.goals = 40;
        assert_eq!(goal_progress(&p, 10), Some(200), "kept to 200");
        p.goals = 0;
        assert_eq!(goal_progress(&p, 10), Some(0));
        let mut clean = season_of(GoalKind::NoOwnFaultAbsence, 0.0, &[7.0]);
        assert_eq!(goal_progress(&clean, 3), Some(100));
        clean.red_cards = 1;
        assert_eq!(goal_progress(&clean, 3), Some(0), "a red card");
        let average = season_of(GoalKind::AverageRating, 7.0, &[7.0, 7.7]);
        assert_eq!(goal_progress(&average, 2), Some(105));
        let unrated = season_of(GoalKind::AverageRating, 7.0, &[]);
        assert_eq!(goal_progress(&unrated, 2), None);
        let form = season_of(GoalKind::Starts, 20.0, &[2.0, 2.0, 6.0, 7.0, 8.0, 6.0, 7.5]);
        assert_eq!(
            recent_rating(&form),
            Some(69),
            "the newest five: 6, 7, 8, 6, 7.5"
        );
        assert_eq!(recent_rating(&season_of(GoalKind::Starts, 20.0, &[])), None);
    }

    /// The record saves and reads back; a damaged file starts empty; each refusal names its
    /// value.
    #[test]
    fn the_record_saves_and_reads_back_and_refuses_bad_values_by_name() {
        let folder = temp("store");
        let data = &folder.0;
        assert_eq!(Season::load(data), Season::default());
        let mut season = Season::default();
        let part = season.clubs.entry("club-a".into()).or_default();
        part.matches.push("m-0".into());
        part.players
            .insert("a-1".into(), season_of(GoalKind::Assists, 9.0, &[7.4]));
        season.save(data).unwrap();
        assert!(!data.join(format!("{SEASON_FILE}.tmp")).exists());
        assert_eq!(Season::load(data), season);
        std::fs::write(data.join(SEASON_FILE), "{ not json").unwrap();
        assert_eq!(Season::load(data), Season::default());
        let refused_copy = data.join(format!("{SEASON_FILE}.refused"));
        assert_eq!(
            std::fs::read_to_string(&refused_copy).unwrap(),
            "{ not json"
        );
        assert!(!data.join(SEASON_FILE).exists());
        std::fs::remove_file(&refused_copy).unwrap();

        let good = serde_json::to_value(&season).unwrap();
        let refused = |edit: &dyn Fn(&mut serde_json::Value)| {
            let mut doc = good.clone();
            edit(&mut doc);
            Season::parse(&doc.to_string()).unwrap_err()
        };
        let text = refused(&|d| d["schema_version"] = 2.into());
        assert!(text.contains("schema_version 2"), "{text}");
        let text = refused(&|d| {
            d["clubs"]["club-a"]["players"]["a-1"]["ratings"][0]["rating"] = 11.0.into();
        });
        assert!(
            text.contains("player a-1 has the rating 11 in match m-0; allowed 1.0 to 10.0"),
            "{text}"
        );
        let text = refused(&|d| {
            d["clubs"]["club-a"]["players"]["a-1"]["goal"]["kind"] = "wins".into();
        });
        assert!(text.contains("wins"), "{text}");
        let text = refused(&|d| {
            let many: Vec<String> = (0..=MAX_MATCHES).map(|k| format!("m-{k}")).collect();
            d["clubs"]["club-a"]["matches"] = many.into();
        });
        assert!(text.contains("201 matches; at most 200"), "{text}");
        let text = refused(&|d| {
            for k in 0..MAX_CLUBS {
                d["clubs"][format!("c-{k}")] = serde_json::json!({});
            }
        });
        assert!(text.contains("501 clubs; at most 500"), "{text}");
        let text = refused(&|d| {
            let one = d["clubs"]["club-a"]["players"]["a-1"]["ratings"][0].clone();
            d["clubs"]["club-a"]["players"]["a-1"]["ratings"] = vec![one; MAX_MATCHES + 1].into();
        });
        assert!(
            text.contains("player a-1 has 201 ratings; at most 200"),
            "{text}"
        );
        let text = refused(&|d| {
            let one = d["clubs"]["club-a"]["players"]["a-1"].clone();
            for k in 0..MAX_PLAYERS {
                d["clubs"]["club-a"]["players"][format!("p-{k}")] = one.clone();
            }
        });
        assert!(text.contains("1001 players; at most 1000"), "{text}");
        // A file over the size limit loads empty and never fails the launch.
        season.save(data).unwrap();
        let size = std::fs::metadata(data.join(SEASON_FILE)).unwrap().len();
        assert_eq!(Season::load_within(data, size), season);
        assert_eq!(Season::load_within(data, size - 1), Season::default());
        // The over-size file is kept aside whole, not lost.
        let kept = data.join(format!("{SEASON_FILE}.refused"));
        assert_eq!(
            Season::parse(&std::fs::read_to_string(kept).unwrap()).unwrap(),
            season
        );
    }

    /// A refused season file survives the next save: the launch keeps the empty record, the
    /// first finished match saves it, and the person's own file is still on disk beside it,
    /// under `.refused`, then `.refused.1` for a second refusal.
    #[test]
    fn a_refused_file_is_kept_aside_and_the_next_save_never_replaces_it() {
        let folder = temp("refused");
        let data = &folder.0;
        let mut season = Season::default();
        season
            .clubs
            .entry("club-a".into())
            .or_default()
            .matches
            .push("m-0".into());
        season.save(data).unwrap();
        let mut doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(data.join(SEASON_FILE)).unwrap())
                .unwrap();
        doc["schema_version"] = 99.into();
        let theirs = doc.to_string();
        std::fs::write(data.join(SEASON_FILE), &theirs).unwrap();

        let mut fresh = Season::load(data);
        assert_eq!(fresh, Season::default());
        fresh
            .clubs
            .entry("club-b".into())
            .or_default()
            .matches
            .push("m-1".into());
        fresh.save(data).unwrap();
        let kept = data.join(format!("{SEASON_FILE}.refused"));
        assert_eq!(std::fs::read_to_string(&kept).unwrap(), theirs);
        assert_eq!(Season::load(data), fresh);

        std::fs::write(data.join(SEASON_FILE), "{ not json").unwrap();
        assert_eq!(Season::load(data), Season::default());
        assert_eq!(std::fs::read_to_string(&kept).unwrap(), theirs);
        let second = data.join(format!("{SEASON_FILE}.refused.1"));
        assert_eq!(std::fs::read_to_string(&second).unwrap(), "{ not json");
    }

    /// A club at the player cap takes no new player: its players already in the record keep
    /// counting, and the saved file loads back whole instead of being refused.
    #[test]
    fn a_club_at_the_player_cap_keeps_its_season_file_through_a_save_and_a_load() {
        let folder = temp("capped");
        let data = &folder.0;
        let (dir, content) = shipped();
        let teams = sample(&content, &dir);
        let files = [&teams[0].file, &teams[1].file];
        let schema = &content.attributes;
        let known = files[0].players[0].id.clone();
        let new = files[0].players[1].id.clone();
        let mut season = Season::default();
        let part = season.clubs.entry(files[0].club.id.clone()).or_default();
        part.matches.push("m-0".into());
        part.players
            .insert(known.clone(), season_of(GoalKind::Starts, 20.0, &[7.0]));
        for k in 1..MAX_PLAYERS {
            part.players.insert(
                format!("old-{k}"),
                season_of(GoalKind::Starts, 20.0, &[6.0]),
            );
        }
        assert_eq!(part.players.len(), MAX_PLAYERS);

        let rated = |id: &str| RatingEntry {
            id: id.into(),
            team: 0,
            rating: 7.5,
            started: true,
            minutes: 90,
            goals: 0,
            assists: 0,
            clean_sheet: false,
            red: 0,
        };
        let stats = record("m-1", files, vec![rated(&known), rated(&new)]);
        assert_eq!(season.ingest(&stats, files, schema), 2);
        let part = &season.clubs[&files[0].club.id];
        assert_eq!(part.players.len(), MAX_PLAYERS, "no player past the cap");
        assert!(!part.players.contains_key(&new));
        assert_eq!(
            part.players[&known].ratings.len(),
            2,
            "a known player counts on"
        );
        assert_eq!(part.matches, ["m-0", "m-1"]);

        season.save(data).unwrap();
        let loaded = Season::load(data);
        assert_eq!(loaded, season, "the capped file loads back whole");
        assert!(!data.join(format!("{SEASON_FILE}.refused")).exists());
    }

    /// The copy: none for a club with no match; a copy with no input set plays the same seed
    /// to the same events as the shipped file; a copy of a club with three good matches loads
    /// with the inputs, and its in-form player starts above neutral: the kick-off mood, end to end.
    #[test]
    fn the_copy_carries_form_and_progress_into_the_next_match() {
        let (dir, content) = shipped();
        let teams = sample(&content, &dir);
        let files = [&teams[0].file, &teams[1].file];
        let schema = &content.attributes;
        let mut season = Season::default();
        assert!(season.conditioned(files[0]).is_none());

        // A club with a match and no player recorded: the copy sets no input.
        season.ingest(&record("m-0", files, Vec::new()), files, schema);
        let bare = season.conditioned(files[0]).unwrap();
        assert_eq!(bare.schema_version, TEAM_VERSION);
        let folder = temp("copy");
        let data = &folder.0;
        let read = |path: &Path| {
            content
                .team_from_bytes(&std::fs::read(path).unwrap(), "team-a.json")
                .unwrap()
                .value
        };
        let loaded = read(&write_copy(data, COPY_FILES[0], &bare).unwrap());
        let events = |a: &TeamFile| play(&content, [a, files[1]], 11, 20).take_events();
        assert_eq!(events(&loaded), events(files[0]), "the copy plays the same");

        // Three good matches for the home side's player 3, a starter: on or ahead of his goal.
        let star = &files[0].players[3];
        for k in 1..=3 {
            let entry = RatingEntry {
                id: star.id.clone(),
                team: 0,
                rating: 8.0,
                started: true,
                minutes: 90,
                goals: 2,
                assists: 2,
                clean_sheet: true,
                red: 0,
            };
            season.ingest(
                &record(&format!("m-{k}"), files, vec![entry]),
                files,
                schema,
            );
        }
        let p = &season.clubs[&files[0].club.id].players[&star.id];
        assert_eq!(recent_rating(p), Some(80));
        assert!(goal_progress(p, 4).unwrap() >= 100, "{p:?}");
        let loaded =
            read(&write_copy(data, COPY_FILES[0], &season.conditioned(files[0]).unwrap()).unwrap());
        let c = loaded.players[3].condition.unwrap();
        assert_eq!(c.recent_rating, Some(80));
        assert!(c.goal_progress.unwrap() >= 100);
        assert_eq!(loaded.players[4].condition, files[0].players[4].condition);
        let config = MatchConfig::new(11, 90, &content, [&loaded, files[1]]).unwrap();
        let sim = Simulation::new(config).unwrap();
        let i = sim
            .players()
            .iter()
            .position(|q| q.team == 0 && q.squad == 3)
            .expect("player 3 starts");
        let moods: Vec<f64> = sim.players().iter().map(|q| q.mood).collect();
        println!(
            "kick-off moods after three good matches ({:?}, progress {:?}): {moods:?}",
            p.goal,
            goal_progress(p, 4)
        );
        assert!(
            sim.mood(i) > 0.0,
            "he starts above neutral: {}",
            sim.mood(i)
        );
        assert!(
            moods.iter().enumerate().all(|(k, &m)| k == i || m == 0.0),
            "no other player has a season: {moods:?}"
        );
    }
}

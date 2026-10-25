//! The PFF FC FIFA World Cup 2022 files (broadcast tracking, from the data-request form), in
//! `<data>/pff-wc2022`: per match `Metadata/<id>.json` (the teams, the pitch, the frame rate
//! and which end the home team starts on), `Rosters/<id>.json` (each player's team, shirt
//! number, position group and whether he started), `Event Data/<id>.json` (the events in
//! video seconds) and `Tracking Data/<id>.jsonl.bz2` (one JSON line per frame at 29.97
//! frames a second, in metres from the centre spot, with the video time in milliseconds).
//! A fixture may hold `<id>.jsonl` uncompressed instead.
//!
//! The reader streams the tracking file one line at a time and keeps the provider's smoothed
//! player and ball positions of periods 1 and 2; the frames of any other period (extra time,
//! the shoot-out, the few frames before the first half) are left out and counted. A person
//! is a roster player, matched in the frames by team and shirt number; it keeps the
//! provider's player id and never a name. It counts the share of the kept player rows the
//! provider estimated off camera and the frames without a ball.

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail, ensure};
use serde::{Deserialize, Deserializer};

use super::side::LeftOut;
use super::{Event, EventKind, Frame, Group, Half, Height, Notes, Person, TrackedMatch};

/// The provider as targets and side files name it.
pub const SOURCE: &str = "PFF";

/// The folder of the World Cup 2022 files under the data folder.
pub const FOLDER: &str = "pff-wc2022";

/// A match read at its native frames, before resampling.
#[derive(Debug, Clone, PartialEq)]
pub struct Native {
    /// The match at its native frames. Its `ticks_per_frame` is 0: the converter takes it
    /// only once resampled.
    pub m: TrackedMatch,
    /// Each kept frame's video time in milliseconds, per half, unrounded.
    pub times: Vec<Vec<f64>>,
    /// The native frames a second.
    pub fps: f64,
    /// Frame lines read, of every period.
    pub lines: u64,
    /// Kept frames without a ball position.
    pub ball_absent: u32,
}

/// The folder that holds the PFF files.
pub fn folder(data: &Path) -> PathBuf {
    data.join(FOLDER)
}

/// The ids of every match of `data` with a metadata file, in numeric order.
pub fn matches(data: &Path) -> anyhow::Result<Vec<String>> {
    let dir = folder(data).join("Metadata");
    let mut ids: Vec<String> = std::fs::read_dir(&dir)
        .with_context(|| format!("cannot list {}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter_map(|n| n.strip_suffix(".json").map(str::to_string))
        .collect();
    ids.sort_by_key(|id| (id.parse::<u64>().unwrap_or(u64::MAX), id.clone()));
    Ok(ids)
}

/// An id or number the files write as a string, an integer or a float such as `3812.0`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Id(String);

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match serde_json::Value::deserialize(d)? {
            serde_json::Value::String(s) => Ok(Id(s)),
            serde_json::Value::Number(n) => match (n.as_i64(), n.as_f64()) {
                (Some(i), _) => Ok(Id(i.to_string())),
                (None, Some(f)) if f.fract() == 0.0 => Ok(Id(format!("{f:.0}"))),
                _ => Err(serde::de::Error::custom(format!("{n} is not an id"))),
            },
            other => Err(serde::de::Error::custom(format!("{other} is not an id"))),
        }
    }
}

#[derive(Deserialize)]
struct Team {
    id: Id,
}

#[derive(Deserialize)]
struct PitchSize {
    length: f64,
    width: f64,
}

#[derive(Deserialize)]
struct Stadium {
    pitches: Vec<PitchSize>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Metadata {
    id: Id,
    fps: f64,
    home_team: Team,
    away_team: Team,
    #[serde(default)]
    home_team_start_left: Option<bool>,
    stadium: Stadium,
}

#[derive(Deserialize)]
struct RosterPlayer {
    id: Id,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RosterRow {
    player: RosterPlayer,
    #[serde(default)]
    position_group_type: Option<String>,
    shirt_number: Id,
    #[serde(default)]
    started: bool,
    team: Team,
}

#[derive(Deserialize)]
struct Row {
    #[serde(rename = "jerseyNum")]
    jersey: Id,
    #[serde(default)]
    visibility: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
}

#[derive(Deserialize)]
struct Ball {
    x: Option<f64>,
    y: Option<f64>,
    #[serde(default)]
    z: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Line {
    frame_num: u32,
    #[serde(default)]
    period: Option<Id>,
    video_time_ms: f64,
    #[serde(default)]
    home_players_smoothed: Option<Vec<Row>>,
    #[serde(default)]
    away_players_smoothed: Option<Vec<Row>>,
    #[serde(default)]
    balls_smoothed: Option<Ball>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GameEvents {
    #[serde(default)]
    game_event_type: Option<String>,
    #[serde(default)]
    period: Option<Id>,
    #[serde(default)]
    team_id: Option<Id>,
    #[serde(default)]
    player_id: Option<Id>,
    #[serde(default)]
    setpiece_type: Option<String>,
    #[serde(default)]
    player_off_id: Option<Id>,
    #[serde(default)]
    player_on_id: Option<Id>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PossessionEvents {
    #[serde(default)]
    possession_event_type: Option<String>,
    #[serde(default)]
    high_point_type: Option<String>,
    #[serde(default)]
    shot_outcome_type: Option<String>,
    #[serde(default)]
    receiver_player_id: Option<Id>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Fouls {
    #[serde(default)]
    final_foul_outcome_type: Option<String>,
    #[serde(default)]
    final_offense_type: Option<String>,
    #[serde(default)]
    final_culprit_player_id: Option<Id>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEvent {
    #[serde(default)]
    start_time: Option<f64>,
    #[serde(default)]
    event_time: Option<f64>,
    #[serde(default)]
    game_events: Option<GameEvents>,
    #[serde(default)]
    possession_events: Option<PossessionEvents>,
    #[serde(default)]
    fouls: Option<Fouls>,
}

/// What a PFF event does to the ball status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flow {
    /// A player is on the ball, from a set piece or in open play: the ball is live.
    OnBall,
    /// The ball goes dead: out of play, the end of a half, a foul or offside given.
    Stop,
    None,
}

/// One PFF event read: its half, its time in video milliseconds, the events it gives, its
/// effect on the ball status and the team on the ball.
#[derive(Debug, Clone)]
pub(crate) struct Read1 {
    pub half: usize,
    pub ms: f64,
    pub events: Vec<Event>,
    pub flow: Flow,
    pub team: Option<usize>,
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> anyhow::Result<T> {
    let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    serde_json::from_reader(BufReader::with_capacity(1 << 20, file))
        .with_context(|| format!("{} is not a PFF file of its kind", path.display()))
}

/// The period of a frame or event as its word: `1`, `2`, ..., or `none`.
fn period_word(p: &Option<Id>) -> String {
    p.as_ref().map_or("none".to_string(), |p| p.0.clone())
}

/// The kept half of a period: 0 for period 1, 1 for period 2, `None` otherwise.
fn half_of(p: &Option<Id>) -> Option<usize> {
    match p.as_ref().map(|p| p.0.as_str()) {
        Some("1") => Some(0),
        Some("2") => Some(1),
        _ => None,
    }
}

/// A pass's height from the highest point the ball reached: on the ground or below the
/// waist a ground pass, between waist and head or above the head a high pass.
fn height(high_point: Option<&str>) -> Option<Height> {
    match high_point? {
        "G" | "L" => Some(Height::Ground),
        "H" | "A" => Some(Height::High),
        _ => None,
    }
}

/// Reads the events of periods 1 and 2 of one match in time order: each event's kinds, time
/// and effect on the ball.
fn read_events(
    path: &Path,
    teams: &[String; 2],
    team_of_person: &HashMap<String, usize>,
) -> anyhow::Result<Vec<Read1>> {
    let raw: Vec<RawEvent> = read_json(path)?;
    let team = |id: &Option<Id>| {
        id.as_ref()
            .and_then(|t| teams.iter().position(|x| *x == t.0))
    };
    let mut out = Vec::new();
    for r in raw {
        let g = r.game_events.unwrap_or_default();
        let Some(half) = half_of(&g.period) else {
            continue;
        };
        let Some(secs) = r.event_time.or(r.start_time) else {
            continue;
        };
        let ms = secs * 1000.0;
        let pe = r.possession_events.unwrap_or_default();
        let fouls = r.fouls.unwrap_or_default();
        let on_team = team(&g.team_id);
        let player = g.player_id.as_ref().map(|p| p.0.clone());
        let event = |kind: EventKind, team: Option<usize>, person: Option<String>| Event {
            kind,
            team,
            person,
            time_ms: Some(ms.round() as i64),
            clock: None,
            decision_ms: None,
            recipient: None,
        };
        let mut events = Vec::new();
        let mut flow = Flow::None;
        let mut on_ball = None;
        match g.game_event_type.as_deref() {
            Some("FIRSTKICKOFF" | "SECONDKICKOFF") => {
                events.push(event(
                    EventKind::KickOff { half_start: true },
                    on_team,
                    player,
                ));
                flow = Flow::OnBall;
                on_ball = on_team;
            }
            Some("OTB") => {
                let kind = match g.setpiece_type.as_deref() {
                    Some("K") => EventKind::KickOff { half_start: false },
                    Some("T") => EventKind::ThrowIn,
                    Some("C") => EventKind::Corner,
                    Some("G") => EventKind::GoalKick,
                    Some("F") => EventKind::FreeKick,
                    Some("P") => EventKind::Penalty,
                    Some("D") => EventKind::DroppedBall,
                    _ => match pe.possession_event_type.as_deref() {
                        Some("PA") => EventKind::Pass {
                            height: height(pe.high_point_type.as_deref()),
                        },
                        Some("CR") => EventKind::Cross,
                        Some("SH") => EventKind::Shot {
                            goal: pe.shot_outcome_type.as_deref() == Some("G"),
                        },
                        other => EventKind::Other(format!(
                            "on_ball_{}",
                            other.unwrap_or("none").to_lowercase()
                        )),
                    },
                };
                let mut e = event(kind, on_team, player);
                e.recipient = pe.receiver_player_id.as_ref().map(|p| p.0.clone());
                events.push(e);
                flow = Flow::OnBall;
                on_ball = on_team;
            }
            Some("OUT") => {
                events.push(event(EventKind::Other("out".into()), None, None));
                flow = Flow::Stop;
            }
            Some("END") => {
                events.push(event(EventKind::FinalWhistle, None, None));
                flow = Flow::Stop;
            }
            Some("SUB") => {
                let (Some(on), Some(off)) = (&g.player_on_id, &g.player_off_id) else {
                    bail!("{}: a substitution names no player", path.display());
                };
                let team = team_of_person.get(&off.0).copied();
                events.push(event(
                    EventKind::Substitution {
                        on: on.0.clone(),
                        off: off.0.clone(),
                    },
                    team,
                    None,
                ));
            }
            Some("OFF") => events.push(event(EventKind::Other("off".into()), on_team, player)),
            Some("ON") => events.push(event(EventKind::Other("on".into()), on_team, player)),
            Some("G") => events.push(event(EventKind::Other("woodwork".into()), None, None)),
            _ => {}
        }
        // A foul or offside given stops play; a card follows it.
        if let Some(outcome @ ("N" | "W" | "Y" | "S" | "R")) =
            fouls.final_foul_outcome_type.as_deref()
        {
            let culprit = fouls.final_culprit_player_id.as_ref().map(|p| p.0.clone());
            let team = culprit
                .as_ref()
                .and_then(|c| team_of_person.get(c).copied());
            let kind = if fouls.final_offense_type.as_deref() == Some("O") {
                EventKind::Offside
            } else {
                EventKind::Foul
            };
            events.push(event(kind, team, culprit.clone()));
            match outcome {
                "Y" => events.push(event(EventKind::Card { sending_off: false }, team, culprit)),
                "S" | "R" => {
                    events.push(event(EventKind::Card { sending_off: true }, team, culprit))
                }
                _ => {}
            }
            flow = Flow::Stop;
        }
        if events.is_empty() {
            continue;
        }
        out.push(Read1 {
            half,
            ms,
            events,
            flow,
            team: on_ball,
        });
    }
    out.sort_by(|a, b| a.ms.total_cmp(&b.ms));
    Ok(out)
}

/// The tracking file of match `id`: `<id>.jsonl` when present (a fixture), else
/// `<id>.jsonl.bz2`.
fn tracking_file(dir: &Path, id: &str) -> anyhow::Result<PathBuf> {
    let plain = dir.join(format!("{id}.jsonl"));
    if plain.is_file() {
        return Ok(plain);
    }
    let packed = dir.join(format!("{id}.jsonl.bz2"));
    ensure!(
        packed.is_file(),
        "match {id}: no tracking file {} or {}",
        plain.display(),
        packed.display()
    );
    Ok(packed)
}

/// Reads match `id` of `data` at its native frames.
pub fn read_native(data: &Path, id: &str) -> anyhow::Result<Native> {
    let dir = folder(data);
    let meta: Vec<Metadata> = read_json(&dir.join("Metadata").join(format!("{id}.json")))?;
    let meta = meta
        .into_iter()
        .next()
        .with_context(|| format!("match {id}: the metadata file is empty"))?;
    ensure!(
        meta.fps > 0.0,
        "match {id}: the metadata gives {} frames a second",
        meta.fps
    );
    let pitch = meta
        .stadium
        .pitches
        .first()
        .with_context(|| format!("match {id}: the metadata names no pitch"))?;
    let teams = [meta.home_team.id.0.clone(), meta.away_team.id.0.clone()];

    let roster: Vec<RosterRow> = read_json(&dir.join("Rosters").join(format!("{id}.json")))?;
    let mut rows_by_team = Vec::new();
    for r in &roster {
        let team = teams
            .iter()
            .position(|t| *t == r.team.id.0)
            .with_context(|| {
                format!(
                    "match {id}: a roster player's team {} plays no part",
                    r.team.id.0
                )
            })?;
        rows_by_team.push((team, r));
    }
    rows_by_team.sort_by_key(|(team, _)| *team);
    let shirt: HashMap<(usize, String), usize> = rows_by_team
        .iter()
        .enumerate()
        .map(|(i, (team, r))| ((*team, r.shirt_number.0.clone()), i))
        .collect();
    ensure!(
        shirt.len() == rows_by_team.len(),
        "match {id}: two roster players of one team share a shirt number"
    );
    let team_of_person: HashMap<String, usize> = rows_by_team
        .iter()
        .map(|(team, r)| (r.player.id.0.clone(), *team))
        .collect();
    let persons: Vec<Person> = rows_by_team
        .iter()
        .map(|(team, r)| Person {
            id: r.player.id.0.clone(),
            team: *team,
            starting: r.started,
            position: r.position_group_type.clone(),
            group: r.position_group_type.as_deref().and_then(Group::from_pff),
        })
        .collect();

    // The frames, one line at a time.
    let path = tracking_file(&dir.join("Tracking Data"), id)?;
    let file = File::open(&path).with_context(|| format!("cannot open {}", path.display()))?;
    let input: Box<dyn Read> = if path.extension().is_some_and(|e| e == "bz2") {
        Box::new(bzip2_rs::DecoderReader::new(BufReader::with_capacity(
            1 << 20,
            file,
        )))
    } else {
        Box::new(file)
    };
    let mut reader = BufReader::with_capacity(1 << 20, input);
    let mut halves: Vec<Half> = ["firstHalf", "secondHalf"]
        .iter()
        .map(|s| Half {
            section: s.to_string(),
            frames: Vec::new(),
            positions: vec![Vec::new(); persons.len()],
        })
        .collect();
    let mut times: Vec<Vec<f64>> = vec![Vec::new(), Vec::new()];
    let mut left: BTreeMap<String, u32> = BTreeMap::new();
    let (mut lines, mut rows, mut estimated, mut ball_absent) = (0u64, 0u64, 0u64, 0u32);
    let mut text = String::new();
    loop {
        text.clear();
        let n = reader
            .read_line(&mut text)
            .with_context(|| format!("cannot read {} after {lines} lines", path.display()))?;
        if n == 0 {
            break;
        }
        if text.trim().is_empty() {
            continue;
        }
        lines += 1;
        let line: Line = serde_json::from_str(&text)
            .with_context(|| format!("{} line {lines} is not a PFF frame", path.display()))?;
        let Some(h) = half_of(&line.period) else {
            *left.entry(period_word(&line.period)).or_insert(0) += 1;
            continue;
        };
        if let Some(&last) = times[h].last() {
            ensure!(
                line.video_time_ms > last,
                "{} line {lines}: frame {} is not later than the frame before it",
                path.display(),
                line.frame_num
            );
        }
        let half = &mut halves[h];
        let ball = line
            .balls_smoothed
            .as_ref()
            .and_then(|b| Some([b.x? as f32, b.y? as f32, b.z.unwrap_or(0.0) as f32]));
        if ball.is_none() {
            ball_absent += 1;
        }
        half.frames.push(Frame {
            n: line.frame_num,
            t_ms: line.video_time_ms.round() as i64,
            ball,
            live: None,
            possession: None,
        });
        times[h].push(line.video_time_ms);
        for p in &mut half.positions {
            p.push(None);
        }
        let sides = [&line.home_players_smoothed, &line.away_players_smoothed];
        for (team, side) in sides.into_iter().enumerate() {
            for r in side.iter().flatten() {
                let (Some(x), Some(y)) = (r.x, r.y) else {
                    continue;
                };
                let Some(&p) = shirt.get(&(team, r.jersey.0.clone())) else {
                    continue;
                };
                rows += 1;
                if r.visibility.as_deref() == Some("ESTIMATED") {
                    estimated += 1;
                }
                if let Some(at) = half.positions[p].last_mut() {
                    *at = Some([x as f32, y as f32]);
                }
            }
        }
    }
    for (h, half) in halves.iter().enumerate() {
        ensure!(
            !half.frames.is_empty(),
            "match {id}: {} has no frame of period {}",
            path.display(),
            h + 1
        );
    }

    let read = read_events(
        &dir.join("Event Data").join(format!("{id}.json")),
        &teams,
        &team_of_person,
    )?;
    let events: Vec<Event> = read.iter().flat_map(|r| r.events.clone()).collect();
    let m = TrackedMatch {
        source: SOURCE.into(),
        match_id: meta.id.0.clone(),
        pitch: [pitch.length, pitch.width],
        teams,
        persons,
        halves,
        events,
        ticks_per_frame: 0,
        resampled_from: None,
        notes: Notes {
            ball_status_from: Some("events".into()),
            estimated_share: (rows > 0).then(|| estimated as f64 / rows as f64),
            left_out: left
                .into_iter()
                .map(|(period, frames)| LeftOut { period, frames })
                .collect(),
            home_starts_left: meta.home_team_start_left,
        },
    };
    Ok(Native {
        m,
        times,
        fps: meta.fps,
        lines,
        ball_absent,
    })
}

#[cfg(test)]
mod tests {
    use super::super::{EventKind, Group};
    use super::*;

    fn data() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracking/pff")
    }

    fn fixture() -> Native {
        read_native(&data(), "9001").unwrap()
    }

    fn person(m: &TrackedMatch, id: &str) -> usize {
        m.persons.iter().position(|p| p.id == id).unwrap()
    }

    #[test]
    fn every_position_group_type_has_its_group() {
        let want = [
            ("GK", Group::Keeper),
            ("LCB", Group::Cb),
            ("RCB", Group::Cb),
            ("MCB", Group::Cb),
            ("LB", Group::Fb),
            ("RB", Group::Fb),
            ("LWB", Group::Fb),
            ("RWB", Group::Fb),
            ("DM", Group::Cm),
            ("CM", Group::Cm),
            ("AM", Group::Cm),
            ("LM", Group::W),
            ("RM", Group::W),
            ("LW", Group::W),
            ("RW", Group::W),
            ("CF", Group::St),
        ];
        for (code, group) in want {
            assert_eq!(Group::from_pff(code), Some(group), "{code}");
        }
        assert_eq!(Group::from_pff("XX"), None);
        // Every code of the fixture's roster has a group.
        let m = fixture().m;
        assert!(m.persons.iter().all(|p| p.group.is_some()));
    }

    /// The roster gives 11 starters a side and the substitutes; the events give the
    /// substitution, the sending-off and the two half-opening kick-offs.
    #[test]
    fn the_reader_gives_starters_the_substitution_and_the_sending_off() {
        let n = fixture();
        let m = &n.m;
        assert_eq!(m.source, "PFF");
        assert_eq!(m.match_id, "9001");
        assert_eq!(m.pitch, [105.0, 68.0]);
        assert_eq!(m.teams, ["900".to_string(), "901".into()]);
        assert_eq!(n.fps, 29.97);
        for team in 0..2 {
            let starters = m.persons.iter().filter(|p| p.team == team && p.starting);
            assert_eq!(starters.count(), 11);
        }
        assert_eq!(m.persons.len(), 24);
        assert_eq!(m.persons[person(m, "90012")].group, Some(Group::Cm));
        assert!(m.events.iter().any(|e| e.kind
            == EventKind::Substitution {
                on: "90012".into(),
                off: "90008".into()
            }));
        let red: Vec<_> = m
            .events
            .iter()
            .filter(|e| e.kind == EventKind::Card { sending_off: true })
            .collect();
        assert_eq!(red.len(), 1);
        assert_eq!(
            (red[0].person.as_deref(), red[0].team),
            (Some("90107"), Some(1))
        );
        let opening = m
            .events
            .iter()
            .filter(|e| e.kind == EventKind::KickOff { half_start: true })
            .count();
        assert_eq!(opening, 2);
        assert!(m.events.windows(2).all(|w| w[0].time_ms <= w[1].time_ms));
        assert_eq!(m.notes.home_starts_left, Some(true));
        assert_eq!(m.notes.ball_status_from.as_deref(), Some("events"));
    }

    /// Periods 1 and 2 become the two halves; the period-0 and period-3 frames are left out
    /// and counted; every frame line is counted.
    #[test]
    fn periods_zero_and_three_are_left_out_and_counted() {
        let n = fixture();
        assert_eq!(n.lines, 182);
        assert_eq!(n.m.halves.len(), 2);
        assert!(n.m.halves.iter().all(|h| h.frames.len() == 90));
        let left: Vec<(String, u32)> =
            n.m.notes
                .left_out
                .iter()
                .map(|l| (l.period.clone(), l.frames))
                .collect();
        assert_eq!(left, [("0".to_string(), 1), ("3".to_string(), 1)]);
        assert_eq!(n.m.halves[0].frames[0].n, 3000);
        assert_eq!(n.m.halves[1].frames[0].n, 12000);
        assert_eq!(n.times[1][0], 400_000.0);
        // Two frames of half 1 have no ball, and two of half 2.
        assert_eq!(n.ball_absent, 4);
        assert!(n.m.halves[0].frames[1].ball.is_none());
        assert!(n.m.halves[0].frames[2].ball.is_some());
    }

    /// 3 of the 3,914 kept player rows are estimated off camera.
    #[test]
    fn the_estimated_share_counts_the_kept_player_rows() {
        let n = fixture();
        assert_eq!(n.m.notes.estimated_share, Some(3.0 / 3914.0));
    }

    /// The compressed copy reads the same match as the plain file.
    #[test]
    fn the_compressed_file_reads_the_same_match() {
        let plain = fixture();
        let mut packed = read_native(&data(), "9002").unwrap();
        packed.m.match_id = plain.m.match_id.clone();
        assert_eq!(packed, plain);
    }

    /// A roster player who is never tracked has no position in any frame; a player missing
    /// from six frames has none in those frames only.
    #[test]
    fn a_player_missing_from_frames_has_no_positions_there() {
        let m = fixture().m;
        let unused = person(&m, "90013");
        assert!(
            m.halves
                .iter()
                .all(|h| h.positions[unused].iter().all(Option::is_none))
        );
        let gap = person(&m, "90106");
        let p = &m.halves[0].positions[gap];
        assert!(p[29].is_some() && p[36].is_some());
        assert!(p[30..36].iter().all(Option::is_none));
        // The 5 m/s player: x from -12 m at the half's first frame.
        let fast = person(&m, "90009");
        assert_eq!(m.halves[0].positions[fast][0], Some([-12.0, 5.0]));
    }
}

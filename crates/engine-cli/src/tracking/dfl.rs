//! The DFL tracking files (Bassek, Rein, Weber and Memmert, 2025): per match a match
//! information file (teams, players with their starting places and position codes, the
//! stadium's pitch), a raw event file and a raw position file of frame sets per half and
//! person, the ball's set carrying the ball status and the team in possession. Positions
//! are in metres, centred on the pitch, at 25 frames a second.
//!
//! The reader streams each file once and fills a [`TrackedMatch`]; it keeps the provider's
//! person ids and never a name.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

use super::time::epoch_ms;
use super::xml::{Tag, TagKind, read_tags};
use super::{Event, EventKind, Frame, Group, Half, Height, Person, TrackedMatch};

/// The provider as targets and side files name it.
pub const SOURCE: &str = "DFL";

/// The three files of one match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Files {
    pub match_id: String,
    pub info: PathBuf,
    pub events: PathBuf,
    pub positions: PathBuf,
}

/// The folder that holds the DFL files: `<data>/idsse` when it exists, else `data` itself.
pub fn folder(data: &Path) -> PathBuf {
    let sub = data.join("idsse");
    if sub.is_dir() {
        sub
    } else {
        data.to_path_buf()
    }
}

/// Every match of `data` with all three files, by match id.
pub fn matches(data: &Path) -> anyhow::Result<Vec<Files>> {
    let dir = folder(data);
    let mut by_id: BTreeMap<String, [Option<PathBuf>; 3]> = BTreeMap::new();
    let entries =
        std::fs::read_dir(&dir).with_context(|| format!("cannot list {}", dir.display()))?;
    for e in entries {
        let path = e?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(stem) = name.strip_suffix(".xml") else {
            continue;
        };
        let which = if stem.starts_with("DFL_02_01_matchinformation_") {
            0
        } else if stem.starts_with("DFL_03_02_events_raw_") {
            1
        } else if stem.starts_with("DFL_04_03_positions_raw_observed_") {
            2
        } else {
            continue;
        };
        let Some(at) = stem.find("DFL-MAT-") else {
            continue;
        };
        by_id.entry(stem[at..].to_string()).or_default()[which] = Some(path.clone());
    }
    Ok(by_id
        .into_iter()
        .filter_map(|(id, [i, e, p])| {
            Some(Files {
                match_id: id,
                info: i?,
                events: e?,
                positions: p?,
            })
        })
        .collect())
}

/// The files of the match `id` in `data`.
pub fn find(data: &Path, id: &str) -> anyhow::Result<Files> {
    matches(data)?
        .into_iter()
        .find(|f| f.match_id == id)
        .with_context(|| {
            format!(
                "no DFL match {id} with its match information, event and position files in {}",
                folder(data).display()
            )
        })
}

/// Reads one match.
pub fn read(files: &Files) -> anyhow::Result<TrackedMatch> {
    let info = read_info(&files.info)?;
    let events = read_events(&files.events, &info)?;
    let halves = read_positions(&files.positions, &info)?;
    Ok(TrackedMatch {
        source: SOURCE.into(),
        match_id: info.match_id,
        pitch: info.pitch,
        teams: info.teams,
        persons: info.persons,
        halves,
        events,
        ticks_per_frame: 2,
        resampled_from: None,
        notes: super::Notes::default(),
    })
}

/// What the match information file gives.
#[derive(Debug, Clone, PartialEq)]
pub struct Info {
    pub match_id: String,
    pub pitch: [f64; 2],
    pub teams: [String; 2],
    pub persons: Vec<Person>,
}

impl Info {
    fn team_of(&self, id: &str) -> Option<usize> {
        self.teams.iter().position(|t| t == id)
    }

    fn person(&self, id: &str) -> Option<usize> {
        self.persons.iter().position(|p| p.id == id)
    }
}

fn open(path: &Path) -> anyhow::Result<BufReader<File>> {
    Ok(BufReader::with_capacity(
        1 << 20,
        File::open(path).with_context(|| format!("cannot open {}", path.display()))?,
    ))
}

fn need<'a>(tag: &'a Tag<'_>, key: &str) -> Result<&'a str, String> {
    tag.attr(key)
        .ok_or_else(|| format!("line {}: <{}> has no {key}", tag.line, tag.name))
}

fn number(tag: &Tag<'_>, key: &str) -> Result<f64, String> {
    let v = need(tag, key)?;
    v.parse::<f64>()
        .map_err(|_| format!("line {}: {key}=\"{v}\" is not a number", tag.line))
}

/// Reads the match information file.
pub fn read_info(path: &Path) -> anyhow::Result<Info> {
    let mut match_id = None;
    let mut home = None;
    let mut guest = None;
    let mut pitch = None;
    let mut team: Option<String> = None;
    let mut players: Vec<(String, Person)> = Vec::new();
    read_tags(open(path)?, |t| {
        if t.kind == TagKind::End {
            if t.name == "Team" {
                team = None;
            }
            return Ok(());
        }
        match t.name {
            "General" => {
                match_id = Some(need(t, "MatchId")?.to_string());
                home = Some(need(t, "HomeTeamId")?.to_string());
                guest = Some(need(t, "GuestTeamId")?.to_string());
            }
            "Environment" => pitch = Some([number(t, "PitchX")?, number(t, "PitchY")?]),
            "Team" => team = Some(need(t, "TeamId")?.to_string()),
            "Player" => {
                let Some(club) = &team else {
                    return Err(format!("line {}: a player outside a team", t.line));
                };
                let position = t.attr("PlayingPosition").map(str::to_string);
                players.push((
                    club.clone(),
                    Person {
                        id: need(t, "PersonId")?.to_string(),
                        team: 0,
                        starting: t.attr("Starting") == Some("true"),
                        group: position.as_deref().and_then(Group::from_dfl),
                        position,
                    },
                ));
            }
            _ => {}
        }
        Ok(())
    })
    .map_err(anyhow::Error::msg)
    .with_context(|| format!("cannot read {}", path.display()))?;
    let what = || format!("{} is not a DFL match information file", path.display());
    let (Some(match_id), Some(home), Some(guest), Some(pitch)) = (match_id, home, guest, pitch)
    else {
        bail!("{}: it has no General or Environment element", what());
    };
    let teams = [home, guest];
    let mut persons = Vec::with_capacity(players.len());
    for (club, mut p) in players {
        p.team = teams
            .iter()
            .position(|t| *t == club)
            .with_context(|| format!("{}: player {} plays for no team", what(), p.id))?;
        persons.push(p);
    }
    Ok(Info {
        match_id,
        pitch,
        teams,
        persons,
    })
}

/// One event as it is read: its kind, the deepest facts its children add.
#[derive(Default)]
struct Open {
    time_ms: Option<i64>,
    top: Option<String>,
    team: Option<String>,
    person: Option<String>,
    recipient: Option<String>,
    decision_ms: Option<i64>,
    height: Option<Height>,
    cross: bool,
    goal: bool,
    half_start: bool,
    sending_off: bool,
    sub: Option<(String, String)>,
}

/// Reads the raw event file: each `<Event>` is one event, its first child names it.
pub fn read_events(path: &Path, info: &Info) -> anyhow::Result<Vec<Event>> {
    let mut out: Vec<Event> = Vec::new();
    let mut current: Option<Open> = None;
    let mut depth = 0usize;
    read_tags(open(path)?, |t| {
        match t.kind {
            TagKind::Start | TagKind::Empty if t.name == "Event" => {
                let time_ms = t.attr("EventTime").map(epoch_ms).transpose()?;
                current = Some(Open {
                    time_ms,
                    ..Open::default()
                });
                depth = 0;
                if t.kind == TagKind::Empty {
                    close(current.take(), info, &mut out);
                }
            }
            TagKind::End if t.name == "Event" => close(current.take(), info, &mut out),
            TagKind::End => depth = depth.saturating_sub(1),
            _ => {
                let Some(ev) = current.as_mut() else {
                    return Ok(());
                };
                child(ev, t, depth)?;
                if t.kind == TagKind::Start {
                    depth += 1;
                }
            }
        }
        Ok(())
    })
    .map_err(anyhow::Error::msg)
    .with_context(|| format!("cannot read {}", path.display()))?;
    // Time order; an event without a time keeps its place after the timed ones before it.
    let mut last = i64::MIN;
    let mut keyed: Vec<(i64, usize, Event)> = out
        .into_iter()
        .enumerate()
        .map(|(i, e)| {
            if let Some(t) = e.time_ms {
                last = t;
            }
            (e.time_ms.unwrap_or(last), i, e)
        })
        .collect();
    keyed.sort_by_key(|(t, i, _)| (*t, *i));
    Ok(keyed.into_iter().map(|(_, _, e)| e).collect())
}

/// One child tag of an event at `depth` (0: the event's own kind).
fn child(ev: &mut Open, t: &Tag<'_>, depth: usize) -> Result<(), String> {
    if depth == 0 && ev.top.is_none() {
        ev.top = Some(t.name.to_string());
        ev.decision_ms = t.attr("DecisionTimestamp").map(epoch_ms).transpose()?;
        if t.name == "KickOff" && t.attr("GameSection").is_some() {
            ev.half_start = true;
        }
    }
    if ev.team.is_none() {
        ev.team = t.attr("Team").map(str::to_string);
    }
    if ev.person.is_none() {
        ev.person = t.attr("Player").map(str::to_string);
    }
    match t.name {
        "Play" => {
            if ev.recipient.is_none() {
                ev.recipient = t.attr("Recipient").map(str::to_string);
            }
            ev.height = match t.attr("Height") {
                Some("high") => Some(Height::High),
                Some("flat") => Some(Height::Ground),
                _ => ev.height,
            };
        }
        "Cross" => ev.cross = true,
        "SuccessfulShot" => ev.goal = true,
        "Caution" => {
            ev.sending_off = matches!(t.attr("CardColor"), Some("red" | "yellowRed"));
        }
        "Substitution" => {
            ev.sub = Some((need(t, "PlayerIn")?.into(), need(t, "PlayerOut")?.into()));
        }
        _ => {}
    }
    Ok(())
}

/// Turns an event read in full into one or two events: a scored penalty is the restart and
/// then the goal.
fn close(ev: Option<Open>, info: &Info, out: &mut Vec<Event>) {
    let Some(ev) = ev else { return };
    let Some(top) = ev.top.as_deref() else { return };
    let kind = match top {
        "Play" if ev.cross => EventKind::Cross,
        "Play" => EventKind::Pass { height: ev.height },
        "ShotAtGoal" => EventKind::Shot { goal: ev.goal },
        "ThrowIn" => EventKind::ThrowIn,
        "FreeKick" => EventKind::FreeKick,
        "GoalKick" => EventKind::GoalKick,
        "CornerKick" => EventKind::Corner,
        "KickOff" => EventKind::KickOff {
            half_start: ev.half_start,
        },
        "Penalty" => EventKind::Penalty,
        "Foul" => EventKind::Foul,
        "Offside" => EventKind::Offside,
        "Caution" => EventKind::Card {
            sending_off: ev.sending_off,
        },
        "Substitution" => match &ev.sub {
            Some((on, off)) => EventKind::Substitution {
                on: on.clone(),
                off: off.clone(),
            },
            None => EventKind::Other(top.into()),
        },
        "FinalWhistle" => EventKind::FinalWhistle,
        "RefereeBall" => EventKind::DroppedBall,
        other => EventKind::Other(other.into()),
    };
    let team = ev.team.as_deref().and_then(|t| info.team_of(t));
    let base = Event {
        kind,
        team,
        person: ev.person.clone(),
        time_ms: ev.time_ms,
        clock: None,
        decision_ms: ev.decision_ms,
        recipient: ev.recipient.clone(),
    };
    let scored_penalty = top == "Penalty" && ev.goal;
    out.push(base.clone());
    if scored_penalty {
        out.push(Event {
            kind: EventKind::Shot { goal: true },
            decision_ms: None,
            recipient: None,
            ..base
        });
    }
}

/// One half's frames as they are read: by frame number from the half's first.
#[derive(Default)]
struct HalfBuild {
    first: Option<u32>,
    times: Vec<i64>,
    ball: Vec<Option<[f32; 3]>>,
    live: Vec<Option<bool>>,
    possession: Vec<Option<usize>>,
    positions: BTreeMap<usize, Vec<Option<[f32; 2]>>>,
}

impl HalfBuild {
    /// The index of frame `n`, growing the arrays to hold it. The first frame number seen
    /// fixes index 0; an earlier one shifts every array.
    fn at(&mut self, n: u32) -> usize {
        let first = *self.first.get_or_insert(n);
        if n < first {
            let shift = (first - n) as usize;
            self.first = Some(n);
            self.times
                .splice(0..0, std::iter::repeat_n(i64::MIN, shift));
            self.ball.splice(0..0, std::iter::repeat_n(None, shift));
            self.live.splice(0..0, std::iter::repeat_n(None, shift));
            self.possession
                .splice(0..0, std::iter::repeat_n(None, shift));
            for p in self.positions.values_mut() {
                p.splice(0..0, std::iter::repeat_n(None, shift));
            }
        }
        let i = (n - self.first.unwrap_or(n)) as usize;
        if i >= self.times.len() {
            self.times.resize(i + 1, i64::MIN);
            self.ball.resize(i + 1, None);
            self.live.resize(i + 1, None);
            self.possession.resize(i + 1, None);
        }
        i
    }
}

/// Reads the raw position file: the ball's set gives the ball, its status and possession;
/// each player's set his positions; the referees' sets are skipped.
pub fn read_positions(path: &Path, info: &Info) -> anyhow::Result<Vec<Half>> {
    let mut halves: Vec<(String, HalfBuild)> = Vec::new();
    // The set being read: its half, and the ball or the person's place in `info`.
    let mut set: Option<(usize, Option<usize>)> = None;
    let mut ball_set = false;
    read_tags(open(path)?, |t| {
        match (t.kind, t.name) {
            (TagKind::Start, "FrameSet") => {
                let section = need(t, "GameSection")?;
                let h = match halves.iter().position(|(s, _)| s == section) {
                    Some(h) => h,
                    None => {
                        halves.push((section.to_string(), HalfBuild::default()));
                        halves.len() - 1
                    }
                };
                let team = need(t, "TeamId")?;
                ball_set = team == "BALL";
                let who = if ball_set {
                    None
                } else {
                    match info.person(need(t, "PersonId")?) {
                        Some(p) if info.team_of(team).is_some() => Some(p),
                        // A referee or a person of no team: skipped.
                        _ => {
                            set = None;
                            return Ok(());
                        }
                    }
                };
                set = Some((h, who));
            }
            (TagKind::End, "FrameSet") => set = None,
            (TagKind::Empty | TagKind::Start, "Frame") => {
                let Some((h, who)) = set else {
                    return Ok(());
                };
                let n: u32 = need(t, "N")?
                    .parse()
                    .map_err(|_| format!("line {}: N is not a frame number", t.line))?;
                let time = epoch_ms(need(t, "T")?).map_err(|e| format!("line {}: {e}", t.line))?;
                let x = number(t, "X")? as f32;
                let y = number(t, "Y")? as f32;
                let b = &mut halves[h].1;
                let i = b.at(n);
                b.times[i] = time;
                match who {
                    None if ball_set => {
                        let z = t
                            .attr("Z")
                            .and_then(|z| z.parse::<f32>().ok())
                            .unwrap_or(0.0);
                        b.ball[i] = Some([x, y, z]);
                        b.live[i] = t.attr("BallStatus").map(|s| s == "1");
                        b.possession[i] = match t.attr("BallPossession") {
                            Some("1") => Some(0),
                            Some("2") => Some(1),
                            _ => None,
                        };
                    }
                    None => {}
                    Some(p) => {
                        let len = b.times.len();
                        let track = b.positions.entry(p).or_insert_with(|| vec![None; len]);
                        if track.len() < len {
                            track.resize(len, None);
                        }
                        track[i] = Some([x, y]);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    })
    .map_err(anyhow::Error::msg)
    .with_context(|| format!("cannot read {}", path.display()))?;
    let mut out = Vec::with_capacity(halves.len());
    for (section, b) in halves {
        let first = b.first.unwrap_or(0);
        let len = b.times.len();
        if let Some(i) = b.times.iter().position(|t| *t == i64::MIN) {
            bail!(
                "{}: frame {} of {section} has no time in any frame set",
                path.display(),
                first + i as u32
            );
        }
        let frames = (0..len)
            .map(|i| Frame {
                n: first + i as u32,
                t_ms: b.times[i],
                ball: b.ball[i],
                live: b.live[i],
                possession: b.possession[i],
            })
            .collect();
        let mut positions = vec![vec![None; len]; info.persons.len()];
        for (p, mut track) in b.positions {
            track.resize(len, None);
            positions[p] = track;
        }
        out.push(Half {
            section,
            frames,
            positions,
        });
    }
    // Halves in time order.
    out.sort_by_key(|h| h.frames.first().map_or(i64::MAX, |f| f.t_ms));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> TrackedMatch {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracking/dfl");
        read(&find(&data, "DFL-MAT-FIX001").unwrap()).unwrap()
    }

    fn person(m: &TrackedMatch, id: &str) -> usize {
        m.persons.iter().position(|p| p.id == id).unwrap()
    }

    #[test]
    fn the_match_information_gives_teams_persons_and_the_pitch() {
        let m = fixture();
        assert_eq!(m.source, "DFL");
        assert_eq!(m.match_id, "DFL-MAT-FIX001");
        assert_eq!(m.pitch, [104.0, 67.0]);
        assert_eq!(m.teams, ["DFL-CLU-FIXH".to_string(), "DFL-CLU-FIXA".into()]);
        let starters = |team| {
            m.persons
                .iter()
                .filter(|p| p.team == team && p.starting)
                .count()
        };
        assert_eq!((starters(0), starters(1)), (11, 11));
        let keeper = &m.persons[person(&m, "DFL-OBJ-FIXH01")];
        assert_eq!(
            (keeper.team, keeper.group, keeper.position.as_deref()),
            (0, Some(Group::Keeper), Some("TW"))
        );
        let sub = &m.persons[person(&m, "DFL-OBJ-FIXH12")];
        assert_eq!((sub.starting, sub.group), (false, Some(Group::W)));
        let bench = &m.persons[person(&m, "DFL-OBJ-FIXH13")];
        assert_eq!((bench.group, bench.position.as_deref()), (None, None));
    }

    #[test]
    fn the_positions_give_each_half_its_frames_ball_and_players() {
        let m = fixture();
        assert_eq!(m.halves.len(), 2);
        let (h1, h2) = (&m.halves[0], &m.halves[1]);
        assert_eq!(
            (h1.section.as_str(), h2.section.as_str()),
            ("firstHalf", "secondHalf")
        );
        assert_eq!((h1.frames.len(), h2.frames.len()), (60, 60));
        assert_eq!((h1.frames[0].n, h2.frames[0].n), (10000, 100000));
        assert_eq!(h1.frames[1].t_ms - h1.frames[0].t_ms, 40);
        // The ball's set of half 1 starts two frames after the players'.
        assert_eq!(
            (h1.frames[0].ball, h1.frames[1].ball.is_some()),
            (None, false)
        );
        assert_eq!(h1.frames[2].ball, Some([0.0, 0.0, 0.0]));
        assert_eq!(h1.frames[2].live, Some(true));
        assert_eq!(h1.frames[2].possession, Some(0));
        assert_eq!(h1.frames[45].live, Some(false));
        assert_eq!(h1.frames[30].possession, Some(1));
        // The 5-frame gap of H07, the referee skipped, the bench never tracked.
        let h07 = person(&m, "DFL-OBJ-FIXH07");
        let missing: Vec<u32> = h1
            .frames
            .iter()
            .zip(&h1.positions[h07])
            .filter(|(_, p)| p.is_none())
            .map(|(f, _)| f.n)
            .collect();
        assert_eq!(missing, (10020..=10024).collect::<Vec<u32>>());
        let h13 = person(&m, "DFL-OBJ-FIXH13");
        assert!(h1.positions[h13].iter().all(Option::is_none));
        // The home keeper defends the positive end in half 1 and the negative in half 2.
        let h01 = person(&m, "DFL-OBJ-FIXH01");
        assert_eq!(h1.positions[h01][0], Some([49.0, 0.0]));
        assert_eq!(h2.positions[h01][0], Some([-49.0, 0.0]));
        // The sent-off A04 is tracked to frame 100044, the substitute H12 from 100024.
        let a04 = person(&m, "DFL-OBJ-FIXA04");
        assert!(h2.positions[a04][44].is_some() && h2.positions[a04][45].is_none());
        let h12 = person(&m, "DFL-OBJ-FIXH12");
        assert!(h2.positions[h12][23].is_none() && h2.positions[h12][24].is_some());
    }

    #[test]
    fn the_events_give_kinds_teams_times_and_restart_decisions() {
        let m = fixture();
        let kinds: Vec<&EventKind> = m.events.iter().map(|e| &e.kind).collect();
        assert_eq!(kinds[0], &EventKind::KickOff { half_start: true });
        assert_eq!(
            m.events[0].team,
            Some(0),
            "the kick-off's team is its play's"
        );
        assert!(kinds.contains(&&EventKind::Pass {
            height: Some(Height::High)
        }));
        assert!(kinds.contains(&&EventKind::Cross));
        assert!(kinds.contains(&&EventKind::Shot { goal: true }));
        assert!(kinds.contains(&&EventKind::KickOff { half_start: false }));
        assert!(kinds.contains(&&EventKind::Card { sending_off: true }));
        assert!(kinds.contains(&&EventKind::Substitution {
            on: "DFL-OBJ-FIXH12".into(),
            off: "DFL-OBJ-FIXH09".into()
        }));
        let throw = m
            .events
            .iter()
            .find(|e| e.kind == EventKind::ThrowIn)
            .unwrap();
        assert_eq!(throw.team, Some(1));
        let h1 = &m.halves[0];
        assert_eq!(throw.time_ms, Some(h1.frames[53].t_ms));
        assert_eq!(throw.decision_ms, Some(h1.frames[45].t_ms));
        // The event file writes +01:00; the instant is the frame's.
        let pass = &m.events[1];
        assert_eq!(pass.time_ms, Some(h1.frames[20].t_ms));
        assert_eq!(pass.recipient.as_deref(), Some("DFL-OBJ-FIXH09"));
        // One event has no event time and no clock.
        let untimed: Vec<&Event> = m.events.iter().filter(|e| e.time_ms.is_none()).collect();
        assert_eq!(untimed.len(), 1);
        assert_eq!(
            (untimed[0].kind.clone(), untimed[0].clock),
            (EventKind::Cross, None)
        );
        // Time order.
        let times: Vec<i64> = m.events.iter().filter_map(|e| e.time_ms).collect();
        assert!(times.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn a_missing_file_or_match_is_named() {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracking/dfl");
        let err = find(&data, "DFL-MAT-NONE").unwrap_err().to_string();
        assert!(err.contains("DFL-MAT-NONE"), "{err}");
        assert_eq!(matches(&data).unwrap().len(), 1);
    }
}

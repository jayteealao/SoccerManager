//! The converter: a tracked match into the engine's tick records and a side file, the tick
//! format unchanged.
//!
//! - Engine tick `base + 2k` holds frame `k` of a half, counted from the half's first frame
//!   with a ball position; half 1's base is 0, each later half's the previous half's last
//!   tick plus 2, as the engine's ticks run on across half time. The odd ticks get no record,
//!   and the side file says so.
//! - Team 0 is the home team. Each team's starters hold slots 0 to 10 in their starting
//!   order with the keeper in slot 0; a substitute takes the replaced player's slot from the
//!   substitution's tick. A slot whose player is not on the pitch (not tracked in a frame,
//!   sent off, or no player at all) holds the engine's parking spot for that slot on the
//!   stadium's pitch, and the side file lists the spell and why.
//! - Coordinates stay in metres as recorded, centred on the stadium's pitch. Each half's
//!   attack direction comes from the keepers' mean x; the whole match is turned by 180
//!   degrees when needed so team 0 attacks towards positive x in the first half, as in the
//!   engine, and a match whose direction does not change at half time is refused.
//! - Every event is placed on a frame ([super::place]); a restart sets the restart bit of
//!   its tick, and the side file lists each placed event and how it was placed.

use std::path::{Path, PathBuf};

use anyhow::{Context, bail, ensure};
use engine::pitch::Pitch;
use engine::record::{FileSink, PLAYER_COUNT, TickHeader, TickRecord, TickSink};
use engine::team::PLAYERS_PER_TEAM;

use super::place;
use super::side::{BallRun, OffPitch, Placed, SCHEMA, Side, SideHalf, Spell};
use super::{EventKind, Group, TrackedMatch};

/// Engine ticks between two of the report's frames, which a side file names: the report
/// reads 25 frames a second, every second engine tick.
pub const TICK_STEP: u32 = 2;
/// The report's frame rate, which a side file names.
pub const FRAME_RATE: u32 = 25;

/// The credit of a source's data, for the side file.
pub fn credit(source: &str) -> &'static str {
    match source {
        "DFL" => {
            "DFL tracking and event data, Bassek, Rein, Weber and Memmert (2025), \
             figshare doi 10.6084/m9.figshare.28196177.v1, CC BY 4.0"
        }
        _ => "",
    }
}

/// The ticks of one half's used frames.
#[derive(Debug, Clone, Copy)]
struct HalfTicks {
    /// The index of the half's first ball frame.
    first: usize,
    /// Its tick.
    base: u32,
    /// Frames used: from `first` to the half's last frame.
    count: usize,
    /// Engine ticks between two frames: the match's ticks per frame.
    step: u32,
}

impl HalfTicks {
    fn tick(&self, k: usize) -> u32 {
        self.base + self.step * k as u32
    }

    fn end(&self) -> u32 {
        self.tick(self.count.saturating_sub(1))
    }
}

/// Every used frame of the match as (time, tick, half, frame index), in time order: what an
/// event is placed against.
pub fn frame_ticks(m: &TrackedMatch) -> anyhow::Result<Vec<(i64, u32, usize, usize)>> {
    let halves = half_ticks(m)?;
    let mut out = Vec::new();
    for (h, ht) in halves.iter().enumerate() {
        for k in 0..ht.count {
            let i = ht.first + k;
            out.push((m.halves[h].frames[i].t_ms, ht.tick(k), h, i));
        }
    }
    Ok(out)
}

fn half_ticks(m: &TrackedMatch) -> anyhow::Result<Vec<HalfTicks>> {
    ensure!(!m.halves.is_empty(), "match {} has no half", m.match_id);
    ensure!(
        matches!(m.ticks_per_frame, 1 | 2),
        "match {}: {} ticks per frame; the converter takes 1 (frames at 50 a second) or 2          (25 a second)",
        m.match_id,
        m.ticks_per_frame
    );
    let mut out: Vec<HalfTicks> = Vec::with_capacity(m.halves.len());
    for half in &m.halves {
        let first = half
            .frames
            .iter()
            .position(|f| f.ball.is_some())
            .with_context(|| {
                format!(
                    "match {}: half {} has no frame with a ball position",
                    m.match_id, half.section
                )
            })?;
        let step = m.ticks_per_frame;
        let base = out.last().map_or(0, |p| p.end() + step);
        out.push(HalfTicks {
            first,
            base,
            count: half.frames.len() - first,
            step,
        });
    }
    Ok(out)
}

/// The nearest used frame to `t_ms`, the earlier on a tie: (tick, half, frame index).
pub fn nearest(frames: &[(i64, u32, usize, usize)], t_ms: i64) -> Option<(u32, usize, usize)> {
    let at = frames.partition_point(|f| f.0 < t_ms);
    let pick = match (at.checked_sub(1).map(|i| frames[i]), frames.get(at)) {
        (Some(a), Some(b)) => {
            if t_ms - a.0 <= b.0 - t_ms {
                a
            } else {
                *b
            }
        }
        (Some(a), None) => a,
        (None, Some(b)) => *b,
        (None, None) => return None,
    };
    Some((pick.1, pick.2, pick.3))
}

/// Team 0's attack direction in each half, in the source's coordinates: -1 when its keeper
/// stands on the positive end, +1 otherwise; from the away keeper when the home one has no
/// frame in the half.
fn directions(m: &TrackedMatch, ticks: &[HalfTicks]) -> anyhow::Result<Vec<f64>> {
    let keeper = |team: usize| {
        m.persons
            .iter()
            .position(|p| p.team == team && p.starting && p.group == Some(Group::Keeper))
    };
    let mean_x = |h: usize, p: usize| {
        let ht = ticks[h];
        let (s, n) = m.halves[h].positions[p][ht.first..]
            .iter()
            .flatten()
            .fold((0.0f64, 0u32), |(s, n), xy| (s + f64::from(xy[0]), n + 1));
        (n > 0).then(|| s / f64::from(n))
    };
    let mut out = Vec::with_capacity(ticks.len());
    for h in 0..ticks.len() {
        let home = keeper(0).and_then(|p| mean_x(h, p));
        let away = keeper(1).and_then(|p| mean_x(h, p));
        let dir = match (home, away) {
            (Some(x), _) => {
                if x > 0.0 {
                    -1.0
                } else {
                    1.0
                }
            }
            (None, Some(x)) => {
                if x > 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
            (None, None) => bail!(
                "match {}: no keeper is tracked in half {}, so its play direction is unknown",
                m.match_id,
                m.halves[h].section
            ),
        };
        if let Some(prev) = out.last()
            && *prev == dir
        {
            bail!(
                "match {}: the play direction does not change at half time (team 0 attacks \
                 the same end in {} and {}); the converter refuses it",
                m.match_id,
                m.halves[h - 1].section,
                m.halves[h].section
            );
        }
        out.push(dir);
    }
    Ok(out)
}

/// What a slot holds at one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Holder {
    Person(usize),
    Empty,
}

/// The converted match.
#[derive(Debug, Clone)]
pub struct Converted {
    pub records: Vec<TickRecord>,
    pub side: Side,
}

/// Converts `m` into tick records and its side file.
pub fn to_ticks(m: &TrackedMatch) -> anyhow::Result<Converted> {
    let ticks = half_ticks(m)?;
    let dirs = directions(m, &ticks)?;
    let rotate = dirs[0] < 0.0;
    let turn = |v: f32| if rotate { -v } else { v };
    let pitch = Pitch::new(m.pitch[0], m.pitch[1])
        .map_err(|e| anyhow::anyhow!("match {}: the pitch is refused: {e}", m.match_id))?;
    let frames = frame_ticks(m)?;

    // Each slot's timeline, (from tick, holder), starts with its starter: the starters in
    // order, the keeper first. Slot `s` of team `t` is record place `t * 11 + s`.
    let mut timeline: Vec<Vec<(u32, Holder)>> = Vec::with_capacity(PLAYER_COUNT);
    for team in 0..2 {
        let mut starters: Vec<usize> = (0..m.persons.len())
            .filter(|&p| m.persons[p].team == team && m.persons[p].starting)
            .collect();
        ensure!(
            starters.len() <= PLAYERS_PER_TEAM,
            "match {}: team {team} has {} starters",
            m.match_id,
            starters.len()
        );
        if let Some(k) = starters
            .iter()
            .position(|&p| m.persons[p].group == Some(Group::Keeper))
        {
            let keeper = starters.remove(k);
            starters.insert(0, keeper);
        }
        for s in 0..PLAYERS_PER_TEAM {
            let holder = starters
                .get(s)
                .map_or(Holder::Empty, |&p| Holder::Person(p));
            timeline.push(vec![(0, holder)]);
        }
    }
    let person_of = |id: &str| m.persons.iter().position(|p| p.id == id);
    let mut sent_off: Vec<(usize, u32)> = Vec::new();
    let mut restarts: Vec<u32> = Vec::new();
    let offset = place::fit_offset(m, &frames);
    let mut placed: Vec<Placed> = Vec::new();
    let mut unplaced = 0u32;
    for e in &m.events {
        let Some(at) = place::place(&frames, offset.as_ref(), e) else {
            unplaced += 1;
            continue;
        };
        let tick = at.tick;
        placed.push(Placed {
            tick,
            kind: e.kind.word().into(),
            team: e.team,
            placed: at.how.word().into(),
        });
        match &e.kind {
            EventKind::Substitution { on, off } => {
                let (Some(on), Some(off)) = (person_of(on), person_of(off)) else {
                    bail!(
                        "match {}: a substitution names a person who is not in the match \
                         information",
                        m.match_id
                    );
                };
                let place = timeline
                    .iter()
                    .position(|tl| tl.last().map(|x| x.1) == Some(Holder::Person(off)))
                    .with_context(|| {
                        format!(
                            "match {}: the substituted player {} holds no slot",
                            m.match_id, m.persons[off].id
                        )
                    })?;
                timeline[place].push((tick, Holder::Person(on)));
            }
            EventKind::Card { sending_off: true } => {
                if let Some(p) = e.person.as_deref().and_then(person_of) {
                    sent_off.push((p, tick));
                }
            }
            EventKind::ThrowIn
            | EventKind::FreeKick
            | EventKind::GoalKick
            | EventKind::Corner
            | EventKind::KickOff { .. }
            | EventKind::Penalty
            | EventKind::DroppedBall => restarts.push(tick),
            _ => {}
        }
    }
    let holder_at = |i: usize, tick: u32| {
        timeline[i]
            .iter()
            .rev()
            .find(|(from, _)| *from <= tick)
            .map_or(Holder::Empty, |x| x.1)
    };
    let group_of = |p: usize, slot_index: usize| -> anyhow::Result<Group> {
        if let Some(g) = m.persons[p].group {
            return Ok(g);
        }
        // A substitute without a position code plays where the man he replaced did.
        timeline[slot_index]
            .iter()
            .filter_map(|(_, h)| match h {
                Holder::Person(q) => m.persons[*q].group,
                Holder::Empty => None,
            })
            .next()
            .with_context(|| {
                format!(
                    "match {}: player {} has no position group",
                    m.match_id, m.persons[p].id
                )
            })
    };

    let mut records = Vec::with_capacity(frames.len());
    let mut off: Vec<OffPitch> = Vec::new();
    let mut ball_runs: Vec<BallRun> = Vec::new();
    let mut ball_missing = Vec::new();
    let mut last_ball = [0.0f32; 3];
    for &(_, tick, h, fi) in &frames {
        let frame = &m.halves[h].frames[fi];
        let ball = match frame.ball {
            Some(b) => {
                last_ball = [turn(b[0]), turn(b[1]), b[2]];
                last_ball
            }
            None => {
                ball_missing.push(tick);
                last_ball
            }
        };
        let mut players = [[0.0f32; 2]; PLAYER_COUNT];
        for (i, xy) in players.iter_mut().enumerate() {
            let (team, slot) = (i / PLAYERS_PER_TEAM, i % PLAYERS_PER_TEAM);
            let holder = holder_at(i, tick);
            let at = match holder {
                Holder::Person(p) => {
                    let gone = sent_off.iter().any(|&(q, t)| q == p && t <= tick);
                    match (gone, m.halves[h].positions[p][fi]) {
                        (false, Some(pos)) => Ok([turn(pos[0]), turn(pos[1])]),
                        (true, _) => Err((Some(p), "sent off")),
                        (false, None) => Err((Some(p), "not tracked")),
                    }
                }
                Holder::Empty => Err((None, "no player")),
            };
            *xy = match at {
                Ok(pos) => pos,
                Err((p, reason)) => {
                    let person = p.map(|p| m.persons[p].id.clone());
                    extend_off(
                        &mut off,
                        team,
                        slot,
                        person,
                        (tick, m.ticks_per_frame),
                        reason,
                    );
                    let spot = pitch.parking_spot(team, slot);
                    [spot.x as f32, spot.y as f32]
                }
            };
        }
        let run = BallRun {
            from_tick: tick,
            to_tick: tick,
            live: frame.live,
            possession: frame.possession,
        };
        match ball_runs.last_mut() {
            Some(r) if r.live == run.live && r.possession == run.possession => r.to_tick = tick,
            _ => ball_runs.push(run),
        }
        records.push(TickRecord {
            tick,
            ball,
            players,
            restart: restarts.contains(&tick),
        });
    }

    let end = frames.last().map_or(0, |f| f.1);
    let mut spells = Vec::new();
    for (i, tl) in timeline.iter().enumerate() {
        let (team, slot) = (i / PLAYERS_PER_TEAM, i % PLAYERS_PER_TEAM);
        for (j, (from, holder)) in tl.iter().enumerate() {
            let to = tl
                .get(j + 1)
                .map_or(end, |next| next.0.saturating_sub(m.ticks_per_frame));
            if let Holder::Person(p) = holder {
                spells.push(Spell {
                    team,
                    slot,
                    person: m.persons[*p].id.clone(),
                    group: group_of(*p, i)?,
                    from_tick: *from,
                    to_tick: to,
                });
            }
        }
    }
    let side = Side {
        schema: SCHEMA,
        source: m.source.clone(),
        credit: credit(&m.source).into(),
        match_id: m.match_id.clone(),
        pitch: m.pitch,
        frame_rate: FRAME_RATE,
        tick_step: TICK_STEP,
        odd_ticks: "absent".into(),
        rotated: rotate,
        halves: m
            .halves
            .iter()
            .zip(&ticks)
            .zip(&dirs)
            .map(|((half, ht), dir)| SideHalf {
                section: half.section.clone(),
                first_frame: half.frames[ht.first].n,
                start_tick: ht.base,
                end_tick: ht.end(),
                team0_attack: if rotate { -dir } else { *dir },
            })
            .collect(),
        slots: spells,
        off_pitch: off,
        ball: ball_runs,
        ball_missing,
        events: placed,
        unplaced,
        clock_offset: offset,
    };
    Ok(Converted { records, side })
}

/// Adds `tick` to the slot's open off-pitch spell for `person` and `reason` when it follows
/// the spell's last tick by `step`, or opens one.
fn extend_off(
    off: &mut Vec<OffPitch>,
    team: usize,
    slot: usize,
    person: Option<String>,
    (tick, step): (u32, u32),
    reason: &str,
) {
    if let Some(s) = off
        .iter_mut()
        .rev()
        .find(|s| s.team == team && s.slot == slot)
        && s.person == person
        && s.reason == reason
        && s.to_tick + step == tick
    {
        s.to_tick = tick;
        return;
    }
    off.push(OffPitch {
        team,
        slot,
        person,
        from_tick: tick,
        to_tick: tick,
        reason: reason.into(),
    });
}

/// Writes the converted match as `<out>/<match>.ticks` and `<out>/<match>.side.json`, and
/// returns both paths.
pub fn write(c: &Converted, out: &Path) -> anyhow::Result<(PathBuf, PathBuf)> {
    std::fs::create_dir_all(out).with_context(|| format!("cannot create {}", out.display()))?;
    let ticks = out.join(format!("{}.ticks", c.side.match_id));
    let side = out.join(Side::file_name(&c.side.match_id));
    let header = TickHeader {
        seed: 0,
        dt: 1.0 / f64::from(engine::TICKS_PER_SECOND),
        expected_ticks: u32::try_from(c.records.len()).context("too many frames")?,
        owner_id: [0; 16],
        match_millis: 0,
    };
    let mut sink = FileSink::create(&ticks, &header)
        .with_context(|| format!("cannot create {}", ticks.display()))?;
    for r in &c.records {
        sink.on_tick(r)?;
    }
    sink.finish()?;
    c.side.write(&side)?;
    Ok((ticks, side))
}

#[cfg(test)]
mod tests {
    use super::super::dfl;
    use super::*;

    fn fixture() -> TrackedMatch {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracking/dfl");
        dfl::read(&dfl::find(&data, "DFL-MAT-FIX001").unwrap()).unwrap()
    }

    /// The side file names how each event was placed: by its event time, or by its game
    /// clock plus the match's offset; an event with neither is counted, not placed.
    #[test]
    fn the_side_file_names_both_placements() {
        let mut m = fixture();
        let plain = to_ticks(&m).unwrap().side;
        assert_eq!(plain.unplaced, 1);
        assert!(plain.events.iter().all(|e| e.placed == "event-time"));
        let e = m.events.iter_mut().find(|e| e.time_ms.is_none()).unwrap();
        e.clock = Some((0, 400));
        let side = to_ticks(&m).unwrap().side;
        assert_eq!(side.unplaced, 0);
        let by_clock: Vec<&Placed> = side
            .events
            .iter()
            .filter(|e| e.placed == "clock-offset")
            .collect();
        assert_eq!(by_clock.len(), 1);
        assert_eq!((by_clock[0].tick, by_clock[0].kind.as_str()), (44, "cross"));
        assert_eq!(side.clock_offset.map(|o| o.ms), Some(480.0));
        let sub = side
            .events
            .iter()
            .find(|e| e.kind == "substitution")
            .unwrap();
        assert_eq!(
            (sub.tick, sub.team, sub.placed.as_str()),
            (156, Some(0), "event-time")
        );
    }
}

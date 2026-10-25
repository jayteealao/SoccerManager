//! Measuring a match from its tick file and side file with the shared shape report
//! ([`crate::report::tracked_shape`]): a converted real match, or an engine match written
//! with a side file, read the same way.
//!
//! A record is a frame when its tick lies in a half of the side file at an even step from
//! the half's start tick. Its players are the slot holders of the side file, less the
//! off-pitch spells; the ball status and possession come from the ball runs, the ball is
//! absent on the ticks the side file lists, and the restarts are the placed restart events
//! of the tick. A kick-off a source places on a half's start tick opens the half and is no
//! stoppage; an engine side file lists only the stoppages it counted.
//!
//! `engine-cli tracking measure` measures every DFL match of the data folder this way and
//! writes their figures, frames, ball-in-play minutes per half, clock offsets and converter
//! checks into one file, `gate/tracking/dfl-matches.json`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use engine::record::TickRecord;
use serde::{Deserialize, Serialize};

use super::side::{ClockOffset, Side};
use crate::report::tracked_shape::{
    FRAMES_PER_SECOND, Frame, Holder, Mode, Restart, TrackedShape, TrackedValues, field_names,
};

const ROSTER: usize = 2 * engine::team::PLAYERS_PER_TEAM;
const PER_TEAM: usize = engine::team::PLAYERS_PER_TEAM;

/// How an engine side file marks the stoppages it lists.
pub const ENGINE_PLACED: &str = "engine-stoppage";

/// The person keys of a side file: a number keeps its value (an engine side file's keys),
/// any other id takes the next free key in order of first appearance.
#[derive(Debug, Default)]
struct Keys(BTreeMap<String, u32>);

impl Keys {
    fn of(&mut self, id: &str) -> u32 {
        if let Ok(n) = id.parse::<u32>() {
            return n;
        }
        let next = 1_000_000 + u32::try_from(self.0.len()).unwrap_or(u32::MAX);
        *self.0.entry(id.to_string()).or_insert(next)
    }
}

/// The frames of a match: every record at an even step from its half's start tick, with
/// what the side file adds.
pub fn frames_of(records: &[TickRecord], side: &Side) -> anyhow::Result<Vec<Frame>> {
    anyhow::ensure!(
        side.tick_step == 2,
        "the side file of {} has a tick step of {}; the report reads 2",
        side.match_id,
        side.tick_step
    );
    let mut keys = Keys::default();
    let spells: Vec<(usize, u32, u32, Holder)> = side
        .slots
        .iter()
        .map(|s| {
            (
                s.team * PER_TEAM + s.slot,
                s.from_tick,
                s.to_tick,
                Holder {
                    person: keys.of(&s.person),
                    group: s.group,
                },
            )
        })
        .collect();
    for s in &side.slots {
        anyhow::ensure!(
            s.team < 2 && s.slot < PER_TEAM,
            "the side file of {} names slot {} of team {}",
            side.match_id,
            s.slot,
            s.team
        );
    }
    let missing: std::collections::BTreeSet<u32> = side.ball_missing.iter().copied().collect();
    let mut events: BTreeMap<u32, Vec<(Restart, usize)>> = BTreeMap::new();
    let starts: Vec<u32> = side.halves.iter().map(|h| h.start_tick).collect();
    for e in &side.events {
        let (Some(r), Some(team)) = (Restart::of_word(&e.kind), e.team) else {
            continue;
        };
        if r == Restart::KickOff && e.placed != ENGINE_PLACED && starts.contains(&e.tick) {
            continue;
        }
        events.entry(e.tick).or_default().push((r, team));
    }
    let mut out = Vec::new();
    for r in records {
        let Some(half) = side
            .halves
            .iter()
            .position(|h| (h.start_tick..=h.end_tick).contains(&r.tick))
        else {
            continue;
        };
        let h = &side.halves[half];
        if !(r.tick - h.start_tick).is_multiple_of(2) {
            continue;
        }
        let t = r.tick;
        let mut on = [None; ROSTER];
        for &(place, from, to, holder) in &spells {
            if (from..=to).contains(&t) {
                on[place] = Some(holder);
            }
        }
        for o in &side.off_pitch {
            if (o.from_tick..=o.to_tick).contains(&t) && o.team < 2 && o.slot < PER_TEAM {
                on[o.team * PER_TEAM + o.slot] = None;
            }
        }
        let run = side
            .ball
            .iter()
            .find(|b| (b.from_tick..=b.to_tick).contains(&t));
        out.push(Frame {
            tick: t,
            half,
            ball: (!missing.contains(&t)).then_some(r.ball),
            players: r.players,
            on,
            live: run.and_then(|b| b.live),
            possession: run.and_then(|b| b.possession),
            restarts: events.remove(&t).unwrap_or_default(),
            attack_x: [h.team0_attack, -h.team0_attack],
        });
    }
    Ok(out)
}

/// The figures of the frames, with `mode`.
pub fn measure_frames(frames: &[Frame], length: f64, mode: Mode) -> anyhow::Result<TrackedValues> {
    let mut report = TrackedShape::new(mode, length);
    for f in frames {
        report.frame(f).map_err(anyhow::Error::msg)?;
    }
    Ok(report.finish())
}

/// The records of a tick file.
pub fn read_records(ticks: &Path) -> anyhow::Result<Vec<TickRecord>> {
    Ok(engine::read_ticks(ticks)
        .with_context(|| format!("cannot read {}", ticks.display()))?
        .records)
}

/// The figures of the match in `ticks` with its side file `side`.
pub fn measure_files(ticks: &Path, side: &Path) -> anyhow::Result<TrackedValues> {
    let side = Side::read(side)?;
    let frames = frames_of(&read_records(ticks)?, &side)?;
    measure_frames(&frames, side.pitch[0], Mode::Full)
}

/// The figures of one match by name, home and away.
pub fn named(v: &TrackedValues) -> BTreeMap<String, [f64; 2]> {
    field_names()
        .into_iter()
        .enumerate()
        .map(|(i, n)| (n, [v[0][i], v[1][i]]))
        .collect()
}

/// One measured match of the tracking file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchFigures {
    pub match_id: String,
    /// Frames measured.
    pub frames: u32,
    /// Ball-in-play minutes of each half.
    pub live_minutes: Vec<f64>,
    pub clock_offset: Option<ClockOffset>,
    /// What the converter reported.
    pub checks: Checks,
    /// Every figure of the shared report, home and away.
    pub figures: BTreeMap<String, [f64; 2]>,
}

/// The converter's checks of one match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checks {
    pub records: u32,
    pub rotated: bool,
    pub off_pitch_spells: u32,
    pub ball_missing: u32,
    pub events_placed: u32,
    pub events_unplaced: u32,
}

/// The tracking file: every measured match of one source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackingFile {
    pub schema: u32,
    pub source: String,
    pub credit: String,
    /// The report's frame rate.
    pub frame_rate: u32,
    pub matches: Vec<MatchFigures>,
}

/// The schema of the tracking file.
pub const TRACKING_SCHEMA: u32 = 1;

impl TrackingFile {
    pub fn read(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let file: Self = serde_json::from_str(&text)
            .with_context(|| format!("{} is not a tracking file", path.display()))?;
        anyhow::ensure!(
            file.schema == TRACKING_SCHEMA,
            "the tracking file {} has schema {}; this program reads {TRACKING_SCHEMA}",
            path.display(),
            file.schema
        );
        Ok(file)
    }

    pub fn write(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("cannot create {}", dir.display()))?;
        }
        let text = serde_json::to_string_pretty(self)? + "\n";
        std::fs::write(path, text).with_context(|| format!("cannot write {}", path.display()))
    }
}

/// The figures of a converted match.
pub fn figures_of(records: &[TickRecord], side: &Side) -> anyhow::Result<MatchFigures> {
    let frames = frames_of(records, side)?;
    let v = measure_frames(&frames, side.pitch[0], Mode::Full)?;
    let mut live = vec![0u32; side.halves.len()];
    for f in &frames {
        if f.live == Some(true) {
            live[f.half] += 1;
        }
    }
    Ok(MatchFigures {
        match_id: side.match_id.clone(),
        frames: u32::try_from(frames.len()).unwrap_or(u32::MAX),
        live_minutes: live
            .into_iter()
            .map(|n| f64::from(n) / (FRAMES_PER_SECOND * 60.0))
            .collect(),
        clock_offset: side.clock_offset,
        checks: Checks {
            records: u32::try_from(records.len()).unwrap_or(u32::MAX),
            rotated: side.rotated,
            off_pitch_spells: u32::try_from(side.off_pitch.len()).unwrap_or(u32::MAX),
            ball_missing: u32::try_from(side.ball_missing.len()).unwrap_or(u32::MAX),
            events_placed: u32::try_from(side.events.len()).unwrap_or(u32::MAX),
            events_unplaced: side.unplaced,
        },
        figures: named(&v),
    })
}

/// Converts and measures every DFL match of `data`, in match-id order.
pub fn measure_dfl(data: &Path) -> anyhow::Result<TrackingFile> {
    let mut matches = Vec::new();
    for files in super::dfl::matches(data)? {
        let m = super::dfl::read(&files)?;
        let c = super::convert::to_ticks(&m)?;
        let figures = figures_of(&c.records, &c.side)
            .with_context(|| format!("cannot measure {}", m.match_id))?;
        matches.push(figures);
    }
    anyhow::ensure!(!matches.is_empty(), "no DFL match in {}", data.display());
    Ok(TrackingFile {
        schema: TRACKING_SCHEMA,
        source: super::dfl::SOURCE.into(),
        credit: super::convert::credit(super::dfl::SOURCE).into(),
        frame_rate: 25,
        matches,
    })
}

/// The tracking file's default place: `gate/tracking/dfl-matches.json` of the checkout
/// holding the content folder.
pub fn default_out(content_root: &Path) -> PathBuf {
    crate::real_targets::gate_dir(content_root).join("tracking/dfl-matches.json")
}

#[cfg(test)]
mod tests {
    use super::super::{convert, dfl};
    use super::*;

    fn fixture() -> convert::Converted {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracking/dfl");
        let m = dfl::read(&dfl::find(&data, "DFL-MAT-FIX001").unwrap()).unwrap();
        convert::to_ticks(&m).unwrap()
    }

    /// The converted fixture's frames: every record, the holders less the off-pitch spells,
    /// the ball status from the runs and the restarts on their ticks, the half-opening
    /// kick-offs left out.
    #[test]
    fn the_frames_of_a_converted_match() {
        let c = fixture();
        let frames = frames_of(&c.records, &c.side).unwrap();
        assert_eq!(frames.len(), c.records.len());
        let at = |t: u32| frames.iter().find(|f| f.tick == t).unwrap();
        // H07 (home slot 6) is not tracked from tick 36 to 44.
        assert!(at(34).on[6].is_some());
        assert!(at(40).on[6].is_none());
        assert!(at(46).on[6].is_some());
        // A04 is sent off from tick 196.
        let a04 = c
            .side
            .off_pitch
            .iter()
            .find(|o| o.reason == "sent off")
            .unwrap();
        assert!(at(200).on[PER_TEAM + a04.slot].is_none());
        // The throw-in at 102 counts; the kick-offs at 0 and 116 open their halves.
        assert_eq!(
            at(102).restarts,
            [(Restart::ThrowIn, at(102).restarts[0].1)]
        );
        assert!(at(0).restarts.is_empty() && at(116).restarts.is_empty());
        // The kick-off after the goal counts.
        assert_eq!(at(232).restarts.len(), 1);
        assert_eq!(at(116).half, 1);
        assert_eq!(at(116).attack_x, [-1.0, 1.0]);
        let dead = frames.iter().filter(|f| f.live == Some(false)).count();
        assert!(dead > 0);
        let v = measure_frames(&frames, c.side.pitch[0], Mode::Full).unwrap();
        let n = crate::report::tracked_shape::place("team.frames").unwrap();
        assert_eq!(v[0][n], frames.len() as f64);
    }

    /// The figures file of the fixture names every figure and the converter's checks.
    #[test]
    fn the_figures_of_a_match_name_every_figure() {
        let c = fixture();
        let f = figures_of(&c.records, &c.side).unwrap();
        assert_eq!(f.figures.len(), field_names().len());
        assert_eq!(f.live_minutes.len(), 2);
        assert_eq!(f.checks.off_pitch_spells, 3);
        assert!(f.checks.rotated);
        assert_eq!(f.checks.events_unplaced, 1);
        let total: f64 = f.live_minutes.iter().sum();
        let live = f.figures["team.live_frames"][0] / (25.0 * 60.0);
        assert!((total - live).abs() < 1e-12);
    }

    /// A side file of another tick step is refused.
    #[test]
    fn a_side_file_of_another_tick_step_is_refused() {
        let mut c = fixture();
        c.side.tick_step = 1;
        let err = frames_of(&c.records, &c.side).unwrap_err().to_string();
        assert!(err.contains("tick step"), "{err}");
    }
}

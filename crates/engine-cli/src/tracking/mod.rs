//! Real tracked matches: the readers of the providers' files, the converter into the
//! engine's tick format plus a side file, and the checks that use them.
//!
//! A reader fills a [`TrackedMatch`], which holds what any tracking source gives and nothing
//! of one provider's layout: the pitch, each half's frames at their native times with the
//! ball and, where the source has them, the ball status and possession, every person with
//! his team, starting place and position group, and the events. The converter and the
//! shape report read only this type, so a second source needs only its own reader.

pub mod contrast;
pub mod convert;
pub mod dfl;
pub mod live_rule;
pub mod measure;
pub mod pff;
pub mod place;
pub mod resample;
pub mod side;
pub mod soccertrack;
pub mod time;
pub mod xml;

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::cli::{TrackSource, TrackingAction, TrackingOpts};

/// A player's position group: the keeper and the five outfield groups of the shape report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    Keeper,
    Cb,
    Fb,
    Cm,
    W,
    St,
}

impl Group {
    /// The outfield groups in the shape report's order.
    pub const OUTFIELD: [Group; 5] = [Group::Cb, Group::Fb, Group::Cm, Group::W, Group::St];

    /// The group's place among [`Self::OUTFIELD`]; `None` for the keeper.
    pub fn outfield(self) -> Option<usize> {
        Self::OUTFIELD.iter().position(|g| *g == self)
    }

    /// The group of a DFL position code: TW keeper; IVL, IVR, IVZ centre-back; LV, RV and the
    /// wing-backs DLM, DRM full-back; DML, DMR, DMZ, ZO, HL, HR central midfield; OLM, ORM,
    /// LM, RM, LA, RA winger; STL, STR, STZ striker. `None` for any other code.
    pub fn from_dfl(code: &str) -> Option<Self> {
        Some(match code {
            "TW" => Group::Keeper,
            "IVL" | "IVR" | "IVZ" => Group::Cb,
            "LV" | "RV" | "DLM" | "DRM" => Group::Fb,
            "DML" | "DMR" | "DMZ" | "ZO" | "HL" | "HR" => Group::Cm,
            "OLM" | "ORM" | "LM" | "RM" | "LA" | "RA" => Group::W,
            "STL" | "STR" | "STZ" => Group::St,
            _ => return None,
        })
    }

    /// The group of a PFF `positionGroupType`: GK keeper; LCB, RCB, MCB centre-back; LB, RB
    /// and the wing-backs LWB, RWB full-back; DM, CM, AM central midfield; LM, RM, LW, RW
    /// winger; CF striker. `None` for any other code.
    pub fn from_pff(code: &str) -> Option<Self> {
        Some(match code {
            "GK" => Group::Keeper,
            "LCB" | "RCB" | "MCB" => Group::Cb,
            "LB" | "RB" | "LWB" | "RWB" => Group::Fb,
            "DM" | "CM" | "AM" => Group::Cm,
            "LM" | "RM" | "LW" | "RW" => Group::W,
            "CF" => Group::St,
            _ => return None,
        })
    }
}

/// One person of a match: a player of one of the two teams.
#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    /// The provider's id, never a name.
    pub id: String,
    /// 0 home, 1 away.
    pub team: usize,
    pub starting: bool,
    /// The provider's position code.
    pub position: Option<String>,
    pub group: Option<Group>,
}

/// One frame of a half at its native time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// The provider's frame number.
    pub n: u32,
    /// Milliseconds since the Unix epoch.
    pub t_ms: i64,
    /// The ball in metres, centred on the pitch; `None` when the frame has no ball.
    pub ball: Option<[f32; 3]>,
    /// Whether the ball is in play, when the source says.
    pub live: Option<bool>,
    /// The team in possession (0 home, 1 away), when the source says.
    pub possession: Option<usize>,
}

/// One half: its frames in order and each person's position in each frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Half {
    /// The provider's name of the half, such as `firstHalf`.
    pub section: String,
    pub frames: Vec<Frame>,
    /// Per person (in [`TrackedMatch::persons`] order), per frame: the position in metres,
    /// centred, or `None` when the person is not tracked in that frame.
    pub positions: Vec<Vec<Option<[f32; 2]>>>,
}

/// How high a pass travels, as the events say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Height {
    Ground,
    High,
}

/// What an event is, in the terms the converter and the checks need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    /// A pass in play; a cross is its own kind.
    Pass {
        height: Option<Height>,
    },
    Cross,
    Shot {
        goal: bool,
    },
    ThrowIn,
    FreeKick,
    GoalKick,
    Corner,
    /// A kick-off: at the start of a half, or after a goal.
    KickOff {
        half_start: bool,
    },
    Penalty,
    Foul,
    Offside,
    /// A card; `sending_off` for a red card or a second yellow.
    Card {
        sending_off: bool,
    },
    Substitution {
        on: String,
        off: String,
    },
    /// The end of a half.
    FinalWhistle,
    /// A dropped ball.
    DroppedBall,
    /// Any other event, by the provider's name.
    Other(String),
}

impl EventKind {
    /// The kind's word in a side file.
    pub fn word(&self) -> &str {
        match self {
            EventKind::Pass { .. } => "pass",
            EventKind::Cross => "cross",
            EventKind::Shot { goal: true } => "goal",
            EventKind::Shot { goal: false } => "shot",
            EventKind::ThrowIn => "throw_in",
            EventKind::FreeKick => "free_kick",
            EventKind::GoalKick => "goal_kick",
            EventKind::Corner => "corner",
            EventKind::KickOff { .. } => "kick_off",
            EventKind::Penalty => "penalty",
            EventKind::Foul => "foul",
            EventKind::Offside => "offside",
            EventKind::Card { sending_off: true } => "sending_off",
            EventKind::Card { sending_off: false } => "card",
            EventKind::Substitution { .. } => "substitution",
            EventKind::FinalWhistle => "final_whistle",
            EventKind::DroppedBall => "dropped_ball",
            EventKind::Other(name) => name,
        }
    }
}

/// One event.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub kind: EventKind,
    pub team: Option<usize>,
    pub person: Option<String>,
    /// The event's time, milliseconds since the Unix epoch, when the source gives one.
    pub time_ms: Option<i64>,
    /// The game clock when the source gives no event time: the half (0 first) and the
    /// milliseconds since that half's first ball frame.
    pub clock: Option<(usize, i64)>,
    /// When the referee decided the stoppage a restart follows, when the source says.
    pub decision_ms: Option<i64>,
    /// The intended receiver of a pass.
    pub recipient: Option<String>,
}

/// A real tracked match, as any reader gives it.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackedMatch {
    /// The provider, such as `DFL`.
    pub source: String,
    pub match_id: String,
    /// The stadium's pitch: length and width in metres.
    pub pitch: [f64; 2],
    /// The provider's team ids, home first.
    pub teams: [String; 2],
    pub persons: Vec<Person>,
    pub halves: Vec<Half>,
    /// The events in time order.
    pub events: Vec<Event>,
    /// Engine ticks between two frames: 2 for a source of 25 frames a second, 1 for frames
    /// resampled to the engine's 50 ticks a second.
    pub ticks_per_frame: u32,
    /// The native frames a second the frames were resampled from, when they were.
    pub resampled_from: Option<f64>,
    /// What the source adds beyond its frames and events, for the side file.
    pub notes: Notes,
}

/// What a source adds beyond its frames and events; nothing for DFL.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Notes {
    /// Where the ball status comes from when the frames do not carry it: `events`.
    pub ball_status_from: Option<String>,
    /// The share of the kept player rows that the provider estimated off camera.
    pub estimated_share: Option<f64>,
    /// The frames left out, by period.
    pub left_out: Vec<side::LeftOut>,
    /// Whether the home team starts the match on the negative-x end, when the source says.
    pub home_starts_left: Option<bool>,
}

/// The half of each event, from 0: each half-opening kick-off after the first opens the
/// next half, and the events before the first belong to the first half.
pub fn event_halves(events: &[Event]) -> Vec<usize> {
    let mut half = 0;
    let mut opened = false;
    events
        .iter()
        .map(|e| {
            if matches!(e.kind, EventKind::KickOff { half_start: true }) {
                if opened {
                    half += 1;
                }
                opened = true;
            }
            half
        })
        .collect()
}

/// The raw data folder: `--data`, or the main checkout's `.scratch/data`.
fn data_folder(content_root: &Path, flag: Option<&Path>) -> anyhow::Result<PathBuf> {
    match flag {
        Some(d) => Ok(d.to_path_buf()),
        None => crate::real_targets::default_data(content_root).context(
            "no data folder: pass --data, or run inside a checkout of this repository whose \
             main checkout holds .scratch/data",
        ),
    }
}

/// Prints one line per measured PFF match, each refused match, and the totals.
fn print_pff(run: &measure::PffRun) {
    let mut live = Vec::new();
    for (m, c) in run.file.matches.iter().zip(&run.counts) {
        let halves: Vec<String> = m.live_minutes.iter().map(|v| format!("{v:.1}")).collect();
        live.push(m.live_minutes.iter().sum::<f64>());
        let left: Vec<String> = c
            .left_out
            .iter()
            .map(|l| format!("period {} {} frames", l.period, l.frames))
            .collect();
        println!(
            "{}: {} frames, ball in play {} minutes by half, {} frame lines ({} repeated), \
             {:.1}% of player rows estimated, {} of {} frames without a ball, left out {}, \
             direction checked{}",
            m.match_id,
            m.frames,
            halves.join(" + "),
            c.lines,
            c.repeated,
            100.0 * c.estimated_share,
            c.ball_absent,
            c.native_frames,
            if left.is_empty() {
                "nothing".to_string()
            } else {
                left.join(", ")
            },
            if c.rotated {
                ", turned by 180 degrees"
            } else {
                ""
            }
        );
    }
    for (id, why) in &run.refused {
        println!("{id}: refused: {why}");
    }
    let n = run.counts.len();
    let sum = |f: &dyn Fn(&measure::PffCounts) -> f64| run.counts.iter().map(f).sum::<f64>();
    let frames: u64 = run.file.matches.iter().map(|m| u64::from(m.frames)).sum();
    let native = sum(&|c| c.native_frames as f64);
    println!(
        "totals: {n} matches measured, {} refused; {frames} frames; {:.0} frame lines; ball in \
         play {:.1} minutes a match on average; {:.1}% of player rows estimated (mean of \
         matches); {:.1}% of frames without a ball",
        run.refused.len(),
        sum(&|c| c.lines as f64),
        live.iter().sum::<f64>() / n.max(1) as f64,
        100.0 * sum(&|c| c.estimated_share) / n.max(1) as f64,
        100.0 * sum(&|c| f64::from(c.ball_absent)) / native.max(1.0)
    );
}

/// Runs `engine-cli tracking`.
pub fn run(content_dir: Option<&Path>, opts: &TrackingOpts) -> anyhow::Result<i32> {
    let dir = engine::ContentDir::resolve(content_dir)?;
    match &opts.action {
        TrackingAction::Convert {
            source,
            data,
            match_id,
            out,
        } => {
            let folder = data_folder(dir.root(), data.as_deref())?;
            let m = match source {
                TrackSource::Dfl => dfl::read(&dfl::find(&folder, match_id)?)?,
                TrackSource::Pff => {
                    let r = pff::read(&folder, match_id)?;
                    println!(
                        "{}: {} frame lines read ({} repeated), {} frames of the two halves \
                         resampled from {} a second",
                        r.m.match_id,
                        r.lines,
                        r.repeated,
                        r.native_frames,
                        r.m.resampled_from.unwrap_or_default()
                    );
                    r.m
                }
            };
            let converted = convert::to_ticks(&m)?;
            let (ticks, side) = convert::write(&converted, out)?;
            println!(
                "{}: {} records, {} halves, {} off-pitch spells{}",
                m.match_id,
                converted.records.len(),
                converted.side.halves.len(),
                converted.side.off_pitch.len(),
                if converted.side.rotated {
                    ", turned by 180 degrees"
                } else {
                    ""
                }
            );
            println!("wrote {}", ticks.display());
            println!("wrote {}", side.display());
            Ok(0)
        }
        TrackingAction::LiveCheck { data, out } => {
            let folder = data_folder(dir.root(), data.as_deref())?;
            let check = live_rule::check_dfl(&folder)?;
            println!("match            rule min  tracked min  difference");
            for m in &check.matches {
                println!(
                    "{:<16} {:>8.1}  {:>11.1}  {:>+10.1}",
                    m.match_id, m.rule_minutes, m.tracked_minutes, m.difference
                );
            }
            println!(
                "verdict: {} (every match within {:.1} minutes); Wyscout rates {}",
                check.verdict, check.tolerance_minutes, check.wyscout_rates
            );
            let path = out
                .clone()
                .unwrap_or_else(|| live_rule::default_out(dir.root()));
            check.write(&path)?;
            println!("wrote {}", path.display());
            Ok(0)
        }
        TrackingAction::Contrast {
            soccertrack,
            tracking,
            out,
        } => {
            let tracking = tracking
                .clone()
                .unwrap_or_else(|| measure::default_out(dir.root()));
            let file = measure::TrackingFile::read(&tracking)?;
            let c = contrast::contrast(soccertrack, &file)?;
            print!("{}", c.table());
            let path = match out {
                Some(p) => p.clone(),
                None => contrast::default_out(&data_folder(dir.root(), None)?),
            };
            c.write(&path, &crate::real_targets::gate_dir(dir.root()))?;
            println!("wrote {}", path.display());
            Ok(0)
        }
        TrackingAction::Measure {
            source: TrackSource::Pff,
            data,
            jobs,
            out,
        } => {
            let folder = data_folder(dir.root(), data.as_deref())?;
            let start = std::time::Instant::now();
            let progress = |id: &str, done: &anyhow::Result<()>| match done {
                Ok(()) => eprintln!(
                    "{id}: measured after {:.0} s",
                    start.elapsed().as_secs_f64()
                ),
                Err(e) => eprintln!("{id}: refused: {e}"),
            };
            let run = measure::measure_pff(&folder, jobs.unwrap_or(1) as usize, &progress)?;
            print_pff(&run);
            println!("wall time {:.0} s", start.elapsed().as_secs_f64());
            let path = out
                .clone()
                .unwrap_or_else(|| measure::default_pff_out(dir.root()));
            run.file.write(&path)?;
            println!("wrote {}", path.display());
            Ok(if run.refused.is_empty() { 0 } else { 1 })
        }
        TrackingAction::Measure {
            source: TrackSource::Dfl,
            data,
            jobs: _,
            out,
        } => {
            let folder = data_folder(dir.root(), data.as_deref())?;
            let file = measure::measure_dfl(&folder)?;
            for m in &file.matches {
                let halves: Vec<String> =
                    m.live_minutes.iter().map(|v| format!("{v:.1}")).collect();
                let offset = m.clock_offset.map_or("none".to_string(), |o| {
                    format!(
                        "{:.0} ms over {} passes (IQR {:.0} ms)",
                        o.ms, o.passes, o.iqr_ms
                    )
                });
                println!(
                    "{}: {} frames, ball in play {} minutes by half, clock offset {}",
                    m.match_id,
                    m.frames,
                    halves.join(" + "),
                    offset
                );
            }
            let path = out
                .clone()
                .unwrap_or_else(|| measure::default_out(dir.root()));
            file.write(&path)?;
            println!("wrote {}", path.display());
            Ok(0)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Events before the first half-opening kick-off belong to the first half.
    #[test]
    fn the_events_before_the_first_kick_off_are_in_the_first_half() {
        let e = |kind: EventKind| Event {
            kind,
            team: None,
            person: None,
            time_ms: Some(0),
            clock: None,
            decision_ms: None,
            recipient: None,
        };
        let events = [
            e(EventKind::Foul),
            e(EventKind::KickOff { half_start: true }),
            e(EventKind::Cross),
            e(EventKind::KickOff { half_start: false }),
            e(EventKind::KickOff { half_start: true }),
            e(EventKind::Cross),
        ];
        assert_eq!(event_halves(&events), [0, 0, 0, 0, 1, 1]);
    }

    #[test]
    fn every_dfl_code_of_the_seven_matches_has_a_group() {
        for code in [
            "DLM", "DML", "DMR", "DMZ", "DRM", "HL", "HR", "IVL", "IVR", "IVZ", "LA", "LM", "LV",
            "OLM", "ORM", "RA", "RM", "RV", "STL", "STR", "STZ", "TW", "ZO",
        ] {
            assert!(Group::from_dfl(code).is_some(), "{code}");
        }
        assert_eq!(Group::from_dfl("TW"), Some(Group::Keeper));
        assert_eq!(Group::from_dfl("DLM"), Some(Group::Fb));
        assert_eq!(Group::from_dfl("XX"), None);
        assert_eq!(Group::Keeper.outfield(), None);
        assert_eq!(Group::St.outfield(), Some(4));
    }
}

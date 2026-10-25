//! Real tracked matches: the readers of the providers' files, the converter into the
//! engine's tick format plus a side file, and the checks that use them.
//!
//! A reader fills a [`TrackedMatch`], which holds what any tracking source gives and nothing
//! of one provider's layout: the pitch, each half's frames at their native times with the
//! ball and, where the source has them, the ball status and possession, every person with
//! his team, starting place and position group, and the events. The converter and the
//! shape report read only this type, so a second source needs only its own reader.

pub mod convert;
pub mod dfl;
pub mod side;
pub mod time;
pub mod xml;

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::cli::{TrackingAction, TrackingOpts};

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

/// Runs `engine-cli tracking`.
pub fn run(content_dir: Option<&Path>, opts: &TrackingOpts) -> anyhow::Result<i32> {
    let dir = engine::ContentDir::resolve(content_dir)?;
    match &opts.action {
        TrackingAction::Convert {
            data,
            match_id,
            out,
        } => {
            let folder = data_folder(dir.root(), data.as_deref())?;
            let m = dfl::read(&dfl::find(&folder, match_id)?)?;
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
    }
}
#[cfg(test)]
mod tests {
    use super::*;

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

//! The side file of a converted match: what the engine's tick format does not carry. The
//! ticks hold the ball and the 22 players; the side file says how the frames map onto the
//! ticks, which tick each half starts and ends on and which way team 0 attacks, who holds
//! each slot and when, when a slot's player is not on the pitch and why, the ball status and
//! possession as runs, and where each event was placed. Person keys are the provider's ids,
//! never names.
//!
//! The file is `<match>.side.json` beside `<match>.ticks`, schema 1; a reader refuses an
//! unknown field and another schema.

use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use super::Group;

/// The schema this program writes and reads.
pub const SCHEMA: u32 = 1;

/// The side file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Side {
    pub schema: u32,
    /// The provider, such as `DFL`, or `engine` for a played match.
    pub source: String,
    /// The credit of the data.
    pub credit: String,
    pub match_id: String,
    /// The stadium's pitch: length and width in metres.
    pub pitch: [f64; 2],
    /// The source's frames a second.
    pub frame_rate: u32,
    /// Engine ticks between two frames.
    pub tick_step: u32,
    /// What the odd ticks from a half's start hold: `absent`, no record (frames at 25 a
    /// second), `resampled`, a frame resampled to 50 a second, or `recorded` (an engine
    /// match).
    pub odd_ticks: String,
    /// The whole match was turned by 180 degrees so team 0 attacks the engine's first-half
    /// direction.
    pub rotated: bool,
    pub halves: Vec<SideHalf>,
    /// Who holds each slot, spell by spell.
    pub slots: Vec<Spell>,
    /// Each spell of a slot whose player is not on the pitch: his record holds the slot's
    /// parking spot.
    pub off_pitch: Vec<OffPitch>,
    /// The ball status and possession as runs of ticks.
    pub ball: Vec<BallRun>,
    /// Ticks whose frame had no ball position: the record repeats the last one.
    pub ball_missing: Vec<u32>,
    /// Each placed event.
    pub events: Vec<Placed>,
    /// Events with neither an event time nor a game clock, which no tick takes.
    pub unplaced: u32,
    /// The match's clock offset fitted on the passes, when any pass gave one.
    pub clock_offset: Option<ClockOffset>,
    /// The native frames a second the ticks were resampled from, when they were: every tick
    /// of a half then holds a resampled frame. A DFL side file has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resampled_from: Option<f64>,
    /// Where the ball status comes from when the frames do not carry it: `events`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ball_status_from: Option<String>,
    /// The share of the converted player rows that the provider estimated off camera.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_share: Option<f64>,
    /// The frames left out, by period, such as extra time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub left_out: Vec<LeftOut>,
}

/// Frames of one period that the converter left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeftOut {
    /// The provider's period, such as `3` for the first half of extra time, or `none`.
    pub period: String,
    pub frames: u32,
}

/// One half.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SideHalf {
    /// The provider's name of the half.
    pub section: String,
    /// The provider's number of the half's first ball frame, which tick `start_tick` holds.
    pub first_frame: u32,
    pub start_tick: u32,
    pub end_tick: u32,
    /// +1 when team 0 attacks towards positive x in this half, -1 otherwise.
    pub team0_attack: f64,
}

/// A slot's holder over a run of ticks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spell {
    pub team: usize,
    pub slot: usize,
    pub person: String,
    pub group: Group,
    pub from_tick: u32,
    pub to_tick: u32,
}

/// A run of ticks in which a slot's player is not on the pitch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OffPitch {
    pub team: usize,
    pub slot: usize,
    /// The slot's holder; `None` for a slot no player holds.
    pub person: Option<String>,
    pub from_tick: u32,
    pub to_tick: u32,
    /// `sent off`, `not tracked` or `no player`.
    pub reason: String,
}

/// A run of ticks with one ball status and one team in possession.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BallRun {
    pub from_tick: u32,
    pub to_tick: u32,
    pub live: Option<bool>,
    pub possession: Option<usize>,
}

/// One event on its tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placed {
    pub tick: u32,
    pub kind: String,
    pub team: Option<usize>,
    /// `event-time` (the frame nearest its time) or `clock-offset` (its game clock plus the
    /// match's clock offset).
    pub placed: String,
}

/// The offset from a game clock to the frames' times.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClockOffset {
    /// The median over the passes, in milliseconds.
    pub ms: f64,
    pub passes: u32,
    /// The interquartile range of the passes' offsets, in milliseconds.
    pub iqr_ms: f64,
}

impl Side {
    /// Writes the side file to `path`.
    pub fn write(&self, path: &Path) -> anyhow::Result<()> {
        let text = serde_json::to_string_pretty(self)? + "\n";
        std::fs::write(path, text).with_context(|| format!("cannot write {}", path.display()))
    }

    /// Reads the side file at `path`, refusing an unknown field or another schema.
    #[cfg(test)]
    pub fn read(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let side: Self = serde_json::from_str(&text)
            .with_context(|| format!("{} is not a side file", path.display()))?;
        anyhow::ensure!(
            side.schema == SCHEMA,
            "the side file {} has schema {}; this program reads {SCHEMA}",
            path.display(),
            side.schema
        );
        Ok(side)
    }

    /// The side file's name beside `<match>.ticks`.
    pub fn file_name(match_id: &str) -> String {
        format!("{match_id}.side.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Side {
        Side {
            schema: SCHEMA,
            source: "DFL".into(),
            credit: "credit".into(),
            match_id: "M".into(),
            pitch: [105.0, 68.0],
            frame_rate: 25,
            tick_step: 2,
            odd_ticks: "absent".into(),
            rotated: true,
            halves: vec![SideHalf {
                section: "firstHalf".into(),
                first_frame: 10000,
                start_tick: 0,
                end_tick: 10,
                team0_attack: 1.0,
            }],
            slots: vec![Spell {
                team: 0,
                slot: 0,
                person: "P".into(),
                group: Group::Keeper,
                from_tick: 0,
                to_tick: 10,
            }],
            off_pitch: vec![OffPitch {
                team: 1,
                slot: 3,
                person: None,
                from_tick: 2,
                to_tick: 4,
                reason: "no player".into(),
            }],
            ball: vec![BallRun {
                from_tick: 0,
                to_tick: 10,
                live: Some(true),
                possession: Some(1),
            }],
            ball_missing: vec![6],
            events: vec![Placed {
                tick: 4,
                kind: "pass".into(),
                team: Some(0),
                placed: "event-time".into(),
            }],
            unplaced: 1,
            clock_offset: Some(ClockOffset {
                ms: 480.0,
                passes: 2,
                iqr_ms: 0.0,
            }),
            resampled_from: Some(29.97),
            ball_status_from: Some("events".into()),
            estimated_share: Some(0.5),
            left_out: vec![LeftOut {
                period: "3".into(),
                frames: 10,
            }],
        }
    }

    #[test]
    fn a_side_file_round_trips() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(Side::file_name("M"));
        sample().write(&path).unwrap();
        assert_eq!(Side::read(&path).unwrap(), sample());
    }

    /// A side file without the resampling fields writes none of them, so a DFL side file
    /// keeps its bytes; a reader takes it with the fields empty.
    #[test]
    fn the_resampling_fields_are_written_only_when_present() {
        let mut plain = sample();
        plain.resampled_from = None;
        plain.ball_status_from = None;
        plain.estimated_share = None;
        plain.left_out.clear();
        let text = serde_json::to_string(&plain).unwrap();
        for field in [
            "resampled_from",
            "ball_status_from",
            "estimated_share",
            "left_out",
        ] {
            assert!(!text.contains(field), "{field}");
        }
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(Side::file_name("M"));
        plain.write(&path).unwrap();
        assert_eq!(Side::read(&path).unwrap(), plain);
    }

    #[test]
    fn an_unknown_field_or_another_schema_is_refused() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("x.side.json");
        let mut doc = serde_json::to_value(sample()).unwrap();
        doc["extra"] = 1.into();
        std::fs::write(&path, doc.to_string()).unwrap();
        assert!(Side::read(&path).is_err());
        let mut other = sample();
        other.schema = 2;
        other.write(&path).unwrap();
        let err = Side::read(&path).unwrap_err().to_string();
        assert!(err.contains("schema 2"), "{err}");
    }
}

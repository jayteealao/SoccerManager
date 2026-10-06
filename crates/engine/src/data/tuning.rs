//! The tuning file: the engine constants, the generator distributions, and the fatigue
//! curve. Bounds live in code; values live in the file.

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::attributes::Group;
use crate::data::team::Position;
use crate::flags::{FlagDef, each_flag_switches_something};
use crate::team::PLAYERS_PER_TEAM;
use crate::tuning::Tuning;

/// Schema version this build reads. Version 2 rewrites the fatigue block, which the engine
/// now reads, and adds the decision weights and injury rates to the engine block.
pub const TUNING_VERSION: u32 = 2;

/// The tuning file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct TuningFile {
    #[garde(skip)]
    pub schema_version: u32,
    #[garde(dive)]
    pub engine: Tuning,
    #[garde(dive)]
    pub generator: GeneratorTuning,
    #[garde(dive)]
    pub fatigue: FatigueTuning,
    #[garde(dive)]
    pub stream: StreamTuning,
    /// Feature flags that switch between a current model and a candidate. Optional: a file
    /// without the block has no flags.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[garde(dive, custom(each_flag_switches_something))]
    pub flags: BTreeMap<String, FlagDef>,
}

/// What the live tick stream holds and how often it sends a keyframe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct StreamTuning {
    /// Ticks the server may run ahead of the client. A full buffer pauses the simulation
    /// thread, which is the backpressure.
    #[garde(range(min = 10, max = 5000))]
    pub buffer_ticks: usize,
    /// Ticks between keyframes. Every other tick is a delta against the one before it.
    #[garde(range(min = 1, max = 500))]
    pub keyframe_interval: u32,
}

/// What the team generator draws from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct GeneratorTuning {
    /// Players per generated club: the eleven starters plus the bench.
    #[garde(range(min = 11, max = 40))]
    pub squad_size: usize,
    /// The position of each formation slot, in slot order.
    #[garde(skip)]
    pub slot_positions: [Position; PLAYERS_PER_TEAM],
    /// The positions of the bench, in order; its length plus eleven is `squad_size`.
    #[garde(custom(bench_matches_squad(self.squad_size)))]
    pub bench_positions: Vec<Position>,
    /// One distribution per position code; every one of the ten codes must be present.
    #[garde(dive, custom(every_position_present))]
    pub per_position: BTreeMap<String, GroupDist>,
    /// What the body fields are drawn from. Optional, so a tuning file written before body
    /// fields (as an old replay embeds it) still loads; `engine-cli generate` needs it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(dive)]
    pub body: Option<BodyTuning>,
}

/// The body-field distributions: height per position, age, and nationality.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct BodyTuning {
    /// Height in centimetres per position code; every one of the ten codes must be present.
    #[garde(dive, custom(every_height_present))]
    pub height: BTreeMap<String, HeightDist>,
    #[garde(dive)]
    pub age: AgeDist,
    #[garde(dive)]
    pub nationality: NationalityDist,
}

/// A height bell curve in centimetres; draws are rounded and kept to 150 to 215.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct HeightDist {
    #[garde(range(min = 150.0, max = 215.0))]
    pub mean: f64,
    #[garde(range(min = 0.0, max = 20.0))]
    pub spread: f64,
}

/// An age bell curve in years; draws are rounded and kept to `min` to `max`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct AgeDist {
    #[garde(range(min = 15.0, max = 45.0))]
    pub mean: f64,
    #[garde(range(min = 0.0, max = 15.0))]
    pub spread: f64,
    #[garde(range(min = 15, max = 45))]
    pub min: u8,
    #[garde(range(min = self.min, max = 45))]
    pub max: u8,
}

/// Nationality: the club's country, the others a foreign player comes from, and the share
/// of foreign players. Codes are three upper-case letters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct NationalityDist {
    #[garde(custom(nation_code))]
    pub home: String,
    #[garde(length(min = 1, max = 64), inner(custom(nation_code)))]
    pub foreign: Vec<String>,
    #[garde(range(min = 0.0, max = 1.0))]
    pub foreign_share: f64,
}

fn nation_code(code: &str, _ctx: &()) -> garde::Result {
    if code.len() == 3 && code.bytes().all(|b| b.is_ascii_uppercase()) {
        Ok(())
    } else {
        Err(garde::Error::new(format!(
            "{code} is not three upper-case letters"
        )))
    }
}

fn every_height_present(map: &BTreeMap<String, HeightDist>, _ctx: &()) -> garde::Result {
    for p in Position::ALL {
        if !map.contains_key(p.code()) {
            return Err(garde::Error::new(format!(
                "position {} has no height",
                p.code()
            )));
        }
    }
    Ok(())
}

/// A mean and spread per attribute group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct GroupDist {
    #[garde(dive)]
    pub technical: Dist,
    #[garde(dive)]
    pub mental: Dist,
    #[garde(dive)]
    pub physical: Dist,
    #[garde(dive)]
    pub goalkeeping: Dist,
}

impl GroupDist {
    /// The distribution for `group`.
    pub fn get(&self, group: Group) -> &Dist {
        match group {
            Group::Technical => &self.technical,
            Group::Mental => &self.mental,
            Group::Physical => &self.physical,
            Group::Goalkeeping => &self.goalkeeping,
        }
    }
}

/// A bell-curve centre and half-width on the 1 to 100 scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Dist {
    #[garde(range(min = 1.0, max = 100.0))]
    pub mean: f64,
    #[garde(range(min = 0.0, max = 40.0))]
    pub spread: f64,
}

/// Fatigue: how fast energy drains, and how far low energy lowers pace and decisions.
/// Energy runs from 1.0 (fresh) down to 0.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct FatigueTuning {
    /// Energy at and above which a player plays at full strength.
    #[garde(range(min = 0.0, max = 1.0))]
    pub threshold: f64,
    /// The curve, as `[energy, multiplier]` points from high energy to low: 2 to 8 points,
    /// energy strictly decreasing from 1.0 to 0.0, multipliers 0.3 to 1.0.
    #[garde(custom(check_curve))]
    pub curve: Vec<[f64; 2]>,
    /// Energy lost per second standing still, and the extra at full speed (scaled by the
    /// square of the speed fraction). A player with stamina 100 drains half as fast as the
    /// average, one with stamina 0 half again as fast.
    #[garde(range(min = 0.0, max = 0.01))]
    pub drain_base_per_s: f64,
    #[garde(range(min = 0.0, max = 0.05))]
    pub drain_effort_per_s: f64,
    /// Energy recovered at half-time by an average player; natural fitness scales it.
    #[garde(range(min = 0.0, max = 1.0))]
    pub half_time_recovery: f64,
    /// Stamina points recovered per rest day. The season layer reads it.
    #[garde(range(min = 0.0, max = 100.0))]
    pub recovery_per_day: f64,
}

fn check_curve(curve: &[[f64; 2]], _ctx: &()) -> garde::Result {
    if !(2..=8).contains(&curve.len()) {
        return Err(garde::Error::new(format!(
            "holds {} points; allowed 2 to 8",
            curve.len()
        )));
    }
    if curve[0][0] != 1.0 || curve[curve.len() - 1][0] != 0.0 {
        return Err(garde::Error::new(
            "the first point must be at energy 1.0 and the last at 0.0",
        ));
    }
    for (i, [energy, multiplier]) in curve.iter().enumerate() {
        if !(0.3..=1.0).contains(multiplier) {
            return Err(garde::Error::new(format!(
                "point {i}: multiplier {multiplier}; allowed 0.3 to 1.0"
            )));
        }
        if i > 0 && *energy >= curve[i - 1][0] {
            return Err(garde::Error::new(format!(
                "point {i}: energy {energy} does not decrease"
            )));
        }
    }
    Ok(())
}

fn bench_matches_squad(squad_size: usize) -> impl FnOnce(&Vec<Position>, &()) -> garde::Result {
    move |bench, _| {
        if bench.len() + PLAYERS_PER_TEAM != squad_size {
            return Err(garde::Error::new(format!(
                "holds {} positions; squad_size {squad_size} needs {}",
                bench.len(),
                squad_size.saturating_sub(PLAYERS_PER_TEAM)
            )));
        }
        Ok(())
    }
}

fn every_position_present(map: &BTreeMap<String, GroupDist>, _ctx: &()) -> garde::Result {
    for code in Position::ALL.iter().map(Position::code) {
        if !map.contains_key(code) {
            return Err(garde::Error::new(format!("position {code} is missing")));
        }
    }
    for key in map.keys() {
        if !Position::ALL.iter().any(|p| p.code() == key) {
            return Err(garde::Error::new(format!(
                "position {key} is not one of the ten codes"
            )));
        }
    }
    Ok(())
}

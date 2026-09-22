//! The tuning file: the engine constants, the generator distributions, and the fatigue
//! parameters later slices consume. Bounds live in code; values live in the file.

use std::collections::BTreeMap;

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::data::attributes::Group;
use crate::data::team::Position;
use crate::team::PLAYERS_PER_TEAM;
use crate::tuning::Tuning;

/// Schema version this build reads.
pub const TUNING_VERSION: u32 = 1;

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

/// Fatigue parameters; the tactics slice consumes them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct FatigueTuning {
    /// Minutes of play until stamina halves at full effort.
    #[garde(range(min = 1.0, max = 600.0))]
    pub minutes_to_half_stamina: f64,
    /// Stamina points recovered per rest day.
    #[garde(range(min = 0.0, max = 100.0))]
    pub recovery_per_day: f64,
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

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

/// Schema version this build reads. Version 2 rewrote the fatigue block and added the
/// decision weights and injury rates to the engine block. Version 3 adds the attribute
/// contract (`engine.contract`) and the world spread of the generator (`generator.world`),
/// and drops the linear speed fields and the per-position distributions they replace; a
/// version 2 file converts ([`crate::data::convert::tuning_v2_to_v3`]). Version 4 adds the
/// state caps and body jobs (`engine.contract.states`, `engine.contract.body`) and the
/// fatigue weights per attribute group (`fatigue.group_weights`); a version 3 file converts
/// ([`crate::data::convert::tuning_v3_to_v4`]). Version 5 adds consistency
/// (`engine.contract.consistency`), the words of the hidden values (`hidden`), the match
/// rating (`match_rating`), and the spread of the hidden values in the generator
/// (`generator.world.hidden`); a version 4 file converts
/// ([`crate::data::convert::tuning_v4_to_v5`]).
pub const TUNING_VERSION: u32 = 5;

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
    /// The word bands of each hidden value and the confidence thresholds.
    #[garde(dive)]
    pub hidden: crate::contract::hidden::HiddenTuning,
    /// The weights of the match rating.
    #[garde(dive)]
    pub match_rating: crate::match_rating::MatchRatingTuning,
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
    /// The world spread: tier levels, club, player and attribute spreads, and each
    /// position's offsets per attribute group.
    #[garde(dive)]
    pub world: WorldTuning,
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

/// The world spread, on the 1 to 20 scale. Each club's level is drawn around its tier's
/// mean, each player's level around his club's, and each attribute around the player's level
/// plus his position's offset for the attribute's group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct WorldTuning {
    /// The tiers, from the top flight down: each with its mean level and the spread of its
    /// clubs' levels around it.
    #[garde(length(min = 1, max = 10), dive)]
    pub tiers: Vec<TierDist>,
    /// The spread of a player's level around his club's.
    #[garde(range(min = 0.0, max = 6.0))]
    pub player_spread: f64,
    /// The spread of an attribute around the player's level plus his position offset.
    #[garde(range(min = 0.0, max = 6.0))]
    pub attribute_spread: f64,
    /// Per position code, the offset of each attribute group from the player's level; every
    /// one of the ten codes must be present.
    #[garde(dive, custom(every_position_present))]
    pub offsets: BTreeMap<String, GroupOffsets>,
    /// The hidden values, drawn around their own mean, independent of the player's level.
    #[garde(dive)]
    pub hidden: HiddenDist,
}

/// The bell curve the hidden values are drawn from, on the 1 to 20 scale.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct HiddenDist {
    #[garde(range(min = 1.0, max = 20.0))]
    pub mean: f64,
    #[garde(range(min = 0.0, max = 6.0))]
    pub spread: f64,
}

/// One tier's mean level and the spread of its clubs around it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct TierDist {
    #[garde(range(min = 1.0, max = 20.0))]
    pub mean: f64,
    #[garde(range(min = 0.0, max = 6.0))]
    pub club_spread: f64,
}

/// A position's offset per attribute group, in 1 to 20 units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct GroupOffsets {
    #[garde(range(min = -19.0, max = 19.0))]
    pub technical: f64,
    #[garde(range(min = -19.0, max = 19.0))]
    pub mental: f64,
    #[garde(range(min = -19.0, max = 19.0))]
    pub physical: f64,
    #[garde(range(min = -19.0, max = 19.0))]
    pub goalkeeping: f64,
}

impl GroupOffsets {
    /// The offset for `group`.
    pub fn get(&self, group: Group) -> f64 {
        match group {
            Group::Technical => self.technical,
            Group::Mental => self.mental,
            Group::Physical => self.physical,
            Group::Goalkeeping => self.goalkeeping,
        }
    }
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
    /// Stamina points recovered per rest day, out of 100: the days-of-rest input sets the
    /// energy at kick-off from it.
    #[garde(range(min = 0.0, max = 100.0))]
    pub recovery_per_day: f64,
    /// How hard tiredness lowers each attribute group it acts on, as a share of the full
    /// curve: physical first, technical and goalkeeping less. The body family does not act
    /// on the mental group.
    #[garde(dive)]
    pub group_weights: GroupWeights,
    /// The highest speed, as a share of the average player's top speed, at which a sprint
    /// still costs more: the effort term reads speed over that top speed up to this share,
    /// so a faster runner pays for his speed up to it.
    #[garde(range(min = 1.0, max = 2.0))]
    pub sprint_cap: f64,
}

/// The fatigue weight per attribute group the body family acts on, 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct GroupWeights {
    #[garde(range(min = 0.0, max = 1.0))]
    pub physical: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub technical: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub goalkeeping: f64,
}

impl GroupWeights {
    /// The weight of each group in [`crate::contract::states::GROUPS`] order; mental is 0.
    pub fn by_group(&self) -> [f64; crate::contract::states::GROUP_COUNT] {
        [self.technical, 0.0, self.physical, self.goalkeeping]
    }
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

fn every_position_present(map: &BTreeMap<String, GroupOffsets>, _ctx: &()) -> garde::Result {
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

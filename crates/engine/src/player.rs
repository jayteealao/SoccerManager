//! Player state, the attribute array, and the derived values the hot path reads.
//!
//! `Attributes` is a fixed array in schema order so `Player` stays `Copy` and no name lookup
//! happens during a tick. `Derived` is computed once at load through the attribute contract
//! ([`crate::contract`]): top speed from the pace map, acceleration and turning from their
//! stages, every stage value, and the skill gates. The states (fatigue, sharpness,
//! adaptation) move a player's ratings within caps ([`crate::contract::states`]), and his
//! effective values are derived the same way from those effective ratings.

use std::collections::BTreeMap;

use crate::contract::body::Body;
use crate::contract::states::GROUP_COUNT;
use crate::contract::{self, Gates, Stage, StageValues, stages::Blend};
use crate::data::attributes::{AttributeSchema, MAX_ATTRIBUTES};
use crate::math::DVec2;
use crate::rating::{MIN_TENTHS, Rating};
use crate::tuning::Tuning;

/// Attribute ratings in tenths of the 1 to 20 scale, in schema order.
#[derive(Debug, Clone, Copy)]
pub struct Attributes {
    pub values: [Rating; MAX_ATTRIBUTES],
    pub len: u8,
}

impl Attributes {
    /// The ratings of a validated team-file entry, in schema order. A missing attribute
    /// reads 1.0, the floor.
    pub fn from_entry(entry: &BTreeMap<String, Rating>, schema: &AttributeSchema) -> Self {
        let mut values = [Rating::default(); MAX_ATTRIBUTES];
        for (slot, def) in values.iter_mut().zip(&schema.attributes) {
            *slot = entry
                .get(&def.name)
                .copied()
                .unwrap_or(Rating::from_tenths(MIN_TENTHS));
        }
        Self {
            values,
            // The schema holds at most MAX_ATTRIBUTES (50) entries.
            len: schema.len() as u8,
        }
    }

    /// The rating at schema index `index`.
    pub fn get(&self, index: usize) -> Rating {
        self.values[index]
    }

    /// The ratings in schema order.
    pub fn iter(&self) -> impl Iterator<Item = Rating> + '_ {
        self.values[..usize::from(self.len)].iter().copied()
    }
}

/// Values the simulation reads every tick, computed once from the attributes through the
/// contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Derived {
    /// Maximum speed in metres per second, from the pace map.
    pub max_speed: f64,
    /// Maximum acceleration in metres per second squared, from the sprint stage.
    pub max_accel: f64,
    /// The factor on the sideways part of a change of velocity, from the turn stage, less
    /// the cost of his height: 1 at rating 10 and the reference height.
    pub turn: f64,
    /// How high he reaches a ball in the air, in metres: the average player's reach times
    /// his jump (the aerial reach knob), plus the standing reach of his height.
    pub reach_m: f64,
    /// The per-player factors on the average player's reaches and ranges.
    pub knobs: Knobs,
    /// The skill gates.
    pub gates: Gates,
}

/// The per-player factors on values every player had alike before the contract, each
/// `1 + spread · (2 · share − 1)` of its stage, exactly 1 at rating 10. They follow his
/// effective ratings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Knobs {
    /// The speed of a ball he controls on receipt (receive execute).
    pub receive: f64,
    /// His reach to a loose ball and how far ahead he reads a run (intercept see).
    pub intercept: f64,
    /// How far he presses from (press choose).
    pub press: f64,
    /// How closely he keeps his place out of possession (shape execute).
    pub shape: f64,
    /// His reach to block a shot (block execute).
    pub block: f64,
    /// How high he reaches in the air (aerial reach execute).
    pub aerial_reach: f64,
    /// A keeper's reach to a shot and his catch (claim execute).
    pub claim_reach: f64,
    /// A keeper's range for a lofted ball in his box (claim choose).
    pub claim_range: f64,
    /// How far from goal a keeper comes off his line for a loose ball (rush choose).
    pub rush: f64,
    /// How tight a keeper keeps the back line across the pitch (organise execute).
    pub organise: f64,
}

impl Knobs {
    /// Every factor 1: the average player.
    pub const NEUTRAL: Knobs = Knobs {
        receive: 1.0,
        intercept: 1.0,
        press: 1.0,
        shape: 1.0,
        block: 1.0,
        aerial_reach: 1.0,
        claim_reach: 1.0,
        claim_range: 1.0,
        rush: 1.0,
        organise: 1.0,
    };
}

impl Derived {
    /// Derives from a validated schema's attributes through the contract, for a player with
    /// no body (the reference height): the values and the stage values.
    pub fn from_attributes(
        a: &Attributes,
        schema: &AttributeSchema,
        t: &Tuning,
    ) -> (Self, StageValues) {
        Self::from_blend(a, &Blend::of(schema), schema, t, Body::default())
    }

    /// [`Derived::from_attributes`] with the schema's blend resolved once, for a squad, and
    /// the player's body.
    pub fn from_blend(
        a: &Attributes,
        blend: &Blend,
        schema: &AttributeSchema,
        t: &Tuning,
        body: Body,
    ) -> (Self, StageValues) {
        let c = &t.contract;
        let stages = blend.values(a, c);
        let [pace, technique, agility] = contract::direct_ratings(a, schema);
        let knob = |action: contract::ActionKind, s: Stage| {
            contract::factor(c.actions.of(action).spread(), stages.share(s))
        };
        use contract::ActionKind as A;
        let g = &schema.gates;
        let penalty = |def: &contract::GateDef| {
            if agility < def.agility_pull_off {
                def.penalty_k
            } else {
                0.0
            }
        };
        let aerial_reach = knob(A::AerialReach, Stage::AERIAL_REACH_EXECUTE);
        let derived = Self {
            max_speed: c.speed.top_speed(pace),
            max_accel: c.accel.anchor * knob(A::Sprint, Stage::SPRINT_EXECUTE),
            turn: c.body.turn_factor(knob(A::Turn, Stage::TURN_EXECUTE), body),
            reach_m: c.body.reach_m(t.reach_height, aerial_reach, body),
            knobs: Knobs {
                receive: knob(A::Receive, Stage::RECEIVE_EXECUTE),
                intercept: knob(A::Intercept, Stage::INTERCEPT_SEE),
                press: knob(A::Press, Stage::PRESS_CHOOSE),
                shape: knob(A::Shape, Stage::SHAPE_EXECUTE),
                block: knob(A::Block, Stage::BLOCK_EXECUTE),
                aerial_reach,
                claim_reach: knob(A::Claim, Stage::CLAIM_EXECUTE),
                claim_range: knob(A::Claim, Stage::CLAIM_CHOOSE),
                rush: knob(A::Rush, Stage::RUSH_CHOOSE),
                organise: knob(A::Organise, Stage::ORGANISE_EXECUTE),
            },
            gates: Gates {
                chip_try: technique >= g.chip.technique_try,
                chip_penalty: penalty(&g.chip),
                take_on_try: technique >= g.take_on.technique_try,
                take_on_penalty: penalty(&g.take_on),
            },
        };
        (derived, stages)
    }
}

/// Whether a player takes part in play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    OnPitch,
    /// Sent off: the player stands at a fixed point beside the pitch until full time.
    SentOff,
    /// Injured: the player left play and stands beside the pitch until a substitute takes
    /// the place, or until full time.
    Injured,
}

/// One player on the pitch.
#[derive(Debug, Clone, Copy)]
pub struct Player {
    /// Roster index, 0 to 21.
    pub id: usize,
    /// Team index, 0 (home) or 1 (away).
    pub team: usize,
    /// Slot index inside the formation, 0 to 10.
    pub slot: usize,
    /// The player's place in the team file; a substitute takes the roster slot of the player
    /// it replaces and brings its own squad index.
    pub squad: usize,
    pub shirt: u8,
    pub attributes: Attributes,
    /// The effective values play reads: his base values (his squad entry's, which
    /// [`crate::sim::Simulation::base`] reads) derived again from his effective ratings
    /// whenever his state deltas change.
    pub derived: Derived,
    /// His state delta per attribute group, in tenths of a rating point, in
    /// [`contract::states::GROUPS`] order: 0 for a player at his base.
    pub deltas: [i8; GROUP_COUNT],
    /// Energy from 1.0 (fresh) down to 0.0.
    pub energy: f64,
    pub pos: DVec2,
    pub vel: DVec2,
    /// Where steering drives the player this tick.
    pub target: DVec2,
    /// Unit vector of the last non-zero velocity.
    pub facing: DVec2,
    pub status: Status,
    /// Yellow cards shown to the player in this match.
    pub yellow: u8,
    /// The first tick the player may attempt a tackle again after a foul.
    pub foul_ready: u32,
    /// The first tick after a concentration lapse; until then the player holds where he
    /// stands.
    pub lapse_until: u32,
}

impl Player {
    /// `true` while the player takes part in play.
    pub fn active(&self) -> bool {
        self.status == Status::OnPitch
    }

    /// Maximum speed in metres per second.
    pub fn max_speed(&self) -> f64 {
        self.derived.max_speed
    }

    /// Maximum acceleration in metres per second squared.
    pub fn max_accel(&self) -> f64 {
        self.derived.max_accel
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// The values and base stage values of a player with every attribute at `tenths` tenths
    /// of the shipped attribute file, for unit tests.
    pub(crate) fn flat(tenths: u8, t: &Tuning) -> (Derived, StageValues) {
        let schema = crate::data::test_support::shipped_content().attributes;
        let attributes = Attributes {
            values: [Rating::from_tenths(tenths); MAX_ATTRIBUTES],
            len: schema.len() as u8,
        };
        Derived::from_attributes(&attributes, &schema, t)
    }

    /// A player with every attribute at `tenths` tenths of the shipped attribute file, for
    /// unit tests.
    pub(crate) fn flat_player(id: usize, tenths: u8, t: &Tuning) -> Player {
        let schema = crate::data::test_support::shipped_content().attributes;
        let attributes = Attributes {
            values: [Rating::from_tenths(tenths); MAX_ATTRIBUTES],
            len: schema.len() as u8,
        };
        let (derived, _) = Derived::from_attributes(&attributes, &schema, t);
        Player {
            id,
            team: 0,
            slot: 0,
            squad: 0,
            shirt: 1,
            attributes,
            derived,
            deltas: [0; GROUP_COUNT],
            energy: 1.0,
            pos: DVec2::ZERO,
            vel: DVec2::ZERO,
            target: DVec2::ZERO,
            facing: DVec2::X,
            status: Status::OnPitch,
            yellow: 0,
            foul_ready: 0,
            lapse_until: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_content;

    fn entry_at(schema: &AttributeSchema, tenths: u8) -> BTreeMap<String, Rating> {
        schema
            .attributes
            .iter()
            .map(|a| (a.name.clone(), Rating::from_tenths(tenths)))
            .collect()
    }

    #[test]
    fn a_rating_ten_player_reads_the_anchors_exactly() {
        let content = shipped_content();
        let (s, t) = (&content.attributes, &content.tuning.engine);
        let (d, stages) =
            Derived::from_attributes(&Attributes::from_entry(&entry_at(s, 100), s), s, t);
        assert_eq!(d.max_speed, t.contract.speed.anchor_ms);
        assert_eq!(d.max_accel, t.contract.accel.anchor);
        assert_eq!(d.turn, 1.0);
        assert_eq!(d.knobs, Knobs::NEUTRAL);
        for i in 0..contract::STAGE_COUNT {
            assert_eq!(stages.f[i], 8.0, "{}", contract::STAGES[i].0.name());
            assert_eq!(stages.share[i], 0.5);
        }
        assert_eq!(d.gates, Gates::OPEN);
    }

    #[test]
    fn top_speed_follows_pace_through_the_map_and_a_missing_attribute_reads_the_floor() {
        let content = shipped_content();
        let (s, t) = (&content.attributes, &content.tuning.engine);
        let mut entry = entry_at(s, 100);
        entry.insert("pace".into(), Rating::from_tenths(200));
        let (fast, _) = Derived::from_attributes(&Attributes::from_entry(&entry, s), s, t);
        assert_eq!(fast.max_speed, t.contract.speed.top_speed(20.0));
        entry.remove("pace");
        let a = Attributes::from_entry(&entry, s);
        assert_eq!(a.get(s.index("pace").unwrap()).tenths(), MIN_TENTHS);
    }

    #[test]
    fn a_low_technique_closes_the_gates_and_a_low_agility_costs_the_execution() {
        let content = shipped_content();
        let (s, t) = (&content.attributes, &content.tuning.engine);
        let mut entry = entry_at(s, 100);
        entry.insert("technique".into(), Rating::from_tenths(50));
        entry.insert("agility".into(), Rating::from_tenths(50));
        let (d, _) = Derived::from_attributes(&Attributes::from_entry(&entry, s), s, t);
        assert!(!d.gates.chip_try && !d.gates.take_on_try);
        assert_eq!(d.gates.chip_penalty, s.gates.chip.penalty_k);
        assert_eq!(d.gates.take_on_penalty, s.gates.take_on.penalty_k);
    }
}

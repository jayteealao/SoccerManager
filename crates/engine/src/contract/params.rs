//! The contract block of the tuning file (`engine.contract`): the curve, each action's
//! strength and limits, the top-speed map, the acceleration anchor, the concentration
//! lapses, the state caps, the body jobs, consistency, and the values of the nearest hooks
//! through which the newer attributes reach play.

use std::collections::BTreeMap;
use std::fmt;

use garde::Validate;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{ACTION_COUNT, ActionKind};

/// The contract block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ContractTuning {
    #[garde(dive)]
    pub curve: CurveTuning,
    /// Per action: the strength `k` of its curve reads and, where the action uses them, the
    /// base chance, the floor and ceiling of a contest, and the spread of a per-player factor.
    #[garde(custom(check_actions))]
    pub actions: ActionMap<ActionParams>,
    #[garde(dive)]
    pub speed: SpeedTuning,
    #[garde(dive)]
    pub accel: AccelTuning,
    #[garde(dive)]
    pub lapse: LapseTuning,
    /// The caps on the states and the groups each state family acts on.
    #[garde(dive)]
    pub states: super::states::StatesTuning,
    /// The jobs of the body fields and the match condition inputs.
    #[garde(dive)]
    pub body: super::body::BodyJobs,
    /// The spread of each player's play around his ratings.
    #[garde(dive)]
    pub consistency: super::consistency::ConsistencyTuning,
    /// The values of the nearest hooks: braking, effort when behind, flair, the direct free
    /// kick, restart delivery, the back line's organisation, the space an attacker makes at
    /// his anchor, and a keeper's rash chases. Optional, so a tuning file written before them
    /// (as an old replay embeds it) still loads, and then every hook plays as before them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(dive)]
    pub hooks: Option<HookTuning>,
    /// The player as a person in play: mood, familiarity, the personality occasions and the
    /// shared floor. Optional, so a tuning file written before them still loads, and then
    /// mood stays 0, familiarity costs nothing, no occasion acts and no floor binds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(dive)]
    pub person: Option<PersonTuning>,
    /// The traits of play: each trait's weight on its choice and the chance a strong trait
    /// overrides an instruction it goes against. Optional, so a tuning file written before
    /// them still loads, and then no trait acts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(dive)]
    pub traits: Option<TraitTuning>,
    /// The weaker foot in play: the cost of a kick on the weaker side without time to shift
    /// the ball, and how far a closing defender shows a one-footed carrier onto it. Optional,
    /// so a tuning file written before it still loads, and then no kick pays and no defender
    /// shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(dive)]
    pub foot: Option<FootTuning>,
    /// The season at kick-off: how a player's recent form and his progress toward his season
    /// goal, weighted by his ambition, set his mood when the match starts. Optional, so a
    /// tuning file written before it still loads, and then every kick-off mood is 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(dive)]
    pub season: Option<SeasonTuning>,
}

/// The traits values a tuning file without a traits block reads: no weight, no chance.
static NO_TRAITS: TraitTuning = TraitTuning::NONE;

impl ContractTuning {
    /// The hook values: the file's, or [`HookTuning::NEUTRAL`] when it has none.
    #[inline]
    pub fn hooks(&self) -> HookTuning {
        self.hooks.unwrap_or(HookTuning::NEUTRAL)
    }

    /// The person values: the file's, or [`PersonTuning::NEUTRAL`] when it has none.
    #[inline]
    pub fn person(&self) -> PersonTuning {
        self.person.unwrap_or(PersonTuning::NEUTRAL)
    }

    /// The traits values: the file's, or [`TraitTuning::NONE`] when it has none.
    #[inline]
    pub fn traits(&self) -> &TraitTuning {
        self.traits.as_ref().unwrap_or(&NO_TRAITS)
    }

    /// The foot values: the file's, or [`FootTuning::NONE`] when it has none.
    #[inline]
    pub fn foot(&self) -> FootTuning {
        self.foot.unwrap_or(FootTuning::NONE)
    }

    /// The season values: the file's, or [`SeasonTuning::NEUTRAL`] when it has none.
    #[inline]
    pub fn season(&self) -> SeasonTuning {
        self.season.unwrap_or(SeasonTuning::NEUTRAL)
    }
}

/// The season block (`engine.contract.season`). A player's kick-off mood is his recent form
/// above or below `form_neutral` times `form_weight`, plus his progress toward his season goal
/// (from a full pace behind to a full pace ahead) times `goal_weight`, which his ambition
/// raises or lowers by up to `ambition_weight` of itself; the sum stays within `cap` either
/// side of neutral.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct SeasonTuning {
    /// The match rating (1 to 10) that counts as neither good nor bad form.
    #[garde(range(min = 1.0, max = 10.0))]
    pub form_neutral: f64,
    /// Mood points (of 100) per rating point of form above or below neutral.
    #[garde(range(min = 0.0, max = 50.0))]
    pub form_weight: f64,
    /// Mood points at a full pace ahead of (or behind) the season goal, at neutral ambition.
    #[garde(range(min = 0.0, max = 50.0))]
    pub goal_weight: f64,
    /// The share of the goal's weight the most ambitious player adds and the least ambitious
    /// loses; below 1, so the goal always counts the same way round.
    #[garde(range(min = 0.0, max = 0.9))]
    pub ambition_weight: f64,
    /// The most mood points the season gives or takes at kick-off.
    #[garde(range(min = 0.0, max = 100.0))]
    pub cap: f64,
}

impl SeasonTuning {
    /// No season at kick-off: every weight 0, so every kick-off mood is 0.
    pub const NEUTRAL: SeasonTuning = SeasonTuning {
        form_neutral: 6.5,
        form_weight: 0.0,
        goal_weight: 0.0,
        ambition_weight: 0.0,
        cap: 0.0,
    };

    /// The shipped season block; a test pins `content/tuning.json` to it.
    pub const fn shipped() -> SeasonTuning {
        SeasonTuning {
            form_neutral: 6.5,
            form_weight: 20.0,
            goal_weight: 15.0,
            ambition_weight: 0.6,
            cap: 40.0,
        }
    }
}

/// The foot block (`engine.contract.foot`). A kick to the side of a player's weaker foot,
/// without time to move the ball onto his strong one, loses up to `cost_cap` rating points on
/// its execute and pressure stages, in proportion to how much weaker the foot is and more
/// under pressure and over distance.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct FootTuning {
    /// The most rating points a kick by the weakest foot can lose; never above 5.
    #[garde(range(min = 0.0, max = 5.0))]
    pub cost_cap: f64,
    /// The share of the cost every weaker-side kick without time pays.
    #[garde(range(min = 0.0, max = 1.0))]
    pub base: f64,
    /// The share the closest pressure adds.
    #[garde(range(min = 0.0, max = 1.0))]
    pub pressure_weight: f64,
    /// The share the longest range adds.
    #[garde(range(min = 0.0, max = 1.0))]
    pub range_weight: f64,
    /// The nearest opponent adds no pressure from this many metres away, all of it at 0.
    #[garde(range(min = 0.5, max = 20.0))]
    pub pressure_m: f64,
    /// A kick this many metres long or longer adds all of the range share.
    #[garde(range(min = 5.0, max = 80.0))]
    pub range_m: f64,
    /// Within this many degrees of straight ahead either foot kicks: the strong one.
    #[garde(range(min = 0.0, max = 90.0))]
    pub either_deg: f64,
    /// Seconds an unpressed carrier needs on the ball to move it onto his strong foot.
    #[garde(range(min = 0.0, max = 5.0))]
    pub shift_s: f64,
    /// Metres a closing presser stands toward the strong side of the most one-footed
    /// carrier.
    #[garde(range(min = 0.0, max = 5.0))]
    pub show_m: f64,
}

impl FootTuning {
    /// No foot in play: no kick pays and no presser shows.
    pub const NONE: FootTuning = FootTuning {
        cost_cap: 0.0,
        base: 0.0,
        pressure_weight: 0.0,
        range_weight: 0.0,
        pressure_m: 5.0,
        range_m: 40.0,
        either_deg: 20.0,
        shift_s: 0.6,
        show_m: 0.0,
    };

    /// The shipped foot block; a test pins `content/tuning.json` to it.
    pub const fn shipped() -> FootTuning {
        FootTuning {
            cost_cap: 5.0,
            base: 0.4,
            pressure_weight: 0.3,
            range_weight: 0.3,
            pressure_m: 5.0,
            range_m: 40.0,
            either_deg: 20.0,
            shift_s: 0.6,
            show_m: 1.5,
        }
    }
}

/// The person block (`engine.contract.person`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PersonTuning {
    #[garde(dive)]
    pub mood: MoodTuning,
    #[garde(dive)]
    pub familiarity: FamiliarityTuning,
    #[garde(dive)]
    pub occasions: OccasionTuning,
    /// The most rating points every temporary effect together may take from a stage: no
    /// stage reads below its base value less this many points on the curve.
    #[garde(range(min = 0.5, max = 12.7))]
    pub floor_drop: f64,
}

impl PersonTuning {
    /// No person values: no impulse, no drift, no mood weight, no familiarity cost, no
    /// occasion, and no floor.
    pub const NEUTRAL: PersonTuning = PersonTuning {
        mood: MoodTuning {
            goal: 0.0,
            assist: 0.0,
            big_miss: 0.0,
            error: 0.0,
            card: 0.0,
            red_card: 0.0,
            fouled: 0.0,
            duel_won: 0.0,
            duel_lost: 0.0,
            team_goal_for: 0.0,
            team_goal_against: 0.0,
            big_chance_xg: 1.0,
            drift_per_minute: 0.0,
            score_weight: 0.0,
            late_from_minute: 120,
            late_pressure: 0.0,
            temperament_weight: 0.0,
            pressure_weight: 0.0,
            leadership_weight: 0.0,
            own_weight: 0.0,
            team_weight: 0.0,
            risk: 0.0,
        },
        familiarity: FamiliarityTuning {
            cost_per_point: 0.0,
            versatility_weight: 0.0,
            adaptability_weight: 0.0,
        },
        occasions: OccasionTuning {
            professionalism: 0.0,
            penalty_pressure: 0.0,
            temper_foul: 0.0,
            cynical_foul: 0.0,
            dive_foul: 0.0,
            adaptation: 0.0,
        },
        floor_drop: f64::INFINITY,
    };
}

/// Mood: the impulse of each event in percent of a full mood, the drift, the swing and how
/// mood reaches play.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct MoodTuning {
    /// The scorer.
    #[garde(range(min = -100.0, max = 100.0))]
    pub goal: f64,
    /// The player whose completed pass reached the scorer.
    #[garde(range(min = -100.0, max = 100.0))]
    pub assist: f64,
    /// The shooter of a big chance that ended without a goal.
    #[garde(range(min = -100.0, max = 100.0))]
    pub big_miss: f64,
    /// The passer whose open-play pass the other side controlled.
    #[garde(range(min = -100.0, max = 100.0))]
    pub error: f64,
    /// A player shown a yellow card.
    #[garde(range(min = -100.0, max = 100.0))]
    pub card: f64,
    /// A player sent off.
    #[garde(range(min = -100.0, max = 100.0))]
    pub red_card: f64,
    /// A fouled player.
    #[garde(range(min = -100.0, max = 100.0))]
    pub fouled: f64,
    /// The tackler who won the ball.
    #[garde(range(min = -100.0, max = 100.0))]
    pub duel_won: f64,
    /// The carrier who lost it.
    #[garde(range(min = -100.0, max = 100.0))]
    pub duel_lost: f64,
    /// Every player of the scoring side.
    #[garde(range(min = -100.0, max = 100.0))]
    pub team_goal_for: f64,
    /// Every player of the conceding side.
    #[garde(range(min = -100.0, max = 100.0))]
    pub team_goal_against: f64,
    /// A shot of at least these expected goals that ends without a goal is a big miss.
    #[garde(range(min = 0.05, max = 1.0))]
    pub big_chance_xg: f64,
    /// How far a mood drifts toward its target in a minute, percent.
    #[garde(range(min = 0.0, max = 50.0))]
    pub drift_per_minute: f64,
    /// The drift target per goal of lead, at most two goals either way, percent.
    #[garde(range(min = 0.0, max = 50.0))]
    pub score_weight: f64,
    /// The minute from which a game within one goal weighs on a player.
    #[garde(range(min = 0, max = 120))]
    pub late_from_minute: u32,
    /// How far that lowers the drift target of the player who handles pressure worst,
    /// percent; half of it for an average player.
    #[garde(range(min = 0.0, max = 50.0))]
    pub late_pressure: f64,
    /// How much temperament narrows the swing.
    #[garde(range(min = 0.0, max = 0.9))]
    pub temperament_weight: f64,
    /// How much pressure handling narrows the swing.
    #[garde(range(min = 0.0, max = 0.9))]
    pub pressure_weight: f64,
    /// How much leadership adds to a player's weight in his side's mood.
    #[garde(range(min = 0.0, max = 3.0))]
    pub leadership_weight: f64,
    /// Rating points on the mental group at a full own mood.
    #[garde(range(min = 0.0, max = 3.0))]
    pub own_weight: f64,
    /// Rating points on the mental group at a full team mood.
    #[garde(range(min = 0.0, max = 3.0))]
    pub team_weight: f64,
    /// How much a full mood raises the score of a shot and of a forward pass and lowers the
    /// score of holding the ball; a low mood the reverse.
    #[garde(range(min = 0.0, max = 2.0))]
    pub risk: f64,
}

/// Familiarity: the cost of playing away from the natural position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct FamiliarityTuning {
    /// Rating points lost per point of gap between the natural rating and the rating at the
    /// position played, before the family cap.
    #[garde(range(min = 0.0, max = 1.0))]
    pub cost_per_point: f64,
    /// How much versatility narrows the cost.
    #[garde(range(min = 0.0, max = 0.9))]
    pub versatility_weight: f64,
    /// How much adaptability narrows the cost.
    #[garde(range(min = 0.0, max = 0.9))]
    pub adaptability_weight: f64,
}

/// The personality occasions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct OccasionTuning {
    /// How much professionalism lowers the chance of a concentration lapse.
    #[garde(range(min = 0.0, max = 0.9))]
    pub professionalism: f64,
    /// How much pressure handling narrows a penalty's spread.
    #[garde(range(min = 0.0, max = 0.9))]
    pub penalty_pressure: f64,
    /// Log-odds a tackler in a full low mood adds to his foul chance, for an average
    /// temperament.
    #[garde(range(min = 0.0, max = 3.0))]
    pub temper_foul: f64,
    /// Log-odds an unsporting tackler adds to his foul chance on a running carrier, per unit
    /// of sportsmanship skill below the average.
    #[garde(range(min = 0.0, max = 3.0))]
    pub cynical_foul: f64,
    /// Log-odds an unsporting carrier adds to the foul chance of a tackle on him (he goes
    /// down), per unit of sportsmanship skill below the average.
    #[garde(range(min = 0.0, max = 3.0))]
    pub dive_foul: f64,
    /// How much adaptability narrows the adaptation drop.
    #[garde(range(min = 0.0, max = 0.9))]
    pub adaptation: f64,
}

/// The values of the nearest hooks (`engine.contract.hooks`). Each hook is neutral for a
/// rating-10 player, so the average player plays as before and the spread between players is
/// what moves; the direct free kick and the restart delivery noise are the exceptions, since
/// play had neither before.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct HookTuning {
    /// The braking of a player whose turn stage is rating 10, m/s²; his turn factor (agility
    /// leading) scales it.
    #[garde(range(min = 0.5, max = 30.0))]
    pub decel_anchor: f64,
    /// How much a presser's determination widens his press reach while his side trails.
    #[garde(range(min = 0.0, max = 1.0))]
    pub behind_effort: f64,
    /// How much a carrier's flair raises the score of a forward pass.
    #[garde(range(min = 0.0, max = 2.0))]
    pub flair_risk: f64,
    /// How many rating points of technique the take-on and chip gates drop for a top flair
    /// player.
    #[garde(range(min = 0.0, max = 5.0))]
    pub flair_gate: f64,
    /// A direct free kick within this many metres of goal is shot; beyond it, delivered.
    #[garde(range(min = 0.0, max = 40.0))]
    pub free_kick_range_m: f64,
    /// The largest turn of a corner or free-kick delivery by an average taker, radians.
    #[garde(range(min = 0.0, max = 0.5))]
    pub restart_noise: f64,
    /// The weight of the back line's own organisation beside the keeper's.
    #[garde(range(min = 0.0, max = 1.0))]
    pub organise_back_line: f64,
    /// How far an attacker with good off-ball movement moves his anchor away from the
    /// nearest defender, metres at the top of the scale.
    #[garde(range(min = 0.0, max = 6.0))]
    pub anchor_space_m: f64,
    /// How much a keeper's eccentricity widens the range from which he comes off his line.
    #[garde(range(min = 0.0, max = 1.0))]
    pub rash_chase: f64,
    /// How far a top presser leans his aim goal-side of the carrier, metres; his press choose
    /// skill (−1 to 1, 0 at rating 10) scales it, so a poor presser comes from the open side.
    /// Absent in a file written before it, and then 0: no lean.
    #[serde(default)]
    #[garde(range(min = 0.0, max = 6.0))]
    pub press_goal_side_m: f64,
}

impl HookTuning {
    /// Every hook as play was before the hooks: braking with the acceleration, no effort when
    /// behind, no flair term, no direct free kick taken, no delivery noise, the keeper alone
    /// organising the line, no anchor space, no rash chase and no press lean.
    pub const NEUTRAL: HookTuning = HookTuning {
        decel_anchor: 0.0,
        behind_effort: 0.0,
        flair_risk: 0.0,
        flair_gate: 0.0,
        free_kick_range_m: 0.0,
        restart_noise: 0.0,
        organise_back_line: 0.0,
        anchor_space_m: 0.0,
        rash_chase: 0.0,
        press_goal_side_m: 0.0,
    };
}

/// Declares a tuning table of one `f64` per row, each held to its range by the tuning check,
/// with `ZERO` and its lookups by [`crate::traits::TraitKind`] from the same list, so a new
/// trait is one line here. A row names its field (the trait's catalogue name), then `=>` and
/// the trait it belongs to, or nothing for a value of the table that is no trait's.
macro_rules! trait_table {
    (
        $(#[doc = $tdoc:literal])*
        $table:ident {
            $( $(#[doc = $doc:literal])* $field:ident $(=> $kind:ident)? in $min:literal..=$max:literal, )*
        }
    ) => {
        $(#[doc = $tdoc])*
        #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
        #[serde(deny_unknown_fields)]
        pub struct $table {
            $(
                $(#[doc = $doc])*
                #[garde(range(min = $min, max = $max))]
                pub $field: f64,
            )*
        }

        impl $table {
            /// Every value 0.
            pub const ZERO: $table = $table { $($field: 0.0,)* };

            /// The value of trait `k`; 0 for a trait the table does not list.
            pub fn of(&self, k: crate::traits::TraitKind) -> f64 {
                #[allow(unreachable_patterns)]
                match k {
                    $($(crate::traits::TraitKind::$kind => self.$field,)?)*
                    _ => 0.0,
                }
            }

            /// Mutable access to the value of trait `k`, for a test that plants one; `None` for
            /// a trait the table does not list.
            pub fn of_mut(&mut self, k: crate::traits::TraitKind) -> Option<&mut f64> {
                #[allow(unreachable_patterns)]
                match k {
                    $($(crate::traits::TraitKind::$kind => Some(&mut self.$field),)?)*
                    _ => None,
                }
            }
        }
    };
}

/// The traits block (`engine.contract.traits`): how much each trait of play weighs on its
/// choice, and the chance a strong trait overrides an instruction it goes against.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct TraitTuning {
    /// The share of a trait's weight a moderate trait plays at; a strong one plays all of it.
    #[garde(range(min = 0.0, max = 0.9))]
    pub moderate_share: f64,
    /// A mood at or below this, percent of a full mood, is a low mood for an override.
    #[garde(range(min = -100.0, max = 0.0))]
    pub low_mood: f64,
    /// Seconds an override that holds or carries the ball lasts.
    #[garde(range(min = 0.2, max = 5.0))]
    pub carry_s: f64,
    #[garde(dive)]
    pub weights: TraitWeights,
    #[garde(dive)]
    pub override_chance: OverrideChances,
}

impl TraitTuning {
    /// No trait acts: every weight and every chance 0.
    pub const NONE: TraitTuning = TraitTuning {
        moderate_share: 0.0,
        low_mood: -25.0,
        carry_s: 1.5,
        weights: TraitWeights::ZERO,
        override_chance: OverrideChances::ZERO,
    };

    /// The shipped traits block; a test pins `content/tuning.json` to it.
    pub const fn shipped() -> TraitTuning {
        TraitTuning {
            moderate_share: 0.5,
            low_mood: -25.0,
            carry_s: 1.5,
            weights: TraitWeights {
                cuts_inside: 0.6,
                hugs_touchline: 0.6,
                shoots_from_distance: 0.6,
                places_shots: 0.25,
                powers_shots: 0.2,
                tries_lobs_and_chips: 0.4,
                tries_killer_balls: 0.8,
                plays_short_simple_passes: 0.4,
                switches_play: 0.4,
                crosses_early: 0.4,
                runs_with_ball_often: 0.3,
                rarely_dribbles: 0.3,
                tries_tricks: 0.4,
                tries_tricks_gate: 2.0,
                plays_one_twos: 0.4,
                holds_up_ball: 0.4,
                releases_ball_early: 0.3,
                dwells_on_ball: 0.3,
                gets_forward: 5.0,
                stays_back: 5.0,
                arrives_late_in_box: 6.0,
                moves_into_channels: 4.0,
                presses_relentlessly: 0.3,
                dives_into_tackles: 0.4,
                stays_on_feet: 0.4,
                goes_down_easily: 0.4,
                argues_with_officials: 0.05,
                rushes_out: 0.3,
                stays_on_line: 0.3,
                punches_rather_than_catches: 0.25,
                kicks_long: 0.4,
                throws_short: 0.4,
            },
            override_chance: OverrideChances {
                tries_killer_balls: 0.0,
                tries_lobs_and_chips: 0.0,
                crosses_early: 0.0,
                kicks_long: 0.0,
                plays_short_simple_passes: 0.0,
                plays_one_twos: 0.0,
                throws_short: 0.0,
                dwells_on_ball: 0.12,
                runs_with_ball_often: 0.0,
                holds_up_ball: 0.028,
                tries_tricks: 0.0,
                releases_ball_early: 0.28,
                cuts_inside: 1.0,
                hugs_touchline: 1.0,
                switches_play: 0.0,
                presses_relentlessly: 0.19,
                gets_forward: 1.0,
                arrives_late_in_box: 0.027,
                stays_back: 0.17,
                shoots_from_distance: 0.0,
                rushes_out: 0.065,
                stays_on_line: 0.0,
            },
        }
    }
}

trait_table! {
    /// Each trait's weight on its choice. Score weights add to an option's score; metres move a
    /// target; shares scale a range, a reach, a speed or a turn; log-odds move a foul chance;
    /// a chance moves a card or a hold chance. The take-on gate's points for tricks are
    /// `tries_tricks_gate`, a value of the table that is no trait's weight.
    TraitWeights {
        /// Share of a wide carry's sideways part turned toward the centre.
        cuts_inside => CutsInside in 0.0..=1.0,
        /// Share of a wide carry's sideways part turned toward the touchline.
        hugs_touchline => HugsTouchline in 0.0..=1.0,
        /// Score added to a shot from outside the penalty area, at the edge of the shooting range.
        shoots_from_distance => ShootsFromDistance in 0.0..=3.0,
        /// Share a placed shot aims nearer the post and loses in speed.
        places_shots => PlacesShots in 0.0..=0.9,
        /// Share a powered shot gains in speed and in spread.
        powers_shots => PowersShots in 0.0..=0.9,
        /// Score added to a chip.
        tries_lobs_and_chips => TriesLobsAndChips in 0.0..=3.0,
        /// Score added to a forward pass through a tight lane, at a full forward pass.
        tries_killer_balls => TriesKillerBalls in 0.0..=3.0,
        /// Score added to a short pass, and taken from a forward one.
        plays_short_simple_passes => PlaysShortSimplePasses in 0.0..=3.0,
        /// Score added to a switch: a pass over 25 m that crosses 25 m of the pitch's width.
        switches_play => SwitchesPlay in 0.0..=3.0,
        /// Score added to a cross from outside the final 18 m.
        crosses_early => CrossesEarly in 0.0..=3.0,
        /// Score added to the dribble.
        runs_with_ball_often => RunsWithBallOften in 0.0..=3.0,
        /// Score taken from the dribble.
        rarely_dribbles => RarelyDribbles in 0.0..=3.0,
        /// Score added to a dribble with an opponent close.
        tries_tricks => TriesTricks in 0.0..=3.0,
        /// Rating points of technique the take-on gate drops for a player who tries tricks.
        tries_tricks_gate in 0.0..=5.0,
        /// Score added to a pass back to the team-mate who passed to him.
        plays_one_twos => PlaysOneTwos in 0.0..=3.0,
        /// Score added to holding the ball with an opponent close.
        holds_up_ball => HoldsUpBall in 0.0..=3.0,
        /// Score taken from holding the ball, and from the carry window.
        releases_ball_early => ReleasesBallEarly in 0.0..=3.0,
        /// Score added to holding the ball; it also slows the hold's fall with time on the ball.
        dwells_on_ball => DwellsOnBall in 0.0..=3.0,
        /// Metres his anchor moves up the pitch while his side has the ball.
        gets_forward => GetsForward in 0.0..=15.0,
        /// Metres his anchor moves back while his side has the ball.
        stays_back => StaysBack in 0.0..=15.0,
        /// Metres his target moves toward goal while his side has the ball in the final third.
        arrives_late_in_box => ArrivesLateInBox in 0.0..=15.0,
        /// Metres his anchor moves toward the channel between full-back and centre-back while his
        /// side has the ball.
        moves_into_channels => MovesIntoChannels in 0.0..=15.0,
        /// Share his press reach widens by.
        presses_relentlessly => PressesRelentlessly in 0.0..=1.0,
        /// Log-odds his tackle's win and foul chances rise by.
        dives_into_tackles => DivesIntoTackles in 0.0..=3.0,
        /// Log-odds his tackle's foul chance falls by.
        stays_on_feet => StaysOnFeet in 0.0..=3.0,
        /// Log-odds the foul chance of a tackle on him rises by.
        goes_down_easily => GoesDownEasily in 0.0..=3.0,
        /// Chance added to his yellow-card chance on a foul (dissent).
        argues_with_officials => ArguesWithOfficials in 0.0..=0.5,
        /// Share a keeper's chase range widens by.
        rushes_out => RushesOut in 0.0..=0.9,
        /// Share a keeper's chase range narrows by.
        stays_on_line => StaysOnLine in 0.0..=0.9,
        /// Chance taken from a keeper's hold of a save: he punches it away instead.
        punches_rather_than_catches => PunchesRatherThanCatches in 0.0..=0.9,
        /// Score added to a keeper's kick over 30 m.
        kicks_long => KicksLong in 0.0..=3.0,
        /// Score added to a keeper's throw.
        throws_short => ThrowsShort in 0.0..=3.0,
    }
}

trait_table! {
    /// The chance per moment that a strong trait overrides the instruction it goes against. A
    /// trait outside the conflict table ([`crate::traits::TraitKind::conflicts`]) never meets a
    /// moment, so its chance is not read; the shipped file holds 0 for it.
    OverrideChances {
        tries_killer_balls => TriesKillerBalls in 0.0..=1.0,
        tries_lobs_and_chips => TriesLobsAndChips in 0.0..=1.0,
        crosses_early => CrossesEarly in 0.0..=1.0,
        kicks_long => KicksLong in 0.0..=1.0,
        plays_short_simple_passes => PlaysShortSimplePasses in 0.0..=1.0,
        plays_one_twos => PlaysOneTwos in 0.0..=1.0,
        throws_short => ThrowsShort in 0.0..=1.0,
        dwells_on_ball => DwellsOnBall in 0.0..=1.0,
        runs_with_ball_often => RunsWithBallOften in 0.0..=1.0,
        holds_up_ball => HoldsUpBall in 0.0..=1.0,
        tries_tricks => TriesTricks in 0.0..=1.0,
        releases_ball_early => ReleasesBallEarly in 0.0..=1.0,
        cuts_inside => CutsInside in 0.0..=1.0,
        hugs_touchline => HugsTouchline in 0.0..=1.0,
        switches_play => SwitchesPlay in 0.0..=1.0,
        presses_relentlessly => PressesRelentlessly in 0.0..=1.0,
        gets_forward => GetsForward in 0.0..=1.0,
        arrives_late_in_box => ArrivesLateInBox in 0.0..=1.0,
        stays_back => StaysBack in 0.0..=1.0,
        shoots_from_distance => ShootsFromDistance in 0.0..=1.0,
        rushes_out => RushesOut in 0.0..=1.0,
        stays_on_line => StaysOnLine in 0.0..=1.0,
    }
}

/// The curve `F(r) = scale · e^((r − center) / width)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CurveTuning {
    #[garde(range(min = 0.1, max = 100.0))]
    pub scale: f64,
    #[garde(range(min = 1.0, max = 20.0))]
    pub center: f64,
    #[garde(range(min = 1.0, max = 100.0))]
    pub width: f64,
}

/// One action's values. Which fields an action carries is fixed by what play reads
/// ([`uses`]); a field the action does not read is refused, and so is a missing one.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionParams {
    /// Log-odds per point of the curve, in a contest and in a skill share.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub k: Option<f64>,
    /// The chance of a contest between two equal players.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<f64>,
    /// The lowest and highest chance a contest may give.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ceiling: Option<f64>,
    /// How far a per-player factor moves an average player's value, either way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spread: Option<f64>,
}

impl ActionParams {
    /// The strength, 0 when the action has none.
    #[inline]
    pub fn k(&self) -> f64 {
        self.k.unwrap_or(0.0)
    }

    /// The floor, 0 when the action has none.
    #[inline]
    pub fn floor(&self) -> f64 {
        self.floor.unwrap_or(0.0)
    }

    /// The ceiling, 1 when the action has none.
    #[inline]
    pub fn ceiling(&self) -> f64 {
        self.ceiling.unwrap_or(1.0)
    }

    /// The spread, 0 when the action has none.
    #[inline]
    pub fn spread(&self) -> f64 {
        self.spread.unwrap_or(0.0)
    }

    /// The base, 0 when the action has none.
    #[inline]
    pub fn base(&self) -> f64 {
        self.base.unwrap_or(0.0)
    }
}

/// Which of the five values play reads for `action`: `[k, base, floor and ceiling, spread]`.
pub fn uses(action: ActionKind) -> [bool; 4] {
    use ActionKind as A;
    // k, base, limits, spread
    match action {
        A::Header => [true, true, true, false],
        A::Tackle | A::StayUp | A::Hold | A::Save | A::OneOnOne => [true, false, true, false],
        A::Claim => [true, false, true, true],
        A::Receive
        | A::Intercept
        | A::Press
        | A::Shape
        | A::Block
        | A::Sprint
        | A::Turn
        | A::AerialReach
        | A::Organise
        | A::Rush => [true, false, false, true],
        A::Shield => [false, false, false, false],
        // The five actions the record added and the two defensive jobs; each may be absent
        // ([`neutral_when_absent`]).
        A::Mark | A::OffBallRun | A::LongThrow | A::Cover | A::RecoveryRun => {
            [true, false, false, true]
        }
        A::Corner | A::FreeKick => [true, false, false, false],
        A::Pass
        | A::Chip
        | A::Cross
        | A::Shot
        | A::LongShot
        | A::Penalty
        | A::Dribble
        | A::Endure
        | A::Recover
        | A::Injury
        | A::KeeperKick
        | A::KeeperThrow => [true, false, false, false],
    }
}

/// `true` for an action a tuning file may leave out, which then reads neutral (strength 0:
/// share 0.5, factor 1): the five actions the record added and the cover and recovery-run
/// stages, so a tuning file written before they had values (as an old replay embeds it) still
/// loads and plays them neutral.
pub fn neutral_when_absent(action: ActionKind) -> bool {
    use ActionKind as A;
    matches!(
        action,
        A::Mark
            | A::OffBallRun
            | A::Corner
            | A::FreeKick
            | A::LongThrow
            | A::Cover
            | A::RecoveryRun
    )
}

/// One value per action, read by index on the hot path; in a file, an object keyed by the
/// action's name. An action whose values play does not read has no entry.
#[derive(Clone, Copy, PartialEq)]
pub struct ActionMap<T>(pub [Option<T>; ACTION_COUNT]);

impl<T: Copy> ActionMap<T> {
    /// The value of `action`, if it has one.
    #[inline]
    pub fn get(&self, action: ActionKind) -> Option<&T> {
        self.0[action as usize].as_ref()
    }
}

impl ActionMap<ActionParams> {
    /// The values of `action`; an action with no entry reads as all absent.
    #[inline]
    pub fn of(&self, action: ActionKind) -> ActionParams {
        self.0[action as usize].unwrap_or(ActionParams {
            k: None,
            base: None,
            floor: None,
            ceiling: None,
            spread: None,
        })
    }
}

impl<T: fmt::Debug> fmt::Debug for ActionMap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(
                ActionKind::ALL
                    .iter()
                    .zip(&self.0)
                    .filter_map(|(a, v)| v.as_ref().map(|v| (a.name(), v))),
            )
            .finish()
    }
}

impl<T: Serialize> Serialize for ActionMap<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let map: BTreeMap<&str, &T> = ActionKind::ALL
            .iter()
            .zip(&self.0)
            .filter_map(|(a, v)| v.as_ref().map(|v| (a.name(), v)))
            .collect();
        map.serialize(s)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for ActionMap<T> {
    /// Refuses a name that is not an action.
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let map = BTreeMap::<String, T>::deserialize(d)?;
        let mut out: [Option<T>; ACTION_COUNT] = std::array::from_fn(|_| None);
        for (name, v) in map {
            let a = ActionKind::from_name(&name)
                .ok_or_else(|| D::Error::custom(format!("{name} is not an action")))?;
            out[a as usize] = Some(v);
        }
        Ok(Self(out))
    }
}

/// Each action carries exactly the values play reads, each in its range.
fn check_actions(map: &ActionMap<ActionParams>, _ctx: &()) -> garde::Result {
    for a in ActionKind::ALL {
        let [k, base, limits, spread] = uses(a);
        let name = a.name();
        let Some(p) = map.get(a) else {
            if (k || base || limits || spread) && !neutral_when_absent(a) {
                return Err(garde::Error::new(format!("action {name} is missing")));
            }
            continue;
        };
        if !(k || base || limits || spread) {
            return Err(garde::Error::new(format!(
                "action {name}: play reads no value of it; remove the entry"
            )));
        }
        let fields = [
            ("k", p.k, k, 0.0, 5.0),
            ("base", p.base, base, 0.0, 1.0),
            ("floor", p.floor, limits, 0.0, 1.0),
            ("ceiling", p.ceiling, limits, 0.0, 1.0),
            ("spread", p.spread, spread, 0.0, 1.0),
        ];
        for (field, value, read, lo, hi) in fields {
            match (value, read) {
                (None, true) => {
                    return Err(garde::Error::new(format!(
                        "action {name}: {field} is missing"
                    )));
                }
                (Some(_), false) => {
                    return Err(garde::Error::new(format!(
                        "action {name}: {field} is not read by play; remove it"
                    )));
                }
                (Some(v), true) if !(lo..=hi).contains(&v) => {
                    return Err(garde::Error::new(format!(
                        "action {name}: {field} is {v}; allowed {lo} to {hi}"
                    )));
                }
                _ => {}
            }
        }
        if limits && p.floor() >= p.ceiling() {
            return Err(garde::Error::new(format!(
                "action {name}: floor {} is not below ceiling {}",
                p.floor(),
                p.ceiling()
            )));
        }
    }
    Ok(())
}

/// Top speed: real sprint speeds in km/h per pace rating, their differences amplified around
/// the average player's engine speed. Pace does not pass through the curve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct SpeedTuning {
    /// `[pace, km/h]` points, pace rising from 1 to 20, km/h rising; straight lines between.
    #[garde(custom(check_kmh))]
    pub kmh: Vec<[f64; 2]>,
    /// How many times the real difference from a pace-10 player the engine speed differs.
    #[garde(range(min = 0.0, max = 10.0))]
    pub amplification: f64,
    /// The engine top speed of a pace-10 player, in metres per second.
    #[garde(range(min = 1.0, max = 15.0))]
    pub anchor_ms: f64,
}

impl SpeedTuning {
    /// The real sprint speed in km/h of a player of pace `r`.
    pub fn kmh(&self, r: f64) -> f64 {
        let pts = &self.kmh;
        if r <= pts[0][0] {
            return pts[0][1];
        }
        for w in pts.windows(2) {
            let ([r0, v0], [r1, v1]) = (w[0], w[1]);
            if r <= r1 {
                return v0 + (v1 - v0) * (r - r0) / (r1 - r0);
            }
        }
        pts[pts.len() - 1][1]
    }

    /// The engine top speed, in metres per second, of a player of pace `r`.
    pub fn top_speed(&self, r: f64) -> f64 {
        self.anchor_ms + self.amplification * (self.kmh(r) - self.kmh(10.0)) / 3.6
    }
}

fn check_kmh(points: &[[f64; 2]], _ctx: &()) -> garde::Result {
    if !(2..=8).contains(&points.len()) {
        return Err(garde::Error::new(format!(
            "holds {} points; allowed 2 to 8",
            points.len()
        )));
    }
    for (i, [r, v]) in points.iter().enumerate() {
        if !(1.0..=20.0).contains(r) || !(10.0..=50.0).contains(v) {
            return Err(garde::Error::new(format!(
                "point {i}: [{r}, {v}]; pace 1 to 20 and 10 to 50 km/h"
            )));
        }
        if i > 0 && (*r <= points[i - 1][0] || *v < points[i - 1][1]) {
            return Err(garde::Error::new(format!(
                "point {i}: pace must rise and speed must not fall"
            )));
        }
    }
    Ok(())
}

/// Acceleration: the average player's, which the sprint stage scales.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct AccelTuning {
    /// Metres per second squared of a player whose sprint stage is rating 10.
    #[garde(range(min = 0.5, max = 30.0))]
    pub anchor: f64,
}

/// Concentration lapses: rare moments a defender stops tracking his place, more often late
/// in a match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct LapseTuning {
    /// The chance per minute that an average defender lapses, before the late growth.
    #[garde(range(min = 0.0, max = 0.2))]
    pub per_minute: f64,
    /// The minute from which the chance grows.
    #[garde(range(min = 0, max = 120))]
    pub late_from_minute: u32,
    /// How much the chance grows per 30 minutes past `late_from_minute`, as a share.
    #[garde(range(min = 0.0, max = 5.0))]
    pub late_growth: f64,
    /// How many ticks a lapse lasts.
    #[garde(range(min = 1, max = 1000))]
    pub ticks: u32,
}

impl Default for ContractTuning {
    /// The shipped contract block; a test pins `content/tuning.json` to it.
    fn default() -> Self {
        use ActionKind as A;
        let knob = |k: f64, spread: f64| ActionParams {
            spread: Some(spread),
            ..self::k_only(k)
        };
        let contest = |k: f64, floor: f64, ceiling: f64| ActionParams {
            floor: Some(floor),
            ceiling: Some(ceiling),
            ..self::k_only(k)
        };
        let mut actions: [Option<ActionParams>; ACTION_COUNT] = [None; ACTION_COUNT];
        for a in ActionKind::ALL {
            actions[a as usize] = match a {
                A::Header => Some(ActionParams {
                    base: Some(0.6),
                    ..contest(0.21, 0.05, 0.95)
                }),
                A::Tackle | A::StayUp => Some(contest(0.21, 0.02, 0.97)),
                A::Hold => Some(contest(0.03, 0.02, 0.97)),
                A::Save | A::OneOnOne => Some(contest(0.012, 0.02, 0.97)),
                A::Claim => Some(ActionParams {
                    spread: Some(0.2),
                    ..contest(0.12, 0.02, 0.97)
                }),
                A::Receive
                | A::Intercept
                | A::Press
                | A::Shape
                | A::Block
                | A::Sprint
                | A::Turn
                | A::AerialReach
                | A::Organise
                | A::Rush => Some(knob(0.054, 0.2)),
                A::Shield => None,
                A::Mark => Some(knob(0.054, 0.4)),
                A::OffBallRun => Some(knob(0.054, 0.2)),
                A::LongThrow => Some(knob(0.054, 0.5)),
                A::Cover => Some(knob(0.054, 0.4)),
                A::RecoveryRun => Some(knob(0.054, 0.2)),
                A::Corner | A::FreeKick => Some(k_only(0.054)),
                A::Shot => Some(k_only(0.07)),
                A::LongShot => Some(k_only(0.02)),
                A::Dribble => Some(k_only(0.071)),
                _ => Some(k_only(0.054)),
            };
        }
        Self {
            curve: CurveTuning {
                scale: 8.0,
                center: 10.0,
                width: 8.0,
            },
            actions: ActionMap(actions),
            speed: SpeedTuning {
                kmh: vec![[1.0, 29.0], [10.0, 32.0], [20.0, 35.5]],
                amplification: 2.0,
                anchor_ms: 7.0,
            },
            accel: AccelTuning { anchor: 5.5 },
            lapse: LapseTuning {
                per_minute: 0.01,
                late_from_minute: 60,
                late_growth: 1.0,
                ticks: 100,
            },
            states: super::states::StatesTuning::default(),
            body: super::body::BodyJobs::default(),
            consistency: super::consistency::ConsistencyTuning {
                match_max: 1.0,
                period_max: 0.75,
                period_minutes: 10,
            },
            hooks: Some(HookTuning {
                decel_anchor: 5.5,
                behind_effort: 0.3,
                flair_risk: 0.4,
                flair_gate: 2.0,
                free_kick_range_m: 30.0,
                restart_noise: 0.06,
                organise_back_line: 0.5,
                anchor_space_m: 2.0,
                rash_chase: 0.3,
                press_goal_side_m: 2.0,
            }),
            person: Some(PersonTuning {
                mood: MoodTuning {
                    goal: 30.0,
                    assist: 15.0,
                    big_miss: -20.0,
                    error: -4.0,
                    card: -10.0,
                    red_card: -25.0,
                    fouled: 3.0,
                    duel_won: 3.0,
                    duel_lost: -3.0,
                    team_goal_for: 8.0,
                    team_goal_against: -10.0,
                    big_chance_xg: 0.3,
                    drift_per_minute: 4.0,
                    score_weight: 15.0,
                    late_from_minute: 75,
                    late_pressure: 10.0,
                    temperament_weight: 0.5,
                    pressure_weight: 0.4,
                    leadership_weight: 1.0,
                    own_weight: 0.8,
                    team_weight: 0.4,
                    risk: 0.25,
                },
                familiarity: FamiliarityTuning {
                    cost_per_point: 0.15,
                    versatility_weight: 0.5,
                    adaptability_weight: 0.3,
                },
                occasions: OccasionTuning {
                    professionalism: 0.5,
                    penalty_pressure: 0.3,
                    temper_foul: 0.6,
                    cynical_foul: 0.4,
                    dive_foul: 0.3,
                    adaptation: 0.3,
                },
                floor_drop: 7.0,
            }),
            traits: Some(TraitTuning::shipped()),
            foot: Some(FootTuning::shipped()),
            season: Some(SeasonTuning::shipped()),
        }
    }
}

/// Values with only a strength.
const fn k_only(k: f64) -> ActionParams {
    ActionParams {
        k: Some(k),
        base: None,
        floor: None,
        ceiling: None,
        spread: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_block_validates_and_every_action_carries_what_play_reads() {
        assert!(ContractTuning::default().validate().is_ok());
    }

    #[test]
    fn a_value_play_does_not_read_is_refused_by_action_and_field() {
        let mut c = ContractTuning::default();
        c.actions.0[ActionKind::Pass as usize] = Some(ActionParams {
            spread: Some(0.2),
            ..k_only(0.2)
        });
        let text = c.validate().unwrap_err().to_string();
        assert!(
            text.contains("action pass: spread is not read by play"),
            "{text}"
        );
        let mut c = ContractTuning::default();
        c.actions.0[ActionKind::Tackle as usize] = Some(k_only(0.2));
        let text = c.validate().unwrap_err().to_string();
        assert!(text.contains("action tackle: floor is missing"), "{text}");
    }

    #[test]
    fn the_newer_actions_may_be_absent_and_read_neutral_but_no_other() {
        let mut c = ContractTuning::default();
        for a in [
            ActionKind::Mark,
            ActionKind::OffBallRun,
            ActionKind::Corner,
            ActionKind::FreeKick,
            ActionKind::LongThrow,
            ActionKind::Cover,
            ActionKind::RecoveryRun,
        ] {
            c.actions.0[a as usize] = None;
            assert_eq!(c.actions.of(a).k(), 0.0);
        }
        assert!(c.validate().is_ok());
        c.actions.0[ActionKind::Press as usize] = None;
        let text = c.validate().unwrap_err().to_string();
        assert!(text.contains("action press is missing"), "{text}");
    }

    #[test]
    fn a_file_without_hooks_reads_them_neutral() {
        let c = ContractTuning {
            hooks: None,
            ..Default::default()
        };
        assert_eq!(c.hooks(), HookTuning::NEUTRAL);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn a_file_without_the_person_block_reads_it_neutral_and_a_wide_floor_is_refused() {
        let c = ContractTuning {
            person: None,
            ..Default::default()
        };
        assert_eq!(c.person(), PersonTuning::NEUTRAL);
        assert!(c.validate().is_ok());
        let mut c = ContractTuning::default();
        if let Some(p) = c.person.as_mut() {
            p.floor_drop = 13.0;
        }
        let text = c.validate().unwrap_err().to_string();
        assert!(text.contains("person.floor_drop"), "{text}");
    }

    #[test]
    fn a_file_without_the_foot_block_reads_no_cost_and_a_cap_above_five_is_refused() {
        let c = ContractTuning {
            foot: None,
            ..Default::default()
        };
        assert_eq!(c.foot(), FootTuning::NONE);
        assert!(c.validate().is_ok());
        let mut c = ContractTuning::default();
        if let Some(f) = c.foot.as_mut() {
            f.cost_cap = 5.5;
        }
        let text = c.validate().unwrap_err().to_string();
        assert!(text.contains("foot.cost_cap"), "{text}");
    }

    #[test]
    fn a_file_without_the_season_block_reads_it_neutral_and_a_wide_cap_is_refused() {
        let c = ContractTuning {
            season: None,
            ..Default::default()
        };
        assert_eq!(c.season(), SeasonTuning::NEUTRAL);
        assert!(c.validate().is_ok());
        let mut c = ContractTuning::default();
        if let Some(s) = c.season.as_mut() {
            s.cap = 120.0;
        }
        let text = c.validate().unwrap_err().to_string();
        assert!(text.contains("season.cap"), "{text}");
    }

    #[test]
    fn the_speed_map_gives_the_anchor_at_pace_ten_and_amplifies_the_real_difference() {
        let s = ContractTuning::default().speed;
        assert_eq!(s.top_speed(10.0), 7.0);
        assert!((s.top_speed(20.0) - (7.0 + 2.0 * 3.5 / 3.6)).abs() < 1e-12);
        assert!((s.kmh(5.5) - 30.5).abs() < 1e-12);
        assert_eq!(s.kmh(0.5), 29.0);
    }
}

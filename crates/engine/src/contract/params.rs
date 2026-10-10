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
}

impl ContractTuning {
    /// The hook values: the file's, or [`HookTuning::NEUTRAL`] when it has none.
    #[inline]
    pub fn hooks(&self) -> HookTuning {
        self.hooks.unwrap_or(HookTuning::NEUTRAL)
    }
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
}

impl HookTuning {
    /// Every hook as play was before the hooks: braking with the acceleration, no effort when
    /// behind, no flair term, no direct free kick taken, no delivery noise, the keeper alone
    /// organising the line, no anchor space and no rash chase.
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
    };
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
        // The five actions the record added; each may be absent ([`neutral_when_absent`]).
        A::Mark | A::OffBallRun | A::LongThrow => [true, false, false, true],
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
/// share 0.5, factor 1): the five actions the record added, so a tuning file written before
/// they had values (as an old replay embeds it) still loads and plays as it did.
pub fn neutral_when_absent(action: ActionKind) -> bool {
    use ActionKind as A;
    matches!(
        action,
        A::Mark | A::OffBallRun | A::Corner | A::FreeKick | A::LongThrow
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
            }),
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
    fn the_five_new_actions_may_be_absent_and_read_neutral_but_no_other() {
        let mut c = ContractTuning::default();
        for a in [
            ActionKind::Mark,
            ActionKind::OffBallRun,
            ActionKind::Corner,
            ActionKind::FreeKick,
            ActionKind::LongThrow,
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
    fn the_speed_map_gives_the_anchor_at_pace_ten_and_amplifies_the_real_difference() {
        let s = ContractTuning::default().speed;
        assert_eq!(s.top_speed(10.0), 7.0);
        assert!((s.top_speed(20.0) - (7.0 + 2.0 * 3.5 / 3.6)).abs() < 1e-12);
        assert!((s.kmh(5.5) - 30.5).abs() < 1e-12);
        assert_eq!(s.kmh(0.5), 29.0);
    }
}

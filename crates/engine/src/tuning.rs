//! Tuning constants. The binary reads them from `content/tuning.json` (the `engine` block);
//! `Tuning::default()` documents the shipped values and a test pins the file to it.
//! Bounds live here as `garde` rules (product-owner choice, plan Q8).

use garde::Validate;
use serde::{Deserialize, Serialize};

use crate::TICKS_PER_SECOND;
use crate::contract::ContractTuning;

/// Refuses a physics step that is not one tick of the match clock: the clock, the minute, and
/// the viewer all count `TICKS_PER_SECOND` ticks to the second, and a different step would
/// run the physics and the clock at different rates without a word.
fn one_clock_tick(dt: &f64, _ctx: &()) -> garde::Result {
    let expected = 1.0 / f64::from(TICKS_PER_SECOND);
    if (dt - expected).abs() > 1e-9 {
        return Err(garde::Error::new(format!(
            "must be {expected} (one tick at {TICKS_PER_SECOND} ticks per second)"
        )));
    }
    Ok(())
}

/// Every constant the simulation reads. Units are metres, seconds, and metres per second.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Tuning {
    /// Simulated seconds per tick. The match clock counts `TICKS_PER_SECOND` ticks to the
    /// second, so the only step it accepts is one tick of that clock.
    #[garde(custom(one_clock_tick))]
    pub dt: f64,
    /// Ticks between two decisions of the same agent. 1 means every tick.
    #[garde(range(min = 1, max = 50))]
    pub decision_interval_ticks: u32,
    /// Steering: slow-down radius for arrive, neighbour radius and strength for separation.
    #[garde(range(min = 0.0, max = 30.0))]
    pub arrive_radius: f64,
    #[garde(range(min = 0.0, max = 15.0))]
    pub separation_radius: f64,
    #[garde(range(min = 0.0, max = 60.0))]
    pub separation_strength: f64,
    /// Minimum distance kept between two players after integration.
    #[garde(range(min = 0.0, max = 4.0))]
    pub min_player_distance: f64,
    /// Ball: ground deceleration, air drag per second, gravity, bounce restitution, speed cap.
    #[garde(range(min = 0.0, max = 40.0))]
    pub ground_friction: f64,
    #[garde(range(min = 0.0, max = 0.5))]
    pub air_drag: f64,
    #[garde(range(min = 0.0, max = 98.1))]
    pub gravity: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub restitution: f64,
    #[garde(range(min = 0.0, max = 400.0))]
    pub ball_max_speed: f64,
    /// Possession: reach radius, height under which the ball is controllable, control speed limit,
    /// ticks after gaining the ball during which no tackle succeeds.
    #[garde(range(min = 0.0, max = 10.0))]
    pub reach_radius: f64,
    /// Goalkeeper reach in metres (arms and dive).
    #[garde(range(min = 0.0, max = 26.0))]
    pub keeper_reach: f64,
    /// Metres the goalkeeper stands off the goal line.
    #[garde(range(min = 0.0, max = 30.0))]
    pub keeper_depth: f64,
    /// Shot aim noise in radians (each side).
    #[garde(range(min = 0.0, max = 1.2))]
    pub shot_noise: f64,
    /// Chance a goalkeeper holds a ball faster than `control_speed`.
    #[garde(range(min = 0.0, max = 1.0))]
    pub keeper_catch_chance: f64,
    /// Metres the carried ball moves toward the carry point per tick.
    #[garde(range(min = 0.0, max = 4.0))]
    pub carry_step: f64,
    /// Crossbar height in metres.
    #[garde(range(min = 0.0, max = 24.4))]
    pub crossbar_height: f64,
    #[garde(range(min = 0.0, max = 20.0))]
    pub reach_height: f64,
    #[garde(range(min = 0.0, max = 120.0))]
    pub control_speed: f64,
    #[garde(range(min = 0, max = 250))]
    pub control_cooldown_ticks: u32,
    /// Distance within which an opponent presses the carrier or chases a loose ball.
    #[garde(range(min = 0.0, max = 180.0))]
    pub press_distance: f64,
    /// Kicks: extra arrival speed for a pass, maximum pass speed, shooting speed, shooting range,
    /// aim noise in radians.
    #[garde(range(min = 0.0, max = 30.0))]
    pub pass_arrival_speed: f64,
    #[garde(range(min = 0.0, max = 280.0))]
    pub pass_max_speed: f64,
    #[garde(range(min = 0.0, max = 270.0))]
    pub shot_speed: f64,
    #[garde(range(min = 0.0, max = 210.0))]
    pub shot_range: f64,
    #[garde(range(min = 0.0, max = 0.6))]
    pub aim_noise: f64,
    /// Formation anchor: fraction of the ball offset added to a slot.
    #[garde(range(min = 0.0, max = 3.0))]
    pub compactness_x: f64,
    #[garde(range(min = 0.0, max = 3.0))]
    pub compactness_y: f64,
    /// The far-player delay of the slide: seconds of follow time per metre of a player's
    /// distance from the ball. Each outfield player's anchor shift follows his own followed
    /// ball point ([`crate::team::follow`]), which eases toward the ball with a time constant
    /// of this value times his distance, so a player near the ball slides at once and a far
    /// one late. The only value tuned against the slide band; 0 turns the delay off (every
    /// followed point is the ball), as before it existed.
    #[serde(default)]
    #[garde(range(min = 0.0, max = 0.5))]
    pub far_delay_s_per_m: f64,
    /// Defending: how far, in metres, the covering defender stands goal-side of the most
    /// advanced central attacker, and the half-width of the central channel in which an
    /// attacker is covered.
    #[serde(default = "default_cover_distance")]
    #[garde(range(min = 0.0, max = 10.0))]
    pub cover_distance: f64,
    #[serde(default = "default_cover_channel")]
    #[garde(range(min = 0.0, max = 34.0))]
    pub cover_channel: f64,
    /// The widest gap, in metres, between two neighbours in the deepest outfield line: the
    /// line is at most `(n - 1)` gaps wide, so it narrows as players leave it.
    #[serde(default = "default_back_line_gap")]
    #[garde(range(min = 4.0, max = 30.0))]
    pub back_line_gap: f64,
    /// Validator: anchor tolerance, the ball distance beyond which it applies, and the ticks the
    /// ball must have been that far away before the rule applies.
    #[garde(range(min = 0.0, max = 150.0))]
    pub anchor_tolerance: f64,
    #[garde(range(min = 0.0, max = 300.0))]
    pub anchor_ball_distance: f64,
    #[garde(range(min = 0, max = 1000))]
    pub anchor_grace_ticks: u32,
    /// Fouls: the chance that a tackle attempt is a foul for an average tackler; the
    /// log-odds it rises per curve point of the tackler's commitment (the tackle choose stage,
    /// led by aggression) and falls per curve point of his tackling (the tackle execute
    /// stage), each measured from rating 10; and the share of fouls after which the fouled
    /// team loses the ball for a fouled player of rating 10 (the rest play on with advantage).
    #[garde(range(min = 0.0, max = 1.0))]
    pub foul_base: f64,
    #[garde(range(min = 0.0, max = 4.0))]
    pub foul_aggression_weight: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub foul_tackling_weight: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub foul_ball_loss: f64,
    /// Cards per foul: the yellow chance at a commitment share of 0, the extra at a share of
    /// 1 (0.5 at rating 10), and the straight red chance.
    #[garde(range(min = 0.0, max = 1.0))]
    pub yellow_base: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub yellow_aggression_weight: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub red_base: f64,
    /// Ticks after a foul during which the same player makes no tackle attempt, and the
    /// factor on the foul chance of a player already booked.
    #[serde(default = "default_foul_cooldown_ticks")]
    #[garde(range(min = 0, max = 1000))]
    pub foul_cooldown_ticks: u32,
    #[serde(default = "default_foul_booked_factor")]
    #[garde(range(min = 0.0, max = 1.0))]
    pub foul_booked_factor: f64,
    /// Twice the chance that a tackle attempt wins the ball cleanly when the tackler's
    /// tackle stage equals the carrier's dribble or shield stage: the even contest is
    /// `tackle_win_base / 2`, as the ratio `tackling / (tackling + dribbling)` gave before the
    /// contract.
    #[serde(default = "default_tackle_win_base")]
    #[garde(range(min = 0.0, max = 0.5))]
    pub tackle_win_base: f64,
    /// Contesting the carrier: the distance in metres from the ball within which an opponent
    /// attempts a tackle, and the distance from the ball within which a presser runs at the
    /// ball itself rather than at the point where he meets the carrier's run.
    #[serde(default = "default_tackle_reach")]
    #[garde(range(min = 0.5, max = 3.0))]
    pub tackle_reach: f64,
    #[serde(default = "default_press_engage")]
    #[garde(range(min = 0.0, max = 10.0))]
    pub press_engage: f64,
    /// The extra chance, on top of `tackle_win_base`, that a tackle wins the ball from a
    /// carrier running with it (faster than 2 m/s); it scales with the same skill ratio. A
    /// carrier who stands and shields the ball is not affected.
    #[serde(default)]
    #[garde(range(min = 0.0, max = 1.0))]
    pub tackle_dribble_win: f64,
    /// Off the ball, the share of the way from his anchor to the onside line (0.5 m short of
    /// the second-last defender) that a team's lone forward moves while his team has the ball.
    #[serde(default = "default_lone_line_hold")]
    #[garde(range(min = 0.0, max = 1.0))]
    pub lone_line_hold: f64,
    /// Seconds a dead ball lasts before the taker may restart play, per restart kind.
    #[garde(dive)]
    pub restart_delay_s: RestartDelays,
    /// Metres from the restart spot within which the taker is ready.
    #[garde(range(min = 0.1, max = 10.0))]
    pub restart_ready_radius: f64,
    /// The weights of the scored-options decision layer.
    #[garde(dive)]
    pub decision: DecisionWeights,
    /// Injury chance for the tackled player on each tackle that wins the ball or is a foul,
    /// and per simulated minute for each player on the pitch, for an average player; injury
    /// resistance scales both.
    #[garde(range(min = 0.0, max = 0.2))]
    pub injury_per_tackle: f64,
    #[garde(range(min = 0.0, max = 0.01))]
    pub injury_per_minute: f64,
    /// The expected-goals model of a shot.
    #[garde(dive)]
    pub xg: XgTuning,
    /// Shots, saves, parries and blocks, penalties included.
    #[serde(default)]
    #[garde(dive)]
    pub shots: ShotTuning,
    /// Clearances: the aim of a carrier's clearance and a defender's clearance of a fast
    /// pass inside his own penalty area.
    #[serde(default)]
    #[garde(dive)]
    pub clearances: ClearanceTuning,
    /// The attribute contract: the curve, each action's strength and limits, top speed,
    /// acceleration, and concentration lapses.
    #[garde(dive)]
    pub contract: ContractTuning,
    /// The pace selector: the six levels, the distances that choose between them, the sprint
    /// chance, the tired mix and the late minutes. A tuning file without this block loads with
    /// the selector off, so every player runs at his top speed, as before it existed.
    #[serde(default = "PaceTuning::off")]
    #[garde(dive)]
    pub pace: PaceTuning,
}

/// The pace selector's values ([`crate::pace`]). Speeds in metres per second, distances in
/// metres, shares and thresholds from 0 to 1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct PaceTuning {
    /// `false` turns the selector off: every player runs at his top speed.
    #[garde(skip)]
    pub enabled: bool,
    /// The upper limits of stand, walk, jog, run and high speed; a sprint is the player's top
    /// speed. Strictly rising and positive.
    #[garde(custom(rising_limits))]
    pub limits_ms: [f64; 5],
    /// Within this distance of his target a player stands.
    #[garde(range(min = 0.0, max = 10.0))]
    pub stand_m: f64,
    /// Farther than this from his target, a player in shape returns to it (NORMAL).
    #[garde(range(min = 0.0, max = 60.0))]
    pub return_far_m: f64,
    /// Farther than this from his target, a NORMAL job runs rather than jogs.
    #[garde(range(min = 0.0, max = 120.0))]
    pub run_from_m: f64,
    /// Within this distance of the ball, a player in shape follows it (NORMAL); farther, he
    /// holds (CALM).
    #[garde(range(min = 0.0, max = 120.0))]
    pub calm_ball_m: f64,
    /// The sprint chance of an URGENT job: `sprint_mid + sprint_slope · (work-rate share − 0.5)`,
    /// inside 0 to 1 at a share of 0 and of 1.
    #[garde(range(min = 0.0, max = 1.0))]
    pub sprint_mid: f64,
    #[garde(range(min = 0.0, max = 2.0), custom(chance_in_range(&self.sprint_mid)))]
    pub sprint_slope: f64,
    /// Below this energy a player saves energy.
    #[garde(range(min = 0.0, max = 1.0))]
    pub tired_from: f64,
    /// A tired player whose saving times his tired mix reaches this moves a step slower at
    /// every level below a sprint.
    #[garde(range(min = 0.0, max = 1.0))]
    pub step_down_at: f64,
    /// Per URGENT job: a tired player whose saving times one less his tired mix reaches this
    /// skips the job's sprint. Lower for a less urgent job.
    #[garde(dive)]
    pub skip_at: SkipAt,
    /// The late minutes of a match: the last this many minutes of its regulation length.
    #[garde(range(min = 0, max = 60))]
    pub late_minutes: u32,
}

/// The sprint skip threshold of each URGENT job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct SkipAt {
    #[garde(range(min = 0.0, max = 1.0))]
    pub carry: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub timed_run: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub press: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub loose_ball: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub recovery: f64,
}

/// Refuses level limits that are not positive and strictly rising.
fn rising_limits(limits: &[f64; 5], _ctx: &()) -> garde::Result {
    if limits[0] <= 0.0 || limits.windows(2).any(|w| w[1] <= w[0]) {
        return Err(garde::Error::new(
            "must be positive and strictly rising".to_string(),
        ));
    }
    Ok(())
}

/// Refuses a sprint chance outside 0 to 1 at a work-rate share of 0 or of 1.
fn chance_in_range(mid: &f64) -> impl FnOnce(&f64, &()) -> garde::Result + '_ {
    move |slope, _| {
        let half = slope / 2.0;
        if mid - half < 0.0 || mid + half > 1.0 {
            return Err(garde::Error::new(format!(
                "gives a sprint chance outside 0 to 1 with sprint_mid {mid}"
            )));
        }
        Ok(())
    }
}

impl PaceTuning {
    /// The shipped values: most open-play time at walk and jog, sprints only for URGENT jobs.
    pub fn shipped() -> Self {
        Self {
            enabled: true,
            limits_ms: [0.2, 2.0, 4.0, 5.5, 7.0],
            stand_m: 1.0,
            return_far_m: 5.0,
            // Started at 15 m. With the team stands off, NORMAL jobs that jogged within 15 m
            // of their target put 5.8 percent of open play at run pace against the real 10.5
            // (band 8 to 13) and left the default teams' matches almost without a duel (about
            // one foul a side). At 0 a NORMAL job runs, and jogs only when tiredness steps it
            // down: run 8.4 percent, walk 54, jog 33 (real 49.5 and 36), the closest fit of the
            // values tried (15, 10, 7.5, 5, 2.5, 2, 1.5, 0).
            run_from_m: 0.0,
            calm_ball_m: 30.0,
            sprint_mid: 0.5,
            sprint_slope: 1.0,
            tired_from: 0.5,
            step_down_at: 0.25,
            skip_at: SkipAt {
                carry: 0.10,
                timed_run: 0.10,
                press: 0.20,
                loose_ball: 0.35,
                recovery: 0.35,
            },
            late_minutes: 15,
        }
    }

    /// The selector off, with the shipped values kept for the day it is turned on: a tuning
    /// file written before the selector existed plays as it did.
    pub fn off() -> Self {
        Self {
            enabled: false,
            ..Self::shipped()
        }
    }
}

/// How a clearance flies. A carrier's clearance goes toward the far half, turned by up to
/// `aim_spread` radians either way. A fast open-play pass inside the defending side's
/// penalty area can be cleared: each active defending outfield player within `cross_reach`
/// metres of the ball under `reach_height` rolls once per flight against `cross_chance`.
/// The cleared ball keeps `cross_speed` of its speed and goes away from the defender's goal
/// centre, turned by up to `cross_spread` radians either way, with a vertical speed of up to
/// `cross_loft` metres per second. A cleared fast pass, or a carrier's clearance within
/// `wide_depth` metres of his own goal line, goes wide with `wide_chance` instead: toward his
/// own goal line on the ball's side, where the ball may go behind for a corner. These values
/// have no sourced figure; they are tuning levers inside their bounds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ClearanceTuning {
    #[garde(range(min = 0.0, max = 1.6))]
    pub aim_spread: f64,
    #[garde(range(min = 0.0, max = 3.0))]
    pub cross_reach: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub cross_chance: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub cross_speed: f64,
    #[garde(range(min = 0.0, max = 3.2))]
    pub cross_spread: f64,
    #[garde(range(min = 0.0, max = 15.0))]
    pub cross_loft: f64,
    /// An older tuning file without this value loads with 0: no clearance goes wide.
    #[serde(default)]
    #[garde(range(min = 0.0, max = 1.0))]
    pub wide_chance: f64,
    /// An older tuning file without this value loads with the depth of the penalty area.
    #[serde(default = "default_wide_depth")]
    #[garde(range(min = 0.0, max = 52.5))]
    pub wide_depth: f64,
}

fn default_wide_depth() -> f64 {
    16.5
}

/// The values that reproduce play before these levers existed: the old clearance aim, no
/// cross clearance and no wide clearance.
impl Default for ClearanceTuning {
    fn default() -> Self {
        Self {
            aim_spread: 0.6,
            cross_reach: 2.0,
            cross_chance: 0.0,
            cross_speed: 0.7,
            cross_spread: 1.6,
            cross_loft: 4.0,
            wide_chance: 0.0,
            wide_depth: 16.5,
        }
    }
}

/// How a shot flies and how a keeper or a defender stops it. A keeper tries to save only a
/// shot heading between the posts under the bar, with a chance that falls linearly from
/// `save_high` at shot quality 0.05 to `save_low` at 0.40 and is flat outside that range.
/// The quality is the expected goals of `quality`, a fixed model kept apart from the
/// reported `xg`, so refitting the reported figure never moves the saves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ShotTuning {
    /// The highest vertical speed of a shot, in metres per second; each shot draws its loft
    /// from 0 to this.
    #[garde(range(min = 0.0, max = 15.0))]
    pub loft_max: f64,
    /// The save chance for the poorest and for the best chances.
    #[garde(range(min = 0.0, max = 1.0))]
    pub save_high: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub save_low: f64,
    /// The shot-quality model the save chance reads.
    #[garde(dive)]
    pub quality: XgTuning,
    /// The share of saves the keeper holds; the rest he parries.
    #[garde(range(min = 0.0, max = 1.0))]
    pub save_hold: f64,
    /// A parry: the share of the shot's speed the ball keeps, the widest angle in radians
    /// either side of the goal line away from the goal centre, and the highest vertical
    /// speed in metres per second.
    #[garde(range(min = 0.0, max = 1.0))]
    pub parry_speed: f64,
    #[garde(range(min = 0.0, max = 3.2))]
    pub parry_spread: f64,
    #[garde(range(min = 0.0, max = 15.0))]
    pub parry_loft: f64,
    /// A block: the distance in metres within which an outfield defender can block a shot,
    /// his chance to block it (one roll per defender per shot), the share of the speed the
    /// ball keeps, and the widest angle in radians either side of the reversed direction.
    #[garde(range(min = 0.0, max = 3.0))]
    pub block_reach: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub block_chance: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub block_speed: f64,
    #[garde(range(min = 0.0, max = 3.2))]
    pub block_spread: f64,
    /// The expected goals, and the save quality, of a penalty.
    #[garde(range(min = 0.0, max = 1.0))]
    pub penalty_xg: f64,
    /// The aim noise of a kick from the penalty mark, against an open-play shot.
    #[garde(range(min = 0.0, max = 2.0))]
    pub penalty_spread: f64,
    /// How far along the goal line, in metres, a shoot-out keeper dives, and the share of
    /// the dive draw's range, either side of 0, for which he stays in the middle.
    #[garde(range(min = 0.0, max = 3.66))]
    pub keeper_dive_m: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub keeper_stays: f64,
}

impl Default for ShotTuning {
    fn default() -> Self {
        Self {
            loft_max: 7.0,
            save_high: 0.891,
            save_low: 0.272,
            quality: XgTuning {
                intercept: -1.0,
                distance_coef: -0.1,
                angle_coef: 1.0,
            },
            save_hold: 0.333,
            parry_speed: 0.8,
            parry_spread: 1.2,
            parry_loft: 4.0,
            block_reach: 3.0,
            block_chance: 0.5,
            block_speed: 0.8,
            block_spread: 3.2,
            penalty_xg: 0.76,
            penalty_spread: 0.25,
            keeper_dive_m: 2.0,
            keeper_stays: 0.1,
        }
    }
}

/// Expected goals of a shot: `1 / (1 + exp(-(intercept + distance_coef * d + angle_coef *
/// a)))`, where `d` is the distance to the goal centre in metres and `a` the angle in
/// radians that the goal mouth subtends from the shot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct XgTuning {
    #[garde(range(min = -10.0, max = 10.0))]
    pub intercept: f64,
    #[garde(range(min = -2.0, max = 0.0))]
    pub distance_coef: f64,
    #[garde(range(min = 0.0, max = 10.0))]
    pub angle_coef: f64,
}

/// The weights of the scored-options decision layer (named mechanism). The ball carrier
/// scores every option as a weighted sum of its features, plus the team plan's offsets
/// (mentality, instructions, role, and duty), plus noise, and takes the highest. Every
/// weight is a tuning value, so retuning moves behaviour without code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct DecisionWeights {
    /// A pass: forward progress (-1 to 1 over 40 m), the open lane (up to 6 m), the space
    /// around the receiver (up to 8 m), and the distance (up to 45 m, a cost).
    #[garde(range(min = -5.0, max = 5.0))]
    pub progress: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub lane: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub space: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub distance: f64,
    /// A pass lane narrower than this many metres is not an option.
    #[garde(range(min = 0.0, max = 10.0))]
    pub min_lane: f64,
    /// How much a skill attribute 50 points above average adds to its option: vision to
    /// forward passes, finishing to shots, dribbling to a dribble.
    #[garde(range(min = 0.0, max = 5.0))]
    pub skill: f64,
    /// A shot: the base, the open shooting lane (0 at 2.5 m, 1 at 5 m), and the distance
    /// (a cost, over the shooting range).
    #[garde(range(min = -5.0, max = 5.0))]
    pub shot_base: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub shot_lane: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub shot_distance: f64,
    /// A dribble: the base, the free space ahead (up to 10 m), and the bonus in the first
    /// 10 ticks after gaining the ball.
    #[garde(range(min = -5.0, max = 5.0))]
    pub dribble_base: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub dribble_space: f64,
    /// Added to a shot with an open lane (wider than 1 m) taken with an opponent within 2 m.
    #[garde(range(min = -5.0, max = 5.0))]
    pub shot_under_pressure: f64,
    /// Subtracted from the dribble score when an opponent is within 2.5 m: a negative value
    /// raises a pressed dribble, so the carrier takes the defender on.
    #[garde(range(min = -5.0, max = 5.0))]
    pub takeon_under_pressure: f64,
    /// The size of the pressure model's penalty ([`crate::pressure`]), for every side: a
    /// player's penalty is his pressure intensity x the action's difficulty x (1 - his
    /// resistance). An outfield pass's aim noise grows by `pass_under_pressure` times the
    /// passer's penalty, and a receiver of his side's pass reaches the ball only within
    /// `1 - touch_under_pressure` times his penalty of his reach, so his first touch more often
    /// lets the ball run on. Each is sized so that the engine's gap between pressed and free
    /// players matches the real gap in the StatsBomb open data (pass completion; miscontrols
    /// per reception). 0 (an older file) leaves play as before.
    #[serde(default)]
    #[garde(range(min = 0.0, max = 500.0))]
    pub pass_under_pressure: f64,
    #[serde(default)]
    #[garde(range(min = 0.0, max = 5.0))]
    pub touch_under_pressure: f64,
    /// The weight error of a pressed pass: an outfield passer's pass runs too hard or too soft
    /// by up to `pass_weight_under_pressure` times his pressure penalty of its speed (the same
    /// penalty that widens its aim), so pressure spoils both the direction and the weight of a
    /// pass. 0 (an older file) leaves every pass at its planned speed.
    #[serde(default)]
    #[garde(range(min = 0.0, max = 5.0))]
    pub pass_weight_under_pressure: f64,
    /// The size of a defender's chance to cut out or block a pass in flight as it passes him
    /// ([`crate::possession`]): his chance is `pass_cut` times his read of the pass (or his
    /// block, for a ball too fast to control) times how close it passes him and how slow it is.
    /// While a pass is in flight an opponent wins it only this way. 0 (an older file) keeps the
    /// old rule: the nearest player within reach of a ball he can control wins it.
    #[serde(default)]
    #[garde(range(min = 0.0, max = 5.0))]
    pub pass_cut: f64,
    /// The size of a defender's contest of a take-on ([`crate::possession`]): a defender in
    /// front of a carrier who runs with the ball meets him once as he comes within the
    /// pressed distance, and wins the ball with `take_on_contest` times the running tackle
    /// contest of the two players, scaled by the angle he meets him at, how square he
    /// stands and the pace the carrier runs at him with, the last two moved by his read of
    /// the take-on. Fitted so that take-ons beat their man as often as the StatsBomb open
    /// data's dribbles (0.529). 0 (an older file) leaves only the tackles within reach.
    #[serde(default)]
    #[garde(range(min = 0.0, max = 5.0))]
    pub take_on_contest: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub first_touch: f64,
    /// A clearance: the base, the bonus under pressure in the own third, and the base for
    /// a goalkeeper.
    #[garde(range(min = -5.0, max = 5.0))]
    pub clear: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub clear_pressure: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub keeper_clear: f64,
    /// Holding the ball: the base, and the cost per second already held.
    #[garde(range(min = -5.0, max = 5.0))]
    pub hold: f64,
    #[garde(range(min = 0.0, max = 5.0))]
    pub hold_per_s: f64,
    /// Noise on every option, each side, for a player with decisions and composure 50; it
    /// shrinks as the two rise.
    #[garde(range(min = 0.0, max = 2.0))]
    pub noise: f64,
    /// A lone carrier (no active outfield team-mate ahead of him): the bonus on a pass to a
    /// team-mate at or behind him, and, when he is pressed, the bonus on holding the ball
    /// and on a dribble (a negative value is a cost).
    #[serde(default)]
    #[garde(range(min = -5.0, max = 5.0))]
    pub lone_layoff: f64,
    #[serde(default)]
    #[garde(range(min = -5.0, max = 5.0))]
    pub lone_hold: f64,
    #[serde(default)]
    #[garde(range(min = -5.0, max = 5.0))]
    pub lone_dribble: f64,
    /// The carry window: for `carry_s` seconds after gaining the ball, a carrier with no
    /// opponent within 2.5 m pays `carry_cost` on a pass and on a clearance. A shot is never
    /// charged.
    #[serde(default)]
    #[garde(range(min = 0.0, max = 5.0))]
    pub carry_s: f64,
    #[serde(default)]
    #[garde(range(min = 0.0, max = 5.0))]
    pub carry_cost: f64,
    /// The weight of a pass candidate's choose stage (decisions, with teamwork and vision
    /// supporting): `teamwork_bonus · (2 · share − 1)`, 0 at rating 10.
    #[garde(range(min = 0.0, max = 5.0))]
    pub teamwork_bonus: f64,
}

/// Seconds between the ball going dead and the restart, per restart kind. Play restarts
/// at three times the delay at the latest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct RestartDelays {
    #[garde(range(min = 0.0, max = 60.0))]
    pub kick_off: f64,
    #[garde(range(min = 0.0, max = 60.0))]
    pub throw_in: f64,
    #[garde(range(min = 0.0, max = 60.0))]
    pub corner: f64,
    #[garde(range(min = 0.0, max = 60.0))]
    pub goal_kick: f64,
    #[garde(range(min = 0.0, max = 60.0))]
    pub free_kick: f64,
    #[garde(range(min = 0.0, max = 60.0))]
    pub penalty: f64,
    /// A dropped ball after play stopped for an injury.
    #[garde(range(min = 0.0, max = 60.0))]
    pub drop_ball: f64,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            dt: 0.02,
            decision_interval_ticks: 1,
            arrive_radius: 3.0,
            separation_radius: 1.5,
            separation_strength: 6.0,
            min_player_distance: 0.4,
            ground_friction: 4.0,
            air_drag: 0.05,
            gravity: 9.81,
            restitution: 0.55,
            ball_max_speed: 40.0,
            reach_radius: 1.0,
            keeper_reach: 2.6,
            keeper_depth: 3.0,
            shot_noise: 0.5,
            keeper_catch_chance: 0.86,
            carry_step: 0.4,
            crossbar_height: 2.44,
            reach_height: 2.0,
            control_speed: 12.0,
            control_cooldown_ticks: 25,
            press_distance: 18.0,
            pass_arrival_speed: 3.0,
            pass_max_speed: 28.0,
            shot_speed: 27.0,
            shot_range: 21.0,
            aim_noise: 0.06,
            compactness_x: 0.3,
            compactness_y: 0.3,
            far_delay_s_per_m: FAR_DELAY_S_PER_M,
            cover_distance: default_cover_distance(),
            cover_channel: default_cover_channel(),
            back_line_gap: default_back_line_gap(),
            anchor_tolerance: 15.0,
            anchor_ball_distance: 30.0,
            anchor_grace_ticks: 100,
            foul_base: FOUL_BASE,
            foul_aggression_weight: 0.05,
            foul_tackling_weight: 0.025,
            foul_ball_loss: 0.6,
            yellow_base: 0.05,
            yellow_aggression_weight: 0.15,
            red_base: 0.005,
            foul_cooldown_ticks: default_foul_cooldown_ticks(),
            foul_booked_factor: default_foul_booked_factor(),
            tackle_win_base: TACKLE_WIN_BASE,
            tackle_reach: default_tackle_reach(),
            press_engage: default_press_engage(),
            tackle_dribble_win: 0.0,
            lone_line_hold: default_lone_line_hold(),
            restart_delay_s: RestartDelays {
                kick_off: 5.0,
                throw_in: 3.0,
                corner: 8.0,
                goal_kick: 6.0,
                free_kick: 8.0,
                penalty: 15.0,
                drop_ball: 20.0,
            },
            restart_ready_radius: 1.0,
            decision: DecisionWeights {
                progress: 0.7,
                lane: 0.5,
                space: 0.4,
                distance: 0.3,
                min_lane: 1.5,
                skill: 0.8,
                shot_base: 1.0,
                shot_lane: 0.8,
                shot_distance: 1.0,
                dribble_base: -0.4,
                dribble_space: 0.55,
                shot_under_pressure: -0.8,
                takeon_under_pressure: -0.8,
                pass_under_pressure: 48.0,
                touch_under_pressure: 0.88,
                pass_weight_under_pressure: 1.0,
                pass_cut: 0.6,
                take_on_contest: 2.95,
                first_touch: 0.6,
                clear: -1.0,
                clear_pressure: 1.2,
                keeper_clear: 0.2,
                hold: -0.4,
                hold_per_s: 0.5,
                noise: 0.1,
                lone_layoff: 0.0,
                lone_hold: 0.5,
                lone_dribble: -0.5,
                carry_s: 0.0,
                carry_cost: 0.0,
                teamwork_bonus: 0.1,
            },
            injury_per_tackle: INJURY_PER_TACKLE,
            injury_per_minute: INJURY_PER_MINUTE,
            xg: XgTuning {
                intercept: -4.191,
                distance_coef: -0.0441,
                angle_coef: 6.3836,
            },
            shots: ShotTuning::default(),
            clearances: ClearanceTuning::default(),
            contract: ContractTuning::default(),
            pace: PaceTuning::shipped(),
        }
    }
}

// The shipped defending and discipline values. An older tuning file without these fields
// loads with them.
fn default_cover_distance() -> f64 {
    3.0
}
fn default_cover_channel() -> f64 {
    12.0
}
fn default_back_line_gap() -> f64 {
    12.0
}
fn default_foul_cooldown_ticks() -> u32 {
    150
}
fn default_foul_booked_factor() -> f64 {
    0.15
}
/// The shipped chance that an even tackle wins the ball cleanly, set so that a formation
/// with one forward holds against 4-4-2.
const TACKLE_WIN_BASE: f64 = 0.5;

// The lone-forward and tackle values. An older tuning file without them loads with the
// values that reproduce the behaviour before they existed, not with the shipped ones.
fn default_tackle_win_base() -> f64 {
    0.05
}
/// The shipped far-player delay: seeded at 0.04 from the real far-player response (players 30
/// to 45 m from the ball follow it over 1 s about half as much as players near it), then
/// tuned only against the slide band (0.44 to 0.68 up the pitch, both phases, on each of four
/// seeds) by halving and then bisecting: 0.04 and 0.02 left one seed below the band, and 0.01
/// kept every figure inside it.
const FAR_DELAY_S_PER_M: f64 = 0.01;

fn default_lone_line_hold() -> f64 {
    0.0
}

// The contest values. An older tuning file without them loads with the values that
// reproduce play before they existed: a tackle from the control reach, a presser who runs
// at the ball from 3 m, and no extra chance against a running carrier.
fn default_tackle_reach() -> f64 {
    1.0
}
fn default_press_engage() -> f64 {
    3.0
}

/// The shipped foul chance per tackle attempt, set for about ten fouls per team in a
/// 90-minute match with the default teams.
const FOUL_BASE: f64 = 0.1;

/// The shipped injury rates: a planning estimate of about 0.6 injuries per match with the
/// default teams, to be replaced with a sourced rate.
const INJURY_PER_TACKLE: f64 = 0.004;
const INJURY_PER_MINUTE: f64 = 0.0002;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_satisfy_every_bound() {
        assert!(Tuning::default().validate().is_ok());
    }

    #[test]
    fn a_step_that_is_not_one_clock_tick_is_refused() {
        let t = Tuning {
            dt: 0.01,
            ..Tuning::default()
        };
        let report = t.validate().unwrap_err();
        assert!(report.to_string().contains("dt: must be 0.02"), "{report}");
    }

    #[test]
    fn a_probability_above_one_is_refused() {
        let t = Tuning {
            keeper_catch_chance: 1.5,
            ..Tuning::default()
        };
        let report = t.validate().unwrap_err();
        assert!(
            report
                .to_string()
                .contains("keeper_catch_chance: greater than 1"),
            "{report}"
        );
    }

    #[test]
    fn pace_levels_that_do_not_rise_or_a_chance_outside_one_are_refused() {
        let mut t = Tuning::default();
        t.pace.limits_ms = [0.2, 2.0, 2.0, 5.5, 7.0];
        let report = t.validate().unwrap_err().to_string();
        assert!(report.contains("pace.limits_ms"), "{report}");
        let mut t = Tuning::default();
        t.pace.limits_ms[0] = 0.0;
        assert!(t.validate().is_err());
        let mut t = Tuning::default();
        t.pace.sprint_mid = 0.8;
        let report = t.validate().unwrap_err().to_string();
        assert!(report.contains("pace.sprint_slope"), "{report}");
        let mut t = Tuning::default();
        t.pace.skip_at.press = 1.5;
        assert!(t.validate().is_err());
    }

    /// A tuning block written before the far-player delay loads with it off, so every
    /// followed ball point is the ball; a delay above the bound is refused.
    #[test]
    fn a_tuning_block_without_the_far_player_delay_loads_with_it_off() {
        let mut json = serde_json::to_value(Tuning::default()).unwrap();
        let block = json.as_object_mut().unwrap();
        assert!(block.remove("far_delay_s_per_m").is_some());
        let t: Tuning = serde_json::from_value(json).unwrap();
        assert_eq!(t.far_delay_s_per_m, 0.0);
        assert!(Tuning::default().far_delay_s_per_m > 0.0);
        let t = Tuning {
            far_delay_s_per_m: 0.6,
            ..Tuning::default()
        };
        let report = t.validate().unwrap_err().to_string();
        assert!(report.contains("far_delay_s_per_m"), "{report}");
    }

    /// A tuning block written before the pace selector loads with it off: every player runs
    /// at his top speed, as before it existed.
    #[test]
    fn a_tuning_block_without_the_pace_block_loads_with_the_selector_off() {
        let mut json = serde_json::to_value(Tuning::default()).unwrap();
        assert!(json.as_object_mut().unwrap().remove("pace").is_some());
        let t: Tuning = serde_json::from_value(json).unwrap();
        assert_eq!(t.pace, PaceTuning::off());
        assert!(!t.pace.enabled);
        assert!(Tuning::default().pace.enabled);
    }
}

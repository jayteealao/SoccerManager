//! Tuning constants. The binary reads them from `content/tuning.json` (the `engine` block);
//! `Tuning::default()` documents the shipped values and a test pins the file to it.
//! Bounds live here as `garde` rules (product-owner choice, plan Q8).

use garde::Validate;
use serde::{Deserialize, Serialize};

/// Every constant the simulation reads. Units are metres, seconds, and metres per second.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Tuning {
    /// Simulated seconds per tick.
    #[garde(range(min = 0.005, max = 0.1))]
    pub dt: f64,
    /// Ticks between two decisions of the same agent. 1 means every tick.
    #[garde(range(min = 1, max = 50))]
    pub decision_interval_ticks: u32,
    /// Player speed at pace 0 and the extra speed at pace 100.
    #[garde(range(min = 0.0, max = 50.0))]
    pub base_speed: f64,
    #[garde(range(min = 0.0, max = 40.0))]
    pub pace_speed: f64,
    /// Player acceleration at acceleration 0 and the extra at acceleration 100.
    #[garde(range(min = 0.0, max = 30.0))]
    pub base_accel: f64,
    #[garde(range(min = 0.0, max = 50.0))]
    pub accel_bonus: f64,
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
    /// Validator: anchor tolerance, the ball distance beyond which it applies, and the ticks the
    /// ball must have been that far away before the rule applies.
    #[garde(range(min = 0.0, max = 150.0))]
    pub anchor_tolerance: f64,
    #[garde(range(min = 0.0, max = 300.0))]
    pub anchor_ball_distance: f64,
    #[garde(range(min = 0, max = 1000))]
    pub anchor_grace_ticks: u32,
    /// Fouls: the chance that a tackle attempt is a foul for an average tackler, how much
    /// aggression raises it, how much tackling skill lowers it, and the share of fouls after
    /// which the fouled team loses the ball (the rest play on with advantage).
    #[garde(range(min = 0.0, max = 1.0))]
    pub foul_base: f64,
    #[garde(range(min = 0.0, max = 4.0))]
    pub foul_aggression_weight: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub foul_tackling_weight: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub foul_ball_loss: f64,
    /// Cards per foul: the yellow chance at aggression 0, the extra at aggression 100, and the
    /// straight red chance.
    #[garde(range(min = 0.0, max = 1.0))]
    pub yellow_base: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub yellow_aggression_weight: f64,
    #[garde(range(min = 0.0, max = 1.0))]
    pub red_base: f64,
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
    /// A dribble: the base, the free space ahead (up to 10 m), the cost of an opponent
    /// within 2.5 m (a negative cost makes the carrier take the defender on), and the bonus
    /// in the first 10 ticks after gaining the ball. The pressure weight is also added to a
    /// shot with an open lane taken with an opponent within 2 m.
    #[garde(range(min = -5.0, max = 5.0))]
    pub dribble_base: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub dribble_space: f64,
    #[garde(range(min = -5.0, max = 5.0))]
    pub pressure: f64,
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
            base_speed: 5.0,
            pace_speed: 4.0,
            base_accel: 3.0,
            accel_bonus: 5.0,
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
            shot_noise: 0.12,
            keeper_catch_chance: 0.7,
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
            anchor_tolerance: 15.0,
            anchor_ball_distance: 30.0,
            anchor_grace_ticks: 100,
            foul_base: FOUL_BASE,
            foul_aggression_weight: 1.0,
            foul_tackling_weight: 0.5,
            foul_ball_loss: 0.6,
            yellow_base: 0.05,
            yellow_aggression_weight: 0.15,
            red_base: 0.005,
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
                skill: 0.6,
                shot_base: 1.0,
                shot_lane: 0.8,
                shot_distance: 1.0,
                dribble_base: -0.4,
                dribble_space: 0.55,
                pressure: -0.8,
                first_touch: 0.6,
                clear: -1.0,
                clear_pressure: 1.2,
                keeper_clear: 0.2,
                hold: -0.4,
                hold_per_s: 0.5,
                noise: 0.1,
            },
            injury_per_tackle: INJURY_PER_TACKLE,
            injury_per_minute: INJURY_PER_MINUTE,
        }
    }
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
}

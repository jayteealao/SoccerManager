//! Tuning constants. The data-schemas slice replaces `Tuning::default()` with a validated file.

/// Every constant the simulation reads. Units are metres, seconds, and metres per second.
#[derive(Debug, Clone)]
pub struct Tuning {
    /// Simulated seconds per tick.
    pub dt: f64,
    /// Ticks between two decisions of the same agent. 1 means every tick.
    pub decision_interval_ticks: u32,
    /// Player speed at pace 0 and the extra speed at pace 100.
    pub base_speed: f64,
    pub pace_speed: f64,
    /// Player acceleration at acceleration 0 and the extra at acceleration 100.
    pub base_accel: f64,
    pub accel_bonus: f64,
    /// Steering: slow-down radius for arrive, neighbour radius and strength for separation.
    pub arrive_radius: f64,
    pub separation_radius: f64,
    pub separation_strength: f64,
    /// Minimum distance kept between two players after integration.
    pub min_player_distance: f64,
    /// Ball: ground deceleration, air drag per second, gravity, bounce restitution, speed cap.
    pub ground_friction: f64,
    pub air_drag: f64,
    pub gravity: f64,
    pub restitution: f64,
    pub ball_max_speed: f64,
    /// Possession: reach radius, height under which the ball is controllable, control speed limit,
    /// ticks after gaining the ball during which no tackle succeeds.
    pub reach_radius: f64,
    /// Goalkeeper reach in metres (arms and dive).
    pub keeper_reach: f64,
    /// Metres the goalkeeper stands off the goal line.
    pub keeper_depth: f64,
    /// Shot aim noise in radians (each side).
    pub shot_noise: f64,
    /// Chance a goalkeeper holds a ball faster than `control_speed`.
    pub keeper_catch_chance: f64,
    /// Metres the carried ball moves toward the carry point per tick.
    pub carry_step: f64,
    /// Crossbar height in metres.
    pub crossbar_height: f64,
    pub reach_height: f64,
    pub control_speed: f64,
    pub control_cooldown_ticks: u32,
    /// Distance within which an opponent presses the carrier or chases a loose ball.
    pub press_distance: f64,
    /// Kicks: extra arrival speed for a pass, maximum pass speed, shooting speed, shooting range,
    /// aim noise in radians.
    pub pass_arrival_speed: f64,
    pub pass_max_speed: f64,
    pub shot_speed: f64,
    pub shot_range: f64,
    pub aim_noise: f64,
    /// Formation anchor: fraction of the ball offset added to a slot.
    pub compactness_x: f64,
    pub compactness_y: f64,
    /// Validator: anchor tolerance, the ball distance beyond which it applies, and the ticks the
    /// ball must have been that far away before the rule applies.
    pub anchor_tolerance: f64,
    pub anchor_ball_distance: f64,
    pub anchor_grace_ticks: u32,
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
        }
    }
}

//! The press: its triggers (a back pass, a poor touch, a receiver facing his own goal, the ball
//! going wide), which side a presser shows the carrier, the offside-trap step, goal-kick
//! screening, the tackle decision's foul cost, and the counter-press and counter weights while a
//! regain window is open.

use super::*;

/// The press triggers, in the order of [`crate::sim::Simulation`]'s trigger ticks: a pass
/// toward the passer's own goal, a gain after the ball came too fast to control, a receiver
/// facing his own goal, and a pass to a full-back or into a wide channel.
pub const TRIGGERS: [&str; 4] = ["back_pass", "poor_touch", "facing_back", "ball_wide"];
pub const TRIGGER_BACK_PASS: usize = 0;
pub const TRIGGER_POOR_TOUCH: usize = 1;
pub const TRIGGER_FACING_BACK: usize = 2;
pub const TRIGGER_WIDE: usize = 3;
/// A pass is a back pass when its direction's cosine with the passer's attack direction is
/// below minus this (more than 60 degrees from square, backward).
pub const BACK_PASS_COS: f64 = 0.5;
/// A receiver faces his own goal when his facing's cosine with his attack direction is below
/// minus this.
pub const FACING_BACK_COS: f64 = 0.5;
/// The press-trigger window (hooks `press.trigger.*`): for this many seconds after a trigger
/// the pressing side switches on fires, while the other side keeps the ball, the side's press
/// reach takes [`TRIGGER_REACH`] and [`TRIGGER_PRESSERS`] more players press, and its nearest
/// outfield player presses whatever his reach, at a sprint (an urgent press job).
pub const TRIGGER_WINDOW_S: f64 = 3.0;
pub const TRIGGER_REACH: f64 = 1.5;
pub const TRIGGER_PRESSERS: usize = 1;
/// Force the opponent (hook `press.force`): metres per unit of force the presser's aim moves
/// across, toward the centre to show the carrier wide (force above 0) or toward the near
/// touchline to show him inside (below 0).
pub const FORCE_M: f64 = 2.0;
/// The offside trap (hook `line.offside_trap`): the back line steps up this many metres
/// together for this many seconds once the trap starts.
pub const TRAP_STEP_M: f64 = 5.0;
pub const TRAP_WINDOW_S: f64 = 2.0;
/// Stop short goal kicks (hook `press.screen_short`) and the goal-kick pick: a team-mate of
/// the kicker with an opponent within this many metres of him is screened.
pub const SCREEN_M: f64 = 4.0;
/// The metres of a goal kick's score a screened team-mate loses.
pub const SCREENED_M: f64 = 20.0;
/// The score a pass from the passer's own penalty area to a screened team-mate loses in the
/// carrier's open-play choice (the keeper's goal kick included): the same read as the
/// restart pick's [`SCREENED_M`], in the option scores' units, where it marks the pass as
/// one he expects to lose, as [`OFFSIDE_READ_COST`] does.
pub const SCREENED_COST: f64 = 1.0;
/// The tackle decision (hook `tackle.decide`): the cost of a foul's chance, in units of the
/// win chance, at no danger; the danger of a foul doubles it near the own goal. Set from the
/// probe of the default sides: at 0.1 a defender declines only a tackle he is far likelier to
/// foul in than to win, and the equal suite's goals stay within 0.05 a match of the automatic
/// tackle's (at 0.25 seed 9 rises 0.29, at 0.5 both seeds about 0.35, over 200 matches).
pub const TACKLE_FOUL_COST: f64 = 0.1;

/// Press triggers (hooks `press.trigger.*`): the factor on pressing side `team`'s press reach,
/// [`TRIGGER_REACH`] while the window of a trigger its plan switches on is open, else 1.
pub(crate) fn trigger_factor(view: &MatchView<'_>, team: usize) -> f64 {
    if trigger_open(view, team) {
        TRIGGER_REACH
    } else {
        1.0
    }
}

/// Press triggers: the extra pressers of side `team`, [`TRIGGER_PRESSERS`] while the window of
/// a trigger its plan switches on is open, else 0.
pub(crate) fn trigger_pressers(view: &MatchView<'_>, team: usize) -> usize {
    if trigger_open(view, team) {
        TRIGGER_PRESSERS
    } else {
        0
    }
}

/// The active outfield player of side `team` (not its keeper `keeper`) nearest `at`, with his
/// distance.
pub(super) fn nearest_outfield(
    view: &MatchView<'_>,
    team: usize,
    keeper: usize,
    at: DVec2,
) -> Option<(f64, usize)> {
    view.players()
        .iter()
        .enumerate()
        .filter(|&(i, p)| p.team == team && p.active() && i != keeper)
        .map(|(i, p)| ((p.pos - at).length(), i))
        .min_by(|a, b| a.0.total_cmp(&b.0))
}

/// Whether the window of a press trigger side `team`'s plan switches on is open.
pub(crate) fn trigger_open(view: &MatchView<'_>, team: usize) -> bool {
    let plan = &view.teams()[team].plan;
    let on = [
        plan.trigger_back_pass,
        plan.trigger_poor_touch,
        plan.trigger_facing_back,
        plan.trigger_wide,
    ];
    on.iter().enumerate().any(|(k, &on)| {
        on && view
            .since_trigger_s(team, k)
            .is_some_and(|s| s < TRIGGER_WINDOW_S)
    })
}

/// The line of engagement's press start (hook `press.engage`): `true` while the ball lies
/// within side `team`'s engage line, a share of the pitch length from its own goal line; the
/// default share 1 never limits.
pub(crate) fn engaged(view: &MatchView<'_>, team: usize, ball: DVec2) -> bool {
    let share = view.teams()[team].plan.engage_line;
    if share >= 1.0 {
        return true;
    }
    let pitch = view.pitch();
    let from_own_line = ball.x * view.teams()[team].attack_x + pitch.half_length();
    from_own_line < share * pitch.length()
}

/// On winning the ball (hook `regain.counter`): the on-winning weight of the player in slot
/// `slot` of side `team` (his own plan's) while the side's regain window is open,
/// [`COUNTER_WINDOW_S`] after it won the ball in open play and still on it; 0 otherwise, and
/// 0 at the default level.
pub(crate) fn counter_weight(view: &MatchView<'_>, team: usize, slot: usize) -> f64 {
    match view.since_regain_s(team) {
        Some(s) if s < COUNTER_WINDOW_S => view.teams()[team].plan_for(slot).counter,
        _ => 0.0,
    }
}

/// On losing the ball and counter-press length (hooks `loss.counter_press` and
/// `loss.counter_press_window`): the factor on how far from the carrier side `team`'s nearest
/// players start to press; how many press stays the pressing level's. While its counter-press
/// window is open, `counter_press_s` seconds after it lost the ball in open play with the
/// other side still on it, the factor is 1 plus its counter-press, never below 0; otherwise 1.
/// The default level's counter-press is 0, so a side presses as its pressing level says. The
/// window is the side's; `counter_press` is the pressing player's own (his plan's).
pub(crate) fn counter_press_factor(view: &MatchView<'_>, team: usize, counter_press: f64) -> f64 {
    match view.since_loss_s(team) {
        Some(s) if s < view.teams()[team].plan.counter_press_s => (1.0 + counter_press).max(0.0),
        _ => 1.0,
    }
}

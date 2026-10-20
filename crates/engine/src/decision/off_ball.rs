//! Off-ball runs while the side has the ball: timed runs in behind, forward runs, runs into the
//! box, and the back line holding its height.

use super::*;

/// Timed runs (the off-ball movement hook) of team `team` while it has the ball and carrier
/// `c` is someone else: every active outfield attacker with off-ball movement above the
/// average (skill `s` of his off-ball run stage, 0 to 1) whose target lies beyond the onside
/// line, 0.5 m short of the second-last defender, is brought back toward that line by the
/// share `s`; and his target moves `hooks.anchor_space_m · s` across the pitch away from the
/// nearest defender, kept on the pitch.
pub(super) fn time_runs(
    view: &MatchView<'_>,
    targets: &mut [DVec2; ROSTER],
    team: usize,
    c: usize,
) {
    let space = view.tuning().contract.hooks().anchor_space_m;
    let side = &view.teams()[team];
    let attack_x = side.attack_x;
    let line = offside::second_last_depth(team, view.players(), attack_x) - ONSIDE_MARGIN;
    let speed = offside::second_last_speed(team, view.players(), attack_x);
    let keeper = view.keeper(team);
    for (i, p) in view.players().iter().enumerate() {
        if p.team != team || i == c || i == keeper || !p.active() {
            continue;
        }
        let s = view.skills(i).skill(Stage::OFF_BALL_RUN_EXECUTE).max(0.0);
        if s <= 0.0 {
            continue;
        }
        let line = line + line_lag(view, i, speed);
        let mut at = targets[i];
        let depth = at.x * attack_x;
        if line.is_finite() && depth > line {
            at.x = (depth + s * (line - depth)) * attack_x;
        }
        if space > 0.0 {
            let nearest = view
                .players()
                .iter()
                .filter(|q| q.team != team && q.active())
                .map(|q| q.pos)
                .min_by(|a, b| (*a - at).length().total_cmp(&(*b - at).length()));
            if let Some(q) = nearest {
                let away = if at.y >= q.y { 1.0 } else { -1.0 };
                at.y += away * space * s;
            }
        }
        targets[i] = view.pitch().clamp(at, 0.5);
    }
}

/// `true` for a slot of the attack line: a winger or a striker by the formation's position.
pub(crate) fn attack_line(position: Option<crate::data::team::Position>) -> bool {
    use crate::data::team::Position;
    matches!(position, Some(Position::LW | Position::RW | Position::ST))
}

/// The farthest, in metres, a carrier looks for a runner in behind: the longest pass he
/// considers.
pub(super) const RUN_PASS_M: f64 = 45.0;

/// How far beyond the onside line a run in behind aims, in metres.
pub(super) const RUN_BEYOND_M: f64 = 6.0;
/// A run coming short aims this far back from where the forward stands, in metres, and lasts
/// at most [`COME_SHORT_S`]. Real forwards' coming-short runs take a median of 7.6 m in 1.6 s
/// (SkillCorner's open data) at speed. An engine forward starts a check from standing in his
/// place and brakes into its end, so a check of 8 m keeps him at a run's speed (4 m/s) for
/// under the 1 s a run needs: the engine's run measure read almost none of them. At 12 m the
/// check is a run by the shared definition.
pub(super) const COME_SHORT_M: f64 = 12.0;
pub(super) const COME_SHORT_S: f64 = 4.0;
/// The nearest a run coming short comes to the carrier's depth, in metres.
pub(super) const COME_SHORT_TO_M: f64 = 5.0;
/// A forward within this many metres of his place stands in it.
pub(super) const SETTLED_M: f64 = 2.0;

/// How far from the defensive line side `team`'s attack-line players may start a timed run:
/// its forward runs (hook `run.start`); inside its regain window a positive on-winning weight
/// lets them start a run in behind from up to `counter x COUNTER_RUN_M` metres.
pub(super) fn run_start(view: &MatchView<'_>, team: usize, slot: usize) -> f64 {
    let start = view.teams()[team].plan_for(slot).run_start_m;
    let counter = counter_weight(view, team, slot);
    if counter > 0.0 {
        start.max(counter * COUNTER_RUN_M)
    } else {
        start
    }
}

/// The side's forward runs (hook `run.start`) while team `team` has the ball and carrier `c`
/// is someone else. Nothing moves at the default of 0. The level sets how far from the
/// defensive line a forward may start a timed run: while a pass is on (the carrier is not
/// pressed and the runner stands ahead of him within a pass's reach), an active attack-line
/// player within `|run_start_m|` of the onside line (0.5 m short of the second-last defender)
/// starts one. Above 0 it is a run in behind: he aims [`RUN_BEYOND_M`] past the onside line,
/// so a pass played once he is past it is offside. Below 0 he checks short from his place
/// (within [`SETTLED_M`] of it), pass on or not: he aims [`COME_SHORT_M`] back toward the ball,
/// never nearer than [`COME_SHORT_TO_M`] ahead of the carrier, keeps that point, and is free
/// after [`COME_SHORT_S`]; back in his place he may check again. A run under way (the
/// runner's job is a timed run) goes on until he arrives, and through a pass that travels
/// forward, while he stays within a pass's reach ([`RUN_PASS_M`]) of the ball. The farther from the line a run may start, the more forwards start one, so the
/// frequency rises with the level. Returns the roster bits of the runners.
pub(super) fn forward_runs(
    view: &MatchView<'_>,
    targets: &mut [DVec2; ROSTER],
    team: usize,
    c: usize,
) -> u32 {
    let side = &view.teams()[team];
    // Each runner's start reads his own plan (a player instruction may set it); the side
    // runs nothing when every player's start is 0.
    let starts: [f64; PLAYERS_PER_TEAM] = std::array::from_fn(|slot| run_start(view, team, slot));
    let mut running = 0u32;
    if starts.iter().all(|&s| s == 0.0) {
        return running;
    }
    let attack_x = side.attack_x;
    let line = offside::second_last_depth(team, view.players(), attack_x) - ONSIDE_MARGIN;
    if !line.is_finite() {
        return running;
    }
    let speed = offside::second_last_speed(team, view.players(), attack_x);
    let carrier = view.player(c);
    let carrier_depth = carrier.pos.x * attack_x;
    let pass_on = !pressed(view, c);
    for (i, p) in view.players().iter().enumerate() {
        if p.team != team || i == c || !p.active() || !attack_line(side.plan.slots[p.slot].position)
        {
            continue;
        }
        let start = starts[p.slot];
        if start == 0.0 {
            continue;
        }
        let behind = start > 0.0;
        let line = line + line_lag(view, i, speed);
        let depth = p.pos.x * attack_x;
        let gap = line - depth;
        let from_carrier = p.pos - carrier.pos;
        // In behind he aims past the line, keeping his place across; coming short he aims at
        // the carrier's feet, `COME_SHORT_TO_M` from him on the line between them.
        let (aim, to_go) = if behind {
            let mut at = targets[i];
            at.x = (line + RUN_BEYOND_M) * attack_x;
            (at, line + RUN_BEYOND_M - depth)
        } else {
            let to = (depth - COME_SHORT_M).max(carrier_depth + COME_SHORT_TO_M);
            let mut at = targets[i];
            at.x = to * attack_x;
            (at, depth - to)
        };
        if to_go <= 1.0 {
            continue;
        }
        let ahead = depth > carrier_depth && from_carrier.length() <= RUN_PASS_M;
        // An onside forward starts a run in behind; any forward may come back short.
        let near = gap <= start.abs() && (!behind || gap >= 0.0);
        // A forward checks short from his place: one away from it (coming back from a check
        // or still moving to it) starts no new check.
        let settled = behind || (p.pos - targets[i]).length() <= SETTLED_M;
        let starts = (pass_on || !behind) && ahead && near && settled;
        // A run coming short lasts [`COME_SHORT_S`]: a check toward the ball, then he is free.
        let short_left = behind
            || f64::from(view.tick().saturating_sub(p.pace.since))
                < COME_SHORT_S * f64::from(crate::TICKS_PER_SECOND);
        // A run goes on only while the runner stays within a pass's reach of the ball, the
        // reach a run starts in: real runs in behind end within it (99.4 percent of 5,850
        // tracked runs end within 45 m of the ball).
        let in_reach = from_carrier.length() <= RUN_PASS_M;
        let under_way = p.pace.job == Job::TimedRun && short_left && in_reach;
        if !(starts || under_way) {
            continue;
        }
        // A check under way keeps the point it set out for; a run in behind follows the line.
        let aim = if under_way && !starts && !behind {
            p.target
        } else {
            aim
        };
        targets[i] = view.pitch().clamp(aim, 0.5);
        running |= 1 << i;
    }
    running
}

/// Box runs (named mechanism) while team `team` has the ball in a wide channel of its final
/// third, carried by `c`: every other active attack-line player runs into the penalty area,
/// to the depth of the penalty mark and no wider than the posts' channel
/// ([`LATE_RUN_DEPTH_M`], [`LATE_RUN_WIDTH_M`]), each keeping his side of the pitch, never past
/// the onside line. A cross then has a team-mate to find. Returns the roster bits of the
/// runners.
pub(super) fn box_runs(
    view: &MatchView<'_>,
    targets: &mut [DVec2; ROSTER],
    team: usize,
    c: usize,
) -> u32 {
    let side = &view.teams()[team];
    let attack_x = side.attack_x;
    let pitch = view.pitch();
    let half = pitch.half_length();
    let ball = view.player(c).pos;
    let mut runners = 0u32;
    if ball.y.abs() <= contract::CROSS_WIDE_M || ball.x * attack_x <= half / 3.0 {
        return runners;
    }
    let line = offside::second_last_depth(team, view.players(), attack_x) - ONSIDE_MARGIN;
    let speed = offside::second_last_speed(team, view.players(), attack_x);
    for (i, p) in view.players().iter().enumerate() {
        if p.team != team || i == c || !p.active() || !attack_line(side.plan.slots[p.slot].position)
        {
            continue;
        }
        let depth = if line.is_finite() {
            (half - LATE_RUN_DEPTH_M).min(line + line_lag(view, i, speed))
        } else {
            half - LATE_RUN_DEPTH_M
        };
        let at = DVec2::new(
            depth * attack_x,
            targets[i].y.clamp(-LATE_RUN_WIDTH_M, LATE_RUN_WIDTH_M),
        );
        targets[i] = pitch.clamp(at, 0.5);
        runners |= 1 << i;
    }
    runners
}

/// The line hold (named mechanism) of team `team`'s lone forward while his team has the
/// ball and carrier `c` is someone else: his target moves from his anchor toward the
/// onside line, 0.5 m short of the second-last defender, by the share `lone_line_hold`,
/// and never past that line. His place across the pitch stays the anchor's.
pub(super) fn hold_the_line(
    view: &MatchView<'_>,
    targets: &mut [DVec2; ROSTER],
    team: usize,
    c: usize,
) {
    let share = view.tuning().lone_line_hold;
    if share == 0.0 {
        return;
    }
    let side = &view.teams()[team];
    let Some(slot) = side.lone_forward() else {
        return;
    };
    let i = team * PLAYERS_PER_TEAM + slot;
    if i == c || !view.player(i).active() {
        return;
    }
    let attack_x = side.attack_x;
    let line = offside::second_last_depth(team, view.players(), attack_x) - ONSIDE_MARGIN;
    if !line.is_finite() {
        return;
    }
    let line = line
        + line_lag(
            view,
            i,
            offside::second_last_speed(team, view.players(), attack_x),
        );
    let anchor = targets[i];
    let depth = anchor.x * attack_x;
    let held = (depth + share * (line - depth)).min(line);
    targets[i] = view
        .pitch()
        .clamp(DVec2::new(held * attack_x, anchor.y), 0.5);
}

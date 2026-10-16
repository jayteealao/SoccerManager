//! Restarts (IFAB Laws 8 and 13 to 17): who takes each restart, where every other player
//! stands while the ball is dead, and when the restart may be taken. A dropped ball after an
//! injury goes to one player of the team that last touched the ball, and every other player
//! keeps 4 m away (IFAB Law 8).
//!
//! While the ball is dead, every player steers to a restart target: the formation anchor
//! around the restart spot, moved where a law requires it. Opponents keep 9.15 m from the ball
//! at a free kick, a corner, and a kick-off, stay outside the penalty area at a goal kick, and
//! at a penalty everyone but the taker and the defending goalkeeper waits outside the area,
//! behind the mark and 9.15 m from it.
//!
//! A stoppage lasts its visible steps plus a drawn extra. The visible steps are done once the
//! taker is at the ball and every player the law moves has moved; the drawn extra comes from
//! the restart wait table (`content/restart-waits.json`) and stands for the unseen part, such
//! as fetching the ball: the ball waits where it left play for the extra, then appears at the
//! spot, and the restart is taken the extra after the visible steps are done. A team leading
//! late takes the table's late-lead factor times the extra, and a leading team with time
//! wasting on its time-wasting level times it again. At a throw-in, a free kick or a goal kick
//! the taker may restart quickly, with no extra, when a team-mate is free and the opponents
//! are out of shape. Content without the table waits the tuning's fixed delays instead, with
//! no fetch and no quick restart.

use crate::data::rules::StoppageKind;
use crate::math::{self, DVec2, toward};
use crate::modules::{MatchView, ModuleCard, RestartsModule};
use crate::pace::Job;
use crate::pitch::{KICK_DISTANCE, PENALTY_AREA_DEPTH, PENALTY_AREA_WIDTH, PENALTY_MARK, Pitch};
use crate::player::Player;
use crate::streams::Action;
use crate::team::{PLAYERS_PER_TEAM, Team};
use crate::tuning::Tuning;

use super::positions::{behind_the_mark, on_goal_line};

/// Distance opponents keep from a throw-in (IFAB Law 15).
pub(crate) const THROW_IN_DISTANCE: f64 = 2.0;
/// Distance every other player keeps from a dropped ball (IFAB Law 8).
pub(crate) const DROP_BALL_DISTANCE: f64 = 4.0;
/// Slack between where a player is sent and where the law is judged, in metres.
const TARGET_MARGIN: f64 = 0.5;
/// Slack the referee allows when judging a distance the law asks for, in metres: readiness
/// and the restart position check both use it.
pub(crate) const JUDGE_MARGIN: f64 = 0.25;

/// A dead ball waiting for its restart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeadBall {
    /// The restart: kick-off, throw-in, corner, goal kick, free kick, penalty, or a dropped
    /// ball after an injury.
    pub kind: StoppageKind,
    /// The team that restarts play.
    pub team: usize,
    pub spot: DVec2,
    /// A direct free kick may be shot at goal; an indirect one is always a pass.
    pub direct: bool,
    /// The tick the ball went dead.
    pub since: u32,
    /// The first tick the restart may be taken, once its visible steps are done. With a wait
    /// table it is [`DeadBall::UNSET`] until the visible steps are first done, then that tick
    /// plus the drawn extra. Without one, and at a shoot-out kick, it is the fixed delay after
    /// `since`.
    pub ready_at: u32,
    /// The roster index of the player who takes the restart.
    pub taker: usize,
    /// Where the ball left play: the crossing point, the place of a foul or an injury, or the
    /// net. The ball waits here until `appear_at`.
    pub exit: DVec2,
    /// The tick the ball appears at the spot: `since` plus the drawn extra. The record of this
    /// tick carries the restart flag.
    pub appear_at: u32,
    /// The drawn extra, in ticks: 0 at a quick restart and without a wait table.
    pub extra: u32,
    /// `true` when the taker restarts quickly, with no drawn extra.
    pub quick: bool,
    /// The tick by which the restart is taken whatever the players are doing.
    pub hard_at: u32,
}

impl DeadBall {
    /// `ready_at` before the visible steps are done.
    pub const UNSET: u32 = u32::MAX;

    /// A dead ball that waits a fixed `delay` from `since` with the ball already at the spot:
    /// the restart of content without a wait table and the shoot-out kick. The hard limit is
    /// three times the delay, and at least one tick after the ball went dead.
    pub fn fixed(
        kind: StoppageKind,
        team: usize,
        spot: DVec2,
        direct: bool,
        since: u32,
        delay: u32,
        taker: usize,
    ) -> Self {
        Self {
            kind,
            team,
            spot,
            direct,
            since,
            ready_at: since + delay,
            taker,
            exit: spot,
            appear_at: since,
            extra: 0,
            quick: false,
            hard_at: since + (3 * delay).max(1),
        }
    }

    /// The tick by which the restart is taken whatever the players are doing.
    pub fn hard_limit(&self) -> u32 {
        self.hard_at
    }
}

/// Ticks a restart of `kind` waits before it may be taken, without a wait table.
pub fn delay_ticks(kind: StoppageKind, t: &Tuning) -> u32 {
    let d = &t.restart_delay_s;
    let seconds = match kind {
        StoppageKind::KickOff => d.kick_off,
        StoppageKind::ThrowIn => d.throw_in,
        StoppageKind::Corner => d.corner,
        StoppageKind::GoalKick => d.goal_kick,
        StoppageKind::Penalty => d.penalty,
        StoppageKind::Injury => d.drop_ball,
        _ => d.free_kick,
    };
    // The tuning bounds keep the delay under 60 s, so the cast cannot truncate.
    (seconds / t.dt).round() as u32
}

/// The player of `team` who takes a restart of `kind` at `spot`: the acting goalkeeper at a
/// goal kick and at a dropped ball inside its own penalty area, the centre-forward at a
/// kick-off (unless that player keeps goal), otherwise the nearest outfield player on the
/// pitch. A team with only its goalkeeper left uses the goalkeeper. The team's own end comes
/// from the way it attacks, never from where a player stands.
pub fn taker(
    kind: StoppageKind,
    team: usize,
    spot: DVec2,
    players: &[Player],
    teams: &[Team; 2],
) -> usize {
    let first = team * PLAYERS_PER_TEAM;
    let keeper = first + teams[team].keeper_slot();
    if let Some(i) = preferred_taker(kind, team, spot, teams)
        && players[i].active()
    {
        return i;
    }
    let mut best: Option<(f64, usize)> = None;
    for (i, p) in players.iter().enumerate() {
        if p.team != team
            || !p.active()
            || (i == keeper && !matches!(kind, StoppageKind::GoalKick | StoppageKind::Injury))
        {
            continue;
        }
        let d = (p.pos - spot).length();
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, i));
        }
    }
    best.map_or(keeper, |(_, i)| i)
}

/// The player the law or custom names for a restart before the nearest player: the
/// keeper for a goal kick and for a dropped ball in his own penalty area, the last lineup
/// slot for a kick-off (unless he keeps goal). `taker` uses him when he is on the pitch.
pub fn preferred_taker(
    kind: StoppageKind,
    team: usize,
    spot: DVec2,
    teams: &[Team; 2],
) -> Option<usize> {
    let first = team * PLAYERS_PER_TEAM;
    let own_end = -teams[team].attack_x;
    let keeper = first + teams[team].keeper_slot();
    match kind {
        StoppageKind::GoalKick => Some(keeper),
        StoppageKind::KickOff => Some(first + PLAYERS_PER_TEAM - 1).filter(|&i| i != keeper),
        StoppageKind::Injury if teams[team].pitch().in_penalty_area(spot, own_end) => Some(keeper),
        _ => None,
    }
}

/// Where a player of `team` stands for a kick-off: the formation slot, mirrored in length and
/// width for a team attacking negative `x` ([`Team::slot_base`]), kept inside its own half.
pub fn kick_off_position(team: &Team, slot: usize) -> DVec2 {
    let base = team.slot_base(slot);
    let depth = base.x * team.attack_x;
    DVec2::new(if depth > -1.0 { -team.attack_x } else { base.x }, base.y)
}

/// The least distance between two players of the other team on the edge of the centre
/// circle at a kick-off, in metres: more than twice the separation that pushes players apart,
/// so two of them never share a place and one push the other inside the circle.
const KICK_OFF_GAP: f64 = 3.0;

/// Where a player of `team` stands for a kick-off by his own team or by the other team
/// (`opponent`): the formation slot in his own half, and for the other team outside the
/// centre circle, 9.15 m from the ball and the target margin more (IFAB Law 8). Players of the
/// other team whose places meet on the edge of the circle, such as forwards brought back from
/// the other half, spread along it, the gap apart, in slot order, alternately to either side
/// of the first. The same rule places the other team at a kick-off after a goal, so both
/// kick-offs agree.
pub fn kick_off_spot(team: &Team, slot: usize, opponent: bool) -> DVec2 {
    let base = kick_off_position(team, slot);
    if !opponent {
        return base;
    }
    let on_edge = |s: usize| {
        outside_on_pitch(
            team.pitch(),
            kick_off_position(team, s),
            DVec2::ZERO,
            KICK_DISTANCE + TARGET_MARGIN,
            DVec2::new(-team.attack_x, 0.0),
            0.5,
        )
    };
    let at = on_edge(slot);
    let shared = (0..slot)
        .filter(|&s| (on_edge(s) - at).length() < KICK_OFF_GAP)
        .count();
    if shared == 0 {
        return at;
    }
    // Alternately one more gap to one side and the other: +1, -1, +2, -2, ...
    let steps = shared.div_ceil(2) as f64;
    let sign = if shared % 2 == 1 { 1.0 } else { -1.0 };
    let radius = at.length();
    let angle = math::atan2(at.y, at.x) + sign * steps * KICK_OFF_GAP / radius;
    let (sin, cos) = math::sin_cos(angle);
    team.pitch().clamp(DVec2::new(cos, sin) * radius, 0.5)
}

/// Where the taker stands: at the ball, or just behind it at a kick-off.
fn taker_target(dead: &DeadBall, teams: &[Team; 2]) -> DVec2 {
    if dead.kind == StoppageKind::KickOff {
        dead.spot - DVec2::new(0.5 * teams[dead.team].attack_x, 0.0)
    } else {
        teams[dead.team].pitch().clamp(dead.spot, 0.2)
    }
}

/// The side of the pitch (the sign of `x`) of the goal nearest `spot`.
fn end_of(spot: DVec2) -> f64 {
    if spot.x < 0.0 { -1.0 } else { 1.0 }
}

/// `p` moved radially out of the circle of `radius` around `centre`.
fn outside(p: DVec2, centre: DVec2, radius: f64, away: DVec2) -> DVec2 {
    let d = (p - centre).length();
    if d >= radius {
        return p;
    }
    let dir = if d > 1e-6 { toward(centre, p) } else { away };
    centre + dir * radius
}

/// `p` moved onto the circle of `radius` around `centre`, biased `away`, then clamped inside
/// the pitch without slipping back inside the circle. Near a touchline or goal line, clamping
/// the primary point straight back onto the pitch can pull it within `radius` of `centre`
/// again (the circle spills off the pitch there), and a player sent to that point would never
/// clear the circle, stalling the restart until the hard time limit.
///
/// When that happens, the direction from `centre` is mirrored across whichever line the clamp
/// touched, so the point swings back onto the pitch instead of piling up on the line. Mirroring
/// keeps the direction tied to `p`, so two players pushed off the same spot from different
/// angles still land on different points instead of colliding on one. If the mirrored point is
/// still pulled back (a spot pinned in a corner, against both a touchline and a goal line), the
/// point falls back to sliding straight toward the centre of the pitch, which the circle always
/// reaches without spilling off since `radius` is well under both the pitch's half-length and
/// half-width.
fn outside_on_pitch(
    pitch: &Pitch,
    p: DVec2,
    centre: DVec2,
    radius: f64,
    away: DVec2,
    margin: f64,
) -> DVec2 {
    let clear_of_circle = |q: DVec2| (q - centre).length() >= radius - 1e-9;

    let raw = outside(p, centre, radius, away);
    let primary = pitch.clamp(raw, margin);
    if clear_of_circle(primary) {
        return primary;
    }

    let mut dir = toward(centre, p);
    if dir == DVec2::ZERO {
        dir = away;
    }
    if primary.x != raw.x {
        dir.x = -dir.x;
    }
    if primary.y != raw.y {
        dir.y = -dir.y;
    }
    let mirrored = pitch.clamp(centre + dir * radius, margin);
    if clear_of_circle(mirrored) {
        return mirrored;
    }

    let mut inward = toward(centre, DVec2::ZERO);
    if inward == DVec2::ZERO {
        inward = away;
    }
    pitch.clamp(centre + inward * radius, margin)
}

/// `p` moved out of the penalty area at the `side` end.
fn out_of_area(pitch: &Pitch, p: DVec2, side: f64) -> DVec2 {
    if pitch.in_penalty_area(p, side) {
        DVec2::new(side * (pitch.half_length() - PENALTY_AREA_DEPTH - 1.0), p.y)
    } else {
        p
    }
}

/// Where a player other than the taker and the defending goalkeeper waits at a penalty kicked
/// from `spot` at the `side` end, from his place `base` (IFAB Law 14): outside the area,
/// 9.15 m from the mark, and behind it. A place beside the area but nearer the goal line than
/// the mark, which a long ground's deep anchors give, moves back to the area's line first.
fn penalty_place(pitch: &Pitch, base: DVec2, spot: DVec2, side: f64) -> DVec2 {
    let keep = KICK_DISTANCE + TARGET_MARGIN;
    let behind = DVec2::new(-side, 0.0);
    let at = outside_on_pitch(
        pitch,
        out_of_area(pitch, base, side),
        spot,
        keep,
        behind,
        0.5,
    );
    if at.x * side <= pitch.half_length() - PENALTY_MARK - TARGET_MARGIN {
        return at;
    }
    let back = DVec2::new(
        side * (pitch.half_length() - PENALTY_AREA_DEPTH - 1.0),
        at.y,
    );
    outside_on_pitch(pitch, back, spot, keep, behind, 0.5)
}

/// The restart target of player `i`.
pub fn target(
    dead: &DeadBall,
    i: usize,
    players: &[Player],
    teams: &[Team; 2],
    t: &Tuning,
) -> DVec2 {
    if i == dead.taker {
        return taker_target(dead, teams);
    }
    let p = &players[i];
    let team = &teams[p.team];
    let pitch = team.pitch();
    let opponent = p.team != dead.team;
    // An opponent pushed off a spot backs toward its own goal.
    let away = DVec2::new(-team.attack_x, 0.0);
    let base = match dead.kind {
        StoppageKind::KickOff => kick_off_position(team, p.slot),
        // The team that takes the restart has the ball.
        _ => team.anchor(p.slot, dead.spot, !opponent, t),
    };
    let keep = KICK_DISTANCE + TARGET_MARGIN;
    let at = match dead.kind {
        // The kick-off is taken from the centre mark: the kick-off places, spread.
        StoppageKind::KickOff if opponent => kick_off_spot(team, p.slot, true),
        StoppageKind::FreeKick | StoppageKind::Corner if opponent => {
            outside_on_pitch(pitch, base, dead.spot, keep, away, 0.5)
        }
        StoppageKind::ThrowIn if opponent => outside_on_pitch(
            pitch,
            base,
            dead.spot,
            THROW_IN_DISTANCE + TARGET_MARGIN,
            away,
            0.5,
        ),
        StoppageKind::GoalKick if opponent => {
            let at = screen_target(dead, i, players, teams).unwrap_or(base);
            out_of_area(pitch, at, end_of(dead.spot))
        }
        StoppageKind::Injury => outside_on_pitch(
            pitch,
            base,
            dead.spot,
            DROP_BALL_DISTANCE + TARGET_MARGIN,
            away,
            0.5,
        ),
        StoppageKind::Penalty => {
            let side = end_of(dead.spot);
            if opponent && team.keeper_slot() == p.slot {
                DVec2::new(side * (pitch.half_length() - 0.3), 0.0)
            } else {
                penalty_place(pitch, base, dead.spot, side)
            }
        }
        _ => base,
    };
    pitch.clamp(at, 0.5)
}

/// Stop short goal kicks (hook `press.screen_short`): at the other side's goal kick, a side
/// whose plan screens covers the kicker's short options nearest the kick first: taking the
/// kicking side's outfield players within the goal kick's short reach in order of their
/// distance from the spot, each is screened by the nearest of the side's active outfield
/// players in front of its back line not yet given one, who stands beside him on the taker's
/// side, [`crate::decision::SCREEN_M`] / 2 metres off. `None` for a player who screens nobody.
fn screen_target(
    dead: &DeadBall,
    i: usize,
    players: &[Player],
    teams: &[Team; 2],
) -> Option<DVec2> {
    let me = &players[i];
    let side = &teams[me.team];
    if !side.plan.screen_short || side.in_back_line(me.slot) || me.slot == side.keeper_slot() {
        return None;
    }
    let short = crate::decision::GOAL_KICK_REACH_M;
    let kicking = &teams[dead.team];
    let mut marks: Vec<usize> = (0..players.len())
        .filter(|&j| {
            let q = &players[j];
            q.team == dead.team
                && q.active()
                && j != dead.taker
                && q.slot != kicking.keeper_slot()
                && (q.pos - dead.spot).length() < short
        })
        .collect();
    marks.sort_by(|&a, &b| {
        (players[a].pos - dead.spot)
            .length()
            .total_cmp(&(players[b].pos - dead.spot).length())
            .then(a.cmp(&b))
    });
    let mut given = 0u32;
    for m in marks {
        let mark = players[m].pos;
        let k = (0..players.len())
            .filter(|&k| {
                let p = &players[k];
                p.team == me.team
                    && p.active()
                    && !side.in_back_line(p.slot)
                    && p.slot != side.keeper_slot()
                    && given & (1u32 << k) == 0
            })
            .min_by(|&a, &b| {
                (players[a].pos - mark)
                    .length()
                    .total_cmp(&(players[b].pos - mark).length())
                    .then(a.cmp(&b))
            })?;
        given |= 1u32 << k;
        if k == i {
            let to_ball = crate::math::toward(mark, dead.spot);
            return Some(mark + to_ball * (0.5 * crate::decision::SCREEN_M));
        }
    }
    None
}
/// Room, in metres, that a player walking round the circle the law keeps him out of leaves
/// outside it.
const ROUND_MARGIN: f64 = 1.0;

/// The distance the law keeps player `i` from the spot of `dead` while it waits: an opponent
/// at a throw-in (2 m, IFAB Law 15), at a free kick, a corner and a kick-off (9.15 m, Laws 13,
/// 17 and 8), everyone but the taker at a dropped ball (4 m, Law 8), and everyone but the taker
/// and the defending goalkeeper at a penalty (9.15 m from the mark, Law 14). `None` for a
/// player the law keeps nowhere, and at a goal kick, whose rule is the penalty area.
fn kept_distance(dead: &DeadBall, i: usize, players: &[Player], teams: &[Team; 2]) -> Option<f64> {
    if i == dead.taker {
        return None;
    }
    let p = &players[i];
    let opponent = p.team != dead.team;
    match dead.kind {
        StoppageKind::ThrowIn if opponent => Some(THROW_IN_DISTANCE),
        StoppageKind::FreeKick | StoppageKind::Corner | StoppageKind::KickOff if opponent => {
            Some(KICK_DISTANCE)
        }
        StoppageKind::Injury => Some(DROP_BALL_DISTANCE),
        StoppageKind::Penalty if !(opponent && teams[p.team].keeper_slot() == p.slot) => {
            Some(KICK_DISTANCE)
        }
        _ => None,
    }
}

/// The point player `i` steers to on his way to his restart `place` while `dead` waits. A
/// player the law keeps a distance from the spot never walks through that circle, so he never
/// pushes the taker off the ball: when he is inside the circle, or his straight path would come
/// nearer the spot than the distance, he heads for a point `ROUND_MARGIN` outside the circle:
/// straight out from the spot when he is inside it, or a step round the circle toward
/// his place's side when he is outside it, the shorter way unless that way leaves the pitch
/// further. The loop asks every tick, so he walks round the circle and then straight to his
/// place. Every other player, a place the law does not clear, and a point the pitch's edge
/// would pull back inside the distance get `place` itself, as before.
pub fn route(
    dead: &DeadBall,
    i: usize,
    players: &[Player],
    teams: &[Team; 2],
    place: DVec2,
) -> DVec2 {
    let Some(distance) = kept_distance(dead, i, players, teams) else {
        return place;
    };
    let spot = dead.spot;
    let pos = players[i].pos;
    let from = (pos - spot).length();
    if (place - spot).length() < distance - JUDGE_MARGIN {
        return place;
    }
    if from >= distance && math::segment_distance(spot, pos, place) >= distance - 1e-9 {
        return place;
    }
    let round = distance + ROUND_MARGIN;
    let pitch = teams[players[i].team].pitch();
    // A point the pitch's edge has not pulled back inside the distance; a spot near a line
    // has part of its circle off the pitch.
    let clear = |q: DVec2| (q - spot).length() >= distance;
    if from < distance {
        // Out of the circle by the shortest way that stays on the pitch.
        let at = outside_on_pitch(pitch, pos, spot, round, toward(spot, place), 0.5);
        return if clear(at) { at } else { place };
    }
    // Round the circle from his own bearing to his place's, the shorter way unless that way
    // leaves the pitch further (a spot near a line has part of its circle off the pitch).
    let out = toward(spot, pos);
    let to = toward(spot, place);
    let short = math::atan2(out.perp_dot(to), out.dot(to));
    let long = if short < 0.0 {
        short + std::f64::consts::TAU
    } else {
        short - std::f64::consts::TAU
    };
    let on_round = |angle: f64| {
        let (sin, cos) = math::sin_cos(angle);
        spot + DVec2::new(out.x * cos - out.y * sin, out.x * sin + out.y * cos) * round
    };
    let off_pitch = |arc: f64| {
        (1..ARC_SAMPLES)
            .map(|k| {
                let q = on_round(arc * k as f64 / ARC_SAMPLES as f64);
                (pitch.clamp(q, 0.5) - q).length()
            })
            .fold(0.0, f64::max)
    };
    let arc = if off_pitch(long) < off_pitch(short) {
        long
    } else {
        short
    };
    // At most the turn whose chord on the walking circle stays outside the distance.
    let most = math::acos(distance / round);
    let at = pitch.clamp(on_round(arc.clamp(-most, most)), 0.5);
    if clear(at) { at } else { place }
}

/// Points along an arc of the walking circle that `route` checks against the pitch's edge.
const ARC_SAMPLES: usize = 12;

/// Where player `i` stands for a kick of the penalty shoot-out (IFAB Law 10): the kicker at
/// the mark, the defending keeper `keepers[1 - dead.team]` on the goal line, the kicking
/// team's keeper on the goal line where it meets the penalty-area line, and every other
/// player inside the centre circle.
pub fn shootout_target(pitch: &Pitch, dead: &DeadBall, i: usize, keepers: [usize; 2]) -> DVec2 {
    let end = end_of(dead.spot);
    if i == dead.taker {
        pitch.clamp(dead.spot, 0.2)
    } else if i == keepers[1 - dead.team] {
        DVec2::new(end * (pitch.half_length() - 0.3), 0.0)
    } else if i == keepers[dead.team] {
        DVec2::new(
            end * (pitch.half_length() - 0.5),
            PENALTY_AREA_WIDTH / 2.0 + 1.0,
        )
    } else {
        // Spread round the centre spot so nobody stands on anybody else.
        let angle = std::f64::consts::TAU * i as f64 / (2 * PLAYERS_PER_TEAM) as f64;
        DVec2::new(
            CIRCLE_SPREAD * math::cos(angle),
            CIRCLE_SPREAD * math::sin(angle),
        )
    }
}

/// Radius, in metres, of the ring the players waiting in the centre circle stand on.
const CIRCLE_SPREAD: f64 = 5.0;

/// `true` when a shoot-out kick may be taken at `tick`: the delay has passed, the kicker is at
/// the mark, the defending keeper is on the goal line, and every other player is outside the
/// penalty area and 9.15 m from the mark.
pub fn shootout_ready(
    pitch: &Pitch,
    dead: &DeadBall,
    tick: u32,
    players: &[Player],
    keepers: [usize; 2],
    t: &Tuning,
) -> bool {
    if tick < dead.ready_at {
        return false;
    }
    let side = end_of(dead.spot);
    players.iter().enumerate().all(|(i, p)| {
        if !p.active() {
            return true;
        }
        if i == dead.taker {
            let at = shootout_target(pitch, dead, i, keepers);
            return (p.pos - at).length() <= t.restart_ready_radius;
        }
        if i == keepers[1 - dead.team] {
            // Law 10: the defending keeper on his goal line between the posts.
            return on_goal_line(pitch, p.pos, side, t.restart_ready_radius);
        }
        !pitch.in_penalty_area(p.pos, side)
            && (p.pos - dead.spot).length() >= KICK_DISTANCE - JUDGE_MARGIN
    })
}

/// `true` when the visible steps of the restart are done: the taker is at the ball, and every
/// player the law moves has moved (at a penalty: out of the area, 9.15 m from the mark and
/// behind it). The wait before the restart is taken is the loop's.
pub fn steps_done(dead: &DeadBall, players: &[Player], teams: &[Team; 2], t: &Tuning) -> bool {
    let taker = &players[dead.taker];
    if (taker.pos - taker_target(dead, teams)).length() > t.restart_ready_radius {
        return false;
    }
    let side = end_of(dead.spot);
    let pitch = teams[0].pitch();
    players.iter().enumerate().all(|(i, p)| {
        if i == dead.taker || !p.active() {
            return true;
        }
        let opponent = p.team != dead.team;
        let distance = (p.pos - dead.spot).length();
        match dead.kind {
            StoppageKind::FreeKick | StoppageKind::Corner => {
                !opponent || distance >= KICK_DISTANCE - JUDGE_MARGIN
            }
            StoppageKind::ThrowIn => !opponent || distance >= THROW_IN_DISTANCE - JUDGE_MARGIN,
            StoppageKind::Injury => distance >= DROP_BALL_DISTANCE - JUDGE_MARGIN,
            StoppageKind::GoalKick => !opponent || !pitch.in_penalty_area(p.pos, side),
            StoppageKind::KickOff => {
                let own_half = p.pos.x * teams[p.team].attack_x <= JUDGE_MARGIN;
                own_half && (!opponent || distance >= KICK_DISTANCE - JUDGE_MARGIN)
            }
            StoppageKind::Penalty => {
                if opponent && teams[p.team].keeper_slot() == p.slot {
                    // Law 14: the defending keeper on his goal line between the posts.
                    on_goal_line(pitch, p.pos, side, t.restart_ready_radius)
                } else {
                    !pitch.in_penalty_area(p.pos, side)
                        && distance >= KICK_DISTANCE - JUDGE_MARGIN
                        && behind_the_mark(pitch, p.pos, side)
                }
            }
            _ => true,
        }
    })
}

/// `true` when `team` leads in the late window of regulation: the last `late_minutes` of the
/// pace tuning, the same late the pace selector reads.
pub(crate) fn leads_late(view: &MatchView<'_>, team: usize) -> bool {
    let goals = view.goals();
    let minute = view.referee().clock.minute(view.tick()).0;
    goals[team] > goals[1 - team]
        && minute + view.tuning().pace.late_minutes >= view.rules().regulation_minutes()
}

/// The drawn extra of a restart of `kind` by `team` for the uniform draw `u`, in ticks: the
/// wait table's draw, times the late-lead factor when `team` leads late, times its
/// time-wasting level when it leads with time wasting on. 0 without a wait table.
pub fn extra_ticks(view: &MatchView<'_>, kind: StoppageKind, team: usize, u: f64) -> u32 {
    let Some(waits) = view.restart_waits() else {
        return 0;
    };
    let mut seconds = waits.draw(kind, u);
    if leads_late(view, team) {
        seconds *= waits.late_lead_factor;
    }
    let goals = view.goals();
    let wasting = view.teams()[team].plan.time_wasting;
    if goals[team] > goals[1 - team] && wasting > 1.0 {
        seconds *= wasting;
    }
    // At most 600 s times factors of at most 5 and 20, so the cast fits.
    (seconds / view.tuning().dt).round() as u32
}

/// The chance that the taker of `dead` restarts quickly, with no drawn extra: 0 without a wait
/// table, at a kind the table does not allow, when his team wastes time, when no team-mate is
/// free (an outfield team-mate on the pitch within `free_mate_m` of the spot with no opponent
/// within `free_space_m` of him), or when the opponents are in shape (at least their
/// mentality minimum of outfield players goal-side of the spot, never more than their outfield
/// players less one). Otherwise the table's chance, higher when his team trails and with his
/// decisions and flair.
pub fn quick_chance(view: &MatchView<'_>, dead: &DeadBall) -> f64 {
    let Some(waits) = view.restart_waits() else {
        return 0.0;
    };
    let teams = view.teams();
    if !waits.quick_kind(dead.kind) || teams[dead.team].plan.time_wasting > 1.0 {
        return 0.0;
    }
    let q = &waits.quick;
    let players = view.players();
    let team = dead.team;
    let other = 1 - team;
    let keepers = [view.keeper(0), view.keeper(1)];
    let outfield = |side: usize| {
        players
            .iter()
            .enumerate()
            .filter(move |&(i, p)| p.team == side && p.active() && i != keepers[side])
    };
    let free = outfield(team).any(|(i, mate)| {
        i != dead.taker
            && (mate.pos - dead.spot).length() <= q.free_mate_m
            && players
                .iter()
                .filter(|o| o.team == other && o.active())
                .all(|o| (o.pos - mate.pos).length() > q.free_space_m)
    });
    if !free {
        return 0.0;
    }
    let side = &teams[other];
    let active = outfield(other).count();
    let minimum = side.plan.rest_defence.min(active.saturating_sub(1));
    let behind = outfield(other)
        .filter(|(_, p)| (p.pos.x - dead.spot.x) * side.attack_x < 0.0)
        .count();
    if behind >= minimum {
        return 0.0;
    }
    let goals = view.goals();
    q.chance(
        goals[team] < goals[other],
        players[dead.taker].derived.quick_share,
    )
}

/// The job of player `i` at `dead`: `Return` (NORMAL) for an opponent the law moves (inside
/// the distance the law asks from the ball, or inside the area at a goal kick or a penalty),
/// `Wait` (CALM) for everyone else, the taker included.
pub fn job(dead: &DeadBall, i: usize, players: &[Player], teams: &[Team; 2]) -> Job {
    let p = &players[i];
    if i == dead.taker || p.team == dead.team {
        return Job::Wait;
    }
    let pitch = teams[p.team].pitch();
    let distance = (p.pos - dead.spot).length();
    let moved = match dead.kind {
        StoppageKind::FreeKick | StoppageKind::Corner | StoppageKind::KickOff => {
            distance < KICK_DISTANCE
        }
        StoppageKind::ThrowIn => distance < THROW_IN_DISTANCE,
        StoppageKind::Injury => distance < DROP_BALL_DISTANCE,
        StoppageKind::GoalKick | StoppageKind::Penalty => {
            pitch.in_penalty_area(p.pos, end_of(dead.spot))
        }
        _ => false,
    };
    if moved { Job::Return } else { Job::Wait }
}

/// Restarts, version 1: the Laws' takers, distances, and visible steps above, the drawn extra
/// of the wait table (or the tuned delays without one), quick restarts, and a longer wait for
/// a leading team late and for one that wastes time.
pub struct RestartsV1;

impl RestartsModule for RestartsV1 {
    fn taker(&self, view: &MatchView<'_>, kind: StoppageKind, team: usize, spot: DVec2) -> usize {
        taker(kind, team, spot, view.players(), view.teams())
    }

    fn extra_ticks(&self, view: &MatchView<'_>, kind: StoppageKind, team: usize, u: f64) -> u32 {
        extra_ticks(view, kind, team, u)
    }

    fn quick_chance(&self, view: &MatchView<'_>, dead: &DeadBall) -> f64 {
        quick_chance(view, dead)
    }

    fn job(&self, view: &MatchView<'_>, dead: &DeadBall, i: usize) -> Job {
        job(dead, i, view.players(), view.teams())
    }

    fn delay_ticks(&self, view: &MatchView<'_>, kind: StoppageKind, team: usize) -> u32 {
        let delay = delay_ticks(kind, view.tuning());
        let goals = view.goals();
        if goals[team] > goals[1 - team] {
            // A leading team with time wasting on takes longer over its restarts.
            let factor = view.teams()[team].plan.time_wasting;
            // The factor is at most 20 and the delay under 3000 ticks, so the cast fits.
            (f64::from(delay) * factor).round() as u32
        } else {
            delay
        }
    }

    fn shootout_delay_ticks(&self, view: &MatchView<'_>) -> u32 {
        delay_ticks(StoppageKind::Penalty, view.tuning())
    }

    fn target(&self, view: &MatchView<'_>, dead: &DeadBall, i: usize) -> DVec2 {
        match view.referee().shootout.as_ref() {
            Some(shootout) => shootout_target(view.pitch(), dead, i, shootout.keepers),
            None => {
                let place = target(dead, i, view.players(), view.teams(), view.tuning());
                route(dead, i, view.players(), view.teams(), place)
            }
        }
    }

    fn steps_done(&self, view: &MatchView<'_>, dead: &DeadBall, now: u32) -> bool {
        match view.referee().shootout.as_ref() {
            Some(shootout) => shootout_ready(
                view.pitch(),
                dead,
                now,
                view.players(),
                shootout.keepers,
                view.tuning(),
            ),
            None => steps_done(dead, view.players(), view.teams(), view.tuning()),
        }
    }

    fn kick_off_position(
        &self,
        view: &MatchView<'_>,
        team: usize,
        slot: usize,
        kicking: usize,
    ) -> DVec2 {
        kick_off_spot(&view.teams()[team], slot, team != kicking)
    }
}

pub const RESTARTS_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Runs restarts and dead balls: who takes each restart, how long it waits (its visible steps plus a drawn extra from the restart wait table, during which the ball waits where it left play), quick restarts, each player's job and restart target while the ball is dead, when the visible steps are done, and the kick-off positions.",
    inputs: "The dead ball, the players' positions, status and decisions-and-flair share, both teams' shapes, attack directions, mentality minimums and time-wasting plans, the score and the clock, the shoot-out keepers, the restart wait table, and the engine tuning.",
    outputs: "The taker, the drawn extra in ticks (or the fixed delay without a wait table), the quick-restart chance, each player's job and restart target, whether the visible steps are done, and each kick-off position; the loop draws and applies them.",
    tuning: &[
        "restart_delay_s",
        "dt",
        "restart_ready_radius",
        "plan.time_wasting",
        "plan.rest_defence",
        "pace.late_minutes",
        "restart_waits",
    ],
    calibration: "none: the drawn extras come from real restart waits minus the engine's measured visible steps (engine-cli restart-waits); placement and readiness follow the Laws",
    keys: &[Action::RestartWait, Action::QuickRestart],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_config;

    fn dead(kind: StoppageKind, team: usize, spot: DVec2, taker: usize) -> DeadBall {
        DeadBall::fixed(kind, team, spot, true, 0, 0, taker)
    }

    #[test]
    fn opponents_keep_nine_metres_at_a_free_kick() {
        let config = shipped_config(1, 90).unwrap();
        let t = &config.tuning;
        let spot = DVec2::new(20.0, 5.0);
        let taker = taker(
            StoppageKind::FreeKick,
            0,
            spot,
            &config.players,
            &config.teams,
        );
        let d = dead(StoppageKind::FreeKick, 0, spot, taker);
        for i in 11..22 {
            let at = target(&d, i, &config.players, &config.teams, t);
            assert!((at - spot).length() >= KICK_DISTANCE, "player {i} at {at}");
        }
    }

    #[test]
    fn opponents_leave_the_area_at_a_goal_kick() {
        let config = shipped_config(1, 90).unwrap();
        let spot = Pitch::DEFAULT.goal_kick_spot(-1.0, 3.0);
        let taker = taker(
            StoppageKind::GoalKick,
            0,
            spot,
            &config.players,
            &config.teams,
        );
        assert_eq!(taker, 0, "the goalkeeper takes a goal kick");
        let d = dead(StoppageKind::GoalKick, 0, spot, taker);
        for i in 11..22 {
            let at = target(&d, i, &config.players, &config.teams, &config.tuning);
            assert!(
                !Pitch::DEFAULT.in_penalty_area(at, -1.0),
                "player {i} at {at}"
            );
        }
    }

    #[test]
    fn only_the_taker_and_the_goalkeeper_stay_in_the_area_at_a_penalty() {
        let config = shipped_config(1, 90).unwrap();
        let spot = Pitch::DEFAULT.penalty_spot(1.0);
        let taker = taker(
            StoppageKind::Penalty,
            0,
            spot,
            &config.players,
            &config.teams,
        );
        let d = dead(StoppageKind::Penalty, 0, spot, taker);
        for i in 0..22 {
            let at = target(&d, i, &config.players, &config.teams, &config.tuning);
            if i == taker {
                assert!((at - spot).length() < 1e-9);
            } else if i == 11 {
                assert!(at.x >= 52.0 && at.y == 0.0, "keeper at {at}");
            } else {
                assert!(
                    !Pitch::DEFAULT.in_penalty_area(at, 1.0),
                    "player {i} at {at}"
                );
                assert!((at - spot).length() >= KICK_DISTANCE);
            }
        }
    }

    /// A place beside the area and nearer the goal line than the mark moves behind the mark and
    /// stays outside the area, on a 120 by 90 ground (a seed-18 match once faulted there); a
    /// place already behind the mark keeps its spot.
    #[test]
    fn a_place_beside_the_area_moves_behind_the_mark_at_a_penalty() {
        let pitch = Pitch::new(120.0, 90.0).unwrap();
        let spot = pitch.penalty_spot(-1.0);
        let at = penalty_place(&pitch, DVec2::new(-50.7, -23.67), spot, -1.0);
        assert!(behind_the_mark(&pitch, at, -1.0), "{at}");
        assert!(!pitch.in_penalty_area(at, -1.0), "{at}");
        assert!((at - spot).length() >= KICK_DISTANCE, "{at}");
        let kept = DVec2::new(-40.0, -30.0);
        assert_eq!(penalty_place(&pitch, kept, spot, -1.0), kept);
    }

    /// The kick-off places of the 4-4-2 and the 4-3-3 for both teams, in metres from the home
    /// goal line and across: the same table the viewer's kick-off test pins
    /// (`viewer/tests/prematch.test.js`), so the two drawings agree. The away team mirrors
    /// the length and the width.
    #[test]
    fn kick_off_places_mirror_for_the_away_team_and_match_the_viewer_table() {
        type Places = [(f64, f64); PLAYERS_PER_TEAM];
        let table: [(&str, Places, Places); 2] = [
            (
                "4-4-2",
                [
                    (5.0, 0.0),
                    (25.0, -22.0),
                    (25.0, -8.0),
                    (25.0, 8.0),
                    (25.0, 22.0),
                    (45.0, -24.0),
                    (45.0, -8.0),
                    (45.0, 8.0),
                    (45.0, 24.0),
                    (51.5, -8.0),
                    (51.5, 8.0),
                ],
                [
                    (100.0, 0.0),
                    (80.0, 22.0),
                    (80.0, 8.0),
                    (80.0, -8.0),
                    (80.0, -22.0),
                    (60.0, 24.0),
                    (60.0, 8.0),
                    (60.0, -8.0),
                    (60.0, -24.0),
                    (53.5, 8.0),
                    (53.5, -8.0),
                ],
            ),
            (
                "4-3-3",
                [
                    (5.0, 0.0),
                    (25.0, -22.0),
                    (25.0, -8.0),
                    (25.0, 8.0),
                    (25.0, 22.0),
                    (43.0, -15.0),
                    (38.0, 0.0),
                    (43.0, 15.0),
                    (51.5, -22.0),
                    (51.5, 22.0),
                    (51.5, 0.0),
                ],
                [
                    (100.0, 0.0),
                    (80.0, 22.0),
                    (80.0, 8.0),
                    (80.0, -8.0),
                    (80.0, -22.0),
                    (62.0, 15.0),
                    (67.0, 0.0),
                    (62.0, -15.0),
                    (53.5, 22.0),
                    (53.5, -22.0),
                    (53.5, 0.0),
                ],
            ),
        ];
        let content = crate::data::test_support::shipped_content();
        let schema = &content.tactics;
        for (name, home, away) in table {
            for (index, places) in [(0, home), (1, away)] {
                let mut team = crate::team::test_support::bare(index);
                let mut tactics = crate::tactics::Tactics::defaults(schema);
                tactics.set_formation(schema.formation_index(name).unwrap() as u8, schema);
                team.set_tactics(tactics, schema, &content.tuning.engine);
                // The viewer draws the formation file's slots; the engine also closes a back
                // line wider than its keeper's organisation allows, which the viewer cannot
                // know before the match. The shared table pins the mirror rule, so the back
                // line keeps its file places here.
                team.plan.back_line_gap = f64::INFINITY;
                team.relayout();
                for (slot, (x, y)) in places.into_iter().enumerate() {
                    let at = kick_off_position(&team, slot);
                    let want = DVec2::new(x - 52.5, y);
                    assert!(
                        (at - want).length() < 1e-9,
                        "{name} team {index} slot {slot}: {at} against {want}"
                    );
                }
            }
        }
    }

    #[test]
    fn everyone_starts_a_kick_off_in_their_own_half() {
        let config = shipped_config(1, 90).unwrap();
        for team in &config.teams {
            for slot in 0..PLAYERS_PER_TEAM {
                assert!(kick_off_position(team, slot).x * team.attack_x < 0.0);
            }
        }
    }

    /// Law 8: at a kick-off the team that does not kick off stands in its own half and
    /// outside the centre circle, on the default ground and on the smallest ground the Laws
    /// allow.
    #[test]
    fn the_other_team_stands_outside_the_centre_circle_at_a_kick_off() {
        for (length, width) in [(105.0, 68.0), (90.0, 45.0)] {
            let mut config = shipped_config(1, 90).unwrap();
            let pitch = Pitch::new(length, width).unwrap();
            config.set_pitch(pitch);
            for team in &config.teams {
                for slot in 0..PLAYERS_PER_TEAM {
                    let own = kick_off_spot(team, slot, false);
                    assert_eq!(own, kick_off_position(team, slot));
                    let other = kick_off_spot(team, slot, true);
                    assert!(
                        other.x * team.attack_x < 0.0,
                        "{length} by {width}: slot {slot} at {other}"
                    );
                    assert!(
                        other.length() >= KICK_DISTANCE + TARGET_MARGIN - 1e-9,
                        "{length} by {width}: slot {slot} at {other}"
                    );
                    assert!(pitch.contains(other));
                }
            }
        }
    }

    /// Law 8 with every shipped formation: the other team's kick-off places stay outside the
    /// centre circle and in its own half, and no two of them are within the separation radius,
    /// so no push keeps a player inside the circle. Forwards brought back from the other half
    /// once met on one place on the edge, where the push held one of them 8.9 m from the ball.
    #[test]
    fn the_other_teams_kick_off_places_never_share_the_edge_of_the_circle() {
        let content = crate::data::test_support::shipped_content();
        let schema = &content.tactics;
        let apart = crate::tuning::Tuning::default().separation_radius;
        for formation in &schema.formations {
            for index in [0, 1] {
                let mut team = crate::team::test_support::bare(index);
                let mut tactics = crate::tactics::Tactics::defaults(schema);
                tactics.set_formation(
                    schema.formation_index(&formation.name).unwrap() as u8,
                    schema,
                );
                team.set_tactics(tactics, schema, &content.tuning.engine);
                team.relayout();
                let places: Vec<DVec2> = (0..PLAYERS_PER_TEAM)
                    .map(|slot| kick_off_spot(&team, slot, true))
                    .collect();
                for (i, a) in places.iter().enumerate() {
                    assert!(a.length() >= KICK_DISTANCE + TARGET_MARGIN - 1e-9);
                    assert!(
                        a.x * team.attack_x < 0.0,
                        "{} slot {i} at {a}",
                        formation.name
                    );
                    for (j, b) in places.iter().enumerate().skip(i + 1) {
                        assert!(
                            (*a - *b).length() >= apart,
                            "{} team {index}: slots {i} and {j} at {a} and {b}",
                            formation.name
                        );
                    }
                }
            }
        }
    }

    /// Walks player `i` toward his route, `step` metres a tick, until he reaches `place`;
    /// returns his nearest approach to the spot.
    fn walk(
        d: &DeadBall,
        i: usize,
        players: &mut [Player],
        teams: &[Team; 2],
        place: DVec2,
    ) -> f64 {
        let mut nearest = f64::MAX;
        for _ in 0..2_000 {
            let to = route(d, i, players, teams, place);
            let p = &mut players[i];
            p.pos += math::clamp_len(to - p.pos, 0.2);
            nearest = nearest.min((p.pos - d.spot).length());
            if (p.pos - place).length() < 1e-6 {
                return nearest;
            }
        }
        panic!("player {i} never reached {place}");
    }

    /// The seed-4 throw-in fault: an opponent whose place lies across the spot walks round the
    /// 2 m circle, never through the taker, and still reaches his place; a team-mate of the
    /// taker, whom the law keeps nowhere, walks straight.
    #[test]
    fn an_opponent_walks_round_the_throw_in_spot_to_his_place() {
        let config = shipped_config(1, 90).unwrap();
        let mut players = config.players.clone();
        let spot = DVec2::new(10.0, 34.0);
        let d = dead(StoppageKind::ThrowIn, 0, spot, 5);
        let place = DVec2::new(14.0, 32.5);
        players[13].pos = DVec2::new(6.0, 33.6);
        let nearest = walk(&d, 13, &mut players, &config.teams, place);
        assert!(nearest >= THROW_IN_DISTANCE, "came within {nearest} m");
        // Straight along the line he would have passed 0.76 m from the spot or nearer.
        assert!(math::segment_distance(spot, DVec2::new(6.0, 33.6), place) < 1.0);
        players[3].pos = DVec2::new(6.0, 33.6);
        assert_eq!(route(&d, 3, &players, &config.teams, place), place);
    }

    /// Every kind with a distance: an opponent (everyone at a dropped ball) walking from one
    /// side of the spot to the other keeps the distance the law asks, from a start outside the
    /// circle and from a start inside it; a path that keeps the distance is left straight.
    #[test]
    fn players_the_law_keeps_away_walk_round_every_restart_spot() {
        let config = shipped_config(1, 90).unwrap();
        let spot = DVec2::new(5.0, -3.0);
        for (kind, distance) in [
            (StoppageKind::ThrowIn, THROW_IN_DISTANCE),
            (StoppageKind::FreeKick, KICK_DISTANCE),
            (StoppageKind::Corner, KICK_DISTANCE),
            (StoppageKind::KickOff, KICK_DISTANCE),
            (StoppageKind::Injury, DROP_BALL_DISTANCE),
            (StoppageKind::Penalty, KICK_DISTANCE),
        ] {
            let d = dead(kind, 0, spot, 5);
            let mut players = config.players.clone();
            let place = spot + DVec2::new(distance + 3.0, 0.5);
            players[13].pos = spot - DVec2::new(distance + 3.0, 0.0);
            let nearest = walk(&d, 13, &mut players, &config.teams, place);
            assert!(
                nearest >= distance - 1e-9,
                "{kind:?}: came within {nearest} m"
            );
            players[13].pos = spot - DVec2::new(0.5 * distance, 0.2);
            let start = (players[13].pos - spot).length();
            let nearest = walk(&d, 13, &mut players, &config.teams, place);
            assert!(
                nearest >= start - 1e-9,
                "{kind:?}: from inside, came within {nearest} m"
            );
            players[13].pos = spot + DVec2::new(0.0, distance + 3.0);
            let wide = spot + DVec2::new(distance + 3.0, distance + 3.0);
            assert_eq!(
                route(&d, 13, &players, &config.teams, wide),
                wide,
                "{kind:?}"
            );
        }
    }

    #[test]
    fn everyone_keeps_four_metres_from_a_dropped_ball() {
        let config = shipped_config(1, 90).unwrap();
        let spot = DVec2::new(5.0, -3.0);
        let taker = taker(
            StoppageKind::Injury,
            1,
            spot,
            &config.players,
            &config.teams,
        );
        assert_eq!(config.players[taker].team, 1);
        assert_ne!(
            config.players[taker].slot, 0,
            "an outfield player takes it in midfield"
        );
        let d = dead(StoppageKind::Injury, 1, spot, taker);
        for i in 0..22 {
            let at = target(&d, i, &config.players, &config.teams, &config.tuning);
            if i == taker {
                assert!((at - spot).length() < 1e-9);
            } else {
                assert!(
                    (at - spot).length() >= DROP_BALL_DISTANCE,
                    "player {i} at {at}"
                );
            }
        }
        assert_eq!(delay_ticks(StoppageKind::Injury, &config.tuning), 1000);
    }

    #[test]
    fn a_dropped_ball_in_the_own_penalty_area_goes_to_the_goalkeeper() {
        let config = shipped_config(1, 90).unwrap();
        // The home team defends the negative end at kick-off.
        let spot = DVec2::new(-45.0, 4.0);
        assert_eq!(
            taker(
                StoppageKind::Injury,
                0,
                spot,
                &config.players,
                &config.teams
            ),
            0
        );
        assert_ne!(
            taker(
                StoppageKind::Injury,
                1,
                spot,
                &config.players,
                &config.teams
            ),
            11
        );
    }

    /// AC-20 and charter step 6: for one draw, a team leading in the late window waits the
    /// late-lead factor longer than in a level game, time wasting multiplies the same part, and
    /// neither touches the visible steps; a trailing side and a side leading early wait as in a
    /// level game.
    #[test]
    fn a_team_leading_late_waits_longer_and_time_wasting_lengthens_the_same_part() {
        use crate::scenario::Scene;
        let mut sim = Scene::new(shipped_config(1, 90).unwrap())
            .at_minute(80)
            .build();
        let factor = sim.config.restart_waits.as_ref().unwrap().late_lead_factor;
        assert_eq!(factor, 1.7);
        let kind = StoppageKind::GoalKick;
        let level = extra_ticks(&sim.view(), kind, 0, 0.5);
        assert!(level > 100, "{level}");
        sim.summary.goals = [1, 0];
        let late = extra_ticks(&sim.view(), kind, 0, 0.5);
        assert!(
            (f64::from(late) - f64::from(level) * factor).abs() <= 1.0,
            "{late} against {level}"
        );
        assert_eq!(
            extra_ticks(&sim.view(), kind, 1, 0.5),
            level,
            "the trailing side"
        );
        let spot = Pitch::DEFAULT.goal_kick_spot(-sim.teams[0].attack_x, 3.0);
        let dead = DeadBall::fixed(kind, 0, spot, false, sim.tick, 0, 0);
        let steps = steps_done(&dead, &sim.players, &sim.teams, &sim.config.tuning);
        sim.teams[0].plan.time_wasting = 1.5;
        let wasting = extra_ticks(&sim.view(), kind, 0, 0.5);
        assert!(
            (f64::from(wasting) - f64::from(level) * factor * 1.5).abs() <= 1.0,
            "{wasting} against {level}"
        );
        assert_eq!(
            steps_done(&dead, &sim.players, &sim.teams, &sim.config.tuning),
            steps,
            "time wasting leaves the visible steps alone"
        );
        let early = Scene::new(shipped_config(1, 90).unwrap())
            .at_minute(30)
            .score([1, 0])
            .build();
        assert_eq!(
            extra_ticks(&early.view(), kind, 0, 0.5),
            level,
            "leading early"
        );
    }

    /// The match with the restarting side's taker on a throw-in spot, one team-mate free 10 m
    /// away, and every opposing outfield player 30 m the wrong side of the ball: out of shape.
    fn quick_scene(goals: [u32; 2]) -> (crate::sim::Simulation, DeadBall) {
        let mut sim = crate::sim::Simulation::new(shipped_config(1, 90).unwrap()).unwrap();
        sim.summary.goals = goals;
        let attack = sim.teams[0].attack_x;
        let spot = DVec2::new(0.0, 34.0);
        let keeper = sim.keeper(1);
        for i in 11..22 {
            if i != keeper {
                sim.players[i].pos = DVec2::new(-30.0 * attack, (i as f64 - 16.0) * 4.0);
            }
        }
        let taker = 7;
        let own_keeper = sim.keeper(0);
        for i in 0..11 {
            if i != own_keeper && i != taker && i != 8 {
                sim.players[i].pos = DVec2::new(-20.0 * attack + i as f64, -30.0);
            }
        }
        sim.players[taker].pos = spot;
        sim.players[8].pos = spot + DVec2::new(5.0 * attack, -9.0);
        (
            sim,
            DeadBall::fixed(StoppageKind::ThrowIn, 0, spot, false, 0, 0, taker),
        )
    }

    /// The number of `n` evenly spread draws below `chance`.
    fn quick_restarts(chance: f64, n: u32) -> u32 {
        (0..n)
            .filter(|k| (f64::from(*k) + 0.5) / f64::from(n) < chance)
            .count() as u32
    }

    /// AC-21: with a free team-mate and the opponents out of shape, a trailing side restarts
    /// quickly more often than a level one; with time wasting on, never; and with no free
    /// team-mate, never (the control).
    #[test]
    fn a_trailing_side_restarts_quickly_more_often_and_a_time_waster_never_does() {
        let (level_sim, dead) = quick_scene([0, 0]);
        let level = quick_chance(&level_sim.view(), &dead);
        let (trailing_sim, _) = quick_scene([0, 1]);
        let trailing = quick_chance(&trailing_sim.view(), &dead);
        assert!(level > 0.0, "{level}");
        assert!(
            quick_restarts(trailing, 2000) > quick_restarts(level, 2000),
            "{trailing} against {level}"
        );
        for goals in [[0, 0], [0, 1], [1, 0]] {
            let (mut sim, dead) = quick_scene(goals);
            sim.teams[0].plan.time_wasting = 1.5;
            assert_eq!(quick_chance(&sim.view(), &dead), 0.0, "{goals:?}");
        }
        let (mut marked, dead) = quick_scene([0, 1]);
        marked.players[16].pos = marked.players[8].pos + DVec2::new(1.0, 0.0);
        assert_eq!(
            quick_chance(&marked.view(), &dead),
            0.0,
            "no team-mate is free"
        );
        let (mut shaped, dead) = quick_scene([0, 1]);
        let attack = shaped.teams[0].attack_x;
        for i in 12..22 {
            shaped.players[i].pos.x = 20.0 * attack;
        }
        assert_eq!(
            quick_chance(&shaped.view(), &dead),
            0.0,
            "the opponents are in shape"
        );
    }

    #[test]
    fn the_delay_comes_from_the_tuning() {
        let t = Tuning::default();
        assert_eq!(delay_ticks(StoppageKind::ThrowIn, &t), 150);
        assert_eq!(delay_ticks(StoppageKind::Penalty, &t), 750);
        // Without a wait table the hard limit is three times the delay.
        let d = DeadBall::fixed(StoppageKind::ThrowIn, 0, DVec2::ZERO, true, 100, 150, 0);
        assert_eq!(d.ready_at, 250);
        assert_eq!(d.hard_limit(), 550);
        assert_eq!((d.appear_at, d.extra, d.exit), (100, 0, DVec2::ZERO));
    }
}

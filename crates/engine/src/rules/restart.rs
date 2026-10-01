//! Restarts (IFAB Laws 8 and 13 to 17): who takes each restart, where every other player
//! stands while the ball is dead, and when the restart may be taken. A dropped ball after an
//! injury goes to one player of the team that last touched the ball, and every other player
//! keeps 4 m away (IFAB Law 8).
//!
//! While the ball is dead, every player steers to a restart target: the formation anchor
//! around the restart spot, moved where a law requires it. Opponents keep 9.15 m from the ball
//! at a free kick, a corner, and a kick-off, stay outside the penalty area at a goal kick, and
//! at a penalty everyone but the taker and the defending goalkeeper waits outside the area
//! and 9.15 m from the mark. The taker may restart once the tuned delay has passed, the taker
//! is at the ball, and every player the law moves has moved.

use crate::data::rules::StoppageKind;
use crate::math::{self, DVec2, toward};
use crate::modules::{MatchView, ModuleCard, RestartsModule};
use crate::pitch::{KICK_DISTANCE, PENALTY_AREA_DEPTH, PENALTY_AREA_WIDTH, Pitch};
use crate::player::Player;
use crate::team::{PLAYERS_PER_TEAM, Team};
use crate::tuning::Tuning;

/// Distance opponents keep from a throw-in (IFAB Law 15).
const THROW_IN_DISTANCE: f64 = 2.0;
/// Distance every other player keeps from a dropped ball (IFAB Law 8).
const DROP_BALL_DISTANCE: f64 = 4.0;
/// Slack between where a player is sent and where the law is judged, in metres.
const TARGET_MARGIN: f64 = 0.5;
const JUDGE_MARGIN: f64 = 0.25;

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
    /// The first tick the restart may be taken.
    pub ready_at: u32,
    /// The roster index of the player who takes the restart.
    pub taker: usize,
}

impl DeadBall {
    /// The tick by which the restart is taken whatever the players are doing: three times
    /// the delay, and at least one tick after the ball went dead.
    pub fn hard_limit(&self) -> u32 {
        self.since + (3 * (self.ready_at - self.since)).max(1)
    }
}

/// Ticks a restart of `kind` waits before it may be taken.
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
        StoppageKind::Injury if teams[team].pitch.in_penalty_area(spot, own_end) => Some(keeper),
        _ => None,
    }
}

/// Where a player of `team` stands for a kick-off: the formation slot, kept inside its own
/// half.
pub fn kick_off_position(team: &Team, slot: usize) -> DVec2 {
    let base = team.slot_base(slot);
    let depth = base.x * team.attack_x;
    DVec2::new(if depth > -1.0 { -team.attack_x } else { base.x }, base.y)
}

/// Where the taker stands: at the ball, or just behind it at a kick-off.
fn taker_target(dead: &DeadBall, teams: &[Team; 2]) -> DVec2 {
    if dead.kind == StoppageKind::KickOff {
        dead.spot - DVec2::new(0.5 * teams[dead.team].attack_x, 0.0)
    } else {
        teams[dead.team].pitch.clamp(dead.spot, 0.2)
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
    let pitch = &team.pitch;
    let opponent = p.team != dead.team;
    // An opponent pushed off a spot backs toward its own goal.
    let away = DVec2::new(-team.attack_x, 0.0);
    let base = match dead.kind {
        StoppageKind::KickOff => kick_off_position(team, p.slot),
        _ => team.anchor(p.slot, dead.spot, t),
    };
    let keep = KICK_DISTANCE + TARGET_MARGIN;
    let at = match dead.kind {
        StoppageKind::KickOff | StoppageKind::FreeKick | StoppageKind::Corner if opponent => {
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
        StoppageKind::GoalKick if opponent => out_of_area(pitch, base, end_of(dead.spot)),
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
                let behind = DVec2::new(-side, 0.0);
                outside_on_pitch(
                    pitch,
                    out_of_area(pitch, base, side),
                    dead.spot,
                    keep,
                    behind,
                    0.5,
                )
            }
        }
        _ => base,
    };
    pitch.clamp(at, 0.5)
}

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
        if i == dead.taker || i == keepers[1 - dead.team] {
            let at = shootout_target(pitch, dead, i, keepers);
            return (p.pos - at).length() <= t.restart_ready_radius;
        }
        !pitch.in_penalty_area(p.pos, side)
            && (p.pos - dead.spot).length() >= KICK_DISTANCE - JUDGE_MARGIN
    })
}

/// `true` when the restart may be taken at `tick`: the delay has passed, the taker is at the
/// ball, and every player the law moves has moved.
pub fn is_ready(
    dead: &DeadBall,
    tick: u32,
    players: &[Player],
    teams: &[Team; 2],
    t: &Tuning,
) -> bool {
    if tick < dead.ready_at {
        return false;
    }
    let taker = &players[dead.taker];
    if (taker.pos - taker_target(dead, teams)).length() > t.restart_ready_radius {
        return false;
    }
    let side = end_of(dead.spot);
    let pitch = &teams[0].pitch;
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
                (opponent && teams[p.team].keeper_slot() == p.slot)
                    || (!pitch.in_penalty_area(p.pos, side)
                        && distance >= KICK_DISTANCE - JUDGE_MARGIN)
            }
            _ => true,
        }
    })
}

/// Restarts, version 1: the Laws' takers, distances, and readiness above, the tuned delays,
/// and a longer wait for a leading team that wastes time.
pub struct RestartsV1;

impl RestartsModule for RestartsV1 {
    fn taker(&self, view: &MatchView<'_>, kind: StoppageKind, team: usize, spot: DVec2) -> usize {
        taker(kind, team, spot, view.players(), view.teams())
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
            None => target(dead, i, view.players(), view.teams(), view.tuning()),
        }
    }

    fn ready(&self, view: &MatchView<'_>, dead: &DeadBall, now: u32) -> bool {
        match view.referee().shootout.as_ref() {
            Some(shootout) => shootout_ready(
                view.pitch(),
                dead,
                now,
                view.players(),
                shootout.keepers,
                view.tuning(),
            ),
            None => is_ready(dead, now, view.players(), view.teams(), view.tuning()),
        }
    }

    fn kick_off_position(&self, view: &MatchView<'_>, team: usize, slot: usize) -> DVec2 {
        kick_off_position(&view.teams()[team], slot)
    }
}

pub const RESTARTS_V1_CARD: ModuleCard = ModuleCard {
    purpose: "Runs restarts and dead balls: who takes each restart, how long it waits, where every player stands while the ball is dead, when it may be taken, and the kick-off positions.",
    inputs: "The dead ball, the players' positions and status, both teams' shapes, attack directions, and time-wasting plans, the score, the shoot-out keepers, and the engine tuning.",
    outputs: "The taker, the delay in ticks, each player's restart target, whether the restart is ready, and each kick-off position; the loop applies them.",
    tuning: &[
        "restart_delay_s",
        "dt",
        "restart_ready_radius",
        "plan.time_wasting",
    ],
    calibration: "none: restart placement and timing follow the Laws; no band measures them",
    keys: &[],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_config;

    fn dead(kind: StoppageKind, team: usize, spot: DVec2, taker: usize) -> DeadBall {
        DeadBall {
            kind,
            team,
            spot,
            direct: true,
            since: 0,
            ready_at: 0,
            taker,
        }
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

    #[test]
    fn everyone_starts_a_kick_off_in_their_own_half() {
        let config = shipped_config(1, 90).unwrap();
        for team in &config.teams {
            for slot in 0..PLAYERS_PER_TEAM {
                assert!(kick_off_position(team, slot).x * team.attack_x < 0.0);
            }
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

    #[test]
    fn the_delay_comes_from_the_tuning() {
        let t = Tuning::default();
        assert_eq!(delay_ticks(StoppageKind::ThrowIn, &t), 150);
        assert_eq!(delay_ticks(StoppageKind::Penalty, &t), 750);
        let d = DeadBall {
            since: 100,
            ready_at: 250,
            ..dead(StoppageKind::ThrowIn, 0, DVec2::ZERO, 0)
        };
        assert_eq!(d.hard_limit(), 550);
    }
}

//! The restart position check (IFAB Laws 8, 10 and 13 to 17): at the moment a restart is
//! taken, the ball is at its spot, every player on the pitch stands on this ground and every
//! player off it at its parking spot, and every player stands where the restart's Law asks.
//!
//! The check reads positions and changes nothing. It is independent of the restarts module
//! that placed the players, so a swapped or faulty module is judged by the Laws, not by its
//! own readiness test. Distances are judged with the referee's margin that readiness uses
//! (`JUDGE_MARGIN`), and a player who must be at a spot or on a line with the ready radius.

use std::fmt;

use crate::data::rules::StoppageKind;
use crate::math::DVec2;
use crate::pitch::{GOAL_WIDTH, KICK_DISTANCE, PENALTY_MARK, Pitch};
use crate::player::Player;
use crate::team::Team;

use super::DeadBall;
use super::restart::{DROP_BALL_DISTANCE, JUDGE_MARGIN, THROW_IN_DISTANCE};

/// The restart being taken.
#[derive(Debug, Clone, Copy)]
pub enum Restart {
    /// A kick-off that starts a period: `team` kicks off and `taker` takes it.
    KickOff { team: usize, taker: usize },
    /// A dead ball's restart, a kick-off after a goal included.
    Dead(DeadBall),
    /// A kick of the penalty shoot-out; `keepers` are each team's keeper in the shoot-out.
    ShootoutKick { dead: DeadBall, keepers: [usize; 2] },
}

impl Restart {
    /// The restart in words.
    pub fn name(&self) -> String {
        match self {
            Restart::KickOff { .. } => "kick-off".into(),
            Restart::Dead(dead) => dead.kind.code().replace('_', " "),
            Restart::ShootoutKick { .. } => "shoot-out kick".into(),
        }
    }
}

/// One position against the Laws.
#[derive(Debug, Clone, PartialEq)]
pub struct Fault {
    /// The player, as roster index, team and formation slot; `None` for the ball.
    pub player: Option<(usize, usize, usize)>,
    pub at: DVec2,
    /// The Law and what it asks, with the measured value.
    pub rule: String,
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.player {
            Some((i, team, slot)) => write!(f, "player {i} (team {team}, slot {slot})")?,
            None => f.write_str("the ball")?,
        }
        write!(f, " at ({:.2}, {:.2}): {}", self.at.x, self.at.y, self.rule)
    }
}

/// `true` when `p` stands on the goal line at the `side` end between the posts, within
/// `radius` of the line.
pub fn on_goal_line(pitch: &Pitch, p: DVec2, side: f64, radius: f64) -> bool {
    (pitch.half_length() - p.x * side).abs() <= radius && p.y.abs() <= GOAL_WIDTH / 2.0
}

/// `true` when `p` stands behind the penalty mark at the `side` end: no nearer the goal line
/// than the mark (Law 14).
pub fn behind_the_mark(pitch: &Pitch, p: DVec2, side: f64) -> bool {
    p.x * side <= pitch.half_length() - PENALTY_MARK + JUDGE_MARGIN
}

/// `true` when `p` stands inside the centre circle (Law 10).
pub fn in_centre_circle(p: DVec2) -> bool {
    p.length() <= KICK_DISTANCE + JUDGE_MARGIN
}

/// The side of the pitch (the sign of `x`) of the goal nearest `spot`.
fn end_of(spot: DVec2) -> f64 {
    if spot.x < 0.0 { -1.0 } else { 1.0 }
}

/// Every position against the Laws as `restart` is taken on `pitch` with the ball at `ball`.
/// `ready_radius` is how near a spot or a line a player must stand to be on it.
pub fn check(
    pitch: &Pitch,
    restart: &Restart,
    ball: DVec2,
    players: &[Player],
    teams: &[Team; 2],
    ready_radius: f64,
) -> Vec<Fault> {
    let mut faults = Vec::new();
    let (spot, team, taker) = match restart {
        Restart::KickOff { team, taker } => (DVec2::ZERO, *team, *taker),
        Restart::Dead(dead) | Restart::ShootoutKick { dead, .. } => {
            (dead.spot, dead.team, dead.taker)
        }
    };
    if (ball - spot).length() > 1e-6 {
        faults.push(Fault {
            player: None,
            at: ball,
            rule: format!(
                "the ball is {:.2} m from the restart spot ({:.2}, {:.2})",
                (ball - spot).length(),
                spot.x,
                spot.y
            ),
        });
    }
    let side = end_of(spot);
    for (i, p) in players.iter().enumerate() {
        let mut fault = |rule: String| {
            faults.push(Fault {
                player: Some((i, p.team, p.slot)),
                at: p.pos,
                rule,
            });
        };
        if !p.active() {
            if !pitch.is_parking_spot(p.pos) {
                fault("Law 3: a player off the pitch stands away from his parking spot".into());
            }
            continue;
        }
        if !pitch.contains(p.pos) {
            fault(format!(
                "Law 1: off the {} by {} ground",
                pitch.length(),
                pitch.width()
            ));
            continue;
        }
        let opponent = p.team != team;
        let distance = (p.pos - spot).length();
        let near = |law: &str, what: &str, keep: f64| {
            format!("{law}: {what} {distance:.2} m from the ball, {keep} m asked")
        };
        let in_area = pitch.in_penalty_area(p.pos, side);
        match restart {
            Restart::ShootoutKick { keepers, .. } => {
                if i == taker {
                    if distance > ready_radius {
                        fault(format!("Law 10: the kicker {distance:.2} m from the mark"));
                    }
                } else if i == keepers[1 - team] {
                    if !on_goal_line(pitch, p.pos, side, ready_radius) {
                        fault("Law 10: the defending goalkeeper is not on his goal line between the posts".into());
                    }
                } else if i == keepers[team] {
                    if in_area {
                        fault(
                            "Law 10: the kicking team's goalkeeper is inside the penalty area"
                                .into(),
                        );
                    }
                } else if !in_centre_circle(p.pos) {
                    fault(format!(
                        "Law 10: {:.2} m from the centre spot, outside the centre circle",
                        p.pos.length()
                    ));
                }
            }
            _ if i == taker => {
                if let Restart::Dead(dead) = restart
                    && dead.kind == StoppageKind::Penalty
                    && distance > ready_radius
                {
                    fault(format!("Law 14: the taker {distance:.2} m from the mark"));
                }
            }
            Restart::KickOff { .. } => kick_off(p, teams, opponent, distance, &mut fault),
            Restart::Dead(dead) => match dead.kind {
                StoppageKind::KickOff => kick_off(p, teams, opponent, distance, &mut fault),
                StoppageKind::FreeKick if opponent && distance < KICK_DISTANCE - JUDGE_MARGIN => {
                    fault(near("Law 13", "an opponent", KICK_DISTANCE));
                }
                StoppageKind::Corner if opponent && distance < KICK_DISTANCE - JUDGE_MARGIN => {
                    fault(near("Law 17", "an opponent", KICK_DISTANCE));
                }
                StoppageKind::ThrowIn
                    if opponent && distance < THROW_IN_DISTANCE - JUDGE_MARGIN =>
                {
                    fault(near("Law 15", "an opponent", THROW_IN_DISTANCE));
                }
                StoppageKind::Injury if distance < DROP_BALL_DISTANCE - JUDGE_MARGIN => {
                    fault(near(
                        "Law 8",
                        "a player at the dropped ball",
                        DROP_BALL_DISTANCE,
                    ));
                }
                StoppageKind::GoalKick if opponent && in_area => {
                    fault("Law 16: an opponent inside the penalty area".into());
                }
                StoppageKind::Penalty => {
                    if opponent && teams[p.team].keeper_slot() == p.slot {
                        if !on_goal_line(pitch, p.pos, side, ready_radius) {
                            fault("Law 14: the defending goalkeeper is not on his goal line between the posts".into());
                        }
                    } else if in_area {
                        fault("Law 14: inside the penalty area".into());
                    } else if distance < KICK_DISTANCE - JUDGE_MARGIN {
                        fault(near("Law 14", "a player", KICK_DISTANCE));
                    } else if !behind_the_mark(pitch, p.pos, side) {
                        fault("Law 14: nearer the goal line than the penalty mark".into());
                    }
                }
                _ => {}
            },
        }
    }
    faults
}

/// Law 8 at a kick-off: everyone but the taker in his own half, the opponents 9.15 m from the
/// ball.
fn kick_off(
    p: &Player,
    teams: &[Team; 2],
    opponent: bool,
    distance: f64,
    fault: &mut impl FnMut(String),
) {
    if p.pos.x * teams[p.team].attack_x > JUDGE_MARGIN {
        fault("Law 8: in the opponents' half at the kick-off".into());
    } else if opponent && distance < KICK_DISTANCE - JUDGE_MARGIN {
        fault(format!(
            "Law 8: an opponent {distance:.2} m from the ball, {KICK_DISTANCE} m asked"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::test_support::shipped_config;
    use crate::sim::MatchConfig;

    /// A match on a ground of `length` by `width`, every player at a legal spot for a goal
    /// kick at the home end: the home side in its own half outside the area's width, the away
    /// side in the middle of the pitch.
    fn on(length: f64, width: f64) -> MatchConfig {
        let mut config = shipped_config(1, 90).unwrap();
        let pitch = Pitch::new(length, width).unwrap();
        config.pitch = pitch;
        for team in &mut config.teams {
            team.pitch = pitch;
        }
        for (i, p) in config.players.iter_mut().enumerate() {
            let k = (i % 11) as f64;
            p.pos =
                DVec2::new(-1.0 - 2.0 * k, -12.0 + 2.0 * k) * if p.team == 0 { 1.0 } else { -1.0 };
        }
        config
    }

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

    fn faults(config: &MatchConfig, restart: Restart, ball: DVec2) -> Vec<Fault> {
        check(
            &config.pitch,
            &restart,
            ball,
            &config.players,
            &config.teams,
            1.0,
        )
    }

    /// Each rule passes a legal position and fails one illegal position, on two grounds.
    #[test]
    fn each_law_passes_a_legal_position_and_fails_an_illegal_one() {
        for (length, width) in [(105.0, 68.0), (100.0, 64.0)] {
            let mut config = on(length, width);
            let pitch = config.pitch;
            let hl = pitch.half_length();
            let attack0 = config.teams[0].attack_x;
            // Kick-off by the home team: everyone in his own half, the away side 9.15 m away.
            for (i, p) in config.players.iter_mut().enumerate() {
                let x = -config.teams[p.team].attack_x * (12.0 + (i % 11) as f64);
                p.pos = DVec2::new(x, (i % 11) as f64 - 5.0);
            }
            let ko = Restart::KickOff { team: 0, taker: 10 };
            assert_eq!(faults(&config, ko, DVec2::ZERO), [], "{length} by {width}");
            let mut bad = config.clone();
            bad.players[20].pos = DVec2::new(attack0 * 1.0, 8.0);
            let f = faults(&bad, ko, DVec2::ZERO);
            assert_eq!(f.len(), 1, "{f:?}");
            assert!(
                f[0].rule.starts_with("Law 8: an opponent 8.06 m"),
                "{}",
                f[0]
            );
            bad.players[20].pos = DVec2::new(attack0 * 10.0, 20.0);
            bad.players[3].pos = DVec2::new(attack0 * 3.0, 20.0);
            let f = faults(&bad, ko, DVec2::ZERO);
            assert_eq!(f.len(), 1, "{f:?}");
            assert!(f[0].rule.contains("opponents' half"), "{}", f[0]);
            // The ball away from its spot.
            assert_eq!(faults(&config, ko, DVec2::new(0.5, 0.0)).len(), 1);
            // A player off this ground.
            let mut off = config.clone();
            off.players[5].pos = DVec2::new(-10.0, pitch.half_width() + 1.0);
            assert!(faults(&off, ko, DVec2::ZERO)[0].rule.starts_with("Law 1"));

            // A free kick, a corner and a throw-in: an opponent too near.
            let spot = DVec2::new(-attack0 * 20.0, 0.0);
            for (kind, keep) in [
                (StoppageKind::FreeKick, KICK_DISTANCE),
                (StoppageKind::Corner, KICK_DISTANCE),
                (StoppageKind::ThrowIn, THROW_IN_DISTANCE),
                (StoppageKind::Injury, DROP_BALL_DISTANCE),
            ] {
                let mut c = config.clone();
                for p in c.players.iter_mut() {
                    if (p.pos - spot).length() < 12.0 {
                        p.pos += DVec2::new(0.0, 25.0);
                    }
                }
                let d = Restart::Dead(dead(kind, 1, spot, 15));
                c.players[15].pos = spot;
                assert_eq!(faults(&c, d, spot), [], "{kind:?}");
                c.players[2].pos = spot + DVec2::new(0.0, keep - 1.0);
                let f = faults(&c, d, spot);
                assert_eq!(f.len(), 1, "{kind:?}: {f:?}");
            }

            // A goal kick at the home end: an away player inside the area.
            let side = -attack0;
            let spot = pitch.goal_kick_spot(side, 2.0);
            let mut c = config.clone();
            let d = Restart::Dead(dead(StoppageKind::GoalKick, 0, spot, 0));
            c.players[0].pos = spot;
            assert_eq!(faults(&c, d, spot), []);
            c.players[15].pos = DVec2::new(side * (hl - 10.0), 5.0);
            assert!(faults(&c, d, spot)[0].rule.starts_with("Law 16"));

            // A penalty for the away side at the home end.
            let mark = pitch.penalty_spot(side);
            let mut c = config.clone();
            c.players[20].pos = mark;
            c.players[0].pos = DVec2::new(side * (hl - 0.3), 0.0);
            let d = Restart::Dead(dead(StoppageKind::Penalty, 1, mark, 20));
            assert_eq!(faults(&c, d, mark), []);
            let mut k = c.clone();
            k.players[0].pos = DVec2::new(side * (hl - 3.0), 0.0);
            assert!(faults(&k, d, mark)[0].rule.contains("goalkeeper"));
            let mut a = c.clone();
            a.players[3].pos = DVec2::new(side * (hl - 14.0), 10.0);
            assert!(
                faults(&a, d, mark)[0]
                    .rule
                    .contains("inside the penalty area")
            );
            let mut b = c.clone();
            b.players[3].pos = DVec2::new(side * (hl - 6.0), 25.0);
            assert!(
                faults(&b, d, mark)[0].rule.contains("penalty mark"),
                "{:?}",
                faults(&b, d, mark)
            );
            let mut t = c.clone();
            t.players[20].pos = mark + DVec2::new(0.0, 2.0);
            assert!(faults(&t, d, mark)[0].rule.contains("the taker"));

            // A shoot-out kick at the home end: everyone else in the centre circle.
            let mut s = c.clone();
            for (i, p) in s.players.iter_mut().enumerate() {
                if i != 0 && i != 11 && i != 20 {
                    p.pos = DVec2::new(3.0 * ((i % 3) as f64 - 1.0), (i % 5) as f64 - 2.0);
                }
            }
            s.players[11].pos = DVec2::new(side * (hl - 0.5), 21.2);
            let kick = Restart::ShootoutKick {
                dead: dead(StoppageKind::Penalty, 1, mark, 20),
                keepers: [0, 11],
            };
            assert_eq!(faults(&s, kick, mark), []);
            let mut o = s.clone();
            o.players[5].pos = DVec2::new(0.0, 12.0);
            assert!(faults(&o, kick, mark)[0].rule.contains("centre circle"));
            let mut kk = s.clone();
            kk.players[11].pos = DVec2::new(side * (hl - 5.0), 10.0);
            assert!(
                faults(&kk, kick, mark)[0]
                    .rule
                    .contains("kicking team's goalkeeper")
            );
        }
    }

    #[test]
    fn a_fault_names_the_player_the_position_and_the_rule() {
        let fault = Fault {
            player: Some((20, 1, 9)),
            at: DVec2::new(1.0, 8.0),
            rule: "Law 8: an opponent 8.06 m from the ball, 9.15 m asked".into(),
        };
        assert_eq!(
            fault.to_string(),
            "player 20 (team 1, slot 9) at (1.00, 8.00): Law 8: an opponent 8.06 m from the ball, 9.15 m asked"
        );
    }
}

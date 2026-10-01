//! The pitch: 105 m by 68 m, centred at the origin. `x` runs along the length. The law
//! geometry (the areas, the marks, and the restart spots) follows IFAB Laws 1 and 13 to 17.

use crate::math::DVec2;

/// Pitch length in metres.
pub const LENGTH: f64 = 105.0;
/// Pitch width in metres.
pub const WIDTH: f64 = 68.0;
/// Goal width in metres.
pub const GOAL_WIDTH: f64 = 7.32;

/// Half the pitch length.
pub const HALF_LENGTH: f64 = LENGTH / 2.0;
/// Half the pitch width.
pub const HALF_WIDTH: f64 = WIDTH / 2.0;

/// Ball radius in metres. The ball is out of play only when all of it has crossed a line.
pub const BALL_RADIUS: f64 = 0.11;
/// Distance of the penalty mark from the goal line.
pub const PENALTY_MARK: f64 = 11.0;
/// Goal area depth from the goal line, and its width.
pub const GOAL_AREA_DEPTH: f64 = 5.5;
pub const GOAL_AREA_WIDTH: f64 = 18.32;
/// Penalty area depth from the goal line, and its width.
pub const PENALTY_AREA_DEPTH: f64 = 16.5;
pub const PENALTY_AREA_WIDTH: f64 = 40.32;
/// Radius of the corner arc.
pub const CORNER_ARC: f64 = 1.0;
/// Distance opponents keep from the ball at a free kick, a corner, a penalty, and a kick-off.
pub const KICK_DISTANCE: f64 = 9.15;
/// Metres beyond the touchline where a sent-off player stands.
const PARKING_OFFSET: f64 = 3.0;

/// The line the whole ball crossed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Line {
    /// A touchline; `side` is the sign of `y` on that side.
    Touch { side: f64 },
    /// A goal line; `side` is the sign of `x` on that side.
    Goal { side: f64 },
}

/// Where the ball left the pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Exit {
    pub line: Line,
    /// The point on the line where the ball's path crossed it.
    pub point: DVec2,
}

/// `true` when `p` lies inside the pitch (touchlines and goal lines included).
pub fn contains(p: DVec2) -> bool {
    p.x.abs() <= HALF_LENGTH && p.y.abs() <= HALF_WIDTH
}

/// Clamps `p` inside the pitch, `margin` metres away from every line.
pub fn clamp(p: DVec2, margin: f64) -> DVec2 {
    DVec2::new(
        p.x.clamp(-HALF_LENGTH + margin, HALF_LENGTH - margin),
        p.y.clamp(-HALF_WIDTH + margin, HALF_WIDTH - margin),
    )
}

/// The goal centre for a team that attacks in direction `attack_x` (+1 or -1).
pub fn goal_centre(attack_x: f64) -> DVec2 {
    DVec2::new(HALF_LENGTH * attack_x, 0.0)
}

/// `true` when a ball moving from `prev` to `p` is wholly over the goal line on the `attack_x`
/// side and crossed it between the posts (IFAB Law 10). The crossing point is read from the
/// step's path, not its end, so a fast diagonal ball that crosses inside a post and ends the
/// step wide of it still scores.
pub fn in_goal(prev: DVec2, p: DVec2, attack_x: f64) -> bool {
    if p.x * attack_x <= HALF_LENGTH + BALL_RADIUS {
        return false;
    }
    let line = HALF_LENGTH * attack_x;
    let dx = p.x - prev.x;
    let t = if dx.abs() < 1e-12 {
        0.0
    } else {
        ((line - prev.x) / dx).clamp(0.0, 1.0)
    };
    let y = prev.y + (p.y - prev.y) * t;
    y.abs() < GOAL_WIDTH / 2.0
}

/// The line a ball moving from `prev` to `ball` left the pitch over, or `None` while any part
/// of the ball is still on or over the pitch. A ball over both lines at once left over the
/// one its path crossed first.
pub fn exit(prev: DVec2, ball: DVec2) -> Option<Exit> {
    let out_x = ball.x.abs() > HALF_LENGTH + BALL_RADIUS;
    let out_y = ball.y.abs() > HALF_WIDTH + BALL_RADIUS;
    if !out_x && !out_y {
        return None;
    }
    let d = ball - prev;
    // The fraction of the step at which the path meets a line.
    let at = |from: f64, step: f64, to: f64| {
        if step.abs() < 1e-12 {
            0.0
        } else {
            ((to - from) / step).clamp(0.0, 1.0)
        }
    };
    let tx = out_x.then(|| at(prev.x, d.x, HALF_LENGTH * ball.x.signum()));
    let ty = out_y.then(|| at(prev.y, d.y, HALF_WIDTH * ball.y.signum()));
    let goal_line = match (tx, ty) {
        (Some(a), Some(b)) => a <= b,
        (Some(_), None) => true,
        _ => false,
    };
    if goal_line {
        let side = ball.x.signum();
        let t = tx.unwrap_or(0.0);
        let y = (prev.y + d.y * t).clamp(-HALF_WIDTH, HALF_WIDTH);
        Some(Exit {
            line: Line::Goal { side },
            point: DVec2::new(HALF_LENGTH * side, y),
        })
    } else {
        let side = ball.y.signum();
        let t = ty.unwrap_or(0.0);
        let x = (prev.x + d.x * t).clamp(-HALF_LENGTH, HALF_LENGTH);
        Some(Exit {
            line: Line::Touch { side },
            point: DVec2::new(x, HALF_WIDTH * side),
        })
    }
}

/// A throw-in spot: the crossing point on the touchline.
pub fn throw_in_spot(point: DVec2, side: f64) -> DVec2 {
    DVec2::new(point.x.clamp(-HALF_LENGTH, HALF_LENGTH), HALF_WIDTH * side)
}

/// A goal-kick spot: inside the goal area, on the half of the goal where the ball went out.
pub fn goal_kick_spot(side: f64, exit_y: f64) -> DVec2 {
    DVec2::new(
        (HALF_LENGTH - GOAL_AREA_DEPTH) * side,
        (GOAL_AREA_WIDTH / 2.0 - 1.0) * sign(exit_y),
    )
}

/// A corner spot: inside the corner arc nearest the point where the ball went out.
pub fn corner_spot(side: f64, exit_y: f64) -> DVec2 {
    let inset = CORNER_ARC * 0.7;
    DVec2::new(
        (HALF_LENGTH - inset) * side,
        (HALF_WIDTH - inset) * sign(exit_y),
    )
}

/// The penalty mark in front of the goal on the `side` end.
pub fn penalty_spot(side: f64) -> DVec2 {
    DVec2::new((HALF_LENGTH - PENALTY_MARK) * side, 0.0)
}

/// `true` when `p` lies inside the penalty area at the `side` end (lines included).
pub fn in_penalty_area(p: DVec2, side: f64) -> bool {
    p.x * side >= HALF_LENGTH - PENALTY_AREA_DEPTH
        && p.x * side <= HALF_LENGTH
        && p.y.abs() <= PENALTY_AREA_WIDTH / 2.0
}

/// Where the sent-off player of `team` in formation `slot` stands: beside the pitch, past
/// the touchline, a metre apart from every other parking spot.
pub fn parking_spot(team: usize, slot: usize) -> DVec2 {
    let side = if team == 0 { -1.0 } else { 1.0 };
    DVec2::new(side * (10.0 + slot as f64), -(HALF_WIDTH + PARKING_OFFSET))
}

/// `true` when `p` is a parking spot, to the precision of a 32-bit record.
pub fn is_parking_spot(p: DVec2) -> bool {
    (p.y + HALF_WIDTH + PARKING_OFFSET).abs() < 0.01 && p.x.abs() >= 9.99 && p.x.abs() <= 20.01
}

/// +1 for zero and a positive value, -1 for a negative value.
fn sign(v: f64) -> f64 {
    if v < 0.0 { -1.0 } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_and_clamp() {
        assert!(contains(DVec2::new(52.5, 34.0)));
        assert!(!contains(DVec2::new(52.6, 0.0)));
        assert_eq!(
            clamp(DVec2::new(100.0, -100.0), 0.5),
            DVec2::new(52.0, -33.5)
        );
    }

    /// The viewer hides a sent-off player by these spots; its tests read them from a shared
    /// file, and this test keeps that file equal to `parking_spot`.
    #[test]
    fn the_viewer_parking_spot_file_matches_the_engine() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../viewer/tests/data/parking-spots.json");
        let text = std::fs::read_to_string(&path).expect("the shared parking-spot file");
        let doc: serde_json::Value = serde_json::from_str(&text).unwrap();
        let spots = doc["spots"].as_array().unwrap();
        assert_eq!(spots.len(), 22);
        for spot in spots {
            let team = spot["team"].as_u64().unwrap() as usize;
            let slot = spot["slot"].as_u64().unwrap() as usize;
            let at = parking_spot(team, slot);
            let cm = |v: f64| (v * 100.0).round() as i64;
            assert_eq!(
                spot["x_cm"].as_i64(),
                Some(cm(at.x)),
                "team {team} slot {slot}"
            );
            assert_eq!(
                spot["y_cm"].as_i64(),
                Some(cm(at.y)),
                "team {team} slot {slot}"
            );
        }
    }

    #[test]
    fn goal_detection_uses_the_posts() {
        assert!(in_goal(DVec2::new(52.0, 3.0), DVec2::new(52.7, 3.0), 1.0));
        assert!(!in_goal(DVec2::new(52.0, 4.0), DVec2::new(52.7, 4.0), 1.0));
        assert!(!in_goal(
            DVec2::new(-52.0, 0.0),
            DVec2::new(-52.7, 0.0),
            1.0
        ));
        // Only part of the ball is over the line: not yet a goal.
        assert!(!in_goal(DVec2::new(52.0, 0.0), DVec2::new(52.6, 0.0), 1.0));
        // Crosses inside the post and ends the step wide of it: a goal.
        assert!(in_goal(DVec2::new(52.0, 3.0), DVec2::new(53.0, 4.0), 1.0));
    }

    #[test]
    fn a_ball_on_the_line_is_in_and_a_ball_fully_over_it_is_out() {
        let on = DVec2::new(10.0, HALF_WIDTH + BALL_RADIUS);
        assert_eq!(exit(DVec2::new(10.0, 33.0), on), None);
        let over = DVec2::new(10.0, HALF_WIDTH + BALL_RADIUS + 0.01);
        let out = exit(DVec2::new(10.0, 33.0), over).unwrap();
        assert_eq!(out.line, Line::Touch { side: 1.0 });
        assert!((out.point - DVec2::new(10.0, HALF_WIDTH)).length() < 1e-9);
    }

    #[test]
    fn the_crossing_point_lies_on_the_path() {
        let out = exit(DVec2::new(50.0, 0.0), DVec2::new(53.0, 3.0)).unwrap();
        assert_eq!(out.line, Line::Goal { side: 1.0 });
        assert!((out.point - DVec2::new(52.5, 2.5)).length() < 1e-9);
        let out = exit(DVec2::new(-10.0, -30.0), DVec2::new(-12.0, -34.5)).unwrap();
        assert_eq!(out.line, Line::Touch { side: -1.0 });
        assert!((out.point.x + 11.777_777).abs() < 1e-5, "{:?}", out.point);
    }

    #[test]
    fn restart_spots_follow_the_laws() {
        assert_eq!(
            throw_in_spot(DVec2::new(12.0, 34.0), 1.0),
            DVec2::new(12.0, 34.0)
        );
        let gk = goal_kick_spot(-1.0, 20.0);
        assert!((gk.x + 47.0).abs() < 1e-9 && gk.y > 0.0 && gk.y < GOAL_AREA_WIDTH / 2.0);
        let corner = corner_spot(1.0, -30.0);
        assert!((corner - DVec2::new(HALF_LENGTH, -HALF_WIDTH)).length() <= CORNER_ARC);
        assert_eq!(penalty_spot(1.0), DVec2::new(41.5, 0.0));
        assert!(in_penalty_area(DVec2::new(40.0, 20.0), 1.0));
        assert!(!in_penalty_area(DVec2::new(35.0, 0.0), 1.0));
        assert!(!in_penalty_area(DVec2::new(40.0, 0.0), -1.0));
    }

    #[test]
    fn parking_spots_are_off_the_pitch_and_apart() {
        let mut spots = Vec::new();
        for team in 0..2 {
            for slot in 0..11 {
                let p = parking_spot(team, slot);
                assert!(!contains(p));
                assert!(is_parking_spot(p));
                spots.push(p);
            }
        }
        for (i, a) in spots.iter().enumerate() {
            for b in &spots[i + 1..] {
                assert!((*a - *b).length() >= 1.0);
            }
        }
        assert!(!is_parking_spot(DVec2::new(0.0, -34.0)));
    }
}

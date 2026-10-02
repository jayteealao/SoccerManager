//! The pitch: a ground's length and width, centred at the origin. `x` runs along the length.
//! The lines of the ground (the touchlines, the goal lines, the halfway line) and every spot
//! measured from them follow the ground's size; the Laws fix the rest (the goal, the areas,
//! the marks, the arcs, and the kick distance), as IFAB Laws 1 and 13 to 17 give them.

use std::fmt;

use crate::math::DVec2;

/// Goal width in metres.
pub const GOAL_WIDTH: f64 = 7.32;
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

/// The length and width of a ground that gives none: 105 m by 68 m.
const DEFAULT_LENGTH: f64 = 105.0;
const DEFAULT_WIDTH: f64 = 68.0;

/// The touchline lengths the Laws allow (Law 1), in metres.
pub const LAWS_LENGTH: (f64, f64) = (90.0, 120.0);
/// The goal line lengths the Laws allow (Law 1), in metres.
pub const LAWS_WIDTH: (f64, f64) = (45.0, 90.0);

/// A ground size the Laws refuse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GroundError {
    /// The touchline is outside 90 to 120 m, or not a number.
    Length(f64),
    /// The goal line is outside 45 to 90 m, or not a number.
    Width(f64),
    /// The touchline is not longer than the goal line.
    NotLonger { length: f64, width: f64 },
}

impl fmt::Display for GroundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            GroundError::Length(v) => write!(
                f,
                "the ground is {v} m long; the Laws allow {} to {} m",
                LAWS_LENGTH.0, LAWS_LENGTH.1
            ),
            GroundError::Width(v) => write!(
                f,
                "the ground is {v} m wide; the Laws allow {} to {} m",
                LAWS_WIDTH.0, LAWS_WIDTH.1
            ),
            GroundError::NotLonger { length, width } => write!(
                f,
                "the ground is {length} m long and {width} m wide; the Laws need the touchline longer than the goal line"
            ),
        }
    }
}

/// One ground's pitch. Every match plays on one pitch, the home team's ground, and it never
/// changes in a match. On the default ground every method gives exactly what the fixed
/// 105 by 68 pitch gave, because each keeps the same arithmetic over the same values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pitch {
    length: f64,
    width: f64,
    half_length: f64,
    half_width: f64,
    /// `length / 105` and `width / 68`, divided once here instead of at every formation
    /// anchor: the same division of the same values, so the same bits.
    scale_x: f64,
    scale_y: f64,
}

impl Default for Pitch {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Pitch {
    /// The ground of a team file that gives none.
    pub const DEFAULT: Pitch = Pitch::raw(DEFAULT_LENGTH, DEFAULT_WIDTH);

    const fn raw(length: f64, width: f64) -> Self {
        Self {
            length,
            width,
            half_length: length / 2.0,
            half_width: width / 2.0,
            scale_x: length / DEFAULT_LENGTH,
            scale_y: width / DEFAULT_WIDTH,
        }
    }

    /// A ground of `length` by `width` metres, or the Laws' reason to refuse it: the
    /// touchline 90 to 120 m, the goal line 45 to 90 m, and the touchline the longer.
    pub fn new(length: f64, width: f64) -> Result<Self, GroundError> {
        let within = |v: f64, (lo, hi): (f64, f64)| v.is_finite() && (lo..=hi).contains(&v);
        if !within(length, LAWS_LENGTH) {
            return Err(GroundError::Length(length));
        }
        if !within(width, LAWS_WIDTH) {
            return Err(GroundError::Width(width));
        }
        if length <= width {
            return Err(GroundError::NotLonger { length, width });
        }
        Ok(Self::raw(length, width))
    }

    /// The touchline length in metres.
    pub fn length(&self) -> f64 {
        self.length
    }

    /// The goal line length in metres.
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Half the touchline: the distance from the centre to a goal line.
    pub fn half_length(&self) -> f64 {
        self.half_length
    }

    /// Half the goal line: the distance from the centre to a touchline.
    pub fn half_width(&self) -> f64 {
        self.half_width
    }

    /// `true` for the default ground.
    pub fn is_default(&self) -> bool {
        *self == Self::DEFAULT
    }

    /// The factors that carry a position drawn for the default ground onto this one: along
    /// the touchline and across it. Both are exactly 1.0 on the default ground.
    pub fn scale(&self) -> (f64, f64) {
        (self.scale_x, self.scale_y)
    }
}

/// `v` clamped to `lo..=hi` with `f64::clamp`'s comparisons in its order (a NaN stays NaN),
/// without its `lo <= hi` check: every margin the engine passes is far smaller than half the
/// narrowest ground the Laws allow, so the bounds never cross, and the check cost a branch at
/// every clamp of the tick path once the bounds came from the ground instead of constants.
#[inline]
fn within(v: f64, lo: f64, hi: f64) -> f64 {
    debug_assert!(lo <= hi, "clamp bounds crossed: {lo} > {hi}");
    let mut v = v;
    if v < lo {
        v = lo;
    }
    if v > hi {
        v = hi;
    }
    v
}

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

impl Pitch {
    /// `true` when `p` lies inside the pitch (touchlines and goal lines included).
    pub fn contains(&self, p: DVec2) -> bool {
        p.x.abs() <= self.half_length && p.y.abs() <= self.half_width
    }

    /// Clamps `p` inside the pitch, `margin` metres away from every line.
    pub fn clamp(&self, p: DVec2, margin: f64) -> DVec2 {
        DVec2::new(
            within(p.x, -self.half_length + margin, self.half_length - margin),
            within(p.y, -self.half_width + margin, self.half_width - margin),
        )
    }

    /// The goal centre for a team that attacks in direction `attack_x` (+1 or -1).
    pub fn goal_centre(&self, attack_x: f64) -> DVec2 {
        DVec2::new(self.half_length * attack_x, 0.0)
    }

    /// `true` when a ball moving from `prev` to `p` is wholly over the goal line on the `attack_x`
    /// side and crossed it between the posts (IFAB Law 10). The crossing point is read from the
    /// step's path, not its end, so a fast diagonal ball that crosses inside a post and ends the
    /// step wide of it still scores.
    pub fn in_goal(&self, prev: DVec2, p: DVec2, attack_x: f64) -> bool {
        if p.x * attack_x <= self.half_length + BALL_RADIUS {
            return false;
        }
        let line = self.half_length * attack_x;
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
    pub fn exit(&self, prev: DVec2, ball: DVec2) -> Option<Exit> {
        let out_x = ball.x.abs() > self.half_length + BALL_RADIUS;
        let out_y = ball.y.abs() > self.half_width + BALL_RADIUS;
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
        let tx = out_x.then(|| at(prev.x, d.x, self.half_length * ball.x.signum()));
        let ty = out_y.then(|| at(prev.y, d.y, self.half_width * ball.y.signum()));
        let goal_line = match (tx, ty) {
            (Some(a), Some(b)) => a <= b,
            (Some(_), None) => true,
            _ => false,
        };
        if goal_line {
            let side = ball.x.signum();
            let t = tx.unwrap_or(0.0);
            let y = (prev.y + d.y * t).clamp(-self.half_width, self.half_width);
            Some(Exit {
                line: Line::Goal { side },
                point: DVec2::new(self.half_length * side, y),
            })
        } else {
            let side = ball.y.signum();
            let t = ty.unwrap_or(0.0);
            let x = (prev.x + d.x * t).clamp(-self.half_length, self.half_length);
            Some(Exit {
                line: Line::Touch { side },
                point: DVec2::new(x, self.half_width * side),
            })
        }
    }

    /// A throw-in spot: the crossing point on the touchline.
    pub fn throw_in_spot(&self, point: DVec2, side: f64) -> DVec2 {
        DVec2::new(
            point.x.clamp(-self.half_length, self.half_length),
            self.half_width * side,
        )
    }

    /// A goal-kick spot: inside the goal area, on the half of the goal where the ball went out.
    pub fn goal_kick_spot(&self, side: f64, exit_y: f64) -> DVec2 {
        DVec2::new(
            (self.half_length - GOAL_AREA_DEPTH) * side,
            (GOAL_AREA_WIDTH / 2.0 - 1.0) * sign(exit_y),
        )
    }

    /// A corner spot: inside the corner arc nearest the point where the ball went out.
    pub fn corner_spot(&self, side: f64, exit_y: f64) -> DVec2 {
        let inset = CORNER_ARC * 0.7;
        DVec2::new(
            (self.half_length - inset) * side,
            (self.half_width - inset) * sign(exit_y),
        )
    }

    /// The penalty mark in front of the goal on the `side` end.
    pub fn penalty_spot(&self, side: f64) -> DVec2 {
        DVec2::new((self.half_length - PENALTY_MARK) * side, 0.0)
    }

    /// `true` when `p` lies inside the penalty area at the `side` end (lines included).
    pub fn in_penalty_area(&self, p: DVec2, side: f64) -> bool {
        p.x * side >= self.half_length - PENALTY_AREA_DEPTH
            && p.x * side <= self.half_length
            && p.y.abs() <= PENALTY_AREA_WIDTH / 2.0
    }

    /// Where the sent-off player of `team` in formation `slot` stands: beside the pitch, past
    /// the touchline, a metre apart from every other parking spot.
    pub fn parking_spot(&self, team: usize, slot: usize) -> DVec2 {
        let side = if team == 0 { -1.0 } else { 1.0 };
        DVec2::new(
            side * (10.0 + slot as f64),
            -(self.half_width + PARKING_OFFSET),
        )
    }

    /// `true` when `p` is a parking spot, to the precision of a 32-bit record.
    pub fn is_parking_spot(&self, p: DVec2) -> bool {
        (p.y + self.half_width + PARKING_OFFSET).abs() < 0.01
            && p.x.abs() >= 9.99
            && p.x.abs() <= 20.01
    }
}

/// +1 for zero and a positive value, -1 for a negative value.
fn sign(v: f64) -> f64 {
    if v < 0.0 { -1.0 } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: Pitch = Pitch::DEFAULT;

    #[test]
    fn contains_and_clamp() {
        assert!(P.contains(DVec2::new(52.5, 34.0)));
        assert!(!P.contains(DVec2::new(52.6, 0.0)));
        assert_eq!(
            P.clamp(DVec2::new(100.0, -100.0), 0.5),
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
            let at = P.parking_spot(team, slot);
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
        assert!(P.in_goal(DVec2::new(52.0, 3.0), DVec2::new(52.7, 3.0), 1.0));
        assert!(!P.in_goal(DVec2::new(52.0, 4.0), DVec2::new(52.7, 4.0), 1.0));
        assert!(!P.in_goal(DVec2::new(-52.0, 0.0), DVec2::new(-52.7, 0.0), 1.0));
        // Only part of the ball is over the line: not yet a goal.
        assert!(!P.in_goal(DVec2::new(52.0, 0.0), DVec2::new(52.6, 0.0), 1.0));
        // Crosses inside the post and ends the step wide of it: a goal.
        assert!(P.in_goal(DVec2::new(52.0, 3.0), DVec2::new(53.0, 4.0), 1.0));
    }

    #[test]
    fn a_ball_on_the_line_is_in_and_a_ball_fully_over_it_is_out() {
        let on = DVec2::new(10.0, P.half_width() + BALL_RADIUS);
        assert_eq!(P.exit(DVec2::new(10.0, 33.0), on), None);
        let over = DVec2::new(10.0, P.half_width() + BALL_RADIUS + 0.01);
        let out = P.exit(DVec2::new(10.0, 33.0), over).unwrap();
        assert_eq!(out.line, Line::Touch { side: 1.0 });
        assert!((out.point - DVec2::new(10.0, P.half_width())).length() < 1e-9);
    }

    #[test]
    fn the_crossing_point_lies_on_the_path() {
        let out = P
            .exit(DVec2::new(50.0, 0.0), DVec2::new(53.0, 3.0))
            .unwrap();
        assert_eq!(out.line, Line::Goal { side: 1.0 });
        assert!((out.point - DVec2::new(52.5, 2.5)).length() < 1e-9);
        let out = P
            .exit(DVec2::new(-10.0, -30.0), DVec2::new(-12.0, -34.5))
            .unwrap();
        assert_eq!(out.line, Line::Touch { side: -1.0 });
        assert!((out.point.x + 11.777_777).abs() < 1e-5, "{:?}", out.point);
    }

    #[test]
    fn restart_spots_follow_the_laws() {
        assert_eq!(
            P.throw_in_spot(DVec2::new(12.0, 34.0), 1.0),
            DVec2::new(12.0, 34.0)
        );
        let gk = P.goal_kick_spot(-1.0, 20.0);
        assert!((gk.x + 47.0).abs() < 1e-9 && gk.y > 0.0 && gk.y < GOAL_AREA_WIDTH / 2.0);
        let corner = P.corner_spot(1.0, -30.0);
        assert!((corner - DVec2::new(P.half_length(), -P.half_width())).length() <= CORNER_ARC);
        assert_eq!(P.penalty_spot(1.0), DVec2::new(41.5, 0.0));
        assert!(P.in_penalty_area(DVec2::new(40.0, 20.0), 1.0));
        assert!(!P.in_penalty_area(DVec2::new(35.0, 0.0), 1.0));
        assert!(!P.in_penalty_area(DVec2::new(40.0, 0.0), -1.0));
    }

    #[test]
    fn parking_spots_are_off_the_pitch_and_apart() {
        let mut spots = Vec::new();
        for team in 0..2 {
            for slot in 0..11 {
                let p = P.parking_spot(team, slot);
                assert!(!P.contains(p));
                assert!(P.is_parking_spot(p));
                spots.push(p);
            }
        }
        for (i, a) in spots.iter().enumerate() {
            for b in &spots[i + 1..] {
                assert!((*a - *b).length() >= 1.0);
            }
        }
        assert!(!P.is_parking_spot(DVec2::new(0.0, -34.0)));
    }

    #[test]
    fn the_laws_bound_the_ground() {
        for (length, width) in [(90.0, 68.0), (120.0, 68.0), (105.0, 45.0), (105.0, 90.0)] {
            assert!(Pitch::new(length, width).is_ok(), "{length} by {width}");
        }
        assert_eq!(Pitch::new(89.0, 68.0), Err(GroundError::Length(89.0)));
        assert_eq!(Pitch::new(121.0, 68.0), Err(GroundError::Length(121.0)));
        assert_eq!(Pitch::new(105.0, 44.0), Err(GroundError::Width(44.0)));
        assert_eq!(Pitch::new(105.0, 91.0), Err(GroundError::Width(91.0)));
        assert!(Pitch::new(f64::NAN, 68.0).is_err());
        assert_eq!(
            Pitch::new(90.0, 90.0),
            Err(GroundError::NotLonger {
                length: 90.0,
                width: 90.0
            })
        );
        assert_eq!(
            GroundError::Length(121.0).to_string(),
            "the ground is 121 m long; the Laws allow 90 to 120 m"
        );
        assert_eq!(
            GroundError::Width(44.0).to_string(),
            "the ground is 44 m wide; the Laws allow 45 to 90 m"
        );
        assert_eq!(
            GroundError::NotLonger {
                length: 90.0,
                width: 90.0
            }
            .to_string(),
            "the ground is 90 m long and 90 m wide; the Laws need the touchline longer than the goal line"
        );
    }

    #[test]
    fn the_default_ground_is_105_by_68_and_scales_by_exactly_one() {
        assert_eq!(Pitch::new(105.0, 68.0), Ok(Pitch::DEFAULT));
        assert_eq!((P.length(), P.width()), (105.0, 68.0));
        assert_eq!((P.half_length(), P.half_width()), (52.5, 34.0));
        assert_eq!(P.scale(), (1.0, 1.0));
        assert!(P.is_default());
    }

    #[test]
    fn the_lines_follow_the_ground_and_the_law_marks_do_not() {
        for (length, width) in [(100.0, 64.0), (120.0, 90.0), (90.0, 45.0)] {
            let g = Pitch::new(length, width).unwrap();
            let (hl, hw) = (length / 2.0, width / 2.0);
            assert!(g.contains(DVec2::new(hl, hw)));
            assert!(!g.contains(DVec2::new(hl + 0.1, 0.0)));
            assert_eq!(
                g.clamp(DVec2::new(500.0, -500.0), 0.5),
                DVec2::new(hl - 0.5, -hw + 0.5)
            );
            assert_eq!(g.goal_centre(-1.0), DVec2::new(-hl, 0.0));
            assert_eq!(g.penalty_spot(1.0), DVec2::new(hl - PENALTY_MARK, 0.0));
            assert_eq!(g.goal_kick_spot(1.0, 5.0).x, hl - GOAL_AREA_DEPTH);
            assert!((g.corner_spot(-1.0, 9.0) - DVec2::new(-hl, hw)).length() <= CORNER_ARC);
            assert_eq!(
                g.throw_in_spot(DVec2::new(3.0, 0.0), -1.0),
                DVec2::new(3.0, -hw)
            );
            assert!(g.in_penalty_area(DVec2::new(hl - PENALTY_AREA_DEPTH, 20.0), 1.0));
            assert!(!g.in_penalty_area(DVec2::new(hl - PENALTY_AREA_DEPTH - 0.1, 0.0), 1.0));
            assert!(g.in_goal(DVec2::new(hl - 0.5, 3.0), DVec2::new(hl + 0.2, 3.0), 1.0));
            assert!(!g.in_goal(DVec2::new(hl - 0.5, 4.0), DVec2::new(hl + 0.2, 4.0), 1.0));
            let out = g
                .exit(DVec2::new(0.0, hw - 1.0), DVec2::new(0.0, hw + 0.5))
                .unwrap();
            assert_eq!(out.line, Line::Touch { side: 1.0 });
            for team in 0..2 {
                for slot in 0..11 {
                    let spot = g.parking_spot(team, slot);
                    assert!(!g.contains(spot) && g.is_parking_spot(spot));
                }
            }
        }
    }
}

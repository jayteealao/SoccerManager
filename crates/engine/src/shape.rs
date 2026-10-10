//! Team shape on top of the formation anchors: the stand options each player may take from
//! his role's menu, and the rest-defence guard of the team in possession.
//!
//! The anchors already stand each position at its stand for the phase (the formation slot
//! moved by the position's stand, the role, the duty and the mentality, then by the ball
//! shift). This pass then lets a player take the first option of his role's menu whose
//! condition holds, read from the ball, the carrier and where his team-mates stand: he drops
//! short for the carrier, widens, moves into the half-space, or overlaps or underlaps a
//! team-mate ahead of him on his side. No option names a partner, so a pairing such as a
//! full-back overlapping a winger follows from whoever stands there.
//!
//! The guard then keeps at least the mentality's minimum of outfield players goal-side of the
//! ball while the team has it: below the minimum it holds back the players with the most
//! defensive duty and the least far forward; above it, a support or defend duty goes forward
//! only with a team-mate goal-side in his lane, an attacking duty goes; and when a full-back
//! goes forward, the nearest team-mate who would also go stays to cover him if he is familiar
//! enough with his place (the familiarity family; neutral until it is fed).
//!
//! Every choice is a pure function of the match state: the pass draws no random number.

use crate::data::tactics::StandOption;
use crate::data::team::Position;
use crate::math::DVec2;
use crate::modules::{MatchView, ROSTER};
use crate::tactics::Menu;
use crate::team::{PLAYERS_PER_TEAM, Team};

/// Metres in depth within which a team-mate standing inside a player makes him widen.
const WIDEN_DEPTH_M: f64 = 10.0;
/// Metres in depth within which a team-mate wider than a player sends him into the
/// half-space.
const HALF_SPACE_DEPTH_M: f64 = 15.0;
/// A player within this many metres of the half-space line is in it already.
const HALF_SPACE_NEAR_M: f64 = 2.0;
/// Metres ahead within which a team-mate on the same side can be overlapped or underlapped,
/// and how much narrower than the player he may stand.
const OVERLAP_DEPTH_M: f64 = 25.0;
const OVERLAP_NARROWER_M: f64 = 4.0;
/// Metres from the touchline within which a team-mate holds the touchline lane: a player
/// runs inside him (underlap), not round him (overlap).
const TOUCHLINE_LANE_M: f64 = 8.0;
/// A gap between two neighbours of the back line wider than this share of the widest gap
/// the tuning allows draws a dropping midfielder into it out of possession.
const BACK_LINE_GAP_SHARE: f64 = 1.25;
/// Margin inside the pitch for every option spot, as for anchors.
const PITCH_MARGIN_M: f64 = 0.5;

/// What the shape pass did to one team, by slot.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ShapeOutcome {
    /// The stand option each slot took, if any.
    pub option: [Option<StandOption>; PLAYERS_PER_TEAM],
    /// One bit per slot the guard held back.
    pub held: u16,
    /// One bit per slot who stayed to cover a full-back who went forward.
    pub covering: u16,
}

impl ShapeOutcome {
    /// `true` when the pass moved slot `slot` off its anchor.
    pub fn touched(&self, slot: usize) -> bool {
        self.option[slot].is_some() || (self.held | self.covering) & (1 << slot) != 0
    }
}

/// The anchor of every active player with `phase` the team in possession (`None`: both
/// teams stand out of possession). An outfield player's ball shift reads his followed ball
/// point ([`MatchView::slide_ball`]), so a far player's anchor slides late; the keeper's
/// reads the ball. An inactive player's entry is his position.
pub fn anchors(view: &MatchView<'_>, phase: Option<usize>) -> [DVec2; ROSTER] {
    let teams = view.teams();
    let t = view.tuning();
    let ball = view.ball().xy();
    let mut out = [DVec2::ZERO; ROSTER];
    for (i, (o, p)) in out.iter_mut().zip(view.players()).enumerate() {
        *o = if p.active() {
            let team = &teams[p.team];
            let seen = if p.slot == team.keeper_slot() {
                ball
            } else {
                view.slide_ball(i)
            };
            team.anchor(p.slot, seen, phase == Some(p.team), t)
        } else {
            p.pos
        };
    }
    out
}

/// Runs the shape pass for both teams: the team in `phase` takes its in-possession options and
/// the rest-defence guard, the other team its out-of-possession options. `anchors` are the
/// players' anchors for `phase` ([`anchors`]); `targets` holds the targets set so far and
/// takes the moves.
pub fn apply(
    view: &MatchView<'_>,
    phase: Option<usize>,
    anchors: &[DVec2; ROSTER],
    targets: &mut [DVec2; ROSTER],
) -> [ShapeOutcome; 2] {
    let mut out = [ShapeOutcome::default(); 2];
    for (team, o) in out.iter_mut().enumerate() {
        *o = team_shape(view, team, phase == Some(team), anchors, targets);
    }
    out
}

/// The team-frame helpers of one team.
struct Frame<'a> {
    view: &'a MatchView<'a>,
    side: &'a Team,
    team: usize,
    first: usize,
    ax: f64,
    keeper: usize,
    carrier: Option<usize>,
}

impl Frame<'_> {
    /// Metres up the pitch in the team's attack direction.
    fn depth(&self, at: DVec2) -> f64 {
        at.x * self.ax
    }

    /// The side of slot `slot` in pitch `y`: -1, +1, or 0 for a central slot.
    fn pitch_side(&self, slot: usize) -> f64 {
        self.side.slot_side(slot) * self.ax
    }

    /// The active outfield players of the team other than the keeper, in roster order.
    fn outfield(&self) -> impl Iterator<Item = usize> + '_ {
        (self.first..self.first + PLAYERS_PER_TEAM)
            .filter(|&i| i != self.keeper && self.view.player(i).active())
    }

    fn clamp(&self, at: DVec2) -> DVec2 {
        self.view.pitch().clamp(at, PITCH_MARGIN_M)
    }
}

/// The shape pass of `team`: options from the role menus of the phase, then, for the team in
/// possession, the rest-defence guard.
pub fn team_shape(
    view: &MatchView<'_>,
    team: usize,
    in_possession: bool,
    anchors: &[DVec2; ROSTER],
    targets: &mut [DVec2; ROSTER],
) -> ShapeOutcome {
    let side = &view.teams()[team];
    let f = Frame {
        view,
        side,
        team,
        first: team * PLAYERS_PER_TEAM,
        ax: side.attack_x,
        keeper: view.keeper(team),
        carrier: view.carrier(),
    };
    let mut out = ShapeOutcome::default();
    if let Some(sizes) = side.plan.sizes {
        options(&f, in_possession, &sizes, anchors, targets, &mut out);
    }
    if in_possession && let Some(rest) = side.plan.rest {
        guard(&f, &rest, anchors, targets, &mut out);
    }
    out
}

fn menu_of(side: &Team, slot: usize, in_possession: bool) -> &Menu {
    let plan = &side.plan.slots[slot];
    if in_possession {
        &plan.menu_in
    } else {
        &plan.menu_out
    }
}

fn has(menu: &Menu, option: StandOption) -> bool {
    menu.contains(&Some(option))
}

/// Each player takes the first option of his menu whose condition holds.
fn options(
    f: &Frame<'_>,
    in_possession: bool,
    sizes: &crate::data::tactics::StandOptionSizes,
    anchors: &[DVec2; ROSTER],
    targets: &mut [DVec2; ROSTER],
    out: &mut ShapeOutcome,
) {
    let view = f.view;
    let players = view.players();
    let ball = view.ball().xy();
    let (_, sy) = view.pitch().scale();
    let half_width = view.pitch().half_width();
    // The block's centre line across the pitch: the ball shift moves the whole block toward
    // the ball, so every across measure of an option is taken from it.
    let centre = ball.y * view.tuning().compactness_y;
    let carrier = f.carrier.filter(|&c| players[c].team == f.team);
    let back_line = f.side.back_line();
    let in_back_line = |i: usize| back_line.contains(&(i - f.first));
    let movers: Vec<usize> = f.outfield().filter(|&i| Some(i) != f.carrier).collect();

    // One dropper per team: in possession, the eligible player nearest the carrier; out of
    // possession, the first eligible player when the back line has a gap.
    let mut dropper: Option<(usize, DVec2)> = None;
    if in_possession {
        if let Some(c) = carrier {
            let carrier_depth = f.depth(players[c].pos);
            let mut best = f64::INFINITY;
            for &i in &movers {
                if !has(menu_of(f.side, i - f.first, true), StandOption::Drop) {
                    continue;
                }
                let me = anchors[i];
                if carrier_depth > f.depth(me) - sizes.drop_gap_m {
                    continue;
                }
                let spot = f.clamp(me - DVec2::new(sizes.drop_m * f.ax, 0.0));
                let filled = movers
                    .iter()
                    .chain(&[c])
                    .any(|&j| j != i && (players[j].pos - spot).length() < sizes.space_m);
                let d = (me - players[c].pos).length();
                if !filled && d < best {
                    best = d;
                    dropper = Some((i, spot));
                }
            }
        }
    } else if let Some(gap) = back_line_gap(f) {
        dropper = movers
            .iter()
            .find(|&&i| has(menu_of(f.side, i - f.first, false), StandOption::Drop))
            .map(|&i| (i, gap));
    }

    for &i in &movers {
        let slot = i - f.first;
        let menu = menu_of(f.side, slot, in_possession);
        if menu[0].is_none() {
            continue;
        }
        let me = anchors[i];
        let my_depth = f.depth(me);
        let s = f.pitch_side(slot);
        // The player's distance from the block's centre line on his own side.
        let my_out = (me.y - centre) * s;
        // Team-mates as they stand, the carrier included.
        let mates = || {
            (f.first..f.first + PLAYERS_PER_TEAM)
                .filter(move |&j| j != i && j != f.keeper && players[j].active())
        };
        let mut pick = None;
        for option in menu.iter().flatten() {
            let spot = match option {
                StandOption::Drop => dropper.filter(|&(d, _)| d == i).map(|(_, spot)| spot),
                StandOption::Widen if s != 0.0 => {
                    let widen = if in_back_line(i) {
                        carrier.is_some_and(|c| c == f.keeper || in_back_line(c))
                    } else {
                        mates().any(|j| {
                            let at = players[j].pos;
                            let out = (at.y - centre) * s;
                            (0.0..my_out).contains(&out)
                                && my_out - out < sizes.space_m
                                && (f.depth(at) - my_depth).abs() < WIDEN_DEPTH_M
                        })
                    };
                    widen.then(|| me + DVec2::new(0.0, s * sizes.widen_m))
                }
                StandOption::HalfSpace if s != 0.0 => {
                    let line = sizes.half_space_y * sy;
                    let wider = mates().any(|j| {
                        let at = players[j].pos;
                        (at.y - centre) * s > my_out
                            && (f.depth(at) - my_depth).abs() < HALF_SPACE_DEPTH_M
                    });
                    (wider && (my_out - line).abs() > HALF_SPACE_NEAR_M)
                        .then(|| DVec2::new(me.x, centre + s * line))
                }
                StandOption::Overlap | StandOption::Underlap if s != 0.0 => {
                    // The nearest team-mate ahead on his side.
                    let ahead = mates()
                        .filter(|&j| {
                            let at = players[j].pos;
                            let d = f.depth(at) - my_depth;
                            (at.y - centre) * s > 0.0 && d > 0.0 && d <= OVERLAP_DEPTH_M
                        })
                        .min_by(|&a, &b| {
                            let da = (players[a].pos - me).length();
                            let db = (players[b].pos - me).length();
                            da.total_cmp(&db).then(a.cmp(&b))
                        });
                    ahead.and_then(|j| {
                        let at = players[j].pos;
                        let near = Some(j) == carrier || (at - ball).length() <= sizes.near_ball_m;
                        let wide = (at.y - centre) * s >= my_out - OVERLAP_NARROWER_M;
                        let lane = at.y.abs() >= half_width - TOUCHLINE_LANE_M;
                        let forward = DVec2::new(f.ax, 0.0);
                        match option {
                            StandOption::Overlap if near && wide && !lane => Some(
                                at + forward * sizes.overlap_ahead_m
                                    + DVec2::new(0.0, s * sizes.overlap_out_m),
                            ),
                            StandOption::Underlap if near && wide && lane => Some(
                                at + forward * sizes.underlap_ahead_m
                                    - DVec2::new(0.0, s * sizes.underlap_in_m),
                            ),
                            _ => None,
                        }
                    })
                }
                _ => None,
            };
            if let Some(spot) = spot {
                pick = Some((*option, f.clamp(spot)));
                break;
            }
        }
        if let Some((option, spot)) = pick {
            out.option[slot] = Some(option);
            targets[i] = spot;
        }
    }
}

/// Out of possession: the centre of the widest gap between two neighbours of the back line as
/// they stand, at the line's depth, when it is wider than the tuning allows by the gap share.
fn back_line_gap(f: &Frame<'_>) -> Option<DVec2> {
    let (_, sy) = f.view.pitch().scale();
    let mut line: Vec<DVec2> = f
        .side
        .back_line()
        .into_iter()
        .map(|slot| f.first + slot)
        .filter(|&i| i != f.keeper && f.view.player(i).active())
        .map(|i| f.view.player(i).pos)
        .collect();
    line.sort_by(|a, b| a.y.total_cmp(&b.y));
    let allowed = f.side.plan.back_line_gap * sy * BACK_LINE_GAP_SHARE;
    line.windows(2)
        .map(|w| (w[1].y - w[0].y, (w[0] + w[1]) / 2.0))
        .filter(|&(gap, _)| gap > allowed)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, centre)| f.clamp(centre))
}

/// The rest-defence guard of the team in possession.
fn guard(
    f: &Frame<'_>,
    rest: &crate::data::tactics::RestDefence,
    anchors: &[DVec2; ROSTER],
    targets: &mut [DVec2; ROSTER],
    out: &mut ShapeOutcome,
) {
    let view = f.view;
    let players = view.players();
    let ball = view.ball().xy();
    let t = view.tuning();
    let plan = &f.side.plan;
    // Goal-side of the ball, or of the halfway line while the ball is in the team's own half:
    // in build-up the whole team stands behind the halfway line and nobody is held back.
    let line = f.depth(ball).max(0.0);
    let outfield: Vec<usize> = f.outfield().collect();
    let candidates: Vec<usize> = outfield
        .iter()
        .copied()
        .filter(|&i| Some(i) != f.carrier)
        .collect();
    let need = plan.rest_defence.min(outfield.len().saturating_sub(1));
    let goal_side = |targets: &[DVec2; ROSTER], i: usize| f.depth(targets[i]) < line;
    let duty = |i: usize| plan.slots[i - f.first].depth;
    // Where a held player stands: across at his stand (any option he took is dropped), in
    // depth his out-of-possession anchor or the margin goal-side of the line, whichever is
    // deeper.
    let hold_at = |i: usize, across: f64| {
        let slot = i - f.first;
        let back = f.side.anchor(slot, ball, false, t);
        let depth = f.depth(back).min(line - rest.hold_margin_m);
        f.clamp(DVec2::new(depth * f.ax, across))
    };
    let hold = |i: usize, across: f64, targets: &mut [DVec2; ROSTER], out: &mut ShapeOutcome| {
        targets[i] = hold_at(i, across);
        out.option[i - f.first] = None;
    };

    // Below the minimum: the most defensive duty first, then the least far forward.
    let count = candidates
        .iter()
        .filter(|&&i| goal_side(targets, i))
        .count();
    if count < need {
        let mut forward: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|&i| !goal_side(targets, i))
            .collect();
        forward.sort_by(|&a, &b| {
            duty(a)
                .total_cmp(&duty(b))
                .then(f.depth(targets[a]).total_cmp(&f.depth(targets[b])))
                .then(a.cmp(&b))
        });
        for &i in forward.iter().take(need - count) {
            hold(i, anchors[i].y, targets, out);
            out.held |= 1 << (i - f.first);
        }
    }

    // Above it: a support or defend duty goes forward only with a team-mate goal-side in his
    // lane; an attacking duty goes.
    for &i in &candidates {
        if goal_side(targets, i) || duty(i) > 0.0 {
            continue;
        }
        let covered = outfield.iter().any(|&j| {
            j != i && goal_side(targets, j) && (targets[j].y - targets[i].y).abs() <= rest.lane_m
        });
        if !covered {
            hold(i, anchors[i].y, targets, out);
            out.held |= 1 << (i - f.first);
        }
    }

    // Cover: a team-mate stays for a full-back who went forward, if he is familiar enough
    // with his place.
    let back_line = f.side.back_line();
    for &b in &outfield {
        let slot = b - f.first;
        if !matches!(plan.slots[slot].position, Some(Position::LB | Position::RB)) {
            continue;
        }
        let at = if Some(b) == f.carrier {
            players[b].pos
        } else {
            targets[b]
        };
        if f.depth(at) < line {
            continue;
        }
        let taken = out.held | out.covering;
        let nearest = candidates
            .iter()
            .copied()
            .filter(|&j| {
                let s = j - f.first;
                j != b && !back_line.contains(&s) && taken & (1 << s) == 0
            })
            .filter(|&j| !goal_side(targets, j))
            .min_by(|&a, &c| {
                (targets[a] - at)
                    .length()
                    .total_cmp(&(targets[c] - at).length())
                    .then(a.cmp(&c))
            });
        if let Some(j) = nearest
            && view.familiarity(j) >= rest.cover_familiarity_min
        {
            let lane = f.side.anchor(slot, ball, false, t).y;
            hold(j, (anchors[j].y + lane) / 2.0, targets, out);
            out.covering |= 1 << (j - f.first);
        }
    }
}

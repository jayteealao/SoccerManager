//! The decision probe of one match: what the carrier's choices cost and where the ball went,
//! counted to diagnose the option scores against real football.
//!
//! - Kicks by kind (shot, restart kick, clearance, pass) and by the kicker's third, and the
//!   clearances that the other side gained next.
//! - Goals with a shot of the scoring side in flight, and goals with none by who kicked the
//!   ball last: the conceding keeper, a conceding outfield player, a player of the scoring
//!   side, or nobody.
//! - The carrier's options in the final third he attacks, scored with every noise draw at its
//!   middle: how often each kind is open and the top, the mean of its score, and the mean of
//!   each term of the best pass ([`PASS_TERMS`]).
//! - Every open-play shot: its expected goals, the keeper's distance off his line, the
//!   shooter's execution share and his read of the chance, and whether it scored.
//!
//! Every count is per side, home first.

use std::cell::RefCell;

/// Kick kinds, in [`Probe::kicks`] order.
pub const KICK_KINDS: [&str; 4] = ["shot", "restart", "clear", "pass"];
/// The option kinds, in the [`crate::measures::CHOICES`] order.
pub const OPTION_KINDS: usize = 5;
/// The terms of a pass's score, in the order the decision module adds them. The pass's chance
/// of losing the ball is split by the contest it meets: the passer's own pressed play of it
/// (`loss_own`), the cut-outs on its way (`loss_cut`) and the receiver's touch and tackle on
/// arrival (`loss_arrival`).
pub const PASS_TERMS: [&str; 17] = [
    "progress",
    "vision",
    "teamwork",
    "lane",
    "space",
    "distance",
    "directness",
    "counter",
    "recycle",
    "loss_own",
    "role_target",
    "plan_bias",
    "loss_cut",
    "loss_arrival",
    "offside_cost",
    "screened",
    "rest",
];

/// One open-play shot.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize)]
pub struct ShotRow {
    pub team: usize,
    pub distance_m: f64,
    pub outside: bool,
    /// The recorded expected goals and the keeper's distance off his goal line at the kick.
    pub xg: f64,
    pub keeper_off_m: f64,
    /// The shooter's execution share at the kick.
    pub strike: f64,
    pub goal: bool,
}

/// The carrier's options in the attacked final third.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize)]
pub struct FinalThird {
    /// Decisions counted (one per change of choice, as the choices count).
    pub decisions: u32,
    /// Per option kind: open, the top-scored, and the sum of its score when open.
    pub open: [u32; OPTION_KINDS],
    pub top: [u32; OPTION_KINDS],
    pub score: [f64; OPTION_KINDS],
    /// The sums of each term of the best pass, when a pass is open.
    pub pass_terms: [f64; 17],
    /// The pass candidates in range before the lane test, and those with a lane open.
    pub candidates: u32,
    pub lane_open: u32,
}

/// The decision probe ([module docs](self)).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize)]
pub struct Probe {
    pub kicks: [[[u32; 3]; 4]; 2],
    pub clear_resolved: [u32; 2],
    pub clear_lost: [u32; 2],
    pub goals_shot: [u32; 2],
    pub goals_no_shot: [[u32; 4]; 2],
    pub final_third: [FinalThird; 2],
    pub shots: Vec<ShotRow>,
    #[serde(skip)]
    clear_pending: Option<usize>,
    #[serde(skip)]
    shot_open: Option<usize>,
}

impl Probe {
    /// `team` kicked a kick of `kind` ([`KICK_KINDS`]) from `third`. Any kick settles the
    /// clearance in progress for the kicker's side and the open shot; a clearance opens a new one.
    pub(crate) fn kick(&mut self, team: usize, kind: usize, third: usize) {
        self.settle(team);
        self.kicks[team][kind][third] += 1;
        if kind == 2 {
            self.clear_pending = Some(team);
        }
        self.shot_open = None;
    }

    /// An open-play shot was struck.
    pub(crate) fn shot(&mut self, row: ShotRow) {
        self.shot_open = Some(self.shots.len());
        self.shots.push(row);
    }

    /// `team` gained the ball or took a restart: the clearance in progress ends with it.
    pub(crate) fn settle(&mut self, team: usize) {
        if let Some(side) = self.clear_pending.take() {
            self.clear_resolved[side] += 1;
            self.clear_lost[side] += u32::from(side != team);
        }
    }

    /// `team` scored: with its shot in flight (`shot`), or with none, the last kick by `by`.
    pub(crate) fn goal(&mut self, team: usize, shot: bool, by: usize) {
        if shot {
            self.goals_shot[team] += 1;
            if let Some(i) = self.shot_open.take()
                && self.shots[i].team == team
            {
                self.shots[i].goal = true;
            }
        } else {
            self.goals_no_shot[team][by] += 1;
        }
        self.clear_pending = None;
        self.shot_open = None;
    }

    /// The ball went dead: the open shot is over.
    pub(crate) fn dead(&mut self) {
        self.shot_open = None;
    }

    /// The carrier of `team` in the final third he attacks, at a decision: his options'
    /// plain scores by kind (`None` when closed), the top kind, the best pass's terms, and
    /// the candidates in range and with a lane.
    pub(crate) fn final_third(
        &mut self,
        team: usize,
        scores: [Option<f64>; OPTION_KINDS],
        top: usize,
        pass_terms: Option<[f64; 17]>,
        candidates: (u32, u32),
    ) {
        let f = &mut self.final_third[team];
        f.decisions += 1;
        f.top[top] += 1;
        f.candidates += candidates.0;
        f.lane_open += candidates.1;
        for (k, s) in scores.iter().enumerate() {
            if let Some(s) = s {
                f.open[k] += 1;
                f.score[k] += s;
            }
        }
        if let Some(terms) = pass_terms {
            for (sum, t) in f.pass_terms.iter_mut().zip(terms) {
                *sum += t;
            }
        }
    }
}

/// The pass terms of the last option draft, noted by the decision module as it scores each
/// pass (a measures build only): the terms of each pass in draft order, and the team-mates in
/// passing range and those of them with a lane open.
#[derive(Debug, Clone, Copy)]
pub struct DraftNotes {
    pub terms: [[f64; 17]; crate::modules::MATES],
    pub in_range: u32,
    pub lane_open: u32,
}

impl Default for DraftNotes {
    fn default() -> Self {
        Self {
            terms: [[0.0; 17]; crate::modules::MATES],
            in_range: 0,
            lane_open: 0,
        }
    }
}

thread_local! {
    static NOTES: RefCell<DraftNotes> = RefCell::new(DraftNotes::default());
}

/// A new option draft starts.
pub(crate) fn start_draft() {
    NOTES.with(|n| *n.borrow_mut() = DraftNotes::default());
}

/// A team-mate in passing range; `open` when his lane passes the narrowest lane.
pub(crate) fn note_candidate(open: bool) {
    NOTES.with(|n| {
        let mut n = n.borrow_mut();
        n.in_range += 1;
        n.lane_open += u32::from(open);
    });
}

/// The terms of the pass at draft index `k`; the last term is the rest of `score`.
pub(crate) fn note_pass(k: usize, mut terms: [f64; 17], score: f64) {
    terms[16] = score - terms[..16].iter().sum::<f64>();
    NOTES.with(|n| n.borrow_mut().terms[k] = terms);
}

/// The notes of the last draft.
pub(crate) fn notes() -> DraftNotes {
    NOTES.with(|n| *n.borrow())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A clearance is lost when the other side gains the ball next, and kept when the
    /// clearer's side does; a dead ball leaves it open until the restart is taken.
    #[test]
    fn a_clearance_is_lost_to_the_side_that_gains_the_ball_next() {
        let mut p = Probe::default();
        p.kick(0, 2, 0);
        p.settle(1);
        p.kick(1, 2, 0);
        p.dead();
        p.settle(1);
        assert_eq!(p.kicks[0][2][0], 1);
        assert_eq!(p.clear_resolved, [1, 1]);
        assert_eq!(p.clear_lost, [1, 0]);
    }

    /// A goal with its side's shot open marks that shot scored; one with none counts by who
    /// kicked the ball last, and a shot the ball went dead after scores nothing.
    #[test]
    fn a_goal_marks_its_open_shot_or_counts_by_the_last_kicker() {
        let mut p = Probe::default();
        let row = ShotRow {
            team: 0,
            ..ShotRow::default()
        };
        p.shot(row);
        p.goal(0, true, 2);
        p.shot(row);
        p.dead();
        p.goal(1, false, 0);
        assert_eq!(p.goals_shot, [1, 0]);
        assert_eq!(p.goals_no_shot, [[0; 4], [1, 0, 0, 0]]);
        assert_eq!(
            p.shots.iter().map(|s| s.goal).collect::<Vec<_>>(),
            [true, false]
        );
    }

    /// The last pass term is the rest of the score, so the terms add up to it.
    #[test]
    fn the_noted_pass_terms_add_up_to_its_score() {
        start_draft();
        note_candidate(true);
        note_candidate(false);
        let mut terms = [0.0; 17];
        terms[0] = 0.5;
        terms[13] = -0.25;
        note_pass(0, terms, 1.0);
        let n = notes();
        assert_eq!((n.in_range, n.lane_open), (2, 1));
        assert!((n.terms[0].iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }
}

//! The job probe: per-side counters of the events each attribute job moves, filled by count
//! sites in the match loop while a sensitivity run plays. It is a read-only seam: no site
//! takes a random draw or writes match state, the probe is never hashed by the replay gate
//! and never stored in a snapshot, and every other match runs with it absent (`None`), so a
//! site costs one branch.

use crate::decision::PassKind;
use crate::math::DVec2;

/// What one side did, as the job statistics count it. Every field counts events of the side's
/// own players, except where the name says the side faced or suffered the event.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SideCounts {
    /// Open-play ground passes of outfield players (up to 25 m, not a cross), and those a
    /// team-mate controlled next.
    pub passes: f64,
    pub passes_completed: f64,
    /// Open-play passes of every kind by outfield players with an opponent close.
    pub pressed_passes: f64,
    pub pressed_completed: f64,
    pub crosses: f64,
    pub crosses_completed: f64,
    /// Chips (lofted passes over 25 m) and take-ons (a dribble started with an opponent
    /// close) attempted.
    pub chips: f64,
    pub take_ons: f64,
    /// A keeper's open-play throws and kicks, and those a team-mate controlled next.
    pub throws: f64,
    pub throws_completed: f64,
    pub keeper_kicks: f64,
    pub keeper_kicks_completed: f64,
    /// Completed outfield passes, and the metres toward the opponents' goal they gained.
    pub progress_passes: f64,
    pub progress_m: f64,
    /// Passes that reached the intended team-mate (came within his reach, or were controlled
    /// by his side), and those his side controlled within one second of the ball reaching
    /// him.
    pub arrivals: f64,
    pub arrivals_kept: f64,
    /// Open-play shots (no penalty), split inside and outside the penalty area, and their
    /// expected goals.
    pub shots: f64,
    pub xg: f64,
    pub shots_in_box: f64,
    pub on_target_in_box: f64,
    pub goals_on_target_in_box: f64,
    pub shots_outside: f64,
    pub on_target_outside: f64,
    /// Tackle attempts by the side's players, and those that won the ball.
    pub tackles: f64,
    pub tackles_won: f64,
    /// Tackle attempts the side's carriers faced while running with the ball, and those
    /// that did not win the ball from him.
    pub running_tackles_faced: f64,
    pub running_tackles_beaten: f64,
    /// Tackles that won the ball from one of the side's carriers while he stood.
    pub standing_tackles_lost: f64,
    /// Fouls the side's carriers suffered, and those after which the side kept the ball.
    pub fouled: f64,
    pub fouled_kept: f64,
    /// Loose balls the side's outfield players won (not a pass of their own side arriving),
    /// and those won with an opponent outfield player within 5 m of the ball.
    pub loose_won: f64,
    pub loose_won_close: f64,
    /// High balls an outfield player of the side reached first, and the headers he won.
    pub headers: f64,
    pub headers_won: f64,
    /// On-target open-play shots the side's keeper faced from within and beyond the
    /// one-on-one distance, and those he saved.
    pub faced_near: f64,
    pub saved_near: f64,
    pub faced_far: f64,
    pub saved_far: f64,
    /// The keeper's saves, and those he held.
    pub saves: f64,
    pub saves_held: f64,
    /// Loose balls the side's keeper took above head height, from an opponent's cross in
    /// flight, and outside his own penalty area.
    pub high_claims: f64,
    pub crosses_claimed: f64,
    pub sweeps: f64,
    /// Samples of the distance between a covering defender of the side and the carrier he
    /// tracks, and their sum in metres.
    pub cover_n: f64,
    pub cover_m: f64,
    /// Passes to an attacker of the side who stood onside within [`TIMED_RUN_M`] of the
    /// second-last defender at the kick.
    pub timed_runs: f64,
    /// Corners the side took, and those a team-mate reached first.
    pub corners: f64,
    pub corners_reached: f64,
    /// Direct free kicks the side took, and those on target or reaching a team-mate first.
    pub free_kicks: f64,
    pub free_kicks_success: f64,
    /// Penalties the side took in play, and those it scored.
    pub penalties: f64,
    pub penalties_scored: f64,
    /// Throw-ins the side took, and the metres from the thrower to where each was aimed.
    pub throws_in: f64,
    pub throw_m: f64,
    /// Open-play passes by the side's outfield players that gain at least
    /// [`FLAIR_PASS_M`] toward the opponents' goal.
    pub flair_passes: f64,
    /// Chases the side's keeper started from beyond the average keeper's chase distance.
    pub rash_chases: f64,
    /// Presses the side's players started while it trailed, and the ticks it trailed while
    /// the other side had the ball.
    pub presses_behind: f64,
    pub behind_ticks: f64,
    /// Headers the side's outfield players contested with an opponent close, and those won.
    pub headers_traffic: f64,
    pub headers_traffic_won: f64,
    /// Ticks a presser of the side ran at a carrier from beyond the engaging range, and those
    /// on which he stood goal-side of the carrier.
    pub press_ticks: f64,
    pub press_goal_side: f64,
    /// Samples of the side's back-line players out of possession, and the sum of their depth
    /// gaps from their line's mean depth, in metres.
    pub line_gap_n: f64,
    pub line_gap_m: f64,
    /// Recovery runs of the side that drew a sprint.
    pub recovery_sprints: f64,
}

/// The distance in metres short of the second-last defender within which an onside attacker
/// receiving a pass has timed his run.
pub const TIMED_RUN_M: f64 = 2.0;

/// The metres toward goal a pass must gain to count as a flair attempt.
pub const FLAIR_PASS_M: f64 = 15.0;

/// The distance in metres from the ball within which an opponent makes a header one in
/// traffic.
pub const TRAFFIC_M: f64 = 2.0;

/// A set piece whose outcome the probe waits on.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SetPiece {
    Corner,
    FreeKick,
    Penalty,
}

/// An open-play pass the carrier chose this tick, until its kick.
#[derive(Debug, Clone, Copy)]
struct Planned {
    carrier: usize,
    mate: usize,
    kind: PassKind,
    pressed: bool,
    keeper: bool,
}

/// An open-play pass in flight.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Flight {
    pub team: usize,
    pub mate: usize,
    kind: PassKind,
    pressed: bool,
    keeper: bool,
    from: DVec2,
    attack_x: f64,
    /// The tick the ball first came within reach of the intended team-mate.
    pub arrived: Option<u32>,
}

/// An open-play shot until the ball is next controlled or dead.
#[derive(Debug, Clone, Copy)]
struct Shot {
    team: usize,
    in_box: bool,
    near: bool,
    on_target: bool,
}

/// The ticks after a pass reaches its team-mate within which his side must control it.
pub const RECEIPT_TICKS: u32 = 50;

/// The distance in metres from the ball within which an opponent makes a loose ball a short
/// race.
pub const CLOSE_RACE_M: f64 = 5.0;

/// The probe: both sides' counts and the open pass, take-on and shot they wait on.
#[derive(Debug, Clone, Default)]
pub struct JobProbe {
    pub sides: [SideCounts; 2],
    planned: Option<Planned>,
    pub(crate) flight: Option<Flight>,
    shot: Option<Shot>,
    /// The roster bit of the carrier whose last decision was a take-on.
    taking_on: u32,
    /// The set piece taken, by which side, until its outcome.
    set_piece: Option<(SetPiece, usize)>,
    /// Whether each side's keeper chased on the last decision.
    chasing: [bool; 2],
    /// The roster bits of the pressers on the last decision.
    pressing: u32,
}

impl JobProbe {
    /// The carrier `c` of `team` chose a dribble (`dribble`) with an opponent close
    /// (`pressed`): a new take-on unless his last decision was one.
    pub(crate) fn decided(&mut self, c: usize, team: usize, dribble: bool, pressed: bool) {
        let bit = 1u32 << c;
        if dribble && pressed {
            if self.taking_on & bit == 0 {
                self.sides[team].take_ons += 1.0;
            }
            self.taking_on = bit;
        } else {
            self.taking_on = 0;
        }
        self.planned = None;
    }

    /// The carrier `c` chose to pass to `mate`.
    pub(crate) fn planned_pass(
        &mut self,
        c: usize,
        mate: usize,
        kind: PassKind,
        pressed: bool,
        keeper: bool,
    ) {
        self.planned = Some(Planned {
            carrier: c,
            mate,
            kind,
            pressed,
            keeper,
        });
    }

    /// Carrier `c` of `team` kicked an open-play pass from `from` toward the goal at
    /// `attack_x`; a pass he planned this tick becomes the pass in flight.
    pub(crate) fn kicked_pass(&mut self, c: usize, team: usize, from: DVec2, attack_x: f64) {
        self.shot = None;
        self.flight = None;
        let Some(p) = self.planned.take().filter(|p| p.carrier == c) else {
            return;
        };
        let s = &mut self.sides[team];
        if p.keeper {
            match p.kind {
                PassKind::Throw => s.throws += 1.0,
                _ => s.keeper_kicks += 1.0,
            }
        } else {
            match p.kind {
                PassKind::Pass => s.passes += 1.0,
                PassKind::Cross => s.crosses += 1.0,
                PassKind::Chip => s.chips += 1.0,
                PassKind::Throw | PassKind::Kick => {}
            }
            if p.pressed {
                s.pressed_passes += 1.0;
            }
        }
        self.flight = Some(Flight {
            team,
            mate: p.mate,
            kind: p.kind,
            pressed: p.pressed,
            keeper: p.keeper,
            from,
            attack_x,
            arrived: None,
        });
    }

    /// Any other kick: a clearance or a restart kick ends the pass and the shot.
    pub(crate) fn kicked_other(&mut self) {
        self.planned = None;
        self.flight = None;
        self.shot = None;
    }

    /// An open-play shot by `team`: inside the penalty area or not, from within the
    /// one-on-one distance of the keeper or not, with expected goals `xg`.
    pub(crate) fn kicked_shot(&mut self, team: usize, in_box: bool, near: bool, xg: f64) {
        self.planned = None;
        self.flight = None;
        let s = &mut self.sides[team];
        s.shots += 1.0;
        s.xg += xg;
        if in_box {
            s.shots_in_box += 1.0;
        } else {
            s.shots_outside += 1.0;
        }
        self.shot = Some(Shot {
            team,
            in_box,
            near,
            on_target: false,
        });
    }

    /// A penalty kick: no shot statistic counts it.
    pub(crate) fn kicked_penalty(&mut self) {
        self.kicked_other();
    }

    /// `team` took a restart of `kind` (`direct` for a direct free kick): a corner, a direct
    /// free kick and a penalty wait on their outcome.
    pub(crate) fn restart(
        &mut self,
        team: usize,
        kind: crate::data::rules::StoppageKind,
        direct: bool,
    ) {
        use crate::data::rules::StoppageKind as K;
        let s = &mut self.sides[team];
        self.set_piece = match kind {
            K::Corner => {
                s.corners += 1.0;
                Some((SetPiece::Corner, team))
            }
            K::FreeKick if direct => {
                s.free_kicks += 1.0;
                Some((SetPiece::FreeKick, team))
            }
            K::Penalty => {
                s.penalties += 1.0;
                Some((SetPiece::Penalty, team))
            }
            _ => None,
        };
    }

    /// `team` threw the ball in toward a point `distance` metres from the thrower.
    pub(crate) fn throw_in(&mut self, team: usize, distance: f64) {
        let s = &mut self.sides[team];
        s.throws_in += 1.0;
        s.throw_m += distance;
    }

    /// A player of `team` reached the ball first after the set piece: a corner or a direct
    /// free kick reached a team-mate when it is his side's.
    fn reached(&mut self, team: usize) {
        if let Some((kind, by)) = self.set_piece.take()
            && by == team
        {
            match kind {
                SetPiece::Corner => self.sides[team].corners_reached += 1.0,
                SetPiece::FreeKick => self.sides[team].free_kicks_success += 1.0,
                SetPiece::Penalty => {}
            }
        }
    }

    /// A covering defender of `team` stood `distance` metres from the carrier he tracks.
    pub(crate) fn cover(&mut self, team: usize, distance: f64) {
        let s = &mut self.sides[team];
        s.cover_n += 1.0;
        s.cover_m += distance;
    }

    /// One decision while the ball is loose: `chasing` says, per side, whether its keeper
    /// chases from beyond the average keeper's distance (`Some(true)`), from within it
    /// (`Some(false)`), or not at all. A rash chase counts once, when it starts.
    pub(crate) fn keepers(&mut self, chasing: [Option<bool>; 2]) {
        for (team, c) in chasing.into_iter().enumerate() {
            if c == Some(true) && !self.chasing[team] {
                self.sides[team].rash_chases += 1.0;
            }
            self.chasing[team] = c.is_some();
        }
    }

    /// One decision while a carrier of the other side has the ball: the roster bits of
    /// `team`'s pressers, and whether `team` trails. A press counts when it starts while the
    /// side trails.
    pub(crate) fn pressers(&mut self, team: usize, bits: u32, behind: bool) {
        self.chasing = [false; 2];
        if behind {
            let s = &mut self.sides[team];
            s.behind_ticks += 1.0;
            s.presses_behind += f64::from((bits & !self.pressing).count_ones());
        }
        self.pressing = bits;
    }

    /// A presser of `team` ran at the carrier from beyond the engaging range; `goal_side` when
    /// he stood between the carrier and his own goal.
    pub(crate) fn press_sample(&mut self, team: usize, goal_side: bool) {
        let s = &mut self.sides[team];
        s.press_ticks += 1.0;
        if goal_side {
            s.press_goal_side += 1.0;
        }
    }

    /// The depth gaps, in metres, of `team`'s back-line players from their line out of
    /// possession.
    pub(crate) fn line_sample(&mut self, team: usize, gaps: &[f64]) {
        let s = &mut self.sides[team];
        s.line_gap_n += gaps.len() as f64;
        s.line_gap_m += gaps.iter().sum::<f64>();
    }

    /// A recovery run of `team` started and drew a sprint.
    pub(crate) fn recovery_sprint(&mut self, team: usize) {
        self.sides[team].recovery_sprints += 1.0;
    }

    /// An outfield player of `team` contested a header with an opponent close; `won` when he
    /// won it.
    pub(crate) fn header_in_traffic(&mut self, team: usize, won: bool) {
        let s = &mut self.sides[team];
        s.headers_traffic += 1.0;
        if won {
            s.headers_traffic_won += 1.0;
        }
    }

    /// The open-play shot just kicked heads on target.
    pub(crate) fn on_target(&mut self) {
        if let Some((SetPiece::FreeKick, team)) = self.set_piece {
            self.set_piece = None;
            self.sides[team].free_kicks_success += 1.0;
        }
        let Some(shot) = self.shot.as_mut() else {
            return;
        };
        shot.on_target = true;
        let (team, in_box, near) = (shot.team, shot.in_box, shot.near);
        if in_box {
            self.sides[team].on_target_in_box += 1.0;
        } else {
            self.sides[team].on_target_outside += 1.0;
        }
        let keeper = &mut self.sides[1 - team];
        if near {
            keeper.faced_near += 1.0;
        } else {
            keeper.faced_far += 1.0;
        }
    }

    /// The keeper of the side that did not shoot saved the open-play shot in flight; `held`
    /// when he kept the ball.
    pub(crate) fn saved(&mut self, held: bool) {
        let Some(shot) = self.shot.filter(|s| s.on_target) else {
            return;
        };
        let keeper = &mut self.sides[1 - shot.team];
        keeper.saves += 1.0;
        if held {
            keeper.saves_held += 1.0;
        }
        if shot.near {
            keeper.saved_near += 1.0;
        } else {
            keeper.saved_far += 1.0;
        }
    }

    /// `team` scored while its shot was in flight, or from the penalty it took.
    pub(crate) fn scored(&mut self, team: usize) {
        if let Some((SetPiece::Penalty, by)) = self.set_piece.take()
            && by == team
        {
            self.sides[team].penalties_scored += 1.0;
        }
        if let Some(shot) = self.shot.take()
            && shot.team == team
            && shot.on_target
            && shot.in_box
        {
            self.sides[team].goals_on_target_in_box += 1.0;
        }
    }

    /// A tackle attempt by a player of `tackler_team` on a carrier who was running
    /// (`running`) or standing: `won` when it won the ball cleanly, `foul` with the ball kept
    /// or lost by the fouled side.
    pub(crate) fn tackle(
        &mut self,
        tackler_team: usize,
        running: bool,
        won: bool,
        foul: Option<bool>,
    ) {
        let carrier_team = 1 - tackler_team;
        let t = &mut self.sides[tackler_team];
        t.tackles += 1.0;
        if won {
            t.tackles_won += 1.0;
        }
        let c = &mut self.sides[carrier_team];
        if running {
            c.running_tackles_faced += 1.0;
            if !won {
                c.running_tackles_beaten += 1.0;
            }
        } else if won {
            c.standing_tackles_lost += 1.0;
        }
        if let Some(ball_lost) = foul {
            c.fouled += 1.0;
            if !ball_lost {
                c.fouled_kept += 1.0;
            }
        }
    }

    /// An outfield player of `team` reached a high ball first and contested the header;
    /// `won` when he won it.
    pub(crate) fn header(&mut self, team: usize, won: bool) {
        let s = &mut self.sides[team];
        s.headers += 1.0;
        if won {
            s.headers_won += 1.0;
        }
    }

    /// A player of `team` takes a loose ball: `keeper` when he keeps goal, `high` above head
    /// height, `in_own_box` inside his own penalty area, `close` with an opponent outfield
    /// player within [`CLOSE_RACE_M`] of the ball. A pass of his own side arriving is a
    /// completion, not a loose ball.
    pub(crate) fn loose(
        &mut self,
        team: usize,
        keeper: bool,
        high: bool,
        in_own_box: bool,
        close: bool,
    ) {
        if !keeper {
            self.reached(team);
        } else {
            self.set_piece = None;
        }
        let own_pass = self.flight.is_some_and(|f| f.team == team);
        let their_cross = self
            .flight
            .is_some_and(|f| f.team != team && f.kind == PassKind::Cross);
        let s = &mut self.sides[team];
        if keeper {
            if high {
                s.high_claims += 1.0;
            }
            if their_cross {
                s.crosses_claimed += 1.0;
            }
            if !in_own_box {
                s.sweeps += 1.0;
            }
        } else if !own_pass {
            s.loose_won += 1.0;
            if close {
                s.loose_won_close += 1.0;
            }
        }
    }

    /// A player of `team` at `at` controls the ball on `tick`: the pass in flight is
    /// completed when it was his side's, and lost otherwise.
    pub(crate) fn gained(&mut self, team: usize, at: DVec2, tick: u32) {
        self.shot = None;
        self.reached(team);
        let Some(f) = self.flight.take() else {
            return;
        };
        if f.team != team {
            return;
        }
        let s = &mut self.sides[team];
        if f.keeper {
            match f.kind {
                PassKind::Throw => s.throws_completed += 1.0,
                _ => s.keeper_kicks_completed += 1.0,
            }
        } else {
            match f.kind {
                PassKind::Pass => s.passes_completed += 1.0,
                PassKind::Cross => s.crosses_completed += 1.0,
                PassKind::Chip | PassKind::Throw | PassKind::Kick => {}
            }
            if f.pressed {
                s.pressed_completed += 1.0;
            }
            s.progress_passes += 1.0;
            s.progress_m += (at.x - f.from.x) * f.attack_x.signum();
        }
        // A pass controlled as it arrives reached its team-mate on this tick.
        match f.arrived {
            None => {
                s.arrivals += 1.0;
                s.arrivals_kept += 1.0;
            }
            Some(a) if tick.saturating_sub(a) <= RECEIPT_TICKS => s.arrivals_kept += 1.0,
            Some(_) => {}
        }
    }

    /// The pass in flight reached its intended team-mate on `tick`.
    pub(crate) fn arrived(&mut self, tick: u32) {
        if let Some(f) = self.flight.as_mut()
            && f.arrived.is_none()
        {
            f.arrived = Some(tick);
            self.sides[f.team].arrivals += 1.0;
        }
    }
}

impl crate::sim::Simulation {
    /// `true` when an active opponent of player `c` stands within the pressing distance.
    fn probe_pressed(&self, c: usize) -> bool {
        let me = &self.players[c];
        self.players.iter().any(|p| {
            p.team != me.team
                && p.active()
                && (p.pos - me.pos).length() < crate::contract::PRESSED_M
        })
    }

    /// Count site: the carrier `c` decided `choice` with `plan`.
    pub(crate) fn probe_plan(
        &mut self,
        c: usize,
        choice: crate::decision::Choice,
        plan: crate::modules::CarrierPlan,
    ) {
        let team = self.players[c].team;
        let pressed = self.probe_pressed(c);
        let pass = match plan {
            crate::modules::CarrierPlan::Pass { j } => {
                let from = self.players[c].pos;
                let to = self.players[j].pos;
                let keeper = c == self.keeper(team);
                let kind = PassKind::of(
                    from,
                    to,
                    (to - from).length(),
                    self.teams[team].attack_x,
                    keeper,
                    self.config.pitch(),
                );
                Some((j, kind, keeper))
            }
            _ => None,
        };
        let dribble = choice == crate::decision::Choice::Dribble;
        if let Some(p) = self.probe.as_deref_mut() {
            p.decided(c, team, dribble, pressed);
            if let Some((j, kind, keeper)) = pass {
                p.planned_pass(c, j, kind, pressed, keeper);
            }
        }
    }

    /// Count site: carrier `c` of `team` kicked an open-play pass. A pass to a team-mate who
    /// stands onside within [`TIMED_RUN_M`] of the second-last defender is a timed run; a
    /// pass gaining [`FLAIR_PASS_M`] toward goal is a flair attempt.
    pub(crate) fn probe_pass(&mut self, c: usize, team: usize) {
        let (from, attack_x) = (self.ball.xy(), self.teams[team].attack_x);
        let mate = self
            .probe
            .as_deref()
            .and_then(|p| p.planned)
            .filter(|p| p.carrier == c && !p.keeper)
            .map(|p| p.mate);
        let (timed, flair) = mate.map_or((false, false), |j| {
            let at = self.players[j].pos;
            let line = crate::rules::offside::second_last_depth(team, &self.players, attack_x);
            let depth = at.x * attack_x;
            let timed = line.is_finite() && depth <= line && depth >= line - TIMED_RUN_M;
            (timed, (at.x - from.x) * attack_x >= FLAIR_PASS_M)
        });
        if let Some(p) = self.probe.as_deref_mut() {
            p.kicked_pass(c, team, from, attack_x);
            let s = &mut p.sides[team];
            if timed {
                s.timed_runs += 1.0;
            }
            if flair {
                s.flair_passes += 1.0;
            }
        }
    }

    /// Count site: a restart is taken.
    pub(crate) fn probe_restart(&mut self, dead: &crate::rules::DeadBall) {
        if let Some(p) = self.probe.as_deref_mut() {
            p.restart(dead.team, dead.kind, dead.direct);
        }
    }

    /// The depth gaps, in metres, of `team`'s active back-line players (the keeper aside) from
    /// the mean depth of the line; none with fewer than two of them.
    fn line_gaps(&self, team: usize) -> Vec<f64> {
        let first = team * crate::team::PLAYERS_PER_TEAM;
        let keeper = self.keeper(team);
        let depths: Vec<f64> = self.teams[team]
            .back_line()
            .into_iter()
            .map(|slot| first + slot)
            .filter(|&i| i != keeper && self.players[i].active())
            .map(|i| self.players[i].pos.x)
            .collect();
        if depths.len() < 2 {
            return Vec::new();
        }
        let line = depths.iter().sum::<f64>() / depths.len() as f64;
        depths.iter().map(|d| (d - line).abs()).collect()
    }

    /// Count site: the decision pass set the targets and wrote `trace`.
    pub(crate) fn probe_targets(&mut self, trace: crate::modules::TargetsTrace) {
        use crate::modules::{CoverTrace, TargetsTrace};
        match trace {
            TargetsTrace::Carrier {
                def,
                count,
                pressers,
                cover,
                ..
            } => {
                let Some(c) = self.carrier else {
                    return;
                };
                let goals = self.summary.goals;
                let behind = goals[def] < goals[1 - def];
                let bits = pressers[..count]
                    .iter()
                    .filter(|&&(_, i)| i != usize::MAX)
                    .fold(0u32, |b, &(_, i)| b | (1 << i));
                let sample = self.tick.is_multiple_of(10);
                let covered = match cover {
                    CoverTrace::Covered {
                        attacker, defender, ..
                    } if attacker == c && sample => {
                        Some((self.players[defender].pos - self.players[c].pos).length())
                    }
                    _ => None,
                };
                // Each presser beyond the engaging range: goal-side when he stands nearer
                // his own goal than the carrier along the line from the carrier to it.
                let at = self.players[c].pos;
                let to_goal = crate::math::toward(at, self.teams[def].own_goal());
                let engage = self.config.tuning.press_engage;
                let presses: Vec<bool> = pressers[..count]
                    .iter()
                    .filter(|&&(_, i)| {
                        i != usize::MAX && (self.players[i].pos - at).length() >= engage
                    })
                    .map(|&(_, i)| (self.players[i].pos - at).dot(to_goal) > 0.0)
                    .collect();
                let gaps = if sample {
                    self.line_gaps(def)
                } else {
                    Vec::new()
                };
                if let Some(p) = self.probe.as_deref_mut() {
                    p.pressers(def, bits, behind);
                    if let Some(d) = covered {
                        p.cover(def, d);
                    }
                    for goal_side in presses {
                        p.press_sample(def, goal_side);
                    }
                    if !gaps.is_empty() {
                        p.line_sample(def, &gaps);
                    }
                }
            }
            TargetsTrace::Loose { keepers, .. } => {
                let ball = self.ball.xy();
                let chasing = keepers.map(|k| {
                    k.map(|gk| crate::decision::rash_chase((self.players[gk].pos - ball).length()))
                });
                if let Some(p) = self.probe.as_deref_mut() {
                    p.keepers(chasing);
                }
            }
        }
    }

    /// Count site: player `i` of `team` contested a header, `won` or not; it counts as one in
    /// traffic when an opponent outfield player stands within [`TRAFFIC_M`] of the ball.
    pub(crate) fn probe_header(&mut self, i: usize, won: bool) {
        let team = self.players[i].team;
        let ball = self.ball.xy();
        let keeper = self.keeper(1 - team);
        let traffic = self.players.iter().enumerate().any(|(j, q)| {
            q.team != team && j != keeper && q.active() && (q.pos - ball).length() <= TRAFFIC_M
        });
        if let Some(p) = self.probe.as_deref_mut() {
            p.header(team, won);
            if traffic {
                p.header_in_traffic(team, won);
            }
        }
    }

    /// Count site: carrier `c` of `team` shot from `from` at the goal at `attack_x`.
    pub(crate) fn probe_shot(
        &mut self,
        c: usize,
        team: usize,
        from: DVec2,
        attack_x: f64,
        penalty: bool,
        xg: f64,
    ) {
        if penalty {
            if let Some(p) = self.probe.as_deref_mut() {
                p.kicked_penalty();
            }
            return;
        }
        let in_box = self.config.pitch().in_penalty_area(from, attack_x);
        let keeper = self.players[self.keeper(1 - team)].pos;
        let near = (self.players[c].pos - keeper).length() <= crate::contract::ONE_ON_ONE_M;
        if let Some(p) = self.probe.as_deref_mut() {
            p.kicked_shot(team, in_box, near, xg);
        }
    }

    /// Count site: player `i` takes the loose ball.
    pub(crate) fn probe_loose(&mut self, i: usize) {
        let me = &self.players[i];
        let team = me.team;
        let keeper = i == self.keeper(team);
        let ball = self.ball.xy();
        let high = self.ball.pos.z > crate::contract::HEAD_HEIGHT_M;
        let own_goal_x = -self.teams[team].attack_x;
        let in_own_box = self.config.pitch().in_penalty_area(ball, own_goal_x);
        let their_keeper = self.keeper(1 - team);
        let close = self.players.iter().enumerate().any(|(j, p)| {
            p.team != team
                && j != their_keeper
                && p.active()
                && (p.pos - ball).length() <= CLOSE_RACE_M
        });
        if let Some(p) = self.probe.as_deref_mut() {
            p.loose(team, keeper, high, in_own_box, close);
        }
    }

    /// Count site: player `i` tackled carrier `c` with `outcome`.
    pub(crate) fn probe_tackle(
        &mut self,
        i: usize,
        c: usize,
        outcome: crate::rules::fouls::Tackle,
    ) {
        use crate::rules::fouls::Tackle;
        let team = self.players[i].team;
        let running = self.players[c].vel.length() > crate::rules::fouls::RUNNING_SPEED;
        let (won, foul) = match outcome {
            Tackle::Win => (true, None),
            Tackle::Foul { ball_lost } => (false, Some(ball_lost)),
            Tackle::Miss => (false, None),
        };
        if let Some(p) = self.probe.as_deref_mut() {
            p.tackle(team, running, won, foul);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_planned_pass_counts_at_its_kick_and_completes_on_control() {
        let mut p = JobProbe::default();
        p.planned_pass(3, 5, PassKind::Pass, true, false);
        p.kicked_pass(3, 0, DVec2::new(0.0, 0.0), 52.5);
        p.gained(0, DVec2::new(12.0, 3.0), 30);
        let s = p.sides[0];
        assert_eq!((s.passes, s.passes_completed), (1.0, 1.0));
        assert_eq!((s.pressed_passes, s.pressed_completed), (1.0, 1.0));
        assert_eq!((s.arrivals, s.arrivals_kept), (1.0, 1.0));
        assert_eq!(s.progress_m, 12.0);
    }

    #[test]
    fn an_intercepted_pass_is_not_completed_and_a_late_control_is_not_kept() {
        let mut p = JobProbe::default();
        p.planned_pass(3, 5, PassKind::Cross, false, false);
        p.kicked_pass(3, 0, DVec2::ZERO, -52.5);
        p.loose(1, true, true, true, false);
        p.gained(1, DVec2::ZERO, 5);
        assert_eq!(p.sides[0].crosses, 1.0);
        assert_eq!(p.sides[0].crosses_completed, 0.0);
        assert_eq!(p.sides[1].crosses_claimed, 1.0);
        assert_eq!(p.sides[1].high_claims, 1.0);
        let mut p = JobProbe::default();
        p.planned_pass(3, 5, PassKind::Pass, false, false);
        p.kicked_pass(3, 0, DVec2::ZERO, 52.5);
        p.arrived(10);
        p.gained(0, DVec2::ZERO, 10 + RECEIPT_TICKS + 1);
        assert_eq!((p.sides[0].arrivals, p.sides[0].arrivals_kept), (1.0, 0.0));
    }

    #[test]
    fn a_take_on_counts_once_while_the_carrier_keeps_dribbling() {
        let mut p = JobProbe::default();
        for _ in 0..5 {
            p.decided(4, 0, true, true);
        }
        p.decided(4, 0, true, false);
        p.decided(4, 0, true, true);
        assert_eq!(p.sides[0].take_ons, 2.0);
    }

    #[test]
    fn a_shot_counts_by_zone_and_the_keeper_faces_it() {
        let mut p = JobProbe::default();
        p.kicked_shot(0, true, true, 0.3);
        p.on_target();
        p.saved(true);
        p.kicked_shot(0, true, false, 0.1);
        p.on_target();
        p.scored(0);
        let (a, k) = (p.sides[0], p.sides[1]);
        assert_eq!(
            (a.shots, a.shots_in_box, a.on_target_in_box),
            (2.0, 2.0, 2.0)
        );
        assert_eq!(a.goals_on_target_in_box, 1.0);
        assert_eq!(
            (k.faced_near, k.saved_near, k.faced_far, k.saved_far),
            (1.0, 1.0, 1.0, 0.0)
        );
        assert_eq!((k.saves, k.saves_held), (1.0, 1.0));
    }

    #[test]
    fn a_corner_counts_when_a_team_mate_reaches_it_and_a_penalty_when_it_is_scored() {
        use crate::data::rules::StoppageKind as K;
        let mut p = JobProbe::default();
        p.restart(0, K::Corner, false);
        p.loose(0, false, true, false, true);
        p.restart(0, K::Corner, false);
        p.loose(1, true, true, true, false);
        assert_eq!((p.sides[0].corners, p.sides[0].corners_reached), (2.0, 1.0));
        p.restart(1, K::FreeKick, true);
        p.kicked_shot(1, false, false, 0.05);
        p.on_target();
        assert_eq!(
            (p.sides[1].free_kicks, p.sides[1].free_kicks_success),
            (1.0, 1.0)
        );
        p.restart(0, K::Penalty, true);
        p.kicked_penalty();
        p.scored(0);
        assert_eq!(
            (p.sides[0].penalties, p.sides[0].penalties_scored),
            (1.0, 1.0)
        );
        p.restart(0, K::FreeKick, false);
        assert_eq!(
            p.sides[0].free_kicks, 0.0,
            "an indirect free kick is not counted"
        );
    }

    #[test]
    fn a_rash_chase_and_a_press_while_behind_count_once_as_they_start() {
        let mut p = JobProbe::default();
        p.keepers([Some(true), None]);
        p.keepers([Some(true), Some(false)]);
        assert_eq!((p.sides[0].rash_chases, p.sides[1].rash_chases), (1.0, 0.0));
        p.pressers(1, 0b11, true);
        p.pressers(1, 0b110, true);
        p.pressers(1, 0b110, false);
        assert_eq!(
            (p.sides[1].presses_behind, p.sides[1].behind_ticks),
            (3.0, 2.0)
        );
    }

    #[test]
    fn the_defensive_job_counters_count_for_their_side() {
        let mut p = JobProbe::default();
        p.press_sample(1, true);
        p.press_sample(1, false);
        p.press_sample(1, true);
        p.line_sample(1, &[0.5, 1.5, 2.0]);
        p.line_sample(1, &[1.0]);
        p.recovery_sprint(1);
        p.recovery_sprint(0);
        let (a, b) = (p.sides[0], p.sides[1]);
        assert_eq!((b.press_ticks, b.press_goal_side), (3.0, 2.0));
        assert_eq!((b.line_gap_n, b.line_gap_m), (4.0, 5.0));
        assert_eq!((a.recovery_sprints, b.recovery_sprints), (1.0, 1.0));
        assert_eq!((a.press_ticks, a.line_gap_n), (0.0, 0.0));
    }

    #[test]
    fn a_tackle_counts_for_both_sides() {
        let mut p = JobProbe::default();
        p.tackle(1, true, false, None);
        p.tackle(1, false, true, None);
        p.tackle(1, false, false, Some(false));
        let (a, b) = (p.sides[0], p.sides[1]);
        assert_eq!((b.tackles, b.tackles_won), (3.0, 1.0));
        assert_eq!(
            (a.running_tackles_faced, a.running_tackles_beaten),
            (1.0, 1.0)
        );
        assert_eq!(a.standing_tackles_lost, 1.0);
        assert_eq!((a.fouled, a.fouled_kept), (1.0, 1.0));
    }
}

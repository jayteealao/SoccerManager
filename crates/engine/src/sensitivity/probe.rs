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

    /// The open-play shot just kicked heads on target.
    pub(crate) fn on_target(&mut self) {
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

    /// `team` scored while its shot was in flight.
    pub(crate) fn scored(&mut self, team: usize) {
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

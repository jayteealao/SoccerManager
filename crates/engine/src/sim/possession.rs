//! The ball and possession passes of the central loop.

use serde_json::json;

use crate::math::{DVec2, DVec3, toward};
use crate::pitch;
use crate::rules::fouls::Tackle;
use crate::shot;
use crate::sim::{Simulation, WIDE_SPREAD, wide_of_goal};
use crate::streams::{Action, Key};
use crate::trace::Point;
use crate::tuning::Tuning;

impl Simulation {
    /// While a shot is in flight and fast, an outfield defender near the ball may block it,
    /// and the keeper may save a shot heading on target. Nobody else touches it. Returns
    /// `false` when no shot is in flight or the ball is slow, and the ordinary contest
    /// applies.
    fn contest_shot(&mut self, t: &Tuning) -> bool {
        let Some(shooter) = self.shot_in_flight else {
            return false;
        };
        if self.ball.speed() <= t.control_speed {
            return false;
        }
        if self.try_block(shooter, t) {
            return true;
        }
        if self.shot_on_target {
            self.try_save(shooter, t);
        }
        true
    }

    /// Each outfield defender within `block_reach` of a shot under `reach_height` rolls once
    /// per shot to block it. A block deflects the ball back the way it came, with the
    /// blocker's side as the last touch, and ends the shot.
    fn try_block(&mut self, shooter: usize, t: &Tuning) -> bool {
        if self.ball.pos.z > t.reach_height {
            return false;
        }
        let ball_xy = self.ball.xy();
        let keeper = self.keeper(1 - shooter);
        for i in 0..self.players.len() {
            let p = self.players[i];
            let bit = 1u32 << i;
            if p.team == shooter
                || i == keeper
                || !p.active()
                || self.blockers_tried & bit != 0
                || (p.pos - ball_xy).length() >= t.shots.block_reach
            {
                continue;
            }
            self.blockers_tried |= bit;
            let blocked = self
                .streams
                .tested(Key::player(Action::Block, &p), &[t.shots.block_chance])
                < t.shots.block_chance;
            if self.trace_on() {
                self.trace_point(Point::ShotBlock, json!({"blocker": i, "blocked": blocked}));
            }
            if blocked {
                let s = &t.shots;
                let back = -DVec2::new(self.ball.vel.x, self.ball.vel.y);
                let angle = self.streams.draw(Key::player(Action::BlockDeflect, &p));
                self.ball.vel = shot::deflect(
                    self.ball.vel,
                    back,
                    s.block_speed,
                    s.block_spread,
                    0.0,
                    angle,
                    0.0,
                );
                self.deflected_by(i);
                self.keeper_beaten = false;
                #[cfg(feature = "scenario")]
                {
                    self.census.blocked += 1;
                }
                return true;
            }
        }
        false
    }

    /// The acting keeper within `keeper_reach` of a shot on target under the bar rolls once
    /// per shot to save it, with a chance that falls with the shot's quality. A save is held
    /// with `save_hold`; otherwise it is parried.
    fn try_save(&mut self, shooter: usize, t: &Tuning) {
        let k = self.keeper(1 - shooter);
        if self.keeper_beaten
            || self.ball.pos.z >= t.crossbar_height
            || !self.players[k].active()
            || (self.players[k].pos - self.ball.xy()).length() >= t.keeper_reach
        {
            return;
        }
        let keeper = self.players[k];
        let save = self
            .config
            .modules
            .shot
            .save_chance(&self.view(), self.shot_quality);
        if self
            .streams
            .tested(Key::player(Action::Save, &keeper), &[save])
            >= save
        {
            self.trace_save(Point::ShotSave, k, "beaten");
            self.keeper_beaten = true;
            return;
        }
        if self
            .streams
            .tested(Key::player(Action::SaveHold, &keeper), &[t.shots.save_hold])
            < t.shots.save_hold
        {
            #[cfg(feature = "scenario")]
            {
                self.census.held += 1;
            }
            self.trace_save(Point::ShotSave, k, "held");
            self.gain(k, t);
            return;
        }
        self.trace_save(Point::ShotSave, k, "parried");
        self.parry(k, t);
        #[cfg(feature = "scenario")]
        {
            self.census.parried += 1;
        }
    }

    /// Records a keeper's save roll at `point`: `beaten`, `held`, `parried`, or, in the
    /// shoot-out, `reached` for a slow ball he picks up.
    pub(crate) fn trace_save(&mut self, point: Point, keeper: usize, outcome: &str) {
        if self.trace_on() {
            self.trace_point(point, json!({"keeper": keeper, "outcome": outcome}));
        }
    }

    /// Keeper `k` parries the ball: it keeps `parry_speed` of its speed and goes along the
    /// goal line away from the goal centre, turned by up to `parry_spread` either way, with
    /// a loft of up to `parry_loft`. The keeper gets no second touch of this flight.
    pub(crate) fn parry(&mut self, k: usize, t: &Tuning) {
        let s = &t.shots;
        let keeper = self.players[k];
        let side = if self.ball.pos.y == 0.0 {
            if self
                .streams
                .tested(Key::player(Action::ParrySide, &keeper), &[0.5])
                < 0.5
            {
                1.0
            } else {
                -1.0
            }
        } else {
            self.ball.pos.y.signum()
        };
        let angle = self.streams.draw(Key::player(Action::ParryAngle, &keeper));
        let loft = self.streams.draw(Key::player(Action::ParryLoft, &keeper));
        self.ball.vel = shot::deflect(
            self.ball.vel,
            DVec2::new(0.0, side),
            s.parry_speed,
            s.parry_spread,
            s.parry_loft,
            angle,
            loft,
        );
        self.deflected_by(k);
        self.keeper_beaten = true;
    }

    /// While an open-play pass is in flight, fast, under `reach_height` and inside the
    /// penalty area of the side that did not play it, each active defending outfield player
    /// within `cross_reach` rolls once per flight to clear it. A clearance deflects the ball
    /// away from his own goal centre, or wide toward his own goal line with `wide_chance`,
    /// with his side as the last touch; the pass is not completed. Returns `true` when the
    /// ball was cleared.
    fn try_clear_cross(&mut self, t: &Tuning) -> bool {
        let Some(passer) = self.pass_in_flight else {
            return false;
        };
        let c = &t.clearances;
        if c.cross_chance <= 0.0
            || self.ball.speed() <= t.control_speed
            || self.ball.pos.z > t.reach_height
        {
            return false;
        }
        let def = 1 - passer;
        let own_goal_x = -self.teams[def].attack_x;
        let ball_xy = self.ball.xy();
        if !pitch::in_penalty_area(ball_xy, own_goal_x) {
            return false;
        }
        let keeper = self.keeper(def);
        for i in 0..self.players.len() {
            let p = self.players[i];
            let bit = 1u32 << i;
            if p.team != def
                || i == keeper
                || !p.active()
                || self.clearers_tried & bit != 0
                || (p.pos - ball_xy).length() >= c.cross_reach
            {
                continue;
            }
            self.clearers_tried |= bit;
            let cleared = self
                .streams
                .tested(Key::player(Action::CrossClear, &p), &[c.cross_chance])
                < c.cross_chance;
            if !cleared && self.trace_on() {
                self.trace_point(Point::CrossClear, json!({"defender": i, "cleared": false}));
            }
            if cleared {
                let wide = c.wide_chance > 0.0
                    && self
                        .streams
                        .tested(Key::player(Action::CrossWide, &p), &[c.wide_chance])
                        < c.wide_chance;
                if self.trace_on() {
                    self.trace_point(
                        Point::CrossClear,
                        json!({"defender": i, "cleared": true, "wide": wide}),
                    );
                }
                let (away, spread) = if wide {
                    (wide_of_goal(ball_xy, own_goal_x), WIDE_SPREAD)
                } else {
                    let goal = pitch::goal_centre(own_goal_x);
                    let away = match toward(goal, ball_xy) {
                        v if v == DVec2::ZERO => DVec2::new(-own_goal_x.signum(), 0.0),
                        v => v,
                    };
                    (away, c.cross_spread)
                };
                let angle = self.streams.draw(Key::player(Action::CrossAngle, &p));
                let loft = self.streams.draw(Key::player(Action::CrossLoft, &p));
                self.ball.vel = shot::deflect(
                    self.ball.vel,
                    away,
                    c.cross_speed,
                    spread,
                    c.cross_loft,
                    angle,
                    loft,
                );
                self.deflected_by(i);
                self.pass_in_flight = None;
                self.clearers_tried = 0;
                self.keeper_beaten = false;
                self.summary.clearances[def] += 1;
                return true;
            }
        }
        false
    }

    /// Player `i` deflected the shot in flight: his side touched the ball last and the shot
    /// is over.
    fn deflected_by(&mut self, i: usize) {
        if self.ball.vel.z > 0.0 && self.ball.pos.z <= 0.0 {
            self.ball.pos.z = 0.001;
        }
        let team = self.players[i].team;
        self.last_touch = Some(team);
        self.last_kicker = Some(i);
        self.end_shot();
    }

    pub(crate) fn move_ball(&mut self, t: &Tuning) {
        match self.carrier {
            Some(c) => {
                let p = &self.players[c];
                let at = pitch::clamp(p.pos + p.facing * 0.5, 0.1);
                let step = crate::math::clamp_len(at - self.ball.xy(), t.carry_step);
                let next = self.ball.xy() + step;
                self.ball.pos = DVec3::new(next.x, next.y, 0.0);
                self.ball.vel = DVec3::new(p.vel.x, p.vel.y, 0.0);
            }
            None => {
                let prev = self.ball.xy();
                self.ball.integrate(t);
                let xy = self.ball.xy();
                if self.referee.shootout.is_some() {
                    self.shootout_ball(prev, xy, t);
                    return;
                }
                for team in 0..2 {
                    if pitch::in_goal(prev, xy, self.teams[team].attack_x)
                        && self.ball.pos.z < t.crossbar_height
                    {
                        self.goal(team);
                        return;
                    }
                }
                if let Some(exit) = pitch::exit(prev, xy) {
                    self.ball_out(exit);
                    return;
                }
            }
        }
        let speed = self.ball.speed();
        if speed > self.summary.ball_max_speed {
            self.summary.ball_max_speed = speed;
        }
        if speed == 0.0 && self.carrier.is_none() {
            self.summary.ball_idle_ticks += 1;
        }
    }

    pub(crate) fn resolve_possession(&mut self, t: &Tuning) {
        if self.referee.shootout.is_some() {
            self.shootout_save(t);
            return;
        }
        let ball_xy = self.ball.xy();
        match self.carrier {
            None => {
                if self.contest_shot(t) {
                    return;
                }
                if self.try_clear_cross(t) {
                    return;
                }
                if self.ball.pos.z > t.reach_height {
                    return;
                }
                let fast = self.ball.speed() > t.control_speed;
                let keepers = [self.keeper(0), self.keeper(1)];
                let mut best: Option<(f64, usize)> = None;
                for (i, p) in self.players.iter().enumerate() {
                    let keeper = i == keepers[p.team];
                    if !p.active() || (fast && !keeper) {
                        continue;
                    }
                    let reach = if keeper {
                        t.keeper_reach
                    } else {
                        t.reach_radius
                    };
                    let d = (p.pos - ball_xy).length();
                    if d < reach && best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, i));
                    }
                }
                if self.trace_on() {
                    self.trace_point(
                        Point::LooseBall,
                        json!({
                            "player": best.map(|(_, i)| i),
                            "distance": best.map(|(d, _)| d),
                            "fast": fast,
                            "keeper_beaten": self.keeper_beaten,
                        }),
                    );
                }
                if let Some((_, i)) = best {
                    if fast {
                        if self.keeper_beaten {
                            return;
                        }
                        let catcher = self.players[i];
                        let caught = self.streams.chance(
                            Key::player(Action::KeeperCatch, &catcher),
                            t.keeper_catch_chance,
                        );
                        if self.trace_on() {
                            self.trace_point(
                                Point::KeeperCatch,
                                json!({"keeper": i, "caught": caught}),
                            );
                        }
                        if !caught {
                            self.keeper_beaten = true;
                            return;
                        }
                    }
                    self.gain(i, t);
                }
            }
            Some(c) => {
                if self.tick.saturating_sub(self.control_since) < t.control_cooldown_ticks {
                    return;
                }
                let carrier = self.players[c];
                for i in 0..self.players.len() {
                    let p = self.players[i];
                    if !p.active()
                        || p.team == carrier.team
                        || self.tick < p.foul_ready
                        || (p.pos - ball_xy).length() > t.tackle_reach
                    {
                        continue;
                    }
                    let fouls = self.config.modules.fouls;
                    let chances = fouls.tackle_chances(&self.view(), i, c);
                    let (p_win, p_foul) = (chances.p_win, chances.p_foul);
                    let draw = self
                        .streams
                        .tested(Key::player(Action::Tackle, &p), &[p_win, p_win + p_foul]);
                    let outcome = fouls.tackle_outcome(&chances, draw);
                    if self.trace_on() {
                        let label = match outcome {
                            Tackle::Win => "win",
                            Tackle::Foul { ball_lost: true } => "foul_ball_lost",
                            Tackle::Foul { ball_lost: false } => "foul_ball_kept",
                            Tackle::Miss => "miss",
                        };
                        self.trace_point(
                            Point::Tackle,
                            json!({
                                "tackler": i,
                                "carrier": c,
                                "p_win": p_win,
                                "p_foul": p_foul,
                                "outcome": label,
                            }),
                        );
                    }
                    match outcome {
                        Tackle::Win => {
                            self.gain(i, t);
                            self.tackle_injury_roll(c);
                            return;
                        }
                        Tackle::Foul { ball_lost } => {
                            self.foul(i, c, ball_lost, t);
                            self.tackle_injury_roll(c);
                            return;
                        }
                        Tackle::Miss => {}
                    }
                }
            }
        }
    }

    /// Player `i` touches the ball first. A player in an offside position is penalised
    /// instead of gaining the ball.
    fn gain(&mut self, i: usize, _t: &Tuning) {
        if self
            .config
            .modules
            .offside
            .is_offence(&self.view(), self.referee.offside, i)
        {
            self.offside_offence(i);
            return;
        }
        self.referee.offside = 0;
        self.keeper_beaten = false;
        let team = self.players[i].team;
        if self.pass_in_flight.take() == Some(team) {
            self.summary.passes_completed[team] += 1;
        }
        self.clearers_tried = 0;
        if self.restart_taker != Some(i) {
            self.restart_taker = None;
        }
        self.end_shot();
        if self.last_touch != Some(team) {
            self.summary.possession_changes += 1;
        }
        self.last_touch = Some(team);
        self.carrier = Some(i);
        self.control_since = self.tick;
        self.script_cache = None;
    }
}

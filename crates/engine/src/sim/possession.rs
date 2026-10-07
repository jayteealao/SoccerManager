//! The ball and possession passes of the central loop. The ball module moves the ball and the
//! possession module says who may contest it; this pass takes every draw on their keys in the
//! order the engine always drew them, and writes every outcome.

use serde_json::json;

use crate::modules::{Crossing, Deflection, ParrySide};
use crate::rules::fouls::Tackle;
use crate::sim::Simulation;
use crate::streams::{Action, Key};
use crate::trace::Point;
use crate::tuning::Tuning;

impl Simulation {
    /// While a shot is in flight and fast, an outfield defender near the ball may block it,
    /// and the keeper may save a shot heading on target. Nobody else touches it. Returns
    /// `false` when no shot is in flight or the ball is slow, and the ordinary contest
    /// applies.
    fn contest_shot(&mut self, t: &Tuning) -> bool {
        let Some(shooter) = self.config.modules.possession.shot_contest(&self.view()) else {
            return false;
        };
        if self.try_block(shooter) {
            return true;
        }
        if self.shot_on_target {
            self.try_save(shooter, t);
        }
        true
    }

    /// Each defender the possession module names rolls once per shot to block it. A block
    /// deflects the ball back the way it came, with the blocker's side as the last touch, and
    /// ends the shot.
    fn try_block(&mut self, shooter: usize) -> bool {
        let contest = self
            .config
            .modules
            .possession
            .blockers(&self.view(), shooter);
        let mut mask = contest.mask;
        while mask != 0 {
            let i = mask.trailing_zeros() as usize;
            mask &= mask - 1;
            let p = crate::streams::PlayerKey::of(&self.players[i]);
            self.blockers_tried |= 1u32 << i;
            let blocked = self
                .streams
                .tested(Key::player(Action::Block, p), &[contest.chance])
                < contest.chance;
            if self.trace_on() {
                self.trace_point(Point::ShotBlock, json!({"blocker": i, "blocked": blocked}));
            }
            if blocked {
                let angle = self.streams.draw(Key::player(Action::BlockDeflect, p));
                self.ball.vel =
                    self.config
                        .modules
                        .ball
                        .deflect(&self.view(), Deflection::Block, angle, 0.0);
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

    /// The keeper the possession module names rolls once per shot to save it, with a chance
    /// that falls with the shot's quality. A save is held below the save-hold threshold;
    /// otherwise it is parried.
    fn try_save(&mut self, shooter: usize, t: &Tuning) {
        let possession = self.config.modules.possession;
        let Some(k) = possession.save_reach(&self.view(), shooter) else {
            return;
        };
        let keeper = crate::streams::PlayerKey::of(&self.players[k]);
        let save = self.config.modules.shot.save_chance(
            &self.view(),
            self.shot_quality,
            k,
            self.last_kicker,
        );
        if self
            .streams
            .tested(Key::player(Action::Save, keeper), &[save])
            >= save
        {
            self.trace_save(Point::ShotSave, k, "beaten");
            self.keeper_beaten = true;
            return;
        }
        let hold = possession.save_hold(&self.view(), k);
        if self
            .streams
            .tested(Key::player(Action::SaveHold, keeper), &[hold])
            < hold
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

    /// Keeper `k` parries the ball to the side the possession module gives, turned by the
    /// ball module. The keeper gets no second touch of this flight.
    pub(crate) fn parry(&mut self, k: usize, _t: &Tuning) {
        let possession = self.config.modules.possession;
        let keeper = crate::streams::PlayerKey::of(&self.players[k]);
        let side = match possession.parry_side(&self.view(), k) {
            ParrySide::Fixed(side) => side,
            ParrySide::Draw(threshold) => {
                let draw = self
                    .streams
                    .tested(Key::player(Action::ParrySide, keeper), &[threshold]);
                possession.parry_side_from_draw(draw, threshold)
            }
        };
        let angle = self.streams.draw(Key::player(Action::ParryAngle, keeper));
        let loft = self.streams.draw(Key::player(Action::ParryLoft, keeper));
        self.ball.vel =
            self.config
                .modules
                .ball
                .deflect(&self.view(), Deflection::Parry { side }, angle, loft);
        self.deflected_by(k);
        self.keeper_beaten = true;
    }

    /// While an open-play pass is in flight, each defender the possession module names rolls
    /// once per flight to clear it. A clearance deflects the ball along the line the module
    /// gives, with his side as the last touch; the pass is not completed. Returns `true` when
    /// the ball was cleared.
    fn try_clear_cross(&mut self) -> bool {
        let possession = self.config.modules.possession;
        let Some(cross) = possession.cross_clearers(&self.view()) else {
            return false;
        };
        let chance = cross.contest.chance;
        let mut mask = cross.contest.mask;
        while mask != 0 {
            let i = mask.trailing_zeros() as usize;
            mask &= mask - 1;
            let p = crate::streams::PlayerKey::of(&self.players[i]);
            self.clearers_tried |= 1u32 << i;
            let cleared = self
                .streams
                .tested(Key::player(Action::CrossClear, p), &[chance])
                < chance;
            if !cleared && self.trace_on() {
                self.trace_point(Point::CrossClear, json!({"defender": i, "cleared": false}));
            }
            if cleared {
                let wide = cross.wide_chance.is_some_and(|w| {
                    self.streams.tested(Key::player(Action::CrossWide, p), &[w]) < w
                });
                if self.trace_on() {
                    self.trace_point(
                        Point::CrossClear,
                        json!({"defender": i, "cleared": true, "wide": wide}),
                    );
                }
                let (away, spread) = possession.clearance_line(&self.view(), i, wide);
                let angle = self.streams.draw(Key::player(Action::CrossAngle, p));
                let loft = self.streams.draw(Key::player(Action::CrossLoft, p));
                self.ball.vel = self.config.modules.ball.deflect(
                    &self.view(),
                    Deflection::Clear { away, spread },
                    angle,
                    loft,
                );
                self.deflected_by(i);
                self.pass_in_flight = None;
                self.clearers_tried = 0;
                self.keeper_beaten = false;
                self.summary.clearances[cross.team] += 1;
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

    /// The ball pass: the ball module carries the ball at the carrier's feet, or moves it one
    /// tick in flight and says which line it crossed.
    pub(crate) fn move_ball(&mut self, t: &Tuning) {
        let ball = self.config.modules.ball;
        match self.carrier {
            Some(c) => {
                self.ball = ball.carry(&self.view(), c);
            }
            None => {
                let prev = self.ball.xy();
                self.ball = ball.integrate(&self.view(), self.ball);
                let xy = self.ball.xy();
                if self.referee.shootout.is_some() {
                    self.shootout_ball(prev, xy, t);
                    return;
                }
                match ball.crossing(&self.view(), prev, &self.ball) {
                    Crossing::Goal(team) => {
                        self.goal(team);
                        return;
                    }
                    Crossing::Out(exit) => {
                        self.ball_out(exit);
                        return;
                    }
                    Crossing::None => {}
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

    /// The possession pass: a contested shot, a cleared cross, a loose ball, or a tackle on
    /// the carrier, each as the possession module allows.
    pub(crate) fn resolve_possession(&mut self, t: &Tuning) {
        if self.referee.shootout.is_some() {
            self.shootout_save(t);
            return;
        }
        let possession = self.config.modules.possession;
        match self.carrier {
            None => {
                if self.contest_shot(t) {
                    return;
                }
                if self.try_clear_cross() {
                    return;
                }
                let Some(loose) = possession.loose_ball(&self.view()) else {
                    return;
                };
                let (best, fast) = (loose.best, loose.fast);
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
                    if let Some(chance) = loose.header {
                        let p = crate::streams::PlayerKey::of(&self.players[i]);
                        let draw = self
                            .streams
                            .tested(Key::player(Action::Header, p), &[chance]);
                        let won = draw < chance;
                        if self.trace_on() {
                            self.trace_point(
                                Point::LooseBall,
                                json!({"header": i, "chance": chance, "won": won}),
                            );
                        }
                        if !won {
                            // The part of the draw above the chance, spread over [0, 1),
                            // sets the angle the ball glances off at.
                            let angle =
                                ((draw - chance) / (1.0 - chance)).clamp(0.0, 1.0 - f64::EPSILON);
                            self.ball.vel = self.config.modules.ball.deflect(
                                &self.view(),
                                Deflection::Block,
                                angle,
                                0.0,
                            );
                            self.deflected_by(i);
                            return;
                        }
                    }
                    if fast {
                        if self.keeper_beaten {
                            return;
                        }
                        let catcher = crate::streams::PlayerKey::of(&self.players[i]);
                        let caught = self.streams.chance(
                            Key::player(Action::KeeperCatch, catcher),
                            loose.catch_chance,
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
                let Some(mut mask) = possession.tacklers(&self.view(), c) else {
                    return;
                };
                while mask != 0 {
                    let i = mask.trailing_zeros() as usize;
                    mask &= mask - 1;
                    let p = crate::streams::PlayerKey::of(&self.players[i]);
                    let fouls = self.config.modules.fouls;
                    let chances = fouls.tackle_chances(&self.view(), i, c);
                    let (p_win, p_foul) = (chances.p_win, chances.p_foul);
                    let draw = self
                        .streams
                        .tested(Key::player(Action::Tackle, p), &[p_win, p_win + p_foul]);
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
    pub(crate) fn gain(&mut self, i: usize, _t: &Tuning) {
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

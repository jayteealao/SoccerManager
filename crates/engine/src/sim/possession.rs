//! The ball and possession passes of the central loop. The ball module moves the ball and the
//! possession module says who may contest it; this pass takes every draw on their keys in the
//! order the engine always drew them, and writes every outcome.

use serde_json::json;

use crate::modules::{Crossing, Deflection, ParrySide};
use crate::rules::fouls::Tackle;
use crate::sim::Simulation;
use crate::sim::tally::Count;
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
                self.tally(i, Count::Block);
                let at = self.ball.xy();
                self.value_won(i, at);
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
        self.tally(k, Count::Save);
        let hold = possession.save_hold(&self.view(), k);
        let held = self
            .streams
            .tested(Key::player(Action::SaveHold, keeper), &[hold])
            < hold;
        #[cfg(feature = "sensitivity")]
        if let Some(p) = self.probe.as_deref_mut() {
            p.saved(held);
        }
        if held {
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
                self.cutters_tried = 0;
                self.duelled = 0;
                self.keeper_beaten = false;
                self.summary.clearances[cross.team] += 1;
                self.tally(i, Count::Clearance);
                let at = self.ball.xy();
                self.value_won(i, at);
                return true;
            }
        }
        false
    }

    /// While a pass is in flight, each defender the possession module names rolls once per
    /// flight, as the ball passes him, to cut it out: a ball he can control he takes (an
    /// interception); a faster one he blocks back the way it came, with his side as the last
    /// touch, and the pass is not completed. Returns `true` when a defender won the ball or
    /// blocked it.
    fn try_cut_pass(&mut self, t: &Tuning) -> bool {
        let Some(passing) = self.pass_in_flight else {
            return false;
        };
        let possession = self.config.modules.possession;
        for i in 0..self.players.len() {
            if self.players[i].team == passing {
                continue;
            }
            let Some(cut) = possession.pass_cut(&self.view(), i) else {
                continue;
            };
            let p = crate::streams::PlayerKey::of(&self.players[i]);
            self.cutters_tried |= 1u32 << i;
            let won = self
                .streams
                .tested(Key::player(Action::PassCut, p), &[cut.chance])
                < cut.chance;
            if self.trace_on() {
                self.trace_point(
                    Point::PassCut,
                    json!({"defender": i, "chance": cut.chance, "control": cut.control, "won": won}),
                );
            }
            if !won {
                continue;
            }
            if cut.control {
                self.gain(i, t);
            } else {
                self.tally(i, Count::Block);
                let at = self.ball.xy();
                self.value_won(i, at);
                #[cfg(feature = "measures")]
                if self.measures.is_some() {
                    self.measure_action(i);
                }
                let angle = self.streams.draw(Key::player(Action::BlockDeflect, p));
                self.ball.vel =
                    self.config
                        .modules
                        .ball
                        .deflect(&self.view(), Deflection::Block, angle, 0.0);
                self.deflected_by(i);
                self.pass_in_flight = None;
                self.cutters_tried = 0;
                self.duelled = 0;
                self.keeper_beaten = false;
            }
            return true;
        }
        false
    }

    /// Each defender the possession module names meets running carrier `c` in a take-on: one
    /// draw per meeting, and a defender who wins takes the ball (a tackle won). A defender
    /// who has left the pressed distance of the carrier may meet him again. Returns `true`
    /// when a defender won the ball.
    fn try_take_on(&mut self, c: usize, t: &Tuning) -> bool {
        let possession = self.config.modules.possession;
        let reach = t.tackle_reach * crate::possession::TAKE_ON_STRETCH;
        for i in 0..self.players.len() {
            if self.duelled & (1u32 << i) != 0
                && (self.players[i].pos - self.players[c].pos).length() >= reach
            {
                self.duelled &= !(1u32 << i);
            }
            let Some(chance) = possession.take_on(&self.view(), c, i) else {
                continue;
            };
            self.duelled |= 1u32 << i;
            let p = crate::streams::PlayerKey::of(&self.players[i]);
            let won = self
                .streams
                .tested(Key::player(Action::TakeOn, p), &[chance])
                < chance;
            #[cfg(feature = "measures")]
            {
                self.measure_tackle(i, c);
                let (team, squad) = (self.players[c].team, self.players[c].squad);
                if let Some(m) = self.measures.as_deref_mut() {
                    m.run_tackle(team, squad, won);
                }
            }
            #[cfg(feature = "sensitivity")]
            if self.probe.is_some() {
                self.probe_tackle(i, c, if won { Tackle::Win } else { Tackle::Miss });
            }
            if self.trace_on() {
                self.trace_point(
                    Point::TakeOn,
                    json!({"defender": i, "carrier": c, "chance": chance, "won": won}),
                );
            }
            if won {
                self.tally(i, Count::TackleWon);
                let at = self.ball.xy();
                self.value_won(i, at);
                self.mood_event(i, crate::sim::mood::MoodEvent::DuelWon);
                self.mood_event(c, crate::sim::mood::MoodEvent::DuelLost);
                self.gain(i, t);
                return true;
            }
        }
        false
    }

    /// Player `i` deflected the ball: his side touched the ball last and the shot in flight is
    /// over; a shot he turned stays its shooter's while the turned ball is in flight.
    pub(crate) fn deflected_by(&mut self, i: usize) {
        if self.ball.vel.z > 0.0 && self.ball.pos.z <= 0.0 {
            self.ball.pos.z = 0.001;
        }
        let team = self.players[i].team;
        #[cfg(feature = "measures")]
        if let Some(m) = self.measures.as_deref_mut() {
            m.touched(team, self.tick);
        }
        self.last_touch = Some(team);
        self.last_kicker = Some(i);
        self.last_kick_dir = self.ball.vel.truncate();
        let turned = self
            .shot_in_flight
            .zip(self.shooter)
            .or(self.shot_deflected);
        let xg = self.shot_xg;
        self.end_shot();
        self.shot_deflected = turned;
        // A turned shot keeps its expected goals for the value of its goal.
        if turned.is_some() {
            self.shot_xg = xg;
        }
    }

    /// Player `i` lost a header and the ball glanced off him. On a pass in flight the passer
    /// stays the last kicker, so the pass is credited to him if his side gains the ball.
    fn header_lost_by(&mut self, i: usize) {
        if self.pass_in_flight.is_none() {
            self.deflected_by(i);
            return;
        }
        if self.ball.vel.z > 0.0 && self.ball.pos.z <= 0.0 {
            self.ball.pos.z = 0.001;
        }
        #[cfg(feature = "measures")]
        if let Some(m) = self.measures.as_deref_mut() {
            m.touched(self.players[i].team, self.tick);
        }
        self.last_touch = Some(self.players[i].team);
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
                if self.try_cut_pass(t) {
                    return;
                }
                self.mark_heavy(t);
                #[cfg(feature = "measures")]
                self.measure_reception(t);
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
                        #[cfg(feature = "sensitivity")]
                        if self.probe.is_some() {
                            self.probe_header(i, won);
                        }
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
                            self.header_lost_by(i);
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
                    #[cfg(feature = "sensitivity")]
                    if self.probe.is_some() {
                        self.probe_loose(i);
                    }
                    self.gain(i, t);
                }
            }
            Some(c) => {
                if self.try_take_on(c, t) {
                    return;
                }
                let Some(mut mask) = possession.tacklers(&self.view(), c) else {
                    return;
                };
                while mask != 0 {
                    let i = mask.trailing_zeros() as usize;
                    mask &= mask - 1;
                    let p = crate::streams::PlayerKey::of(&self.players[i]);
                    let fouls = self.config.modules.fouls;
                    let chances = fouls.tackle_chances(&self.view(), i, c);
                    // The tackle decision: a defender who declines draws nothing.
                    let decision = self.config.modules.decision;
                    let Some(chances) = decision.tackle(&self.view(), i, c, chances) else {
                        continue;
                    };
                    #[cfg(feature = "measures")]
                    {
                        self.measure_tackle(i, c);
                        if let Some(m) = self.measures.as_deref_mut() {
                            m.tackle_decided(i);
                        }
                    }
                    let (p_win, p_foul) = (chances.p_win, chances.p_foul);
                    let draw = self
                        .streams
                        .tested(Key::player(Action::Tackle, p), &[p_win, p_win + p_foul]);
                    let outcome = fouls.tackle_outcome(&chances, draw);
                    #[cfg(feature = "measures")]
                    {
                        let carrier = &self.players[c];
                        if carrier.vel.length() > crate::rules::fouls::RUNNING_SPEED {
                            let lost =
                                matches!(outcome, Tackle::Win | Tackle::Foul { ball_lost: true });
                            let (team, squad) = (carrier.team, carrier.squad);
                            if let Some(m) = self.measures.as_deref_mut() {
                                m.run_tackle(team, squad, lost);
                            }
                        }
                    }
                    #[cfg(feature = "sensitivity")]
                    if self.probe.is_some() {
                        self.probe_tackle(i, c, outcome);
                    }
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
                            self.tally(i, Count::TackleWon);
                            let at = self.ball.xy();
                            self.value_won(i, at);
                            self.mood_event(i, crate::sim::mood::MoodEvent::DuelWon);
                            self.mood_event(c, crate::sim::mood::MoodEvent::DuelLost);
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
        #[cfg(feature = "measures")]
        let (keeper, fast) = (
            i == self.keeper(team),
            self.ball.vel.truncate().length() > _t.pass_arrival_speed,
        );
        #[cfg(feature = "measures")]
        let in_box = {
            let (ball, pitch) = (self.ball.xy(), self.config.pitch());
            let attack = [self.teams[0].attack_x, self.teams[1].attack_x];
            move |side: usize| pitch.in_penalty_area(ball, attack[side])
        };
        #[cfg(feature = "measures")]
        if let Some(m) = self.measures.as_deref_mut() {
            m.probe.box_reached(in_box);
            if self.pass_in_flight == Some(team) {
                if self.last_kicker == Some(i) {
                    m.self_regained(team, keeper, fast);
                }
                m.completed(team);
            }
            m.touched(team, self.tick);
            m.gained(team, self.tick);
        }
        let changed = self.last_touch != Some(team);
        if self.pass_in_flight == Some(team) && self.possession_phase == Some(team) {
            self.receive_triggers(i, team);
        }
        #[cfg(feature = "sensitivity")]
        if self.probe.is_some() {
            let (at, tick) = (self.players[i].pos, self.tick);
            if let Some(p) = self.probe.as_deref_mut() {
                p.gained(team, at, tick);
            }
        }
        let at = self.ball.xy();
        self.value_gain(i, at);
        match self.pass_in_flight.take() {
            Some(passing) if passing == team => {
                self.summary.passes_completed[team] += 1;
                if let Some(k) = self.last_kicker {
                    self.tally(k, Count::PassCompleted);
                    self.last_pass = Some((k, i));
                }
            }
            Some(_) => {
                self.tally(i, Count::Interception);
                self.value_won(i, at);
                #[cfg(feature = "measures")]
                if self.measures.is_some() {
                    self.measure_action(i);
                }
                if let Some(k) = self.last_kicker {
                    self.mood_event(k, crate::sim::mood::MoodEvent::Error);
                }
            }
            None => {}
        }
        // The other side has the ball: no pass of the old side can be an assist.
        if self.last_touch != Some(team) {
            self.last_pass = None;
        }
        self.clearers_tried = 0;
        self.cutters_tried = 0;
        self.duelled = 0;
        if self.restart_taker != Some(i) {
            self.restart_taker = None;
        }
        self.end_shot();
        if self.last_touch != Some(team) {
            self.summary.possession_changes += 1;
        }
        self.last_touch = Some(team);
        // A gain from the other side in open play opens the transition windows: the regain
        // window of the side that won the ball and the counter-press window of the side that
        // lost it.
        if self.possession_phase == Some(1 - team) {
            self.windows.won(team, self.tick);
        }
        self.possession_phase = Some(team);
        self.carrier = Some(i);
        self.control_since = self.tick;
        self.script_cache = None;
        // The gain ends the overrides it ends, and a side that gains the ball is a moment
        // for its anchor traits.
        self.end_acts_on_gain(i);
        if changed {
            self.gain_override(team);
        }
    }
}

impl Simulation {
    /// Press trigger `kind` of the side with the ball fires for pressing side `def`: its
    /// window opens now.
    pub(crate) fn fire_trigger(&mut self, def: usize, kind: usize) {
        self.windows.fire_trigger(def, kind, self.tick);
        #[cfg(feature = "measures")]
        if let Some(m) = self.measures.as_deref_mut() {
            m.trigger(def, kind, self.tick);
        }
    }

    /// While a pass of the side in possession is in flight, each team-mate of the passer (not
    /// the passer) who has the ball within his reach while it is too fast for him to control
    /// joins the heavy mask: if he gains it later, his touch was a poor one.
    fn mark_heavy(&mut self, t: &Tuning) {
        let Some(team) = self.pass_in_flight else {
            return;
        };
        let speed = self.ball.speed();
        let xy = self.ball.xy();
        let z = self.ball.pos.z;
        for (i, p) in self.players.iter().enumerate() {
            if p.team != team || !p.active() || Some(i) == self.last_kicker || z > p.derived.reach_m
            {
                continue;
            }
            let k = &p.derived.knobs;
            if speed > t.control_speed * k.receive
                && (p.pos - xy).length() < t.reach_radius * k.intercept
            {
                self.heavy_mask |= 1u32 << i;
            }
        }
    }

    /// Player `i` of `team` gained his side's pass in open play: the press triggers it fires
    /// for the other side. A gain after the ball came too fast to control is a poor touch; a
    /// receiver facing his own goal faces back; a full-back or a receiver in a wide channel is
    /// a ball wide.
    fn receive_triggers(&mut self, i: usize, team: usize) {
        use crate::decision::{TRIGGER_FACING_BACK, TRIGGER_POOR_TOUCH, TRIGGER_WIDE};
        let def = 1 - team;
        if self.heavy_mask & (1u32 << i) != 0 {
            self.fire_trigger(def, TRIGGER_POOR_TOUCH);
        }
        let p = &self.players[i];
        let attack_x = self.teams[team].attack_x;
        if p.facing.x * attack_x < -crate::decision::FACING_BACK_COS {
            self.fire_trigger(def, TRIGGER_FACING_BACK);
        }
        let p = &self.players[i];
        let fullback = matches!(
            self.teams[team].plan.slots[p.slot].position,
            Some(crate::data::team::Position::LB | crate::data::team::Position::RB)
        );
        if fullback || crate::decision::channel(p.pos.y, attack_x) != 1 {
            self.fire_trigger(def, TRIGGER_WIDE);
        }
        self.heavy_mask = 0;
    }

    /// Where player `i`'s defensive action lies: his side, the ball's share of the pitch
    /// length and its metres from his own goal line, and whether his slot is a centre-back or a
    /// full-back.
    #[cfg(feature = "measures")]
    fn action_place(&self, i: usize) -> (usize, (f64, f64), bool) {
        use crate::data::team::Position;
        let p = &self.players[i];
        let team = p.team;
        let pitch = self.config.pitch();
        let m = self.ball.xy().x * self.teams[team].attack_x + pitch.half_length();
        let m = m.clamp(0.0, pitch.length());
        let defender = matches!(
            self.teams[team].plan.slots[p.slot].position,
            Some(Position::CB | Position::LB | Position::RB)
        );
        (team, (m / pitch.length(), m), defender)
    }

    /// Count site: while a pass of the side in possession is in flight, the first team-mate of
    /// the passer (not the passer) who has the ball within his full reach, at a height and a
    /// speed he can control, receives it, under pressure or free.
    #[cfg(feature = "measures")]
    fn measure_reception(&mut self, t: &Tuning) {
        let Some(team) = self.pass_in_flight else {
            return;
        };
        let speed = self.ball.speed();
        let xy = self.ball.xy();
        let z = self.ball.pos.z;
        let keeper = self.keeper(team);
        let last = self.last_kicker;
        let Some(i) = self.players.iter().enumerate().position(|(i, p)| {
            let k = &p.derived.knobs;
            p.team == team
                && p.active()
                && i != keeper
                && Some(i) != last
                && z <= p.derived.reach_m
                && speed <= t.control_speed * k.receive
                && (p.pos - xy).length() < t.reach_radius * k.intercept
        }) else {
            return;
        };
        let pressed = crate::decision::pressed(&self.view(), i);
        let band = crate::pressure::band(crate::decision::nearest_opponent_m(&self.view(), i));
        let squad = self.players[i].squad;
        if let Some(m) = self.measures.as_deref_mut() {
            m.received(team, pressed, band, squad);
        }
    }

    /// Count site: defender `i` tackled carrier `c`.
    #[cfg(feature = "measures")]
    pub(crate) fn measure_tackle(&mut self, i: usize, c: usize) {
        let (def, share, defender) = self.action_place(i);
        if let Some(m) = self.measures.as_deref_mut() {
            m.tackle(def, i, c, share, defender);
        }
    }

    /// Count site: player `i` intercepted a pass or fouled the carrier.
    #[cfg(feature = "measures")]
    pub(crate) fn measure_action(&mut self, i: usize) {
        let (def, share, defender) = self.action_place(i);
        if let Some(m) = self.measures.as_deref_mut() {
            m.defensive_action(def, share, defender);
        }
    }
}

//! The decision pass of the central loop: every player's target and the carrier's kick. The
//! decision module decides; this pass draws every noise and aim value on the module's keys,
//! in the order the decision maker always drew them, and writes the targets.

use serde_json::json;

use crate::data::rules::StoppageKind;
use crate::decision::{Kick, Options, offsets_json};
use crate::math::DVec2;
use crate::modules::{CarrierPlan, CoverTrace, OptionDraws, ShotDraws, TargetsTrace};
use crate::sim::Simulation;
use crate::streams::{Action, Key};
use crate::trace::Point;

impl Simulation {
    /// Sets every player's target for this tick and returns the carrier's kick, if any.
    pub(crate) fn decide(&mut self) -> Option<Kick> {
        let decision = self.config.modules.decision;
        let targets = decision.targets(&self.view());
        for (p, &target) in self.players.iter_mut().zip(targets.targets.iter()) {
            p.target = target;
        }
        match targets.trace {
            TargetsTrace::Carrier {
                def,
                count,
                reach,
                pressers,
                cover,
            } => {
                if self.trace_on() {
                    let chosen: Vec<_> = pressers[..count]
                        .iter()
                        .filter(|&&(_, i)| i != usize::MAX)
                        .map(|&(d, i)| json!({"player": i, "distance": d}))
                        .collect();
                    self.trace_point(
                        Point::Press,
                        json!({"team": def, "count": count, "reach": reach, "pressers": chosen}),
                    );
                    let detail = match cover {
                        CoverTrace::NoAttacker => json!({"team": def, "attacker": null}),
                        CoverTrace::NoDefender { attacker } => {
                            json!({"team": def, "attacker": attacker, "defender": null})
                        }
                        CoverTrace::Covered {
                            attacker,
                            defender,
                            distance,
                        } => json!({
                            "team": def,
                            "attacker": attacker,
                            "defender": defender,
                            "distance": distance,
                        }),
                    };
                    self.trace_point(Point::Cover, detail);
                }
                let c = self.carrier?;
                self.decide_carrier(c)
            }
            TargetsTrace::Loose {
                nearest,
                nearest_dist,
                keepers,
            } => {
                if self.trace_on() {
                    let chasers: Vec<_> = (0..2)
                        .map(|team| {
                            let i = nearest[team];
                            json!({
                                "team": team,
                                "player": (i != usize::MAX).then_some(i),
                                "distance": (i != usize::MAX).then_some(nearest_dist[team]),
                                "keeper": keepers[team],
                            })
                        })
                        .collect();
                    self.trace_point(Point::Chase, json!({ "chasers": chasers }));
                }
                None
            }
        }
    }

    /// Scores every option carrier `c` has: the decision module scores each option up to its
    /// noise term, this pass draws the noise of each in order (the shot, each pass candidate
    /// in roster order, the dribble and the hold, the clearance), and the module adds it.
    pub(crate) fn options(&mut self, c: usize) -> Options {
        let decision = self.config.modules.decision;
        let draft = decision.options(&self.view(), c);
        let carrier = self.players[c];
        let mut draws = OptionDraws::default();
        if draft.shot.is_some() {
            draws.shot = self.streams.draw(Key::player(Action::ShotScore, &carrier));
        }
        for draw in &mut draws.passes[..draft.pass_count] {
            *draw = self.streams.draw(Key::player(Action::PassScore, &carrier));
        }
        if draft.dribble_hold.is_some() {
            let dribble = self
                .streams
                .draw(Key::player(Action::DribbleScore, &carrier));
            let hold = self.streams.draw(Key::player(Action::HoldScore, &carrier));
            draws.dribble_hold = (dribble, hold);
        }
        draws.clear = self.streams.draw(Key::player(Action::ClearScore, &carrier));
        let scored = decision.scored(&draft, &draws);
        if let Some(trace) = self.streams.trace_mut() {
            for &(j, score) in &scored.candidates[..scored.count] {
                trace.push_candidate(j, score);
            }
        }
        scored.options
    }

    pub(crate) fn decide_carrier(&mut self, c: usize) -> Option<Kick> {
        let options = self.options(c);
        let offsets = self.script_offsets(c);
        let decision = self.config.modules.decision;
        let (o, choice) = decision.choose(&self.view(), c, &options, offsets);
        if let Some(trace) = self.streams.trace_mut() {
            let candidates: Vec<_> = trace
                .take_candidates()
                .into_iter()
                .map(|(mate, score)| json!({"mate": mate, "score": score}))
                .collect();
            let detail = json!({
                "carrier": c,
                "candidates": candidates,
                "shot": o.shot,
                "pass": o.pass.map(|(score, mate)| json!({"mate": mate, "score": score})),
                "dribble": o.dribble,
                "hold": o.hold,
                "clear": o.clear,
                "offsets": offsets.as_ref().map(offsets_json),
                "choice": choice.name(),
            });
            trace.push_point(Point::Carrier, detail);
        }
        let carrier = self.players[c];
        match decision.plan(&self.view(), c, &o, choice) {
            CarrierPlan::Shot { goal, keeper } => Some(self.shot_kick(c, goal, keeper, 1.0)),
            CarrierPlan::Pass { j } => {
                let aim = self.streams.draw(Key::player(Action::PassAim, &carrier));
                Some(decision.pass_kick(&self.view(), c, j, aim))
            }
            CarrierPlan::Clear { wide_chance } => {
                let wide = wide_chance.is_some_and(|p| {
                    self.streams
                        .tested(Key::player(Action::ClearWide, &carrier), &[p])
                        < p
                });
                let aim = self.streams.draw(Key::player(Action::ClearAim, &carrier));
                Some(decision.clear_kick(&self.view(), c, wide, aim))
            }
            CarrierPlan::Move { target } => {
                self.players[c].target = target;
                None
            }
        }
    }
}

impl Simulation {
    /// Player `c` shoots at the goal centred on `goal`, which `keeper` defends; `spread_scale`
    /// scales the aim noise (1 in open play, less for a kick from the penalty mark). An
    /// open-play shot and a shoot-out kick both come from here, and the draws are taken in
    /// the same order either way: the side (only with the keeper off the pitch), the aim, the
    /// spread, and the loft.
    pub(crate) fn shot_kick(
        &mut self,
        c: usize,
        goal: DVec2,
        keeper: usize,
        spread_scale: f64,
    ) -> Kick {
        let decision = self.config.modules.decision;
        let carrier = self.players[c];
        let side = decision.shot_draws_side(&self.view(), keeper).then(|| {
            self.streams
                .chance(Key::player(Action::ShotSide, &carrier), 0.5)
        });
        let aim = self.streams.draw(Key::player(Action::ShotAim, &carrier));
        let spread = self.streams.draw(Key::player(Action::ShotSpread, &carrier));
        let loft = self.streams.draw(Key::player(Action::ShotLoft, &carrier));
        let draws = ShotDraws {
            side,
            aim,
            spread,
            loft,
        };
        decision.shot_kick(&self.view(), c, goal, keeper, spread_scale, &draws)
    }
}

impl Simulation {
    /// The kick that takes a restart, from the decision module.
    pub(crate) fn restart_pass(&mut self, taker: usize, kind: StoppageKind) -> Kick {
        let pass = self
            .config
            .modules
            .decision
            .restart_pass(&self.view(), taker, kind);
        if let Some(trace) = self.streams.trace_mut() {
            let candidates: Vec<_> = pass.candidates[..pass.count]
                .iter()
                .map(|&(mate, score)| json!({"mate": mate, "score": score}))
                .collect();
            trace.push_point(
                Point::RestartPass,
                json!({
                    "taker": taker,
                    "kind": kind.code(),
                    "candidates": candidates,
                    "target": [pass.target.x, pass.target.y],
                    "forward_fallback": pass.fallback,
                }),
            );
        }
        pass.kick
    }
}

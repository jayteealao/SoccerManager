//! The trait override's moments, its draw and its event ([`crate::traits`]). A moment is a
//! decision that happens in play: a carrier's release of the ball, his first decision after
//! gaining it, the start of his carry from a wide channel, a new opposing carrier for a
//! presser, his side gaining the ball for an anchor, and a loose ball reaching a keeper's
//! range. At a moment that meets the override's conditions the player
//! draws once on his own `trait_override` key; the draw happens nowhere else, so a match with
//! no strong conflicting trait draws nothing new.
//!
//! Each moment lists the traits whose choice it reads: the traits with a row in the conflict
//! table ([`TraitKind::conflicts`]). A trait given a row needs its moment here too, with a
//! test that reaches it.

use serde_json::json;

use crate::contract;
use crate::decision::{self, Choice, Options};
use crate::modules::{Scored, TargetsTrace};
use crate::sim::{EngineEventKind, EventDetail, Simulation};
use crate::streams::{Action, Key, PlayerKey};
use crate::tactics::Instruction;
use crate::trace::Point;
use crate::traits::{ActEnd, OverrideCause, TraitAct, TraitKind, TraitSet, cause};

/// What a trait does when it wins its moment.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Forced {
    /// A pass to this team-mate instead.
    Pass(usize),
    /// The trait's own action, held until it ends.
    Act(ActEnd),
}

/// A trait whose moment play met, with the instruction it goes against and its chance.
#[derive(Debug, Clone, Copy)]
struct Candidate {
    kind: TraitKind,
    instruction: Instruction,
    chance: f64,
    forced: Forced,
}

/// The candidates of one moment, the larger chance first. On a tie at the top the
/// instruction holds and nobody draws (the shape's edge case: two conflicting traits at one
/// moment, the stronger wins, on a tie the instruction holds).
fn ordered(mut found: Vec<Candidate>) -> Vec<Candidate> {
    found.sort_by(|a, b| b.chance.total_cmp(&a.chance).then(a.kind.cmp(&b.kind)));
    if found.len() > 1 && found[0].chance == found[1].chance {
        found.clear();
    }
    found
}

impl Simulation {
    /// Player `i`'s traits, from his squad entry.
    #[inline]
    pub fn traits(&self, i: usize) -> &TraitSet {
        let p = &self.players[i];
        &self.teams[p.team].squad[p.squad].trait_set
    }

    /// The team-mate whose completed pass reached player `i`, while his side keeps the ball.
    #[inline]
    pub fn last_passer(&self, i: usize) -> Option<usize> {
        self.last_pass
            .and_then(|(passer, receiver)| (receiver == i).then_some(passer))
    }

    /// The override player `i` plays now, while its action lasts.
    #[inline]
    pub fn trait_act(&self, i: usize) -> Option<TraitAct> {
        let act = self.trait_acts[i]?;
        let live = match act.ends {
            ActEnd::Tick(end) => self.tick < end && self.carrier == Some(i),
            ActEnd::Carrier(c) => self.carrier == Some(c),
            ActEnd::Possession(team) => self.last_touch == Some(team),
            ActEnd::LooseBall => self.carrier.is_none(),
        };
        live.then_some(act)
    }

    /// The conditions an override shares for player `i` and trait `kind`: the traits block,
    /// the trait strong, an instruction it goes against held for him, its chance above 0, and
    /// pressure or a low mood. The cause, the instruction and the chance.
    fn conditions(&self, i: usize, kind: TraitKind) -> Option<(OverrideCause, Instruction, f64)> {
        let tuning = self.config.tuning.contract.traits.as_ref()?;
        if !self.traits(i).is_strong(kind) {
            return None;
        }
        let p = &self.players[i];
        let instruction = kind.conflict_in(self.teams[p.team].plan_for(p.slot).held[p.slot])?;
        let chance = tuning.override_chance.of(kind);
        if chance <= 0.0 {
            return None;
        }
        let pressed = decision::pressed(&self.view(), i);
        let why = cause(pressed, self.mood(i), tuning)?;
        Some((why, instruction, chance))
    }

    /// `true` when player `i` may override at all: a traits block and a strong trait that an
    /// instruction held for him goes against. The cheap test before any moment is read.
    #[inline]
    fn may_override(&self, i: usize) -> bool {
        if self.config.tuning.contract.traits.is_none() {
            return false;
        }
        let set = self.traits(i);
        if set.is_empty() {
            return false;
        }
        let p = &self.players[i];
        let held = self.teams[p.team].plan_for(p.slot).held[p.slot];
        held != 0
            && TraitKind::ALL
                .iter()
                .any(|&k| set.is_strong(k) && k.conflict_in(held).is_some())
    }

    /// Draws the candidates of player `i`'s moment in order until one wins, writes its event,
    /// and returns it.
    fn draw_override(
        &mut self,
        i: usize,
        found: Vec<Candidate>,
        why: &[OverrideCause],
    ) -> Option<Candidate> {
        for (k, c) in ordered(found).into_iter().enumerate() {
            let key = Key::player(Action::TraitOverride, PlayerKey::of(&self.players[i]));
            let won = self.streams.tested(key, &[c.chance]) < c.chance;
            let cause = why[k.min(why.len() - 1)];
            #[cfg(feature = "sensitivity")]
            if let Some(p) = self.probe.as_deref_mut() {
                p.trait_moment(i, won);
            }
            if self.trace_on() {
                self.trace_point(
                    Point::TraitOverride,
                    json!({
                        "player": i,
                        "trait": c.kind.name(),
                        "instruction": c.instruction.code(),
                        "cause": cause.code(),
                        "chance": c.chance,
                        "won": won,
                    }),
                );
            }
            if won {
                let team = self.players[i].team;
                let mut event = self.event(EngineEventKind::TraitOverride, Some(team));
                event.player = Some(i);
                event.detail = Some(EventDetail::Override {
                    trait_kind: c.kind,
                    cause,
                    instruction: c.instruction,
                });
                self.events.push(event);
                return Some(c);
            }
        }
        None
    }

    /// Collects the candidates of player `i` among `kinds`, each with the trait's choice
    /// `choice(kind)` when the moment applies, and draws them. The cause of each comes from
    /// the shared conditions.
    fn moment(
        &mut self,
        i: usize,
        kinds: &[TraitKind],
        choice: impl Fn(&Self, TraitKind) -> Option<Forced>,
    ) -> Option<Candidate> {
        let mut found = Vec::new();
        let mut why = Vec::new();
        for &kind in kinds {
            let Some((cause, instruction, chance)) = self.conditions(i, kind) else {
                continue;
            };
            let Some(forced) = choice(self, kind) else {
                continue;
            };
            found.push(Candidate {
                kind,
                instruction,
                chance,
                forced,
            });
            why.push(cause);
        }
        if found.is_empty() {
            return None;
        }
        // Every candidate of one moment shares the player's pressure and mood, so one cause.
        let cause = why[0];
        self.draw_override(i, found, &[cause])
    }

    /// The carrier's moments after his choice: a release (a pass or a clearance about to be
    /// kicked), his first decision after gaining the ball, and the start of a carry from a
    /// wide channel. Returns the choice and options play goes on with: the trait's when it
    /// wins, else the decision's. A carrier playing an override's carry keeps it.
    pub(crate) fn carrier_override(
        &mut self,
        c: usize,
        scored: &Scored,
        options: Options,
        choice: Choice,
    ) -> (Options, Choice) {
        let (options, choice) = match self.trait_act(c) {
            Some(act) => match act.kind {
                TraitKind::DwellsOnBall | TraitKind::HoldsUpBall if options.hold.is_some() => {
                    (options, Choice::Hold)
                }
                TraitKind::CutsInside | TraitKind::HugsTouchline if options.dribble.is_some() => {
                    (options, Choice::Dribble)
                }
                _ => (options, choice),
            },
            None if self.may_override(c) => self.carrier_moment(c, scored, options, choice),
            None => (options, choice),
        };
        let bit = 1u32 << c;
        if choice == Choice::Dribble {
            self.carrying |= bit;
        } else {
            self.carrying &= !bit;
        }
        (options, choice)
    }

    fn carrier_moment(
        &mut self,
        c: usize,
        scored: &Scored,
        options: Options,
        choice: Choice,
    ) -> (Options, Choice) {
        let kinds = [
            TraitKind::DwellsOnBall,
            TraitKind::HoldsUpBall,
            TraitKind::ReleasesBallEarly,
            TraitKind::CutsInside,
            TraitKind::HugsTouchline,
        ];
        let won = self.moment(c, &kinds, |sim, kind| {
            sim.carrier_choice(kind, c, scored, &options, choice)
        });
        let Some(won) = won else {
            return (options, choice);
        };
        let mut o = options;
        match won.forced {
            Forced::Pass(j) => {
                let score = scored.candidates[..scored.count]
                    .iter()
                    .find(|&&(m, _)| m == j)
                    .map_or(0.0, |&(_, s)| s);
                o.pass = Some((score, j));
                (o, Choice::Pass)
            }
            Forced::Act(ends) => {
                self.trait_acts[c] = Some(TraitAct {
                    kind: won.kind,
                    ends,
                });
                let choice = match won.kind {
                    TraitKind::DwellsOnBall | TraitKind::HoldsUpBall => Choice::Hold,
                    _ => Choice::Dribble,
                };
                (o, choice)
            }
        }
    }

    /// The tick an override's carry of `traits.carry_s` ends.
    fn carry_end(&self) -> ActEnd {
        let s = self.config.tuning.contract.traits().carry_s;
        let ticks = (s / self.config.tuning.dt).round().max(1.0) as u32;
        ActEnd::Tick(self.tick + ticks)
    }

    /// The trait's choice for carrier `c` at this decision, when its moment applies and the
    /// choice differs from the one play took and is available.
    fn carrier_choice(
        &self,
        kind: TraitKind,
        c: usize,
        scored: &Scored,
        options: &Options,
        choice: Choice,
    ) -> Option<Forced> {
        let me = &self.players[c];
        // A release: a pass or a clearance about to be kicked. A shot is his own call and
        // stays one.
        let release = matches!(choice, Choice::Pass | Choice::Clear);
        let passing_to = match choice {
            Choice::Pass => options.pass.map(|(_, j)| j),
            _ => None,
        };
        let mates = &scored.candidates[..scored.count];
        let best = |keep: &dyn Fn(usize) -> bool| {
            mates
                .iter()
                .filter(|&&(j, _)| keep(j))
                .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
                .map(|&(j, _)| j)
        };
        let pass = |j: Option<usize>| j.filter(|&j| Some(j) != passing_to).map(Forced::Pass);
        let pressed = decision::pressed(&self.view(), c);
        let wide = me.pos.y.abs() > contract::CROSS_WIDE_M;
        match kind {
            TraitKind::DwellsOnBall if release && options.hold.is_some() => {
                Some(Forced::Act(self.carry_end()))
            }
            TraitKind::HoldsUpBall if release && pressed && options.hold.is_some() => {
                Some(Forced::Act(self.carry_end()))
            }
            TraitKind::ReleasesBallEarly
                if matches!(choice, Choice::Hold | Choice::Dribble) && self.first_decision() =>
            {
                pass(best(&|_| true))
            }
            TraitKind::CutsInside | TraitKind::HugsTouchline
                if choice == Choice::Dribble && wide && self.carrying & (1u32 << c) == 0 =>
            {
                let (_, perp, side) =
                    decision::dribble_line(&self.view(), c, options.nearest_opp_pos);
                let inside = decision::inside_side(me.pos, perp);
                let wanted = if kind == TraitKind::CutsInside {
                    inside
                } else {
                    -inside
                };
                (wanted != side).then(|| Forced::Act(self.carry_end()))
            }
            _ => None,
        }
    }

    /// `true` on the carrier's first decision after he gained the ball.
    fn first_decision(&self) -> bool {
        let held = self.tick.saturating_sub(self.control_since);
        (1..=self.config.tuning.decision_interval_ticks.max(1)).contains(&held)
    }

    /// The new-carrier moment of the side out of possession: on the first decision after
    /// carrier `c` gained the ball, a defender who presses relentlessly, within his reach of
    /// the ball and not one of the chosen pressers, may press him until the ball leaves him.
    pub(crate) fn press_override(&mut self, c: usize, trace: &TargetsTrace) {
        let TargetsTrace::Carrier {
            def,
            count,
            pressers,
            ..
        } = *trace
        else {
            return;
        };
        if !self.first_decision() {
            return;
        }
        let ball = self.ball.xy();
        let weights = self.config.tuning.contract.traits().weights;
        for i in def * 11..def * 11 + 11 {
            let p = self.players[i];
            if !p.active()
                || i == self.keeper(def)
                || pressers[..count].iter().any(|&(_, k)| k == i)
                || self.trait_act(i).is_some()
                || !self.may_override(i)
            {
                continue;
            }
            // Each player's own press distance (a player instruction may set it).
            let reach = self.teams[def].plan_for(p.slot).press_distance;
            let far = reach * p.derived.knobs.press * (1.0 + weights.presses_relentlessly);
            if (p.pos - ball).length() >= far {
                continue;
            }
            let won = self.moment(i, &[TraitKind::PressesRelentlessly], |_, _| {
                Some(Forced::Act(ActEnd::Carrier(c)))
            });
            if let Some(won) = won
                && let Forced::Act(ends) = won.forced
            {
                self.trait_acts[i] = Some(TraitAct {
                    kind: won.kind,
                    ends,
                });
            }
        }
    }

    /// The possession-gain moment: side `team` gained the ball, so each of its players whose
    /// strong anchor trait goes against his duty may play the trait's anchor until the side
    /// loses it. The other side's anchor overrides end, and so does any anchor of `team` left
    /// from an earlier spell: a new spell is a new moment, drawn afresh.
    pub(crate) fn gain_override(&mut self, team: usize) {
        for act in &mut self.trait_acts {
            if act.is_some_and(|a| matches!(a.ends, ActEnd::Possession(_))) {
                *act = None;
            }
        }
        for i in team * 11..team * 11 + 11 {
            if !self.players[i].active() || !self.may_override(i) {
                continue;
            }
            let kinds = [
                TraitKind::GetsForward,
                TraitKind::ArrivesLateInBox,
                TraitKind::StaysBack,
            ];
            let won = self.moment(i, &kinds, |_, _| {
                Some(Forced::Act(ActEnd::Possession(team)))
            });
            if let Some(won) = won
                && let Forced::Act(ends) = won.forced
            {
                self.trait_acts[i] = Some(TraitAct {
                    kind: won.kind,
                    ends,
                });
            }
        }
    }

    /// The loose-ball moment of each keeper, once per loose ball: one who rushes out meets it
    /// when the ball enters his trait's range but not the range his line's instruction keeps
    /// him to; one who stays on his line meets it when the ball enters his normal range.
    pub(crate) fn keeper_override(&mut self) {
        let ball = self.ball.xy();
        for team in 0..2 {
            let gk = self.keeper(team);
            let bit = 1u32 << gk;
            if self.chase_seen & bit != 0
                || !self.players[gk].active()
                || self.trait_act(gk).is_some()
                || !self.may_override(gk)
            {
                continue;
            }
            let d = (self.players[gk].pos - ball).length();
            let (normal, wide) = decision::keeper_ranges(&self.view(), gk);
            let set = *self.traits(gk);
            let rush = set.is_strong(TraitKind::RushesOut) && d >= normal && d < wide;
            let stay = set.is_strong(TraitKind::StaysOnLine) && d < normal;
            if !rush && !stay {
                continue;
            }
            self.chase_seen |= bit;
            let kind = if rush {
                TraitKind::RushesOut
            } else {
                TraitKind::StaysOnLine
            };
            let won = self.moment(gk, &[kind], |_, _| Some(Forced::Act(ActEnd::LooseBall)));
            if let Some(won) = won
                && let Forced::Act(ends) = won.forced
            {
                self.trait_acts[gk] = Some(TraitAct {
                    kind: won.kind,
                    ends,
                });
            }
        }
    }

    /// Ends the overrides player `i`'s gain of the ball ends: every keeper's loose-ball act,
    /// every press on another carrier, and every other player's carry; and starts the next
    /// loose ball's keeper moments afresh.
    pub(crate) fn end_acts_on_gain(&mut self, i: usize) {
        for (k, act) in self.trait_acts.iter_mut().enumerate() {
            let ends = act.is_some_and(|a| match a.ends {
                ActEnd::LooseBall => true,
                ActEnd::Carrier(c) => c != i,
                ActEnd::Tick(_) => k != i,
                ActEnd::Possession(_) => false,
            });
            if ends {
                *act = None;
            }
        }
        self.chase_seen = 0;
        self.carrying &= !(1u32 << i);
    }

    /// A restart or a kick-off gives side `team` the ball: the other side's spell is over, so
    /// its anchor overrides end and cannot come back when it next has the ball.
    pub(crate) fn end_acts_on_restart(&mut self, team: usize) {
        for act in &mut self.trait_acts {
            if act.is_some_and(|a| a.ends == ActEnd::Possession(1 - team)) {
                *act = None;
            }
        }
    }

    /// Ends every override whose action a dead ball ends: a carry, a press and a keeper's
    /// loose-ball act. An anchor override lasts while the side keeps the ball.
    pub(crate) fn end_acts_on_stoppage(&mut self) {
        for act in &mut self.trait_acts {
            if act.is_some_and(|a| !matches!(a.ends, ActEnd::Possession(_))) {
                *act = None;
            }
        }
        self.chase_seen = 0;
        self.carrying = 0;
    }
}

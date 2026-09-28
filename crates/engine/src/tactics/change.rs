//! The stoppage-gated change queue (named mechanism), inside the engine.
//!
//! A manager, human or AI, queues a tactics change or a substitution with
//! [`Simulation::queue_change`]. The engine applies the queue on the tick that opens a
//! stoppage, after the referee and the clock and before overlap resolution, so the change
//! lands on the tick the record marks as a restart. The rule pack decides what each stoppage
//! admits; a change waits for a stoppage that admits its kind. Substitutions apply first, in
//! queue order, then tactics changes. A tactics change that names a role for a player the
//! same stoppage substituted off is rejected whole, with the player named.
//!
//! A substitution counts toward the rule pack's limit. A window is a stoppage at which a team
//! makes at least one substitution; a stoppage kind the rule pack lists in `windows_exempt`
//! (half-time) uses no window. Every verdict becomes an engine event: `ChangeApplied` with
//! the tick, or `ChangeRejected` with the reason.

use crate::trace::Point;
use serde_json::json;
use std::fmt;

use crate::data::rules::StoppageKind;
use crate::math::DVec2;
use crate::pitch;
use crate::player::Status;
use crate::rules::{Phase, Stoppage, restart};
use crate::sim::{EngineEventKind, EventDetail, Simulation};
use crate::tactics::TacticsPatch;
use crate::team::PLAYERS_PER_TEAM;

/// What a queued change does. Players are named by squad index (their place in the team
/// file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Tactics(TacticsPatch),
    Substitution { off: usize, on: usize },
}

impl Change {
    pub fn kind(&self) -> ChangeKind {
        match self {
            Change::Tactics(_) => ChangeKind::Tactics,
            Change::Substitution { .. } => ChangeKind::Substitution,
        }
    }
}

/// The kind of a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Tactics,
    Substitution,
}

impl ChangeKind {
    pub fn code(&self) -> &'static str {
        match self {
            ChangeKind::Tactics => "tactics",
            ChangeKind::Substitution => "substitution",
        }
    }
    /// The kind's place in per-kind arrays: tactics 0, substitution 1.
    pub fn index(&self) -> usize {
        match self {
            ChangeKind::Tactics => 0,
            ChangeKind::Substitution => 1,
        }
    }
}

/// A queue identifier, written `q-{tick}-{n}` like the socket queue's, so a later bridge
/// can carry an identifier through unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeId {
    /// The tick the change was queued on.
    pub tick: u32,
    /// The queue's running count.
    pub n: u32,
}

impl fmt::Display for ChangeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "q-{}-{}", self.tick, self.n)
    }
}

/// One change waiting for an admitting stoppage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedChange {
    pub id: ChangeId,
    pub team: usize,
    pub change: Change,
}

/// The changes waiting, in the order they were queued.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeQueue {
    pub pending: Vec<QueuedChange>,
    pub next: u32,
    /// The tick of the latest stoppage that admitted each kind, tactics first, then
    /// substitutions. A change still waiting at full time is expired when no stoppage that
    /// admits its kind opened after it was queued, and never applied when one did.
    pub admitted: [Option<u32>; 2],
}

/// How the changes still waiting at full time split.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Unapplied {
    /// Changes an admitting stoppage opened after, yet did not apply or reject.
    pub never_applied: u32,
    /// Changes queued after the last stoppage that admits their kind.
    pub expired: u32,
}

impl ChangeQueue {
    /// Splits the changes still waiting into never applied and expired.
    pub fn unapplied(&self) -> Unapplied {
        let mut out = Unapplied::default();
        for q in &self.pending {
            let admitted = self.admitted[q.change.kind().index()];
            if admitted.is_some_and(|at| at > q.id.tick) {
                out.never_applied += 1;
            } else {
                out.expired += 1;
            }
        }
        out
    }

    /// `true` when a substitution for `team` taking `off` off is waiting.
    pub fn has_substitution(&self, team: usize, off: usize) -> bool {
        self.pending.iter().any(|q| {
            q.team == team && matches!(q.change, Change::Substitution { off: o, .. } if o == off)
        })
    }

    /// Substitutions waiting for `team`.
    pub fn substitutions(&self, team: usize) -> usize {
        self.pending
            .iter()
            .filter(|q| q.team == team && q.change.kind() == ChangeKind::Substitution)
            .count()
    }

    /// Squad indices `team` has queued to bring on.
    pub fn incoming(&self, team: usize) -> Vec<usize> {
        self.pending
            .iter()
            .filter_map(|q| match q.change {
                Change::Substitution { on, .. } if q.team == team => Some(on),
                _ => None,
            })
            .collect()
    }
}

/// One team's substitutions so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SubLedger {
    /// Substitutions made.
    pub used: u8,
    /// Windows used.
    pub windows: u8,
    /// The stoppage tick of the window in use, so every substitution at one stoppage shares
    /// it.
    pub window_at: Option<u32>,
}

/// Why a change was rejected. Players are named by squad index; [`RejectReason::text`] names
/// them by player id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    LimitReached {
        limit: u8,
    },
    NoWindowLeft {
        windows: u8,
    },
    NotOnPitch {
        squad: usize,
    },
    SentOff {
        squad: usize,
    },
    NotOnBench {
        squad: usize,
    },
    /// The conflict rule: the change names a player the same stoppage substituted off.
    LeftThePitch {
        squad: usize,
    },
    OutOfRange,
}

impl RejectReason {
    /// The reason in words a viewer can show, naming players by `ids` (the team's player ids
    /// in file order).
    pub fn text(&self, ids: &[String]) -> String {
        let id = |s: &usize| ids.get(*s).cloned().unwrap_or_else(|| format!("#{s}"));
        match self {
            RejectReason::LimitReached { limit } => {
                format!("substitution limit reached ({limit} of {limit})")
            }
            RejectReason::NoWindowLeft { windows } => {
                format!("no substitution window left ({windows} of {windows})")
            }
            RejectReason::NotOnPitch { squad } => {
                format!("player {} is not on the pitch", id(squad))
            }
            RejectReason::SentOff { squad } => {
                format!("player {} was sent off and cannot be replaced", id(squad))
            }
            RejectReason::NotOnBench { squad } => {
                format!("player {} is not on the bench", id(squad))
            }
            RejectReason::LeftThePitch { squad } => format!(
                "player {} left the pitch at this stoppage; the substitution applies first",
                id(squad)
            ),
            RejectReason::OutOfRange => {
                "the change names a formation, mentality, level, role, or duty the tactics \
                 file does not hold"
                    .into()
            }
        }
    }
}

impl Simulation {
    /// Queues `change` for `team` and returns its identifier. It applies at the next
    /// stoppage the rule pack admits it at, or is rejected there with a reason.
    pub fn queue_change(&mut self, team: usize, change: Change) -> ChangeId {
        let id = ChangeId {
            tick: self.tick,
            n: self.queue.next,
        };
        self.queue.next += 1;
        self.summary.changes_queued += 1;
        tracing::debug!(signal = "change.queued", tick = self.tick, team, kind = change.kind().code(), queue_id = %id);
        self.queue.pending.push(QueuedChange { id, team, change });
        id
    }

    /// The changes still waiting, in queue order.
    pub fn pending_changes(&self) -> &[QueuedChange] {
        &self.queue.pending
    }

    /// The changes still waiting, split into never applied and expired.
    pub fn unapplied_changes(&self) -> Unapplied {
        self.queue.unapplied()
    }

    /// Each team's substitutions so far, home first.
    pub fn ledgers(&self) -> [SubLedger; 2] {
        self.ledgers
    }

    /// The substitution limit and window count in force: the rule pack's, plus its
    /// extra-time allowance once extra time has started (IFAB Law 3). The allowance follows
    /// from the clock, so a resumed match needs no stored limit.
    pub(crate) fn substitution_limits(&self) -> (u8, u8) {
        let rules = &self.config.rules;
        let (mut limit, mut windows) = (rules.substitutions.limit, rules.substitutions.windows);
        if self.referee.clock.in_extra_time() {
            limit += rules.extra_time.extra_substitutions;
            windows += rules.extra_time.extra_windows;
        }
        (limit, windows)
    }

    /// Applies the queue at the stoppage this tick opened. Nothing applies during the
    /// penalty shoot-out: a change queued then stays queued.
    pub(crate) fn apply_changes(&mut self, stoppage: Stoppage) {
        if self.referee.shootout.is_some() {
            return;
        }
        let (tactics_ok, subs_ok) = self.config.rules.admits(stoppage.kind);
        let now = self.tick + 1;
        for (admits, at) in [tactics_ok, subs_ok]
            .into_iter()
            .zip(&mut self.queue.admitted)
        {
            if admits {
                *at = Some(now);
            }
        }
        if self.queue.pending.is_empty() || (!tactics_ok && !subs_ok) {
            return;
        }
        let mut off_now: Vec<(usize, usize)> = Vec::new();
        let mut entered = [0usize; 2];
        let mut changed = false;
        if subs_ok {
            let mut keep = Vec::new();
            for q in std::mem::take(&mut self.queue.pending) {
                match q.change {
                    Change::Substitution { off, on } => {
                        match self.substitute(q.team, off, on, stoppage.kind, now, &mut entered) {
                            Ok(()) => {
                                off_now.push((q.team, off));
                                changed = true;
                                self.verdict(&q, now, stoppage.kind, None);
                            }
                            Err(reason) => self.verdict(&q, now, stoppage.kind, Some(reason)),
                        }
                    }
                    Change::Tactics(_) => keep.push(q),
                }
            }
            self.queue.pending = keep;
        }
        if tactics_ok {
            let mut keep = Vec::new();
            for q in std::mem::take(&mut self.queue.pending) {
                match &q.change {
                    Change::Tactics(patch) => {
                        let patch = patch.clone();
                        match self.retactic(q.team, &patch, &off_now) {
                            Ok(()) => {
                                changed = true;
                                self.verdict(&q, now, stoppage.kind, None);
                            }
                            Err(reason) => self.verdict(&q, now, stoppage.kind, Some(reason)),
                        }
                    }
                    Change::Substitution { .. } => keep.push(q),
                }
            }
            self.queue.pending = keep;
        }
        if !changed {
            return;
        }
        if !off_now.is_empty()
            && let Phase::DeadBall(mut dead) = self.referee.phase
        {
            // A substitute may have replaced the taker; the law's choice of taker stands.
            dead.taker =
                restart::taker(dead.kind, dead.team, dead.spot, &self.players, &self.teams);
            self.referee.phase = Phase::DeadBall(dead);
        }
        self.timeline.push((now, self.teams.clone()));
    }

    /// Records the verdict on `q` as an event, a count, and a signal.
    fn verdict(
        &mut self,
        q: &QueuedChange,
        now: u32,
        kind: StoppageKind,
        reason: Option<RejectReason>,
    ) {
        let event_kind = if reason.is_some() {
            EngineEventKind::ChangeRejected
        } else {
            EngineEventKind::ChangeApplied
        };
        let mut event = self.event(event_kind, Some(q.team));
        event.detail = Some(EventDetail::Change {
            id: q.id,
            kind: q.change.kind(),
            reason,
        });
        self.events.push(event);
        if self.trace_on() {
            let (point, why) = match reason {
                None => (Point::ChangeApplied, None),
                Some(r) => (
                    Point::ChangeRejected,
                    Some(r.text(&self.teams[q.team].player_ids)),
                ),
            };
            self.trace_point(
                point,
                json!({
                    "team": q.team,
                    "id": q.id.to_string(),
                    "change": format!("{:?}", q.change),
                    "stoppage": kind.code(),
                    "reason": why,
                }),
            );
        }
        match reason {
            None => {
                self.summary.changes_applied += 1;
                tracing::debug!(
                    signal = "change.applied",
                    tick = now,
                    team = q.team,
                    kind = q.change.kind().code(),
                    queue_id = %q.id,
                    waited_ticks = now.saturating_sub(q.id.tick),
                    stoppage = kind.code()
                );
            }
            Some(reason) => {
                self.summary.changes_rejected += 1;
                tracing::info!(
                    signal = "change.rejected",
                    tick = now,
                    team = q.team,
                    kind = q.change.kind().code(),
                    queue_id = %q.id,
                    reason = %reason.text(&self.teams[q.team].player_ids)
                );
            }
        }
    }

    /// Puts squad player `on` in the place of squad player `off`, or says why not.
    fn substitute(
        &mut self,
        team: usize,
        off: usize,
        on: usize,
        kind: StoppageKind,
        now: u32,
        entered: &mut [usize; 2],
    ) -> Result<(), RejectReason> {
        let (limit, windows) = self.substitution_limits();
        let rules = &self.config.rules.substitutions;
        let ledger = self.ledgers[team];
        if ledger.used >= limit {
            return Err(RejectReason::LimitReached { limit });
        }
        let needs_window = !rules.exempt(kind) && ledger.window_at != Some(now);
        if needs_window && ledger.windows >= windows {
            return Err(RejectReason::NoWindowLeft { windows });
        }
        let side = &self.teams[team];
        let slot = side
            .lineup
            .iter()
            .position(|&s| s == off)
            .ok_or(RejectReason::NotOnPitch { squad: off })?;
        let i = team * PLAYERS_PER_TEAM + slot;
        let leaving = self.players[i];
        if leaving.status == Status::SentOff {
            return Err(RejectReason::SentOff { squad: off });
        }
        if !side.bench.contains(&on) {
            return Err(RejectReason::NotOnBench { squad: on });
        }
        let ledger = &mut self.ledgers[team];
        ledger.used += 1;
        if needs_window {
            ledger.windows += 1;
            ledger.window_at = Some(now);
        }
        // At half-time the substitute takes the leaving player's kick-off place; otherwise it
        // enters at the halfway line on the near touchline, on its own side.
        let at = if kind == StoppageKind::HalfTime && leaving.active() {
            leaving.pos
        } else {
            let k = entered[team] as f64;
            entered[team] += 1;
            DVec2::new(
                -side.attack_x * (1.0 + 1.5 * k),
                -(pitch::HALF_WIDTH - ENTRY_MARGIN),
            )
        };
        let side = &mut self.teams[team];
        side.bench.retain(|&s| s != on);
        side.lineup[slot] = on;
        let mut incoming = side.player(slot, on, at);
        incoming.target = self.players[i].target;
        self.players[i] = incoming;
        // A goalkeeper coming on for an outfield player keeping goal takes over the goal, so
        // every line is laid out again after any substitution.
        let side = &mut self.teams[team];
        if side.active[slot] {
            side.relayout();
        } else {
            side.restore(slot);
        }
        self.summary.substitutions[team] += 1;
        let mut event = self.event(EngineEventKind::Substitution, Some(team));
        event.player = Some(i);
        event.detail = Some(EventDetail::Substitution { off, on });
        self.events.push(event);
        Ok(())
    }

    /// Applies a tactics change to `team`, or says why not.
    fn retactic(
        &mut self,
        team: usize,
        patch: &TacticsPatch,
        off_now: &[(usize, usize)],
    ) -> Result<(), RejectReason> {
        let schema = &self.config.tactics;
        if !patch.in_range(schema) {
            return Err(RejectReason::OutOfRange);
        }
        let side = &self.teams[team];
        for (squad, _) in &patch.roles {
            if off_now.contains(&(team, *squad)) {
                return Err(RejectReason::LeftThePitch { squad: *squad });
            }
            let on_pitch = side
                .lineup
                .iter()
                .position(|s| s == squad)
                .is_some_and(|slot| self.players[team * PLAYERS_PER_TEAM + slot].active());
            if !on_pitch {
                return Err(RejectReason::NotOnPitch { squad: *squad });
            }
        }
        let tactics = patch.applied_to(side.tactics, &side.lineup, schema);
        let tuning = self.config.tuning.clone();
        let schema = self.config.tactics.clone();
        let side = &mut self.teams[team];
        side.set_tactics(tactics, &schema, &tuning);
        tracing::debug!(
            signal = "tactics.plan",
            tick = self.tick + 1,
            team,
            formation = %schema.formations[usize::from(tactics.formation)].name,
            mentality = %schema.mentalities[usize::from(tactics.mentality)].name,
            press_count = side.plan.press_count,
            block_depth = side.plan.block_depth
        );
        Ok(())
    }
}

/// How far inside the touchline a substitute enters, in metres.
const ENTRY_MARGIN: f64 = 0.5;

#[cfg(test)]
mod tests {
    use super::*;

    fn queued(tick: u32, change: Change) -> QueuedChange {
        QueuedChange {
            id: ChangeId { tick, n: 0 },
            team: 0,
            change,
        }
    }

    #[test]
    fn a_change_an_admitting_stoppage_passed_is_never_applied_and_a_later_one_expired() {
        let sub = Change::Substitution { off: 3, on: 12 };
        let tactics = Change::Tactics(TacticsPatch::mentality(1));
        let queue = ChangeQueue {
            pending: vec![
                // A substitution stoppage opened on tick 501, after this was queued on 500.
                queued(500, sub.clone()),
                // Queued on 501, after that stoppage opened.
                queued(501, sub),
                // No stoppage ever admitted tactics.
                queued(100, tactics),
            ],
            next: 3,
            admitted: [None, Some(501)],
        };
        assert_eq!(
            queue.unapplied(),
            Unapplied {
                never_applied: 1,
                expired: 2
            }
        );
    }
}

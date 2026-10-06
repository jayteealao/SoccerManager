//! The control channel: start, pause, set speed, set the lineup, and queue a change. Every
//! verdict on a change is answered to the client and written as a `match-event` row, so a
//! change that is acknowledged and never applied is visible on the record.
//!
//! A session that waits for the page's lineup opens on a held gate: nothing is produced until
//! the first `start`, and `set-lineup` is accepted only before it. An admitted change is read
//! into the engine's own change type here, on the socket thread, and left in the [`Inbox`]
//! for the simulation thread, which queues it in the engine while the match runs.
//!
//! `cancel-change` withdraws a change a stoppage has not settled: from the inbox at once, or
//! from the engine's queue at the simulation thread's next hold. It is answered at once, and a
//! change already applied or refused is refused by name. A withdrawal writes no row: the
//! change's `queued` row stays, and no verdict row ever follows it.
//!
//! A viewer that reports the tick it has drawn (`seen`) bounds the engine's lead: once a lead
//! bound is set and a first report arrives, the producer waits while it is that many ticks
//! ahead of the drawn tick. A client that never reports is never held by it.
//!
//! A test seam, the fast-forward, lets a started match run flat out to a named tick: before
//! it, neither a pause nor the lead bound holds the producer; from it on, both hold as usual.
//! The simulation is the same either way; only when its ticks are sent changes. A test may
//! move that tick later in mid-match with `jump`, which the gate accepts only when the
//! session allows it (`serve --test-jump`) and only after the first `start`.
//!
//! A skip (`skip`) plays the rest of a started match at full speed: from it on, neither a
//! pause nor the lead bound holds the producer, so the same simulation steps on from its
//! exact state to full time and every tick and message streams as usual. Nothing is saved,
//! restored or rebuilt. A skip before kick-off is refused, and a second skip is answered again.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use engine::data::tactics::TacticsSchema;
use engine::{Change, RoleDuty, TacticsPatch};
use protocol::command::Verdict;
use protocol::{
    Ack, ChangeDetail, ChangeKind, ChangeOutcome, ChangeState, ClientCommand, MatchEvent,
    PatchWire, Queue, Reject, ServerMessage, SetLineup,
};

use crate::StreamError;
use crate::events::EventWriter;
use crate::session::MatchState;

/// Players in a lineup.
const STARTERS: usize = 11;

/// Start and pause, shared between the socket thread and the simulation thread.
pub struct Gate {
    state: Mutex<GateState>,
    changed: Condvar,
    /// The playback speed the client last asked for, in hundredths, so it is lock-free.
    speed_centis: AtomicU32,
}

struct GateState {
    running: bool,
    stopped: bool,
    /// `true` once the first `start` arrived, or from the outset for a gate that never holds.
    started: bool,
    /// The newest tick the client reported drawing, once it has reported one.
    seen: Option<u32>,
    /// The most ticks the producer may run ahead of `seen`; `None` leaves the lead unbounded.
    lead_bound: Option<u32>,
    /// A test seam: once started, the producer is not held before this tick.
    fast_forward_to: Option<u32>,
    /// `true` once the client skipped to the result: the producer never waits again.
    skipping: bool,
    /// A test seam: `true` when the session accepts `jump`.
    jump_allowed: bool,
}

/// What the gate did with a `jump`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpVerdict {
    /// The producer now runs flat out up to the target.
    Jumping,
    /// The target is at or behind the current tick: the gate is unchanged.
    Behind,
    /// The session does not accept `jump`: the gate is unchanged.
    NotAllowed,
    /// The match has not kicked off: the gate is unchanged.
    NotStarted,
}

impl GateState {
    /// `true` while producing the tick after `tick` must wait: the client paused, or the
    /// lead bound is reached. A started match that skipped, or is inside its fast-forward,
    /// never waits.
    fn holds(&self, tick: u32) -> bool {
        if self.started && self.skipping {
            return false;
        }
        if self.started && self.fast_forward_to.is_some_and(|to| tick < to) {
            return false;
        }
        !self.running || self.beyond_lead(tick)
    }

    /// `true` while producing the tick after `tick` would pass the lead bound.
    fn beyond_lead(&self, tick: u32) -> bool {
        match (self.seen, self.lead_bound) {
            (Some(seen), Some(bound)) => tick >= seen.saturating_add(bound),
            _ => false,
        }
    }
}

impl Gate {
    /// A gate that starts producing at once. A client that never sends `start` still
    /// receives the whole match.
    pub fn new() -> Self {
        Self::with(true)
    }

    /// A gate that holds before kick-off: nothing is produced until the first `start`.
    pub fn held() -> Self {
        Self::with(false)
    }

    fn with(running: bool) -> Self {
        Self {
            state: Mutex::new(GateState {
                running,
                stopped: false,
                started: running,
                seen: None,
                lead_bound: None,
                fast_forward_to: None,
                skipping: false,
                jump_allowed: false,
            }),
            changed: Condvar::new(),
            speed_centis: AtomicU32::new(100),
        }
    }

    /// Blocks while the client has paused or has not started. Returns `false` once the
    /// session is stopped.
    pub fn wait_until_running(&self) -> bool {
        let mut state = self.state.lock().expect("the gate lock is never poisoned");
        while !state.running && !state.stopped {
            state = self
                .changed
                .wait(state)
                .expect("the gate lock is never poisoned");
        }
        !state.stopped
    }

    /// Waits until the gate runs and, once the client has reported a drawn tick, until the
    /// tick after `tick` lies within the lead bound of it. `false` when the session ended.
    pub fn wait_for_room(&self, tick: u32) -> bool {
        let mut state = self.state.lock().expect("the gate lock is never poisoned");
        while !state.stopped && state.holds(tick) {
            state = self
                .changed
                .wait(state)
                .expect("the gate lock is never poisoned");
        }
        !state.stopped
    }

    /// Bounds the producer to `ticks` ahead of the newest tick the client reports drawing.
    pub fn set_lead_bound(&self, ticks: u32) {
        let mut state = self.state.lock().expect("the gate lock is never poisoned");
        state.lead_bound = Some(ticks);
        self.changed.notify_all();
    }

    /// A test seam: after the first `start`, produce every tick up to `tick` without waiting
    /// for a pause to end or for the drawn tick to catch up. The match is unchanged.
    pub fn set_fast_forward(&self, tick: u32) {
        let mut state = self.state.lock().expect("the gate lock is never poisoned");
        state.fast_forward_to = Some(tick);
        self.changed.notify_all();
    }

    /// A test seam: from now on the session accepts `jump`. Without it every jump is refused.
    pub fn allow_jump(&self) {
        let mut state = self.state.lock().expect("the gate lock is never poisoned");
        state.jump_allowed = true;
    }

    /// A test seam: moves the fast-forward of a started match on to `to`, so every tick up to
    /// it is produced without waiting for a pause to end or the drawn tick to catch up. `now`
    /// is the newest tick produced. A target at or behind it, or behind a fast-forward
    /// already set, changes nothing. A target past full time needs no clamp: the match ends
    /// there. The simulation is untouched.
    pub fn jump(&self, to: u32, now: u32) -> JumpVerdict {
        let mut state = self.state.lock().expect("the gate lock is never poisoned");
        if !state.jump_allowed {
            return JumpVerdict::NotAllowed;
        }
        if !state.started {
            return JumpVerdict::NotStarted;
        }
        if to <= now {
            return JumpVerdict::Behind;
        }
        state.fast_forward_to = Some(state.fast_forward_to.map_or(to, |at| at.max(to)));
        self.changed.notify_all();
        JumpVerdict::Jumping
    }

    /// The tick the producer runs flat out to, set at launch or moved by a jump.
    pub fn fast_forward_to(&self) -> Option<u32> {
        self.state
            .lock()
            .expect("the gate lock is never poisoned")
            .fast_forward_to
    }

    /// Skips a started match to its result: from now on neither a pause nor the lead bound
    /// holds the producer, so the match plays to full time at full speed. Refused (`false`,
    /// the gate unchanged) before the first `start`. The simulation is untouched.
    pub fn skip(&self) -> bool {
        let mut state = self.state.lock().expect("the gate lock is never poisoned");
        if !state.started {
            return false;
        }
        state.skipping = true;
        self.changed.notify_all();
        true
    }

    /// `true` once the client skipped to the result.
    pub fn skipping(&self) -> bool {
        self.state
            .lock()
            .expect("the gate lock is never poisoned")
            .skipping
    }

    /// The newest tick the client has drawn. A rewind reports a lower tick, and the bound
    /// follows it.
    pub fn set_seen(&self, tick: u32) {
        let mut state = self.state.lock().expect("the gate lock is never poisoned");
        state.seen = Some(tick);
        self.changed.notify_all();
    }

    /// Starts or pauses production.
    pub fn set_running(&self, running: bool) {
        let mut state = self.state.lock().expect("the gate lock is never poisoned");
        state.running = running;
        state.started |= running;
        self.changed.notify_all();
    }

    /// `true` once the match has been started.
    pub fn started(&self) -> bool {
        self.state
            .lock()
            .expect("the gate lock is never poisoned")
            .started
    }

    /// Ends the session; a waiting producer wakes and stops.
    pub fn stop(&self) {
        let mut state = self.state.lock().expect("the gate lock is never poisoned");
        state.stopped = true;
        self.changed.notify_all();
    }

    /// The playback speed the client last asked for.
    pub fn speed(&self) -> f32 {
        self.speed_centis.load(Ordering::Relaxed) as f32 / 100.0
    }

    /// Stores a playback speed, clamped to the range the viewer offers.
    pub fn set_speed(&self, speed: f32) -> f32 {
        let clamped = speed.clamp(0.25, 8.0);
        self.speed_centis
            .store((clamped * 100.0).round() as u32, Ordering::Relaxed);
        clamped
    }
}

impl Default for Gate {
    fn default() -> Self {
        Self::new()
    }
}

/// What the home lineup must satisfy before kick-off.
#[derive(Debug, Clone)]
pub struct LineupRules {
    /// `true` for each squad player whose position is goalkeeper, in squad order.
    pub keepers: Vec<bool>,
    /// The most substitutes a bench may name.
    pub bench_size: usize,
    /// The tactics file, for the indices a pre-match patch names.
    pub tactics: TacticsSchema,
}

/// The lineup the page chose, held until kick-off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSetup {
    /// Squad indices in slot order.
    pub lineup: [usize; STARTERS],
    pub bench: Vec<usize>,
    /// Pre-match tactics, applied before kick-off.
    pub patch: Option<TacticsPatch>,
}

/// The pre-match state a session shares with the code that builds the match after `start`.
#[derive(Debug, Default)]
pub struct PreMatch {
    rules: Option<LineupRules>,
    chosen: Mutex<Option<PageSetup>>,
}

impl PreMatch {
    /// A session that takes the home lineup from the page, under `rules`.
    pub fn new(rules: LineupRules) -> Self {
        Self {
            rules: Some(rules),
            chosen: Mutex::new(None),
        }
    }

    /// A session that takes no lineup: every `set-lineup` is refused.
    pub fn none() -> Self {
        Self::default()
    }

    /// The lineup the page chose, if it chose one. The computer manager's setup stands
    /// otherwise.
    pub fn take(&self) -> Option<PageSetup> {
        self.chosen
            .lock()
            .expect("the pre-match lock is never poisoned")
            .take()
    }
}

/// One change the socket admitted, waiting for the simulation thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    /// The identifier the client was given.
    pub queue_id: String,
    pub change: Change,
    /// The tick the socket admitted the change on: the tick its `queued` row carries.
    pub tick: u32,
}

/// The page's changes between the socket thread and the simulation thread: the changes the
/// socket admitted and the engine has not queued yet (oldest first), the identifiers the
/// engine holds, the withdrawals the engine has still to make, and the changes a stoppage
/// has settled.
///
/// The simulation thread holds the inbox ([`Inbox::hold`]) from the moment it queues and
/// withdraws changes until the step after it has settled, so a withdrawal the socket
/// acknowledges can never meet a change that the same step applies.
#[derive(Debug, Default)]
pub struct Inbox(Mutex<Mail>);

#[derive(Debug, Default)]
struct Mail {
    waiting: Vec<Admitted>,
    /// Identifiers the engine has queued and not settled.
    engine: Vec<String>,
    /// Identifiers withdrawn from the engine's queue at its next hold.
    cancels: Vec<String>,
    /// Identifiers a stoppage settled, with `true` for an applied change.
    settled: Vec<(String, bool)>,
}

/// The inbox, held by the simulation thread.
pub struct Held<'a>(std::sync::MutexGuard<'a, Mail>);

impl Held<'_> {
    /// Every waiting change, oldest first, leaving none waiting. The engine queues each one
    /// now, so a later withdrawal goes to the engine.
    pub fn drain(&mut self) -> Vec<Admitted> {
        let waiting = std::mem::take(&mut self.0.waiting);
        self.0
            .engine
            .extend(waiting.iter().map(|a| a.queue_id.clone()));
        waiting
    }

    /// The identifiers to withdraw from the engine's queue now.
    pub fn take_cancels(&mut self) -> Vec<String> {
        std::mem::take(&mut self.0.cancels)
    }

    /// Records that a stoppage applied (`true`) or refused the change `queue_id`.
    pub fn settle(&mut self, queue_id: &str, applied: bool) {
        self.0.engine.retain(|id| id != queue_id);
        self.0.settled.push((queue_id.to_string(), applied));
    }

    /// Marks changes a resumed match already holds in its queue, so the page can withdraw
    /// them over a new connection.
    pub fn adopt(&mut self, queue_ids: impl IntoIterator<Item = String>) {
        self.0.engine.extend(queue_ids);
    }
}

impl Inbox {
    pub fn push(&self, admitted: Admitted) {
        self.hold().0.waiting.push(admitted);
    }

    /// Every waiting change, oldest first, leaving the inbox empty.
    pub fn drain(&self) -> Vec<Admitted> {
        self.hold().drain()
    }

    /// The inbox, locked until the returned value drops.
    pub fn hold(&self) -> Held<'_> {
        Held(self.0.lock().expect("the inbox lock is never poisoned"))
    }

    /// Withdraws the change `queue_id`: at once when it still waits here, or at the engine's
    /// next hold when the engine has queued it. Refused, with the reason in words, for a
    /// change a stoppage settled or an identifier this connection never gave.
    pub fn cancel(&self, queue_id: &str) -> Result<(), String> {
        let mut held = self.hold();
        let mail = &mut held.0;
        if let Some(at) = mail.waiting.iter().position(|a| a.queue_id == queue_id) {
            mail.waiting.remove(at);
            return Ok(());
        }
        if let Some(at) = mail.engine.iter().position(|id| id == queue_id) {
            mail.engine.remove(at);
            mail.cancels.push(queue_id.to_string());
            return Ok(());
        }
        match mail.settled.iter().find(|(id, _)| id == queue_id) {
            Some((_, true)) => Err(format!("change {queue_id} has already applied")),
            Some((_, false)) => Err(format!("change {queue_id} was already refused")),
            None => Err(format!("unknown change {queue_id}")),
        }
    }
}

/// A wire patch as the engine's tactics change.
pub fn tactics_patch(wire: &PatchWire) -> TacticsPatch {
    TacticsPatch {
        formation: wire.formation,
        mentality: wire.mentality,
        instructions: wire.instructions.unwrap_or([None; 6]),
        roles: wire
            .roles
            .iter()
            .map(|r| {
                (
                    usize::from(r.squad),
                    RoleDuty {
                        role: r.role,
                        duty: r.duty,
                    },
                )
            })
            .collect(),
    }
}

/// A read change detail as the engine's change.
pub fn engine_change(detail: &ChangeDetail) -> Change {
    match detail {
        ChangeDetail::Patch(wire) => Change::Tactics(tactics_patch(wire)),
        ChangeDetail::Swap { off, on } => Change::Substitution {
            off: usize::from(*off),
            on: usize::from(*on),
        },
    }
}

/// The structural check of a page lineup: eleven starters, every index in the squad, no
/// player named twice, a bench within its size, and a goalkeeper in slot 0. The pre-match
/// patch must name tactics the file holds and roles only for starters. The football rules
/// that apply during play stay with the engine.
pub fn check_lineup(rules: &LineupRules, set: &SetLineup) -> Result<PageSetup, String> {
    let squad = rules.keepers.len();
    if set.lineup.len() != STARTERS {
        return Err(format!(
            "{} starters; a match needs {STARTERS}",
            set.lineup.len()
        ));
    }
    let mut seen = vec![false; squad];
    for &index in set.lineup.iter().chain(&set.bench) {
        let i = usize::from(index);
        if i >= squad {
            return Err(format!("squad index {i} is not in the squad of {squad}"));
        }
        if seen[i] {
            return Err(format!("squad index {i} is placed twice"));
        }
        seen[i] = true;
    }
    if set.bench.len() > rules.bench_size {
        return Err(format!(
            "{} substitutes; the bench holds {}",
            set.bench.len(),
            rules.bench_size
        ));
    }
    if !rules.keepers[usize::from(set.lineup[0])] {
        return Err("slot 0 needs a goalkeeper".into());
    }
    let lineup: [usize; STARTERS] = std::array::from_fn(|slot| usize::from(set.lineup[slot]));
    let patch = set.patch.as_ref().map(tactics_patch);
    if let Some(patch) = &patch {
        if !patch.in_range(&rules.tactics) {
            return Err(
                "the pre-match tactics name a formation, mentality, level, role, or \
                        duty the tactics file does not hold"
                    .into(),
            );
        }
        if let Some((squad, _)) = patch.roles.iter().find(|(s, _)| !lineup.contains(s)) {
            return Err(format!(
                "squad index {squad} has a pre-match role and is not in the lineup"
            ));
        }
    }
    Ok(PageSetup {
        lineup,
        bench: set.bench.iter().map(|&i| usize::from(i)).collect(),
        patch,
    })
}

/// Everything the socket thread needs to answer a command.
pub struct CommandContext {
    pub owner_id: String,
    pub match_id: String,
    pub gate: Arc<Gate>,
    pub state: Arc<MatchState>,
    pub events: Arc<Mutex<EventWriter>>,
    pub queue: Queue,
    /// The lineup rules and the stored lineup; [`PreMatch::none`] refuses every lineup.
    pub pre_match: Arc<PreMatch>,
    /// Where an admitted change waits for the engine.
    pub inbox: Arc<Inbox>,
}

impl CommandContext {
    /// Reads one JSON text frame and answers it. A queued change or a refused change also
    /// becomes a `match-event` row, written before the answer goes out.
    pub fn handle(
        &mut self,
        text: &str,
    ) -> Result<(ServerMessage, Option<MatchEvent>), StreamError> {
        let command: ClientCommand = match serde_json::from_str(text) {
            Ok(c) => c,
            Err(e) => {
                return Ok((
                    ServerMessage::Reject(Reject {
                        command: "unknown".into(),
                        reason: format!("cannot read the command: {e}"),
                    }),
                    None,
                ));
            }
        };
        let tick = self.state.tick();
        let ack = |command: &ClientCommand, speed: Option<f32>| {
            ServerMessage::Ack(Ack {
                command: command.name().into(),
                queue_id: None,
                queued_tick: tick,
                state: None,
                speed,
            })
        };
        let answer = match &command {
            ClientCommand::Start | ClientCommand::Pause => {
                self.gate
                    .set_running(matches!(command, ClientCommand::Start));
                (ack(&command, None), None)
            }
            ClientCommand::SetSpeed(set) => {
                let speed = self.gate.set_speed(set.speed);
                (ack(&command, Some(speed)), None)
            }
            ClientCommand::Seen(seen) => {
                self.gate.set_seen(seen.tick);
                (ack(&command, None), None)
            }
            ClientCommand::SetLineup(set) => (
                match self.set_lineup(set) {
                    Ok(()) => ack(&command, None),
                    Err(reason) => ServerMessage::Reject(Reject {
                        command: command.name().into(),
                        reason,
                    }),
                },
                None,
            ),
            ClientCommand::QueueChange(change) => self.queue_change(change, tick),
            ClientCommand::CancelChange(cancel) => (
                match self.inbox.cancel(&cancel.queue_id) {
                    Ok(()) => ServerMessage::Ack(Ack {
                        command: command.name().into(),
                        queue_id: Some(cancel.queue_id.clone()),
                        queued_tick: tick,
                        state: None,
                        speed: None,
                    }),
                    Err(reason) => ServerMessage::Reject(Reject {
                        command: command.name().into(),
                        reason,
                    }),
                },
                None,
            ),
            ClientCommand::Skip => (
                if self.gate.skip() {
                    tracing::info!(signal = "socket.skip", tick);
                    ack(&command, None)
                } else {
                    ServerMessage::Reject(Reject {
                        command: command.name().into(),
                        reason: "the match has not kicked off; there is nothing to skip".into(),
                    })
                },
                None,
            ),
            ClientCommand::Jump(jump) => (
                match self.gate.jump(jump.tick, tick) {
                    JumpVerdict::Jumping => {
                        tracing::info!(signal = "socket.jump", tick, to = jump.tick);
                        ack(&command, None)
                    }
                    JumpVerdict::Behind => {
                        tracing::info!(signal = "socket.jump_ignored", tick, to = jump.tick);
                        ack(&command, None)
                    }
                    JumpVerdict::NotAllowed => ServerMessage::Reject(Reject {
                        command: command.name().into(),
                        reason: "the engine was not started with --test-jump; jump is a test seam"
                            .into(),
                    }),
                    JumpVerdict::NotStarted => ServerMessage::Reject(Reject {
                        command: command.name().into(),
                        reason: "the match has not kicked off; there is nothing to jump".into(),
                    }),
                },
                None,
            ),
        };
        if let Some(event) = &answer.1 {
            self.events
                .lock()
                .expect("the event writer lock is never poisoned")
                .write(event)?;
        }
        Ok(answer)
    }

    fn set_lineup(&self, set: &SetLineup) -> Result<(), String> {
        if self.gate.started() {
            return Err("the match has started; a lineup can be set only before kick-off".into());
        }
        let rules = self
            .pre_match
            .rules
            .as_ref()
            .ok_or("this session takes no lineup from the page")?;
        let chosen = check_lineup(rules, set)?;
        *self
            .pre_match
            .chosen
            .lock()
            .expect("the pre-match lock is never poisoned") = Some(chosen);
        Ok(())
    }

    fn queue_change(
        &mut self,
        change: &protocol::QueueChange,
        tick: u32,
    ) -> (ServerMessage, Option<MatchEvent>) {
        let refused = |reason: String| {
            let event = MatchEvent::change(
                &self.owner_id,
                &self.match_id,
                tick,
                self.state.scores(),
                ChangeOutcome {
                    kind: ChangeKind::parse(&change.kind),
                    queue_id: None,
                    state: ChangeState::Rejected,
                    rejected_reason: Some(reason.clone()),
                },
            );
            (
                ServerMessage::Reject(Reject {
                    command: "queue-change".into(),
                    reason,
                }),
                Some(event),
            )
        };
        if !self.gate.started() {
            return refused(
                "the match has not started; set the pre-match tactics with the lineup".into(),
            );
        }
        // An unknown kind reads as no detail, and the queue refuses it by name.
        let detail = match change.detail_typed() {
            Some(Err(reason)) => return refused(reason),
            Some(Ok(detail)) => Some(detail),
            None => None,
        };
        match self.queue.submit(&change.kind, change.detail.clone(), tick) {
            Verdict::Ack(ack) => {
                if let (Some(detail), Some(queue_id)) = (&detail, &ack.queue_id) {
                    self.inbox.push(Admitted {
                        queue_id: queue_id.clone(),
                        change: engine_change(detail),
                        tick,
                    });
                }
                let event = MatchEvent::change(
                    &self.owner_id,
                    &self.match_id,
                    tick,
                    self.state.scores(),
                    ChangeOutcome {
                        kind: ChangeKind::parse(&change.kind),
                        queue_id: ack.queue_id.clone(),
                        state: ChangeState::Queued,
                        rejected_reason: None,
                    },
                );
                (ServerMessage::Ack(ack), Some(event))
            }
            Verdict::Reject(reject) => refused(reject.reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{ChangeKind, RoleWire};

    fn rules() -> LineupRules {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let dir = engine::ContentDir::resolve(Some(&dir)).unwrap();
        let content = engine::Content::load(&dir).unwrap();
        // Squad 0 and 11 are the goalkeepers, as in the shipped team files.
        let keepers = (0..22).map(|i| i == 0 || i == 11).collect();
        LineupRules {
            keepers,
            bench_size: 7,
            tactics: content.tactics,
        }
    }

    fn context(dir: &std::path::Path, gate: Gate, pre_match: PreMatch) -> CommandContext {
        CommandContext {
            owner_id: "0123456789abcdef0123456789abcdef".into(),
            match_id: "000000000000002a-1".into(),
            gate: Arc::new(gate),
            state: Arc::new(MatchState::default()),
            events: Arc::new(Mutex::new(
                EventWriter::open(dir, "000000000000002a-1").unwrap(),
            )),
            queue: Queue::new(ChangeKind::ALL.to_vec()),
            pre_match: Arc::new(pre_match),
            inbox: Arc::new(Inbox::default()),
        }
    }

    fn temp(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("stream-control-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn lineup(lineup: Vec<u16>, bench: Vec<u16>) -> String {
        serde_json::to_string(&ClientCommand::SetLineup(SetLineup {
            lineup,
            bench,
            patch: None,
        }))
        .unwrap()
    }

    fn reason(answer: ServerMessage) -> String {
        match answer {
            ServerMessage::Reject(r) => r.reason,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn pause_then_start_releases_a_waiting_producer() {
        let gate = Arc::new(Gate::new());
        assert!(gate.wait_until_running());
        gate.set_running(false);
        let waiter = Arc::clone(&gate);
        let handle = std::thread::spawn(move || waiter.wait_until_running());
        std::thread::sleep(std::time::Duration::from_millis(20));
        gate.set_running(true);
        assert!(handle.join().unwrap());
        gate.stop();
        assert!(!gate.wait_until_running());
    }

    #[test]
    fn a_reported_drawn_tick_bounds_the_lead_and_a_silent_client_is_not_held() {
        let gate = Arc::new(Gate::new());
        gate.set_lead_bound(500);
        assert!(
            gate.wait_for_room(10_000),
            "no report yet: the lead is unbounded"
        );
        gate.set_seen(1_000);
        assert!(gate.wait_for_room(1_499), "tick 1500 is within the bound");
        let waiter = Arc::clone(&gate);
        let handle = std::thread::spawn(move || waiter.wait_for_room(1_500));
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(!handle.is_finished(), "tick 1501 would pass the bound");
        gate.set_seen(1_001);
        assert!(handle.join().unwrap());
        let waiter = Arc::clone(&gate);
        let handle = std::thread::spawn(move || waiter.wait_for_room(9_000));
        std::thread::sleep(std::time::Duration::from_millis(20));
        gate.stop();
        assert!(
            !handle.join().unwrap(),
            "a stopped session releases the producer"
        );
    }

    #[test]
    fn a_fast_forward_runs_past_a_pause_and_the_lead_bound_up_to_its_tick_only() {
        let gate = Arc::new(Gate::held());
        gate.set_lead_bound(500);
        gate.set_fast_forward(9_000);
        let waiter = Arc::clone(&gate);
        let handle = std::thread::spawn(move || waiter.wait_for_room(10));
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(
            !handle.is_finished(),
            "before the first start nothing is produced"
        );
        gate.set_running(true);
        assert!(handle.join().unwrap());
        gate.set_seen(0);
        gate.set_running(false);
        assert!(
            gate.wait_for_room(8_999),
            "paused and 8999 ticks ahead: still produced"
        );
        let waiter = Arc::clone(&gate);
        let handle = std::thread::spawn(move || waiter.wait_for_room(9_000));
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(
            !handle.is_finished(),
            "from the fast-forward tick the pause holds again"
        );
        gate.set_seen(8_600);
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(
            !handle.is_finished(),
            "within the bound, the pause still holds"
        );
        gate.set_running(true);
        assert!(handle.join().unwrap());
        let waiter = Arc::clone(&gate);
        let handle = std::thread::spawn(move || waiter.wait_for_room(9_100));
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(
            !handle.is_finished(),
            "past the fast-forward the lead bound holds again"
        );
        gate.stop();
        assert!(!handle.join().unwrap());
    }

    #[test]
    fn a_skip_runs_past_a_pause_and_the_lead_bound_to_the_end() {
        let gate = Arc::new(Gate::held());
        gate.set_lead_bound(1);
        gate.set_running(true);
        gate.set_seen(1_000);
        gate.set_running(false);
        let waiter = Arc::clone(&gate);
        let handle = std::thread::spawn(move || waiter.wait_for_room(1_001));
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(
            !handle.is_finished(),
            "paused at the bound: the producer waits"
        );
        assert!(!gate.skipping());
        assert!(gate.skip());
        assert!(handle.join().unwrap(), "the skip releases the producer");
        assert!(gate.skipping());
        for tick in [1_002, 50_000, 269_999] {
            assert!(gate.wait_for_room(tick), "tick {tick} never waits");
        }
        gate.set_seen(0);
        gate.set_running(false);
        assert!(
            gate.wait_for_room(200_000),
            "a later pause or seen has no effect"
        );
        assert!(gate.skip(), "a second skip is answered again");
        gate.stop();
        assert!(!gate.wait_for_room(200_001), "a stopped session still ends");
    }

    #[test]
    fn a_skip_before_kick_off_is_refused_and_the_gate_still_holds() {
        let dir = temp("skip-early");
        let mut ctx = context(&dir, Gate::held(), PreMatch::none());
        let (answer, event) = ctx.handle("{\"type\":\"skip\"}").unwrap();
        assert!(event.is_none());
        let ServerMessage::Reject(reject) = answer else {
            panic!("a skip before kick-off must be refused: {answer:?}");
        };
        assert_eq!(reject.command, "skip");
        assert_eq!(
            reject.reason,
            "the match has not kicked off; there is nothing to skip"
        );
        assert!(!ctx.gate.skipping());
        let waiter = Arc::clone(&ctx.gate);
        let handle = std::thread::spawn(move || waiter.wait_for_room(0));
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(
            !handle.is_finished(),
            "the gate still holds before kick-off"
        );
        ctx.gate.stop();
        assert!(!handle.join().unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_skip_after_kick_off_is_acknowledged_through_handle() {
        let dir = temp("skip");
        let mut ctx = context(&dir, Gate::held(), PreMatch::none());
        ctx.handle("{\"type\":\"start\"}").unwrap();
        ctx.state.set_tick(90_000);
        for _ in 0..2 {
            let (answer, event) = ctx.handle("{\"type\":\"skip\"}").unwrap();
            assert!(event.is_none(), "a skip writes no row");
            let ServerMessage::Ack(ack) = answer else {
                panic!("a skip after kick-off must be acknowledged: {answer:?}");
            };
            assert_eq!(ack.command, "skip");
            assert_eq!(ack.queued_tick, 90_000);
        }
        assert!(ctx.gate.skipping());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn jump(ctx: &mut CommandContext, tick: u32) -> ServerMessage {
        let text = serde_json::to_string(&ClientCommand::Jump(protocol::Jump { tick })).unwrap();
        let (answer, event) = ctx.handle(&text).unwrap();
        assert!(event.is_none(), "a jump writes no row");
        answer
    }

    /// `true` while a producer asking to pass `tick` is still held after a short wait. The
    /// waiting thread is released by stopping the gate.
    fn still_holds(gate: &Arc<Gate>, tick: u32) -> bool {
        let waiter = Arc::clone(gate);
        let handle = std::thread::spawn(move || waiter.wait_for_room(tick));
        std::thread::sleep(std::time::Duration::from_millis(30));
        let held = !handle.is_finished();
        gate.stop();
        let _ = handle.join();
        held
    }

    #[test]
    fn a_jump_without_the_test_flag_is_refused_and_the_pause_still_holds() {
        let dir = temp("jump-refused");
        let mut ctx = context(&dir, Gate::held(), PreMatch::none());
        ctx.gate.set_lead_bound(500);
        ctx.handle("{\"type\":\"start\"}").unwrap();
        ctx.gate.set_seen(3_000);
        ctx.handle("{\"type\":\"pause\"}").unwrap();
        ctx.state.set_tick(3_000);
        let ServerMessage::Reject(reject) = jump(&mut ctx, 20_000) else {
            panic!("a jump without --test-jump must be refused");
        };
        assert_eq!(reject.command, "jump");
        assert_eq!(
            reject.reason,
            "the engine was not started with --test-jump; jump is a test seam"
        );
        assert_eq!(ctx.gate.fast_forward_to(), None);
        assert!(
            still_holds(&ctx.gate, 3_001),
            "the pause still holds the producer"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_jump_before_kick_off_is_refused() {
        let dir = temp("jump-early");
        let mut ctx = context(&dir, Gate::held(), PreMatch::none());
        ctx.gate.allow_jump();
        let ServerMessage::Reject(reject) = jump(&mut ctx, 20_000) else {
            panic!("a jump before kick-off must be refused");
        };
        assert_eq!(reject.command, "jump");
        assert_eq!(
            reject.reason,
            "the match has not kicked off; there is nothing to jump"
        );
        assert_eq!(ctx.gate.fast_forward_to(), None);
        assert!(
            still_holds(&ctx.gate, 0),
            "the gate still holds before kick-off"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_jump_behind_the_current_tick_is_acknowledged_and_changes_nothing() {
        let dir = temp("jump-behind");
        let mut ctx = context(&dir, Gate::held(), PreMatch::none());
        ctx.gate.allow_jump();
        ctx.handle("{\"type\":\"start\"}").unwrap();
        ctx.handle("{\"type\":\"pause\"}").unwrap();
        ctx.state.set_tick(5_000);
        for to in [4_000, 5_000] {
            let ServerMessage::Ack(ack) = jump(&mut ctx, to) else {
                panic!("a jump to {to} at tick 5000 is acknowledged");
            };
            assert_eq!(ack.command, "jump");
            assert_eq!(ack.queued_tick, 5_000);
        }
        assert_eq!(ctx.gate.fast_forward_to(), None);
        assert!(
            still_holds(&ctx.gate, 5_000),
            "the paused producer still holds"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_jump_runs_past_a_pause_and_the_lead_bound_up_to_its_tick_only() {
        let dir = temp("jump");
        let mut ctx = context(&dir, Gate::held(), PreMatch::none());
        ctx.gate.allow_jump();
        ctx.gate.set_lead_bound(500);
        ctx.handle("{\"type\":\"start\"}").unwrap();
        ctx.gate.set_seen(3_000);
        ctx.handle("{\"type\":\"pause\"}").unwrap();
        ctx.state.set_tick(3_000);
        let gate = Arc::clone(&ctx.gate);
        let waiter = Arc::clone(&gate);
        let handle = std::thread::spawn(move || waiter.wait_for_room(3_000));
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(!handle.is_finished(), "paused: the producer waits");
        let ServerMessage::Ack(ack) = jump(&mut ctx, 9_000) else {
            panic!("a jump after kick-off with the flag is acknowledged");
        };
        assert_eq!(ack.queued_tick, 3_000);
        assert!(handle.join().unwrap(), "the jump releases the producer");
        assert_eq!(gate.fast_forward_to(), Some(9_000));
        assert!(
            gate.wait_for_room(8_999),
            "paused and far past the bound: still produced"
        );
        // A later, shorter jump never pulls the target back.
        ctx.state.set_tick(4_000);
        jump(&mut ctx, 6_000);
        assert_eq!(gate.fast_forward_to(), Some(9_000));
        assert!(
            still_holds(&gate, 9_000),
            "from the target tick the pause holds again"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_jump_past_full_time_never_holds_again_before_it() {
        let gate = Arc::new(Gate::held());
        gate.allow_jump();
        gate.set_lead_bound(1);
        gate.set_running(true);
        gate.set_seen(100);
        gate.set_running(false);
        assert_eq!(gate.jump(u32::MAX, 100), JumpVerdict::Jumping);
        for tick in [101, 50_000, 269_999, u32::MAX - 1] {
            assert!(gate.wait_for_room(tick), "tick {tick} never waits");
        }
        gate.stop();
    }

    #[test]
    fn a_held_gate_produces_nothing_until_the_first_start() {
        let gate = Arc::new(Gate::held());
        assert!(!gate.started());
        let waiter = Arc::clone(&gate);
        let handle = std::thread::spawn(move || waiter.wait_until_running());
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(!handle.is_finished(), "a held gate must block the producer");
        gate.set_running(true);
        assert!(handle.join().unwrap());
        assert!(gate.started());
        gate.set_running(false);
        assert!(
            gate.started(),
            "a pause after kick-off does not undo the start"
        );
        assert!(Gate::new().started());
    }

    #[test]
    fn set_speed_is_stored_and_echoed() {
        let dir = temp("speed");
        let mut ctx = context(&dir, Gate::new(), PreMatch::none());
        let (answer, event) = ctx
            .handle("{\"type\":\"set-speed\",\"speed\":8.0}")
            .unwrap();
        assert!(event.is_none());
        let ServerMessage::Ack(ack) = answer else {
            panic!("set-speed must be acknowledged");
        };
        assert_eq!(ack.speed, Some(8.0));
        assert_eq!(ctx.gate.speed(), 8.0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unreadable_command_is_refused_without_an_event() {
        let dir = temp("garbage");
        let mut ctx = context(&dir, Gate::new(), PreMatch::none());
        let (answer, event) = ctx.handle("not json").unwrap();
        assert!(event.is_none());
        let ServerMessage::Reject(reject) = answer else {
            panic!("garbage must be refused");
        };
        assert_eq!(reject.command, "unknown");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_legal_lineup_is_stored_before_kick_off_and_refused_after_it() {
        let dir = temp("lineup");
        let mut ctx = context(&dir, Gate::held(), PreMatch::new(rules()));
        let starters: Vec<u16> = vec![11, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let (answer, _) = ctx.handle(&lineup(starters.clone(), vec![0, 12])).unwrap();
        let ServerMessage::Ack(ack) = answer else {
            panic!("a legal lineup must be acknowledged: {answer:?}");
        };
        assert_eq!(ack.command, "set-lineup");
        ctx.handle("{\"type\":\"start\"}").unwrap();
        let (answer, _) = ctx.handle(&lineup(starters, vec![0])).unwrap();
        assert_eq!(
            reason(answer),
            "the match has started; a lineup can be set only before kick-off"
        );
        let chosen = ctx.pre_match.take().expect("the first lineup is kept");
        assert_eq!(chosen.lineup[0], 11);
        assert_eq!(chosen.bench, vec![0, 12]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_illegal_lineup_is_refused_with_its_reason() {
        let dir = temp("illegal");
        let mut ctx = context(&dir, Gate::held(), PreMatch::new(rules()));
        let eleven: Vec<u16> = (0..11).collect();
        let cases = [
            (
                lineup((0..10).collect(), vec![]),
                "10 starters; a match needs 11",
            ),
            (
                lineup(vec![1, 0, 2, 3, 4, 5, 6, 7, 8, 9, 10], vec![]),
                "slot 0 needs a goalkeeper",
            ),
            (
                lineup(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 9], vec![]),
                "squad index 9 is placed twice",
            ),
            (
                lineup(eleven.clone(), vec![3]),
                "squad index 3 is placed twice",
            ),
            (
                lineup(eleven.clone(), (11..19).collect()),
                "8 substitutes; the bench holds 7",
            ),
            (
                lineup(eleven.clone(), vec![40]),
                "squad index 40 is not in the squad of 22",
            ),
        ];
        for (command, expected) in cases {
            let (answer, event) = ctx.handle(&command).unwrap();
            assert!(event.is_none());
            assert_eq!(reason(answer), expected);
        }
        let with_role = serde_json::to_string(&ClientCommand::SetLineup(SetLineup {
            lineup: eleven,
            bench: vec![],
            patch: Some(PatchWire {
                roles: vec![RoleWire {
                    squad: 15,
                    role: 0,
                    duty: 0,
                }],
                ..PatchWire::default()
            }),
        }))
        .unwrap();
        let (answer, _) = ctx.handle(&with_role).unwrap();
        assert_eq!(
            reason(answer),
            "squad index 15 has a pre-match role and is not in the lineup"
        );
        assert!(ctx.pre_match.take().is_none());
        let (answer, _) = context(&dir, Gate::held(), PreMatch::none())
            .handle(&lineup((0..11).collect(), vec![]))
            .unwrap();
        assert_eq!(reason(answer), "this session takes no lineup from the page");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_admitted_change_reaches_the_inbox_with_its_acknowledged_id() {
        let dir = temp("queue");
        let mut ctx = context(&dir, Gate::new(), PreMatch::none());
        ctx.state.set_tick(1_200);
        let (answer, event) = ctx
            .handle(
                "{\"type\":\"queue-change\",\"change.kind\":\"tactics\",\
                 \"detail\":{\"patch\":{\"mentality\":4}}}",
            )
            .unwrap();
        let event = event.expect("a queued change is on the record");
        assert_eq!(event.change_queue_id.as_deref(), Some("q-1200-0"));
        assert_eq!(event.change_state, Some(ChangeState::Queued));
        let ServerMessage::Ack(ack) = answer else {
            panic!("a readable change must be acknowledged");
        };
        let (_, _) = ctx
            .handle(
                "{\"type\":\"queue-change\",\"change.kind\":\"substitution\",\
                 \"detail\":{\"off\":9,\"on\":14}}",
            )
            .unwrap();
        let waiting = ctx.inbox.drain();
        assert_eq!(waiting.len(), 2);
        assert_eq!(Some(waiting[0].queue_id.clone()), ack.queue_id);
        assert_eq!(
            waiting[0].change,
            Change::Tactics(TacticsPatch::mentality(4))
        );
        assert_eq!(waiting[1].change, Change::Substitution { off: 9, on: 14 });
        assert!(ctx.inbox.drain().is_empty());

        let (answer, event) = ctx
            .handle("{\"type\":\"queue-change\",\"change.kind\":\"formation\"}")
            .unwrap();
        let event = event.expect("a refused change is on the record");
        assert_eq!(
            event.change_rejected_reason.as_deref(),
            Some("unknown change type formation")
        );
        assert_eq!(reason(answer), "unknown change type formation");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn cancel(id: &str) -> String {
        serde_json::to_string(&ClientCommand::CancelChange(protocol::CancelChange {
            queue_id: id.into(),
        }))
        .unwrap()
    }

    fn queue_sub(ctx: &mut CommandContext) -> String {
        let (answer, _) = ctx
            .handle(
                "{\"type\":\"queue-change\",\"change.kind\":\"substitution\",\
                 \"detail\":{\"off\":9,\"on\":14}}",
            )
            .unwrap();
        let ServerMessage::Ack(ack) = answer else {
            panic!("a readable change must be acknowledged: {answer:?}");
        };
        ack.queue_id.expect("a queued change has an identifier")
    }

    #[test]
    fn a_change_still_in_the_inbox_is_withdrawn_at_once() {
        let dir = temp("cancel-inbox");
        let mut ctx = context(&dir, Gate::new(), PreMatch::none());
        let id = queue_sub(&mut ctx);
        let (answer, event) = ctx.handle(&cancel(&id)).unwrap();
        assert!(event.is_none(), "a withdrawal writes no row");
        let ServerMessage::Ack(ack) = answer else {
            panic!("a waiting change must be withdrawn: {answer:?}");
        };
        assert_eq!(ack.command, "cancel-change");
        assert_eq!(ack.queue_id.as_deref(), Some(id.as_str()));
        let mut held = ctx.inbox.hold();
        assert!(held.drain().is_empty(), "the engine never sees it");
        assert!(
            held.take_cancels().is_empty(),
            "nothing is left to withdraw"
        );
        drop(held);
        assert_eq!(
            reason(ctx.handle(&cancel(&id)).unwrap().0),
            format!("unknown change {id}")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_change_the_engine_queued_is_withdrawn_at_its_next_hold() {
        let dir = temp("cancel-engine");
        let mut ctx = context(&dir, Gate::new(), PreMatch::none());
        let id = queue_sub(&mut ctx);
        assert_eq!(ctx.inbox.hold().drain().len(), 1, "the engine queues it");
        let (answer, _) = ctx.handle(&cancel(&id)).unwrap();
        assert!(matches!(answer, ServerMessage::Ack(_)), "{answer:?}");
        assert_eq!(ctx.inbox.hold().take_cancels(), vec![id.clone()]);
        assert!(ctx.inbox.hold().take_cancels().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_change_a_stoppage_settled_cannot_be_withdrawn() {
        let dir = temp("cancel-settled");
        let mut ctx = context(&dir, Gate::new(), PreMatch::none());
        let applied = queue_sub(&mut ctx);
        let refused = queue_sub(&mut ctx);
        {
            let mut held = ctx.inbox.hold();
            held.drain();
            held.settle(&applied, true);
            held.settle(&refused, false);
        }
        assert_eq!(
            reason(ctx.handle(&cancel(&applied)).unwrap().0),
            format!("change {applied} has already applied")
        );
        assert_eq!(
            reason(ctx.handle(&cancel(&refused)).unwrap().0),
            format!("change {refused} was already refused")
        );
        assert!(ctx.inbox.hold().take_cancels().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_change_the_socket_cannot_read_is_refused_and_reaches_no_inbox() {
        let dir = temp("unreadable");
        let mut ctx = context(&dir, Gate::new(), PreMatch::none());
        let (answer, event) = ctx
            .handle(
                "{\"type\":\"queue-change\",\"change.kind\":\"substitution\",\
                 \"detail\":{\"out\":9,\"in\":14}}",
            )
            .unwrap();
        let text = reason(answer);
        assert!(
            text.starts_with("cannot read the substitution change"),
            "{text}"
        );
        let event = event.expect("a refused change is on the record");
        assert_eq!(event.change_state, Some(ChangeState::Rejected));
        assert!(ctx.inbox.drain().is_empty());

        let mut held = context(&dir, Gate::held(), PreMatch::new(rules()));
        let (answer, _) = held
            .handle(
                "{\"type\":\"queue-change\",\"change.kind\":\"tactics\",\
                 \"detail\":{\"patch\":{\"mentality\":4}}}",
            )
            .unwrap();
        assert_eq!(
            reason(answer),
            "the match has not started; set the pre-match tactics with the lineup"
        );
        assert!(held.inbox.drain().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

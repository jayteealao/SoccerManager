//! The control channel: start, pause, set speed, set the lineup, and queue a change. Every
//! verdict on a change is answered to the client and written as a `match-event` row, so a
//! change that is acknowledged and never applied is visible on the record.
//!
//! A session that waits for the page's lineup opens on a held gate: nothing is produced until
//! the first `start`, and `set-lineup` is accepted only before it. An admitted change is read
//! into the engine's own change type here, on the socket thread, and left in the [`Inbox`]
//! for the simulation thread, which queues it in the engine while the match runs.
//!
//! A viewer that reports the tick it has drawn (`seen`) bounds the engine's lead: once a lead
//! bound is set and a first report arrives, the producer waits while it is that many ticks
//! ahead of the drawn tick. A client that never reports is never held by it.

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
}

impl GateState {
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
        while !state.stopped && (!state.running || state.beyond_lead(tick)) {
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
}

/// The changes the socket admitted and the engine has not queued yet, oldest first.
#[derive(Debug, Default)]
pub struct Inbox(Mutex<Vec<Admitted>>);

impl Inbox {
    pub fn push(&self, admitted: Admitted) {
        self.0
            .lock()
            .expect("the inbox lock is never poisoned")
            .push(admitted);
    }

    /// Every waiting change, oldest first, leaving the inbox empty.
    pub fn drain(&self) -> Vec<Admitted> {
        std::mem::take(&mut *self.0.lock().expect("the inbox lock is never poisoned"))
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

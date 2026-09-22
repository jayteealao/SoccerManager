//! The control channel: start, pause, set speed, and queue a change. Every verdict is
//! answered to the client and written as a `match-event` row, so a change that is
//! acknowledged and never applied is visible on the record.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use protocol::command::Verdict;
use protocol::{
    Ack, ChangeKind, ChangeOutcome, ChangeState, ClientCommand, MatchEvent, Queue, Reject,
    ServerMessage,
};

use crate::StreamError;
use crate::events::EventWriter;
use crate::session::MatchState;

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
}

impl Gate {
    /// A gate that starts producing at once. A client that never sends `start` still
    /// receives the whole match.
    pub fn new() -> Self {
        Self {
            state: Mutex::new(GateState {
                running: true,
                stopped: false,
            }),
            changed: Condvar::new(),
            speed_centis: AtomicU32::new(100),
        }
    }

    /// Blocks while the client has paused. Returns `false` once the session is stopped.
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

    /// Starts or pauses production.
    pub fn set_running(&self, running: bool) {
        let mut state = self.state.lock().expect("the gate lock is never poisoned");
        state.running = running;
        self.changed.notify_all();
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

/// Everything the socket thread needs to answer a command.
pub struct CommandContext {
    pub owner_id: String,
    pub match_id: String,
    pub gate: Arc<Gate>,
    pub state: Arc<MatchState>,
    pub events: Arc<Mutex<EventWriter>>,
    pub queue: Queue,
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
        let scores = self.state.scores();
        let answer = match &command {
            ClientCommand::Start | ClientCommand::Pause => {
                self.gate
                    .set_running(matches!(command, ClientCommand::Start));
                (
                    ServerMessage::Ack(Ack {
                        command: command.name().into(),
                        queue_id: None,
                        queued_tick: tick,
                        state: None,
                        speed: None,
                    }),
                    None,
                )
            }
            ClientCommand::SetSpeed(set) => {
                let speed = self.gate.set_speed(set.speed);
                (
                    ServerMessage::Ack(Ack {
                        command: command.name().into(),
                        queue_id: None,
                        queued_tick: tick,
                        state: None,
                        speed: Some(speed),
                    }),
                    None,
                )
            }
            ClientCommand::QueueChange(change) => {
                match self.queue.submit(&change.kind, change.detail.clone(), tick) {
                    Verdict::Ack(ack) => {
                        let event = MatchEvent::change(
                            &self.owner_id,
                            &self.match_id,
                            tick,
                            scores,
                            ChangeOutcome {
                                kind: ChangeKind::parse(&change.kind),
                                queue_id: ack.queue_id.clone(),
                                state: ChangeState::Queued,
                                rejected_reason: None,
                            },
                        );
                        (ServerMessage::Ack(ack), Some(event))
                    }
                    Verdict::Reject(reject) => {
                        let event = MatchEvent::change(
                            &self.owner_id,
                            &self.match_id,
                            tick,
                            scores,
                            ChangeOutcome {
                                kind: ChangeKind::parse(&change.kind),
                                queue_id: None,
                                state: ChangeState::Rejected,
                                rejected_reason: Some(reject.reason.clone()),
                            },
                        );
                        (ServerMessage::Reject(reject), Some(event))
                    }
                }
            }
        };
        if let Some(event) = &answer.1 {
            self.events
                .lock()
                .expect("the event writer lock is never poisoned")
                .write(event)?;
        }
        Ok(answer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::ChangeKind;

    fn context(dir: &std::path::Path) -> CommandContext {
        CommandContext {
            owner_id: "0123456789abcdef0123456789abcdef".into(),
            match_id: "000000000000002a-1".into(),
            gate: Arc::new(Gate::new()),
            state: Arc::new(MatchState::default()),
            events: Arc::new(Mutex::new(
                EventWriter::open(dir, "000000000000002a-1").unwrap(),
            )),
            queue: Queue::new(ChangeKind::ALL.to_vec()),
        }
    }

    fn temp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("stream-control-{}-{name}", std::process::id()))
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
    fn set_speed_is_stored_and_echoed() {
        let dir = temp("speed");
        let _ = std::fs::remove_dir_all(&dir);
        let mut ctx = context(&dir);
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
        let _ = std::fs::remove_dir_all(&dir);
        let mut ctx = context(&dir);
        let (answer, event) = ctx.handle("not json").unwrap();
        assert!(event.is_none());
        let ServerMessage::Reject(reject) = answer else {
            panic!("garbage must be refused");
        };
        assert_eq!(reject.command, "unknown");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_queued_change_writes_a_match_event_row() {
        let dir = temp("queue");
        let _ = std::fs::remove_dir_all(&dir);
        let mut ctx = context(&dir);
        ctx.state.set_tick(1_200);
        let (_, event) = ctx
            .handle("{\"type\":\"queue-change\",\"change.kind\":\"tactics\"}")
            .unwrap();
        let event = event.expect("a queued change is on the record");
        assert_eq!(event.change_queue_id.as_deref(), Some("q-1200-0"));
        assert_eq!(event.change_state, Some(ChangeState::Queued));

        let (answer, event) = ctx
            .handle("{\"type\":\"queue-change\",\"change.kind\":\"formation\"}")
            .unwrap();
        let event = event.expect("a refused change is on the record");
        assert_eq!(
            event.change_rejected_reason.as_deref(),
            Some("unknown change type formation")
        );
        let ServerMessage::Reject(reject) = answer else {
            panic!("an unknown kind must be refused");
        };
        assert_eq!(reject.reason, "unknown change type formation");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

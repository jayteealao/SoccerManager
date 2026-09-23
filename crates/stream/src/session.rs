//! One client session: the bounded producer buffer, the encoder on the simulation thread,
//! and the socket thread that reads commands and writes frames.
//!
//! The buffer is a `sync_channel`. When it fills, `send` blocks the simulation thread until
//! the socket thread drains it, which is the backpressure: no tick is dropped and memory
//! stays flat. The socket has a bound of its own, so a write that would exceed it answers
//! `WriteBufferFull`, and the session treats that as the same pause.

use std::collections::VecDeque;
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use engine::EngineError;
use engine::record::{TickRecord, TickSink};
use protocol::{Frame, Hello, Quantised, ServerMessage, TickFrame};
use tungstenite::{Message, WebSocket};

use crate::control::CommandContext;
use crate::events::EventWriter;
use crate::server::Connection;
use crate::{StreamError, peer_gone, would_block};

/// Messages the socket thread holds before it writes them.
const OUTBOX_LIMIT: usize = 32;
/// How long the socket thread sleeps when neither direction has work.
const IDLE_SLEEP: Duration = Duration::from_micros(100);

/// The tick and the score, shared between the simulation thread and the socket thread.
#[derive(Debug, Default)]
pub struct MatchState {
    tick: AtomicU32,
    /// The home score in the high 32 bits, the away score in the low 32.
    scores: AtomicU64,
    /// The newest tick frame the socket thread has flushed to the stream.
    sent_tick: AtomicU32,
}

impl MatchState {
    /// The newest tick whose frame the socket has flushed to the stream, or 0 before the
    /// first. Every message queued before that frame was flushed with it or earlier, because
    /// the producer buffer keeps order.
    pub fn sent_tick(&self) -> u32 {
        self.sent_tick.load(Ordering::Acquire)
    }

    /// Records a flushed tick frame. The socket thread is the only writer.
    pub fn set_sent_tick(&self, tick: u32) {
        self.sent_tick.store(tick, Ordering::Release);
    }

    pub fn tick(&self) -> u32 {
        self.tick.load(Ordering::Relaxed)
    }

    pub fn set_tick(&self, tick: u32) {
        self.tick.store(tick, Ordering::Relaxed);
    }

    pub fn scores(&self) -> [u32; 2] {
        let packed = self.scores.load(Ordering::Relaxed);
        [(packed >> 32) as u32, packed as u32]
    }

    pub fn set_scores(&self, scores: [u32; 2]) {
        self.scores.store(
            u64::from(scores[0]) << 32 | u64::from(scores[1]),
            Ordering::Relaxed,
        );
    }
}

/// What the producer buffer did during a match.
#[derive(Debug, Default)]
pub struct Gauge {
    bound: AtomicUsize,
    depth: AtomicUsize,
    high_water: AtomicUsize,
    pauses: AtomicU32,
    paused_ms: AtomicU64,
}

impl Gauge {
    /// A gauge for a buffer of `bound` ticks.
    pub fn new(bound: usize) -> Self {
        let gauge = Self::default();
        gauge.bound.store(bound, Ordering::Relaxed);
        gauge
    }

    /// The buffer bound.
    pub fn bound(&self) -> usize {
        self.bound.load(Ordering::Relaxed)
    }

    /// The deepest the buffer ever was.
    pub fn high_water(&self) -> usize {
        self.high_water.load(Ordering::Relaxed)
    }

    /// How many times the producer paused at the bound.
    pub fn pauses(&self) -> u32 {
        self.pauses.load(Ordering::Relaxed)
    }

    /// How long the producer spent paused, in milliseconds.
    pub fn paused_ms(&self) -> u64 {
        self.paused_ms.load(Ordering::Relaxed)
    }

    fn entered(&self) {
        let depth = self.depth.fetch_add(1, Ordering::Relaxed) + 1;
        self.high_water.fetch_max(depth, Ordering::Relaxed);
    }

    fn left(&self) {
        self.depth.fetch_sub(1, Ordering::Relaxed);
    }

    fn paused(&self, ms: u64) -> u32 {
        self.paused_ms.fetch_add(ms, Ordering::Relaxed);
        self.pauses.fetch_add(1, Ordering::Relaxed) + 1
    }
}

/// Where encoded frames go.
pub trait FrameOut {
    fn send(&mut self, frame: Frame) -> Result<(), StreamError>;
}

/// The bounded producer buffer. A full buffer blocks the caller.
pub struct ChannelOut {
    tx: SyncSender<Frame>,
    gauge: Arc<Gauge>,
    state: Arc<MatchState>,
}

impl FrameOut for ChannelOut {
    fn send(&mut self, frame: Frame) -> Result<(), StreamError> {
        // The depth is counted before the frame enters the buffer. The consumer only
        // decrements a frame it has taken out, so the count never goes below zero.
        self.gauge.entered();
        match self.tx.try_send(frame) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(frame)) => {
                let started = Instant::now();
                if self.tx.send(frame).is_err() {
                    self.gauge.left();
                    return Err(StreamError::ClientGone);
                }
                let paused_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                let resumes = self.gauge.paused(paused_ms);
                // Once per pause, never once per blocked tick, so a slow client cannot
                // flood the log.
                tracing::info!(
                    signal = "socket.backpressure",
                    tick = self.state.tick(),
                    bound = self.gauge.bound(),
                    paused_ms,
                    resumes
                );
                Ok(())
            }
            Err(TrySendError::Disconnected(_)) => {
                self.gauge.left();
                Err(StreamError::ClientGone)
            }
        }
    }
}

/// Turns tick records into wire frames: a keyframe every `keyframe_interval` ticks, at
/// every restart, and whenever a step is too wide for a delta; a delta otherwise.
pub struct FrameSink<O: FrameOut> {
    out: O,
    prev: Option<Quantised>,
    since_keyframe: u32,
    keyframe_interval: u32,
    keyframes: u64,
    deltas: u64,
}

impl<O: FrameOut> FrameSink<O> {
    /// A sink that sends a keyframe every `keyframe_interval` ticks.
    pub fn new(out: O, keyframe_interval: u32) -> Self {
        Self {
            out,
            prev: None,
            since_keyframe: 0,
            keyframe_interval: keyframe_interval.max(1),
            keyframes: 0,
            deltas: 0,
        }
    }

    /// Keyframes and deltas sent so far.
    pub fn counts(&self) -> (u64, u64) {
        (self.keyframes, self.deltas)
    }

    /// Returns the output, so the caller can finish it.
    pub fn into_inner(self) -> O {
        self.out
    }

    /// Encodes and sends one tick.
    pub fn push(&mut self, record: &TickRecord) -> Result<(), StreamError> {
        let quantised = Quantised::from_metres(record.tick, record.ball, &record.players);
        let delta = if record.restart || self.since_keyframe + 1 >= self.keyframe_interval {
            None
        } else {
            self.prev
                .as_ref()
                .and_then(|prev| TickFrame::delta(&quantised, prev))
        };
        let frame = match delta {
            Some(frame) => {
                self.since_keyframe += 1;
                self.deltas += 1;
                frame
            }
            None => {
                self.since_keyframe = 0;
                self.keyframes += 1;
                TickFrame::keyframe(&quantised, record.restart)
            }
        };
        self.prev = Some(quantised);
        self.out.send(Frame::Tick(frame))
    }
}

impl<O: FrameOut> TickSink for FrameSink<O> {
    fn on_tick(&mut self, record: &TickRecord) -> Result<(), EngineError> {
        self.push(record).map_err(EngineError::from)
    }
}

/// Everything the socket thread needs.
pub struct SessionConfig {
    pub buffer_ticks: usize,
    pub keyframe_interval: u32,
    pub hello: Hello,
    pub commands: CommandContext,
    /// A test seam: once a tick at or past this one is flushed, the socket thread shuts the
    /// connection down without a close frame, as a dropped network link would.
    pub drop_at: Option<u32>,
}

/// How a session ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEnd {
    /// The producer finished and every frame was written.
    Done,
    /// The client sent a close frame: it left on purpose.
    Closed,
    /// The connection went away without a close frame.
    Dropped,
}

/// A running client session. `finish` closes it: a sink that still holds a sending half
/// must not keep the socket thread alive, so the end of the match is signalled explicitly.
pub struct Session {
    tx: Option<SyncSender<Frame>>,
    gauge: Arc<Gauge>,
    state: Arc<MatchState>,
    keyframe_interval: u32,
    closing: Arc<AtomicBool>,
    handle: Option<JoinHandle<Result<SessionEnd, StreamError>>>,
}

impl Session {
    /// Sends the hello, then starts the socket thread.
    pub fn start(connection: Connection, config: SessionConfig) -> Result<Self, StreamError> {
        let SessionConfig {
            buffer_ticks,
            keyframe_interval,
            hello,
            commands,
            drop_at,
        } = config;
        let gauge = Arc::new(Gauge::new(buffer_ticks));
        let state = Arc::clone(&commands.state);
        let (tx, rx) = sync_channel(buffer_ticks);
        let thread_gauge = Arc::clone(&gauge);
        let closing = Arc::new(AtomicBool::new(false));
        let thread_closing = Arc::clone(&closing);
        let handle = std::thread::Builder::new()
            .name("stream-socket".into())
            .spawn(move || {
                let gate = Arc::clone(&commands.gate);
                let pumped = pump(
                    connection.socket,
                    rx,
                    commands,
                    Pumped {
                        gauge: thread_gauge,
                        closing: thread_closing,
                        hello,
                        drop_at,
                    },
                );
                // However the socket ends, a producer still waiting at the gate (a match held
                // before kick-off, or paused) must wake and stop rather than wait for ever.
                gate.stop();
                pumped
            })
            .map_err(|e| StreamError::io("cannot start the socket thread", e))?;
        Ok(Self {
            tx: Some(tx),
            gauge,
            state,
            keyframe_interval,
            closing,
            handle: Some(handle),
        })
    }

    /// A handle on the bounded producer buffer.
    pub fn out(&self) -> ChannelOut {
        ChannelOut {
            tx: self.tx.clone().expect("the session is still open"),
            gauge: Arc::clone(&self.gauge),
            state: Arc::clone(&self.state),
        }
    }

    /// A frame sink feeding this session.
    pub fn sink(&self) -> FrameSink<ChannelOut> {
        FrameSink::new(self.out(), self.keyframe_interval)
    }

    /// The producer buffer gauge.
    pub fn gauge(&self) -> &Arc<Gauge> {
        &self.gauge
    }

    /// Queues one JSON text message behind every tick already buffered.
    pub fn send(&self, message: &ServerMessage) -> Result<(), StreamError> {
        let text = serde_json::to_string(message)
            .map_err(|source| protocol::ProtocolError::Json { source })?;
        self.out().send(Frame::Text(text))
    }

    /// Closes the buffer and waits for the socket thread, and says how the session ended.
    /// The socket thread stops once the buffer is drained, whether or not a sink still holds
    /// a sending half.
    pub fn finish(mut self) -> Result<SessionEnd, StreamError> {
        self.closing.store(true, Ordering::Release);
        self.tx = None;
        match self.handle.take() {
            Some(handle) => handle
                .join()
                .map_err(|_| StreamError::Fixture("the socket thread panicked".into()))?,
            None => Ok(SessionEnd::Done),
        }
    }
}

/// What the socket thread owns besides the socket, the buffer, and the command context.
struct Pumped {
    gauge: Arc<Gauge>,
    closing: Arc<AtomicBool>,
    hello: Hello,
    drop_at: Option<u32>,
}

/// The socket thread: read commands, write frames, and stop when either side is done.
fn pump(
    mut socket: WebSocket<TcpStream>,
    rx: Receiver<Frame>,
    mut commands: CommandContext,
    pumped: Pumped,
) -> Result<SessionEnd, StreamError> {
    let Pumped {
        gauge,
        closing,
        hello,
        drop_at,
    } = pumped;
    // The hello goes out on the blocking socket, so it is on the wire before any tick.
    crate::send_whole(
        &mut socket,
        Message::text(
            serde_json::to_string(&ServerMessage::Hello(Box::new(hello)))
                .map_err(|source| protocol::ProtocolError::Json { source })?,
        ),
    )?;
    socket
        .get_ref()
        .set_nonblocking(true)
        .map_err(|e| StreamError::io("cannot make the socket non-blocking", e))?;

    let state = Arc::clone(&commands.state);
    // Each message, with its tick when it is a tick frame.
    let mut outbox: VecDeque<(Option<u32>, Message)> = VecDeque::new();
    let mut producer_done = false;
    // The tick of the newest frame taken from the buffer; a delta is the tick after it.
    let mut taken_tick = 0u32;
    // The newest tick frame the socket accepted and has not yet flushed.
    let mut written_tick: Option<u32> = None;
    loop {
        let mut idle = true;

        // Commands from the client. Every answer joins the outbox.
        loop {
            match socket.read() {
                Ok(Message::Text(text)) => {
                    idle = false;
                    let (answer, event) = commands.handle(text.as_str())?;
                    if let Some(event) = event {
                        outbox.push_back((
                            None,
                            text_message(&ServerMessage::Event(Box::new(event)))?,
                        ));
                    }
                    outbox.push_back((None, text_message(&answer)?));
                }
                Ok(Message::Close(_)) => {
                    commands.gate.stop();
                    finish_socket(&mut socket)?;
                    return Ok(SessionEnd::Closed);
                }
                Ok(_) => idle = false,
                Err(tungstenite::Error::Io(e)) if would_block(&e) => break,
                Err(e) if peer_gone(&e) => return Ok(dropped(&commands)),
                Err(e) => {
                    commands.gate.stop();
                    return Err(e.into());
                }
            }
        }

        // Frames from the simulation thread.
        while outbox.len() < OUTBOX_LIMIT && !producer_done {
            match rx.try_recv() {
                Ok(frame) => {
                    idle = false;
                    gauge.left();
                    let tick = match &frame {
                        Frame::Tick(tick) => {
                            taken_tick = crate::record::tick_of(tick).unwrap_or(taken_tick + 1);
                            Some(taken_tick)
                        }
                        Frame::Text(_) => None,
                    };
                    outbox.push_back((tick, wire_message(frame)));
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // The producer said it is done and the buffer is drained.
                    producer_done = closing.load(Ordering::Acquire);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    producer_done = true;
                    break;
                }
            }
        }

        // Writes. A full socket buffer stops the drain; the frames stay in the outbox.
        while let Some((tick, message)) = outbox.pop_front() {
            idle = false;
            match socket.write(message) {
                Ok(()) => written_tick = tick.or(written_tick),
                // The socket buffered the frame and the stream was not ready.
                Err(tungstenite::Error::Io(e)) if would_block(&e) => {
                    written_tick = tick.or(written_tick);
                    break;
                }
                Err(tungstenite::Error::WriteBufferFull(message)) => {
                    outbox.push_front((tick, *message));
                    break;
                }
                Err(e) if peer_gone(&e) => return Ok(dropped(&commands)),
                Err(e) => {
                    commands.gate.stop();
                    return Err(e.into());
                }
            }
        }
        match socket.flush() {
            Ok(()) => {
                if let Some(tick) = written_tick.take() {
                    state.set_sent_tick(tick);
                    if drop_at.is_some_and(|at| tick >= at) {
                        // The test seam: the connection goes away with no close frame.
                        let _ = socket.get_ref().shutdown(Shutdown::Both);
                        tracing::info!(signal = "socket.drop_injected", tick);
                        return Ok(dropped(&commands));
                    }
                }
            }
            Err(tungstenite::Error::Io(e)) if would_block(&e) => {}
            Err(e) if peer_gone(&e) => return Ok(dropped(&commands)),
            Err(e) => {
                commands.gate.stop();
                return Err(e.into());
            }
        }

        if producer_done && outbox.is_empty() {
            finish_socket(&mut socket)?;
            return Ok(SessionEnd::Done);
        }
        if idle {
            std::thread::sleep(IDLE_SLEEP);
        }
    }
}

/// The connection went away without a close frame: stop the producer and say so.
fn dropped(commands: &CommandContext) -> SessionEnd {
    commands.gate.stop();
    SessionEnd::Dropped
}

/// Sends the close frame and drives the close handshake to its end.
fn finish_socket(socket: &mut WebSocket<TcpStream>) -> Result<(), StreamError> {
    let _ = socket.close(None);
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        match socket.flush() {
            Ok(()) => return Ok(()),
            Err(tungstenite::Error::Io(e)) if would_block(&e) => {
                std::thread::sleep(IDLE_SLEEP);
            }
            Err(e) if peer_gone(&e) => return Ok(()),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// One frame as the WebSocket message it travels in.
fn wire_message(frame: Frame) -> Message {
    match frame {
        Frame::Tick(tick) => Message::binary(tick.as_bytes().to_vec()),
        Frame::Text(text) => Message::text(text),
    }
}

/// One server message as a JSON text frame.
fn text_message(message: &ServerMessage) -> Result<Message, StreamError> {
    let text = serde_json::to_string(message)
        .map_err(|source| protocol::ProtocolError::Json { source })?;
    Ok(Message::text(text))
}

/// Opens the event writer for one match, behind the lock both threads share.
pub fn shared_events(
    data_dir: &std::path::Path,
    match_id: &str,
) -> Result<Arc<Mutex<EventWriter>>, StreamError> {
    Ok(Arc::new(Mutex::new(EventWriter::open(data_dir, match_id)?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keeps every frame, so the encoder can be exercised without a socket.
    #[derive(Default)]
    struct VecOut(Vec<Frame>);

    impl FrameOut for VecOut {
        fn send(&mut self, frame: Frame) -> Result<(), StreamError> {
            self.0.push(frame);
            Ok(())
        }
    }

    fn record(tick: u32, shift: f32, restart: bool) -> TickRecord {
        TickRecord {
            tick,
            ball: [shift, 0.0, 0.0],
            players: [[shift, -shift]; 22],
            restart,
        }
    }

    #[test]
    fn a_keyframe_opens_every_cycle_and_deltas_fill_it() {
        let mut sink = FrameSink::new(VecOut::default(), 50);
        for tick in 1..=100u32 {
            sink.push(&record(tick, tick as f32 * 0.1, false)).unwrap();
        }
        assert_eq!(sink.counts(), (2, 98));
        let frames = sink.into_inner().0;
        assert_eq!(frames[0].payload().len(), 99);
        assert_eq!(frames[1].payload().len(), 48);
        assert_eq!(frames[50].payload().len(), 99);
    }

    #[test]
    fn a_restart_tick_always_sends_a_tagged_keyframe() {
        let mut sink = FrameSink::new(VecOut::default(), 50);
        sink.push(&record(1, 0.0, false)).unwrap();
        sink.push(&record(2, 0.1, true)).unwrap();
        let frames = sink.into_inner().0;
        let Frame::Tick(frame) = &frames[1] else {
            panic!("a tick frame is binary");
        };
        assert!(frame.is_restart());
    }

    #[test]
    fn a_step_too_wide_for_a_delta_sends_a_keyframe() {
        let mut sink = FrameSink::new(VecOut::default(), 50);
        sink.push(&record(1, 0.0, false)).unwrap();
        sink.push(&record(2, 5.0, false)).unwrap();
        assert_eq!(sink.counts(), (2, 0));
    }

    #[test]
    fn the_gauge_follows_the_buffer_and_the_score_packs_both_halves() {
        let gauge = Gauge::new(4);
        gauge.entered();
        gauge.entered();
        gauge.left();
        assert_eq!(gauge.high_water(), 2);
        assert_eq!(gauge.bound(), 4);
        assert_eq!(gauge.paused(7), 1);
        assert_eq!(gauge.paused_ms(), 7);

        let state = MatchState::default();
        state.set_scores([3, 1]);
        assert_eq!(state.scores(), [3, 1]);
    }
}

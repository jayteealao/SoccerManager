//! The replayer: the mock server. It serves a recorded fixture over the same protocol,
//! writing the stored bytes with no re-encoding, paced at `speed` times real time.

use std::time::{Duration, Instant};

use protocol::{Frame, Hello, ServerMessage};
use tungstenite::Message;

use crate::record::Fixture;
use crate::server::Server;
use crate::{StreamError, would_block};

/// Seconds of play one tick represents.
const TICK_SECONDS: f64 = 0.02;

/// Serves one fixture to one client.
#[derive(Debug)]
pub struct Replayer {
    fixture: Fixture,
    shown: String,
    hello: Hello,
}

impl Replayer {
    /// Reads the fixture's own hello, which the recorder stored as the first entry.
    ///
    /// The hello is never rebuilt here. A rebuilt hello can only describe the replaying
    /// build, so it would name the wrong engine version, the default keyframe interval
    /// rather than the recorded one, and two placeholder clubs with no kit colours.
    /// A fixture that carries no hello is refused by name instead.
    pub fn new(fixture: Fixture, shown: impl Into<String>) -> Result<Self, StreamError> {
        let shown = shown.into();
        let hello = match fixture.frames.first().map(|stored| &stored.frame) {
            Some(Frame::Text(text)) => match serde_json::from_str::<ServerMessage>(text) {
                Ok(ServerMessage::Hello(hello)) => *hello,
                _ => return Err(StreamError::FixtureNoHello { path: shown }),
            },
            _ => return Err(StreamError::FixtureNoHello { path: shown }),
        };
        Ok(Self {
            fixture,
            shown,
            hello,
        })
    }

    /// The hello the recording engine sent, forwarded rather than rebuilt.
    pub fn hello(&self) -> &Hello {
        &self.hello
    }

    /// The fixture being served.
    pub fn fixture(&self) -> &Fixture {
        &self.fixture
    }

    /// Waits for one client and writes every stored frame, paced by its tick index.
    /// A `speed` of 1.0 is real time; 8.0 is eight times faster.
    ///
    /// `sustain` caps the delivered rate below `speed`. It is a test harness for the
    /// viewer's lag notice and exists on `replay` alone, never on `serve`.
    ///
    /// `fast_forward_to` is a test seam: every frame before that tick goes out at once, and
    /// the pacing starts from the first frame at or after it. The bytes do not change.
    pub fn serve(
        &self,
        server: &Server,
        speed: f32,
        sustain: Option<f32>,
        fast_forward_to: Option<u32>,
    ) -> Result<u32, StreamError> {
        let connection = server.accept(&self.hello.match_id)?;
        let mut socket = connection.socket;

        let speed = f64::from(speed.clamp(0.01, 1000.0));
        let delivered = match sustain {
            Some(cap) => speed.min(f64::from(cap.clamp(0.01, 1000.0))),
            None => speed,
        };
        let from = fast_forward_to.unwrap_or(0);
        let mut started = None;
        let mut sent = 0u32;
        for stored in &self.fixture.frames {
            if stored.tick >= from {
                let started = *started.get_or_insert_with(Instant::now);
                let due = Duration::from_secs_f64(
                    f64::from(stored.tick - from) * TICK_SECONDS / delivered,
                );
                let elapsed = started.elapsed();
                if due > elapsed {
                    std::thread::sleep(due - elapsed);
                }
            }
            let message = match &stored.frame {
                Frame::Tick(tick) => Message::binary(tick.as_bytes().to_vec()),
                Frame::Text(text) => Message::text(text.clone()),
            };
            match crate::send_whole(&mut socket, message) {
                Ok(()) => sent += 1,
                Err(tungstenite::Error::Io(e)) if would_block(&e) => sent += 1,
                Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                    break;
                }
                Err(e) => return Err(e.into()),
            }
        }
        let _ = socket.close(None);
        let _ = socket.flush();
        tracing::info!(
            signal = "fixture.replayed",
            path = %self.shown,
            frames = sent,
            speed = speed,
            sustain = ?sustain,
            fast_forward_to = ?fast_forward_to,
            delivered = delivered,
            hash = %self.fixture.hash
        );
        Ok(sent)
    }
}

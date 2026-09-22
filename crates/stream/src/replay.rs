//! The replayer: the mock server. It serves a recorded fixture over the same protocol,
//! writing the stored bytes with no re-encoding, paced at `speed` times real time.

use std::time::{Duration, Instant};

use protocol::{Frame, Hello, PROTOCOL_VERSION, ServerMessage, TeamRef};
use tungstenite::Message;

use crate::record::Fixture;
use crate::server::Server;
use crate::{StreamError, would_block};

/// Seconds of play one tick represents.
const TICK_SECONDS: f64 = 0.02;

/// Serves one fixture to one client.
pub struct Replayer {
    fixture: Fixture,
    shown: String,
}

impl Replayer {
    /// Adopts a fixture read from disk. `shown` names it on the record.
    pub fn new(fixture: Fixture, shown: impl Into<String>) -> Self {
        Self {
            fixture,
            shown: shown.into(),
        }
    }

    /// The fixture being served.
    pub fn fixture(&self) -> &Fixture {
        &self.fixture
    }

    /// The hello a replay sends. It names the fixture's match, so a page that reaches a
    /// replay instead of a live engine can tell.
    pub fn hello(&self, owner_id: &str) -> Hello {
        Hello {
            protocol_version: PROTOCOL_VERSION,
            engine_version: engine::version().to_string(),
            build_hash: engine::build_hash().to_string(),
            owner_id: owner_id.to_string(),
            match_id: format!("{:016x}-{}", self.fixture.seed, self.fixture.match_millis),
            seed: self.fixture.seed,
            dt_ms: TICK_SECONDS * 1000.0,
            ticks_expected: self.fixture.ticks,
            keyframe_interval: protocol::DEFAULT_KEYFRAME_INTERVAL,
            teams: [
                TeamRef {
                    id: "fixture-home".into(),
                    name: "Fixture home".into(),
                },
                TeamRef {
                    id: "fixture-away".into(),
                    name: "Fixture away".into(),
                },
            ],
        }
    }

    /// Waits for one client and writes every stored frame, paced by its tick index.
    /// A `speed` of 1.0 is real time; 8.0 is eight times faster.
    pub fn serve(&self, server: &Server, owner_id: &str, speed: f32) -> Result<u32, StreamError> {
        let match_id = self.hello(owner_id).match_id.clone();
        let connection = server.accept(&match_id)?;
        let mut socket = connection.socket;
        socket.send(Message::text(
            serde_json::to_string(&ServerMessage::Hello(self.hello(owner_id)))
                .map_err(|source| protocol::ProtocolError::Json { source })?,
        ))?;

        let speed = f64::from(speed.clamp(0.01, 1000.0));
        let started = Instant::now();
        let mut sent = 0u32;
        for stored in &self.fixture.frames {
            let due = Duration::from_secs_f64(f64::from(stored.tick) * TICK_SECONDS / speed);
            let elapsed = started.elapsed();
            if due > elapsed {
                std::thread::sleep(due - elapsed);
            }
            let message = match &stored.frame {
                Frame::Tick(tick) => Message::binary(tick.as_bytes().to_vec()),
                Frame::Text(text) => Message::text(text.clone()),
            };
            match socket.send(message) {
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
            hash = %self.fixture.hash
        );
        Ok(sent)
    }
}

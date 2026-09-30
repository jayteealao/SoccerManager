//! The live match stream: a WebSocket server on loopback, the bounded producer buffer, the
//! change queue behind the control channel, the fixture recorder, and the replayer.
//!
//! A browser page can open a WebSocket or an HTTP connection and nothing else, so the
//! transport is WebSocket over a loopback TCP socket. The server binds `127.0.0.1` only and
//! guards the handshake with an `Origin` allowlist and a protocol-version check.

pub mod client;
pub mod control;
pub mod events;
mod migrate;
pub mod record;
pub mod replay;
pub mod server;
pub mod session;
pub mod snapshots;

use thiserror::Error;

pub use client::{Client, Incoming};
pub use control::{
    Admitted, CommandContext, Gate, Held, Inbox, LineupRules, PageSetup, PreMatch, check_lineup,
    engine_change, tactics_patch,
};
pub use events::EventWriter;
// The migration chain has no step yet, so only the tests that build their own chain reach it.
#[cfg(feature = "test-support")]
pub use migrate::{Chain, FORMAT_FIRST_MIGRATED, RawEntry, RawReplay, STEPS, Step};
#[cfg(feature = "test-support")]
pub use record::parse_fixture_with;
pub use record::{
    ChangeEntry, ChangeSource, EngineIdentity, FIXTURE_MAGIC, FORMAT_LEGACY, FORMAT_VERSION,
    Fixture, FixtureSummary, InputFile, InputInfo, LoggedChange, ManagerKind, MatchSettings,
    Outcome, Recorder, ReplayRecord, SharedRecorder, StoredFrame, Watchdog, check_frame_count,
    check_inputs, decode_frame, decode_record, outcome_of, parse_fixture, read_fixture,
    split_input, write_fixture,
};
pub use replay::Replayer;
pub use server::{Connection, Server};
pub use session::{
    ChannelOut, FrameOut, FrameSink, Gauge, MatchState, Session, SessionConfig, SessionEnd,
};
pub use snapshots::GatedSnapshots;

/// Errors this crate returns.
#[derive(Debug, Error)]
pub enum StreamError {
    /// A socket or file operation failed.
    #[error("{what}")]
    Io {
        what: String,
        #[source]
        source: std::io::Error,
    },
    /// The engine refused a record or a configuration.
    #[error("engine error")]
    Engine {
        #[source]
        source: engine::EngineError,
    },
    /// A frame or a message could not be read.
    #[error("protocol error")]
    Protocol {
        #[source]
        source: protocol::ProtocolError,
    },
    /// The WebSocket layer failed.
    #[error("websocket error")]
    WebSocket {
        #[source]
        source: Box<tungstenite::Error>,
    },
    /// The handshake was refused. The reason names the rule that refused it.
    #[error("handshake refused from origin {origin}: {reason}")]
    Refused { origin: String, reason: String },
    /// A fixture file is malformed, truncated, or of an unknown protocol version.
    #[error("fixture format error: {0}")]
    Fixture(String),
    /// A fixture does not open with a hello, so it was recorded before the recorder
    /// stored one and a replay cannot name the match it holds.
    #[error("{path} does not open with a hello; record it again")]
    FixtureNoHello { path: String },
    /// The client closed the connection before the match ended.
    #[error("the viewer disconnected")]
    ClientGone,
}

impl StreamError {
    /// An input or output failure, named by what was being done.
    pub fn io(what: impl Into<String>, source: std::io::Error) -> Self {
        StreamError::Io {
            what: what.into(),
            source,
        }
    }
}

impl From<engine::EngineError> for StreamError {
    fn from(source: engine::EngineError) -> Self {
        StreamError::Engine { source }
    }
}

impl From<protocol::ProtocolError> for StreamError {
    fn from(source: protocol::ProtocolError) -> Self {
        StreamError::Protocol { source }
    }
}

impl From<tungstenite::Error> for StreamError {
    fn from(source: tungstenite::Error) -> Self {
        StreamError::WebSocket {
            source: Box::new(source),
        }
    }
}

impl From<StreamError> for engine::EngineError {
    fn from(err: StreamError) -> Self {
        engine::EngineError::Sink(format!("{err}"))
    }
}

/// True when the peer is gone: it closed, it reset the connection, or it went away without
/// the closing handshake. A viewer that closes its page ends here, and that is not a failure.
pub(crate) fn peer_gone(err: &tungstenite::Error) -> bool {
    match err {
        tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed => true,
        tungstenite::Error::Protocol(
            tungstenite::error::ProtocolError::ResetWithoutClosingHandshake,
        ) => true,
        tungstenite::Error::Io(e) => matches!(
            e.kind(),
            std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::BrokenPipe
                | std::io::ErrorKind::NotConnected
        ),
        _ => false,
    }
}

/// Sends one message on a blocking socket, whole, even when it is longer than the write
/// bound that paces the tick frames. The bound refuses any single frame longer than itself
/// (source: tungstenite-0.30.0/src/protocol/frame/mod.rs:254, `buffer_frame`), and the hello,
/// which carries the squad and the tactics file, is longer. The bound is lifted for this one
/// message and restored at once, so the pacing of every later frame is unchanged.
pub(crate) fn send_whole(
    socket: &mut tungstenite::WebSocket<std::net::TcpStream>,
    message: tungstenite::Message,
) -> Result<(), tungstenite::Error> {
    let config = *socket.get_config();
    let needed = config.write_buffer_size + message.len() + 16;
    if needed <= config.max_write_buffer_size {
        return socket.send(message);
    }
    socket.set_config(|c| c.max_write_buffer_size = needed);
    let sent = socket.send(message);
    socket.set_config(|c| c.max_write_buffer_size = config.max_write_buffer_size);
    sent
}

/// True when an error only says the non-blocking socket has nothing to do yet.
pub(crate) fn would_block(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::Interrupted
    )
}

//! The loopback WebSocket server. It binds `127.0.0.1` on a port the operating system
//! chooses, writes that port into the runtime data folder, and guards every handshake with
//! an `Origin` allowlist and a protocol-version check.

use std::cell::RefCell;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use protocol::PROTOCOL_VERSION;
use tungstenite::WebSocket;
use tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tungstenite::http;
use tungstenite::protocol::WebSocketConfig;

use crate::StreamError;

/// File inside the runtime data folder that holds the port the server chose.
pub const PORT_FILE: &str = "engine.port";
/// The least time a reconnecting client gets to finish its upgrade request, even at the
/// very end of the reconnect wait.
const MIN_HANDSHAKE_WAIT: Duration = Duration::from_millis(500);
/// Bytes the socket may buffer before it writes to the stream.
pub const WRITE_BUFFER_BYTES: usize = 4 * 1024;
/// Bytes the socket may hold in total. Past this, a write answers `WriteBufferFull`, which
/// the session treats as a pause, exactly like a full producer buffer.
pub const MAX_WRITE_BUFFER_BYTES: usize = 8 * 1024;

/// The listening socket. Dropping it removes the port file.
pub struct Server {
    listener: TcpListener,
    port: u16,
    port_path: PathBuf,
}

/// One accepted client.
pub struct Connection {
    pub(crate) socket: WebSocket<TcpStream>,
    pub origin: String,
}

/// What the handshake callback learned about the request.
#[derive(Default)]
struct Seen {
    origin: String,
    refusal: Option<String>,
}

impl Server {
    /// Binds a loopback port and writes it to `<data_dir>/engine.port`.
    pub fn bind(data_dir: &Path, match_id: &str) -> Result<Self, StreamError> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| StreamError::io("cannot bind a loopback port", e))?;
        let port = listener
            .local_addr()
            .map_err(|e| StreamError::io("cannot read the bound port", e))?
            .port();
        std::fs::create_dir_all(data_dir)
            .map_err(|e| StreamError::io("cannot create the data folder", e))?;
        let port_path = data_dir.join(PORT_FILE);
        std::fs::write(&port_path, format!("{port}\n"))
            .map_err(|e| StreamError::io(format!("cannot write {PORT_FILE}"), e))?;
        tracing::info!(
            signal = "socket.listening",
            port,
            protocol_version = PROTOCOL_VERSION,
            match_id = %match_id
        );
        Ok(Self {
            listener,
            port,
            port_path,
        })
    }

    /// The port the operating system chose.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The address a page connects to.
    pub fn address(&self) -> String {
        format!("ws://127.0.0.1:{}/?v={PROTOCOL_VERSION}", self.port)
    }

    /// Waits for a client this build will talk to. A refused handshake names the origin and
    /// the rule that refused it, on the record and to the client, and the server keeps
    /// listening: one page with the wrong origin must not end the match for the right one.
    pub fn accept(&self, match_id: &str) -> Result<Connection, StreamError> {
        loop {
            match self.accept_once(match_id) {
                Ok(connection) => return Ok(connection),
                Err(StreamError::Refused { .. }) => continue,
                Err(other) => return Err(other),
            }
        }
    }

    /// Waits up to `wait` for a client this build will talk to, and `None` when none came.
    /// A page that lost its connection reconnects here, on the same port.
    pub fn accept_within(
        &self,
        match_id: &str,
        wait: Duration,
    ) -> Result<Option<Connection>, StreamError> {
        let deadline = Instant::now() + wait;
        self.listener
            .set_nonblocking(true)
            .map_err(|e| StreamError::io("cannot poll the listener", e))?;
        let accepted = loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    // An accepted socket inherits the listener's mode on Windows.
                    if let Err(e) = stream.set_nonblocking(false) {
                        break Err(StreamError::io("cannot make the client socket blocking", e));
                    }
                    // A client that connects and never finishes the upgrade must not hold
                    // the wait open past its deadline: the handshake gets what is left of it.
                    let left = deadline
                        .saturating_duration_since(Instant::now())
                        .max(MIN_HANDSHAKE_WAIT);
                    match self.handshake_within(stream, match_id, left) {
                        Ok(connection) => break Ok(Some(connection)),
                        Err(StreamError::Refused { .. }) if Instant::now() < deadline => continue,
                        Err(StreamError::Refused { .. }) => break Ok(None),
                        Err(other) => break Err(other),
                    }
                }
                Err(e) if crate::would_block(&e) => {
                    if Instant::now() >= deadline {
                        break Ok(None);
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) => break Err(StreamError::io("cannot accept a client", e)),
            }
        };
        let _ = self.listener.set_nonblocking(false);
        accepted
    }

    /// Waits for exactly one client, admitted or refused.
    fn accept_once(&self, match_id: &str) -> Result<Connection, StreamError> {
        let (stream, _) = self
            .listener
            .accept()
            .map_err(|e| StreamError::io("cannot accept a client", e))?;
        self.handshake(stream, match_id)
    }

    /// Runs the WebSocket handshake on one accepted stream, giving the client at most `wait`
    /// to send its upgrade request. A client that stays silent is refused like any other, and
    /// the admitted socket goes back to no timeout, because the session sets its own mode.
    fn handshake_within(
        &self,
        stream: TcpStream,
        match_id: &str,
        wait: Duration,
    ) -> Result<Connection, StreamError> {
        let limit = |s: &TcpStream, t: Option<Duration>| {
            s.set_read_timeout(t)
                .and_then(|()| s.set_write_timeout(t))
                .map_err(|e| StreamError::io("cannot set the handshake timeout", e))
        };
        limit(&stream, Some(wait))?;
        let connection = self.handshake(stream, match_id)?;
        limit(connection.socket.get_ref(), None)?;
        Ok(connection)
    }

    /// Runs the WebSocket handshake on one accepted stream.
    fn handshake(&self, stream: TcpStream, match_id: &str) -> Result<Connection, StreamError> {
        stream
            .set_nodelay(true)
            .map_err(|e| StreamError::io("cannot disable Nagle buffering", e))?;
        let seen = RefCell::new(Seen::default());
        let config = WebSocketConfig::default()
            .write_buffer_size(WRITE_BUFFER_BYTES)
            .max_write_buffer_size(MAX_WRITE_BUFFER_BYTES);
        let result = tungstenite::accept_hdr_with_config(
            stream,
            |request: &Request, response: Response| {
                let mut seen = seen.borrow_mut();
                seen.origin = origin_of(request);
                if let Some(reason) = refusal(request, &seen.origin) {
                    seen.refusal = Some(reason.clone());
                    return Err(rejection(&reason));
                }
                Ok(response)
            },
            Some(config),
        )
        // The handshake error owns the callback, which borrows `seen`; naming the failure
        // here ends that borrow.
        .map_err(|e| e.to_string());
        let Seen { origin, refusal } = seen.into_inner();
        match result {
            Ok(socket) => {
                tracing::info!(
                    signal = "socket.client",
                    origin = %origin,
                    protocol_version = PROTOCOL_VERSION,
                    match_id = %match_id
                );
                Ok(Connection { socket, origin })
            }
            Err(err) => {
                let reason = refusal.unwrap_or(err);
                tracing::warn!(signal = "socket.refused", origin = %origin, reason = %reason);
                Err(StreamError::Refused { origin, reason })
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // A stale port file would point a page at an engine that is gone.
        let _ = std::fs::remove_file(&self.port_path);
    }
}

/// The request's `Origin` header, or `none` when it carries none.
fn origin_of(request: &Request) -> String {
    request
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("none")
        .to_string()
}

/// The rule that refuses this request, or `None` when it may connect.
fn refusal(request: &Request, origin: &str) -> Option<String> {
    if !origin_allowed(origin) {
        return Some(format!("origin {origin} is not on the loopback allowlist"));
    }
    match requested_version(request) {
        Some(v) if v == PROTOCOL_VERSION => None,
        Some(v) => Some(format!(
            "protocol version {v}; this build speaks {PROTOCOL_VERSION}"
        )),
        None => Some(format!(
            "no protocol version in the query; ask for ?v={PROTOCOL_VERSION}"
        )),
    }
}

/// A page served from the local machine, or a request with no origin at all (a native
/// client). Any port of `localhost` or `127.0.0.1` is allowed, because the viewer's
/// development server chooses its own. The opaque origin `null` is refused: a browser sends
/// it from a sandboxed frame or a `data:` document of any remote site. A `file://` page is
/// refused too; the viewer cannot run from one, because it loads module scripts.
fn origin_allowed(origin: &str) -> bool {
    const LOCAL: [&str; 2] = ["http://localhost", "http://127.0.0.1"];
    origin == "none"
        || LOCAL
            .iter()
            .any(|p| origin == *p || origin.starts_with(&format!("{p}:")))
}

/// The `v` query parameter of the request line.
fn requested_version(request: &Request) -> Option<u16> {
    request
        .uri()
        .query()?
        .split('&')
        .find_map(|pair| pair.strip_prefix("v="))
        .and_then(|v| v.parse().ok())
}

/// A refusal the client can read: a status, and the reason as the body.
fn rejection(reason: &str) -> ErrorResponse {
    let status = if reason.starts_with("origin ") {
        http::StatusCode::FORBIDDEN
    } else {
        http::StatusCode::BAD_REQUEST
    };
    let mut response = ErrorResponse::new(Some(reason.to_string()));
    *response.status_mut() = status;
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(origin: &str, query: &str) -> Request {
        let mut builder = http::Request::get(format!("/{query}"));
        if origin != "none" {
            builder = builder.header("origin", origin);
        }
        builder.body(()).unwrap()
    }

    #[test]
    fn every_loopback_origin_is_allowed_and_a_remote_page_is_not() {
        for origin in [
            "none",
            "http://localhost",
            "http://localhost:5173",
            "http://127.0.0.1:8080",
        ] {
            assert!(origin_allowed(origin), "{origin} must be allowed");
        }
        for origin in [
            "null",
            "file://",
            "http://example.com",
            "https://localhost",
            "http://localhost.evil.com",
        ] {
            assert!(!origin_allowed(origin), "{origin} must be refused");
        }
    }

    #[test]
    fn a_refusal_names_both_versions() {
        let reason = refusal(&request("none", "?v=9"), "none").unwrap();
        assert_eq!(
            reason,
            format!("protocol version 9; this build speaks {PROTOCOL_VERSION}")
        );
        let current = format!("?v={PROTOCOL_VERSION}");
        assert!(refusal(&request("none", &current), "none").is_none());
        let reason = refusal(&request("none", ""), "none").unwrap();
        assert!(reason.contains(&current), "{reason}");
        let reason = refusal(
            &request("http://example.com", &current),
            "http://example.com",
        )
        .unwrap();
        assert!(reason.starts_with("origin http://example.com"), "{reason}");
    }

    #[test]
    fn the_port_file_is_written_and_removed_with_the_server() {
        let dir = std::env::temp_dir().join(format!("stream-port-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join(PORT_FILE);
        {
            let server = Server::bind(&dir, "test-match").unwrap();
            let text = std::fs::read_to_string(&path).unwrap();
            assert_eq!(text.trim().parse::<u16>().unwrap(), server.port());
        }
        assert!(!path.exists(), "the port file outlived the server");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

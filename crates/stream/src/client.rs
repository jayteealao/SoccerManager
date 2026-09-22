//! A loopback client. The viewer is a browser page and speaks this protocol in JavaScript;
//! this client exists so the benchmark and the tests can read the stream from Rust.

use std::net::TcpStream;

use protocol::{ClientCommand, Frame, Quantised, ServerMessage, TickFrame};
use tungstenite::{ClientRequestBuilder, Message, WebSocket};

use crate::StreamError;

/// What the client read.
#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    /// A binary tick frame and the absolute positions it decodes to.
    Tick(TickFrame, Quantised),
    /// A JSON text frame.
    Message(Box<ServerMessage>),
    /// The server closed the connection.
    Closed,
}

/// One connection to a live engine or to a replay.
#[derive(Debug)]
pub struct Client {
    socket: WebSocket<TcpStream>,
    previous: Option<Quantised>,
    ticks: u64,
}

impl Client {
    /// Connects to `127.0.0.1:<port>`, asking for `version` and presenting `origin`.
    pub fn connect(port: u16, version: u16, origin: &str) -> Result<Self, StreamError> {
        let stream = TcpStream::connect(("127.0.0.1", port))
            .map_err(|e| StreamError::io("cannot reach the engine socket", e))?;
        stream
            .set_nodelay(true)
            .map_err(|e| StreamError::io("cannot disable Nagle buffering", e))?;
        let uri = format!("ws://127.0.0.1:{port}/?v={version}")
            .parse()
            .map_err(|_| StreamError::Fixture("cannot build the socket address".into()))?;
        let request = ClientRequestBuilder::new(uri).with_header("Origin", origin);
        let (socket, _) = tungstenite::client(request, stream).map_err(|e| match e {
            tungstenite::HandshakeError::Failure(e) => StreamError::from(e),
            tungstenite::HandshakeError::Interrupted(_) => {
                StreamError::Fixture("the handshake was interrupted".into())
            }
        })?;
        Ok(Self {
            socket,
            previous: None,
            ticks: 0,
        })
    }

    /// Connects with the defaults a local page uses.
    pub fn connect_local(port: u16) -> Result<Self, StreamError> {
        Self::connect(port, protocol::PROTOCOL_VERSION, "http://localhost:5173")
    }

    /// Ticks read so far.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Reads the next frame. Ping and pong frames are skipped.
    pub fn read(&mut self) -> Result<Incoming, StreamError> {
        loop {
            match self.socket.read() {
                Ok(Message::Binary(bytes)) => {
                    let frame = TickFrame::from_bytes(&bytes)?;
                    let quantised = frame.decode(self.previous.as_ref())?;
                    self.previous = Some(quantised);
                    self.ticks += 1;
                    return Ok(Incoming::Tick(frame, quantised));
                }
                Ok(Message::Text(text)) => {
                    let message: ServerMessage = serde_json::from_str(text.as_str())
                        .map_err(|source| protocol::ProtocolError::Json { source })?;
                    return Ok(Incoming::Message(Box::new(message)));
                }
                Ok(Message::Close(_)) => return Ok(Incoming::Closed),
                Ok(_) => continue,
                Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                    return Ok(Incoming::Closed);
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Reads the next frame exactly as it arrived, with no decoding.
    pub fn read_raw(&mut self) -> Result<Option<Frame>, StreamError> {
        loop {
            match self.socket.read() {
                Ok(Message::Binary(bytes)) => {
                    return Ok(Some(Frame::Tick(TickFrame::from_bytes(&bytes)?)));
                }
                Ok(Message::Text(text)) => return Ok(Some(Frame::Text(text.to_string()))),
                Ok(Message::Close(_)) => return Ok(None),
                Ok(_) => continue,
                Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                    return Ok(None);
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Sends one command as a JSON text frame.
    pub fn send(&mut self, command: &ClientCommand) -> Result<(), StreamError> {
        let text = serde_json::to_string(command)
            .map_err(|source| protocol::ProtocolError::Json { source })?;
        self.socket.send(Message::text(text))?;
        Ok(())
    }

    /// Closes the connection.
    pub fn close(mut self) -> Result<(), StreamError> {
        let _ = self.socket.close(None);
        loop {
            match self.socket.read() {
                Ok(_) => continue,
                Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                    return Ok(());
                }
                Err(_) => return Ok(()),
            }
        }
    }
}

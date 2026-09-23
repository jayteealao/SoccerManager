//! A reconnect wait ends on time even when a client connects and never sends its upgrade.

mod common;

use std::net::TcpStream;
use std::time::{Duration, Instant};

use stream::Server;

#[test]
fn a_silent_client_does_not_hold_the_reconnect_wait_open() {
    let dir = common::temp_dir("accept-wait");
    let server = Server::bind(&dir, "000000000000002a-1").unwrap();
    // Connects, then says nothing: no HTTP upgrade request ever arrives.
    let _silent = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
    let started = Instant::now();
    let accepted = server
        .accept_within("000000000000002a-1", Duration::from_millis(300))
        .unwrap();
    assert!(accepted.is_none(), "a silent client is not a connection");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the wait ended {:?} after it began",
        started.elapsed()
    );
}

//! AC-a: a client that connects receives a hello with the engine version, the owner
//! identifier, and the match identifier before the first tick frame.

mod common;

use protocol::{PROTOCOL_VERSION, ServerMessage};
use stream::{Client, Incoming, StreamError};

#[test]
fn the_hello_arrives_before_the_first_tick_frame() {
    let served = common::Served::start("hello", 1, 500);
    let mut client = Client::connect_local(served.port).unwrap();
    let first = client.read().unwrap();
    let Incoming::Message(message) = first else {
        panic!("the first frame must be a message, not a tick");
    };
    let ServerMessage::Hello(hello) = *message else {
        panic!("the first message must be the hello");
    };
    assert_eq!(hello.protocol_version, PROTOCOL_VERSION);
    assert_eq!(hello.engine_version, engine::version());
    assert_eq!(hello.owner_id.len(), 32);
    assert_eq!(
        hello.match_id,
        format!("{:016x}-1700000000000", common::SEED)
    );
    assert_eq!(hello.seed, common::SEED);
    assert_eq!(hello.ticks_expected, served.ticks_expected);
    assert_eq!(hello.dt_ms, 20.0);
    assert_ne!(hello.teams[0].id, hello.teams[1].id);
    assert!(!hello.teams[0].name.is_empty());
    // The kit colours the viewer draws its markers from, straight from the team files.
    assert_eq!(hello.teams[0].kit_primary, "#c8102e");
    assert_eq!(hello.teams[0].kit_secondary, "#000000");
    assert_eq!(hello.teams[1].kit_primary, "#6a0dad");
    assert_eq!(hello.teams[1].kit_secondary, "#ff6a13");

    // The next frame is the opening tick, and it is a keyframe.
    let Incoming::Tick(frame, quantised) = client.read().unwrap() else {
        panic!("the hello is followed by a tick frame");
    };
    assert_eq!(quantised.tick, 1);
    assert_eq!(frame.as_bytes().len(), 99);
    client.close().unwrap();
    served.join();
}

#[test]
fn an_unknown_protocol_version_is_refused_naming_both() {
    let served = common::Served::start("hello-version", 1, 500);
    let err = Client::connect(served.port, 9, "http://127.0.0.1").unwrap_err();
    let StreamError::WebSocket { source } = &err else {
        panic!("a refused handshake is a websocket error: {err}");
    };
    assert!(
        source.to_string().contains("400"),
        "the refusal answers 400: {source}"
    );
    // The server refused, so the match never runs; a second client completes it.
    let client = Client::connect_local(served.port).unwrap();
    client.close().unwrap();
    served.join();
}

#[test]
fn a_page_from_another_origin_is_refused() {
    let served = common::Served::start("hello-origin", 1, 500);
    let err = Client::connect(served.port, PROTOCOL_VERSION, "http://example.com").unwrap_err();
    let StreamError::WebSocket { source } = &err else {
        panic!("a refused handshake is a websocket error: {err}");
    };
    assert!(
        source.to_string().contains("403"),
        "a forbidden origin answers 403: {source}"
    );
    let client = Client::connect_local(served.port).unwrap();
    client.close().unwrap();
    served.join();
}

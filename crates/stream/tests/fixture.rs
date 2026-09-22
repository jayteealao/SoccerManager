//! AC-d: a recorded fixture holds the full stream, and a replay of it gives a client a
//! byte-identical sequence.

mod common;

use std::path::Path;
use std::sync::Arc;

use engine::record::TickSink;
use engine::{Simulation, ticks_for_minutes};
use protocol::Frame;
use stream::session::{FrameOut, FrameSink};
use stream::{Client, Recorder, Replayer, Server, SharedRecorder, read_fixture};

/// Records one whole seeded match to a fixture, exactly as it would travel on the wire.
fn record(path: &Path, minutes: u32) -> u32 {
    let config = common::match_config(minutes);
    let recorder =
        SharedRecorder::new(Recorder::create(path, 1_700_000_000_000, common::SEED).unwrap());
    let mut sink = FrameSink::new(recorder.clone(), 50);
    let mut messages = recorder.clone();
    let mut sim = Simulation::new(config).unwrap();
    let ticks = ticks_for_minutes(minutes);
    for _ in 0..ticks {
        sim.step();
        sink.on_tick(&sim.record()).unwrap();
    }
    sim.finish();
    messages
        .send(Frame::Text("{\"type\":\"stats\",\"tick\":0}".into()))
        .unwrap();
    drop(sink);
    drop(messages);
    let summary = recorder.finish().unwrap();
    assert_eq!(summary.ticks, ticks);
    ticks
}

#[test]
fn a_replayed_fixture_is_byte_identical_to_the_recording() {
    let dir = common::temp_dir("fixture");
    let path = dir.join("match.smfx");
    let ticks = record(&path, 2);
    let fixture = read_fixture(&path).unwrap();
    assert_eq!(fixture.ticks, ticks);
    assert_eq!(fixture.seed, common::SEED);
    assert_eq!(fixture.frames.len() as u32, ticks + 1);

    let server = Server::bind(&dir, "replay-match").unwrap();
    let port = server.port();
    let replayer = Arc::new(Replayer::new(fixture.clone(), "match.smfx"));
    let serving = Arc::clone(&replayer);
    let handle = std::thread::spawn(move || {
        serving
            .serve(&server, "0123456789abcdef0123456789abcdef", 1000.0)
            .unwrap()
    });

    let mut client = Client::connect_local(port).unwrap();
    let mut received: Vec<Frame> = Vec::new();
    while let Some(frame) = client.read_raw().unwrap() {
        received.push(frame);
    }
    handle.join().unwrap();

    // The first frame is the hello, which is per connection and not part of the recording.
    let stored: Vec<&Frame> = fixture.frames.iter().map(|f| &f.frame).collect();
    let replayed: Vec<&Frame> = received.iter().skip(1).collect();
    assert_eq!(
        replayed.len(),
        stored.len(),
        "every stored frame is replayed"
    );
    for (i, (a, b)) in replayed.iter().zip(stored.iter()).enumerate() {
        assert_eq!(a.payload(), b.payload(), "frame {i} differs on the wire");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_fixture_of_another_protocol_version_is_refused() {
    let dir = common::temp_dir("fixture-version");
    let path = dir.join("match.smfx");
    record(&path, 1);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[4..6].copy_from_slice(&9u16.to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();
    let err = read_fixture(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("protocol version 9; this build speaks 1"),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

//! AC-d: a recorded fixture holds the full stream, and a replay of it gives a client a
//! byte-identical sequence.

mod common;

use std::path::Path;
use std::sync::Arc;

use engine::Simulation;
use engine::record::TickSink;
use protocol::{Frame, Hello, PROTOCOL_VERSION, ServerMessage, TeamRef};
use stream::session::{FrameOut, FrameSink};
use stream::{Client, Recorder, Replayer, Server, SharedRecorder, read_fixture};

/// Records one whole seeded match to a fixture, exactly as it would travel on the wire.
fn record(path: &Path, minutes: u32) -> u32 {
    let config = common::match_config(minutes);
    let recorder =
        SharedRecorder::new(Recorder::create(path, 1_700_000_000_000, common::SEED).unwrap());
    let mut sink = FrameSink::new(recorder.clone(), 50);
    let mut messages = recorder.clone();
    // The hello is the fixture's first entry, exactly as `engine-cli record` writes it.
    let hello = Hello {
        protocol_version: PROTOCOL_VERSION,
        engine_version: engine::version().to_string(),
        build_hash: engine::build_hash().to_string(),
        owner_id: "0123456789abcdef0123456789abcdef".into(),
        match_id: format!("{:016x}-1700000000000", common::SEED),
        seed: common::SEED,
        dt_ms: config.tuning.dt * 1000.0,
        ticks_expected: config.max_ticks(),
        keyframe_interval: 50,
        teams: [
            TeamRef {
                id: config.teams[0].club_id.clone(),
                name: config.teams[0].name.clone(),
                kit_primary: config.teams[0].kit.primary.clone(),
                kit_secondary: config.teams[0].kit.secondary.clone(),
                roster: Vec::new(),
            },
            TeamRef {
                id: config.teams[1].club_id.clone(),
                name: config.teams[1].name.clone(),
                kit_primary: config.teams[1].kit.primary.clone(),
                kit_secondary: config.teams[1].kit.secondary.clone(),
                roster: Vec::new(),
            },
        ],
    };
    messages
        .send(Frame::Text(
            serde_json::to_string(&ServerMessage::Hello(hello)).unwrap(),
        ))
        .unwrap();

    let mut sim = Simulation::new(config).unwrap();
    let mut ticks = 0;
    while !sim.is_over() {
        sim.step();
        sink.on_tick(&sim.record()).unwrap();
        ticks += 1;
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
    // The hello, every tick frame, and the closing statistics.
    assert_eq!(fixture.frames.len() as u32, ticks + 2);

    let server = Server::bind(&dir, "replay-match").unwrap();
    let port = server.port();
    let replayer = Arc::new(Replayer::new(fixture.clone(), "match.smfx").unwrap());
    let serving = Arc::clone(&replayer);
    let handle = std::thread::spawn(move || serving.serve(&server, 1000.0, None).unwrap());

    let mut client = Client::connect_local(port).unwrap();
    let mut received: Vec<Frame> = Vec::new();
    while let Some(frame) = client.read_raw().unwrap() {
        received.push(frame);
    }
    handle.join().unwrap();

    // Every frame is replayed, the hello included: a replay forwards the recorded hello
    // rather than describing the replaying build.
    let stored: Vec<&Frame> = fixture.frames.iter().map(|f| &f.frame).collect();
    let replayed: Vec<&Frame> = received.iter().collect();
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
fn the_sustain_cap_delivers_below_the_requested_speed() {
    let dir = common::temp_dir("fixture-sustain");
    let path = dir.join("match.smfx");
    record(&path, 1);
    let fixture = read_fixture(&path).unwrap();
    let server = Server::bind(&dir, "sustain-match").unwrap();
    let port = server.port();
    let replayer = Replayer::new(fixture, "match.smfx").unwrap();
    // Eight times real time is asked for and three times real time is delivered, which is
    // the only way to drive the viewer's lag notice: nothing else in the workspace can
    // sustain less than it is asked for on demand.
    let handle = std::thread::spawn(move || replayer.serve(&server, 8.0, Some(3.0)));

    let mut client = Client::connect_local(port).unwrap();
    let mut ticks = 0u32;
    let mut started = None;
    let mut elapsed = None;
    while let Some(frame) = client.read_raw().unwrap() {
        if matches!(frame, Frame::Tick(_)) {
            ticks += 1;
            if ticks == 100 {
                started = Some(std::time::Instant::now());
            }
            if ticks == 700 {
                elapsed = started.map(|t: std::time::Instant| t.elapsed());
                break;
            }
        }
    }
    drop(client);
    let _ = handle.join().unwrap();

    // 600 ticks are 12 seconds of play. At three times real time they take 4 seconds.
    let measured = elapsed.expect("the window is measured").as_secs_f64();
    let rate = 600.0 * 0.02 / measured;
    assert!(
        (rate - 3.0).abs() / 3.0 < 0.05,
        "delivered at {rate:.3}x over {measured:.3}s; the cap is 3x"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_replayed_hello_carries_the_recorded_kit_colours() {
    let dir = common::temp_dir("fixture-hello");
    let path = dir.join("match.smfx");
    record(&path, 1);
    let fixture = read_fixture(&path).unwrap();
    let replayer = Replayer::new(fixture, "match.smfx").unwrap();
    let hello = replayer.hello();
    assert_eq!(hello.seed, common::SEED);
    assert_eq!(hello.teams[0].kit_primary, "#c8102e");
    assert_eq!(hello.teams[0].kit_secondary, "#000000");
    assert_eq!(hello.teams[1].kit_primary, "#6a0dad");
    assert_ne!(hello.teams[0].name, hello.teams[1].name);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_fixture_that_does_not_open_with_a_hello_is_refused_naming_the_path() {
    let dir = common::temp_dir("fixture-no-hello");
    let path = dir.join("stale.smfx");
    // A fixture recorded before the recorder stored a hello: tick frames and nothing else.
    let config = common::match_config(1);
    let recorder =
        SharedRecorder::new(Recorder::create(&path, 1_700_000_000_000, common::SEED).unwrap());
    let mut sink = FrameSink::new(recorder.clone(), 50);
    let mut sim = Simulation::new(config).unwrap();
    for _ in 0..10 {
        sim.step();
        sink.on_tick(&sim.record()).unwrap();
    }
    drop(sink);
    recorder.finish().unwrap();

    let fixture = read_fixture(&path).unwrap();
    let err = Replayer::new(fixture, "stale.smfx").unwrap_err();
    assert!(err.to_string().contains("stale.smfx"), "{err}");
    assert!(err.to_string().contains("record it again"), "{err}");
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
        err.to_string().contains(&format!(
            "protocol version 9; this build speaks {PROTOCOL_VERSION}"
        )),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

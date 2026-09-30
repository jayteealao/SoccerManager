//! A connection that drops is told apart from one the viewer closed, the socket reports the
//! newest tick it flushed, snapshots are written only once the viewer holds everything
//! before them, and a resumed events file keeps exactly the rows up to the resume tick.

mod common;

use std::sync::{Arc, Mutex};

use engine::record::TickSink;
use engine::{Simulation, Snapshot};
use protocol::{ClientCommand, EventType, Hello, MatchEvent, PROTOCOL_VERSION, Queue};
use stream::session::{MatchState, SessionConfig};
use stream::{
    Client, CommandContext, EventWriter, Gate, GatedSnapshots, Inbox, Incoming, PreMatch, Server,
    Session, SessionEnd,
};

/// Serves one match on a thread and returns the port, the shared tick state, and the
/// handle that yields how the session ended.
fn serve(
    name: &str,
    drop_at: Option<u32>,
) -> (u16, Arc<MatchState>, std::thread::JoinHandle<SessionEnd>) {
    let data = common::temp_dir(name);
    let (port_tx, port_rx) = std::sync::mpsc::channel();
    let state = Arc::new(MatchState::default());
    let shared = Arc::clone(&state);
    let handle = std::thread::spawn(move || {
        let match_id = "000000000000002a-1".to_string();
        let server = Server::bind(&data, &match_id).unwrap();
        port_tx.send(server.port()).unwrap();
        let connection = server.accept(&match_id).unwrap();
        let gate = Arc::new(Gate::new());
        let session = Session::start(
            connection,
            SessionConfig {
                buffer_ticks: 500,
                keyframe_interval: 50,
                hello: Hello {
                    protocol_version: PROTOCOL_VERSION,
                    engine_version: "test".into(),
                    build_hash: "test".into(),
                    owner_id: "0".repeat(32),
                    match_id: match_id.clone(),
                    seed: 42,
                    dt_ms: 20.0,
                    ticks_expected: 3_000,
                    keyframe_interval: 50,
                    teams: [team("a"), team("b")],
                    tactics: serde_json::Value::Null,
                    substitutions: protocol::SubstitutionRules::default(),
                    knockout: false,
                },
                commands: CommandContext {
                    owner_id: "0".repeat(32),
                    match_id: match_id.clone(),
                    gate: Arc::clone(&gate),
                    state: Arc::clone(&shared),
                    events: Arc::new(Mutex::new(EventWriter::open(&data, &match_id).unwrap())),
                    queue: Queue::new(Vec::new()),
                    pre_match: Arc::new(PreMatch::none()),
                    inbox: Arc::new(Inbox::default()),
                },
                drop_at,
            },
        )
        .unwrap();
        let mut sink = session.sink();
        let mut sim = Simulation::new(common::match_config(1)).unwrap();
        while !sim.is_over() {
            sim.step();
            let record = sim.record();
            shared.set_tick(record.tick);
            if sink.on_tick(&record).is_err() {
                break;
            }
        }
        drop(sink);
        let end = session.finish().unwrap();
        let _ = std::fs::remove_dir_all(&data);
        end
    });
    (port_rx.recv().unwrap(), state, handle)
}

/// Reads ticks until `until` of them have arrived, or the stream ends.
fn read_ticks(client: &mut Client, until: u32) -> u32 {
    let mut last = 0;
    while last < until {
        match client.read() {
            Ok(Incoming::Tick(_, q)) => last = q.tick,
            Ok(Incoming::Message(_)) => {}
            Ok(Incoming::Closed) | Err(_) => break,
        }
    }
    last
}

#[test]
fn a_connection_lost_without_a_close_frame_is_a_drop() {
    let (port, state, handle) = serve("reconnect-drop", None);
    let mut client = Client::connect_local(port).unwrap();
    client.send(&ClientCommand::Start).unwrap();
    let read = read_ticks(&mut client, 200);
    assert!(read >= 200);
    // Dropping the client closes the stream with no close frame.
    drop(client);
    assert_eq!(handle.join().unwrap(), SessionEnd::Dropped);
    assert!(state.sent_tick() >= 200, "sent {}", state.sent_tick());
}

#[test]
fn a_close_frame_is_a_close() {
    let (port, _, handle) = serve("reconnect-close", None);
    let mut client = Client::connect_local(port).unwrap();
    read_ticks(&mut client, 100);
    client.close().unwrap();
    assert_eq!(handle.join().unwrap(), SessionEnd::Closed);
}

#[test]
fn the_drop_seam_cuts_the_connection_at_its_tick() {
    let (port, state, handle) = serve("reconnect-seam", Some(1_000));
    let mut client = Client::connect_local(port).unwrap();
    let last = read_ticks(&mut client, 3_000);
    assert_eq!(handle.join().unwrap(), SessionEnd::Dropped);
    let sent = state.sent_tick();
    assert!(sent >= 1_000, "sent {sent}");
    // The cut comes on the first flush that reaches the seam: one flush carries at most one
    // outbox of frames (32), so the connection ends within that many ticks of tick 1 000.
    assert!(sent < 1_000 + 32, "the cut came late: sent {sent}");
    // The viewer never reads past what the socket flushed.
    assert!(last <= sent, "read {last}, flushed {sent}");
    assert!(last < 3_000, "the match ran on past the drop: read {last}");
}

#[test]
fn a_snapshot_is_written_only_once_the_socket_has_passed_it() {
    let data = common::temp_dir("reconnect-gated");
    let state = Arc::new(MatchState::default());
    let mut gated = GatedSnapshots::new(&data, "m", [7; 16], 1, Arc::clone(&state));
    let mut sim = Simulation::new(common::match_config(3)).unwrap();
    // The kick-off capture is written once the first tick frame is flushed.
    gated.capture(&sim);
    state.set_sent_tick(1);
    gated.on_tick(&sim.record()).unwrap();
    assert_eq!(Snapshot::read(gated.path(), "snapshot").unwrap().tick(), 0);
    let mut stoppages = vec![0];
    while !sim.is_over() {
        sim.step();
        let record = sim.record();
        // The socket lags 400 ticks behind the simulation.
        state.set_sent_tick(record.tick.saturating_sub(400).max(1));
        gated.on_tick(&record).unwrap();
        if let Some(stoppage) = sim.stoppage() {
            stoppages.push(stoppage.tick);
            gated.on_stoppage(&stoppage, &sim).unwrap();
        }
        assert!(gated.held() <= stream::snapshots::RING);
        if gated.path().exists() {
            let written = Snapshot::read(gated.path(), "snapshot").unwrap();
            assert!(
                written.tick() < state.sent_tick(),
                "snapshot at {} was written with the socket at {}",
                written.tick(),
                state.sent_tick()
            );
        }
    }
    assert!(stoppages.len() > 1, "three minutes of play has stoppages");
    assert!(gated.writes > 0);
    let sent = state.sent_tick();
    let newest = gated.newest_before(sent).map(Snapshot::tick);
    let expected = stoppages.iter().copied().filter(|&t| t < sent).max();
    assert_eq!(newest, expected);
    // A rewind to kick-off keeps only a kick-off capture, if the ring still holds it.
    gated.rewind(0);
    assert!(gated.held() <= 1);
    assert!(gated.newest_before(u32::MAX).is_none_or(|s| s.tick() == 0));
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn a_resumed_events_file_keeps_the_rows_up_to_the_resume_tick() {
    let data = common::temp_dir("reconnect-events");
    {
        let mut writer = EventWriter::open(&data, "m").unwrap();
        for tick in [1u32, 500, 1_000, 1_001, 4_000] {
            writer
                .write(&MatchEvent::play(
                    "owner",
                    "m",
                    tick,
                    EventType::ThrowIn,
                    None,
                    [0, 0],
                ))
                .unwrap();
        }
    }
    let mut writer = EventWriter::resume(&data, "m", 1_000).unwrap();
    assert_eq!(writer.written(), 3);
    writer
        .write(&MatchEvent::play(
            "owner",
            "m",
            1_001,
            EventType::Corner,
            None,
            [0, 0],
        ))
        .unwrap();
    let text = std::fs::read_to_string(writer.path()).unwrap();
    let ticks: Vec<u64> = text
        .lines()
        .map(|l| {
            serde_json::from_str::<serde_json::Value>(l).unwrap()["tick"]
                .as_u64()
                .unwrap()
        })
        .collect();
    assert_eq!(ticks, vec![1, 500, 1_000, 1_001]);
    drop(writer);
    // A match with no events file yet resumes with an empty one.
    let fresh = EventWriter::resume(&data, "other", 10).unwrap();
    assert_eq!(fresh.written(), 0);
    let _ = std::fs::remove_dir_all(&data);
}

fn team(id: &str) -> protocol::TeamRef {
    protocol::TeamRef {
        id: id.into(),
        name: id.into(),
        kit_primary: "#ffffff".into(),
        kit_secondary: "#000000".into(),
        roster: Vec::new(),
        squad: Vec::new(),
        setup: None,
        formation: String::new(),
    }
}

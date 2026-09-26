//! AC-e: an unknown change type is refused naming the type; a valid change is acknowledged
//! with a queue identifier. Every verdict is also a `match-event` row.

mod common;

use protocol::{ClientCommand, QueueChange, ServerMessage};
use stream::{Client, Incoming};

/// Reads until a text message that is neither the hello nor an event arrives.
fn next_answer(client: &mut Client) -> ServerMessage {
    loop {
        match client.read().unwrap() {
            Incoming::Message(message) => match *message {
                ServerMessage::Hello(_) | ServerMessage::Event(_) => continue,
                other => return other,
            },
            Incoming::Tick(_, _) => continue,
            Incoming::Closed => panic!("the server closed before it answered"),
        }
    }
}

#[test]
fn an_unknown_change_type_is_refused_and_a_valid_one_is_queued() {
    let served = common::Served::start("commands", 90, 500);
    let mut client = Client::connect_local(served.port).unwrap();

    client
        .send(&ClientCommand::QueueChange(QueueChange {
            kind: "formation".into(),
            detail: serde_json::Value::Null,
        }))
        .unwrap();
    let ServerMessage::Reject(reject) = next_answer(&mut client) else {
        panic!("an unknown change type must be refused");
    };
    assert_eq!(reject.command, "queue-change");
    assert_eq!(reject.reason, "unknown change type formation");

    client
        .send(&ClientCommand::QueueChange(QueueChange {
            kind: "substitution".into(),
            detail: serde_json::json!({"off": 9, "on": 14}),
        }))
        .unwrap();
    let ServerMessage::Ack(ack) = next_answer(&mut client) else {
        panic!("a valid change must be acknowledged");
    };
    assert_eq!(ack.command, "queue-change");
    let queue_id = ack.queue_id.expect("an acknowledgement carries a queue id");
    assert!(queue_id.starts_with("q-"), "{queue_id}");
    assert_eq!(ack.state, Some(protocol::ChangeState::Queued));

    // Every row is flushed as it is written, so the record is readable while the match runs
    // and before the harness removes its folder.
    let events_path = served
        .data_dir
        .join("matches")
        .join(format!("{:016x}-1700000000000", common::SEED))
        .join("events.jsonl");
    let rows = std::fs::read_to_string(&events_path).unwrap();
    assert!(
        rows.contains("\"change.rejected_reason\":\"unknown change type formation\""),
        "the refusal is on the record: {rows}"
    );
    assert!(
        rows.contains(&format!("\"change.queue_id\":\"{queue_id}\"")),
        "the queued change is on the record: {rows}"
    );
    assert!(rows.contains("\"record.kind\":\"match-event\""), "{rows}");

    client.close().unwrap();
    served.join();
}

#[test]
fn pause_and_set_speed_are_acknowledged() {
    let served = common::Served::start("commands-speed", 90, 500);
    let mut client = Client::connect_local(served.port).unwrap();

    client.send(&ClientCommand::Pause).unwrap();
    let ServerMessage::Ack(ack) = next_answer(&mut client) else {
        panic!("pause must be acknowledged");
    };
    assert_eq!(ack.command, "pause");

    client
        .send(&ClientCommand::SetSpeed(protocol::SetSpeed { speed: 99.0 }))
        .unwrap();
    let ServerMessage::Ack(ack) = next_answer(&mut client) else {
        panic!("set-speed must be acknowledged");
    };
    assert_eq!(ack.command, "set-speed");
    assert_eq!(ack.speed, Some(8.0), "a speed above the range is clamped");

    client.close().unwrap();
    // The match was paused, so it stops short of its ticks when the client leaves.
    assert!(served.join() < 270_000);
}

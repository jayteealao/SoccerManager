//! AC-b: a client that reads as fast as it can receives every one of the 270,000 ticks of a
//! full match, in order and with no gap, and the producer buffer never exceeds its bound.

mod common;

use protocol::frame::{KIND_DELTA, KIND_KEYFRAME, KIND_RESTART};
use stream::{Client, Incoming};

#[test]
fn a_full_match_arrives_in_order_with_no_gap() {
    let served = common::Served::start("throughput", 90, 500);
    let bound = 500;
    let mut client = Client::connect_local(served.port).unwrap();
    let gauge = served.gauge();

    let mut expected_tick = 0u32;
    let mut keyframes = 0u64;
    let mut deltas = 0u64;
    let mut full_time = false;
    loop {
        match client.read().unwrap() {
            Incoming::Tick(frame, quantised) => {
                expected_tick += 1;
                assert_eq!(
                    quantised.tick, expected_tick,
                    "a tick arrived out of order or a tick was dropped"
                );
                match frame.kind() {
                    KIND_KEYFRAME | KIND_RESTART => keyframes += 1,
                    KIND_DELTA => deltas += 1,
                    other => panic!("unknown frame kind {other:#04x}"),
                }
            }
            Incoming::Message(message) => {
                if let protocol::ServerMessage::Event(event) = &*message
                    && event.event_type == protocol::EventType::FullTime
                {
                    full_time = true;
                }
            }
            Incoming::Closed => break,
        }
    }
    client.close().unwrap();

    assert_eq!(expected_tick, 270_000, "the match is 270,000 ticks");
    assert_eq!(keyframes + deltas, 270_000);
    assert!(keyframes >= 5_400, "a keyframe opens every 50-tick cycle");
    assert!(full_time, "the stream names full time before it closes");
    // `sync_channel(bound)` holds at most `bound` frames. The gauge can read one higher
    // during the hand-off between a blocked send and the matching receive.
    assert!(
        gauge.high_water() <= bound + 1,
        "the buffer reached {} against a bound of {bound}",
        gauge.high_water()
    );
    assert_eq!(served.join(), 270_000);
}

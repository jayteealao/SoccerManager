//! AC-c: a client that reads slower than the engine produces makes the server pause at the
//! buffer bound and resume when the client drains it. No tick is dropped.

// Windows only: Linux loopback socket buffers are large enough to hold the match, so the
// producer cannot be made to reach the bound there. The Windows run checks the pause.
#![cfg(windows)]

mod common;

use std::time::Duration;

use stream::{Client, Incoming};

#[test]
fn a_slow_client_pauses_the_producer_and_loses_no_tick() {
    let bound = 64;
    let served = common::Served::start("backpressure", 5, bound);
    let mut client = Client::connect_local(served.port).unwrap();
    let gauge = served.gauge();

    let mut ticks = 0u32;
    let mut read_since_pause = 0u32;
    loop {
        match client.read().unwrap() {
            Incoming::Tick(_, quantised) => {
                ticks += 1;
                assert_eq!(quantised.tick, ticks, "a tick was dropped or reordered");
                read_since_pause += 1;
                // Read in small bursts, then stop. Five minutes of play is far more than
                // the socket buffers hold, so the producer must reach the bound and wait.
                if read_since_pause == 64 {
                    read_since_pause = 0;
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            Incoming::Message(_) => continue,
            Incoming::Closed => break,
        }
    }
    client.close().unwrap();

    assert_eq!(ticks, 15_000, "five minutes of play is 15,000 ticks");
    assert!(
        gauge.pauses() > 0,
        "a slow client must pause the producer at the bound"
    );
    // `sync_channel(bound)` holds at most `bound` frames, and that channel is the bound the
    // acceptance criterion names. The gauge counts `entered - left`, so it reads up to two
    // higher: one frame sits at a blocked sender, which counts itself in before it tries to
    // send, and one frame is received but not yet counted out.
    assert!(
        gauge.high_water() <= bound + 2,
        "the buffer reached {} against a bound of {bound}",
        gauge.high_water()
    );
    assert_eq!(served.join(), 15_000);
}

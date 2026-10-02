//! A page still reading a backlog when the match ends gets every tick and the close frame.
//!
//! The engine closes the socket once the producer is done and its outbox is empty, but the
//! page may still be reading frames the operating system holds, and still sending `seen` as
//! it draws them, as it does after Skip to result, when the rest of the match arrives at
//! once. A socket closed with those messages unread answers with a reset, which throws away
//! the frames the page had not read yet: the page loses the end of the match and its full
//! time. The close must wait for the page's own close answer.

mod common;

use std::time::Duration;

use protocol::{ClientCommand, Seen};
use stream::{Client, Incoming};

#[test]
fn a_page_reading_a_backlog_at_the_end_gets_every_tick_and_the_close() {
    let served = common::Served::start("close-drain", 5, 64);
    let mut client = Client::connect_local(served.port).unwrap();

    let mut ticks = 0u32;
    loop {
        match client.read().unwrap() {
            Incoming::Tick(_, quantised) => {
                ticks += 1;
                assert_eq!(quantised.tick, ticks, "a tick was dropped or reordered");
                // The page reports what it drew, and draws slowly enough to fall behind.
                if ticks.is_multiple_of(50) {
                    client
                        .send(&ClientCommand::Seen(Seen { tick: ticks }))
                        .unwrap();
                }
                if ticks.is_multiple_of(64) {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            Incoming::Message(_) => continue,
            Incoming::Closed => break,
        }
    }
    client.close().unwrap();

    assert_eq!(
        ticks, 15_000,
        "five minutes of play is 15,000 ticks, every one received"
    );
    assert_eq!(served.join(), 15_000);
}

/// A page that is still reading, and still answering what it drew, is never cut off by the
/// close wait, however long its backlog takes on a busy machine: the wait counts from the
/// page's last message, not from the close.
#[test]
fn a_page_still_reading_after_the_close_wait_gets_every_tick_and_the_close() {
    // Longer than the engine's close wait (10 s), measured from when the match ends.
    const READ_FOR: Duration = Duration::from_secs(12);
    let served = common::Served::start("close-drain-slow", 5, 64);
    let mut client = Client::connect_local(served.port).unwrap();

    // Where the slow reading starts: the match has ended by then and every later tick
    // waits in the socket.
    const SLOW_FROM: u32 = 14_000;
    let mut slow_since = None;
    let mut ticks = 0u32;
    loop {
        match client.read().unwrap() {
            Incoming::Tick(_, quantised) => {
                ticks += 1;
                assert_eq!(quantised.tick, ticks, "a tick was dropped or reordered");
                if ticks.is_multiple_of(50) {
                    client
                        .send(&ClientCommand::Seen(Seen { tick: ticks }))
                        .unwrap();
                }
                // The last ticks arrive after the match has ended; read them over READ_FOR, as
                // a page starved of time on a busy machine does.
                if ticks == SLOW_FROM {
                    slow_since = Some(std::time::Instant::now());
                }
                if let Some(since) = slow_since
                    && ticks.is_multiple_of(50)
                {
                    let due = READ_FOR
                        .mul_f64(f64::from(ticks - SLOW_FROM) / f64::from(15_000 - SLOW_FROM));
                    if let Some(wait) = due.checked_sub(since.elapsed()) {
                        std::thread::sleep(wait);
                    }
                }
            }
            Incoming::Message(_) => continue,
            Incoming::Closed => break,
        }
    }
    client.close().unwrap();

    let slow = slow_since.expect("the page read slowly at the end");
    assert!(
        slow.elapsed() >= READ_FOR,
        "the page read slowly for the whole time"
    );
    assert_eq!(ticks, 15_000, "every tick received after the close wait");
    assert_eq!(served.join(), 15_000);
}

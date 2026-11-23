//! `confined_transport::stream` through a confined process: flow control, a stop mid-stream, a
//! worker that over-sends, one that stalls, one that sends a malformed item, and the two budgets
//! that keep the worker (ADR 3502).
//!
//! Every expected outcome is the stream contract's, stated in `confined_transport::stream`'s module
//! documentation; none is a measurement.

#![expect(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test code: a worker that cannot be started has not found a defect in the stream"
)]

use std::num::NonZeroU32;
use std::path::Path;
use std::time::{Duration, Instant};

use confined_transport::stream::{Budget, Spent, StreamError};
use confined_transport::{Canceller, Host, TransportError};

const PROGRAM: &str = env!("CARGO_BIN_EXE_stream-witness");
const MAGIC: &[u8; 8] = b"STRMWIT1";
const PING: u8 = 1;
const COUNT: u8 = 2;

const KEEP: u8 = 0;
const OVER_SEND: u8 = 1;
const STALL: u8 = 2;
const MALFORMED: u8 = 3;

fn start() -> Host {
    Host::start(Path::new(PROGRAM), MAGIC, &Canceller::new()).expect("the witness starts")
}

fn request(mode: u8, count: u32, size: u32) -> Vec<u8> {
    let mut out = vec![mode];
    out.extend_from_slice(&count.to_be_bytes());
    out.extend_from_slice(&size.to_be_bytes());
    out
}

fn window(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}

fn budget(frames: u32, bytes: u64, seconds: u64) -> Budget {
    Budget {
        frames,
        bytes,
        wall: Duration::from_secs(seconds),
    }
}

/// The worker is kept, and answers an ordinary request on the wire the stream left.
fn answers_a_ping(host: &mut Host) {
    assert_eq!(
        host.exchange(PING, &[], None).expect("the worker answers"),
        (PING, b"pong".to_vec())
    );
}

/// Flow control: a stream longer than its window arrives whole and in order, and the wire is
/// usable after it — which it would not be if a grant were left unread in the worker's input.
#[test]
fn a_stream_longer_than_its_window_arrives_whole_and_leaves_the_wire_clean() {
    let mut host = start();
    let mut stream = host
        .stream(
            COUNT,
            &request(KEEP, 25, 100),
            window(3),
            budget(100, 1 << 20, 60),
        )
        .unwrap();
    let mut items = 0;
    while let Some(body) = stream.next_item().unwrap() {
        assert_eq!(body, vec![0x5A; 100]);
        items += 1;
    }
    assert_eq!(items, 25);
    assert_eq!(stream.closing(), Some(&b"done"[..]));
    drop(stream);
    answers_a_ping(&mut host);
}

/// A stream whose window divides its length exactly ends on a worker that waited for its grant.
#[test]
fn a_stream_that_spends_its_last_grant_exactly_still_ends_cleanly() {
    let mut host = start();
    let mut stream = host
        .stream(
            COUNT,
            &request(KEEP, 6, 1),
            window(3),
            budget(6, 1 << 20, 60),
        )
        .unwrap();
    let mut items = 0;
    while stream.next_item().unwrap().is_some() {
        items += 1;
    }
    assert_eq!(items, 6);
    drop(stream);
    answers_a_ping(&mut host);
}

/// Cancellation mid-stream: the worker reads the stop and ends, and is asked something else.
#[test]
fn a_stop_mid_stream_ends_the_stream_and_keeps_the_worker() {
    let mut host = start();
    let mut stream = host
        .stream(
            COUNT,
            &request(KEEP, 1_000_000, 10),
            window(4),
            budget(u32::MAX, u64::MAX, 60),
        )
        .unwrap();
    for _ in 0..7 {
        assert!(stream.next_item().unwrap().is_some());
    }
    let closing = stream.stop().unwrap();
    assert!(
        closing.is_empty(),
        "a stopped stream closes with nothing said"
    );
    answers_a_ping(&mut host);
}

/// The frames budget: a worker with more to send is stopped at the budget, said by name, and kept.
#[test]
fn the_frames_budget_stops_a_worker_with_more_to_send() {
    let mut host = start();
    let mut stream = host
        .stream(
            COUNT,
            &request(KEEP, 50, 1),
            window(4),
            budget(10, 1 << 20, 60),
        )
        .unwrap();
    let mut items = 0;
    let outcome = loop {
        match stream.next_item() {
            Ok(Some(_)) => items += 1,
            other => break other,
        }
    };
    assert!(
        matches!(outcome, Err(StreamError::Spent(Spent::Frames))),
        "{outcome:?}"
    );
    assert_eq!(items, 10);
    drop(stream);
    answers_a_ping(&mut host);
}

/// The bytes budget: an item past it is read past rather than allocated, and the stream stops.
#[test]
fn the_bytes_budget_reads_past_the_item_that_passes_it() {
    let mut host = start();
    let mut stream = host
        .stream(
            COUNT,
            &request(KEEP, 50, 1000),
            window(4),
            budget(100, 2500, 60),
        )
        .unwrap();
    let mut items = 0;
    let outcome = loop {
        match stream.next_item() {
            Ok(Some(_)) => items += 1,
            other => break other,
        }
    };
    assert!(
        matches!(outcome, Err(StreamError::Spent(Spent::Bytes))),
        "{outcome:?}"
    );
    assert_eq!(items, 2);
    drop(stream);
    answers_a_ping(&mut host);
}

/// A worker that over-sends is seen where the host granted nothing, and ended.
#[test]
fn a_worker_that_over_sends_is_ended() {
    let mut host = start();
    let mut stream = host
        .stream(
            COUNT,
            &request(OVER_SEND, 40, 1),
            window(2),
            budget(4, 1 << 20, 60),
        )
        .unwrap();
    let outcome = loop {
        match stream.next_item() {
            Ok(Some(_)) => {}
            other => break other,
        }
    };
    assert!(
        matches!(outcome, Err(StreamError::Violation(_))),
        "{outcome:?}"
    );
    drop(stream);
    assert!(matches!(
        host.exchange(PING, &[], None),
        Err(TransportError::Cancelled)
    ));
}

/// A worker that stalls is ended by the wall-clock budget, which a blocked read could not check.
#[test]
fn a_worker_that_stalls_is_ended_at_the_wall_clock_budget() {
    let mut host = start();
    let began = Instant::now();
    let mut stream = host
        .stream(
            COUNT,
            &request(STALL, 1, 1),
            window(2),
            Budget {
                frames: 10,
                bytes: 1 << 20,
                wall: Duration::from_millis(300),
            },
        )
        .unwrap();
    let outcome = stream.next_item();
    assert!(
        matches!(outcome, Err(StreamError::Spent(Spent::Wall))),
        "{outcome:?}"
    );
    assert!(began.elapsed() >= Duration::from_millis(300));
    assert!(
        began.elapsed() < Duration::from_secs(30),
        "{:?}",
        began.elapsed()
    );
    drop(stream);
    assert!(host.is_cancelled());
}

/// A malformed item mid-stream ends the worker, after the items before it were handed on.
#[test]
fn a_malformed_item_mid_stream_ends_the_worker() {
    let mut host = start();
    let mut stream = host
        .stream(
            COUNT,
            &request(MALFORMED, 0, 8),
            window(4),
            budget(10, 1 << 20, 60),
        )
        .unwrap();
    assert!(stream.next_item().unwrap().is_some());
    assert!(stream.next_item().unwrap().is_some());
    let outcome = stream.next_item();
    assert!(
        matches!(outcome, Err(StreamError::Violation(ref why)) if why.contains("carriage")),
        "{outcome:?}"
    );
    drop(stream);
    assert!(host.is_cancelled());
}

/// A stream dropped before its end ends its worker rather than leave the wire mid-stream.
#[test]
fn a_stream_dropped_open_ends_its_worker() {
    let mut host = start();
    let mut stream = host
        .stream(
            COUNT,
            &request(KEEP, 100, 1),
            window(2),
            budget(100, 1 << 20, 60),
        )
        .unwrap();
    assert!(stream.next_item().unwrap().is_some());
    drop(stream);
    assert!(matches!(
        host.exchange(PING, &[], None),
        Err(TransportError::Cancelled)
    ));
}

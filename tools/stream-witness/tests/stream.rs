//! `confined_transport::stream` through a confined process: flow control, a stop mid-stream, a
//! worker that over-sends, one that stalls, one that sends a malformed item, and the two budgets
//! that keep the worker (ADR 3502); and the same stream through a shared-memory ring — slots reused
//! in turn, a slot the worker does not hold, a second ring, the ring's own bound and the budgets
//! through it, a host that ends mid-stream — and what the ring saves, measured (ADR 3516).
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

#[cfg(target_os = "linux")]
use confined_transport::ring::{self, RingError, Shape};
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
#[cfg(target_os = "linux")]
const RING: u8 = 4;
#[cfg(target_os = "linux")]
const RING_WRONG_SLOT: u8 = 5;
#[cfg(target_os = "linux")]
const INLINE_FILLED: u8 = 6;

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

/// A ring of three slots of `slot_len` bytes, attached.
#[cfg(target_os = "linux")]
fn start_with_ring(slot_len: usize) -> Host {
    let mut host = start();
    host.attach_ring(Shape::new(slot_len, 3, 1 << 30).unwrap())
        .expect("a ring attaches on Linux");
    host
}

/// Item `q` of a ring stream is `size` bytes of `q`'s low byte.
#[cfg(target_os = "linux")]
fn expected(sequence: u32, size: usize) -> Vec<u8> {
    vec![sequence.to_be_bytes()[3]; size]
}

/// Slots in turn: ten items through three slots arrive whole and in order, each slot written three
/// or four times, and every byte the one its item was given — which a slot read while the worker
/// wrote the next item into it would not be.
#[cfg(target_os = "linux")]
#[test]
fn a_stream_through_the_ring_reuses_its_slots_and_arrives_whole() {
    let mut host = start_with_ring(16_384);
    let mut stream = host
        .stream(
            COUNT,
            &request(RING, 10, 10_000),
            window(8),
            budget(100, 1 << 20, 60),
        )
        .unwrap();
    let mut items = 0u32;
    while let Some(body) = stream.next_item().unwrap() {
        assert_eq!(body, expected(items, 10_000), "item {items}");
        items += 1;
    }
    assert_eq!(items, 10);
    assert_eq!(stream.closing(), Some(&b"done"[..]));
    drop(stream);
    answers_a_ping(&mut host);
}

/// A small item still crosses inline on a worker that holds a ring.
#[cfg(target_os = "linux")]
#[test]
fn small_items_cross_inline_beside_the_ring() {
    let mut host = start_with_ring(16_384);
    let mut stream = host
        .stream(
            COUNT,
            &request(RING, 4, 100),
            window(3),
            budget(100, 1 << 20, 60),
        )
        .unwrap();
    let mut items = 0u32;
    while let Some(body) = stream.next_item().unwrap() {
        assert_eq!(body, expected(items, 100));
        items += 1;
    }
    assert_eq!(items, 4);
}

/// A worker that names a slot it does not hold has broken the contract, and is ended.
#[cfg(target_os = "linux")]
#[test]
fn a_worker_that_writes_a_slot_it_does_not_hold_is_ended() {
    let mut host = start_with_ring(4096);
    let mut stream = host
        .stream(
            COUNT,
            &request(RING_WRONG_SLOT, 1, 1),
            window(3),
            budget(10, 1 << 20, 60),
        )
        .unwrap();
    assert!(matches!(stream.next_item(), Err(StreamError::Violation(_))));
    drop(stream);
    assert!(host.is_cancelled());
}

/// Once a worker: the host refuses a second ring, and a descriptor sent beside a second ring frame
/// by hand never reaches the worker, because the first fills the one slot its confinement leaves.
#[cfg(target_os = "linux")]
#[test]
fn a_second_ring_is_refused_by_the_host_and_its_descriptor_by_the_kernel() {
    use std::os::fd::AsFd as _;

    let mut host = start_with_ring(4096);
    let shape = Shape::new(4096, 2, 1 << 20).unwrap();
    assert!(matches!(host.attach_ring(shape), Err(RingError::Attached)));
    let object = rustix::fs::memfd_create("second-ring", rustix::fs::MemfdFlags::CLOEXEC).unwrap();
    rustix::fs::ftruncate(&object, 8192).unwrap();
    let answer = host
        .exchange(ring::RING, &shape.encode(), Some(object.as_fd()))
        .expect("the worker answers");
    assert_eq!(
        answer,
        (ring::RING, vec![0]),
        "no descriptor arrived beside the second ring"
    );
    answers_a_ping(&mut host);
}

/// The ring is bounded by the worker's memory: at most half its address-space ceiling.
#[cfg(target_os = "linux")]
#[test]
fn a_ring_past_half_the_workers_address_space_is_refused() {
    let mut host = start();
    let ceiling = host.confinement().address_space_limit;
    assert!(ceiling > 0, "the witness is confined under a ceiling");
    let slot = usize::try_from(ceiling / 4).unwrap();
    let shape = Shape::new(slot, 3, u64::MAX).unwrap();
    assert!(matches!(host.attach_ring(shape), Err(RingError::Shape(_))));
    assert!(host.ring_shape().is_none());
    answers_a_ping(&mut host);
}

/// The stream's budgets hold through the ring: bytes are a slot item's length, counted before its
/// slot is copied, and frames bound the grants; both keep the worker.
#[cfg(target_os = "linux")]
#[test]
fn the_budgets_hold_through_the_ring() {
    let mut host = start_with_ring(16_384);
    let mut stream = host
        .stream(
            COUNT,
            &request(RING, 6, 10_000),
            window(3),
            budget(100, 25_000, 60),
        )
        .unwrap();
    let mut items = 0;
    let outcome = loop {
        match stream.next_item() {
            Ok(Some(_)) => items += 1,
            other => break other,
        }
    };
    assert!(matches!(outcome, Err(StreamError::Spent(Spent::Bytes))));
    assert_eq!(items, 2, "the third item's 10 000 bytes pass 25 000");
    drop(stream);
    answers_a_ping(&mut host);

    let mut stream = host
        .stream(
            COUNT,
            &request(RING, 10, 10_000),
            window(3),
            budget(4, 1 << 20, 60),
        )
        .unwrap();
    let mut items = 0;
    let outcome = loop {
        match stream.next_item() {
            Ok(Some(_)) => items += 1,
            other => break other,
        }
    };
    assert!(matches!(outcome, Err(StreamError::Spent(Spent::Frames))));
    assert_eq!(items, 4);
    drop(stream);
    answers_a_ping(&mut host);
}

/// A host that ends mid-stream ends its worker before its own mapping goes: `Host`'s drop kills and
/// reaps, and only then are its fields — the ring among them — dropped and unmapped. Nothing faults
/// on either side, and the drop does not wait on the worker.
#[cfg(target_os = "linux")]
#[test]
fn a_host_that_unmaps_its_ring_mid_stream_ends_the_worker_first() {
    let canceller = Canceller::new();
    let mut host = Host::start(Path::new(PROGRAM), MAGIC, &canceller).expect("starts");
    host.attach_ring(Shape::new(16_384, 3, 1 << 20).unwrap())
        .expect("attaches");
    let mut stream = host
        .stream(
            COUNT,
            &request(RING, 1000, 10_000),
            window(3),
            budget(1000, 1 << 30, 60),
        )
        .unwrap();
    assert_eq!(stream.next_item().unwrap(), Some(expected(0, 10_000)));
    let began = Instant::now();
    drop(stream);
    drop(host);
    assert!(canceller.is_cancelled(), "the worker was ended");
    assert!(began.elapsed() < Duration::from_secs(5));
}

// not a gate: a frame rate read by hand for ADR 3516, which records the three runs it took; a rate
// under load is no band, and the contract it measures is held by the tests above.
/// What the ring saves, on this machine: a 1080p RGBA frame — 8 294 400 bytes — streamed inline
/// and through the ring, each written afresh by the worker for every item, as a decoder would.
///
/// Ignored because it is a measurement, not a contract: `cargo test -p stream-witness --test
/// stream -- --ignored --nocapture the_copy` prints frames per second for each carriage.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "a measurement, printed rather than asserted"]
fn the_copy_the_ring_saves_measured() {
    const FRAME: u32 = 1920 * 1080 * 4;
    const FRAMES: u32 = 120;
    let size = usize::try_from(FRAME).unwrap();
    for (name, mode) in [("inline", INLINE_FILLED), ("ring", RING)] {
        let mut host = start_with_ring(size);
        let began = Instant::now();
        let mut stream = host
            .stream(
                COUNT,
                &request(mode, FRAMES, FRAME),
                window(3),
                budget(FRAMES, u64::MAX, 600),
            )
            .unwrap();
        let mut items = 0u32;
        while let Some(body) = stream.next_item().unwrap() {
            assert_eq!(body.len(), size);
            items += 1;
        }
        let seconds = began.elapsed().as_secs_f64();
        assert_eq!(items, FRAMES);
        println!(
            "{name}: {FRAMES} frames of {FRAME} bytes in {seconds:.3} s, {:.1} frames per second",
            f64::from(FRAMES) / seconds
        );
    }
}

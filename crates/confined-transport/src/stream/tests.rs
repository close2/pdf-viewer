//! The stream's contract without a process: the host's [`Reception`] and the worker's
//! [`Emitter`] over byte buffers. `tools/stream-witness` runs the same contract through a confined
//! worker, where a kill and a stall are real.

use std::num::NonZeroU32;
use std::time::Duration;

use super::{
    Accepted, Body, Budget, Control, END, Emitted, Emitter, ITEM, Next, Reason, Reception, STOP,
    Spent, Stopping, decode_end, decode_open, encode_credit, encode_end, encode_item_header,
    encode_open, read_control,
};
use crate::frame;

fn n(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}

fn budget(frames: u32, bytes: u64) -> Budget {
    Budget {
        frames,
        bytes,
        wall: Duration::from_secs(60),
    }
}

fn item(sequence: u32, body: &[u8]) -> Vec<u8> {
    let mut out = encode_item_header(sequence, super::Carriage::Inline).to_vec();
    out.extend_from_slice(body);
    out
}

fn control(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = frame::header(kind, payload.len()).to_vec();
    out.extend_from_slice(payload);
    out
}

/// Splits a worker's output into its frames.
fn frames(mut bytes: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut out = Vec::new();
    while !bytes.is_empty() {
        let header: [u8; frame::HEADER_LEN] = bytes[..frame::HEADER_LEN].try_into().unwrap();
        let (kind, length) = frame::parse_header(header).unwrap();
        let end = frame::HEADER_LEN.saturating_add(length);
        out.push((kind, bytes[frame::HEADER_LEN..end].to_vec()));
        bytes = &bytes[end..];
    }
    out
}

#[test]
fn an_open_request_round_trips_and_a_grant_of_nothing_is_refused() {
    let payload = encode_open(9, n(3), b"clip");
    let opened = decode_open(&payload).unwrap();
    assert_eq!(
        (opened.kind, opened.credit, opened.request),
        (9, n(3), &b"clip"[..])
    );
    let mut nothing = payload.clone();
    nothing[1..5].copy_from_slice(&0u32.to_be_bytes());
    assert!(decode_open(&nothing).is_err());
    assert!(decode_open(&payload[..4]).is_err());
}

/// Flow control: the host grants a window at a time, and only once the last has arrived.
#[test]
fn the_host_grants_a_window_only_when_the_last_has_arrived() {
    let (mut reception, first) = Reception::new(n(2), budget(5, 1 << 20)).unwrap();
    assert_eq!(first, n(2));
    assert_eq!(reception.to_write(), Next::Read);
    for sequence in 0..2 {
        assert_eq!(reception.to_write(), Next::Read);
        let payload = item(sequence, b"x");
        assert_eq!(
            reception.accept(ITEM, &payload).unwrap(),
            Accepted::Item(Body::Inline(b"x"))
        );
    }
    assert_eq!(reception.to_write(), Next::Credit(n(2)));
    assert_eq!(reception.to_write(), Next::Read);
    for sequence in 2..4 {
        reception.accept(ITEM, &item(sequence, b"x")).unwrap();
    }
    // One frame of the budget is left, so the grant is one, never the window.
    assert_eq!(reception.to_write(), Next::Credit(n(1)));
    reception.accept(ITEM, &item(4, b"x")).unwrap();
    assert_eq!(reception.to_write(), Next::Stop);
    assert_eq!(reception.stopping(), Some(Stopping::Spent(Spent::Frames)));
}

/// A worker that over-sends — an item past the credit — has broken the contract.
#[test]
fn an_item_past_the_credit_is_refused() {
    let (mut reception, _) = Reception::new(n(1), budget(10, 1 << 20)).unwrap();
    reception.accept(ITEM, &item(0, b"a")).unwrap();
    let wrong = reception.accept(ITEM, &item(1, b"b")).unwrap_err();
    assert!(wrong.0.contains("past the credit"), "{wrong}");
}

/// Malformed replies mid-stream: out of sequence, a carriage this build does not read, a short
/// item, a frame of another kind, an end giving a stop nobody asked for.
#[test]
fn a_malformed_frame_mid_stream_is_refused() {
    let fresh = || Reception::new(n(4), budget(10, 1 << 20)).unwrap().0;
    let mut reception = fresh();
    reception.accept(ITEM, &item(0, b"a")).unwrap();
    assert!(reception.accept(ITEM, &item(2, b"c")).is_err());

    let mut carriage = item(0, b"a");
    carriage[4] = 1;
    assert!(fresh().accept(ITEM, &carriage).is_err());
    assert!(fresh().accept(ITEM, &[0, 0]).is_err());
    assert!(fresh().accept(1, b"an ordinary reply").is_err());
    assert!(
        fresh()
            .accept(END, &encode_end(Reason::Stopped, &[]))
            .is_err()
    );
    assert!(fresh().accept(END, &[7]).is_err());
    assert!(fresh().refused(END, 1 << 20).is_err());
}

/// The bytes are checked against the item's header before it is read, and an item past them stops
/// the stream; the items the host had granted are counted and thrown away.
#[test]
fn an_item_past_the_bytes_stops_the_stream() {
    let (mut reception, _) = Reception::new(n(3), budget(10, 4)).unwrap();
    assert_eq!(reception.admitted(), super::MAX_CLOSING + 1);
    assert_eq!(
        reception.accept(ITEM, &item(0, b"abc")).unwrap(),
        Accepted::Item(Body::Inline(b"abc"))
    );
    reception.refused(ITEM, 100).unwrap();
    assert_eq!(reception.stopping(), Some(Stopping::Spent(Spent::Bytes)));
    assert_eq!(
        reception.accept(ITEM, &item(2, b"")).unwrap(),
        Accepted::Discarded
    );
    assert_eq!(reception.to_write(), Next::Stop);
    assert!(matches!(
        reception
            .accept(END, &encode_end(Reason::Stopped, &[]))
            .unwrap(),
        Accepted::Ended {
            reason: Reason::Stopped,
            ..
        }
    ));
}

/// Cancellation: what was granted is drained, then the stop goes where the grant would have.
#[test]
fn a_stop_waits_for_the_granted_items_and_then_is_sent() {
    let (mut reception, _) = Reception::new(n(3), budget(100, 1 << 20)).unwrap();
    reception.accept(ITEM, &item(0, b"a")).unwrap();
    reception.stop();
    assert_eq!(reception.to_write(), Next::Read);
    assert_eq!(
        reception.accept(ITEM, &item(1, b"b")).unwrap(),
        Accepted::Discarded
    );
    reception.accept(ITEM, &item(2, b"c")).unwrap();
    assert_eq!(reception.to_write(), Next::Stop);
    assert_eq!(reception.to_write(), Next::Read);
}

/// The worker's half: it sends what it holds credit for, then reads before it writes again; and
/// the host reading the same frames writes exactly the control frames the worker read.
#[test]
fn the_worker_reads_a_grant_when_its_credit_is_spent_and_ends_on_a_stop() {
    let mut input: Vec<u8> = control(super::CREDIT, &encode_credit(n(2)));
    input.extend(control(STOP, &[]));
    let mut input = &input[..];
    let mut output = Vec::new();
    let mut emitter = Emitter::new(n(2));
    for body in [&b"one"[..], b"two", b"three", b"four"] {
        assert_eq!(
            emitter.emit(&mut input, &mut output, body).unwrap(),
            Emitted::Sent
        );
    }
    assert_eq!(
        emitter.emit(&mut input, &mut output, b"five").unwrap(),
        Emitted::Stopped
    );
    assert!(input.is_empty(), "every control frame was read");
    assert!(emitter.emit(&mut input, &mut output, b"six").is_err());
    let sent = frames(&output);
    let kinds: Vec<u8> = sent.iter().map(|(kind, _)| *kind).collect();
    assert_eq!(kinds, [ITEM, ITEM, ITEM, ITEM, END]);

    let (mut reception, _) = Reception::new(n(2), budget(10, 1 << 20)).unwrap();
    let mut written = Vec::new();
    for (kind, payload) in &sent {
        match reception.to_write() {
            Next::Credit(credit) => written.push(Control::Credit(credit)),
            Next::Stop => written.push(Control::Stop),
            Next::Read => {}
        }
        reception.accept(*kind, payload).unwrap();
        if reception.received() == 4 {
            reception.stop();
        }
    }
    assert_eq!(written, [Control::Credit(n(2)), Control::Stop]);
    assert!(reception.is_ended());
}

/// A worker with nothing more to send still waits for what it is owed before its end.
#[test]
fn a_finished_worker_with_no_credit_reads_its_grant_first() {
    let input = control(super::CREDIT, &encode_credit(n(4)));
    let mut input = &input[..];
    let mut output = Vec::new();
    let mut emitter = Emitter::new(n(1));
    emitter.emit(&mut input, &mut output, b"only").unwrap();
    emitter.finish(&mut input, &mut output, b"done").unwrap();
    assert!(input.is_empty(), "the grant was read");
    let sent = frames(&output);
    assert_eq!(
        decode_end(&sent[1].1).unwrap(),
        (Reason::Finished, &b"done"[..])
    );
}

#[test]
fn a_control_frame_that_is_neither_a_grant_nor_a_stop_is_refused() {
    let wrong = control(ITEM, &item(0, b""));
    assert!(read_control(&mut &wrong[..]).is_err());
    let zero = control(super::CREDIT, &0u32.to_be_bytes());
    assert!(read_control(&mut &zero[..]).is_err());
}

fn slot_item(sequence: u32, slot: u32, length: u32) -> Vec<u8> {
    let mut out = encode_item_header(sequence, super::Carriage::Slot).to_vec();
    out.extend_from_slice(&super::encode_slot(slot, length));
    out
}

fn ring(slot_len: usize, slots: u32) -> crate::ring::Shape {
    crate::ring::Shape::new(slot_len, slots, u64::MAX).unwrap()
}

#[test]
fn a_ring_clamps_the_window_to_its_slots_and_items_take_their_slots_in_turn() {
    let (mut reception, first) =
        Reception::with_ring(n(8), budget(10, 1 << 20), Some(ring(64, 3))).unwrap();
    assert_eq!(
        first,
        n(3),
        "a grant never covers more items than the ring has slots"
    );
    for sequence in 0..3 {
        assert_eq!(
            reception.accept(ITEM, &slot_item(sequence, sequence, 64)),
            Ok(Accepted::Item(Body::Slot {
                slot: sequence,
                length: 64
            }))
        );
    }
    assert_eq!(reception.to_write(), Next::Credit(n(3)));
    assert_eq!(
        reception.accept(ITEM, &slot_item(3, 0, 1)),
        Ok(Accepted::Item(Body::Slot { slot: 0, length: 1 })),
        "item three is slot nought again: the ring has turned"
    );
    assert_eq!(
        reception.accept(ITEM, &item(4, b"small")),
        Ok(Accepted::Item(Body::Inline(b"small"))),
        "a small item still crosses inline"
    );
}

#[test]
fn an_item_in_a_slot_the_worker_does_not_hold_is_refused() {
    let (mut reception, _) =
        Reception::with_ring(n(3), budget(10, 1 << 20), Some(ring(64, 3))).unwrap();
    assert!(
        reception.accept(ITEM, &slot_item(0, 1, 8)).is_err(),
        "item nought is slot nought's"
    );
    let (mut reception, _) =
        Reception::with_ring(n(3), budget(10, 1 << 20), Some(ring(64, 3))).unwrap();
    assert!(
        reception.accept(ITEM, &slot_item(0, 0, 65)).is_err(),
        "no longer than a slot"
    );
    let (mut reception, _) = Reception::new(n(3), budget(10, 1 << 20)).unwrap();
    assert!(
        reception.accept(ITEM, &slot_item(0, 0, 8)).is_err(),
        "no slot without a ring"
    );
    let mut short = encode_item_header(0, super::Carriage::Slot).to_vec();
    short.extend_from_slice(&[0; 7]);
    assert!(super::decode_item(&short).is_err(), "a slot is eight bytes");
}

#[test]
fn a_slot_item_counts_its_length_against_the_bytes() {
    let (mut reception, _) =
        Reception::with_ring(n(2), budget(10, 100), Some(ring(64, 2))).unwrap();
    assert!(matches!(
        reception.accept(ITEM, &slot_item(0, 0, 64)),
        Ok(Accepted::Item(_))
    ));
    assert_eq!(
        reception.accept(ITEM, &slot_item(1, 1, 64)),
        Ok(Accepted::Discarded),
        "128 bytes are past 100"
    );
    assert_eq!(reception.stopping(), Some(Stopping::Spent(Spent::Bytes)));
}

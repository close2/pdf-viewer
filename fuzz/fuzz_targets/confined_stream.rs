//! Fuzzes `confined_transport::stream`'s framing from both sides (ADR 3501): the host's
//! `Reception` over a run of frames a subverted worker chose, and the worker's `read_control` and
//! `decode_open` over bytes a host chose.
//!
//! **The host's side is the one a document reaches.** A stream's items come from a worker that has
//! read a hostile file, and the host that reads them is not confined, so every frame a worker can
//! send — any kind, any length, any order — is a parser input here (RFC 0015 section 4.7).
//!
//! The input is read as a window — whose top bit attaches a ring of 64-byte slots, as many as its
//! low nibble and one — three budgets and then frames, each a kind byte, a two-byte
//! length and that many payload bytes; a length past the bytes left stands for a frame the host's
//! bounded read reads past. Beyond never panicking — overflow checks stay on in this profile — the
//! properties are the contract's:
//!
//! - **No grant passes the window or the frames left**, and **the host never writes while items it
//!   granted are still to come**, so at most one control frame is ever in flight.
//! - **No item is handed on past the frames budget, and none past the bytes budget**: the bodies
//!   handed on sum to at most the bytes budget.
//! - **An item handed on is never one past the credit granted.**
//! - **Nothing is accepted after the end.**
//! - **With a ring, no grant passes its slots, and a slot item handed on names the slot its
//!   sequence number holds and fits it** — the ring's ownership rule (ADR 3513).

#![no_main]
#![expect(
    clippy::expect_used,
    reason = "a fuzz target states its properties by failing: `expect` is how a violated one reaches libFuzzer, and each message names the property"
)]

use std::num::NonZeroU32;
use std::time::Duration;

use confined_transport::ring::Shape;
use confined_transport::stream::{self, Accepted, Body, Budget, Next, Reception};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = stream::decode_open(data);
    let _ = stream::decode_end(data);
    let _ = stream::read_control(&mut &data[..]);
    let _ = Shape::decode(data);
    let _ = stream::decode_item(data);

    let Some((head, mut frames)) = data.split_first_chunk::<7>() else {
        return;
    };
    let Some(window) = NonZeroU32::new(u32::from(head[0] & 0x7F)) else {
        return;
    };
    let ring = (head[0] & 0x80 != 0).then(|| {
        Shape::new(64, u32::from(head[0] & 0x0F) + 1, u64::MAX)
            .expect("sixteen slots of 64 bytes at most is a ring")
    });
    let budget = Budget {
        frames: u32::from(u16::from_be_bytes([head[1], head[2]])),
        bytes: u64::from(u32::from_be_bytes([head[3], head[4], head[5], head[6]])),
        wall: Duration::from_secs(1),
    };
    let Some((mut reception, first)) = Reception::with_ring(window, budget, ring) else {
        return;
    };
    assert!(
        first <= window && first.get() <= budget.frames,
        "the first grant is bounded"
    );
    let mut granted = u64::from(first.get());
    let mut handed_bytes = 0u64;
    let mut handed = 0u32;
    while let Some((&[kind, high, low], rest)) = frames.split_first_chunk::<3>() {
        let outstanding = reception.outstanding();
        match reception.to_write() {
            Next::Credit(credit) => {
                assert_eq!(outstanding, 0, "a grant only once the last has arrived");
                assert!(credit <= window, "a grant within the window");
                if let Some(ring) = ring {
                    assert!(credit <= ring.slots(), "a grant within the ring's slots");
                }
                granted = granted.saturating_add(u64::from(credit.get()));
                assert!(
                    granted <= u64::from(budget.frames),
                    "grants within the frames budget"
                );
            }
            Next::Stop => assert_eq!(outstanding, 0, "a stop only once the last has arrived"),
            Next::Read => {}
        }
        let length = usize::from(u16::from_be_bytes([high, low]));
        let Some(payload) = rest.get(..length) else {
            if reception.refused(kind, length).is_err() {
                return;
            }
            frames = &[];
            continue;
        };
        frames = rest.get(length..).unwrap_or_default();
        let before = reception.received();
        match reception.accept(kind, payload) {
            Ok(Accepted::Item(body)) => {
                if let Body::Slot { slot, length } = body {
                    let ring = ring.expect("no slot item without a ring");
                    assert_eq!(slot, ring.slot_of(before), "the slot its sequence holds");
                    assert!(
                        usize::try_from(length).is_ok_and(|length| length <= ring.slot_len()),
                        "a slot item fits its slot"
                    );
                }
                handed = handed.saturating_add(1);
                handed_bytes = handed_bytes.saturating_add(body.len());
                assert!(
                    u64::from(handed) <= granted,
                    "no item past the credit granted"
                );
                assert!(handed <= budget.frames, "no item past the frames budget");
                assert!(
                    handed_bytes <= budget.bytes,
                    "no bytes past the bytes budget"
                );
                assert_eq!(
                    reception.received(),
                    before.saturating_add(1),
                    "an item is counted"
                );
            }
            Ok(Accepted::Discarded) => {}
            Ok(Accepted::Ended { .. }) => {
                assert!(
                    reception.accept(kind, payload).is_err(),
                    "nothing is accepted after the end"
                );
                return;
            }
            Err(_) => return,
        }
    }
});

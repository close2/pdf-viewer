//! The deadline an in-process decode is held to, and what it costs that a thread cannot be killed.
//!
//! One test, and a test binary of its own, because what it asserts is process-wide: how many
//! abandoned decodes are still running decides whether the next one starts
//! ([`in_process::MAX_ABANDONED`]), and a second test in the same process would find the slots
//! this one fills. The order inside it is the argument: a decode that is slow for an honest reason
//! is abandoned and later gives its slot back, and then the seven symbol dictionaries of minutes,
//! which may never give theirs back, are each ended inside a second either way (ADR 1447).

use std::time::{Duration, Instant};

use pdf_sandbox::{Request, SandboxError, in_process};

mod symbol_dictionaries;

/// How long the honest decode below may take to finish once abandoned, before this test gives up
/// on seeing its slot come back. It takes about two seconds in a release build.
const RETURNS_WITHIN: Duration = Duration::from_mins(2);

/// A segment header of ITU-T T.88 section 7.2 for the embedded organisation: segment number,
/// flags carrying the type, no referred-to segments, page 1, and the data length.
fn segment(number: u32, kind: u8, data: &[u8]) -> Vec<u8> {
    let length = u32::try_from(data.len()).unwrap_or(u32::MAX);
    let mut out = Vec::with_capacity(data.len().saturating_add(11));
    out.extend_from_slice(&number.to_be_bytes());
    out.extend_from_slice(&[kind, 0x00, 0x01]);
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(data);
    out
}

/// A page and one immediate generic region covering it, `side` pixels square: section 7.4.8's
/// page information (type 48) and section 7.4.6's region (type 38), template 0 with its nominal
/// adaptive pixels, arithmetic-coded from four bytes. The decoder is fed 1-bits past them, so the
/// whole region decodes, and its cost is its area — a decode that is slow for an honest reason.
fn one_region(side: u32) -> Vec<u8> {
    let mut page = Vec::with_capacity(19);
    for field in [side, side, 0, 0] {
        page.extend_from_slice(&field.to_be_bytes());
    }
    page.extend_from_slice(&[0x00, 0x00, 0x00]);

    let mut region = Vec::with_capacity(30);
    for field in [side, side, 0, 0] {
        region.extend_from_slice(&field.to_be_bytes());
    }
    region.push(0x00);
    region.push(0x00);
    region.extend_from_slice(&[0x03, 0xff, 0xfd, 0xff, 0x02, 0xfe, 0xfe, 0xfe]);
    region.extend_from_slice(&[0x00; 4]);

    let mut stream = segment(0, 48, &page);
    stream.extend_from_slice(&segment(1, 38, &region));
    stream
}

#[test]
fn an_in_process_decode_past_its_deadline_is_abandoned_counted_and_ended() {
    // 2^28 pixels, the largest bilevel image the filter produces: seconds of honest work.
    let large = one_region(1 << 14);
    let small = one_region(64);
    let large = Request::Jbig2 {
        data: &large,
        globals: &[],
    };
    let small = Request::Jbig2 {
        data: &small,
        globals: &[],
    };
    let deadline = Duration::from_millis(100);

    for already in 0..in_process::MAX_ABANDONED {
        assert_eq!(in_process::overran_still_running(), already);
        let started = Instant::now();
        let outcome = pdf_sandbox::decode_here_within(&large, deadline);
        assert!(
            matches!(outcome, Err(SandboxError::Overran { after }) if after == deadline),
            "a decode past its deadline is abandoned and says so: {outcome:?}"
        );
        assert!(started.elapsed() < symbol_dictionaries::REFUSED_WITHIN);
    }
    assert!(
        matches!(
            pdf_sandbox::decode_here_within(&small, deadline),
            Err(SandboxError::TooManyOverran { running }) if running == in_process::MAX_ABANDONED
        ),
        "with every slot held, the next decode is refused rather than started"
    );

    // An honest decode ends, and when it does its slot and its thread come back.
    let waiting = Instant::now();
    while in_process::overran_still_running() > 0 {
        assert!(
            waiting.elapsed() < RETURNS_WITHIN,
            "an abandoned decode that ends gives its slot back"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        pdf_sandbox::decode_here_within(&small, Duration::from_secs(30)).is_ok(),
        "and the next decode is answered"
    );

    // Now the inputs that may hold their slots for ever: each is ended inside the bound, by the
    // deadline, by the refusal to start another, or by the codec's own sentence where it has one.
    for (name, framed) in symbol_dictionaries::INPUTS {
        let started = Instant::now();
        let outcome = pdf_sandbox::decode_here_within(
            &symbol_dictionaries::request(framed),
            symbol_dictionaries::DEADLINE,
        );
        let spent = started.elapsed();
        assert!(
            symbol_dictionaries::ended(&outcome),
            "{name}: ended, not {outcome:?}"
        );
        assert!(
            spent < symbol_dictionaries::REFUSED_WITHIN,
            "{name}: inside the bound: {spent:?}"
        );
    }
}

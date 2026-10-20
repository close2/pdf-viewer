//! The deadline an in-process decode is held to, and what it costs that a thread cannot be killed.
//!
//! One test, and a test binary of its own, because what it asserts is process-wide: how many
//! abandoned decodes are still running decides whether the next one starts
//! ([`in_process::MAX_ABANDONED`]), and a second test in the same process would find the slots
//! this one fills. The order inside it is the argument: a decode that is slow for an honest reason
//! is abandoned and later gives its slot back, and then the seven symbol dictionaries of minutes,
//! which would never give theirs back, are each refused by the codec's own bound (ADR 1447).

use std::time::{Duration, Instant};

use pdf_sandbox::{Request, SandboxError, in_process};

mod symbol_dictionaries;

/// How long the honest decode below may take to finish once abandoned, before this test gives up
/// on seeing its slot come back. It takes about two seconds in a release build.
const RETURNS_WITHIN: Duration = Duration::from_mins(2);

#[test]
fn an_in_process_decode_past_its_deadline_is_abandoned_counted_and_ended() {
    // 2^28 pixels, the largest bilevel image the filter produces: seconds of honest work.
    let large = symbol_dictionaries::one_region(1 << 14);
    let small = symbol_dictionaries::one_region(64);
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
        // A poll interval, kept as a timed bound: the count is all `in_process` says of an
        // abandoned decode's end, and a notification beside it would be machinery in the decode
        // path for this test alone. The verdict is the slot's return within `RETURNS_WITHIN`; the
        // interval is only how late it is seen (ADR 1800).
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        pdf_sandbox::decode_here_within(&small, Duration::from_secs(30)).is_ok(),
        "and the next decode is answered"
    );

    // Now the inputs that would hold their slots for ever: each is refused by the codec's own
    // bound, before any deadline is reached.
    for (name, framed) in symbol_dictionaries::INPUTS {
        let outcome = pdf_sandbox::decode_here_within(
            &symbol_dictionaries::request(framed),
            symbol_dictionaries::BOUND_DEADLINE,
        );
        assert!(
            symbol_dictionaries::refused_by_the_codec(&outcome),
            "{name}: refused by the codec's bound, not {outcome:?}"
        );
    }
}

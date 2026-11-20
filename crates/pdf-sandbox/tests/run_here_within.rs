//! [`pdf_sandbox::run_here_within`]: a caller's own decoder held to the in-process budget.
//!
//! One test, and a binary of its own, for `in_process_deadline.rs`'s reason: abandoned work is
//! counted process-wide, and the order inside the test is the argument (ADR 3289).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use pdf_sandbox::{SandboxError, in_process, run_here_within};

#[test]
fn work_run_here_answers_spends_its_budget_or_says_it_ended_without_answering() {
    // Work that finishes answers with its value.
    assert_eq!(
        run_here_within(Duration::from_secs(5), None, || 41 + 1).ok(),
        Some(42)
    );

    // Work that sleeps spends no processor time, so a budget shorter than the sleep does not end
    // it; the caller's other work is asked for while it waits.
    let mut asked = 0_u32;
    let mut meanwhile = || {
        asked = asked.saturating_add(1);
        false
    };
    let slept = run_here_within(
        Duration::from_millis(200),
        Some(&mut meanwhile as &mut dyn FnMut() -> bool),
        || std::thread::sleep(Duration::from_millis(50)),
    );
    assert!(slept.is_ok(), "{slept:?}");
    assert!(
        asked >= 1,
        "the waiting caller was asked for its other work"
    );

    // Work that spins spends its budget and is abandoned: the image's refusal, or under a load
    // that starves its thread the ceiling's, the machine's (ADR 3150).
    let stop = Arc::new(AtomicBool::new(false));
    let spinning = Arc::clone(&stop);
    let refused = run_here_within(Duration::from_millis(100), None, move || {
        let mut turns = 0_u64;
        while !spinning.load(Ordering::Relaxed) {
            turns = std::hint::black_box(turns.wrapping_add(1));
        }
        turns
    });
    assert!(
        matches!(
            refused,
            Err(SandboxError::Exhausted { .. } | SandboxError::Overran { .. })
        ),
        "{refused:?}"
    );
    assert_eq!(
        in_process::overran_still_running(),
        1,
        "abandoned, and counted"
    );
    stop.store(true, Ordering::Relaxed);
    let began = Instant::now();
    while in_process::overran_still_running() > 0 {
        assert!(
            began.elapsed() < Duration::from_secs(60),
            "the abandoned work returned its slot"
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    // Work that panics ends its thread without an answer, and says so.
    let panicked = run_here_within(Duration::from_secs(5), None, || -> u8 {
        panic!("a decoder's defect")
    });
    assert!(
        matches!(panicked, Err(SandboxError::Malformed { .. })),
        "{panicked:?}"
    );
}

//! How many threads this library has started, counted where each one is started.
//!
//! A frame enters threads inside [`Device::render`](crate::device::Device::render) and has left
//! them before the call returns (`encode/parallel.rs`'s module comment has the host's reasons), so
//! every fan-out that takes threads spawns them again: the encode's rasterising fan-out once per
//! drain of its queue, the exact meet's helpers once per frame that reaches their floor, and the
//! device's ramp tables and image reductions once per realisation that reaches theirs. What that
//! costs a page turn is a question about how many there are, and no other instrument says:
//! `strace -f` can count `clone3` but cannot tell one frame from the next, and the kernel keeps no
//! running count a process can read.
//!
//! **A process's count, read as a difference.** The figure is cumulative, like a page-fault
//! count, so a caller reads it before and after a frame (`examples/first_frame`,
//! `examples/frame_budget`) and the difference is the frame's. It counts threads *started*,
//! never threads that failed to start, and it is a relaxed atomic addition per thread beside a
//! thread start that costs thousands of times more (ADR 1686).

use std::sync::atomic::{AtomicU64, Ordering};

/// Threads started since the process began, by every fan-out this library runs.
static STARTED: AtomicU64 = AtomicU64::new(0);

/// How many threads this library has started in this process, all frames and all devices
/// together: read it before and after a frame, and the difference is the frame's.
#[must_use]
pub fn started() -> u64 {
    STARTED.load(Ordering::Relaxed)
}

/// Count `count` threads as started.
pub(crate) fn count(count: usize) {
    STARTED.fetch_add(u64::try_from(count).unwrap_or(u64::MAX), Ordering::Relaxed);
}

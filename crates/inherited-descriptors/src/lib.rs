//! A helper program's first statement: close every descriptor it inherited past standard input,
//! output and error.
//!
//! **Why a helper closes them, and why only the helper can.** A library that spawns in the parent
//! beside a helper's start — `GLib`'s `dbus-launch`, whose pipes are made without close-on-exec —
//! hands every such descriptor to a child started meanwhile, and a write end held in the child is
//! an end of file the library waits for and never gets (ADR 1907). Every parent in this tree passes
//! a helper exactly the three, so whatever else is open at `main` is somebody else's leak; in a
//! confined worker it would also be a channel the sandbox did not grant. The defect itself is the
//! library's, and the owner wants it fixed there too (`doc/questions/A430`, ADR 1945 §2).
//!
//! **Why one crate.** The close is one `unsafe` call, and each program that made it used to lift
//! `unsafe_code` for it, so every confined helper added a name to the tree's list of crates that
//! lift the denial (`viewer-qt`'s `tests/unsafe_position.rs`). The call is here once, audited once,
//! and every helper calls [`keep_only_the_standard_descriptors`] with no `unsafe` of its own (ADR
//! 3361, `doc/questions/A979`).
//!
//! **What keeps the safe signature honest** is where it is called, which the type system cannot
//! say: closing a descriptor by number is sound only while no object in the process owns one past
//! the three. `tests/callers.rs` reads every source in the tree back and asserts that each call is
//! a program's `main`'s first statement, and that nothing else names this crate.

#![deny(unsafe_code)]

/// Closes every descriptor this process inherited past standard input, output and error (ADR 1945).
///
/// **Call it as `main`'s first statement and nowhere else.** At that point nothing in the process
/// has opened a descriptor or made an object that owns one past the standard three, so nothing is
/// left to use or close what this closes. Called later, it would close descriptors that objects of
/// the process own — what the standard library's I/O safety exists to prevent — and
/// `tests/callers.rs` fails on any call that is not in that position.
///
/// On Linux one `close_range(3, ~0, 0)`. Elsewhere it does nothing: `close_range` is Linux's, no
/// other platform confines these helpers yet (`doc/todo/35`), and each owes its own answer with its
/// confinement.
///
/// # Errors
///
/// The system's error where the kernel refuses the close. A helper that could not let go of a
/// leaked descriptor says so on standard error and exits with failure rather than carry it into
/// its confinement.
#[cfg(target_os = "linux")]
#[expect(
    unsafe_code,
    reason = "closes by number the descriptors no object in this process owns (ADR 1945)"
)]
pub fn keep_only_the_standard_descriptors() -> std::io::Result<()> {
    // SAFETY: the function's contract, held by `tests/callers.rs`, is that it is `main`'s first
    // statement, before anything in this process has opened a descriptor or made an object that
    // owns one past the standard three, so no owner is left to use or close what this closes; and
    // `close_range` reads and writes none of this process's memory.
    let closed = unsafe { libc::close_range(3, u32::MAX, 0) };
    if closed == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// Closes nothing: `close_range` is Linux's, and no other platform confines these helpers yet
/// (`doc/todo/35`).
///
/// # Errors
///
/// None on these platforms; the signature is Linux's, so that a caller is written once.
#[cfg(not(target_os = "linux"))]
pub fn keep_only_the_standard_descriptors() -> std::io::Result<()> {
    Ok(())
}

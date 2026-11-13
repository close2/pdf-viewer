//! The two macOS calls `pdf-sandbox` needs and cannot make itself, each behind a safe function.
//!
//! **Why a crate of its own.** `pdf-sandbox` decodes untrusted images and is
//! `#![forbid(unsafe_code)]`, which `CLAUDE.md`'s third principle makes a compiler fact for every
//! crate that touches PDF bytes. On Linux the calls its confinement needs are wrapped by crates
//! written for the purpose — `landlock`, `seccompiler`, `rustix` — and nothing in this tree spells
//! an `unsafe` block for them. macOS has no such crate for the two calls here, so this is that
//! crate: it reads no PDF byte, holds no state, and each `unsafe` block is one foreign call whose
//! contract is written beside it (`doc/todo/35`, ADR 2864).
//!
//! - [`enter`] is `sandbox_init(3)`: the calling process enters a Seatbelt profile, irreversibly.
//!   Apple marked the function deprecated in OS X 10.8 in favour of the App Sandbox's
//!   entitlements, which attach to a signed application bundle when it is built and so cannot
//!   confine a helper started with a pipe. The mechanism under both is the same kernel policy
//!   module, and browsers still confine their renderers with it; ADR 2864 has the argument.
//! - [`usage`] is `proc_pid_rusage(2)` with `RUSAGE_INFO_V2`: another process's physical
//!   footprint, the number macOS itself charges a process for memory, and the start time that
//!   says the process is still the one first asked about.
//!
//! On every other platform the crate is empty.

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::{EnterError, Usage, enter, usage};

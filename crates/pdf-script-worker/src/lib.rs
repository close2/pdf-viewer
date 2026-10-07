//! A document's scripts run in a confined process of their own: RFC 0008 section 6.2's third worker.
//!
//! One responsibility: *carry a field event to an engine that cannot reach anything, and carry back
//! what the script did*. The engine is `pdf_script`'s; this crate is where it runs and how a host
//! reaches it. Two halves, and only one of them needs the engine:
//!
//! - **The host's half** ([`ScriptWorker`]) is a `pdf_model::view::ScriptRunner`, the one place a
//!   host's level for scripts reaches a view state (ADR 1591). Constructing one spawns nothing: the
//!   worker is started on the **first trigger** the view state hands it, so a document with no
//!   script, or a reader whose level supplies no runner, never starts it (RFC 0008 section 6.6).
//!   Every trigger has a deadline, and a worker that misses it, or dies, is killed, named by the
//!   trigger it was running, and replaced at the next trigger (ADR 1609).
//! - **The worker's half** ([`serve`], behind the `engine` feature) confines its own process with
//!   `pdf_sandbox::lockdown::Profile::Script` before it reads a byte — one thread, no descriptor,
//!   no executable memory, a ceiling in the tens of megabytes (ADR 1608) — says what the kernel
//!   granted, and answers each run with `pdf_script::run` under `pdf_script::Budget::FIELD_EVENT`.
//!
//! [`wire`] is what crosses between them: the script once, then the event's fields, and back an
//! `Outcome`. The worker never holds the document's bytes; it is handed text and gives text back.
//!
//! # Why not inside `pdf-view-worker`
//!
//! RFC 0008 section 6.2: the budget has to bound the hostile thing without bounding the work the
//! person is waiting for. A script that spins under the view worker's address-space ceiling takes
//! the page with it; here it takes a process that held nothing but the scripts, and the page is
//! still drawn.

#![forbid(unsafe_code)]

mod client;
pub mod wire;

#[cfg(feature = "engine")]
mod worker;

pub use client::{Cause, DEADLINE, Death, MAX_DEATHS, OpenCost, ScriptWorker};

#[cfg(feature = "engine")]
pub use worker::serve;

/// The worker program's name, which is also the name its diagnostics are reported under.
pub const WORKER_PROGRAM: &str = "pdf-script-worker";

/// The variable that names the worker program where it is not beside the running executable.
pub const WORKER_PATH_VARIABLE: &str = "PDF_SCRIPT_WORKER";

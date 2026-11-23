//! A sound's samples decoded in a confined process of its own: RFC 0015 section 5's first step.
//!
//! One responsibility: *carry a sound object's data to a decoder that cannot reach anything, and
//! carry back samples a sink can play*. ISO 32000-2 §13.3 states the formats — raw, signed, µ-law
//! and A-law samples under Table 305's `/R`, `/C`, `/B` and `/E` — and "Sound players shall convert
//! between formats, downsample rates, and combine channels as necessary"; this crate does the
//! first of the three, and the window's sink the other two (ADR 3297). Two halves:
//!
//! - **The host's half** ([`MediaWorker`]) starts nothing when it is made. The worker is started at
//!   the first sound decoded — the first play the reader's level lets run — and never at open, so
//!   nothing here is on the launch path (RFC 0015 section 4.6). Each request is waited for under a
//!   processor-time budget read from outside the worker, with the wall clock as the machine's
//!   ceiling behind it (ADR 3133), and a worker that spends it is killed and replaced at the next
//!   sound.
//! - **The worker's half** ([`serve`]) confines its own process with
//!   `pdf_sandbox::lockdown::Profile::Media` before it reads a byte — no filesystem, no network, no
//!   descriptor, one thread, no executable memory, an address-space ceiling sized against one
//!   request — says what the kernel granted, and answers each request with [`sound::decode`].
//!
//! [`wire`] is what crosses between them; [`sound`] decodes §13.3's samples and [`aac`] an MP4
//! clip's AAC, and [`wave`] and [`mp4`] find a file's samples. Those four are the only code here
//! that reads a byte the document wrote, run only in the worker and by the fuzz targets.
//!
//! # Why not inside `pdf-view-worker`
//!
//! RFC 0008 section 6.2's reason, which RFC 0015 section 4.1 applies to every clause 13 payload:
//! the budget has to bound the hostile thing without bounding the work the person is waiting for.
//! A decoder that exhausts its ceiling takes a process that held nothing but one request's bytes,
//! and the page is still drawn.

#![forbid(unsafe_code)]

pub mod aac;
mod client;
pub mod mp4;
pub mod sound;
pub mod wave;
pub mod wire;
mod worker;

pub use client::{
    Deliver, MAX_TOP_LEVEL_BOXES, MAX_WAVE_WINDOWS, MediaError, MediaWorker, PROCESSOR_BUDGET, Run,
    Sound, Streamed,
};
pub use worker::{Worker, answer, serve};

/// The worker program's name, which is also the name its diagnostics are reported under.
pub const WORKER_PROGRAM: &str = "pdf-media-worker";

/// The variable that names the worker program where it is not beside the running executable.
pub const WORKER_PATH_VARIABLE: &str = "PDF_MEDIA_WORKER";

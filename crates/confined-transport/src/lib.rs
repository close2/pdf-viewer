//! The wire under a confined worker, with no opinion about what crosses it.
//!
//! Two programs in this tree hand a document to a process that has no filesystem and ask it
//! questions: `viewer-confined`'s `pdf-view-worker`, which interprets and draws a page for a
//! window (ADR 0713), and `pdf-vfs`'s `pdf-vfs-worker`, which generates the files a
//! document-as-a-directory offers (RFC 0003 section 6). **Their vocabularies have nothing in
//! common and their transport is the same transport**, so it is here rather than twice: the frame
//! header, the greeting that carries what the kernel actually granted, the socket that carries a
//! descriptor beside a frame, the supervision of the child that answers on it, and the arithmetic
//! that turns an address-space ceiling into a message budget.
//!
//! # What is *not* here, and why the line is where it is
//!
//! A message. Neither crate's `Query`, `Command`, `Answer` or `Event` appears in this one, and
//! neither ever will: what makes a transport shareable is that it carries a kind byte and a
//! length and does not know what either means. So this crate defines no discriminants, validates
//! no kind — [`frame::parse_header`] answers a length and hands the kind back untouched, and the
//! caller says whether its own protocol defines it — and holds no encoder.
//!
//! **The confinement is not here either.** [`pdf_sandbox::lockdown`] installs it and this crate
//! merely reports what it reached, which is the direction that keeps the allow-list a property of
//! the *work* rather than of the wire: a worker that needed a system call would be asking
//! `pdf-sandbox` for a profile, not asking this crate for a feature.
//!
//! # The shape of a conversation
//!
//! A host calls [`Host::start`] with the worker's path and its own eight-byte magic. That spawns
//! the program with a socket as its standard input, a pipe as its standard output, a pipe as its
//! standard error — never the inherited descriptor, because `RLIMIT_FSIZE` is zero inside the
//! confinement and a worker whose standard error is a *file* is killed by `SIGXFSZ` before it can
//! explain itself (ADR 0597) — and reads the greeting. Every exchange after that is
//! [`Host::exchange`]: one frame out, one frame back.
//!
//! The worker's side is [`link::Link`], which reads its frames with `recvmsg` so that a descriptor
//! sent beside a header arrives with it, and [`greeting::encode`], which it writes before it reads
//! anything at all.

#![forbid(unsafe_code)]

pub mod ceiling;
pub mod frame;
pub mod greeting;
pub mod link;
mod supervision;

#[cfg(not(unix))]
mod channel_pipe;
#[cfg(unix)]
mod channel_unix;

mod host;

pub use crate::host::{Host, TransportError};
pub use crate::supervision::{Canceller, ProgramMissing, describe_exit, program_beside_executable};

#[cfg(not(unix))]
pub use crate::channel_pipe::{Channel, SentDescriptor};
#[cfg(unix)]
pub use crate::channel_unix::{Channel, SentDescriptor};

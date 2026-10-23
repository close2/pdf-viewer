//! The C entry points, and the only `unsafe` in this crate.
//!
//! Every function here does the same three things and nothing else: turn raw arguments into safe
//! Rust values, call one safe function in a module beside this one, and write the result through
//! an out-parameter. **No decision is taken in this file** — not about a clause, not about a
//! command's meaning, not about an error's wording — so that the audit a reviewer owes is
//! "does this handle pointers correctly" and never "is this the right answer".
//!
//! # The pointer contract, stated once
//!
//! It is the same for every function and is not repeated in each `# Safety` section beyond a
//! pointer back here:
//!
//! - every pointer is either null or valid for the type it names, aligned, and pointing at a
//!   live object this library produced (`quorra_viewer_new`, `quorra_open`, …);
//! - **null is always checked** and answers [`Status::NullArgument`](crate::status::Status::NullArgument). It is the one bad pointer
//!   this side can detect, and detecting it is worth doing precisely because it is the one a C
//!   caller produces by accident rather than by arithmetic;
//! - an owning handle is freed exactly once, with its own `_free`, and is not used afterwards;
//! - a buffer given for output is writable for the number of bytes stated beside it;
//! - a `const char *` argument is NUL-terminated and is UTF-8. A password and a fragment are the
//!   host's own strings, and one that is not UTF-8 is refused rather than repaired: an invented
//!   replacement character in a password is a password that does not open the file, said quietly;
//! - **no handle may be used from two threads at once.** A `quorra_render_request *` may be *moved*
//!   to another thread and rasterised there, which is what the round trip is for; a
//!   `quorra_viewer *` may not be shared, exactly as `viewer-core` is not `Sync`.
//!
//! # Why `unsafe fn` and why `unsafe_op_in_unsafe_fn` is lifted here
//!
//! Every entry point is `pub unsafe extern "C" fn`. C does not see the word — the symbol and the
//! calling convention are identical — and Rust does: a function with preconditions a compiler
//! cannot check says so in its signature, which is the whole of what `unsafe fn` means. A safe
//! `extern "C" fn` that dereferenced its arguments would be an unsound API for any Rust caller,
//! and this crate is an `rlib` as well as a `cdylib`.
//!
//! `unsafe_op_in_unsafe_fn` is lifted for this module because these bodies are *entirely* the
//! unsafe operation: a block around three lines of a four-line shim marks nothing out, and the
//! lint's value — "here, inside this function, is the part that is delicate" — is not available
//! where the answer is "all of it". The position that replaces it is the one
//! `tests/unsafe_position.rs` enforces: **one `unsafe` token per entry point, in the signature,
//! and none anywhere else in the crate.**
//!
//! # Panics
//!
//! None of these panics. An `extern "C"` function that unwound would abort the process, which is
//! Rust's own defined behaviour for it and is not something this crate relies on: every fallible
//! step returns a [`Status`](crate::status::Status) and every index is checked before it is used.

// See the module documentation. The lift is deliberate, narrow to this file, and the position it
// gives up is replaced by a test that reads the sources back.
#![allow(unsafe_op_in_unsafe_fn)]

mod collection_reads;
mod commands;
mod conversion;
mod edits;
mod event_stream;
mod forms;
mod host;
pub(crate) mod identity;
pub(crate) mod layouts;
mod page_reads;
mod panel_reads;
mod pointer;
mod popup_reads;
mod queries;
mod rendering;
mod shared;
mod structure_reads;
mod window_calls;

pub use self::collection_reads::*;
pub use self::commands::*;
pub use self::conversion::*;
pub use self::edits::*;
pub use self::event_stream::*;
pub use self::forms::*;
pub use self::host::*;
pub use self::identity::*;
pub use self::layouts::*;
pub use self::page_reads::*;
pub use self::panel_reads::*;
pub use self::pointer::*;
pub use self::popup_reads::*;
pub use self::queries::*;
pub use self::rendering::*;
pub use self::structure_reads::*;
pub use self::window_calls::*;

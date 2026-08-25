//! A Qt 6 Widgets host for [`viewer_core`] — the *second* native consumer of the UI boundary,
//! and the one with a C++ bridge.
//!
//! # Why this crate exists, and what its product is
//!
//! `doc/todo/30` states the order and calls it non-negotiable: GTK4 first because `gtk4-rs` is a
//! safe Rust binding with no C++ bridge, **Qt second because it costs one and should not be the
//! experiment that shapes the interface**, and `viewer-ffi` last — *"do not freeze a C ABI until
//! two Rust consumers have shaken the API out"*. This is the second of those two consumers.
//!
//! Its product is therefore not an application but an answer: **what a toolkit unlike GTK4 asks
//! of the boundary.** The four-hundred-and-eighth session's headline was that a whole native host
//! added no new message; this crate tests that against a different widget model
//! (`QAbstractItemModel` against `GtkTreeListModel`), a different ownership model (C++ owns the
//! host; in GTK the Rust side owns the widgets) and a different language on the far side. ADR
//! 0246 is the comparison, and the short answer is that the vocabulary needed nothing again.
//!
//! # Tier 1, and Qt Widgets offers no other either
//!
//! `doc/ui-boundary.md` names three pixel tiers: a CPU raster, a raw window handle we drive
//! ourselves, and the host's own GPU texture. This host takes tier 1 — [`viewer_core::Rendered`]`::Raster`
//! from `render-cpu`, copied into a `QImage` and painted by `QPainter`. That is the tier a
//! `QWidget` admits: a widget has no surface of its own to hand a foreign renderer (Qt composites
//! its own backing store), and `QRhi`, which owns the device Qt draws through, is a private
//! module a Qt version may change without notice. A `QOpenGLWidget` or a `QVulkanWindow` would
//! reach tier 2, and both are a different widget with a different place in a layout — so the
//! *comparable* host, the one that puts the page in a `QSplitter` beside a real sidebar, is tier
//! 1. ADR 0246.
//!
//! And the copy is the same copy: [`pdf_render::Raster`] is `QImage::Format_RGBA8888` exactly, so
//! there is no conversion at all, only a `memcpy` — which is what makes the number comparable
//! with ADR 0244's ≈3.2 GB/s.
//!
//! # `#![deny(unsafe_code)]`, and what that costs
//!
//! Every crate in this tree that touches PDF bytes holds `#![forbid(unsafe_code)]`, and
//! `viewer-gtk` holds it too because `gtk4-rs` is a safe binding. **This crate cannot**:
//! `#[cxx::bridge]` expands to `unsafe extern "C"` declarations, `unsafe` blocks and
//! `#[export_name]` functions, and a `forbid` cannot be lifted by an inner `allow` — which is the
//! whole point of `forbid` and the reason it is the attribute the parsers carry.
//!
//! So the position is stated exactly, and it is narrower than "this crate has unsafe":
//!
//! - the crate root **denies** `unsafe_code`;
//! - the exemption is one attribute, on `mod bridge`, and it is the only one in this tree;
//! - **the whole crate contains one hand-written `unsafe` token**: the `unsafe extern "C++"`
//!   block header. That token is not a licence but an *obligation* — it is `cxx`'s way of asking
//!   the author to assert that the C++ declared there is the C++ that exists and is safe to call
//!   with those types, and it is discharged by `cpp/host.h` declaring one function with no Qt
//!   type in it and `cpp/window.cpp` defining exactly that one;
//! - every other `unsafe` in the compiled crate is macro expansion, which no source line
//!   contains.
//!
//! `tests/unsafe_position.rs` reads the crate's own sources back and asserts all four, including
//! the file and line of the one token — because a `deny` with an exemption is a rule with a hole
//! in it, and a hole nobody measures is a hole that grows.
//!
//! `doc/todo/30` reserves `unsafe` for `viewer-ffi` alone. ADR 0246 argues that the rule was
//! about *reviewable* `unsafe` — a promise a person makes that a compiler cannot check — and that
//! it survives with one such promise made in one place, checked by a test rather than claimed by
//! a comment.

#![deny(unsafe_code)]

mod access;

// The one exemption, and the reason is in `bridge`'s own documentation: `#[cxx::bridge]` is a
// procedural macro whose expansion is `unsafe` by construction, and the lint sees expanded code.
// Nothing under this attribute is written by hand — `tests/unsafe_position.rs` is what says so.
//
// The other two are the same phenomenon and are worth naming rather than lumping in: the macro
// re-emits each shared struct's name qualified (`unused_qualifications`) and marks every generated
// item `pub` inside a module this crate does not export (`unreachable_pub`). Neither is a choice
// this crate made and neither can be fixed inside it. `expect` rather than `allow` for those two,
// so that a `cxx` release which stops emitting them turns this into a warning to delete.
#[allow(unsafe_code)]
#[expect(
    unused_qualifications,
    unreachable_pub,
    reason = "cxx's expansion qualifies the names it generates and makes them `pub` in a module               this crate keeps to itself; neither is reachable from a hand-written line"
)]
mod bridge;
mod host;
mod keys;
mod page;

pub use host::{Host, HostError};

/// Builds the Qt window around a host, shows it, and runs the application.
///
/// Blocks until the window is closed or `quit_after` milliseconds have passed, whichever comes
/// first; zero means never, which is what a person gets and a run under `Xvfb` does not.
///
/// **This is the only call from Rust into C++ in the whole crate**, which is what keeps the
/// ownership story to one sentence: C++ takes the host and owns it for the life of
/// `QApplication::exec`, and everything after this line is Qt calling in.
#[must_use]
pub fn run(host: Host, quit_after: i32) -> i32 {
    bridge::ffi::run_qt_host(Box::new(host), quit_after)
}

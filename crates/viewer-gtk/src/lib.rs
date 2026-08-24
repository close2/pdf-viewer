//! A GTK4 host for [`viewer_core`] — the first *native* consumer of the UI boundary.
//!
//! `doc/todo/30` states the order and calls it non-negotiable: GTK4 first because it is Rust-safe
//! with no C++ bridge and it is this project's development platform, Qt second, and the C ABI
//! last, because a frozen ABI is the one mistake that cannot be taken back and two Rust consumers
//! have to shake the vocabulary out before it is worth freezing. This crate is the first of those
//! two; `crates/viewer-qt` is the second.
//!
//! # What is here, and what turned out not to be GTK's
//!
//! Four of this crate's original eight modules named no GTK type at all, and the second host wanted
//! all four unchanged — so they are [`viewer_host`] now: §12.3.3's, §8.11.4.3's, §7.11.4's,
//! §12.4.3's and §14.3.3's answers as one row shape, which of them a window offers as a panel
//! ([`viewer_host::Tab`]), §12.7.5's field as the control it is, §12.7.6.4's file policy, and
//! the launch timeline. What is left here is the toolkit and nothing else, which is why this
//! crate's whole public interface is [`Host`] and [`HostError`] (ADR 0246).
//!
//! # What it is for, and what it is not
//!
//! `viewer-ui` (winit and a graphics device) and `viewer-core/tests/headless.rs` (no display at
//! all) are the two consumers that already exist, and neither can answer the question this one
//! asks. `viewer-ui` draws its own sidebar, its own selection and its own form; it proves that
//! [`viewer_core::Query`] answers enough **for us**. A native host proves the answers are enough
//! for *somebody else's widgets* — a [`gtk4::ListView`] over §12.3.3's outline, a
//! [`gtk4::Entry`] over §12.7.5.3's text field, a [`gtk4::CheckButton`] over §12.7.5.2.3's check
//! box, and chrome drawn in the colour the desktop chose rather than one this project picked.
//!
//! So the product of this crate is **what the boundary turns out to be missing**, and the honest
//! answer is written down in ADR 0244. The largest of them, hit on the first
//! run: a page drawn *without* its widget appearances. A native control placed over a widget's
//! rectangle sat on top of the appearance stream this program drew for the same widget, so a
//! person saw the field twice — **closed in the four-hundred-and-ninth session** by
//! [`viewer_core::Command::Delegate`], which this host sends on every open (ADR 0245).
//!
//! # Tier 1, and GTK4 offers no other
//!
//! `doc/ui-boundary.md` names three pixel tiers: a CPU raster, a raw window handle we drive
//! ourselves, and the host's own GPU texture. This host takes tier 1 — [`viewer_core::Rendered`]`::Raster`
//! from `render-cpu`, wrapped in a [`gtk4::gdk::MemoryTexture`] and uploaded by GSK. That is not
//! only the honest first step; it is the only tier GTK4's public API admits. GTK4 gives a widget
//! no native surface of its own (tier 2 needs one, and `raw-window-handle` has nothing to bind
//! to below the toplevel), and it exposes neither its Vulkan device nor its GL context for a
//! foreign renderer to share (tier 3). ADR 0244 argues it with the measurement of what the copy
//! costs.
//!
//! # `#![forbid(unsafe_code)]`, in a crate that binds a C library
//!
//! That is the property `doc/todo/30` chose GTK4 for. `gtk4-rs` is a safe binding: the `unsafe`
//! is inside `gtk4-sys` and `glib`, and a host written against the safe layer needs none of its
//! own. Every crate in this tree that touches PDF bytes keeps the same attribute, and none of
//! them may name a toolkit — this crate is the only place `gtk4` appears in a manifest.

#![forbid(unsafe_code)]

mod controls;
mod host;
mod page;
mod pages;
mod tree;

pub use host::{Host, HostError};

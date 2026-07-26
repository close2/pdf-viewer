//! PDF object model validation, generated from the Arlington PDF Model.
//!
//! The Arlington PDF Model (`doc/arlington-pdf-model`) describes every object type
//! in ISO 32000-2 as tab-separated data: required keys, permitted types, default
//! values, the version a key appeared in, and the version it was deprecated in.
//!
//! This crate's `build.rs` turns that data into Rust validation tables and typed
//! accessors. The reason is deliberate: hand-writing conformance checks for the
//! whole object model would mean thousands of `if` statements that no reviewer could
//! meaningfully audit against the specification. Generating them keeps conformance
//! reviewable as *data*, and makes version-awareness a property of the input rather
//! than something each check must remember.
//!
//! Implemented in Phase 5C, which first resolves how much of Arlington's
//! `SpecialCase` predicate language is worth handling in codegen.

#![forbid(unsafe_code)]

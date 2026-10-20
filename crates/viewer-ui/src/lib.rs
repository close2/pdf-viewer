//! Window, input, dialogs and accessibility.
//!
//! The shell: a `winit` window and event loop, input translated into `viewer-core`
//! intents, cards this crate draws itself where a toolkit would open a dialogue — the file
//! to open is a path typed into one — and an accessibility tree exposed through `AccessKit`.
//!
//! Drawing those cards itself is what keeps this crate free of every widget toolkit, which
//! ADR 2048 makes a rule and `tools/conformance/tests/toolkits.rs` checks. `AccessKit` covers
//! the other reason a mature toolkit would have been needed, exposing AT-SPI on Linux so a
//! custom-drawn interface is still reachable by a screen reader.
//!
//! Accessibility is treated as a requirement here rather than an enhancement,
//! because it is far more expensive to retrofit than to build in.
//!
//! Implemented after Phase 5.

#![forbid(unsafe_code)]
pub mod chrome;
pub mod clipboard;
pub mod software;

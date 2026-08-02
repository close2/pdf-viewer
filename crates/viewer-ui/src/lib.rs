//! Window, input, dialogs and accessibility.
//!
//! The shell: a `winit` window and event loop, input translated into `viewer-core`
//! intents, file dialogs through the XDG desktop portal via `ashpd`, and an
//! accessibility tree exposed through `AccessKit`.
//!
//! Using the portal rather than a toolkit's own dialog is what allows this crate to
//! avoid Qt while still presenting the desktop's native file chooser — under Plasma,
//! `xdg-desktop-portal-kde` serves the real KDE dialog. `AccessKit` covers the other
//! reason a mature toolkit would have been needed, exposing AT-SPI on Linux so a
//! custom-drawn interface is still reachable by a screen reader.
//!
//! Accessibility is treated as a requirement here rather than an enhancement,
//! because it is far more expensive to retrofit than to build in.
//!
//! Implemented after Phase 5.

#![forbid(unsafe_code)]
pub mod chrome;

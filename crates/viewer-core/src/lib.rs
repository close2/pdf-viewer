//! Toolkit-independent application logic.
//!
//! Owns everything the user interface needs but that does not depend on a windowing
//! toolkit: the open-document set, view state (page, zoom, scroll), the render
//! scheduler and its tile cache, search state, and the history of navigation.
//!
//! No type from a windowing or graphics library appears in this crate's API. That is
//! what keeps `viewer-ui` replaceable — the project has already changed windowing
//! decisions once, from Qt to winit — and it is what makes the application logic
//! testable without a display.
//!
//! Implemented after Phase 5.

#![forbid(unsafe_code)]

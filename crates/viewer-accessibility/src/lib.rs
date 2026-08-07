//! ISO 32000-2 §14.7's logical structure, handed to a platform accessibility API.
//!
//! `viewer_core::Query::AccessibilityTree` has answered with §14.7's elements, §14.9's spoken
//! form of each and the quadrilaterals they cover since the hundred-and-forty-ninth session
//! (ADR 0134), and nothing consumed it. This crate is the consumer: it maps §14.8.4's forty-one
//! standard structure types onto `accesskit::Role`, builds the tree an assistive technology
//! reads, and — on Linux — puts it on AT-SPI through `accesskit_unix`.
//!
//! # Why it is a crate of its own
//!
//! One responsibility, and one dependency nothing else in this tree may take. `accesskit_unix`
//! reaches D-Bus through `zbus`, which needs an asynchronous executor — and `CLAUDE.md` allows
//! one only where something genuinely requires it. Confining it to a crate that nothing on the
//! render path, the parse path or the launch path depends on is what makes that allowance
//! narrow rather than general. ADR 0214 is the decision, with the launch measured before and
//! after.
//!
//! # The two halves
//!
//! [`role`] and [`tree`] are plain data on every platform: no D-Bus, no threads, no I/O, and
//! tested everywhere. [`Bridge`] is the platform half, and it is Linux's — what the other two
//! platforms do instead is *named* by [`Bridge::shortfall`] rather than silently absent.

#![forbid(unsafe_code)]

mod bridge;
pub mod role;
pub mod tree;

pub use bridge::{Bridge, Requested};
pub use tree::PageView;

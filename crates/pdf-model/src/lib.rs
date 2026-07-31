//! Document and page-tree model built over parsed PDF objects.
//!
//! Gives meaning to the object graph from `pdf-syntax`: the catalogue, the page tree with
//! attribute inheritance, and the content streams that describe what a page looks like.
//!
//! # What this crate decides
//!
//! Page access is lazy. Time-to-first-page is the metric a user perceives, and eagerly
//! walking a page tree of several thousand nodes to open a document is the most common
//! reason viewers feel slow to start, so nothing here requires a full-document traversal to
//! render page one.
//!
//! Content interpretation happens once, here, producing the resolved display list that
//! `pdf-render` defines. Backends therefore contain no PDF semantics — which is what allows
//! the CPU backend to validate the GPU one on byte-identical input.
//!
//! # Incomplete pages are visible
//!
//! Text and images are not yet drawn. [`content::Interpretation`] reports what it could not
//! draw rather than silently omitting it, because a page that looks plausible and is wrong
//! is the worst failure a viewer can have — and it would make the comparison harness report
//! a pass on a page missing half its content.

#![forbid(unsafe_code)]

pub mod accessibility;
pub mod action;
mod annotation;
mod appearance;
pub mod attachment;
pub mod colour;
pub mod content;
pub mod destination;
pub mod function;
pub mod icc;
pub mod image;
pub mod inline_image;
pub mod link;
pub mod mesh;
pub mod navigation;
pub mod optional_content;
pub mod outline;
pub mod page;
pub mod page_label;
pub mod requirements;
pub mod shading;
mod soft_mask;
pub mod structure;
pub mod type3;
pub mod uri;
mod variable_text;
pub mod view;
pub mod viewer_preferences;

pub use content::{Interpretation, Unsupported, interpret};
pub use page::{Page, Pages};

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
//! [`content::Interpretation`] reports what it could not draw rather than silently omitting it,
//! because a page that looks plausible and is wrong is the worst failure a viewer can have — and
//! it would make the comparison harness report a pass on a page missing half its content.
//!
//! **This paragraph opened "[t]ext and images are not yet drawn" until the two-hundred-and-
//! twenty-first session**, which was true of the sixth session and of no session since the
//! thirteenth. A crate's own front door is where a reader learns what it does, and a retired
//! claim there outlives every ledger row that says otherwise: `doc/todo/01`'s sweeps were
//! pointed at the ledger, and this is what the same regular expression finds when it is pointed
//! at the source.

#![forbid(unsafe_code)]

pub mod accessibility;
pub mod action;
mod annotation;
mod appearance;
pub mod article;
pub mod attachment;
mod bigint;
pub mod cms;
pub mod collection;
pub mod colour;
pub mod content;
pub mod der;
pub mod destination;
pub mod document_part;
pub mod dsa;
pub mod ecdsa;
pub mod eddsa;
pub mod file_spec;
pub mod form;
pub mod forms_data;
pub mod fragment;
pub mod function;
pub mod icc;
mod icon;
pub mod image;
pub mod inline_image;
pub mod link;
mod markup;
pub mod measurement;
pub mod mesh;
pub mod metadata;
pub mod named_page;
pub mod navigation;
pub mod optional_content;
pub mod outline;
pub mod page;
pub mod page_label;
pub mod pkcs1;
pub mod popup;
pub mod pss;
pub mod requirements;
pub mod restriction;
pub mod retrieval;
pub mod shading;
pub mod signature;
mod soft_mask;
pub mod structure;
pub mod tab_order;
pub mod thumbnail;
pub mod type3;
pub mod uri;
mod variable_text;
pub mod view;
pub mod viewer_preferences;
pub mod x509;
pub mod xmp;

pub use content::{Interpretation, Unsupported, interpret};
pub use page::{MediaBoxSubstitution, Page, Pages};

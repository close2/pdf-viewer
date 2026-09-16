//! PDF colour spaces and their conversions.
//!
//! Given a colour space dictionary parsed by `pdf-syntax`, this crate answers the one question
//! §8.6 asks of every space: what device colour does a set of components become? It holds the
//! CIE-based and device families ([`colour`]), the ICC profiles a document embeds ([`icc`]),
//! the §7.10 functions those and shadings evaluate ([`function`]), the §8.7.4 shadings and their
//! §8.7.4.5 mesh tessellation ([`shading`], [`mesh`]), and §10.5's transfer function
//! ([`transfer`]).
//!
//! # Why a crate of its own
//!
//! A colour reaches a raster by three routes and only one of them is a content-stream operator;
//! parsing a space where its samples are converted keeps the conversion beside the pixels it
//! feeds. Separating it from the content interpreter that calls it — `pdf_model::content` — is
//! what lets the interpreter stay a question about *marks* while this crate stays a question
//! about *colour*, with no dependency from here back up to the model. `pdf_model` re-exports
//! every module below, so a consumer still writes `pdf_model::colour::ColourSpace`.
//!
//! # What it does not decide
//!
//! Nothing here reads a content stream or a page tree: the caller resolves the object graph and
//! hands a dictionary in. The one navigation a colour space needs — following an object
//! reference to its stream — goes through the `pdf_syntax::Document` the caller already holds.

#![forbid(unsafe_code)]

pub mod colour;
pub mod function;
pub mod icc;
pub mod mesh;
pub mod shading;
pub mod transfer;

//! GPU rasteriser backend built on Vello and wgpu.
//!
//! Implements [`pdf_render::Rasterizer`] against the GPU, where continuous
//! zoom and pan, large vector artwork, high-DPI output and thumbnail grids are
//! decisively faster than on the CPU.
//!
//! # Safety posture
//!
//! Unlike the parsing and model crates, this crate permits `unsafe`: creating a
//! surface from a raw window handle requires it. That is acceptable only because no
//! untrusted document bytes reach this code — it consumes a
//! [`pdf_render::DisplayList`], which is data we produced ourselves. Every `unsafe`
//! block must carry a comment establishing that invariant, enforced by the
//! `undocumented_unsafe_blocks` lint.
//!
//! GPU drivers are large bodies of unsafe C and are themselves an attack surface, so
//! this crate is a candidate for its own sandboxed process. See
//! `crates/pdf-sandbox`.
//!
//! Implemented in Phase 5B.

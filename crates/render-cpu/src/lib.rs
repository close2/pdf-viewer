//! CPU rasteriser backend; the correctness oracle for the GPU backend.
//!
//! Implements [`pdf_render::Rasterizer`] on the CPU. Its role is twofold: it is the
//! fallback when no usable GPU is present, and — more importantly — it is the
//! reference against which `render-gpu` is validated.
//!
//! That second role is why this backend exists first. Both backends consume the same
//! [`pdf_render::DisplayList`], so any difference between their outputs is a backend
//! defect rather than a difference in how the document was interpreted. That is a far
//! tighter test than comparing against another PDF viewer, where antialiasing and
//! colour handling differ for legitimate reasons.
//!
//! Correctness therefore outranks speed here, and where the two conflict this
//! backend chooses the clearer construction. Speed is `render-gpu`'s responsibility.
//!
//! Implemented in Phase 5A. The rasteriser library choice is still open — see
//! `doc/adr/0002-cpu-rasteriser.md`.

#![forbid(unsafe_code)]

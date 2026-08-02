//! Backend-agnostic rendering interface.
//!
//! This crate defines *what* is to be drawn; it contains no rasteriser. Backends
//! (`render-cpu`, `render-gpu`) consume the types defined here and produce pixels.
//!
//! # Why a resolved display list
//!
//! A PDF content stream is a state machine: `q`/`Q` push and pop a graphics state
//! containing the current transformation matrix, clip path, colour, and more. Each
//! drawing operator acts relative to that accumulated state.
//!
//! Passing that model to a backend would force every backend to reimplement the
//! state machine — and to reimplement it *identically*, or the backends would
//! disagree. Instead, the content-stream interpreter resolves the state machine
//! once: every [`Command`] in a [`DisplayList`] carries its absolute transform and
//! an explicit clip reference. Backends therefore contain no PDF semantics at all.
//!
//! This is what makes the CPU backend usable as a correctness oracle for the GPU
//! backend: both consume byte-identical input, so any difference in their output is
//! a backend bug rather than a difference in interpretation.

#![forbid(unsafe_code)]

pub mod backend;
pub mod collapsed;
pub mod degenerate;
pub mod display_list;
pub mod geom;
pub mod paint;
pub mod shading;
pub mod soft_mask;
pub mod strips;

pub use backend::{
    BackendError, MAX_EXTENT, MAX_GROUP_DEPTH, Raster, RasterFormat, Rasterizer, TargetSpec,
    impose_on_medium,
};
pub use collapsed::{CollapsedFill, split_collapsed_fill};
pub use degenerate::{
    DegenerateStroke, ZERO_DASH, dash_mark, dashes_showing_direction, split_dash_marks,
    split_degenerate,
};
pub use display_list::{Clip, ClipId, Command, DisplayList, DisplayListError};
pub use geom::{Path, PathCommand, Point, Rect, Size, Transform};
pub use paint::{
    BlendMode, Color, FillRule, Image, LineCap, LineJoin, Paint, Stroke, thinnest_line,
};
pub use shading::{MeshRaster, Ramp, Shading, ShadingKind, Stop, Triangle};
pub use soft_mask::{SoftMask, SoftMaskId, SoftMaskKind, Transfer};
pub use strips::{
    command_extents, replay_ratio, row_costs, strip_boundaries, strip_boundaries_avoiding,
    unsplittable_rows,
};

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
pub mod blending;
pub mod closing;
pub mod collapsed;
pub mod crop;
pub mod degenerate;
pub mod display_list;
pub mod edge;
pub mod geom;
pub mod group_cost;
pub mod medium;
pub mod mitre;
pub mod outline;
pub mod paint;
pub mod program;
pub mod repeat;
pub mod shading;
pub mod soft_mask;
pub mod strips;
pub mod sub_pixel;

pub use backend::{
    BackendError, Interrupt, MAX_EXTENT, MAX_GROUP_DEPTH, Raster, RasterFormat, Rasterizer,
    TargetSpec,
};
pub use blending::{
    BlendingSpace, ColourCube, GreyCurve, resolve as resolve_blending, resolve_cube, resolve_grey,
};
pub use closing::opened_where_a_dash_ends_at_the_close;
pub use collapsed::{CollapsedFill, split_collapsed_fill};
pub use crop::cropped_rectangle;
pub use degenerate::{
    DegenerateStroke, ZERO_DASH, dash_mark, dashes_showing_direction, split_dash_marks,
    split_degenerate,
};
pub use display_list::{Clip, ClipId, Command, DisplayList, DisplayListError, GroupBlending};
pub use edge::{
    DeviceRectangles, RECTANGLES_PER_PATH, device_rectangle, device_rectangles, rectangle_coverage,
};
pub use geom::{Path, PathCommand, Point, Rect, Size, Transform};
pub use group_cost::{MAX_GROUP_BLIT_PIXELS, check_group_blit, group_blit_demand};
pub use medium::{
    Medium, SURROUND, crop_area, crop_to_page, impose_on_medium, impose_within, page_area,
};
pub use mitre::{mitre_wedges, sharpest_admitted_mitre};
pub use outline::{marked_bounds, stroked_bounds};
pub use paint::{
    BlendMode, Color, DeferredImage, FillRule, Grid, Image, ImageAtDeviceScale, ImageSource,
    LineCap, LineJoin, Paint, Reduction, Stroke, paint_space, thinnest_line,
};
pub use program::{ProgramOperator, ProgramRange, ProgramStep, ShadingProgram};
pub use repeat::{Cell, Mark, Repeats, Tiles, repeated_subpaths, without_subpaths};
pub use shading::{
    ColourGrid, ColoursAtDeviceScale, Corners, DeferredColours, MeshRaster, Patch, Radial,
    RadialRaster, Ramp, Shading, ShadingKind, ShadingRaster, Stop, Triangle, blend_parameter,
};
pub use soft_mask::{Luminance, SoftMask, SoftMaskId, SoftMaskKind, Transfer};
pub use strips::{
    command_extents, replay_ratio, row_costs, strip_boundaries, strip_boundaries_avoiding,
    unsplittable_rows,
};
pub use sub_pixel::{
    EnlargedMark, PointMark, SubPixelBand, enlarged_mark, expressible_coverage,
    marks_in_their_pixels, only_flat_subpaths, pixel_containing, point_mark, sub_pixel_bands,
    sub_pixel_caps, substitute_width,
};

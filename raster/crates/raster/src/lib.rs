//! A GPU renderer for **documents**.
//!
//! Not a general 2D vector renderer pointed at documents — the other way round. Its fast
//! paths assume that most of a page is the same few glyph outlines repeated at many
//! sub-pixel phases plus axis-aligned rectangles, and it treats general curve filling as
//! the rare case rather than the uniform one. It expresses the transparency model of
//! ISO 32000-2 clause 11 natively, because that is the part an SVG-shaped renderer cannot
//! be patched into.
//!
//! # State
//!
//! The scene's whole vocabulary is drawn — rectangles, fills and strokes over uploaded outlines,
//! one-segment lines, images, clips, masks and groups — on a device that is headless or
//! surface-attached, into the three targets of the brief's section 2.4. Each mark's coverage comes
//! from the lane [`gpu::frame::LaneCounts`] counts it in (an analytic rectangle or line, a tile
//! of the glyph atlas, a tile of the frame's sheet, an image quad), the sheet filled by the
//! processor or the device as [`gpu::startup::Coverage`] chooses; a frame that does not change is
//! replayed as a [`RetainedScene`]. The requirements are `raster/doc/RENDER_LIBRARY.md` and the
//! decisions `raster/doc/adr/`; what the viewer draws through this library is the workspace's
//! `doc/state-of-play.md`.
//!
//! # The shape of the API
//!
//! Two halves, and the split is enforced by the dependency graph rather than by review
//! (`raster/doc/adr/0001`):
//!
//! - [`scene`] — what is to be drawn. **No viewport, no resolution, no device transform, no
//!   target size**, and no device: a scene is `Send + Sync`, cheap to clone, and built on
//!   whatever thread interpreted the page. Zoom, scroll, resize and tiled output are all the
//!   same scene at a different viewport.
//! - [`gpu`] — the device, the pipelines, the atlas, and a frame that tells the truth about
//!   itself.
//!
//! ```no_run
//! use raster::{Affine, Color, Device, Options, Point, Rect, SceneBuilder, Target, Viewport};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut device = Device::headless(&Options::default())?; // background thread welcome
//!
//! let mut builder = SceneBuilder::new(); // no device needed
//! builder.rect(
//!     Rect::new(Point::new(10.0, 10.0), Point::new(90.5, 40.25)),
//!     Affine::IDENTITY,
//!     Color::new(0.0, 0.0, 0.0, 1.0),
//!     None, // no clip
//!     None, // no soft mask
//! )?;
//! let scene = builder.finish();
//!
//! let viewport = Viewport::full(200, 100, Affine::IDENTITY);
//! let frame = device.render(&scene, &viewport, Target::Readback)?;
//! for report in frame.reports() {
//!     // what could not be drawn as asked
//! }
//! let raster = frame.into_raster()?; // straight-alpha RGBA8
//! # Ok(())
//! # }
//! ```
//!
//! # Two promises worth reading before the rest
//!
//! **A frame is drawn, or it is refused.** Never a blank target under an `Ok`. Limits are
//! discoverable before the frame, a failure is an `Err` that names what overflowed, and
//! anything drawn differently from what the scene asked is a `Report` — never a silent
//! approximation.
//!
//! **We do not colour-manage, load fonts, shape text, lay out anything, or parse any
//! document format.** Colours arrive as device RGB and glyphs arrive as positioned outlines,
//! because the caller has already decided those questions and deciding them twice is how two
//! renderers disagree. The full list is section 9 of the brief.

#![forbid(unsafe_code)]

/// What is to be drawn: the device-independent scene model.
pub use raster_scene as scene;

/// The device: pixels, and the measurements that make a frame accountable.
pub use raster_gpu as gpu;

pub use raster_scene::{
    Affine, BlendMode, ClipDef, ClipId, Color, Command, Compose, Cost, FillRule, GroupSpec,
    ImageId, ImageSpec, LineCap, LineJoin, MAX_COORDINATE, MAX_GROUP_DEPTH, MaskId, MeshId,
    MeshSpec, OutlineId, Paint, Point, RampId, Rect, ResourceId, Scene, SceneBuilder, SceneError,
    Segment, Size, Stop, Stroke,
};

pub use raster_gpu::{
    Counters, DEFAULT_MAX_FRAME_BYTES, DEFAULT_MAX_RESOURCE_BYTES, Device, DeviceError,
    EncodeSource, ForeignPresenter, Frame, Layer, LayerProblem, Limits, Options, PipelineProblem,
    PresentCost, Presenter, Raster, RenderError, Report, ReportKind, ResourceProblem,
    RetainedScene, StartupTimings, SurfaceProblem, Target, TimingProvenance, Timings, Viewport,
    WarmUp, create_instance, create_instance_with,
};

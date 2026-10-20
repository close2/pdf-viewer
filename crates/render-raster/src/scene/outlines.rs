//! Path outlines as the device holds them: uploaded once by the source path's identity, or
//! transiently for a clip, a dash or a stroke this crate expands itself.
//!
//! The one caching rule is the root's: a stroke this crate expands is a *stable* resource, keyed by
//! [`crate::cache::StrokeKey`], and never a fresh transient (ADR 0402).

use std::sync::Arc;

use pdf_render::Path;

use crate::QuorraRasterError;

use super::{Encoder, segments};

impl Encoder<'_> {
    /// One call across raster's boundary, with a clock on it.
    ///
    /// **Every `upload_*` in this file goes through here, and that is what divides the `scene`
    /// phase in two.** A cold frame of a large drawing spends most of `scene` handing resources
    /// over — 58 029 of them on the document `doc/todo/44` is about — and until ADR 0423 the
    /// trace could say only that the display list took a long time to become a scene
    /// (`doc/todo/45` §5). What it can say now is which side of the boundary the time is on: this
    /// crate's walk of the display list, or raster's own upload. They have different owners, so
    /// the division is the difference between an item to take and an item to report upstream.
    ///
    /// The argument the resource is built from is deliberately *not* inside the timed span — a
    /// caller binds it to a local first — because expanding a stroke, averaging an image or
    /// converting a path's segments is this crate's work and would otherwise be charged to
    /// raster's.
    pub(super) fn handing_over<T>(
        &mut self,
        upload: impl FnOnce(&mut raster_gpu::Device) -> T,
    ) -> T {
        let began = std::time::Instant::now();
        let handed = upload(self.device);
        self.caches.hand_over(began.elapsed());
        handed
    }

    /// One outline across the boundary, timed and **counted in segments**.
    ///
    /// The three outline call sites go through here rather than through
    /// [`Self::handing_over`] directly, because an outline is the one resource whose upload
    /// costs by the segment: see [`crate::cache::ResourceCaches::segments`] for why the
    /// duration is worth nothing without that denominator.
    fn upload_outline(
        &mut self,
        outline: &[raster_scene::Segment],
    ) -> Result<raster_scene::OutlineId, raster_gpu::DeviceError> {
        self.caches.count_segments(outline.len());
        self.handing_over(|device| device.upload_outline(outline))
    }

    /// The uploaded outline for a path, keyed by the `Arc`'s identity
    /// (`RENDER_LIBRARY.md` section 2.2: one glyph outline, thousands of fills, one
    /// upload). Pinning and eviction are [`crate::cache::ResourceCaches`]' business.
    pub(crate) fn outline(
        &mut self,
        path: &Arc<Path>,
    ) -> Result<raster_scene::OutlineId, QuorraRasterError> {
        if let Some(id) = self.caches.outline(path) {
            return Ok(id);
        }
        let outline = segments(path);
        let id = self.upload_outline(&outline)?;
        self.caches.store_outline(path, id);
        Ok(id)
    }

    /// The outline expanded from `path` under `key`, uploading it only if this device does not
    /// already hold one — and calling `expand` only then, which is the larger half of what a hit
    /// saves (see [`crate::cache::StrokeKey`]).
    ///
    /// `expand` is a closure rather than a computed argument for exactly that reason: a caller
    /// that produced the geometry before asking would run `kurbo::stroke` over every stroke on
    /// every frame and save only the upload.
    pub(crate) fn expanded_stroke(
        &mut self,
        path: &Arc<Path>,
        key: crate::cache::StrokeKey,
        expand: impl FnOnce() -> Path,
    ) -> Result<raster_scene::OutlineId, QuorraRasterError> {
        if let Some(id) = self.caches.stroke(key) {
            return Ok(id);
        }
        let outline = segments(&expand());
        let id = self.upload_outline(&outline)?;
        self.caches.store_stroke(path, key, id);
        Ok(id)
    }

    /// A per-frame outline for geometry this frame computed (dashed or degenerate
    /// strokes), released after the frame.
    pub(crate) fn transient_outline(
        &mut self,
        path: &Path,
    ) -> Result<raster_scene::OutlineId, QuorraRasterError> {
        let outline = segments(path);
        let id = self.upload_outline(&outline)?;
        self.transient.push(id.into());
        Ok(id)
    }
}

//! What the page area paints: the viewer's frames, and for a page with no frame of its own the
//! stand-in `viewer_host::stand_in` places — the rules `viewer-gtk` paints by too (ADRs 1843,
//! 1865, 1866; filed here by ADR 1908).

use viewer_core::{Answer, Query};

use crate::bridge::ffi::{self, QtFrame};
use crate::page;
use std::pin::Pin;

use super::Host;

/// A refresh-local descriptor; pixel ownership remains in the host.
pub(super) struct QtFrameView<'a> {
    pub(super) frame: QtFrame,
    pub(super) pixels: &'a [u8],
}

impl Host {
    /// Copies one placement snapshot into a caller-local, non-reentrant image sink.
    pub(crate) fn collect_frames(&self, mut sink: Pin<&mut ffi::QtFrameSink>, probing: bool) {
        let began = probing.then(std::time::Instant::now);
        let views = self.frame_views();
        if let Some(began) = began {
            eprintln!(
                "zoom-probe qt snapshot retrieval_ns={} frames={}",
                began.elapsed().as_nanos(),
                views.len()
            );
        }
        for view in views {
            ffi::append_frame(sink.as_mut(), view.frame, view.pixels);
        }
    }

    /// The pages this window paints and the rectangle each fills: the viewer's frames, with a page
    /// still being drawn put where it belongs as it stands — `viewer_host::drawing::over_frames`
    /// (ADR 1825) — a frame from before a zoom scaled to where the page is now —
    /// `viewer_host::stand_in::placed` (ADR 1843) — and a page with no frame shown from its
    /// retained picture — `viewer_host::stand_in::Retained` (ADR 1866), the rules `viewer-gtk`
    /// paints by too.
    pub(super) fn frames(&self) -> Vec<viewer_host::stand_in::Placed<'_>> {
        #[cfg(test)]
        tests::placement_query();
        let geometry = |page| match self.state.viewer.query(Query::PageGeometry(page)) {
            Answer::Geometry(geometry) => Some(geometry),
            _ => None,
        };
        let focused = self.state.documents.focused();
        // A viewer with no document yet — the window's own, before the opening thread's is taken
        // in — answers no frames at all, which is the moment a look is shown.
        let frames = match self.state.viewer.query(Query::Frame) {
            Answer::Frame(frames) => frames,
            _ => Vec::new(),
        };
        // Page one as it stood at its last look while it was read, until the page has pixels of
        // its own (ADR 1867): the page's own picture at the target its first frame is drawn at,
        // placed where that frame will be — and only while that frame is still coming, so a page
        // whose read failed is not left showing part of itself (trap 5).
        if frames.is_empty()
            && !self.state.oversized.shows(focused)
            && self.state.drawing.partial().is_none()
            && (self.state.anticipated.is_some() || self.state.drawing_interval().is_some())
            && let Some(look) = self.state.read_look.as_ref()
        {
            return vec![look.placed()];
        }
        // And a page with no frame at all shown from its retained picture (ADR 1866), with a page
        // zoomed past its window shown over it from what that drew (ADRs 1892, 1930).
        self.state.oversized.place(
            self.state.retained.fill(
                viewer_host::stand_in::placed(
                    viewer_host::drawing::over_frames(
                        frames,
                        self.state.drawing.partial(),
                        focused,
                        geometry,
                    ),
                    geometry,
                ),
                focused,
                geometry,
            ),
            self.state.drawing.partial(),
            focused,
            geometry,
            self.state.viewport,
        )
    }

    /// One immutable placement snapshot with pixel slices borrowing this host.
    ///
    /// The caller copies the pixels before mutating the host or re-entering the event loop.
    pub(super) fn frame_views(&self) -> Vec<QtFrameView<'_>> {
        if let Some(raster) = self.playing.as_ref() {
            return vec![describe(raster, (0.0, 0.0), (raster.width, raster.height))];
        }
        self.frames()
            .into_iter()
            .map(|placed| describe(placed.frame.raster, placed.shown_at(), placed.extent))
            .collect()
    }
}

/// A descriptor borrows only raster storage, never the temporary placement vector.
fn describe(
    raster: &pdf_render::Raster,
    origin: (f32, f32),
    extent: (u32, u32),
) -> QtFrameView<'_> {
    match page::describe(raster) {
        Ok((width, height)) => QtFrameView {
            frame: QtFrame {
                present: true,
                width,
                height,
                origin_x: origin.0,
                origin_y: origin.1,
                shown_width: extent.0,
                shown_height: extent.1,
            },
            pixels: raster.data.as_slice(),
        },
        Err(error) => {
            // A malformed raster is refused by name, before C++ can read its bytes.
            eprintln!("note: {error}");
            QtFrameView {
                frame: no_frame(),
                pixels: &[],
            }
        }
    }
}

/// No frame at all, which is what a viewer with nothing open answers.
fn no_frame() -> QtFrame {
    QtFrame {
        present: false,
        width: 0,
        height: 0,
        origin_x: 0.0,
        origin_y: 0.0,
        shown_width: 0,
        shown_height: 0,
    }
}

#[cfg(test)]
#[path = "stand_in_tests.rs"]
mod tests;

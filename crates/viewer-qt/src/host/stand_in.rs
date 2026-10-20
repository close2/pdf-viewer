//! What the page area paints: the viewer's frames, and for a page with no frame of its own the
//! stand-in `viewer_host::stand_in` places — the rules `viewer-gtk` paints by too (ADRs 1843,
//! 1865, 1866; filed here by ADR 1908).

use viewer_core::{Answer, Query};

use crate::bridge::ffi::QtFrame;
use crate::page;

use super::Host;

impl Host {
    /// How many pages Table 29's arrangement is showing pixels for.
    ///
    /// One under `SinglePage`; more under a column or a spread, and the window paints each of them
    /// where [`Self::frame`] says.
    pub(crate) fn frame_count(&self) -> usize {
        // §12.4.4.1: while one of Table 164's effects is in flight the window shows the effect and
        // not the page — one picture, because a frame is already two pages placed where they
        // belong. Answered here rather than pushed across the bridge, so the C++ side draws a
        // transition with the code it already draws a page with.
        if self.playing.is_some() {
            return 1;
        }
        self.frames().len()
    }

    /// The pages this window paints and the rectangle each fills: the viewer's frames, with a page
    /// still being drawn put where it belongs as it stands — `viewer_host::drawing::over_frames`
    /// (ADR 1825) — a frame from before a zoom scaled to where the page is now —
    /// `viewer_host::stand_in::placed` (ADR 1843) — and a page with no frame shown from its
    /// retained picture — `viewer_host::stand_in::Retained` (ADR 1866), the rules `viewer-gtk`
    /// paints by too.
    pub(super) fn frames(&self) -> Vec<viewer_host::stand_in::Placed<'_>> {
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
        )
    }

    /// Where one page's pixels belong and how big they are.
    pub(crate) fn frame(&self, index: usize) -> QtFrame {
        if let Some(raster) = self.playing.as_ref() {
            return match page::describe(raster) {
                // A transition frame is already in the viewport's own pixels, so its origin is
                // the viewport's own.
                Ok((width, height)) => QtFrame {
                    present: index == 0,
                    width,
                    height,
                    origin_x: 0.0,
                    origin_y: 0.0,
                    shown_width: width,
                    shown_height: height,
                },
                Err(error) => {
                    eprintln!("note: {error}");
                    no_frame()
                }
            };
        }
        let frames = self.frames();
        let Some(placed) = frames.get(index) else {
            return no_frame();
        };
        match page::describe(placed.frame.raster) {
            Ok((width, height)) => QtFrame {
                present: true,
                width,
                height,
                origin_x: placed.shown_at().0,
                origin_y: placed.shown_at().1,
                shown_width: placed.extent.0,
                shown_height: placed.extent.1,
            },
            Err(error) => {
                // Trap 5: a host that quietly showed the previous page would be telling a
                // person something false about this one.
                eprintln!("note: {error}");
                no_frame()
            }
        }
    }

    /// The pixels themselves, borrowed rather than copied.
    ///
    /// Tier 1 is priced at "one copy per frame" and this is what keeps it to one: the `QImage`
    /// the C++ side builds copies out of this slice, and nothing on this side copies into it.
    pub(crate) fn frame_pixels(&self, index: usize) -> &[u8] {
        if let Some(raster) = self.playing.as_ref() {
            return if index == 0 {
                raster.data.as_slice()
            } else {
                &[]
            };
        }
        self.frames()
            .get(index)
            .map_or(&[][..], |placed| placed.frame.raster.data.as_slice())
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

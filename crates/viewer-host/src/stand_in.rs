//! The rectangle each page's pixels are shown in, which after a zoom is not the pixels' own size.
//!
//! A zoom changes where the arrangement places a page and how big it is at once, and the viewer
//! answers both for the new magnification the moment the command is taken
//! (`viewer_core::Query::PageGeometry`), while the pixels it holds for the page are the last frame
//! drawn, at the old magnification, until the new one lands. A window that put those pixels at the
//! new origin at their own size showed the page at the old size anchored at the new corner — the
//! owner's "incorrect frames during zooming" in both toolkit windows. [`placed`] says where the old
//! pixels belong instead: **scaled to the extent the page now occupies, at the origin it now has**,
//! which is the picture `quorra` stands in with until its frame lands (`stale.rs`), made by the
//! toolkit's own scaled draw rather than by a resample of ours (ADR 1843).
//!
//! It is a wrong picture shown on purpose — the old pixels enlarged are blurred, and what the new
//! frame adds is absent — so it is held to `doc/todo/37`'s rules as far as a toolkit window can
//! hold them: the frame that lands replaces it (rule 1, `RenderReady` sets the pixels and the
//! extent agrees again), a window says so on its `frames` topic (rule 3, [`Placed::stands_in`]),
//! and the scale is drawn by the toolkit beside the render on another thread, clipped to the
//! window (rule 4).

use viewer_core::{FrameView, PageGeometry};

/// One page's pixels and the rectangle a window shows them in, in device pixels of the viewport.
#[derive(Debug)]
pub struct Placed<'a> {
    /// What the viewer, or `drawing::over_frames`, handed over for the page.
    pub frame: FrameView<'a>,
    /// The width and height the arrangement gives the page now, which the pixels are drawn to fill.
    pub extent: (u32, u32),
}

impl Placed<'_> {
    /// Whether the pixels are shown at a size other than their own: the page's last frame
    /// standing in, scaled, for the one being drawn at the new magnification.
    #[must_use]
    pub fn stands_in(&self) -> bool {
        self.extent != (self.frame.raster.width, self.frame.raster.height)
    }
}

/// The window's sentence for a stand-in on its `frames` topic, `doc/todo/37`'s rule 3: which page,
/// at what size, scaled from what — so that a photograph of a blurred page is read as the stand-in
/// it is, and a frame line with no such sentence after a zoom says the frame landed first.
impl std::fmt::Display for Placed<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "page {} stands in at {}x{}, its {}x{} frame scaled until the frame for this size lands",
            self.frame.page.saturating_add(1),
            self.extent.0,
            self.extent.1,
            self.frame.raster.width,
            self.frame.raster.height
        )
    }
}

/// Where each frame is shown: at the extent `geometry` says the page occupies, where the pixels
/// are the same shape at another size, and at their own size otherwise.
///
/// `geometry` is `viewer_core::Query::PageGeometry`'s answer, which this crate holds no viewer to
/// ask; a page it does not answer for keeps its pixels' own size.
///
/// **Only a uniform scale stands in.** A zoom multiplies both sides of a page by one factor, and
/// each side is rounded up to a whole pixel (`viewer_core`'s `raster_extent`), so the old pixels
/// and the new extent agree in shape to within that rounding: `old_h × new_w` and
/// `new_h × old_w` then differ by less than the sum of the four sides. A change of shape — §7.7.3.3's
/// `/Rotate` turned by the view, a quarter turn of anything but a square — is not a zoom, and
/// stretching the old pixels to it would show the page distorted; those keep their own size, as
/// every frame did before this module, until their frame lands.
#[must_use]
pub fn placed(
    frames: Vec<FrameView<'_>>,
    mut geometry: impl FnMut(usize) -> Option<PageGeometry>,
) -> Vec<Placed<'_>> {
    frames
        .into_iter()
        .map(|frame| {
            let own = (frame.raster.width, frame.raster.height);
            let extent = geometry(frame.page)
                .map(|placed| (placed.width, placed.height))
                .filter(|&now| now == own || same_shape(own, now))
                .unwrap_or(own);
            Placed { frame, extent }
        })
        .collect()
}

/// Whether `now` is `was` scaled by one factor on both axes, to within each side's rounding up.
fn same_shape(was: (u32, u32), now: (u32, u32)) -> bool {
    if was.0 == 0 || was.1 == 0 || now.0 == 0 || now.1 == 0 {
        return false;
    }
    // Two `u32` multiplied and four added are inside a `u64`, so neither saturation is reachable.
    let crossed = u64::from(was.1).saturating_mul(u64::from(now.0));
    let other = u64::from(now.1).saturating_mul(u64::from(was.0));
    let slack = [was.0, was.1, now.0, now.1]
        .into_iter()
        .map(u64::from)
        .fold(0_u64, u64::saturating_add);
    crossed.abs_diff(other) <= slack
}

#[cfg(test)]
mod tests {
    use pdf_render::{Raster, RasterFormat, Size};
    use viewer_core::{FrameView, PageGeometry};

    use super::placed;

    fn raster(width: u32, height: u32) -> Raster {
        Raster {
            width,
            height,
            format: RasterFormat::Rgba8,
            // `placed` reads a raster's extent and never its bytes.
            data: Vec::new(),
        }
    }

    fn geometry(width: u32, height: u32) -> PageGeometry {
        PageGeometry {
            page: Size {
                width: 612.0,
                height: 792.0,
            },
            scale: 1.0,
            width,
            height,
            origin: (10.0, 20.0),
        }
    }

    /// The owner's case, at the sizes the drive photographed: `quorra-qt` held 619 × 802 after a
    /// notch in had placed the page at 774 × 1002 (1.25 times both, each side rounded up).
    #[test]
    fn a_frame_from_before_a_zoom_fills_the_extent_the_page_has_now() {
        let old = raster(619, 802);
        let frames = vec![FrameView {
            page: 0,
            raster: &old,
            origin: (10.0, 20.0),
        }];
        let shown = placed(frames, |_| Some(geometry(774, 1002)));
        assert_eq!(shown[0].extent, (774, 1002));
        assert!(shown[0].stands_in());
        let out = vec![FrameView {
            page: 0,
            raster: &old,
            origin: (10.0, 20.0),
        }];
        let shown = placed(out, |_| Some(geometry(496, 642)));
        assert_eq!(shown[0].extent, (496, 642), "a notch out stands in too");
    }

    #[test]
    fn a_frame_at_its_own_size_is_shown_at_it() {
        let held = raster(619, 802);
        let frames = vec![FrameView {
            page: 0,
            raster: &held,
            origin: (0.0, 0.0),
        }];
        let shown = placed(frames, |_| Some(geometry(619, 802)));
        assert_eq!(shown[0].extent, (619, 802));
        assert!(!shown[0].stands_in());
    }

    /// A quarter turn is not a zoom: 619 × 802 stretched to 802 × 619 would be the page distorted.
    #[test]
    fn a_change_of_shape_is_not_stretched_into() {
        let held = raster(619, 802);
        let frames = vec![FrameView {
            page: 0,
            raster: &held,
            origin: (0.0, 0.0),
        }];
        let shown = placed(frames, |_| Some(geometry(802, 619)));
        assert_eq!(shown[0].extent, (619, 802));
        let frames = vec![FrameView {
            page: 0,
            raster: &held,
            origin: (0.0, 0.0),
        }];
        assert_eq!(placed(frames, |_| None)[0].extent, (619, 802));
    }

    /// The rounding the slack is there for, at its worst over a whole gesture: every step of
    /// `viewer_core`'s 1.25 from a twentieth of the page to sixteen times it, each side rounded up,
    /// is the same shape as every other.
    #[test]
    fn every_zoom_step_of_a_page_is_the_same_shape_as_every_other() {
        let side = |points: f64, magnification: f64| {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a positive extent of at most ten thousand pixels"
            )]
            let pixels = (points * magnification).ceil() as u32;
            pixels
        };
        let steps: Vec<(u32, u32)> = (-13..=12)
            .map(|step| {
                let magnification = 1.25_f64.powi(step);
                (side(612.0, magnification), side(792.0, magnification))
            })
            .collect();
        for &was in &steps {
            for &now in &steps {
                assert!(super::same_shape(was, now), "{was:?} against {now:?}");
            }
        }
        assert!(!super::same_shape((612, 792), (792, 612)));
        assert!(!super::same_shape((0, 792), (612, 792)));
    }
}

//! How large a page is drawn: the zoom a mode or a step resolves to, and where the scroll goes.
//!
//! The arithmetic that agrees with `TargetSpec::for_page` to the pixel, so that the chrome and the
//! frame it is drawn over never disagree by one.

use pdf_render::{Point, Rect, Size};

use super::{Open, ZOOM_RANGE, ZOOM_STEP};
use crate::command::Zoom;
use crate::viewer::px;

impl Open {
    /// Device pixels per user space unit for the page showing, given the viewport.
    ///
    /// `None` for a page that cannot be read or has no extent — a zero-sized crop box, which
    /// Table 30 does not forbid and a fuzzer produces immediately.
    pub(crate) fn magnification(&self, viewport: (u32, u32), scale: f32) -> Option<f32> {
        let size = self.page_size(self.page_index)?;
        if size.width <= 0.0 || size.height <= 0.0 || scale <= 0.0 {
            return None;
        }
        // A fit is worked out in *device* pixels and never divided back into logical ones: the
        // round trip through `/ scale` and `* scale` is not the identity in `f32`, and this is
        // the one place where a pixel of error becomes a scrollbar.
        let magnification = match self.zoom {
            Zoom::FitPage => fitted(viewport.0, size.width).min(fitted(viewport.1, size.height)),
            Zoom::FitWidth => fitted(viewport.0, size.width),
            Zoom::FitHeight => fitted(viewport.1, size.height),
            Zoom::Scale(zoom) => zoom.clamp(ZOOM_RANGE.0, ZOOM_RANGE.1) * scale,
            // Neither reaches here: `Viewer` resolves a step into the scale it lands on
            // before storing it, because a chain of steps has to compose and a mode cannot.
            Zoom::In | Zoom::Out => return None,
        };
        (magnification.is_finite() && magnification > 0.0).then_some(magnification)
    }

    /// The magnification one step in or out from where this is now.
    pub(crate) fn stepped(current: f32, direction: Zoom) -> f32 {
        let stepped = match direction {
            Zoom::In => current * ZOOM_STEP,
            Zoom::Out => current / ZOOM_STEP,
            Zoom::FitPage | Zoom::FitWidth | Zoom::FitHeight | Zoom::Scale(_) => current,
        };
        stepped.clamp(ZOOM_RANGE.0, ZOOM_RANGE.1)
    }

    /// Where the current page's raster sits in the viewport, in device pixels.
    ///
    /// **The arrangement decides this since Table 29's layouts were obeyed**, and it is one call
    /// rather than an arithmetic here so that a page standing beside or below another is placed
    /// by the same code that places the current one — ADR 0118's rule about a second opinion.
    /// Under `SinglePage` the answer is what it always was: centred where the page is smaller
    /// than the viewport and scrolled where it is larger.
    pub(crate) fn origin(
        &self,
        viewport: (u32, u32),
        scale: f32,
        magnification: f32,
    ) -> (f32, f32) {
        crate::layout::origin_of(self, self.page_index, viewport, scale, magnification)
            .unwrap_or((0.0, 0.0))
    }

    /// Scrolls so that a point of the viewport keeps the point of the page it was over.
    ///
    /// `before` and `after` are magnifications and `at` is a viewport point in device pixels.
    /// The page point under `at` is `at - origin` in the old raster's coordinates; it is that
    /// same fraction of the new raster, so scaling it by the ratio and putting it back under
    /// `at` is the whole of it. **Through `origin` rather than through the scroll**, because a
    /// page smaller than the viewport is *centred* and its scroll is zero — the arithmetic that
    /// reads the scroll alone is right only while there is something to scroll, and a wheel is
    /// pointed at pages of both kinds.
    ///
    /// The scroll it produces may exceed what the new raster permits; `Viewer::settle` clamps it
    /// against the target it is about to ask for, which is where the new raster's size is known.
    pub(crate) fn hold(
        &mut self,
        viewport: (u32, u32),
        scale: f32,
        before: f32,
        after: f32,
        at: Option<(f32, f32)>,
    ) {
        let origin = self.origin(viewport, scale, before);
        let at = at.unwrap_or((px(viewport.0) / 2.0, px(viewport.1) / 2.0));
        let ratio = after / before;
        let hold = |at: f32, origin: f32| ((at - origin) * ratio - at).max(0.0);
        self.scroll = (hold(at.0, origin.0), hold(at.1, origin.1));
    }
}

/// The raster a page of this size occupies at this magnification.
///
/// The same rounding `TargetSpec::for_page` does — up, so the raster contains the page — stated
/// here because `apply_view` clamps a scroll before there is a target to ask.
///
/// Shared with [`crate::viewer`] rather than copied into it: every caller that places page
/// coordinates on the screen has to agree with the drawn frame to the pixel, and two copies of
/// this rounding are two chances to disagree (ADR 0118).
///
/// **The product is taken in `f64`, as [`pdf_render::TargetSpec::for_page`] takes it.** Agreeing on the
/// rounding is not enough if the multiplication that precedes it differs: `595.276 × 40` is a
/// number `f32` cannot hold, and one of the two arithmetics can land above an integer while the
/// other lands below it, after which `ceil` puts the chrome one pixel off the frame it is drawn
/// over. The inputs are `f32` and the caller's; the *product* is where the precision has to
/// match, and this is the only place that can make it.
pub(crate) fn raster_extent(size: Size, magnification: f32) -> (u32, u32) {
    let extent = |value: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a float-to-integer cast saturates in Rust, and an extent that saturated \
                      is refused by `pixel_extent` wherever a target is actually built"
        )]
        let pixels = (f64::from(value) * f64::from(magnification))
            .ceil()
            .max(1.0) as u32;
        pixels
    };
    (extent(size.width), extent(size.height))
}

/// The box the `/FitB` forms magnify to fit: what the page's contents cover, cut to the page.
///
/// §12.3.2.2 states the bounding box and then bounds it, in the same paragraph:
///
/// > The page's bounding box is the smallest rectangle enclosing all of its contents. (If any
/// > side of the bounding box lies outside the page's crop box, the corresponding side of the
/// > crop box shall be used instead; see 14.11.2, "Page boundaries" for further discussion of
/// > the crop box.)
///
/// The cut is the part a display list makes necessary. `pdf_model::interpret` puts the box the
/// page is displayed in *at the origin* rather than clipping the content to it, so the list
/// carries every mark the stream made, including the ones off the edge — and `content_bounds`
/// unions all of them. Without this, a `/FitB` on a page whose producer left ink outside the
/// crop box would magnify to fit ink this viewer never draws. In the list's own space the
/// displayed box is `0,0 … width,height`, so the clause's "corresponding side of the crop box"
/// is that rectangle's side.
///
/// `None` where nothing survives the cut, which is the same answer as a page covering nothing:
/// the three forms fall back to the page box, because the alternative is a magnification of
/// infinity.
pub(super) fn content_box(bounds: Option<Rect>, size: Size) -> Option<Rect> {
    bounds
        .map(|box_| Rect {
            min: Point::new(box_.min.x.max(0.0), box_.min.y.max(0.0)),
            max: Point::new(box_.max.x.min(size.width), box_.max.y.min(size.height)),
        })
        .filter(|box_| box_.max.x > box_.min.x && box_.max.y > box_.min.y)
}

/// How many times a fit steps down before it settles for the extra pixel.
///
/// One step has always been enough — the loop's condition is what decides, and this is the
/// bound that keeps a pathological extent from spinning rather than a number anything derives.
const FIT_STEPS: u32 = 4;

/// The largest magnification at which a page extent still fits `viewport` device pixels.
///
/// Not simply the ratio. [`pdf_render::TargetSpec::for_page`] rounds a raster **up** so that it contains
/// the page, and the nearest `f32` to the exact ratio is above it as often as below — so the
/// plain division produces a raster one pixel larger than the viewport it was computed to fit
/// about half the time, which is a page fitted to the window with a scrollbar down the side.
/// Stepping to the next smaller representable scale until the rounding lands is exact, costs
/// nothing measurable once per render, and needs no epsilon anybody had to choose.
pub(super) fn fitted(viewport: u32, extent: f32) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a viewport dimension divided by a page dimension is a small ratio, and the \
                  loop below is what makes its rounding acceptable"
    )]
    let mut scale = (f64::from(viewport) / f64::from(extent)) as f32;
    for _ in 0..FIT_STEPS {
        if (f64::from(extent) * f64::from(scale)).ceil() <= f64::from(viewport) {
            break;
        }
        scale = f32::from_bits(scale.to_bits().saturating_sub(1));
    }
    scale
}

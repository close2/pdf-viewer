//! §12.3.2.1's destination views: where the window sits and how large once a destination is reached.
//!
//! Table 149's eight forms, each resolved to a magnification and a point of the page put at the
//! window's top-left corner, applied on the first settled frame.

use pdf_render::Rect;

use super::Open;
use super::magnification::{content_box, fitted};
use crate::command::Zoom;

impl Open {
    /// Applies §12.3.2.1's other two items — where the window sits, and how large.
    ///
    /// Table 149's eight forms, and every one of them is two decisions: a **magnification**,
    /// which becomes a [`Zoom`], and a **point of the page put at the window's top-left corner**,
    /// which becomes a scroll. The clause states them in default user space and this works in
    /// device pixels of a raster, so each coordinate goes through the page's own transform
    /// (§7.7.3.3's rotation, the crop box's origin, `/UserUnit`) and then through the
    /// magnification, with the y axis flipped once on the way — a raster's rows count down from
    /// the top and PDF's coordinates count up from the bottom.
    ///
    /// §12.3.2.2, of Table 149's own null:
    ///
    /// > A null value for any of the parameters left , top , or zoom specifies that the current
    /// > value of that parameter shall be retained unchanged.
    ///
    /// So an absent coordinate leaves the scroll where it is and an absent zoom leaves the
    /// magnification, which is why each is applied separately rather than as a whole position.
    ///
    /// `bounds` is what the page's contents cover, in the display list's own space, and is only
    /// consulted by the three `/FitB` forms — "the smallest rectangle enclosing all of its
    /// contents", which no page dictionary states and only a display list can answer. **It is
    /// bounded by the page**, which is the same clause's parenthesis:
    ///
    /// > (If any side of the bounding box lies outside the page's crop box, the corresponding
    /// > side of the crop box shall be used instead; see 14.11.2, "Page boundaries" for further
    /// > discussion of the crop box.)
    ///
    /// Returns whether anything changed, which is what decides a redraw.
    pub(crate) fn apply_view(
        &mut self,
        view: pdf_model::destination::View,
        viewport: (u32, u32),
        scale: f32,
        bounds: Option<Rect>,
    ) -> bool {
        use pdf_model::destination::View;

        let Some(size) = self.page_size(self.page_index) else {
            return false;
        };
        let before = (self.zoom, self.scroll);
        // The box each form fits: the page for five of them, its contents for the `/FitB`
        // family. A `/FitB` on a page whose contents cover nothing falls back to the page,
        // because the alternative is a magnification of infinity.
        let content = content_box(bounds, size);
        let (fit_width, fit_height) = match view {
            View::FitB | View::FitBH { .. } | View::FitBV { .. } => content
                .map_or((size.width, size.height), |box_| {
                    (box_.max.x - box_.min.x, box_.max.y - box_.min.y)
                }),
            _ => (size.width, size.height),
        };

        match view {
            View::Xyz { zoom, .. } => {
                if let Some(zoom) = zoom.filter(|zoom| zoom.is_finite() && *zoom > 0.0) {
                    self.zoom = Zoom::Scale(zoom);
                }
            }
            View::Fit | View::FitB => {
                self.zoom = Zoom::Scale(
                    fitted(viewport.0, fit_width).min(fitted(viewport.1, fit_height)) / scale,
                );
            }
            View::FitH { .. } | View::FitBH { .. } => {
                self.zoom = Zoom::Scale(fitted(viewport.0, fit_width) / scale);
            }
            View::FitV { .. } | View::FitBV { .. } => {
                self.zoom = Zoom::Scale(fitted(viewport.1, fit_height) / scale);
            }
            View::FitR { rect } => {
                let (width, height) = ((rect[2] - rect[0]).abs(), (rect[3] - rect[1]).abs());
                // Table 149 says "magnified just enough to fit *the rectangle*", so a rectangle
                // with no extent in one direction states no magnification in it and the other
                // one decides alone. Neither: the file has stated a point, and the zoom stands.
                self.zoom = match (width > 0.0, height > 0.0) {
                    (true, true) => Zoom::Scale(
                        fitted(viewport.0, width).min(fitted(viewport.1, height)) / scale,
                    ),
                    (true, false) => Zoom::Scale(fitted(viewport.0, width) / scale),
                    (false, true) => Zoom::Scale(fitted(viewport.1, height) / scale),
                    (false, false) => self.zoom,
                };
            }
        }

        // The magnification the new zoom lands on, which is what turns a user-space coordinate
        // into a device one. Read back rather than remembered: `Zoom::Scale` is clamped.
        let Some(magnification) = self.magnification(viewport, scale) else {
            self.zoom = before.0;
            return false;
        };
        let corner = match view {
            View::Xyz { left, top, .. } => (left, top),
            View::FitH { top } | View::FitBH { top } => (None, top),
            View::FitV { left } | View::FitBV { left } => (left, None),
            // Table 149 puts the rectangle's own corner at the window's.
            View::FitR { rect } => (Some(rect[0].min(rect[2])), Some(rect[1].max(rect[3]))),
            // "Display the page ... with its contents magnified just enough to fit the entire
            // page within the window" — the whole page is in view, so there is nothing to
            // scroll to and the top-left corner is the answer in both directions.
            View::Fit => (Some(0.0), Some(size.height)),
            View::FitB => content.map_or((Some(0.0), Some(size.height)), |box_| {
                (Some(box_.min.x), Some(box_.max.y))
            }),
        };
        self.scroll_to(corner, view, magnification, size.height);
        crate::layout::settle_scroll(self, viewport, scale, magnification);
        (self.zoom, self.scroll) != before
    }

    /// Puts a point of the page at the window's top-left corner.
    ///
    /// The `/FitB` forms already carry display-list coordinates, because that is the space a
    /// content bounding box is measured in; every other form states default user space and goes
    /// through the page's transform first.
    fn scroll_to(
        &mut self,
        corner: (Option<f32>, Option<f32>),
        view: pdf_model::destination::View,
        magnification: f32,
        page_height: f32,
    ) {
        use pdf_model::destination::View;

        let already_page_space = matches!(
            view,
            View::Fit | View::FitB | View::FitBH { .. } | View::FitBV { .. }
        );
        let (x, y) = match (corner.0, corner.1) {
            (None, None) => return,
            (left, top) => {
                let point = (left.unwrap_or(0.0), top.unwrap_or(page_height));
                if already_page_space {
                    point
                } else {
                    match self.shown_page() {
                        Some(page) => pdf_model::content::page_space_at(page, point.0, point.1),
                        None => point,
                    }
                }
            }
        };
        if corner.0.is_some() {
            self.scroll.0 = (x * magnification).max(0.0);
        }
        if corner.1.is_some() {
            // The display list's y counts up from the bottom of the page and a raster's rows
            // count down from the top, so the distance scrolled is measured from the *top*.
            self.scroll.1 = ((page_height - y) * magnification).max(0.0);
        }
    }
}

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
    /// So an absent zoom leaves the magnification, and an absent coordinate keeps the *coordinate*
    /// — the point of default user space at the window's corner before the destination, not the
    /// pixel scroll that showed it. A new magnification moves every pixel of the page, so a kept
    /// scroll would be a different `left` and `top`; and a page turn starts the new page at its
    /// top, so the point is read before the turn ([`Self::retain_corner`]) and taken here. ADR 2814.
    ///
    /// **`/FitBH`'s `top` and `/FitBV`'s `left` are coordinates like any other form's.** Table 149
    /// calls them "the vertical coordinate top" and "the horizontal coordinate left", in the same
    /// words as `/FitH` and `/FitV`; only the magnification is the bounding box's. So they go
    /// through the page's transform, and only `/Fit`'s and `/FitB`'s corners — which this function
    /// works out itself, in the display list's space — do not.
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
        // Read before the zoom changes: the point at the corner is a function of the
        // magnification it was scrolled under.
        let current = self.retained_corner.take().or_else(|| {
            self.magnification(viewport, scale)
                .and_then(|magnification| self.window_corner(magnification))
        });
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
        // The point put at the window's top-left corner, and which of its two components the
        // destination decides. `/Fit` and `/FitB` are worked out here in the display list's
        // space; every other form states default user space, where a null takes the current
        // value (`current`) and only a coordinate with neither stays where the scroll has it.
        let corner = match view {
            // "Display the page ... with its contents magnified just enough to fit the entire
            // page within the window" — the whole page is in view, so there is nothing to
            // scroll to and the top-left corner is the answer in both directions.
            View::Fit => Some(((0.0, size.height), (true, true))),
            View::FitB => Some(content.map_or(((0.0, size.height), (true, true)), |box_| {
                ((box_.min.x, box_.max.y), (true, true))
            })),
            View::Xyz { left, top, .. } => self.user_corner((left, top), current, size.height),
            View::FitH { top } | View::FitBH { top } => {
                self.user_corner((None, top), current, size.height)
            }
            View::FitV { left } | View::FitBV { left } => {
                self.user_corner((left, None), current, size.height)
            }
            // Table 149 puts the rectangle's own corner at the window's.
            View::FitR { rect } => self.user_corner(
                (Some(rect[0].min(rect[2])), Some(rect[1].max(rect[3]))),
                current,
                size.height,
            ),
        };
        if let Some((point, decided)) = corner {
            self.scroll_to(point, decided, magnification, size.height);
        }
        crate::layout::settle_scroll(self, viewport, scale, magnification);
        (self.zoom, self.scroll) != before
    }

    /// Records the point at the window's corner before a page turn discards the scroll, so that a
    /// waiting destination's null `left` or `top` can keep it (Table 149).
    ///
    /// Does nothing where no view is waiting: a turn with no destination starts the page at its
    /// top, and that is the turn's own rule rather than a value a destination retains.
    pub(crate) fn retain_corner(&mut self, viewport: (u32, u32), scale: f32) {
        if self.pending_views.is_empty() {
            return;
        }
        self.retained_corner = self
            .magnification(viewport, scale)
            .and_then(|magnification| self.window_corner(magnification));
    }

    /// The point of default user space at the window's top-left corner of the current page.
    fn window_corner(&self, magnification: f32) -> Option<(f32, f32)> {
        let size = self.page_size(self.page_index)?;
        let page = self.shown_page()?;
        // The inverse of `scroll_to`: a scroll counts down from the top, the display list's y
        // counts up from the bottom.
        pdf_model::content::user_space_at(
            page,
            self.scroll.0 / magnification,
            size.height - self.scroll.1 / magnification,
        )
    }

    /// A corner stated in default user space, with each null filled from `current`, carried into
    /// the display list's space through the page's transform (§7.7.3.3's rotation, the crop
    /// box's origin, `/UserUnit`).
    ///
    /// Answers which components were decided — stated or retained — beside the point; `None`
    /// where neither was, which leaves the scroll as it is.
    fn user_corner(
        &self,
        stated: (Option<f32>, Option<f32>),
        current: Option<(f32, f32)>,
        page_height: f32,
    ) -> Option<((f32, f32), (bool, bool))> {
        let left = stated.0.or(current.map(|corner| corner.0));
        let top = stated.1.or(current.map(|corner| corner.1));
        if left.is_none() && top.is_none() {
            return None;
        }
        let (x, y) = (left.unwrap_or(0.0), top.unwrap_or(page_height));
        let point = match self.shown_page() {
            Some(page) => pdf_model::content::page_space_at(page, x, y),
            None => (x, y),
        };
        Some((point, (left.is_some(), top.is_some())))
    }

    /// Puts a point of the display list's space at the window's top-left corner, in each
    /// direction `decided` names.
    fn scroll_to(
        &mut self,
        (x, y): (f32, f32),
        decided: (bool, bool),
        magnification: f32,
        page_height: f32,
    ) {
        if decided.0 {
            self.scroll.0 = (x * magnification).max(0.0);
        }
        if decided.1 {
            // The display list's y counts up from the bottom of the page and a raster's rows
            // count down from the top, so the distance scrolled is measured from the *top*.
            self.scroll.1 = ((page_height - y) * magnification).max(0.0);
        }
    }
}

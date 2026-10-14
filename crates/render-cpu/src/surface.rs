//! The rows a draw may mark, and the map from page space onto them — ADR 0219.
//!
//! A [`Band`] is a run of page rows, a [`Surface`] the page and the band it is drawn into, and a
//! [`ToDevice`] the page transform with that band's first row subtracted last, which is what makes
//! a strip's pixels the page's.

use pdf_render::{Command, TargetSpec, Transform};

/// A contiguous run of **page** rows: the only rows a command is allowed to mark.
///
/// A command can only change pixels its clip admits, and a clip usually admits very
/// little of the page. Restricting the drawing surface to those rows is what keeps a
/// command's cost proportional to what it can actually change, and it is not a micro
/// optimisation: on `bug1721218_reduced.pdf` (3576 distinct clips, each covering a
/// mean 1.2% of the page's height) `callgrind` attributed **80% of the whole render**
/// to the raster pipeline's gradient stage, evaluating shadings across the full page
/// only for the clip to discard almost all of it. Bounding the surface removes that
/// work rather than masking it away afterwards.
///
/// Rows and not a rectangle, because a pixmap's rows are contiguous in memory and its
/// columns are not: `tiny_skia::PixmapMut::from_bytes` can borrow a band of a pixmap,
/// and `tiny-skia` exposes no sub-rectangle view. The same constraint applies to the
/// clip mask, which must share the pixmap's row stride, so it is band-tall and
/// page-wide too.
///
/// **Page rows, and a [`Surface`]'s own rows are one of these too.** A strip of a page and
/// the band a clip admits are the same idea at two scales, and counting both from the page's
/// first row is what lets the offset from page space reach the geometry once (ADR 0219).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Band {
    /// First page row this band covers.
    pub(crate) top: u32,
    /// Number of rows.
    pub(crate) height: u32,
}

impl Band {
    /// The band covering `bounds`, clipped to the rows `surface` holds.
    ///
    /// Returns `None` when `bounds` covers no row of the surface.
    ///
    /// `bounds` is widened by a row before rounding outward. Clip bounds are computed
    /// from a path's control points and then transformed, while the mask is built by
    /// transforming the path itself; the two agree to within floating-point rounding
    /// rather than exactly, and a band one row short would erase a row of a shading
    /// that the clip admits. A spare row costs a fraction of a percent of the band and
    /// removes that class of defect entirely.
    pub(crate) fn covering(bounds: tiny_skia::Rect, surface: Surface) -> Option<Self> {
        let rows = bounds.outset(0.0, 1.0)?.round_out()?;
        let first = i32::try_from(surface.rows.top).ok()?;
        let limit = first.checked_add(i32::try_from(surface.rows.height).ok()?)?;
        let top = rows.top().clamp(first, limit);
        let bottom = rows.bottom().clamp(first, limit);
        let rows = u32::try_from(bottom.checked_sub(top)?).ok()?;
        (rows > 0).then_some(Self {
            top: u32::try_from(top).ok()?,
            height: rows,
        })
    }

    /// Borrows this band's rows of `pixmap`, which covers `surface`, as a pixmap of its own.
    ///
    /// `None` only if the band does not lie within the surface, which [`Band::covering`]
    /// does not produce.
    pub(crate) fn rows<'a>(
        self,
        pixmap: &'a mut tiny_skia::PixmapMut<'_>,
        surface: Surface,
    ) -> Option<tiny_skia::PixmapMut<'a>> {
        let width = pixmap.width();
        let stride = (width as usize).checked_mul(4)?;
        let start = (self.top.checked_sub(surface.rows.top)? as usize).checked_mul(stride)?;
        let end = start.checked_add((self.height as usize).checked_mul(stride)?)?;
        let rows = pixmap.data_mut().get_mut(start..end)?;
        tiny_skia::PixmapMut::from_bytes(rows, width, self.height)
    }

    /// Bytes a mask covering this band of a `width`-wide target occupies.
    pub(crate) fn mask_bytes(self, width: u32) -> usize {
        (self.height as usize).saturating_mul(width as usize)
    }

    /// The rows this band and `other` share, or `None` where they share none.
    pub(crate) fn intersection(self, other: Self) -> Option<Self> {
        let top = self.top.max(other.top);
        let bottom = self
            .top
            .saturating_add(self.height)
            .min(other.top.saturating_add(other.height));
        (bottom > top).then(|| Self {
            top,
            height: bottom.saturating_sub(top),
        })
    }
}

/// The rows of a page one call draws into, and the page they belong to.
///
/// # Why the page's transform travels and a strip's does not
///
/// A strip is a run of the page's rows, and the obvious way to hand one to a rasteriser is to
/// give it a target whose transform has been shifted up by the rows above it, and **that makes
/// the picture depend on how the page was divided** (ADR 0219): `Transform::then` folds the
/// shift into the page transform's `f`, so
/// a mark's own translation is then added to a number of a different magnitude and the sum
/// rounds elsewhere. One `ulp` of the composed matrix is one supersample of a glyph's edge,
/// which `tiny-skia` quantises to 16 of 255 — the pixel ADR 0219 was written for.
///
/// So the page's own target is what travels, and where the surface starts is a separate whole
/// number of rows, applied once and last by [`ToDevice`]. Every band, clip mask and group
/// buffer below is measured in page rows for the same reason.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Surface {
    /// The whole page's target: `transform` maps page space onto the *page's* device grid,
    /// however few of its rows this surface holds.
    pub(crate) page: TargetSpec,
    /// The page rows it holds.
    pub(crate) rows: Band,
}

impl Surface {
    /// The surface covering a whole page.
    pub(crate) fn whole(page: TargetSpec) -> Self {
        Self {
            page,
            rows: Band {
                top: 0,
                height: page.height,
            },
        }
    }

    /// Pixels across, which is the page's width: a surface is a run of whole rows.
    pub(crate) fn width(self) -> u32 {
        self.page.width
    }

    /// The map onto the device grid of `band`, which must be rows this surface holds.
    pub(crate) fn to_device(self, band: Band) -> ToDevice {
        ToDevice {
            page: self.page.transform,
            top: band.top,
        }
    }
}

/// Page space onto the device grid of a surface starting at page row `top`.
///
/// Two values rather than one matrix, and that is the whole of ADR 0219: the row offset is
/// composed **last**, onto the fully composed matrix, where subtracting a whole number of rows
/// from a device coordinate is exact. Folded in first — as `target.transform.then(offset)` —
/// it changes the magnitude every later composition rounds at, and a page drawn in strips stops
/// being the page drawn whole.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ToDevice {
    /// Page space to the whole page's device grid.
    pub(crate) page: Transform,
    /// First page row of the surface being drawn into.
    pub(crate) top: u32,
}

impl ToDevice {
    /// The matrix geometry stated in `transform`'s space is drawn under.
    pub(crate) fn of(self, transform: Transform) -> Transform {
        #[expect(
            clippy::cast_precision_loss,
            reason = "rasterize rejects a target taller than MAX_EXTENT = 2^24, and every \
                      integer below that is exactly representable as f32"
        )]
        let rows = self.top as f32;
        transform
            .then(self.page)
            .then(Transform::translate(0.0, -rows))
    }
}

/// The band's own pixels, as a range into a buffer the size of the whole surface.
///
/// Every buffer in this module is surface-sized and every band is a run of its rows
/// (`groups::initial_backdrop`'s own slice arithmetic), so one range indexes all of them alike.
pub(crate) fn band_pixels(surface: Surface, band: Band) -> core::ops::Range<usize> {
    let width = surface.width() as usize;
    let first = (band.top.saturating_sub(surface.rows.top) as usize).saturating_mul(width);
    let last = first.saturating_add((band.height as usize).saturating_mul(width));
    first..last
}

/// Whether a command's own extent lies wholly above or below `surface`'s rows.
///
/// A group answers `false`: its elements carry the extents and each is asked in turn.
///
/// Measured on the *page's* grid, so a strip asks the same question of the same numbers the
/// whole page asks — only over a different run of rows.
pub(crate) fn misses_surface(command: &Command, surface: Surface) -> bool {
    let Some(bounds) = command.device_bounds(surface.page.transform) else {
        return false;
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "a target height below MAX_EXTENT = 2^24 is exact in f32"
    )]
    let (top, bottom) = (
        surface.rows.top as f32,
        surface.rows.top.saturating_add(surface.rows.height) as f32,
    );
    bounds.max.y < top - 1.0 || bounds.min.y > bottom + 1.0
}

#[cfg(test)]
mod tests {

    use super::ToDevice;

    use pdf_render::Transform;

    /// A strip's matrix is the page's matrix with a whole number of rows taken off, exactly.
    ///
    /// This is the whole of ADR 0219 in one assertion, and it is stated in `f64` so that it is
    /// about the arithmetic rather than about itself: recomputing the subtraction in `f32`
    /// would round the same way twice and prove nothing. Every component but `f` must be the
    /// page's own bits, and `f` must be the page's `f` minus the offset with no rounding at all
    /// — which is what makes composing the offset *last* different from folding it into the
    /// page transform first, where the mark's own translation is then added at another
    /// magnitude and rounds elsewhere.
    #[test]
    fn the_offset_is_composed_last_and_costs_nothing() {
        // A page transform of the shape `TargetSpec::for_page` builds, and mark transforms of
        // the shapes a content stream states: a glyph at 8 and at 20 points, and a form's.
        let pages = [
            Transform::new(1.0, 0.0, 0.0, -1.0, 0.0, 841.89),
            Transform::new(0.838_926_2, 0.0, 0.0, -0.838_926_2, 0.0, 706.283_57),
            Transform::new(2.019_7, 0.0, 0.0, -2.019_7, 0.0, 1_683.783_4),
        ];
        let marks = [
            Transform::IDENTITY,
            Transform::new(8.0, 0.0, 0.0, 8.0, 488.387_54, 51.023_6),
            Transform::new(20.0, 0.0, 0.0, 20.0, 51.023_6, 51.023_6),
            Transform::new(0.5, 0.25, -0.25, 0.5, 133.777, 642.101),
        ];

        for page in pages {
            for mark in marks {
                let whole = ToDevice { page, top: 0 }.of(mark);
                for top in [1_u32, 8, 64, 236, 512, 887, 1024] {
                    let strip = ToDevice { page, top }.of(mark);
                    assert_eq!(
                        (
                            strip.a.to_bits(),
                            strip.b.to_bits(),
                            strip.c.to_bits(),
                            strip.d.to_bits(),
                            strip.e.to_bits()
                        ),
                        (
                            whole.a.to_bits(),
                            whole.b.to_bits(),
                            whole.c.to_bits(),
                            whole.d.to_bits(),
                            whole.e.to_bits()
                        ),
                        "a row offset moved something other than the vertical translation"
                    );
                    #[expect(
                        clippy::float_cmp,
                        reason = "exactly what is asserted: the subtraction of a whole number \
                                  of rows from a device coordinate rounds not at all, so an \
                                  epsilon here would assert nothing"
                    )]
                    let exact = (f64::from(whole.f) - f64::from(top)) == f64::from(strip.f);
                    assert!(
                        exact,
                        "{top} rows off {whole:?} gave {strip:?}, which is not the page's \
                         translation minus {top}: the subtraction rounded"
                    );
                }
            }
        }
    }
}

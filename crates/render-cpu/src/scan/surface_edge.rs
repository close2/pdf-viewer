//! A rectangle the surface's edge cuts is drawn as the rows of the rectangle the edge does not cut
//! — ISO 32000-2 §10.7.4, ADR 2084.
//!
//! A strip is a window of the page, and §10.7.4 fixes the pixel it is a window onto as the
//! half-open square `[i, i+1) × [j, j+1)`: a row of a strip and the same row of the page are the
//! same pixels, so a rectangle owes them the same coverage. `tiny-skia`'s rectangle converter
//! clips a rectangle to the surface before it walks it in 8.8 fixed point, and a rectangle with one
//! row left inside the surface goes down its one-row branch, which states that row's coverage as
//! `bottom − top − 1` and its corners as a product shifted down eight bits, where the same row of
//! the rectangle unclipped is `256 − top` and each corner its own overlap — a level apart. So a
//! rectangle a strip cut crossed beside its edge came out a level from the page drawn whole, at
//! every coverage of that row (`tests/rectangle_across_a_cut.rs` prints them), and a page drawn in
//! one strip and in four differed at 144 pixels of an ordinary 368 × 542 page.
//!
//! ADR 0139 measured a cut across an axis-aligned edge exact, on the library's *path* converter,
//! and the planner has divided pages on that since; ADR 0476 moved rectangles to this converter,
//! and this is that claim measured again on it.
//!
//! What closes it is to hand the converter the rows a surface's edge leaves few of in a surface
//! with a row beyond that edge: the rectangle is then unclipped at the rows the strip holds, and
//! the converter states them as it states them on the page, by its own arithmetic. Neither of the
//! two other constructions does that. The closed form `scan` writes past 8191 states §10.7.4's exact
//! area, which is neither the page's value nor the cut row's — 191.25 against the page's 192 —
//! and keeping the cuts off a rectangle's edge rows, measured, forbids rows on enough pages that 35
//! more of the corpus's first pages lose `Underway::in_strips`' drawing ahead, whose cuts are even
//! and fixed before the page is read (ADR 2084).
//!
//! The page drawn in one strip takes the same road at its own edges, where it meets a rectangle
//! with a row or two inside the page and the rest outside it, so every division of a page states
//! each of its rows the same way.

/// The most rows of a rectangle a surface may hold for its edge to be the reason it is clipped
/// there: the converter's one-row branch is taken for a rectangle with one row inside, stated to
/// 1/256, and a second row is the margin over that rounding.
const FEW_ROWS: f32 = 2.0;

/// The rows of a surface a rectangle reaches where the surface's edge clips it to [`FEW_ROWS`] or
/// fewer, and on which sides the rectangle reaches past the surface.
#[derive(Clone, Copy, Debug)]
struct EdgeRows {
    /// The first row of the surface the rectangle reaches.
    from: u32,
    /// The row after the last.
    until: u32,
    /// Whether the rectangle reaches above the surface's first row.
    above: bool,
    /// Whether it reaches below the surface's last row.
    below: bool,
}

impl EdgeRows {
    /// The rows of a `height`-row surface `rect` reaches, where the surface's edge leaves it few.
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a surface the rectangle converter reaches is at most 8191 rows, exact in f32, and \
                  both rows are clamped to it before the casts"
    )]
    fn of(rect: tiny_skia::Rect, height: u32) -> Option<Self> {
        let rows = height as f32;
        let above = rect.top() < 0.0;
        let below = rect.bottom() > rows;
        let top = rect.top().max(0.0);
        let bottom = rect.bottom().min(rows);
        let few = bottom > top && bottom - top <= FEW_ROWS;
        if !(above || below) || !few {
            return None;
        }
        Some(Self {
            from: top.floor() as u32,
            until: (bottom.ceil() as u32).min(height),
            above,
            below,
        })
    }

    /// How many rows the surface beyond its edge holds: the rows reached and one past each edge
    /// the rectangle reaches past.
    fn scratch_height(self) -> u32 {
        self.until
            .saturating_sub(self.from)
            .saturating_add(u32::from(self.above))
            .saturating_add(u32::from(self.below))
    }

    /// The surface row the scratch surface's first row stands for, which is the row above the
    /// first reached where the rectangle reaches above the surface.
    fn first(self) -> f32 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a row of a surface of at most 8191 rows, exact in f32"
        )]
        let from = self.from as f32;
        from - f32::from(u8::from(self.above))
    }

    /// The bytes of the rows reached in a surface `stride` bytes a row, and where they sit in the
    /// scratch surface.
    fn spans(self, stride: usize) -> (std::ops::Range<usize>, std::ops::Range<usize>) {
        let reached = (self.until.saturating_sub(self.from) as usize).saturating_mul(stride);
        let on_surface = (self.from as usize).saturating_mul(stride);
        let in_scratch = usize::from(self.above).saturating_mul(stride);
        (
            on_surface..on_surface.saturating_add(reached),
            in_scratch..in_scratch.saturating_add(reached),
        )
    }
}

/// Fills `rect`, in the surface's own pixels, through `tiny-skia`'s rectangle converter, every row
/// of it inside the surface stated as the rectangle the surface's edge does not clip states it.
///
/// `paint`'s shader already carries the device transform, as [`super::fill_rectangles`] moved it.
/// Where the surface's edge leaves the rectangle more than [`FEW_ROWS`] rows, or does not cut it at
/// all, the converter states every row of it unclipped already and is asked directly.
pub(super) fn fill_rect(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    rect: tiny_skia::Rect,
    paint: &tiny_skia::Paint<'_>,
    mask: Option<&tiny_skia::Mask>,
) {
    let identity = tiny_skia::Transform::identity();
    let Some(rows) = EdgeRows::of(rect, pixmap.height()) else {
        pixmap.fill_rect(rect, paint, identity, mask);
        return;
    };
    if !beyond_the_edge(pixmap, rect, paint, mask, rows) {
        // A scratch surface of a few rows that cannot be made, or a mask of another size, which
        // the converter answers by drawing nothing: the rectangle is drawn as the surface clips it,
        // which is a level at most from the page in the rows reached.
        pixmap.fill_rect(rect, paint, identity, mask);
    }
}

/// [`fill_rect`]'s construction: the rows reached copied into a surface with a row beyond each
/// edge the rectangle crosses, the rectangle drawn there, and the rows copied back. Returns `false`
/// where it could not be made, before anything was drawn.
fn beyond_the_edge(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    rect: tiny_skia::Rect,
    paint: &tiny_skia::Paint<'_>,
    mask: Option<&tiny_skia::Mask>,
    rows: EdgeRows,
) -> bool {
    let (width, height) = (pixmap.width(), rows.scratch_height());
    if mask.is_some_and(|mask| mask.width() != width || mask.height() != pixmap.height()) {
        return false;
    }
    let Some(mut scratch) = tiny_skia::Pixmap::new(width, height) else {
        return false;
    };
    let pixels = (width as usize).saturating_mul(4);
    let (on_surface, in_scratch) = rows.spans(pixels);
    let (Some(from), Some(into)) = (
        pixmap.data_mut().get(on_surface.clone()),
        scratch.data_mut().get_mut(in_scratch.clone()),
    ) else {
        return false;
    };
    into.copy_from_slice(from);
    let scratch_mask = match mask {
        None => None,
        Some(mask) => {
            let Some(mut rows_of) = tiny_skia::Mask::new(width, height) else {
                return false;
            };
            let (on_mask, in_rows) = rows.spans(width as usize);
            let (Some(from), Some(into)) = (
                mask.data().get(on_mask),
                rows_of.data_mut().get_mut(in_rows),
            ) else {
                return false;
            };
            into.copy_from_slice(from);
            Some(rows_of)
        }
    };
    // The scratch surface's first row is the surface's row `first`, so everything drawn there is
    // moved up by it — a whole number of rows, as a strip's own origin is (ADR 0219).
    let first = rows.first();
    let Some(moved) = tiny_skia::Rect::from_ltrb(
        rect.left(),
        rect.top() - first,
        rect.right(),
        rect.bottom() - first,
    ) else {
        return false;
    };
    let mut paint = paint.clone();
    paint
        .shader
        .transform(tiny_skia::Transform::from_translate(0.0, -first));
    scratch.fill_rect(
        moved,
        &paint,
        tiny_skia::Transform::identity(),
        scratch_mask.as_ref(),
    );
    let (Some(from), Some(into)) = (
        scratch.data().get(in_scratch),
        pixmap.data_mut().get_mut(on_surface),
    ) else {
        // Both spans were read above from the same two surfaces, so this is unreachable; the
        // rectangle has then been drawn nowhere, and declining draws it as the surface clips it.
        return false;
    };
    into.copy_from_slice(from);
    true
}

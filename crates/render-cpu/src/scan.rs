//! The one place a path reaches `tiny-skia`'s scan converter, and the range that bounds it.
//!
//! # Why this module exists
//!
//! ISO 32000-2 §10.7 leaves scan conversion to the device and states no bound on a
//! coordinate, and §7.3.3 hands the range of a number to the implementation outright:
//!
//! > The range and precision of numbers may be limited by the internal representations used in
//! > the computer on which the PDF processor is running; Annex C, "Advice on maximising
//! > portability", gives these limits for typical implementations.
//!
//! That annex is informative and gives no figure for a coordinate — its entry for real numbers
//! says only that computers "often" use IEEE 754. So the magnitude a page may state is this
//! processor's to decide, and a page whose content stream was damaged in transit states one
//! nobody decided.
//!
//! `tiny-skia`'s scan converter does its arithmetic in 16.16 fixed point, and its
//! anti-aliased path supersamples by four before it gets there. The library says the
//! consequence itself, in the comment above `DrawTiler`: its fixed-point types are limited by
//! 8192 and 32768, which means that it cannot render a path larger than 8192 onto a pixmap —
//! and again beside the constant that enforces it, 8K being one too big because `8K << 2` is
//! 32768 and too big for `Fixed`. What it bounds by that number is the **pixmap**, which it
//! tiles; what it does not bound is the **path**, whose coordinates run through the same
//! arithmetic. `SuperBlitter::blit_h` carries a comment admitting as much — a hack, it says,
//! until somebody works out why the cubics go beyond the bounds — and handles only the left
//! side of the overrun.
//!
//! A path that leaves the range therefore produces coverage the arithmetic does not define,
//! and on the right geometry it walks `AlphaRuns`' run buffer past its end and unwraps a
//! `None`. That is a **panic in a dependency reachable from a document**, and under
//! `[profile.release]`'s `panic = "abort"` it is the whole process: the four-hundred-and-
//! thirty-third session met it on two of 65 944 crawled documents (ADR 0269).
//!
//! # What is done about it, and why it is not a refusal
//!
//! The **non**-anti-aliased scan converter takes the same geometry without complaint, which
//! is checkable and was checked: over the reduction in ADR 0269, magnitudes from 10³ to 10³⁰
//! all return, and where the anti-aliased converter also returns the two agree to within the
//! anti-aliasing of the edges (13 250 820 against 13 252 311 of ink, 0.011%). `tiny-skia`
//! makes the same substitution one branch over for its own overflow test — `path_aa::fill_path`
//! calls `path::fill_path` when the clipped bounds would overflow the shift — so this is the
//! library's own remedy applied where its test does not reach.
//!
//! So a path outside the range is **drawn without anti-aliasing** rather than refused. A
//! refusal would cost the page: `CpuRasterError` stops the whole raster, so one damaged path
//! would take every mark after it, and this backend is the correctness oracle. What the
//! substitution costs is at most half a pixel of edge quality on a shape whose own extent is
//! thousands of pages across.

/// The largest device coordinate the anti-aliased scan converter's arithmetic can express.
///
/// `tiny-skia`'s own number, for its own stated reason: supersampling shifts a coordinate left
/// by two before it becomes 16.16 fixed point, and `8192 << 2` is 32768, which its `Fixed` type
/// cannot hold. The library applies it to the pixmap it draws into; this module applies it to
/// the geometry, which is the half it leaves open.
const SUPERSAMPLED_LIMIT: f32 = 8191.0;

/// Whether `bounds` lies inside [`SUPERSAMPLED_LIMIT`] on both axes.
fn within(bounds: Option<tiny_skia::Rect>) -> bool {
    // `None` is a rectangle the transform could not produce — a coordinate that overflowed or
    // is not finite — which is exactly the case this is here to keep out.
    bounds.is_some_and(|bounds| {
        bounds.left() >= -SUPERSAMPLED_LIMIT
            && bounds.top() >= -SUPERSAMPLED_LIMIT
            && bounds.right() <= SUPERSAMPLED_LIMIT
            && bounds.bottom() <= SUPERSAMPLED_LIMIT
    })
}

/// Whether `path` drawn under `at` stays inside the range, grown by `outset` first.
///
/// The outset is in the **path's** own space, which is where a stroke's width is stated, so it
/// is applied before `at` rather than after: a caller with a transform does not have to take a
/// singular value of it to say how far a stroke reaches. A fill passes zero.
fn expressible(path: &tiny_skia::Path, at: tiny_skia::Transform, outset: f32) -> bool {
    let reach = if outset.is_finite() {
        outset.max(0.0)
    } else {
        f32::MAX
    };
    within(
        path.bounds()
            .outset(reach, reach)
            .and_then(|bounds| bounds.transform(at)),
    )
}

/// Anti-aliasing, kept only where the scan converter's arithmetic reaches the geometry.
fn keep_anti_alias(anti_alias: bool, expressible: bool) -> bool {
    anti_alias && expressible
}

/// What the mask a mark is drawn through **is**, which is what decides how the two compose.
///
/// The two are different mechanisms in ISO 32000-2 and the standard states them with different
/// words. §10.7.4's clipping paragraph states a region as a *set of pixels*:
///
/// > For clipping, the clipping region consists of the set of pixels that would be included by
/// > a fill operation. Subsequent painting operations shall affect a region that is the
/// > intersection of the set of pixels defined by the clipping region with the set of pixels
/// > for the region to be painted.
///
/// §11.6.5.2 states a soft mask as a *value*, and Table 142 makes it a factor of the object's
/// alpha — a product the standard asks for. A mask that carries both is a `Value`: once the
/// clip has been multiplied into a soft mask there is no clip left to intersect with.
///
/// [`mask_intersect`] is the same distinction one step earlier, where two clips meet each other;
/// this one is where a clip meets the mark. ADR 0280 took the first and left this second.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Clip<'a> {
    /// Nothing masks the mark.
    Unclipped,
    /// §10.7.4's clipping region, on its own, with the buffer the composition needs.
    Region {
        /// The region's coverage.
        mask: &'a tiny_skia::Mask,
        /// Where the mark's own coverage is built before the two are composed.
        scratch: &'a Scratch,
    },
    /// A coverage that multiplies the mark's: §11.6.5's soft mask, alone or with a clip
    /// already multiplied into it.
    Value(&'a tiny_skia::Mask),
}

impl<'a> Clip<'a> {
    /// The mask itself, for the callers that hand one to `tiny-skia` unexamined.
    pub(crate) fn mask(self) -> Option<&'a tiny_skia::Mask> {
        match self {
            Clip::Unclipped => None,
            Clip::Region { mask, .. } | Clip::Value(mask) => Some(mask),
        }
    }
}

/// The coverage buffer [`intersected`] builds a mark in, kept for the length of one band.
///
/// **It is a buffer rather than an allocation per mark because that is what it measures.**
/// `tiny-skia` will only take a mask of the pixmap's own size, so the buffer is a band's worth
/// of bytes; the corpus's heaviest clip page states 3554 clipped fills on one 612×792 page, and
/// allocating and zeroing one of these for each of them cost **+54% of that page's
/// rasterisation** where reusing one costs a twentieth of that. Only the pixels a mark can reach
/// are cleared, which is why the reuse is cheaper than the allocator's own zeroing rather than
/// merely equal to it.
///
/// One lives in each [`MaskCache`](crate::MaskCache) — one per strip of a parallel render, which
/// is what keeps it out of any shared state — and the cell is borrowed for the length of a single
/// fill, which cannot nest.
#[derive(Debug, Default)]
pub(crate) struct Scratch {
    coverage: std::cell::RefCell<Option<tiny_skia::Mask>>,
}

/// [`tiny_skia::PixmapMut::fill_path`], with the range applied to `paint.anti_alias` and
/// [`Clip::Region`] composed with the mark's own coverage by `min` rather than by a product.
pub(crate) fn fill(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    path: &tiny_skia::Path,
    paint: &tiny_skia::Paint<'_>,
    fill_rule: tiny_skia::FillRule,
    at: tiny_skia::Transform,
    clip: Clip<'_>,
) {
    let mut paint = paint.clone();
    paint.anti_alias = keep_anti_alias(paint.anti_alias, expressible(path, at, 0.0));
    if let Clip::Region { mask, scratch } = clip
        && intersected(pixmap, path, &paint, fill_rule, at, (mask, scratch))
    {
        return;
    }
    pixmap.fill_path(path, &paint, fill_rule, at, clip.mask());
}

/// Draws `path` with its own coverage and `region`'s composed by `min`, ISO 32000-2 §10.7.4.
///
/// Returns `false` where it declined, which leaves the caller's ordinary draw to run: this is a
/// substitution for one composition rather than a second scan converter, and everything it
/// cannot state it hands back rather than approximating.
///
/// # Why a mark needs this and `tiny_skia::PixmapMut::fill_path` cannot give it
///
/// That method multiplies the mask into the mark's coverage, so a mark whose own boundary falls
/// in a pixel a clip boundary also crosses is painted at the product of two fractions. §10.7.4
/// asks for the *intersection of two sets of pixels* and §8.5.4 for the intersection of two
/// shapes — "[t]he effective shape is the intersection of the object's intrinsic shape with the
/// clipping path; the source shape value shall be 0.0 outside this intersection" — and neither
/// lowers a value the clip admits. The whole argument for `min` over the product is
/// [`mask_intersect`]'s, unchanged: it is exact where the two boundaries coincide or nest, and
/// never below the product where they merely share a pixel, so it never moves away from the
/// clause's whole pixel.
///
/// # The three things it declines, each because the substitution would say something else
///
/// - **A clip that is already a set** — every value under the mark either 0 or 255. There the
///   product *is* the intersection, pixel for pixel, so the ordinary draw already carries the
///   clause out and the cheaper path is also the correct one. This is what keeps the cost off
///   the pages that do not need it.
/// - **A mark that is not anti-aliased**, whose coverage is 0 or 255 for the same reason.
/// - **`BlendMode::Source`**, which is [`crate::carries_coverage_as_alpha`]'s exclusion and is
///   excluded here for its own half of that reason: this construction delivers the composed
///   coverage as the *mask* of a fully covered run, and `tiny-skia` applies a mask by scaling
///   the source where it applies a path's coverage by interpolating towards the destination.
///   The two agree for every mode whose result has Porter-Duff's form — scaling a premultiplied
///   source by `c` and interpolating the blend by `c` are the same function there, which is
///   what `BlendMode::should_pre_scale_coverage` says of the modes it names and what the
///   algebra says of the rest — and they part for Source, where the destination does not enter
///   the result at all. §11.4.6's knockout is the one place this backend states that mode.
fn intersected(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    path: &tiny_skia::Path,
    paint: &tiny_skia::Paint<'_>,
    fill_rule: tiny_skia::FillRule,
    at: tiny_skia::Transform,
    (region, scratch): (&tiny_skia::Mask, &Scratch),
) -> bool {
    if !crate::carries_coverage_as_alpha(paint.anti_alias, paint.blend_mode) {
        return false;
    }
    // `tiny-skia` draws nothing at all through a mask of another size, so a mismatch is left to
    // the ordinary call, which answers it the same way it does today.
    if region.width() != pixmap.width() || region.height() != pixmap.height() {
        return false;
    }
    let Some(reach) = reached_pixels(path, at, pixmap.width(), pixmap.height()) else {
        return false;
    };
    let Some(rect) = reach.rect() else {
        return false;
    };
    if is_a_set(region, reach, pixmap.width()) {
        return false;
    }
    let Ok(mut held) = scratch.coverage.try_borrow_mut() else {
        // A fill cannot nest inside another fill, so this is unreachable; declining is the
        // answer that draws the mark anyway if it ever stops being.
        return false;
    };
    let coverage = match held.as_mut() {
        Some(mask) if mask.width() == pixmap.width() && mask.height() == pixmap.height() => mask,
        _ => {
            *held = tiny_skia::Mask::new(pixmap.width(), pixmap.height());
            match held.as_mut() {
                Some(mask) => mask,
                None => return false,
            }
        }
    };
    let stride = pixmap.width() as usize;
    // Only what this mark can reach is cleared and composed. The rest of the buffer holds the
    // last mark's coverage and is never read: `reach` contains the path's own device bounds, so
    // the mark is zero outside it, and the rectangle drawn through it below is `reach` itself.
    for row in reach.rows() {
        let (from, until) = reach.span(row, stride);
        if let Some(row) = coverage.data_mut().get_mut(from..until) {
            row.fill(0);
        }
    }
    mask_fill(coverage, path, fill_rule, paint.anti_alias, at);
    let admitted = region.data();
    let mark = coverage.data_mut();
    for row in reach.rows() {
        let (from, until) = reach.span(row, stride);
        let (Some(mark), Some(admitted)) = (mark.get_mut(from..until), admitted.get(from..until))
        else {
            continue;
        };
        for (mark, &admitted) in mark.iter_mut().zip(admitted) {
            *mark = (*mark).min(admitted);
        }
    }
    // The composed coverage is now the mask, so what is drawn through it is a run of whole
    // pixels — §10.7.4's own construction for a mark it cannot measure, and the shape whose
    // coverage cannot enter the product a second time. The paint carries the transform the
    // library would have applied to it: `fill_path` transforms the shader and then draws with
    // an identity transform, and this is that same step performed one call earlier so that the
    // rectangle can be stated on the device's own grid. Trap 2 is what it would cost to get
    // wrong, and `clip_intersection.rs` is the scene that watches it.
    let mut paint = paint.clone();
    paint.shader.transform(at);
    paint.anti_alias = false;
    pixmap.fill_rect(
        rect,
        &paint,
        tiny_skia::Transform::identity(),
        Some(coverage),
    );
    true
}

/// The pixels a mark drawn under `at` can reach, clamped to a raster `width` by `height`.
///
/// Rounded outwards and grown by a pixel, for the reason `Band::covering` takes one: the extent
/// comes from control points and the coverage from the scan converter, and a mark must not lose
/// ink to this rectangle. `None` where the transform states no rectangle at all, or where the
/// mark falls outside the raster entirely.
fn reached_pixels(
    path: &tiny_skia::Path,
    at: tiny_skia::Transform,
    width: u32,
    height: u32,
) -> Option<Reach> {
    let bounds = path.bounds().transform(at)?;
    let left = clamped(bounds.left() - 1.0, width);
    let top = clamped(bounds.top() - 1.0, height);
    let right = clamped(bounds.right() + 1.0, width);
    let bottom = clamped(bounds.bottom() + 1.0, height);
    (left < right && top < bottom).then_some(Reach {
        left,
        top,
        right,
        bottom,
    })
}

/// `value` as a whole number of pixels inside `0..=limit`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the value is clamped into 0..=limit as a float before the cast back, so it is \
              non-negative and whole; a raster's own dimension is far inside f32's exact range \
              and [`SUPERSAMPLED_LIMIT`] is the bound that says so"
)]
fn clamped(value: f32, limit: u32) -> u32 {
    if value.is_nan() {
        return 0;
    }
    value.floor().clamp(0.0, limit as f32) as u32
}

/// A rectangle of whole pixels, right and bottom exclusive.
#[derive(Clone, Copy, Debug)]
struct Reach {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

impl Reach {
    /// The rows it covers.
    fn rows(self) -> std::ops::Range<u32> {
        self.top..self.bottom
    }

    /// Where `row` starts and ends in a buffer of `stride` pixels per row.
    fn span(self, row: u32, stride: usize) -> (usize, usize) {
        let start = (row as usize).saturating_mul(stride);
        (
            start.saturating_add(self.left as usize),
            start.saturating_add(self.right as usize),
        )
    }

    /// The same rectangle as `tiny-skia` states one, or `None` where it will not have it.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a raster's own dimension, far inside f32's exactly representable range"
    )]
    fn rect(self) -> Option<tiny_skia::Rect> {
        tiny_skia::Rect::from_ltrb(
            self.left as f32,
            self.top as f32,
            self.right as f32,
            self.bottom as f32,
        )
    }
}

/// Whether `region` is 0 or 255 at every pixel of `reach` — a set rather than a coverage.
fn is_a_set(region: &tiny_skia::Mask, reach: Reach, stride: u32) -> bool {
    let data = region.data();
    reach.rows().all(|row| {
        let (from, until) = reach.span(row, stride as usize);
        data.get(from..until)
            .is_none_or(|row| row.iter().all(|&value| value == 0 || value == u8::MAX))
    })
}

/// [`tiny_skia::PixmapMut::stroke_path`], with the range applied to `paint.anti_alias`.
///
/// The outset is a whole width times the miter limit rather than the half-width the stroke
/// actually reaches: a miter join extends past the outline by the limit, and over-estimating
/// here costs a path within a hair of the bound its anti-aliasing and nothing else.
///
/// A [`Clip::Region`] still meets this mark by a *product*, which [`fill`] no longer does, and
/// the reason is that a stroke's coverage is not a shape this side holds: `tiny-skia` converts a
/// wide stroke to its outline and fills that, and draws a stroke under a device pixel wide as a
/// hairline that is not that outline at all (ADR 0268). Rasterising the coverage here would mean
/// choosing between duplicating the library's stroker and contradicting its hairline, and the
/// substitutions §10.7.4 already asks for on a sub-pixel rule are *fills* and go through [`fill`].
/// `doc/todo/11` item 4 carries what is left with the population it is worth.
pub(crate) fn stroke(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    path: &tiny_skia::Path,
    paint: &tiny_skia::Paint<'_>,
    style: &tiny_skia::Stroke,
    at: tiny_skia::Transform,
    clip: Clip<'_>,
) {
    let mut paint = paint.clone();
    let outset = style.width * style.miter_limit.max(1.0);
    paint.anti_alias = keep_anti_alias(paint.anti_alias, expressible(path, at, outset));
    pixmap.stroke_path(path, &paint, style, at, clip.mask());
}

/// [`tiny_skia::Mask::fill_path`], with the range applied to `anti_alias`.
pub(crate) fn mask_fill(
    mask: &mut tiny_skia::Mask,
    path: &tiny_skia::Path,
    fill_rule: tiny_skia::FillRule,
    anti_alias: bool,
    at: tiny_skia::Transform,
) {
    let anti_alias = keep_anti_alias(anti_alias, expressible(path, at, 0.0));
    mask.fill_path(path, fill_rule, anti_alias, at);
}

/// Narrows `mask` by a further clip path, taking the **smaller** of the two coverages.
///
/// `scratch` must have `mask`'s dimensions; it is cleared here and holds the new clip's own
/// coverage while the two are composed.
///
/// # Why this is not [`tiny_skia::Mask::intersect_path`]
///
/// That method multiplies the two coverages, and ISO 32000-2 §10.7.4 states a clip as a set
/// rather than as a coverage:
///
/// > For clipping, the clipping region consists of the set of pixels that would be included by
/// > a fill operation. Subsequent painting operations shall affect a region that is the
/// > intersection of the set of pixels defined by the clipping region with the set of pixels
/// > for the region to be painted.
///
/// §8.5.4 says the same thing from the transparent imaging model's side — "[t]he effective
/// shape is the intersection of the object's intrinsic shape with the clipping path; the source
/// shape value shall be 0.0 outside this intersection" — which constrains a clip's effect
/// *outside* the region and says nothing about lowering anything inside it.
///
/// A clipping region taken by the fill rule is 0 or 1 at every pixel, and on such a pair `min`
/// and a product are the same function, so neither composition is derived from the clause
/// directly. What decides between them is that this backend **anti-aliases**, which is
/// departure (1) of §10.7.4's ledger row: a boundary pixel carries a fraction, and two clip
/// boundaries falling in the same pixel then meet. There the two compositions part:
///
/// - a product raises that fraction to a power, so a rectangle stated as a clip *n* times over
///   is drawn with an edge at `cᶰ`, further from the clause's whole pixel with every restatement
///   and in the direction the same subclause's "[t]he area covered by painted pixels shall
///   always be at least as large as the area of the original shape" forbids;
/// - `min` is exact where the two boundaries coincide or nest — restating a clip then changes
///   nothing, which is what a set intersection does — and elsewhere it is never below the
///   product, so it is never further from the clause than the product is.
///
/// Neither is the clause's own answer for two *unrelated* boundaries sharing a pixel: the
/// exact one there is the area of the intersection of the paths, which needs a conflation-free
/// rasteriser. `min` is the composition that never moves away from the clause, and the bound on
/// what it does not reach is [`doc/todo/11`](../../../doc/todo/11-shapes-that-still-disappear.md).
pub(crate) fn mask_intersect(
    mask: &mut tiny_skia::Mask,
    scratch: &mut tiny_skia::Mask,
    path: &tiny_skia::Path,
    fill_rule: tiny_skia::FillRule,
    anti_alias: bool,
    at: tiny_skia::Transform,
) {
    scratch.clear();
    mask_fill(scratch, path, fill_rule, anti_alias, at);
    for (kept, &added) in mask.data_mut().iter_mut().zip(scratch.data()) {
        *kept = (*kept).min(added);
    }
}

#[cfg(test)]
mod tests {
    use super::{SUPERSAMPLED_LIMIT, expressible};

    /// A half-plane whose vertical edge falls at `x`, covering the rest of a 4-row mask.
    fn half_plane(x: f32) -> tiny_skia::Path {
        let mut builder = tiny_skia::PathBuilder::new();
        builder.move_to(x, -1.0);
        builder.line_to(8.0, -1.0);
        builder.line_to(8.0, 5.0);
        builder.line_to(x, 5.0);
        builder.close();
        builder.finish().expect("a rectangle")
    }

    /// Builds the mask for a chain of half-planes, root first, the way `MaskCache::build` does.
    fn chain(edges: &[f32]) -> Vec<u8> {
        let (root, nested) = edges.split_first().expect("a root");
        let mut mask = tiny_skia::Mask::new(8, 4).expect("a mask");
        let mut scratch = tiny_skia::Mask::new(8, 4).expect("a scratch mask");
        super::mask_fill(
            &mut mask,
            &half_plane(*root),
            tiny_skia::FillRule::Winding,
            true,
            tiny_skia::Transform::identity(),
        );
        for edge in nested {
            super::mask_intersect(
                &mut mask,
                &mut scratch,
                &half_plane(*edge),
                tiny_skia::FillRule::Winding,
                true,
                tiny_skia::Transform::identity(),
            );
        }
        mask.data().to_vec()
    }

    /// §10.7.4's clipping paragraph: a clip is a set of pixels, and a set intersected with
    /// itself is that set. The edge is fractional, which is the only placement where a product
    /// and a minimum differ at all.
    #[test]
    fn restating_a_clip_leaves_the_mask_alone() {
        let once = chain(&[2.25]);
        assert!(
            once.iter().any(|&value| value > 0 && value < 255),
            "the edge must be partly covered for this to discriminate: {once:?}"
        );
        for rungs in 2..=6 {
            assert_eq!(
                chain(&vec![2.25; rungs]),
                once,
                "{rungs} coincident clips must give one clip's mask"
            );
        }
    }

    /// Two boundaries that merely share a pixel take the smaller coverage rather than their
    /// product — never below the product, so never further from the clause's whole pixel.
    #[test]
    fn two_boundaries_in_one_pixel_take_the_smaller_coverage() {
        let wide = chain(&[2.25]);
        let narrow = chain(&[2.75]);
        let both = chain(&[2.25, 2.75]);
        for (index, &value) in both.iter().enumerate() {
            assert_eq!(
                value,
                wide[index].min(narrow[index]),
                "cell {index} of the composed chain"
            );
        }
        let edge = 2_usize;
        assert!(
            both[edge] > 0,
            "the shared column must survive the composition: {both:?}"
        );
    }

    /// The mask a half-plane at `x` states, page-sized, the way a clip chain's root is built.
    fn region(x: f32) -> tiny_skia::Mask {
        let mut mask = tiny_skia::Mask::new(8, 4).expect("a mask");
        super::mask_fill(
            &mut mask,
            &half_plane(x),
            tiny_skia::FillRule::Winding,
            true,
            tiny_skia::Transform::identity(),
        );
        mask
    }

    /// Fills the half-plane at `mark` through `clip`, and returns the alpha of every pixel of
    /// the first row.
    ///
    /// Black onto transparency, so the alpha channel *is* the coverage the composition arrived
    /// at and nothing has to be undone to read it.
    fn painted(
        mark: f32,
        clip: impl for<'a> FnOnce(&'a tiny_skia::Mask, &'a super::Scratch) -> super::Clip<'a>,
    ) -> Vec<u8> {
        let mut pixmap = tiny_skia::Pixmap::new(8, 4).expect("a pixmap");
        let region = region(2.25);
        let scratch = super::Scratch::default();
        let paint = tiny_skia::Paint {
            anti_alias: true,
            ..tiny_skia::Paint::default()
        };
        super::fill(
            &mut pixmap.as_mut(),
            &half_plane(mark),
            &paint,
            tiny_skia::FillRule::Winding,
            tiny_skia::Transform::identity(),
            clip(&region, &scratch),
        );
        pixmap
            .pixels()
            .iter()
            .take(8)
            .map(|pixel| pixel.alpha())
            .collect()
    }

    /// §8.5.4's closed form: "[t]he effective shape is the intersection of the object's
    /// intrinsic shape with the clipping path", and a set intersected with a set that contains
    /// it is itself. So a mark whose own boundary coincides with its clip's must be painted at
    /// the coverage it would have been painted at unclipped — at *every* pixel, the boundary's
    /// included.
    ///
    /// The placement is fractional deliberately: on an integer boundary every coverage is 0 or
    /// 255, where `min` and a product are the same function and this would pass against either.
    #[test]
    fn a_clip_that_contains_the_mark_leaves_its_coverage_alone() {
        let unclipped = painted(2.25, |_, _| super::Clip::Unclipped);
        let boundary = 2;
        assert!(
            (1..255).contains(&unclipped[boundary]),
            "the boundary column must be partly covered for this to discriminate: {unclipped:?}"
        );
        assert_eq!(
            painted(2.25, |mask, scratch| super::Clip::Region { mask, scratch }),
            unclipped,
            "a clip coincident with the mark's own edge must not lower its coverage"
        );
    }

    /// The same scene composed as a product, which is what the clause's *other* mechanism does
    /// and what this backend did everywhere before: §11.6.5's soft mask is a value and Table 142
    /// multiplies it into the object's alpha. Two coverages of `c` give `c²`, so the two
    /// compositions must part on this geometry — which is what makes the assertion above a test
    /// of the composition rather than of the scan converter.
    #[test]
    fn a_value_multiplies_where_a_region_intersects() {
        let region = painted(2.25, |mask, scratch| super::Clip::Region { mask, scratch });
        let value = painted(2.25, |mask, _| super::Clip::Value(mask));
        let boundary = 2;
        assert!(
            u32::from(region[boundary]) > u32::from(value[boundary]) + 32,
            "a region must admit far more of the boundary than a product does: \
             {region:?} against {value:?}"
        );
    }

    /// A clip whose values are all 0 or 255 under the mark is a set already, and there the
    /// product *is* the intersection — so the substitution declines and the ordinary draw
    /// stands, byte for byte.
    #[test]
    fn a_clip_on_the_pixel_grid_is_drawn_the_ordinary_way() {
        let mut pixmap = tiny_skia::Pixmap::new(8, 4).expect("a pixmap");
        let region = region(3.0);
        assert!(
            region.data().iter().all(|&v| v == 0 || v == u8::MAX),
            "a clip on the grid must be a set for this to be the case it is about"
        );
        let paint = tiny_skia::Paint {
            anti_alias: true,
            ..tiny_skia::Paint::default()
        };
        let mark = half_plane(2.25);
        super::fill(
            &mut pixmap.as_mut(),
            &mark,
            &paint,
            tiny_skia::FillRule::Winding,
            tiny_skia::Transform::identity(),
            super::Clip::Region {
                mask: &region,
                scratch: &super::Scratch::default(),
            },
        );
        let mut ordinary = tiny_skia::Pixmap::new(8, 4).expect("a pixmap");
        ordinary.as_mut().fill_path(
            &mark,
            &paint,
            tiny_skia::FillRule::Winding,
            tiny_skia::Transform::identity(),
            Some(&region),
        );
        assert_eq!(pixmap.data(), ordinary.data());
    }

    /// A triangle running from the page out to `reach`, which is what a damaged content
    /// stream states and what ADR 0269's two witnesses state.
    fn spike(reach: f32) -> tiny_skia::Path {
        let mut builder = tiny_skia::PathBuilder::new();
        builder.move_to(10.0, 10.0);
        builder.line_to(reach, reach * 200.0);
        builder.line_to(20.0, 10.0);
        builder.close();
        builder
            .finish()
            .expect("three lines and a close form a path")
    }

    #[test]
    fn a_path_inside_the_range_keeps_its_anti_aliasing() {
        assert!(expressible(
            &spike(4.0),
            tiny_skia::Transform::identity(),
            0.0
        ));
    }

    #[test]
    fn a_path_outside_the_range_loses_it() {
        assert!(!expressible(
            &spike(SUPERSAMPLED_LIMIT + 1.0),
            tiny_skia::Transform::identity(),
            0.0
        ));
    }

    /// The bound is on the *device* coordinates, so a transform that brings the geometry back
    /// inside the range is a path that keeps its anti-aliasing.
    #[test]
    fn the_transform_is_what_the_range_is_read_after() {
        let path = spike(1.0e6);
        let at = tiny_skia::Transform::identity();
        assert!(!expressible(&path, at, 0.0));
        assert!(expressible(
            &path,
            tiny_skia::Transform::from_scale(1.0e-6, 1.0e-8),
            0.0
        ));
    }

    /// tiny-skia 0.12.0 aborts the process on this geometry with anti-aliasing on, and
    /// returns without it. Both halves are asserted, because the second is what makes the
    /// substitution a remedy rather than a silence.
    #[test]
    fn the_witness_geometry_is_drawn_without_anti_aliasing() {
        let mut pixmap = tiny_skia::Pixmap::new(368, 542).expect("a page-sized pixmap");
        let path = spike(1.0e7);
        let at = tiny_skia::Transform::from_translate(163.0, 319.0).pre_scale(1.0, -1.0);
        assert!(!expressible(&path, at, 0.0));
        let paint = tiny_skia::Paint {
            anti_alias: true,
            ..tiny_skia::Paint::default()
        };
        super::fill(
            &mut pixmap.as_mut(),
            &path,
            &paint,
            tiny_skia::FillRule::Winding,
            at,
            super::Clip::Unclipped,
        );
        assert!(
            pixmap.data().iter().any(|&byte| byte != 0),
            "the visible part of the spike is still drawn"
        );
    }
}

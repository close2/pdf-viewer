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

/// The device rectangles a mark **is**, where `pdf_render::edge` says it is rectangles at all —
/// ISO 32000-2 §10.7.4 and §11.6.2.
///
/// A rectangle's coverage of a pixel is the product of two one-dimensional overlaps, exactly, at
/// every placement (`pdf_render::rectangle_coverage`), where the supersampled path converter
/// rounds an axis-aligned edge to a quarter. This is what says whether that closed form applies
/// and to which rectangles, and it is the shared crate's answer rather than this backend's: trap
/// 2's rule is that a decision either backend can make alone is a decision neither has made.
///
/// # Why [`Exact::Several`] carries an invariant rather than merely a list
///
/// §11.6.2 makes a path's subpaths portions of **one object** and forbids compositing portions
/// with one another. Drawing them one at a time composites nothing only while no device pixel
/// receives two of them, which is `pdf_render::share_a_device_pixel`'s question — so this variant
/// may be built only from rectangles that answer it `false`, and every consumer here is written on
/// that basis: [`mask_rectangle`] takes the larger of what it writes and what is there, which is
/// the covered area only where no pixel is written twice. Where two portions *do* share a pixel
/// the mark keeps the one supersampled conversion that accumulates the whole path, which honours
/// the clause already and only measures it to a quarter. ADR 0583.
#[derive(Debug, Default, Clone)]
pub(crate) enum Exact {
    /// The mark is not a rectangle, or nobody asked.
    #[default]
    Unknown,
    /// One rectangle, which is the common case and allocates nothing.
    One(tiny_skia::Rect),
    /// Several, whose device pixel footprints are pairwise disjoint — see the type's comment.
    Several(Vec<tiny_skia::Rect>),
}

impl Exact {
    /// The rectangles, in the path's own order. Empty for [`Exact::Unknown`].
    fn iter(&self) -> impl Iterator<Item = tiny_skia::Rect> + '_ {
        let (one, several) = match self {
            Self::Unknown => (None, [].as_slice()),
            Self::One(rect) => (Some(*rect), [].as_slice()),
            Self::Several(rects) => (None, rects.as_slice()),
        };
        one.into_iter().chain(several.iter().copied())
    }

    /// Whether `pdf_render::edge` answered at all for this mark.
    pub(crate) fn is_some(&self) -> bool {
        !matches!(self, Self::Unknown)
    }

    /// Whether the closed form applies here at all: rectangles were found and every one of them
    /// lies inside the range this converter's arithmetic reaches.
    fn usable(&self) -> bool {
        self.is_some() && self.iter().all(|rect| within(Some(rect)))
    }
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
/// §11.6.5 defines a soft mask's *values*, and §11.3.7.2 states what is done with them: the
/// mask shape `fₘ` is one of three inputs of which "[t]he three shape inputs shall be
/// multiplied together, producing an intermediate value called the source shape". A product
/// the standard asks for, in other words, where the clip is a set the standard intersects.
///
/// **§8.5.4 also states the order the two go in**, which is what lets a mask carrying both stay
/// apart rather than collapsing into a `Value`: the clipping path constrains the *object's* own
/// shape — "[t]he effective shape is the intersection of the object's intrinsic shape with the
/// clipping path" — and it is that effective shape which then enters §11.3.7.2's product. So
/// `fₛ = (fⱼ ∩ C) · fₘ` and not `fⱼ · (C · fₘ)`, and [`Both`](Clip::Both) is the variant that
/// keeps the two factors of the second bracket apart far enough to say so.
///
/// [`mask_intersect`] is the same distinction one step earlier, where two clips meet each other;
/// this one is where a clip meets the mark. ADR 0280 took the first, ADR 0355 the second for a
/// clip alone, and ADR 0363 the second for a clip beside a soft mask.
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
    /// A coverage that multiplies the mark's: §11.6.5's soft mask on its own, where there is
    /// no set to intersect and Table 136's `fₘ` is the whole of what masks the mark.
    Value(&'a tiny_skia::Mask),
    /// §10.7.4's set and §11.6.5's value together, the product kept beside the value it was
    /// made from so that the mark can meet each as what it is.
    Both {
        /// The clip and the soft mask multiplied — what an ordinary draw is handed, and what
        /// [`intersected`] takes as the upper bound `C · S` of the composition.
        product: &'a tiny_skia::Mask,
        /// The soft mask's own values, over exactly `product`'s rows.
        value: &'a [u8],
        /// Where the mark's own coverage is built before the three are composed.
        scratch: &'a Scratch,
    },
}

impl<'a> Clip<'a> {
    /// The mask itself, for the callers that hand one to `tiny-skia` unexamined.
    pub(crate) fn mask(self) -> Option<&'a tiny_skia::Mask> {
        match self {
            Clip::Unclipped => None,
            Clip::Region { mask, .. } | Clip::Value(mask) | Clip::Both { product: mask, .. } => {
                Some(mask)
            }
        }
    }

    /// The composition's three inputs, or `None` where there is no set to intersect with.
    ///
    /// [`Clip::Region`] is the case `S ≡ 1`: a clip with no soft mask beside it, where the
    /// value that multiplies the intersection is one everywhere and the arithmetic below
    /// reduces to `min(M, C)`.
    fn composable(self) -> Option<(&'a tiny_skia::Mask, Option<&'a [u8]>, &'a Scratch)> {
        match self {
            Clip::Unclipped | Clip::Value(_) => None,
            Clip::Region { mask, scratch } => Some((mask, None, scratch)),
            Clip::Both {
                product,
                value,
                scratch,
            } => Some((product, Some(value), scratch)),
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
    if let Some(inputs) = clip.composable()
        && intersected(
            pixmap,
            path,
            &paint,
            fill_rule,
            (at, &Exact::Unknown),
            inputs,
        )
    {
        return;
    }
    pixmap.fill_path(path, &paint, fill_rule, at, clip.mask());
}

/// [`fill`], for a mark `pdf_render::edge` says is axis-aligned rectangles on the device's own
/// grid — ISO 32000-2 §10.7.4 and §11.6.2.
///
/// `exact` is those rectangles, already in device space; `path` is the same shape in the space
/// `at` maps from, and is what every branch below that is not the rectangle scan converter falls
/// back to. The two are the same mark stated twice, which is what lets this decline without the
/// caller having to know it did.
///
/// # Several rectangles are one object, and that is why they are one call
///
/// A path's subpaths are portions of one graphics object, and §11.6.2 says "[p]ortions of an
/// object shall not be composited with one another". [`Exact::Several`] therefore carries the
/// invariant its own comment states — no two of its rectangles fall in one device pixel — so the
/// loop below composites each portion with the backdrop and none of them with each other, and the
/// result is the same mark the supersampled converter would have accumulated, measured exactly
/// instead of to a quarter. The fill rule stops mattering for the same reason the rectangles are
/// disjoint: every point lies in at most one of them, so `Winding` and `EvenOdd` select one set.
/// ADR 0583.
///
/// # Why a rectangle gets its own call, when `fill_path` would draw it
///
/// `tiny-skia`'s anti-aliased **path** scan converter supersamples four times per axis. An
/// axis-aligned edge looks the same to all four sub-rows, so it is measured four ways and answers
/// the same one: an edge's coverage comes out **rounded to a quarter of a pixel**, and to nothing
/// at all below an eighth. §10.7.4's third sentence is what that is on the wrong side of —
///
/// > The area covered by painted pixels shall always be at least as large as the area of the
/// > original shape.
///
/// — and the anti-aliasing departure §10.7.1's NOTE licenses does not reach it, because
/// "anti-aliasing gives the shape's area; coming out *under* it is a defect" is this tree's own
/// rule for telling the two apart (`doc/todo/_scan-conversion.md`).
///
/// `tiny_skia::PixmapMut::fill_rect` is the same library's **rectangle** scan converter, and it is
/// the clause's own arithmetic rather than a sampler: it walks the rectangle as one interior run,
/// four edges and four corners in 8.8 fixed point, giving each boundary pixel the product of its
/// two one-dimensional overlaps — which `pdf_render::edge` derives from §10.7.4's definition of a
/// pixel as `[i, i+1) × [j, j+1)`. So the nine pieces `doc/todo/11` item 7 priced are one call,
/// and the call is *cheaper* than the path it replaces rather than dearer: no supersampled
/// accumulation, no alpha-run buffer, and a `memset` for the interior. ADR 0476 has the
/// measurement.
///
/// # A composable clip goes the other way, and it has to
///
/// Where [`intersected`] applies, the mark's coverage is built into a buffer and met by `min`
/// rather than by a product (ADR 0355), so it cannot go through the library's rectangle call at
/// all. It is handed the rectangle instead — [`mask_fill`] writes the same closed form into that
/// buffer — and **the alternative is not "keep the quantum there", it is a broken clause**: a mark
/// painted at its exact area under a region still measured to a quarter differs from the same mark
/// unclipped by up to 26 levels of 255, where §10.7.4's own set identity `S ∩ C = S` says a clip
/// containing a mark takes nothing from it. `render-cpu/tests/clip_intersection.rs` measures
/// exactly that and is what caught it.
///
/// # The two things it does hand back
///
/// - **A mark whose anti-aliasing was withdrawn**, which is `fill`'s range rule: outside
///   [`SUPERSAMPLED_LIMIT`] the aliased converter is what draws, and the two round a boundary
///   pixel to opposite ends. Nothing about the range changes here.
/// - **Nothing else.** Every blend mode and every shader is safe, because `fill_rect_aa` and
///   `fill_path` hand their coverage to the *same* `RasterPipelineBlitter` — as alpha runs and as
///   one-pixel masks — so the composition each pixel receives is the same function of its
///   coverage. That is why this does not carry [`crate::carries_coverage_as_alpha`]'s condition,
///   which is about a construction that delivers coverage some other way.
pub(crate) fn fill_rectangles(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    (path, exact): (&tiny_skia::Path, &Exact),
    paint: &tiny_skia::Paint<'_>,
    at: tiny_skia::Transform,
    clip: Clip<'_>,
) {
    let mut paint = paint.clone();
    paint.anti_alias = keep_anti_alias(paint.anti_alias, exact.usable());
    if let Some(inputs) = clip.composable()
        && intersected(
            pixmap,
            path,
            &paint,
            tiny_skia::FillRule::Winding,
            (at, exact),
            inputs,
        )
    {
        return;
    }
    if !paint.anti_alias {
        pixmap.fill_path(path, &paint, tiny_skia::FillRule::Winding, at, clip.mask());
        return;
    }
    // The paint carries the transform the library would have applied to it. `fill_rect` reaches
    // its own scan converter only for an identity transform — with any other it builds a path and
    // goes back to the supersampled one — so the shape is stated on the device's grid and the
    // shader is moved there one call earlier, which is what [`intersected`] does for the same
    // reason. Trap 2 is what this costs to get wrong.
    paint.shader.transform(at);
    for rect in exact.iter() {
        pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), clip.mask());
    }
}

/// Draws `path` with its own coverage meeting `admitted` by `min`, ISO 32000-2 §10.7.4.
///
/// `admitted` is `C · S` — the clipping region times whatever soft mask stands beside it — and
/// `value` is that `S` alone, or `None` where there is none and `S ≡ 1`. What is composed is
/// §8.5.4's effective shape times §11.3.7.2's mask shape.
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
/// # The closed form, and why the soft mask needs no third buffer
///
/// §8.5.4 puts the clip inside the object's shape and §11.3.7.2 puts the soft mask outside it,
/// so what is wanted is `min(M, C) · S` where `M` is the mark's own coverage. Multiplication by
/// a non-negative value is monotone, so it distributes over a minimum:
///
/// ```text
///   min(M, C) · S  =  min(M · S, C · S)  =  min(M · S, P)
/// ```
///
/// and `P` is the product this cache already holds for every other draw. So the composition
/// needs the soft mask's own rows and the product, never the clip's region by itself — and the
/// rounding costs nothing either, because rounding is monotone too and `min` therefore commutes
/// with it: `min(round(M·S), round(C·S))` is `round(min(M·S, C·S))` exactly.
///
/// # The three things it declines, each because the substitution would say something else
///
/// - **A clip that is already a set** — `P` either 0 or `S` at every pixel under the mark,
///   which for `S ≡ 1` is the plain "0 or 255". There the product *is* the intersection, pixel
///   for pixel, so the ordinary draw already carries the clause out and the cheaper path is
///   also the correct one. This is what keeps the cost off the pages that do not need it.
///
///   **With a soft mask the test is inexact in one direction, by less than half a level**, and
///   that is a bound rather than an observation. `P = round(C·S/255)` equals `S` for any
///   `C ≥ 255 − 127/S`, so a faint mask can read as a set where the clip is not one; but the
///   two compositions differ there by `S·(min(M,C) − M·C/255)/255`, which that same inequality
///   holds under `½` for every `M`. Half a level of 255 is below what an eight-bit raster can
///   hold, so declining costs the page nothing it could show.
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
    (at, exact): (tiny_skia::Transform, &Exact),
    (admitted, value, scratch): (&tiny_skia::Mask, Option<&[u8]>, &Scratch),
) -> bool {
    if !crate::carries_coverage_as_alpha(paint.anti_alias, paint.blend_mode) {
        return false;
    }
    // `tiny-skia` draws nothing at all through a mask of another size, so a mismatch is left to
    // the ordinary call, which answers it the same way it does today.
    if admitted.width() != pixmap.width() || admitted.height() != pixmap.height() {
        return false;
    }
    // The soft mask's rows are laid out over the product's, so a length that disagrees is a
    // pairing this cannot make; declining draws the mark the way it was drawn before.
    if value.is_some_and(|value| value.len() != admitted.data().len()) {
        return false;
    }
    let Some(reach) = reached_pixels(path, at, pixmap.width(), pixmap.height()) else {
        return false;
    };
    let Some(rect) = reach.rect() else {
        return false;
    };
    if is_a_set(admitted, value, reach, pixmap.width()) {
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
    mask_fill(coverage, path, fill_rule, paint.anti_alias, (at, exact));
    let bound = admitted.data();
    let mark = coverage.data_mut();
    for row in reach.rows() {
        let (from, until) = reach.span(row, stride);
        let (Some(mark), Some(bound)) = (mark.get_mut(from..until), bound.get(from..until)) else {
            continue;
        };
        let value = match value {
            None => None,
            // Unreachable: `value` is `admitted`'s own length, checked above, and this span
            // was just taken from that. Skipping the row rather than composing without the
            // value keeps the mark from being painted at more than the mask admits.
            Some(value) => match value.get(from..until) {
                Some(row) => Some(row),
                None => continue,
            },
        };
        match value {
            None => {
                for (mark, &bound) in mark.iter_mut().zip(bound) {
                    *mark = (*mark).min(bound);
                }
            }
            Some(value) => {
                for ((mark, &bound), &value) in mark.iter_mut().zip(bound).zip(value) {
                    *mark = scaled(*mark, value).min(bound);
                }
            }
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
    /// Every pixel of a buffer this wide and this tall.
    ///
    /// A mark reaches the pixels its path covers; a group's raster covers the band it was
    /// accumulated over, so [`intersect_group`] asks for that band whole. `None` for an empty
    /// one, which has nothing to compose.
    fn whole(width: u32, height: u32) -> Option<Self> {
        (width > 0 && height > 0).then_some(Self {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        })
    }

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

/// Two eight-bit coverages multiplied, rounded.
///
/// **One function rather than two copies, because the agreement is load-bearing.**
/// `MaskCache::combine` builds the clip × soft-mask product with this, and [`intersected`]
/// scales the mark's own coverage by the same soft mask before taking the minimum of the two.
/// A minimum commutes with a monotone rounding, so `min(round(M·S), round(C·S))` is
/// `round(min(M, C)·S)` exactly — and only while both sides round the same way. ADR 0363.
pub(crate) fn scaled(coverage: u8, value: u8) -> u8 {
    // 255 × 255 = 65 025 fits a `u16`, and the rounded quotient keeps a fully open pair open.
    let scaled = u16::from(coverage)
        .saturating_mul(u16::from(value))
        .saturating_add(127);
    u8::try_from(scaled / 255).unwrap_or(u8::MAX)
}

/// Whether the clip inside `admitted` is a set rather than a coverage, over `reach`.
///
/// `admitted` is `C · S` and `value` is `S`, so the clip is 0 or 1 exactly where the product is
/// 0 or `S`. With no soft mask beside it — `value` of `None`, `S ≡ 1` — that is the plain test
/// against 0 and 255 the clip's own mask answers.
///
/// The direction it is loose in, and its bound, are [`intersected`]'s first decline.
fn is_a_set(admitted: &tiny_skia::Mask, value: Option<&[u8]>, reach: Reach, stride: u32) -> bool {
    let data = admitted.data();
    reach.rows().all(|row| {
        let (from, until) = reach.span(row, stride as usize);
        let value = value.and_then(|value| value.get(from..until));
        data.get(from..until).is_none_or(|row| match value {
            None => row.iter().all(|&value| value == 0 || value == u8::MAX),
            Some(value) => row
                .iter()
                .zip(value)
                .all(|(&product, &value)| product == 0 || product == value),
        })
    })
}

/// §8.5.4's intersection of a **group's** shape with the clip at its blit, written into the
/// group's own raster (ISO 32000-2 §8.5.4, §10.7.4, §11.4.4).
///
/// [`intersected`] is this same composition for a mark whose coverage the caller rasterises;
/// this is it for a group, whose coverage has already been accumulated into `band` and needs no
/// second scan converter. §8.5.4 states the group case in its own sentence:
///
/// > Similarly, the shape of a transparency group (defined as the union of the shapes of its
/// > constituent objects) shall be influenced both by the clipping path in effect when each of
/// > the objects is painted
///
/// — and, the sentence goes on, by the one in effect at the time the group's results are
/// painted onto its backdrop. (The quotation stops where it does because the extraction in
/// `doc/md/` breaks that last word across a space, so the rest cannot be quoted verbatim.)
///
/// # What it may do this to, and what makes it exact
///
/// Table 139 returns a group's shape `f` and its alpha `α` separately and `band` holds one
/// number per pixel. The caller passes `alpha_is_shape` only for a group whose opacity is 1.0
/// everywhere (`pdf_render::Command::Group`'s field), where §11.3.7.1's `α = f × q` makes the
/// stored alpha the shape itself — so `min(α, C)` is the clause's intersection rather than an
/// estimate of it. Where the flag is false the group's `f` is not in this buffer at all and the
/// caller draws through the mask as before.
///
/// The soft mask beside the clip needs no third buffer, for [`intersected`]'s reason and by the
/// same identity: `min(f, C) · S = min(f · S, C · S) = min(f · S, P)`, with `P` the product the
/// cache already holds, and `min` commutes with the rounding because rounding is monotone.
///
/// # Why the colours are rescaled rather than the mask handed on
///
/// `band` is premultiplied, so lowering a pixel's alpha without lowering its colour would state
/// a colour brighter than its own alpha admits. Each channel is therefore scaled by `α′ / α`,
/// which is the group's *unpremultiplied* colour left exactly where it was — §11.4.4 changes a
/// group's shape here and not its colour. `α′ ≤ α` always, so nothing can overflow, and where
/// `α` is zero so is `α′` and the pixel is left alone.
///
/// Returns `false` where it declined, which leaves the caller's ordinary masked blit to run.
/// The one decline is [`intersected`]'s first and it is there for the same reason: where the
/// clip is already a set of pixels the product *is* the intersection, so the cheaper path is
/// also the correct one, and that is what keeps this pass off the pages that do not need it.
pub(crate) fn intersect_group(band: &mut [u8], clip: Clip<'_>) -> bool {
    let Some((admitted, value, _)) = clip.composable() else {
        return false;
    };
    let stride = admitted.width();
    let pixels = admitted.data().len();
    // A band whose bytes are not four per mask sample is a pairing this cannot make; declining
    // composites the group the way it was composited before.
    if band.len() != pixels.saturating_mul(4) {
        return false;
    }
    if value.is_some_and(|value| value.len() != pixels) {
        return false;
    }
    let Some(reach) = Reach::whole(stride, admitted.height()) else {
        return false;
    };
    if is_a_set(admitted, value, reach, stride) {
        return false;
    }
    for (index, sample) in band.chunks_exact_mut(4).enumerate() {
        let (Some(&bound), Some(&alpha)) = (admitted.data().get(index), sample.get(3)) else {
            continue;
        };
        if alpha == 0 {
            continue;
        }
        let soft = value.map_or(u8::MAX, |value| value.get(index).copied().unwrap_or(0));
        let composed = scaled(alpha, soft).min(bound);
        if composed == alpha {
            continue;
        }
        for channel in sample {
            // `channel ≤ alpha` holds for every premultiplied sample and `composed ≤ alpha`,
            // so the quotient is at most `composed` and the clamp is belt and braces rather
            // than arithmetic. Rounded, because the caller's blit rounds and a colour biased
            // down at every group boundary is what ADR 0418 measured the cost of.
            //
            // The divisor cannot be zero — `alpha == 0` was skipped above — and `checked_div`
            // rather than `/` because a division that cannot fail still has to say so.
            let scaled = u32::from(*channel)
                .saturating_mul(u32::from(composed))
                .saturating_add(u32::from(alpha) / 2)
                .checked_div(u32::from(alpha))
                .unwrap_or(0);
            *channel = u8::try_from(scaled).unwrap_or(u8::MAX).min(composed);
        }
    }
    true
}

/// [`tiny_skia::PixmapMut::stroke_path`], with the range applied to `paint.anti_alias`.
///
/// The outset is a whole width times the miter limit rather than the half-width the stroke
/// actually reaches: a miter join extends past the outline by the limit, and over-estimating
/// here costs a path within a hair of the bound its anti-aliasing and nothing else.
///
/// **A [`Clip::Region`] still meets this mark by a product, and what is left here is the residue
/// rather than the case.** Since ADR 0535 a stroke above the coverage quantum is drawn as the fill
/// of its own outline (`render_cpu::draw_stroked_outline`) and composes through [`fill`], and
/// §10.7.4's substitutions on a sub-pixel rule were fills already. What still arrives here is a
/// stroke at or under the quantum for which every one of those substitutions declined:
/// `crate::carries_coverage_as_alpha` answering `false`, which is [`intersected`]'s own first
/// decline as well and so loses no composition; a transform with no thinnest line; or a path the
/// stroker or the dasher refused, which draws nothing here either. `doc/todo/11` item 4 carries
/// what is left of the item.
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
///
/// `exact` is the device rectangle `pdf_render::device_rectangle` says this path is, where it says
/// so, and it is the same substitution [`fill_rectangle`] makes for a mark: §10.7.4 says the
/// clipping region "consists of the set of pixels that would be included by a fill operation", so a
/// rectangular clip is scan-converted by the rule a rectangular fill is. **The two have to agree or
/// the identity `S ∩ C = S` breaks**: a mark painted at its exact area under a region measured to a
/// quarter is drawn at the quarter, which is 26 levels of 255 at a boundary pixel and is what
/// `render-cpu/tests/clip_intersection.rs` measures. ADR 0476.
pub(crate) fn mask_fill(
    mask: &mut tiny_skia::Mask,
    path: &tiny_skia::Path,
    fill_rule: tiny_skia::FillRule,
    anti_alias: bool,
    (at, exact): (tiny_skia::Transform, &Exact),
) {
    let anti_alias = keep_anti_alias(anti_alias, expressible(path, at, 0.0));
    if anti_alias && exact.usable() {
        for rect in exact.iter() {
            mask_rectangle(mask, rect);
        }
        return;
    }
    mask.fill_path(path, fill_rule, anti_alias, at);
}

/// Writes an axis-aligned rectangle's own coverage into `mask` — ISO 32000-2 §10.7.4.
///
/// The same nine pieces [`fill_rectangle`] hands to `tiny-skia`'s rectangle scan converter — one
/// interior run, four edges, four corners — written out here because a `Mask` has no rectangle
/// entry point of its own, only `fill_path`. `pdf_render::rectangle_coverage` is the arithmetic in
/// both places, so the mark and the region are measured by one rule.
///
/// A positive coverage under one level of 255 is stated *at* one level, which is
/// `pdf_render::expressible_coverage` and ADR 0419's reading of "no shape ever disappears": a
/// region that admits a sliver of a pixel must not be a region that admits nothing there.
///
/// **It takes the larger of what is there and what it writes**, which keeps
/// [`tiny_skia::Mask::fill_path`]'s "draws on top of existing data" contract in the only direction
/// that matters. Every caller in this crate fills a region that is clear — a fresh mask, a cleared
/// scratch, or the rows [`intersected`] has just zeroed — so the two are the same value there.
///
/// **The interior is a run rather than a loop of pixels**, and that is the measurement rather than
/// a preference: a page-wide rectangular clip is the commonest clip in the corpus, and asking
/// `pdf_render::rectangle_coverage` per pixel over one costs `colors.pdf`'s page **+33% of its
/// whole rasterisation**, where filling the run costs nothing measurable. The three calls a row
/// makes are its two boundary columns and one interior column, whose answer is the row's own
/// overlap because an interior column's is 1.
fn mask_rectangle(mask: &mut tiny_skia::Mask, rect: tiny_skia::Rect) {
    let (width, height) = (mask.width(), mask.height());
    let stride = width as usize;
    let (left, right) = (
        clamped(rect.left(), width),
        clamped(rect.right().ceil(), width),
    );
    let (top, bottom) = (
        clamped(rect.top(), height),
        clamped(rect.bottom().ceil(), height),
    );
    if left >= right || top >= bottom {
        return;
    }
    let area = pdf_render::Rect::from_corners(
        pdf_render::Point::new(rect.left(), rect.top()),
        pdf_render::Point::new(rect.right(), rect.bottom()),
    );
    // The columns whose overlap is a whole pixel, which is where the run goes. Both boundary
    // columns are outside it, whether or not their own overlap happens to be whole.
    let inner =
        clamped(rect.left().ceil(), width) as usize..clamped(rect.right().floor(), width) as usize;
    let (left, right) = (left as usize, right as usize);
    let (top, bottom) = (top as usize, bottom as usize);
    for (row, scanline) in mask
        .data_mut()
        .chunks_exact_mut(stride)
        .enumerate()
        .skip(top)
        .take(bottom.saturating_sub(top))
    {
        let mut boundaries = [None, None];
        if inner.start > left {
            boundaries[0] = Some(left);
        }
        if inner.end < right {
            boundaries[1] = Some(right.saturating_sub(1));
        }
        for column in boundaries.into_iter().flatten() {
            let coverage = pdf_render::rectangle_coverage(area, f32_of(column), f32_of(row));
            if coverage > 0.0
                && let Some(byte) = scanline.get_mut(column)
            {
                *byte = (*byte).max(level_of(pdf_render::expressible_coverage(coverage)));
            }
        }
        if inner.is_empty() {
            continue;
        }
        let down = pdf_render::rectangle_coverage(area, f32_of(inner.start), f32_of(row));
        if down <= 0.0 {
            continue;
        }
        let level = level_of(pdf_render::expressible_coverage(down));
        if let Some(run) = scanline.get_mut(inner.start..inner.end) {
            for byte in run {
                *byte = (*byte).max(level);
            }
        }
    }
}

/// A pixel index as the coordinate of its own lower corner, which is what §10.7.4's `i` and `j`
/// are. Exact for every raster this backend can allocate.
#[expect(
    clippy::cast_precision_loss,
    reason = "a raster's own pixel index, bounded by `MAX_EXTENT` and far inside f32's integers"
)]
fn f32_of(index: usize) -> f32 {
    index as f32
}

/// A coverage in `0.0..=1.0` as the level an eight-bit mask holds, rounded to nearest.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is a coverage clamped to 0..=1 multiplied by 255, so the cast is in range \
              by construction"
)]
fn level_of(coverage: f32) -> u8 {
    (coverage.clamp(0.0, 1.0) * 255.0).round() as u8
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
    (at, exact): (tiny_skia::Transform, &Exact),
) {
    scratch.clear();
    mask_fill(scratch, path, fill_rule, anti_alias, (at, exact));
    for (kept, &added) in mask.data_mut().iter_mut().zip(scratch.data()) {
        *kept = (*kept).min(added);
    }
}

#[cfg(test)]
mod tests {
    use super::{Exact, SUPERSAMPLED_LIMIT, expressible};

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
            (tiny_skia::Transform::identity(), &Exact::Unknown),
        );
        for edge in nested {
            super::mask_intersect(
                &mut mask,
                &mut scratch,
                &half_plane(*edge),
                tiny_skia::FillRule::Winding,
                true,
                (tiny_skia::Transform::identity(), &Exact::Unknown),
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
            (tiny_skia::Transform::identity(), &Exact::Unknown),
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

    /// The mask a soft mask of constant value states, over the whole 8×4 raster.
    ///
    /// Constant because §11.6.5.1 gives a soft mask a value everywhere, and a mask over an
    /// empty group is that constant — the one soft mask whose every pixel can be written down.
    fn flat(value: u8) -> tiny_skia::Mask {
        let mut mask = tiny_skia::Mask::new(8, 4).expect("a mask");
        mask.data_mut().fill(value);
        mask
    }

    /// A clip at `x` folded into a soft mask of constant `value`, the way `MaskCache::combine`
    /// folds them: the product for every draw, and the soft mask's own rows beside it.
    fn folded(x: f32, value: u8) -> (tiny_skia::Mask, Vec<u8>) {
        let mut product = region(x);
        for pixel in product.data_mut() {
            *pixel = super::scaled(*pixel, value);
        }
        let values = vec![value; 8 * 4];
        (product, values)
    }

    /// Fills the half-plane at `mark` through `clip`, and returns the first row's alphas.
    ///
    /// Black onto transparency, so the alpha channel *is* the coverage the composition
    /// arrived at, exactly as in [`painted`].
    fn filled(mark: f32, clip: super::Clip<'_>) -> Vec<u8> {
        let mut pixmap = tiny_skia::Pixmap::new(8, 4).expect("a pixmap");
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
            clip,
        );
        pixmap
            .pixels()
            .iter()
            .take(8)
            .map(|pixel| pixel.alpha())
            .collect()
    }

    /// The same identity with §11.6.5's value standing beside §10.7.4's set.
    ///
    /// §8.5.4 intersects the clipping path with the object's *intrinsic* shape, and
    /// §11.3.7.2 multiplies the mask shape into what comes out — so `min(M, C) · S`, and with
    /// `M ⊆ C` that is `M · S`, which is the mark under the soft mask and no clip at all. The
    /// clip must therefore take nothing, exactly as it takes nothing when it is alone.
    ///
    /// The boundary column of this scene, a half-plane at device 2.25 under a coincident clip
    /// and a soft mask of 128 of 255:
    ///
    /// ```text
    ///   the mark's own coverage, unmasked and unclipped   192
    ///   the mark under the soft mask alone                 96   = round(192 × 128 / 255)
    ///   the product taken as a value, which was drawn      72   = round(192 ×  96 / 255)
    ///   min(M · S, C · S), which is drawn now              96
    /// ```
    #[test]
    fn a_clip_folded_into_a_soft_mask_still_takes_nothing_from_the_mark() {
        let soft = 128;
        let unclipped = filled(2.25, super::Clip::Value(&flat(soft)));
        let boundary = 2;
        assert!(
            (1..255).contains(&unclipped[boundary]),
            "the boundary column must be partly covered for this to discriminate: {unclipped:?}"
        );
        let (product, values) = folded(2.25, soft);
        let scratch = super::Scratch::default();
        let clipped = filled(
            2.25,
            super::Clip::Both {
                product: &product,
                value: &values,
                scratch: &scratch,
            },
        );
        for (index, (&got, &want)) in clipped.iter().zip(unclipped.iter()).enumerate() {
            assert!(
                got.abs_diff(want) <= 1,
                "cell {index}: a coincident clip moved the soft-masked mark, \
                 {clipped:?} against {unclipped:?}"
            );
        }
    }

    /// The same scene with the product taken as a plain value — the composition this replaces
    /// — which must part from it by far more than the level the assertion above allows.
    ///
    /// Without this the test above would pass against a construction that never looked at the
    /// clip, since the two agree everywhere except in the boundary column.
    #[test]
    fn folding_the_clip_into_the_value_would_square_the_boundary() {
        let soft = 128;
        let (product, values) = folded(2.25, soft);
        let squared = filled(2.25, super::Clip::Value(&product));
        let scratch = super::Scratch::default();
        let composed = filled(
            2.25,
            super::Clip::Both {
                product: &product,
                value: &values,
                scratch: &scratch,
            },
        );
        let boundary = 2;
        assert!(
            u32::from(composed[boundary]) > u32::from(squared[boundary]) + 16,
            "the intersection must admit far more of the boundary than the product does: \
             {composed:?} against {squared:?}"
        );
    }

    /// The same identity where the clip **contains** the mark instead of coinciding with it,
    /// which is the axis every coincident scene leaves at its default.
    ///
    /// **A coincident clip cannot see whether the mark was scaled by the value at all**, and
    /// that is arithmetic rather than luck: where `C = M` the wrong composition `min(M, C·S)`
    /// is `min(M, M·S)`, which is `M·S` because `S ≤ 1` — the right answer, by coincidence of
    /// the scene rather than of the code. Every other scene here puts the clip's edge on the
    /// mark's, so all of them pass against a composition that never multiplies the value in.
    /// This one offsets the two by half a pixel inside one column, where the mark's coverage
    /// falls *below* the product and the two part.
    ///
    /// The mark at device 2.75 under a clip at 2.25 and a soft mask of 128 of 255, column 2:
    ///
    /// ```text
    ///   the mark's own coverage                             64 of 255
    ///   the clip's                                         192
    ///   the product `C · S`, which bounds the composition   96
    ///   min(M · S, C · S) = M · S, which is drawn           32   — the mark lies inside the clip
    ///   min(M, C · S) — the value never applied to the mark 64
    /// ```
    #[test]
    fn a_clip_that_contains_the_mark_takes_nothing_from_it_under_a_soft_mask() {
        let soft = 128;
        let alone = filled(2.75, super::Clip::Value(&flat(soft)));
        let boundary = 2;
        assert!(
            (1..255).contains(&alone[boundary]),
            "the boundary column must be partly covered for this to discriminate: {alone:?}"
        );
        let (product, values) = folded(2.25, soft);
        assert!(
            product.data()[boundary] != 0 && product.data()[boundary] != soft,
            "the clip must be fractional under the mark, or the composition declines"
        );
        let scratch = super::Scratch::default();
        let contained = filled(
            2.75,
            super::Clip::Both {
                product: &product,
                value: &values,
                scratch: &scratch,
            },
        );
        for (index, (&got, &want)) in contained.iter().zip(alone.iter()).enumerate() {
            assert!(
                got.abs_diff(want) <= 1,
                "cell {index}: a clip containing the mark moved the soft-masked mark, \
                 {contained:?} against {alone:?}"
            );
        }
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

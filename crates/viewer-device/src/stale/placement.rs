//! Whether the picture held depicts the view asked for, and the one placement that carries it there
//! or the refusal that says it cannot (ADR 0442).

use pdf_render::{TargetSpec, Transform};

use super::Placed;
use super::plan::Refusal;

/// Whether the picture held already depicts the arrangement being asked for.
///
/// The same pages, in the same order, at the same placements — all three, because Table 29's
/// arrangement is a *set* of placed pages and a picture that is right about two of three is a
/// window with a hole in it. Under `SinglePage` this is the one comparison it has always been.
pub(super) fn depicts(settled: &[Placed], asked: &[Placed]) -> bool {
    settled.len() == asked.len() && settled.iter().zip(asked).all(|(was, now)| was.is(now))
}

/// How far apart two placements put the picture held, in device pixels, at its worst point.
///
/// The picture is a rectangle of texels — the window's own, since the base is a texture of the
/// window — and the two placements are affine, so their difference is affine too: a point at
/// `p` is put at `first(p)` by one and `then(p)` by the other, and the distance between those is
/// `|Δ(p)|` for `Δ = then − first`, which is a **convex** function of `p`. A convex function on a
/// convex polygon attains its maximum at a vertex, so four points answer this exactly and there is
/// no norm to bound and no estimate to be conservative about.
///
/// Both placements map the *same* texels — the held texture's — so the rectangle is that texture's
/// extent, which is the asked target's: [`Stale::carries`] and [`Stale::reproject`] both refuse a
/// window that changed shape before reaching here ([`Refusal::Resized`]).
///
/// [`Stale::carries`]: super::Stale::carries
/// [`Stale::reproject`]: super::Stale::reproject
fn disagreement(first: Transform, then: Transform, over: &TargetSpec) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a raster dimension, capped at pdf_render::MAX_EXTENT = 2^24, which f32 \
                  represents exactly"
    )]
    let (width, height) = (over.width as f32, over.height as f32);
    [(0.0, 0.0), (width, 0.0), (0.0, height), (width, height)]
        .into_iter()
        .map(|(x, y)| {
            let corner = pdf_render::Point::new(x, y);
            let (one, other) = (first.apply(corner), then.apply(corner));
            (one.x - other.x).hypot(one.y - other.y)
        })
        .fold(0.0_f32, f32::max)
}

/// How far two pages' placements may disagree and still be one placement: **half a device pixel**.
///
/// # Where the number comes from, which is the geometry and not a corpus
///
/// A device pixel is a sample of unit spacing, and half of one is the largest distance a point can
/// move without leaving the pixel it is a sample of. So two placements whose worst disagreement
/// over the whole picture is under half a pixel put every feature of that picture **in the same
/// pixel**; the grid this frame is drawn on cannot state them apart. It is the same argument
/// `TargetSpec::for_page` already rounds a page's extent by (ADR 0064) and it is stated in the
/// display's own unit, which is the discipline rules 4 and 5 were re-grounded on in ADR 0384.
///
/// **What it claims is exactly that and no more.** It does not claim the two resamples are
/// bit-identical: a filtered sample is a continuous function of the placement, so a sub-pixel
/// shift moves its weights by a fraction of one. It claims that the choice between them moves
/// nothing into a different pixel — on a picture that is a deliberate approximation to begin with,
/// where the *other* error present is a whole-texture resample.
///
/// # Why an exact comparison was wrong, which is arithmetic rather than arrangement
///
/// ADR 0442 compared the compositions with `==`, on the argument that a threshold would be a number
/// nobody measured a purpose for. On a real continuous column that refused ordinary **scrolls**:
/// every page moves by the same distance and the composition still goes through
/// `Transform::invert`, whose division by the determinant is not exact in `f32`, so two placements
/// that print identically to three decimals are not equal. A measured trace has the pair — `(1.000
/// 0.000 0.000 1.000 0.000 -371.000)` refused against `(1.000 0.000 0.000 1.000 0.000 -371.000)` —
/// and the cost of `==` is the sharp layer refused for most view changes in a continuous layout.
///
/// # What the residual measures, and why this is not a tolerance fitted to it
///
/// The bound was derived first and the measurement taken second, which is the order
/// `doc/traps/pixels-and-rasterisers.md` trap 12 is about. `doc/PDF20_AN001-BPC.pdf` in
/// `OneColumn` in an 800×1000 window, through fifteen scrolls of 137 px and five zoom steps —
/// the arrangement `viewer_core::layout` states rather than one composed here, which is
/// `tests::a_scroll_of_a_real_column_is_carried_and_a_zoom_of_one_is_still_refused`:
///
/// | view change | of a picture of | disagreement | against this bound |
/// |---|---|---|---|
/// | 15 scrolls | 2–3 pages | 0 – 0.000183 px | 2 700× under it |
/// | 5 zoom steps | 2–3 pages | 1.25 – 2.75 px | 2.5× to 5.5× over it |
///
/// The two populations are **four orders of magnitude apart** and the bound sits between them
/// touching neither. That separation is the finding rather than the threshold: the residual is
/// `f32` round-off in an inverse, three of the fifteen scrolls carrying it and twelve exactly
/// zero, and a zoom's disagreement is `GAP × (1 − k)` per gap crossed — a real rearrangement of
/// pages, in units of the gap `viewer_core::layout` states in logical pixels. A later measurement
/// finding a residual near this bound would be a defect in the composition to fix and not a reason
/// to raise it, which is the shape trap 12 warns about read the right way round.
pub(super) const AGREEMENT: f32 = 0.5;

/// The one affine that carries every page of the settled picture onto the view being asked for.
///
/// **This is what replaced a single page's `settled⁻¹ ∘ asked` when a column arrived**. The window
/// presents its pages as *one* texture under *one* placement, so a reprojection is defensible only
/// where one affine is true of every page in the picture. For each page in both the settled
/// arrangement and the one being asked for — matched by [`Picture`], what the page's picture is
/// *of*, and not by the `Arc`'s address — this composes that page's own `settled⁻¹ ∘ asked`, and
/// answers only where they all agree to within [`AGREEMENT`], which is where they all put the
/// picture in the same pixels.
///
/// The match used to be `Arc::ptr_eq` on the list, and that was the same defect ADR 0457 removed
/// from the retained pages, at the other gate: §12.5.3 makes a `NoZoom` page re-interpret on
/// every magnification change, a re-interpretation is a new `Arc` over the same ink, and pairing
/// by address turned every zoom of such a page into [`Refusal::AnotherPage`] — no stand-in, a
/// window frozen for the length of the real frame. [`Picture`] carries the ink, so what the
/// address test actually guarded — a picture of superseded ink standing in — still refuses here,
/// as the page turn it amounts to.
///
/// The three ways it refuses are three different facts, and the caller tells them apart:
///
/// - **no page is in both**, which is a page turn — nothing about the outgoing pages' pixels is
///   true of the incoming ones, at any placement;
/// - **a placement does not invert**, or the composition is not finite;
/// - **the pages disagree**, which is the case a column introduced. A *scroll* moves every page by
///   the same distance, so what separates its placements is the arithmetic of an inverse in `f32`
///   and a column reprojects as a single page always did. A *zoom* does not:
///   `viewer_core::layout`'s gap between rows is stated in logical pixels and so does not scale
///   with the magnification, while the pages either side of it do — so no one affine carries both,
///   and moving the picture whole would put a page where it is not. A surface that draws in quads
///   carries it a page at a time instead ([`carrying_on_quads`], ADR 2416); one that resamples a
///   window of pixels waits for the real frame.
///
/// The disagreement it did absorb comes back with the answer, because a bound nobody can see the
/// far side of is a bound nobody can re-measure ([`Carried::within`]).
///
/// [`Picture`]: super::Picture
fn one_placement(settled: &[Placed], asked: &[Placed]) -> Result<Carried, Refusal> {
    let mut agreed: Option<Transform> = None;
    let mut within = 0.0_f32;
    for placed in asked {
        let Some(was) = settled
            .iter()
            .find(|settled| settled.of == placed.of)
            .map(|settled| settled.target)
        else {
            continue;
        };
        let target = &placed.target;
        let moved = moved(&was, target)?;
        match agreed {
            // Every page is compared against the placement that will actually be used rather than
            // against its neighbour, so `within` is a statement about the picture drawn: no page
            // of it is further than that from where this view puts it.
            None => agreed = Some(moved),
            Some(first) => {
                let apart = disagreement(first, moved, target);
                if apart > AGREEMENT {
                    return Err(Refusal::Rearranged {
                        first,
                        then: moved,
                        by: apart,
                    });
                }
                within = within.max(apart);
            }
        }
    }
    agreed
        .map(|placement| Carried {
            placement: snapped(placement),
            within,
        })
        .ok_or(Refusal::AnotherPage)
}

/// The placement that carries one page's picture, drawn at `was`, onto `now`: `was⁻¹ ∘ now`, in
/// the picture's own texel space.
///
/// Asked once a page by both answers — [`one_placement`], which needs every page's to agree, and
/// [`page_by_page`], which puts each page up under its own.
fn moved(was: &TargetSpec, now: &TargetSpec) -> Result<Transform, Refusal> {
    // A scroll changes only the translation, and the linear parts say so by bit
    // equality — so the placement is *constructed* as the exact translation
    // `t2 − t1` rather than computed through an inverse whose round-trip would
    // leave the scale at 0.99999994 and the whole window resampled for it. This
    // is what lets the snap recognise a scroll with no tolerance anywhere.
    let same_linear = [
        (was.transform.a, now.transform.a),
        (was.transform.b, now.transform.b),
        (was.transform.c, now.transform.c),
        (was.transform.d, now.transform.d),
    ]
    .iter()
    .all(|(then, now)| then.to_bits() == now.to_bits());
    let moved = if same_linear {
        Some(Transform {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: now.transform.e - was.transform.e,
            f: now.transform.f - was.transform.f,
        })
    } else {
        was.transform.invert().map(|back| back.then(now.transform))
    };
    let Some(moved) = moved else {
        return Err(Refusal::NoPlacement);
    };
    if ![moved.a, moved.b, moved.c, moved.d, moved.e, moved.f]
        .iter()
        .all(|coefficient| coefficient.is_finite())
    {
        return Err(Refusal::NoPlacement);
    }
    Ok(moved)
}

/// Says how far the pages of the picture disagreed about the placement it went up at.
///
/// **Rule 3 over `AGREEMENT` rather than over the picture**, and it is what makes the bound a
/// thing a run measures instead of a thing this file asserts: a person driving a column sees what
/// an ordinary scroll's residual is and what a zoom's rearrangement is, in the same unit, on their
/// own document. Silent for a single page, which has nothing to disagree with, and so silent for
/// every `SinglePage` frame this program has ever drawn.
///
/// A free function rather than a method on [`Stale`]: it reads nothing that has been decided and
/// counts nothing, which is the difference between it and [`Stale::declined`].
///
/// [`Stale`]: super::Stale
/// [`Stale::declined`]: super::Stale::declined
pub fn absorbed(carried: Carried, pages: usize, trace: impl crate::FrameLines) {
    if pages < 2 {
        return;
    }
    trace.say(format_args!(
        "one placement for {pages} pages: they disagree by {:.4} px at the worst corner, \
             against a bound of {AGREEMENT} — half a device pixel, which moves nothing into \
             another pixel",
        carried.within
    ));
}

/// Where the picture held goes so that it depicts the view being asked for, and how far the pages
/// of it disagreed about that.
///
/// The second field is what makes `AGREEMENT` measurable from outside this module: the frame
/// line prints it for an arrangement of more than one page, so a run says what the residual
/// actually is rather than leaving the bound a claim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Carried {
    /// The one placement, in the picture's own texel space.
    pub placement: Transform,
    /// The largest disagreement absorbed, in device pixels. Zero for a single page, which has
    /// nothing to disagree with.
    pub(crate) within: f32,
}

/// A pure translation snapped to whole device pixels; anything else unchanged.
///
/// A scroll's stand-in used to move the base by the scroll's own fractional offset,
/// and a bilinear tap at a fractional offset resamples every texel — the owner's
/// report, verbatim: *"scrolling with the mouse still shows blurry text"*. Snapping
/// costs at most half a pixel of position for the one refresh the stand-in lives,
/// and buys text that stays text while it moves. Exact comparisons, deliberately:
/// the translation this recognises was *constructed* as one (`one_placement`), so
/// the identity coefficients are the literals written there, not survivors of an
/// inverse.
fn snapped(placement: Transform) -> Transform {
    let pure_translation = placement.a.to_bits() == 1.0_f32.to_bits()
        && placement.d.to_bits() == 1.0_f32.to_bits()
        && placement.b.to_bits() == 0.0_f32.to_bits()
        && placement.c.to_bits() == 0.0_f32.to_bits();
    if !pure_translation {
        return placement;
    }
    Transform {
        e: placement.e.round(),
        f: placement.f.round(),
        ..placement
    }
}

/// Whether a picture of `drawn` carries onto `wanted`, and where it goes.
///
/// **The whole of what makes a rendering usable for a view it is not of**, stated once and asked
/// from three places: [`Stale::carries`] plans a frame with it, [`Stale::reproject`] takes the
/// placement out of it, and [`could_stand_in`] asks it of a frame that has not been drawn yet.
///
/// A resize changes what the window is as well as where the pages are in it, and what is held is
/// that window's whole picture — see [`Refusal::Resized`] for why the chrome in it makes this an
/// impossibility rather than the revealed edge under another name. It is not an impossibility for
/// the retained pages, which are pictures of a *page* and know nothing about the window: a resize
/// is one of the cases they now answer.
///
/// [`Stale::carries`]: super::Stale::carries
/// [`Stale::reproject`]: super::Stale::reproject
pub(super) fn carrying(drawn: &[Placed], wanted: &[Placed]) -> Result<Carried, Refusal> {
    /// The window a picture of these pages is of, read off the first placement.
    fn extent(pages: &[Placed]) -> Option<(u32, u32)> {
        pages
            .first()
            .map(|placed| (placed.target.width, placed.target.height))
    }
    let asked = extent(wanted);
    if asked.is_some() && extent(drawn) != asked {
        return Err(Refusal::Resized);
    }
    one_placement(drawn, wanted)
}

/// One page's rectangle of the picture held and where it goes — what carries a column's last frame
/// through a zoom, a page at a time (ADR 2416).
///
/// `texels` is `None` for the whole picture under one placement, which is every view change but a
/// column's zoom, and `Some([left, top, right, bottom])` for one page of it: the rectangle the
/// page's own box covered when the picture was drawn, grown outward to whole texels and clipped to
/// the picture, so that its edge row is the page's edge row and no texel of the next page is in it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Part {
    /// Which texels of the picture held, left and top the first inside, right and bottom the
    /// first past it; `None` for all of them.
    pub texels: Option<[u32; 4]>,
    /// Where they go, from the picture's own texel space to the window's pixels.
    pub placement: Transform,
}

impl Part {
    /// The whole picture under one placement.
    #[must_use]
    pub const fn whole(placement: Transform) -> Self {
        Self {
            texels: None,
            placement,
        }
    }
}

/// How the last rendering goes up under a view it is not of, on a surface that draws in quads.
#[derive(Debug, Clone, PartialEq)]
pub enum Carry {
    /// One placement is true of every page, to within half a device pixel: the picture goes up whole.
    Whole(Carried),
    /// No one placement is — a zoom in a column, whose gap does not scale — so each page goes up
    /// under its own, and the gap between them is the window's again (ADR 2416).
    ByPage {
        /// One part a page, for every page of the view that the picture held has.
        parts: Vec<Part>,
        /// How far apart one placement would have put the pages, in device pixels at the worst
        /// corner — the [`Refusal::Rearranged`] this answers instead of refusing.
        apart: f32,
    },
}

impl Carry {
    /// The parts the presenter draws, in the picture's texel space.
    #[must_use]
    pub fn parts(&self) -> Vec<Part> {
        match self {
            Self::Whole(carried) => vec![Part::whole(carried.placement)],
            Self::ByPage { parts, .. } => parts.clone(),
        }
    }

    /// Rule 3 over how the picture went up: [`absorbed`]'s sentence for a whole picture, and for
    /// one carried a page at a time, how far one placement would have put its pages apart.
    pub fn say(&self, pages: usize, trace: impl crate::FrameLines) {
        match self {
            Self::Whole(carried) => absorbed(*carried, pages, trace),
            Self::ByPage { parts, apart } => trace.say(format_args!(
                "{} of {pages} pages carried one by one: one placement would put them {apart:.4} \
                 px apart at the worst corner, past the bound of {AGREEMENT}, so each page goes \
                 up under its own and the gap between them is kept",
                parts.len()
            )),
        }
    }
}

/// Whether a picture of `drawn` carries onto `wanted` on a surface that draws in quads, and how.
///
/// [`carrying`]'s answer where it has one. Where it refuses because the pages disagree, a surface
/// that puts a texture up in quads can still carry the picture: one quad a page, each under that
/// page's own placement and showing that page's rectangle alone, which is the arrangement the
/// retained layer is already drawn in ([`Proxies::placements`]). Every other refusal stands.
///
/// [`Proxies::placements`]: super::Proxies::placements
pub(super) fn carrying_on_quads(drawn: &[Placed], wanted: &[Placed]) -> Result<Carry, Refusal> {
    match carrying(drawn, wanted) {
        Ok(carried) => Ok(Carry::Whole(carried)),
        Err(Refusal::Rearranged { by, .. }) => {
            page_by_page(drawn, wanted).map(|parts| Carry::ByPage { parts, apart: by })
        }
        Err(other) => Err(other),
    }
}

/// One [`Part`] a page of `wanted` that `drawn` holds: the page's rectangle of the picture, and
/// that page's own `was⁻¹ ∘ now`.
///
/// The rectangle is the page's box — `[0, 0]` to its `page_size` in the user space the page's
/// list is in (`TargetSpec::for_page`) — under the placement it was drawn at, so it is the page
/// the picture shows and nothing beside it; a page the picture held entirely off its own edge has
/// no texel to carry and is left out, as a retained page whose composition fails is.
fn page_by_page(drawn: &[Placed], wanted: &[Placed]) -> Result<Vec<Part>, Refusal> {
    let mut parts = Vec::new();
    for placed in wanted {
        let Some(was) = drawn.iter().find(|held| held.of == placed.of) else {
            continue;
        };
        let placement = snapped(moved(&was.target, &placed.target)?);
        if let Some(texels) = page_texels(was) {
            parts.push(Part {
                texels: Some(texels),
                placement,
            });
        }
    }
    if parts.is_empty() {
        return Err(Refusal::AnotherPage);
    }
    Ok(parts)
}

/// The texels of its picture a page's box covered: left, top, right, bottom, grown outward to
/// whole texels and clipped to the picture, or `None` where nothing of the page is in it.
fn page_texels(page: &Placed) -> Option<[u32; 4]> {
    let size = page.list.page_size;
    let place = page.target.transform;
    let corners = [
        (0.0, 0.0),
        (size.width, 0.0),
        (0.0, size.height),
        (size.width, size.height),
    ]
    .map(|(x, y)| place.apply(pdf_render::Point::new(x, y)));
    let span = |pick: fn(&pdf_render::Point) -> f32, limit: u32| {
        let low = corners.iter().map(pick).fold(f32::INFINITY, f32::min);
        let high = corners.iter().map(pick).fold(f32::NEG_INFINITY, f32::max);
        // In `f64` so that a page placed far outside the window clamps rather than overflows.
        let limit = f64::from(limit);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to [0, the picture's extent], which u32 holds"
        )]
        let texel = |at: f64| at.clamp(0.0, limit) as u32;
        (texel(f64::from(low).floor()), texel(f64::from(high).ceil()))
    };
    let (left, right) = span(|point| point.x, page.target.width);
    let (top, bottom) = span(|point| point.y, page.target.height);
    (left < right && top < bottom).then_some([left, top, right, bottom])
}

/// Whether a frame *now being drawn* of `drawn` would be worth anything once it landed.
///
/// **This is the one question ADR 0657's policy turns on**, and it is deliberately `crate::stale`'s
/// rather than `quorra::composer`'s: what makes a superseded frame worth finishing is that its
/// pixels can still stand in for the view the person has moved to, and the rules deciding that are
/// the five `crate::stale` states. A frame that answers `false` here will be neither presented —
/// the view has moved — nor used as a base, so the thread drawing it is spending itself on pixels
/// this program has already decided it will never show.
///
/// `true` where it carries and `true` where the view has not moved at all, which is the common
/// case and the one a caller must not interrupt.
#[must_use]
pub fn could_stand_in(drawn: &[Placed], wanted: &[Placed]) -> bool {
    depicts(drawn, wanted) || carrying(drawn, wanted).is_ok()
}

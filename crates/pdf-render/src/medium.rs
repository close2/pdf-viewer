//! Where a page stops on a target, and what that decides: the colour under it, the colour
//! beside it, and the ink the standard says shall not be shown beyond it.
//!
//! One boundary, three consequences. [`page_area`] is the boundary; [`impose_within`] puts
//! §11.4.7's 𝑊 inside it and [`SURROUND`] outside; [`crop_to_page`] is §14.11.2.1's `shall`,
//! which is about the page's own marks rather than about either colour.
//!
//! # The page's colour is stated
//!
//! ISO 32000-2 §11.4.7, of the page group:
//!
//! > Ordinarily, the page shall be imposed directly on an output medium, such as paper or a
//! > display screen. The page group shall be treated as an isolated group, whose results
//! > shall then be composited with a backdrop colour appropriate for the medium. The backdrop
//! > is nominally white (in a colour space chosen by the PDF processor), although varying
//! > according to the actual properties of the medium.
//!
//! Table 141 names that colour and says whose it is: 𝑊 is the "[i]nitial colour of the page
//! (nominally white but may vary depending on the properties of the medium or the needs of the
//! application)". **A property of the page**, and the clause's own sentence about an
//! interactive processor stays inside that boundary — "some interactive PDF processors may
//! choose to provide a different backdrop, such as a checker board or grid to aid in
//! visualizing the effects of transparency in the artwork" is a different backdrop *for the
//! page*, so that the page's own transparency can be seen through it.
//!
//! Where the page stops is §14.11.2.1's, and it is a `shall`:
//!
//! > The crop box defines the region to which the contents of the page shall be clipped
//! > (cropped) when displayed or printed.
//!
//! and, two sentences later, that the same box is what places the page on the medium:
//!
//! > However, in the absence of additional information (such as imposition instructions
//! > specified in a JDF j ob ticket), the crop box determines how the page's contents shall be
//! > positioned on the output medium.
//!
//! [`DisplayList::page_bounds`] is that region — `pdf_model::interpret` puts the box the page is
//! displayed in at the origin — so mapping it through a target's transform says, in that
//! target's own pixels, exactly where 𝑊 applies.
//!
//! # The colour outside every page is not stated anywhere
//!
//! Searched rather than assumed, because "the specification defines nothing here" is itself a
//! claim about the specification and this project has had one of those be wrong: §11.4.7 and
//! Table 141 (above), §11.4.5's isolated groups, §11.3's compositing formulas — every backdrop
//! they name is a *group's* or a page's — §14.11.2 on the five page boundaries, and Table 147's
//! twenty-two viewer preferences, which say what to hide, which boundary to display and which to
//! clip to, and nothing about what surrounds the result. `tools/spec-errata`'s `emit` reports no
//! erratum anywhere in clause 11. The standard describes one page imposed on one medium; that a
//! window may show two pages at once, with something between them, is outside its subject.
//!
//! So [`SURROUND`] is **a choice this program makes**, written down as one, and [`Medium`] is
//! what keeps it from being confused with 𝑊. One colour served as both until the
//! six-hundred-and-eleventh session, which is why a continuous column's pages ran into each
//! other: the surround was page white, so the gap between two pages was the same colour as the
//! paper on either side of it. Making the *medium* grey to see the gap turned the **pages** grey,
//! and that is the proof that one value was doing two jobs (ADR 0442's last finding).
//!
//! # The same boundary keeps the page's own ink in, and that is the clause rather than a choice
//!
//! §14.11.2.1 quoted above is a `shall` about *the contents of the page*, and the sentence after
//! it says the box means nothing else: "[u]nlike the other boxes, the crop box has no defined
//! meaning in terms of physical page geometry or intended use; it merely imposes clipping on the
//! page contents." `pdf_model::interpret` deliberately keeps the marks a content stream made
//! outside that box — a display list is what the file says — so something has to put the clip
//! back, and until the six-hundred-and-twelfth session nothing did.
//!
//! It went unseen because **a page-sized target met the requirement by accident**: the raster is
//! the boundary's own extent, so the raster's edge did the cutting and no gate could tell the
//! difference. A window is larger than its page. What a reader saw there was ink beside the page
//! and over the next page of a column — `doc/traps/instruments-and-reports.md`'s shape exactly,
//! a `shall` satisfied by the instrument rather than by the code.
//!
//! [`crop_to_page`] is the clip, and it is applied to the finished page rather than to each mark.
//! That is not an approximation and the reason is §11.4.7's: the page group "shall be treated as
//! an isolated group", every operator clause 11 composites it with is per-pixel, and the region
//! is the same for every mark in it — so clipping the group's result is clipping its contents.
//! What the region *is* is §10.7.4's set of whole pixels rather than an area, which is what makes
//! the pass exact and what keeps every page-sized raster in this tree unmoved; [`crop_to_page`]
//! has both of the clause's sentences and the two readings the corpus rejected first.
//!
//! # Why the decision is here rather than in a backend
//!
//! `doc/traps/pixels-and-rasterisers.md` trap 2: *a decision either backend can make alone is a
//! decision neither has made*. Three rasterisers draw this program's pages and the boundary
//! between 𝑊 and the surround has to fall in the same place in all three, so the boundary, the
//! composite, the colour and the crop are stated once, here.

use crate::backend::TargetSpec;
use crate::display_list::DisplayList;
use crate::geom::{Point, Rect};
use crate::paint::Color;

/// The colour this program shows where no page lies.
///
/// **A documented choice, not a reading**: see this module's header for the search that
/// establishes the standard states nothing about the area outside a page. Two things decide it
/// and neither is anybody else's output:
///
/// - It has to differ from 𝑊 — which is nominally white — by enough that the boundary between two
///   pages of a column is visible at a glance, including where the pages themselves are mostly
///   white paper. A quarter of full scale is 64 of 255, four times the difference a JPEG artefact
///   or a scanned page's grey cast can produce.
/// - It has to be neutral and darker than the page, so that the eye reads the page as the lit
///   thing and the surround as the absence of one. A surround *lighter* than 𝑊 would make every
///   page look like a shadow of the window.
///
/// It is not configurable and there is no user interface for it: a theme is a larger question
/// than this constant and is deliberately not being answered here.
pub const SURROUND: Color = Color::grey(0.25);

/// §11.4.7's 𝑊, and the colour outside every page's own boundary.
///
/// The two are separate fields because they are separate things — see this module's header.
/// A rasteriser is handed both and the boundary decides which applies where; nothing downstream
/// of [`impose_within`] has to know that there were ever two.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Medium {
    /// §11.4.7's 𝑊, the initial colour of the page: "nominally white".
    ///
    /// A fully transparent value means the caller wants the page's own alpha back rather than a
    /// page composited onto anything, which is what a caller compositing the page over something
    /// else asks for.
    pub page: Color,
    /// What the target shows outside every page's boundary.
    ///
    /// Not the standard's — [`SURROUND`] has the argument.
    pub surround: Color,
}

impl Medium {
    /// The whole of the target is the page: 𝑊 white, and no surround to speak of.
    ///
    /// **The default for a [`crate::Rasterizer`]**, because a rasteriser is handed one display
    /// list and a target for it, and every target this tree builds for a single page — the
    /// corpus gate's, the oracle's, a tier-1 host's page raster — is the page's own extent.
    /// A target *larger* than its page is a window, and a window says so with [`Self::WINDOW`].
    pub const PAGE_ONLY: Self = Self {
        page: Color::WHITE,
        surround: Color::WHITE,
    };

    /// A window: §11.4.7's white page, and [`SURROUND`] everywhere else.
    pub const WINDOW: Self = Self {
        page: Color::WHITE,
        surround: SURROUND,
    };

    /// Nothing is composited at all: the caller wants the page's own alpha.
    pub const NONE: Self = Self {
        page: Color::TRANSPARENT,
        surround: Color::TRANSPARENT,
    };

    /// One colour for both, which is what every target that *is* its page wants.
    #[must_use]
    pub const fn uniform(colour: Color) -> Self {
        Self {
            page: colour,
            surround: colour,
        }
    }

    /// The same 𝑊, with nothing outside the page at all.
    ///
    /// What a caller compositing one page of an arrangement *over* the rest asks for: the
    /// surround is painted once, under everything, and a page drawn afterwards carries only its
    /// own paper. A page that took the surround with it would erase whatever is beside it.
    #[must_use]
    pub const fn on_transparency(self) -> Self {
        Self {
            page: self.page,
            surround: Color::TRANSPARENT,
        }
    }

    /// Whether the two colours are one, in which case there is no boundary to draw.
    ///
    /// [`impose_within`] takes exactly the path [`impose_on_medium`] always took when this is
    /// true, which is what makes a page-sized target's bytes unchanged by the separation.
    #[must_use]
    pub fn is_uniform(self) -> bool {
        self.page == self.surround
    }

    /// Whether anything at all is composited under the page.
    #[must_use]
    pub fn marks_anything(self) -> bool {
        self.page.a > 0.0 || self.surround.a > 0.0
    }
}

/// Where a page lies in a target's pixels: the region §11.4.7's 𝑊 covers and no more.
///
/// §14.11.2.1's crop box is the page — quoted in this module's header — and
/// [`DisplayList::page_bounds`] is that box with its corner at the origin, so the target's own
/// transform is the whole of the mapping.
///
/// **The bounding box of the image, for a transform that could rotate.** Every target this tree
/// builds is a scale, a y flip and a translation ([`TargetSpec::for_page`] and the placement a
/// host composes onto it), for which the bounding box *is* the image; a caller that rotates a
/// page in the target transform gets the smallest upright rectangle containing it, which is the
/// only answer a row-by-row composite can use.
#[must_use]
pub fn page_area(list: &DisplayList, target: TargetSpec) -> Rect {
    list.page_bounds().mapped(target.transform)
}

/// Where §14.11.2.1 stops this page's ink in a target's pixels, or `None` where no pixel of the
/// target lies outside it.
///
/// `None` has two causes and they are one answer: the list is not a page at all — a host's own
/// chrome, which [`DisplayList::content_clip`] leaves unset — or no whole pixel of the target
/// lies beyond the boundary, which is every page-sized raster this tree builds. A backend that
/// gets `None` does exactly what it did before this function existed.
///
/// **The test is against the boundary grown to whole pixels**, because §10.7.4's clipping region
/// is a set of pixels rather than an area — see [`crop_to_page`], which is where the clause is
/// quoted. `TargetSpec::for_page` rounds a raster *up* so that it contains its page (ADR 0064),
/// so on a fifth of this corpus the target overhangs the boundary by a fraction of a pixel; under
/// the clause's own rule that fraction is inside the clipping region, and answering `None` here
/// is the same statement made once instead of per pixel.
///
/// **The region is [`DisplayList::content_clip`] and not [`page_bounds`](DisplayList::page_bounds)**,
/// because §12.2 makes them two questions: `/ViewArea` decides which boundary is *displayed* —
/// which is the extent [`page_area`] answers with — and `/ViewClip` which one the contents are
/// clipped to. A document may display its media box while clipping ink to its trim box, and the
/// margin between them is then blank rather than absent.
#[must_use]
pub fn crop_area(list: &DisplayList, target: TargetSpec) -> Option<Rect> {
    let region = list.content_clip()?.mapped(target.transform);
    let whole = Rect::from_corners(
        Point::new(0.0, 0.0),
        #[expect(
            clippy::cast_precision_loss,
            reason = "a target's extent is bounded by MAX_EXTENT = 2^24, which f32 holds exactly"
        )]
        Point::new(target.width as f32, target.height as f32),
    );
    let pixels = Rect::from_corners(
        Point::new(region.min.x.floor(), region.min.y.floor()),
        Point::new(region.max.x.ceil(), region.max.y.ceil()),
    );
    if pixels.contains(whole) {
        return None;
    }
    Some(region)
}

/// Applies §14.11.2.1's clip to a rendered page: the ink outside `area` is not shown.
///
/// > The crop box defines the region to which the contents of the page shall be clipped
/// > (cropped) when displayed or printed.
///
/// `data` is **premultiplied** RGBA8, as [`impose_within`] wants it and as the rasterisers hold
/// it before they convert at their boundary, and may be a run of rows of the target rather than
/// the whole of it — `first_row` says which row it starts at. `area` is [`crop_area`].
///
/// **Called before [`impose_within`], never after.** What is being cut here is the page's own
/// marks; the colours the medium puts under them are not the page's and are not §14.11.2.1's
/// subject, and a pass that ran the other way round would erase the surround instead.
///
/// # A clipping region is a set of whole pixels, and the standard says so twice
///
/// §10.7.4, of what a clip *is*:
///
/// > For clipping, the clipping region consists of the set of pixels that would be included by
/// > a fill operation. Subsequent painting operations shall affect a region that is the
/// > intersection of the set of pixels defined by the clipping region with the set of pixels
/// > for the region to be painted.
///
/// and, of which pixels a fill includes — a `shall`, with the reason attached:
///
/// > A shape shall be scan-converted by painting any pixel whose half-open square region
/// > intersects the shape, no matter how small the intersection is. This ensures that no shape
/// > ever disappears as a result of unfavourable placement relative to the device pixel grid, as
/// > might happen with other possible scan conversion rules. The area covered by painted pixels
/// > shall always be at least as large as the area of the original shape.
///
/// So a pixel the boundary touches at all is **inside** the region and keeps its ink whole, and
/// only a pixel wholly beyond it is cleared. There is no fraction to apply and no arithmetic to
/// get wrong: the last sentence quoted forbids one outright, since attenuating a partly covered
/// pixel would leave the painted area *smaller* than the shape.
///
/// **This was written the other way first and the corpus said so.** Multiplying the boundary
/// pixel by its coverage moved 37 of 957 first pages; taking the smaller of the two coverages
/// instead moved 11; the clause's own rule moves none, and those eleven were pages whose extent
/// is not a whole number of pixels — `red_stamp.pdf`'s crop box is 315.001 units tall, so its
/// raster's last row is a thousandth of a pixel of page, and a fraction would have erased ink
/// four reference renderers draw. The references are corroboration and not the ground: the
/// ground is that §10.7.4 states a clip as a set of pixels and forbids painting less area than
/// the shape.
///
/// # This is not [`impose_within`]'s rule, and the difference is the standard's
///
/// The medium's own boundary *is* fractional, because painting 𝑊 is a composite rather than a
/// clip: §11.4.7 gives the page a colour and says nothing about pixel grids, and a page-to-page
/// gap narrower than a device pixel has to survive it (ADR 0446). A clip is §10.7.4's set. Two
/// mechanisms, two clauses, one boundary — which is why they are stated beside each other here.
pub fn crop_to_page(data: &mut [u8], width: u32, first_row: u32, area: Rect) {
    let stride = (width as usize).saturating_mul(4);
    if stride == 0 {
        return;
    }
    for (offset, row) in data.chunks_mut(stride).enumerate() {
        let Some(top) = row_edge(first_row, offset) else {
            // Past what an `f32` resolves to the pixel; a target that tall is refused long
            // before this by `MAX_EXTENT`, and cutting is the half that shows nothing the
            // standard forbids.
            row.fill(0);
            continue;
        };
        if !touches(top, area.min.y, area.max.y) {
            row.fill(0);
            continue;
        }
        for (column, pixel) in row.chunks_exact_mut(4).enumerate() {
            let Some(left) = row_edge(0, column) else {
                pixel.fill(0);
                continue;
            };
            if !touches(left, area.min.x, area.max.x) {
                pixel.fill(0);
            }
        }
    }
}

/// Whether the half-open pixel `[edge, edge + 1)` intersects the half-open span `[lo, hi)`.
///
/// §10.7.4's rule in one line: "any pixel whose half-open square region intersects the shape, no
/// matter how small the intersection is". Half-open on both operands, which is the clause's own
/// convention — a pixel that begins exactly where the span ends is outside it.
fn touches(edge: f32, lo: f32, hi: f32) -> bool {
    edge < hi && edge + 1.0 > lo
}

/// Composites a rendered page onto the colour of the medium it is imposed on.
///
/// ISO 32000-2 §11.4.7, of the page group — the group every object on a page belongs to:
///
/// > Ordinarily, the page shall be imposed directly on an output medium, such as paper or a
/// > display screen. The page group shall be treated as an isolated group, whose results
/// > shall then be composited with a backdrop colour appropriate for the medium.
///
/// **Isolated**, so the page's own initial backdrop is transparent and not the medium: an
/// object blending against an unpainted part of the page sees nothing there, and §11.3.6's
/// formula gives it its own colour. Painting the medium's colour *first* and drawing over it
/// is the natural implementation and a different picture — every blend mode then sees white
/// where the standard says it sees nothing. `transparency_group.pdf` is the case: an ellipse
/// under `/BM /Difference` is crimson where it covers white in all four reference renderers,
/// and the inverse of crimson if the white is a backdrop.
///
/// Every backend therefore renders onto transparency and calls this at the end, so that
/// none of them can make the choice differently.
///
/// **This one composites the same colour over the whole of `data`**, which is the right answer
/// for a target that *is* its page and the wrong one for a window: [`impose_within`] is the
/// same composite with §14.11.2.1's boundary in it.
///
/// `data` is **premultiplied** RGBA8, which is what the rasterisers hold before they convert
/// at their boundary. Premultiplied and not straight, because this is a source-over
/// composite and doing it in premultiplied form is both the natural arithmetic and lossless:
/// a page rendered onto transparency and demultiplied first would divide every antialiased
/// edge by its own alpha and multiply it back here, and the two roundings cost a level per
/// channel across every glyph on the page.
pub fn impose_on_medium(data: &mut [u8], medium: Color) {
    let backdrop = backdrop(medium, 1.0);
    if backdrop[3] == 0 {
        // Nothing to composite onto: the caller wants the page's own alpha, which is what
        // the raster already holds.
        return;
    }

    for pixel in data.chunks_exact_mut(4) {
        if pixel[3] == u8::MAX {
            continue;
        }
        // A pixel nothing marked is the medium and no arithmetic. Exact rather than a
        // shortcut: with `clear` at 255 the quotient below is `(below × 255 + 127) / 255`,
        // which is `below` for every `below` in 0..=255, and the sum it is added to is zero.
        // Worth stating as its own case because it is *most* of a page — §11.4.7's page group
        // is isolated, so an unmarked pixel is transparent — and the general case is eight
        // integer divisions. On a 1192×1684 page this took the pass from 7.8 ms to 1.4.
        if *pixel == [0, 0, 0, 0] {
            pixel.copy_from_slice(&backdrop);
            continue;
        }
        over(pixel, backdrop);
    }
}

/// Composites a rendered frame onto §11.4.7's 𝑊 inside the page's boundary and onto the
/// program's own [`SURROUND`] outside it.
///
/// `data` is premultiplied RGBA8 as [`impose_on_medium`] wants it, and may be a *run of rows*
/// of the target rather than the whole of it — `first_row` says which row it starts at, so that
/// a rasteriser splitting the pass across threads hands each thread its own offset. `area` is
/// the page's own rectangle in this target's pixels, which is [`page_area`].
///
/// # The boundary is a coverage rather than a pixel edge
///
/// A page's edge lands wherever the magnification and the scroll put it, which is almost never
/// on a pixel boundary. Each pixel therefore takes the fraction of itself the page covers —
/// exact box coverage of an upright rectangle, the product of the two axes' overlaps — and 𝑊 is
/// composited at that fraction, over the surround. Snapping the boundary to whole pixels instead
/// would be cheaper and would be wrong in the way that matters here: two pages whose gap is
/// under a device pixel would each round outward, the gap would close, and the separation this
/// function exists for would disappear at exactly the magnification a reader is most likely to
/// use it at.
///
/// **The page's own marks are not clipped to `area` here** and this function must not start
/// doing it: §14.11.2.1's clip is [`crop_to_page`]'s, it runs before this pass, and a composite
/// that also erased ink would be a second, silent statement of a rule that belongs in one place.
/// What is decided here is only which colour lies *under* the page at each pixel — and the two
/// are not the same boundary in general, because §12.2 lets a document display one of
/// §14.11.2's boxes and clip its contents to another.
pub fn impose_within(data: &mut [u8], width: u32, first_row: u32, area: Rect, medium: Medium) {
    if medium.is_uniform() {
        // No boundary: the same colour on both sides of it is the composite this pass has
        // always been, byte for byte, which is what leaves every page-sized target unmoved.
        impose_on_medium(data, medium.page);
        return;
    }
    let stride = (width as usize).saturating_mul(4);
    if stride == 0 {
        return;
    }
    let surround = backdrop(medium.surround, 1.0);
    for (offset, row) in data.chunks_mut(stride).enumerate() {
        let Some(top) = row_edge(first_row, offset) else {
            // Past what an `f32` resolves to the pixel; a target that tall is refused long
            // before this by `MAX_EXTENT`, and answering with the surround is the safe half.
            impose_on_medium(row, medium.surround);
            continue;
        };
        let down = overlap(top, area.min.y, area.max.y);
        if down <= 0.0 {
            impose_on_medium(row, medium.surround);
            continue;
        }
        for (column, pixel) in row.chunks_exact_mut(4).enumerate() {
            let Some(left) = row_edge(0, column) else {
                over_pixel(pixel, surround);
                continue;
            };
            let coverage = overlap(left, area.min.x, area.max.x) * down;
            if coverage > 0.0 {
                over_pixel(pixel, backdrop(medium.page, coverage));
            }
            if coverage < 1.0 {
                over_pixel(pixel, surround);
            }
        }
    }
}

/// The top (or left) edge of the pixel `offset` rows past `first`, or `None` past `f32`'s
/// exact integers.
fn row_edge(first: u32, offset: usize) -> Option<f32> {
    let index = u64::from(first).checked_add(offset as u64)?;
    if index > u64::from(crate::backend::MAX_EXTENT) {
        return None;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "bounded to MAX_EXTENT = 2^24 immediately above, which f32 holds exactly"
    )]
    Some(index as f32)
}

/// How much of the unit-wide pixel starting at `edge` lies inside `lo..hi`, in `0.0..=1.0`.
fn overlap(edge: f32, lo: f32, hi: f32) -> f32 {
    let covered = (edge + 1.0).min(hi) - edge.max(lo);
    if covered.is_finite() {
        covered.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// The medium's premultiplied bytes at a coverage, ready to composite under a pixel.
fn backdrop(medium: Color, coverage: f32) -> [u8; 4] {
    let alpha = medium.a.clamp(0.0, 1.0) * coverage.clamp(0.0, 1.0);
    let scale = |component: f32| {
        let value = component.clamp(0.0, 1.0) * alpha * 255.0 + 0.5;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the product of two values in 0..=1 scaled by 255 is in 0..=255"
        )]
        {
            value as u8
        }
    };
    [
        scale(medium.r),
        scale(medium.g),
        scale(medium.b),
        scale(1.0),
    ]
}

/// One pixel over one backdrop, with [`impose_on_medium`]'s two shortcuts.
fn over_pixel(pixel: &mut [u8], backdrop: [u8; 4]) {
    if backdrop[3] == 0 || pixel[3] == u8::MAX {
        return;
    }
    if *pixel == [0, 0, 0, 0] {
        pixel.copy_from_slice(&backdrop);
        return;
    }
    over(pixel, backdrop);
}

/// Source-over in premultiplied eight-bit form: `pixel` above, `backdrop` below.
fn over(pixel: &mut [u8], backdrop: [u8; 4]) {
    let clear = u16::from(u8::MAX.wrapping_sub(pixel[3]));
    for (channel, below) in pixel.iter_mut().zip(backdrop) {
        // `below * clear` peaks at 255 * 255 = 65 025, inside `u16`, and the quotient is
        // at most 255 — so the whole of this is exact in eight bits, which is the point
        // of doing it here rather than after the conversion to straight alpha.
        let contribution = (u16::from(below).saturating_mul(clear).saturating_add(127)) / 255;
        *channel =
            u8::try_from(u16::from(*channel).saturating_add(contribution)).unwrap_or(u8::MAX);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Medium, SURROUND, crop_area, crop_to_page, impose_on_medium, impose_within, overlap,
        page_area,
    };
    use crate::backend::TargetSpec;
    use crate::display_list::DisplayList;
    use crate::geom::{Point, Rect, Size, Transform};
    use crate::paint::Color;

    /// The rectangle from two corners, without four `Point::new`s at every call site.
    fn box_of(min: (f32, f32), max: (f32, f32)) -> Rect {
        Rect::from_corners(Point::new(min.0, min.1), Point::new(max.0, max.1))
    }

    /// The surround's own red channel as this composite quantises it.
    fn surround_level() -> f32 {
        (SURROUND.r * 255.0 + 0.5).floor()
    }

    /// The one property that keeps every gate's bytes where they were: a target that is its own
    /// page asks for one colour, and one colour is the composite this pass always was.
    #[test]
    fn a_uniform_medium_is_the_composite_it_always_was() {
        for medium in [
            Color::WHITE,
            Color::grey(0.25),
            Color::rgba(0.2, 0.4, 0.6, 0.5),
        ] {
            let start: Vec<u8> = (0..64_u8)
                .flat_map(|n| [n, n / 2, n / 3, n.wrapping_mul(3)])
                .collect();
            let mut whole = start.clone();
            let mut within = start;
            impose_on_medium(&mut whole, medium);
            impose_within(
                &mut within,
                8,
                0,
                box_of((0.0, 0.0), (8.0, 8.0)),
                Medium::uniform(medium),
            );
            assert_eq!(whole, within, "a uniform medium took a different path");
        }
    }

    /// §11.4.7's 𝑊 stops at the page and the surround starts there — the whole point of the
    /// separation, stated on a target twice the page's width.
    #[test]
    fn the_page_takes_w_and_everything_else_takes_the_surround() {
        // Four pixels across, one row; the page is the left half exactly.
        let mut data = vec![0_u8; 4 * 4];
        impose_within(
            &mut data,
            4,
            0,
            box_of((0.0, 0.0), (2.0, 1.0)),
            Medium::WINDOW,
        );
        let pixels: Vec<&[u8]> = data.chunks_exact(4).collect();
        assert_eq!(pixels[0], [255, 255, 255, 255], "the page is 𝑊");
        assert_eq!(pixels[1], [255, 255, 255, 255], "the page is 𝑊");
        for outside in &pixels[2..] {
            assert!(
                (f32::from(outside[0]) - surround_level()).abs() < 1.0
                    && outside[3] == 255
                    && outside[0] == outside[1]
                    && outside[1] == outside[2],
                "outside the page is not the surround: {outside:?}"
            );
        }
    }

    /// A page edge that falls inside a pixel gives that pixel a mixture, which is what keeps a
    /// gap narrower than a device pixel visible instead of rounding it away.
    #[test]
    fn an_edge_inside_a_pixel_mixes_the_two() {
        let mut data = vec![0_u8; 4 * 3];
        // The page ends a quarter of the way through the middle pixel.
        impose_within(
            &mut data,
            3,
            0,
            box_of((0.0, 0.0), (1.25, 1.0)),
            Medium::WINDOW,
        );
        let middle = &data[4..8];
        let grey = surround_level();
        let expected = 255.0_f32.mul_add(0.25, grey * 0.75);
        assert!(
            f32::from(middle[0]) > grey && f32::from(middle[0]) < 255.0,
            "the boundary pixel is neither colour: {middle:?}"
        );
        assert!(
            (f32::from(middle[0]) - expected).abs() <= 2.0,
            "the boundary pixel is not the coverage mixture: {middle:?} against {expected}"
        );
        assert_eq!(middle[3], 255, "the frame stays opaque");
    }

    /// The page's ink survives the boundary: a mark inside the page is composited over 𝑊 and
    /// not over the surround.
    #[test]
    fn ink_inside_the_page_composites_over_the_page_colour() {
        // One opaque black pixel inside the page, one untouched pixel outside it.
        let mut data = vec![0_u8; 4 * 2];
        data[3] = 255;
        impose_within(
            &mut data,
            2,
            0,
            box_of((0.0, 0.0), (1.0, 1.0)),
            Medium::WINDOW,
        );
        assert_eq!(
            &data[0..4],
            &[0, 0, 0, 255],
            "ink is untouched by the medium"
        );
    }

    /// [`page_area`] is the page's own box in the target's pixels, including the placement a
    /// window composes onto the transform.
    #[test]
    fn the_page_area_follows_the_targets_transform() {
        let list = DisplayList::new(Size::new(200.0, 100.0));
        let target = TargetSpec {
            width: 400,
            height: 300,
            transform: Transform::scale(2.0, -2.0).then(Transform::translate(10.0, 200.0)),
        };
        let area = page_area(&list, target);
        assert!((area.min.x - 10.0).abs() < 1e-3, "{area:?}");
        assert!((area.max.x - 410.0).abs() < 1e-3, "{area:?}");
        assert!((area.min.y - 0.0).abs() < 1e-3, "{area:?}");
        assert!((area.max.y - 200.0).abs() < 1e-3, "{area:?}");
    }

    /// §14.11.2.1's `shall`, at its plainest: a mark beyond the boundary is not shown.
    #[test]
    fn ink_outside_the_boundary_is_not_shown() {
        // Four opaque black pixels in a row; the page is the left half exactly.
        let mut data: Vec<u8> = (0..4).flat_map(|_| [0, 0, 0, 255]).collect();
        crop_to_page(&mut data, 4, 0, box_of((0.0, 0.0), (2.0, 1.0)));
        let pixels: Vec<&[u8]> = data.chunks_exact(4).collect();
        assert_eq!(
            pixels[0],
            [0, 0, 0, 255],
            "ink inside the page is untouched"
        );
        assert_eq!(
            pixels[1],
            [0, 0, 0, 255],
            "ink inside the page is untouched"
        );
        assert_eq!(pixels[2], [0, 0, 0, 0], "ink outside it is gone");
        assert_eq!(pixels[3], [0, 0, 0, 0], "ink outside it is gone");
    }

    /// A row the boundary does not reach at all is cleared without looking at its pixels.
    #[test]
    fn a_row_beyond_the_boundary_is_cleared_whole() {
        let mut data: Vec<u8> = (0..4).flat_map(|_| [10, 20, 30, 40]).collect();
        // The page occupies row 0 only; this call starts at row 1.
        crop_to_page(&mut data, 2, 1, box_of((0.0, 0.0), (2.0, 1.0)));
        assert!(data.iter().all(|byte| *byte == 0), "{data:?}");
    }

    /// §10.7.4's rule at the pixel the boundary passes through: "any pixel whose half-open
    /// square region intersects the shape, no matter how small the intersection is".
    #[test]
    fn a_pixel_the_boundary_only_touches_keeps_its_ink_whole() {
        // Two pixels; the page ends a quarter of the way through the second.
        let mut data = vec![255, 0, 0, 255, 255, 0, 0, 255];
        crop_to_page(&mut data, 2, 0, box_of((0.0, 0.0), (1.25, 1.0)));
        assert_eq!(
            data,
            vec![255, 0, 0, 255, 255, 0, 0, 255],
            "a partly covered pixel is inside the clipping region and is not attenuated"
        );
        // And a thousandth of a pixel is still "no matter how small": `red_stamp.pdf`'s page is
        // 315.001 units tall and its raster's last row is exactly this case.
        let mut sliver = vec![255, 0, 0, 255, 255, 0, 0, 255];
        crop_to_page(&mut sliver, 2, 0, box_of((0.0, 0.0), (1.001, 1.0)));
        assert_eq!(sliver, vec![255, 0, 0, 255, 255, 0, 0, 255]);
    }

    /// The pixel after the one the boundary ends in is outside, and goes whole.
    #[test]
    fn the_pixel_past_the_boundary_goes_whole() {
        let mut data = vec![255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255];
        crop_to_page(&mut data, 3, 0, box_of((0.0, 0.0), (1.25, 1.0)));
        assert_eq!(&data[8..12], &[0, 0, 0, 0], "the third pixel is beyond it");
    }

    /// The one property that keeps every page-sized raster in this tree byte for byte where it
    /// was: a target its own page covers has nothing to cut.
    #[test]
    fn a_target_its_page_covers_asks_for_no_crop() {
        let mut list = DisplayList::new(Size::new(200.0, 100.0));
        let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("a page-sized target");
        list.set_content_clip(list.page_bounds());
        assert_eq!(crop_area(&list, target), None);
        // A window twice the page's size does have something to cut, and it is the page.
        let window = TargetSpec {
            width: 400,
            height: 300,
            transform: target.transform,
        };
        let area = crop_area(&list, window).expect("a window crops");
        assert!((area.max.x - 200.0).abs() < 1e-3, "{area:?}");
        assert!((area.max.y - 100.0).abs() < 1e-3, "{area:?}");
    }

    /// A list that is not a page — a host's own chrome — is not §14.11.2.1's subject.
    #[test]
    fn a_list_that_is_not_a_page_is_never_cropped() {
        let list = DisplayList::new(Size::new(20.0, 10.0));
        let target = TargetSpec {
            width: 400,
            height: 300,
            transform: Transform::IDENTITY,
        };
        assert_eq!(list.content_clip(), None);
        assert_eq!(crop_area(&list, target), None);
    }

    #[test]
    fn a_pixels_overlap_with_a_span_is_its_covered_fraction() {
        assert!((overlap(0.0, 0.0, 4.0) - 1.0).abs() < 1e-6);
        assert!((overlap(3.0, 0.0, 3.5) - 0.5).abs() < 1e-6);
        assert!(overlap(4.0, 0.0, 3.5).abs() < 1e-6);
        assert!((overlap(0.0, 0.25, 0.75) - 0.5).abs() < 1e-6);
    }
}

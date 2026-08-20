//! What a page is painted on, and what lies outside it — two colours, and only one of them
//! is the standard's.
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
//! # Why the decision is here rather than in a backend
//!
//! `doc/traps/pixels-and-rasterisers.md` trap 2: *a decision either backend can make alone is a
//! decision neither has made*. Three rasterisers draw this program's pages and the boundary
//! between 𝑊 and the surround has to fall in the same place in all three, so the boundary, the
//! composite and the colour are stated once, here.

use crate::backend::TargetSpec;
use crate::display_list::DisplayList;
use crate::geom::Rect;
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
/// doing it: §14.11.2.1's clip is the interpreter's to apply, `pdf_model::interpret` deliberately
/// keeps the marks a stream made outside the box, and a composite that erased them would be a
/// second, silent statement of a rule that belongs in one place. What is decided here is only
/// which colour lies *under* the page at each pixel.
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
    use super::{Medium, SURROUND, impose_on_medium, impose_within, overlap, page_area};
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

    #[test]
    fn a_pixels_overlap_with_a_span_is_its_covered_fraction() {
        assert!((overlap(0.0, 0.0, 4.0) - 1.0).abs() < 1e-6);
        assert!((overlap(3.0, 0.0, 3.5) - 0.5).abs() < 1e-6);
        assert!(overlap(4.0, 0.0, 3.5).abs() < 1e-6);
        assert!((overlap(0.0, 0.25, 0.75) - 0.5).abs() < 1e-6);
    }
}

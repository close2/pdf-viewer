//! The stroke kernel — ISO 32000-2 §8.5.3.2: a path stroked under its line width, dash, caps and
//! joins.
//!
//! A stroke that folds within its own half-width is built from pieces (`folding`, ADR 1348); a
//! rule thinner than a device pixel is drawn at the coverage its area implies (`sub_pixel_rule`,
//! ADRs 0226 and 0268); a segment whose ink lies in two by two pixels is `crate::thin_segment`'s.

mod folding;
mod sub_pixel_rule;

use pdf_render::{Paint, Path, Stroke, Transform};

use crate::compose::carries_coverage_as_alpha;
use crate::shading::Extent;
use crate::surface::{Placed, ToDevice};
use crate::{CpuRasterError, CpuRasterizer, convert, scan, thin_segment};
use folding::{curves_may_fold, folding_stroke_outline};
use sub_pixel_rule::draw_sub_pixel_rule;

impl CpuRasterizer {
    /// Turns a dash pattern's zero-length dashes into marks, ISO 32000-2 §8.5.3.2.
    ///
    /// > This rule shall apply only to zero-length subpaths of the path being stroked, and
    /// > not to zero-length dashes in a dash pattern of a non-degenerate subpath. In the
    /// > latter case, the line caps shall always be painted, since their orientation is
    /// > determined by the direction of the underlying path
    ///
    /// Errata Collection 3 qualifies that sentence — "In the opaque imaging model, this"
    /// for "This", Issue #103 — and `pdf_render::degenerate`'s module comment carries the
    /// reading; the quotation above is `doc/md/`'s, which is what the quotation gate reads.
    ///
    /// `tiny-skia` paints such a cap, so a dotted line is *drawn* here without this — but it
    /// faces a square cap upright, because Skia's dasher loses the direction and its stroker
    /// says so: "since the zero length segment has no direction, set the orientation to
    /// upright as the default orientation". The clause says the direction is the path's, and
    /// on a diagonal dotted line the two answers cover different pixels. Doing the dashing
    /// here keeps the direction (see [`pdf_render::ZERO_DASH`]) and puts the answer in the
    /// crate both backends share.
    ///
    /// Returns `None` when the pattern holds no zero-length dash whose cap would show, which
    /// leaves the ordinary path untouched: `tiny-skia` does its own dashing and this
    /// allocates nothing.
    fn zero_length_dashes(
        geometry: &Path,
        stroke: &Stroke,
        (width, substitute): (f32, Option<Transform>),
        dots: &mut Path,
    ) -> Option<(Path, f32)> {
        let pattern = pdf_render::dashes_showing_direction(&stroke.dash_array, stroke.cap)?;
        let source = convert::path(geometry)?;
        let dash = tiny_skia::StrokeDash::new(pattern, stroke.dash_phase)?;
        // The resolution scale bounds how finely a curve is measured for arc length.
        // `tiny-skia` derives it from the transform for stroking; one is what it uses for a
        // path already in device-sized units, and a dash's *position* is what this needs
        // rather than the smoothness of its ends.
        let dashed = source.dash(&dash, 1.0)?;
        let split = pdf_render::split_dash_marks(
            &convert::from_skia_path(&dashed),
            stroke.cap,
            width,
            substitute,
        );
        dots.extend(split.dots.commands());
        Some((split.stroked, split.coverage))
    }

    /// Draws a stroked path, including the marks its own geometry has no length to make.
    ///
    /// Split out of [`CpuRasterizer::draw`] because ISO 32000-2 §8.5.3.2 turns one command
    /// into two draws — the subpaths that span a distance are stroked, and the ones that do
    /// not are *filled*, as circles `pdf-render` states rather than as whatever `tiny-skia`'s
    /// caps would have produced.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError`] for a path the rasteriser rejects or a paint this backend
    /// does not implement.
    #[expect(
        clippy::too_many_arguments,
        reason = "the fields of one display list command, passed through rather than \
                  regrouped: a struct here would be a second spelling of `Command::Stroke`"
    )]
    pub(crate) fn draw_stroke(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        path: &Path,
        transform: Transform,
        stroke: &Stroke,
        paint: &Paint,
        blend: tiny_skia::BlendMode,
        to_device: ToDevice,
        clip: scan::Clip<'_>,
    ) -> Result<(), CpuRasterError> {
        // §8.3.4 NOTE 3: a matrix with no inverse carries this path onto a line or a point, so
        // the *mark* is refused and the rest of the page is drawn. Asked before any geometry is
        // built, because none of it would be used, and asked through `pdf-render` so that all
        // three backends refuse the same marks (trap 2). `pdf_model` reports the count.
        let Some(page_to_path) = pdf_render::paint_space(transform) else {
            return Ok(());
        };
        let at = to_device.of(transform);
        // A straight stroke whose ink lies in two by two pixels — §8.5.3.2's round dot among them,
        // the same set at length zero, so it never reaches `split_degenerate` below — is painted
        // from its closed-form area before any outline, edge or pipeline is built for it. The
        // population and the reason are `pdf_render::thin_segment`'s, one copy for every backend
        // (trap 2), and the composition `thin_segment::draw`'s, on the page's own grid as the strip
        // planner asks (ADRs 1829, 1847, 1857).
        let (mark, site) = ((path, stroke, paint), (transform, to_device));
        let closed_form = self.anti_alias && blend == tiny_skia::BlendMode::SourceOver;
        if closed_form && thin_segment::draw(pixmap, mark, site, clip) {
            return Ok(());
        }
        // A dot inside a longer path is the same disc where it is the only mark in its pixels, and
        // `pdf_render::thin_dots` takes such dots out of the path: the rest is stroked below and the
        // dots painted after it, the order `split_degenerate`'s rest and circles are drawn in (ADR
        // 1875).
        let thin_dots = closed_form.then(|| thin_segment::dots(pixmap, mark, site, clip));
        let thin_dots = thin_dots.flatten();
        let path = thin_dots.as_ref().map_or(path, |thin| &thin.rest);
        let width = stroke.device_width(at);
        // §8.5.3.2's marks are circles and squares of the line's width, so a width under the
        // device's coverage quantum makes each of them a shape whose *area* is under the square
        // of it — a 0.2-unit dot is 0.03 of a pixel at scale 1 and this rasteriser drew nothing
        // at all for it. §10.7.4 forbids that by name, and `pdf_render::point_mark` is where the
        // substitution is decided, for all three backends at once (trap 2). Handing it the
        // transform is what asks for it; it is withheld on the same question as every other use
        // of the coverage-as-alpha identity.
        let substitute = carries_coverage_as_alpha(self.anti_alias, blend).then_some(at);
        // ISO 32000-2 §8.5.3.2's two rules about a stroke with no length. Neither is
        // Skia's answer: it paints a projecting square cap where the clause asks for
        // no output, it refuses a path that is only a `m` rather than drawing
        // nothing, and it faces a zero-length dash's square cap upright rather than
        // along the path. Both are decided in `pdf-render` so that the two backends
        // cannot answer them differently.
        let split = pdf_render::split_degenerate(path, stroke.cap, width, substitute);
        let geometry = split.as_ref().map_or(path, |s| &s.stroked);
        // §8.4.3.6's "the end cap is painted before the corner" at the vertex a closed subpath's
        // own close makes, decided in `pdf-render` for all three backends (trap 2). Skia's dasher
        // merges the first and last dash of a closed contour whenever both are on, which is
        // §8.4.3.4's answer for a dash the close cuts short and the wrong one for a dash that
        // finishes there.
        let opened = pdf_render::opened_where_a_dash_ends_at_the_close(
            geometry,
            &stroke.dash_array,
            stroke.dash_phase,
        );
        let geometry = opened.as_ref().unwrap_or(geometry);
        let mut dots = split.as_ref().map_or_else(Path::new, |s| s.dots.clone());
        // The two rules make marks of one width under one cap, so they are stated at one
        // coverage; whichever produced marks answers for both.
        let mut coverage = split.as_ref().map_or(1.0, |s| s.coverage);
        let (geometry, dashed) =
            match Self::zero_length_dashes(geometry, stroke, (width, substitute), &mut dots) {
                Some((remainder, dashed_coverage)) => {
                    coverage = dashed_coverage;
                    (remainder, true)
                }
                None => (geometry.clone(), false),
            };

        if !geometry.is_empty() {
            let converted = convert::path(&geometry).ok_or(CpuRasterError::InvalidPath)?;
            let mut style = convert::stroke(stroke, at);
            // The dashes have already been dispensed, so what is left is solid.
            if dashed {
                style.dash = None;
            }
            let (mut levels, mut scratch) = (None, None);
            let brush = self.mark_brush(
                (paint, blend),
                || ink_extent(&geometry, stroke, to_device.placed(transform)),
                (transform, page_to_path, to_device),
                (pixmap.width(), pixmap.height()),
                (&mut levels, &mut scratch),
            )?;
            let outline = SolidOutline::of(&converted, &style, at);
            let outlined = (&converted, &outline);
            let placed = to_device.placed(transform);
            if !draw_sub_pixel_rule(
                pixmap,
                (&geometry, &converted),
                (&style, stroke.cap),
                (placed, &outline),
                &brush,
                clip,
            ) && !draw_long_mitres(
                pixmap,
                (&geometry, &converted),
                (stroke, &style),
                placed,
                &brush,
                clip,
            ) && !draw_stroked_outline(pixmap, outlined, &style, site, &brush, clip)
            {
                scan::stroke(pixmap, &converted, &brush, &style, placed, clip);
            }
        }
        // The marks are *filled*, not stroked: §8.5.3.2 asks for "a filled circle
        // centred at the single point", and the stroking paint is what fills it.
        if !dots.is_empty()
            && let Some(converted) = convert::path(&dots)
        {
            let (mut levels, mut scratch) = (None, None);
            let mut brush = self.mark_brush(
                (paint, blend),
                || Some(Extent::Filled(&converted)),
                (transform, page_to_path, to_device),
                (pixmap.width(), pixmap.height()),
                (&mut levels, &mut scratch),
            )?;
            // The marks were stated by `pdf-render` above, so this is the area they gave up.
            brush.shader.apply_opacity(coverage);
            // Measured on the page's grid, so a window's or a strip's discs are the page's (ADR
            // 2148).
            scan::fill_stated(
                pixmap,
                (&converted, &scan::Exact::Unknown),
                &brush,
                tiny_skia::FillRule::Winding,
                to_device.placed(transform),
                clip,
            );
        }
        thin_segment::paint_dots(pixmap, thin_dots, paint, to_device, clip);
        Ok(())
    }

    /// The brush one of a stroke's marks is painted with: an opaque axial or radial ramp's own
    /// levels where [`crate::shading::ramp_shader`] draws the paint, one cell per device pixel over
    /// every pixel `ink` says the mark can reach (ADRs 2356, 2404), and [`CpuRasterizer::paint`]'s
    /// brush otherwise.
    ///
    /// `ink` is asked only of a shading, because a solid paint has no cells to bound and a stroke's
    /// extent walks its path. `levels` holds the ramp's cells and `scratch` a sampled shading's,
    /// each lent for as long as the brush.
    fn mark_brush<'a, 'e>(
        &self,
        (paint, blend): (&Paint, tiny_skia::BlendMode),
        ink: impl FnOnce() -> Option<Extent<'e>>,
        (transform, page_to_path, to_device): (Transform, Transform, ToDevice),
        target: (u32, u32),
        (levels, scratch): (
            &'a mut Option<tiny_skia::Pixmap>,
            &'a mut Option<tiny_skia::Pixmap>,
        ),
    ) -> Result<tiny_skia::Paint<'a>, CpuRasterError> {
        let ramp = match paint {
            Paint::Shading(shading) => ink().and_then(|extent| {
                crate::shading::ramp_shader(
                    shading,
                    extent,
                    (
                        to_device.placed(transform),
                        to_device.placed(shading.transform),
                    ),
                    target,
                    levels,
                )
            }),
            _ => None,
        };
        match ramp {
            Some(shader) => Ok(self.brush(shader, blend)),
            None => self.paint(
                paint,
                blend,
                page_to_path,
                to_device.of(Transform::IDENTITY),
                target,
                scratch,
            ),
        }
    }
}

/// The pixels a ramp's cells cover for a stroke of `path` under `stroke`, placed by `placed` (ADR
/// 2404): the stroked outline's device bounds on the page's grid, `pdf_render::stroked_bounds`'s,
/// which ISO 32000-2 §8.4.3's width, caps, joins and dashes bound term by term, and a reach past
/// them for the marks no outline at the stated width holds.
///
/// Those are the marks drawn one device pixel wide — §10.7.4's band under [`draw_sub_pixel_rule`]
/// and the library's hairline under `scan::stroke` — whose ink lies within the same multiple of
/// half *that* width that the clause's joins and caps allow at the stated one: half a width along
/// the path, `√2` halves at a projecting square cap's corner (§8.4.3.3), and the mitre limit's
/// halves at a mitre's tip (§8.4.3.5). The half pixel inside which a centre lies is added to it,
/// as [`Extent::Filled`]'s bounds add it. `None` for a path that names no point, which keeps the
/// gradient's answer.
fn ink_extent(path: &Path, stroke: &Stroke, placed: Placed) -> Option<Extent<'static>> {
    /// A projecting square cap's far corner sits `√2` half-widths from the endpoint.
    const SQUARE_CAP_CORNER: f32 = std::f32::consts::SQRT_2;
    let half_widths = match stroke.join {
        pdf_render::LineJoin::Miter => stroke.miter_limit.max(SQUARE_CAP_CORNER),
        pdf_render::LineJoin::Round | pdf_render::LineJoin::Bevel => SQUARE_CAP_CORNER,
    };
    Some(Extent::Stroked {
        ink: pdf_render::stroked_bounds(path, stroke, placed.page)?,
        reach: 0.5 + half_widths / 2.0,
    })
}

/// The verbs an axis-aligned rectangle's outline can take, above which [`draw_stroked_outline`]
/// stops asking whether it is one.
///
/// A **cost guard and not a condition**: `pdf_render::device_rectangle` decides whether a shape is
/// a rectangle, and this only decides whether asking is worth a path conversion. `tiny-skia`'s
/// stroker closes a butt-capped straight rule with a move, three lines and a close — five verbs —
/// and `the_outline_of_a_straight_rule_is_a_rectangle` pins that count, so the margin here is for a
/// stroker that spells the same rectangle a verb or two differently rather than for a shape with
/// more corners.
const RECTANGULAR_OUTLINE_VERBS: usize = 8;

/// One stroke's undashed outline, built by the stroker at most once however many constructions ask
/// for it.
///
/// `sub_pixel_rule::draw_rule_as_bands` and [`draw_stroked_outline`] both measure the outline `tiny_skia`'s
/// stroker makes of the same path, under the same style and at the same resolution, and the first
/// declines whenever the outline has an arc in it — every round-capped rule. Stroking it twice
/// was the largest single waste on the owner's `demo_heavy_engineering_A1.pdf`, a drawing of
/// 4 400 060 round-capped strokes each under a device pixel: `tiny_skia::Path::stroke` was 39.3 G
/// of the 217.6 G instructions `examples/callgrind_rasterise` counted for the page at 150 dpi,
/// half of them a second build of an outline already in hand (ADR 1813). The stroker is a pure
/// function of its three inputs, so the outline handed out is the one each construction built
/// before, and the pixels are those it drew.
struct SolidOutline<'a> {
    /// The path, in the space `at` maps from.
    path: &'a tiny_skia::Path,
    /// The stroke's style with its dashes withdrawn: both constructions stroke a dash-free path.
    solid: tiny_skia::Stroke,
    /// The resolution the stroker offsets curves at, `PathStroker::compute_resolution_scale`'s.
    scale: f32,
    /// The outline, once asked for; `None` inside where the stroker refuses the path.
    built: std::cell::OnceCell<Option<tiny_skia::Path>>,
}

impl<'a> SolidOutline<'a> {
    /// The outline of `path` stroked under `style` and drawn under `at`, not yet built.
    fn of(path: &'a tiny_skia::Path, style: &tiny_skia::Stroke, at: Transform) -> Self {
        let mut solid = style.clone();
        solid.dash = None;
        Self {
            path,
            solid,
            scale: tiny_skia::PathStroker::compute_resolution_scale(&convert::transform(at)),
            built: std::cell::OnceCell::new(),
        }
    }

    /// The outline, built on the first call; `None` where the stroker refuses the path.
    fn get(&self) -> Option<&tiny_skia::Path> {
        self.built
            .get_or_init(|| self.path.stroke(&self.solid, self.scale))
            .as_ref()
    }
}

/// ISO 32000-2 §8.4.3's stroked outline, filled — which is the shape §10.7.4's clipping paragraph
/// can meet as a *set* where a hairline and a library-internal fill cannot.
///
/// Returns `false` for a stroke at or under the coverage quantum, which is
/// [`draw_sub_pixel_rule`]'s, and for a path the stroker or the dasher refuses.
///
/// # Why a stroke needs this
///
/// §10.7.4 states the clip as a set of pixels intersected with "the set of pixels for the region
/// to be painted", and §8.5.4 as the intersection of the clipping path with "the object's
/// intrinsic shape". Neither sentence knows which operator painted the region. This backend did:
/// a *fill* has met its clip by `min` since ADR 0355, and a stroke went through
/// `tiny_skia::PixmapMut::stroke_path`, which hands the finished mask to its own `fill_path` and
/// multiplies. A boundary pixel covered `c` by the mark and `c` by a coincident clip was painted
/// `c²` — `crates/pdf-model/examples/coincident_edge_probe` is the ladder and
/// `render-cpu/tests/clip_intersection.rs` is the identity.
///
/// # Why it is not a second stroker
///
/// `scan::stroke`'s comment priced this as "choosing between duplicating the library's stroker and
/// contradicting its hairline", and the first half was wrong in ADR 0476's direction: **the
/// library already contains the pieces**. `tiny_skia::Path::stroke` is the same
/// `PathStroker` `stroke_path` calls, `tiny_skia::Path::dash` the same dasher, and `stroke_path`'s
/// own non-hairline branch is exactly the two of them followed by a `fill_path` under the non-zero
/// rule. So the mark drawn here is the mark the library drew, reached one call earlier so that
/// `scan::fill` can compose it — the same move `draw_long_mitres` already makes for the paths
/// §8.4.3.5 takes out of the stroker's hands.
///
/// # The second half: the hairline boundary is not the *library's*
///
/// The remaining half of that sentence — contradicting the hairline — is answered by never letting
/// the library choose. `tiny-skia` draws every stroke at or under one device pixel as a hairline,
/// which lays one pixel per step along the longer device axis and so carries `cos θ` of a turned
/// rule's area — 29.3% short at 45°, measured by `render-raster/examples/sub_pixel_marks` at every
/// thickness. That reads directly against "\[t]he area covered by painted pixels shall always be at
/// least as large as the area of the original shape". A boundary either backend can be given by
/// its own library is a boundary neither backend chose (trap 2), so this function takes every
/// width the raster can state a coverage for — down to [`pdf_render::unmeasurable_width`] — and
/// fills the outline the library's own stroker built. The hairline is reached only through
/// `scan::stroke`, where [`carries_coverage_as_alpha`] has already withdrawn every substitution
/// this module makes.
///
/// # And what falls out of it, because a stroke's mark is now a fill's
///
/// A butt-capped straight rule along a device axis has an outline that is *one axis-aligned
/// rectangle*, so ADR 0476's exact coverage — the product of a pixel's two overlaps, from
/// §10.7.4's own definition of a pixel — applies to it and did not before. That is the same rule
/// the clip region beside it is already measured by, which is what the subclause asks for in so
/// many words: the region "consists of the set of pixels that would be included by a fill
/// operation". `a_stroke_and_the_fill_of_its_outline_are_one_mark` is the scene, and it read three
/// levels apart before this.
fn draw_stroked_outline(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    (path, solid_outline): (&tiny_skia::Path, &SolidOutline<'_>),
    style: &tiny_skia::Stroke,
    (transform, to_device): (Transform, ToDevice),
    brush: &tiny_skia::Paint<'_>,
    clip: scan::Clip<'_>,
) -> bool {
    let placed = to_device.placed(transform);
    let at = placed.at;
    // ISO 32000-2 §10.7.4's substitutions own a stroke only where the raster cannot state its
    // coverage at all, which is one level of 255 and not one device pixel (ADR 1102). Everything
    // above that is a shape this backend can measure, and its outline is what it measures.
    if pdf_render::unmeasurable_width(at).is_some_and(|floor| style.width <= floor) {
        return false;
    }
    let scale = solid_outline.scale;
    let solid = &solid_outline.solid;
    // §8.4.3.6's pattern is measured along the path and the stroker knows nothing about it, which
    // is the order `stroke_path` dispenses them in too.
    let dashed;
    let path = match style.dash.as_ref() {
        None => path,
        Some(dash) => {
            let Some(cut) = path.dash(dash, scale) else {
                return false;
            };
            dashed = cut;
            &dashed
        }
    };
    // A curve bending more tightly than the half-width is stroked piece by piece (ADR 1348). The
    // question is not asked of one straight segment, which states no curve and is most strokes:
    // `issue12295.pdf` draws 65 859 of them.
    let straight = is_one_straight_segment(path);
    if !straight && let Some(pieces) = folding_stroke_outline(path, solid, scale) {
        scan::fill_stated(
            pixmap,
            (&pieces, &scan::Exact::Pieces),
            brush,
            tiny_skia::FillRule::Winding,
            placed,
            clip,
        );
        return true;
    }
    // An undashed path's outline is the one [`draw_rule_as_bands`] may already have built.
    let built;
    let outline = if style.dash.is_none() {
        solid_outline.get()
    } else {
        built = path.stroke(solid, scale);
        built.as_ref()
    };
    let Some(outline) = outline else {
        return false;
    };
    // On the page's grid and moved by the surface's corner, as a fill's rectangles are (ADR 2133).
    if outline.len() <= RECTANGULAR_OUTLINE_VERBS
        && let Some(rect) =
            pdf_render::device_rectangle(&convert::from_skia_path(outline), placed.page)
        && let Some(rect) = convert::to_skia_rect(rect)
    {
        scan::fill_rectangles(
            pixmap,
            (outline, &scan::Exact::One(rect).moved(placed.corner)),
            brush,
            scan::Place::of(placed),
            clip,
        );
        return true;
    }
    // The outline of one straight segment is a rectangle with a cap at each end, which covers no
    // point twice; every other outline may — at a join, where the inner offsets cross, or where two
    // subpaths' outlines meet — and a thin one shows no sign of it in its own coverage (ADR 1341).
    // The outline's vertices on the page's grid, so a strip or a window draws the page's pixels
    // (ADR 2133); the non-zero rule, which is what `stroke_path` fills an outline with.
    let exact = if straight {
        scan::Exact::Unknown
    } else {
        scan::Exact::Outline
    };
    scan::fill_stated(
        pixmap,
        (outline, &exact),
        brush,
        tiny_skia::FillRule::Winding,
        placed,
        clip,
    );
    true
}

/// Whether `path` is a single open subpath of one straight segment, whose stroked outline cannot
/// overlap itself.
fn is_one_straight_segment(path: &tiny_skia::Path) -> bool {
    let mut segments = path.segments();
    matches!(
        (segments.next(), segments.next(), segments.next()),
        (
            Some(tiny_skia::PathSegment::MoveTo(_)),
            Some(tiny_skia::PathSegment::LineTo(_)),
            None
        )
    )
}

/// The mitre-length ratio above which this library's stroker draws a bevel whatever `M` says.
///
/// `tiny-skia`'s stroker classifies a join before it consults the limit: `dot_to_angle_type` calls
/// a normals' dot product within `SCALAR_NEARLY_ZERO` — `1/4096` — of −1 `Nearly180` and sends it
/// to `do_blunt_or_clipped`. ISO 32000-2 §8.4.3.5's ratio is `1 / sin(φ/2)` and the library's own
/// comment gives `sin(φ/2) = sqrt((1 + dot) / 2)`, so that angle test is the ratio
/// `1 / sqrt((1/4096) / 2)` = 90.51 in disguise, and every sharper join is bevelled with the file's
/// limit unread.
///
/// **Measured as well as derived**, by `render-raster/examples/mitre_ladder`: this backend draws
/// the mitre at a ratio of 90.23 and nothing at all at 95.50, where the two graphics-device
/// backends draw the tip within a pixel of the clause's arithmetic at every rung.
const BEVELLED_BY_THE_STROKER: f32 = 90.51;

/// ISO 32000-2 §8.4.3.5's mitre, where [`BEVELLED_BY_THE_STROKER`] says the library will not draw
/// one: the outline bevelled by the stroker with `pdf-render`'s own mitres filled into it.
///
/// Returns `false` — leaving the caller's ordinary stroke to run — for every path that has no such
/// join, which is every path in the corpus bar the ones `long_mitre_census` counts. The condition
/// costs one comparison for an ordinary stroke: a join sharper than the ratio above is only drawn
/// where the file's own limit admits it, so a stroke whose `M` is under that ratio cannot have one
/// and `pdf_render::mitre_wedges` says so without walking the path.
///
/// # Why the two shapes are drawn as one path
///
/// A mitre join is a bevel join plus one triangle per join, and the triangle's base is the bevel's
/// own outer edge (`pdf_render::mitre`). Two draws of two shapes sharing an edge composite by
/// §11.3.7.3's union function and leave a seam along it — `doc/todo/11` item 5 — where coverage
/// accumulated inside one scan conversion adds. So the stroker's outline and the wedges go into one
/// `tiny_skia::Path` and are filled once, under the non-zero rule, which is what
/// `tiny_skia::PixmapMut::stroke_path` does with an outline anyway.
///
/// # What it declines
///
/// A stroke at or under one device pixel, where the library draws a hairline that has no joins at
/// all and where §10.7.4's own substitutions ([`draw_sub_pixel_rule`]) own the geometry — a wedge
/// under the coverage quantum is the case those two ADRs argue, not this one. And a path the
/// stroker itself refuses, or one whose wedges do not convert, both of which leave the ordinary
/// draw to report in its own way.
fn draw_long_mitres(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    geometry: (&Path, &tiny_skia::Path),
    (stroke, style): (&Stroke, &tiny_skia::Stroke),
    placed: Placed,
    brush: &tiny_skia::Paint<'_>,
    clip: scan::Clip<'_>,
) -> bool {
    let at = placed.at;
    // Asked first, and it answers a stroke whose stated limit cannot admit such a join without
    // walking the path — which is every stroke in the corpus bar eight. The width is the stroke's
    // in the path's own space, which is where §8.4.3.2 states it and where the stroker offsets;
    // `convert::stroke` puts the same number in `style.width`.
    let Some(wedges) = pdf_render::mitre_wedges(
        geometry.0,
        stroke,
        style.width / 2.0,
        BEVELLED_BY_THE_STROKER,
    ) else {
        return false;
    };
    // The same boundary [`draw_stroked_outline`] takes: under one level of the raster the mark is
    // §10.7.4's substitution and has no joins to draw, and above it the stroker builds the same
    // joins for a sub-pixel stroke as for any other (ADR 1102).
    if pdf_render::unmeasurable_width(at).is_some_and(|floor| style.width <= floor) {
        return false;
    }
    // A curve bending more tightly than the half-width is [`draw_stroked_outline`]'s, whose
    // pieces state §8.4.3.5's mitre themselves and have no stroker's bevel to make good.
    if curves_may_fold(geometry.1, style.width / 2.0) {
        return false;
    }
    let Some(wedges) = convert::path(&wedges) else {
        return false;
    };
    let mut bevelled = style.clone();
    bevelled.line_join = tiny_skia::LineJoin::Bevel;
    let scale = tiny_skia::PathStroker::compute_resolution_scale(&convert::transform(at));
    let Some(outline) = geometry.1.stroke(&bevelled, scale) else {
        return false;
    };
    let mut builder = tiny_skia::PathBuilder::new();
    builder.push_path(&outline);
    builder.push_path(&wedges);
    let Some(combined) = builder.finish() else {
        return false;
    };
    // On the page's grid, as the outline it replaces is (ADR 2148).
    scan::fill_stated(
        pixmap,
        (&combined, &scan::Exact::Unknown),
        brush,
        tiny_skia::FillRule::Winding,
        placed,
        clip,
    );
    true
}

/// Whether a curved stroke reaches [`area`] as one outline, the stroker's, rather than as ADR 1348's
/// convex pieces, which `scan::mask_fill` measures from their union's boundary unless that declines
/// the mark to `tiny-skia`'s converter (ADR 2122).
///
/// [`draw_stroked_outline`] cuts a stroke into pieces only where [`curves_may_fold`] says a curve
/// may bend more tightly than the half-width, and that test reads the path in its own space and
/// the width [`convert::stroke`] resolves, neither of which a strip changes — so asked here of the
/// page it is the draw's answer in every division (ADR 1847, trap 146). A dashed stroke is not
/// asked: its dashes cut the curves, and the test is the dashed pieces', so it keeps its rows.
///
/// [`area`]: crate::area
pub(crate) fn outlined_whole(stroke: &pdf_render::PlannedStroke<'_>) -> bool {
    if !stroke.stroke.dash_array.is_empty() {
        return false;
    }
    let Some(converted) = convert::path(stroke.path) else {
        return false;
    };
    let half = convert::stroke(stroke.stroke, stroke.at).width / 2.0;
    half.is_finite() && half > 0.0 && !curves_may_fold(&converted, half)
}

#[cfg(test)]
mod tests {

    use super::RECTANGULAR_OUTLINE_VERBS;

    use crate::convert;

    use pdf_render::Transform;

    /// What [`RECTANGULAR_OUTLINE_VERBS`] is a margin above, read off the library rather than
    /// assumed — and that the shape it spells really is the rectangle §10.7.4's closed form is
    /// about.
    ///
    /// A stroker that started spelling this outline with more verbs than the constant admits
    /// would take every axis-aligned rule back to the supersampled converter's quarter with no
    /// gate failing anywhere else, because the mark would still be the right shape.
    #[test]
    fn the_outline_of_a_straight_rule_is_a_rectangle() {
        let mut builder = tiny_skia::PathBuilder::new();
        builder.move_to(10.0, 20.0);
        builder.line_to(30.0, 20.0);
        let rule = builder.finish().expect("a two-point path");
        let stroke = tiny_skia::Stroke {
            width: 4.0,
            line_cap: tiny_skia::LineCap::Butt,
            ..tiny_skia::Stroke::default()
        };
        let outline = rule.stroke(&stroke, 1.0).expect("a stroked rule");
        assert!(
            outline.len() <= RECTANGULAR_OUTLINE_VERBS,
            "the outline takes {} verbs, above the constant's {RECTANGULAR_OUTLINE_VERBS}",
            outline.len()
        );
        let rect =
            pdf_render::device_rectangle(&convert::from_skia_path(&outline), Transform::IDENTITY)
                .expect("a butt-capped axis-aligned rule outlines to one rectangle");
        assert_eq!(
            (rect.min.x, rect.min.y, rect.max.x, rect.max.y),
            (10.0, 18.0, 30.0, 22.0)
        );
    }
}

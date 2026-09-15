//! ISO 32000-2 §10.7.4's clip is a **set of pixels**, and a set that contains a mark takes
//! nothing from it — on the backend `CLAUDE.md` principle 2 makes the correctness oracle.
//!
//! > For clipping, the clipping region consists of the set of pixels that would be included by
//! > a fill operation. Subsequent painting operations shall affect a region that is the
//! > intersection of the set of pixels defined by the clipping region with the set of pixels for
//! > the region to be painted.
//!
//! §8.5.4 states the same of the shape — "[t]he effective shape is the intersection of the
//! object's intrinsic shape with the clipping path" — so where the mark lies inside the region,
//! the intersection is the mark, boundary pixels included.
//!
//! **This backend has met a clip by `min` since ADR 0355, and `min` is not enough**: it is exact
//! where the two boundaries coincide **and both are measured the same way**, and they are not.
//! A clip stating a rectangle is measured by §10.7.4's own closed form (ADR 0476) where a mark
//! `render_cpu::area` declines falls back to the library's converter and its boundary pixel is
//! rounded to a quarter (ADR 1082); the quarter that rounded up is then cut back to the clip's
//! exact figure and the quarter that rounded down is not restored. `MaskCache::cuts_nothing`
//! answers the clause by not stating such a clip at all (ADR 1095), which is what
//! `render_raster::scene`'s own `cuts_nothing` does on the other backend (ADR 1088).
//!
//! # The scene is `bug1844576.pdf`'s, in miniature
//!
//! A `1 w` rectangle stroke inside a `/BBox` clip its own outline exactly fills — an annotation
//! border, which is what the corpus states thousands of. The transform carries that document's
//! own horizontal stretch of `1.0000001`, because that hair is load-bearing: it puts
//! `pdf_render::band_substitute_width` a unit in the last place under `pdf_render::thinnest_line`,
//! and a `1 w` rule then fell between the two tests and reached `tiny_skia::PixmapMut::
//! stroke_path`, where a clip multiplies rather than intersecting. That floor is ADR 1095's other
//! half and `pdf_render::sub_pixel`'s own tests pin it.
//!
//! # What the numbers here are, and how they were calibrated (trap 13)
//!
//! Every figure is the outline's own area in whole device pixels, computed from the geometry
//! below and not from the backend. Both halves of the rule were planted back off in turn, to
//! confirm this file names the defect rather than passing either way — the readings are in
//! `doc/history/1081-*`. The 8x rung is the control: there the stroke is eight device pixels
//! wide, no §10.7.4 substitution is in play, and `render_cpu::area` measures the outline exactly,
//! so the clip costs nothing whatever the rule says.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a backend that cannot draw one stroked rectangle on a small page is \
              the failure this file reports, and the sums are over a raster this file sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Clip, Color, Command, DisplayList, FillRule, LineCap, LineJoin, Paint, Path,
    PathCommand, Point, Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// The page every mark below is drawn on.
const PAGE: Size = Size {
    width: 10.0,
    height: 8.15,
};

/// `bug1844576.pdf`'s own appearance matrix, to the two hairs that matter.
///
/// The **stretch** is one unit in the last place above one, which is what parts the two width
/// statements and is the whole of this file's first half.
///
/// The **offset** is there with the page's own height of `8.15` so that no side of the outline
/// lands on a quarter of a device pixel. `tiny-skia`'s converter — which `render_cpu::area`
/// declines this ring to, the ring having a same-wound crossing at the close (ADR 1082) — states
/// a boundary pixel's coverage to the nearest quarter, and a side that *is* a quarter is stated
/// exactly by both converters and by neither's error. Trap 13 found that the hard way: this
/// file's first draft put every side on `.75`, and it passed with the rule planted off.
const WITNESS: Transform = Transform {
    a: 1.000_000_1,
    b: 0.0,
    c: 0.0,
    d: 1.0,
    e: 0.15,
    f: 0.0,
};

/// The stroke's path: a rectangle whose coordinates are exact halves, so that the outline
/// `+/- 0.5` deposits around it is stated in `f32` without a rounding of its own and coincides
/// with [`CONTAINING`] to the bit. [`WITNESS`] and the page's height are what carry it off the
/// pixel grid.
const PATH: (f32, f32, f32, f32) = (2.0, 2.0, 7.0, 5.0);

/// The clip: exactly the outline [`PATH`] stroked one unit wide deposits, so that all four of its
/// sides are coincident with the mark's own and none of them cuts.
const CONTAINING: (f32, f32, f32, f32) = (1.5, 1.5, 7.5, 5.5);

/// A clip whose left edge stands a whole unit inside the outline's, so that it cuts.
///
/// What it removes is the outline's left bar and the ends of its top and bottom bars: the whole
/// of `1.5 <= x < 2.5` under the ring, which is one unit across and four tall — **4.0** of the
/// outline's 16.
const CUTTING: (f32, f32, f32, f32) = (2.5, 1.5, 7.5, 5.5);

/// The outline's own area, in whole device pixels at the page's own scale.
///
/// [`PATH`] stroked one unit wide is a ring: an outer rectangle 6 x 4 and an inner one 4 x 2, so
/// `24 - 8`.
const AREA: f64 = 16.0;

/// How far the clipped reading may sit from the unclipped one, as a fraction of the area.
///
/// §10.7.4's identity is exact — the same mark, the same converter, one of the two readings with
/// a composition that may take nothing — so this is eight-bit coverage's own slack and nothing
/// else: a level of 255 over the outline's boundary pixels.
const IDENTICAL: f64 = 0.001;

/// How much of [`CUTTING`]'s removal the backend must at least show, in whole device pixels.
///
/// `min` over-states an intersection where both coverages are fractional in one pixel, so the
/// removal is not owed exactly; what is asserted is that the clip was *applied*, at half the
/// geometry's own figure. A containment test that answered `true` for every clip would show
/// nothing here at all.
const AT_LEAST_CUT: f64 = 4.0 / 2.0;

/// How far *above* its own area the unclipped outline may read, as a fraction of it.
///
/// **Not a rounding, and the figure is the point.** `render_cpu::area` declines this ring — the
/// stroker closes it with a same-wound crossing, which is ADR 1082's own condition — so its
/// boundary pixels come off `tiny-skia`'s converter rounded to a quarter, and at 1x that states
/// **5.4%** more ink than the geometry. Over-stating is the side §10.7.4's third sentence asks
/// for — "[t]he area covered by painted pixels shall always be at least as large as the area of
/// the original shape" — which is why this file holds a floor at the area and a ceiling here,
/// and why the clause's identity above is checked against the *unclipped reading* rather than
/// against the geometry: a clip may take nothing from what the converter stated, whatever that
/// was.
const AT_MOST_OVER: f64 = 0.10;

/// A rectangle as a closed path.
fn rectangle((x0, y0, x1, y1): (f32, f32, f32, f32)) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    path
}

/// [`PATH`] stroked one unit wide, under `clip` where one is given.
fn scene(clip: Option<(f32, f32, f32, f32)>) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    let clip = clip.map(|region| {
        list.add_clip(Clip {
            path: rectangle(region),
            transform: WITNESS,
            fill_rule: FillRule::NonZero,
            parent: None,
        })
        .expect("one clip fits in a display list")
    });
    list.push(Command::Stroke {
        path: Arc::new(rectangle(PATH)),
        transform: WITNESS,
        stroke: Stroke {
            width: 1.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 10.0,
            dash_array: Vec::new(),
            dash_phase: 0.0,
            adjust: false,
        },
        paint: Paint::Solid(Color::BLACK),
        clip,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The ink on one raster, in whole device pixels of full coverage.
fn ink(raster: &Raster) -> f64 {
    let sum: u64 = raster
        .data
        .chunks_exact(4)
        .map(|pixel| 765 - u64::from(pixel[0]) - u64::from(pixel[1]) - u64::from(pixel[2]))
        .sum();
    #[expect(
        clippy::cast_precision_loss,
        reason = "765 times a raster of at most 80 x 64 pixels is a small integer"
    )]
    let sum = sum as f64;
    sum / 765.0
}

/// The oracle's ink for one scene at one scale, scale-normalised so that the two rungs are
/// comparable with the geometry's own area.
fn drawn(list: &DisplayList, scale: f32) -> f64 {
    let target = TargetSpec::for_page(list, scale, 1 << 20).expect("a target for a 10 x 8.15 page");
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the oracle draws one stroked rectangle");
    ink(&raster) / (f64::from(scale) * f64::from(scale))
}

/// The clause: a clip that contains a mark leaves the mark exactly as it was, at both scales.
///
/// Read against the **unclipped** reading of the same scene rather than against the geometry,
/// which is what §10.7.4's `S n C = S` actually says: the intersection is the mark, whatever the
/// converter measured the mark to be. Before ADR 1095 the 1x rung read 12.86 against 16.87.
#[test]
fn a_clip_that_contains_a_mark_takes_nothing_from_it() {
    for scale in [1.0_f32, 8.0] {
        let bare = drawn(&scene(None), scale);
        let clipped = drawn(&scene(Some(CONTAINING)), scale);
        assert!(
            (clipped - bare).abs() <= AREA * IDENTICAL,
            "at {scale}x the oracle drew {clipped} of an outline it draws {bare} of unclipped,              and the clip is exactly that outline's own rectangle — §10.7.4's intersection with              a region that contains the mark is the mark"
        );
    }
}

/// And the other half, which is what stops the rule above from being too generous: a clip that
/// **does** cut takes what it cuts.
///
/// Without this, a containment test that answered `true` for every clip would pass the test above
/// and lose nothing in it. [`CUTTING`] takes 4 of the outline's 16, and the oracle is held to at
/// least half of that — see [`AT_LEAST_CUT`] for why not to the figure itself.
#[test]
fn a_clip_that_cuts_still_cuts() {
    for scale in [1.0_f32, 8.0] {
        let bare = drawn(&scene(None), scale);
        let cut = drawn(&scene(Some(CUTTING)), scale);
        assert!(
            cut <= bare - AT_LEAST_CUT,
            "at {scale}x the oracle drew {cut} of an outline it draws {bare} of unclipped, under \
             a clip that removes 4 of it: a clip that cuts may not be left off the mark"
        );
    }
}

/// And §10.7.4's third sentence, which is what the identity above is allowed to preserve.
///
/// The outline is never drawn short of its own area, and [`AT_MOST_OVER`] says how far over the
/// converter this ring falls to may go.
#[test]
fn the_unclipped_outline_is_never_drawn_short_of_its_own_area() {
    for scale in [1.0_f32, 8.0] {
        let bare = drawn(&scene(None), scale);
        assert!(
            (AREA..=AREA * (1.0 + AT_MOST_OVER)).contains(&bare),
            "at {scale}x the oracle drew {bare} of an unclipped outline whose area is {AREA}"
        );
    }
}

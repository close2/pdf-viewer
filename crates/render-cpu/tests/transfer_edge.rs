//! What §11.7.5.2 asks for at the antialiased edge of a transferred object, and where this
//! tree's pre-composite application of §10.5's transfer function diverges from it.
//!
//! # The reading this measures
//!
//! ISO 32000-2 §11.7.5.2 chooses the transfer function at a point by the topmost object covering
//! it, and defines that object by *shape* rather than by full coverage:
//!
//! > The topmost object at any point shall be defined to be the topmost elementary object in the
//! > entire page stack that has a nonzero object shape value ( f j) at that point (that is, for
//! > which the point is inside the object).
//!
//! A pixel an antialiased edge covers partially has a **nonzero** shape there, so it is inside
//! that object and takes that object's function — although the pixel's colour is a blend of that
//! object's colour with the backdrop's. §11.7.5.3's NOTE says when the function runs:
//!
//! > This differs from the current halftone and transfer function, whose values are used only
//! > when all colour compositing has been completed and rasterization is being performed.
//!
//! So the clause composites the raw colours first and maps the finished pixel once: at an edge
//! pixel the clause draws `transfer(blend(object, backdrop))`. This tree applies §10.5's transfer
//! to the object's colour *before* compositing (`pdf-model`'s `fill_paint`, `stroke_paint` and
//! image sample map), so at the same pixel it draws `blend(transfer(object), backdrop)`. The two
//! agree wherever the object fully covers the pixel — the interior — and diverge at the edge by
//! as much as the transfer bends the colour.
//!
//! # Why this is a measurement rather than a fix
//!
//! Closing the gap needs the per-pixel transfer-identity channel `doc/todo/13` and ADR 1125
//! derive: composite raw, carry per pixel the index of the topmost opaque object's function, and
//! map once at the end. That is a second channel in every backend's target, and both backends
//! here composite through `tiny-skia`, which exposes no per-pixel "topmost object" hook — so the
//! index is a separate rasterisation pass, not a field on an existing one. `examples/`'
//! `transfer_function_census` measures the population that would move at **one** document, a
//! fully opaque image with no translucent overlap, so the change has no corpus witness and this
//! is the fixture (trap 8, trap 13) that stands in for one: it plants the divergence and confirms
//! the raster shows it, so the channel's later arrival is legible against it. When the channel
//! lands, [`the_edge_pixel_diverges_from_the_clause`]'s asserted pipeline value becomes the
//! clause value and this file's arithmetic says so in one place.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a panic with a message is the intended failure mode, and the arithmetic \
              is over a raster this file sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Raster,
    Rasterizer, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything here.
const GENEROUS: u64 = 1 << 30;

/// The page every scene is drawn on, at one device pixel per user unit.
const PAGE: Size = Size {
    width: 24.0,
    height: 16.0,
};

/// Three levels of 255: `tiny-skia`'s edge coverage and one solid composite, the tolerance
/// `edge_coverage.rs` derives for the same arithmetic.
const TOLERANCE: f32 = 2.0 / 255.0;

/// The object's own grey, before any transfer. One value on all three channels.
const OBJECT: f32 = 0.25;

/// §10.5's transfer, applied per component: the permutation `issue6931_reduced.pdf` states in
/// spirit — an inversion, which no renderer that ignores the function can fake. `map(0.25)` is
/// `0.75`, far enough from the input that a pixel drawn with the wrong ordering is unmistakable.
fn transfer(value: f32) -> f32 {
    1.0 - value
}

/// A closed axis-aligned rectangle.
fn rectangle(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    path
}

/// One opaque grey fill on a white page. The right edge sits at device x = 12.5, so column 12 is
/// covered a half — the antialiased edge this file measures — while column 7 is an interior pixel.
///
/// `grey` is the colour handed to the paint: `OBJECT` for the raw scene the clause composites,
/// `transfer(OBJECT)` for the scene this tree draws, the transfer already inside the colour.
fn scene(grey: f32) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    list.push(Command::Fill {
        path: Arc::new(rectangle(3.0, 3.0, 12.5, 13.0)),
        // Device y counts down from the top of the page; x maps straight through.
        transform: Transform::new(1.0, 0.0, 0.0, -1.0, 0.0, PAGE.height),
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color {
            r: grey,
            g: grey,
            b: grey,
            a: 1.0,
        }),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The value of one channel at a pixel, in `0.0..=1.0`. The scene is grey, so any channel does.
fn channel(raster: &Raster, x: u32, y: u32) -> f32 {
    let at = ((y * raster.width + x) as usize) * 4;
    f32::from(raster.data[at]) / 255.0
}

/// Renders one scene at one device pixel per unit.
fn render(list: &DisplayList) -> Raster {
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .rasterize(list, target)
        .expect("a solid fill is supported")
}

/// An interior pixel, fully inside the object.
const INTERIOR: (u32, u32) = (7, 7);
/// The half-covered edge pixel: the object's right edge is device x = 12.5.
const EDGE: (u32, u32) = (12, 7);

/// At a fully covered pixel the two orderings agree, and the transfer reaches it.
///
/// Where coverage is 1.0 the composite *is* the object's colour, so `transfer(blend) =
/// transfer(object) = blend(transfer(object))`: the interior is where this tree's per-object
/// application is already exactly §11.7.5.2's per-point one. The test is that the transferred
/// scene's interior is the transfer of the raw scene's interior — that the function is applied at
/// all, and applied correctly where the clause and the code cannot differ.
#[test]
fn the_interior_pixel_is_the_transfer_of_the_object() {
    let raw = render(&scene(OBJECT));
    let transferred = render(&scene(transfer(OBJECT)));

    let raw_interior = channel(&raw, INTERIOR.0, INTERIOR.1);
    let transferred_interior = channel(&transferred, INTERIOR.0, INTERIOR.1);

    assert!(
        (raw_interior - OBJECT).abs() <= TOLERANCE,
        "the raw interior should be the object's own colour {OBJECT}, was {raw_interior}"
    );
    assert!(
        (transferred_interior - transfer(raw_interior)).abs() <= TOLERANCE,
        "the interior should carry transfer({raw_interior}) = {}, was {transferred_interior}",
        transfer(raw_interior)
    );
}

/// At the antialiased edge the pipeline draws `blend(transfer(object), backdrop)` where the
/// clause asks for `transfer(blend(object, backdrop))`, and the gap is large.
///
/// The raw scene's edge pixel is the composite the clause would then map: `blend(object,
/// backdrop)`. §11.7.5.2 maps it once, giving `transfer` of it. This tree's transferred scene
/// instead composites the already-transferred colour, giving `blend(transfer(object), backdrop)`.
/// The test asserts both — that the pipeline is the second, and that the second is nowhere near
/// the clause's first — so the divergence is a measured quantity and not a claim.
#[test]
fn the_edge_pixel_diverges_from_the_clause() {
    let raw = render(&scene(OBJECT));
    let transferred = render(&scene(transfer(OBJECT)));

    let raw_edge = channel(&raw, EDGE.0, EDGE.1);
    let pipeline_edge = channel(&transferred, EDGE.0, EDGE.1);

    // The edge is genuinely partial: strictly between the object and the white backdrop.
    assert!(
        OBJECT + TOLERANCE < raw_edge && raw_edge < 1.0 - TOLERANCE,
        "EDGE must be an antialiased pixel; raw coverage put it at {raw_edge}"
    );

    // The clause maps the finished composite once: transfer(blend(object, backdrop)).
    let clause_edge = transfer(raw_edge);

    // What the pipeline actually draws: the composite of the transferred colour with the same
    // backdrop, blend(transfer(object), backdrop). The blend weight is the raw edge's coverage,
    // recovered from `raw_edge = coverage·OBJECT + (1 − coverage)·1`.
    let coverage = (1.0 - raw_edge) / (1.0 - OBJECT);
    let pipeline_expected = coverage * transfer(OBJECT) + (1.0 - coverage) * 1.0;
    assert!(
        (pipeline_edge - pipeline_expected).abs() <= TOLERANCE,
        "the pipeline should draw blend(transfer(object), backdrop) = {pipeline_expected} at the \
         edge, was {pipeline_edge}"
    );

    // The measured gap: half a unit at a half-covered edge under an inverting transfer. This is
    // what the per-pixel transfer channel (ADR 1125, doc/todo/13) closes; until it lands, the
    // edge carries the pipeline value above rather than `clause_edge`.
    let gap = (pipeline_edge - clause_edge).abs();
    assert!(
        gap > 0.25,
        "the edge should diverge from §11.7.5.2's transfer(composite) = {clause_edge} by much \
         more than antialiasing tolerance; measured gap was {gap}"
    );
}

/// With no transfer in force the edge is the plain composite, unchanged: the control.
///
/// A scene whose colour is `OBJECT` and which states no transfer is what every one of the 973
/// corpus documents without a `/TR` is, and its edge pixel must be `blend(object, backdrop)` with
/// nothing applied to it — the value both the clause and this tree agree on when the function is
/// the identity.
#[test]
fn without_a_transfer_the_edge_is_unchanged() {
    let raw = render(&scene(OBJECT));
    let raw_edge = channel(&raw, EDGE.0, EDGE.1);

    let coverage = (1.0 - raw_edge) / (1.0 - OBJECT);
    let plain = coverage * OBJECT + (1.0 - coverage) * 1.0;
    assert!(
        (raw_edge - plain).abs() <= TOLERANCE,
        "the control edge should be the plain composite {plain}, was {raw_edge}"
    );
}

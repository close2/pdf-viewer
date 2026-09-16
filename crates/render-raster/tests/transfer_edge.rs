//! §11.7.5.2 at an antialiased edge, measured on **both** backends: they agree today, and a
//! one-backend fix would split them.
//!
//! `render-cpu/tests/transfer_edge.rs` planted this measurement for the CPU oracle in the
//! one-thousand-one-hundred-and-eighteenth session; this is its raster half. The point of the
//! pair is `CLAUDE.md` principle 2: the two backends must agree, so the per-pixel transfer
//! channel ADR 1125 designs has to land in both at once. Until it does, both backends draw the
//! same *pre-composite* ordering at a transferred edge — this file measures that they do, so the
//! divergence from §11.7.5.2 is a cross-backend fact rather than a CPU-only one, and so the
//! witness flips on both backends together when the channel arrives.
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
//! that object and takes that object's function on the *composited* colour — §11.7.5.3's NOTE
//! puts the mapping "only when all colour compositing has been completed". So at an edge pixel the
//! clause draws `transfer(blend(object, backdrop))`. This tree applies §10.5's transfer to a
//! colour *before* compositing (`pdf-model`'s `fill_paint`, `stroke_paint` and image sample map),
//! so at the same pixel each backend draws `blend(transfer(object), backdrop)`. The two agree
//! wherever the object fully covers the pixel — the interior — and diverge at the edge by as much
//! as the transfer bends the colour.
//!
//! # Why this is a measurement rather than a fix
//!
//! Closing the gap needs the per-pixel transfer channel ADR 1125 and `doc/todo/13` derive, and it
//! must land in both backends or they disagree at every transferred edge. That the raster backend
//! is the `raster-gpu` compute rasteriser rather than a `tiny-skia` one does not stand in the way
//! of *agreement* — the channel's per-pixel index is a pure function of geometry and opacity, so
//! both backends can apply the identical final map to the read-back raster (this backend already
//! runs such passes; see `QuorraRasterizer::rasterize`). What is not yet buildable is the per-mark
//! carrier the map reads: §11.7.5.2's function is a property of an elementary object, so it rides
//! on `Command::Fill`/`Command::Image`, whose construction sites span crates this round does not
//! own. ADR 1125 records the corrected pricing; this test is the raster half of its fixture.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code: an explanatory panic is the intended failure mode, and the arithmetic \
              is over a raster this file sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Raster,
    Rasterizer, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;
use render_raster::QuorraRasterizer;

/// Pixel budget for a target; far above anything here.
const GENEROUS: u64 = 1 << 30;

/// The page every scene is drawn on, at one device pixel per user unit.
const PAGE: Size = Size {
    width: 24.0,
    height: 16.0,
};

/// Per-backend tolerance: three levels of 255 for the CPU oracle's arithmetic, widened to six for
/// the raster backend's straight-alpha read-back and premultiply round trip.
const CPU_TOLERANCE: f32 = 3.0 / 255.0;
/// See [`CPU_TOLERANCE`].
const RASTER_TOLERANCE: f32 = 6.0 / 255.0;
/// Cross-backend tolerance at one antialiased edge pixel: the localised bar `headless_quorra`
/// holds the two backends to, expressed in `0.0..=1.0`. Far below the half-unit gap to the clause.
const AGREEMENT: f32 = 6.0 / 255.0;

/// The object's own grey, before any transfer. One value on all three channels.
const OBJECT: f32 = 0.25;

/// §10.5's transfer, applied per component: an inversion, the permutation `issue6931_reduced.pdf`
/// states in spirit — no renderer that ignores the function can fake it. `transfer(0.25)` is
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

/// An interior pixel, fully inside the object.
const INTERIOR: (u32, u32) = (7, 7);
/// The half-covered edge pixel: the object's right edge is device x = 12.5.
const EDGE: (u32, u32) = (12, 7);

/// The raster backend, or a panic naming why the whole suite should fail rather than skip.
///
/// The discipline is `headless_quorra`'s (ADR 0004): a skipped suite reports success while
/// verifying nothing, so this fails loudly where no adapter is present instead.
fn raster() -> QuorraRasterizer {
    QuorraRasterizer::new_headless().unwrap_or_else(|e| {
        panic!(
            "no adapter available for raster: {e}\n\
             Install a Vulkan driver (mesa-vulkan-drivers for a software one). These tests do \
             not skip, because a skipped suite reports success while verifying nothing."
        )
    })
}

/// Renders one scene through a rasteriser at one device pixel per unit.
fn render<R: Rasterizer>(rasterizer: &mut R, list: &DisplayList) -> Raster {
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("valid target");
    rasterizer
        .rasterize(list, target)
        .unwrap_or_else(|_| panic!("a solid fill is supported"))
}

/// The raw and pipeline edge channels a backend draws: `(raw_edge, pipeline_edge)`.
///
/// `raw_edge` is the plain composite `blend(object, backdrop)` — what the clause would then map
/// once; `pipeline_edge` is the transferred scene's edge, `blend(transfer(object), backdrop)` —
/// what this tree draws, the transfer already inside the colour before compositing.
fn edges<R: Rasterizer>(rasterizer: &mut R) -> (f32, f32) {
    let raw = render(rasterizer, &scene(OBJECT));
    let transferred = render(rasterizer, &scene(transfer(OBJECT)));
    (
        channel(&raw, EDGE.0, EDGE.1),
        channel(&transferred, EDGE.0, EDGE.1),
    )
}

/// The raster backend draws the same pre-composite ordering the CPU oracle does at a transferred
/// edge, and diverges from §11.7.5.2 by the same half unit.
///
/// The clause maps the finished composite once — `transfer(blend(object, backdrop))` — and this
/// tree instead composites the already-transferred colour — `blend(transfer(object), backdrop)`.
/// The test asserts, on the raster backend, that the pipeline draws the second and that the second
/// is nowhere near the clause's first, exactly as `render-cpu`'s fixture asserts for the oracle.
#[test]
fn the_raster_edge_diverges_from_the_clause_as_the_oracle_does() {
    let (raw_edge, pipeline_edge) = edges(&mut raster());

    // The edge is genuinely partial: strictly between the object and the white backdrop.
    assert!(
        OBJECT + RASTER_TOLERANCE < raw_edge && raw_edge < 1.0 - RASTER_TOLERANCE,
        "EDGE must be an antialiased pixel; raw coverage put it at {raw_edge}"
    );

    // What the pipeline actually draws: the composite of the transferred colour with the same
    // backdrop, at this backend's own edge coverage, recovered from `raw_edge`.
    let coverage = (1.0 - raw_edge) / (1.0 - OBJECT);
    let pipeline_expected = coverage * transfer(OBJECT) + (1.0 - coverage) * 1.0;
    assert!(
        (pipeline_edge - pipeline_expected).abs() <= RASTER_TOLERANCE,
        "the raster edge should draw blend(transfer(object), backdrop) = {pipeline_expected}, \
         was {pipeline_edge}"
    );

    // The clause maps the finished composite once; the measured gap is half a unit at a
    // half-covered edge under an inverting transfer. This is what the per-pixel transfer channel
    // (ADR 1125, doc/todo/13) closes on both backends at once.
    let clause_edge = transfer(raw_edge);
    let gap = (pipeline_edge - clause_edge).abs();
    assert!(
        gap > 0.25,
        "the raster edge should diverge from §11.7.5.2's transfer(composite) = {clause_edge} by \
         much more than antialiasing tolerance; measured gap was {gap}"
    );
}

/// The two backends agree on the transferred edge today — so the fix must move both, and a
/// one-backend change would split them.
///
/// This is the linchpin of the deferral `CLAUDE.md` principle 2 requires: the CPU oracle and the
/// raster backend draw the same pipeline value at the edge, within the localised bar
/// `headless_quorra` holds them to. A channel added to one backend alone would break this
/// agreement at every transferred edge; a correct channel flips both to the clause value together.
#[test]
fn the_two_backends_agree_on_the_transferred_edge() {
    let (_, cpu_edge) = edges(&mut CpuRasterizer::new());
    let (_, raster_edge) = edges(&mut raster());

    assert!(
        (cpu_edge - raster_edge).abs() <= AGREEMENT,
        "the two backends must agree at the transferred edge (principle 2): oracle {cpu_edge}, \
         raster {raster_edge}"
    );
}

/// At a fully covered pixel the two orderings agree, so both backends carry the transfer of the
/// object there — the interior is where this tree's per-object application is already §11.7.5.2's
/// per-point one, and where a channel would change nothing.
#[test]
fn the_interior_agrees_and_carries_the_transfer() {
    let mut cpu = CpuRasterizer::new();
    let mut raster = raster();

    let cpu_interior = channel(
        &render(&mut cpu, &scene(transfer(OBJECT))),
        INTERIOR.0,
        INTERIOR.1,
    );
    let raster_interior = channel(
        &render(&mut raster, &scene(transfer(OBJECT))),
        INTERIOR.0,
        INTERIOR.1,
    );

    assert!(
        (cpu_interior - transfer(OBJECT)).abs() <= CPU_TOLERANCE,
        "the oracle interior should carry transfer({OBJECT}) = {}, was {cpu_interior}",
        transfer(OBJECT)
    );
    assert!(
        (raster_interior - transfer(OBJECT)).abs() <= RASTER_TOLERANCE,
        "the raster interior should carry transfer({OBJECT}) = {}, was {raster_interior}",
        transfer(OBJECT)
    );
    assert!(
        (cpu_interior - raster_interior).abs() <= AGREEMENT,
        "the two backends must agree at the interior: oracle {cpu_interior}, raster \
         {raster_interior}"
    );
}

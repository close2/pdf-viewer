//! §11.7.5.2 at an antialiased edge, measured on **both** backends: they draw the clause's value
//! and they agree on it.
//!
//! `render-cpu/tests/transfer_edge.rs` is the other half. The point of the pair is `CLAUDE.md`
//! principle 2: the two backends must agree, so the per-pixel transfer channel ADR 1125 designs
//! had to land in both at once — and this file is what says it did. Session 1118 planted the CPU
//! half measuring the *divergence*, session 1137 planted this one measuring that the divergence
//! was the same on both, and session 1148 built `pdf_render::resolve_transfers`, after which both
//! files measure the clause.
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
//! that object and takes that object's function on the *composited* colour — §11.7.5.3's NOTE puts
//! the mapping "only when all colour compositing has been completed". So at an edge pixel the
//! clause draws `transfer(blend(object, backdrop))`, and at an interior pixel `transfer(object)`.
//!
//! # Why this is a fixture and not a corpus page
//!
//! `pdf-model`'s `examples/transfer_function_census` counts the population that can move at
//! **one** document — `issue6931_reduced.pdf`, a fully opaque image with no translucent mark over
//! it — so the edge case has no corpus witness and this stands in for one (trap 8, trap 13).
//!
//! # Why the two backends can agree at all
//!
//! The index §11.7.5.2 chooses is a pure function of geometry and opacity, independent of colour,
//! so it need not thread through either backend's compositing: `pdf_render::resolve_transfers`
//! rasterises the marks' shapes through whichever backend is asking and applies the identical map
//! to its own read-back. This backend is the `raster-gpu` compute rasteriser and the oracle is
//! `tiny-skia`; what they share is the pass, not the pipeline.

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
    Rasterizer, Size, TargetSpec, TransferBuilder, TransferMap, Transform,
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
/// holds the two backends to, expressed in `0.0..=1.0`.
const AGREEMENT: f32 = 6.0 / 255.0;

/// The object's own grey, before any transfer. One value on all three channels.
const OBJECT: f32 = 0.25;

/// §10.5's transfer, applied per component: an inversion, the permutation `issue6931_reduced.pdf`
/// states in spirit — no renderer that ignores the function can fake it. `transfer(0.25)` is
/// `0.75`, far enough from the input that a pixel drawn with the wrong ordering is unmistakable.
fn transfer(value: f32) -> f32 {
    1.0 - value
}

/// The same inversion as [`pdf_render::TransferMap`] carries it: one table per component,
/// evaluated at every eight-bit input, which is what `pdf_model` builds out of the file's
/// functions.
fn inverting() -> TransferMap {
    let mut table = [0_u8; 256];
    for (index, entry) in table.iter_mut().enumerate() {
        *entry = 255_u8.saturating_sub(u8::try_from(index).unwrap_or(255));
    }
    TransferMap::from_samples([table, table, table])
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
/// `transferred` is what `pdf_model` does with a `/TR` in force: the colour stays *raw* and the
/// function rides on the mark, which is §11.7.5.2's channel.
fn scene(transferred: bool) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    let mark = Command::Fill {
        path: Arc::new(rectangle(3.0, 3.0, 12.5, 13.0)),
        // Device y counts down from the top of the page; x maps straight through.
        transform: Transform::new(1.0, 0.0, 0.0, -1.0, 0.0, PAGE.height),
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color {
            r: OBJECT,
            g: OBJECT,
            b: OBJECT,
            a: 1.0,
        }),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    };
    if transferred {
        let mut channel = TransferBuilder::default();
        channel.push(Some(Arc::new(inverting())), &mark);
        if let Some(channel) = channel.finish() {
            list.set_transfers(channel);
        }
    }
    list.push(mark);
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

/// The raw and transferred edge channels a backend draws: `(raw_edge, transferred_edge)`.
fn edges<R: Rasterizer>(rasterizer: &mut R) -> (f32, f32) {
    let raw = render(rasterizer, &scene(false));
    let transferred = render(rasterizer, &scene(true));
    (
        channel(&raw, EDGE.0, EDGE.1),
        channel(&transferred, EDGE.0, EDGE.1),
    )
}

/// The raster backend draws §11.7.5.2's value at a transferred edge: the function of the
/// composite, not the composite of the function.
#[test]
fn the_raster_edge_takes_the_clause_value() {
    let (raw_edge, transferred_edge) = edges(&mut raster());

    // The edge is genuinely partial: strictly between the object and the white backdrop.
    assert!(
        OBJECT + RASTER_TOLERANCE < raw_edge && raw_edge < 1.0 - RASTER_TOLERANCE,
        "EDGE must be an antialiased pixel; raw coverage put it at {raw_edge}"
    );

    // §11.7.5.2 with §11.7.5.3's NOTE: composite first, map once.
    let clause_edge = transfer(raw_edge);
    assert!(
        (transferred_edge - clause_edge).abs() <= RASTER_TOLERANCE,
        "the raster edge should carry §11.7.5.2's transfer(composite) = {clause_edge}, was \
         {transferred_edge}"
    );

    // And nowhere near what the pre-composite ordering drew, so this cannot pass by accident.
    let coverage = (1.0 - raw_edge) / (1.0 - OBJECT);
    let pre_composite = coverage * transfer(OBJECT) + (1.0 - coverage) * 1.0;
    assert!(
        (transferred_edge - pre_composite).abs() > 0.25,
        "the raster edge must not be the pre-composite ordering's {pre_composite}"
    );
}

/// The two backends agree on the transferred edge, which is what makes the change shippable.
///
/// `CLAUDE.md` principle 2: a channel added to one backend alone would split them at every
/// transferred edge. They share `pdf_render::resolve_transfers` and nothing else about how the
/// pixel got there, so this is a real cross-backend claim rather than a shared implementation
/// asserting about itself.
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

/// At a fully covered pixel the composite is the object's colour, so both backends carry the
/// transfer of the object there — and agree.
#[test]
fn the_interior_agrees_and_carries_the_transfer() {
    let mut cpu = CpuRasterizer::new();
    let mut raster = raster();

    let cpu_interior = channel(&render(&mut cpu, &scene(true)), INTERIOR.0, INTERIOR.1);
    let raster_interior = channel(&render(&mut raster, &scene(true)), INTERIOR.0, INTERIOR.1);

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

/// A list that states no transfer is byte for byte the list it was before the channel existed, on
/// this backend as on the oracle: no channel, no shape pass, no map.
#[test]
fn a_list_without_a_transfer_is_untouched() {
    let mut raster = raster();
    let control = render(&mut raster, &scene(false));
    let again = render(&mut raster, &scene(false));
    assert!(scene(false).transfers().is_none());
    assert!(
        control.data == again.data,
        "a list with no transfer must rasterise identically every time"
    );
}

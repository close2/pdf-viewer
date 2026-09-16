//! What §11.7.5.2 asks for at the antialiased edge of a transferred object, and that this tree
//! now draws it.
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
//! pixel the clause draws `transfer(blend(object, backdrop))`, and at an interior pixel — where
//! the composite *is* the object's colour — `transfer(object)`, which is the same number a
//! pre-composite application would have produced. The two orderings agree over the interior and
//! diverge at the edge by as much as the transfer bends the colour.
//!
//! # Why this is a fixture and not a corpus page
//!
//! `pdf-model`'s `examples/transfer_function_census` counts the population that can move at
//! **one** document — `issue6931_reduced.pdf`, a fully opaque image with no translucent mark over
//! it — so the edge case has no corpus witness at all and this stands in for one (trap 8, trap
//! 13). Session 1118 planted it measuring the *divergence*; session 1148 built
//! `pdf_render::resolve_transfers` and it measures the clause instead, in one place.
//! `render-raster/tests/transfer_edge.rs` is its other half: the same three numbers on the other
//! backend, and that the two agree (`CLAUDE.md` principle 2).

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a panic with a message is the intended failure mode, and the arithmetic \
              is over a raster this file sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Raster,
    Rasterizer, Size, TargetSpec, TransferBuilder, TransferMap, Transform,
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
/// function rides on the mark, which is §11.7.5.2's channel. `false` is the control, the scene a
/// page that states no transfer builds.
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

/// At a fully covered pixel the composite *is* the object's colour, so the clause's mapping of it
/// is the transfer of the object.
#[test]
fn the_interior_pixel_is_the_transfer_of_the_object() {
    let raw = render(&scene(false));
    let transferred = render(&scene(true));

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

/// At the antialiased edge the pixel takes §11.7.5.2's value: the function of the *composite*.
///
/// The raw scene's edge pixel is `blend(object, backdrop)`, which is what the clause then maps —
/// the object's shape there is nonzero, so the point is inside it and takes its function. Before
/// `pdf_render::resolve_transfers` this tree composited the already-transferred colour and drew
/// `blend(transfer(object), backdrop)` instead: 0.875 against the clause's 0.375 at a half-covered
/// edge under an inverting transfer, which is the half-unit gap sessions 1118 and 1137 measured
/// and this asserts is gone.
#[test]
fn the_edge_pixel_takes_the_clause_value() {
    let raw = render(&scene(false));
    let transferred = render(&scene(true));

    let raw_edge = channel(&raw, EDGE.0, EDGE.1);
    let pipeline_edge = channel(&transferred, EDGE.0, EDGE.1);

    // The edge is genuinely partial: strictly between the object and the white backdrop.
    assert!(
        OBJECT + TOLERANCE < raw_edge && raw_edge < 1.0 - TOLERANCE,
        "EDGE must be an antialiased pixel; raw coverage put it at {raw_edge}"
    );

    // §11.7.5.2 with §11.7.5.3's NOTE: composite first, map once.
    let clause_edge = transfer(raw_edge);
    assert!(
        (pipeline_edge - clause_edge).abs() <= TOLERANCE,
        "the edge should carry §11.7.5.2's transfer(composite) = {clause_edge}, was {pipeline_edge}"
    );

    // And it is nowhere near what the pre-composite ordering drew, so this cannot pass by
    // accident on a pipeline that applies the function to the colour instead.
    let coverage = (1.0 - raw_edge) / (1.0 - OBJECT);
    let pre_composite = coverage * transfer(OBJECT) + (1.0 - coverage) * 1.0;
    assert!(
        (pipeline_edge - pre_composite).abs() > 0.25,
        "the edge must not be the pre-composite ordering's {pre_composite}"
    );
}

/// With no transfer in force the edge is the plain composite, unchanged: the control.
///
/// A scene whose colour is `OBJECT` and which states no transfer is what every one of the 973
/// corpus documents without a `/TR` is, and its edge pixel must be `blend(object, backdrop)` with
/// nothing applied to it. Unchanged by the channel, which is what
/// [`pdf_render::DisplayList::transfers`] being `None` buys those pages.
#[test]
fn without_a_transfer_the_edge_is_unchanged() {
    let raw = render(&scene(false));
    let raw_edge = channel(&raw, EDGE.0, EDGE.1);

    let coverage = (1.0 - raw_edge) / (1.0 - OBJECT);
    let plain = coverage * OBJECT + (1.0 - coverage) * 1.0;
    assert!(
        (raw_edge - plain).abs() <= TOLERANCE,
        "the control edge should be the plain composite {plain}, was {plain}"
    );
    assert!(
        render(&scene(false)).data == raw.data,
        "a list with no transfer must rasterise identically every time"
    );
}

/// The channel is `None` where nothing states a function, so no second rasterisation runs.
///
/// The cost rule this tree binds itself to (`CLAUDE.md` principle 2): a page that states no
/// transfer must be exactly the page it was before the channel existed.
#[test]
fn a_list_without_a_transfer_carries_no_channel() {
    assert!(scene(false).transfers().is_none());
    assert!(scene(true).transfers().is_some());
    // The identity is not a function worth a pass: `/TR /Identity` states one and asks for
    // nothing, which is what `TransferMap::is_identity` is read for.
    let mut identity = [0_u8; 256];
    for (index, entry) in identity.iter_mut().enumerate() {
        *entry = u8::try_from(index).unwrap_or(255);
    }
    assert!(TransferMap::from_samples([identity, identity, identity]).is_identity());
}

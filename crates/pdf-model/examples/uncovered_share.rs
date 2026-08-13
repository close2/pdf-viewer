//! How much of what a page draws early still shines through what it draws later.
//!
//! `doc/todo/_scan-conversion.md` lists four departures from §10.7.4, and the second is that
//! "the painted area is *not* always at least the shape's" — the consequence of the first, which
//! is that both backends anti-alias. On most pages that costs an edge pixel here and there. On a
//! page that states one region as *many* opaque fills it is a different thing: every internal
//! boundary falls inside some device pixel, each mark composites at its own coverage, and
//! §11.3.7.3's union of two halves is three quarters. A quarter of whatever lies under the region
//! survives, and where that is a dark rule the survivor is a seam a person can see.
//!
//! This measures it on any page, without needing to know which marks abut. A page-covering opaque
//! fill is spliced in at command `index` and the page is drawn twice, once with that fill black
//! and once white; everything else is identical, so the two rasters differ at a pixel by
//! **exactly** the share of the spliced layer that the marks after it failed to cover.
//!
//! ```sh
//! cargo run --release -p pdf-model --example uncovered_share -- <file.pdf> <page> <index> [scale]...
//! ```
//!
//! `index` chooses the depth the question is asked at: `0` asks what survives of the medium
//! itself, and a larger one asks what survives of a rule the page draws before its fills.
//!
//! **Only the interior is counted**, and that is what makes the number mean something. A pixel on
//! the outer edge of what the marks cover is *supposed* to be partly painted — that is
//! anti-aliasing working — so a page-wide average would be dominated by legitimate edges and
//! would rank a page of text as the worst thing in the corpus. A pixel every one of whose eight
//! neighbours is also touched is inside the region the marks describe, and anything of the layer
//! below still showing there is the departure and nothing else. The columns, per scale:
//!
//! - **interior** — pixels with all eight neighbours touched.
//! - **lost** — the area, in whole pixels, that §10.7.4 says those marks would have painted there
//!   and did not.
//! - **mean**, **worst** — that loss per interior pixel.
//!
//! A page whose regions are stated once reads a mean in the thousandths as soon as its marks are
//! more than a few pixels across — page 100 of ISO 32000-2 reads 0.0105 at 2× and 0.0017 at 4×,
//! and 0.4203 at 1× only because eight-point type at 1× has almost no interior to average over. A
//! drawing exported as tens of thousands of filled polygons reads two tenths at page scale and
//! roughly halves with every doubling of it, because a mark's interior grows with the scale and
//! its boundary does not — which is why such a seam goes away when the reader magnifies the page.

#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "a measurement example: its output is the point, a bad argument or an unopenable \
              file should stop it loudly rather than be handled, and the indices below walk a \
              raster this program sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Rasterizer,
    TargetSpec, Transform,
};

/// `list` with an opaque page-covering fill of `colour` spliced in before command `index`.
fn witnessed(list: &DisplayList, index: usize, colour: Color) -> DisplayList {
    let bounds = list.page_bounds();
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(bounds.min.x, bounds.min.y)));
    path.push(PathCommand::LineTo(Point::new(bounds.max.x, bounds.min.y)));
    path.push(PathCommand::LineTo(Point::new(bounds.max.x, bounds.max.y)));
    path.push(PathCommand::LineTo(Point::new(bounds.min.x, bounds.max.y)));
    path.push(PathCommand::Close);
    let witness = Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(colour),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    };

    // Spliced by cutting the list rather than rebuilding it: the clip and soft-mask tables are
    // referred to by index, so a list built afresh would renumber them and the first clipped
    // command would be refused by name.
    let mut built = list.clone();
    let rest = built.split_off_commands(index.min(list.command_count()));
    built.push(witness);
    for command in rest {
        built.push(command);
    }
    built
}

/// Draws `list` at `scale`.
fn draw(list: &DisplayList, scale: f32) -> pdf_render::Raster {
    let target = TargetSpec::for_page(list, scale, 1 << 34).expect("a target");
    render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("drawn")
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().expect("a pdf");
    let page: usize = arguments
        .next()
        .expect("a page number")
        .parse()
        .expect("a number");
    let index: usize = arguments
        .next()
        .expect("a command index")
        .parse()
        .expect("a number");
    let scales: Vec<f32> = arguments
        .map(|scale| scale.parse().expect("a number"))
        .collect();
    let scales = if scales.is_empty() {
        vec![1.0, 2.0, 4.0, 8.0]
    } else {
        scales
    };

    let document =
        pdf_syntax::Document::open(std::fs::read(&path).expect("readable")).expect("a PDF");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(page.saturating_sub(1)).expect("that page");
    let list = pdf_model::interpret(&document, &page).display_list;
    let dark = witnessed(&list, index, Color::BLACK);
    let light = witnessed(&list, index, Color::WHITE);

    println!(
        "{path}: {} commands, witness spliced before command {index}",
        list.command_count()
    );
    println!(
        "{:>7}  {:>12}  {:>12}  {:>10}  {:>8}  {:>8}",
        "scale", "raster", "interior", "lost", "mean", "worst"
    );
    for scale in scales {
        let dark = draw(&dark, scale);
        let light = draw(&light, scale);
        let (width, height) = (light.width as usize, light.height as usize);
        let showing: Vec<f64> = light
            .data
            .chunks_exact(4)
            .zip(dark.data.chunks_exact(4))
            .map(|(over, under)| f64::from(over[0].saturating_sub(under[0])) / 255.0)
            .collect();

        let (mut interior, mut unpainted, mut worst) = (0_u64, 0.0_f64, 0.0_f64);
        for y in 1..height.saturating_sub(1) {
            for x in 1..width.saturating_sub(1) {
                let touched = |dy: usize, dx: usize| {
                    showing[(y + dy).saturating_sub(1) * width + (x + dx).saturating_sub(1)] < 1.0
                };
                if !(0..3).all(|dy| (0..3).all(|dx| touched(dy, dx))) {
                    continue;
                }
                let here = showing[y * width + x];
                interior = interior.saturating_add(1);
                unpainted += here;
                worst = worst.max(here);
            }
        }
        let denominator = if interior == 0 { 1.0 } else { interior as f64 };
        println!(
            "{scale:>7.2}  {:>5} x {:<4}  {interior:>12}  {unpainted:>10.1}  {:>8.4}  {worst:>8.4}",
            light.width,
            light.height,
            unpainted / denominator,
        );
    }
}

//! Public-raster regression for the 1.01 near-similarity cutoff.
//!
//! §8.4.3.2 allows up to two device pixels of achieved-width difference from pixel-grid
//! placement. A 1000-unit stroke under scale(1.009, 1) is still routed through the scalar
//! near-similarity branch, but scaling the transverse width by `max_stretch` makes a horizontal
//! band 1009 pixels wide instead of 1000. The 9-pixel discrepancy is larger than that allowance.
//! The L path avoids the single-segment line shortcut; samples are on straight segments and far
//! from the join, so this asserts band width only.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::indexing_slicing,
    clippy::expect_used,
    reason = "bounded raster indexing and explicit setup failures"
)]

#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../tools/conformance/test-support/gpu_lock_hook.rs"]
mod gpu_lock_hook;

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_raster::QuorraRasterizer;

const WIDTH: u32 = 3000;
const HEIGHT: u32 = 2000;

fn list(path: Path, transform: Transform) -> DisplayList {
    let mut list = DisplayList::new(Size {
        width: WIDTH as f32,
        height: HEIGHT as f32,
    });
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform,
        stroke: Stroke {
            width: 1000.0,
            adjust: false,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 10.0,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

fn horizontal_l() -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(200.0, 1000.0)));
    path.push(PathCommand::LineTo(Point::new(2600.0, 1000.0)));
    path.push(PathCommand::LineTo(Point::new(2600.0, 1600.0)));
    path
}

fn vertical_l() -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(1200.0, 200.0)));
    path.push(PathCommand::LineTo(Point::new(1200.0, 1800.0)));
    path.push(PathCommand::LineTo(Point::new(1800.0, 1800.0)));
    path
}

fn draw(list: &DisplayList, rasterizer: &mut QuorraRasterizer) -> Raster {
    let target = TargetSpec::for_page(list, 1.0, 1 << 28).expect("bounded target");
    let raster = rasterizer
        .rasterize(list, target)
        .expect("stroke rasterizes");
    assert_eq!((raster.width, raster.height), (WIDTH, HEIGHT));
    raster
}

fn covered_rows(raster: &Raster, x: usize) -> usize {
    (0..raster.height as usize)
        .filter(|&y| {
            let i = (y * raster.width as usize + x) * 4;
            raster.data[i + 3] > 127 && raster.data[i] < 128
        })
        .count()
}

fn covered_columns(raster: &Raster, y: usize) -> usize {
    (0..raster.width as usize)
        .filter(|&x| {
            let i = (y * raster.width as usize + x) * 4;
            raster.data[i + 3] > 127 && raster.data[i] < 128
        })
        .count()
}

#[test]
fn near_similarity_large_stroke_preserves_transverse_width_within_two_pixels() {
    gpu_lock_hook::install();
    let mut renderer = QuorraRasterizer::with_options(&raster_gpu::Options {
        coverage: raster_gpu::Coverage::Cpu,
        ..render_raster::options()
    })
    .expect("CPU coverage available");

    eprintln!("adapter: {}", renderer.adapter_description());

    // Independent identity control: a 1000-user-unit stroke at unit scale covers 1000 rows.
    let identity = draw(&list(horizontal_l(), Transform::IDENTITY), &mut renderer);
    let identity_rows = covered_rows(&identity, 1000);
    assert!(
        (998..=1002).contains(&identity_rows),
        "identity control should be 1000 rows ±2, got {identity_rows}"
    );

    let near_similarity = Transform::scale(1.009, 1.0);
    let horizontal = draw(&list(horizontal_l(), near_similarity), &mut renderer);
    let horizontal_rows = covered_rows(&horizontal, 1009);

    // Orthogonal control: the same CTM scales a vertical stroke's normal by 1.009.
    let vertical = draw(&list(vertical_l(), near_similarity), &mut renderer);
    let vertical_columns = covered_columns(&vertical, 1000);
    assert!(
        (1007..=1011).contains(&vertical_columns),
        "vertical transverse width is 1000 × 1.009 = 1009 ±2, got {vertical_columns}"
    );
    eprintln!(
        "identity={identity_rows}, horizontal={horizontal_rows}, vertical={vertical_columns}"
    );
    assert!(
        (998..=1002).contains(&horizontal_rows),
        "horizontal transverse width is unchanged by x scaling; expected 1000 rows ±2, got {horizontal_rows}"
    );
}

fn near_similarity_pdf() -> Vec<u8> {
    let content = "1.009 0 0 1 0 0 cm 1000 w 200 1000 m 2600 1000 l 2600 1600 l S\n";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 3000 2000] /Resources << >> /Contents 4 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{content}endstream", content.len()),
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    bytes
}

#[test]
fn a_pdf_stroke_under_near_similarity_keeps_its_horizontal_width() {
    gpu_lock_hook::install();
    let document = pdf_syntax::Document::open(near_similarity_pdf()).expect("literal PDF opens");
    assert!(!document.was_recovered(), "fixture xref is exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpreted = pdf_model::interpret(&document, &page);
    assert!(interpreted.is_complete(), "{:?}", interpreted.unsupported);
    let target = TargetSpec::for_page(&interpreted.display_list, 1.0, 1 << 28).expect("target");
    let reference = render_cpu::CpuRasterizer::new()
        .rasterize(&interpreted.display_list, target)
        .expect("CPU reference draws");
    assert_eq!((reference.width, reference.height), (WIDTH, HEIGHT));
    let reference_rows = covered_rows(&reference, 1009);
    assert!(
        (998..=1002).contains(&reference_rows),
        "independent width1000 oracle: CPU has {reference_rows}"
    );

    let mut renderer = QuorraRasterizer::with_options(&raster_gpu::Options {
        coverage: raster_gpu::Coverage::Cpu,
        ..render_raster::options()
    })
    .expect("CPU coverage lane is available");
    eprintln!("adapter: {}", renderer.adapter_description());
    let actual = draw(&interpreted.display_list, &mut renderer);
    let actual_rows = covered_rows(&actual, 1009);
    eprintln!("parsed PDF: CPU width={reference_rows}, raster width={actual_rows}");
    assert!(
        (998..=1002).contains(&actual_rows),
        "horizontal width1000 ±2 under x-only scaling; got {actual_rows}"
    );
}

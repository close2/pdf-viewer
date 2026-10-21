//! §14.11.2.1's crop removes pixels; it cannot recolour the pixels it keeps.
//! The straight-alpha boundary uses the same nearest rounding on both routes (ADR 2183).
use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Medium, Paint, Path, PathCommand, Point,
    Rasterizer as _, Rect, Size, TargetSpec, Transform,
};
use render_raster::QuorraRasterizer;
use std::sync::Arc;

#[test]
fn cropping_preserves_the_straight_colour_of_surviving_pixels() {
    let mut path = Path::new();
    for command in [
        PathCommand::MoveTo(Point::new(0.0, 0.0)),
        PathCommand::LineTo(Point::new(8.0, 0.0)),
        PathCommand::LineTo(Point::new(8.0, 8.0)),
        PathCommand::LineTo(Point::new(0.0, 8.0)),
        PathCommand::Close,
    ] {
        path.push(command);
    }
    let mut list = DisplayList::new(Size {
        width: 8.0,
        height: 8.0,
    });
    list.push(Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color {
            r: 0.5,
            g: 0.25,
            b: 0.75,
            a: 0.5,
        }),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("small target");
    let mut backend = QuorraRasterizer::new_headless()
        .expect("real adapter")
        .with_medium(Medium::NONE);
    eprintln!("cropped alpha: {}", backend.adapter_description());
    let whole = backend.rasterize(&list, target).expect("whole scene");
    list.set_content_clip(Rect::from_corners(
        Point::new(0.0, 0.0),
        Point::new(4.0, 8.0),
    ));
    assert!(
        pdf_render::crop_area(&list, target).is_some(),
        "must exercise the crop route"
    );
    let cropped = backend.rasterize(&list, target).expect("cropped scene");
    if let Some(directory) = std::env::var_os("PDFVIEWER_ALPHA_PICTURES") {
        let directory = std::path::Path::new(&directory);
        std::fs::create_dir_all(directory).expect("picture directory");
        for (name, raster) in [("whole", &whole), ("cropped", &cropped)] {
            let file =
                std::fs::File::create(directory.join(format!("{name}.png"))).expect("picture file");
            let mut encoder = png::Encoder::new(file, raster.width, raster.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .expect("PNG header")
                .write_image_data(&raster.data)
                .expect("PNG pixels");
        }
    }
    let mut translucent = 0;
    for (index, (before, after)) in whole
        .data
        .chunks_exact(4)
        .zip(cropped.data.chunks_exact(4))
        .enumerate()
    {
        if index % 8 < 4 {
            assert_eq!(before, after, "surviving pixel {index}");
            translucent += usize::from(before[3] > 0 && before[3] < 255);
        } else {
            assert_eq!(after, [0, 0, 0, 0], "removed pixel {index}");
        }
    }
    assert_eq!(translucent, 32, "the witness must reach demultiply");
}

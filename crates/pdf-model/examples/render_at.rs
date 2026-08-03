//! Renders one page at a chosen scale and writes a PNG.
//!
//! `doc/todo/00-ambiguous-bucket.md` step 6 measures ink at rising resolution to find the
//! geometry a page states. Doing that with *this* renderer as well as with a reference is what
//! separates "our scan conversion differs" from "we draw different shapes".
//!
//! `cargo run -p pdf-model --example render_at -- <file.pdf> <page> <scale> <out.png>`

#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    reason = "a diagnostic binary: its output is the point, and a bad argument or an \
              unopenable file should stop it loudly rather than be handled"
)]
fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("a pdf");
    let page: usize = args
        .next()
        .expect("a page number")
        .parse()
        .expect("a number");
    let scale: f32 = args.next().expect("a scale").parse().expect("a number");
    let out = args.next().expect("an output png");

    let document =
        pdf_syntax::Document::open(std::fs::read(&path).expect("readable")).expect("a PDF");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(page.saturating_sub(1)).expect("that page");
    let interpretation = pdf_model::interpret(&document, &page);
    let list = interpretation.display_list;
    let target = pdf_render::TargetSpec::for_page(&list, scale, 1 << 34).expect("a target");
    let mut rasterizer = render_cpu::CpuRasterizer::new();
    let raster = <render_cpu::CpuRasterizer as pdf_render::Rasterizer>::rasterize(
        &mut rasterizer,
        &list,
        target,
    )
    .expect("drawn");
    println!("{}x{}", raster.width, raster.height);
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, raster.width, raster.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("header");
        writer.write_image_data(&raster.data).expect("data");
    }
    std::fs::write(out, png).expect("written");
}

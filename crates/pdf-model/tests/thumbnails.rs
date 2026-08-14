//! ISO 32000-2 §12.3.4's thumbnail images, over the corpus.
//!
//! A `/Thumb` is the producer's own miniature of a page, which makes it the rarest thing a
//! corpus offers: **a second statement about what a page looks like, made by the program that
//! made the page**. Every other check in this tree compares us against a specification or
//! against another reader; this one compares us against the file's own picture of itself.
//!
//! It is a weak instrument on purpose. A thumbnail may be stale, may be 76 pixels wide, and its
//! producer resampled it with a filter nobody states — so the comparison here is a coarse
//! luminance grid and a printed number, and the assertion is only that every thumbnail the
//! corpus states decodes. The numbers are printed because a page whose miniature looks nothing
//! like our render is worth a person's eye, and no other gate would ever say so.

use std::path::{Path, PathBuf};

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above an A4 page at 72 dpi.
const GENEROUS: u64 = 1 << 30;

/// Side of the coarse grid a raster is reduced to before comparing.
///
/// Eight is chosen to be far below the smallest thumbnail in the corpus, so that both sides of
/// the comparison are *reductions* and neither is being magnified into agreement.
const GRID: usize = 8;

/// The pdf.js corpus, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    files.sort();
    Some(files)
}

/// Every `/Thumb` in the corpus decodes, and each is compared with our render of its page.
#[expect(
    clippy::too_many_lines,
    reason = "one measurement over the corpus, whose counts read better together than split \
              across helpers that each take five arguments"
)]
#[test]
fn every_thumbnail_decodes_and_is_compared_with_the_page_it_stands_for() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let artefacts = Path::new(env!("CARGO_TARGET_TMPDIR")).join("thumbnails");
    std::fs::create_dir_all(&artefacts).expect("writable");

    let mut stating = 0usize;
    let mut decoded = 0usize;
    let mut failed = Vec::new();
    let mut forbidden_space = Vec::new();
    let mut wrong_subtype = Vec::new();
    let mut differences = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let pages = pdf_model::Pages::new(&document);
        let Some(page) = pages.get(0) else {
            continue;
        };
        let Some(thumbnail) = pdf_model::thumbnail::read(&document, &page.dict) else {
            continue;
        };
        stating = stating.saturating_add(1);
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let thumbnail = match thumbnail {
            Ok(thumbnail) => thumbnail,
            Err(error) => {
                failed.push(format!("{name}: {error}"));
                continue;
            }
        };
        decoded = decoded.saturating_add(1);
        if !thumbnail.permitted_colour_space {
            forbidden_space.push(name.clone());
        }
        if !thumbnail.permitted_subtype {
            wrong_subtype.push(name.clone());
        }

        // Our own render of the same page, at the thumbnail's own scale, reduced to the same
        // coarse grid. Only pages we claim to draw completely take part: a page missing a font
        // would be compared against a miniature that has one, and the difference would say
        // nothing about either.
        let interpretation = pdf_model::interpret(&document, &page);
        if !interpretation.is_complete() {
            continue;
        }
        let scale = f32::from(u16::try_from(thumbnail.image.width).unwrap_or(u16::MAX))
            / interpretation.display_list.page_size.width.max(1.0);
        let Ok(target) = TargetSpec::for_page(&interpretation.display_list, scale, GENEROUS) else {
            continue;
        };
        let Ok(ours) = CpuRasterizer::new().rasterize(&interpretation.display_list, target) else {
            continue;
        };
        side_by_side(&artefacts, &name, &thumbnail.image, &ours);
        let theirs = grid(
            &thumbnail.image.data,
            thumbnail.image.width,
            thumbnail.image.height,
        );
        let ours = grid(&ours.data, ours.width, ours.height);
        let mean = ours
            .iter()
            .zip(theirs.iter())
            .map(|(a, b)| f64::from(a.abs_diff(*b)))
            .sum::<f64>()
            / f64::from(u16::try_from(GRID.saturating_mul(GRID)).unwrap_or(u16::MAX));
        differences.push((mean, name, thumbnail.image.width, thumbnail.image.height));
    }

    differences.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut by_name = differences.clone();
    by_name.sort_by(|a, b| a.1.cmp(&b.1));
    println!("{stating} documents state a /Thumb on page one, {decoded} decode");
    println!("  colour space outside the clause's three forms: {forbidden_space:?}");
    println!("  a /Subtype that is not /Image: {wrong_subtype:?}");
    println!("  artefacts under {}", artefacts.display());
    println!("  mean coarse difference against our own render, worst first:");
    for (mean, name, width, height) in &differences {
        println!("    {mean:6.1}  {name} ({width}x{height})");
    }

    assert_eq!(stating, 11, "documents stating a thumbnail on page one");
    assert!(
        failed.is_empty(),
        "thumbnails that did not decode: {failed:?}"
    );
    assert_eq!(
        forbidden_space,
        ["issue19517.pdf"],
        "thumbnails whose colour space is none of the clause's three forms"
    );

    // A ratchet on the *disagreements*, not on the numbers: each of these four has been looked
    // at, and in every case the thumbnail or the instrument is what is wrong, never the page.
    //
    //  - `issue11144_reduced.pdf` — a file pdf.js cut down from a larger one. The miniature is
    //    of the page as it was, and shows tables our render's page no longer has.
    //  - `issue19326.pdf` — a 1x1 thumbnail, which states one average colour and cannot agree
    //    with anything.
    //  - `issue19517.pdf` — the other 1x1 thumbnail, comparable since session 486 made its page
    //    draw (the reduced-resolution JPEG 2000 decode, ADR 0321), and looked at then: the
    //    miniature is the page's own orange (254,39,0) and our render's top pixel matches it at
    //    (255,40,0). What disagrees is the instrument — our render at the thumbnail's scale is
    //    one pixel wide and 1.33 tall, so its second row is two-thirds white canvas — which is
    //    the same class of statement a 1x1 comparison makes: none.
    //  - `transparency_group.pdf` — the thumbnail draws four petals; the page draws two
    //    overlapping ellipses, and **all five renderers including us draw the ellipses**
    //    (oracle: ambiguous, our worst mean 0.87 against a bound of 1.00). A stale miniature.
    //
    // So the finding this list records is about the instrument: where a thumbnail and a render
    // disagree, it has been the thumbnail every time. A *new* name here is the interesting case
    // and is what this assertion exists to surface.
    let disagreeing: Vec<&str> = by_name
        .iter()
        .filter(|(mean, ..)| *mean > 15.0)
        .map(|(_, name, ..)| name.as_str())
        .collect();
    assert_eq!(
        disagreeing,
        [
            "issue11144_reduced.pdf",
            "issue19326.pdf",
            "issue19517.pdf",
            "transparency_group.pdf"
        ],
        "pages whose own miniature disagrees with our render"
    );
}

/// Reduces an RGBA raster to a `GRID`×`GRID` grid of mean luminance.
///
/// Box averaging over each cell, and the cell boundaries are computed per output pixel so that
/// a 76×99 thumbnail and a 76×99 render divide the same way. Luminance is the plain BT.601
/// weighting — this is a similarity smell test, not a colour comparison, and §11.5.3's
/// coefficients would claim a precision the exercise does not have.
fn grid(data: &[u8], width: u32, height: u32) -> [u8; GRID * GRID] {
    let mut out = [0u8; GRID * GRID];
    let (width, height) = (width as usize, height as usize);
    if width == 0 || height == 0 {
        return out;
    }
    for (index, cell) in out.iter_mut().enumerate() {
        let (cx, cy) = (index % GRID, index / GRID);
        let x0 = cx.saturating_mul(width) / GRID;
        let x1 = (cx.saturating_add(1).saturating_mul(width) / GRID).max(x0.saturating_add(1));
        let y0 = cy.saturating_mul(height) / GRID;
        let y1 = (cy.saturating_add(1).saturating_mul(height) / GRID).max(y0.saturating_add(1));
        let mut total = 0u64;
        let mut count = 0u64;
        for y in y0..y1.min(height) {
            for x in x0..x1.min(width) {
                let at = y.saturating_mul(width).saturating_add(x).saturating_mul(4);
                let Some(pixel) = data.get(at..at.saturating_add(4)) else {
                    continue;
                };
                let luma = 0.299 * f64::from(pixel[0])
                    + 0.587 * f64::from(pixel[1])
                    + 0.114 * f64::from(pixel[2]);
                // An unpainted pixel is white on a page and white in a thumbnail: both sides
                // are composited onto the medium before they are looked at.
                let alpha = f64::from(pixel[3]) / 255.0;
                let composited = luma.mul_add(alpha, 255.0 * (1.0 - alpha));
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a luminance composited onto white, so 0.0 ..= 255.0 by construction"
                )]
                {
                    total = total.saturating_add(composited as u64);
                }
                count = count.saturating_add(1);
            }
        }
        *cell = u8::try_from(total.checked_div(count).unwrap_or(255)).unwrap_or(255);
    }
    out
}

/// Writes the thumbnail beside our render of the same page, for a person to look at.
///
/// Trap 1's instrument, applied to the one comparison in this tree whose other side is the
/// producer's own picture: a number saying two images differ says nothing about which of them
/// is wrong. Our render is drawn at the thumbnail's own scale, so the two panels are the same
/// size to within the rounding `TargetSpec` does.
fn side_by_side(dir: &Path, name: &str, thumbnail: &pdf_render::Image, ours: &pdf_render::Raster) {
    let height = thumbnail.height.max(ours.height);
    let width = thumbnail.width.saturating_add(ours.width);
    if width == 0 || height == 0 {
        return;
    }
    let mut canvas = vec![
        255u8;
        (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4)
    ];
    let mut blit = |data: &[u8], w: u32, h: u32, at: u32| {
        for y in 0..h as usize {
            for x in 0..w as usize {
                let from = y
                    .saturating_mul(w as usize)
                    .saturating_add(x)
                    .saturating_mul(4);
                let to = y
                    .saturating_mul(width as usize)
                    .saturating_add(x.saturating_add(at as usize))
                    .saturating_mul(4);
                let (Some(pixel), Some(slot)) = (
                    data.get(from..from.saturating_add(4)),
                    canvas.get_mut(to..to.saturating_add(4)),
                ) else {
                    continue;
                };
                slot.copy_from_slice(pixel);
            }
        }
    };
    blit(&thumbnail.data, thumbnail.width, thumbnail.height, 0);
    blit(&ours.data, ours.width, ours.height, thumbnail.width);

    let out = dir.join(format!("{name}.png"));
    let Ok(handle) = std::fs::File::create(&out) else {
        return;
    };
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(handle), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    if let Ok(mut writer) = encoder.write_header() {
        let _ = writer.write_image_data(&canvas);
    }
}

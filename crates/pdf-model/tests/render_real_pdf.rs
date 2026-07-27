//! End to end: parse a real PDF, interpret its content, render it, and compare against
//! independent reference renderers.
//!
//! This closes the loop the harness was built for. Until now the display list was written
//! by hand alongside the PDF; here it is *derived from* the PDF, so a difference against
//! poppler, mupdf and ghostscript is evidence about our parser and interpreter rather than
//! about a fixture.

#![expect(
    clippy::print_stdout,
    // Fires only in helper functions: clippy's test allowance covers `#[test]` bodies, and
    // a helper that cannot read its fixture must fail loudly rather than skip quietly.
    clippy::expect_used,
    reason = "test code: the measurements are the point of the exercise"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above an A4 page at 72 dpi.
const GENEROUS: u64 = 1 << 30;

/// The page tree must be navigable and report the attributes the specification requires.
#[test]
fn the_fixture_page_is_modelled_correctly() {
    let document = Document::open(test_scenes::basic_pdf()).expect("valid PDF");
    let pages = pdf_model::Pages::new(&document);

    assert_eq!(pages.len(), 1);
    let page = pages.get(0).expect("page one exists");

    // Exact comparison is intended: these values come from the fixture's own
    // `/MediaBox [0 0 595 842]` and are integers, so any difference is a real defect
    // rather than accumulated floating-point error.
    #[expect(
        clippy::float_cmp,
        reason = "the fixture declares exact integral page bounds"
    )]
    {
        assert_eq!(page.media_box, [0.0, 0.0, 595.0, 842.0]);
        assert_eq!(
            page.crop_box, page.media_box,
            "no crop box means the media box applies"
        );
    }
    assert_eq!(page.rotate, 0);
}

/// Interpreting the fixture must reproduce the display list `test-scenes` describes by
/// hand — the two were written to be the same page, so this checks the interpreter against
/// an independent statement of the same intent.
#[test]
fn interpreting_the_fixture_matches_the_hand_written_display_list() {
    let document = Document::open(test_scenes::basic_pdf()).expect("valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");

    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the fixture uses only paths, so nothing should be unsupported: {:?}",
        interpretation.unsupported
    );

    let parsed = interpretation.display_list;
    let hand_written = test_scenes::basic();

    assert_eq!(parsed.page_size, hand_written.page_size);
    assert_eq!(
        parsed.commands().len(),
        hand_written.commands().len(),
        "the same page should produce the same number of drawing commands"
    );
}

/// Both renders of the same page — one from the parsed PDF, one from the hand-written
/// display list — must produce identical pixels.
///
/// If they differ, either the interpreter or the fixture pairing is wrong, and this says so
/// without involving an external renderer.
#[test]
fn rendering_the_parsed_pdf_matches_rendering_the_hand_written_scene() {
    let document = Document::open(test_scenes::basic_pdf()).expect("valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let parsed = pdf_model::interpret(&document, &page).display_list;

    let hand_written = test_scenes::basic();
    let target = TargetSpec::for_page(&hand_written, 1.0, GENEROUS).expect("valid target");

    let from_pdf = CpuRasterizer::new()
        .rasterize(&parsed, target)
        .expect("supported");
    let from_scene = CpuRasterizer::new()
        .rasterize(&hand_written, target)
        .expect("supported");

    let comparison = raster_compare::compare(&from_pdf, &from_scene).expect("same size");
    assert_eq!(
        comparison.max_error, 0,
        "parsing the PDF and building the scene by hand must agree exactly; \
         mean {:.4}, worst tile {:.4} at {:?}",
        comparison.mean_error, comparison.worst_tile_error, comparison.worst_tile_at
    );
}

/// The full pipeline against three independent implementations.
///
/// Bytes in, pixels out, compared with renderers that share no code with ours.
#[test]
fn our_rendering_of_a_parsed_pdf_agrees_with_the_reference_consensus() {
    let work_dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("render-real");
    std::fs::create_dir_all(&work_dir).expect("writable");
    let pdf_path = work_dir.join("basic.pdf");
    std::fs::write(&pdf_path, test_scenes::basic_pdf()).expect("writable");

    let available = pdfref::Reference::available();
    assert!(
        available.len() >= 2,
        "at least two reference renderers are needed to triangulate; found {}",
        available.len()
    );

    // Our render, derived entirely from the file's bytes.
    let document = Document::open(test_scenes::basic_pdf()).expect("valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );

    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS).expect("valid target");
    let ours = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("supported");

    let mut references = Vec::new();
    for reference in available {
        let raster = reference
            .render(&pdf_path, 1, 72, &work_dir)
            .unwrap_or_else(|e| panic!("{reference} failed: {e}"));
        references.push((reference, raster));
    }

    let triangulation =
        pdfref::triangulate(&ours, &references, &pdfref::Tolerance::VECTOR).expect("comparable");
    print!(
        "{}",
        pdfref::report::summarise("parsed-basic", &triangulation)
    );

    let artefacts = pdfref::report::write_artefacts(
        &work_dir,
        "parsed-basic",
        &ours,
        &references,
        &triangulation,
    )
    .expect("writable");
    for path in &artefacts {
        println!("artefact: {}", path.display());
    }

    match &triangulation.outcome {
        pdfref::Outcome::Agrees { with } => {
            assert!(with.len() >= 2, "consensus needs two references");
        }
        other => panic!("expected agreement, got {other:?}; see the artefacts above"),
    }
}

/// Every specification PDF must be interpretable without panicking, and must report what it
/// could not draw.
#[test]
fn every_specification_pdf_interprets() {
    let doc_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc");
    let mut files: Vec<_> = std::fs::read_dir(&doc_dir)
        .expect("readable")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    files.sort();

    for path in files {
        let bytes = std::fs::read(&path).expect("readable");
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let document = Document::open(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));

        let pages = pdf_model::Pages::new(&document);
        let page = pages
            .get(0)
            .unwrap_or_else(|| panic!("{name}: no first page"));
        let interpretation = pdf_model::interpret(&document, &page);

        // These are text-heavy documents, so incompleteness is expected — the requirement is
        // that it is *reported*, which is what lets the harness exclude them.
        println!(
            "{name}: {} pages, {} commands, unsupported: {:?}",
            pages.len(),
            interpretation.display_list.commands().len(),
            interpretation.unsupported
        );
    }
}

/// Text that adds itself to the clipping path must be reported, because we do not clip.
///
/// ISO 32000-2 §9.3.6 Table 106 gives modes 4 to 7 as "fill/stroke/... and add to path for
/// clipping", and §9.4.1 says that path takes effect at `ET` and lasts until the graphics
/// state is restored. We build no such clip, so whatever is painted afterwards in the
/// expectation of being cut to the glyph shapes covers its whole area instead.
///
/// The cost is not subtle, which is why this is reported rather than left to look right by
/// luck: on `text_clip_cff_cid.pdf` the reference renderers show the word "ABC123" and we
/// drew a solid blue bar over it — with `unsupported: []`. The reference-oracle gate found
/// it; no metric we own could have.
///
/// Implementing the clip should make this test fail, and that is the right moment to
/// revisit it.
#[test]
fn text_that_adds_to_the_clipping_path_is_reported() {
    let Some(interpretation) = corpus_page_one("text_clip_cff_cid.pdf") else {
        return;
    };
    let reported = format!("{:?}", interpretation.unsupported);
    assert!(
        reported.contains("text render mode 7"),
        "a text object that clips must say so: {reported}"
    );
}

/// An image's `/Mask` must be reported, because we do not apply one.
///
/// ISO 32000-2 §8.9.6.4 gives `/Mask` as a stencil mask stream and §8.9.6.5 as a colour-key
/// range array; either makes part of the image transparent. We honour only `/SMask`, so the
/// masked-out part is drawn. On `colorkeymask.pdf` that is a whole red band that all three
/// reference renderers hide — drawn by us, with `unsupported: []`, until this landed.
///
/// Implementing either form should make this test fail, which is the right moment to
/// revisit it.
#[test]
fn an_image_mask_we_do_not_apply_is_reported() {
    let Some(interpretation) = corpus_page_one("colorkeymask.pdf") else {
        return;
    };
    let reported = format!("{:?}", interpretation.unsupported);
    assert!(
        reported.contains("/Mask"),
        "an image we draw through no mask must say so: {reported}"
    );
}

/// Interprets page one of a corpus document, or `None` when the submodule is absent.
///
/// The corpus is optional, so a checkout without submodules reports being skipped rather
/// than failing — the same rule `corpus.rs` follows.
fn corpus_page_one(name: &str) -> Option<pdf_model::Interpretation> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    let Ok(bytes) = std::fs::read(&path) else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return None;
    };
    let document = Document::open(bytes).expect("valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    Some(pdf_model::interpret(&document, &page))
}

/// Renders real pages to PNGs for visual inspection.
///
/// The metrics say text and images are being emitted; this is how a human confirms they
/// are in the right places. Not an assertion about pixels — there is no reference to
/// assert against while the page is still incomplete — but a failure to render at all, or
/// a page that comes out blank, is caught here.
///
/// The two documents are chosen to exercise the two routes a CFF font can take to a
/// glyph, which no metric distinguishes: `PDF20_AN001-BPC.pdf` embeds name-keyed CFF in
/// simple fonts, and `ISO_32000-2_sponsored_EC3.pdf` embeds CID-keyed CFF in composite
/// ones. A mapping defect in either shows up as wrong glyphs, not as an error.
#[test]
fn writes_inspectable_renders_of_real_pages() {
    let doc_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc");
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("real-page");
    std::fs::create_dir_all(&dir).expect("writable");

    for (file, label) in [
        ("PDF20_AN001-BPC.pdf", "name-keyed-cff"),
        ("ISO_32000-2_sponsored_EC3.pdf", "cid-keyed-cff"),
        ("Well-Tagged-PDF-WTPDF-1.0.pdf", "mixed"),
        // An axial shading painted by `sh`, plus a soft mask that is still unsupported.
        ("ISO-14289-1-2014-sponsored.pdf", "axial-shading"),
    ] {
        let bytes = std::fs::read(doc_dir.join(file)).expect("corpus file is readable");
        let document = Document::open(bytes).expect("valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let interpretation = pdf_model::interpret(&document, &page);

        let list = interpretation.display_list;
        assert!(
            !list.commands().is_empty(),
            "{file}: a real page should produce drawing commands"
        );

        let target = TargetSpec::for_page(&list, 150.0 / 72.0, GENEROUS).expect("valid target");
        let raster = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("supported");

        let out = dir.join(format!("{label}.png"));
        let handle = std::fs::File::create(&out).expect("writable");
        let mut encoder =
            png::Encoder::new(std::io::BufWriter::new(handle), raster.width, raster.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .expect("valid header")
            .write_image_data(&raster.data)
            .expect("pixel data matches the dimensions");

        println!("wrote {}", out.display());
        println!("  {file} unsupported: {:?}", interpretation.unsupported);
    }
}

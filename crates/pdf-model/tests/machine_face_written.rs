//! A value set in a face from this machine, saved, and drawn from the file alone (ADR 1425).
//!
//! Its own binary because the second half turns the machine's fonts off for the whole process —
//! `pdf_font::substitute::no_machine_fonts` cannot be undone, deliberately — and a test sharing a
//! process with it would find the machine's faces gone.
//!
//! The claim is §9.9.1's and §9.9.2's together: the program written into the file is a subset of
//! the face, with the tables a `/FontFile2` "shall include" and no `cmap` under a `CIDFont`, and
//! it draws the value exactly as the face it was cut from did. So the page is rasterised twice —
//! once as the viewer shows the typed value, drawing from the machine's face, and once from the
//! saved file re-opened with no machine face available, drawing from the subset in it — and the
//! two rasters are compared byte for byte. An independent reader checks the file's structure:
//! `qpdf --check` where it is installed, as evidence about the file and not the definition of it.

#![expect(
    clippy::expect_used,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::fmt::Write as _;

use pdf_model::view::{Entered, ViewState};
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// A raster budget no fixture here comes near.
const GENEROUS: u64 = 1 << 30;

/// One page with one text field, its `/DA` naming a standard font that has no Arabic.
///
/// `/HeBo` is not among the `/DR` fonts, so the font is one the layout invents; a font the
/// document itself names is set as the document says, and never swapped for a machine face.
fn one_field() -> Vec<u8> {
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] \
                /DR << /Font << /Helv 6 0 R >> >> >> >>\nendobj\n\
                2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
                3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
                /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
                4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
                5 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
                /T (field) /P 3 0 R /DA (/HeBo 12 Tf 0 g) >>\nendobj\n\
                6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
                /Encoding /WinAnsiEncoding >>\nendobj\n";
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Page one rasterised against a viewer state.
fn draw(document: &Document, view: &ViewState) -> pdf_render::Raster {
    let page = pdf_model::Pages::new(document).get(0).expect("page one");
    let list = pdf_model::content::interpret_with(document, &page, view).display_list;
    let target = TargetSpec::for_page(&list, 2.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported")
}

/// Whether this machine offers a face covering every character `text` displays once shaped, in
/// the weight `/HeBo` names — the question `variable_text` asks before it sets such a value.
fn machine_offers_a_face_covering(text: &str) -> bool {
    let paragraphs = pdf_font::shaping::Paragraphs::new(text);
    let Some(wanted) = pdf_font::shaping::displayed_characters(text, paragraphs.as_ref()) else {
        return false;
    };
    let wanted: Vec<char> = wanted.keys().copied().collect();
    let request = pdf_font::substitute::Request {
        family: pdf_font::substitute::Family::SansSerif,
        bold: true,
        italic: false,
        standard: false,
    };
    pdf_font::substitute::installed_covering(request, &wanted).is_some()
}

/// An Arabic value saved in a machine face is drawn from the file alone, identically.
///
/// Where the machine offers no face covering the value, there is nothing to save and the test
/// says so and stops (ADR 1154).
#[test]
fn an_arabic_value_saved_in_a_machine_face_draws_from_the_file_alone() {
    let typed = "\u{633}\u{644}\u{627}\u{645} \u{633}\u{644}\u{627}\u{645}";
    if !machine_offers_a_face_covering(typed) {
        println!("skipped: no face on this machine covers the Arabic value's shaped characters");
        return;
    }
    let document = Document::open(one_field()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "field", &Entered::Text(typed.to_owned())),
        1
    );
    let on_screen = draw(&document, &view);
    let written = view.save(&document).expect("the fixture can be written");
    assert!(
        written.unconstructed.is_empty(),
        "{:?}",
        written.unconstructed
    );

    let saved_path =
        std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("machine_face_written.pdf");
    std::fs::write(&saved_path, &written.bytes).expect("the target's scratch directory");
    // qpdf absent is a machine without the witness, and the file's own re-reading below still binds.
    if let Ok(output) = std::process::Command::new("qpdf")
        .arg("--check")
        .arg(&saved_path)
        .output()
    {
        let said = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && said.contains("No syntax or stream encoding errors"),
            "qpdf --check: {said}{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // From here on no face on this machine can be read: the covering search answers none, and
    // the only glyphs there are to draw are the subset the file carries.
    pdf_font::substitute::no_machine_fonts();
    assert!(
        !machine_offers_a_face_covering(typed),
        "the machine's faces are off"
    );
    let saved = Document::open(written.bytes).expect("the save re-opens");
    let from_the_file = draw(&saved, &ViewState::of(&saved));
    assert!(
        from_the_file.data.iter().any(|byte| *byte != 0),
        "the value is drawn from the file"
    );
    assert_eq!(
        (from_the_file.width, from_the_file.height),
        (on_screen.width, on_screen.height)
    );
    assert!(
        from_the_file.data == on_screen.data,
        "the page drawn from the file is the page the viewer showed"
    );
}

//! Saved variable-text default matrices, placement and resolved font size.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "complete saved-PDF fixtures and exact output assertions must fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::view::{Entered, ViewState};
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::{Document, ObjectId};
use render_cpu::CpuRasterizer;

fn source(default_appearance: &str, quadding: u8) -> Vec<u8> {
    let old = "/Tx BMC q Q EMC";
    let appearance = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 160 30] \
         /Resources << /Font << /Helv 6 0 R >> >> /Length {} >>\nstream\n{old}\nendstream",
        old.len().saturating_add(1)
    );
    let objects = [
        format!(
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
             /DA ({default_appearance}) /DR << /Font << /Helv 6 0 R >> >> >> >>"
        ),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << >> /Annots [4 0 R] >>"
            .to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /P 3 0 R \
             /FT /Tx /T (field) /V () /Q {quadding} /AP << /N 5 0 R >> >>"
        ),
        appearance,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{object}\nendobj", index.saturating_add(1));
    }
    let xref = out.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn save(default_appearance: &str, quadding: u8, value: &str) -> Document {
    let document = Document::open(source(default_appearance, quadding)).expect("complete PDF");
    assert!(!document.was_recovered(), "source xref offsets are exact");
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "field", &Entered::Text(value.to_owned())),
        1,
        "one text field accepted the value"
    );
    let written = view
        .save(&document)
        .expect("updated widget appearance is saved");
    assert!(
        written.unconstructed.is_empty(),
        "the field was fully laid out"
    );
    let saved = Document::open(written.bytes).expect("saved incremental update");
    assert!(!saved.was_recovered(), "saved xref offsets are exact");
    let page = pdf_model::Pages::new(&saved).get(0).expect("saved page");
    let interpreted = pdf_model::content::interpret_with(&saved, &page, &ViewState::of(&saved));
    assert!(interpreted.is_complete(), "{:?}", interpreted.unsupported);
    assert!(
        interpreted.text.contains(value),
        "saved text preserves the whole new value"
    );
    saved
}

fn tx_content(document: &Document) -> String {
    let widget = document
        .get(ObjectId {
            number: 4,
            generation: 0,
        })
        .as_dict()
        .cloned()
        .expect("widget dictionary");
    let appearances = document
        .get_key(&widget, "AP")
        .as_dict()
        .cloned()
        .expect("AP dictionary");
    let normal = document
        .get_key(&appearances, "N")
        .as_stream()
        .cloned()
        .expect("normal appearance stream");
    let data = document
        .decoded_stream_data(&normal)
        .expect("appearance stream decodes");
    let text = String::from_utf8(data.to_vec()).expect("generated appearance is ASCII");
    let start = text.find("/Tx BMC").expect("marked content start");
    let rest = text.get(start..).expect("start is in the stream");
    let end = rest.find("EMC").expect("matching marked content end");
    rest.get(..end.saturating_add(3))
        .expect("Tx marked content is sliced")
        .to_owned()
}

fn operands_before<'a>(content: &'a str, operator: &str, count: usize) -> Vec<&'a str> {
    let tokens: Vec<&str> = content.split_ascii_whitespace().collect();
    let at = tokens
        .iter()
        .position(|token| *token == operator)
        .unwrap_or_else(|| panic!("missing {operator} in {content}"));
    tokens
        .get(at.saturating_sub(count)..at)
        .expect("operator has enough operands")
        .to_vec()
}

fn matrix(content: &str) -> [f32; 6] {
    let operands = operands_before(content, "Tm", 6);
    let mut result = [0.0_f32; 6];
    for (slot, operand) in result.iter_mut().zip(operands) {
        *slot = operand.parse().expect("Tm operand is numeric");
    }
    result
}

fn raster(document: &Document) -> pdf_render::Raster {
    let page = pdf_model::Pages::new(document).get(0).expect("page one");
    let interpreted = pdf_model::content::interpret_with(document, &page, &ViewState::of(document));
    assert!(
        interpreted.is_complete(),
        "saved appearance interpreted: {:?}",
        interpreted.unsupported
    );
    let target =
        TargetSpec::for_page(&interpreted.display_list, 1.0, 1 << 20).expect("page target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpreted.display_list, target)
        .expect("CPU rasterization")
}

fn ink_span(raster: &pdf_render::Raster) -> (u32, u32) {
    let mut columns = Vec::new();
    for x in 0..raster.width {
        let ink = (0..raster.height).any(|row| {
            let pixel = usize::try_from(
                row.saturating_mul(raster.width)
                    .saturating_add(x)
                    .saturating_mul(4),
            )
            .unwrap_or(usize::MAX);
            raster
                .data
                .get(pixel.saturating_add(3))
                .copied()
                .unwrap_or(0)
                > 0
        });
        if ink {
            columns.push(x);
        }
    }
    (
        columns.first().copied().expect("text has visible ink"),
        columns.last().copied().expect("text has visible ink"),
    )
}

#[test]
#[expect(
    clippy::many_single_char_names,
    reason = "the six standard text matrix coefficients"
)]
fn saved_da_tm_keeps_linear_terms_and_replaces_nonzero_translation_for_quadding() {
    let appearance = "/Helv 12 Tf 0 g 2 0 0 1 17 19 Tm";
    let left = save(appearance, 0, "Hi");
    let right = save(appearance, 2, "Hi");
    let left_content = tx_content(&left);
    let right_content = tx_content(&right);

    for content in [&left_content, &right_content] {
        let [a, b, c, d, x, y] = matrix(content);
        assert!(
            (a - 2.0).abs() < 0.0001
                && b.abs() < 0.0001
                && c.abs() < 0.0001
                && (d - 1.0).abs() < 0.0001,
            "the DA's linear matrix remains: {a} {b} {c} {d}"
        );
        assert!(
            (x - 17.0).abs() >= 0.0001 && (y - 19.0).abs() >= 0.0001,
            "the DA translation is replaced: {x} {y}"
        );
        assert_eq!(
            content.matches(" Tm").count(),
            1,
            "exactly one matrix is emitted"
        );
    }
    let left_x = matrix(&left_content)[4];
    let right_x = matrix(&right_content)[4];
    assert!(
        right_x > left_x,
        "right quadding changes the computed origin"
    );
    assert!(
        (175..=179).contains(&ink_span(&raster(&right)).1),
        "the saved, scaled text ends at the right edge"
    );
}

#[test]
fn saved_default_appearance_without_tm_gets_one_before_text_showing() {
    let saved = save("/Helv 12 Tf 0 g", 2, "Hi");
    let content = tx_content(&saved);
    let tf = content.find("Tf").expect("font and size from DA");
    let colour = content.find(" g").expect("the default colour operator");
    let tm = content.find("Tm").expect("processor inserts text matrix");
    let show = content.find("Tj").expect("variable value is shown");
    assert!(
        tf < tm && colour < tm && tm < show,
        "Tf, inserted Tm, and text order: {content}"
    );
    assert_eq!(content.matches(" Tm").count(), 1, "one inserted matrix");
    assert!(
        (175..=179).contains(&ink_span(&raster(&saved)).1),
        "without a DA Tm, right-quadded saved text still fits the box"
    );
}

#[test]
fn saved_zero_size_da_is_written_as_a_readable_positive_font_size() {
    let long = "The quick brown fox jumps over the lazy dog again and again and again";
    let saved = save("/Helv 0 Tf 0 g", 0, long);
    let content = tx_content(&saved);
    let font = content.find("Tf").expect("font setting from DA");
    let size = content
        .split_ascii_whitespace()
        .take_while(|token| *token != "Tf")
        .last()
        .expect("Tf has a size operand")
        .parse::<f32>()
        .expect("serialized size is numeric");
    assert!(
        size > 0.0 && size < 12.0,
        "long value requires a positive size smaller than 12, got {size}"
    );
    assert!(font < content.find("Tj").expect("text showing"));
    let (start, end) = ink_span(&raster(&saved));
    assert!(
        start >= 20 && end <= 180,
        "saved auto-sized appearance fits the widget: {start}..{end}"
    );
}

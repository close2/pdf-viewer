//! ISO 32000-2 §6.3.2.2's "unless otherwise instructed", checked by what is left on the page.
//!
//! A native host places its own controls over §12.7's fields, so it asks for the page without the
//! pictures of them. What that must not do is take ink off the page that nothing replaces, which
//! is why every assertion here is about a *region* of the raster rather than about a command
//! count: the widget's rectangle goes empty, and every other rectangle on the page is untouched.
//!
//! The fixture carries four annotations on purpose, because the interesting cases are the ones
//! that look like widgets and are not delegated:
//!
//! | object | what it is | delegated |
//! |---|---|---|
//! | 5 | a text field's widget, reached from `/AcroForm /Fields` | yes |
//! | 7 | a `/Square` markup with an appearance stream | no — §12.5.6.10 has no control |
//! | 9 | `/Subtype /Widget` with no `/T` and no `/Parent` | no — §12.7.4.2's "simply a Widget annotation" |
//! | 11 | a widget with an appearance and no `/Rect` | no — nothing can place a control over it |

#![expect(
    clippy::expect_used,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

use pdf_model::view::{ViewState, WidgetAppearances};

/// Pixel budget, far above the 200×100 page this fixture builds.
const GENEROUS: u64 = 1 << 30;

/// The page the fixture draws on, in points.
const PAGE: (u32, u32) = (200, 100);

/// A one-page PDF with the four annotations the module table describes.
///
/// Each appearance fills its whole `/BBox`, so "was it drawn" is a question about a rectangle of
/// the raster and needs no glyph and no font.
fn fixture() -> Vec<u8> {
    let (width, height) = PAGE;
    // One appearance per annotation, each filling its own `/BBox` with a colour of its own, so
    // that "was it drawn" is a question about a rectangle of the raster and needs no font.
    //
    // The last of the four states its box in the page's own coordinates rather than at the
    // origin, because it has no `/Rect`: §12.5.5's algorithm has nothing to map onto, and this
    // crate places such an annotation where its appearance's box says (ADR 0113's converse).
    let stream = |box_: [u32; 4], colour: &str| {
        let contents = format!(
            "{colour} {} {} {} {} re f",
            box_[0],
            box_[1],
            box_[2].saturating_sub(box_[0]),
            box_[3].saturating_sub(box_[1])
        );
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [{} {} {} {}] /Length {} >>\nstream\n\
             {contents}\nendstream",
            box_[0],
            box_[1],
            box_[2],
            box_[3],
            contents.len().saturating_add(1)
        )
    };
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R 7 0 R 9 0 R 11 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Tx /T (typed) /Rect [10 10 90 30] \
         /F 4 /AP << /N 6 0 R >> >>\nendobj\n\
         6 0 obj\n{}\nendobj\n\
         7 0 obj\n<< /Type /Annot /Subtype /Square /Rect [110 10 190 30] /F 4 \
         /AP << /N 8 0 R >> >>\nendobj\n\
         8 0 obj\n{}\nendobj\n\
         9 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [10 60 90 80] /F 4 \
         /AP << /N 10 0 R >> >>\nendobj\n\
         10 0 obj\n{}\nendobj\n\
         11 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Tx /T (unplaced) /F 4 \
         /AP << /N 12 0 R >> >>\nendobj\n\
         12 0 obj\n{}\nendobj\n",
        stream(FIELD, "0 0 1 rg"),
        stream(MARKUP, "1 0 0 rg"),
        stream(FIELDLESS, "0 1 0 rg"),
        stream(RECTLESS, "1 0 1 rg"),
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    // §7.5.4's table, written whole: the objects are numbered contiguously from 1, so one entry
    // per offset after the free head is the section the clause describes.
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

/// Rasterises the fixture under one policy.
fn drawn(appearances: WidgetAppearances) -> pdf_render::Raster {
    let document = Document::open(fixture()).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.widget_appearances(),
        WidgetAppearances::Drawn,
        "§6.3.2.2's default is what a document opens in"
    );
    view.set_widget_appearances(appearances);
    let list = pdf_model::content::interpret_with(&document, &page, &view).display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_background(pdf_render::Color::TRANSPARENT)
        .rasterize(&list, target)
        .expect("supported")
}

/// How many opaque pixels a user-space rectangle of the page holds.
///
/// The raster's rows run down from the top of a 100-point page, so a PDF y becomes `100 - y`.
fn ink(raster: &pdf_render::Raster, rect: [u32; 4]) -> usize {
    let (top, bottom) = (
        PAGE.1.saturating_sub(rect[3]),
        PAGE.1.saturating_sub(rect[1]),
    );
    (rect[0]..rect[2])
        .flat_map(|x| (top..bottom).map(move |y| (x, y)))
        .filter(|(x, y)| {
            let index = (y.saturating_mul(raster.width).saturating_add(*x) as usize)
                .saturating_mul(4)
                .saturating_add(3);
            raster.data.get(index).copied().unwrap_or_default() > 0
        })
        .count()
}

/// The text field's widget, which is the one a host is handed a control for.
const FIELD: [u32; 4] = [10, 10, 90, 30];
/// §12.5.6.10's square, which no toolkit has a widget for.
const MARKUP: [u32; 4] = [110, 10, 190, 30];
/// The widget §12.7.4.2 leaves outside the form.
const FIELDLESS: [u32; 4] = [10, 60, 90, 80];
/// The widget with no `/Rect`, placed by its appearance's own box.
const RECTLESS: [u32; 4] = [110, 60, 190, 80];

#[test]
fn a_delegated_page_loses_the_field_and_keeps_everything_else() {
    let before = drawn(WidgetAppearances::Drawn);
    let after = drawn(WidgetAppearances::Delegated);

    assert!(
        ink(&before, FIELD) > 0,
        "the field's own appearance is drawn when nobody has instructed otherwise"
    );
    assert_eq!(
        ink(&after, FIELD),
        0,
        "§6.3.2.2's instruction takes the field's picture off the page"
    );

    // The other three, which no host was handed a control for. A rule that suppressed by
    // `/Subtype` alone would empty two of these.
    for (name, rect) in [
        ("the markup", MARKUP),
        ("the fieldless widget", FIELDLESS),
        ("the widget with no /Rect", RECTLESS),
    ] {
        assert_eq!(
            ink(&before, rect),
            ink(&after, rect),
            "{name} is drawn either way, and by the same pixels"
        );
        assert!(ink(&after, rect) > 0, "{name} is drawn at all");
    }
}

#[test]
fn only_the_widgets_a_host_was_told_about_are_delegated() {
    let document = Document::open(fixture()).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let view = ViewState::of(&document);

    let told: std::collections::BTreeSet<_> = pdf_model::form::fields(&document, &page, &view)
        .iter()
        .flat_map(|field| field.widgets.iter())
        .map(|widget| widget.annotation)
        .collect();
    let delegated = pdf_model::form::delegated_widgets(&document, &page, &view);
    assert_eq!(
        delegated, told,
        "what leaves the page is exactly what `Query::Fields` handed over"
    );
    assert_eq!(
        delegated.len(),
        1,
        "the fixture has four annotations, three of which no control replaces: {delegated:?}"
    );
    assert!(
        delegated.iter().all(|id| id.number == 5),
        "and the one is the text field's widget: {delegated:?}"
    );
}

/// The readback follows the picture, which is not obvious and is the right answer.
///
/// §12.5.6.19 makes a widget's appearance the field's, so a page drawn without it reads back
/// without the value in it — and a host that drew the control has the characters in its own
/// control, where the platform's own selection reaches them. A readback that still named text
/// nobody can see would put a selection over a rectangle with nothing in it.
#[test]
fn a_delegated_field_leaves_the_readback_with_the_page() {
    let document = Document::open(fixture()).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut view = ViewState::of(&document);
    let before = pdf_model::content::interpret_with(&document, &page, &view);
    view.set_widget_appearances(WidgetAppearances::Delegated);
    let after = pdf_model::content::interpret_with(&document, &page, &view);

    assert_eq!(
        before.text, after.text,
        "this fixture's appearances draw no glyphs, so neither reading names any text"
    );
    assert!(
        after.display_list.commands().len() < before.display_list.commands().len(),
        "and the list is shorter by what the widget drew: {} against {}",
        after.display_list.commands().len(),
        before.display_list.commands().len()
    );
}

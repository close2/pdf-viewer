//! Every name-like vocabulary this reader looks a *document's own name* up in, pinned as bytes.
//!
//! ISO 32000-2 §7.3.5 states the rule once and it binds every one of them:
//!
//! > Beginning with PDF 1.2 a name object is an atomic symbol uniquely defined by a sequence of
//! > any characters (8-bit values) except null (character code 0). Uniquely defined means that
//! > any two name objects that, after all escaping is expanded (see below), and the resulting
//! > sequences of bytes are not an exact binary match denote different objects.
//!
//! and permits the one exception these tests are careful not to touch:
//!
//! > Ordinarily, the bytes making up the name are never treated as text to be presented to a
//! > human user or to an application external to a PDF processor.
//!
//! A *report* is that need; a *lookup* is not. Session 603 found the sentence broken for a
//! resource name (ADR 0438) and this file is the sweep that followed it (ADR 0439): a pair per
//! vocabulary, each pair differing only in the byte the rule is about.
//!
//! # Why every one of these is synthetic
//!
//! Trap 8, and measured rather than assumed — the measurement is in ADR 0439. No document of the
//! 974-document corpus states a name outside UTF-8 at all, so nothing here could have a witness
//! from that population; the one witness this project holds is a crawled document recorded by
//! digest and never committed. **The collision direction has no witness anywhere**, which is what
//! makes it the direction worth pinning: it draws the wrong thing rather than nothing.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;

/// Assembles a one-page PDF.
///
/// Object numbering is fixed so that a fixture can refer to its own objects: 1 catalog,
/// 2 pages, 3 page, 4 contents, and 5 onwards whatever `extra` defines.
fn pdf(page_extra: &str, resources: &str, content: &str, extra: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << {resources} >> /Contents 4 0 R {page_extra} >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n{extra}",
        content.len().saturating_add(1)
    );

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

/// Interprets page one of a fixture.
fn interpret(page_extra: &str, resources: &str, content: &str, extra: &str) -> Interpreted {
    let bytes = pdf(page_extra, resources, content, extra);
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    Interpreted {
        commands: interpretation.display_list.command_count(),
        glyphs: interpretation.glyphs,
        reports: interpretation
            .unsupported
            .iter()
            .map(|report| format!("{report:?}"))
            .collect(),
    }
}

/// The three things these tests ask a page: what it drew, what it wrote, what it said.
struct Interpreted {
    commands: usize,
    glyphs: usize,
    reports: Vec<String>,
}

/// A Type 3 font whose one glyph description is named `name`, encoded at code 65.
///
/// §9.6.4's steps a) and b) in a fixture: the `/Encoding` maps the code to a glyph name and
/// `/CharProcs` maps that name to the description. `encoded` is what the encoding names and
/// `defined` is what the font defines, so a test can make them differ by one byte.
fn type_3_font(encoded: &str, defined: &str) -> String {
    let glyph = "1000 0 0 0 750 750 d1 0 0 750 750 re f";
    format!(
        "5 0 obj\n<< /Type /Font /Subtype /Type3 /FontMatrix [0.001 0 0 0.001 0 0] \
         /FontBBox [0 0 750 750] /FirstChar 65 /LastChar 65 /Widths [1000] \
         /CharProcs << /{defined} 6 0 R >> \
         /Encoding << /Type /Encoding /Differences [65 /{encoded}] >> >>\nendobj\n\
         6 0 obj\n<< /Length {} >>\nstream\n{glyph}\nendstream\nendobj\n",
        glyph.len().saturating_add(1)
    )
}

/// A widget annotation whose normal appearance is a dictionary of states.
///
/// §12.5.5: where `/AP`'s `/N` is a subdictionary, "the appearance state … shall be used to
/// select the appropriate appearance stream", and `/AS` names it.
fn widget(state: &str, defined: &str) -> String {
    let content = "0 g 10 10 30 30 re f";
    format!(
        "5 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [0 0 50 50] /F 4 \
         /AS /{state} /AP << /N << /{defined} 6 0 R >> >> >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 50 50] /Length {} >>\n\
         stream\n{content}\nendstream\nendobj\n",
        content.len().saturating_add(1)
    )
}

/// §8.6.8's `cs`: "the name of a colour space in the `ColorSpace` subdictionary of the current
/// resource dictionary".
///
/// `/A#F4` expands to a name whose second byte is 0xF4, which is a byte no UTF-8 sequence begins.
#[test]
fn a_colour_space_name_that_is_not_utf_8_is_found() {
    let found = interpret(
        "",
        "/ColorSpace << /A#F4 [/CalRGB << /WhitePoint [0.95 1 1.09] >>] >>",
        "/A#F4 cs 0.2 0.4 0.6 sc 10 10 30 30 re f",
        "",
    );

    assert_eq!(
        found.reports,
        Vec::<String>::new(),
        "the name the page defines and the name `cs` states are an exact binary match"
    );
    assert_eq!(found.commands, 1, "the rectangle is filled in that space");
}

/// The other half of §7.3.5's sentence, in the direction that draws the wrong thing.
///
/// Two names differing only in a byte outside UTF-8 both fold to one replacement character, so a
/// `cs` naming one of them used to find the other — a fill in a colour space the page did not
/// state, in silence.
#[test]
fn two_colour_space_names_differing_only_outside_utf_8_are_two_names() {
    let found = interpret(
        "",
        "/ColorSpace << /A#F4 [/CalRGB << /WhitePoint [0.95 1 1.09] >>] >>",
        "/A#F5 cs 0.2 0.4 0.6 sc 10 10 30 30 re f",
        "",
    );

    assert_eq!(
        found.reports.len(),
        1,
        "0xF4 and 0xF5 are different names, so the page states no space under the second: {:?}",
        found.reports
    );
}

/// §9.6.4 step b): the glyph name from the encoding "shall be used to look up the glyph
/// description in the `CharProcs` dictionary".
///
/// > If the name is not present as a key in CharProcs , no glyph shall be painted.
///
/// So a Type 3 font whose glyph names carry a byte outside UTF-8 used to paint nothing at all,
/// with nothing said: the clause makes the absence silent, which is what a text conversion
/// turned into a page missing its glyphs.
#[test]
fn a_type_3_glyph_name_that_is_not_utf_8_paints() {
    let found = interpret(
        "",
        "/Font << /F0 5 0 R >>",
        "BT /F0 24 Tf 10 10 Td (A) Tj ET",
        &type_3_font("g#F4", "g#F4"),
    );

    assert_eq!(found.glyphs, 1, "the code is shown in the Type 3 font");
    assert_eq!(
        found.commands, 1,
        "and reaches its glyph description, whose `re f` marks the page"
    );
}

/// The collision direction for `/CharProcs`, which paints a glyph the font did not select.
#[test]
fn two_type_3_glyph_names_differing_only_outside_utf_8_are_two_names() {
    let found = interpret(
        "",
        "/Font << /F0 5 0 R >>",
        "BT /F0 24 Tf 10 10 Td (A) Tj ET",
        &type_3_font("g#F5", "g#F4"),
    );

    // `glyphs` counts the codes a Type 3 font was asked to show rather than the descriptions
    // that were found — §9.6.4 makes a name absent from `/CharProcs` a defined outcome and not
    // a shortfall — so what a collision would change is the *marks*.
    assert_eq!(
        found.commands, 0,
        "the encoding names /g\u{f5} and the font defines /g\u{f4}, so no glyph is painted"
    );
}

/// §12.5.5's appearance state, which selects a stream out of `/AP`'s `/N` subdictionary.
///
/// The keys there are names the *file* invents — `/Off` and a checkbox's export value are the
/// conventional ones, and nothing in the clause restricts them to ASCII.
#[test]
fn an_appearance_state_that_is_not_utf_8_selects_its_stream() {
    let found = interpret("/Annots [5 0 R]", "", "", &widget("On#F4", "On#F4"));

    assert!(
        found.commands > 0,
        "the state the annotation shows is the state the appearance dictionary defines"
    );
}

/// The collision direction for an appearance state: the wrong widget appearance on the page.
#[test]
fn two_appearance_states_differing_only_outside_utf_8_are_two_states() {
    let found = interpret("/Annots [5 0 R]", "", "", &widget("On#F5", "On#F4"));

    assert_eq!(
        found.commands, 0,
        "/On\u{f5} is not /On\u{f4}, so the annotation shows no stream"
    );
}

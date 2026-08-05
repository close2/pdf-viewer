//! Type 3 fonts, checked against ISO 32000-2 §9.6.4's own rules one at a time.
//!
//! A Type 3 glyph is a content stream rather than an outline, which puts four rules in
//! places nothing else in this tree exercises: glyph space is the font's own `/FontMatrix`,
//! the widths are in that space, the encoding is the only mapping there is, and a `d1` glyph
//! description states a shape whose colour comes from outside it.
//!
//! These assert against the *display list* rather than against pixels, because each rule is
//! about a number — an advance, a transform, a paint — and a rasterised page answers those
//! only through what it happens to cover. `tests/render_real_pdf.rs` and the corpus oracle
//! cover the other direction: that real documents come out looking right.
//!
//! The fixtures are built from §9.6.4's own EXAMPLE, which defines a two-glyph font of a
//! filled square and a filled triangle. It is worth using verbatim: it is the one Type 3
//! font whose intended appearance the standard itself states.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and these pages are 100 \
              units square where no index can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Command, Paint};
use pdf_syntax::Document;

/// A one-page fixture using a Type 3 font, with every part of the font under the test's
/// control.
///
/// `font_matrix`, `widths` and the two glyph descriptions are what the individual tests
/// vary; `content` is the page's own content stream, which selects the font and shows text.
struct Fixture<'a> {
    font_matrix: &'a str,
    widths: &'a str,
    square: &'a str,
    triangle: &'a str,
    content: &'a str,
}

impl Default for Fixture<'_> {
    /// §9.6.4's EXAMPLE, as written: a 1000-unit glyph space, two glyphs one em wide, a
    /// filled square and a filled triangle.
    fn default() -> Self {
        Self {
            font_matrix: "[0.001 0 0 0.001 0 0]",
            widths: "[1000 1000]",
            square: "1000 0 0 0 750 750 d1\n0 0 750 750 re f",
            triangle: "1000 0 d0\n0 0 m 375 750 l 750 0 l f",
            content: "BT /FT3 10 Tf 0 0 Td (ab) Tj ET",
        }
    }
}

impl Fixture<'_> {
    /// Assembles the fixture into PDF bytes.
    fn build(&self) -> Vec<u8> {
        let Self {
            font_matrix,
            widths,
            square,
            triangle,
            content,
        } = *self;
        let body = format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
             /Resources << /Font << /FT3 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
             4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
             5 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 750 750] \
             /FontMatrix {font_matrix} /CharProcs 7 0 R /Encoding 6 0 R \
             /FirstChar 97 /LastChar 98 /Widths {widths} >>\nendobj\n\
             6 0 obj\n<< /Type /Encoding /Differences [97 /square /triangle] >>\nendobj\n\
             7 0 obj\n<< /square 8 0 R /triangle 9 0 R >>\nendobj\n\
             8 0 obj\n<< /Length {} >>\nstream\n{square}\nendstream\nendobj\n\
             9 0 obj\n<< /Length {} >>\nstream\n{triangle}\nendstream\nendobj\n",
            content.len() + 1,
            square.len() + 1,
            triangle.len() + 1,
        );

        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for object in body.split_inclusive("endobj\n") {
            offsets.push(out.len());
            out.push_str(object);
        }
        let xref_at = out.len();
        let size = offsets.len() + 1;
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

    /// Interprets the fixture's page one.
    fn interpret(&self) -> pdf_model::Interpretation {
        let document = Document::open(self.build()).expect("the fixture is a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        pdf_model::interpret(&document, &page)
    }
}

/// Where each fill in a display list is placed, in page units.
fn fill_origins(interpretation: &pdf_model::Interpretation) -> Vec<(f32, f32)> {
    interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill { transform, .. } => Some((transform.e, transform.f)),
            _ => None,
        })
        .collect()
}

/// The colours the display list's fills paint in.
fn fill_colours(interpretation: &pdf_model::Interpretation) -> Vec<[f32; 4]> {
    interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill {
                paint: Paint::Solid(colour),
                ..
            } => Some([colour.r, colour.g, colour.b, colour.a]),
            _ => None,
        })
        .collect()
}

/// A glyph description is run, and run once per character shown.
///
/// §9.6.4 step c): "Invoke the glyph description." Before the tenth session a Type 3 font was
/// refused outright and the page reported `Text`, so this is the whole feature's floor.
#[test]
fn each_shown_code_runs_its_glyph_description() {
    let interpretation = Fixture::default().interpret();
    assert!(
        interpretation.is_complete(),
        "a Type 3 font is drawn, not reported: {:?}",
        interpretation.unsupported
    );
    assert_eq!(
        fill_origins(&interpretation).len(),
        2,
        "one fill per glyph, from the two `re f`/`f` descriptions"
    );
}

/// The advance comes from `/Widths` *through* `/FontMatrix`.
///
/// Table 110: the widths "shall be interpreted in glyph space as specified by `FontMatrix`
/// (unlike the widths of a Type 1 font, which are in thousandths of a unit of text space)".
/// The thousandfold difference between the two readings is what this pins: with the
/// conventional matrix a 1000-unit width advances one em, and with a ten-times matrix the
/// same number advances ten.
///
/// It is checked through the *position of the second glyph*, which is the only place an
/// advance is observable, and the transform is the one the second description drew under.
#[test]
fn a_width_advances_through_the_font_matrix() {
    let conventional = Fixture::default().interpret();
    let origins = fill_origins(&conventional);
    assert_eq!(origins.len(), 2);
    // 1000 glyph units × 0.001 × a font size of 10 = 10 page units.
    assert!(
        (origins[1].0 - origins[0].0 - 10.0).abs() < 1e-3,
        "the second glyph should be one em along, and is at {origins:?}"
    );

    let ten_times = Fixture {
        font_matrix: "[0.01 0 0 0.01 0 0]",
        ..Fixture::default()
    }
    .interpret();
    let origins = fill_origins(&ten_times);
    assert_eq!(origins.len(), 2);
    assert!(
        (origins[1].0 - origins[0].0 - 100.0).abs() < 1e-3,
        "a ten-times glyph space advances ten times as far, and gave {origins:?}"
    );
}

/// A `d1` description's colour operators are ignored; a `d0` description's are not.
///
/// Table 111 on `d1`: such a description "should not execute any operators that set the
/// colour ... any use of such operators shall be ignored", and "its colour shall be
/// determined by the graphics state in effect each time this glyph is painted by a
/// text-showing operator". `d0` is the opposite case in the same table — the operator a
/// description uses precisely when it *does* set its own colour.
///
/// The fixture paints the page red and has each glyph try to set blue.
#[test]
fn a_d1_description_takes_its_colour_from_the_page_and_a_d0_one_does_not() {
    let uncoloured = Fixture {
        square: "1000 0 0 0 750 750 d1\n0 0 1 rg\n0 0 750 750 re f",
        content: "1 0 0 rg BT /FT3 10 Tf 0 0 Td (a) Tj ET",
        ..Fixture::default()
    }
    .interpret();
    assert_eq!(
        fill_colours(&uncoloured),
        vec![[1.0, 0.0, 0.0, 1.0]],
        "a d1 glyph is painted in the colour the text-showing operator was using"
    );

    let coloured = Fixture {
        square: "1000 0 d0\n0 0 1 rg\n0 0 750 750 re f",
        content: "1 0 0 rg BT /FT3 10 Tf 0 0 Td (a) Tj ET",
        ..Fixture::default()
    }
    .interpret();
    assert_eq!(
        fill_colours(&coloured),
        vec![[0.0, 0.0, 1.0, 1.0]],
        "a d0 glyph specifies its own colour, which is what the operator declares"
    );
}

/// A `d1` glyph is one colour even where its description strokes.
///
/// The same clause's reason for admitting an image mask is that it "merely defines a region
/// of the page to be painted with the current colour", so a stroke inside an uncoloured
/// description is part of that region rather than a request for the stroking colour. The
/// page below sets a red fill and a green stroke; the glyph strokes, and comes out red.
///
/// `Type3WordSpacing.pdf` in the corpus is the same case with three renderers on it:
/// `poppler` and `ghostscript` read it this way, `mupdf` uses the stroking colour.
#[test]
fn an_uncoloured_glyph_strokes_in_the_colour_it_is_painted_with() {
    let interpretation = Fixture {
        square: "1000 0 0 0 750 750 d1\n50 w\n30 30 690 690 re S",
        content: "1 0 0 rg 0 1 0 RG BT /FT3 10 Tf 0 0 Td (a) Tj ET",
        ..Fixture::default()
    }
    .interpret();

    let strokes: Vec<[f32; 4]> = interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Stroke {
                paint: Paint::Solid(colour),
                ..
            } => Some([colour.r, colour.g, colour.b, colour.a]),
            _ => None,
        })
        .collect();
    assert_eq!(
        strokes,
        vec![[1.0, 0.0, 0.0, 1.0]],
        "the glyph's stroke is part of its shape, painted in the text's own colour"
    );
}

/// A code whose glyph name is not in `/CharProcs` paints nothing and still advances.
///
/// §9.6.4 step b): "If the name is not present as a key in `CharProcs`, no glyph shall be
/// painted." It is not an error, and the width still applies — Table 110 gives `/Widths` an
/// entry for every code in the range whether or not a description exists.
#[test]
fn a_code_with_no_glyph_description_paints_nothing_and_still_advances() {
    let interpretation = Fixture {
        // `/triangle` is dropped from `/CharProcs`, so code 98 names a glyph that is absent.
        content: "BT /FT3 10 Tf 0 0 Td (ba) Tj ET",
        triangle: "1000 0 d0",
        ..Fixture::default()
    }
    .interpret();
    assert!(
        interpretation.is_complete(),
        "a missing glyph is defined behaviour, not a report: {:?}",
        interpretation.unsupported
    );

    let origins = fill_origins(&interpretation);
    assert_eq!(origins.len(), 1, "only the square paints");
    assert!(
        (origins[0].0 - 10.0).abs() < 1e-3,
        "the square follows an advance for the glyph that painted nothing, and is at {origins:?}"
    );
}

/// A Type 3 glyph showing itself is bounded rather than recursing forever.
///
/// `ContentStreamCycleType3insideType3.pdf` in the corpus is a file built to do this. A
/// glyph description may legitimately show text in another Type 3 font, so the recursion is
/// real and only its depth is bounded — by the same bound a chain of form `XObject`s has,
/// because it is the same danger.
#[test]
fn a_glyph_that_shows_itself_reaches_a_bound_and_stops() {
    let interpretation = Fixture {
        square: "1000 0 d0\nBT /FT3 10 Tf (a) Tj ET",
        ..Fixture::default()
    }
    .interpret();

    assert!(
        interpretation.unsupported.iter().any(|item| matches!(
            item,
            pdf_model::Unsupported::LimitReached {
                limit: "MAX_FORM_DEPTH"
            }
        )),
        "the cycle should reach the nesting bound and say so: {:?}",
        interpretation.unsupported
    );
}

/// §9.10.2's second method reaches a Type 3 font, and its last resort reaches the rest.
///
/// > If the font is a simple font and the glyph selection algorithm (see 9.6.5, "Character
/// > encoding") uses a glyph name, that name can be looked up in the Adobe Glyph List and Adobe
/// > Glyph List for New Fonts to obtain the corresponding Unicode value.
///
/// A Type 3 font is a simple font and §9.6.4's step b) is a name — "[g]et the glyph name from
/// the Encoding entry" — so the method applies, and this module refused it for three hundred
/// sessions on the argument that the name "names a procedure". It names a procedure *and* a
/// character, and a producer calling one `/colon` has said which.
#[test]
fn a_glyph_name_the_adobe_glyph_list_knows_is_what_the_code_means() {
    let named = Fixture {
        // The two glyphs keep their shapes and take names the list knows.
        content: "BT /FT3 10 Tf 0 0 Td (ab) Tj ET",
        ..Fixture::default()
    };
    let mut bytes = String::from_utf8(named.build()).expect("the fixture is ASCII");
    bytes = bytes.replace(
        "/Differences [97 /square /triangle]",
        "/Differences [97 /A /colon]",
    );
    let document = Document::open(rebuilt(&bytes)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    assert_eq!(pdf_model::interpret(&document, &page).text, "A:");
}

/// And where the name is not in the list, §9.10.2's own permission takes the code.
///
/// > If these methods fail to produce a Unicode value, there is no way to determine what the
/// > character code represents in which case a PDF processor may choose a character code of
/// > their choosing.
///
/// `dvips` names every glyph `aNN` after its own code, which the Adobe Glyph List cannot
/// resolve — `issue918.pdf` is 193 words of English that read back as nothing before this.
#[test]
fn a_name_the_list_does_not_know_leaves_the_code_itself() {
    let bytes = String::from_utf8(Fixture::default().build()).expect("the fixture is ASCII");
    let bytes = bytes.replace(
        "/Differences [97 /square /triangle]",
        "/Differences [97 /a97 /a98]",
    );
    let document = Document::open(rebuilt(&bytes)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    assert_eq!(
        pdf_model::interpret(&document, &page).text,
        "ab",
        "the codes are 97 and 98, which are the printable bytes the choice is bounded to"
    );
}

/// Text a glyph description shows is the glyph, not text of the page.
///
/// §9.6.4 makes a description "a content stream that contains the operators that paint the
/// glyph", so a `Tj` inside one is how the glyph is drawn. `pr4922.pdf` draws its Type 3 glyphs
/// by showing a character of another font, and reading both said every character twice.
#[test]
fn text_shown_inside_a_glyph_description_is_not_read_back() {
    let bytes = String::from_utf8(
        Fixture {
            // The square's description shows a character of the page's own font instead of
            // painting a rectangle, which is `pr4922.pdf`'s shape in four operators.
            square: "1000 0 0 0 750 750 d1\nBT /FT3 10 Tf (b) Tj ET",
            ..Fixture::default()
        }
        .build(),
    )
    .expect("the fixture is ASCII");
    let bytes = bytes.replace(
        "/Differences [97 /square /triangle]",
        "/Differences [97 /A /colon]",
    );
    let document = Document::open(rebuilt(&bytes)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    assert_eq!(
        pdf_model::interpret(&document, &page).text,
        "A:",
        "the description's own show operator is the glyph being painted"
    );
}

/// Rebuilds a fixture's cross-reference table after a string replacement changed its offsets.
fn rebuilt(source: &str) -> Vec<u8> {
    let body: String = source
        .split_inclusive("endobj\n")
        .filter(|part| part.contains(" 0 obj"))
        .collect();
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
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

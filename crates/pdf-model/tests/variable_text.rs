//! ISO 32000-2 §12.7.4.3's variable text, checked by where the ink lands.
//!
//! Every assertion here is about a *position* or an *absence*, never about a glyph's shape.
//! The `/DA` strings below name `/Helv`, which no fixture embeds, so the glyphs come from
//! whatever sans-serif face this machine has installed (`pdf-font`'s `substitute`) — their
//! outlines differ from one machine to the next and their advances come from the font
//! program. What does not differ is the clause: left-justified text starts at the left edge,
//! right-justified text ends at the right one, a comb field puts one character per cell, and a
//! password field shows something other than its value. So the fixtures are built to make the
//! *rule* visible and the tolerances are wide enough that a different face cannot break them.
//!
//! Trap 8's converse applies and is the reason this file exists at all: the corpus contains no
//! comb field with a value, no right-quadded field and no password field, so these rules are
//! defended by nothing else.

#![expect(
    clippy::expect_used,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::fmt::Write as _;

use pdf_model::view::Entered;
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 200×100 pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// The page every fixture draws on, in points.
const PAGE: (u32, u32) = (200, 100);

/// Assembles a one-page PDF with an interactive form, one widget, and a `/DR` naming `/Helv`.
///
/// `form` and `annotation` are written verbatim into the interactive form dictionary and the
/// annotation, which is what lets one builder serve a text field, a comb field, a check box
/// and a free text annotation.
fn pdf_with(form: &str, annotation: &str) -> Vec<u8> {
    pdf_with_appearance(form, annotation, "0 0 1 rg 0 0 10 10 re f")
}

/// The same, with the stored appearance stream's contents given.
fn pdf_with_appearance(form: &str, annotation: &str, appearance: &str) -> Vec<u8> {
    pdf_with_font(form, annotation, appearance, "")
}

/// The same again, with a font descriptor given to the `/Helv` the `/DR` defines.
///
/// §9.8.1's Table 120 is where a field's baseline comes from when the document states one, and
/// the standard 14 the other fixtures name have no descriptor at all — so this is the only way
/// to put those two entries in front of the layout. `descriptor` is written verbatim into a
/// dictionary the font's `/FontDescriptor` reaches.
fn pdf_with_descriptor(annotation: &str, descriptor: &str) -> Vec<u8> {
    pdf_with_font(
        "",
        annotation,
        "0 0 1 rg 0 0 10 10 re f",
        &format!(
            "/FontDescriptor << /Type /FontDescriptor /FontName /Helvetica /Flags 32 \
             /ItalicAngle 0 /StemV 80 /FontBBox [-100 -300 1000 900] {descriptor} >>"
        ),
    )
}

/// The one builder the three above share, with entries added to the `/DR` font dictionary.
fn pdf_with_font(form: &str, annotation: &str, appearance: &str, font: &str) -> Vec<u8> {
    let (width, height) = PAGE;
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm \
         << /Fields [5 0 R] /DR << /Font << /Helv 7 0 R >> >> {form} >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n{annotation}\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 160 30] /Length {} >>\n\
         stream\n{appearance}\nendstream\nendobj\n\
         7 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding /WinAnsiEncoding {font} >>\nendobj\n",
        appearance.len().saturating_add(1)
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

/// Interprets a fixture, returning what it reported and the marks it made.
fn draw(bytes: Vec<u8>) -> (Vec<String>, pdf_render::Raster) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let reports = interpretation
        .unsupported
        .iter()
        .map(|item| format!("{item:?}"))
        .collect();

    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported");
    (reports, raster)
}

/// Rasterises one page against a viewer state a person has changed.
///
/// [`draw`] with the state the document opens in; this one is what a test about §12.7.5.2.3's
/// `/V` needs, because the whole question is what the picture does when a value is replaced.
fn draw_with(
    document: &Document,
    page: &pdf_model::Page,
    view: &pdf_model::view::ViewState,
) -> pdf_render::Raster {
    let list = pdf_model::content::interpret_with(document, page, view).display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported")
}

/// Every column, in PDF x, that any glyph reached.
fn inked_columns(raster: &pdf_render::Raster) -> Vec<u32> {
    (0..raster.width)
        .filter(|x| (0..raster.height).any(|y| opacity(raster, *x, y) > 0))
        .collect()
}

/// A pixel's opacity, addressed in raster rows rather than PDF's upward y.
fn opacity(raster: &pdf_render::Raster, x: u32, row: u32) -> u8 {
    let index = (row.saturating_mul(raster.width).saturating_add(x) as usize).saturating_mul(4);
    raster.data[index.saturating_add(3)]
}

/// Every row, in PDF y, that any glyph reached.
fn inked_rows(raster: &pdf_render::Raster) -> Vec<u32> {
    (0..raster.height)
        .filter(|row| (0..raster.width).any(|x| opacity(raster, x, *row) > 0))
        // The raster's rows run downward and PDF's y runs upward.
        .map(|row| raster.height.saturating_sub(1).saturating_sub(row))
        .collect()
}

/// A page coordinate as the `f32` every question in this crate's vocabulary takes.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a /Rect's midpoint, which §7.7.3.3 bounds far below an f32's exact range"
)]
fn f64_as_f32(value: f64) -> f32 {
    value as f32
}

/// Table 166's `/Contents` as §7.9.2.2's text string, read out of a saved file.
fn contents_of(document: &Document, dict: &pdf_syntax::Dictionary) -> Option<String> {
    match document.get_key(dict, "Contents") {
        pdf_syntax::Object::String(bytes) => Some(pdf_syntax::text_string::text_string(&bytes)),
        _ => None,
    }
}

/// The leftmost and rightmost inked column, which is what quadding moves.
fn ink_span(raster: &pdf_render::Raster) -> (u32, u32) {
    let columns = inked_columns(raster);
    assert!(!columns.is_empty(), "nothing was drawn at all");
    (
        columns.first().copied().unwrap_or_default(),
        columns.last().copied().unwrap_or_default(),
    )
}

/// A text field filling most of the page, with the value, quadding and flags given.
fn text_field(value: &str, quadding: &str, flags: &str) -> Vec<u8> {
    pdf_with(
        "",
        &format!(
            "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
             /T (field) /V ({value}) /DA (/Helv 12 Tf 0 g) {quadding} {flags} >>"
        ),
    )
}

/// Table 228's `/Q`: left starts at the left edge, right ends at the right one.
///
/// The rule the clause states in full — "0 Left-justified 1 Centred 2 Right-justified" — and
/// the one place a measurement of the string's own width becomes visible on the page. A
/// reader that ignores `/Q` draws all three identically, which is what this catches.
#[test]
fn quadding_moves_the_line_within_its_box() {
    let (_, left) = draw(text_field("Hi", "/Q 0", ""));
    let (_, centred) = draw(text_field("Hi", "/Q 1", ""));
    let (_, right) = draw(text_field("Hi", "/Q 2", ""));

    let (left_start, left_end) = ink_span(&left);
    let (centre_start, centre_end) = ink_span(&centred);
    let (right_start, right_end) = ink_span(&right);

    // §12.5.4's default border is one point wide and drawn inside `/Rect`, so the text box
    // runs from 21 to 179.
    assert!(
        (20..=24).contains(&left_start),
        "left-justified text starts at the left edge, not {left_start}"
    );
    assert!(
        (175..=179).contains(&right_end),
        "right-justified text ends at the right edge, not {right_end}"
    );
    assert!(
        centre_start > left_start && centre_end < right_end,
        "centred text sits between the two: {centre_start}..{centre_end}"
    );
    // The three are the same string, so they must be the same width to within one pixel of
    // rounding. A layout that measured differently per quadding would fail here.
    let widths = [
        left_end - left_start,
        centre_end - centre_start,
        right_end - right_start,
    ];
    assert!(
        widths.iter().max().unwrap_or(&0) - widths.iter().min().unwrap_or(&0) <= 1,
        "one string, three widths: {widths:?}"
    );
}

/// A size of zero is auto-sized, and the result fits (§12.7.4.3).
///
/// > A zero value for size means that the font shall be auto-sized : its size shall be
/// > computed as an implementation dependent function.
///
/// The function is ours, so what is asserted is the property the clause implies rather than
/// any particular number: a value too long for its box at a stated size still fits when the
/// size is left to the processor, and a *longer* value is set smaller than a shorter one.
#[test]
fn a_zero_size_is_auto_sized_until_the_value_fits() {
    let long = "The quick brown fox jumps over the lazy dog again and again and again";
    let (reports, raster) = draw(pdf_with(
        "",
        &format!(
            "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
             /T (field) /V ({long}) /DA (/Helv 0 Tf 0 g) >>"
        ),
    ));
    assert!(reports.is_empty(), "{reports:?}");
    let (start, end) = ink_span(&raster);
    assert!(
        start >= 20 && end <= 180,
        "auto-sized text escaped its box: {start}..{end}"
    );

    let (_, short) = draw(pdf_with(
        "",
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
         /T (field) /V (Hi) /DA (/Helv 0 Tf 0 g) >>",
    ));
    assert!(
        inked_rows(&short).len() > inked_rows(&raster).len(),
        "a shorter value must be set larger, not smaller"
    );
}

/// Table 231 bit 13: a multiline field wraps, and the lines run downward.
#[test]
fn a_multiline_value_wraps_into_lines_that_run_down_the_box() {
    let value = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
    let (reports, raster) = draw(pdf_with(
        "",
        &format!(
            "<< /Type /Annot /Subtype /Widget /Rect [20 10 100 90] /F 4 /FT /Tx /Ff 4096 \
             /T (field) /V ({value}) /DA (/Helv 10 Tf 0 g) >>"
        ),
    ));
    assert!(reports.is_empty(), "{reports:?}");

    // Wrapping is visible as height, not as gaps: at ten points the lines sit 10.83 apart
    // (§12.7.5.3's own example spaces them at 13/12 of the size) and consecutive lines can
    // touch, so what distinguishes three lines from one is that the ink is three lines tall.
    let rows = inked_rows(&raster);
    let top = rows.iter().max().copied().unwrap_or_default();
    let bottom = rows.iter().min().copied().unwrap_or_default();
    assert!(
        top - bottom >= 25,
        "three lines of ten-point text span at least 25 points, not {}",
        top - bottom
    );
    // And the first line starts at the top of the box, which is what makes several lines run
    // *down* rather than being centred as one line is.
    assert!(
        top >= 80,
        "the first line should start near the top, not {top}"
    );

    let (start, end) = ink_span(&raster);
    assert!(
        start >= 20 && end <= 100,
        "a wrapped line escaped its box: {start}..{end}"
    );
}

/// §12.7.5.3's Table 231 bit 25: a comb field puts one character in each `/MaxLen` cell.
///
/// > If set, the field shall be automatically divided into as many equally spaced positions,
/// > or combs , as the value of MaxLen , and the text is laid out into those combs.
///
/// Four characters in eight cells across 160 points is one character every 20 points, so the
/// ink must reach past the middle of the box and stop well before its right edge — which
/// plain left-justified text of four characters at this size would not do.
#[test]
fn a_comb_field_spreads_one_character_per_cell() {
    let (reports, raster) = draw(pdf_with(
        "",
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx /Ff 16777216 \
         /MaxLen 8 /T (field) /V (1234) /DA (/Helv 12 Tf 0 g) >>",
    ));
    assert!(reports.is_empty(), "{reports:?}");

    let (start, end) = ink_span(&raster);
    assert!(
        (20..=30).contains(&start),
        "the first character belongs in the first cell, not at {start}"
    );
    assert!(
        (85..=105).contains(&end),
        "four of eight cells reach the middle of the box, not {end}"
    );

    // The proof that they are *spread*: four characters at 12 points run about 27 points
    // when set normally, and four cells of 20 points run 80.
    let (_, plain) = draw(text_field("1234", "", ""));
    let (plain_start, plain_end) = ink_span(&plain);
    assert!(
        end - start > (plain_end - plain_start) * 2,
        "the comb must be far wider than the same string set normally"
    );
}

/// §12.7.5.3's Table 231 bit 14: a password field shows something other than what it holds.
///
/// > Characters typed from the keyboard shall instead be echoed in some unreadable form, such
/// > as asterisks or bullet characters.
///
/// The same value with and without the flag must not produce the same marks. Asserting only
/// the difference — rather than which substitute is used — keeps the test about the clause,
/// which names two acceptable ones and requires neither.
#[test]
fn a_password_field_does_not_echo_its_value() {
    let (_, plain) = draw(text_field("secret", "", ""));
    let (reports, hidden) = draw(text_field("secret", "", "/Ff 8192"));
    assert!(reports.is_empty(), "{reports:?}");
    assert_ne!(
        plain.data, hidden.data,
        "a password field drew its value as it stands"
    );
    assert!(
        !inked_columns(&hidden).is_empty(),
        "a password field must still show that it holds something"
    );
}

/// §12.7.5.3's Table 231 bit 14, second sentence: what a person types is not written to the file.
///
/// > NOTE To protect password confidentiality, it is imperative that PDF processors never store
/// > the value of the text field in the PDF file if this flag is set.
///
/// A NOTE is informative and this one is obeyed anyway, because the alternative is a person's
/// password in somebody's file in clear text. The proof is taken **from the saved bytes**: the
/// same edit on the same fixture with and without the flag, read back through a second
/// `Document`, so what is asserted is what a next reader sees rather than what this program
/// intended. `withheld` names the field, because the refusal has to be one a host can say.
#[test]
fn a_password_fields_typed_value_is_not_written_into_the_file() {
    let typed = "hunter2";
    for (flags, kept) in [("", true), ("/Ff 8192", false)] {
        let document =
            Document::open(text_field("", "", flags)).expect("the fixture is a valid PDF");
        let mut view = pdf_model::view::ViewState::of(&document);
        assert_eq!(
            view.set_field(&document, "field", &Entered::Text(typed.to_owned())),
            1,
            "the fixture has one widget for the field"
        );
        let written = view.save(&document).expect("the fixture can be written");
        // **Searched for in the bytes**, not read back through `field_value`: that answer is
        // Table 231 bit 14's bullets for a password field whether or not the characters reached
        // the file, so an assertion on it would pass while the secret sat in the update. The
        // fixture is unencrypted, so a `/V` that was written is there to be found.
        let present = written
            .bytes
            .windows(typed.len())
            .any(|run| run == typed.as_bytes());
        assert_eq!(present, kept, "flags {flags:?}: the characters in the file");
        let reopened = Document::open(written.bytes).expect("what was written is a PDF");
        let read_back = pdf_model::view::ViewState::of(&reopened)
            .field_value(&reopened, "field")
            .expect("a text field states a value");
        if kept {
            assert_eq!(read_back.text, typed, "an ordinary field keeps its value");
            assert!(written.withheld.is_empty(), "{:?}", written.withheld);
        } else {
            assert!(read_back.text.is_empty(), "{:?}", read_back.text);
            assert_eq!(written.withheld, vec!["field".to_owned()]);
        }
    }
}

/// The `/DA` string's colour operators reach the text (§12.7.4.3, Table 228).
///
/// > The default appearance string ( DA ) contains any graphics state or text state operators
/// > needed to establish the graphics state parameters, such as text size and colour
#[test]
fn the_default_appearance_strings_colour_is_the_texts_colour() {
    // **Asserted as a difference between two colours rather than as a pixel count.** The count
    // is a property of the substitute face, and this test used to require more than ten
    // strongly-red pixels — which "Hi" at 12 points gives with some faces and not others. It
    // stopped being true in the hundred-and-forty-eighth session, when the standard 14 became
    // compiled-in and `/Helvetica` began resolving to Liberation Sans on every machine rather
    // than to whatever was installed: nine pixels, and the clause still honoured.
    let field = |colour: &str| {
        let (reports, raster) = draw(pdf_with(
            "",
            &format!(
                "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
                 /T (field) /V (Hi) /DA ({colour} /Helv 12 Tf) >>"
            ),
        ));
        assert!(reports.is_empty(), "{reports:?}");
        let mut red = 0_usize;
        let mut blue = 0_usize;
        for row in 0..raster.height {
            for x in 0..raster.width {
                let index =
                    (row.saturating_mul(raster.width).saturating_add(x) as usize).saturating_mul(4);
                let (r, g, b, a) = (
                    raster.data[index],
                    raster.data[index.saturating_add(1)],
                    raster.data[index.saturating_add(2)],
                    raster.data[index.saturating_add(3)],
                );
                if a > 200 && g < 60 {
                    if r > 200 && b < 60 {
                        red = red.saturating_add(1);
                    }
                    if b > 200 && r < 60 {
                        blue = blue.saturating_add(1);
                    }
                }
            }
        }
        (red, blue)
    };

    let (red_when_red, blue_when_red) = field("1 0 0 rg");
    let (red_when_blue, blue_when_blue) = field("0 0 1 rg");
    assert!(red_when_red > 0, "the /DA's red never reached the glyphs");
    assert!(
        blue_when_blue > 0,
        "the /DA's blue never reached the glyphs"
    );
    assert_eq!(blue_when_red, 0, "red text drew blue pixels");
    assert_eq!(red_when_blue, 0, "blue text drew red pixels");
    assert_eq!(
        red_when_red, blue_when_blue,
        "the same value in two colours drew different shapes"
    );
}

/// A `/DA` naming a font the `/DR` does not define draws the value anyway, and says so.
///
/// §12.7.4.3 makes the match a requirement — "The specified font value shall match a resource
/// name in the Font entry of the default resource dictionary" — and states no recovery for a
/// document that breaks it. What it *does* state is that the value goes on the page, so the
/// field is drawn in a stand-in and the report names the font the `/DR` lacked: two true
/// statements where the refusal that stood until the hundred-and-twenty-third session made one
/// false page. Six corpus documents write such a `/DA`, five of them naming `/Helv`.
#[test]
fn a_font_the_default_resources_lack_is_stood_in_for_and_named() {
    let (reports, raster) = draw(pdf_with(
        "",
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
         /T (field) /V (Hi) /DA (/Nope 12 Tf 0 g) >>",
    ));
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(
        reports[0].contains("/Nope") && reports[0].contains("/DR"),
        "the report must name the font and where it was looked for: {reports:?}"
    );
    assert!(
        !inked_columns(&raster).is_empty(),
        "and the value must still be on the page"
    );
}

/// A `/DA` naming one of the fourteen conventional abbreviations is not a stand-in.
///
/// The same malformation as the test above — §12.7.4.3's "[t]he specified font value shall match
/// a resource name in the Font entry of the default resource dictionary", broken — and a
/// different answer, because `/Helv` is not an arbitrary resource name. The fourteen four-letter
/// abbreviations are a bijection with §9.6.2.2's fourteen font programs, and this binary carries
/// all fourteen (ADR 0133). So the value is drawn in the face the name means, from the binary,
/// and nothing is owed: five corpus documents name `/Helv` and none of them defines it.
///
/// **What it buys is stated in ADR 0133's terms**: those pages reproduce on a machine with no
/// fonts installed, where a stand-in chosen by family match would draw them in whatever sans
/// face the machine happened to offer.
#[test]
fn a_da_naming_a_standard_fourteen_abbreviation_is_drawn_from_the_binary() {
    let (reports, raster) = draw(pdf_with(
        "",
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
         /T (field) /V (Hi) /DA (/Helv 12 Tf 0 g) >>",
    ));
    assert!(reports.is_empty(), "nothing is owed: {reports:?}");
    assert!(!inked_columns(&raster).is_empty(), "and it is on the page");

    // And the narrowness is the point: a name that is not one of the fourteen still stands in
    // and still says so.
    let (reports, _) = draw(pdf_with(
        "",
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
         /T (field) /V (Hi) /DA (/Helvetical 12 Tf 0 g) >>",
    ));
    assert_eq!(reports.len(), 1, "{reports:?}");
}

/// A no-break space is written as a space, because the encoding gives it no code of its own.
///
/// Annex D's note 6 — under §9.6.5.2's Latin character set — says the space U+0020 is *also*
/// encoded at 240 octal in `WinAnsiEncoding`, that Windows Code Page 1252 associates that code
/// with the non-breaking space U+00A0, and that a producer meaning the second must say so with a
/// `/Differences` array naming `nonbreakingspace`.
///
/// So a `/V` holding U+00A0 — `bug1871353.pdf`'s does, nine of them between two letters — has a
/// character §12.7.4.3 must write and the encoding cannot spell. The two characters differ only
/// in where a line may break, and this module breaks on the value's own white space before any
/// code exists; what the alternative costs is the whole field, because a font this program
/// inferred may not fall short.
#[test]
fn a_no_break_space_is_drawn_as_a_space() {
    let (reports, raster) = draw(pdf_with(
        "",
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
         /T (field) /V <FEFF004100A000A00042> /DA (/Helv 12 Tf 0 g) >>",
    ));
    assert!(reports.is_empty(), "{reports:?}");
    let columns = inked_columns(&raster);
    assert!(!columns.is_empty(), "A and B are drawn");
    // Two no-break spaces between the letters, so the gap is wider than one glyph.
    let (first, last) = ink_span(&raster);
    assert!(
        last.saturating_sub(first) > 20,
        "the spaces take room: {first}..{last}"
    );
}

/// A stand-in draws the whole value or none of it, never part.
///
/// The asymmetry is the finding, and `freetext_no_appearance.pdf` is where it came from: its
/// value is a paragraph of Arabic, and a Latin stand-in draws its spaces and full stops and
/// nothing else — a scatter of dots on an otherwise empty page, which is trap 1's archetype and
/// worse than the blank a refusal leaves. Where the *document* names the font, a code it lacks
/// is reported and the rest is drawn, because there the shortfall is the document's own choice.
///
/// **Which of the two outcomes this machine gets is a property of this machine**, and that is
/// why the assertion is the equivalence rather than the blank. Until the
/// four-hundred-and-thirty-fourth session a stand-in was always one of §9.6.2.2's compiled-in
/// faces, none of which has Arabic, so the refusal was the only outcome and this test asserted
/// it. Since ADR 0270 a substituted face is chosen by whether it covers the characters the
/// encoding names — here an invented `/Differences` naming four Arabic glyphs — so a machine
/// with an Arabic face draws the value in full and a machine without one still declines. What
/// holds on both, and is what the test is for, is that the two go together: ink exactly when
/// nothing was owed for a missing code.
#[test]
fn a_stand_in_draws_the_whole_value_or_none_of_it() {
    let (reports, raster) = draw(pdf_with(
        "",
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
         /T (field) /V <FEFF0627064406220646> /DA (/Nope 12 Tf 0 g) >>",
    ));
    // The `/DR` that does not define `/Nope` is reported either way: that is the document's
    // defect and no face on any machine changes it.
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(reports[0].contains("/Nope"), "{reports:?}");
    let fell_short = reports[0].contains("states no code for");
    assert_eq!(
        inked_columns(&raster).is_empty(),
        fell_short,
        "a value a stand-in cannot show may not be shown in part: {reports:?}"
    );
}

/// The Arabic free text annotation declines whole and names both halves (§12.5.6.6, §12.7.4.3).
///
/// `freetext_no_appearance.pdf` is the one corpus document §12.7.4.3's construction refuses: a
/// paragraph of Arabic under `/DA (/Helv 10 Tf 0 g)`, no `/AP`, no `/DR`, and the page has no
/// other content at all — so what this test pins is that the page stays *blank with a report*
/// rather than becoming a partial or reordered drawing. Both alternatives were looked at and are
/// worse (trap 1): `pdftoppm` lays out what its Latin face can represent and draws the value's
/// full stops scattered over an otherwise empty page, and a face with the glyphs but without
/// Unicode's joining-form selection and right-to-left ordering would draw isolated forms
/// left-to-right — a wrong-but-plausible page that reports nothing. ADR 0348 is the reading of
/// what drawing this value would actually take, and why nothing in this binary can start it: no
/// compiled-in face has one Arabic glyph.
///
/// The refusal is machine-independent twice over, which is what lets a picture assertion live in
/// a gate: the value has more distinct missing characters than the invented `/Differences` has
/// free codes, and the Adobe Glyph List `read-fonts` carries has no name for any of them — so
/// `named_glyphs_reach_more` cannot reach an installed face on any machine.
#[test]
fn the_arabic_free_text_declines_whole_and_names_both_halves() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs/freetext_no_appearance.pdf");
    let Ok(bytes) = std::fs::read(path) else {
        // The pdf.js corpus is an optional submodule; without it there is nothing to open.
        return;
    };
    let (reports, raster) = draw(bytes);
    assert!(
        inked_columns(&raster).is_empty(),
        "the refusal is whole: a partial drawing of this value is trap 1's archetype"
    );
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(
        reports[0].contains("/Helv") && reports[0].contains("not drawn at all"),
        "the report names the undefined name and the wholeness of the refusal: {reports:?}"
    );
}

/// A check box shows Table 192's caption only in its on state (§12.7.5.2.3).
///
/// The clause makes `/AS` decide, so the two fixtures differ in nothing but that name. An
/// implementation that drew the caption unconditionally would tick every box on the page.
#[test]
fn a_check_box_shows_its_caption_only_when_it_is_on() {
    let box_of = |state: &str| {
        pdf_with(
            "",
            &format!(
                "<< /Type /Annot /Subtype /Widget /Rect [20 40 60 70] /F 4 /FT /Btn \
                 /T (box) /V /{state} /AS /{state} /MK << /CA (4) >> /DA (/Helv 12 Tf 0 g) >>"
            ),
        )
    };
    let (on_reports, on) = draw(box_of("Yes"));
    let (off_reports, off) = draw(box_of("Off"));

    assert!(on_reports.is_empty(), "{on_reports:?}");
    assert!(off_reports.is_empty(), "{off_reports:?}");
    assert!(
        !inked_columns(&on).is_empty(),
        "a box that is on must show its caption"
    );
    assert!(
        inked_columns(&off).is_empty(),
        "a box that is off must show nothing"
    );
}

/// A person checking a box changes what the page draws (§12.7.5.2.3).
///
/// > The value of the V key shall also be the value of the AS key. If they are not equal, then the
/// > value of the AS key shall be used instead of the V key to determine which appearance to use.
///
/// **A `shall` this tree obeyed only backwards until the three-hundred-and-ninety-eighth
/// session.** The file's `/AS` decided, always — so a reader that changed `/V` left `/AS` behind
/// and the widget went on drawing the state it was saved in. The sentence binds both entries
/// together, and the processor that changes one is the one that has to carry the other; ADR 0235
/// records the reading.
///
/// Two fixtures, because the two halves of the clause meet the value by different routes: a box
/// with an `/AP` state subdictionary picks a stream by name, and one with none has
/// `crate::appearance` construct Table 192's `/CA` caption. Both were wrong and both are checked
/// here.
#[test]
fn checking_a_box_draws_the_state_the_new_value_names() {
    let stored = pdf_with_appearance(
        "",
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 60 70] /F 4 /FT /Btn \
         /T (box) /V /Off /AS /Off /AP << /N << /Yes 6 0 R >> >> >>",
        "0 0 1 rg 0 0 20 20 re f",
    );
    let constructed = pdf_with(
        "",
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 60 70] /F 4 /FT /Btn \
         /T (box) /V /Off /AS /Off /MK << /CA (4) >> /DA (/Helv 12 Tf 0 g) >>",
    );
    for (what, bytes) in [
        ("a stored /AP state", stored),
        ("a /CA caption", constructed),
    ] {
        let document = Document::open(bytes).expect("the fixture is a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let mut view = pdf_model::view::ViewState::of(&document);
        let before = draw_with(&document, &page, &view);
        assert!(
            inked_columns(&before).is_empty(),
            "{what}: the box starts off and shows nothing"
        );

        // What a host sends: the name §12.7.5.2.3 makes `/V`, which `pdf_model::form` answers
        // with as the widget's on state. Nothing else in the file could tell it that string.
        assert_eq!(
            view.set_field(&document, "box", &Entered::Text("Yes".to_owned())),
            1,
            "{what}: one widget takes the value"
        );
        let after = draw_with(&document, &page, &view);
        assert!(
            !inked_columns(&after).is_empty(),
            "{what}: a checked box shows its on state"
        );

        // And back off again, which is the same rule with §12.7.5.2.4's default in it.
        assert_eq!(
            view.set_field(&document, "box", &Entered::Text("Off".to_owned())),
            1
        );
        assert!(
            inked_columns(&draw_with(&document, &page, &view)).is_empty(),
            "{what}: unchecking it takes the mark away"
        );
    }
}

/// A check box that is on with no `/AP` and no caption states a tick and shows none.
#[test]
fn a_check_box_with_nothing_to_draw_says_so() {
    let (reports, _) = draw(pdf_with(
        "",
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 60 70] /F 4 /FT /Btn \
         /T (box) /V /Yes /AS /Yes >>",
    ));
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(reports[0].contains("12.7.5.2.3"), "{reports:?}");
}

/// §12.5.6.6: a free text annotation's `/Contents` is its appearance.
#[test]
fn a_free_text_annotation_draws_its_contents() {
    let (reports, raster) = draw(pdf_with(
        "",
        "<< /Type /Annot /Subtype /FreeText /Rect [20 40 180 70] /F 4 \
         /Contents (visible) /DA (/Helv 12 Tf 0 g) /Border [0 0 0] >>",
    ));
    assert!(reports.is_empty(), "{reports:?}");
    let (start, end) = ink_span(&raster);
    assert!(
        start >= 20 && end <= 180,
        "the text escaped the annotation: {start}..{end}"
    );
}

/// Table 177's `/RC` generates the appearance where the file states no `/Contents` (§12.5.6.6).
///
/// > A rich text string (see Adobe XML Architecture, XML Forms Architecture (XFA) Specification,
/// > version 3.3 ) that shall be used to generate the appearance of the annotation.
///
/// A `shall` about this annotation's own marks, which is what separates it from the entry of the
/// same name on Table 172: the NOTE beside it says "[a]s freetext annotations do not have an open
/// state this cannot apply to the popup window". The characters are taken and the XFA markup is
/// not, which is ADR 0199's reading applied to the second of the standard's two `/RC` entries.
/// Nothing in the corpus reaches this — 22 free text annotations state `/RC` and every one of
/// them also states `/Contents` (`examples/markup_text_census`) — so this test is the whole
/// defence. ADR 0224.
#[test]
fn a_free_text_annotations_rich_text_draws_where_it_states_no_contents() {
    let (reports, raster) = draw(pdf_with(
        "",
        "<< /Type /Annot /Subtype /FreeText /Rect [20 40 180 70] /F 4 \
         /RC (<body xmlns=\"http://www.w3.org/1999/xhtml\"><p>visible</p></body>) \
         /DA (/Helv 12 Tf 0 g) /Border [0 0 0] >>",
    ));
    assert!(reports.is_empty(), "{reports:?}");
    let (start, end) = ink_span(&raster);
    assert!(
        start >= 20 && end <= 180,
        "the rich text escaped the annotation: {start}..{end}"
    );
}

/// `/Contents` outranks `/RC`, which is §12.5.6.2's NOTE 1 read for this subtype.
///
/// > When both Contents and RC entries are present, it is expected that the contents of both
/// > entries are textually equivalent.
///
/// Expected rather than required, so the two can disagree — and the plain string is the one this
/// crate can hand over without reading a specification it does not have. The fixture makes them
/// disagree in *width*, which is the only thing a raster can be asked: a one-character
/// `/Contents` beside a long `/RC` must ink like the short one.
#[test]
fn a_free_text_annotations_contents_outranks_its_rich_text() {
    let (reports, raster) = draw(pdf_with(
        "",
        "<< /Type /Annot /Subtype /FreeText /Rect [20 40 180 70] /F 4 /Contents (i) \
         /RC (<body xmlns=\"http://www.w3.org/1999/xhtml\"><p>wwwwwwwwwwwwwwww</p></body>) \
         /DA (/Helv 12 Tf 0 g) /Border [0 0 0] >>",
    ));
    assert!(reports.is_empty(), "{reports:?}");
    let (start, end) = ink_span(&raster);
    let width = end.saturating_sub(start);
    assert!(
        width < 20,
        "the /Contents is one narrow character; /RC must not have been laid out: {start}..{end}"
    );
}

/// Table 224's `/NeedAppearances` rewrites the `/Tx` region and leaves the rest (§12.7.4.3).
///
/// > The interactive PDF processor shall then replace the existing contents of the appearance
/// > stream from … BMC to the matching EMC with the corresponding new contents
///
/// > If the existing appearance stream contains no marked-content with tag … the new contents
/// > shall be appended to the end of the original stream.
///
/// Two fixtures differing in one thing — whether the blue square sits inside the marked-content
/// pair — and the clause gives them opposite answers. A reader that rebuilt the appearance from
/// `/MK` instead would erase the square in both, and one that ignored the flag would show the
/// value in neither.
#[test]
fn need_appearances_rewrites_the_marked_content_region_and_keeps_the_rest() {
    let field = |appearance: &str| {
        pdf_with_appearance(
            "/NeedAppearances true",
            "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
             /T (field) /V (Hi) /AP << /N 6 0 R >> /DA (/Helv 12 Tf 0 g) >>",
            appearance,
        )
    };

    let (inside_reports, inside) = draw(field("/Tx BMC 0 0 1 rg 0 0 160 30 re f EMC"));
    assert!(inside_reports.is_empty(), "{inside_reports:?}");
    assert!(
        !is_blue(&inside),
        "marks inside /Tx BMC … EMC are replaced by the new value"
    );
    assert!(
        !inked_columns(&inside).is_empty(),
        "the value must be drawn"
    );

    let (outside_reports, outside) = draw(field("0 0 1 rg 0 0 160 30 re f"));
    assert!(outside_reports.is_empty(), "{outside_reports:?}");
    assert!(
        is_blue(&outside),
        "marks outside the pair survive; the new contents are appended"
    );
}

/// The flag does not reach a button, because §12.7.5.2.3 puts its appearance in `/AP`.
///
/// §12.7.4.3's subject is a field whose text "is not known until viewing time". A check box
/// has no such text — its value selects among stored appearance streams — so regenerating one
/// would throw away artwork the file does state in exchange for nothing the clause asks for.
#[test]
fn need_appearances_does_not_reach_a_button() {
    let (reports, raster) = draw(pdf_with_appearance(
        "/NeedAppearances true",
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Btn \
         /T (field) /V /Hi /AS /Hi /AP << /N 6 0 R >> /DA (/Helv 12 Tf 0 g) >>",
        "/Tx BMC 0 0 1 rg 0 0 160 30 re f EMC",
    ));
    assert!(reports.is_empty(), "{reports:?}");
    assert!(
        is_blue(&raster),
        "a button's appearance is its /AP, whatever /NeedAppearances says"
    );
}

/// Whether anything on the page is the fixtures' blue.
fn is_blue(raster: &pdf_render::Raster) -> bool {
    (0..raster.height)
        .flat_map(|row| (0..raster.width).map(move |x| (x, row)))
        .any(|(x, row)| {
            let index =
                (row.saturating_mul(raster.width).saturating_add(x) as usize).saturating_mul(4);
            raster.data[index.saturating_add(2)] > 200
                && raster.data[index] < 60
                && raster.data[index.saturating_add(3)] > 200
        })
}

/// §12.7.6.3: a reset-form action draws the field's *default* value instead of its value.
///
/// The clause is a display change, not only a data one: it "shall set the value of the V entry
/// in the field dictionary to that of the DV entry", and §12.7.4.3 lays out whatever that entry
/// holds. So the same page, drawn twice from the same file, differs by exactly one action —
/// which is the property `ViewState` exists for, and the reason nothing is written back to the
/// document.
///
/// Two fields, because the clause states two outcomes: one with a `/DV`, whose text changes, and
/// one without, whose "V entry shall be removed" and which then draws nothing at all.
#[test]
fn a_reset_form_action_draws_the_default_value_instead_of_the_value() {
    let with_default = pdf_with(
        "/NeedAppearances false",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (f) /V (typed) /DV (default) \
         /Rect [20 40 180 70] /DA (/Helv 12 Tf 0 g) >>",
    );
    let document = Document::open(with_default).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");

    let before = pdf_model::interpret(&document, &page);
    let mut state = pdf_model::view::ViewState::of(&document);
    let action = pdf_model::action::read(
        &document,
        &pdf_syntax::Object::Dictionary({
            let mut dict = pdf_syntax::Dictionary::new();
            dict.insert(
                pdf_syntax::Name::new(b"S".to_vec()),
                pdf_syntax::Object::Name(pdf_syntax::Name::new(b"ResetForm".to_vec())),
            );
            dict
        }),
    );
    state.perform_all(&document, &action);
    let after = pdf_model::content::interpret_with(&document, &page, &state);

    assert!(
        before.glyphs > 0 && after.glyphs > 0,
        "both draw text, or this machine has no sans-serif face and the test is vacuous"
    );
    assert_ne!(
        before.glyphs, after.glyphs,
        "\"typed\" and \"default\" are different numbers of glyphs, so the marks differ"
    );

    // The other outcome: no `/DV` anywhere, so the value is *removed* and nothing is laid out.
    let without_default = pdf_with(
        "/NeedAppearances false",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (f) /V (typed) \
         /Rect [20 40 180 70] /DA (/Helv 12 Tf 0 g) >>",
    );
    let document = Document::open(without_default).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut state = pdf_model::view::ViewState::of(&document);
    state.perform_all(&document, &action);
    let after = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(
        pdf_model::interpret(&document, &page).glyphs > 0,
        "the field draws its value before the reset"
    );
    assert_eq!(
        after.glyphs, 0,
        "and after it draws nothing, which is what \"its V entry shall be removed\" means \
         for a program that does not write to the file"
    );
}

/// §12.5.6.19's Table 192 `/R`: a widget's contents are turned inside `/Rect`.
///
/// > The number of degrees by which the widget annotation shall be rotated counterclockwise
/// > relative to the page. The value shall be a multiple of 90. Default value: 0 .
///
/// The measurement is the *shape of the ink*, because that is what a quarter turn changes and
/// nothing else about the fixture does. A line of text in a wide, short field is wider than it
/// is tall; turned a quarter, it is taller than it is wide — and it still lands inside `/Rect`,
/// which is the half a rotation is easy to get wrong.
#[test]
fn a_widgets_contents_turn_by_table_192s_r() {
    let field = |rotation: &str| {
        pdf_with(
            "",
            &format!(
                "<< /Type /Annot /Subtype /Widget /Rect [20 20 180 80] /F 4 /FT /Tx \
                 /T (field) /V (Illlllllll) /DA (/Helv 10 Tf 0 g) /MK << {rotation} >> >>"
            ),
        )
    };

    let (reports, upright) = draw(field(""));
    assert!(reports.is_empty(), "{reports:?}");
    let (columns, rows) = (inked_columns(&upright), inked_rows(&upright));
    let (wide, tall) = (span(&columns), span(&rows));
    assert!(
        wide > tall,
        "a line of text is wider than it is tall: {wide}x{tall}"
    );

    let (reports, turned) = draw(field("/R 90"));
    assert!(
        reports.is_empty(),
        "a quarter turn is drawn, not refused: {reports:?}"
    );
    let (columns, rows) = (inked_columns(&turned), inked_rows(&turned));
    assert!(
        span(&rows) > span(&columns),
        "turned a quarter it is taller than it is wide: {}x{}",
        span(&columns),
        span(&rows)
    );
    // §12.5.5: an appearance is "rendered inside the annotation rectangle", and the turn must
    // not take it out of one.
    assert!(
        columns.first().copied().unwrap_or_default() >= 20
            && columns.last().copied().unwrap_or_default() <= 180,
        "inside /Rect's x range: {:?}..{:?}",
        columns.first(),
        columns.last()
    );
    assert!(
        rows.iter().min().copied().unwrap_or_default() >= 20
            && rows.iter().max().copied().unwrap_or_default() <= 80,
        "inside /Rect's y range: {:?}..{:?}",
        rows.iter().min(),
        rows.iter().max()
    );

    // A half turn keeps the box and reverses the direction, so the ink is still wide — and it is
    // in a different place, because left-justified text starts at the other end.
    let (reports, half) = draw(field("/R 180"));
    assert!(reports.is_empty(), "{reports:?}");
    assert!(span(&inked_columns(&half)) > span(&inked_rows(&half)));
    assert_ne!(
        ink_span(&half),
        ink_span(&upright),
        "a half turn is not the identity for text"
    );

    // "The value shall be a multiple of 90" is a requirement on the file, so 45 is not a
    // rotation this could draw and is refused by name rather than rounded.
    let (reports, _) = draw(field("/R 45"));
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(reports[0].contains("multiple of 90"), "{reports:?}");
}

/// The extent of a list of inked coordinates, or 0 where nothing was drawn.
///
/// `max − min` rather than `last − first`: `inked_rows` returns PDF y, which runs the other way
/// from the raster's rows, so the list it produces is descending.
fn span(values: &[u32]) -> u32 {
    let (Some(low), Some(high)) = (values.iter().min(), values.iter().max()) else {
        return 0;
    };
    high.saturating_sub(*low)
}

/// A one-page fixture holding one `Ch` widget, with the entries the caller spells added to it.
///
/// The `/Rect` is 160 × 30 at 12 points of leading, so three options are more than it holds —
/// which is the arrangement §12.7.5.4 calls "a scrollable list box" and the one Table 234's
/// `/TI` exists for.
fn choice_field(entries: &str) -> Vec<u8> {
    choice_field_sized("/Helv 12 Tf 0 g", entries)
}

/// The same, with the `/DA` given — which is the entry a test about auto-sizing has to move.
///
/// A parameter rather than another entry in `entries`, because §7.3.7 gives a dictionary "an
/// associative table containing pairs of objects" and this tree's parser answers a duplicated
/// key with the first of the two: a fixture appending a second `/DA` would state a size and be
/// laid out at the first one, silently, and this test suite spent a round finding that out.
fn choice_field_sized(default_appearance: &str, entries: &str) -> Vec<u8> {
    pdf_with(
        "",
        &format!(
            "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Ch \
             /T (choice) /DA ({default_appearance}) {entries} >>"
        ),
    )
}

/// ISO 32000-2 §12.7.5.4: a combo box draws its value, and a list box draws its `/Opt` array.
///
/// > The Opt array specifies the list of options in the choice field, each of which shall be
/// > represented by a text string that shall be displayed on the screen.
///
/// The two halves of the clause are one bit apart, which is why they are one test. A combo box
/// shows its *value* in an edit box, so it is a text field with a different `/FT`; a list box
/// shows the *options*, and Table 233 bit 20 fixes their order — "PDF readers shall display the
/// options in the order in which they occur in the Opt array".
///
/// **This test used to assert that a list box draws nothing**, on the reasoning that §12.7.5.4
/// states no appearance for the selection. It states none for the *highlight*; it states the
/// options outright, and a mark added over an item that is drawn either way may not take the
/// item down with it (ADR 0106's test). So the options are drawn and the missing mark is
/// reported — checked by name, because a report that fired for some other reason would satisfy
/// "something was reported".
#[test]
fn a_combo_box_draws_its_value_and_a_list_box_draws_its_options() {
    let options = "/V (Beta) /Opt [(Alpha) (Beta) (Gamma)]";

    // Bit 18 is the combo flag, which is 1 << 17.
    let (reports, raster) = draw(choice_field(&format!("{options} /Ff 131072")));
    assert!(
        !inked_columns(&raster).is_empty(),
        "a combo box draws its value like a text field: {reports:?}"
    );
    assert!(
        reports.is_empty(),
        "and reports nothing while doing it: {reports:?}"
    );

    let (reports, text) = read_back(choice_field(options));
    assert_eq!(
        text.replace('\n', " ").trim(),
        "Alpha Beta Gamma",
        "a list box draws every option, in the array's own order"
    );
    let named = reports.iter().any(|report| report.contains("selects"));
    assert!(
        named,
        "and says that which of them the value selects is not marked: {reports:?}"
    );
}

/// Table 234's `/TI`: "the index in the Opt array of the first option visible in the list".
///
/// The entry only means something where the array is longer than the box shows, which is what
/// makes it the clause's own statement that a list box is a *window* onto its options. Three
/// values, and the third is the one a hand-written fixture would never think of: the default,
/// an index inside the array, and one past its end — where obeying the entry literally would
/// erase what §12.7.5.4 says shall be displayed, so it is clamped to the last option instead
/// (ADR 0111's rule about an optional entry).
#[test]
fn the_top_index_says_which_option_the_list_starts_at() {
    let with = |entry: &str| {
        read_back(choice_field(&format!(
            "/V (Gamma) /Opt [(Alpha) (Beta) (Gamma)] {entry}"
        )))
        .1
        .replace('\n', " ")
        .trim()
        .to_owned()
    };

    assert_eq!(with(""), "Alpha Beta Gamma", "the table's default is 0");
    assert_eq!(with("/TI 1"), "Beta Gamma", "and the list starts there");
    assert_eq!(
        with("/TI 9"),
        "Gamma",
        "an index the array does not have scrolls to its end rather than emptying the list"
    );
}

/// Table 234's own answer for a choice field that states no `/Opt` (ISO 32000-2 §12.7.5.4).
///
/// > If this entry is not present, no choices should be presented to the user.
///
/// So an empty list box is the entry's absence working, not a shortfall — nothing is drawn and
/// nothing is owed. And §12.7.5.4 gives `/V` the default null, "indicating that no item is
/// currently selected": with the options present and nothing selected there is no mark to make,
/// so the list draws and reports nothing either. Trap 11's rule, on the report's own condition.
#[test]
fn a_list_box_reports_only_what_the_clause_leaves_unsaid() {
    let (reports, raster) = draw(choice_field("/V (Beta)"));
    assert!(
        inked_columns(&raster).is_empty(),
        "no /Opt, so no choices are presented: {reports:?}"
    );
    assert!(reports.is_empty(), "and none is owed: {reports:?}");

    let (reports, text) = read_back(choice_field("/Opt [(Alpha) (Beta)]"));
    assert_eq!(
        text.replace('\n', " ").trim(),
        "Alpha Beta",
        "the options are drawn with no value selecting any of them"
    );
    assert!(
        reports.is_empty(),
        "and nothing is owed, because there is no selection to mark: {reports:?}"
    );
}

/// Choosing an item rebuilds the list box's appearance, which is where this was owed.
///
/// §12.7.4.3's own NOTE names the case: "scrollable list boxes whose contents are determined
/// interactively at the time the document is displayed". Every list box in the pdf.js corpus
/// states an `/AP`, so the *file's* picture is right for a document nobody has touched — and
/// this program has been able to choose an item since `Edit::SetField` learned to carry indices
/// (ADR 0248), after which the page went on showing the producer's stream and the clause's
/// splice could not be performed. The fixture makes that visible by filling the `/Tx` region
/// with a blue square: what the page shows before the choice is the file's own marks, and what
/// it shows after is §12.7.5.4's array in their place.
#[test]
fn choosing_an_item_replaces_the_stored_list_with_the_clauses_own_options() {
    let bytes = pdf_with_appearance(
        "",
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Ch /T (choice) \
         /V (Alpha) /Opt [(Alpha) (Beta)] /AP << /N 6 0 R >> /DA (/Helv 12 Tf 0 g) >>",
        "/Tx BMC 0 0 1 rg 0 0 160 30 re f EMC",
    );
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");

    let mut view = pdf_model::view::ViewState::of(&document);
    let untouched = pdf_model::interpret(&document, &page);
    assert!(
        untouched.text.trim().is_empty() && is_blue(&draw_with(&document, &page, &view)),
        "an untouched document shows the stream its producer wrote: {:?}",
        untouched.text
    );
    assert!(untouched.unsupported.is_empty(), "{untouched:?}");

    let applied = view.set_field(&document, "choice", &Entered::Chosen(vec![1]));
    assert_eq!(applied, 1, "one widget takes the choice");
    assert!(
        !is_blue(&draw_with(&document, &page, &view)),
        "and the marks inside /Tx BMC … EMC give way to the new contents"
    );

    let chosen = pdf_model::content::interpret_with(&document, &page, &view);
    assert_eq!(
        chosen.text.replace('\n', " ").trim(),
        "Alpha Beta",
        "and a document whose value a person changed shows the options"
    );
    let reports: Vec<String> = chosen
        .unsupported
        .iter()
        .map(|item| format!("{item:?}"))
        .collect();
    assert!(
        reports.iter().any(|report| report.contains("selects")),
        "with the mark that would say which is chosen named as owed: {reports:?}"
    );
}

/// §12.7.4.3's auto-sizing fits the options Table 234's `/TI` makes visible, not the whole array.
///
/// > A zero value for size means that the font shall be auto-sized : its size shall be computed
/// > as an implementation dependent function.
///
/// The clause hands the function over and this tree gives every shape the same one — the largest
/// size at which what is laid out fits the box — so what decides a list box's size is *which
/// options are laid out*, and `/TI` decides that: "the index in the Opt array of the first
/// option visible in the list". A reader that laid out the whole array and scrolled afterwards
/// would size a scrolled list exactly as it sizes an unscrolled one, which is what this
/// discriminates. Same array, same box, one entry apart.
#[test]
fn auto_sizing_a_list_box_fits_the_options_the_top_index_shows() {
    let options = (0..20).fold(String::new(), |mut all, index| {
        let _ = write!(all, "(Item{index}) ");
        all
    });
    let at = |top: &str| {
        let (reports, raster) = draw(choice_field_sized(
            "/Helv 0 Tf 0 g",
            &format!("/V (Item19) /Opt [{options}] {top}"),
        ));
        (reports, span(&inked_columns(&raster)))
    };

    // The ink's *width* rather than its height, and the fixture is why: twenty options in a
    // 30-point box are set small enough that their rows touch, so a band of ink says nothing
    // about one line. `Item18` and `Item19` are the widest option in both fixtures, so how far
    // across the box the ink reaches is the size, measured on the axis the lines do not share.
    let (whole, narrow) = at("");
    let (scrolled, wide) = at("/TI 18");
    assert!(
        narrow > 0,
        "twenty options are drawn at some size: {whole:?}"
    );
    assert!(
        wide > narrow.saturating_mul(2),
        "and the two the /TI leaves are drawn far larger: {narrow} then {wide}, {scrolled:?}"
    );
}

/// What §12.7.5.4's options put on the page, and what the page says it could not do.
///
/// The readback rather than the ink, because the question is *which* strings were laid out and
/// no measurement of pixels can say that (ADR 0299's rule, one clause over).
fn read_back(bytes: Vec<u8>) -> (Vec<String>, String) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let reports = interpretation
        .unsupported
        .iter()
        .map(|item| format!("{item:?}"))
        .collect();
    (reports, interpretation.text.clone())
}

/// What a person typing into one field reaches the page as.
///
/// The readback rather than the ink, because what is being checked here is *which characters*
/// were laid out and no measurement of pixels can say that.
fn typed(bytes: Vec<u8>, value: &str) -> (usize, String) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let mut view = pdf_model::view::ViewState::of(&document);
    let applied = view.set_field(&document, "field", &Entered::Text(value.to_owned()));
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::content::interpret_with(&document, &page, &view);
    (applied, interpretation.text.clone())
}

/// The longest prefix of `value` the readback holds, which is what "no further text" leaves.
///
/// The readback puts a line break between two lines the layout wrapped and the value's own space
/// stays at the end of the first, so dropping the breaks turns the readback back into the run of
/// characters that was accepted.
fn prefix_shown(value: &str, shown: &str) -> usize {
    let shown = shown.replace('\n', "");
    value
        .char_indices()
        .map(|(at, character)| at.saturating_add(character.len_utf8()))
        .take_while(|end| {
            value
                .get(..*end)
                .is_some_and(|prefix| shown.contains(prefix))
        })
        .last()
        .unwrap_or_default()
}

/// §12.7.5.3's Table 231 bit 24: a field that may not scroll stops accepting text when it is full.
///
/// > If set, the field shall not scroll (horizontally for single-line fields, vertically for
/// > multiple-line fields) to accommodate more text than fits within its annotation rectangle.
/// > Once the field is full, no further text shall be accepted for interactive form filling
///
/// Two sentences and only the second binds a reader — a `shall` about *accepting* text, which
/// became this tree's to obey in the hundred-and-thirty-fifth session, when `set_field` made it
/// a program a person fills a field with. The same value goes into the same box twice and the
/// fixtures differ in one flag, so this is a test of the clause rather than of a layout: with
/// the bit clear the value is taken whole and the box clips it, with the bit set the value is
/// cut where the box ends and the characters past that were never accepted.
#[test]
fn a_field_that_may_not_scroll_takes_only_what_fits() {
    let long = "the quick brown fox jumps over the lazy dog";

    let (applied, shown) = typed(text_field("", "", ""), long);
    assert_eq!(applied, 1);
    assert!(
        shown.contains(long),
        "with the flag clear the whole value is accepted: {shown:?}"
    );

    // Bit 24 is 1 << 23.
    let (applied, shown) = typed(text_field("", "", "/Ff 8388608"), long);
    assert_eq!(
        applied, 1,
        "the field still accepts text, just not all of it"
    );
    let kept = prefix_shown(long, &shown);
    assert!(
        kept > 0 && kept < long.len(),
        "a proper, non-empty prefix of the value: {kept} of {}",
        long.len()
    );
    // 160 points of twelve-point Helvetica is a little over twenty characters, and the bound is
    // stated as a range because the face is whatever this machine substitutes (see this file's
    // own header). What the clause decides is that it is *not* all forty-two.
    assert!(
        (10..40).contains(&kept),
        "roughly a box's worth of characters, not {kept}"
    );
}

/// The same flag on a multiline field, where §12.7.5.3's Table 231 bit 24 names the other axis:
/// "horizontally for single-line fields, vertically for multiple-line fields".
///
/// One axis per shape is the whole of the sentence's parenthesis, so the fixture is a box that
/// is wide enough for any of these words and tall enough for two lines of them: the value is cut
/// where the *lines* run out and not where a line does.
#[test]
fn a_multiline_field_that_may_not_scroll_stops_at_the_last_line_that_fits() {
    let value = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi \
                 omicron pi rho sigma tau upsilon phi chi psi omega";
    // Bit 13 (multiline, 1 << 12) with bit 24 (1 << 23).
    let field = |flags: &str| {
        pdf_with(
            "",
            &format!(
                "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx {flags} \
                 /T (field) /DA (/Helv 10 Tf 0 g) >>"
            ),
        )
    };

    let (_, whole) = typed(field("/Ff 4096"), value);
    assert_eq!(
        prefix_shown(value, &whole),
        value.len(),
        "with the flag clear the whole value is accepted: {whole:?}"
    );

    let (_, shown) = typed(field("/Ff 8392704"), value);
    let kept = prefix_shown(value, &shown);
    assert!(
        kept > 0 && kept < value.len(),
        "a proper, non-empty prefix of the value: {kept} of {}",
        value.len()
    );
    // Thirty points holds two lines of ten-point text at 13/12 leading and not three, and this
    // value takes five or so lines of this box's width — so what comes back is about two of
    // them, stated as a range because the face is whatever this machine substitutes.
    assert!(
        (30..90).contains(&kept),
        "about two lines' worth of characters, not {kept}"
    );
}

/// Where the caret goes, measured against the ink the same fixture draws.
///
/// **The standard states no caret** — §12.5.6.11's caret *annotation* is a different object, a
/// mark left in a document to say text was edited there — so what is checked here is not a
/// clause but the one relation that makes a text cursor mean anything: it stands where the next
/// character will be drawn. §12.7.4.3 is what decides that, since it is what lays the value out,
/// and the assertions below compare the caret against the marks that layout produced (ADR 0211).
fn caret_of(bytes: Vec<u8>, offset: usize) -> [f32; 4] {
    caret_in(bytes, offset, (100.0, 55.0))
}

/// The same, from a point the caller chooses inside the widget's `/Rect`.
fn caret_in(bytes: Vec<u8>, offset: usize, at: (f32, f32)) -> [f32; 4] {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let view = pdf_model::view::ViewState::of(&document);
    view.caret_at(&document, &page, at.0, at.1, offset)
        .expect("the widget under the point lays text out")
}

#[test]
fn a_caret_stands_where_the_next_character_will_be_drawn() {
    // `/Rect [20 40 180 70]`, left-justified, three characters at twelve points.
    let field = || text_field("abc", "", "");
    let (_, raster) = draw(field());
    let (first_ink, last_ink) = ink_span(&raster);

    let start = caret_of(field(), 0);
    let end = caret_of(field(), 3);

    // A caret has no width: both ends share an x, which is what lets a host stroke it at
    // whatever thickness its platform draws a text cursor at.
    assert!(
        (start[0] - start[2]).abs() < 0.001 && (end[0] - end[2]).abs() < 0.001,
        "a caret is a vertical segment: {start:?} {end:?}"
    );
    // Before the first character it is at the box's left edge, which is where the first glyph
    // starts — `/Rect`'s own edge inset by §12.5.4's border width, which this widget states
    // as the Table 168 default of 1.
    assert!(
        (start[0] - 21.0).abs() < 0.5,
        "the caret before a left-justified value sits at the left edge, not {}",
        start[0]
    );
    #[expect(
        clippy::cast_precision_loss,
        reason = "a column index on a 200-point page"
    )]
    let (first_ink, last_ink) = (first_ink as f32, last_ink as f32);
    assert!(
        (start[0] - first_ink).abs() < 2.0,
        "and within a pixel or two of the first mark at {first_ink}: {}",
        start[0]
    );
    // After the last character it is past the last mark and no further than one glyph beyond
    // it — the advance of a character rather than the width of its ink.
    assert!(
        end[0] > last_ink && end[0] < last_ink + 12.0,
        "the caret after the value stands just past the last mark at {last_ink}, not {}",
        end[0]
    );
    // And the segment spans one line: the standard 14 have no font descriptor, so
    // `variable_text`'s documented three-to-one split of the em is what states the ascent and
    // the descent, and at twelve points that is twelve points from end to end.
    let height = end[3] - end[1];
    assert!(
        (height - 12.0).abs() < 0.01,
        "one em of a twelve-point line, not {height}"
    );
}

#[test]
fn a_caret_in_an_empty_field_is_where_the_first_character_will_go() {
    // The case the layout skips when it is only drawing: an empty field draws nothing, and it
    // still has somewhere the next character goes — which is the whole of what a person clicking
    // into an untouched field is shown.
    let empty = caret_of(text_field("", "", ""), 0);
    let filled = caret_of(text_field("abc", "", ""), 0);
    assert!(
        (empty[0] - filled[0]).abs() < 0.001 && (empty[1] - filled[1]).abs() < 0.001,
        "an empty field's caret is where the same field's first character would be: \
         {empty:?} against {filled:?}"
    );
}

#[test]
fn a_caret_follows_the_quadding_the_field_states() {
    // Table 228's `/Q`: with the value centred, the place the *next* character goes moves with
    // the line rather than staying at the box's edge. Both carets are taken at the end of the
    // same value, so the only difference between them is the clause.
    let left = caret_of(text_field("abc", "/Q 0", ""), 3);
    let centred = caret_of(text_field("abc", "/Q 1", ""), 3);
    let right = caret_of(text_field("abc", "/Q 2", ""), 3);
    assert!(
        left[0] < centred[0] && centred[0] < right[0],
        "left {left:?}, centred {centred:?}, right {right:?}"
    );
    // Right-justified text ends at the right edge of the box, so the caret after the last
    // character stands there.
    assert!(
        (right[0] - 179.0).abs() < 0.5,
        "the caret after a right-justified value sits at the right edge, not {}",
        right[0]
    );
}

#[test]
fn a_caret_in_a_multiline_field_moves_down_a_line_at_a_time() {
    // Table 231 bit 13 with a value that wraps: an offset in the second line's characters puts
    // the caret on the second line, which is lower and further left than the end of the first.
    let value = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
    let field = || {
        pdf_with(
            "",
            &format!(
                "<< /Type /Annot /Subtype /Widget /Rect [20 10 100 90] /F 4 /FT /Tx /Ff 4096 \
                 /T (field) /V ({value}) /DA (/Helv 10 Tf 0 g) >>"
            ),
        )
    };
    let first = caret_in(field(), 0, (60.0, 50.0));
    let last = caret_in(field(), value.len(), (60.0, 50.0));
    assert!(
        last[1] < first[1] - 10.0,
        "the end of a wrapped value is at least one line below its start: {first:?} {last:?}"
    );
    assert!(
        first[1] > 70.0,
        "and a multiline field's first line starts at the top of its box, not {}",
        first[1]
    );
}

#[test]
fn a_caret_in_a_comb_field_stands_at_the_next_cell() {
    // Table 231 bit 25 divides the box into `/MaxLen` positions, so the place the next character
    // goes is a *cell* rather than a gap between glyphs: eight cells across the 158 points inside
    // `/Rect [20 40 180 70]`'s border are 19.75 apart, and each caret is one cell on from the
    // last.
    let comb = || {
        pdf_with(
            "",
            "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx /Ff 16777216 \
             /MaxLen 8 /T (field) /V (1234) /DA (/Helv 12 Tf 0 g) >>",
        )
    };
    let cells: Vec<f32> = (0..5).map(|at| caret_of(comb(), at)[0]).collect();
    for pair in cells.windows(2) {
        let step = pair[1] - pair[0];
        assert!(
            (step - 19.75).abs() < 0.01,
            "one comb of 19.75 points, not {step} ({cells:?})"
        );
    }
}

/// Which byte a point inside a value names, from the same fixtures the caret is measured on.
///
/// The caret's inverse, and **the standard states neither**: nothing in ISO 32000-2 describes a
/// text cursor, a click that places one or a selection inside a field's value. What is checked is
/// therefore the relation the two halves have to each other rather than a clause — an offset fed
/// to `caret_at` gives a place, and that place fed back to `offset_at` gives the offset (ADR
/// 0225).
fn offset_at(bytes: Vec<u8>, point: (f32, f32)) -> usize {
    offset_in(bytes, point, (100.0, 55.0))
}

/// The same, naming the widget from a point the caller chooses.
fn offset_in(bytes: Vec<u8>, point: (f32, f32), at: (f32, f32)) -> usize {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let view = pdf_model::view::ViewState::of(&document);
    view.offset_at(&document, &page, at, point)
        .expect("the widget under the point lays text out")
}

/// The shapes covering a range of a value, in default user space.
fn selection_of(bytes: Vec<u8>, from: usize, to: usize) -> Vec<[f32; 8]> {
    selection_in(bytes, from, to, (100.0, 55.0))
}

/// The same, naming the widget from a point the caller chooses.
fn selection_in(bytes: Vec<u8>, from: usize, to: usize, at: (f32, f32)) -> Vec<[f32; 8]> {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let view = pdf_model::view::ViewState::of(&document);
    view.field_selection(&document, &page, at, (from, to))
        .expect("the widget under the point lays text out")
}

#[test]
fn a_point_names_the_byte_whose_caret_stands_there() {
    // The round trip, over every position of the value: the caret for an offset is a place, and
    // that place names the offset again. Anything that moved the glyphs without moving the
    // boundaries — or the other way round — breaks this and nothing else in the file.
    let field = || text_field("abc", "", "");
    for offset in 0..=3 {
        let caret = caret_of(field(), offset);
        let middle = (caret[1] + caret[3]) * 0.5;
        assert_eq!(
            offset_at(field(), (caret[0], middle)),
            offset,
            "the caret at {offset} is at {caret:?}, and that point names {offset} again"
        );
    }
}

#[test]
fn a_point_outside_the_value_names_the_end_it_is_nearest() {
    // **Nearest rather than inside**, which is the choice a point in the empty part of a field
    // forces: a press a host has already decided belongs to the field has to leave the cursor
    // somewhere. `/Rect [20 40 180 70]` holds three characters at the left, so the right half of
    // the box is past the value entirely.
    let field = || text_field("abc", "", "");
    assert_eq!(offset_at(field(), (175.0, 55.0)), 3, "past the last glyph");
    assert_eq!(offset_at(field(), (21.0, 55.0)), 0, "before the first");
    // And an empty field answers the only offset it has rather than refusing, which is what a
    // click into an untouched field is.
    assert_eq!(offset_at(text_field("", "", ""), (100.0, 55.0)), 0);
}

#[test]
fn a_point_in_a_multiline_field_names_the_line_it_landed_on() {
    // Table 231 bit 13, with the same wrapped value the caret test uses: a point on the second
    // line names a byte after the first line's characters, which is the case a host cannot work
    // out for itself — where `wrap` broke the value is this crate's alone.
    let value = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
    let field = || {
        pdf_with(
            "",
            &format!(
                "<< /Type /Annot /Subtype /Widget /Rect [20 10 100 90] /F 4 /FT /Tx /Ff 4096 \
                 /T (field) /V ({value}) /DA (/Helv 10 Tf 0 g) >>"
            ),
        )
    };
    let first = caret_in(field(), 0, (60.0, 50.0));
    let second_line = first[1] - 12.0;
    let offset = offset_in(field(), (25.0, second_line), (60.0, 50.0));
    assert!(
        offset > 0 && offset < value.len(),
        "a point on the second line is inside the value, not {offset}"
    );
    // And it is where the caret goes back to: the two agree on the line as well as on the byte.
    // The caret is a segment from the line's descent to its ascent, so what "the same line" means
    // is that the point is between its ends.
    let back = caret_in(field(), offset, (60.0, 50.0));
    assert!(
        back[1] <= second_line && second_line <= back[3],
        "the offset a second-line point named puts the caret back on that line: {back:?} \
         against {second_line}"
    );
    assert!(
        back[1] < first[1],
        "and below the first line: {back:?} against {first:?}"
    );
}

#[test]
fn a_point_in_a_comb_field_names_the_cell_it_landed_on() {
    // Table 231 bit 25's eight cells of 19.75 points across `/Rect [20 40 180 70]`, so the third
    // cell begins at 20 + 1 + 2 × 19.75 = 60.5 and a point in the middle of it names the byte
    // that cell holds.
    let comb = || {
        pdf_with(
            "",
            "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx /Ff 16777216 \
             /MaxLen 8 /T (field) /V (1234) /DA (/Helv 12 Tf 0 g) >>",
        )
    };
    for cell in 0..4_usize {
        #[expect(clippy::cast_precision_loss, reason = "four cells")]
        let middle = 21.0 + (cell as f32 + 0.5) * 19.75;
        assert_eq!(
            offset_at(comb(), (middle, 55.0)),
            cell,
            "the middle of cell {cell} names the character in it"
        );
    }
}

#[test]
fn a_selection_covers_the_glyphs_between_two_offsets() {
    // One shape for a single-line value, and it runs from the caret at one end to the caret at
    // the other — the two are the same arithmetic, which is the property that keeps a highlight
    // under the text a person swept rather than beside it.
    let field = || text_field("abc", "", "");
    let start = caret_of(field(), 1);
    let end = caret_of(field(), 3);
    let quads = selection_of(field(), 1, 3);
    assert_eq!(quads.len(), 1, "one line, one shape: {quads:?}");
    let quad = quads[0];
    let (left, right) = (quad[0].min(quad[2]), quad[0].max(quad[2]));
    assert!(
        (left - start[0]).abs() < 0.001 && (right - end[0]).abs() < 0.001,
        "the shape runs between the two carets: {quad:?} against {start:?} and {end:?}"
    );
    // And it is as tall as the caret is, so a host draws a highlight the height of its cursor.
    let (low, high) = (quad[1].min(quad[5]), quad[1].max(quad[5]));
    assert!(
        (high - low - (start[3] - start[1])).abs() < 0.001,
        "the shape is one line tall: {quad:?} against {start:?}"
    );
    // Two equal offsets select nothing, which is what a caret is: the shapes are absent rather
    // than a rectangle of no width, because a host draws the caret itself for that.
    assert!(selection_of(field(), 2, 2).is_empty());
    // The order of the two ends does not change what is between them.
    let backwards = selection_of(field(), 3, 1);
    assert_eq!(backwards.len(), 1);
    assert!((backwards[0][0] - quad[0]).abs() < 0.001);
}

#[test]
fn a_selection_across_a_wrap_is_one_shape_per_line() {
    // **The case that makes this a question of its own rather than two carets.** A host holding
    // both ends of a selection could join them itself on one line; where §12.7.5.3's Multiline
    // flag let `wrap` break the value, the lines between the ends are this crate's to name.
    let value = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
    let field = || {
        pdf_with(
            "",
            &format!(
                "<< /Type /Annot /Subtype /Widget /Rect [20 10 100 90] /F 4 /FT /Tx /Ff 4096 \
                 /T (field) /V ({value}) /DA (/Helv 10 Tf 0 g) >>"
            ),
        )
    };
    let whole = selection_in(field(), 0, value.len(), (60.0, 50.0));
    assert!(
        whole.len() > 1,
        "a wrapped value selected end to end covers every line it wrapped onto: {whole:?}"
    );
    // Each shape is a line further down the box than the one before it, in the order the lines
    // are shown.
    for pair in whole.windows(2) {
        assert!(
            pair[1][1] < pair[0][1],
            "the shapes run down the box: {pair:?}"
        );
    }
}

/// An annotation a person drew, typed into, and asked where the cursor is (§12.5.6.6).
///
/// **The three questions ADR 0211 and ADR 0225 answered for a field, asked of an annotation.**
/// §12.5.6.6 sends its own subtype to §12.7.4.3 — "[s]ubclause 12.7.4.3, 'Variable text',
/// describes the process of using these entries to generate the appearance of the text in these
/// annotations" — so the layout is the same layout and the caret is the same arithmetic; what this
/// pins is that the *way in* reaches it. Nothing in the corpus can: no document contains an
/// annotation this program added, by construction.
#[test]
fn a_free_text_annotation_a_person_added_answers_the_carets_three_questions() {
    let bytes = pdf_with("", "<< /Type /Annot /Subtype /Link /Rect [0 0 1 1] >>");
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let id = page
        .id
        .expect("a page reached through the tree is an object");
    let mut view = pdf_model::view::ViewState::of(&document);
    let annotation = view
        .add_free_text(&document, id, [20.0, 40.0, 180.0, 70.0], "abc", [0.0; 3])
        .expect("a rectangle with area is something to write in");

    // Inside the rectangle, which is what names the annotation — the same rule `Query::FieldAt`
    // and every question beside it follow.
    let at = (100.0, 55.0);
    let start = view
        .caret_at(&document, &page, at.0, at.1, 0)
        .expect("the annotation under the point lays text out");
    let end = view
        .caret_at(&document, &page, at.0, at.1, 3)
        .expect("and at the end of its text");
    assert!(
        (start[0] - start[2]).abs() < 0.001,
        "a caret is a vertical segment: {start:?}"
    );
    assert!(
        (start[0] - 20.0).abs() < 0.5,
        "Table 177 states no border inset for the text, so a left-quadded line starts at /Rect's \
         own left edge, not {}",
        start[0]
    );
    assert!(
        end[0] > start[0],
        "three characters later the caret has moved right: {start:?} {end:?}"
    );

    // The inverse, which is what a click needs: an offset handed back as a point comes back as
    // the offset (ADR 0225's round trip, on the other subtype).
    for offset in 0..=3 {
        let caret = view
            .caret_at(&document, &page, at.0, at.1, offset)
            .expect("a caret at every boundary of the value");
        let middle = ((caret[0] + caret[2]) * 0.5, (caret[1] + caret[3]) * 0.5);
        assert_eq!(
            view.offset_at(&document, &page, at, middle),
            Some(offset),
            "the point the caret stands at names the byte it stands before"
        );
    }

    // And a selection over the whole of it is one line's worth of shape.
    let shapes = view
        .field_selection(&document, &page, at, (0, 3))
        .expect("a range of the annotation's own text");
    assert_eq!(shapes.len(), 1, "one line, one shape: {shapes:?}");

    // Then what a person typed reaches the page, and it is the text they typed rather than the
    // empty box they drew.
    assert!(view.set_free_text(&document, annotation, "abcdef"));
    let raster = draw_with(&document, &page, &view);
    let (first, last) = ink_span(&raster);
    assert!(
        first >= 20 && last <= 180,
        "the text escaped the annotation: {first}..{last}"
    );
    let wider = last;
    assert!(view.set_free_text(&document, annotation, "a"));
    let (_, last) = ink_span(&draw_with(&document, &page, &view));
    assert!(
        last < wider,
        "one character inks less far than six: {last} against {wider}"
    );
}

/// The **file's own** free text annotation, retyped, redrawn and written back (§12.5.6.6, §7.5.6).
///
/// Three claims in one fixture, because they are one mechanism seen at three places:
///
/// - **The stored appearance is set aside.** Table 177 makes the file's own `/AP` decisive over
///   its `/DA` — "[t]he annotation dictionary's AP entry, if present, shall take precedence over
///   the DA entry" — and that is a precedence between two things the *file* says about one text.
///   Once a person has changed the text, the stream describes an annotation that no longer exists,
///   and §12.5.6.6 states where the appearance comes from instead: "12.7.4.3, 'Variable text',
///   describes the process of using these entries to generate the appearance of the text in these
///   annotations". The fixture's stored stream fills the whole rectangle, so a page still drawing
///   it is unmistakable.
/// - **The saved file says what the viewer showed**, which is the assertion that makes the other
///   two worth having: the update is read back through a second `Document` and drawn, and its ink
///   is compared with the ink of the state that was saved.
/// - **Replaced rather than spliced.** §12.7.4.3's closing paragraph appends where a stream holds
///   no `/Tx` marked content, which for this subtype would draw the new text on top of the old —
///   56 of the corpus's 67 free text appearance streams have no such region
///   (`examples/free_text_census`). The fixture's stream has none either, so an append would leave
///   the block behind.
#[test]
fn a_free_text_annotation_the_file_states_is_retyped_redrawn_and_written_back() {
    let bytes = pdf_with_appearance(
        "",
        "<< /Type /Annot /Subtype /FreeText /Rect [20 40 180 70] /F 4 /Contents (the producer's) \
         /DA (/Helv 12 Tf 0 g) /Border [0 0 0] /AP << /N 6 0 R >> >>",
        // The producer's own appearance, and deliberately nothing like text: a solid block over
        // the whole rectangle, with no `/Tx` marked-content region to splice into.
        "0 0 0 rg 0 0 160 30 re f",
    );
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let annotation = pdf_syntax::ObjectId::new(5, 0);

    let stored = draw_with(&document, &page, &pdf_model::view::ViewState::of(&document));
    let filled = inked_columns(&stored).len();
    assert!(
        filled >= 150,
        "the file's own appearance fills its rectangle: {filled} columns"
    );

    let mut view = pdf_model::view::ViewState::of(&document);
    assert_eq!(
        view.free_text_at(&document, &page, 100.0, 55.0),
        Some((annotation, "the producer's".to_owned())),
        "a point inside the rectangle names the file's own annotation and what it says"
    );
    assert!(
        view.set_free_text(&document, annotation, "i"),
        "an annotation the file states takes the text"
    );
    assert!(
        !view.set_free_text(&document, pdf_syntax::ObjectId::new(3, 0), "i"),
        "the page is not a free text annotation"
    );

    let retyped = draw_with(&document, &page, &view);
    let narrow = inked_columns(&retyped).len();
    assert!(
        narrow < 20,
        "one character replaces the block rather than joining it: {narrow} columns"
    );

    // §7.5.6, and the assertion the other two exist for: what the next reader sees.
    let written = view.save(&document).expect("the fixture can be written");
    assert!(written.unappeared.is_empty(), "{:?}", written.unappeared);
    let reopened = Document::open(written.bytes).expect("what was written is a PDF");
    let reopened_page = pdf_model::Pages::new(&reopened).get(0).expect("page one");
    let dict = reopened
        .get(annotation)
        .as_dict()
        .cloned()
        .expect("the annotation was replaced rather than removed");
    assert_eq!(
        contents_of(&reopened, &dict).as_deref(),
        Some("i"),
        "Table 166's /Contents is what the person typed"
    );
    let saved = draw_with(
        &reopened,
        &reopened_page,
        &pdf_model::view::ViewState::of(&reopened),
    );
    assert_eq!(
        inked_columns(&saved),
        inked_columns(&retyped),
        "the saved file draws the annotation the viewer was drawing"
    );
}

/// The same edit on a **producer's** annotation rather than on a fixture (§12.5.6.6, §7.5.6).
///
/// Trap 4, one clause over: the fixture above is written by the test that reads it, and
/// `tracemonkey_freetext.pdf` is written by Firefox — its annotation states `/Rect [446.4 509.1
/// 504.3 529]`, `/DA (/Helv 10 Tf 0 g)`, `/Contents (Hello World)` and an `/AP` whose stream this
/// program did not compose. What is asserted is what a next reader sees: the annotation is found by
/// a point on its own page, retyped, written, reopened, and read back.
#[test]
fn a_corpus_documents_own_free_text_annotation_is_retyped_and_written_back() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs/tracemonkey_freetext.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let document = Document::open(bytes).expect("tracemonkey_freetext.pdf opens");
    let pages = pdf_model::Pages::new(&document);
    let mut view = pdf_model::view::ViewState::of(&document);

    // The point is taken from the *document* rather than from the code under test, which is trap
    // 12a's rule: the annotation's own rectangle names its middle.
    let (index, annotation, before) = (0..pages.len())
        .find_map(|index| {
            let page = pages.get(index)?;
            let annots = document.get_key(&page.dict, "Annots");
            let id = annots
                .as_array()?
                .iter()
                .filter_map(pdf_syntax::Object::as_reference)
                .find(|id| {
                    document
                        .get(*id)
                        .as_dict()
                        .map(|dict| document.get_key(dict, "Subtype"))
                        .and_then(|subtype| {
                            subtype.as_name().map(|name| name.as_bytes() == b"FreeText")
                        })
                        .unwrap_or(false)
                })?;
            let dict = document.get(id).as_dict().cloned()?;
            let rect = document.get_key(&dict, "Rect");
            let mut corners = rect
                .as_array()?
                .iter()
                .filter_map(|value| document.resolve(value).as_number());
            let (x0, y0, x1, y1) = (
                corners.next()?,
                corners.next()?,
                corners.next()?,
                corners.next()?,
            );
            // A page coordinate, so the narrowing is exact for anything a `/MediaBox` admits.
            let middle = (f64_as_f32((x0 + x1) * 0.5), f64_as_f32((y0 + y1) * 0.5));
            let (found, text) = view.free_text_at(&document, &page, middle.0, middle.1)?;
            (found == id).then_some((index, id, text))
        })
        .expect("the document states a free text annotation with a rectangle");
    assert_eq!(before, "Hello World", "the producer's own text");

    assert!(view.set_free_text(&document, annotation, "reread"));
    let written = view.save(&document).expect("the document can be written");
    assert!(written.unappeared.is_empty(), "{:?}", written.unappeared);

    let reopened = Document::open(written.bytes).expect("what was written is a PDF");
    let page = pdf_model::Pages::new(&reopened)
        .get(index)
        .expect("the page the annotation is on");
    let dict = reopened
        .get(annotation)
        .as_dict()
        .cloned()
        .expect("the annotation survives");
    assert_eq!(contents_of(&reopened, &dict).as_deref(), Some("reread"));
    // And a reader that never heard of this edit draws the new text, because the appearance was
    // replaced with it: the file's own `/AP` is what Table 177 makes decisive.
    let interpretation = pdf_model::interpret(&reopened, &page);
    assert!(
        interpretation.text.contains("reread"),
        "the page reads back the new note: {:?}",
        interpretation.text
    );
}

/// Table 167's bit 8 and bit 10 are not the same flag, and the table says so (§12.5.3).
///
/// **A pair of fixtures differing only in that rule**, which is trap 8's shape and is the only way
/// to see this one: the corpus's 73 free text annotations carry bit 10 once and bit 8 three times,
/// never on the same annotation, and no document at all pairs them (`examples/free_text_census`).
///
/// §12.5.3's Table 167, bit 8:
///
/// > If set, do not allow the annotation to be deleted or its properties (including position and
/// > size) to be modified by the user.
///
/// and bit 10:
///
/// > If set, do not allow the contents of the annotation to be modified by the user.
///
/// Bit 8's row goes on to say — in prose here rather than as a quotation, because `doc/md/`'s
/// conversion splits *changes* into "chang es" and the sentence cannot be checked against it —
/// that the flag does not restrict changes to the annotation's contents, naming a form field's
/// value as the example. The PDF has the word whole; `pdftotext -layout` is where that was
/// settled, which is the handover's rule for a gate that accuses the standard of a gap.
///
/// So one of the two restricts this operation and the other explicitly does not — and what either
/// produces is a **reason** rather than a refusal, because `CLAUDE.md` makes obeying a document's
/// restrictions the reader's own policy. `pdf_model` is asked, `viewer_core` decides.
#[test]
fn table_167s_locked_flag_is_not_its_locked_contents_flag() {
    use pdf_model::restriction::{Operation, Restriction, asserted};

    let annotation = pdf_syntax::ObjectId::new(5, 0);
    // 4 is Table 167 bit 3, Print, which both carry so that the two files differ in one bit only.
    for (flags, expected) in [
        (4 | (1 << 9), vec![Restriction::AnnotationLocked]),
        (4 | (1 << 7), Vec::new()),
    ] {
        let document = Document::open(pdf_with(
            "",
            &format!(
                "<< /Type /Annot /Subtype /FreeText /Rect [20 40 180 70] /F {flags} \
                 /Contents (locked?) /DA (/Helv 12 Tf 0 g) /Border [0 0 0] >>"
            ),
        ))
        .expect("the fixture is a valid PDF");
        assert_eq!(
            asserted(&document, Operation::Annotate, None, Some(annotation)),
            expected,
            "/F {flags}"
        );
        // And the model itself does neither: a refusal at the point of the operation could never
        // become `CLAUDE.md`'s *ask*, which is what `crate::restriction` exists to keep possible.
        let mut view = pdf_model::view::ViewState::of(&document);
        assert!(
            view.set_free_text(&document, annotation, "typed"),
            "/F {flags}: the flag is read where a host can apply a policy to it, not here"
        );
    }
}

/// A note whose text a person removed draws nothing, and does not fall back to Table 177's `/RC`.
///
/// The distinction [`pdf_model::view::AnnotationView::contents`] exists for. `/RC` is the file's
/// *second* statement of the same text — §12.5.6.2 NOTE 1 makes the two "textually equivalent" —
/// so a reader that fell back to it would answer an empty note with the producer's words. That is
/// the same rule §12.7.6.3 states for a field: "its V entry shall be removed", and a cleared value
/// is not an untouched one.
#[test]
fn clearing_a_free_text_annotation_does_not_uncover_its_rich_text() {
    let document = Document::open(pdf_with(
        "",
        "<< /Type /Annot /Subtype /FreeText /Rect [20 40 180 70] /F 4 /Contents (mmmmmmmmmm) \
         /RC (wwwwwwwwww) /DA (/Helv 12 Tf 0 g) /Border [0 0 0] >>",
    ))
    .expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let annotation = pdf_syntax::ObjectId::new(5, 0);

    let mut view = pdf_model::view::ViewState::of(&document);
    assert!(
        !inked_columns(&draw_with(&document, &page, &view)).is_empty(),
        "the file's own text is drawn"
    );
    assert!(view.set_free_text(&document, annotation, ""));
    assert!(
        inked_columns(&draw_with(&document, &page, &view)).is_empty(),
        "an emptied note draws nothing at all"
    );

    // And the saved file says the same: no `/Contents`, and no appearance to contradict it.
    let written = view.save(&document).expect("the fixture can be written");
    let reopened = Document::open(written.bytes).expect("what was written is a PDF");
    let dict = reopened
        .get(annotation)
        .as_dict()
        .cloned()
        .expect("replaced");
    assert!(
        contents_of(&reopened, &dict).is_none_or(|text| text.is_empty()),
        "the note is empty in the file too"
    );
    assert!(
        reopened.get_key(&dict, "AP").as_dict().is_none(),
        "and carries no appearance stream drawing what it no longer says"
    );
}

/// A free text annotation carrying whatever Table 177 entries the caller states.
///
/// One builder for the callout tests below, so that a pair of fixtures differs in the entry
/// under test and in nothing else — trap 8's shape, and the only shape available here: **no
/// corpus document states a `/CL` at all**, on any page, which `examples/free_text_census`
/// counts. The `/Rect` is `[20 40 180 70]` and every callout below runs in the empty half of the
/// page under it, so the marks the callout makes and the marks the text makes never meet.
fn callout_annotation(entries: &str) -> Vec<u8> {
    pdf_with(
        "",
        &format!(
            "<< /Type /Annot /Subtype /FreeText /Rect [20 40 180 70] /F 4 /Contents (visible) \
             /DA (/Helv 12 Tf 0 g) /Border [0 0 0] {entries} >>"
        ),
    )
}

/// Every point below the annotation's rectangle that a mark reached, in PDF coordinates.
///
/// The text sits at y 40 and above, so this is the callout's ink and nothing else.
fn callout_ink(raster: &pdf_render::Raster) -> Vec<(u32, u32)> {
    let mut points = Vec::new();
    for y in 0..38 {
        for x in 0..raster.width {
            let row = raster.height.saturating_sub(1).saturating_sub(y);
            if opacity(raster, x, row) > 0 {
                points.push((x, y));
            }
        }
    }
    points
}

/// Table 177's `/CL` drawn: two points make a straight line and three bend at the knee.
///
/// ISO 32000-2 §12.5.6.6, Table 177:
///
/// > Six numbers [ x 1 y 1 x 2 y 2 x 3 y 3 ] represent the starting, knee point, and ending
/// > coordinates of the line in default user space, as shown in "Figure 79 - Free text
/// > annotation with callout". Four numbers [ x 1 y 1 x 2 y 2 ] represent the starting and
/// > ending coordinates of the line.
///
/// The pair differs in that sentence alone. Both start at (30, 10) and end at (60, 40); the
/// three-point one puts its knee at (30, 25), so its first leg is the vertical x = 30 while the
/// straight one is at x = 40 by the time it reaches y = 20. Each fixture is inked where the other
/// is not, which is what makes this a test of the rule rather than of "something was drawn".
///
/// **It is also the check that a callout is not clipped to `/Rect`.** The whole line lies below
/// the annotation's rectangle, and ADR 0193's rule is why it survives: this entry's coordinates
/// are "in default user space", exactly as §12.5.6.7's `/L` is, so `/Rect` is not a box the marks
/// are inside.
#[test]
fn a_callout_lines_knee_is_where_its_third_pair_of_coordinates_puts_it() {
    let (straight_reports, straight) =
        draw(callout_annotation("/IT /FreeTextCallout /CL [30 10 60 40]"));
    let (bent_reports, bent) = draw(callout_annotation(
        "/IT /FreeTextCallout /CL [30 10 30 25 60 40]",
    ));
    assert!(straight_reports.is_empty(), "{straight_reports:?}");
    assert!(bent_reports.is_empty(), "{bent_reports:?}");

    let straight = callout_ink(&straight);
    let bent = callout_ink(&bent);
    assert!(
        straight.contains(&(40, 20)) && !straight.contains(&(30, 20)),
        "the two-point line runs diagonally: {straight:?}"
    );
    assert!(
        bent.contains(&(30, 20)) && !bent.contains(&(40, 20)),
        "the three-point line goes up from its start before it turns: {bent:?}"
    );
}

/// Table 177's `/LE` puts one of Table 179's shapes at (x1, y1) and nowhere else.
///
/// ISO 32000-2 §12.5.6.6, Table 177:
///
/// > The name shall specify the line ending style for the endpoint defined by the pairs of
/// > coordinates ( x 1 , y 1 ).
///
/// So the pair below states the same `/CL` and differs in `/LE` alone: the ending adds marks
/// around the *start*, which is Figure 79's arrow tip, and adds none around the end that meets
/// the note. Table 179's shapes are §12.5.6.7's and are drawn by the same code, so what this
/// asserts is the wiring — one name rather than an array of two, and the endpoint the table
/// names.
#[test]
fn a_callouts_line_ending_decorates_the_end_the_table_names() {
    let plain = callout_ink(&draw(callout_annotation("/IT /FreeTextCallout /CL [30 10 60 40]")).1);
    let (reports, arrowed) = draw(callout_annotation(
        "/IT /FreeTextCallout /CL [30 10 60 40] /LE /ClosedArrow",
    ));
    assert!(reports.is_empty(), "{reports:?}");
    let arrowed = callout_ink(&arrowed);

    let extra: Vec<(u32, u32)> = arrowed
        .iter()
        .copied()
        .filter(|point| !plain.contains(point))
        .collect();
    assert!(!extra.is_empty(), "the ending marks the page");
    for (x, y) in extra {
        assert!(
            x < 40 && y < 20,
            "every mark the ending added is at the start (30, 10), not at the end (60, 40): \
             ({x}, {y})"
        );
    }
}

/// Table 177 makes `/CL` meaningful under one intent, and the other two draw nothing.
///
/// Table 177 gives `/IT` three values with `FreeText` the default, and says of the third that the
/// annotation "is intended to function as a click-to-type or typewriter object and **no callout
/// line is drawn**". So the same six numbers mean a line under one intent and nothing under the
/// others — and nothing is *reported* under the others either, because a report names what this
/// program owes and the table owes a mark only where it says one is meaningful (trap 11).
#[test]
fn a_callout_line_is_drawn_only_under_the_intent_that_makes_it_meaningful() {
    let line = "/CL [30 10 30 25 60 40] /LE /OpenArrow";
    let asked = draw(callout_annotation(&format!("/IT /FreeTextCallout {line}")));
    assert!(asked.1.width > 0 && !callout_ink(&asked.1).is_empty());

    for intent in ["/IT /FreeTextTypeWriter ", "/IT /FreeText ", ""] {
        let (reports, raster) = draw(callout_annotation(&format!("{intent}{line}")));
        assert!(
            callout_ink(&raster).is_empty(),
            "{intent:?} draws no callout line"
        );
        assert!(
            reports.is_empty(),
            "and owes nothing for an entry the table calls meaningless: {reports:?}"
        );
    }
}

/// `/RD` insets the text and leaves the callout where the page put it.
///
/// Table 177 states the two in different spaces on purpose: the inner rectangle "is where the
/// annotation's text should be displayed", while `/CL`'s numbers are "in default user space". The
/// pair below states the same callout and differs in `/RD` alone, so the text moves and the line
/// does not — which is the entry that makes a `/Rect` big enough for both readable at all.
#[test]
fn a_free_texts_inner_rectangle_moves_its_text_and_not_its_callout() {
    let line = "/IT /FreeTextCallout /CL [30 10 30 25 60 40]";
    let (_, flush) = draw(callout_annotation(line));
    let (_, inset) = draw(callout_annotation(&format!("{line} /RD [40 0 0 0]")));

    assert_eq!(
        callout_ink(&flush),
        callout_ink(&inset),
        "the callout is in the page's space and `/RD` says nothing about it"
    );
    let (flush, inset) = (inked_columns(&flush), inked_columns(&inset));
    let leftmost = |columns: &[u32]| columns.iter().copied().min().expect("something was drawn");
    assert!(
        leftmost(&inset) > leftmost(&flush),
        "and the text starts forty points further in: {:?} against {:?}",
        leftmost(&inset),
        leftmost(&flush)
    );
}

/// What the callout refuses, and that it refuses out loud while still drawing what it can.
///
/// Two shapes of malformed statement, each named rather than guessed at (trap 5):
///
/// - **A `/CL` of five numbers.** Table 177 states "[a]n array of four or six numbers", and which
///   of five a reader should keep is not something the table answers.
/// - **A `/LE` outside Table 179.** The table's ten names are the whole of it, and the default
///   `None` answers an *absent* entry rather than an unreadable one — so the line is still drawn
///   and the ending is what is named, which is ADR 0075's rule applied one entry over.
#[test]
fn a_callout_this_reader_cannot_read_is_named_rather_than_invented() {
    let (reports, raster) = draw(callout_annotation(
        "/IT /FreeTextCallout /CL [30 10 30 25 60]",
    ));
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(reports[0].contains("/CL"), "{reports:?}");
    assert!(
        callout_ink(&raster).is_empty(),
        "and no line is drawn from a prefix of it"
    );
    assert!(
        !inked_columns(&raster).is_empty(),
        "while the note itself still says what it says"
    );

    let (reports, raster) = draw(callout_annotation(
        "/IT /FreeTextCallout /CL [30 10 30 25 60 40] /LE /Wedge",
    ));
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(reports[0].contains("/LE"), "{reports:?}");
    let drawn = callout_ink(&raster);
    let plain = callout_ink(
        &draw(callout_annotation(
            "/IT /FreeTextCallout /CL [30 10 30 25 60 40]",
        ))
        .1,
    );
    assert_eq!(
        drawn, plain,
        "the line is drawn without the ending it named"
    );
}

/// A text field whose `/DR` font states the descriptor given, at 24 points in a 60-point box.
///
/// One helper for the four tests below because the arithmetic they check is one arithmetic and
/// its inputs must not drift: §12.5.4's default border is one point wide and drawn inside
/// `/Rect`, so the box the text is laid out in is `[21 21 179 79]` and 58 points tall. A single
/// line is centred in it, which puts the baseline at `21 + (58 + 24(A − D))/2 − 24A`, or
/// **50 − 12(A + D)** for a pair `(A, D)` in ems — so a pair that raises `A + D` moves the
/// baseline *down*, by twelve points per em.
///
/// The value has no descender on purpose: with `HI` the topmost inked row is the baseline plus a
/// cap height, so a difference between two fixtures drawn in the same face is exactly the
/// difference between their baselines, and the face this machine substitutes cancels out.
fn field_with_descriptor(descriptor: &str) -> Vec<u8> {
    pdf_with_descriptor(
        "<< /Type /Annot /Subtype /Widget /Rect [20 20 180 80] /F 4 /FT /Tx \
         /T (field) /V (HI) /DA (/Helv 24 Tf 0 g) >>",
        descriptor,
    )
}

/// The topmost row any glyph reached, in PDF y.
fn ink_top(bytes: Vec<u8>) -> u32 {
    let (_, raster) = draw(bytes);
    let rows = inked_rows(&raster);
    *rows.iter().max().expect("the value was drawn")
}

/// A pair no measurement of a face could produce is answered by the em-relative split.
///
/// ISO 32000-2 §9.8.1's Table 120 defines `/Ascent` as "[t]he maximum height above the baseline
/// reached by glyphs in this font" and `/Descent` as "[t]he maximum depth below the baseline
/// reached by glyphs in this font", both in glyph space — so `/Ascent 4000 /Descent -1140` is a
/// line five ems tall, which is a number in the wrong unit rather than a face. The old guard
/// asked only that the pair straddle the baseline and believed it, which put this field's
/// baseline at 50 − 12 × 2.86 = 15.7 points: *below its own rectangle*, where the clause's own
/// clip then took most of the value away. `pdf_font::measured_extent` refuses it and the split
/// answers, which is the same picture as a descriptor stating nothing at all (ADR 0216, 0240).
#[test]
fn a_fields_baseline_ignores_a_descriptor_that_cannot_be_a_measurement() {
    let refused = ink_top(field_with_descriptor("/Ascent 4000 /Descent -1140"));
    let silent = ink_top(field_with_descriptor(""));
    assert_eq!(
        refused, silent,
        "a pair the band refuses draws where a descriptor stating neither entry does"
    );
}

/// A pair the band believes is believed to the number, which is the control on the one above.
///
/// `/Ascent 1200 /Descent -300` is a 1.5 em line, inside the band, and `A + D` is 0.9 against the
/// split's 0.5 — so the baseline sits 12 × 0.4 = 4.8 points lower. A band that threw away true
/// statements would draw this at the same height as the fixture with no descriptor at all.
#[test]
fn a_fields_baseline_follows_a_descriptor_the_band_believes() {
    let stated = ink_top(field_with_descriptor("/Ascent 1200 /Descent -300"));
    let silent = ink_top(field_with_descriptor(""));
    let dropped = silent.saturating_sub(stated);
    assert!(
        (4..=6).contains(&dropped),
        "the stated pair moves the baseline 4.8 points down, not {dropped}"
    );
}

/// Table 120's sign convention is a convention, and a positive `/Descent` states a depth.
///
/// ISO 32000-2 §9.8.1's Table 120, on `/Descent`:
///
/// > The maximum depth below the baseline reached by glyphs in this font. The value shall be a
/// > negative number.
///
/// Two sentences: the first defines a depth, which is a magnitude, and the second is how to write
/// it down. `/Ascent 905 /Descent 211` is Arial's real metrics with the second broken, so it is
/// read as the pair the first states — and the strongest form of that claim is that it draws
/// *identically* to the same file with the sign put back. The old guard refused it and the split
/// stood in, 2.3 points away. **A choice and not a clause**, argued in ADR 0216 and shared with a
/// field's baseline here. Errata Collection 3 (Issue #190) keeps both sentences' shape — the
/// amended floor is *less than or equal to zero*, and its inserted NOTE says font programs
/// write descenders in either sign while *PDF always expects negative values* — so the file
/// this fixture models is still malformed and the repair is still a repair.
#[test]
fn a_fields_baseline_reads_a_positive_descent_as_the_depth_it_states() {
    let unsigned = ink_top(field_with_descriptor("/Ascent 905 /Descent 211"));
    let signed = ink_top(field_with_descriptor("/Ascent 905 /Descent -211"));
    assert_eq!(
        unsigned, signed,
        "a descent written without its sign is the same face as one written with it"
    );
}

/// A `/Descent` of zero is a face whose glyphs stop at the baseline, not a face that said nothing.
///
/// Since Errata Collection 3 (Issue #190) that is the entry's own floor — *less than or equal to
/// zero* — rather than this program's reading of the published "negative number".
/// The old guard asked for `descent < 0` and fell back on `/Ascent 1000 /Descent 0`, which states
/// a line of exactly one em — §9.2.2's own nominal line, "arranged so that the nominal height of
/// tightly spaced lines of text is 1 unit". `A + D` is 1.0 against the split's 0.5, so believing
/// it puts the baseline six points lower.
#[test]
fn a_fields_baseline_reads_a_zero_descent_as_a_face_with_no_descenders() {
    let stated = ink_top(field_with_descriptor("/Ascent 1000 /Descent 0"));
    let silent = ink_top(field_with_descriptor(""));
    let dropped = silent.saturating_sub(stated);
    assert!(
        (5..=7).contains(&dropped),
        "a one-em line moves the baseline six points down, not {dropped}"
    );
}

// ---------------------------------------------------------------------------------------------
// A composite `/DA` font (§9.7, §12.7.4.3)
//
// **Trap 8's territory, and measured to be so**: `examples/variable_text_census` finds no corpus
// document whose `/DA` names a Type 0 font, so every rule below is defended by these fixtures and
// by nothing else. They come in pairs differing in one entry, which is the only construction that
// says *which* rule a difference is about.
//
// The descendant embeds no font program, deliberately. §9.7.4.2's substituted route is reached
// through `/ToUnicode`, so the glyph *shapes* are this machine's — and the **advances are not**:
// `/W` and `/DW` are the document's own statement (§9.7.4.3), so every position asserted here is
// the file's arithmetic rather than the installed face's.
// ---------------------------------------------------------------------------------------------

/// A one-page form whose `/DR` defines one composite font, assembled from §9.7's own parts.
///
/// `encoding` is written verbatim as Table 119's `/Encoding` — a predefined name, or `7 0 R` for
/// the `CMap` stream this writes. An empty `to_unicode` omits the entry rather than writing an
/// empty stream, because the absence is what one of the fixtures below is about.
fn composite_form(
    annotation: &str,
    encoding: &str,
    cmap: &str,
    to_unicode: &str,
    descendant: &str,
) -> Vec<u8> {
    let (width, height) = PAGE;
    let unicode_entry = if to_unicode.is_empty() {
        ""
    } else {
        "/ToUnicode 8 0 R"
    };
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] \
         /DR << /Font << /Comp 6 0 R >> >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n{annotation}\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /Helvetica /Encoding {encoding} \
         /DescendantFonts [9 0 R] {unicode_entry} >>\nendobj\n\
         7 0 obj\n<< /Type /CMap /CMapName /Fixture-H \
         /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
         /Length {} >>\nstream\n{cmap}\nendstream\nendobj\n\
         8 0 obj\n<< /Length {} >>\nstream\n{to_unicode}\nendstream\nendobj\n\
         9 0 obj\n<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Helvetica \
         /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
         {descendant} >>\nendobj\n",
        cmap.len().saturating_add(1),
        to_unicode.len().saturating_add(1)
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

/// An embedded `CMap` file (§9.7.5.3) with one codespace range and one `cidrange`.
fn cmap_program(codespace: &str, cid_range: &str) -> String {
    format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CMapName /Fixture-H def\n/CMapType 1 def\n/WMode 0 def\n\
         1 begincodespacerange\n{codespace}\nendcodespacerange\n\
         1 begincidrange\n{cid_range}\nendcidrange\n\
         endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n"
    )
}

/// A `/ToUnicode` `CMap` (§9.10.3) mapping one run of codes to consecutive characters.
fn to_unicode_program(codespace: &str, bf_range: &str) -> String {
    format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         1 begincodespacerange\n{codespace}\nendcodespacerange\n\
         1 beginbfrange\n{bf_range}\nendbfrange\n\
         endcmap\nend\nend\n"
    )
}

/// The widget every composite fixture below hangs the same value on.
const COMPOSITE_WIDGET: &str = "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 \
                                /FT /Tx /T (field) /V (AB) /DA (/Comp 12 Tf 0 g) >>";

/// The same two glyphs, spelled in one-byte codes and in two-byte codes (§9.7.6.2).
///
/// > A sequence of one or more bytes shall be extracted from the string and matched against the
/// > codespace ranges in the CMap. That is, the first byte shall be matched against 1-byte
/// > codespace ranges; if no match is found, a second byte shall be extracted, and the 2-byte
/// > code shall be matched against 2-byte codespace ranges.
///
/// §12.7.4.3 has this processor *write* the string, so the sentence has to be obeyed in reverse:
/// the codespace decides how many bytes the code for a character occupies, and the same two
/// characters are a two-byte string in one of these files and a four-byte string in the other.
///
/// **The pair differs in one thing** — the bounds of the `begincodespacerange`, carried through
/// the `cidrange` and the `bfrange`, which state codes of that same length — while the CIDs, the
/// widths and the value are identical. So the picture must be identical too, and it is the
/// picture that is asserted rather than the bytes: a reader that wrote one byte where the
/// codespace asks for two produces a string that decodes to entirely different codes.
#[test]
fn a_composite_da_fonts_codes_are_as_long_as_its_codespace_says() {
    let (narrow_reports, narrow) = draw(composite_form(
        COMPOSITE_WIDGET,
        "7 0 R",
        &cmap_program("<00> <FF>", "<41> <5A> 1"),
        &to_unicode_program("<00> <FF>", "<41> <5A> <0041>"),
        "/DW 1000 /W [1 [500 500]]",
    ));
    let (wide_reports, wide) = draw(composite_form(
        COMPOSITE_WIDGET,
        "7 0 R",
        &cmap_program("<0000> <FFFF>", "<0041> <005A> 1"),
        &to_unicode_program("<0000> <FFFF>", "<0041> <005A> <0041>"),
        "/DW 1000 /W [1 [500 500]]",
    ));

    assert!(narrow_reports.is_empty(), "{narrow_reports:?}");
    assert!(wide_reports.is_empty(), "{wide_reports:?}");
    let (start, end) = ink_span(&narrow);
    assert_eq!(
        (start, end),
        ink_span(&wide),
        "one value, two codespaces, two different places"
    );
    // **The span says whose advances placed the glyphs.** The pen starts at the box's left edge
    // and moves 500/1000 of an em at 12 points, which is the document's `/W`; the right edge is
    // the second glyph's own outline, which is this machine's face. A reader that took the
    // advances from the face instead — Helvetica's `A` is 722 — would put the pair five points
    // further along, which is outside this range and inside a generous one.
    assert!(
        (20..=22).contains(&start) && (29..=35).contains(&end),
        "the /W widths did not place the two glyphs: {start}..{end}"
    );
}

/// A composite font whose codes stand for nothing reports rather than drawing (§9.10.2).
///
/// The same file as the one-byte fixture above with the `/ToUnicode` entry removed, which is the
/// only difference. A CID is an index into the descendant's own glyphs, so with no program
/// embedded and no registered collection named there is nothing a character can be turned into
/// — §9.7.4.2's "CIDs shall not participate in glyph selection" from one side and §9.10.2 from
/// the other. Trap 5: the field draws nothing and the page says why.
#[test]
fn a_composite_da_font_that_names_no_characters_is_reported() {
    let (reports, raster) = draw(composite_form(
        COMPOSITE_WIDGET,
        "7 0 R",
        &cmap_program("<00> <FF>", "<41> <5A> 1"),
        "",
        "/DW 1000 /W [1 [500 500]]",
    ));
    assert!(
        reports.iter().any(|report| report.contains("/Comp")),
        "the font that could not be used is not named: {reports:?}"
    );
    assert!(
        inked_columns(&raster).is_empty(),
        "nothing can be drawn for a value no code spells"
    );
}

/// A `/DA` naming a vertical composite font is refused by name (§9.7.5.1).
///
/// > A CMap shall specify the writing mode … for any CIDFont with which the CMap is combined.
/// > The writing mode determines which metrics shall be used when glyphs are painted from that
/// > font.
///
/// §12.7.4.3's layout here places glyphs along one axis and measures them with §9.4.4's `w0`;
/// a `CMap` in writing mode 1 says that is the wrong displacement and that each glyph sits at
/// `-v` from the position. Drawing it horizontally would be a confident wrong mark rather than
/// a partial one, so the whole appearance is refused — which also leaves a document's own
/// stored stream standing where it has one.
///
/// The pair differs in one byte of one name: Table 116's two Identity `CMap`s differ only in
/// their writing mode.
#[test]
fn a_vertical_composite_da_font_is_refused_and_says_which() {
    let identity_unicode = to_unicode_program("<0000> <FFFF>", "<0041> <005A> <0041>");
    let (horizontal_reports, horizontal) = draw(composite_form(
        COMPOSITE_WIDGET,
        "/Identity-H",
        "",
        &identity_unicode,
        "/DW 1000 /W [65 [500 500]]",
    ));
    let (vertical_reports, vertical) = draw(composite_form(
        COMPOSITE_WIDGET,
        "/Identity-V",
        "",
        &identity_unicode,
        "/DW 1000 /W [65 [500 500]]",
    ));

    assert!(horizontal_reports.is_empty(), "{horizontal_reports:?}");
    assert!(
        !inked_columns(&horizontal).is_empty(),
        "the horizontal half of the pair has to draw, or the pair proves nothing"
    );
    assert!(
        vertical_reports
            .iter()
            .any(|report| report.contains("writing mode 1") && report.contains("/Comp")),
        "a vertical /DA font must be refused by name: {vertical_reports:?}"
    );
    assert!(
        inked_columns(&vertical).is_empty(),
        "a refusal draws nothing rather than drawing along the wrong axis"
    );
}

/// A free text annotation stating no text at all, so that every mark on the page is its border.
///
/// `/Rect` is the callout fixtures' `[20 40 180 70]` and `/Contents` is absent, which
/// [`pdf_model::appearance`] lays out as nothing — leaving §12.5.4's border as the only thing the
/// construction can draw. Trap 8's shape again: `examples/free_text_census` counts **six** corpus
/// free text annotations with no appearance stream, every one of them stating `/Border` and four
/// of them stating a width of zero, so no corpus page can distinguish these cases.
fn bordered_annotation(entries: &str) -> Vec<u8> {
    pdf_with(
        "",
        &format!(
            "<< /Type /Annot /Subtype /FreeText /Rect [20 40 180 70] /F 4 \
             /DA (/Helv 12 Tf 0 g) {entries} >>"
        ),
    )
}

/// How many rows the border crosses at the middle of the rectangle: its top edge plus its bottom.
///
/// A rectangular outline at x = 100 is two runs of the border's own width, so this is twice the
/// width in pixels and grows with it — which is what makes it a test of §12.5.4's width rather
/// than of "something was drawn".
fn edges_at_middle(raster: &pdf_render::Raster) -> usize {
    (0..raster.height)
        .filter(|row| opacity(raster, 100, *row) > 0)
        .count()
}

/// §12.5.4's border is drawn round a free text annotation, at the width its entries state.
///
/// ISO 32000-2 §12.5.4:
///
/// > If neither the Border nor the BS entry is present, the border shall be drawn as a solid line
/// > with a width of 1 point.
///
/// and Table 166, on the entry that overrides it: "if the border width is 0, no border is drawn",
/// with "[i]f an annotation dictionary includes the BS entry, then the Border entry is ignored".
/// Four fixtures differing in those entries alone:
///
/// - **neither** — the sentence above fires, and one point of border appears;
/// - **`/Border [0 0 0]`** — a producer saying there is no border, and there is none;
/// - **`/Border [0 0 6]`** — six times the default, and six times the pixels;
/// - **`/Border [0 0 0] /BS << /W 6 >>`** — the two disagreeing, where Table 166 says `/BS` wins.
///
/// The colour is this program's choice and is not what these assert; see `free_text_border`'s
/// doc comment for what black is taken from and why the standard states nothing here.
#[test]
fn a_free_texts_border_is_drawn_at_the_width_its_entries_state() {
    let (default_reports, defaulted) = draw(bordered_annotation(""));
    let (none_reports, none) = draw(bordered_annotation("/Border [0 0 0]"));
    let (wide_reports, wide) = draw(bordered_annotation("/Border [0 0 6]"));
    let (style_reports, styled) = draw(bordered_annotation("/Border [0 0 0] /BS << /W 6 >>"));
    for reports in [
        &default_reports,
        &none_reports,
        &wide_reports,
        &style_reports,
    ] {
        assert!(reports.is_empty(), "{reports:?}");
    }

    assert!(
        inked_rows(&none).is_empty(),
        "a stated width of zero draws no border at all"
    );
    let thin = edges_at_middle(&defaulted);
    assert!(
        (2..=4).contains(&thin),
        "§12.5.4's default is one point on each of two edges: {thin}"
    );
    let thick = edges_at_middle(&wide);
    assert!(
        thick >= 10 && thick > thin,
        "six points of border cover six times the rows: {thick} against {thin}"
    );
    assert_eq!(
        edges_at_middle(&styled),
        thick,
        "Table 166: a stated /BS is what a /Border beside it is ignored in favour of"
    );
}

/// Table 169's cloudy border is named rather than drawn as the straight one it is not.
///
/// §12.5.4 gives this subtype the entry — "Beginning with PDF 1.6, free text annotations may also
/// have a BE entry" — and Table 169 says the border "should be drawn as a series of convex curved
/// line segments in a manner that simulates the appearance of a cloud". A rectangle in its place
/// is a shape the file did not describe, which is ADR 0106's substitutive case and the same
/// refusal `square_or_circle` takes.
///
/// **The note itself is still drawn**, which is the other half of that rule: the border is one
/// mark of two and the text is what the subtype *is*.
#[test]
fn a_free_texts_cloudy_border_is_named_rather_than_drawn_straight() {
    let (reports, cloudy) = draw(bordered_annotation("/Border [0 0 6] /BE << /S /C >>"));
    assert!(
        reports.iter().any(|report| report.contains("cloudy")),
        "{reports:?}"
    );
    assert!(
        inked_rows(&cloudy).is_empty(),
        "and nothing is drawn in its place"
    );

    let (told, drawn) = draw(pdf_with(
        "",
        "<< /Type /Annot /Subtype /FreeText /Rect [20 40 180 70] /F 4 /Contents (visible) \
         /DA (/Helv 12 Tf 0 g) /Border [0 0 6] /BE << /S /C >> >>",
    ));
    assert!(
        told.iter().any(|report| report.contains("cloudy")),
        "{told:?}"
    );
    assert!(
        !inked_rows(&drawn).is_empty(),
        "a border this reader cannot draw is not a reason to withhold the note"
    );
}

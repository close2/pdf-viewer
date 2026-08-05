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
         /Encoding /WinAnsiEncoding >>\nendobj\n",
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
        .with_background(pdf_render::Color::TRANSPARENT)
        .rasterize(&list, target)
        .expect("supported");
    (reports, raster)
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

/// A stand-in that cannot draw the value declines instead of drawing part of it.
///
/// The asymmetry is the finding, and `freetext_no_appearance.pdf` is where it came from: its
/// value is a paragraph of Arabic, and a Latin stand-in draws its spaces and full stops and
/// nothing else — a scatter of dots on an otherwise empty page, which is trap 1's archetype and
/// worse than the blank a refusal leaves. Where the *document* names the font, a code it lacks
/// is reported and the rest is drawn, because there the shortfall is the document's own choice.
#[test]
fn a_stand_in_that_cannot_draw_the_value_declines() {
    let (reports, raster) = draw(pdf_with(
        "",
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
         /T (field) /V <FEFF0627064406220646> /DA (/Nope 12 Tf 0 g) >>",
    ));
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(reports[0].contains("/Nope"), "{reports:?}");
    assert!(
        inked_columns(&raster).is_empty(),
        "a value a stand-in cannot show may not be shown in part"
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

/// ISO 32000-2 §12.7.5.4: a combo box draws its value and a list box is refused.
///
/// > A choice field shall have a field type of Ch that contains several text items, one or more
/// > of which shall be selected as the field value. The items may be presented to the user in
/// > one of the following two forms:
///
/// Half the clause draws and half does not, and the two halves are one bit apart — which is
/// exactly why they are one test. A combo box shows its value in an edit box, so it is a text
/// field with a different `/FT`; a list box shows a *selection* out of `/Opt`, and the clause
/// states no highlight colour, no rule and no metric for one. Drawing a list box as unmarked
/// text would put every option on the page with nothing saying which is chosen, which is worse
/// than refusing and is the plausible-looking wrong page trap 1 is about.
///
/// The refusal is checked by name rather than by count: a report that fired for some other
/// reason would satisfy "something was reported".
#[test]
fn a_combo_box_draws_its_value_and_a_list_box_says_it_cannot() {
    let choice = |flags: &str| {
        pdf_with(
            "",
            &format!(
                "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Ch \
                 /T (choice) /V (Beta) /Opt [(Alpha) (Beta) (Gamma)] /DA (/Helv 12 Tf 0 g) \
                 {flags} >>"
            ),
        )
    };

    // Bit 18 is the combo flag, which is 1 << 17.
    let (reports, raster) = draw(choice("/Ff 131072"));
    assert!(
        !inked_columns(&raster).is_empty(),
        "a combo box draws its value like a text field: {reports:?}"
    );
    assert!(
        reports.is_empty(),
        "and reports nothing while doing it: {reports:?}"
    );

    let (reports, raster) = draw(choice(""));
    assert!(
        inked_columns(&raster).is_empty(),
        "a list box draws nothing, because the clause states no appearance for a selection"
    );
    let named = reports.iter().any(|report| report.contains("list box"));
    assert!(named, "and says so by name: {reports:?}");
}

/// What a person typing into one field reaches the page as.
///
/// The readback rather than the ink, because what is being checked here is *which characters*
/// were laid out and no measurement of pixels can say that.
fn typed(bytes: Vec<u8>, value: &str) -> (usize, String) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let mut view = pdf_model::view::ViewState::of(&document);
    let applied = view.set_field(&document, "field", Some(value));
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

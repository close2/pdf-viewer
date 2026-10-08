//! Rich text, laid out: Table 228's `/RV` and `/DS`, Table 177's `/RC` and `/DS`, read against
//! XFA 3.3 section 27, the Rich Text Reference.
//!
//! Each fixture holds one element or one property chapter 27 names, and each says which of the
//! chapter's examples states the rule it holds. The markup is written for these tests, in the
//! XHTML and CSS grammar the chapter uses, with words of its own: the chapter's examples are cited
//! by number and page and not copied (`doc/third-party-data.md`, ADR 0187).
//!
//! The assertions are about where ink lands and what colour it is, never about a glyph's shape,
//! and every fixture names one of §9.6.2.2's fourteen families — Helvetica, Times, Courier — so
//! the faces are the ones this binary carries and the pictures are the same on every machine.
//! ADRs 1634 and 1635.

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

/// Pixel budget, far above the pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// Table 231 bit 26, `RichText`, and bit 13, `Multiline`: `1 << 25 | 1 << 12`.
const RICH_MULTILINE: u32 = 33_558_528;

/// Table 231 bit 26 alone.
const RICH: u32 = 33_554_432;

/// A one-page PDF with an interactive form whose `/DR` holds Helvetica and Helvetica-Bold, and one
/// annotation written verbatim.
fn pdf(annotation: &str, form: &str) -> Vec<u8> {
    pdf_with(annotation, form, "")
}

/// The same, with object definitions numbered from 8 written after the seven every fixture has.
fn pdf_with(annotation: &str, form: &str, extra: &str) -> Vec<u8> {
    pdf_with_fonts(annotation, form, extra, "")
}

/// The same, with more `/DR` fonts named beside the two.
fn pdf_with_fonts(annotation: &str, form: &str, extra: &str, fonts: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm \
         << /Fields [5 0 R] /DR << /Font << /Helv 6 0 R /HeBo 7 0 R {fonts} >> >> {form} >> >>\n\
         endobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n{annotation}\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding /WinAnsiEncoding >>\nendobj\n\
         7 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold \
         /Encoding /WinAnsiEncoding >>\nendobj\n{extra}"
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

/// A rich text field across most of the page, with no appearance stream, so §12.7.4.3 builds one.
///
/// `value` is `/V`'s characters and `markup` the `/RV` beside them; `extra` is written into the
/// field as it stands.
fn field(flags: u32, value: &str, markup: &str, extra: &str) -> Vec<u8> {
    pdf(
        &format!(
            "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 190] /F 4 /FT /Tx /Ff {flags} \
             /T (f) /V ({value}) /RV ({markup}) /DA (/Helv 12 Tf 0 g) {extra} >>"
        ),
        "",
    )
}

/// The same field with a paragraph of markup, multiline.
fn rich(value: &str, markup: &str) -> Vec<u8> {
    field(
        RICH_MULTILINE,
        value,
        &format!("<body><p>{markup}</p></body>"),
        "",
    )
}

/// Interprets a fixture's page, returning what it reported and the raster.
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

/// The constructed appearance stream's content and its font resources' base font names.
fn appearance(bytes: Vec<u8>) -> (String, Vec<String>) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let annotation = document
        .get(pdf_syntax::ObjectId::new(5, 0))
        .as_dict()
        .cloned()
        .expect("the annotation");
    let written =
        pdf_model::appearance::for_annotation(&document, &annotation).expect("an appearance");
    let stream = written
        .stream
        .as_stream()
        .cloned()
        .expect("a form XObject is a stream");
    let content = String::from_utf8_lossy(&stream.data).into_owned();
    let mut faces = Vec::new();
    if let Some(fonts) = stream
        .dict
        .get("Resources")
        .and_then(pdf_syntax::Object::as_dict)
        .and_then(|resources| resources.get("Font"))
        .and_then(pdf_syntax::Object::as_dict)
    {
        for (_, font) in fonts.iter() {
            if let Some(name) = document.resolve(font).as_dict().and_then(|font| {
                font.get("BaseFont")
                    .and_then(pdf_syntax::Object::as_name)
                    .cloned()
            }) {
                faces.push(String::from_utf8_lossy(name.as_bytes()).into_owned());
            }
        }
    }
    (content, faces)
}

/// A pixel's channels, addressed in raster rows.
fn pixel(raster: &pdf_render::Raster, x: u32, row: u32) -> [u8; 4] {
    let index = (row.saturating_mul(raster.width).saturating_add(x) as usize).saturating_mul(4);
    let get = |at: usize| {
        raster
            .data
            .get(index.saturating_add(at))
            .copied()
            .unwrap_or(0)
    };
    [get(0), get(1), get(2), get(3)]
}

/// Every inked pixel, as `(x, y)` with y in PDF's upward sense.
fn inked(raster: &pdf_render::Raster) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for row in 0..raster.height {
        for x in 0..raster.width {
            if pixel(raster, x, row)[3] > 64 {
                out.push((x, raster.height.saturating_sub(1).saturating_sub(row)));
            }
        }
    }
    out
}

/// The leftmost and rightmost inked column.
fn columns(raster: &pdf_render::Raster) -> (u32, u32) {
    let ink = inked(raster);
    assert!(!ink.is_empty(), "nothing was drawn");
    (
        ink.iter().map(|(x, _)| *x).min().unwrap_or_default(),
        ink.iter().map(|(x, _)| *x).max().unwrap_or_default(),
    )
}

/// The lowest and highest inked row, in PDF y.
fn rows(raster: &pdf_render::Raster) -> (u32, u32) {
    let ink = inked(raster);
    assert!(!ink.is_empty(), "nothing was drawn");
    (
        ink.iter().map(|(_, y)| *y).min().unwrap_or_default(),
        ink.iter().map(|(_, y)| *y).max().unwrap_or_default(),
    )
}

/// The distinct bands of inked rows, top first, each as `(top, bottom)` in PDF y.
fn bands(raster: &pdf_render::Raster) -> Vec<(u32, u32)> {
    let mut lines: Vec<u32> = inked(raster).iter().map(|(_, y)| *y).collect();
    lines.sort_unstable();
    lines.dedup();
    let mut out: Vec<(u32, u32)> = Vec::new();
    for y in lines.into_iter().rev() {
        match out.last_mut() {
            Some((_, bottom)) if *bottom == y.saturating_add(1) => *bottom = y,
            _ => out.push((y, y)),
        }
    }
    out
}

/// The leftmost inked column within one band of rows.
fn first_column_in(raster: &pdf_render::Raster, band: (u32, u32)) -> u32 {
    inked(raster)
        .iter()
        .filter(|(_, y)| *y <= band.0 && *y >= band.1)
        .map(|(x, _)| *x)
        .min()
        .unwrap_or_default()
}

/// How many inked pixels lean to one channel.
fn tinted(raster: &pdf_render::Raster, channel: usize) -> usize {
    (0..raster.height)
        .flat_map(|row| (0..raster.width).map(move |x| (x, row)))
        .filter(|(x, row)| {
            let p = pixel(raster, *x, *row);
            p[3] > 64
                && p.get(channel).copied().unwrap_or(0) > 150
                && (0..3)
                    .filter(|other| *other != channel)
                    .all(|other| p.get(other).copied().unwrap_or(0) < 100)
        })
        .count()
}

/// A rich text string restating the field's own style lands where the plain value does.
///
/// The choice [`pdf_model`]'s rich layout makes for every position the two texts hand a
/// processor is the plain layout's (ADR 1634 section 5), so a string with no formatting of its
/// own is the control every other fixture here is measured against.
#[test]
fn a_string_with_no_style_of_its_own_lands_where_the_plain_value_does() {
    let (reports, styled) = draw(rich("Plain words", "Plain words"));
    assert!(reports.is_empty(), "{reports:?}");
    let (_, plain) = draw(field(4096, "Plain words", "", ""));
    assert_eq!(columns(&styled), columns(&plain));
    assert_eq!(rows(&styled), rows(&plain));
}

/// `b` and `font-weight:bold` set a run in the family's bold face — chapter 27's *Bold*, Example
/// 27.18 (page 1199) — and the bold face `/DR` holds is the one taken.
#[test]
fn bold_sets_a_run_in_the_bold_face_the_document_holds() {
    let (content, _) = appearance(rich("ab", "a<b>b</b>"));
    assert!(
        content.contains("/HeBo"),
        "the bold run names /DR's own bold face: {content}"
    );
    let (content, _) = appearance(rich("ab", "a<span style=\"font-weight:bold\">b</span>"));
    assert!(content.contains("/HeBo"), "{content}");
    let (_, bold) = draw(rich("Wide", "<b>Wide</b>"));
    let (_, regular) = draw(rich("Wide", "Wide"));
    let width = |raster: &pdf_render::Raster| {
        let (from, to) = columns(raster);
        to.saturating_sub(from)
    };
    assert!(
        width(&bold) > width(&regular),
        "Helvetica-Bold's advances are wider"
    );
}

/// `i` and `font-style:italic` set a run in the family's oblique face — *Italic*, Example 27.22
/// (page 1203). `/DR` holds none, so §9.6.2.2's own is taken.
#[test]
fn italic_sets_a_run_in_the_familys_oblique_face() {
    for markup in ["a<i>b</i>", "a<span style=\"font-style:italic\">b</span>"] {
        let (_, faces) = appearance(rich("ab", markup));
        assert!(
            faces.iter().any(|face| face == "Helvetica-Oblique"),
            "{markup}: {faces:?}"
        );
    }
}

/// `font-family` names a face by family, and a list is a search path — *Font*, Example 27.20
/// (page 1202): a family no face is found for gives way to the next.
#[test]
fn a_family_list_is_a_search_path_through_the_faces() {
    let (_, faces) = appearance(rich("ab", "a<span style=\"font-family:Courier\">b</span>"));
    assert!(faces.iter().any(|face| face == "Courier"), "{faces:?}");
    let (_, faces) = appearance(rich(
        "ab",
        "a<span style=\"font-family:'Times', Courier\">b</span>",
    ));
    assert!(faces.iter().any(|face| face == "Times-Roman"), "{faces:?}");
}

/// `font-size` sets a run's size — *Font*, Example 27.20's last sentence (page 1202).
#[test]
fn a_larger_size_inks_taller() {
    let (_, large) = draw(rich("X", "<span style=\"font-size:36pt\">X</span>"));
    let (_, small) = draw(rich("X", "X"));
    let height = |raster: &pdf_render::Raster| {
        let (low, high) = rows(raster);
        high.saturating_sub(low)
    };
    assert!(
        height(&large) > height(&small) * 2,
        "36 points is three times 12"
    );
}

/// `color`, in both of chapter 27's forms, on a paragraph and on a span inside it — *Color*,
/// Example 27.19 (page 1200).
#[test]
fn colour_reaches_the_paragraph_and_a_span_inside_it() {
    let (reports, raster) = draw(field(
        RICH_MULTILINE,
        "Blue and green",
        "<body><p style=\"color:#0000ff\">Blue and <span style=\"color:rgb(0,255,0)\">green</span></p></body>",
        "",
    ));
    assert!(reports.is_empty(), "{reports:?}");
    assert!(tinted(&raster, 2) > 10, "the paragraph is blue");
    assert!(tinted(&raster, 1) > 10, "the span is green");
    assert_eq!(tinted(&raster, 0), 0, "and nothing is red");
}

/// Table 228's `/DS` styles text nothing else styles, beneath the markup: a field with no `/RV`
/// at all is drawn in it.
#[test]
fn the_default_style_string_styles_a_plain_value() {
    let bytes = pdf(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 190] /F 4 /FT /Tx /Ff 33558528 \
         /T (f) /V (Red) /DS (font: 30pt Helvetica; color:#ff0000) /DA (/Helv 12 Tf 0 g) >>",
        "",
    );
    let (reports, raster) = draw(bytes);
    assert!(reports.is_empty(), "{reports:?}");
    assert!(
        tinted(&raster, 0) > 10,
        "the default style's colour is drawn"
    );
    let (low, high) = rows(&raster);
    assert!(high.saturating_sub(low) > 15, "in the default style's size");
}

/// Without Table 231 bit 26 the field is plain: §12.7.5.3 presents its text "in a single style
/// (font, size, colour, and so forth), as specified by the DA (default appearance) string", so
/// neither `/DS` nor `/RV` is read.
#[test]
fn a_field_without_the_flag_reads_neither_entry() {
    let bytes = pdf(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 190] /F 4 /FT /Tx /Ff 4096 \
         /T (f) /V (Red) /RV (<body><p style=\"color:#00ff00\">Red</p></body>) \
         /DS (color:#ff0000) /DA (/Helv 12 Tf 0 g) >>",
        "",
    );
    let (reports, raster) = draw(bytes);
    assert!(reports.is_empty(), "{reports:?}");
    assert_eq!(
        tinted(&raster, 0) + tinted(&raster, 1),
        0,
        "drawn in the /DA's black"
    );
}

/// `text-align` moves a line within the paragraph — *Horizontal Alignment*, Example 27.3 (page
/// 1190) — and outranks Table 228's `/Q`.
#[test]
fn text_align_moves_the_line_and_outranks_q() {
    let line = |style: &str, extra: &str| {
        field(
            RICH_MULTILINE,
            "Aligned",
            &format!("<body><p style=\"{style}\">Aligned</p></body>"),
            extra,
        )
    };
    let (_, left) = draw(line("text-align:left", "/Q 2"));
    let (_, right) = draw(line("text-align:right", ""));
    let (_, centre) = draw(line("text-align:center", ""));
    let (left_start, _) = columns(&left);
    let (_, right_end) = columns(&right);
    let (centre_start, centre_end) = columns(&centre);
    assert!(left_start < 16, "left, though /Q says right: {left_start}");
    assert!(right_end > 283, "right: {right_end}");
    let middle = u32::midpoint(centre_start, centre_end);
    assert!((145..=155).contains(&middle), "centred: {middle}");
}

/// `justify` sets every line of a paragraph but its last to the whole width, and `justify-all`
/// its last too — *Horizontal Alignment* (page 1190).
#[test]
fn justify_fills_every_line_but_the_last_and_justify_all_the_last_too() {
    let paragraph = |align: &str| {
        field(
            RICH_MULTILINE,
            "a b",
            &format!("<body><p style=\"text-align:{align}\">a b</p></body>"),
            "",
        )
    };
    let (_, justified) = draw(paragraph("justify"));
    let (_, all) = draw(paragraph("justify-all"));
    let (_, end) = columns(&justified);
    assert!(
        end < 60,
        "one line is a last line, and is not justified: {end}"
    );
    let (_, end) = columns(&all);
    assert!(
        end > 283,
        "justify-all sets even the last line to the width: {end}"
    );
}

/// `margin-left` indents a paragraph — *Left Margin*, Example 27.4 (page 1191): half an inch is
/// 36 points.
#[test]
fn a_left_margin_indents_the_paragraph() {
    let (_, raster) = draw(field(
        RICH_MULTILINE,
        "Indented",
        "<body><p style=\"margin-left:0.5in\">Indented</p></body>",
        "",
    ));
    let (start, _) = columns(&raster);
    assert!(
        (46..=50).contains(&start),
        "10 for the box and 36 for the margin: {start}"
    );
}

/// `text-indent` indents the first line only — *First Line Indent*, Example 27.2 (page 1190).
#[test]
fn a_first_line_indent_moves_the_first_line_alone() {
    let (_, raster) = draw(field(
        RICH_MULTILINE,
        "First Second",
        "<body><p style=\"text-indent:0.5in\">First<br/>Second</p></body>",
        "",
    ));
    let lines = bands(&raster);
    assert_eq!(lines.len(), 2, "{lines:?}");
    let first = first_column_in(&raster, lines[0]);
    let second = first_column_in(&raster, lines[1]);
    assert!(
        first >= second + 30,
        "the first line starts half an inch in: {first} {second}"
    );
}

/// `br` ends a line — *Line Break* (page 1191) — and `line-height` sets the distance between
/// baselines — *Line Spacing*, Example 27.5 (page 1191).
#[test]
fn a_line_break_ends_a_line_and_line_height_spaces_the_lines() {
    let lines = |style: &str| {
        let (_, raster) = draw(field(
            RICH_MULTILINE,
            "One Two",
            &format!("<body><p style=\"{style}\">One<br/>Two</p></body>"),
            "",
        ));
        bands(&raster)
    };
    let default = lines("");
    let spaced = lines("line-height:0.5in");
    assert_eq!(default.len(), 2, "{default:?}");
    assert_eq!(spaced.len(), 2, "{spaced:?}");
    let gap = |bands: &[(u32, u32)]| bands[0].0.saturating_sub(bands[1].0);
    assert!(
        (12..=14).contains(&gap(&default)),
        "13/12 of 12 points: {default:?}"
    );
    assert!(
        (35..=37).contains(&gap(&spaced)),
        "half an inch: {spaced:?}"
    );
}

/// `margin-top` and `margin-bottom` space paragraphs apart — *Space Before Paragraph* and *Space
/// After Paragraph*, Examples 27.12 and 27.13 (page 1196) — and adjoining ones collapse to the
/// larger, CSS2 section 8.3.1.
#[test]
fn paragraph_margins_space_paragraphs_and_collapse() {
    let gap = |markup: &str| {
        let (_, raster) = draw(field(RICH_MULTILINE, "One Two", markup, ""));
        let lines = bands(&raster);
        assert_eq!(lines.len(), 2, "{lines:?}");
        lines[0].0.saturating_sub(lines[1].0)
    };
    let plain = gap("<body><p>One</p><p>Two</p></body>");
    let after = gap("<body><p style=\"margin-bottom:0.5in\">One</p><p>Two</p></body>");
    let before = gap("<body><p>One</p><p style=\"margin-top:0.5in\">Two</p></body>");
    let both = gap(
        "<body><p style=\"margin-bottom:0.5in\">One</p><p style=\"margin-top:0.25in\">Two</p></body>",
    );
    assert!(
        (35..=37).contains(&after.saturating_sub(plain)),
        "{plain} {after}"
    );
    assert!(
        (35..=37).contains(&before.saturating_sub(plain)),
        "{plain} {before}"
    );
    assert_eq!(both, after, "the two margins collapse to the larger");
}

/// `vertical-align` raises and lowers a run without changing its size, from the line's own
/// baseline — *Baseline Adjustment*, Example 27.17 (page 1199). The run beside it stays on that
/// baseline, so the two words' tops are as far apart as the measurement.
#[test]
fn vertical_align_raises_and_lowers_a_run() {
    let tops = |markup: &str| {
        let (_, raster) = draw(rich("xx xx", markup));
        let ink = inked(&raster);
        let (from, to) = columns(&raster);
        let middle = u32::midpoint(from, to);
        let top = |left: bool| {
            ink.iter()
                .filter(|(x, _)| (*x < middle) == left)
                .map(|(_, y)| *y)
                .max()
                .unwrap_or_default()
        };
        (top(true), top(false))
    };
    let (level, beside) = tops("xx xx");
    assert!(
        level.abs_diff(beside) <= 1,
        "the control: one baseline, {level} {beside}"
    );
    let (base, raised) = tops("xx <span style=\"vertical-align:6pt\">xx</span>");
    assert!(
        (5..=7).contains(&raised.saturating_sub(base)),
        "{base} {raised}"
    );
    let (base, lowered) = tops("xx <span style=\"vertical-align:-6pt\">xx</span>");
    assert!(
        (5..=7).contains(&base.saturating_sub(lowered)),
        "{base} {lowered}"
    );
}

/// `sub` lowers by 15% and `sup` raises by 31% of the font height, both at 66% of it —
/// *Subscript* and *Superscript*, Examples 27.24 and 27.25 (pages 1204, 1205).
#[test]
fn subscript_and_superscript_move_by_the_chapters_figures() {
    let (sub, _) = appearance(rich("x2", "x<sub>2</sub>"));
    let (sup, _) = appearance(rich("x2", "x<sup>2</sup>"));
    assert!(sub.contains("/Helv 7.92 Tf"), "66% of 12: {sub}");
    assert!(sup.contains("/Helv 7.92 Tf"), "{sup}");
    let baseline = |content: &str, line: usize| {
        content
            .lines()
            .filter(|text| text.ends_with(" Tm"))
            .nth(line)
            .and_then(|text| text.split_whitespace().nth(5))
            .and_then(|y| y.parse::<f32>().ok())
            .expect("a Tm")
    };
    let drop = baseline(&sub, 0) - baseline(&sub, 1);
    let lift = baseline(&sup, 1) - baseline(&sup, 0);
    assert!((drop - 1.8).abs() < 0.01, "15% of 12: {drop}");
    assert!((lift - 3.72).abs() < 0.01, "31% of 12: {lift}");
}

/// `text-decoration` draws an underline and a line through in the run's colour — *Underline and
/// Strikethrough*, Examples 27.29 and 27.30 (pages 1208, 1209) — and `word` breaks at the space.
#[test]
fn decorations_are_drawn_beneath_and_through_the_run() {
    // The clip is written `re W n`; a decoration is a filled `re` of its own line.
    let (plain, _) = appearance(rich("a b", "a b"));
    assert!(
        !plain.contains(" re\n"),
        "nothing decorates an undecorated run: {plain}"
    );
    let (under, _) = appearance(rich(
        "a b",
        "<span style=\"text-decoration:underline\">a b</span>",
    ));
    assert_eq!(under.matches(" re\n").count(), 1, "one underline: {under}");
    let (word, _) = appearance(rich(
        "a b",
        "<span style=\"text-decoration:word\">a b</span>",
    ));
    assert_eq!(word.matches(" re\n").count(), 2, "a line a word: {word}");
    let (double, _) = appearance(rich("a", "<span style=\"text-decoration:double\">a</span>"));
    assert_eq!(double.matches(" re\n").count(), 2, "two lines: {double}");
    let (both, _) = appearance(rich(
        "a",
        "<span style=\"text-decoration:line-through underline\">a</span>",
    ));
    assert_eq!(
        both.matches(" re\n").count(),
        2,
        "an underline and a line through: {both}"
    );
    let (_, raster) = draw(rich(
        "Under",
        "<span style=\"text-decoration:underline\">Under</span>",
    ));
    let (_, plain) = draw(rich("Under", "Under"));
    assert!(
        rows(&raster).0 < rows(&plain).0,
        "the underline inks below the glyphs"
    );
}

/// `letter-spacing` widens a run — *Letter Spacing*, Example 27.23 (page 1204) — and an `em` is
/// the run's own size.
#[test]
fn letter_spacing_widens_the_run() {
    let width = |markup: &str| {
        let (from, to) = columns(&draw(rich("abcd", markup)).1);
        to.saturating_sub(from)
    };
    let plain = width("abcd");
    let spaced = width("<span style=\"letter-spacing:0.5em\">abcd</span>");
    // Four glyphs, six points after each; the last one's lies past its ink.
    assert!(
        (17..=19).contains(&spaced.saturating_sub(plain)),
        "{plain} {spaced}"
    );
}

/// `xfa-font-horizontal-scale` widens the glyphs — *Font Scale*, Example 27.21 (page 1202).
#[test]
fn horizontal_glyph_scale_widens_the_run() {
    let width = |markup: &str| {
        let (from, to) = columns(&draw(rich("mmmm", markup)).1);
        to.saturating_sub(from)
    };
    let plain = width("mmmm");
    let wide = width("<span style=\"xfa-font-horizontal-scale:150%\">mmmm</span>");
    let ratio = f64::from(wide) / f64::from(plain);
    assert!((1.4..=1.6).contains(&ratio), "{plain} {wide}");
}

/// `xfa-font-vertical-scale` shortens the glyphs — *Font Scale*, Example 27.21 (page 1202).
#[test]
fn vertical_glyph_scale_shortens_the_run() {
    let height = |markup: &str| {
        let (low, high) = rows(&draw(rich("XXXX", markup)).1);
        high.saturating_sub(low)
    };
    let plain = height("XXXX");
    let short = height("<span style=\"xfa-font-vertical-scale:50%\">XXXX</span>");
    let ratio = f64::from(short) / f64::from(plain);
    assert!((0.4..=0.6).contains(&ratio), "{plain} {short}");
}

/// `text-valign` places the block at the bottom of the box — *Vertical Alignment*, Example 27.14
/// (page 1197) — where a multiline field's text otherwise starts at the top.
#[test]
fn text_valign_moves_the_block_to_the_bottom() {
    let block = |style: &str| {
        rows(
            &draw(field(
                RICH_MULTILINE,
                "Low",
                &format!("<body><p style=\"{style}\">Low</p></body>"),
                "",
            ))
            .1,
        )
    };
    let (_, top) = block("");
    let (bottom, _) = block("text-valign:bottom");
    assert!(top > 170, "the default runs from the top: {top}");
    assert!(
        bottom < 20,
        "and text-valign:bottom sets it on the bottom: {bottom}"
    );
}

/// A run of spaces chapter 27 would compress is kept under `xfa-spacerun:yes` — *Retaining
/// Consecutive Spaces*, Examples 27.35 and 27.36 (pages 1220, 1221).
#[test]
fn a_space_run_keeps_its_spaces() {
    let width = |markup: &str| {
        let (from, to) = columns(&draw(rich("a b", markup)).1);
        to.saturating_sub(from)
    };
    let compressed = width("a      b");
    let kept = width("a<span style=\"xfa-spacerun:yes\">      </span>b");
    assert!(kept > compressed + 10, "{compressed} {kept}");
}

/// A list's items take their tags left of the content, half an inch in — *List Support*,
/// Examples 27.31 and 27.34 (pages 1209, 1218).
#[test]
fn a_list_item_takes_its_tag_left_of_its_content() {
    let (reports, raster) = draw(field(
        RICH_MULTILINE,
        "First Second",
        "<body><ol><li>First</li><li>Second</li></ol></body>",
        "",
    ));
    assert!(reports.is_empty(), "{reports:?}");
    let lines = bands(&raster);
    assert_eq!(lines.len(), 2, "{lines:?}");
    let tag = first_column_in(&raster, lines[0]);
    assert!(
        tag < 46 && tag > 20,
        "the tag sits in the list indent: {tag}"
    );
    let (content, _) = appearance(field(
        RICH_MULTILINE,
        "First Second",
        "<body><ol><li>First</li><li>Second</li></ol></body>",
        "",
    ));
    assert!(
        content.contains("(1.) Tj") && content.contains("(2.) Tj"),
        "{content}"
    );
}

/// The hyperlink takes the style chapter 27 recommends — *Hyperlink Support* (page 1189) — and
/// nothing is owed: following it is not the formatting §12.7.4.3 brings XFA 3.3 in for, and the
/// drawing is complete (ADR 1660).
#[test]
fn a_hyperlink_is_drawn_blue_and_underlined() {
    let (reports, raster) = draw(rich(
        "see here",
        "see <a href=\"http://example.invalid/\">here</a>",
    ));
    assert!(reports.is_empty(), "{reports:?}");
    assert!(tinted(&raster, 2) > 10, "the link is blue");
}

/// A property chapter 27 names and this tree does not carry out as stated is named, and the rest
/// drawn: a width the document's faces do not hold.
#[test]
fn a_property_not_carried_out_is_named() {
    let (reports, raster) = draw(rich(
        "narrow",
        "<span style=\"font-stretch:condensed\">narrow</span>",
    ));
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(reports[0].contains("font-stretch:condensed"), "{reports:?}");
    assert!(!inked(&raster).is_empty(), "the text is still drawn");
}

/// Where `/RV` states other characters than `/V`, `/V` wins: §12.7.5.3 makes it hold the field's
/// text (ADR 1635). The disagreement is said.
#[test]
fn a_value_its_rich_text_does_not_describe_is_drawn_as_the_value() {
    let (reports, raster) = draw(field(
        RICH_MULTILINE,
        "i",
        "<body><p style=\"color:#ff0000\">wwwwwwwwwwwwwwww</p></body>",
        "",
    ));
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(
        reports[0].contains("/RV") && reports[0].contains("/V"),
        "{reports:?}"
    );
    let (from, to) = columns(&raster);
    assert!(
        to.saturating_sub(from) < 10,
        "one narrow character: {from}..{to}"
    );
    assert_eq!(
        tinted(&raster, 0),
        0,
        "and none of the formatting that described other text"
    );
}

/// A single-line rich field joins its paragraphs onto the one line Table 231 allows it.
#[test]
fn a_single_line_field_keeps_its_rich_text_on_one_line() {
    let (_, raster) = draw(field(
        RICH,
        "One Two",
        "<body><p>One</p><p><b>Two</b></p></body>",
        "",
    ));
    assert_eq!(bands(&raster).len(), 1);
}

/// Table 177's `/RC` formats a free text annotation's appearance, and `/DS` styles it beneath:
/// "shall be used to generate the appearance of the annotation".
#[test]
fn a_free_text_annotations_rich_text_is_drawn_formatted() {
    let note = |rc: &str, ds: &str| {
        pdf(
            &format!(
                "<< /Type /Annot /Subtype /FreeText /Rect [10 10 290 190] /F 4 /Contents (Note) \
                 /RC ({rc}) {ds} /DA (/Helv 12 Tf 0 g) /Border [0 0 0] >>"
            ),
            "",
        )
    };
    let (reports, raster) = draw(note(
        "<body><p><span style=\"color:#00ff00\">Note</span></p></body>",
        "",
    ));
    assert!(reports.is_empty(), "{reports:?}");
    assert!(tinted(&raster, 1) > 10, "the run's colour");
    let (reports, raster) = draw(note("<body><p>Note</p></body>", "/DS (color:#ff0000)"));
    assert!(reports.is_empty(), "{reports:?}");
    assert!(
        tinted(&raster, 0) > 10,
        "the default style's colour beneath it"
    );
}

/// `/Contents` wins where `/RC` disagrees with it: §12.5.6.2 says it "specifies the displayed
/// text" (ADR 1635).
#[test]
fn a_free_text_annotations_contents_wins_where_its_rich_text_disagrees() {
    let (reports, raster) = draw(pdf(
        "<< /Type /Annot /Subtype /FreeText /Rect [10 10 290 190] /F 4 /Contents (i) \
         /RC (<body><p>wwwwwwwwwwwwwwww</p></body>) /DA (/Helv 12 Tf 0 g) /Border [0 0 0] >>",
        "",
    ));
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(
        reports[0].contains("/RC") && reports[0].contains("/Contents"),
        "{reports:?}"
    );
    let (from, to) = columns(&raster);
    assert!(to.saturating_sub(from) < 10, "{from}..{to}");
}

/// §12.7.4.3: for a rich text field "the entire annotation appearance shall be regenerated each
/// time the value is changed" — the artwork outside `/Tx` goes with the old value, where a plain
/// field's splice keeps it.
#[test]
fn a_changed_rich_value_regenerates_the_whole_appearance() {
    let content = "1 0 0 rg 0 0 280 180 re f /Tx BMC EMC";
    let stored = |flags: u32| {
        pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 190] /F 4 /FT /Tx /Ff {flags} \
                 /T (f) /V (old) /DA (/Helv 12 Tf 0 g) /DS (font-size:12pt) \
                 /AP << /N 8 0 R >> >>"
            ),
            "",
            &format!(
                "8 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 280 180] /Length {} >>\n\
                 stream\n{content}\nendstream\nendobj\n",
                content.len().saturating_add(1)
            ),
        )
    };
    for (flags, keeps) in [(RICH_MULTILINE, false), (4096, true)] {
        let document = Document::open(stored(flags)).expect("a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let mut view = pdf_model::view::ViewState::of(&document);
        assert_eq!(
            view.set_field(&document, "f", &Entered::Text("new".to_owned())),
            1
        );
        let list = pdf_model::content::interpret_with(&document, &page, &view).display_list;
        let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
        let raster = CpuRasterizer::new()
            .with_medium(pdf_render::Medium::NONE)
            .rasterize(&list, target)
            .expect("supported");
        assert_eq!(
            tinted(&raster, 0) > 1000,
            keeps,
            "flags {flags}: the stored red rectangle is {} the appearance drawn",
            if keeps { "kept in" } else { "not in" }
        );
    }
}

/// Table 231 bit 26's `/RV` is written beside the `/V` a person set — "[i]f the field has a
/// value, the RV entry of the field dictionary … shall specify the rich text string" — and it
/// reads back as that value; a cleared value takes its `/RV` with it (ADR 1635).
#[test]
fn a_save_writes_the_rich_text_string_beside_the_value() {
    let bytes = field(
        RICH_MULTILINE,
        "old",
        "<body><p><b>old</b></p></body>",
        "/DS (font-size:12pt)",
    );
    let document = Document::open(bytes).expect("a valid PDF");
    let widget = pdf_syntax::ObjectId::new(5, 0);
    let mut view = pdf_model::view::ViewState::of(&document);
    assert_eq!(
        view.set_field(
            &document,
            "f",
            &Entered::Text("a  <new> & value".to_owned())
        ),
        1
    );
    let written = view.save(&document).expect("the fixture can be written");
    let reopened = Document::open(written.bytes).expect("what was written is a PDF");
    let dict = reopened.get(widget).as_dict().cloned().expect("the field");
    let rv = match reopened.get_key(&dict, "RV") {
        pdf_syntax::Object::String(bytes) => pdf_syntax::text_string::text_string(&bytes),
        other => panic!("an /RV string, not {other:?}"),
    };
    assert!(rv.contains("xfa:spec=\"3.3\""), "{rv}");
    assert!(rv.contains("a  &lt;new&gt; &amp; value"), "{rv}");
    assert!(
        rv.contains("xfa-spacerun:yes"),
        "the doubled space is kept: {rv}"
    );

    let mut view = pdf_model::view::ViewState::of(&document);
    assert_eq!(view.set_field(&document, "f", &Entered::Cleared), 1);
    let written = view.save(&document).expect("the fixture can be written");
    let reopened = Document::open(written.bytes).expect("what was written is a PDF");
    let dict = reopened.get(widget).as_dict().cloned().expect("the field");
    assert!(
        reopened.get_key(&dict, "RV").is_null(),
        "a value gone takes its /RV with it"
    );
}

/// An FDF file with the header §12.7.8.2.2 states and the trailer §12.7.8.2.4 does.
fn fdf(fdf_dictionary: &str) -> Document {
    let bytes = format!(
        "%FDF-1.2\n1 0 obj\n<< /FDF {fdf_dictionary} >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n"
    );
    Document::open(bytes.into_bytes()).expect("an FDF file is opened by the PDF reader")
}

/// Page one under a view, as its reports and its raster.
fn draw_with(
    document: &Document,
    view: &pdf_model::view::ViewState,
) -> (Vec<String>, pdf_render::Raster) {
    let page = pdf_model::Pages::new(document).get(0).expect("page one");
    let interpretation = pdf_model::content::interpret_with(document, &page, view);
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

/// How wide the ink is.
fn ink_width(raster: &pdf_render::Raster) -> u32 {
    let (from, to) = columns(raster);
    to.saturating_sub(from)
}

/// Table 249's `/RV` replaces Table 228's: §12.7.8.3.2's "importing a field causes the values of
/// the entries in the FDF field dictionary to replace those of the corresponding entries in the
/// field with the same fully qualified name in the target document". An imported value with a bold
/// `/RV` beside it is drawn bold — the width the same rich string draws at when the file states it
/// — and one whose `/RV` states other characters is drawn as the value, said (ADRs 1635, 1648).
#[test]
fn an_imported_rich_value_is_drawn_in_its_formatting() {
    let (_, stated_bold) = draw(rich("Wide", "<b>Wide</b>"));
    let (_, plain) = draw(rich("Wide", "Wide"));
    let target = rich("Old", "Old");
    let document = Document::open(target).expect("a valid PDF");

    let mut view = pdf_model::view::ViewState::of(&document);
    let data = pdf_model::forms_data::FormsData::read(&fdf(
        "<< /Fields [ << /T (f) /V (Wide) /RV (<body><p><b>Wide</b></p></body>) >> ] >>",
    ))
    .expect("an FDF catalog");
    assert!(data.fields[0].owed.is_empty(), "{:?}", data.fields[0].owed);
    assert_eq!(view.import(&document, &data).widgets, 1);
    let (reports, imported) = draw_with(&document, &view);
    assert!(reports.is_empty(), "{reports:?}");
    assert_eq!(ink_width(&imported), ink_width(&stated_bold));
    assert!(ink_width(&imported) > ink_width(&plain));

    // The target's own `/RV` is not replaced by an FDF field stating none, and it describes `Old`:
    // the imported value is drawn as it stands, and the disagreement is said.
    let mut view = pdf_model::view::ViewState::of(&document);
    let data =
        pdf_model::forms_data::FormsData::read(&fdf("<< /Fields [ << /T (f) /V (Wide) >> ] >>"))
            .expect("an FDF catalog");
    assert_eq!(view.import(&document, &data).widgets, 1);
    let (reports, imported) = draw_with(&document, &view);
    assert_eq!(ink_width(&imported), ink_width(&plain));
    assert!(
        reports.iter().any(|report| report.contains("/RV")),
        "{reports:?}"
    );
}

/// A save writes what an import replaced: §12.7.8.3.2's "importing a field causes the values of
/// the entries in the FDF field dictionary to replace those of the corresponding entries in the
/// field with the same fully qualified name in the target document", so the saved field states
/// the imported `/V`, the `/RV` beside it and the flags Table 249's `/SetFf` changed, and the file
/// read back draws what the screen showed (ADR 1661).
#[test]
fn a_save_writes_the_imported_value_and_its_rich_text_string() {
    let (_, stated_bold) = draw(rich("Wide", "<b>Wide</b>"));
    let document = Document::open(rich("Old", "Old")).expect("a valid PDF");
    let widget = pdf_syntax::ObjectId::new(5, 0);
    let mut view = pdf_model::view::ViewState::of(&document);
    // Table 227 bit 1, `ReadOnly`, set by the import.
    let data = pdf_model::forms_data::FormsData::read(&fdf(
        "<< /Fields [ << /T (f) /V (Wide) /RV (<body><p><b>Wide</b></p></body>) /SetFf 1 >> ] >>",
    ))
    .expect("an FDF catalog");
    assert_eq!(view.import(&document, &data).widgets, 1);
    let (_, shown) = draw_with(&document, &view);
    let written = view.save(&document).expect("the fixture can be written");
    assert!(written.withheld.is_empty(), "{:?}", written.withheld);
    let reopened = Document::open(written.bytes).expect("what was written is a PDF");
    let dict = reopened.get(widget).as_dict().cloned().expect("the field");
    let text = |key: &str| match reopened.get_key(&dict, key) {
        pdf_syntax::Object::String(bytes) => pdf_syntax::text_string::text_string(&bytes),
        other => panic!("a /{key} string, not {other:?}"),
    };
    assert_eq!(text("V"), "Wide");
    assert_eq!(text("RV"), "<body><p><b>Wide</b></p></body>");
    assert_eq!(
        reopened.get_key(&dict, "Ff").as_integer(),
        Some(i64::from(RICH_MULTILINE) | 1)
    );
    let (reports, saved) = draw_with(&reopened, &pdf_model::view::ViewState::of(&reopened));
    assert!(reports.is_empty(), "{reports:?}");
    assert_eq!(ink_width(&saved), ink_width(&shown));
    assert_eq!(ink_width(&saved), ink_width(&stated_bold));

    // A person who types after the import has made the later statement, and the save writes it;
    // this import leaves the flags alone, because `ReadOnly` refuses a person.
    let mut view = pdf_model::view::ViewState::of(&document);
    let data =
        pdf_model::forms_data::FormsData::read(&fdf("<< /Fields [ << /T (f) /V (Wide) >> ] >>"))
            .expect("an FDF catalog");
    assert_eq!(view.import(&document, &data).widgets, 1);
    assert_eq!(
        view.set_field(&document, "f", &Entered::Text("typed".to_owned())),
        1
    );
    let written = view.save(&document).expect("the fixture can be written");
    let reopened = Document::open(written.bytes).expect("what was written is a PDF");
    let dict = reopened.get(widget).as_dict().cloned().expect("the field");
    assert_eq!(
        reopened
            .get_key(&dict, "V")
            .as_string()
            .map(pdf_syntax::text_string),
        Some("typed".to_owned())
    );
}

/// XFDF 3.0's `<value-richtext>` is Table 249's `/RV`, and with no `<value>` beside it the field's
/// value is the string's characters (*The value and value-richtext elements in fields*, page 31;
/// *value-richtext*, page 36): the import draws the formatting the element states (ADR 1648).
#[test]
fn an_xfdf_value_richtext_is_imported_as_the_rich_value() {
    let (_, stated_bold) = draw(rich("Wide", "<b>Wide</b>"));
    let document = Document::open(rich("Old", "Old")).expect("a valid PDF");
    let data = pdf_model::xfdf::read(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <xfdf xmlns=\"http://ns.adobe.com/xfdf/\" xml:space=\"preserve\"><fields>\
         <field name=\"f\"><value-richtext><body xmlns=\"http://www.w3.org/1999/xhtml\">\
         <p><b>Wide</b></p></body></value-richtext></field>\
         <field name=\"g\"><value-richtext>plain words</value-richtext></field>\
         </fields></xfdf>"
            .as_bytes(),
    )
    .expect("well-formed XFDF");
    assert!(data.owed.is_empty(), "{:?}", data.owed);
    let text = |at: usize| {
        data.fields[at]
            .value
            .as_ref()
            .and_then(pdf_syntax::Object::as_string)
            .map(pdf_syntax::text_string)
    };
    assert_eq!(text(0).as_deref(), Some("Wide"));
    assert!(
        data.fields[0]
            .rich_value
            .as_deref()
            .is_some_and(|markup| markup.starts_with("<body") && markup.contains("<b>Wide</b>")),
        "{:?}",
        data.fields[0].rich_value
    );
    // Plain text inside the element is the value, and there is no rich text string to carry.
    assert_eq!(text(1).as_deref(), Some("plain words"));
    assert_eq!(data.fields[1].rich_value, None);

    let mut view = pdf_model::view::ViewState::of(&document);
    assert_eq!(view.import(&document, &data).widgets, 1);
    let (reports, imported) = draw_with(&document, &view);
    assert!(reports.is_empty(), "{reports:?}");
    assert_eq!(ink_width(&imported), ink_width(&stated_bold));
}

/// Table 231 bit 25, `Comb`: `1 << 24`.
const COMB: u32 = 16_777_216;

/// A caret at a byte of the value, asked at the middle of the fixture's box.
fn caret(bytes: &[u8], offset: usize) -> [f32; 4] {
    let document = Document::open(bytes.to_vec()).expect("a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let view = pdf_model::view::ViewState::of(&document);
    view.caret_at(&document, &page, 150.0, 100.0, offset)
        .expect("the field lays its text out")
}

/// The glyphs a host's text interface is told of, left to right.
fn glyphs(bytes: &[u8]) -> Vec<pdf_model::view::FieldGlyph> {
    let document = Document::open(bytes.to_vec()).expect("a valid PDF");
    let view = pdf_model::view::ViewState::of(&document);
    view.field_glyphs(&document, pdf_syntax::ObjectId::new(5, 0))
        .expect("the field lays its text out")
}

/// A host's caret, point and range are answered from the runs as drawn, each in its own size:
/// the caret after three 24-point capitals stands three of Helvetica's 944-unit advances at 24
/// points past the caret before them — §9.4.4's `w0 × Tfs` — where a one-style answer would put it
/// half as far (ADR 1649).
#[test]
fn a_caret_point_and_range_follow_the_runs_as_drawn() {
    let bytes = rich("aWWWb", "a<span style=\"font-size:24pt\">WWW</span>b");
    let (before, after) = (caret(&bytes, 1), caret(&bytes, 4));
    let expected = 3.0 * 0.944 * 24.0;
    assert!(
        (after[0] - before[0] - expected).abs() < 0.05,
        "{before:?} {after:?}"
    );
    // The caret stands between the line's descent and ascent, and the line holds a 24-point run.
    assert!(after[3] - after[1] > 20.0, "{after:?}");

    let document = Document::open(bytes.clone()).expect("a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let view = pdf_model::view::ViewState::of(&document);
    let offset = view.offset_at(
        &document,
        &page,
        (150.0, 100.0),
        (after[0] + 0.5, (after[1] + after[3]) * 0.5),
    );
    assert_eq!(offset, Some(4), "a point at a caret names its offset");
    let shapes = view
        .field_selection(&document, &page, (150.0, 100.0), (1, 4))
        .expect("a range");
    assert_eq!(shapes.len(), 1, "{shapes:?}");
    assert!((shapes[0][0] - before[0]).abs() < 0.01 && (shapes[0][2] - after[0]).abs() < 0.01);

    let placed = glyphs(&bytes);
    let ranges: Vec<_> = placed.iter().map(|glyph| glyph.bytes.clone()).collect();
    assert_eq!(ranges, [0..1, 1..2, 2..3, 3..4, 4..5]);
}

/// The offsets a host sends are the value's, and a rich text string compresses the value's white
/// space (chapter 27, page 1194): the caret after a doubled space in `/V` stands where the one
/// space the string draws ends.
#[test]
fn a_caret_offset_is_the_values_where_the_string_compresses_its_spaces() {
    let bytes = rich("a  b", "a b");
    let doubled = caret(&bytes, 3);
    let single = caret(&rich("a b", "a b"), 2);
    assert!(
        (doubled[0] - single[0]).abs() < 0.01,
        "{doubled:?} {single:?}"
    );
}

/// Table 231 bit 25 over the runs: the field is divided into `/MaxLen` equally spaced positions
/// and the text laid out into them, and each character a run states is set in its cell in that
/// run's face — the bold one here — with nothing laid out in one style.
#[test]
fn a_comb_field_sets_each_cell_in_its_runs_style() {
    let bytes = field(
        RICH | COMB,
        "abcde",
        "<body><p>ab<b>c</b>de</p></body>",
        "/MaxLen 5",
    );
    let (content, _) = appearance(bytes.clone());
    assert!(content.contains("/HeBo"), "{content}");
    let (reports, _) = draw(bytes.clone());
    assert!(reports.is_empty(), "{reports:?}");
    let placed = glyphs(&bytes);
    assert_eq!(placed.len(), 5);
    let widths: Vec<f32> = placed
        .iter()
        .map(|glyph| glyph.quad[2] - glyph.quad[0])
        .collect();
    for width in &widths {
        assert!((width - widths[0]).abs() < 0.01, "equal cells: {widths:?}");
    }
    assert!(
        (widths[0] * 5.0 - 278.0).abs() < 0.5,
        "five cells across the box inside its border: {widths:?}"
    );
}

/// UAX #9 across runs: a right-to-left override makes the whole run read right to left, so its
/// last character — the bold one — is displayed first, and a caret at the value's end stands at
/// the line's left (ADRs 1413, 1649).
#[test]
fn a_right_to_left_run_is_displayed_in_uax_9_order_across_its_styles() {
    // `/V` as §7.9.2.2's UTF-16BE text string, `/RV` with XML's character reference.
    let bytes = pdf(
        &format!(
            "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 190] /F 4 /FT /Tx \
             /Ff {RICH_MULTILINE} /T (f) /V <FEFF202E00610062> \
             /RV (<body><p>&#x202E;a<b>b</b></p></body>) /DA (/Helv 12 Tf 0 g) >>"
        ),
        "",
    );
    let placed = glyphs(&bytes);
    let starts: Vec<usize> = placed.iter().map(|glyph| glyph.bytes.start).collect();
    assert_eq!(starts, [4, 3], "b is shown left of a: {placed:?}");
    let (start, end) = (caret(&bytes, 3), caret(&bytes, 5));
    assert!(end[0] < start[0], "{start:?} {end:?}");
    let (content, _) = appearance(bytes);
    let bold = content.find("/HeBo").expect("the bold run");
    let regular = content.rfind("/Helv").expect("the regular run");
    assert!(
        bold < regular,
        "the bold run is written first, leftmost: {content}"
    );
}

/// `font-stretch` is chapter 27's width (*Font*, page 1201), and Table 120's `/FontStretch` names
/// the same nine widths in the same order: a run asking for `condensed` is set in the `/DR` face
/// whose descriptor states `/Condensed`, and nothing is reported (ADR 1649). Where no face states
/// the width, the run is drawn in the nearest one its family's faces hold and the width is said
/// (`a_width_no_face_states_is_set_in_the_nearest_one`, `a_property_not_carried_out_is_named`).
#[test]
fn a_width_is_set_in_the_face_whose_descriptor_states_it() {
    let bytes = pdf_with_fonts(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 190] /F 4 /FT /Tx /Ff 33558528 \
         /T (f) /V (ab) /RV (<body><p>a<span style=\"font-stretch:condensed\">b</span></p></body>) \
         /DA (/Helv 12 Tf 0 g) >>",
        "",
        "8 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Condensed \
         /Encoding /WinAnsiEncoding /FontDescriptor 9 0 R >>\nendobj\n\
         9 0 obj\n<< /Type /FontDescriptor /FontName /Helvetica-Condensed \
         /FontFamily (Helvetica) /FontStretch /Condensed /FontWeight 400 /Flags 32 \
         /FontBBox [0 -200 800 900] /ItalicAngle 0 /Ascent 750 /Descent -250 /CapHeight 700 \
         /StemV 80 >>\nendobj\n",
        "/HeCo 8 0 R",
    );
    let (content, _) = appearance(bytes.clone());
    assert!(content.contains("/HeCo"), "{content}");
    let (reports, _) = draw(bytes);
    assert!(reports.is_empty(), "{reports:?}");
}

/// A width no `/DR` face of the family states is set in the nearest one that does, and said:
/// CSS2 section 15.5 states no matching criterion for `font-stretch` and leaves the best match to
/// the processor, and the order is ADR 1660's choice — a condensed request tries the narrower
/// widths first, so `ultra-condensed` takes the `/Condensed` face over the normal one, while an
/// expanded request with no wider face takes the nearest narrower one.
#[test]
fn a_width_no_face_states_is_set_in_the_nearest_one() {
    let with = |width: &str| {
        pdf_with_fonts(
            &format!(
                "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 190] /F 4 /FT /Tx /Ff 33558528 \
                 /T (f) /V (ab) /RV (<body><p>a<span style=\"font-stretch:{width}\">b</span></p></body>) \
                 /DA (/Helv 12 Tf 0 g) >>"
            ),
            "",
            "8 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Condensed \
             /Encoding /WinAnsiEncoding /FontDescriptor 9 0 R >>\nendobj\n\
             9 0 obj\n<< /Type /FontDescriptor /FontName /Helvetica-Condensed \
             /FontFamily (Helvetica) /FontStretch /Condensed /FontWeight 400 /Flags 32 \
             /FontBBox [0 -200 800 900] /ItalicAngle 0 /Ascent 750 /Descent -250 /CapHeight 700 \
             /StemV 80 >>\nendobj\n",
            "/HeCo 8 0 R",
        )
    };
    let (content, _) = appearance(with("ultra-condensed"));
    assert!(content.contains("/HeCo"), "{content}");
    let (reports, _) = draw(with("ultra-condensed"));
    assert!(
        reports
            .first()
            .is_some_and(|report| report.contains("font-stretch:ultra-condensed")),
        "{reports:?}"
    );
    // `/Helv` is the family's normal width, stated without a descriptor, and so the nearest to
    // `semi-expanded` on its narrower side; nothing wider is held.
    let (content, _) = appearance(with("semi-expanded"));
    assert!(!content.contains("/HeCo"), "{content}");
}

/// A character none of a run's faces draws, in a face this program chose, is set in a face from
/// this machine and every other run keeps its own style: the italic run is still written in the
/// oblique face beside it, and nothing is laid out in one style (ADRs 1414, 1660). Where the
/// machine offers no face covering the character there is nothing to set it in, and the test says
/// so and stops (ADR 1154).
#[test]
fn a_character_no_run_face_draws_is_set_in_a_machine_face_beside_the_runs() {
    let missing = '\u{416}';
    let request = pdf_font::substitute::Request {
        family: pdf_font::substitute::Family::SansSerif,
        bold: false,
        italic: true,
        standard: false,
    };
    if pdf_font::substitute::installed_covering(request, &[missing]).is_none() {
        println!("skipped: no face on this machine covers U+0416");
        return;
    }
    // `/V` as §7.9.2.2's UTF-16BE text string, `/RV` with XML's character reference.
    let bytes = pdf(
        &format!(
            "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 190] /F 4 /FT /Tx \
             /Ff {RICH_MULTILINE} /T (f) /V <FEFF0061002004160062> \
             /RV (<body><p>a <i>&#x416;b</i></p></body>) /DA (/Helv 12 Tf 0 g) >>"
        ),
        "",
    );
    let (content, fonts) = appearance(bytes.clone());
    assert!(
        fonts.iter().any(|font| font.contains("Oblique")),
        "the italic run keeps its oblique face: {fonts:?}"
    );
    // The machine's face, which the appearance writes as §9.9.2's subset, tagged (ADR 1425).
    assert!(
        fonts
            .iter()
            .any(|font| font.split_once('+').is_some_and(|(tag, _)| tag.len() == 6)),
        "{fonts:?}"
    );
    assert!(
        content.contains("/Helv"),
        "the plain run keeps the /DA's face: {content}"
    );
    let (reports, raster) = draw(bytes);
    assert!(
        reports
            .iter()
            .all(|report| !report.contains("one style") && !report.contains("default style alone")),
        "{reports:?}"
    );
    assert!(!inked(&raster).is_empty());
}

/// A tab in a paragraph read right to left moves to the next stop on the left — chapter 2's
/// *Tab Stops* (page 61) — and a default stop right-aligns what follows it there (chapter 27,
/// page 1205): `אב` ends at the right margin, and `ג` after one tab ends at the nearest multiple of
/// the half-inch interval left of where `אב` begins, 252 points from the margin, the paragraph
/// aligned to the edge it starts at (ADR 1660). The
/// Hebrew is set in a face from this machine beside the italic run's, so where the machine offers
/// none there is nothing to place, and the test says so and stops (ADR 1154).
#[test]
fn a_tab_in_a_paragraph_read_right_to_left_moves_to_the_stop_on_the_left() {
    let letters = ['\u{5d0}', '\u{5d1}', '\u{5d2}'];
    let request = pdf_font::substitute::Request {
        family: pdf_font::substitute::Family::SansSerif,
        bold: false,
        italic: true,
        standard: false,
    };
    if pdf_font::substitute::installed_covering(request, &letters).is_none() {
        println!("skipped: no face on this machine covers the Hebrew letters");
        return;
    }
    let bytes = pdf(
        &format!(
            "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 190] /F 4 /FT /Tx \
             /Ff {RICH_MULTILINE} /T (f) /V <FEFF05D005D105D2> \
             /RV (<body><p style=\"tab-interval:36pt;text-align:right\"><i>&#x5D0;&#x5D1;\
             <span style=\"xfa-tab-count:1\"/>&#x5D2;</i></p></body>) /DA (/Helv 12 Tf 0 g) >>"
        ),
        "",
    );
    let placed = glyphs(&bytes);
    let span = |byte: usize| {
        let glyph = placed
            .iter()
            .find(|glyph| glyph.bytes.start == byte)
            .unwrap_or_else(|| panic!("a glyph from byte {byte}: {placed:?}"));
        let xs = [glyph.quad[0], glyph.quad[2], glyph.quad[4], glyph.quad[6]];
        (
            xs.iter().copied().fold(f32::INFINITY, f32::min),
            xs.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        )
    };
    let (alef, bet, gimel) = (span(0), span(2), span(4));
    let right_margin = 290.0 - 1.0;
    assert!(
        (alef.1 - right_margin).abs() < 0.01,
        "the first letter at the right margin: {placed:?}"
    );
    assert!(bet.1 <= alef.0 + 0.01, "read right to left: {placed:?}");
    assert!(
        (gimel.1 - (MARGIN + 252.0)).abs() < 0.01,
        "right-aligned at the stop: {placed:?}"
    );
    let (reports, _) = draw(bytes);
    assert!(reports.is_empty(), "{reports:?}");
}

/// A generated tag no run's face draws whole is set a face at a time: `japanese-informal`'s 1234
/// is six ideographs a face from this machine draws, and its full stop — *List Layout*'s suffix
/// (page 1219) — is the italic run's own, written as a group of its own (ADR 1660). Where the
/// machine offers no face for the ideographs the test says so and stops (ADR 1154).
#[test]
fn a_tag_in_two_faces_is_written_a_face_at_a_time() {
    let numeral = [
        '\u{5343}', '\u{4e8c}', '\u{767e}', '\u{4e09}', '\u{5341}', '\u{56db}',
    ];
    let request = pdf_font::substitute::Request {
        family: pdf_font::substitute::Family::SansSerif,
        bold: false,
        italic: true,
        standard: false,
    };
    if pdf_font::substitute::installed_covering(request, &numeral).is_none() {
        println!("skipped: no face on this machine covers the numeral");
        return;
    }
    let (content, _) = appearance(field(
        RICH_MULTILINE,
        "sen",
        "<body><ol style=\"font-style:italic;list-style-type:japanese-informal\" start=\"1234\">\
         <li>sen</li></ol></body>",
        "",
    ));
    assert!(
        content.contains("(.) Tj"),
        "the full stop in its own face: {content}"
    );
    assert!(content.matches(" Tf").count() >= 2, "{content}");
}

/// Where each glyph of a field starts, by line, from the glyphs a host is told of.
fn starts_by_line(bytes: &[u8]) -> Vec<Vec<f32>> {
    let mut lines: Vec<Vec<f32>> = Vec::new();
    for glyph in glyphs(bytes) {
        if lines.len() <= glyph.line {
            lines.resize(glyph.line.saturating_add(1), Vec::new());
        }
        if let Some(line) = lines.get_mut(glyph.line) {
            line.push(glyph.quad[0]);
        }
    }
    lines
}

/// The left margin of the fixture's box: `/Rect`'s left side inset by the one-point border.
const MARGIN: f32 = 11.0;

/// `tab-interval` and `xfa-tab-count` — *Tab Stops*, Example 27.26 (page 1206): default stops at
/// every multiple of the interval from the left margin, and a count of two advances past two of
/// them. Half an inch is 36 points, so after a one-letter run the second stop is at 72 and the
/// next past the following letter at 108.
#[test]
fn a_tab_count_advances_by_the_default_stops() {
    let bytes = field(
        RICH_MULTILINE,
        "XYZ",
        "<body><p style=\"tab-interval:0.5in\">X<span style=\"xfa-tab-count:2\"/>Y\
         <span style=\"xfa-tab-count:1\"/>Z</p></body>",
        "",
    );
    let (reports, _) = draw(bytes.clone());
    assert!(reports.is_empty(), "{reports:?}");
    let lines = starts_by_line(&bytes);
    assert_eq!(lines.len(), 1, "{lines:?}");
    let starts = &lines[0];
    assert!((starts[0] - MARGIN).abs() < 0.01, "{starts:?}");
    assert!((starts[1] - (MARGIN + 72.0)).abs() < 0.01, "{starts:?}");
    assert!((starts[2] - (MARGIN + 108.0)).abs() < 0.01, "{starts:?}");
}

/// `tab-stops` at stated positions — Example 27.27 (page 1207): two left stops, and a `br` that
/// sends the cursor back to the margin and the tab index to zero, so both lines' second words
/// start at the second stop.
#[test]
fn stated_tab_stops_align_each_line_and_a_break_restarts_them() {
    let bytes = field(
        RICH_MULTILINE,
        "Ann Lee\rBo Ng",
        "<body><p style=\"tab-stops:left 0.5in left 2in\"><span style=\"xfa-tab-count:1\"/>Ann \
         <span style=\"xfa-tab-count:1\"/>Lee<br/><span style=\"xfa-tab-count:1\"/>Bo \
         <span style=\"xfa-tab-count:1\"/>Ng</p></body>",
        "",
    );
    let lines = starts_by_line(&bytes);
    assert_eq!(lines.len(), 2, "{lines:?}");
    for line in &lines {
        assert!((line[0] - (MARGIN + 36.0)).abs() < 0.01, "{lines:?}");
    }
    assert!((lines[0][4] - (MARGIN + 144.0)).abs() < 0.01, "{lines:?}");
    assert!((lines[1][3] - (MARGIN + 144.0)).abs() < 0.01, "{lines:?}");
}

/// A decimal stop — Example 27.28 (page 1207): each line's first radix character stands at the
/// stop, and a line with none ends there. Courier's every advance is 600 units, 7.2 points at
/// twelve, so the arithmetic is the clause's and the font's alone.
#[test]
fn a_decimal_stop_aligns_the_radix_or_the_right_edge() {
    let bytes = field(
        RICH_MULTILINE,
        "1.25\r99\r.5",
        "<body><p style=\"font-family:Courier;tab-stops:decimal 0.5in\">\
         <span style=\"xfa-tab-count:1\"/>1.25<br/><span style=\"xfa-tab-count:1\"/>99<br/>\
         <span style=\"xfa-tab-count:1\"/>.5</p></body>",
        "",
    );
    let lines = starts_by_line(&bytes);
    assert_eq!(lines.len(), 3, "{lines:?}");
    let stop = MARGIN + 36.0;
    assert!(
        (lines[0][1] - stop).abs() < 0.01,
        "the radix at the stop: {lines:?}"
    );
    assert!(
        (lines[1][0] - (stop - 14.4)).abs() < 0.01,
        "the right edge: {lines:?}"
    );
    assert!((lines[2][0] - stop).abs() < 0.01, "{lines:?}");
}

/// `xfa-tab-stops`' leader (chapter 2's *Tab Leader Pattern*, pages 63 to 65) fills the room
/// before the text its stop aligns: `dots()` as the run's full stops, whole cycles only, since the
/// chapter leaves a partial cycle unrendered; a solid rule as one piece across the room, a dashed
/// one as pieces, and content repeated as many whole times as fit; the stop is
/// still used, and only `page` alignment, whose page edge a field does not know, is said
/// (ADR 1660).
#[test]
fn a_tab_leader_fills_the_room_before_its_stop() {
    let with = |leader: &str| {
        field(
            RICH_MULTILINE,
            "AB",
            &format!(
                "<body><p style=\"xfa-tab-stops:left {leader} 1in\">A\
                 <span style=\"xfa-tab-count:1\"/>B</p></body>"
            ),
            "",
        )
    };
    let positioned = |content: &str| content.matches(" Tm").count();
    // Filled pieces, not the clip the appearance opens with (`re W n`).
    let pieces = |content: &str| content.lines().filter(|line| line.ends_with(" re")).count();
    let bytes = with("leader(dots())");
    let (reports, _) = draw(bytes.clone());
    assert!(reports.is_empty(), "{reports:?}");
    let lines = starts_by_line(&bytes);
    assert!((lines[0][1] - (MARGIN + 72.0)).abs() < 0.01, "{lines:?}");
    let (dots, _) = appearance(bytes);
    // The room runs from after `A`, 8.004 points from the margin at 12 points, to the stop at 72.
    // A full stop is 3.336 points wide; on the grid from the margin the first whole cycle starts
    // at the third (10.008) and the last ends at the twenty-first (70.056): 18 cycles.
    assert_eq!(positioned(&dots), 2 + 18, "{dots}");

    // Twice the width: from the second cycle (13.344) to the tenth's end (66.72), 8 of them.
    let (wide, _) = appearance(with("leader(dots() none 6.672pt)"));
    assert_eq!(positioned(&wide), 2 + 8, "a cycle twice the dot's: {wide}");

    let (solid, _) = appearance(with("leader(rule(solid 1pt))"));
    assert_eq!(pieces(&solid), 1, "{solid}");
    assert!(solid.contains(" 1 re"), "one point thick: {solid}");
    let (dashed, _) = appearance(with("leader(rule(dashed 1pt))"));
    assert!(pieces(&dashed) > 10, "{dashed}");
    let (none, _) = appearance(with("leader(rule(none))"));
    assert_eq!(pieces(&none), 0, "{none}");
    assert_eq!(positioned(&none), 2, "{none}");

    let (content, _) = appearance(with("leader(use-content('-'))"));
    assert!(positioned(&content) > 2, "{content}");

    let (reports, _) = draw(with("leader(dots() page)"));
    assert!(
        reports
            .first()
            .is_some_and(|report| report.contains("leader alignment to the page")),
        "{reports:?}"
    );
}

/// Liberation Sans with its own kerning taken out and one pair written in: a `kern` table whose
/// one format 0 subtable kerns `A` against `V` by −300 of the face's 2048 units, so every expected
/// value below is the table this fixture wrote (trap 8; ADR 1682).
fn kerned_face() -> Vec<u8> {
    use read_fonts::TableProvider as _;
    const LIBERATION: &[u8] =
        include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");
    let font = read_fonts::FontRef::new(LIBERATION).expect("Liberation Sans is an sfnt");
    let cmap = font.cmap().expect("a cmap");
    let glyph = |character: char| {
        u16::try_from(
            cmap.map_codepoint(character)
                .expect("Liberation Sans draws it")
                .to_u32(),
        )
        .expect("a 16-bit glyph index")
    };
    let words = |values: &[u16]| -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_be_bytes())
            .collect()
    };
    let kern = words(&[
        0,
        1,
        0,
        20,
        0x0001,
        1,
        6,
        0,
        0,
        glyph('A'),
        glyph('V'),
        (-300_i16).cast_unsigned(),
    ]);
    let bare = pdf_font::embedding::without_tables(LIBERATION, &[*b"GPOS", *b"kern"])
        .expect("Liberation Sans is an sfnt");
    pdf_font::embedding::with_tables(&bare, &[(*b"kern", kern)]).expect("the table goes in")
}

/// A rich text field set in `/Kern`, the face [`kerned_face`] embeds, with `markup` as its `/RV`.
fn in_kerned_face(value: &str, markup: &str) -> Vec<u8> {
    let program = kerned_face().iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02X}");
        hex
    });
    // Liberation Sans is metric-compatible with Helvetica: 1366 of 2048 units is 667 thousandths
    // for `A` and `V`, 569 is 278 for the space.
    let mut widths = vec!["0"; 55];
    widths[0] = "278";
    widths[33] = "667";
    widths[54] = "667";
    pdf_with_fonts(
        &format!(
            "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 190] /F 4 /FT /Tx /Ff {RICH} \
             /T (f) /V ({value}) /RV (<body><p>{markup}</p></body>) /DA (/Kern 12 Tf 0 g) >>"
        ),
        "",
        &format!(
            "8 0 obj\n<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans \
             /FirstChar 32 /LastChar 86 /Widths [{}] /Encoding /WinAnsiEncoding \
             /FontDescriptor 9 0 R >>\nendobj\n\
             9 0 obj\n<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
             /FontBBox [-203 -303 1050 910] /ItalicAngle 0 /Ascent 905 /Descent -212 \
             /CapHeight 729 /StemV 80 /FontFile2 10 0 R >>\nendobj\n\
             10 0 obj\n<< /Length {} /Filter /ASCIIHexDecode >>\nstream\n{program}>\n\
             endstream\nendobj\n",
            widths.join(" "),
            program.len().saturating_add(1),
        ),
        "/Kern 8 0 R",
    )
}

/// `kerning-mode:pair` kerns a run by its face's own pairs (XFA 3.3 chapter 27, *Kerning*, pages
/// 1203 and 1204): the `A V` pair the fixture's `kern` table states is drawn −300/2048 of an em
/// closer, written as §9.4.3's `TJ` adjustment of 1000 × 300 / 2048 = 146.484375 thousandths,
/// and `V A`, which the table does not state, is not moved. Over `AVAV` at 12 points the two
/// pairs take 2 × 12 × 300 / 2048 = 3.515625 points out of the ink's width. Nothing is said,
/// because the property is carried out (ADR 1682).
#[test]
fn pair_kerning_draws_a_pair_the_face_states_closer() {
    let kerned = in_kerned_face("AVAV", "<span style=\"kerning-mode:pair\">AVAV</span>");
    let plain = in_kerned_face("AVAV", "AVAV");
    let (content, _) = appearance(kerned.clone());
    assert!(
        content.contains("[(A) 146.48438 (VA) 146.48438 (V)] TJ"),
        "{content}"
    );
    let (unkerned_content, _) = appearance(plain.clone());
    assert!(unkerned_content.contains("(AVAV) Tj"), "{unkerned_content}");
    let (reports, kerned_raster) = draw(kerned);
    assert!(reports.is_empty(), "{reports:?}");
    let (reports, plain_raster) = draw(plain);
    assert!(reports.is_empty(), "{reports:?}");
    let narrower = ink_width(&plain_raster).saturating_sub(ink_width(&kerned_raster));
    assert!((3..=4).contains(&narrower), "{narrower} px");
}

/// A face that states no pairs is not kerned by numbers that are not its own, and the run says
/// so: §9.6.2.2's fourteen carry none in this tree until `doc/questions/Q308` is answered.
#[test]
fn pair_kerning_in_a_face_without_pairs_is_said() {
    let (reports, raster) = draw(rich(
        "AVAV",
        "<span style=\"kerning-mode:pair\">AVAV</span>",
    ));
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(
        reports[0].contains("kerning-mode:pair, in a face"),
        "{reports:?}"
    );
    assert!(!inked(&raster).is_empty(), "the text is still drawn");
}

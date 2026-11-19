//! A font whose embedded program is present and unreadable is drawn from a substitute face.
//!
//! ISO 32000-2 §9.5 says a font dictionary carries
//!
//! > information that can be used to provide a substitute when the font program is not available
//!
//! and §9.8.1 that a descriptor's metrics
//!
//! > provide information that enables a PDF processor to synthesise a substitute font or select a
//! > similar font when the font program is unavailable
//!
//! A program this reader cannot read — data that does not decode, stops short, or is not a program
//! in the format it is filed under — is unavailable in that sense, and §9.9 counts the front of one
//! as none (ADR 2619). So the window draws such a font as it draws one naming no program (ADR 0007)
//! and says the refusal out loud as before (`doc/questions/A837`, ADR 2870). What it does not
//! change is §9.7.5.2's forbidden combination, "\[t]he Identity-H and Identity-V `CMaps` shall not
//! be used with a non-embedded font", nor a composite font §9.7.4.2 leaves no route to a
//! substitute: both still draw nothing (ADR 1094).

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that stops opening should fail loudly"
)]

use std::fmt::Write as _;
use std::path::Path;

use pdf_model::{Interpretation, Pages, interpret};
use pdf_render::{Medium, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Page one of `bytes`, interpreted.
fn page_one(bytes: Vec<u8>) -> Interpretation {
    let document = Document::open(bytes).expect("the document opens");
    let page = Pages::new(&document)
        .get(0)
        .expect("the document has a page");
    interpret(&document, &page)
}

/// How many pixels of the page, drawn on the processor at 72 dpi with no medium under it, carry
/// any coverage.
fn marked_pixels(interpretation: &Interpretation) -> usize {
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 22).expect("a bounded target");
    let raster = CpuRasterizer::new()
        .with_medium(Medium::NONE)
        .rasterize(list, target)
        .expect("the page draws");
    raster
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[3] != 0)
        .count()
}

/// A one-page document showing `shown` through `/F1`, the font object `font` with any further
/// objects `more` numbered from 6, on a 200 × 100 page with no other mark.
///
/// No cross-reference table: `Document::open` rebuilds by scanning for object headers.
fn document(shown: &str, font: &str, more: &[&str]) -> Vec<u8> {
    let content = format!("BT /F1 48 Tf 10 30 Td {shown} Tj ET\n");
    let mut out = String::from("%PDF-1.7\n");
    let _ = write!(
        out,
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n\
         5 0 obj\n{font}\nendobj\n",
        content.len(),
    );
    for (number, object) in (6..).zip(more) {
        let _ = write!(out, "{number} 0 obj\n{object}\nendobj\n");
    }
    out.push_str("trailer\n<< /Root 1 0 R >>\n%%EOF\n");
    out.into_bytes()
}

/// A stream object holding `data`.
fn stream(data: &str) -> String {
    format!("<< /Length {} >>\nstream\n{data}\nendstream", data.len())
}

/// Bytes that are not an sfnt, a bare CFF or a Type 1 program, so every reader `pdf-font` offers
/// declines them: the program is present and unreadable, §9.9's case rather than §9.8's.
const NOT_A_PROGRAM: &str = "not a font";

/// A simple `TrueType` font whose `/FontFile2` is [`NOT_A_PROGRAM`], stating `/Widths` for `A`.
fn simple_unreadable() -> Vec<u8> {
    document(
        "(AA)",
        "<< /Type /Font /Subtype /TrueType /BaseFont /Arial /FirstChar 65 /LastChar 65 \
         /Widths [667] /Encoding /WinAnsiEncoding /FontDescriptor 6 0 R >>",
        &[
            "<< /Type /FontDescriptor /FontName /Arial /Flags 32 /ItalicAngle 0 /StemV 80 \
             /FontBBox [-665 -325 2000 1006] /Ascent 905 /Descent -212 /CapHeight 716 \
             /FontFile2 7 0 R >>",
            &stream(NOT_A_PROGRAM),
        ],
    )
}

/// A Type 0 font with `/Encoding /Identity-H` over a `CIDFontType2` whose descriptor states
/// `program` (a descriptor key and its reference, or nothing), showing CIDs 36 and 37, with a
/// `/ToUnicode` naming them where `named` holds.
fn identity(program: &str, named: bool) -> Vec<u8> {
    let to_unicode = "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
                      /CMapName /Test def\n/CMapType 2 def\n\
                      1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
                      2 beginbfchar\n<0024> <0041>\n<0025> <0042>\nendbfchar\n\
                      endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend";
    let font = format!(
        "<< /Type /Font /Subtype /Type0 /BaseFont /Arial /Encoding /Identity-H \
         /DescendantFonts [6 0 R] {} >>",
        if named { "/ToUnicode 9 0 R" } else { "" }
    );
    document(
        "<00240025>",
        &font,
        &[
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Arial /CIDSystemInfo \
             << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R \
             /DW 1000 /W [36 [667 667]] >>",
            &format!(
                "<< /Type /FontDescriptor /FontName /Arial /Flags 32 /ItalicAngle 0 /StemV 80 \
                 /FontBBox [-665 -325 2000 1006] /Ascent 905 /Descent -212 /CapHeight 716 \
                 {program} >>"
            ),
            &stream(NOT_A_PROGRAM),
            &stream(to_unicode),
        ],
    )
}

/// A simple font's unreadable program draws its text from a substitute where it drew nothing.
///
/// §9.5's dictionary information is all there: `/Widths`, the descriptor's flags and metrics, and
/// `/WinAnsiEncoding`, which names `A` for code 65 (Annex D). So the page carries marks, and the
/// text readback names what the marks are.
#[test]
fn a_simple_font_whose_program_is_unreadable_draws_its_text_from_a_substitute() {
    let interpretation = page_one(simple_unreadable());
    assert!(
        marked_pixels(&interpretation) > 0,
        "the substitute draws AA where the page was blank paper"
    );
    assert_eq!(interpretation.text.trim(), "AA");
}

/// The refusal is still said, with what stood in (trap 5): the producer's shapes are not the
/// page's.
#[test]
fn the_unreadable_program_is_still_reported() {
    let interpretation = page_one(simple_unreadable());
    let said = format!("{:?}", interpretation.unsupported);
    assert!(!interpretation.is_complete(), "the page is incomplete");
    assert!(said.contains("font /F1 could not be parsed"), "{said}");
    assert!(
        said.contains("drawn from a substitute face instead"),
        "{said}"
    );
}

/// A composite font's unreadable program takes the route a composite font naming no program
/// takes: §9.7.4.2's "CIDs shall not participate in glyph selection", so its glyphs are reached
/// through the `/ToUnicode` characters, from a face this machine offers.
#[test]
fn a_composite_font_whose_program_is_unreadable_draws_through_its_to_unicode() {
    let bytes = identity("/FontFile2 8 0 R", true);
    let document = Document::open(bytes.clone()).expect("it opens");
    let font = document.get(pdf_syntax::ObjectId::new(5, 0));
    let dropped = pdf_font::without_program(&document, font.as_dict().expect("the font"))
        .expect("a descriptor to copy");
    if !pdf_font::LoadedFont::machine_offers_a_substitute(&document, &dropped) {
        eprintln!("skipped: this machine offers no face a composite font can be addressed in");
        return;
    }
    let interpretation = page_one(bytes);
    assert!(marked_pixels(&interpretation) > 0, "AB is drawn");
    assert_eq!(interpretation.text.trim(), "AB");
}

/// With no `/ToUnicode` and an `Adobe-Identity` collection, §9.7.4.2 leaves a substitute no route
/// to a glyph: the composite font draws nothing, at its `/W` widths (ADR 1094), and the report is
/// the program's refusal — not §9.7.5.2's sentence, which is about a file that embedded nothing.
#[test]
fn a_composite_font_with_no_route_to_a_substitute_still_draws_nothing() {
    let interpretation = page_one(identity("/FontFile2 8 0 R", false));
    assert_eq!(marked_pixels(&interpretation), 0);
    let said = format!("{:?}", interpretation.unsupported);
    assert!(said.contains("could not be parsed"), "{said}");
    assert!(!said.contains("§9.7.5.2"), "{said}");
}

/// §9.7.5.2's forbidden combination keeps its treatment: `/Identity-H` over a descendant naming no
/// program, with nothing to reach a substitute by, is refused by that clause's sentence and draws
/// nothing.
#[test]
fn the_forbidden_identity_combination_is_unchanged() {
    let interpretation = page_one(identity("", false));
    assert_eq!(marked_pixels(&interpretation), 0);
    let said = format!("{:?}", interpretation.unsupported);
    assert!(said.contains("§9.7.5.2 says shall not be used"), "{said}");
}

/// The premise of [`the_witnesses_draw_their_text`], asked of the machine before a page is drawn:
/// the name of a composite font on one of the witness's pages for which, its program dropped,
/// this machine offers no face. A composite font is reached by character only (§9.7.4.2), so
/// its substitute is a face the machine has installed and never a compiled-in one; where there is
/// none the page draws no text, which is a fact about the machine and not about the code under
/// test (ADR 3130). Asked through `pdf-font`'s own search, so there is no second matcher.
fn composite_without_a_machine_face(document: &Document, pages: &Pages<'_>) -> Option<String> {
    (0..pages.len())
        .filter_map(|index| pages.get(index))
        .find_map(|page| {
            let fonts = document.get_key(&page.resources, "Font");
            fonts.as_dict()?.iter().find_map(|(name, font)| {
                let font = document.resolve(font);
                let dropped = pdf_font::without_program(document, font.as_dict()?)?;
                (!pdf_font::LoadedFont::machine_offers_a_substitute(document, &dropped))
                    .then(|| String::from_utf8_lossy(name.as_bytes()).into_owned())
            })
        })
}

/// The corpus witnesses `doc/questions/A837` names: every page on which the window now draws an
/// unreadable program from a substitute reads text back, where it drew none. Read from the optional
/// checkouts, and said when absent.
#[test]
fn the_witnesses_draw_their_text() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc");
    for (path, expected) in [
        ("pdf.js/test/pdfs/issue11651.pdf", "Hello"),
        (
            "corpora/format-corpus/govdocs1-error-pdfs/error_set_2/178360.pdf",
            "",
        ),
        (
            "corpora/format-corpus/govdocs1-error-pdfs/error_set_2/435321.pdf",
            "",
        ),
        (
            "corpora/format-corpus/govdocs1-error-pdfs/error_set_2/268346.pdf",
            "",
        ),
    ] {
        let Ok(bytes) = std::fs::read(root.join(path)) else {
            eprintln!("skipped: {path} is not checked out");
            continue;
        };
        let document = Document::open(bytes).expect("the witness opens");
        let pages = Pages::new(&document);
        if let Some(font) = composite_without_a_machine_face(&document, &pages) {
            eprintln!(
                "skipped: {path}: this machine offers no face composite font /{font} can be drawn \
                 from without its program"
            );
            continue;
        }
        let mut substituted = 0;
        for index in 0..pages.len() {
            let page = pages.get(index).expect("a page the tree counts");
            let interpretation = interpret(&document, &page);
            let said = format!("{:?}", interpretation.unsupported);
            if !said.contains("drawn from a substitute face instead") {
                continue;
            }
            substituted += 1;
            assert!(
                interpretation.text.contains(expected) && !interpretation.text.trim().is_empty(),
                "{path} page {}: {:?}",
                index + 1,
                interpretation.text
            );
        }
        assert!(substituted > 0, "{path}: no page draws a substitute");
    }
}

//! A code of a substituted font that what the file states takes to no glyph is reported per code.
//!
//! A substitute is no program the document named, so ISO 32000-2's routes reach it only through
//! what a code means. For a simple font that is the glyph name the encoding gives the code
//! (§9.6.5.1), which §9.6.5.4 maps through the Adobe Glyph List to a character and, where "the
//! glyph name cannot be mapped as specified", looks up in the face's `post` table; a bare CFF face
//! keys its glyphs by the name itself (§9.6.5.2). `/ToUnicode` selects no glyph of a simple font:
//! §9.10.2 states it for extracting text. Where the encoding names a code nothing, or names it with
//! a name no route reaches, no face on any machine draws it, and the window says so for those
//! codes, whatever else the font drew — the font's own report says it is drawn from a substitute,
//! and that sentence has to stay true (ADR 2880). A character the machine's face lacks is a
//! different statement, about the machine, and keeps ADR 0152's condition.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that stops opening should fail loudly"
)]

use std::fmt::Write as _;
use std::path::Path;

use pdf_model::{Interpretation, Pages, interpret};
use pdf_syntax::Document;

/// Page one of `bytes`, interpreted.
fn page_one(bytes: Vec<u8>) -> Interpretation {
    let document = Document::open(bytes).expect("the document opens");
    let page = Pages::new(&document)
        .get(0)
        .expect("the document has a page");
    interpret(&document, &page)
}

/// Every report the interpretation carries, as one string.
fn reports(interpretation: &Interpretation) -> String {
    format!("{:?}", interpretation.unsupported)
}

/// A one-page document showing `shown` through `/F1`, the font object `font` with any further
/// objects `more` numbered from 6. No cross-reference table: `Document::open` rebuilds by
/// scanning for object headers.
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

/// A `/ToUnicode` stream mapping each single-byte code of `pairs` to its character.
fn to_unicode(pairs: &[(u8, char)]) -> String {
    let mut lines = String::new();
    for (code, character) in pairs {
        let _ = writeln!(lines, "<{code:02X}> <{:04X}>", u32::from(*character));
    }
    let data = format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CMapName /F1 def\n/CMapType 2 def\n1 begincodespacerange\n<00> <FF>\n\
         endcodespacerange\n{} beginbfchar\n{lines}endbfchar\nendcmap\n\
         CMapName currentdict /CMap defineresource pop\nend\nend",
        pairs.len()
    );
    format!("<< /Length {} >>\nstream\n{data}\nendstream", data.len())
}

/// `/Helvetica` names no program, so the compiled-in face stands in whatever this machine holds,
/// and that face keys its glyphs by name (§9.6.5.2): `g100` is in no Adobe Glyph List and in no
/// charset of it. Code 65 is named `g100` by the `/Differences`, code 66 keeps
/// `StandardEncoding`'s `B`; the string shows 65 twice and 66 once, so two codes are lost.
#[test]
fn a_glyph_name_no_route_reaches_is_reported_for_its_codes() {
    let interpretation = page_one(document(
        "(ABA)",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding << /Differences [65 /g100] >> /ToUnicode 6 0 R >>",
        &[&to_unicode(&[(65, '\u{4E00}'), (66, 'B')])],
    ));
    let said = reports(&interpretation);
    assert!(
        said.contains("font /F1 is substituted, and 2 of the code(s) the page shows through it")
            && said.contains("the glyph name /g100,")
            && said.contains("§9.6.5.4"),
        "the two codes /g100 selects are said, and the clause whose route ended: {said}"
    );
    assert!(
        !interpretation.display_list.commands().is_empty(),
        "`B` is drawn, so the font did draw: the report is per code, not per font"
    );
}

/// A code `StandardEncoding` names nothing — code 1 — and no `/Differences` names either is a code
/// the encoding gives no glyph name (§9.6.5.1's table "that maps from character codes to glyph
/// names" has no entry), so no face is asked for anything.
#[test]
fn a_code_the_encoding_names_nothing_is_reported() {
    let interpretation = page_one(document(
        "<0142>",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 6 0 R >>",
        &[&to_unicode(&[(1, 'X'), (66, 'B')])],
    ));
    let said = reports(&interpretation);
    assert!(
        said.contains("font /F1 is substituted, and 1 of the code(s)")
            && said.contains("is given no glyph name by the font's encoding"),
        "code 1 is said: {said}"
    );
}

/// The control on the condition: a code that reads back as a space is meant to draw nothing, and
/// a name the Adobe Glyph List maps is the machine's face's matter (ADR 0152), so neither is said.
#[test]
fn a_space_and_a_character_the_face_lacks_are_not_said_per_code() {
    let interpretation = page_one(document(
        "(AB)",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding << /Differences [65 /g100 /afii10017] >> /ToUnicode 6 0 R >>",
        &[&to_unicode(&[(65, ' '), (66, '\u{0410}')])],
    ));
    let said = reports(&interpretation);
    assert!(
        !said.contains("is substituted, and"),
        "a space, and a Cyrillic character the Glyph List names, are not this report's: {said}"
    );
}

/// The witness: `issue13316_reduced.pdf`'s `/F1` (`YouYuan`) has a `/FontFile2` this reader
/// refuses, so the window draws it from a substitute (ADR 2870), and its `/Differences` names
/// codes 65 and 67 to 70 `/g5167`, `/g11927`, `/g17737`, `/g11540` and `/g2180` — no Adobe Glyph
/// List name — and 66 `/space`. Its one string `ABCBDBEBF` shows five codes no route reaches and
/// four spaces. Its `/ToUnicode` decodes (65 is U+5F00), and selects nothing (§9.10.2).
#[test]
fn the_witness_says_which_codes_its_substitute_cannot_draw() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs/issue13316_reduced.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    let interpretation = page_one(bytes);
    let said = reports(&interpretation);
    assert!(
        said.contains("drawn from a substitute face instead"),
        "the program's refusal is still said: {said}"
    );
    assert!(
        said.contains("font /F1 is substituted, and 5 of the code(s) the page shows through it")
            && said.contains("the first selects the glyph name /g5167,"),
        "and so are the five codes the substitute cannot be asked for: {said}"
    );
}

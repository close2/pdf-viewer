//! Audit 2180 of ISO 32000-2 §9.5 to §9.10 against the Type 3 font reader: a bound has two sides.
//!
//! §9.6.4's Table 110, in its `/Widths` row, bounds the array by both of the font's range entries —
//!
//! > An array of ( LastChar - FirstChar + 1) numbers, each element being the glyph width for the
//! > character code that equals FirstChar plus the array index. For character codes outside the
//! > range FirstChar to LastChar , the width shall be 0.
//!
//! — so an array longer than the range states nothing about the codes past `/LastChar`, and each
//! of them advances by 0. The control is the lower side of the same range, which holds on main.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixtures are deliberately small; malformed fixtures should fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::type3::Type3Font;
use pdf_syntax::{Document, ObjectId};

/// A document whose object 1 is a Type 3 font with `/FirstChar 97 /LastChar 97` and the
/// `/Widths` array given, its encoding naming codes 96 to 98 `/z`, `/a` and `/b`.
fn font(widths: &str) -> (Document, pdf_syntax::Dictionary) {
    const GLYPH: &str = "1000 0 d0\n0 0 750 750 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 750 750] \
         /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 2 0 R \
         /Encoding << /Type /Encoding /Differences [96 /z /a /b] >> \
         /FirstChar 97 /LastChar 97 /Widths {widths} >>\nendobj\n\
         2 0 obj\n<< /z 3 0 R /a 3 0 R /b 3 0 R >>\nendobj\n\
         3 0 obj\n<< /Length {} >>\nstream\n{GLYPH}\nendstream\nendobj\n",
        GLYPH.len() + 1,
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
    let document = Document::open(out.into_bytes()).expect("the fixture is a valid PDF");
    assert!(!document.was_recovered(), "the fixture has a complete xref");
    let dict = document
        .get(ObjectId {
            number: 1,
            generation: 0,
        })
        .as_dict()
        .expect("object 1 is the font dictionary")
        .clone();
    (document, dict)
}

#[test]
fn a_type3_code_past_last_char_advances_by_zero_spec_audit() {
    // Control: the range's lower side. Code 96 is below `/FirstChar` and advances by 0, and
    // code 97 advances by its one element through `/FontMatrix`: 1000 × 0.001.
    let (document, dict) = font("[1000]");
    let exact = Type3Font::read(&document, &dict, "T3").expect("the Type 3 font reads");
    assert!(
        (exact.advance(97) - 1.0).abs() < 1e-6,
        "control: code 97 is in the range"
    );
    assert!(
        exact.advance(96).abs() < 1e-6,
        "control: below /FirstChar the width is 0"
    );
    assert!(
        exact.advance(98).abs() < 1e-6,
        "control: past the array the width is 0"
    );

    // The upper side. The array carries a second element, which the range does not reach:
    // code 98 is outside `/FirstChar` to `/LastChar`, and Table 110 gives it 0.
    let (document, dict) = font("[1000 1000]");
    let long = Type3Font::read(&document, &dict, "T3").expect("the Type 3 font reads");
    assert!(
        (long.advance(97) - 1.0).abs() < 1e-6,
        "code 97 is in the range"
    );
    assert!(
        long.advance(98).abs() < 1e-6,
        "code 98 is past /LastChar 97, so Table 110 gives it width 0; it advanced by {}",
        long.advance(98)
    );
}

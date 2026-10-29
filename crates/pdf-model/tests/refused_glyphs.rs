//! A glyph whose charstring the evaluator refuses is named in the page's report, with the font,
//! the glyph and the evaluator's own reason, and is drawn as nothing exactly as before (ADR 2586).
//!
//! The program is a bare CFF under `/FontFile3` `/Type1C` (ISO 32000-2 §9.9, Table 124) whose
//! glyph draws a triangle and then calls the first of ten global subroutines, each calling the
//! next `k` times: Adobe Technical Note #5177's ten levels of nesting, entered about `k`⁹ times.
//! At `k` = 3 that is well inside `pdf_font::cff::CHARSTRING_OPERATIONS` and the glyph is drawn;
//! at `k` = 5 it is past it, and the evaluator stops (ADR 2572).

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and its sums are offsets inside a \
              program of a few hundred bytes"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;

/// A CFF INDEX with four-byte offsets.
fn index(items: &[Vec<u8>]) -> Vec<u8> {
    let mut out = u16::try_from(items.len())
        .expect("a fixture INDEX is small")
        .to_be_bytes()
        .to_vec();
    if items.is_empty() {
        return out;
    }
    out.push(4);
    let mut offset = 1u32;
    out.extend_from_slice(&offset.to_be_bytes());
    for item in items {
        offset = offset
            .checked_add(u32::try_from(item.len()).expect("a fixture item is small"))
            .expect("a fixture INDEX is small");
        out.extend_from_slice(&offset.to_be_bytes());
    }
    for item in items {
        out.extend_from_slice(item);
    }
    out
}

/// A bare CFF of three glyphs under the predefined `ISOAdobe` charset — `.notdef`, `space` and
/// `exclam` — whose `exclam` draws a triangle and calls a fan-out of `k` calls at ten levels.
fn hostile_cff(k: usize) -> Vec<u8> {
    const CALLGSUBR: u8 = 29;
    const RETURN: u8 = 11;
    const ENDCHAR: u8 = 14;
    const LEVELS: u8 = 10;
    // Fewer than 1240 subroutines: the bias is 107, and a one-byte operand `v` is written as
    // `v + 139`, so subroutine `i` is named by the byte `i + 32`.
    let subrs = (0..LEVELS)
        .map(|level| {
            let mut subr = Vec::new();
            if level + 1 < LEVELS {
                for _ in 0..k {
                    subr.extend([level + 33, CALLGSUBR]);
                }
            }
            subr.push(RETURN);
            subr
        })
        .collect::<Vec<_>>();
    // `0 0 rmoveto 100 0 rlineto 0 100 rlineto`, then subroutine 0, then `endchar`.
    let exclam = vec![
        139, 139, 21, 239, 139, 5, 139, 239, 5, 32, CALLGSUBR, ENDCHAR,
    ];
    let glyphs = index(&[vec![ENDCHAR], vec![ENDCHAR], exclam]);

    let name = index(&[b"Hostile".to_vec()]);
    let strings = index(&[]);
    let global = index(&subrs);
    // The Top DICT is one item of `29 <offset> 17`: a count, an offset size, two offsets and six
    // bytes, after the four-byte header and the Name INDEX.
    let charstrings_at = 4 + name.len() + 2 + 1 + 2 * 4 + 6 + strings.len() + global.len();
    let mut top = vec![29];
    top.extend_from_slice(
        &i32::try_from(charstrings_at)
            .expect("a fixture is small")
            .to_be_bytes(),
    );
    top.push(17);
    let mut out = vec![1, 0, 4, 4];
    out.extend_from_slice(&name);
    out.extend_from_slice(&index(&[top]));
    out.extend_from_slice(&strings);
    out.extend_from_slice(&global);
    assert_eq!(
        out.len(),
        charstrings_at,
        "the Top DICT's offset is the INDEX's"
    );
    out.extend_from_slice(&glyphs);
    out
}

/// One page showing `!` through `/F1`, a simple Type 1 font embedding [`hostile_cff`]`(k)`.
fn page(k: usize) -> Document {
    let program = hostile_cff(k);
    let content = "BT /F1 48 Tf 100 100 Td (!) Tj ET";
    let objects: Vec<Vec<u8>> = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] \
          /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
            content.len()
        )
        .into_bytes(),
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Hostile /FirstChar 33 \
          /LastChar 33 /Widths [500] /FontDescriptor 6 0 R >>\nendobj\n"
            .to_vec(),
        b"6 0 obj\n<< /Type /FontDescriptor /FontName /Hostile /Flags 32 \
          /FontBBox [0 0 1000 1000] /ItalicAngle 0 /Ascent 800 /Descent -200 \
          /CapHeight 700 /StemV 80 /FontFile3 7 0 R >>\nendobj\n"
            .to_vec(),
        [
            format!(
                "7 0 obj\n<< /Subtype /Type1C /Length {} >>\nstream\n",
                program.len()
            )
            .into_bytes(),
            program,
            b"\nendstream\nendobj\n".to_vec(),
        ]
        .concat(),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in &objects {
        offsets.push(out.len());
        out.extend_from_slice(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let mut trailer = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(trailer.as_bytes());
    Document::open(out).expect("the fixture opens")
}

/// Page one's reports, as the corpus gate prints them, and how many commands it drew.
fn interpreted(document: &Document) -> (String, usize) {
    let page = pdf_model::Pages::new(document)
        .get(0)
        .expect("the fixture has a page");
    let interpretation = pdf_model::interpret(document, &page);
    (
        format!("{:?}", interpretation.unsupported),
        interpretation.display_list.commands().len(),
    )
}

#[test]
fn a_glyph_past_the_operation_budget_is_named_with_the_evaluators_reason() {
    let (reported, _) = interpreted(&page(5));
    assert!(
        reported.contains(
            "font /F1: the evaluator refuses the outline of glyph 2 (the CFF font program could \
             not be read: "
        ),
        "the refusal names the font, the glyph and the evaluator's reason: {reported}"
    );
    assert!(
        reported.contains("and 1 of the code(s) the page shows through it reach a refused glyph"),
        "and counts the codes it cost: {reported}"
    );
    // The page draws nothing through `/F1` and the refusal is the reason, so the report that a
    // program describes no outline for any code does not also fire.
    assert!(
        !reported.contains("has no outline for any"),
        "a refused glyph is not one the program describes empty: {reported}"
    );
}

#[test]
fn the_same_glyph_inside_the_budget_is_drawn_and_reports_nothing() {
    let (refused, refused_commands) = interpreted(&page(5));
    let (drawn, drawn_commands) = interpreted(&page(3));
    assert!(
        !drawn.contains("evaluator refuses"),
        "a glyph inside the budget is drawn: {drawn}"
    );
    assert!(
        !drawn.contains("font /F1"),
        "and the page says nothing about its font: {drawn}"
    );
    // A refused glyph is drawn as nothing, as it was before it was reported.
    assert!(
        refused_commands < drawn_commands,
        "the refused glyph puts no mark on the page ({refused_commands} commands against \
         {drawn_commands}): {refused}"
    );
}

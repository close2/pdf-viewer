//! These tests use an embedded Type 3 glyph so
//! their placement oracles do not depend on installed fonts or corpus availability.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "standalone test fixture must fail loudly and coordinates are bounded"
)]

use std::fmt::Write as _;

use pdf_render::Command;
use pdf_syntax::Document;

const GLYPH: &str = "1000 0 0 0 750 750 d1\n0 0 750 750 re f";
const SPACE: &str = "250 0 d0";

/// A one-page PDF 2.0 fixture with one embedded Type 3 square glyph (`A`, width 1000)
/// and one nonpainting space (width 250). `FontMatrix` .001 makes glyph and declared advance
/// dimensions explicit and machine-independent.
fn fixture(content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 750 750] \
         /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 7 0 R /Encoding 6 0 R \
         /FirstChar 32 /LastChar 65 /Widths [{}] /Resources << >> >>\nendobj\n\
         6 0 obj\n<< /Type /Encoding /Differences [32 /space 65 /A] >>\nendobj\n\
         7 0 obj\n<< /space 8 0 R /A 9 0 R >>\nendobj\n\
         8 0 obj\n<< /Length {} >>\nstream\n{SPACE}\nendstream\nendobj\n\
         9 0 obj\n<< /Length {} >>\nstream\n{GLYPH}\nendstream\nendobj\n",
        content.len() + 1,
        (0..34)
            .map(|i| if i == 0 {
                "250"
            } else if i == 33 {
                "1000"
            } else {
                "0"
            })
            .collect::<Vec<_>>()
            .join(" "),
        SPACE.len() + 1,
        GLYPH.len() + 1,
    );
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899aabbccddeeff> <00112233445566778899aabbccddeeff>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn placements(content: &str) -> Vec<pdf_render::Transform> {
    let document = Document::open(fixture(content)).expect("well-formed PDF fixture");
    assert!(!document.was_recovered(), "fixture has a complete xref");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpreted = pdf_model::interpret(&document, &page);
    assert!(
        interpreted.unsupported.is_empty(),
        "{:?}",
        interpreted.unsupported
    );
    let positions: Vec<_> = interpreted
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill { transform, .. } | Command::Stroke { transform, .. } => Some(*transform),
            _ => None,
        })
        .collect();
    assert!(!positions.is_empty(), "fixture must paint its A glyphs");
    positions
}

/// A positive TJ number is subtracted in thousandths of text space. With a 1000-unit
/// Type 3 advance at 20 points, a 250 adjustment shortens the next origin by exactly 5.
#[test]
fn tj_positive_number_moves_left_by_thousandths_of_text_space() {
    let plain = placements("BT /F1 20 Tf 0 0 Td [(A) (A)] TJ ET");
    let adjusted = placements("BT /F1 20 Tf 0 0 Td [(A) 250 (A)] TJ ET");
    assert_eq!(plain.len(), 2);
    assert_eq!(adjusted.len(), 2);
    assert!(
        (plain[1].e - 20.0).abs() < 0.001,
        "plain second glyph at {}",
        plain[1].e
    );
    assert!(
        (adjusted[1].e - 15.0).abs() < 0.001,
        "positive 250 must subtract 5, got {}",
        adjusted[1].e
    );
}

/// T* starts from Tlm rather than the text matrix left after showing the previous glyph.
/// The second Td changes the line origin; glyph advances then move only Tm. T* must return
/// to x=30 and lower y by 12 from that saved line origin.
#[test]
fn tstar_returns_to_line_matrix_after_text_advance() {
    let placed = placements("BT /F1 10 Tf 12 TL 20 50 Td (A) Tj 10 0 Td (A) Tj T* (A) Tj ET");
    assert_eq!(placed.len(), 3);
    assert!((placed[0].e - 20.0).abs() < 0.001 && (placed[0].f - 50.0).abs() < 0.001);
    assert!((placed[1].e - 30.0).abs() < 0.001 && (placed[1].f - 50.0).abs() < 0.001);
    assert!(
        (placed[2].e - 30.0).abs() < 0.001 && (placed[2].f - 38.0).abs() < 0.001,
        "T* must use Tlm x=30, y=38; got ({}, {})",
        placed[2].e,
        placed[2].f
    );
}

/// Tm replaces the preceding translation and line origin, then its rotation is composed
/// with the known font size. A concatenating implementation would retain the old (20,30).
#[test]
fn tm_replaces_current_matrix_and_applies_rotation() {
    let placed = placements("BT /F1 10 Tf 20 30 Td 0 1 -1 0 40 50 Tm (A) Tj ET");
    assert_eq!(placed.len(), 1);
    let matrix = placed[0];
    assert!(
        (matrix.e - 40.0).abs() < 0.001 && (matrix.f - 50.0).abs() < 0.001,
        "Tm must replace translation, got ({}, {})",
        matrix.e,
        matrix.f
    );
    // Type 3 commands retain the CharProc's glyph-space coordinate system; the active
    // transform therefore includes FontMatrix (.001) × text size (10), yielding .01.
    assert!(
        matrix.a.abs() < 0.001
            && (matrix.b - 0.01).abs() < 0.001
            && (matrix.c + 0.01).abs() < 0.001
            && matrix.d.abs() < 0.001,
        "rotated 10-point glyph matrix was [{}, {}, {}, {}]",
        matrix.a,
        matrix.b,
        matrix.c,
        matrix.d
    );
}

/// The quote operator's first operand is word spacing and second is character spacing;
/// its effect matches the equivalent explicit state updates, T*, and Tj.
#[test]
fn double_quote_sets_spacing_then_moves_line_and_shows() {
    let quoted = placements("BT /F1 10 Tf 15 TL 20 50 Td 12 5 ( A) \" ET");
    let explicit = placements("BT /F1 10 Tf 15 TL 20 50 Td 12 Tw 5 Tc T* ( A) Tj ET");
    assert_eq!(quoted, explicit);
    assert_eq!(quoted.len(), 1);
    // space advance 2.5 + Tw 12 + Tc 5, at the new baseline y=35.
    assert!(
        (quoted[0].e - 39.5).abs() < 0.001 && (quoted[0].f - 35.0).abs() < 0.001,
        "A after quoted space must be at (39.5,35), got ({},{})",
        quoted[0].e,
        quoted[0].f
    );
}

/// EC3 #368 is in §9.4.1. A show after Q observes Tm; T* is needed to observe Tlm.
#[test]
fn q_restore_recovers_the_line_matrix_used_by_the_next_line_operator() {
    let saved = placements("BT /F1 10 Tf 12 TL 20 50 Td (A) Tj q 100 100 Td (A) Tj Q T* (A) Tj ET");
    let control = placements("BT /F1 10 Tf 12 TL 20 50 Td (A) Tj 100 100 Td (A) Tj T* (A) Tj ET");
    assert_eq!(saved.len(), 3);
    assert_eq!(control.len(), 3);
    assert!((control[2].e - 120.0).abs() < 0.001 && (control[2].f - 138.0).abs() < 0.001);
    assert!(
        (saved[1].e - 120.0).abs() < 0.001 && (saved[1].f - 150.0).abs() < 0.001,
        "saved scope really moved the line"
    );
    assert!(
        (saved[2].e - 20.0).abs() < 0.001 && (saved[2].f - 38.0).abs() < 0.001,
        "T* must use the restored line origin, got {:?}",
        saved[2]
    );
}

//! Undefined path painting recovery, ISO 32000-2 §8.5.3.1, EC3 Issue #549.
//!
//! The EC3 erratum replaces “shall generate an error” with “shall be ignored”. The fixture
//! exercises an undefined path at page start and again after a valid painting operator, then
//! confirms a later path still paints normally.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "a malformed fixture must fail loudly; the raster-free assertion indexes no data"
)]

use std::fmt::Write as _;

use pdf_render::Command;
use pdf_syntax::Document;

fn fixture(content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len() + 1
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

#[test]
fn undefined_painting_operators_are_ignored_and_do_not_consume_the_next_path() {
    let document =
        Document::open(fixture("S f 10 10 m 30 10 l 30 30 l f S")).expect("fixture is valid");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let commands = interpretation.display_list.commands();
    let clean_document =
        Document::open(fixture("10 10 m 30 10 l 30 30 l f")).expect("clean control is valid");
    let clean_page = pdf_model::Pages::new(&clean_document)
        .get(0)
        .expect("control page one");
    let clean_interpretation = pdf_model::interpret(&clean_document, &clean_page);
    let clean_commands = clean_interpretation.display_list.commands();

    assert_eq!(
        commands, clean_commands,
        "undefined path-painting operators leave the later valid path and state unchanged"
    );

    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, Command::Fill { .. }))
            .count(),
        1,
        "leading undefined f is ignored; the later valid fill remains"
    );
    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, Command::Stroke { .. }))
            .count(),
        0,
        "undefined S at stream start and after fill add no marks"
    );
    assert!(
        interpretation.unsupported.is_empty(),
        "ignored is not reported as an error"
    );
}

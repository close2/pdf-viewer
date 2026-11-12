//! `Query::Text`: the text of the pages on the screen crosses the boundary as a question, the
//! readback ISO 32000-2 §9.10.1 says a processor needs "during operations such as searching,
//! indexing, and exporting of text to other file formats" (`doc/todo/63`, ADR 2836).
//!
//! The expected characters are derived from the standard, not from any extractor. The font is a
//! simple font whose `/Encoding` is `/WinAnsiEncoding`, one of §9.6.5's "predefined encodings",
//! and Annex D.2's table puts the glyph name `eacute` at that encoding's code octal 351. §9.10.2's
//! second method then names the character: "If the font is a simple font and the glyph selection
//! algorithm (see 9.6.5, "Character encoding") uses a glyph name, that name can be looked up in
//! the Adobe Glyph List and Adobe Glyph List for New Fonts to obtain the corresponding Unicode
//! value", which for `eacute` is U+00E9.

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);

/// One page showing `Caf` and the `WinAnsiEncoding` code octal 351 in Helvetica.
fn one_page() -> Vec<u8> {
    let content = "BT /F1 12 Tf 20 40 Td (Caf\\351) Tj ET";
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len().saturating_add(1)
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = bodies.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Nothing open is `Answer::None`; an open page answers with its own index and §9.10.2's
/// characters, and they are the characters `Query::Find` searches — one readback, not two.
#[test]
fn the_text_of_the_page_on_the_screen_crosses_as_its_characters() {
    let mut viewer = Viewer::new(400, 300, 1.0);
    assert!(
        matches!(viewer.query(Query::Text), Answer::None),
        "with no document open there is no answer, which is not an answer of no text"
    );

    let _ = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: one_page().into(),
            password: None,
            fragment: None,
        })
        .count();

    let Answer::Text(pages) = viewer.query(Query::Text) else {
        panic!("a page that was interpreted answers with its text");
    };
    let [page] = pages.as_slice() else {
        panic!("one page is on the screen and it has been read: {pages:?}");
    };
    assert_eq!(page.page, 0, "the entry says which page it is the text of");
    assert_eq!(
        page.text.trim_end(),
        "Caf\u{e9}",
        "WinAnsi octal 351 is Annex D.2's eacute, U+00E9"
    );

    let Answer::Found(found) = viewer.query(Query::Find("Caf\u{e9}")) else {
        panic!("a search of the page answers with its matches");
    };
    assert_eq!(
        found.len(),
        1,
        "what a program reads of the page is what a person finds on it"
    );
}

//! A PDF whose one object is a font dictionary, for stating a rule about a dictionary.
//!
//! Several rules in clause 9 are about what a font *dictionary* says, and reach a glyph
//! only afterwards. Building a font program to test one would test the program's reader
//! instead, so the fixtures that need only a dictionary build only a dictionary.

use std::fmt::Write as _;

use pdf_syntax::{Dictionary, Document, ObjectId};

/// A one-object document whose object 1 is a font dictionary with the given entries.
pub(crate) fn font_dictionary(entries: &str) -> (Document, Dictionary) {
    let body = format!("1 0 obj\n<< /Type /Font /Subtype /Type1 {entries} >>\nendobj\n");
    let mut out = String::from("%PDF-1.7\n");
    let offset = out.len();
    out.push_str(&body);
    let xref_at = out.len();
    out.push_str("xref\n0 2\n0000000000 65535 f \n");
    let _ = writeln!(out, "{offset:010} 00000 n ");
    let _ = write!(
        out,
        "trailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    let document = Document::open(out.into_bytes()).expect("the fixture is a valid PDF");
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

/// The same, with object 2 a stream the descriptor points at as `/FontFile2`.
///
/// The bytes are deliberately not a font: what the callers need is a *deterministic*
/// failure that every simple `/Subtype` reaches identically, and a program `skrifa`
/// refuses gives one, where an absent program would send the load into substitution
/// and make the test depend on which faces this machine has installed.
pub(crate) fn font_with_program(subtype: &str, program: &[u8]) -> (Document, Dictionary) {
    let mut body = format!(
        "1 0 obj\n<< /Type /Font /Subtype /{subtype} \
         /FontDescriptor << /Flags 32 /FontFile2 2 0 R >> >>\nendobj\n"
    );
    let _ = write!(
        body,
        "2 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        program.len(),
        String::from_utf8_lossy(program)
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
    let document = Document::open(out.into_bytes()).expect("the fixture is a valid PDF");
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

/// The same shape with the program carried byte for byte, for a program that has to *work*.
///
/// [`font_with_program`] routes the bytes through a `String`, which is fine for its callers —
/// they want a deterministic refusal — and destroys any real font program, whose tables are
/// not UTF-8. This builds the whole file as bytes, so a test can state a working `glyf` and
/// watch it draw. The descriptor sets Table 121's Symbolic flag (bit 3 of the descriptor's `/Flags`),
/// which is what lets a fixture's own `(1, 0)` `cmap` subtable answer for its codes without
/// this machine's encodings in the room.
pub(crate) fn symbolic_font_with_binary_program(
    subtype: &str,
    program: &[u8],
) -> (Document, Dictionary) {
    let mut body: Vec<u8> = Vec::new();
    let mut offsets = Vec::new();

    offsets.push(body.len());
    body.extend_from_slice(
        format!(
            "1 0 obj\n<< /Type /Font /Subtype /{subtype} \
             /FontDescriptor << /Flags 4 /FontFile2 2 0 R >> >>\nendobj\n"
        )
        .as_bytes(),
    );
    offsets.push(body.len());
    body.extend_from_slice(
        format!("2 0 obj\n<< /Length {} >>\nstream\n", program.len()).as_bytes(),
    );
    body.extend_from_slice(program);
    body.extend_from_slice(b"\nendstream\nendobj\n");

    let mut out: Vec<u8> = b"%PDF-1.7\n".to_vec();
    let base = out.len();
    out.extend_from_slice(&body);
    let xref_at = out.len();
    let mut trailer = String::from("xref\n0 3\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(trailer, "{:010} 00000 n ", base.saturating_add(*offset));
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(trailer.as_bytes());

    let document = Document::open(out).expect("the fixture is a valid PDF");
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

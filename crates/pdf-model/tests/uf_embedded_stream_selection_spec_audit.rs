//! §7.11.3/Table 43 public-consumer probe: UF wins over F, including its corresponding EF stream.
//!
//! The source permits both filename forms and says the reader shall use UF instead of F.
//! Each EF entry contains its matching file's distinct bytes. This test checks selection through
//! the public attachment list, not host extraction or filesystem behavior.

#![expect(
    clippy::expect_used,
    reason = "fixture construction and expected attachment entries"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "fixture object numbers are generated from a slice"
)]

use std::fmt::Write as _;

use pdf_model::attachment;
use pdf_syntax::Document;

fn document(objects: &[&str]) -> Document {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    );
    Document::open(out.into_bytes()).expect("strict-xref PDF fixture")
}

fn embedded(payload: &[u8]) -> String {
    let text = std::str::from_utf8(payload).expect("ASCII fixture payload");
    format!(
        "<< /Type /EmbeddedFile /Subtype /application#2Foctet-stream /Length {} \
         /Params << /Size {} /ModDate (D:20260102030405Z) >> >>\nstream\n{}\nendstream",
        payload.len(),
        payload.len(),
        text
    )
}

#[test]
fn unicode_filename_selects_its_corresponding_embedded_stream_with_fallback_control() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles 3 0 R >> >>",
        "<< /Type /Pages /Count 1 /Kids [4 0 R] >>",
        "<< /Names [(fallback) 5 0 R (preferred) 6 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        "<< /Type /Filespec /F (fallback.bin) /Desc (fallback control) /EF << /F 7 0 R >> >>",
        "<< /Type /Filespec /F (legacy.bin) /UF <FEFF0075006E00690063006F00640065002E00620069006E> /Desc (preferred target) \
         /EF << /F 8 0 R /UF 9 0 R >> >>",
        &embedded(b"fallback-payload"),
        &embedded(b"legacy-payload"),
        &embedded(b"unicode-payload"),
    ]);
    assert!(
        !doc.was_recovered(),
        "xref entries address the exact fixture objects"
    );

    let files = attachment::attachments(&doc);
    let fallback = files
        .iter()
        .find(|file| file.name == "fallback")
        .expect("F-only control");
    let preferred = files
        .iter()
        .find(|file| file.name == "preferred")
        .expect("F/UF target");
    assert_eq!(fallback.file_name.as_deref(), Some("fallback.bin"));
    assert_eq!(
        doc.decoded_stream_data(&fallback.stream).as_deref(),
        Some(&b"fallback-payload"[..])
    );
    assert_eq!(preferred.file_name.as_deref(), Some("unicode.bin"));
    assert_eq!(
        doc.decoded_stream_data(&preferred.stream).as_deref(),
        Some(&b"unicode-payload"[..]),
        "Table 43: use UF instead of F and follow the corresponding EF/UF stream"
    );
}

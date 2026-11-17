//! Current Table 409 child references and `DPart` associated-file traversal.
//! The catalog control has a separate payload; page back-pointers and `DPart` ranges agree.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "small exact-xref test fixture"
)]

use std::fmt::Write as _;

use pdf_model::attachment::{self, Relationship};
use pdf_syntax::Document;

fn fixture() -> Vec<u8> {
    // The fixture follows the Params recommendation and its conditional ModDate requirement.
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /DPartRoot 4 0 R /AF [11 0 R] >>",
        "<< /Type /Pages /Kids [3 0 R 9 0 R] /Count 2 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /DPart 6 0 R >>",
        "<< /Type /DPartRoot /DPartRootNode 5 0 R /RecordLevel 0 /NodeNameList [/Document /Chapter] >>",
        "<< /Type /DPart /Parent 4 0 R /DParts [6 0 R 7 0 R] >>",
        "<< /Type /DPart /Parent 5 0 R /Start 3 0 R /AF [8 0 R] >>",
        "<< /Type /DPart /Parent 5 0 R /Start 9 0 R /DPM << /TicketID (part-two) >> /AF [10 0 R] >>",
        "<< /Type /Filespec /F (first-part.txt) /UF (first-part.txt) /AFRelationship /Data /EF << /F 12 0 R >> >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /DPart 7 0 R >>",
        "<< /Type /Filespec /F (second-part.txt) /UF (second-part.txt) /AFRelationship /Supplement /EF << /F 13 0 R >> >>",
        "<< /Type /Filespec /F (catalog-control.txt) /UF (catalog-control.txt) /AFRelationship /Source /EF << /F 14 0 R >> >>",
        "<< /Type /EmbeddedFile /Subtype /text#2Fplain /Params << /ModDate (D:20260501000000Z) /Size 3 >> /Length 3 >>\nstream\none\nendstream",
        "<< /Type /EmbeddedFile /Subtype /text#2Fplain /Params << /ModDate (D:20260501000000Z) /Size 3 >> /Length 3 >>\nstream\ntwo\nendstream",
        "<< /Type /EmbeddedFile /Subtype /text#2Fplain /Params << /ModDate (D:20260501000000Z) /Size 3 >> /Length 3 >>\nstream\nctl\nendstream",
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (i, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", i + 1);
    }
    let xref = out.len();
    let size = bodies.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<0123456789abcdef0123456789abcdef> <fedcba9876543210fedcba9876543210>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

#[test]
fn dpart_af_is_listed_through_the_ec3_current_child_array_and_page_links() {
    let bytes = fixture();
    let document = Document::open(bytes).expect("exact-xref PDF 2.0 fixture opens");
    assert!(
        !document.was_recovered(),
        "all xref offsets are calculated from the bytes"
    );

    let files = attachment::attachments(&document);
    let names: Vec<_> = files.iter().map(|file| file.file_name.as_deref()).collect();
    assert_eq!(
        names,
        [
            Some("catalog-control.txt"),
            Some("first-part.txt"),
            Some("second-part.txt")
        ],
        "the DPart files are discovered from the catalog root; there is no EmbeddedFiles tree"
    );
    assert_eq!(files[0].relationship, Relationship::Source);
    assert_eq!(files[1].relationship, Relationship::Data);
    assert_eq!(files[2].relationship, Relationship::Supplement);
    assert_eq!(
        document.decoded_stream_data(&files[1].stream).as_deref(),
        Some(&b"one"[..])
    );
    assert_eq!(
        document.decoded_stream_data(&files[2].stream).as_deref(),
        Some(&b"two"[..])
    );

    let hierarchy = pdf_model::document_part::hierarchy(&document).expect("catalog DPartRoot");
    assert_eq!(hierarchy.levels(), 2);
    assert_eq!(hierarchy.parts[1].start.map(|id| id.number), Some(3));
    assert_eq!(hierarchy.parts[2].start.map(|id| id.number), Some(9));
    assert!(
        hierarchy.parts[2].dictionary.get("DPM").is_some(),
        "opaque DPM stays available on the part; no print-production semantics are inferred"
    );
}

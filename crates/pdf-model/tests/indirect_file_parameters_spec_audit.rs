//! Public-consumer direct/indirect equivalence control for §7.11.3 Table 43 and §7.11.4.1 Tables 44–45.
//!
//! Both entries are associated embedded files with independent literal payload/digest/date
//! expectations. The indirect target places the file-name strings, `AFRelationship` name, EF
//! dictionary, stream Subtype name, Params dictionary, and each Params value in separate indirect
//! objects. The direct entry is a positive control using the same semantic fields inline.

#![expect(clippy::expect_used, clippy::arithmetic_side_effects)]

use std::fmt::Write as _;

use pdf_model::attachment::{self, Relationship};
use pdf_syntax::Document;

fn document(objects: &[&str]) -> Document {
    let mut out = String::from("%PDF-2.0\n");
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
    Document::open(out.into_bytes()).expect("complete strict-xref fixture")
}

fn direct_stream() -> String {
    let bytes = b"direct-file-bytes";
    format!(
        "<< /Type /EmbeddedFile /Subtype /application#2Foctet-stream /Length {} \
         /Params << /Size 17 /CreationDate (D:20260102030405Z) \
         /ModDate (D:20260203040506Z) /CheckSum <f38822b5cbb6cf0773bce80a9e3ef429> >> >>\n\
         stream\n{}\nendstream",
        bytes.len(),
        std::str::from_utf8(bytes).expect("fixture bytes are ASCII")
    )
}

fn indirect_stream() -> String {
    let bytes = b"indirect-file-bytes";
    format!(
        "<< /Type /EmbeddedFile /Subtype 13 0 R /Length {} /Params 14 0 R >>\n\
         stream\n{}\nendstream",
        bytes.len(),
        std::str::from_utf8(bytes).expect("fixture bytes are ASCII")
    )
}

#[test]
fn indirect_file_spec_and_params_values_reach_the_attachment_consumer() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /AF [5 0 R 6 0 R] \
         /Names << /EmbeddedFiles 3 0 R >> >>",
        "<< /Type /Pages /Count 1 /Kids [4 0 R] >>",
        "<< /Names [(direct) 5 0 R (indirect) 6 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        "<< /Type /Filespec /F (direct.bin) /UF (direct.bin) /AFRelationship /Data \
         /EF << /F 7 0 R >> >>",
        "<< /Type /Filespec /F 8 0 R /UF 9 0 R /AFRelationship 10 0 R /EF 11 0 R >>",
        &direct_stream(),
        "(indirect.bin)",
        "(indirect.bin)",
        "/Source",
        "<< /F 12 0 R >>",
        &indirect_stream(),
        "/application#2Foctet-stream",
        "<< /Size 15 0 R /CreationDate 16 0 R /ModDate 17 0 R /CheckSum 18 0 R >>",
        "19",
        "(D:20260102030405Z)",
        "(D:20260203040506Z)",
        "<1942b44a5dd9d3105ba824b02c814a32>",
    ]);
    assert!(
        !doc.was_recovered(),
        "the xref points to every fixture object"
    );

    let files = attachment::attachments(&doc);
    let direct = files
        .iter()
        .find(|file| file.name == "direct")
        .expect("direct control");
    let indirect = files
        .iter()
        .find(|file| file.name == "indirect")
        .expect("indirect target");
    for (file, name, relation, bytes, size, checksum) in [
        (
            direct,
            "direct.bin",
            Relationship::Data,
            &b"direct-file-bytes"[..],
            17,
            "f38822b5cbb6cf0773bce80a9e3ef429",
        ),
        (
            indirect,
            "indirect.bin",
            Relationship::Source,
            &b"indirect-file-bytes"[..],
            19,
            "1942b44a5dd9d3105ba824b02c814a32",
        ),
    ] {
        assert_eq!(file.file_name.as_deref(), Some(name));
        assert_eq!(file.relationship, relation);
        assert_eq!(file.media_type.as_deref(), Some("application/octet-stream"));
        assert_eq!(file.size, Some(size));
        assert_eq!(file.created.as_deref(), Some("D:20260102030405Z"));
        assert_eq!(file.modified.as_deref(), Some("D:20260203040506Z"));
        assert_eq!(
            file.created_date(),
            Some(pdf_syntax::Date {
                year: 2026,
                month: 1,
                day: 2,
                hour: 3,
                minute: 4,
                second: 5,
                offset: Some(0),
            })
        );
        assert_eq!(
            file.modified_date(),
            Some(pdf_syntax::Date {
                year: 2026,
                month: 2,
                day: 3,
                hour: 4,
                minute: 5,
                second: 6,
                offset: Some(0),
            })
        );
        assert_eq!(
            doc.decoded_stream_data(&file.stream).as_deref(),
            Some(bytes),
            "the /EF route reaches the exact embedded bytes"
        );
        let expected_digest = (0..checksum.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&checksum[index..index + 2], 16).expect("hex"))
            .collect::<Vec<_>>();
        assert_eq!(file.checksum, Some(expected_digest));
        assert_eq!(file.checksum_matches(bytes), Some(true));
        assert_eq!(file.checksum_matches(b"wrong bytes"), Some(false));
    }
}

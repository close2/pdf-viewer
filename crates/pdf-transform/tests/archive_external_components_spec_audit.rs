//! Integration regression for the public archive `ExternalData::Named` report.
//!
//! A reachable Form `XObject` uses the external-stream `/F` field, which the archive checker
//! reports without opening it. Both files have valid exact xrefs and PDF/A-4 identification;
//! the expected components are obtained from the literal slash-separated path, independently of
//! `FileSpec::components`.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test fixture bytes and offsets are built locally and must fail loudly if malformed"
)]

use std::io::Write as _;

use pdf_archive::{Flavour, Target};
use pdf_syntax::Document;
use pdf_transform::archive::{ExternalData, external_stream_data};

fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

fn pdf(path: &str) -> Vec<u8> {
    let packet = "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
         xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
         <rdf:Description rdf:about=\"\" xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\
         <pdfaid:part>4</pdfaid:part><pdfaid:rev>2020</pdfaid:rev>\
         </rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>";
    let page_contents = b"q /Fm0 Do Q";
    let bodies = [
        b"<< /Type /Catalog /Pages 2 0 R /Metadata 5 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
           /Resources << /XObject << /Fm0 6 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        stream("", page_contents),
        stream("/Type /Metadata /Subtype /XML", packet.as_bytes()),
        stream(
            &format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << >> /F ({path})"
            ),
            b"",
        ),
    ];

    let mut bytes = b"%PDF-2.0\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj", index + 1);
        bytes.extend_from_slice(body);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = bytes.len();
    let size = offsets.len() + 1;
    let _ = write!(bytes, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<0102030405060708090A0B0C0D0E0F10> \
         <0102030405060708090A0B0C0D0E0F10>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    bytes
}

fn named_components(path: &str) -> Vec<Vec<u8>> {
    let document =
        Document::open(pdf(path)).expect("exact-xref PDF with PDF/A-4 target identification");
    assert!(!document.was_recovered());
    assert!(!document.cross_reference_chain_was_cut());
    let report = pdf_archive::check(&document, Target::Four(Flavour::Plain));
    let found = external_stream_data(&document, &report);
    let [stream] = found.as_slice() else {
        panic!("one reachable external-data stream is reported: {found:?}");
    };
    let ExternalData::Named {
        shown,
        components,
        absolute,
    } = &stream.data
    else {
        panic!(
            "ordinary path is named, not URL/unreadable: {:?}",
            stream.data
        );
    };
    assert_eq!(shown, path);
    assert!(!absolute);
    components.clone()
}

#[test]
fn archive_report_preserves_all_components_at_and_beyond_the_parser_limit() {
    // Positive control at FileSpec's current boundary.
    let at_limit = vec!["x"; 256].join("/");
    assert_eq!(named_components(&at_limit), vec![b"x".to_vec(); 256]);

    // This remains an in-memory report test. The test does not resolve the name or open it.
    let over_limit = vec!["x"; 257].join("/");
    assert_eq!(
        named_components(&over_limit),
        vec![b"x".to_vec(); 257],
        "report must preserve separators or return an explicit, truthful refusal"
    );
}

//! Host-properties regression for independent document metadata sources.
//!
//! §14.3.4 gives readers discretion over inconsistent dates, and does not prescribe a general
//! precedence rule for other metadata. This pins the product's choice to preserve the source
//! labels and both values instead of merging them into one ambiguous "Title".

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref fixture"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;
use viewer_core::{Answer, Command, DocumentId, Query, Viewer};
use viewer_host::panel::property_rows;

const PACKET: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/">
      <dc:title><rdf:Alt><rdf:li xml:lang="x-default">Packet title</rdf:li></rdf:Alt></dc:title>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;

fn metadata_pdf(with_catalog_packet: bool) -> Vec<u8> {
    let catalog_metadata = if with_catalog_packet {
        " /Metadata 5 0 R"
    } else {
        ""
    };
    let packet_object = if with_catalog_packet {
        format!(
            "5 0 obj\n<< /Type /Metadata /Subtype /XML /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            PACKET.len(),
            PACKET
        )
    } else {
        // Keep object numbering contiguous while making the Catalog's no-packet control exact.
        String::from("5 0 obj\n<< >>\nendobj\n")
    };
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R{catalog_metadata} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         {packet_object}\
         6 0 obj\n<< /Title (Dictionary title) >>\nendobj\n"
    );
    let mut output = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(output.len());
        output.push_str(object);
    }
    let xref_at = output.len();
    let size = offsets.len() + 1;
    let _ = writeln!(output, "xref\n0 {size}");
    output.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(output, "{offset:010} 00000 n ");
    }
    let _ = write!(
        output,
        "trailer\n<< /Size {size} /Root 1 0 R /Info 6 0 R \
         /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\n\
         startxref\n{xref_at}\n%%EOF\n"
    );
    output.into_bytes()
}

fn opened(bytes: Vec<u8>) -> Viewer {
    let document = Document::open(bytes.clone()).expect("complete source PDF");
    assert!(!document.was_recovered(), "fixture uses exact xref offsets");
    let mut viewer = Viewer::new(200, 200, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
}

#[test]
fn properties_keep_conflicting_info_and_packet_titles_separate() {
    let viewer = opened(metadata_pdf(true));
    let Answer::Properties {
        information,
        metadata,
    } = viewer.query(Query::Properties)
    else {
        panic!("an open document answers its properties");
    };
    assert_eq!(information.title.as_deref(), Some("Dictionary title"));
    let packet = metadata
        .as_ref()
        .expect("Catalog names a metadata stream")
        .as_ref()
        .expect("stream is readable XMP");
    assert_eq!(packet.title(), Some("Packet title"));

    let rows = property_rows(&information, metadata.as_ref());
    let dictionary_title = rows
        .iter()
        .find(|row| row.label == "Title:")
        .expect("dictionary property row");
    let packet_title = rows
        .iter()
        .find(|row| row.label == "dc:title:")
        .expect("separately labeled XMP property row");
    assert_eq!(dictionary_title.detail.as_deref(), Some("Dictionary title"));
    assert_eq!(packet_title.detail.as_deref(), Some("Packet title"));

    let plain = opened(metadata_pdf(false));
    let Answer::Properties {
        information,
        metadata,
    } = plain.query(Query::Properties)
    else {
        panic!("an open document answers its properties");
    };
    assert_eq!(information.title.as_deref(), Some("Dictionary title"));
    assert!(metadata.is_none(), "the catalog does not name object 5");
    assert!(
        !property_rows(&information, metadata.as_ref())
            .iter()
            .any(|row| row.label == "dc:title:"),
        "the no-stream control does not synthesize an XMP title"
    );
}

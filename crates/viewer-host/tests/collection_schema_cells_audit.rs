//! Integration tests for ISO 32000-2 §12.3.5.1 Tables 153 and 155.
//!
//! The exact-xref PDF
//! includes a PDF 2.0 trailer ID, binary header marker, indirect CI and an independently valid
//! compressed payload. UF/Desc/Size are positive controls;
//! decoded Size=3 and compressed stream Length=11 make the numeric sources distinguishable.

#![expect(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "bounded exact-xref fixtures and explicit assertion failures"
)]

use viewer_core::{Answer, Command, DocumentId, Query, Viewer};
use viewer_host::collection_rows;

fn pdf_with_ci_schema() -> Vec<u8> {
    use std::io::Write as _;

    let mut embedded = b"<< /Type /EmbeddedFile /Filter /FlateDecode /Length 11 \
        /Params << /Size 3 >> >>\nstream\n"
        .to_vec();
    // RFC 1950 zlib stream decoding to exactly abc.
    embedded.extend_from_slice(&[
        0x78, 0x9c, 0x4b, 0x4c, 0x4a, 0x06, 0x00, 0x02, 0x4d, 0x01, 0x27,
    ]);
    embedded.extend_from_slice(b"\nendstream");

    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles << /Names [(alpha.txt) 4 0 R] >> >> /Collection 6 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_vec(),
        b"<< /Type /Filespec /F (alpha.txt) /UF <FEFF0061006C007000680061002E007400780074> /Desc (positive-control-description) /CI 8 0 R /EF << /F 5 0 R /UF 5 0 R >> >>".to_vec(),
        embedded,
        b"<< /Type /Collection /D (alpha.txt) /View /D /Schema 7 0 R >>".to_vec(),
        b"<< /Type /CollectionSchema \
          /Author << /Subtype /S /N (Author) /O 1 /V true >> \
          /Received << /Subtype /D /N (Received) /O 2 /V true >> \
          /Rank << /Subtype /N /N (Rank) /O 3 /V true >> \
          /Filename << /Subtype /F /N (Filename) /O 4 /V true >> \
          /Description << /Subtype /Desc /N (Description) /O 5 /V true >> \
          /Size << /Subtype /Size /N (Size) /O 6 /V true >> \
          /Packed << /Subtype /CompressedSize /N (Compressed size) /O 7 /V true >> >>".to_vec(),
        b"<< /Type /CollectionItem /Author (Ada Lovelace) /Received (D:20240102030405Z) /Rank 7 >>".to_vec(),
    ];

    let mut bytes = b"%PDF-2.0\n%\x80\x81\x82\x83\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let number = index + 1;
        writeln!(&mut bytes, "{number} 0 obj").unwrap();
        bytes.extend_from_slice(body);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref_offset = bytes.len();
    let size = objects.len() + 1;
    write!(&mut bytes, "xref\n0 {size}\n0000000000 65535 f \n").unwrap();
    for offset in offsets {
        writeln!(&mut bytes, "{offset:010} 00000 n ").unwrap();
    }
    write!(
        &mut bytes,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\nstartxref\n{xref_offset}\n%%EOF\n"
    )
    .unwrap();
    bytes
}

fn detail_cells() -> std::collections::BTreeMap<String, String> {
    let bytes = pdf_with_ci_schema();
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("fixture parses");
    assert!(!parsed.was_recovered(), "fixture uses exact xref offsets");
    assert!(!parsed.cross_reference_chain_was_cut());
    let embedded = parsed.get(pdf_syntax::ObjectId::new(5, 0));
    let stream = embedded.as_stream().expect("embedded file stream");
    assert_eq!(
        stream.data.len(),
        11,
        "encoded size differs from decoded size"
    );
    assert_eq!(
        parsed
            .decoded_stream_data(stream)
            .expect("valid zlib")
            .as_ref(),
        b"abc"
    );
    let filespec = parsed.get(pdf_syntax::ObjectId::new(4, 0));
    let item =
        pdf_model::collection::item(&parsed, filespec.as_dict().expect("file specification"));
    assert_eq!(
        item.values
            .get("Author")
            .expect("CI Author")
            .data
            .as_string(),
        Some(b"Ada Lovelace".as_slice())
    );
    assert_eq!(
        item.values
            .get("Received")
            .expect("CI Received")
            .data
            .as_string(),
        Some(b"D:20240102030405Z".as_slice())
    );
    assert_eq!(
        item.values.get("Rank").expect("CI Rank").data.as_integer(),
        Some(7)
    );

    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);

    let Answer::Collection {
        collection,
        initial,
        order,
    } = viewer.query(Query::Collection)
    else {
        panic!("catalog contains /Collection");
    };
    let Answer::Attachments(attachments) = viewer.query(Query::Attachments) else {
        panic!("EmbeddedFiles contains one attachment");
    };
    assert_eq!(attachments.len(), 1, "independent population control");
    let rows = collection_rows(&collection, &initial, &order, &attachments);
    let row = rows
        .iter()
        .find(|row| row.label == "alpha.txt")
        .expect("one visible file row");
    let cells: Vec<(&str, &str)> = row
        .cells
        .iter()
        .map(|cell| (cell.heading.as_str(), cell.value.as_str()))
        .collect();

    // Positive controls run before any missing-CI assertion, through the same public consumer.
    assert!(
        cells.contains(&("Filename", "alpha.txt")),
        "file-name control: {cells:?}"
    );
    assert!(
        cells.contains(&("Description", "positive-control-description")),
        "description control: {cells:?}"
    );
    assert!(
        cells.contains(&("Size", "3")),
        "decoded-size control: {cells:?}"
    );
    cells
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

#[test]
fn details_view_presents_the_ci_text_value() {
    let cells = detail_cells();
    assert_eq!(
        cells.get("Author").map(String::as_str),
        Some("Ada Lovelace")
    );
}

#[test]
fn details_view_presents_a_ci_date_cell() {
    let cells = detail_cells();
    // Table 155 requires presentation; it leaves date formatting to the UI. The fixture's
    // exact raw date was checked above. This detects omission, not all date-format errors.
    assert!(
        cells.get("Received").is_some_and(|value| !value.is_empty()),
        "visible date omitted: {cells:?}"
    );
}

#[test]
fn details_view_presents_the_ci_numeric_value() {
    let cells = detail_cells();
    assert_eq!(cells.get("Rank").map(String::as_str), Some("7"));
}

#[test]
fn details_view_distinguishes_compressed_size_from_decoded_size() {
    let cells = detail_cells();
    assert_eq!(cells.get("Compressed size").map(String::as_str), Some("11"));
}

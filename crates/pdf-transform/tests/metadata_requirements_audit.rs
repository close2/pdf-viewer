//! ISO 32000-2 §14.3: public metadata writers and the conditional Table 349 `/ModDate` requirement.

#![expect(
    clippy::expect_used,
    reason = "audit tests use complete self-contained fixtures and fail loudly on an invalid writer result"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, Object};
use pdf_transform::merge::{Input, MergePlan};
use pdf_transform::update::{Edit, InfoEntry, UpdatePlan};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Refusal, Source, apply};

fn one_page_pdf(version: &str, catalog_entries: &str, info: Option<&str>) -> Vec<u8> {
    let mut body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {catalog_entries} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n"
    );
    let info_entry = info.map_or_else(String::new, |dict| {
        let _ = write!(body, "5 0 obj\n{dict}\nendobj\n");
        String::from(" /Info 5 0 R")
    });

    let mut bytes = format!("%PDF-{version}\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref_at = bytes.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R{info_entry} /ID [<0123456789abcdef0123456789abcdef> <0123456789abcdef0123456789abcdef>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    bytes.into_bytes()
}

fn output(sinks: MemorySinks) -> Vec<u8> {
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1, "one metadata operation writes one file");
    outputs.remove(0).1
}

fn assert_metadata_stream(document: &Document, expected_title: &str) {
    let catalog = document.catalog().expect("output has a Catalog");
    let metadata = document.get_key(&catalog, "Metadata");
    let stream = metadata
        .as_stream()
        .expect("Catalog /Metadata resolves to a stream");
    assert_eq!(
        document
            .get_key(&stream.dict, "Type")
            .as_name()
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"Metadata"[..]),
        "Table 347 requires /Type /Metadata"
    );
    assert_eq!(
        document
            .get_key(&stream.dict, "Subtype")
            .as_name()
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"XML"[..]),
        "Table 347 requires /Subtype /XML"
    );
    let decoded = document
        .decoded_stream_data(stream)
        .expect("metadata stream decodes");
    let packet = std::str::from_utf8(&decoded).expect("PDF XMP producer output is UTF-8");
    let xmp =
        pdf_model::xmp::Xmp::parse(packet.as_bytes()).expect("emitted packet is readable XMP");
    assert_eq!(xmp.title(), Some(expected_title));
}

#[test]
fn emitted_metadata_streams_have_table347_names_and_utf8_packets() {
    let title = "Méta report";
    let update_source = one_page_pdf("2.0", "", None);
    let source = Document::open(update_source.clone()).expect("valid update source opens");
    assert!(
        !source.was_recovered(),
        "positive control uses a complete xref"
    );
    assert!(
        source
            .catalog()
            .expect("source catalog")
            .get("Metadata")
            .is_none(),
        "positive control has no metadata stream before the edit"
    );
    let sinks = MemorySinks::new();
    apply(
        &Plan::Update(UpdatePlan {
            source: 0,
            edit: Edit::SetInformation {
                entries: vec![InfoEntry {
                    key: String::from("Title"),
                    value: Some(title.to_owned()),
                }],
            },
            names: "updated.pdf".parse().expect("output name parses"),
        }),
        &[Source::new(update_source)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("valid update writes metadata");
    let updated = Document::open(output(sinks)).expect("update output opens");
    assert!(
        !updated.was_recovered(),
        "update output has valid xref data"
    );
    assert_metadata_stream(&updated, title);

    let merge_source = one_page_pdf("1.7", "", None);
    let source = Document::open(merge_source.clone()).expect("valid merge source opens");
    assert!(
        !source.was_recovered(),
        "positive control uses a complete xref"
    );
    let sinks = MemorySinks::new();
    apply(
        &Plan::Merge(MergePlan {
            inputs: vec![Input {
                source: 0,
                pages: "1".parse().expect("one-page selection parses"),
            }],
            collate: false,
            information: vec![InfoEntry {
                key: String::from("Title"),
                value: Some(title.to_owned()),
            }],
            names: "merged.pdf".parse().expect("output name parses"),
        }),
        &[Source::new(merge_source)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("valid merge writes metadata");
    let merged = Document::open(output(sinks)).expect("merge output opens");
    assert!(!merged.was_recovered(), "merge output has valid xref data");
    assert_metadata_stream(&merged, title);
}

#[test]
fn pieceinfo_keeps_required_moddate_when_update_requests_removal() {
    let catalog_entries = "/PieceInfo << /Example << /LastModified (D:20261001120000Z) \
        /Private << /note (kept) >> >> >>";
    let dated_info = Some("<< /ModDate (D:20261001120000Z) >>");

    optional_moddate_can_be_removed();

    // Positive control 2: a valid Catalog PieceInfo has one processor data dictionary; its
    // required LastModified and optional Private entries are nonempty. A different Info edit
    // preserves the existing required ModDate in raw trailer /Info.
    let kept_source = one_page_pdf("1.7", catalog_entries, dated_info);
    let kept = Document::open(kept_source.clone()).expect("complete PieceInfo source opens");
    assert!(
        !kept.was_recovered(),
        "positive control has a complete xref"
    );
    let kept_catalog = kept.catalog().expect("source Catalog");
    let piece_value = kept.get_key(&kept_catalog, "PieceInfo");
    let piece_info = piece_value.as_dict().expect("PieceInfo is a dictionary");
    let data_value = kept.get_key(piece_info, "Example");
    let data = data_value
        .as_dict()
        .expect("processor name maps to a data dictionary");
    assert!(kept.get_key(data, "LastModified").as_string().is_some());
    assert!(kept.get_key(data, "Private").as_dict().is_some());
    assert!(kept_catalog.get("Metadata").is_none());
    let kept_bytes = update_information(
        kept_source,
        vec![InfoEntry {
            key: String::from("Title"),
            value: Some(String::from("A new title")),
        }],
    )
    .expect("unrelated metadata edit applies");
    let kept_output = Document::open(kept_bytes).expect("kept-date output opens");
    assert!(!kept_output.was_recovered());
    assert!(
        kept_output
            .catalog()
            .expect("output Catalog")
            .get("PieceInfo")
            .is_some()
    );
    assert_eq!(
        raw_info(&kept_output)
            .get("ModDate")
            .and_then(Object::as_string)
            .map(<[u8]>::to_vec),
        Some(b"D:20261001120000Z".to_vec()),
        "the unrelated edit preserves the date Table 349 requires"
    );

    // Regression: removing that required entry must either be declined specifically or leave a
    // required date in the output. Inspect raw trailer /Info; do not use the view that falls back
    // to the XMP counterpart, the source fixture has no Catalog /Metadata.
    let source_bytes = one_page_pdf("1.7", catalog_entries, dated_info);
    let source = Document::open(source_bytes.clone()).expect("complete source opens");
    assert!(
        !source.was_recovered(),
        "regression source has a complete xref"
    );
    let source_catalog = source.catalog().expect("source Catalog");
    assert!(
        source
            .get_key(&source_catalog, "PieceInfo")
            .as_dict()
            .is_some()
    );
    assert!(source_catalog.get("Metadata").is_none());
    assert_eq!(
        raw_info(&source)
            .get("ModDate")
            .and_then(Object::as_string)
            .map(<[u8]>::to_vec),
        Some(b"D:20261001120000Z".to_vec()),
        "positive control states the required date in Info"
    );

    match update_information(
        source_bytes,
        vec![InfoEntry {
            key: String::from("ModDate"),
            value: None,
        }],
    ) {
        Err(Refusal::Pattern(reason) | Refusal::Assembly(reason)) => assert!(
            reason.contains("ModDate") || reason.contains("14.3.3"),
            "a refusal identifies the required entry: {reason}"
        ),
        Err(other) => panic!("writer refused for an unrelated reason: {other}"),
        Ok(updated_bytes) => {
            let updated = Document::open(updated_bytes).expect("updated PDF opens");
            assert!(
                !updated.was_recovered(),
                "updated output has valid xref data"
            );
            let catalog = updated.catalog().expect("updated Catalog");
            assert!(updated.get_key(&catalog, "PieceInfo").as_dict().is_some());
            assert!(
                raw_info(&updated)
                    .get("ModDate")
                    .and_then(Object::as_string)
                    .map(pdf_syntax::text_string)
                    .and_then(|date| pdf_syntax::Date::parse(&date))
                    .is_some(),
                "Table 349 requires /ModDate while Catalog /PieceInfo remains present"
            );
        }
    }
}

fn update_information(source: Vec<u8>, entries: Vec<InfoEntry>) -> Result<Vec<u8>, Refusal> {
    let sinks = MemorySinks::new();
    apply(
        &Plan::Update(UpdatePlan {
            source: 0,
            edit: Edit::SetInformation { entries },
            names: "updated.pdf".parse().expect("output name parses"),
        }),
        &[Source::new(source)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )?;
    Ok(output(sinks))
}

fn raw_info(document: &Document) -> pdf_syntax::Dictionary {
    let info = document
        .trailer()
        .get("Info")
        .expect("trailer states /Info");
    document
        .resolve(info)
        .as_dict()
        .expect("trailer /Info resolves to a dictionary")
        .clone()
}

fn optional_moddate_can_be_removed() {
    let dated_info = Some("<< /ModDate (D:20261001120000Z) >>");
    // Positive control 1: without Catalog /PieceInfo, ModDate is optional and the same edit may
    // remove it. The source has no Catalog packet, so no XMP fallback can hide the dictionary.
    let optional_source = one_page_pdf("1.7", "", dated_info);
    let optional = Document::open(optional_source.clone()).expect("complete optional source opens");
    assert!(
        !optional.was_recovered(),
        "positive control has a complete xref"
    );
    assert!(
        optional
            .catalog()
            .expect("Catalog")
            .get("PieceInfo")
            .is_none()
    );
    assert!(
        optional
            .catalog()
            .expect("Catalog")
            .get("Metadata")
            .is_none()
    );
    let optional_bytes = update_information(
        optional_source,
        vec![InfoEntry {
            key: String::from("ModDate"),
            value: None,
        }],
    )
    .expect("optional ModDate may be removed");
    let optional_output = Document::open(optional_bytes).expect("optional-date output opens");
    assert!(!optional_output.was_recovered());
    let optional_info = raw_info(&optional_output);
    assert!(optional_info.get("ModDate").is_none());
}

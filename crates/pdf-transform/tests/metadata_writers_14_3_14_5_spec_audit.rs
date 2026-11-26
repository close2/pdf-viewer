//! ISO 32000-2 §14.3 to §14.5 against the transform suite's writers: the date a packet is given
//! beside the document information dictionary, the dictionary a catalog's `/PieceInfo` needs,
//! a page's private data after its content changed, and a packet an update creates in an
//! encrypted file (review 2253, `doc/reviews/2253-specification-audit-14-1-14-5-14-10-14-13.md`).
//!
//! Each `*_spec_audit` test is a behaviour finding and fails on the tree it was written against;
//! each has a control beside it that passes there, so the failure is the finding and not the
//! fixture.

#![expect(
    clippy::expect_used,
    reason = "audit tests use complete self-contained fixtures and fail loudly on an invalid writer result"
)]

use std::fmt::Write as _;

use pdf_syntax::object::ObjectId;
use pdf_syntax::serialize::{
    Access, Assembly, Entropy, Form, ObjectStreams, Options, Protection, Streams,
    serialize_encrypted,
};
use pdf_syntax::{Document, Limits, Object, Version};
use pdf_transform::optimize::OptimizePlan;
use pdf_transform::redact::RedactPlan;
use pdf_transform::update::{Edit, InfoEntry, UpdatePlan};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

/// Writes `objects` (numbered from one) into a §7.5.4 cross-referenced file of `version`, with
/// `trailer` added to the trailer dictionary.
fn assemble(version: &str, objects: &[String], trailer: &str) -> Vec<u8> {
    let mut out = format!("%PDF-{version}\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{object}\nendobj", index.saturating_add(1));
    }
    let at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R {trailer} /ID [<00112233445566778899aabbccddeeff> \
         <00112233445566778899aabbccddeeff>] >>\nstartxref\n{at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A stream object's text with its `/Length` stated.
fn stream(entries: &str, data: &str) -> String {
    format!(
        "<< {entries} /Length {} >>\nstream\n{data}\nendstream",
        data.len()
    )
}

/// The one output file of `plan` over `source`.
fn run(plan: &Plan, source: Vec<u8>) -> Vec<u8> {
    let sinks = MemorySinks::new();
    apply(
        plan,
        &[Source::new(source)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the plan applies to a valid fixture");
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1, "one source, one output");
    outputs.remove(0).1
}

/// `SetInformation` with one entry.
fn set_information(source: Vec<u8>, key: &str, value: &str) -> Vec<u8> {
    run(
        &Plan::Update(UpdatePlan {
            source: 0,
            edit: Edit::SetInformation {
                entries: vec![InfoEntry {
                    key: key.to_owned(),
                    value: Some(value.to_owned()),
                }],
            },
            names: "updated.pdf".parse().expect("output name parses"),
        }),
        source,
    )
}

/// The document-level packet of `document`, parsed.
fn packet(document: &Document) -> pdf_model::xmp::Xmp {
    pdf_model::xmp::Xmp::document(document)
        .expect("the catalog names a metadata stream")
        .expect("the metadata stream is a readable packet")
}

// ---------------------------------------------------------------------------------------------
// 2253-B01: a date with no zone is restated into the packet with no zone.
// ---------------------------------------------------------------------------------------------

/// A PDF 1.7 page whose catalog names a packet stating `xmp:CreatorTool` and no dates.
fn dated_source() -> Vec<u8> {
    let xmp = "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\
               <x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
               xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
               <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">\
               <xmp:CreatorTool>Fixture</xmp:CreatorTool></rdf:Description></rdf:RDF>\
               </x:xmpmeta><?xpacket end=\"w\"?>";
    assemble(
        "1.7",
        &[
            "<< /Type /Catalog /Pages 2 0 R /Metadata 5 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> \
             /Contents 4 0 R >>"
                .to_owned(),
            stream("", ""),
            stream("/Type /Metadata /Subtype /XML", xmp),
            "<< /Producer (Fixture) >>".to_owned(),
        ],
        "/Info 6 0 R",
    )
}

/// The `/ModDate` and `xmp:ModifyDate` an update writes for `value`.
fn written_modification(value: &str) -> (String, String) {
    let updated = Document::open(set_information(dated_source(), "ModDate", value))
        .expect("the update opens");
    let info = updated
        .resolve(
            updated
                .trailer()
                .get("Info")
                .expect("the trailer names /Info"),
        )
        .as_dict()
        .cloned()
        .expect("/Info is a dictionary");
    let dictionary = info
        .get("ModDate")
        .and_then(Object::as_string)
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        .expect("the dictionary states the new /ModDate");
    let restated = packet(&updated)
        .modified()
        .expect("the packet states xmp:ModifyDate beside it")
        .to_owned();
    (dictionary, restated)
}

/// Control: a date that states its zone crosses into the packet as the same instant.
#[test]
fn a_zoned_modification_date_is_restated_as_the_same_instant() {
    let (dictionary, restated) = written_modification("D:20250102030405+01'00");
    assert_eq!(dictionary, "D:20250102030405+01'00");
    assert_eq!(restated, "2025-01-02T03:04:05+01:00");
}

/// §14.3.4: "When writing the time and date of the most recent modification … a PDF processor
/// shall ensure that the data in the document information dictionary and the document level
/// metadata stream -if both are written -are fully equivalent." §7.9.4 as Errata Collection 3's
/// Issue #251 (approved) amends it: where no UT information is specified, "the missing timezone
/// offset shall be assumed to be the same as Greenwich Mean Time's timezone offset (+0'00)". So
/// `D:20250102030405` is 03:04:05 UTC, and the packet's equivalent states that offset; a time of
/// day with no designator is none of the XMP Specification's six profiles of ISO 8601 (W3C's
/// Date and Time Formats, which ISO 16684-1 references normatively, and ADR 1013 section 5).
#[test]
fn a_zoneless_modification_date_is_restated_at_utc_spec_audit() {
    let (dictionary, restated) = written_modification("D:20250102030405");
    assert_eq!(
        dictionary, "D:20250102030405",
        "the dictionary keeps the value"
    );
    assert!(
        restated == "2025-01-02T03:04:05Z" || restated == "2025-01-02T03:04:05+00:00",
        "§14.3.4 with #251: /ModDate D:20250102030405 is 03:04:05 at +0'00, and the packet's \
         xmp:ModifyDate states {restated:?}, a time with no zone designator"
    );
}

// ---------------------------------------------------------------------------------------------
// 2253-B02: a direct /Info is dropped while the catalog's /PieceInfo is carried.
// ---------------------------------------------------------------------------------------------

/// A page under a catalog stating `/PieceInfo`, with the information dictionary `info` and the
/// trailer naming it as `trailer`.
fn piece_source(info: Option<&str>, trailer: &str) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /PieceInfo << /Example << \
         /LastModified (D:20250101000000Z) >> >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> /Contents 4 0 R >>"
            .to_owned(),
        stream("", "0 0 5 5 re f"),
    ];
    if let Some(info) = info {
        objects.push(info.to_owned());
    }
    assemble("1.7", &objects, trailer)
}

/// What optimising `source` leaves: whether the catalog states `/PieceInfo`, and the
/// information dictionary's `/ModDate`.
fn optimized(source: Vec<u8>) -> (bool, Option<Vec<u8>>) {
    let output = run(
        &Plan::Optimize(OptimizePlan {
            source: 0,
            names: "optimized.pdf".parse().expect("output name parses"),
            prune: true,
            object_streams: ObjectStreams::Disable,
            streams: Streams::Carry,
            linearize: false,
            images: None,
        }),
        source,
    );
    let document = Document::open(output).expect("the optimised file opens");
    let catalog = document.catalog().expect("the output has a catalog");
    let pieces = !document.get_key(&catalog, "PieceInfo").is_null();
    let date = document
        .get_key(document.trailer(), "Info")
        .as_dict()
        .and_then(|info| info.get("ModDate").and_then(Object::as_string))
        .map(<[u8]>::to_vec);
    (pieces, date)
}

/// Control: an indirect `/Info` crosses with the `/PieceInfo` it dates.
#[test]
fn an_indirect_information_dictionary_crosses_beside_piece_info() {
    let (pieces, date) = optimized(piece_source(
        Some("<< /ModDate (D:20250101000000Z) >>"),
        "/Info 5 0 R",
    ));
    assert!(pieces, "the catalog's /PieceInfo is carried");
    assert_eq!(date.as_deref(), Some(&b"D:20250101000000Z"[..]));
}

/// Table 349's `/ModDate`: "( Required if `PieceInfo` is present in the document catalog
/// dictionary; otherwise optional; PDF 1.1 )". Errata Collection 3's Issue #106 (approved)
/// struck "; shall be an indirect reference" from Table 15's `/Info`, so a trailer stating the
/// dictionary directly is a valid file, and `optimize` reads the entry only as a reference: the
/// output keeps the catalog's `/PieceInfo` and has no `/ModDate` anywhere.
#[test]
fn a_direct_information_dictionary_crosses_beside_piece_info_spec_audit() {
    let (pieces, date) = optimized(piece_source(
        None,
        "/Info << /ModDate (D:20250101000000Z) >>",
    ));
    assert!(pieces, "the catalog's /PieceInfo is carried");
    assert_eq!(
        date.as_deref(),
        Some(&b"D:20250101000000Z"[..]),
        "Table 349 requires /ModDate beside the catalog's /PieceInfo, and the source's direct \
         /Info stated it"
    );
}

// ---------------------------------------------------------------------------------------------
// 2253-B03: a redacted page keeps private data its own dates call current.
// ---------------------------------------------------------------------------------------------

/// A page whose content shows `SECRET` under a `/Redact` annotation, carrying a page-piece
/// dictionary whose data dictionary's `LastModified` equals the page's.
fn redaction_source() -> Vec<u8> {
    let content = "BT /F1 12 Tf 20 50 Td (SECRET) Tj ET";
    assemble(
        "1.7",
        &[
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /Annots [6 0 R] \
             /LastModified (D:20250101000000Z) /PieceInfo << /Editor << \
             /LastModified (D:20250101000000Z) /Private << /Art (SECRET) >> >> >> >>"
                .to_owned(),
            stream("", content),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
            "<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] >>".to_owned(),
        ],
        "",
    )
}

/// The redacted page of [`redaction_source`].
fn redacted_page() -> (Document, pdf_syntax::Dictionary) {
    let output = run(
        &Plan::Redact(RedactPlan {
            source: 0,
            names: "redacted.pdf".parse().expect("output name parses"),
            image_samples: None,
        }),
        redaction_source(),
    );
    let document = Document::open(output).expect("the redacted file opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the output has its page");
    let dict = page.dict.clone();
    (document, dict)
}

/// Control: the redaction removed the page's text.
#[test]
fn the_redaction_removes_the_text_it_covers() {
    let (document, page) = redacted_page();
    let content = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the output has its page")
        .content(&document);
    assert!(
        !content.windows(6).any(|window| window == b"SECRET"),
        "the redacted text is gone from the content stream"
    );
    assert!(page.get("Annots").is_none_or(|annots| {
        document
            .resolve(annots)
            .as_array()
            .is_none_or(<[Object]>::is_empty)
    }));
}

/// §14.5: a data dictionary's `LastModified` is "the date and time when the contents of the
/// document, page, or form were most recently modified by this PDF processor", and its dates
/// "can be compared with those in the corresponding entry of the page object … to ascertain
/// whether the data dictionary corresponds to the current content of the document, page or
/// form", "only for equality". Table 31's `/LastModified` is "( Required if `PieceInfo` is
/// present; optional otherwise; PDF 1.3 ) The date and time when the page's contents were most
/// recently modified." Redaction modifies the page's contents and carries both dates unchanged,
/// so the written page states a modification time before its last modification, and the
/// equality comparison declares private data — here holding the redacted text — current.
#[test]
fn a_redacted_page_does_not_call_its_private_data_current_spec_audit() {
    let (document, page) = redacted_page();
    let page_date = document.get_key(&page, "LastModified");
    let pieces = document.get_key(&page, "PieceInfo");
    let current: Vec<String> = pieces.as_dict().map_or_default(|pieces| {
        pieces
            .iter()
            .filter(|(_, data)| {
                document
                    .resolve(data)
                    .as_dict()
                    .is_some_and(|data| document.get_key(data, "LastModified") == page_date)
            })
            .map(|(name, _)| String::from_utf8_lossy(name.as_bytes()).into_owned())
            .collect()
    });
    assert!(
        current.is_empty(),
        "after the redaction changed the page's contents, the page's /LastModified still equals \
         the LastModified of data dictionaries {current:?}, which §14.5's comparison reads as \
         corresponding to the current content"
    );
}

// ---------------------------------------------------------------------------------------------
// 2253-B04: a packet an update creates in an encrypted file ignores /EncryptMetadata false.
// ---------------------------------------------------------------------------------------------

/// A counting randomness source, so the encrypted fixture is a function of its plan.
struct Counter(u8);

impl Entropy for Counter {
    fn fill(&mut self, out: &mut [u8]) -> bool {
        for byte in out.iter_mut() {
            *byte = self.0;
            self.0 = self.0.wrapping_add(1);
        }
        true
    }
}

/// A PDF 2.0 page with no metadata stream, written encrypted under the default user password
/// with `encrypt_metadata` as given.
fn encrypted_source(encrypt_metadata: bool) -> Vec<u8> {
    let plain = assemble(
        "2.0",
        &[
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> \
             /Contents 4 0 R >>"
                .to_owned(),
            stream("", "0 0 5 5 re f"),
        ],
        "",
    );
    let source = Document::open_with_limits(plain, Limits::DEFAULT).expect("the fixture opens");
    let mut assembly = Assembly::new(vec![&source]);
    for number in 1..=4 {
        assembly
            .copy(0, ObjectId::new(number, 0))
            .expect("the assembly takes the object");
    }
    assembly.set_root(ObjectId::new(1, 0));
    let mut out = Vec::new();
    serialize_encrypted(
        &assembly,
        Version { major: 2, minor: 0 },
        Options::new(Form::Table),
        &Protection {
            user_password: "",
            owner_password: "keeper",
            access: Access::ALL,
            encrypt_metadata,
        },
        &mut Counter(0),
        &mut out,
    )
    .expect("the encrypted fixture is written");
    out
}

/// The title the packet of an updated encrypted file states, read back with the default
/// password.
fn title_after_update(encrypt_metadata: bool) -> Option<String> {
    let updated = set_information(encrypted_source(encrypt_metadata), "Title", "Annual report");
    let document = Document::open_with_password(updated, Limits::DEFAULT, "")
        .expect("the default user password opens the update");
    pdf_model::xmp::Xmp::document(&document)
        .expect("the update created a document-level packet")
        .ok()
        .and_then(|xmp| xmp.title().map(str::to_owned))
}

/// Control: with `/EncryptMetadata` true the created packet is encrypted and reads back.
#[test]
fn a_created_packet_reads_back_when_metadata_is_encrypted() {
    assert_eq!(title_after_update(true).as_deref(), Some("Annual report"));
}

/// Table 21's `/EncryptMetadata` (§7.6.4) decides whether "the document-level metadata stream
/// (see 14.3.2, "Metadata streams")" is encrypted, and a reader decrypts it under that answer.
/// `SetInformation` in a PDF 2.0 file creates the packet §14.3.1 sends the title to and names it
/// from a rewritten catalog, but the update encrypts it as an ordinary stream, because the
/// exemption is decided against the source's catalog, which names no packet: the file then
/// says its metadata is in the clear and holds ciphertext there.
#[test]
fn a_created_packet_honours_encrypt_metadata_false_spec_audit() {
    assert_eq!(
        title_after_update(false).as_deref(),
        Some("Annual report"),
        "under /EncryptMetadata false the document-level packet the update created is read in \
         the clear, so it has to have been written in the clear"
    );
}

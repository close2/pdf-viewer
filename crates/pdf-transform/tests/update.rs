//! The document edited **in place** — ISO 32000-2 §7.5.6's update, and what each edit claims.
//!
//! The property every case here rests on is the clause's own, and it is checked rather than
//! assumed: "changes shall be appended to the end of the file, leaving its original contents
//! intact", so every output begins with the source's bytes, byte for byte.

#![expect(
    clippy::expect_used,
    reason = "a test states what it needs and stops where it is not there"
)]

mod support;

use pdf_model::Pages;
use pdf_model::metadata::Information;
use pdf_model::page_label::PageLabels;
use pdf_model::xmp;
use pdf_syntax::Document;
use pdf_transform::update::{Edit, InfoEntry, UpdatePlan};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Refusal, Source, apply};

/// Runs one in-place edit and hands back the whole updated file.
fn amend(bytes: &[u8], edit: Edit, also: Option<&[u8]>) -> Result<(Vec<u8>, Vec<String>), Refusal> {
    let sinks = MemorySinks::new();
    let mut sources = vec![Source::new(bytes.to_vec())];
    if let Some(other) = also {
        sources.push(Source::new(other.to_vec()));
    }
    let report = apply(
        &Plan::Update(UpdatePlan {
            source: 0,
            edit,
            names: "out.pdf".parse().expect("a pattern"),
        }),
        &sources,
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )?;
    let out = sinks
        .into_outputs()
        .into_iter()
        .next()
        .expect("one output")
        .1;
    Ok((
        out,
        report
            .warnings
            .into_iter()
            .map(|warning| warning.detail)
            .collect(),
    ))
}

/// §7.5.6's own sentence, as a property: the update is *appended*, so the source is a prefix.
fn appended(source: &[u8], updated: &[u8]) {
    assert!(
        updated.len() > source.len(),
        "§7.5.6 appends, so the file grows"
    );
    assert_eq!(
        &updated[..source.len()],
        source,
        "§7.5.6: \"changes shall be appended to the end of the file, leaving its original \
         contents intact\""
    );
}

/// The document, opened.
fn read(bytes: &[u8]) -> Document {
    Document::open(bytes.to_vec()).expect("the update reads back")
}

/// A page taken out is a page the tree no longer holds, and every other page is where it was.
///
/// §7.7.3.2's `/Count` is "[t]he number of leaf nodes (page objects) that are descendants of
/// this node within the page tree", so the count falls by one; §12.4.2's indices "shall be
/// fixed, running consecutively through the document starting from 0 for the first page", so a
/// surviving page keeps the label it had at the position it now holds.
#[test]
fn a_deleted_page_leaves_the_tree_and_the_others_keep_their_labels() {
    let source = std::fs::read(support::committed("PDF20_AN001-BPC.pdf")).expect("a document");
    let before = read(&source);
    let count = Pages::new(&before).len();
    assert!(count >= 3, "the witness has pages to take one out of");
    let labels_before: Vec<Option<String>> = {
        let labels = PageLabels::read(&before);
        (0..count).map(|index| labels.label(index)).collect()
    };
    let texts_before: Vec<String> = {
        let pages = Pages::new(&before);
        (0..count)
            .map(|index| {
                let page = pages.get(index).expect("a page");
                pdf_model::interpret(&before, &page).text
            })
            .collect()
    };

    let (updated, warnings) = amend(&source, Edit::DeletePage { page: 2 }, None).expect("deleted");
    appended(&source, &updated);
    let after = read(&updated);
    assert_eq!(Pages::new(&after).len(), count - 1);
    assert!(
        warnings.iter().any(|detail| detail.contains("§7.5.6")),
        "the deletion says out loud that the bytes stay in the file: {warnings:?}"
    );

    let pages = Pages::new(&after);
    let labels = PageLabels::read(&after);
    let kept: Vec<usize> = (0..count).filter(|index| *index != 1).collect();
    for (position, source_index) in kept.into_iter().enumerate() {
        let page = pages.get(position).expect("a surviving page");
        assert_eq!(
            pdf_model::interpret(&after, &page).text,
            texts_before[source_index],
            "the page that was at {source_index} is now at {position}"
        );
        assert_eq!(
            labels.label(position),
            labels_before[source_index],
            "§12.4.2's label follows its page rather than its position"
        );
    }
}

/// The pages of another document arrive at the position the caller names, and everything that
/// was there moves down.
///
/// Table 31 gives a page one `/Parent`, so the carried pages are new objects in this document's
/// numbering; §7.7.3.4's four inheritable attributes are flattened onto them, because the
/// ancestors they would be inherited from are not coming with them.
#[test]
fn inserted_pages_arrive_at_the_position_named_and_the_incumbent_moves() {
    let source = std::fs::read(support::committed("PDF20_AN001-BPC.pdf")).expect("a document");
    let other = std::fs::read(support::committed("PDF20_AN002-AF.pdf")).expect("a document");
    let before = read(&source);
    let incoming = read(&other);
    let count = Pages::new(&before).len();
    let carried = Pages::new(&incoming).len();
    assert!(count >= 3 && carried >= 1);

    let text_of = |document: &Document, index: usize| {
        let pages = Pages::new(document);
        let page = pages.get(index).expect("a page");
        pdf_model::interpret(document, &page).text
    };
    let incumbent = text_of(&before, 2);
    let first_carried = text_of(&incoming, 0);

    let (updated, _) =
        amend(&source, Edit::InsertPages { from: 1, at: 3 }, Some(&other)).expect("inserted");
    appended(&source, &updated);
    let after = read(&updated);
    assert_eq!(Pages::new(&after).len(), count + carried);
    assert_eq!(
        text_of(&after, 2),
        first_carried,
        "the carried block starts where the name said"
    );
    assert_eq!(
        text_of(&after, 2 + carried),
        incumbent,
        "the page that was third is now after the block"
    );
}

/// One past the end appends, and a position outside the list is a refusal by name.
#[test]
fn a_position_past_one_past_the_end_is_refused() {
    let source = std::fs::read(support::committed("PDF20_AN001-BPC.pdf")).expect("a document");
    let other = std::fs::read(support::committed("PDF20_AN002-AF.pdf")).expect("a document");
    let count = Pages::new(&read(&source)).len();

    let (updated, _) = amend(
        &source,
        Edit::InsertPages {
            from: 1,
            at: count + 1,
        },
        Some(&other),
    )
    .expect("one past the end appends");
    assert_eq!(
        Pages::new(&read(&updated)).len(),
        count + Pages::new(&read(&other)).len()
    );

    match amend(
        &source,
        Edit::InsertPages {
            from: 1,
            at: count + 2,
        },
        Some(&other),
    ) {
        Err(Refusal::Position { position, .. }) => assert_eq!(position, count + 2),
        other => panic!("a position the list does not have is refused: {other:?}"),
    }
}

/// §14.3.3's entries set, and read back as the entries they were set to.
///
/// Table 349 makes eight of the nine text strings and `/Trapped` "a name object"; §7.9.4 makes a
/// date "a text string value" whose "prefix ' D: ' shall be present". Each of those is a refusal
/// where it is broken, and none of them is a value this writes anyway.
#[test]
fn the_information_dictionary_is_set_and_read_back() {
    // PDF 1.7, where the dictionary is every entry's home; a 2.0 file's is the test after next.
    let source = support::with_metadata("1.7", Some("/Author (Somebody)"), None);
    let (updated, _) = amend(
        &source,
        Edit::SetInformation {
            entries: vec![
                InfoEntry {
                    key: "Title".to_owned(),
                    value: Some("A title with a — dash".to_owned()),
                },
                InfoEntry {
                    key: "Trapped".to_owned(),
                    value: Some("True".to_owned()),
                },
                InfoEntry {
                    key: "ModDate".to_owned(),
                    value: Some("D:20260903120000Z".to_owned()),
                },
                InfoEntry {
                    key: "Author".to_owned(),
                    value: None,
                },
            ],
        },
        None,
    )
    .expect("set");
    appended(&source, &updated);
    let information = Information::read(&read(&updated));
    assert_eq!(information.title.as_deref(), Some("A title with a — dash"));
    assert_eq!(information.modified.as_deref(), Some("D:20260903120000Z"));
    assert_eq!(information.author, None);
    assert_eq!(
        information.trapped,
        pdf_model::metadata::Trapped::Fully,
        "Table 349: \"This shall be the name True , not the boolean value true .\""
    );
}

/// A key Table 349 does not define, a `/Trapped` that is not one of its three names, and a date
/// that is not §7.9.4's: three refusals rather than three values written into somebody's file.
#[test]
fn table_349_is_a_closed_list_and_two_of_its_types_are_checked() {
    let source = std::fs::read(support::committed("PDF20_AN001-BPC.pdf")).expect("a document");
    for entry in [
        InfoEntry {
            key: "Publisher".to_owned(),
            value: Some("nobody".to_owned()),
        },
        InfoEntry {
            key: "Trapped".to_owned(),
            value: Some("true".to_owned()),
        },
        InfoEntry {
            key: "CreationDate".to_owned(),
            value: Some("2026-09-03".to_owned()),
        },
    ] {
        let key = entry.key.clone();
        match amend(
            &source,
            Edit::SetInformation {
                entries: vec![entry],
            },
            None,
        ) {
            Err(Refusal::Pattern(_)) => {}
            other => panic!("{key} is refused by name: {other:?}"),
        }
    }
}

/// In a PDF 2.0 file every entry but the two dates goes into §14.3.2's packet alone.
///
/// §14.3.1: "Except for the `CreationDate` and `ModDate` entries, the use of the document
/// information dictionary for document metadata is deprecated in PDF 2.0", and §3.15 defines the
/// word as "a part of ISO 32000 that should not be written into a PDF 2.0 document". So a title
/// stated on a 2.0 file with no packet creates one, the catalog names it, the dictionary's old
/// title goes, and the date — which the clause excepts — is stated in both, as the same instant
/// (§14.3.4). An entry the operator did not state is the producer's and is left alone (ADR 1473).
#[test]
fn in_a_pdf_2_0_file_the_deprecated_entries_go_into_the_packet_alone() {
    let source = support::with_metadata(
        "2.0",
        Some("/Title (An old title) /Author (A producer's author)"),
        None,
    );
    let (updated, _) = amend(
        &source,
        Edit::SetInformation {
            entries: vec![
                InfoEntry {
                    key: "Title".to_owned(),
                    value: Some("A new title".to_owned()),
                },
                InfoEntry {
                    key: "Trapped".to_owned(),
                    value: Some("False".to_owned()),
                },
                InfoEntry {
                    key: "ModDate".to_owned(),
                    value: Some("D:20261001120000Z".to_owned()),
                },
            ],
        },
        None,
    )
    .expect("set");
    appended(&source, &updated);
    let document = read(&updated);
    assert_eq!(support::info_text(&document, "Title"), None);
    assert_eq!(support::info_text(&document, "Trapped"), None);
    assert_eq!(
        support::info_text(&document, "Author").as_deref(),
        Some("A producer's author"),
        "an entry nobody stated is the producer's"
    );
    assert_eq!(
        support::info_text(&document, "ModDate").as_deref(),
        Some("D:20261001120000Z")
    );
    let packet = support::document_packet(&document).expect("the catalog names a packet");
    assert_eq!(packet.text(xmp::DC, "title"), Some("A new title"));
    assert_eq!(packet.text(xmp::PDF, "Trapped"), Some("False"));
    assert_eq!(
        packet.text(xmp::XMP, "ModifyDate"),
        Some("2026-10-01T12:00:00Z"),
        "§14.3.4: \"fully equivalent\""
    );
}

/// In a file of an earlier version both sources state each entry, and say the same thing.
///
/// The dictionary is not deprecated before 2.0, so it keeps every entry; a packet the document
/// already holds is restated beside it under Table 349's NOTEs — NOTE 1's `dc:title`, NOTE 4's
/// `pdf:Keywords`, NOTE 5's `xmp:CreatorTool` — so the two never name two titles. The old title's
/// other language alternative goes with it, an entry removed leaves both, and a date is one
/// instant in each.
#[test]
fn in_an_earlier_file_both_sources_state_each_entry_and_agree() {
    let source = support::with_metadata(
        "1.7",
        Some("/Title (Old) /Keywords (old, words)"),
        Some(&support::packet(
            "<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">Old</rdf:li>\
             <rdf:li xml:lang=\"de\">Alt</rdf:li></rdf:Alt></dc:title>\n\
             <pdf:Keywords>old, words</pdf:Keywords>\n\
             <xmp:CreatorTool>An old tool</xmp:CreatorTool>\n\
             <pdf:Producer>Kept as it was</pdf:Producer>",
        )),
    );
    let (updated, warnings) = amend(
        &source,
        Edit::SetInformation {
            entries: vec![
                InfoEntry {
                    key: "Title".to_owned(),
                    value: Some("New & <escaped>".to_owned()),
                },
                InfoEntry {
                    key: "Creator".to_owned(),
                    value: Some("A new tool".to_owned()),
                },
                InfoEntry {
                    key: "Keywords".to_owned(),
                    value: None,
                },
                InfoEntry {
                    key: "ModDate".to_owned(),
                    value: Some("D:20261001083000-05'00".to_owned()),
                },
            ],
        },
        None,
    )
    .expect("set");
    appended(&source, &updated);
    assert!(warnings.is_empty(), "nothing to say: {warnings:?}");
    let document = read(&updated);
    let packet = support::document_packet(&document).expect("the packet is still there");
    for (key, namespace, local) in [
        ("Title", xmp::DC, "title"),
        ("Creator", xmp::XMP, "CreatorTool"),
        ("Keywords", xmp::PDF, "Keywords"),
    ] {
        assert_eq!(
            support::info_text(&document, key).as_deref(),
            packet.text(namespace, local),
            "/{key} and its Table 349 counterpart agree"
        );
    }
    assert_eq!(packet.text(xmp::DC, "title"), Some("New & <escaped>"));
    assert_eq!(packet.text_in(xmp::DC, "title", "de"), None);
    assert_eq!(packet.text(xmp::PDF, "Keywords"), None);
    assert_eq!(packet.text(xmp::PDF, "Producer"), Some("Kept as it was"));
    // §14.3.4's fourth rule: "a PDF processor shall ensure that the data in the document
    // information dictionary and the document level metadata stream -if both are written -are
    // fully equivalent" — one instant in each text's own spelling.
    assert_eq!(
        support::info_text(&document, "ModDate").as_deref(),
        Some("D:20261001083000-05'00")
    );
    assert_eq!(
        packet.text(xmp::XMP, "ModifyDate"),
        Some("2026-10-01T08:30:00-05:00")
    );
}

/// An earlier file with no packet is given none: its dictionary is the version's own method.
#[test]
fn an_earlier_file_with_no_packet_is_given_none() {
    let source = support::with_metadata("1.7", None, None);
    let (updated, _) = amend(
        &source,
        Edit::SetInformation {
            entries: vec![InfoEntry {
                key: "Subject".to_owned(),
                value: Some("A subject".to_owned()),
            }],
        },
        None,
    )
    .expect("set");
    let document = read(&updated);
    assert_eq!(
        support::info_text(&document, "Subject").as_deref(),
        Some("A subject")
    );
    assert!(support::document_packet(&document).is_none());
}

/// §14.12's hierarchy in the document being edited: a page list edit that would make it false
/// is refused by name, because an update appends and does not rebuild the hierarchy (ADR 1461).
///
/// "Each page object defined in the PDF file shall be included in the page range defined by one
/// and only one `DPart` dictionary", so a page inserted is in no leaf's range; and a page a leaf
/// names as its `/Start` cannot be taken out without that range naming nothing.
#[test]
fn an_edit_that_would_falsify_the_document_parts_is_refused() {
    let parts = support::document_parts();
    let other = std::fs::read(support::committed("PDF20_AN002-AF.pdf")).expect("a document");

    match amend(&parts, Edit::InsertPages { from: 1, at: 2 }, Some(&other)) {
        Err(Refusal::Assembly(reason)) => assert!(reason.contains("§14.12"), "{reason}"),
        other => panic!("an insertion into a document with parts is refused: {other:?}"),
    }
    match amend(&parts, Edit::DeletePage { page: 1 }, None) {
        Err(Refusal::Assembly(reason)) => assert!(reason.contains("/Start"), "{reason}"),
        other => panic!("taking out a part's first page is refused: {other:?}"),
    }
}

/// A page carried in from a document with parts states no `/DPart` in its new holder.
///
/// Table 31 permits the entry only "if this page is within the range of a `DPart`", and the
/// incoming hierarchy does not come with the page.
#[test]
fn a_carried_page_leaves_its_document_part_behind() {
    let source = std::fs::read(support::committed("PDF20_AN001-BPC.pdf")).expect("a document");
    let parts = support::document_parts();
    let (updated, _) =
        amend(&source, Edit::InsertPages { from: 1, at: 1 }, Some(&parts)).expect("it inserts");
    appended(&source, &updated);
    assert!(!support::states_document_parts(&read(&updated)));
}

/// An entry that changes nothing the document states touches neither source.
///
/// `pdf-vfs`'s `meta/info.json` is `update::stated_information`'s view, written back whole, so
/// restating what it read has to leave the producer's packet byte for byte: here a title the
/// packet states and the dictionary does not, and an `/Author` the dictionary states.
#[test]
fn restating_what_the_document_says_leaves_the_packet_alone() {
    let source = support::with_metadata(
        "2.0",
        Some("/Author (Somebody)"),
        Some(&support::packet(
            "<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">The producer's title</rdf:li>\
             </rdf:Alt></dc:title>",
        )),
    );
    let (updated, warnings) = amend(
        &source,
        Edit::SetInformation {
            entries: vec![
                InfoEntry {
                    key: "Title".to_owned(),
                    value: Some("The producer's title".to_owned()),
                },
                InfoEntry {
                    key: "Author".to_owned(),
                    value: Some("Somebody".to_owned()),
                },
            ],
        },
        None,
    )
    .expect("set");
    assert!(warnings.is_empty(), "{warnings:?}");
    let document = read(&updated);
    assert_eq!(
        pdf_transform::update::stated_information(&document),
        pdf_transform::update::stated_information(&read(&source))
    );
    let packet = support::document_packet(&document).expect("the packet");
    assert_eq!(packet.text(xmp::DC, "title"), Some("The producer's title"));
    assert_eq!(packet.text(xmp::DC, "creator"), None);
    assert_eq!(
        support::info_text(&document, "Author").as_deref(),
        Some("Somebody")
    );
}

/// A file whose table states object 3 with an entry this reader cannot read, under a `/Size` that
/// understates it, or states it free; `/Keep` names it from the catalog either way.
///
/// `entry` is the cross-reference line for object 3 and `body` the object itself, if there is one.
/// The catalog names it so that a writer reusing the number is visible as the catalog's reference
/// changing what it points at (ADR 2362).
fn with_object_three(entry: &str, body: &str) -> Vec<u8> {
    let mut file = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in [
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Keep 3 0 R >>\nendobj\n",
        "2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n",
        body,
    ] {
        offsets.push(file.len());
        file.extend_from_slice(object.as_bytes());
    }
    let xref = file.len();
    let table = format!(
        "xref\n0 4\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \n{entry}\n\
         trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets[0], offsets[1]
    );
    file.extend_from_slice(table.as_bytes());
    file
}

/// A title set on a file with no information dictionary, which gives the dictionary a new number.
fn title_set(source: &[u8]) -> Vec<u8> {
    amend(
        source,
        Edit::SetInformation {
            entries: vec![InfoEntry {
                key: "Title".to_owned(),
                value: Some("A title".to_owned()),
            }],
        },
        None,
    )
    .expect("set")
    .0
}

/// An object whose cross-reference entry was declared and not read keeps its number.
///
/// §7.5.4's subsection header states "the number of entries in the subsection", so object 3 is a
/// number the file stated even where its entry will not lex, and the object is still found by its
/// own `N G obj` header (ADR 0789). Table 15's `/Size` 3 understates it. A new object numbered from
/// the numbers the reader kept would be object 3, and the catalog's `/Keep 3 0 R` would then name
/// the new information dictionary.
#[test]
fn an_unread_entry_s_number_is_not_given_to_a_new_object() {
    let source = with_object_three("xxxxxxxxxx 00000 n ", "3 0 obj\n(kept)\nendobj\n");
    let three = pdf_syntax::ObjectId::new(3, 0);
    assert!(
        read(&source).get(three).as_string().is_some(),
        "the plant: object 3 resolves through its own header"
    );
    let updated = title_set(&source);
    appended(&source, &updated);
    let document = read(&updated);
    assert_eq!(
        document.get(three).as_string().map(<[u8]>::to_vec),
        Some(b"kept".to_vec()),
        "the catalog's /Keep 3 0 R still names the object the file wrote"
    );
    assert_eq!(
        Information::read(&document).title.as_deref(),
        Some("A title")
    );
}

/// A number whose entry is free is not given to a new object at generation 0.
///
/// §7.5.4: "[t]he entry's generation number shall be incremented by 1 to indicate the generation
/// number to be used the next time an object with that object number is created." Object 3 was
/// freed with its next generation 1 and `/Size` 3 understates it, so a new object 3 0 would both
/// break that sentence and turn the catalog's dangling `/Keep 3 0 R` into a reference to it.
#[test]
fn a_free_entry_s_number_is_not_given_to_a_new_object() {
    let source = with_object_three("0000000000 00001 f ", "");
    let three = pdf_syntax::ObjectId::new(3, 0);
    let updated = title_set(&source);
    appended(&source, &updated);
    let document = read(&updated);
    assert_eq!(
        document.get(three),
        pdf_syntax::Object::Null,
        "the catalog's /Keep 3 0 R still names nothing"
    );
    assert_eq!(
        Information::read(&document).title.as_deref(),
        Some("A title")
    );
}

/// A one-page document whose page shows `word` in Helvetica through an unfiltered content stream.
fn one_page_showing(word: &str) -> Vec<u8> {
    one_page_with(word, "")
}

/// [`one_page_showing`], its content stream's dictionary stating `entries` before `/Length`.
fn one_page_with(word: &str, entries: &str) -> Vec<u8> {
    let content = format!("BT /F1 12 Tf 20 100 Td ({word}) Tj ET");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 5 0 R \
         >> >> /Contents 4 0 R >>"
            .to_owned(),
        format!(
            "<< {entries}/Length {} >>\nstream\n{content}\nendstream",
            content.len().saturating_add(1)
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(
            format!("{} 0 obj\n{object}\nendobj\n", index.saturating_add(1)).as_bytes(),
        );
    }
    let xref = bytes.len();
    bytes.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    bytes
}

/// `clear` written again under §7.6.4's standard security handler at `/V` 5 (AES-256), with the
/// empty user password, so that it opens without one.
fn under_aes(clear: &[u8], owner: &str) -> Vec<u8> {
    use pdf_syntax::serialize::{Assembly, Form, Options};
    let document = read(clear);
    let mut assembly = Assembly::new(vec![&document]);
    for number in 1..=5 {
        assembly
            .copy(0, pdf_syntax::ObjectId::new(number, 0))
            .expect("the fixture's object copies");
    }
    assembly.set_root(pdf_syntax::ObjectId::new(1, 0));
    let mut out = Vec::new();
    pdf_syntax::serialize_encrypted(
        &assembly,
        pdf_syntax::Version { major: 1, minor: 7 },
        Options::new(Form::of(&document)),
        &pdf_syntax::Protection::owner_only(owner),
        &mut pdf_syntax::SystemEntropy,
        &mut out,
    )
    .expect("the fixture is encrypted");
    out
}

/// Inserts `incoming`'s page in front of `destination`'s and answers the updated file, after
/// checking the carried page's content stream: the `/Length` the update states is the number of
/// bytes it wrote, its `/Filter` names no §7.4.10 crypt filter, and the page shows `word`.
fn insert_and_check(destination: &[u8], destination_encrypted: bool, incoming: &[u8], word: &str) {
    let (updated, _) = amend(
        destination,
        Edit::InsertPages { from: 1, at: 1 },
        Some(incoming),
    )
    .expect("inserted");
    appended(destination, &updated);
    let after = read(&updated);
    let pages = Pages::new(&after);
    assert_eq!(pages.len(), 2);
    let page = pages.get(0).expect("the carried page");
    let contents = page
        .dict
        .get("Contents")
        .and_then(pdf_syntax::Object::as_reference)
        .expect("the carried page's contents are one stream");
    let stream = after.get(contents);
    let stream = stream.as_stream().expect("a stream");
    // The reader hands the data back out of §7.6's encryption, so what the file holds is that
    // plaintext in the clear, or — under the destination's AES — §7.6.3.1's sixteen-byte
    // initialisation vector and the plaintext padded to whole sixteen-byte blocks.
    let plaintext = stream.data.len();
    let written = if destination_encrypted {
        plaintext
            .div_euclid(16)
            .saturating_add(1)
            .saturating_mul(16)
            .saturating_add(16)
    } else {
        plaintext
    };
    assert_eq!(
        stream
            .dict
            .get("Length")
            .and_then(pdf_syntax::Object::as_integer),
        i64::try_from(written).ok(),
        "§7.3.8.2: the /Length states the bytes written, not the encrypted source's"
    );
    assert!(
        stream.dict.get("Filter").is_none(),
        "the source's stream was unfiltered, and §7.6's encryption is not a /Filter it carries"
    );
    assert!(
        after.repairs().is_empty(),
        "the update reads back with nothing repaired: {:?}",
        after.repairs()
    );
    assert_eq!(pdf_model::interpret(&after, &page).text.trim(), word);
}

/// ISO 32000-2 §7.3.8.2: "Every stream dictionary shall have a Length entry that indicates how
/// many bytes of the PDF file are used for the stream's data". A page carried out of an
/// AES-encrypted document holds its data decrypted (§7.6.2), sixteen bytes of initialisation
/// vector and the padding shorter than the source's `/Length` counted; into an unencrypted
/// document that plaintext is what is written, under a `/Length` that counts it.
#[test]
fn a_page_carried_out_of_an_encrypted_document_states_its_plaintext_length() {
    insert_and_check(
        &one_page_showing("HERE"),
        false,
        &under_aes(&one_page_showing("CARRIED"), "source-owner"),
        "CARRIED",
    );
}

/// The same page carried into a document encrypted under another key: the update encrypts it
/// under the destination's key (§7.6.2 on the way out), and its `/Length` counts that ciphertext.
#[test]
fn a_page_carried_between_two_encrypted_documents_states_its_ciphertext_length() {
    insert_and_check(
        &under_aes(&one_page_showing("HERE"), "destination-owner"),
        true,
        &under_aes(&one_page_showing("CARRIED"), "source-owner"),
        "CARRIED",
    );
}

/// `encrypted` with its page's content stream naming its crypt filter itself: §7.4.10's `/Crypt`
/// first in `/Filter`, Table 14's `/Name` the document's own `/StdCF`, written back by an update
/// that encrypts it under that filter (§7.6.6).
fn with_own_crypt_filter(encrypted: &[u8]) -> Vec<u8> {
    use pdf_syntax::{Dictionary, Name, Object, Stream};
    let document = read(encrypted);
    let page = Pages::new(&document).get(0).expect("a page");
    let id = page
        .dict
        .get("Contents")
        .and_then(Object::as_reference)
        .expect("one content stream");
    let read = document.get(id);
    let stream = read.as_stream().expect("a stream");
    let mut dict = stream.dict.clone();
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Array(vec![Object::Name(Name::new(&b"Crypt"[..]))]),
    );
    let mut parms = Dictionary::new();
    parms.insert(
        Name::new(&b"Name"[..]),
        Object::Name(Name::new(&b"StdCF"[..])),
    );
    dict.insert(
        Name::new(&b"DecodeParms"[..]),
        Object::Array(vec![Object::Dictionary(parms)]),
    );
    let replacement = Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: std::sync::Arc::clone(&stream.data),
        decryption_failed: false,
    }));
    let updated = pdf_syntax::write::incremental_update(
        &document,
        &std::collections::BTreeMap::from([(id, replacement)]),
    )
    .expect("the fixture's update is written");
    assert!(
        read_text(&updated).contains("CARRIED"),
        "the plant: the stream decrypts through its own /Crypt filter"
    );
    updated
}

/// Page one's text.
fn read_text(bytes: &[u8]) -> String {
    let document = read(bytes);
    let page = Pages::new(&document).get(0).expect("a page");
    pdf_model::interpret(&document, &page).text
}

/// ISO 32000-2 §7.4.10: the `Crypt` filter "allows the document-level security handler (see 7.6,
/// "Encryption") to determine which algorithms should be used to decrypt the input data", and
/// Table 14's `/Name` "shall correspond to an entry in the CF entry of the encryption
/// dictionary". The carried data is already out of the source's crypt filter, and the
/// destination's `/CF` is not the source's, so the stage goes and the destination's own §7.6.2
/// decides how the bytes are written. Kept, it names a filter an unencrypted destination does
/// not define, and the page reads back with no content.
#[test]
fn a_carried_stream_s_own_crypt_filter_stays_with_its_source() {
    insert_and_check(
        &one_page_showing("HERE"),
        false,
        &with_own_crypt_filter(&under_aes(&one_page_showing("CARRIED"), "source-owner")),
        "CARRIED",
    );
}

/// The same into an encrypted destination, which writes the bytes under its own `/StmF`.
#[test]
fn a_carried_stream_s_own_crypt_filter_gives_way_to_the_destination_s() {
    insert_and_check(
        &under_aes(&one_page_showing("HERE"), "destination-owner"),
        true,
        &with_own_crypt_filter(&under_aes(&one_page_showing("CARRIED"), "source-owner")),
        "CARRIED",
    );
}

/// A stream §7.6's decryption refused is refused by name rather than carried empty.
///
/// The document carried in states no encryption dictionary, and its content stream's `/Crypt`
/// stage names `/Secret`, which Table 14's `/Name` "shall correspond to an entry in the CF entry
/// of the encryption dictionary": there is none, so the reader holds the stream empty and marks
/// it, and writing it would replace the page's content with nothing.
#[test]
fn a_stream_whose_decryption_failed_is_refused_rather_than_carried_empty() {
    let bytes = one_page_with(
        "CARRIED",
        "/Filter /Crypt /DecodeParms << /Name /Secret >> ",
    );
    assert!(
        read_text(&bytes).trim().is_empty(),
        "the plant: the reader holds the stream empty"
    );
    let refused = amend(
        &one_page_showing("HERE"),
        Edit::InsertPages { from: 1, at: 1 },
        Some(&bytes),
    );
    assert!(
        matches!(&refused, Err(Refusal::Assembly(detail)) if detail.contains("decryption")),
        "{refused:?}"
    );
}

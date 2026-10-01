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

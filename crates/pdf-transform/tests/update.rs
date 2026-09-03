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
    let source = std::fs::read(support::committed("PDF20_AN001-BPC.pdf")).expect("a document");
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

/// Setting a date on a document that also states §14.3.2's stream is the case §14.3.4 is about,
/// and it is named rather than left to be found.
#[test]
fn a_date_beside_a_metadata_stream_is_named() {
    let source = std::fs::read(support::committed("PDF20_AN001-BPC.pdf")).expect("a document");
    let has_stream = {
        let document = read(&source);
        document
            .catalog()
            .ok()
            .map(|catalog| document.get_key(&catalog, "Metadata"))
            .is_some_and(|object| object.as_stream().is_some())
    };
    let (_, warnings) = amend(
        &source,
        Edit::SetInformation {
            entries: vec![InfoEntry {
                key: "ModDate".to_owned(),
                value: Some("D:20260903120000Z".to_owned()),
            }],
        },
        None,
    )
    .expect("set");
    assert_eq!(
        warnings.iter().any(|detail| detail.contains("§14.3.4")),
        has_stream,
        "the warning fires exactly where both sources exist: {warnings:?}"
    );
}

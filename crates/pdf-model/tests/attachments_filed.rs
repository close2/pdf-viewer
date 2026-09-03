//! §7.11.4's embedded files attached and detached through `ViewState`, and written by §7.5.6's
//! update — the viewer's half of the writer `pdf-transform attachments` consumes (ADR 0814).
//!
//! Every expected value is the standard's: §7.7.4's tree filed by key, Table 45's `/Size` and
//! `/CheckSum` from the bytes, §12.5.6.15's annotation carrying its `/FS` and its `/Contents`,
//! §7.5.6's "leaving its original contents intact" as the prefix property and its "marked as
//! deleted by means of their cross-reference entries" as a freed number answering `null`. What is
//! read back is read by this tree's own reader (`pdf_model::attachment`), which is what the
//! writer mirrors and what every host lists with.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture's object numbers are counted from a slice of six"
)]

use std::fmt::Write as _;

use pdf_model::attachment::filing::Payload;
use pdf_model::view::{AttachRefusal, Detached, Filing, FilingHome, ViewState};
use pdf_syntax::{Document, Object, ObjectId};

/// A file built from its objects, numbered from 1, with `/Root 1 0 R`.
fn document(objects: &[&str]) -> Document {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    );
    Document::open(out.into_bytes()).expect("a valid file")
}

/// One page, and one file already in §7.7.4's tree — object 5 the specification, 6 the stream.
fn with_one_embedded_file() -> Document {
    document(&[
        "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles 4 0 R >> >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] >>",
        "<< /Names [(old.txt) 5 0 R] >>",
        "<< /Type /Filespec /F (old.txt) /UF (old.txt) /EF << /F 6 0 R >> >>",
        "<< /Type /EmbeddedFile /Length 3 /Params << /Size 3 >> >>\nstream\nold\nendstream",
    ])
}

/// The one page's object, which is what a page home is filed against.
fn page_of(doc: &Document) -> ObjectId {
    pdf_model::Pages::new(doc)
        .get(0)
        .and_then(|page| page.id)
        .expect("the page has a number")
}

fn filing(name: &str, bytes: &[u8], home: FilingHome) -> Filing {
    Filing {
        bytes: Payload::new(bytes.to_vec()),
        name: name.to_owned(),
        description: Some(format!("about {name}")),
        media_type: Some("text/plain".to_owned()),
        home,
    }
}

/// §7.7.4: a file attached to the document is in the list at once, and in the tree after a
/// save — read back by the same reader, bytes and Table 45's checksum agreeing, with the
/// producer's bytes byte-for-byte a prefix of what was written (§7.5.6).
#[test]
fn a_file_attached_to_the_document_is_listed_now_and_in_the_tree_after_a_save() {
    let doc = with_one_embedded_file();
    let mut view = ViewState::of(&doc);
    view.attach(&doc, filing("new.txt", b"fresh", FilingHome::Document))
        .expect("a free name");

    let listed = view.attachments(&doc);
    assert_eq!(
        listed
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        ["old.txt", "new.txt"],
        "the document's own entry and then the one attached"
    );
    let new = &listed[1];
    assert_eq!(new.size, Some(5));
    assert_eq!(new.media_type.as_deref(), Some("text/plain"));
    assert_eq!(new.description.as_deref(), Some("about new.txt"));
    assert_eq!(
        doc.decoded_stream_data(&new.stream).as_deref(),
        Some(&b"fresh"[..]),
        "the stream the list carries is the one an extraction decodes"
    );
    assert_eq!(new.checksum_matches(b"fresh"), Some(true));

    let written = view.save(&doc).expect("the fixture can be updated");
    assert!(written.still_reached.is_empty());
    assert!(
        written
            .bytes
            .starts_with(doc.bytes().whole().expect("the fixture is held in memory")),
        "§7.5.6: the original contents are intact under the update"
    );
    let reopened = Document::open(written.bytes).expect("what was written is a PDF");
    let files = pdf_model::attachment::attachments(&reopened);
    assert_eq!(
        files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        ["new.txt", "old.txt"],
        "§7.9.6: the tree's keys in lexical order"
    );
    let new = &files[0];
    assert_eq!(new.file_name.as_deref(), Some("new.txt"), "Table 43's /UF");
    assert_eq!(new.size, Some(5), "Table 45's /Size");
    assert_eq!(
        reopened.decoded_stream_data(&new.stream).as_deref(),
        Some(&b"fresh"[..])
    );
    assert_eq!(
        new.checksum_matches(b"fresh"),
        Some(true),
        "Table 45's /CheckSum"
    );
    assert_eq!(
        reopened.decoded_stream_data(&files[1].stream).as_deref(),
        Some(&b"old"[..]),
        "the entry that was there is still reachable through the rewritten tree"
    );
}

/// §12.5.6.15: a file attached to a page is an annotation among the additions at once — so the
/// page draws its icon before any save — and not in the document-wide list; after a save the
/// page's `/Annots` reaches it and `of_annotation` reads the file with the annotation's own
/// `/Contents` as its description.
#[test]
fn a_file_attached_to_a_page_is_an_annotation_now_and_on_the_page_after_a_save() {
    let doc = with_one_embedded_file();
    let page = page_of(&doc);
    let mut view = ViewState::of(&doc);
    view.attach(
        &doc,
        filing(
            "figures.csv",
            b"1,2,3",
            FilingHome::Page {
                page,
                rect: [40.0, 40.0, 60.0, 60.0],
            },
        ),
    )
    .expect("a free name");

    let added: Vec<_> = view.added_on(Some(page)).collect();
    let [annotation] = added.as_slice() else {
        panic!("one annotation added: {added:?}");
    };
    assert_eq!(
        annotation
            .dict
            .get("Subtype")
            .and_then(Object::as_name)
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"FileAttachment"[..])
    );
    assert_eq!(
        annotation
            .dict
            .get("Name")
            .and_then(Object::as_name)
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"PushPin"[..]),
        "Table 187's default, always written"
    );
    assert_eq!(
        view.attachments(&doc)
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        ["old.txt"],
        "the list is the tree's home, and a page's file is on the page"
    );

    let written = view.save(&doc).expect("the fixture can be updated");
    let reopened = Document::open(written.bytes).expect("a PDF");
    let page = pdf_model::Pages::new(&reopened).get(0).expect("the page");
    let annotations = pdf_model::retrieval::annotations(&reopened, &page);
    let file = annotations
        .iter()
        .find_map(|annotation| pdf_model::attachment::of_annotation(&reopened, annotation))
        .expect("the page's annotation names an embedded file");
    assert_eq!(file.name, "figures.csv");
    assert_eq!(
        file.description.as_deref(),
        Some("about figures.csv"),
        "§12.5.6.15: the annotation's /Contents is the description"
    );
    assert_eq!(
        reopened.decoded_stream_data(&file.stream).as_deref(),
        Some(&b"1,2,3"[..])
    );
    assert!(
        pdf_model::attachment::attachments(&reopened)
            .iter()
            .all(|file| file.name == "old.txt"),
        "nothing was filed in the tree"
    );
}

/// §7.5.6: a detached entry of the document's own tree leaves the list at once and the tree at
/// the save, and what it alone reached is "marked as deleted by means of their cross-reference
/// entries" — the numbers answer `null` to the reader while the bytes stay under the update.
#[test]
fn a_detached_entry_leaves_the_tree_and_its_objects_are_marked_free() {
    let doc = with_one_embedded_file();
    let mut view = ViewState::of(&doc);
    assert_eq!(view.detach(&doc, "old.txt"), Detached::Unfiled);
    assert!(view.attachments(&doc).is_empty());
    assert_eq!(
        view.detach(&doc, "old.txt"),
        Detached::Nothing,
        "detached once is detached"
    );
    assert_eq!(view.detached().collect::<Vec<_>>(), ["old.txt"]);

    let written = view.save(&doc).expect("the fixture can be updated");
    assert!(
        written
            .bytes
            .starts_with(doc.bytes().whole().expect("the fixture is held in memory"))
    );
    let reopened = Document::open(written.bytes).expect("a PDF");
    assert!(pdf_model::attachment::attachments(&reopened).is_empty());
    for number in [5, 6] {
        assert!(
            matches!(reopened.get(ObjectId::new(number, 0)), Object::Null),
            "object {number} is marked free"
        );
    }
    assert!(
        matches!(reopened.get(ObjectId::new(4, 0)), Object::Dictionary(_)),
        "the old root's number carries the rewritten tree, so it is replaced and not freed"
    );
}

/// §7.11.4.1: a stream another home still reaches is not deleted by the tree letting go of it.
/// The entry leaves the tree, the objects stay in use, and `Written::still_reached` names the home.
#[test]
fn a_detached_entry_another_home_reaches_is_kept_in_use_and_said() {
    let doc = document(&[
        "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles 4 0 R >> /AF [5 0 R] >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] >>",
        "<< /Names [(source.xml) 5 0 R] >>",
        "<< /Type /Filespec /F (source.xml) /AFRelationship /Source /EF << /F 6 0 R >> >>",
        "<< /Type /EmbeddedFile /Length 3 >>\nstream\nxml\nendstream",
    ]);
    let mut view = ViewState::of(&doc);
    assert_eq!(view.detach(&doc, "source.xml"), Detached::Unfiled);
    let written = view.save(&doc).expect("the fixture can be updated");
    assert_eq!(
        written.still_reached,
        vec![("source.xml".to_owned(), "the catalog's /AF".to_owned())]
    );
    let reopened = Document::open(written.bytes).expect("a PDF");
    assert!(
        matches!(reopened.get(ObjectId::new(6, 0)), Object::Stream(_)),
        "the stream is still in use"
    );
    let files = pdf_model::attachment::attachments(&reopened);
    assert_eq!(
        files.len(),
        1,
        "the catalog's /AF still lists it, under its own name: {files:?}"
    );
    assert_eq!(
        files[0].relationship,
        pdf_model::attachment::Relationship::Source
    );
}

/// §7.9.6: one namespace, no overlap — with the document's tree, with this session, and across
/// both homes; and a name nothing files detaches nothing.
#[test]
fn a_name_is_filed_once_across_both_homes_and_an_unknown_name_detaches_nothing() {
    let doc = with_one_embedded_file();
    let page = page_of(&doc);
    let mut view = ViewState::of(&doc);
    assert_eq!(
        view.attach(&doc, filing("old.txt", b"x", FilingHome::Document)),
        Err(AttachRefusal::NameTaken),
        "the tree already files it"
    );
    view.attach(
        &doc,
        filing(
            "notes.txt",
            b"x",
            FilingHome::Page {
                page,
                rect: [0.0, 0.0, 20.0, 20.0],
            },
        ),
    )
    .expect("a free name");
    assert_eq!(
        view.attach(&doc, filing("notes.txt", b"y", FilingHome::Document)),
        Err(AttachRefusal::NameTaken),
        "a page's file and the tree's share the namespace"
    );
    assert_eq!(view.detach(&doc, "nowhere.txt"), Detached::Nothing);
    // A detached document entry frees its name for this session.
    assert_eq!(view.detach(&doc, "old.txt"), Detached::Unfiled);
    view.attach(&doc, filing("old.txt", b"again", FilingHome::Document))
        .expect("the name was released by the detach");
    let written = view.save(&doc).expect("the fixture can be updated");
    let reopened = Document::open(written.bytes).expect("a PDF");
    let files = pdf_model::attachment::attachments(&reopened);
    assert_eq!(files.len(), 1);
    assert_eq!(
        reopened.decoded_stream_data(&files[0].stream).as_deref(),
        Some(&b"again"[..]),
        "the tree's old.txt is the new one"
    );
}

/// A number handed out is never handed out again in one sitting: a file detached again does not
/// give its numbers to the next annotation, which the log names by number.
#[test]
fn object_numbers_are_never_reused_while_anything_allocated_is_held() {
    let doc = with_one_embedded_file();
    let page = page_of(&doc);
    let mut view = ViewState::of(&doc);
    view.attach(
        &doc,
        filing(
            "a.txt",
            b"a",
            FilingHome::Page {
                page,
                rect: [0.0, 0.0, 20.0, 20.0],
            },
        ),
    )
    .expect("a free name");
    let first_annotation = view.attached()[0].annotation.expect("a page home has one");
    let note = view
        .add_free_text(&doc, page, [10.0, 10.0, 90.0, 40.0], "", [0.0, 0.0, 0.0])
        .expect("a free text annotation");
    assert_eq!(view.detach(&doc, "a.txt"), Detached::Filed);
    assert!(
        view.added_on(Some(page)).all(|added| added.id == note),
        "the file's annotation went with it"
    );
    view.attach(&doc, filing("c.txt", b"c", FilingHome::Document))
        .expect("a free name");
    let again = &view.attached()[0];
    assert!(
        again.stream.number > note.number && again.specification.number > note.number,
        "fresh numbers past the note's ({}): {again:?}",
        note.number
    );
    assert_ne!(again.stream, first_annotation);
    assert_ne!(again.specification, first_annotation);

    // And the whole state cleared starts the numbering again, which a replay depends on.
    view.clear_all_additions();
    view.clear_all_free_text();
    view.clear_all_attachments();
    view.attach(&doc, filing("d.txt", b"d", FilingHome::Document))
        .expect("a free name");
    assert_eq!(
        view.attached()[0].stream.number,
        7,
        "the first number past the fixture's six objects"
    );
}

//! What ISO 32000-2 §7.5.6's update writes for a field a person changed.
//!
//! `viewer-core`'s `a_saved_document_carries_the_edit_and_the_file_under_it` is the end-to-end
//! statement — a keystroke goes in and a second viewer reads the value out of the saved bytes.
//! What is here is the half that end cannot see: **which objects the update contains**, and in
//! particular the two ways §12.7.4.3's appearance stream can be got wrong without changing what
//! this program itself draws.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use pdf_model::view::ViewState;
use pdf_syntax::object::ObjectId;
use pdf_syntax::{Document, Object};

/// One corpus document, or `None` when the submodule is not checked out.
fn corpus(name: &str) -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    std::fs::read(path).ok()
}

/// The document `160F-2019.pdf` becomes when `field` is set to `value`.
fn saved(name: &str, field: &str, value: &str) -> Option<Document> {
    let bytes = corpus(name)?;
    let document = Document::open(bytes).expect("the fixture opens");
    let mut view = ViewState::of(&document);
    let applied = view.set_field(&document, field, Some(value));
    assert!(applied > 0, "{name}: {field} is a field this document has");
    let out = view
        .save(&document)
        .expect("the fixture can be written")
        .bytes;
    Some(Document::open(out).expect("what was written can be read"))
}

/// The object `key` names in `dict`, resolved.
fn entry(document: &Document, id: ObjectId, key: &str) -> Object {
    let object = document.get(id);
    let dict = object.as_dict().expect("the object is a dictionary");
    document.get_key(dict, key)
}

#[test]
fn a_text_widget_with_no_appearance_stream_is_given_one() {
    // §7.3.8.1 makes every stream an indirect object, so a widget that had no `/AP` needs an
    // object *added* rather than replaced — which is the half of §7.5.6's "changed, replaced, or
    // deleted" this writer did not do until the hundred-and-forty-fifth session.
    let Some(document) = saved("160F-2019.pdf", "X.minus1", "Ada Lovelace") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };

    // Object 417 is a text widget whose original file states no `/AP` at all.
    let widget = ObjectId {
        number: 417,
        generation: 0,
    };
    let appearances = entry(&document, widget, "AP");
    let normal = appearances
        .as_dict()
        .map(|appearances| document.get_key(appearances, "N"))
        .expect("the widget now has an /AP");
    let stream = normal.as_stream().expect("its /N is a stream");

    // §8.10.2's required entries, and §12.7.4.3's marked-content region inside them.
    assert_eq!(
        document
            .get_key(&stream.dict, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"Form".to_vec())
    );
    assert!(document.get_key(&stream.dict, "BBox").as_array().is_some());
    let content = document
        .decoded_stream_data(stream)
        .expect("the written stream decodes");
    let content = String::from_utf8_lossy(&content);
    assert!(content.contains("/Tx BMC"), "{content}");
    assert!(content.contains("Ada Lovelace"), "{content}");
}

#[test]
fn a_check_boxs_states_are_not_replaced_by_one_stream() {
    // §12.7.5.2.3 makes a check box's states "defined by an appearance stream in the appearance
    // dictionary of the field's widget annotation", and the value *selects* among them. A writer
    // that regenerated every widget's appearance the way it regenerates a text field's would
    // replace that subdictionary with a single stream and lose the off state — a page that still
    // renders, in this program and in every other, until somebody clicks the box.
    //
    // The check box is the field *being changed*, which is what makes this discriminating: a
    // save only touches the widgets a person edited, so setting some other field would leave
    // this one alone however wrong the writer was.
    // `bug1675139.pdf`'s `C1` is the fixture rather than any check box, because it is one whose
    // `/MK` states a background *and* a border: a check box that states neither draws nothing at
    // all, so a writer without the guard would produce no stream for it and the test would pass
    // for the wrong reason.
    let Some(document) = saved("bug1675139.pdf", "C1", "On") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };

    // Object 50 is that check box, and its `/AP` `/N` is a dictionary of states.
    let widget = ObjectId {
        number: 50,
        generation: 0,
    };
    let appearances = entry(&document, widget, "AP");
    let normal = appearances
        .as_dict()
        .map(|appearances| document.get_key(appearances, "N"))
        .expect("the check box still has an /AP");
    // `matches!` rather than `as_dict()`, and the difference is the whole test: `Object::as_dict`
    // answers for a *stream* too, so a `/N` replaced by one passes an `is_some()` check on it.
    // Confirmed by removing the guard in `appearance::for_saving` — the states really are
    // replaced, and the first version of this assertion did not notice.
    assert!(
        matches!(normal, Object::Dictionary(_)),
        "the check box's states survive: {normal:?}"
    );
}

#[test]
fn nothing_is_owed_to_the_next_reader_when_every_stream_was_written() {
    // Table 224's `/NeedAppearances` asks the next reader to construct what this program could
    // not. A document where it *could* must not set it — a reader that honours the flag would
    // redo work that is already correct, and one that does not would be right to ignore it.
    let Some(document) = saved("form_two_pages.pdf", "Text1", "Ada Lovelace") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let catalog = document.catalog().expect("a /Root");
    let form = document.get_key(&catalog, "AcroForm");
    let flag = form
        .as_dict()
        .map(|form| document.get_key(form, "NeedAppearances"));
    assert!(
        !matches!(flag, Some(Object::Boolean(true))),
        "{flag:?} — every changed widget got a stream, so nothing is owed"
    );
}

/// §14.3.4's one requirement that binds a program which modifies an existing document.
///
/// The clause states four rules for a *writer* and the second is the only one this tree can
/// break:
///
/// > When writing modifications to an existing PDF document, if the PDF document already contains
/// > time and date of creation in both the document information dictionary and in the document's
/// > metadata stream, and the two are not equivalent, a PDF processor should leave the
/// > inconsistent values unchanged.
///
/// It is met by construction rather than by code, and that is worth a test rather than a
/// sentence: §7.5.6's update *appends*, `carry_forward` repeats the trailer's `/Info` as the same
/// indirect reference, and the catalog's `/Metadata` is never among the objects written. So both
/// sources survive a save byte for byte, whatever they said and whether or not they agreed.
///
/// The other three rules are conditioned on writing a date, which this program does not do, and
/// **that is the deliberate part**: adding a `/ModDate` on save would put the clause's fourth
/// rule — a `shall` — into force, and satisfying it would mean writing `xmp:ModifyDate` into the
/// document's metadata stream as well. Table 349 makes `/ModDate` optional except beside a
/// `/PieceInfo` this program never adds, so the cost of the date is an XMP *writer* and the
/// benefit is a field nobody asked for.
#[test]
fn saving_leaves_both_of_a_documents_metadata_sources_exactly_as_they_were() {
    let Some(original) = corpus("160F-2019.pdf") else {
        println!("the pdf.js submodule is not checked out; skipping");
        return;
    };
    let before = Document::open(original).expect("the fixture opens");
    let information = pdf_model::metadata::Information::read(&before);
    assert!(
        information.created.is_some() && information.modified.is_some(),
        "the fixture states both of Table 349's dates, which is what the rule is about"
    );
    let packet = pdf_model::xmp::Xmp::document(&before)
        .expect("the fixture carries §14.3.2's stream")
        .expect("which reads");

    let after = saved("160F-2019.pdf", "X.minus1", "Ada Lovelace")
        .expect("the fixture is checked out and has that field");

    assert_eq!(
        pdf_model::metadata::Information::read(&after),
        information,
        "§14.3.3's dictionary is repeated by reference and not rewritten"
    );
    assert_eq!(
        pdf_model::xmp::Xmp::document(&after)
            .expect("the stream is still named")
            .expect("and still reads"),
        packet,
        "§14.3.2's stream is not among the objects an update writes"
    );
}

/// §12.5.6.10's markup a person added reaches the file, and reaches its page.
///
/// Two objects, because §12.5.2 says where an annotation lives: the annotation itself, and the
/// page's `/Annots` with the reference appended. Both are checked by reading the saved bytes
/// back with this crate's own parser, which is the only reader that can be pointed at an
/// arbitrary object number — `viewer-core`'s end-to-end test says the picture changes and cannot
/// say which objects carry it.
#[test]
fn a_markup_a_person_added_is_written_and_attached_to_its_page() {
    let Some(bytes) = corpus("160F-2019.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let document = Document::open(bytes.clone()).expect("the fixture opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("page one")
        .id
        .expect("a page reached through the tree is an object");
    let before = match entry(&document, page, "Annots") {
        Object::Array(entries) => entries.len(),
        _ => 0,
    };

    let mut view = ViewState::of(&document);
    let added = view
        .add_markup(
            &document,
            page,
            pdf_model::view::Markup::Highlight,
            [1.0, 1.0, 0.0],
            &[[100.0, 700.0, 300.0, 700.0, 100.0, 690.0, 300.0, 690.0]],
        )
        .expect("one quadrilateral is something to mark up");
    let out = view.save(&document).expect("it can be written").bytes;
    assert!(
        out.starts_with(&bytes),
        "§7.5.6 appends: the producer's bytes are untouched underneath"
    );

    let saved = Document::open(out).expect("what was written can be read");
    assert_eq!(
        entry(&saved, added, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"Highlight".to_vec())
    );
    // Read raw rather than through `entry`, which resolves: what is being checked is that the
    // entry *is* a reference, which Table 166 requires of `/P`.
    let annotation = saved.get(added);
    assert_eq!(
        annotation
            .as_dict()
            .and_then(|dict| dict.get("P"))
            .and_then(Object::as_reference),
        Some(page),
        "Table 166's /P names the page the annotation was added to"
    );
    let quads = entry(&saved, added, "QuadPoints");
    assert_eq!(
        quads.as_array().map(<[Object]>::len),
        Some(8),
        "one quadrilateral is eight numbers"
    );

    // And the marks themselves, because a reader that constructs no appearance would otherwise
    // show the page unmarked (ADR 0130's argument, one clause over). §12.5.5 maps a form's
    // `/BBox` onto its `/Rect`, and §12.5.6.10's quadrilaterals are in default user space, so
    // the two rectangles are the same one and the map is the identity.
    let appearance = entry(&saved, added, "AP");
    let normal = appearance
        .as_dict()
        .map(|appearances| saved.get_key(appearances, "N"))
        .expect("the annotation carries an /AP");
    let stream = normal.as_stream().expect("its /N is a stream");
    assert_eq!(
        saved.get_key(&stream.dict, "BBox").as_array().map(|box_| {
            box_.iter()
                .filter_map(Object::as_number)
                .collect::<Vec<f64>>()
        }),
        entry(&saved, added, "Rect").as_array().map(|rect| {
            rect.iter()
                .filter_map(Object::as_number)
                .collect::<Vec<f64>>()
        }),
        "the appearance's box is the annotation's rectangle"
    );
    let content = saved
        .decoded_stream_data(stream)
        .expect("the stream this program wrote decodes");
    let content = String::from_utf8_lossy(&content);
    assert!(
        content.contains("gs"),
        "§11.3.5.2's mode is what keeps the text under a highlight readable: {content}"
    );
    // And the state that operator names is in the stream's own resources, which is the half a
    // content stream cannot state: a `gs` naming nothing draws in `Normal` and hides the words.
    let named = saved
        .get_key(&stream.dict, "Resources")
        .as_dict()
        .map(|resources| saved.get_key(resources, "ExtGState"))
        .and_then(|states| states.as_dict().cloned())
        .expect("the appearance carries the graphics state it names");
    let blend = named
        .iter()
        .next()
        .map(|(_, state)| saved.resolve(state))
        .and_then(|state| state.as_dict().map(|state| saved.get_key(state, "BM")))
        .and_then(|mode| mode.as_name().map(|name| name.as_bytes().to_vec()));
    assert_eq!(blend, Some(b"Multiply".to_vec()));

    // And the page names it, after whatever it already had: §12.5.2 makes the array's order the
    // drawing order, so a mark made last is drawn on top.
    let annotations = entry(&saved, page, "Annots");
    let entries = annotations.as_array().expect("the page has an /Annots now");
    assert_eq!(entries.len(), before + 1);
    assert_eq!(
        entries.last().and_then(Object::as_reference),
        Some(added),
        "appended rather than inserted"
    );
}

/// §12.5.6.6's annotation, written whole, with the `/DR` §12.7.4.3 requires beside it.
///
/// The free text half of the test above, and it asks one thing that one cannot: §12.7.4.3 puts a
/// `shall` on the `/DA` this program *writes*, and it is about a different dictionary —
///
/// > The specified font value shall match a resource name in the Font entry of the default
/// > resource dictionary (referenced from the DR entry of the interactive form dictionary; see
/// > "Table 224 -Entries in the interactive form dictionary").
///
/// — so a file this program produced that named a font `/DR` did not define would be a file
/// breaking a clause this program otherwise recovers six corpus documents from. `alphatrans.pdf`
/// states no interactive form dictionary at all, which is the case that has to build one.
#[test]
fn a_free_text_annotation_carries_the_font_its_default_appearance_names() {
    let Some(bytes) = corpus("alphatrans.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let document = Document::open(bytes.clone()).expect("the fixture opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("page one")
        .id
        .expect("a page reached through the tree is an object");
    assert!(
        document
            .catalog()
            .expect("the fixture has a catalog")
            .get("AcroForm")
            .is_none(),
        "the point of this fixture is that it has no interactive form dictionary"
    );

    let mut view = ViewState::of(&document);
    let added = view
        .add_free_text(
            &document,
            page,
            [72.0, 600.0, 300.0, 680.0],
            "note",
            [0.7, 0.1, 0.1],
        )
        .expect("a rectangle with area is something to write in");
    let out = view
        .save(&document)
        .expect("the fixture can be written")
        .bytes;
    assert!(
        out.starts_with(&bytes),
        "§7.5.6 appends: the producer's bytes are untouched underneath"
    );
    let saved = Document::open(out).expect("what was written can be read");

    assert_eq!(
        entry(&saved, added, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"FreeText".to_vec()),
        "Table 177 makes the subtype Required"
    );
    let contents = entry(&saved, added, "Contents");
    let contents = contents
        .as_string()
        .expect("Table 166's /Contents is a string");
    assert_eq!(
        pdf_syntax::text_string::text_string(contents),
        "note",
        "the text is what a person typed"
    );
    let default_appearance = entry(&saved, added, "DA");
    let default_appearance = String::from_utf8_lossy(
        default_appearance
            .as_string()
            .expect("Table 177 makes /DA Required"),
    )
    .into_owned();
    assert!(
        default_appearance.contains("/Helv 12 Tf") && default_appearance.contains("rg"),
        "§12.7.4.3: the string shall include a Tf with its two operands — {default_appearance}"
    );

    // The `shall` this test exists for. Table 224's `/DR` is where §12.7.4.3 says the name is
    // resolved, and the whole chain — catalog, form, `/DR`, `/Font` — had to be created here.
    let catalog = saved.catalog().expect("the saved file has a catalog");
    let form = saved.get_key(&catalog, "AcroForm");
    let form = form.as_dict().expect("an interactive form dictionary now");
    assert!(
        matches!(saved.get_key(form, "Fields"), Object::Array(ref fields) if fields.is_empty()),
        "Table 224 makes /Fields Required, and this document has no fields"
    );
    let resources = saved.get_key(form, "DR");
    let resources = resources.as_dict().expect("Table 224's /DR");
    let fonts = saved.get_key(resources, "Font");
    let fonts = fonts.as_dict().expect("its /Font");
    let helvetica = saved.get_key(fonts, "Helv");
    let helvetica = helvetica.as_dict().expect("the name the /DA states");
    assert_eq!(
        saved
            .get_key(helvetica, "BaseFont")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"Helvetica".to_vec()),
        "one of §9.6.2.2's fourteen, which is what /Helv denotes"
    );

    // And the appearance, because a reader that constructs none would show the page unmarked.
    let appearance = entry(&saved, added, "AP");
    let normal = appearance
        .as_dict()
        .map(|appearances| saved.get_key(appearances, "N"))
        .expect("the annotation carries an /AP");
    let stream = normal.as_stream().expect("its /N is a stream");
    let content = saved
        .decoded_stream_data(stream)
        .expect("the stream this program wrote decodes");
    let content = String::from_utf8_lossy(&content);
    assert!(
        content.contains("Tj") || content.contains("TJ"),
        "§12.5.6.6's text is what the annotation is, so the stream shows some: {content}"
    );
}

/// A document that already defines the name is left exactly as it was.
///
/// §12.7.4.3's sentence is satisfied the moment `/DR` states the name, and what it states is then
/// the document's own opinion about its own resource — the same rule `variable_text`'s
/// `Resolution::Named` follows when drawing. `160F-2019.pdf` defines `/Helv` already.
#[test]
fn a_documents_own_default_font_is_not_replaced() {
    let Some(bytes) = corpus("160F-2019.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let document = Document::open(bytes).expect("the fixture opens");
    let catalog = document.catalog().expect("a catalog");
    let form = catalog
        .get("AcroForm")
        .and_then(Object::as_reference)
        .expect("this fixture states its form indirectly");
    let before = document.get(form);
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("page one")
        .id
        .expect("a page reached through the tree is an object");

    let mut view = ViewState::of(&document);
    view.add_free_text(
        &document,
        page,
        [72.0, 600.0, 300.0, 680.0],
        "note",
        [0.0, 0.0, 0.0],
    )
    .expect("a rectangle with area is something to write in");
    let out = view
        .save(&document)
        .expect("the fixture can be written")
        .bytes;
    let saved = Document::open(out).expect("what was written can be read");
    assert_eq!(
        format!("{:?}", saved.get(form)),
        format!("{before:?}"),
        "the form dictionary is untouched where /DR already names the font"
    );
}

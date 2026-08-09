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

use pdf_model::view::Entered;
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
    let applied = view.set_field(&document, field, &Entered::Text(value.to_owned()));
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

/// A one-page document with the choice field the caller spells.
///
/// The `/Opt` array is Table 234's second form throughout — "an array consisting of two text
/// strings: the option's export value and the text that shall be displayed as the name of the
/// option" — because that is the form that makes the export value and the label two different
/// strings, and §12.7.5.4 says only the second reaches `/V`.
fn choice_document(flags: i64, entries: &str) -> Document {
    let objects = format!(
        "1 0 obj << /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >> endobj\n\
         2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
         3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R] >> endobj\n\
         4 0 obj << /Type /Annot /Subtype /Widget /Rect [10 20 210 120] /FT /Ch /T (databases) \
         /Ff {flags} /DA (/Helv 9 Tf 0 g) {entries} \
         /Opt [[(oracle) (Oracle)] [(sqlServer) (SQL Server)] [(db2) (DB2)] [(pg) (PostgreSQL)]] \
         >> endobj\n"
    );
    Document::open(assembled(&objects)).expect("the fixture parses")
}

/// A fixture with §7.5.4's cross-reference table actually written.
///
/// Not a nicety: `pdf_syntax::write::incremental_update` refuses a document whose table it had to
/// rebuild by scanning (`UpdateError::Recovered`), because §7.5.6's update is defined against the
/// offsets the *file* states and this program may not invent them.
fn assembled(objects: &str) -> Vec<u8> {
    use std::fmt::Write as _;

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in objects.split_inclusive("endobj\n") {
        if !object.contains(" 0 obj") {
            continue;
        }
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Table 233 bit 22 set, and what §12.7.5.4 then says `/V` and Table 234's `/I` are.
///
/// > If multiple items are selected, V is an array of such strings.
#[test]
fn several_selected_items_are_written_as_an_array_and_an_index_list() {
    // Bit 22 is 1 << 21.
    let document = choice_document(1 << 21, "");
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "databases", &Entered::Chosen(vec![2, 0])),
        1,
        "one widget takes it"
    );

    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let described = pdf_model::form::fields(&document, &page, &view);
    let pdf_model::form::Control::Choice(choice) = &described[0].control else {
        panic!("{:?}", described[0].control)
    };
    assert_eq!(
        choice.selected,
        vec![0, 2],
        "ascending, whatever order the host clicked in"
    );

    let out = view.save(&document).expect("the fixture writes").bytes;
    let saved = Document::open(out).expect("what was written can be read");
    let field = ObjectId {
        number: 4,
        generation: 0,
    };
    let value = entry(&saved, field, "V");
    let items = value.as_array().expect("/V is an array for several items");
    let labels: Vec<String> = items
        .iter()
        .filter_map(|item| item.as_string().map(pdf_syntax::text_string))
        .collect();
    assert_eq!(
        labels,
        vec!["Oracle".to_owned(), "DB2".to_owned()],
        "the name string is the second of the two array elements"
    );
    let indices = entry(&saved, field, "I");
    let indices: Vec<i64> = indices
        .as_array()
        .expect("Table 234's /I")
        .iter()
        .filter_map(Object::as_integer)
        .collect();
    assert_eq!(indices, vec![0, 2], "sorted in ascending order");

    // And the file says the same thing to a reader that has never seen this session: `/V` and
    // `/I` agree, so §12.7.5.4's tie-break has nothing to arbitrate.
    let reopened = ViewState::of(&saved);
    let page = pdf_model::Pages::new(&saved).get(0).expect("page one");
    let described = pdf_model::form::fields(&saved, &page, &reopened);
    let pdf_model::form::Control::Choice(choice) = &described[0].control else {
        panic!("{:?}", described[0].control)
    };
    assert_eq!(choice.selected, vec![0, 2]);
}

/// Table 233 bit 22 clear: "if clear, at most one item shall be selected".
#[test]
fn a_single_select_choice_field_takes_the_first_of_what_was_chosen() {
    let document = choice_document(0, "");
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "databases", &Entered::Chosen(vec![1, 3])),
        1
    );
    let out = view.save(&document).expect("the fixture writes").bytes;
    let saved = Document::open(out).expect("what was written can be read");
    let field = ObjectId {
        number: 4,
        generation: 0,
    };
    // "V is a text string representing the selected item", not an array of one.
    let value = entry(&saved, field, "V");
    assert_eq!(
        value.as_string().map(pdf_syntax::text_string),
        Some("SQL Server".to_owned()),
        "{value:?}"
    );
    let indices: Vec<i64> = entry(&saved, field, "I")
        .as_array()
        .expect("Table 234's /I")
        .iter()
        .filter_map(Object::as_integer)
        .collect();
    assert_eq!(indices, vec![1]);
}

/// An index past the end of Table 234's `/Opt` names no option, and none left is no selection.
///
/// ISO 32000-2 §12.7.5.4:
///
/// > The default value of V is null , indicating that no item is currently selected.
#[test]
fn a_selection_of_nothing_removes_both_entries() {
    // The file states a selection of its own, so what is being tested is a *removal*.
    let document = choice_document(1 << 21, "/V [(Oracle) (DB2)] /I [0 2]");
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "databases", &Entered::Chosen(vec![9])),
        1,
        "the edit applies; it is the index that names nothing"
    );
    let out = view.save(&document).expect("the fixture writes").bytes;
    let saved = Document::open(out).expect("what was written can be read");
    let field = ObjectId {
        number: 4,
        generation: 0,
    };
    assert_eq!(entry(&saved, field, "V"), Object::Null, "V is removed");
    assert_eq!(entry(&saved, field, "I"), Object::Null, "and so is /I");
}

/// A `/I` the file states does not survive a value that is not a selection.
///
/// Table 234 defines the entry against `/Opt` positions, and §12.7.5.4 makes the value decide
/// where they disagree — so leaving a stale one beside a new `/V` would write a file whose two
/// entries describe different selections and rely on the reader's tie-break to hide it.
#[test]
fn typing_into_a_combo_box_takes_the_stated_index_list_out() {
    // Bit 18 (Combo) and bit 19 (Edit): "the user can type a value other than the predefined
    // choices".
    let document = choice_document((1 << 17) | (1 << 18), "/V (DB2) /I [2]");
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(
            &document,
            "databases",
            &Entered::Text("something else".to_owned())
        ),
        1
    );
    let out = view.save(&document).expect("the fixture writes").bytes;
    let saved = Document::open(out).expect("what was written can be read");
    let field = ObjectId {
        number: 4,
        generation: 0,
    };
    assert_eq!(
        entry(&saved, field, "V")
            .as_string()
            .map(pdf_syntax::text_string),
        Some("something else".to_owned())
    );
    assert_eq!(entry(&saved, field, "I"), Object::Null);
}

/// [`Entered::Chosen`] on a field that is not §12.7.5.4's is refused rather than reinterpreted.
///
/// Table 230's `/Opt` is a *button's* export values and carries the same key, so resolving an
/// index against it would write an export value where §12.7.5.2.3 wants an appearance-state name.
#[test]
fn choosing_an_option_of_a_text_field_applies_to_nothing() {
    let objects = "1 0 obj << /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >> \
                   endobj\n\
                   2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
                   3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R] \
                   >> endobj\n\
                   4 0 obj << /Type /Annot /Subtype /Widget /Rect [10 20 210 60] /FT /Tx \
                   /T (address) /Opt [(one) (two)] >> endobj\n";
    let document = Document::open(assembled(objects)).expect("the fixture parses");
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "address", &Entered::Chosen(vec![0])),
        0
    );
    assert_eq!(view.edits().count(), 0, "and nothing is logged");
}

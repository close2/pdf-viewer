//! ADRs 1724 and 1725 driven through `pdf_model::view::ViewState` with the in-process engine: the
//! `Doc` members a view state answers — the pages, a named destination, `title`, `calculate`,
//! `app.activeDocs` — and a choice field's options read from `/Opt`, chosen by index, and
//! rewritten (ADR 1737).
//!
//! Every expected value is the clause the member reads — §12.4.2's labels, Table 31's boxes and
//! `/Rotate`, §12.3.2.4's named destinations, Table 230's and Table 234's `/Opt`, §12.7.5.4's `/V`
//! and `/I` — or this program's documented choice in the ADR the member cites, where Adobe's
//! reference states the member and not its answer (rotated user space, an unlabelled page's label).

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::view::{Entered, ViewState};
use pdf_script::{Budget, Engine};
use pdf_syntax::Document;

/// A document of `bodies`, object 1 its catalog, with `trailer` in its trailer.
fn document(bodies: &[String], trailer: &str) -> Document {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = bodies.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R {trailer} >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// References to `numbers`, as an array's contents.
fn references(numbers: &[u32]) -> String {
    numbers
        .iter()
        .map(|number| format!("{number} 0 R"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Two pages, the first turned a quarter with a crop box inset, the second its sibling; labels
/// `i` and `A-5`; a named destination on page two; the text field `Out` (object 6) on page one;
/// `objects` from object 7, of which `fields` are the form's further fields — `form` the rest of
/// its dictionary — and `annots` page one's further annotations; and `open` as the document's open
/// action.
fn two_pages(
    open: &str,
    form: &str,
    (fields, annots): (&[u32], &[u32]),
    objects: &[String],
) -> Document {
    let fields = references(&[&[6], fields].concat());
    let annots = references(&[&[6], annots].concat());
    let mut bodies = vec![
        format!(
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [{fields}] {form} >> \
             /PageLabels << /Nums [0 << /S /r >> 1 << /P (A-) /S /D /St 5 >>] >> \
             /Dests << /chapter2 [5 0 R /Fit] >> \
             /OpenAction << /S /JavaScript /JS ({open}) >> >>"
        ),
        "<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 /MediaBox [0 0 612 792] >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /CropBox [10 36 600 780] /Rotate 90 /Annots [{annots}] >>"
        ),
        "<< /Info 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Out) /Rect [10 10 210 40] /F 4 /P 3 0 R \
         /DA (/Helv 10 Tf 0 g) >>"
            .to_owned(),
    ];
    bodies.extend(objects.iter().cloned());
    document(&bodies, "/Info 4 0 R")
}

/// No further fields and no further annotations.
const NONE: (&[u32], &[u32]) = (&[], &[]);

/// A view state of `document` with the engine supplied and the document opened.
fn opened(document: &Document) -> ViewState {
    let mut view = ViewState::of(document);
    view.run_scripts_with(Some(Arc::new(Engine::new(Budget::FIELD_EVENT))));
    view.run_open_scripts(document, 0);
    view
}

/// What a field shows, as text.
fn value(view: &ViewState, document: &Document, name: &str) -> String {
    view.field_value(document, name)
        .map(|shown| shown.text)
        .unwrap_or_default()
}

#[test]
fn a_page_s_label_rotation_and_boxes_are_the_file_s() {
    let document = two_pages(
        "this.getField\\('Out'\\).value = [getPageLabel\\(0\\), getPageLabel\\(1\\), \
         getPageRotation\\(0\\), getPageRotation\\(1\\), getPageBox\\('Crop', 0\\), \
         getPageBox\\({cBox: 'Media', nPage: 1}\\), getPageBox\\(\\)].join\\('|'\\);",
        "",
        NONE,
        &[],
    );
    let view = opened(&document);
    // §12.4.2: page one is in an `/r` range, page two in a `/D` range prefixed `A-` from 5;
    // Table 31: the first page's `/Rotate` is its own, the second inherits none; ADR 1724's rotated
    // space puts the quarter-turned crop box 36 from the left and 10 from the top of a page 792
    // wide and 612 high.
    assert_eq!(
        value(&view, &document, "Out"),
        "i|A-5|90|0|36,602,780,12|0,792,612,0|36,602,780,12",
        "{:?}",
        view.script_reports()
    );
}

#[test]
fn a_page_past_the_last_is_a_range_error_and_bbox_is_refused_by_name() {
    let document = two_pages(
        "var said = []; try { getPageBox\\('Crop', 2\\); } catch \\(e\\) { said.push\\(e.name\\); } \
         try { getPageBox\\('BBox'\\); } catch \\(e\\) { said.push\\(e.name\\); } \
         this.getField\\('Out'\\).value = said.join\\('|'\\);",
        "",
        NONE,
        &[],
    );
    let view = opened(&document);
    assert_eq!(value(&view, &document, "Out"), "RangeError|NotAllowedError");
    assert!(
        view.script_reports()
            .iter()
            .any(|report| report.contains("getPageBox(\"BBox\")")),
        "{:?}",
        view.script_reports()
    );
}

#[test]
fn a_named_destination_turns_to_its_page_and_an_unknown_one_turns_none() {
    let document = two_pages("gotoNamedDest\\('chapter2'\\);", "", NONE, &[]);
    let mut view = opened(&document);
    assert_eq!(
        view.take_page_request(),
        Some(1),
        "§12.3.2.4's catalog /Dests"
    );
    let document = two_pages("gotoNamedDest\\('nowhere'\\);", "", NONE, &[]);
    let mut view = opened(&document);
    assert_eq!(view.take_page_request(), None);
    assert!(
        view.script_reports()
            .iter()
            .any(|report| report.contains("\"nowhere\"")),
        "{:?}",
        view.script_reports()
    );
}

#[test]
fn title_is_the_information_dictionary_s_and_active_docs_is_this_document_alone() {
    let document = two_pages(
        "this.getField\\('Out'\\).value = [this.title, app.activeDocs.length, \
         app.activeDocs[0] === this].join\\('|'\\);",
        "",
        NONE,
        &[],
    );
    let view = opened(&document);
    // The trailer's /Info is object 4, whose /Title is unset — so `title` is undefined, joined as
    // the empty string.
    assert_eq!(value(&view, &document, "Out"), "|1|true");
}

#[test]
fn calculate_false_stops_table_224_s_order_until_a_script_sets_it_true() {
    let calculated = |name: &str, number: u32, script: &str| {
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T ({name}) /Rect [10 50 210 80] /F 4 \
             /P 3 0 R /DA (/Helv 10 Tf 0 g) /AA << {script} >> >> % {number}"
        )
    };
    let widgets = [
        calculated("A", 7, ""),
        calculated(
            "B",
            8,
            "/V << /S /JavaScript /JS (this.calculate = true;) >>",
        ),
        calculated(
            "Total",
            9,
            "/C << /S /JavaScript /JS (AFSimple_Calculate\\('SUM', ['A', 'B']\\);) >>",
        ),
    ];
    // The calculation order is the form's own: Table 224's /CO.
    let document = two_pages(
        "this.calculate = false; this.getField\\('Out'\\).value = String\\(this.calculate\\);",
        "/CO [9 0 R]",
        (&[7, 8, 9], &[7, 8, 9]),
        &widgets,
    );
    let mut view = opened(&document);
    assert_eq!(value(&view, &document, "Out"), "false");
    view.set_field(&document, "A", &Entered::Text("2".to_owned()));
    view.commit_field(&document, "A");
    assert_eq!(value(&view, &document, "Total"), "", "no calculation runs");
    view.set_field(&document, "B", &Entered::Text("3".to_owned()));
    view.commit_field(&document, "B");
    assert_eq!(
        value(&view, &document, "Total"),
        "5",
        "B's validation set calculate true, and its commit walks /CO: {:?}",
        view.script_reports()
    );
}

/// A list box `Colour` with Table 234's three entries, two of them pairs, selecting `value`.
fn colours(value: &str, flags: u32) -> String {
    format!(
        "<< /Type /Annot /Subtype /Widget /FT /Ch /Ff {flags} /T (Colour) /Rect [10 50 210 80] \
         /F 4 /P 3 0 R /DA (/Helv 10 Tf 0 g) /Opt [[(r) (Red)] (Green) [(b) (Blue)]] /V {value} >>"
    )
}

#[test]
fn a_choice_field_s_items_are_its_opt_and_its_value_s_index_is_read() {
    let document = two_pages(
        "var f = this.getField\\('Colour'\\); this.getField\\('Out'\\).value = [f.numItems, \
         f.getItemAt\\(0\\), f.getItemAt\\(0, false\\), f.getItemAt\\(1\\), f.getItemAt\\(-1\\), \
         f.currentValueIndices].join\\('|'\\);",
        "",
        (&[7], &[7]),
        &[colours("(Green)", 0)],
    );
    let view = opened(&document);
    // Table 234: an entry is a text string or a pair of export value and displayed text; the
    // reference's getItemAt answers the export value where there is one, the text otherwise.
    assert_eq!(value(&view, &document, "Out"), "3|r|Red|Green|b|1");
}

#[test]
fn a_multiple_selection_is_an_array_and_a_value_off_the_list_is_minus_one() {
    // Table 233 bit 22, MultiSelect, and §12.7.5.4's /V as an array of the items' texts.
    let document = two_pages(
        "this.getField\\('Out'\\).value = \
         JSON.stringify\\(this.getField\\('Colour'\\).currentValueIndices\\);",
        "",
        (&[7], &[7]),
        &[colours("[(Red) (Blue)]", 1 << 21)],
    );
    let view = opened(&document);
    assert_eq!(value(&view, &document, "Out"), "[0,2]");
    let document = two_pages(
        "this.getField\\('Out'\\).value = this.getField\\('Colour'\\).currentValueIndices;",
        "",
        (&[7], &[7]),
        &[colours("(Mauve)", 0)],
    );
    let view = opened(&document);
    assert_eq!(value(&view, &document, "Out"), "-1");
}

#[test]
fn choosing_by_index_commits_the_item_as_a_person_s_choice() {
    let document = two_pages(
        "var f = this.getField\\('Colour'\\); f.currentValueIndices = 2; \
         this.getField\\('Out'\\).value = f.currentValueIndices + '|' + f.value;",
        "",
        (&[7], &[7]),
        &[colours("(Green)", 0)],
    );
    let view = opened(&document);
    assert_eq!(value(&view, &document, "Out"), "2|Blue");
    // The choice is the view state's, as a person's is: the field's control selects item 2.
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let described = pdf_model::form::fields(&document, &page, &view);
    let colour = described
        .iter()
        .find(|field| field.partial == "Colour")
        .expect("the list box is on page one");
    let pdf_model::form::Control::Choice(choice) = &colour.control else {
        panic!("{:?}", colour.control)
    };
    assert_eq!(choice.selected, vec![2]);
}

#[test]
fn rewriting_a_text_field_s_options_is_refused_by_name_and_export_values_reads_table_230() {
    let radio = |number: u32, state: &str| {
        format!(
            "<< /Type /Annot /Subtype /Widget /Parent 9 0 R /Rect [10 50 30 70] /F 4 /P 3 0 R \
             /AP << /N << /{state} 10 0 R /Off 10 0 R >> >> >> % {number}"
        )
    };
    let widgets = [
        radio(7, "0"),
        radio(8, "1"),
        "<< /FT /Btn /Ff 32768 /T (Way) /Kids [7 0 R 8 0 R] /Opt [(Nord) (S\\374d)] /V /Off >>"
            .to_owned(),
        "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length 0 >>\nstream\n\nendstream"
            .to_owned(),
    ];
    let document = two_pages(
        "var said = [this.getField\\('Way'\\).exportValues.join\\(','\\)]; \
         try { this.getField\\('Out'\\).setItems\\(['x']\\); } catch \\(e\\) { said.push\\(e.name\\); } \
         this.getField\\('Out'\\).value = said.join\\('|'\\);",
        "",
        (&[9], &[7, 8]),
        &widgets,
    );
    let view = opened(&document);
    // Table 230: one text string per widget, the export value of each.
    assert_eq!(
        value(&view, &document, "Out"),
        "Nord,Süd|NotAllowedError",
        "{:?}",
        view.script_reports()
    );
}

/// The list box `Colour`'s options as a host's control lists them, and what it selects.
fn control(view: &ViewState, document: &Document) -> (Vec<String>, Vec<usize>) {
    let page = pdf_model::Pages::new(document).get(0).expect("page one");
    let described = pdf_model::form::fields(document, &page, view);
    let colour = described
        .iter()
        .find(|field| field.partial == "Colour")
        .expect("the list box is on page one");
    // A control of another kind answers with itself in place of the options, so the assertion
    // that reads this prints what was found.
    let pdf_model::form::Control::Choice(choice) = &colour.control else {
        return (vec![format!("{:?}", colour.control)], Vec::new());
    };
    (
        choice
            .options
            .iter()
            .map(|option| option.label.clone())
            .collect(),
        choice.selected.clone(),
    )
}

/// `setItems` replaces Table 234's `/Opt` whole: the realm reads the new list, a host's control
/// lists it, a person's choice by index selects from it, and a save writes it — an item given as
/// the reference's pair of text and export value written as the table's pair of export value and
/// text. `Green`, selected before, is selected where it now stands (§12.7.5.4's `/V` names it by
/// its text).
#[test]
fn set_items_rewrites_opt_for_the_realm_the_control_a_choice_and_the_save() {
    let document = two_pages(
        "var f = this.getField\\('Colour'\\); f.setItems\\([['One', '1'], 'Two', 'Green']\\); \
         this.getField\\('Out'\\).value = [f.numItems, f.getItemAt\\(0\\), \
         f.getItemAt\\(0, false\\), f.currentValueIndices].join\\('|'\\);",
        "",
        (&[7], &[7]),
        &[colours("(Green)", 0)],
    );
    let mut view = opened(&document);
    assert_eq!(
        value(&view, &document, "Out"),
        "3|1|One|2",
        "{:?}",
        view.script_reports()
    );
    assert_eq!(
        control(&view, &document),
        (
            vec!["One".to_owned(), "Two".to_owned(), "Green".to_owned()],
            vec![2]
        )
    );
    view.set_field(&document, "Colour", &Entered::Chosen(vec![1]));
    assert_eq!(
        control(&view, &document).1,
        vec![1],
        "the new list's second item"
    );
    let written = view.save(&document).expect("the update writes");
    let saved = Document::open(written.bytes).expect("the update reads back");
    let field = saved
        .get(pdf_syntax::ObjectId {
            number: 7,
            generation: 0,
        })
        .as_dict()
        .cloned()
        .expect("the list box");
    let text = |object: &pdf_syntax::Object| match object {
        pdf_syntax::Object::String(bytes) => pdf_syntax::text_string(bytes),
        pdf_syntax::Object::Array(pair) => pair
            .iter()
            .map(|entry| match entry {
                pdf_syntax::Object::String(bytes) => pdf_syntax::text_string(bytes),
                _ => String::new(),
            })
            .collect::<Vec<_>>()
            .join("/"),
        _ => String::new(),
    };
    let options: Vec<String> = saved
        .get_key(&field, "Opt")
        .as_array()
        .map(|items| items.iter().map(text).collect())
        .unwrap_or_default();
    assert_eq!(options, ["1/One", "Two", "Green"]);
}

/// `insertItemAt` and `deleteItemAt` move the selection with its item, and deleting the selected
/// item — `deleteItemAt` with no index — leaves the field with no selection, as the reference
/// says; `clearItems` leaves no option.
#[test]
fn inserting_and_deleting_keep_the_selection_with_its_item_until_it_is_deleted() {
    let document = two_pages(
        "var f = this.getField\\('Colour'\\); var said = []; \
         f.insertItemAt\\('Mauve', 'm', 0\\); said.push\\(f.numItems, f.currentValueIndices\\); \
         f.insertItemAt\\({cName: 'Last', nIdx: -1}\\); said.push\\(f.getItemAt\\(-1\\)\\); \
         f.deleteItemAt\\(\\); said.push\\(f.numItems, f.currentValueIndices, f.value\\); \
         f.deleteItemAt\\(\\); \
         try { f.deleteItemAt\\(9\\); } catch \\(e\\) { said.push\\(e.name\\); } \
         this.getField\\('Out'\\).value = said.join\\('|'\\);",
        "",
        (&[7], &[7]),
        &[colours("(Green)", 0)],
    );
    let view = opened(&document);
    assert_eq!(
        value(&view, &document, "Out"),
        "4|2|Last|4|-1||RangeError",
        "{:?}",
        view.script_reports()
    );
    assert_eq!(
        control(&view, &document),
        (
            vec![
                "Mauve".to_owned(),
                "Red".to_owned(),
                "Blue".to_owned(),
                "Last".to_owned()
            ],
            Vec::new()
        )
    );
    assert!(
        view.script_reports()
            .iter()
            .any(|sentence| sentence.contains("nothing is selected, so no option is deleted")),
        "{:?}",
        view.script_reports()
    );
    let document = two_pages(
        "var f = this.getField\\('Colour'\\); f.clearItems\\(\\); \
         this.getField\\('Out'\\).value = [f.numItems, f.currentValueIndices].join\\('|'\\);",
        "",
        (&[7], &[7]),
        &[colours("(Green)", 0)],
    );
    let view = opened(&document);
    assert_eq!(value(&view, &document, "Out"), "0|-1");
    assert_eq!(control(&view, &document), (Vec::new(), Vec::new()));
}

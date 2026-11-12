//! ISO 32000-2 §12.7.5.3, Table 232's `/MaxLen` held by the model, whoever sets the value.
//!
//! Table 232: `/MaxLen` is "The maximum length of the field's text, in characters." The entry
//! states the *field's* text, so a value a host sets through [`ViewState::set_field`], a value a
//! calculation script writes (Table 199's `/C`) and a value an FDF file imports (§12.7.8.3.2) are
//! all cut to their first `/MaxLen` characters — the owner's answer `doc/questions/A834`, item 1,
//! and ADR 2840. Every expected value below is the stated value's first `/MaxLen` characters.

#![expect(
    clippy::expect_used,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::fmt::Write as _;

use pdf_model::forms_data::FormsData;
use pdf_model::view::{Entered, ViewState};
use pdf_syntax::{Document, ObjectId};

/// A one-page document whose objects are given from 4 on, with an interactive form dictionary
/// whose body is `form`, every object after the page being one of its widgets.
fn form_document(form: &str, objects: &[&str]) -> Document {
    let mut bodies = vec![
        format!("<< /Type /Catalog /Pages 2 0 R /AcroForm {form} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
    ];
    let annots: Vec<String> = (0..objects.len())
        .map(|index| format!("{} 0 R", index.saturating_add(4)))
        .collect();
    bodies.push(format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [{}] >>",
        annots.join(" ")
    ));
    bodies.extend(objects.iter().map(|body| (*body).to_owned()));
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
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// A text field widget stating `/MaxLen` `max_len` and `/AA` `actions`.
fn limited(name: &str, rect: &str, max_len: u32, actions: &str) -> String {
    format!(
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T ({name}) /Rect [{rect}] /MaxLen {max_len} \
         /DA (/Helv 10 Tf 0 g) /AA << {actions} >> >>"
    )
}

/// The `/V` a save writes for the widget object `widget`, decoded as §7.9.2.2's text string.
fn saved_value(view: &ViewState, document: &Document, widget: u32) -> String {
    let bytes = view.save(document).expect("the update writes").bytes;
    let saved = Document::open(bytes).expect("the update reads back");
    let object = saved.get(ObjectId {
        number: widget,
        generation: 0,
    });
    let dict = object.as_dict().expect("a widget");
    let value = saved.get_key(dict, "V");
    pdf_syntax::text_string::text_string(value.as_string().expect("a text /V"))
}

/// Table 232 over `set_field`: fourteen characters into `/MaxLen 10` keep the first ten, in the
/// value read back and in the `/V` a save writes.
#[test]
fn a_value_set_through_the_model_is_cut_to_max_len() {
    let document = form_document(
        "<< /Fields [4 0 R] >>",
        &[&limited("Name", "10 10 390 40", 10, "")],
    );
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(
            &document,
            "Name",
            &Entered::Text("Fish & <chips>".to_owned())
        ),
        1
    );
    let shown = view.field_value(&document, "Name").map(|shown| shown.text);
    assert_eq!(shown.as_deref(), Some("Fish & <ch"));
    assert_eq!(saved_value(&view, &document, 4), "Fish & <ch");
}

/// The control: a value within `/MaxLen` is taken whole, so the cut is the limit and not a
/// change to every value.
#[test]
fn a_value_within_max_len_is_taken_whole() {
    let document = form_document(
        "<< /Fields [4 0 R] >>",
        &[&limited("Name", "10 10 390 40", 10, "")],
    );
    let mut view = ViewState::of(&document);
    view.set_field(&document, "Name", &Entered::Text("Fish".to_owned()));
    assert_eq!(
        view.field_value(&document, "Name")
            .map(|shown| shown.text)
            .as_deref(),
        Some("Fish")
    );
}

/// Table 232 over §12.7.8.3.2's import: an FDF `/V` longer than the target's `/MaxLen` keeps its
/// first `/MaxLen` characters; `/MaxLen` is inherited from the parent field (Table 226).
#[test]
fn an_imported_value_is_cut_to_the_inherited_max_len() {
    let document = form_document(
        "<< /Fields [5 0 R] >>",
        &[
            "<< /Type /Annot /Subtype /Widget /Parent 5 0 R /Rect [10 10 390 40] \
             /DA (/Helv 10 Tf 0 g) >>",
            "<< /FT /Tx /T (Code) /MaxLen 4 /Kids [4 0 R] >>",
        ],
    );
    let fdf = Document::open(
        b"%FDF-1.2\n1 0 obj\n<< /FDF << /Fields [ << /T (Code) /V (ABCDEFGH) >> ] >> >>\nendobj\n\
          trailer\n<< /Root 1 0 R >>\n%%EOF\n"
            .to_vec(),
    )
    .expect("an FDF file is opened by the PDF reader");
    let data = FormsData::read(&fdf).expect("an FDF catalog");
    let mut view = ViewState::of(&document);
    assert_eq!(view.import(&document, &data).widgets, 1);
    assert_eq!(
        view.field_value(&document, "Code")
            .map(|shown| shown.text)
            .as_deref(),
        Some("ABCD")
    );
}

/// Table 232 over Table 199's `/C`: a calculation's result is the field's text, so `12.5` into
/// `/MaxLen 2` keeps `12`.
#[test]
fn a_calculated_value_is_cut_to_max_len() {
    let sum =
        "/C << /S /JavaScript /JS (AFSimple_Calculate\\(\"SUM\", [\"Line.1\", \"Line.2\"]\\)) >>";
    let document = form_document(
        "<< /Fields [4 0 R 5 0 R 6 0 R] /CO [6 0 R] >>",
        &[
            &limited("Line.1", "10 10 110 30", 20, ""),
            &limited("Line.2", "10 40 110 60", 20, ""),
            &limited("Total", "10 70 110 90", 2, sum),
        ],
    );
    let mut view = ViewState::of(&document);
    view.set_field(&document, "Line.1", &Entered::Text("10".to_owned()));
    view.set_field(&document, "Line.2", &Entered::Text("2.5".to_owned()));
    assert_eq!(
        view.field_value(&document, "Total")
            .map(|shown| shown.text)
            .as_deref(),
        Some("12")
    );
}

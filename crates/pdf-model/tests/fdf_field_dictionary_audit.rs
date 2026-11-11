//! complete-source consumer regressions for Table 249's /Opt and field/widget flags.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::unreachable,
    reason = "tiny complete fixtures have bounded object counts and known choice types"
)]

use std::fmt::Write as _;

use pdf_model::{forms_data::FormsData, view::ViewState};
use pdf_syntax::{Document, FileBytes, Object, ObjectId};

fn file(header: &str, root: &str, objects: &[String]) -> Vec<u8> {
    let mut out = String::from(header);
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj", index + 1);
        out.push_str(object);
        out.push_str("\nendobj\n");
    }
    let xref_at = out.len();
    let size = objects.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root {root} /ID [<30313233343536373839616263646566> <66656463626139383736353433323130>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn source() -> Document {
    // The widget is the terminal field, as allowed by §12.7.4.1; PDF 1.7 NeedAppearances
    // permits the fixture to omit a stored widget appearance.
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /NeedAppearances true /DA (/Helv 9 Tf 0 g) /DR << /Font << /Helv 5 0 R >> >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Annots [4 0 R] >>".to_owned(),
        String::from("<< /Type /Annot /Subtype /Widget /Rect [10 10 190 50] /F 4 /FT /Ch /T (choice) /Ff 1 /V (old-blue) /Opt [(old-red) (old-blue)] /DA (/Helv 9 Tf 0 g) >>"),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_owned(),
    ];
    let bytes = file("%PDF-1.7\n", "1 0 R", &objects);
    let document = Document::open(FileBytes::from(bytes)).expect("valid PDF syntax");
    assert!(!document.was_recovered(), "the source has an exact xref");
    document
}

fn fdf(field: &str) -> Document {
    let objects = [format!("<< /FDF << /Fields [{field}] >> >>")];
    let bytes = file("%FDF-1.2\n", "1 0 R", &objects);
    let document = Document::open(FileBytes::from(bytes)).expect("valid FDF syntax");
    assert!(!document.was_recovered(), "the FDF has an exact xref");
    document
}

fn import_and_save(source: &Document, field: &str) -> Document {
    let data = FormsData::read(&fdf(field)).expect("FDF catalog and fields");
    let mut view = ViewState::of(source);
    let imported = view.import(source, &data);
    assert_eq!(imported.widgets, 1, "the fully qualified field matched");
    assert!(imported.unmatched.is_empty(), "{:?}", imported.unmatched);
    let saved = view.save(source).expect("an imported field can be saved");
    let saved = Document::open(saved.bytes).expect("the saved update is a PDF");
    assert!(!saved.was_recovered(), "exact saved xref");
    saved
}

fn integer(document: &Document, object: ObjectId, key: &str) -> Option<i64> {
    let field = document.get(object);
    let dictionary = field.as_dict().expect("field/widget dictionary");
    document.get_key(dictionary, key).as_integer()
}

/// Table 249's general import rule replaces stated corresponding entries. `/Opt` is an entry
/// required for choice fields, and Table 234 says it supplies the options presented to the user.
/// The old list and new list are disjoint, so retaining the target's options cannot pass.
#[test]
fn imported_choice_options_replace_the_target_options() {
    let source = source();
    let original_field = source.get(ObjectId::new(4, 0));
    let before = original_field.as_dict().expect("choice field");
    assert_eq!(
        source.get_key(before, "V").as_string(),
        Some(b"old-blue".as_slice())
    );
    assert_eq!(
        source
            .get_key(before, "Opt")
            .as_array()
            .map(<[Object]>::len),
        Some(2)
    );

    let saved = import_and_save(
        &source,
        "<< /T (choice) /V (new-green) /Opt [(new-amber) (new-green)] >>",
    );
    let saved_field = saved.get(ObjectId::new(4, 0));
    let field = saved_field.as_dict().expect("saved choice field");
    assert_eq!(
        saved.get_key(field, "V").as_string(),
        Some(b"new-green".as_slice())
    );
    let saved_options = saved.get_key(field, "Opt");
    let options = saved_options.as_array().expect("Table 249 /Opt");
    let labels: Vec<_> = options.iter().map(Object::as_string).collect();
    assert_eq!(
        labels,
        [Some(b"new-amber".as_slice()), Some(b"new-green".as_slice())]
    );
}

/// `/Ff` and `/F` replace their target words and suppress their Set/Clear companions. Without
/// replacement, Set is applied before Clear. The three cases exercise the field and widget words
/// together and include an unchanged control so value import alone cannot satisfy the assertions.
#[test]
fn imported_flags_replace_or_apply_set_then_clear() {
    let base = source();
    let replacement = import_and_save(
        &base,
        "<< /T (choice) /V (old-red) /Opt [(old-red) (old-blue)] /Ff 4 /SetFf 8 /ClrFf 4 /F 2 /SetF 4 /ClrF 2 >>",
    );
    assert_eq!(integer(&replacement, ObjectId::new(4, 0), "Ff"), Some(4));
    assert_eq!(integer(&replacement, ObjectId::new(4, 0), "F"), Some(2));

    let modified = import_and_save(
        &base,
        "<< /T (choice) /V (old-red) /Opt [(old-red) (old-blue)] /SetFf 4 /ClrFf 4 /SetF 2 /ClrF 2 >>",
    );
    // Both pairs overlap: Clear after Set leaves the original bits (1 and 4); reversing them
    // would instead leave field flags 5 and annotation flags 6.
    assert_eq!(integer(&modified, ObjectId::new(4, 0), "Ff"), Some(1));
    assert_eq!(integer(&modified, ObjectId::new(4, 0), "F"), Some(4));

    let unchanged = import_and_save(
        &base,
        "<< /T (choice) /V (old-red) /Opt [(old-red) (old-blue)] >>",
    );
    assert_eq!(integer(&unchanged, ObjectId::new(4, 0), "Ff"), Some(1));
    assert_eq!(integer(&unchanged, ObjectId::new(4, 0), "F"), Some(4));
}

/// The save deferral does not establish that the imported options reach presentation first.
#[test]
fn imported_choice_options_reach_the_form_control_before_saving() {
    let source = source();
    let page = pdf_model::Pages::new(&source).get(0).expect("one page");
    let mut view = ViewState::of(&source);
    let labels = |state: &ViewState| {
        let fields = pdf_model::form::fields(&source, &page, state);
        assert_eq!(fields.len(), 1, "one field descriptor");
        match &fields[0].control {
            pdf_model::form::Control::Choice(choice) => choice
                .options
                .iter()
                .map(|option| option.label.clone())
                .collect::<Vec<_>>(),
            control => unreachable!("fixture is a choice field: {control:?}"),
        }
    };
    assert_eq!(labels(&view), ["old-red", "old-blue"]);
    let data = FormsData::read(&fdf(
        "<< /T (choice) /V (new-green) /Opt [(new-amber) (new-green)] >>",
    ))
    .expect("FDF fields");
    let imported = view.import(&source, &data);
    assert_eq!(imported.widgets, 1);
    assert!(imported.unmatched.is_empty());
    assert_eq!(labels(&view), ["new-amber", "new-green"]);
}

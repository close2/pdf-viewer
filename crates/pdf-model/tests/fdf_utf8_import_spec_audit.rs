//! Table 246 UTF-8 names, values and options reach live controls and saved fields.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    reason = "small exact-xref fixtures with explicit parse failures"
)]

use pdf_model::{forms_data::FormsData, view::ViewState};
use pdf_syntax::{Document, ObjectId};
use std::fmt::Write as _;

fn file(header: &str, bodies: &[String], trailer: &str) -> Document {
    let mut bytes = header.to_owned();
    let mut offsets = Vec::new();
    for (i, body) in bodies.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = write!(bytes, "{} 0 obj\n{body}\nendobj\n", i + 1);
    }
    let xref = bytes.len();
    let _ = write!(bytes, "xref\n0 {}\n0000000000 65535 f \n", bodies.len() + 1);
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R {trailer} >>\nstartxref\n{xref}\n%%EOF\n",
        bodies.len() + 1
    );
    let document = Document::open(bytes.into_bytes()).expect("exact-xref fixture");
    assert!(!document.was_recovered());
    assert!(!document.cross_reference_chain_was_cut());
    document
}

fn target() -> Document {
    file("%PDF-1.7\n", &[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R] /NeedAppearances true /DA (/Helv 9 Tf 0 g) /DR << /Font << /Helv 6 0 R >> >> >> >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Annots [4 0 R 5 0 R] >>".into(),
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 50] /F 4 /FT /Tx /T <FEFF8868> /V (old) >>".into(),
        "<< /Type /Annot /Subtype /Widget /Rect [110 10 190 50] /F 4 /FT /Tx /T (A) /V (old) >>".into(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".into(),
    ], "/ID [<30313233343536373839616263646566><30313233343536373839616263646566>]")
}

fn value(document: &Document, id: u32) -> String {
    let object = document.get(ObjectId::new(id, 0));
    let dictionary = object.as_dict().expect("text field");
    let object = document.get_key(dictionary, "V");
    pdf_syntax::text_string(object.as_string().expect("text value"))
}

fn imported(fields: &str, expected_widgets: usize, expected: [&str; 2]) {
    let source = target();
    // FDF's header remains 1.2; catalog Version1.4 admits the Encoding entry introduced in1.3.
    let fdf = file(
        "%FDF-1.2\n",
        &[format!(
            "<< /Version /1.4 /FDF << /Encoding /utf_8 /Fields [{fields}] >> >>"
        )],
        "",
    );
    let data = FormsData::read(&fdf).expect("FDF catalog");
    let mut view = ViewState::of(&source);
    let result = view.import(&source, &data);
    let saved = view.save(&source).expect("save imported fields");
    let saved = Document::open(saved.bytes).expect("saved incremental PDF");
    assert!(!saved.was_recovered());
    assert!(!saved.cross_reference_chain_was_cut());
    assert_eq!(
        (
            result.widgets,
            result.unmatched.is_empty(),
            [value(&saved, 4), value(&saved, 5)]
        ),
        (expected_widgets, true, expected.map(str::to_owned)),
        "unmatched: {:?}",
        result.unmatched
    );
    assert_eq!([value(&source, 4), value(&source, 5)], ["old", "old"]);
}

#[test]
fn ascii_utf8_field_import_is_a_positive_control() {
    imported("<< /T (A) /V (new) >>", 1, ["old", "new"]);
}

#[test]
fn utf16_bom_overrides_the_fdf_encoding_for_names_and_values() {
    imported("<< /T <FEFF8868> /V <FEFF00E9> >>", 1, ["é", "old"]);
}

#[test]
fn utf8_non_bom_field_name_matches_the_target_on_import() {
    // E8 A1 A8 encodes U+8868; the PDF target independently names it in UTF-16BE.
    imported(
        "<< /T <E8A1A8> /V (new) >> << /T (A) /V (control) >>",
        2,
        ["new", "control"],
    );
}

#[test]
fn utf8_non_bom_value_is_transcoded_on_import_and_save() {
    // C3 A9 encodes U+00E9, available in the target's Standard14 WinAnsi font.
    imported("<< /T (A) /V <C3A9> >>", 1, ["old", "é"]);
}

fn choice_target() -> Document {
    file(
        "%PDF-1.7\n",
        &[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /NeedAppearances true /DA (/Helv 9 Tf 0 g) /DR << /Font << /Helv 5 0 R >> >> >> >>".into(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Annots [4 0 R] >>".into(),
            "<< /Type /Annot /Subtype /Widget /Rect [10 10 190 50] /F 4 /FT /Ch /T (choice) /Ff 0 /V (old-beta) /Opt [(old-alpha) (old-beta)] /DA (/Helv 9 Tf 0 g) >>".into(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".into(),
        ],
        "/ID [<30313233343536373839616263646566><30313233343536373839616263646566>]",
    )
}

fn fdf(options: &str, value: &str) -> Document {
    file(
        "%FDF-1.2\n",
        &[format!(
            "<< /Version /1.4 /FDF << /Encoding /utf_8 /Fields [<< /T (choice) /V {value} /Opt [{options}] >>] >> >>"
        )],
        "",
    )
}

fn control(document: &Document, view: &ViewState) -> (Vec<String>, Vec<usize>) {
    let page = pdf_model::Pages::new(document)
        .get(0)
        .expect("one target page");
    let fields = pdf_model::form::fields(document, &page, view);
    assert_eq!(fields.len(), 1);
    match &fields[0].control {
        pdf_model::form::Control::Choice(choice) => (
            choice
                .options
                .iter()
                .map(|option| option.label.clone())
                .collect(),
            choice.selected.clone(),
        ),
        other => panic!("target is a choice field: {other:?}"),
    }
}

fn semantic_strings(document: &Document, object: ObjectId, key: &str) -> Vec<String> {
    let field = document.get(object);
    let field = field.as_dict().expect("widget field");
    let value = document.get_key(field, key);
    value
        .as_array()
        .expect("array entry")
        .iter()
        .map(|item| pdf_syntax::text_string(item.as_string().expect("plain text option")))
        .collect()
}

fn imported_choice(
    options: &str,
    value: &str,
    expected_labels: &[&str],
    selected: &[usize],
    expected_value: &str,
) {
    let target = choice_target();
    let mut view = ViewState::of(&target);
    assert_eq!(
        control(&target, &view),
        (vec!["old-alpha".into(), "old-beta".into()], vec![1])
    );

    // FDF 1.2 plus catalog Version 1.4 makes Table 246's PDF 1.3 Encoding entry valid.
    let data = FormsData::read(&fdf(options, value)).expect("version-valid FDF catalog");
    let result = view.import(&target, &data);
    assert_eq!(result.widgets, 1);
    assert!(result.unmatched.is_empty(), "{:?}", result.unmatched);

    let labels: Vec<String> = expected_labels
        .iter()
        .map(|label| (*label).into())
        .collect();
    let live = control(&target, &view);

    let written = view.save(&target).expect("save imported choice options");
    let saved = Document::open(written.bytes).expect("saved incremental update");
    assert!(!saved.was_recovered());
    assert!(!saved.cross_reference_chain_was_cut());
    let saved_options = semantic_strings(&saved, ObjectId::new(4, 0), "Opt");
    assert_eq!(
        (live, saved_options),
        ((labels.clone(), selected.to_vec()), labels)
    );
    assert_eq!(
        control(&target, &ViewState::of(&target)),
        (vec!["old-alpha".into(), "old-beta".into()], vec![1])
    );
    let field = saved.get(ObjectId::new(4, 0));
    let field = field.as_dict().expect("saved choice field");
    assert_eq!(
        pdf_syntax::text_string(
            saved
                .get_key(field, "V")
                .as_string()
                .expect("selected value")
        ),
        expected_value
    );
}

#[test]
fn ascii_utf8_option_labels_are_a_positive_control() {
    imported_choice(
        "(new-alpha) (new-beta)",
        "(new-beta)",
        &["new-alpha", "new-beta"],
        &[1],
        "new-beta",
    );
}

#[test]
fn utf16_bom_option_label_is_a_positive_control_under_utf8_encoding() {
    imported_choice(
        "<FEFF00E9> (new-beta)",
        "<FEFF00E9>",
        &["é", "new-beta"],
        &[0],
        "é",
    );
}

#[test]
fn non_bom_utf8_option_label_is_decoded_for_control_and_save() {
    // /V is the same Unicode label, written as an ordinary PDF text string with its BOM. This
    // isolates the Table 246 no-BOM /Opt path from the separate no-BOM /V path.
    imported_choice(
        "<C3A9> (new-beta)",
        "<FEFF00E9>",
        &["é", "new-beta"],
        &[0],
        "é",
    );
}

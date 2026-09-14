//! ISO 32000-2 §12.7.6.2's submission, checked by what a host would be handed.
//!
//! The clause's `shall` is to "transmit the names and values of selected interactive form fields
//! to a specified uniform resource locator (URL)", and every part of that except the verb is a
//! question about the document: *which* fields, under *which* of Tables 239 and 240's rules, in
//! *which* of the four formats. `pdf_model::submission::compose` answers all of it and a host
//! sends what comes back (ADR 1062), so these check the answer rather than a network.
//!
//! Trap 8 is why the witnesses are built: `examples/refused_action_census` finds one submit-form
//! action in the corpus, on `webCapture.pdf`, and it names no `/Fields`, sets no flag past bit 3
//! and has no filled form behind it — so it exercises the default path and nothing else. What is
//! below is one fixture with every shape Table 240 distinguishes.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::fmt::Write as _;

use pdf_model::action::{Action, SubmitForm};
use pdf_model::forms_data::FormsData;
use pdf_model::submission::{Click, Format, Method, Refusal, Submission, compose};
use pdf_model::view::ViewState;
use pdf_syntax::{Document, Object, ObjectId};

/// One form with every shape Tables 239, 240 and 231 distinguish, and nothing else.
///
/// Object by object: 5 a plain text field with a value; 6 a check box whose `/V` is the
/// §12.7.5.2.3 state name `/On`; 9 a text field with Table 227 bit 3's `NoExport` set; 10 a text
/// field with Table 231 bit 21's `FileSelect` set; 11 a text field stating no `/V` at all; 12 a
/// push-button, which §12.7.6.2 names separately; 13 a parent field with 14 as its kid, so that
/// §12.7.4.2's "group.inner" is in the table; 15 a text field with a `/TM` mapping name.
fn form() -> Vec<u8> {
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm \
         << /Fields [5 0 R 6 0 R 9 0 R 10 0 R 11 0 R 12 0 R 13 0 R 15 0 R] \
         /DR << /Font << /Helv 7 0 R >> >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Contents 4 0 R /Annots [5 0 R 6 0 R 9 0 R 10 0 R 11 0 R 12 0 R 14 0 R 15 0 R] \
         >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
         /T (name) /V (a value) /DA (/Helv 12 Tf 0 g) >>\nendobj\n\
         6 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 10 40 30] /F 4 /FT /Btn \
         /T (agree) /V /On /AS /On /DA (/Helv 0 Tf 0 g) >>\nendobj\n\
         7 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding /WinAnsiEncoding >>\nendobj\n\
         8 0 obj\n<< >>\nendobj\n\
         9 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 80 180 100] /F 4 /FT /Tx \
         /T (secret) /V (kept) /Ff 4 /DA (/Helv 12 Tf 0 g) >>\nendobj\n\
         10 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 110 180 130] /F 4 /FT /Tx \
         /T (upload) /V (report.txt) /Ff 1048576 /DA (/Helv 12 Tf 0 g) >>\nendobj\n\
         11 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 140 180 160] /F 4 /FT /Tx \
         /T (blank) /DA (/Helv 12 Tf 0 g) >>\nendobj\n\
         12 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [100 10 180 30] /F 4 /FT /Btn \
         /T (send) /Ff 65536 /DA (/Helv 0 Tf 0 g) >>\nendobj\n\
         13 0 obj\n<< /FT /Tx /T (group) /Kids [14 0 R] >>\nendobj\n\
         14 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 170 180 190] /F 4 /Parent 13 0 R \
         /T (inner) /V (nested) /DA (/Helv 12 Tf 0 g) >>\nendobj\n\
         15 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [100 40 180 70] /F 4 /FT /Tx \
         /T (mapped) /TM (short) /V (m) /DA (/Helv 12 Tf 0 g) >>\nendobj\n"
        .to_owned();

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
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
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<0102> <0304>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The fixture, opened.
fn document() -> Document {
    Document::open(pdf_syntax::FileBytes::from(form())).expect("the fixture parses")
}

/// A submit-form action over the fixture's URL with these flags and this `/Fields` array.
fn action(flags: u32, fields: &str) -> SubmitForm {
    let source = format!(
        "%PDF-2.0\n1 0 obj\n<< /Type /Catalog >>\nendobj\n\
         2 0 obj\n<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/cgi) >> \
         /Flags {flags} {fields} >>\nendobj\n\
         trailer\n<< /Root 1 0 R /Size 3 >>\n"
    );
    let holder = Document::open(pdf_syntax::FileBytes::from(source.into_bytes()))
        .expect("the action parses");
    let read = pdf_model::action::read(&holder, &Object::Reference(ObjectId::new(2, 0)));
    match read.into_iter().next() {
        Some(Action::SubmitForm(submit)) => submit,
        other => panic!("Table 239's action is read whole: {other:?}"),
    }
}

/// What [`compose`] makes of one action over the untouched fixture.
fn composed(flags: u32, fields: &str) -> Submission {
    let document = document();
    let view = ViewState::of(&document);
    compose(&document, &view, &action(flags, fields), None).expect("the composition succeeds")
}

/// The body, as text, for the two formats that have one.
fn body(submission: &Submission) -> String {
    String::from_utf8_lossy(&submission.body).into_owned()
}

/// The FDF body read back as §12.7.8 defines it, name and value.
///
/// A round trip rather than a search for `(a value)` in the bytes, and the reason is the clause
/// rather than convenience: §7.3.4.3 lets a string be written literal or hexadecimal and
/// `pdf_syntax::write` chooses the second, so an assertion over the raw bytes would be pinning
/// this tree's choice of spelling instead of §12.7.8.3.2's structure. `FormsData::read` is the
/// reader §12.7.6.4's import already uses, so what these assert is that a submission this
/// program composes is a forms-data file this program would accept.
fn fdf_fields(submission: &Submission) -> Vec<(String, Option<String>)> {
    let document = Document::open(pdf_syntax::FileBytes::from(submission.body.clone()))
        .expect("the composed FDF parses as §12.7.8.1's file");
    FormsData::read(&document)
        .expect("and identifies itself with §12.7.8.3's /FDF")
        .fields
        .into_iter()
        .map(|field| {
            let value = field.value.map(|value| match value {
                Object::String(bytes) => pdf_syntax::text_string(&bytes),
                Object::Name(name) => format!("/{}", String::from_utf8_lossy(name.as_bytes())),
                other => format!("{other:?}"),
            });
            (field.name, value)
        })
        .collect()
}

/// Whether the read-back FDF states this name with this value.
fn holds(submission: &Submission, name: &str, value: Option<&str>) -> bool {
    fdf_fields(submission)
        .iter()
        .any(|(held, got)| held == name && got.as_deref() == value)
}

/// Whether the read-back FDF names this field at all.
fn names(submission: &Submission, name: &str) -> bool {
    fdf_fields(submission).iter().any(|(held, _)| held == name)
}

/// §12.7.6.2 names what goes and what it is called:
///
/// > The name submitted for each field shall be its fully qualified name (see 12.7.4.2, "Field
/// > names"), and the value shall be specified by the V entry in its field dictionary.
///
/// With every default in force — `/Flags` absent is "[d]efault value: 0", which is FDF by POST —
/// and the three rules that apply without any flag being set: `NoExport` excluded, a field with
/// no `/V` excluded, and a push-button excluded because "[i]f the submit-form action dictionary
/// contains no Fields entry, such pushbutton fields shall not be submitted".
#[test]
fn the_default_submission_is_an_fdf_of_every_field_with_a_value() {
    let submission = composed(0, "");
    assert_eq!(submission.method, Method::Post);
    assert_eq!(submission.format, Format::Fdf);
    // The registry's name and not the vendor-tree one: `application/fdf`, registered by ISO
    // TC 171/SC 2, which is the committee that owns this standard.
    assert_eq!(submission.format.content_type(), "application/fdf");
    assert_eq!(submission.url, "https://example.invalid/cgi");

    let body = body(&submission);
    assert!(
        body.starts_with("%FDF-1.2\n"),
        "§12.7.8.2.2's header: {body}"
    );
    assert!(
        body.contains("/Root 1 0 R"),
        "§12.7.8.2.4's trailer: {body}"
    );
    // Table 246's `/ID` is "taken from the ID entry in the file's trailer dictionary".
    assert!(
        body.contains("/ID"),
        "the file's identifier travels: {body}"
    );

    // §12.7.8.3.2's field dictionaries, read back by the reader §12.7.6.4's import uses.
    assert!(holds(&submission, "name", Some("a value")), "{body}");
    // §12.7.5.2.3 makes a check box's value the on-state *name*, and §7.3.5 makes a name's bytes
    // the thing it is — so it stays a name in the FDF rather than becoming a string.
    assert!(holds(&submission, "agree", Some("/On")), "{body}");
    // §12.7.4.2's qualified name nests down `/Kids`, exactly as the field tree does, and the
    // reader rebuilds it from every `/T` on the path.
    assert!(holds(&submission, "group.inner", Some("nested")), "{body}");
    assert!(
        body.contains("/Kids"),
        "written as a tree and not a flat list: {body}"
    );

    assert!(
        !names(&submission, "secret"),
        "Table 227 bit 3's NoExport: {body}"
    );
    assert!(!names(&submission, "blank"), "a field with no /V: {body}");
    assert!(
        !names(&submission, "send"),
        "a push-button with no /Fields: {body}"
    );
    assert_eq!(submission.fields, 5, "name, agree, upload, mapped, inner");
}

/// §12.7.6.2, on the one rule that outranks the action's own array:
///
/// > The NoExport flag in the field dictionary's Ff entry … takes precedence over the action's
/// > Fields array and Include/ Exclude flag. Fields whose NoExport flag is set shall not be
/// > included in a submit-form action.
///
/// The calibration is the pair: the same `/Fields` array naming the same two fields puts one of
/// them in the body and not the other, so the absence is the flag's rather than the array's.
#[test]
fn a_no_export_field_is_left_out_even_where_the_action_names_it() {
    let both = composed(0, "/Fields [(name) (secret)]");
    assert!(names(&both, "name"), "the array selects it");
    assert!(
        !names(&both, "secret"),
        "and the field's own flag overrules the array: {:?}",
        fdf_fields(&both)
    );
    assert_eq!(both.fields, 1);
}

/// Table 240 bit 1: "[i]f set, the Fields array tells which fields to exclude."
#[test]
fn the_include_exclude_flag_turns_the_array_inside_out() {
    let included = composed(0, "/Fields [(name)]");
    assert!(names(&included, "name"));
    assert_eq!(included.fields, 1);

    let excluded = composed(1, "/Fields [(name)]");
    assert!(!names(&excluded, "name"), "bit 1 excludes it");
    assert!(names(&excluded, "agree"), "and keeps the rest");
}

/// Table 240 bit 2: "For fields without a value, only the field name shall be transmitted."
#[test]
fn include_no_value_fields_sends_the_name_alone() {
    let without = composed(0, "");
    assert!(!names(&without, "blank"), "clear: it is not submitted");

    let with = composed(2, "");
    // "by name only, with no associated value" — so the name is there and the value is `None`,
    // which is a different answer from a value that is empty.
    assert!(holds(&with, "blank", None), "set: {:?}", fdf_fields(&with));
}

/// Table 240 bit 3: "If set, field names and values shall be submitted in HTML Form format."
///
/// HTML 4.01 section 17.13.4 is what that names: `name=value` pairs joined by `&`, spaces as `+` and
/// everything else non-alphanumeric as `%HH`.
#[test]
fn the_export_format_flag_writes_html_forms_url_encoding() {
    let submission = composed(4, "");
    assert_eq!(submission.format, Format::HtmlForm);
    assert_eq!(
        submission.format.content_type(),
        "application/x-www-form-urlencoded"
    );
    assert_eq!(submission.method, Method::Post);
    let body = body(&submission);
    assert!(body.contains("name=a+value"), "unencoded: {body}");
}

/// Table 240 bit 4: "If set, field names and values shall be submitted using an HTTP GET request."
///
/// RFC 3986 section 3.4 makes the query what follows the first `?`, so a GET's data is in the URL
/// and its body is empty — which is the one place a reader can tell the two methods apart without
/// a network.
#[test]
fn the_get_method_flag_puts_the_data_in_the_url_and_leaves_the_body_empty() {
    let submission = composed(4 | 8, "/Fields [(name)]");
    assert_eq!(submission.method, Method::Get);
    assert!(submission.body.is_empty(), "a GET has no body");
    assert_eq!(
        submission.url, "https://example.invalid/cgi?name=a+value",
        "the query is the form data"
    );
}

/// Table 240 bit 4 again, against a clear bit 3: "if `ExportFormat` is clear, this flag shall also
/// be clear."
///
/// The table forbids the combination and states no behaviour for it, so what a document wrote is
/// neither obeyed nor dropped: the FDF goes by POST and the contradiction is named.
#[test]
fn a_get_against_a_clear_export_format_is_named_rather_than_obeyed() {
    let submission = composed(8, "");
    assert_eq!(submission.format, Format::Fdf);
    assert_eq!(submission.method, Method::Post);
    assert!(
        submission
            .owed
            .iter()
            .any(|owed| owed.contains("GetMethod") && owed.contains("POST")),
        "the contradiction is said out loud: {:?}",
        submission.owed
    );
}

/// Table 240 bit 9: "If set, the document shall be submitted as PDF … If set, all other flags
/// shall be ignored except `GetMethod`."
#[test]
fn submit_pdf_sends_the_document_and_ignores_the_other_flags() {
    // Bit 3, bit 6 and bit 1 are all set beside bit 9, and none of them may show: an HTML body,
    // an XFDF refusal or an inverted selection would each be a flag that was not ignored.
    let submission = composed(1 | 4 | 32 | 256, "/Fields [(name)]");
    assert_eq!(submission.format, Format::Pdf);
    assert_eq!(submission.format.content_type(), "application/pdf");
    assert_eq!(submission.method, Method::Post);
    assert!(
        submission.body.starts_with(b"%PDF-1.7"),
        "the body is the document itself"
    );
    assert!(
        submission.body.len() > form().len(),
        "with §7.5.6's update appended: {} against {}",
        submission.body.len(),
        form().len()
    );
}

/// Table 240 bit 9 and bit 4 together, which is the one pair the table leaves meaningful.
#[test]
fn submit_pdf_still_reads_the_get_method_flag() {
    assert_eq!(composed(256 | 8, "").method, Method::Get);
}

/// Table 240 bit 6: "If set, field names and values shall be submitted as XFDF."
///
/// Refused rather than composed, and the reason is the standard: XFDF is "a version of FDF based
/// on XML as defined by ISO 19444-1", that document is not held, and `CLAUDE.md` principle 5
/// makes a grammar taken from another reader not a reading of a specification at all.
#[test]
fn xfdf_is_refused_by_name_rather_than_submitted_as_something_else() {
    let document = document();
    let view = ViewState::of(&document);
    let refusal = compose(&document, &view, &action(32, ""), None)
        .expect_err("ISO 19444-1 is not on this disk");
    assert!(
        matches!(refusal, Refusal::Xfdf) && refusal.to_string().contains("19444-1"),
        "the refusal names the document it wants: {refusal}"
    );
}

/// §12.7.5.3, under Table 231 bit 21:
///
/// > If the FileSelect flag ( PDF 1.4 ) is set, the field shall function as a file-select
/// > control. In this case, the field's text represents the pathname of a file whose contents
/// > shall be submitted as the field's value
///
/// The clause then splits by format, and so does this: in FDF "the value of the V entry in the
/// FDF field dictionary … shall be a file specification (7.11, "File specifications")
/// identifying the selected file", which is the pathname; in HTML Form format "the submission
/// shall use the MIME content type multipart / form-data", whose part is the file's *contents* —
/// bytes on a filesystem this crate has none of, and therefore owed rather than invented.
#[test]
fn a_file_select_field_carries_its_pathname_to_fdf_and_owes_its_contents_to_html() {
    let fdf = composed(0, "");
    assert!(
        holds(&fdf, "upload", Some("report.txt")),
        "the selected file is named: {:?}",
        fdf_fields(&fdf)
    );

    let html = composed(4, "");
    assert!(
        !body(&html).contains("upload="),
        "the pathname is not the value HTML Form format asks for: {}",
        body(&html)
    );
    assert!(
        html.owed
            .iter()
            .any(|owed| owed.contains("upload") && owed.contains("multipart/form-data")),
        "and what is missing is said: {:?}",
        html.owed
    );
}

/// Table 240 bit 5, in full, from §12.7.6.2:
///
/// > They shall be represented in the data in the format name . x = xval & name . y = yval where
/// > name is the field's mapping name ( TM in the field dictionary) if present; otherwise, name
/// > is the field name.
///
/// "The coordinate values are relative to the upper-left corner of the field's widget annotation
/// rectangle", so the y axis runs the other way from the page's: object 15's `/Rect` is
/// `[100 40 180 70]` and a click at (120, 60) is 20 across and 10 down.
#[test]
fn the_coordinates_are_measured_down_from_the_widgets_upper_left_corner() {
    let document = document();
    let view = ViewState::of(&document);
    let click = Click {
        point: (120.0, 60.0),
        widget: Some(ObjectId::new(15, 0)),
    };
    let submission = compose(
        &document,
        &view,
        &action(4 | 16, "/Fields [(mapped)]"),
        Some(&click),
    )
    .expect("the composition succeeds");
    let body = body(&submission);
    // The mapping name and not the field name: `/TM (short)` against `/T (mapped)`.
    assert!(body.ends_with("short.x=20&short.y=10"), "{body}");
    assert!(!body.contains("mapped.x"), "Table 226's /TM wins: {body}");
}

/// The same bit with no click behind it, which §12.6.4.8 shows is a real case: an action reached
/// from an outline item has no cursor position at all.
#[test]
fn coordinates_asked_for_without_a_click_are_owed_rather_than_invented() {
    let submission = composed(4 | 16, "/Fields [(mapped)]");
    assert!(
        !body(&submission).contains(".x="),
        "nothing is made up: {}",
        body(&submission)
    );
    assert!(
        submission
            .owed
            .iter()
            .any(|owed| owed.contains("SubmitCoordinates") && owed.contains("not invoked by a")),
        "{:?}",
        submission.owed
    );
}

/// Trap 5 at the level of a whole table: every flag of Table 240 that this composition does not
/// carry out arrives as a sentence, because a submission quietly missing one reports nothing.
///
/// Bits 7, 8, 10, 11 and 14, all set at once against an FDF submission.
#[test]
fn every_flag_the_composition_does_not_apply_is_named() {
    let submission = composed(64 | 128 | 512 | 1024 | 8192, "");
    let owed = submission.owed.join("\n");
    for named in [
        "IncludeAppendSaves",
        "IncludeAnnotations",
        "CanonicalFormat",
        "ExclNonUserAnnots",
        "EmbedForm",
    ] {
        assert!(
            owed.contains(named),
            "{named} is not silently dropped: {owed}"
        );
    }
}

/// §12.7.6.2 gives a push-button its own sentence:
///
/// > For push-button fields submitted in FDF, the value submitted shall be that of the AP entry
/// > in the field's widget annotation dictionary.
///
/// Named by `/Fields`, so the sentence above applies and the one after it does not — and what
/// that value *is* is this document's appearance streams, which an FDF has no object space to
/// carry. Owed rather than written, which is the difference between a gap that reports and one
/// that ships.
#[test]
fn a_push_button_named_by_the_action_is_owed_rather_than_dropped() {
    let submission = composed(0, "/Fields [(send)]");
    assert_eq!(submission.fields, 0);
    assert!(
        submission
            .owed
            .iter()
            .any(|owed| owed.contains("push-button send") && owed.contains("/AP")),
        "{:?}",
        submission.owed
    );
}

/// A value a person typed is what goes, and that is the reason this takes a [`ViewState`] at all:
/// §12.7.4's four statements about a field's value have an order, and the latest is a person's.
#[test]
fn the_value_submitted_is_the_one_the_reader_entered() {
    let document = document();
    let mut view = ViewState::of(&document);
    let taken = view.set_field(
        &document,
        "name",
        &pdf_model::view::Entered::Text("typed".to_owned()),
    );
    assert_eq!(taken, 1, "the field took the value");
    let submission = compose(&document, &view, &action(0, "/Fields [(name)]"), None)
        .expect("the composition succeeds");
    assert!(
        holds(&submission, "name", Some("typed")),
        "not the file's own: {:?}",
        fdf_fields(&submission)
    );
}

/// §12.7.5.3 lets a text field's value be "a text string (or, beginning with PDF 1.5, a stre am)",
/// and a stream can fail to decode — an unsupported filter, or one over the document's own budget.
///
/// The value is then *unknown*, which is a different thing from absent, and §12.7.6.2 has an
/// answer only for absent: "[f]ields with no value (that is, whose field dictionary does not
/// contain a V entry)". A field that has one this reader could not read is neither, so it is left
/// out of the body and named — including where Table 240 bit 2 is set, whose sentence is about
/// fields "without a value" and not about fields whose value did not decode. An empty string in
/// its place would submit the form as though the person had left the field blank.
#[test]
fn a_value_whose_stream_will_not_decode_is_named_rather_than_submitted_empty() {
    let source = "%PDF-1.7\n\
         1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [5 0 R] \
         >>\nendobj\n\
         4 0 obj\n<< /Length 4 /Filter /NoSuchDecode >>\nstream\nabcd\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
         /T (note) /V 4 0 R >>\nendobj\n\
         trailer\n<< /Root 1 0 R /Size 6 >>\n";
    let document = Document::open(pdf_syntax::FileBytes::from(source.to_owned().into_bytes()))
        .expect("the fixture parses");
    let view = ViewState::of(&document);

    for flags in [0, 2] {
        let submission =
            compose(&document, &view, &action(flags, ""), None).expect("the composition succeeds");
        assert_eq!(submission.fields, 0, "with /Flags {flags}");
        assert!(
            !names(&submission, "note"),
            "the field is left out rather than sent blank: {:?}",
            fdf_fields(&submission)
        );
        assert!(
            submission
                .owed
                .iter()
                .any(|owed| owed.contains("field note") && owed.contains("could not decode")),
            "and what happened is said: {:?}",
            submission.owed
        );
    }
}

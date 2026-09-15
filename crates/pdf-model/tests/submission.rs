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
use pdf_model::submission::{Click, Format, Method, Submission, compose};
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

    assembled(&body)
}

/// §7.5.4's cross-reference table and §7.5.5's trailer over a body of `N 0 obj … endobj` objects.
fn assembled(body: &str) -> Vec<u8> {
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

/// Two pages of annotations, of both kinds Table 171's `Markup` column distinguishes.
///
/// Object by object: 3 is page one, carrying 5 a `/Highlight` — markup, with an `/AP` naming
/// object 8 and a `/P` naming its page — and 6 a `/Link`, which the column says is not; 4 is page
/// two, carrying 7 a `/Text` (markup) and 9 a `/Popup` (not).
fn marked_up() -> Vec<u8> {
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [5 0 R 6 0 R] \
         >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [7 0 R 9 0 R] \
         >>\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Highlight /Rect [10 10 90 30] /P 3 0 R \
         /T (Ada) /Contents (looks wrong) /QuadPoints [10 30 90 30 10 10 90 10] /AP 8 0 R \
         >>\nendobj\n\
         6 0 obj\n<< /Type /Annot /Subtype /Link /Rect [10 40 90 60] /P 3 0 R >>\nendobj\n\
         7 0 obj\n<< /Type /Annot /Subtype /Text /Rect [20 20 40 40] /P 4 0 R /T (Grace) \
         /Contents (a note) >>\nendobj\n\
         8 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         9 0 obj\n<< /Type /Annot /Subtype /Popup /Rect [50 50 90 90] /P 4 0 R >>\nendobj\n";
    assembled(body)
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

/// The composed FDF, opened, and the Table 246 dictionary inside it.
///
/// The document is returned with the dictionary because it owns the bytes every value in it was
/// read from.
fn fdf_dictionary(submission: &Submission) -> (Document, pdf_syntax::Dictionary) {
    let document = Document::open(pdf_syntax::FileBytes::from(submission.body.clone()))
        .expect("the composed FDF parses as §12.7.8.1's file");
    let catalog = document.catalog().expect("§12.7.8.2.4's /Root");
    let fdf = document
        .get_key(&catalog, "FDF")
        .as_dict()
        .cloned()
        .expect("Table 245's Required /FDF");
    (document, fdf)
}

/// The composed FDF read by the reader §12.7.6.4's import uses.
fn fdf_data(submission: &Submission) -> FormsData {
    let document = Document::open(pdf_syntax::FileBytes::from(submission.body.clone()))
        .expect("the composed FDF parses as §12.7.8.1's file");
    FormsData::read(&document).expect("and identifies itself with §12.7.8.3's /FDF")
}

/// Table 240 bit 7: "the submitted FDF file shall include the contents of all incremental updates
/// to the underlying PDF document, as contained in the Differences entry in the FDF dictionary".
///
/// Table 246 says what those bytes are — "[a] stream containing all the bytes in all incremental
/// updates made to the underlying PDF document since it was opened" — and requires the update
/// that produces them: "[a]n incremental update shall be automatically performed just before the
/// submission takes place, in order to capture all changes made to the document." The assertion
/// is therefore an identity and not a search: the file as it was opened, followed by this stream,
/// is byte for byte what `ViewState::save` writes.
#[test]
fn include_append_saves_carries_the_update_and_nothing_that_came_before_it() {
    let document = document();
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(
            &document,
            "name",
            &pdf_model::view::Entered::Text("typed".to_owned())
        ),
        1,
        "there is something for the update to carry"
    );
    let submission =
        compose(&document, &view, &action(64, ""), None).expect("the composition succeeds");

    let (fdf_document, fdf) = fdf_dictionary(&submission);
    let differences = fdf_document.get_key(&fdf, "Differences");
    let differences = differences
        .as_stream()
        .expect("Table 246 types /Differences as a stream");
    let saved = view
        .save(&document)
        .expect("§7.5.6's update is writable")
        .bytes;
    let opened = form().len();
    assert_eq!(
        &differences.data[..],
        &saved[opened..],
        "the bytes the update appended, and those only"
    );
    assert!(
        !differences.data.starts_with(b"%PDF"),
        "an update rather than a file: bit 9 is what sends the whole document"
    );

    // The calibration (trap 13): the same document, the same view, the bit clear. An entry that
    // appeared either way would be saying nothing about the flag.
    let without = compose(&document, &view, &action(0, ""), None).expect("composes");
    let (without_document, without_fdf) = fdf_dictionary(&without);
    assert!(
        without_document
            .get_key(&without_fdf, "Differences")
            .is_null(),
        "\"If clear, the incremental updates shall not be included.\""
    );
}

/// Table 240 bit 8, whose FDF carries "all markup annotations in the underlying PDF document
/// (see 12.5.6.2, "Markup annotations")".
///
/// §12.7.8.3.4 makes the page ordinal required of each — Table 254's `/Page`, "[t]he ordinal page
/// number on which this annotation shall appear, where page 0 is the first page" — so the fixture
/// puts one markup annotation on each of two pages and one non-markup annotation beside each:
/// Table 171's `Markup` column says `No` of `/Link` and of `/Popup`, and Table 246's own `/Annots`
/// note excludes `Link` by name.
#[test]
fn include_annotations_writes_the_markup_ones_with_the_page_each_is_on() {
    let document =
        Document::open(pdf_syntax::FileBytes::from(marked_up())).expect("the fixture parses");
    let view = ViewState::of(&document);
    let with = compose(&document, &view, &action(128, ""), None).expect("the composition succeeds");

    let read = fdf_data(&with);
    let carried: Vec<(Option<&str>, Option<usize>)> = read
        .annotations
        .iter()
        .map(|annotation| (annotation.subtype.as_deref(), annotation.page))
        .collect();
    assert_eq!(
        carried,
        vec![(Some("Highlight"), Some(0)), (Some("Text"), Some(1))],
        "the two markup annotations, each with its own page ordinal"
    );

    // What could not travel is named rather than dropped: `/AP` resolves to a stream of this
    // document's and `/P` is "[a]n indirect reference to the page object", which Table 254's
    // `/Page` replaces.
    assert!(
        with.owed
            .iter()
            .any(|owed| owed.contains("/AP") && owed.contains("no object space")),
        "{:?}",
        with.owed
    );

    // The calibration (trap 13): clear the bit and the same document yields no `/Annots` at all.
    let without = compose(&document, &view, &action(0, ""), None).expect("composes");
    assert!(
        fdf_data(&without).annotations.is_empty(),
        "\"If clear, markup annotations shall not be included.\""
    );
}

/// §12.7.6.2's Table 240 bit 11, which narrows bit 8 by a name this program cannot learn:
///
/// > If set, it shall include only those markup annotations whose T entry … matches the name of
/// > the current user, as determined by the remote server to which the form is being submitted.
///
/// *As determined by the remote server* — so the predicate belongs to the party with the network,
/// which principle 3 keeps out of this process. Of the two ways to be wrong, sending an annotation
/// that does not match breaks the bit's own *only* and sending none breaks nothing the table
/// states, because bit 11 is itself a narrowing of bit 8. The narrowing is applied whole, and the
/// sentence goes to the host that has the server.
#[test]
fn excluding_other_users_annotations_withholds_them_all_and_says_whose_name_is_missing() {
    let document =
        Document::open(pdf_syntax::FileBytes::from(marked_up())).expect("the fixture parses");
    let view = ViewState::of(&document);

    let all = compose(&document, &view, &action(128, ""), None).expect("composes");
    assert_eq!(
        fdf_data(&all).annotations.len(),
        2,
        "bit 8 alone carries both"
    );

    let narrowed = compose(&document, &view, &action(128 | 1024, ""), None).expect("composes");
    assert!(
        fdf_data(&narrowed).annotations.is_empty(),
        "and bit 11 beside it carries neither"
    );
    assert!(
        narrowed
            .owed
            .iter()
            .any(|owed| owed.contains("ExclNonUserAnnots") && owed.contains("current user")),
        "with the reason: {:?}",
        narrowed.owed
    );
}

/// Table 240 bit 14: "the F entry of the submitted FDF shall be a file specification containing an
/// embedded file stream representing the PDF file from which the FDF is being submitted".
#[test]
fn embed_form_puts_the_whole_document_in_the_fdfs_file_specification() {
    let submission = composed(8192, "");
    let (fdf_document, fdf) = fdf_dictionary(&submission);
    // §7.5.2 measures every offset in the table from the header, and this file has a second
    // thing that looks like one a few hundred bytes in — the embedded PDF's own. A document
    // recovered by scanning here would have found *that* file's objects (ADR 1066).
    assert!(
        !fdf_document.was_recovered(),
        "the FDF's own cross-reference table is what was read"
    );

    let specification = fdf_document.get_key(&fdf, "F");
    let specification = specification
        .as_dict()
        .expect("§7.11.3's dictionary form, which is the only one that can hold an /EF");
    assert!(
        fdf_document
            .get_key(specification, "Type")
            .as_name()
            .is_some_and(|name| name == &"Filespec"),
        "Table 43's /Type is \"[r]equired if an EF, EP or RF entry is present\""
    );
    let embedded = fdf_document.get_key(specification, "EF");
    let embedded = embedded.as_dict().expect("Table 43's /EF");
    let file = fdf_document.get_key(embedded, "F");
    let file = file.as_stream().expect("§7.11.4's embedded file stream");
    assert!(
        file.data.starts_with(b"%PDF-1.7"),
        "the file it was submitted from"
    );
    assert!(
        file.data.len() > form().len(),
        "with §7.5.6's update on the end of it: {} against {}",
        file.data.len(),
        form().len()
    );
    // Table 44's `/Subtype` "shall conform to the MIME media type names defined in Internet RFC
    // 2046", and §7.3.5 is what spells its SOLIDUS `#2F` on the way out.
    assert!(
        fdf_document
            .get_key(&file.dict, "Subtype")
            .as_name()
            .is_some_and(|name| name == &"application/pdf"),
        "Table 44's media type"
    );

    // What the specification cannot say is said instead, because Table 43 requires it and this
    // crate has no path to put in it.
    assert!(
        submission
            .owed
            .iter()
            .any(|owed| owed.contains("EmbedForm") && owed.contains("states no name")),
        "{:?}",
        submission.owed
    );
    // And the reader this program imports with sees the embedded file rather than reading the
    // specification as naming nothing.
    assert!(
        fdf_data(&submission)
            .owed
            .contains(&"/F: a file specification carrying the source document as an embedded file"),
        "{:?}",
        fdf_data(&submission).owed
    );

    // The calibration (trap 13): the bit clear, and there is no `/F` for anything to be in.
    let without = composed(0, "");
    let (without_document, without_fdf) = fdf_dictionary(&without);
    assert!(without_document.get_key(&without_fdf, "F").is_null());
}

/// Table 240 bit 12 against bit 14: "If set, the submitted FDF shall exclude the F entry."
///
/// Two flags of one table asking for opposite things about one entry, and the exclusion wins:
/// writing the `/F` would be choosing which of the two to disobey, where leaving it out obeys the
/// one that speaks about the entry's presence. What bit 14 then loses is named.
#[test]
fn excluding_the_f_key_beats_embedding_the_form() {
    let submission = composed(2048 | 8192, "");
    let (fdf_document, fdf) = fdf_dictionary(&submission);
    assert!(
        fdf_document.get_key(&fdf, "F").is_null(),
        "the entry bit 12 excludes"
    );
    assert!(
        submission
            .owed
            .iter()
            .any(|owed| owed.contains("ExclFKey") && owed.contains("EmbedForm")),
        "{:?}",
        submission.owed
    );
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
    // Table 240 bit 1 excludes object 10, the fixture's file-select control: §12.7.5.3 makes any
    // submission holding one `multipart/form-data` instead, which is the test below this.
    let submission = composed(4 | 1, "/Fields [(upload)]");
    assert_eq!(submission.format, Format::HtmlForm);
    assert_eq!(
        submission.media_type, "application/x-www-form-urlencoded",
        "no file-select control, so the format's own type"
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
/// The body is ISO 19444-1's. Its section 5.5.2 settles the first two lines of every XFDF
/// document — the UTF-8 declaration and `<xfdf xmlns=… xml:space="preserve">` — and its section
/// 5.6.2 lays out the `<ids>` and `<fields>` that follow. The identifier is the document's own
/// `/ID`, written as the uppercase hexadecimal of the same two byte strings the FDF body writes.
#[test]
fn xfdf_writes_the_document_the_specification_states() {
    let submission = composed(32, "");
    assert_eq!(submission.format, Format::Xfdf);
    assert_eq!(submission.media_type, "application/xfdf");
    assert_eq!(submission.method, Method::Post);
    let body = String::from_utf8(submission.body.clone()).expect("section 5.5.2 makes it UTF-8");
    assert!(
        body.starts_with(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <xfdf xmlns=\"http://ns.adobe.com/xfdf/\" xml:space=\"preserve\">\n"
        ),
        "section 5.5.2's two lines: {body}"
    );
    assert!(
        body.contains("<ids original=\"0102\" modified=\"0304\"/>"),
        "section 5.4.1 maps <ids> onto the FDF /ID, which this document states as <0102> <0304>: \
         {body}"
    );
    assert!(
        body.contains("<field name=\"name\">\n      <value>a value</value>"),
        "section 5.6.2's shape: {body}"
    );
    // ISO 19444-1:2019 section 5.6.3 represents a hierarchical name as nested `field` elements,
    // so the fixture's `group.inner` is two of them and not one name with a full stop in it.
    assert!(
        body.contains("<field name=\"group\">") && body.contains("<field name=\"inner\">"),
        "section 5.6.3's nesting: {body}"
    );
    // Section 5.6.2 explains `<f href>` as pointing at the PDF document holding the form fields,
    // which this process has no file name for — said rather than guessed at.
    assert!(
        submission.owed.iter().any(|owed| owed.contains("<f href>")),
        "the element that is not written is named: {:?}",
        submission.owed
    );
}

/// The round trip §12.7.6.4 and §12.7.6.2 make one question: what this program submits as XFDF is
/// what it imports from one.
///
/// Trap 13's calibration in the form the two clauses give it — the writer and the reader are
/// different code over the same grammar, so a name either of them spelled differently would show
/// here and nowhere else.
#[test]
fn an_xfdf_submission_is_read_back_by_the_importer_that_reads_one() {
    let submission = composed(32, "");
    let read = pdf_model::xfdf::read(&submission.body).expect("what this program wrote is XFDF");
    let names: Vec<&str> = read
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    assert!(
        names.contains(&"name") && names.contains(&"group.inner"),
        "§12.7.4.2's qualified names survive the nesting: {names:?}"
    );
    let value = read
        .fields
        .iter()
        .find(|field| field.name == "name")
        .and_then(|field| field.value.as_ref())
        .and_then(Object::as_string)
        .map(pdf_syntax::text_string);
    assert_eq!(value.as_deref(), Some("a value"));
    // §14.4's identifier crossed as the hexadecimal of the same bytes and came back as bytes.
    assert_eq!(
        read.identifier.as_ref().map(|pair| pair[0].clone()),
        Some(vec![0x01, 0x02])
    );
}

/// The eight bits Table 240 puts aside when bit 6 is set, each named rather than dropped.
///
/// Five of them say it in the same words — each "shall be used only when the form is being
/// submitted in Forms Data Format", which the table then defines as both bit 6 and bit 3 being
/// clear — and bits 3, 4 and 5 reach the same place through bit 3's own condition, which is that
/// bits 9 and 6 are clear. A document may set all of them, and this program neither applies one
/// nor stays quiet about it.
#[test]
fn the_bits_table_240_puts_aside_for_xfdf_are_named_one_by_one() {
    // Bits 3, 4, 5, 6, 7, 8, 10, 11, 12 and 14 together.
    let submission = composed(4 | 8 | 16 | 32 | 64 | 128 | 512 | 1024 | 2048 | 8192, "");
    assert_eq!(
        submission.format,
        Format::Xfdf,
        "bit 3 is meaningful only where bit 6 is clear, so bit 6 decides"
    );
    for bit in [
        "bit 3", "bit 4", "bit 5", "bit 7", "bit 8", "bit 10", "bit 11", "bit 12", "bit 14",
    ] {
        assert!(
            submission.owed.iter().any(|owed| owed.contains(bit)),
            "{bit} is named: {:?}",
            submission.owed
        );
    }
}

/// §12.7.5.3, under Table 231 bit 21:
///
/// > If the FileSelect flag ( PDF 1.4 ) is set, the field shall function as a file-select
/// > control. In this case, the field's text represents the pathname of a file whose contents
/// > shall be submitted as the field's value
///
/// The clause then splits by format, and this is the fixture's `upload`, whose `/V` is §7.11.1's
/// string form and so names a file without carrying it. In FDF that string *is* "a file
/// specification (7.11, "File specifications") identifying the selected file" and is what the
/// body states; in HTML Form format the part would carry the file's *contents*, which are on a
/// filesystem this crate has none of, so the field is named rather than sent under a value the
/// clause did not ask for. The specification that does carry its file is two tests below.
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
/// Two are left, and each is somebody else's knowledge rather than unwritten code. Bit 10's
/// `CanonicalFormat` converts "any submitted field values representing dates", and its own NOTE 1
/// says which those are is "not specified explicitly in the field itself but only in the
/// ECMAScript code that processes it" — which `CLAUDE.md` excludes. Bit 11's `ExclNonUserAnnots`
/// narrows by a name "determined by the remote server".
#[test]
fn every_flag_the_composition_does_not_apply_is_named() {
    let submission = composed(512 | 1024, "");
    let owed = submission.owed.join("\n");
    for named in ["CanonicalFormat", "ExclNonUserAnnots"] {
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

/// A form whose one field is §12.7.5.3's file-select control, with §7.11.3's dictionary form in
/// its `/V` and §7.11.4's embedded file stream inside that.
///
/// `stream_dictionary` is the embedded file stream's own dictionary, so that a stream this reader
/// cannot decode can be planted in the one place that matters.
fn uploading(stream_dictionary: &str, contents: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< >>\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
         /T (upload) /Ff 1048576 /V 6 0 R >>\nendobj\n\
         6 0 obj\n<< /Type /Filespec /F (report.txt) /UF (report.txt) \
         /EF << /F 7 0 R >> >>\nendobj\n\
         7 0 obj\n{stream_dictionary}\nstream\n{contents}\nendstream\nendobj\n"
    );
    assembled(&body)
}

/// The composition over [`uploading`], under these Table 240 flags.
fn uploaded(stream_dictionary: &str, contents: &str, flags: u32) -> Submission {
    let document = Document::open(pdf_syntax::FileBytes::from(uploading(
        stream_dictionary,
        contents,
    )))
    .expect("the fixture parses");
    let view = ViewState::of(&document);
    compose(&document, &view, &action(flags, ""), None).expect("the composition succeeds")
}

/// §12.7.5.3, the first bullet under Table 231 bit 21:
///
/// > For fields submitted in HTML Form format, the submission shall use the MIME content type
/// > multipart / form-data, as described in Internet RFC 2045.
///
/// The condition is the flag being set on a field in the submission, not the file behind it being
/// readable — the sentence is about the *submission* — so the fixture's `upload`, whose `/V` is a
/// pathname and nothing more, is enough to decide what the body is written as.
#[test]
fn a_file_select_control_makes_the_whole_html_body_multipart_form_data() {
    let submission = composed(4, "");
    assert_eq!(submission.format, Format::HtmlForm);
    assert!(
        submission
            .media_type
            .starts_with("multipart/form-data; boundary="),
        "RFC 2046 section 5.1.1 requires the parameter: {}",
        submission.media_type
    );
    let body = body(&submission);
    assert!(
        body.contains("\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\na value\r\n"),
        "HTML 4.01 section 17.13.4.2's part, CRLF throughout: {body:?}"
    );
    assert!(
        body.ends_with("--quorra-form-data-0--\r\n"),
        "RFC 2046 section 5.1.1's closing delimiter: {body:?}"
    );
    // The pathname is not submitted under the field's name: the clause asks for the contents.
    assert!(!body.contains("report.txt"), "{body:?}");
    assert!(
        submission
            .owed
            .iter()
            .any(|owed| owed.contains("field upload") && owed.contains("no filesystem")),
        "and the field is named rather than dropped: {:?}",
        submission.owed
    );
}

/// The same clause, where the specification carries its file: "a file specification (7.11, "File
/// specifications") identifying the selected file".
///
/// §7.11.1 gives a specification two forms and only the dictionary reaches §7.11.4's embedded file
/// stream, which is the one place a process with no filesystem can read a file from.
#[test]
fn a_file_select_controls_embedded_file_is_what_the_multipart_body_carries() {
    let submission = uploaded(
        "<< /Type /EmbeddedFile /Subtype /text#2Fplain /Length 11 >>",
        "hello there",
        4,
    );
    let body = body(&submission);
    assert!(
        body.contains(
            "Content-Disposition: form-data; name=\"upload\"; filename=\"report.txt\"\r\n\
             Content-Type: text/plain\r\n\r\nhello there\r\n"
        ),
        "HTML 4.01 section 17.13.4.2's file part: {body:?}"
    );
    assert_eq!(submission.fields, 1);
    assert!(submission.owed.is_empty(), "{:?}", submission.owed);
}

/// Trap 5, planted: an embedded file stream that will not decode is a file whose contents this
/// reader does not know, which is not a file of no bytes.
#[test]
fn an_embedded_file_that_will_not_decode_is_named_rather_than_submitted_empty() {
    let submission = uploaded(
        "<< /Type /EmbeddedFile /Filter /NoSuchDecode /Length 4 >>",
        "abcd",
        4,
    );
    assert_eq!(submission.fields, 0, "the field is left out");
    assert!(
        !body(&submission).contains("upload"),
        "{}",
        body(&submission)
    );
    assert!(
        submission
            .owed
            .iter()
            .any(|owed| owed.contains("field upload") && owed.contains("could not decode")),
        "{:?}",
        submission.owed
    );
}

/// The second bullet: "For Forms Data Format (FDF) submission, the value of the V entry in the FDF
/// field dictionary … shall be a file specification (7.11, "File specifications") identifying the
/// selected file."
///
/// The file travels rather than its name, because an FDF has a body to put §7.11.4's stream in and
/// the server has no copy of the sender's disk.
#[test]
fn a_file_select_controls_file_travels_into_the_fdf_as_a_specification() {
    let submission = uploaded(
        "<< /Type /EmbeddedFile /Subtype /text#2Fplain /Length 11 >>",
        "hello there",
        0,
    );
    let (document, fdf) = fdf_dictionary(&submission);
    let fields = document.get_key(&fdf, "Fields");
    let fields = fields.as_array().expect("Table 246's /Fields");
    let field = document.resolve(fields.first().expect("the one field"));
    let field = field.as_dict().expect("Table 249's dictionary").clone();
    let specification = document.get_key(&field, "V");
    let specification = specification.as_dict().expect("§7.11.3's dictionary form");
    assert_eq!(
        document
            .get_key(specification, "UF")
            .as_string()
            .map(pdf_syntax::text_string),
        Some("report.txt".to_owned()),
        "Table 43's name, carried from the document's own specification"
    );
    let attachment = pdf_model::attachment::read(&document, specification, String::new())
        .expect("Table 43's /EF names §7.11.4's stream");
    assert_eq!(
        document
            .decoded_stream_data_reported(&attachment.stream)
            .expect("it decodes")
            .data
            .as_ref(),
        b"hello there"
    );
    assert_eq!(attachment.media_type.as_deref(), Some("text/plain"));
}

/// Table 240 bit 4 against §12.7.5.3, which the document may ask for at once and which cannot both
/// be met: an HTTP GET's data is a URL query and has no entity body for a media type to describe.
#[test]
fn a_get_that_would_have_to_carry_a_file_is_composed_as_a_post() {
    let submission = composed(4 | 8, "");
    assert_eq!(submission.method, Method::Post);
    assert!(
        submission.media_type.starts_with("multipart/form-data"),
        "{}",
        submission.media_type
    );
    assert!(
        submission
            .owed
            .iter()
            .any(|owed| owed.contains("GetMethod") && owed.contains("multipart/form-data")),
        "{:?}",
        submission.owed
    );
}

/// HTML 4.01 section 17.13.4.2: "Part boundaries should not occur in any of the data; how this is
/// done lies outside the scope of this specification."
///
/// Planted, because an uncalibrated search for a free delimiter would return the first candidate
/// whatever the parts held (trap 13): the embedded file *is* the first candidate's delimiter line,
/// so a composition that did not look would split its own body in the wrong place.
#[test]
fn a_delimiter_the_parts_already_contain_is_not_the_one_the_body_is_written_with() {
    let contents = "--quorra-form-data-0";
    let submission = uploaded(
        &format!(
            "<< /Type /EmbeddedFile /Subtype /text#2Fplain /Length {} >>",
            contents.len()
        ),
        contents,
        4,
    );
    assert_eq!(
        submission.media_type, "multipart/form-data; boundary=quorra-form-data-1",
        "the first candidate is in the data"
    );
    let body = body(&submission);
    assert!(
        body.ends_with("--quorra-form-data-1--\r\n"),
        "and it is the one the body uses: {body:?}"
    );
}

/// Table 240 bit 5's two pairs, in a body that has no `&` and no `=` to join them with.
///
/// The table states the format `name . x = xval & name . y = yval`, which is two name/value pairs;
/// HTML 4.01 section 17.13.4.1 writes a pair as `name=value` and section 17.13.4.2 writes one as a
/// part, so the pairs are the same and the punctuation is the format's.
#[test]
fn the_coordinates_are_two_parts_in_a_multipart_body() {
    let document = document();
    let view = ViewState::of(&document);
    let click = Click {
        point: (120.0, 60.0),
        widget: Some(ObjectId::new(15, 0)),
    };
    let submission = compose(&document, &view, &action(4 | 16, ""), Some(&click))
        .expect("the composition succeeds");
    let body = body(&submission);
    assert!(
        body.contains("Content-Disposition: form-data; name=\"short.x\"\r\n\r\n20\r\n"),
        "{body:?}"
    );
    assert!(
        body.contains("Content-Disposition: form-data; name=\"short.y\"\r\n\r\n10\r\n"),
        "{body:?}"
    );
}

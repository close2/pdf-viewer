//! A Required empty field is reported, while Table 240 still controls its export shape and the
//! owner's host policy still controls whether the composed request may leave the machine.
//! The deliberately missing value does not meet Table 227 bit 2 at export. This characterizes
//! ADR 1198’s reporting policy; it does not claim fully conforming export. No request is sent.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "exact-xref fixture and policy test assertions"
)]

use std::fmt::Write as _;

use pdf_model::action::Action;
use pdf_model::forms_data::FormsData;
use pdf_model::submission::{Submission, compose};
use pdf_model::view::ViewState;
use pdf_syntax::{Document, FileBytes, Object, ObjectId};
use viewer_core::{Command, DocumentId, Event, Viewer};
use viewer_host::{Sending, Submissions, may_submit};

fn pdf(include_no_value_fields: bool) -> Vec<u8> {
    let flags = if include_no_value_fields { 2 } else { 0 };
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /OpenAction 7 0 R /AcroForm << /Fields [5 0 R] \
         /NeedAppearances true /DA (/Helv 12 Tf 0 g) \
         /DR << /Font << /Helv 6 0 R >> >> >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Contents 4 0 R /Annots [5 0 R] >>",
        "<< /Length 0 >>\nstream\n\nendstream",
        "<< /Type /Annot /Subtype /Widget /Rect [20 60 180 90] /P 3 0 R /F 4 \
         /FT /Tx /T (required) /Ff 2 /DA (/Helv 12 Tf 0 g) /A 7 0 R >>",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        // Reserved example URL; only the policy seam is called, never a network client.
        "<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/submit) >> \
         /Fields [(required)] /Flags FLAGS >>",
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = vec![0_usize];
    for (index, source) in objects.iter().enumerate() {
        let number = index.saturating_add(1);
        offsets.push(out.len());
        let object = source.replace("FLAGS", &flags.to_string());
        writeln!(out, "{number} 0 obj\n{object}\nendobj").expect("in-memory write");
    }
    let xref = out.len();
    writeln!(out, "xref\n0 {}\n0000000000 65535 f ", offsets.len()).expect("in-memory write");
    for offset in offsets.iter().skip(1) {
        writeln!(out, "{offset:010} 00000 n ").expect("in-memory write");
    }
    write!(out, "trailer\n<< /Size {} /Root 1 0 R /ID [<01020304> <05060708>] >>\nstartxref\n{xref}\n%%EOF\n", offsets.len())
        .expect("in-memory write");
    out.into_bytes()
}

fn compose_fixture(include_no_value_fields: bool) -> Submission {
    let document = Document::open(FileBytes::from(pdf(include_no_value_fields)))
        .expect("well-formed PDF fixture");
    assert!(!document.was_recovered(), "xref offsets are exact");
    let actions = pdf_model::action::read(&document, &Object::Reference(ObjectId::new(7, 0)));
    let Some(Action::SubmitForm(action)) = actions.into_iter().next() else {
        panic!("the widget's action is SubmitForm");
    };
    let view = ViewState::of(&document);
    compose(&document, &view, &action, None).expect("action composes")
}

fn exported_fields(submission: &Submission) -> Vec<(String, bool)> {
    let exported = Document::open(FileBytes::from(submission.body.clone())).expect("FDF parses");
    let fields = FormsData::read(&exported)
        .expect("body identifies itself as FDF")
        .fields;
    fields
        .into_iter()
        .map(|field| (field.name, field.value.is_some()))
        .collect()
}

#[test]
fn required_empty_is_reported_and_include_no_value_policy_remains_explicit() {
    let included = compose_fixture(true);
    assert_eq!(included.fields, 1);
    assert_eq!(exported_fields(&included), [("required".to_owned(), false)]);
    assert_eq!(
        included.owed.len(),
        1,
        "one required field has no value: {:?}",
        included.owed
    );
    assert!(included.owed[0].contains("Table 227 bit 2"));
    assert!(included.owed[0].contains("required"));

    // The report does not silently turn into a refusal: the owner-level Send choice can proceed,
    // while Ask and Refuse remain distinguishable. This inspects a policy result only; it opens
    // no socket and makes no claim that ISO requires one particular host choice.
    assert_eq!(may_submit(&included, Submissions::Send), Sending::Send);
    assert!(matches!(
        may_submit(&included, Submissions::Warn),
        Sending::Warn(_)
    ));
    assert!(matches!(
        may_submit(&included, Submissions::Ask),
        Sending::Ask(_)
    ));
    assert!(matches!(
        may_submit(&included, Submissions::Refuse),
        Sending::Refuse(_)
    ));

    let omitted = compose_fixture(false);
    assert_eq!(omitted.fields, 0);
    assert_eq!(exported_fields(&omitted), [] as [(String, bool); 0]);
    assert_eq!(omitted.owed.len(), 1);
    assert!(omitted.owed[0].contains("required"));
}

#[test]
fn required_diagnostic_reaches_the_host_before_the_submission_event() {
    // Exercise the real core event route. This asks no host to transmit the request.
    let mut viewer = Viewer::new(400, 400, 1.0);
    let mut events: Vec<_> = viewer
        .handle(Command::Open {
            id: DocumentId(92),
            bytes: pdf(true).into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Opened { pages: 1, .. }))
    );
    events.extend(viewer.handle(Command::Presented));
    let reports: Vec<_> = events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| match event {
            Event::Reported { notes, .. }
                if notes
                    .iter()
                    .any(|note| note.contains("Table 227 bit 2") && note.contains("required")) =>
            {
                Some(index)
            }
            _ => None,
        })
        .collect();
    assert_eq!(reports.len(), 1, "one Required diagnostic reaches the host");
    let submissions: Vec<_> = events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| match event {
            Event::Submit { submission, .. } => Some((index, submission)),
            _ => None,
        })
        .collect();
    assert_eq!(
        submissions.len(),
        1,
        "one composed submission reaches the host"
    );
    let (index, submission) = submissions[0];
    assert!(
        reports[0] < index,
        "the host is told before it receives the request"
    );
    assert_eq!(
        exported_fields(submission),
        [("required".to_owned(), false)]
    );
    assert_eq!(may_submit(submission, Submissions::Send), Sending::Send);
}

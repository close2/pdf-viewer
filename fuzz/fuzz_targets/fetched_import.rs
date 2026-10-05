//! Fuzzes a form's data as a server answers it: Table Annex O.4's `fdf` naming an absolute URI, or
//! §12.7.6.2's submission, fetched by a host and handed to the viewer as `Command::Respond`
//! (ADR 1527).
//!
//! A network answer is the most untrusted byte this program reads. A document is a file a person
//! chose to open; a server's body is whatever the server sent, under whatever `Content-Type` it
//! claimed, to a request a *document* caused. `forms_data` and `xfdf` fuzz the two readers on their
//! own, against a target with no fields or with one. This target fuzzes what a host does with the
//! bytes: `viewer_core::Viewer::handle` on `Command::Respond`, which reaches `interact::import` —
//! the reader the name chose, §14.4's identifier comparison, Table 246's `/Status`,
//! `ViewState::import` against a form with a field tree, a check box, a choice and a §12.7.7
//! template, the page count a template moves, and the outcome applied to the document in front or
//! to one behind it.
//!
//! The first byte chooses the route; the rest is the body:
//!
//! - bit 0: §12.7.8.1's extension, FDF or XFDF, which is the reader `data_format` would choose;
//! - bit 1: the answer is for the document in front, or for the one behind it (ADR 1527 section
//!   3: an answer arrives when the server sends it, and the person may have changed tabs);
//! - bit 2: the request it answers, a submission or a fetched import.
//!
//! Beyond never panicking, two properties, both ADR 1527's:
//!
//! - **The answer is said.** Every answer to an open document ends in at least one sentence about
//!   that document — what was imported, or why nothing could be — because `interact::import`
//!   returning nothing would be a network answer dropped in silence.
//! - **The answer is said about the document it names, and only that one.** No sentence the answer
//!   causes is reported against the other document, whichever is in front (trap 104).

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_model::action::DataFormat;
use viewer_core::{Answered, Command, DocumentId, Event, Viewer};

/// The document opened first, which is behind once the second opens.
const BEHIND: DocumentId = DocumentId(1);
/// The document opened second, which is in front.
const IN_FRONT: DocumentId = DocumentId(2);

/// A one-page form: a text field under a non-terminal `b`, a check box, a list box and a template
/// named `t`, with a `/ID` an FDF's `/ID` is compared against. No cross-reference section, so
/// `Document::open`'s scan finds the objects, as it does for most real FDF files.
const FORM: &[u8] = b"%PDF-1.7
1 0 obj << /Type /Catalog /Pages 2 0 R /Names << /Templates << /Names [(t) 7 0 R] >> >>
  /AcroForm << /Fields [4 0 R 5 0 R 6 0 R] /DA (/Helv 10 Tf 0 g)
  /DR << /Font << /Helv 9 0 R >> >> >> >> endobj
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [10 0 R 5 0 R 6 0 R] >> endobj
4 0 obj << /T (b) /Kids [10 0 R] >> endobj
10 0 obj << /Type /Annot /Subtype /Widget /FT /Tx /T (c) /Parent 4 0 R /Rect [10 10 190 40] /P 3 0 R >> endobj
5 0 obj << /Type /Annot /Subtype /Widget /FT /Btn /T (check) /V /Off /AS /Off /Rect [10 50 30 70] /P 3 0 R
  /AP << /N << /On 8 0 R /Off 8 0 R >> >> >> endobj
6 0 obj << /Type /Annot /Subtype /Widget /FT /Ch /T (list) /Opt [(one) (two)] /Rect [10 80 190 120] /P 3 0 R >> endobj
7 0 obj << /Type /Template /MediaBox [0 0 100 50] /Contents 8 0 R >> endobj
8 0 obj << /Length 0 >>
stream

endstream
endobj
9 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >> endobj
trailer << /Root 1 0 R /Size 11 /ID [<0102> <0304>] >>
%%EOF
";

fuzz_target!(|data: &[u8]| {
    let Some((&route, body)) = data.split_first() else {
        return;
    };
    // Longer bodies say nothing new about either reader and turn every case into a memory test of
    // the object reader, which `document` already is; `forms_data` stops at the same size.
    if body.len() > 16 * 1024 {
        return;
    }
    let format = if route & 1 == 0 {
        DataFormat::Fdf
    } else {
        DataFormat::Xfdf
    };
    let (answered, other) = if route & 2 == 0 {
        (IN_FRONT, BEHIND)
    } else {
        (BEHIND, IN_FRONT)
    };
    let answers = if route & 4 == 0 {
        Answered::Import
    } else {
        Answered::Submission
    };

    let mut viewer = Viewer::new(400, 400, 1.0);
    for id in [BEHIND, IN_FRONT] {
        let failed = viewer
            .handle(Command::Open {
                id,
                bytes: FORM.to_vec().into(),
                password: None,
                fragment: Some("fdf=https://example.org/answer.fdf".to_owned()),
            })
            .any(|event| matches!(event, Event::OpenFailed { .. }));
        assert!(!failed, "the fixed form opens");
    }

    let mut said = false;
    for event in viewer.handle(Command::Respond {
        document: answered,
        answers,
        source: "https://example.org/answer.fdf".to_owned(),
        format,
        bytes: body.to_vec(),
    }) {
        if let Event::Reported { document, .. } = event {
            assert!(
                document != other,
                "an answer for {answered:?} was reported against {other:?}"
            );
            said |= document == answered;
        }
    }
    assert!(
        said,
        "an answer to an open document was dropped without a sentence"
    );
});

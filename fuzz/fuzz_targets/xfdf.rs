//! Fuzzes the XML Forms Data Format reader, ISO 32000-2 §12.7.6.4's `/F` "FDF, XFDF or any other
//! data format file from which to import the data", and the import it feeds.
//!
//! An XFDF file is a file a person was handed, as untrusted as a PDF: `pdf_model::xfdf::read`
//! tokenises XML, follows a nesting of `<field>` elements into fully qualified names, and reads an
//! `<annots>` element into annotations the import may create (ADR 1297). The grammar is ISO
//! 19444-1's; its sections are cited, never quoted, in the module itself.
//!
//! Beyond never panicking, three properties:
//!
//! - **The reader is a function of the bytes.** Reading the same file twice gives the same
//!   answer; a tokenizer state that leaked between elements is what one pass cannot see.
//! - **Its budget holds**: no more fields than the reader's own bound of 65 536 at the top level,
//!   which is `forms_data`'s `MAX_FIELDS` and the one it shares with FDF.
//! - **The import terminates** against a one-page document with one text field, which is the
//!   half that creates annotations and walks both name spaces at once.

#![no_main]
#![expect(
    clippy::expect_used,
    reason = "a fuzz target states its properties by failing: `expect` and `panic!` are how a violated one reaches libFuzzer, and each message here names the property rather than the call"
)]

use libfuzzer_sys::fuzz_target;
use pdf_syntax::Document;

/// The reader's field bound, as `pdf_model::forms_data` states it.
const MAX_FIELDS: usize = 65536;

/// A one-page document with one text field named `a`, for the import to meet.
const TARGET: &[u8] = b"%PDF-1.7
1 0 obj << /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >> endobj
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [4 0 R] >> endobj
4 0 obj << /Type /Annot /Subtype /Widget /FT /Tx /T (a) /Rect [10 10 190 40] /P 3 0 R >> endobj
trailer << /Root 1 0 R /Size 5 >>
%%EOF
";

fuzz_target!(|bytes: &[u8]| {
    let Ok(data) = pdf_model::xfdf::read(bytes) else {
        return;
    };
    assert_eq!(
        pdf_model::xfdf::read(bytes).ok().as_ref(),
        Some(&data),
        "reading the same file twice gives the same answer"
    );
    assert!(
        data.fields.len() <= MAX_FIELDS,
        "no more fields than the reader's bound"
    );
    let document = Document::open(TARGET.to_vec()).expect("the fixed target document opens");
    let mut view = pdf_model::view::ViewState::of(&document);
    let _ = view.import(&document, &data);
});

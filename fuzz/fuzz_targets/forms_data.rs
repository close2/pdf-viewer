//! Fuzzes ISO 32000-2 §12.7.8's Forms Data Format and §7.9.4's date strings.
//!
//! Two parsers, one target, because both take **bytes a second file chooses** and neither is
//! reached by the four targets that existed before this one. An FDF file is a whole document a
//! person may be handed beside the one they opened, and §12.7.6.4's import-data action is a
//! document *asking* for it — so its bytes are exactly as untrusted as a PDF's, and rather more
//! surprising, because nothing else in this tree opens a second file at all.
//!
//! The fuzzer's bytes are handed to `Document::open` as an FDF file, which exercises:
//!
//! - **§12.7.8.2.2's header.** `xref::read` looks for `%FDF-` where `%PDF-` is absent, and
//!   §7.5.2's rule makes every byte offset in the file relative to whichever it found. An input
//!   that puts one marker inside the other is what this notices.
//! - **§12.7.8.3.2's `/Kids` tree.** A path down it is bounded by `MAX_FIELD_DEPTH` and a fully
//!   qualified name is built by concatenation, so a deep tree of long partial names is a
//!   quadratic allocation waiting to be found.
//! - **Table 249's flag arithmetic.** `/Ff`, `/SetFf` and `/ClrFf` are integers a file states and
//!   the arithmetic combines them.
//! - **§7.9.4's date grammar**, which every FDF field's `/T` is *not* — so the dates are fed
//!   separately below, directly, because reaching `Date::parse` through a document would test the
//!   document reader instead. The two `#[expect(arithmetic_side_effects)]` blocks in `date.rs`
//!   are the reason this half exists: each is argued from bounds the parse establishes, and a
//!   fuzzer is what checks the argument rather than the sentence.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_syntax::{Date, Document, Limits};

fuzz_target!(|bytes: &[u8]| {
    // Longer inputs say nothing new about either grammar and turn every case into a memory
    // test of the object reader, which the `document` target already is.
    if bytes.len() > 8192 {
        return;
    }

    // §7.9.4 first, on the bytes as they stand: the grammar is over characters, so the input is
    // taken as a text string exactly as a `/M` or a `/CreationDate` would be.
    let text = pdf_syntax::text_string(bytes);
    if let Some(date) = Date::parse(&text) {
        // The two properties worth stating. An instant is arithmetic over a year the parse has
        // bounded to four digits, and it is what an ordering rests on; `to_string` writes every
        // field out and must not disagree with itself about how many characters that takes.
        let _ = date.instant();
        let _ = date.to_string();
        assert!(date.month >= 1 && date.month <= 12, "{date:?}");
        assert!(date.day >= 1 && date.day <= 31, "{date:?}");
        assert!(
            date.hour <= 23 && date.minute <= 59 && date.second <= 59,
            "{date:?}"
        );
        assert!(
            date.offset
                .is_none_or(|minutes| minutes.abs() <= 23 * 60 + 59),
            "{date:?}"
        );
    }

    // Then the whole input as an FDF file. `Document::open` recovers by scanning where there is
    // no cross-reference table, which is how most real FDF files are written and what
    // §12.7.8.1 permits.
    let Ok(document) = Document::open_with_limits(bytes.to_vec(), Limits::default()) else {
        return;
    };
    let Ok(data) = pdf_model::forms_data::FormsData::read(&document) else {
        return;
    };
    // Reading the file is half; matching it against a document is the other, and it is the half
    // that walks both name spaces at once. The target document here is the FDF file itself,
    // which is a document with no `/AcroForm` — so what this exercises is the pairing's answer
    // for a form that has none of the names, which is the commonest real answer.
    let mut view = pdf_model::view::ViewState::of(&document);
    let _ = view.import(&document, &data);
});

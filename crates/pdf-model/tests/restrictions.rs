//! §7.6.4.2's Table 22 against the corpus documents that actually withhold something.
//!
//! `restriction.rs`'s own tests state the table as arithmetic, which is the only honest way to
//! test a revision-2 rule the corpus has one witness for. This is the other end: a real file,
//! encrypted by a real producer, whose flag word withholds one of this program's two operations
//! and grants the other — which is what makes it evidence that the two are read separately.

use std::path::{Path, PathBuf};

use pdf_model::restriction::{Operation, Restriction, asserted};
use pdf_syntax::{Document, Limits};

/// A corpus document's bytes, or `None` when the submodule is not checked out.
fn corpus_bytes(name: &str) -> Option<Vec<u8>> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    std::fs::read(path).ok()
}

/// One encrypted document grants filling in a field and withholds annotating.
///
/// `bug1815476.pdf` states `/R 4` and `/P -1084`, which is `0xFFFFFBC4`: bit 6 clear, bit 9 set.
/// §7.6.4.2's Table 22 gives those two positions different jobs —
///
/// Bit 6:
///
/// > Add or modify text annotations, fill in interactive form fields, and, if bit 4 is also set,
/// > create or modify interactive form fields (including signature fields).
///
/// Bit 9:
///
/// > ( Security handlers of revision 3 or greater ) Fill in existing interactive form fields
/// > (including signature fields), even if bit 6 is clear.
///
/// — so this file's author permitted a person to fill the form in and not to comment on it. It
/// carries an `/AcroForm`, so both operations are ones a reader could actually attempt here.
///
/// **Measured over the whole corpus in the three-hundred-and-seventy-third session**, by running
/// `asserted` over every one of them: 26 of the 974 documents carry an `/Encrypt`, 19 open (the
/// other 7 want a password nobody has), 4 of those open as the *owner* — and **6 withhold one of
/// these two operations**: annotating is withheld by all six and filling in by four.
/// Over the whole 974 — encrypted or not — **968 open and 7 assert something against one of the
/// two operations**: the six here, and `xfa_filled_imm1344e.pdf`'s certification signature below.
/// `bug1815476.pdf` and `secHandler.pdf` withhold annotating alone; `issue17215.pdf`,
/// `issue19484_1.pdf` and `issue19484_2.pdf` withhold both; and `bug900822.pdf` withholds both
/// **because of the revision rule** — it is `/R 2 /P -60`, so its bit 9 is set by Table 22's own
/// reservation and means nothing, and a reader consulting it would let a person fill in a form
/// its author had closed. `print_protection.pdf` clears every bit and is exempt regardless,
/// because `1234` is its owner password and §7.6.4.1 says that "should allow full (owner)
/// access".
#[test]
fn an_encrypted_document_can_grant_one_operation_and_withhold_the_other() {
    let Some(bytes) = corpus_bytes("bug1815476.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let document =
        Document::open_with_password(bytes, Limits::DEFAULT, "").expect("opens with no password");
    let permissions = document.permissions().expect("the document is encrypted");
    assert_eq!(permissions.revision, 4, "/R 4");
    assert!(!permissions.owner, "the empty password is the user's here");

    assert_eq!(
        asserted(&document, Operation::FillInForm, None, None),
        Vec::new(),
        "bit 9 grants filling in a field at revision 3 or greater"
    );
    assert_eq!(
        asserted(&document, Operation::Annotate, None, None),
        vec![Restriction::AccessDenied { bit: 6 }],
        "and bit 6, which is what annotating needs, is clear"
    );
}

/// The corpus's one certification signature withholds annotating and permits filling in.
///
/// `xfa_filled_imm1344e.pdf` is the only one of the 974 that states a `/Perms /DocMDP`, and its
/// `/P` is 2 — Table 257's "filling in forms, instantiating page templates, and signing", which
/// level 3 would extend with "annotation creation, deletion, and modification". So the file that
/// has 2.5 MB of filled-in form appended after its own signature is also the file that says a
/// reader may not comment on it, and this program can now tell the two apart.
#[test]
fn the_corpus_certification_permits_filling_in_and_not_annotating() {
    let Some(bytes) = corpus_bytes("xfa_filled_imm1344e.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let document = Document::open(bytes).expect("a valid PDF");
    assert_eq!(
        asserted(&document, Operation::FillInForm, None, None),
        Vec::new()
    );
    assert_eq!(
        asserted(&document, Operation::Annotate, None, None),
        vec![Restriction::Certified {
            level: pdf_model::signature::Modification::FormFilling
        }]
    );
}

/// The same signature's *second* transform, which this tree read nothing of until ADR 0403.
///
/// `xfa_filled_imm1344e.pdf`'s `/Perms /DocMDP` signature carries a `/Reference` array of two
/// signature reference dictionaries — a `DocMDP` and a **`FieldMDP`** — and §12.8.2.1 makes that
/// array plural for exactly this reason: "[t]ransform methods, along with transform parameters,
/// shall determine which objects are included and excluded in revision comparison". Reading the
/// first and stopping is what a `/Reference` array cannot be read as.
///
/// **The field it names is the whole of this document's form**, and its name is Table 259's
/// verbatim: `form1[0].SignatureField3[0]`, which `pdf_model::form::fields` independently derives
/// as §12.7.4.2's fully qualified name for the one field on page 1. That agreement is what
/// `FieldSelection::covers` was written on argument alone and now has a producer's own file for —
/// a partial-name reading would have matched here too, and a document with two `SignatureField3`
/// under different parents is what it would have got wrong.
///
/// The restriction that results does not withhold anything this program does: the covered field
/// is a signature field, and `ViewState::set_field` fills in text and choice fields. It is
/// asserted anyway, because [`asserted`] answers what the *document* says rather than what this
/// program happens to be able to do — a host that grows a way to sign is owed the sentence
/// without anyone remembering to add it.
#[test]
fn the_corpus_certification_also_covers_a_field_by_name() {
    let Some(bytes) = corpus_bytes("xfa_filled_imm1344e.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let document = Document::open(bytes).expect("a valid PDF");
    assert_eq!(
        asserted(
            &document,
            Operation::FillInForm,
            Some("form1[0].SignatureField3[0]"),
            None
        ),
        vec![Restriction::FieldCovered]
    );
    assert_eq!(
        asserted(
            &document,
            Operation::FillInForm,
            Some("form1[0].SignatureField3"),
            None
        ),
        Vec::new(),
        "the name Table 259 states is the whole name, and a prefix of it is another field"
    );
    // §12.8.2.4 is about "the values of a list of form fields" and says nothing about annotating,
    // so the only reason that survives here is §12.8.2.2's level.
    assert_eq!(
        asserted(
            &document,
            Operation::Annotate,
            Some("form1[0].SignatureField3[0]"),
            None
        ),
        vec![Restriction::Certified {
            level: pdf_model::signature::Modification::FormFilling
        }]
    );
}

/// A document that is not encrypted and states no `/Perms` asserts nothing.
///
/// The answer for 961 of the 968 corpus documents that open, and worth pinning: an empty list has
/// to mean "nothing withheld" rather than "not looked at", because that is what the caller acts
/// on.
#[test]
fn an_ordinary_document_asserts_nothing_against_either_operation() {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
    let bytes = std::fs::read(&path).expect("a committed document");
    let document = Document::open(bytes).expect("a valid PDF");
    assert!(document.permissions().is_none(), "it is not encrypted");
    for operation in [Operation::FillInForm, Operation::Annotate] {
        assert_eq!(asserted(&document, operation, None, None), Vec::new());
    }
}

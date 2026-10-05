//! §7.6.4.2's Table 22 against the corpus documents that actually withhold something.
//!
//! `restriction.rs`'s own tests state the table as arithmetic, which is the only honest way to
//! test a revision-2 rule the corpus has one witness for. This is the other end: a real file,
//! encrypted by a real producer, whose flag word withholds one of this program's two operations
//! and grants the other — which is what makes it evidence that the two are read separately.

use std::path::{Path, PathBuf};

use pdf_model::restriction::{Bit, Operation, Restriction, asserted};
use pdf_syntax::{Document, Limits};

#[path = "support/corpus_passwords.rs"]
#[expect(
    dead_code,
    reason = "the references' spelling of a password is the oracle's; this census opens the \
              document itself and has no reference to hand one to"
)]
mod corpus_passwords;

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
/// **Measured over the whole corpus** (ADR 0212), by running `asserted` over every one of them,
/// each opened on the default user password and then on the password `corpus_passwords` publishes
/// for it (ADR 1534) — [`the_corpus_census_names_every_document_that_withholds_either_operation`]
/// below, and `encryption_census` for the encrypted ones: 25 of the 974 documents carry an
/// `/Encrypt`, 24 open (`PDFBOX-4352-0.pdf`'s resolves to null and is refused), 4 of those as the
/// *owner* — and **9 withhold one of these two operations**: annotating is withheld by all nine
/// and filling in by six. Over the whole 974 — encrypted or not — **973 open and 10 assert
/// something against one of the two operations**: the nine here, and `xfa_filled_imm1344e.pdf`'s
/// certification signature below. `bug1815476.pdf`, `issue21579.pdf` and `secHandler.pdf`
/// withhold annotating alone; `bug1782186.pdf`, `issue15893_reduced.pdf`, `issue17215.pdf`,
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
        vec![Restriction::AccessDenied { bit: Bit::Annotate }],
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
            level: pdf_signature::signature::Modification::FormFilling
        }]
    );
}

/// The same certification against the three batch operations, through `decide`, at every level.
///
/// Table 257's three levels are all about "changes to the document": rendering a page and
/// copying a file out change nothing, so no level of certification withholds `Print` or
/// `Extract`; writing a file in is a change none of the three permits, so `Modify` is withheld
/// at `/P 2` — and what a caller then does is the level's, [`Level::verdict`], not this crate's.
#[test]
fn a_certification_withholds_a_change_and_not_a_reading_at_every_level() {
    use pdf_model::restriction::{Level, Verdict, decide};
    let Some(bytes) = corpus_bytes("xfa_filled_imm1344e.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let document = Document::open(bytes).expect("a valid PDF");
    let certified = vec![Restriction::Certified {
        level: pdf_signature::signature::Modification::FormFilling,
    }];
    for level in [Level::Off, Level::On, Level::Ask, Level::Warn] {
        assert_eq!(
            decide(level, &document, Operation::Print, None, None),
            Verdict::Proceed
        );
        assert_eq!(
            decide(level, &document, Operation::Extract, None, None),
            Verdict::Proceed
        );
        let expected = match level {
            Level::Off => Verdict::Proceed,
            Level::On => Verdict::Refuse(certified.clone()),
            Level::Ask => Verdict::Ask(certified.clone()),
            Level::Warn => Verdict::Warn(certified.clone()),
        };
        assert_eq!(
            decide(level, &document, Operation::Modify, None, None),
            expected,
            "{level:?}"
        );
    }
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
            level: pdf_signature::signature::Modification::FormFilling
        }]
    );
}

/// A document that is not encrypted and states no `/Perms` asserts nothing.
///
/// The answer for 963 of the 973 corpus documents that open, and worth pinning: an empty list has
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

/// One corpus document, as [`the_corpus_census_names_every_document_that_withholds_either_operation`]
/// found it.
#[expect(
    clippy::struct_excessive_bools,
    reason = "a census row is one column per question asked of one document, each a yes or a no"
)]
struct Row<'a> {
    /// The file's name under `doc/pdf.js/test/pdfs/`.
    name: &'a str,
    /// Whether it opened, on the default user password or on its published one.
    opened: bool,
    /// Whether it is encrypted: opened under a security handler, or refused stating `/Encrypt`.
    encrypted: bool,
    /// Whether the password that matched was the owner's.
    owner: bool,
    /// Whether anything it states withholds annotating.
    withholds_annotating: bool,
    /// Whether anything it states withholds filling in a field.
    withholds_filling: bool,
}

/// The corpus census the counts in this file's comments come from, by name rather than by number.
///
/// Every pdf.js corpus document is opened as §7.6.4.1 has a reader open it — the default user
/// password first, then the password `corpus_passwords` publishes for it (ADR 1534) — and asked,
/// through [`asserted`], what it withholds from filling in a field and from annotating. It prints
/// the counts and holds the documents that withhold either by name, so a document that starts or
/// stops asserting a restriction is a named change rather than a number that drifted:
///
/// ```sh
/// cargo test -p pdf-model --test restrictions -- --ignored --nocapture the_corpus_census
/// ```
#[test]
// not a gate: the census behind this file's counts, run when one is re-derived; it walks a
// checkout the gates do not require, and the corpus gate already opens every document (ADR 1534)
#[ignore = "a walk over the pdf.js corpus: run it behind the heavy-walk lock"]
fn the_corpus_census_names_every_document_that_withholds_either_operation() {
    let directory: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let Ok(entries) = std::fs::read_dir(&directory) else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            Path::new(name)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
        })
        .collect();
    names.sort();

    let mut rows = Vec::new();
    for name in &names {
        let Some(bytes) = corpus_bytes(name) else {
            continue;
        };
        let states_encrypt = bytes.windows(b"/Encrypt".len()).any(|w| w == b"/Encrypt");
        let password = corpus_passwords::corpus_password(name).map_or("", |known| known.password);
        let document = Document::open_with_password(bytes.clone(), Limits::DEFAULT, "")
            .or_else(|_| Document::open_with_password(bytes, Limits::DEFAULT, password));
        rows.push(match document {
            Err(_) => Row {
                name,
                opened: false,
                encrypted: states_encrypt,
                owner: false,
                withholds_annotating: false,
                withholds_filling: false,
            },
            Ok(document) => Row {
                name,
                opened: true,
                encrypted: document.is_encrypted(),
                owner: document.permissions().is_some_and(|granted| granted.owner),
                withholds_annotating: !asserted(&document, Operation::Annotate, None, None)
                    .is_empty(),
                withholds_filling: !asserted(&document, Operation::FillInForm, None, None)
                    .is_empty(),
            },
        });
    }
    let count = |keep: fn(&Row<'_>) -> bool| rows.iter().filter(|row| keep(row)).count();
    let unopened: Vec<&str> = rows
        .iter()
        .filter(|row| row.encrypted && !row.opened)
        .map(|row| row.name)
        .collect();
    println!(
        "{} documents, {} open; {} encrypted, {} of those open ({} as the owner), and these do \
         not: {unopened:?}",
        rows.len(),
        count(|row| row.opened),
        count(|row| row.encrypted),
        count(|row| row.opened && row.encrypted),
        count(|row| row.owner),
    );
    let withholding: Vec<&Row<'_>> = rows
        .iter()
        .filter(|row| row.withholds_annotating || row.withholds_filling)
        .collect();
    for row in &withholding {
        println!(
            "  {}: withholds annotating {}, filling in {}",
            row.name, row.withholds_annotating, row.withholds_filling
        );
    }
    let withholders: Vec<&str> = withholding.iter().map(|row| row.name).collect();
    assert_eq!(
        withholders,
        [
            "bug1782186.pdf",
            "bug1815476.pdf",
            "bug900822.pdf",
            "issue15893_reduced.pdf",
            "issue17215.pdf",
            "issue19484_1.pdf",
            "issue19484_2.pdf",
            "issue21579.pdf",
            "secHandler.pdf",
            "xfa_filled_imm1344e.pdf",
        ],
        "the documents that withhold either operation, by name"
    );
}

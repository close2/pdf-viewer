//! ISO 32000-2 §7.6's populations, counted rather than remembered.
//!
//! Three ledger rows state a count over the corpus's encrypted documents — §7.6's "of the
//! corpus's N encrypted documents", §7.6.4.1's locked files and §7.6.4.2's "N carry an
//! `/Encrypt`, N open, N of those as the owner" — and none of them named a command. `doc/todo/01`'s
//! rule is that a note stating a count over the corpus names the command that produces it; this is
//! that command, and it exists because two of those rows disagreed with each other and with the
//! corpus gate's own ratchet, with nothing in the tree able to say which was right.
//!
//! What it reports, per document that states a trailer `/Encrypt`:
//!
//! - Table 20's `/Filter` and `/V` and Table 21's `/R`, which is what decides which algorithms
//!   run at all;
//! - whether the default user password §7.6.4.1 makes a reader try first opens it, whether the
//!   file is refused outright as an encryption this reader does not implement, and whether it is
//!   locked — encrypted, well formed, and opened by neither the empty password nor any password
//!   given on the command line;
//! - for the ones that open, whether the password that matched was the owner's, and what
//!   §7.6.4.2 Table 22 withholds from each of the two operations this program has.
//!
//! **The withholding is asked of [`pdf_model::restriction::withheld`] rather than restated
//! here**, because Table 22's positions mean different things at different revisions and a census
//! that reimplemented that rule would be measuring its own second reading of the table.
//!
//! ```sh
//! cargo run --release -p pdf-model --example encryption_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! A password is supplied for a named document with `--password NAME=SECRET`, repeated, which is
//! how the "opens with its own password" column is filled: the corpus's locked files have
//! passwords recorded in `crates/pdf-syntax/tests/encryption.rs` and nowhere else.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_model::restriction::{Operation, withheld};
use pdf_syntax::{Document, Limits, Object, SyntaxError};

/// How one document was disposed of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// Opened on the empty string — §7.6.4.1's default user password.
    OpensByDefault,
    /// Opened only on a password supplied on the command line.
    OpensWithItsOwn,
    /// Encrypted, well formed, and no password offered here matched.
    Locked,
    /// Refused by name: a handler or method §7.6 does not specify or this reader declines.
    Unsupported,
    /// Would not open for a reason that is not about encryption at all.
    Malformed,
}

impl Outcome {
    /// The word this census prints for it.
    fn as_str(self) -> &'static str {
        match self {
            Self::OpensByDefault => "default",
            Self::OpensWithItsOwn => "password",
            Self::Locked => "locked",
            Self::Unsupported => "refused",
            Self::Malformed => "malformed",
        }
    }
}

/// One document's row.
struct Row {
    /// The file's name, without its directory.
    name: String,
    /// How it was disposed of.
    outcome: Outcome,
    /// Table 20's `/Filter`, as written.
    filter: String,
    /// Table 20's `/V`, or zero where the entry is absent or unreadable.
    version: i64,
    /// Table 21's `/R`, or zero where the entry is absent or unreadable.
    revision: i64,
    /// Whether the password that opened it was the owner's, where it opened.
    owner: bool,
    /// Whether Table 22 withholds adding an annotation, where it opened.
    withholds_annotate: bool,
    /// Whether Table 22 withholds filling in a form field, where it opened.
    withholds_fill: bool,
    /// The reason, where it did not open.
    reason: String,
}

impl Row {
    /// Whether a password of either role matched.
    fn opens(&self) -> bool {
        matches!(
            self.outcome,
            Outcome::OpensByDefault | Outcome::OpensWithItsOwn
        )
    }
}

/// Table 20's `/Filter` and `/V` and Table 21's `/R`, as the *bytes* say them.
///
/// §7.6.2 exempts the encryption dictionary's own strings from encryption, so this reads without
/// a key; it asks only for names and integers in any case.
fn encryption_dictionary(document: &Document) -> Option<(String, i64, i64)> {
    let Object::Dictionary(dictionary) = document.get_key(document.trailer(), "Encrypt") else {
        return None;
    };
    let name = |key: &str| match document.get_key(&dictionary, key) {
        Object::Name(name) => String::from_utf8_lossy(name.as_bytes()).into_owned(),
        _ => String::new(),
    };
    let integer = |key: &str| match document.get_key(&dictionary, key) {
        Object::Integer(value) => value,
        _ => 0,
    };
    Some((name("Filter"), integer("V"), integer("R")))
}

/// Reads one document, trying the empty password and then any password given for it.
///
/// `None` for a document that states no `/Encrypt` at all, which is the whole corpus bar this
/// census's population.
fn read(path: &Path, passwords: &BTreeMap<String, String>) -> Option<Row> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    let bytes = std::fs::read(path).ok()?;
    let (outcome, reason, document) =
        match Document::open_with_password(bytes.clone(), Limits::DEFAULT, "") {
            Ok(document) if document.is_encrypted() => {
                (Outcome::OpensByDefault, String::new(), Some(document))
            }
            Ok(_) => return None,
            Err(SyntaxError::PasswordRequired) => {
                let secret = passwords.get(&name).cloned().unwrap_or_default();
                match Document::open_with_password(bytes, Limits::DEFAULT, &secret) {
                    Ok(document) if !secret.is_empty() => {
                        (Outcome::OpensWithItsOwn, String::new(), Some(document))
                    }
                    _ => (Outcome::Locked, "no password matched".to_owned(), None),
                }
            }
            Err(error @ SyntaxError::UnsupportedEncryption { .. }) => {
                (Outcome::Unsupported, error.to_string(), None)
            }
            Err(error) => {
                // A file that will not parse cannot be asked what it holds, so the crudest
                // possible probe decides whether it belongs in the population at all. It
                // over-reports rather than under, which is the direction a census of a *stated*
                // entry wants: a document naming `/Encrypt` in a comment is a row to read.
                if !bytes.windows(b"/Encrypt".len()).any(|w| w == b"/Encrypt") {
                    return None;
                }
                (Outcome::Malformed, error.to_string(), None)
            }
        };

    let (filter, version, revision) = document
        .as_ref()
        .and_then(encryption_dictionary)
        .unwrap_or_else(|| (String::new(), 0, 0));
    let permissions = document.as_ref().and_then(Document::permissions);
    let withholds = |operation: Operation| {
        permissions.is_some_and(|granted| withheld(granted, operation).is_some())
    };
    Some(Row {
        name,
        outcome,
        filter,
        version,
        revision,
        owner: permissions.is_some_and(|granted| granted.owner),
        withholds_annotate: withholds(Operation::Annotate),
        withholds_fill: withholds(Operation::FillInForm),
        reason,
    })
}

fn main() {
    let mut passwords = BTreeMap::new();
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--password" {
            if let Some(pair) = arguments.next()
                && let Some((name, secret)) = pair.split_once('=')
            {
                passwords.insert(name.to_owned(), secret.to_owned());
            }
        } else {
            paths.push(PathBuf::from(argument));
        }
    }

    let mut rows: Vec<Row> = paths
        .iter()
        .filter_map(|path| read(path, &passwords))
        .collect();
    rows.sort_by(|left, right| left.name.cmp(&right.name));

    println!(
        "{:<40}{:<10}{:<10}{:>3}{:>3}  notes",
        "file", "opens", "/Filter", "V", "R"
    );
    for row in &rows {
        let withheld = match (row.withholds_annotate, row.withholds_fill) {
            (true, true) => "withholds annotating and filling",
            (true, false) => "withholds annotating",
            (false, true) => "withholds filling",
            (false, false) => "",
        };
        println!(
            "{:<40}{:<10}{:<10}{:>3}{:>3}  {}{}{}",
            row.name,
            row.outcome.as_str(),
            row.filter,
            row.version,
            row.revision,
            if row.owner { "as the owner " } else { "" },
            withheld,
            row.reason
        );
    }

    let count = |outcome: Outcome| rows.iter().filter(|row| row.outcome == outcome).count();
    let opens = rows.iter().filter(|row| row.opens()).count();
    println!(
        "\n{} document(s) state an /Encrypt: {} open on the default user password, {} on one \
         supplied here, {} locked, {} refused as an encryption this reader does not implement, \
         {} malformed for another reason.",
        rows.len(),
        count(Outcome::OpensByDefault),
        count(Outcome::OpensWithItsOwn),
        count(Outcome::Locked),
        count(Outcome::Unsupported),
        count(Outcome::Malformed),
    );
    println!(
        "Of the {opens} that open: {} as the owner, {} withhold annotating, {} withhold filling \
         in a field, {} withhold at least one of the two.",
        rows.iter().filter(|row| row.owner).count(),
        rows.iter().filter(|row| row.withholds_annotate).count(),
        rows.iter().filter(|row| row.withholds_fill).count(),
        rows.iter()
            .filter(|row| row.withholds_annotate || row.withholds_fill)
            .count(),
    );
    let mut revisions: BTreeMap<i64, usize> = BTreeMap::new();
    for row in &rows {
        let tally: &mut usize = revisions.entry(row.revision).or_default();
        *tally = tally.saturating_add(1);
    }
    println!(
        "By Table 21 /R, over every row above: {revisions:?} — 0 is a document whose /R this \
         census could not read, which is every one that did not open."
    );
}

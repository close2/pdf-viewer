//! What each repair the reader makes breaches, and the rows that read the reader's log (ADR 3477,
//! ADR 3506).
//!
//! # Why the validator reads the reader's repairs
//!
//! This crate judges a document through `pdf-syntax`, which reads a malformed file as far as it
//! can — a table rebuilt by scanning, a stream read to its `endstream`, a 34-digit integer read as
//! a real. A requirement that looks only at what was read judges the repair rather than the
//! bytes, and certifies a file the reader had to mend. So every repair is recorded on the
//! document's one channel ([`pdf_syntax::Document::repairs`]), and [`MAPPING`] says, for every
//! class the reader crates declare, which requirements of this crate's table it breaches.
//!
//! **Every class breaches section 5.1** unless it is mapped to [`Breaches::NoRow`] by name: a
//! file the reader had to repair does not adhere to ISO 32000-2, which is section 5.1's whole
//! requirement. Where a repair also touches a rule this part states itself — section 6.1.13's
//! integer bound, section 6.1.9's `endobj` — that row reads the log too. A class mapped to no row
//! is a repair of something that is not the file's departure: a bound of this reader's, which a
//! conforming file can reach.
//!
//! The closing gate of ADR 3477 (`tools/conformance/tests/repair_classes.rs`, to come) holds this
//! table to the reader crates' `CLASSES` slices: no class unmapped, no mapping naming no class,
//! no row that [`crate::table::requirements`] does not hold.

use std::collections::BTreeSet;

use pdf_syntax::{Object, Repair, Site};

use crate::examination::Examination;
use crate::finding::{Findings, Where};

/// Section 5.1's row, which every class but a [`Breaches::NoRow`] one breaches.
pub const BASE_STANDARD: &str = "conformance/adheres-to-the-base-standard";

/// What a class of repair breaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breaches {
    /// Section 5.1's row and these rows besides, which may be none.
    Rows(&'static [&'static str]),
    /// No requirement, for the reason given: the repair is not the file's departure.
    NoRow(&'static str),
}

/// One class of repair, by its stable name, and what it breaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapped {
    /// [`pdf_syntax::RepairClass::name`].
    pub class: &'static str,
    /// What it breaches.
    pub breaches: Breaches,
}

/// Section 5.1 alone.
const ONLY_THE_BASE_STANDARD: Breaches = Breaches::Rows(&[]);

/// The reason every bound of this reader's is mapped to no row.
const A_BOUND_OF_THIS_READER: &str = "a bound this reader keeps, which a conforming file can \
                                      reach; reaching it says nothing about the file";

/// Every class of repair the reader crates declare, and what it breaches (ADR 3506).
pub static MAPPING: &[Mapped] = &[
    // The lexer.
    base("syntax/delimiter-unmatched"),
    base("syntax/name-escape-incomplete"),
    base("syntax/string-unterminated"),
    base("syntax/hexadecimal-string-unterminated"),
    // Section 6.1.6's row re-lexes every hexadecimal string from the bytes and reports a stray
    // byte on its own, so the log adds only section 5.1.
    base("syntax/hexadecimal-string-stray-byte"),
    rows(
        "syntax/integer-past-i64",
        &["implementation-limits/integer-values"],
    ),
    base("syntax/number-with-an-exponent"),
    base("syntax/number-salvaged"),
    // The object parser. Section 6.1.7.1's two rows read the stream keywords and the stated
    // length from the bytes themselves and find these three on their own, so the log adds only
    // section 5.1 for them.
    base("syntax/reference-out-of-range"),
    base("syntax/duplicate-key"),
    base("syntax/key-not-a-name"),
    base("syntax/stream-keyword-line-ending"),
    base("syntax/stream-extent-from-endstream"),
    base("syntax/endstream-missing"),
    rows(
        "syntax/endobj-missing",
        &["file-structure/indirect-object-syntax"],
    ),
    base("syntax/generation-out-of-range"),
    // Objects.
    base("syntax/object-unparsable"),
    base("syntax/reference-cycle"),
    no_row("syntax/reference-chain-too-deep", A_BOUND_OF_THIS_READER),
    base("syntax/xref-offset-searched"),
    base("syntax/compressed-object-absent"),
    no_row("syntax/scan-refused", A_BOUND_OF_THIS_READER),
    // The cross-reference information. Section 6.1.4 states only the line ending after `xref`,
    // which its own row reads from the bytes; the rest of the table's correctness is ISO 32000-2's.
    base("syntax/xref-rebuilt"),
    base("syntax/xref-chain-cut"),
    base("syntax/xref-entries-lost"),
    base("syntax/xref-subsection-realigned"),
    base("syntax/trailer-found-by-scan"),
    base("syntax/trailer-unreadable"),
    base("syntax/catalog-found-by-scan"),
    base("syntax/xref-stream-fields"),
    base("syntax/xref-stream-record"),
    base("syntax/startxref-searched"),
    // Object streams.
    base("syntax/object-stream-header"),
    base("syntax/object-stream-member-lost"),
    base("syntax/object-stream-members-enumerated"),
    // Stream data.
    base("syntax/stream-data-damaged"),
    base("syntax/stream-data-undecodable"),
    base("syntax/decode-parms-mismatched"),
    // Encryption, which section 6.1.3 forbids outright.
    rows(
        "syntax/encryption-id-missing",
        &["file-structure/no-encryption"],
    ),
    rows(
        "syntax/decryption-failed",
        &["file-structure/no-encryption"],
    ),
    rows("syntax/encryption-entry", &["file-structure/no-encryption"]),
];

/// A class that breaches section 5.1 alone.
const fn base(class: &'static str) -> Mapped {
    Mapped {
        class,
        breaches: ONLY_THE_BASE_STANDARD,
    }
}

/// A class that breaches section 5.1 and `more`.
const fn rows(class: &'static str, more: &'static [&'static str]) -> Mapped {
    Mapped {
        class,
        breaches: Breaches::Rows(more),
    }
}

/// A class that breaches nothing, for `why`.
const fn no_row(class: &'static str, why: &'static str) -> Mapped {
    Mapped {
        class,
        breaches: Breaches::NoRow(why),
    }
}

/// What the class named `class` breaches; `None` for a class the table does not name, which the
/// closing gate forbids and which is read as section 5.1 alone meanwhile — a repair this crate
/// has not classified is still a repair.
#[must_use]
pub fn breaches(class: &str) -> Option<Breaches> {
    MAPPING
        .iter()
        .find(|mapped| mapped.class == class)
        .map(|mapped| mapped.breaches)
}

/// Whether a repair of the class named `class` breaches the requirement `row`.
#[must_use]
pub fn breaches_row(class: &str, row: &str) -> bool {
    match breaches(class) {
        Some(Breaches::NoRow(_)) => false,
        Some(Breaches::Rows(more)) => row == BASE_STANDARD || more.contains(&row),
        None => row == BASE_STANDARD,
    }
}

/// Every row any class names besides section 5.1, once each.
#[must_use]
pub fn rows_named() -> BTreeSet<&'static str> {
    MAPPING
        .iter()
        .filter_map(|mapped| match mapped.breaches {
            Breaches::Rows(more) => Some(more.iter().copied()),
            Breaches::NoRow(_) => None,
        })
        .flatten()
        .collect()
}

/// Records, against the requirement `row`, every repair in the document's log that breaches it.
///
/// The document is read whole first ([`Examination::read_whole`]), because the reader records a
/// repair when it makes it and makes it when an object is asked for: a log read before the whole
/// file was would be the log of whatever earlier rows happened to look at.
pub(crate) fn record_logged(exam: &Examination<'_>, row: &str, findings: &mut Findings) {
    exam.read_whole();
    for repair in exam.document.repairs() {
        if breaches_row(repair.class.name, row) {
            findings.record(place(&repair), what(&repair));
        }
    }
}

/// ISO 19005-2 section 5.1 and ISO 19005-4 section 5.1: the base standard, over the log.
pub(crate) fn adheres_to_the_base_standard(exam: &Examination<'_>, findings: &mut Findings) {
    record_logged(exam, BASE_STANDARD, findings);
}

/// Where a finding about `repair` is placed.
fn place(repair: &Repair) -> Where {
    match repair.at {
        Site::Object(id) | Site::Within(id, _) => Where::object(id),
        Site::Offset(offset) => Where::file().named(format!("byte {offset}")),
        Site::WholeFile => Where::file(),
    }
}

/// What a finding about `repair` says: the class's name and what was read instead.
fn what(repair: &Repair) -> String {
    format!(
        "{} ({}): {}",
        repair.class.name, repair.class.clause, repair.class.read_instead
    )
}

/// Decodes every stream of the population, the codec of an image excepted, so that a stream whose
/// filters stop short is in the log whether or not a row asked for its data.
pub(crate) fn decode_every_stream(exam: &Examination<'_>) {
    let document = exam.document;
    for (_, object) in exam.objects() {
        let Object::Stream(stream) = object else {
            continue;
        };
        if document.image_codec(stream).is_some() {
            let _ = document.image_stream_reported(stream);
        } else {
            let _ = document.decoded_stream_data_reported(stream);
        }
    }
}

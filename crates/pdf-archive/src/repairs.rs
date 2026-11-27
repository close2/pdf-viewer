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
//! The closing gate of ADR 3477 (`tools/conformance/tests/repair_classes.rs`) holds this
//! table to the reader crates' `CLASSES` slices: no class unmapped, no mapping naming no class,
//! no row that [`crate::table::requirements`] does not hold.

use std::collections::BTreeSet;

use pdf_syntax::{Object, ObjectId, Repair, Site};

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
    base("syntax/name-holds-null"),
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
    base("syntax/object-number-spelling"),
    base("syntax/duplicate-key"),
    base("syntax/key-not-a-name"),
    base("syntax/stream-keyword-line-ending"),
    base("syntax/stream-extent-from-endstream"),
    base("syntax/stream-not-indirect"),
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
    // A file with no header is section 6.1.2's departure too, which that row reads from the
    // bytes on its own; the rewrite writes a header (ADR 3621).
    rows("syntax/header-missing", &["file-structure/file-header"]),
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
    // A text string read with U+FFFD for bytes its encoding does not give (ADR 3621).
    base("syntax/text-string-undecodable"),
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
    // The page tree (ADR 3541). `/Count`, `/Kids` and the scan replace what the tree states, and
    // section 6.1.4's rows are about the cross-reference table's bytes, so section 5.1 alone.
    base("model/catalog-unreadable"),
    base("model/page-count-unusable"),
    base("model/page-tree-found-by-scanning"),
    base("model/page-dictionary-damaged"),
    base("model/kids-entry-not-a-node"),
    base("model/page-tree-cycle"),
    no_row("model/page-tree-past-a-bound", A_BOUND_OF_THIS_READER),
    base("model/resources-not-a-dictionary"),
    base("model/resources-absent"),
    // Section 6.1.13's boundary row measures the rectangles a page states and passes over one it
    // does not, since §14.11.2.1 defaults it to another the row measures. A box that is no
    // rectangle with an area has no size the limit can be applied to and no default standing
    // behind it — the reader drew on its own box or read the entry as absent — so the row fails
    // where the log says so, beside section 5.1.
    rows(
        "model/media-box-substituted",
        &["implementation-limits/page-boundary-sizes"],
    ),
    rows(
        "model/page-boundary-unusable",
        &["implementation-limits/page-boundary-sizes"],
    ),
    base("model/rotate-not-an-integer"),
    base("model/rotate-not-a-multiple-of-ninety"),
    base("model/user-unit-unusable"),
    // A content stream's parts, its inline objects and its operators' operands (ADR 3541).
    base("model/contents-entry-not-a-stream"),
    no_row("model/content-past-a-bound", A_BOUND_OF_THIS_READER),
    base("model/inline-object-malformed"),
    base("model/inline-image-key-twice"),
    base("model/inline-image-colour-space-unresolved"),
    base("model/inline-image-data-extent"),
    // An image's depth read at 8 and a soft mask not applied: recorded where an image is decoded,
    // so a conversion meets them only where it draws a page (ADR 3691).
    base("model/image-depth-absent"),
    base("model/soft-mask-undecodable"),
    // An `/SMaskInData` that is no integer, read as 0 where the image is decoded: the whole-file
    // rewrite carries the entry as stated, and the output's own check refuses by the class's name
    // (ADR 3757).
    base("model/soft-mask-in-data-unusable"),
    base("model/operand-of-the-wrong-type"),
    base("model/operator-short-of-operands"),
    base("model/restore-with-nothing-saved"),
    base("model/text-operator-outside-a-text-object"),
    // Section 5.1 alone, and section 6.2.4.3's device-space rows by name not: those rows judge
    // the space the stream *selected*, which is what the file says, and the device space the
    // reader painted three operands in under `DeviceGray` is this reader's reading of the
    // producer's intent rather than a space the file uses.
    base("model/colour-operands-do-not-fit-the-space"),
    // The interpreter's own tolerances (ADR 3575). Each is a departure from ISO 32000-2 that no
    // section of this part states a rule for, so section 5.1 alone. The interpreter records them
    // where it makes them, for the viewer; the survey does not run the interpreter (ADR 3542), so
    // a check finds them in the log only where the page was drawn from the same document first.
    base("model/operands-left-over"),
    base("model/compatibility-sections-unbalanced"),
    base("model/marked-content-unbalanced"),
    base("model/property-list-unresolved"),
    base("model/graphics-state-entry-of-the-wrong-type"),
    base("model/transfer-function-unreadable"),
    base("model/form-matrix-malformed"),
    base("model/form-bbox-unusable"),
    base("model/form-resources-not-a-dictionary"),
    base("model/catalog-display-entry-unusable"),
    base("model/viewer-preference-unusable"),
    base("model/action-entry-unusable"),
    // A page's `/Annots` that is no array: a page entry, left as ADR 3543 leaves the others to the
    // rewrites that restate a page and to the output's own check (ADR 3750).
    base("model/page-annots-not-an-array"),
    base("model/annotation-entry-unusable"),
    base("model/field-entry-unusable"),
    // A group's required `/Name` absent, outside content: the whole-file rewrite carries the group
    // as the file states it, and the output's own check refuses by the class's name (ADR 3669).
    base("model/optional-content-group-name-unusable"),
    // A configuration or usage dictionary of the wrong type, outside content: answered as the
    // group's `/Name` is, the rewrite carrying the entry as stated and the output's own check
    // refusing by the class's name (ADR 3749).
    base("model/optional-content-entry-unusable"),
    base("model/file-specification-entry-unusable"),
    base("model/alpha-constant-out-of-range"),
    base("model/halftone-unreadable"),
    // A `/BM` name outside Tables 134 and 135, logged at the content site as the alpha constants
    // are. Sections 6.2.10 and 6.2.9 hold a graphics state's and an annotation's `/BM` to the
    // defined modes in two rows that read the dictionaries on their own, and the log cannot tell
    // which dictionary set the mode, so it adds section 5.1 alone (ADR 3621).
    base("model/blend-mode-unknown"),
    // A resource category of another type, outside content: the whole-file rewrite carries it as
    // the file states it, and the output's own check refuses by the class's name (ADR 3621).
    base("model/resource-category-not-a-dictionary"),
    base("model/halftone-transfer-function-unreadable"),
    // A rich text string's and a coordinate system's string, read where a field's or a free text
    // note's appearance is built and where a measure dictionary is read: outside content, so the
    // whole-file rewrite carries each string as the file states it, and the output's own check
    // refuses by the class's name where it reads the entry again (ADR 3709).
    base("model/rich-text-malformed"),
    base("model/rich-text-attribute-unusable"),
    base("model/xfa-template-attribute-unusable"),
    // A packet's undefined reference: the conversion keeps a producer's packet but for the
    // schemas it rewrites, so a reference elsewhere crosses as written and the output's own check
    // refuses by the class's name (ADR 3752).
    base("model/xmp-reference-undefined"),
    base("model/measurement-entry-unusable"),
    // A localeSet's unnamed locale, a collection's split or folder entry, a structure element's
    // span or writing mode, a page's `/BoxColorInfo`: each outside content, so the whole-file
    // rewrite carries the entry as the file states it, and the output's own check refuses by the
    // class's name (ADR 3757).
    base("model/xfa-locale-name-absent"),
    base("model/collection-entry-unusable"),
    base("model/structure-attribute-unusable"),
    base("model/box-colour-info-unusable"),
    // An outline item's `/F` or `/C`, a page label range's `/St`, a luminosity soft mask's `/BC`:
    // each a dictionary's entry outside content, so the whole-file rewrite carries it as the
    // file states it, and the output's own check refuses by the class's name (ADR 3765).
    base("model/outline-entry-unusable"),
    base("model/page-label-entry-unusable"),
    base("model/soft-mask-backdrop-unusable"),
    // A requirement's `/Penalty` and a media criterion's `/L`: dictionaries outside content, so
    // the whole-file rewrite carries the entry as the file states it, and the output's own check
    // refuses by the class's name (ADR 3777).
    base("model/requirement-entry-unusable"),
    base("model/media-criterion-unusable"),
    base("model/wkt-name-absent"),
    base("model/wkt-identifier-unusable"),
    base("model/wkt-auxiliary-sphere-type-absent"),
    base("model/wkt-false-origin-absent"),
    // The embedded program (ADR 3565, ADR 3569). Section 5.1, and section 6.2.11.4.1's row for a
    // font a content stream renders: the program the file embeds is not the one the window drew,
    // and the converter answers the row by writing the program as this reader read it
    // (`pdf_font::as_read`), so the row's failure no longer reads as a font with no program.
    // Section 6.2.11.2's own row stays unchecked: the log is what the reader repaired, not a
    // validator of sfnt, CFF and Type 1.
    rows("font/sfnt-loca-extent", &[EMBEDDED]),
    rows("font/sfnt-loca-format", &[EMBEDDED]),
    rows("font/sfnt-loca-order", &[EMBEDDED]),
    rows("font/sfnt-table-past-the-program", &[EMBEDDED]),
    rows("font/collection-cut-to-one-face", &[EMBEDDED]),
    rows("font/cff-font-dicts-rewritten", &[EMBEDDED]),
    // A program the loader refused is one `fonts/font-programs-embedded` refuses on its own
    // reading of the same stream, so the log adds only section 5.1 for it.
    base("font/program-unreadable"),
    // `fonts/font-programs-filed-by-their-format` reads a `/FontFile3`'s `/Subtype` against the
    // program's bytes on its own, so the log adds only section 5.1.
    base("font/font-file-subtype-disagrees"),
    // An OpenType Type 1 program's `CFF ` table that did not read, so its codes were mapped through
    // its `cmap`: the program crosses as the file embeds it, and the converter has no charset to
    // restate, so the log adds only section 5.1 (ADR 3777).
    base("font/opentype-cff-unreadable"),
    // The maps. Section 6.2.11.7.2's rows judge a `/ToUnicode` as read and exempt whole classes
    // of font from it, which a repair's place cannot tell, so a map this reader read past is
    // section 5.1's. Those the converter restates as read (ADR 3570) are also the row of section
    // 5.1 that names them, `fonts/font-maps-read-as-written`; a damaged `/ToUnicode` is among
    // them, written again from the mappings that read (ADR 3781). A damaged encoding `CMap`
    // selects glyphs, and is section 5.1's alone (ADR 3782).
    rows("font/to-unicode-not-a-stream", &[MAPS_READ]),
    rows("font/to-unicode-damaged", &[MAPS_READ]),
    base("font/cmap-damaged"),
    rows("font/differences-malformed", &[MAPS_READ]),
    rows("font/encoding-name-unknown", &[MAPS_READ]),
    // Table D.1's name (ADR 3753), read as no base encoding as an unknown name is. On a
    // nonsymbolic TrueType font it is the name section 6.2.11.6 asks to be MacRomanEncoding or
    // WinAnsiEncoding, which that row reports on its own and the converter's TrueType rewrite
    // restates; on every other font it is a map restated as read. The class cannot say which
    // font it was made on, so `fonts/font-maps-read-as-written` reads that from the file.
    rows(
        "font/encoding-name-standard",
        &[MAPS_READ, TRUETYPE_ENCODING],
    ),
    // The widths. Section 6.2.11.5's row compares the widths the dictionary states with the
    // program's, and a malformed entry the reader read as the missing width is one the row
    // compared without the file stating it; it reads these at the fonts a content stream renders.
    rows("font/widths-malformed", &[WIDTHS_AGREE]),
    rows("font/cid-widths-malformed", &[WIDTHS_AGREE]),
    // A CMap stream's `/WMode` read as the nearer mode: the file's departure, and no row of this
    // part states a rule for it.
    base("font/wmode-unusable"),
    base("font/descriptor-style-unusable"),
    no_row("font/to-unicode-past-a-bound", A_BOUND_OF_THIS_READER),
];

/// Section 6.2.11.4.1's row: every font a content stream renders embeds its program.
pub(crate) const EMBEDDED: &str = "fonts/font-programs-embedded";

/// Section 5.1's row for the font maps this reader read past: a `/ToUnicode` that is no stream
/// or holds entries that do not read, an `/Encoding` or `/BaseEncoding` name the base standard
/// does not predefine, a malformed `/Differences` (ADR 3570, ADR 3781).
pub(crate) const MAPS_READ: &str = "fonts/font-maps-read-as-written";

/// Section 6.2.11.6's row: a nonsymbolic `TrueType` font names one of the two encodings ISO 19005
/// admits for it.
pub(crate) const TRUETYPE_ENCODING: &str = "fonts/non-symbolic-truetype-uses-a-standard-encoding";

/// Section 6.2.11.5's row: the widths the dictionary states agree with the program's.
pub(crate) const WIDTHS_AGREE: &str = "fonts/widths-agree-with-the-program";

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
        record_repair(&repair, row, findings);
    }
}

/// Records `repair` against the requirement `row` where its class breaches it: one entry of
/// [`record_logged`], for a row that passes over some repairs of a class by what it reads from
/// the file.
pub(crate) fn record_repair(repair: &Repair, row: &str, findings: &mut Findings) {
    if breaches_row(repair.class.name, row) {
        findings.record(place(repair), what(repair));
    }
}

/// [`record_logged`] narrowed to repairs placed at one of `objects`: a row whose population is a
/// subset of the file — the fonts a content stream renders — fails only for a repair made in an
/// object of that subset, where section 5.1 fails for every one (ADR 3565).
pub(crate) fn record_logged_at(
    exam: &Examination<'_>,
    row: &str,
    objects: &BTreeSet<ObjectId>,
    findings: &mut Findings,
) {
    exam.read_whole();
    for repair in exam.document.repairs() {
        let placed = match repair.at {
            Site::Object(id) | Site::Within(id, _) | Site::Content(Some(id)) => Some(id),
            Site::Offset(_) | Site::WholeFile | Site::Content(None) => None,
        };
        if placed.is_some_and(|id| objects.contains(&id)) && breaches_row(repair.class.name, row) {
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
        Site::Object(id) | Site::Within(id, _) | Site::Content(Some(id)) => Where::object(id),
        Site::Offset(offset) => Where::file().named(format!("byte {offset}")),
        Site::WholeFile | Site::Content(None) => Where::file(),
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

#[cfg(test)]
mod tests {
    use super::{Breaches, MAPPING, breaches, rows_named};

    /// Every class `pdf_font` declares is mapped, and every row a font class names is one this
    /// crate's table holds (ADR 3565): the font half of ADR 3477's closing gate.
    #[test]
    fn every_font_class_is_mapped_to_rows_the_table_holds() {
        for class in pdf_font::repair::CLASSES {
            assert!(breaches(class.name).is_some(), "{} is unmapped", class.name);
        }
        let held: std::collections::BTreeSet<_> =
            crate::table::requirements().map(|row| row.id).collect();
        for row in rows_named() {
            assert!(held.contains(row), "{row} is no row of the table");
        }
        for mapped in MAPPING {
            if mapped.class.starts_with("font/") {
                assert!(
                    pdf_font::repair::CLASSES
                        .iter()
                        .any(|class| class.name == mapped.class),
                    "{} is no class pdf_font declares",
                    mapped.class
                );
                if let Breaches::NoRow(why) = mapped.breaches {
                    assert!(
                        why.len() > 1,
                        "{} is mapped to no row without a reason",
                        mapped.class
                    );
                }
            }
        }
    }
}

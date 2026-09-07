//! Clause 6.1: the file's own shape — its trailer, its filters, its object numbering.
//!
//! The cheapest tranche and the one with the most shared rules: both parts forbid encryption,
//! forbid `LZWDecode` and forbid a stream whose data lives outside the file, in almost the same
//! words. Where they differ they differ sharply — part 2 sets implementation limits and part 4
//! states none, part 4 forbids the document information dictionary that part 2 merely ignores —
//! and each of those is a row of its own.
//!
//! # The three populations this tranche reads, and why they are different sizes
//!
//! Most of clause 6.1 is about *objects*, and those rules read [`Examination::objects`]: the
//! population both parts' own exemption describes, an indirect object no cross-reference section
//! names being exempt from everything (ISO 19005-2 §6.1.4, ISO 19005-4 §6.1.4).
//!
//! Two rules are about *bytes* rather than objects — the indirect object's own `N G obj … endobj`
//! shape, and the header — and those read the file through [`pdf_syntax::FileBytes`] at the
//! offsets the cross-reference table gives. That costs a small read per object, which is written
//! down where it is spent rather than hidden.
//!
//! One rule is about what a *content stream* does: an inline image's filter, which
//! [`crate::survey`] already collects while it walks the pages.
//!
//! # ISO 19005-2 §6.1.13 is many requirements, and it is many rows
//!
//! The clause is a list of independent limits — on integers, on real numbers, on the length of a
//! string and of a name, on how many indirect objects a file may have, on how deep `q` and `Q`
//! may nest, on a `DeviceN`'s colourants, on a CID, on the size of a page boundary — and this
//! tree can answer most of them from the objects and the page tree while three need readers it
//! does not have. One row per limit is what makes that difference legible: a single row would
//! either claim the checks it does not make or throw away the ones it does.
//! `doc/questions/Q20`'s discipline applied at the granularity the clause itself uses.
//!
//! The two rows that fold two of the clause's sentences together do so because the sentences are
//! two halves of one bound — an integer's ceiling and its floor, a real number's largest
//! magnitude and its smallest — and a document that broke one of a pair would be told the same
//! thing either way.
//!
//! **Part 4 states no implementation limits at all**, which is why every one of those rows is a
//! [`Clauses::only_two`] — see `doc/pdf-a-conversion-limits.md`, and the PDF Association's own
//! issue 626 recording that §6.1.13 was dropped rather than renumbered.

use pdf_syntax::{Dictionary, Document, Lexer, Location, Name, Object, ObjectId, Stream, Token};

use crate::Examination;
use crate::target::Part;

use crate::finding::{Findings, Where};
use crate::requirement::{Applies, Check, Clauses, Requirement};

/// The rows this module contributes, which `super::REQUIREMENTS` concatenates.
pub(super) static REQUIREMENTS: &[Requirement] = &[
    Requirement {
        id: "file-structure/no-encryption",
        asks: "The trailer shall not contain an Encrypt key, which forbids encryption and \
               password-protected access permissions.",
        clauses: Clauses::both("6.1.3", "6.1.3"),
        applies: Applies::Always,
        check: Check::Implemented(no_encryption),
    },
    Requirement {
        id: "conformance/adheres-to-the-base-standard",
        asks: "A conforming file shall adhere to every requirement of the base standard, as \
               modified by this part of ISO 19005.",
        clauses: Clauses::both("5.1", "5.1"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "this crate checks the requirements ISO 19005 *adds*; whether a document conforms to \
             the whole of its base standard is a different and far larger question, and one this \
             project tracks about its own reading rather than about a document (`doc/PLAN.md` \
             §5a's conformance ledger). Named here rather than left out, because a verdict that \
             passed over the clause every other requirement is a modification of would be \
             claiming the larger thing while checking the smaller",
        ),
    },
    Requirement {
        id: "file-structure/file-header",
        asks: "The header shall begin at byte zero and state the base standard's version, and \
               shall be followed by a comment line of at least four bytes above 127.",
        clauses: Clauses::both("6.1.2", "6.1.2"),
        applies: Applies::Always,
        check: Check::Implemented(file_header),
    },
    Requirement {
        id: "file-structure/nothing-after-the-last-end-of-file-marker",
        asks: "No data shall follow the last end-of-file marker.",
        clauses: Clauses::only_four("6.1.3"),
        applies: Applies::Always,
        check: Check::Implemented(nothing_after_the_end),
    },
    Requirement {
        id: "file-structure/file-identifier",
        asks: "The trailer dictionary shall contain an ID key whose value is the pair of file \
               identifiers the base standard defines.",
        // Part 2 states it itself. Part 4 does not — and it does not have to, because its base
        // standard requires it of every PDF 2.0 file and §5.1 makes that binding. ISO 32000-2
        // Table 15:
        //
        // > (Required in PDF 2.0 and later, or if an Encrypt entry is present; optional
        // > otherwise; PDF 1.1) An array of two byte-strings constituting a PDF file identifier
        //
        // Cited at §5.1 for part 4 so that a reader who looks up §6.1.3 and finds nothing is not
        // left thinking this crate invented the rule.
        clauses: Clauses::both("6.1.3", "5.1"),
        applies: Applies::Always,
        check: Check::Implemented(file_identifier),
    },
    Requirement {
        id: "file-structure/document-information-dictionary-needs-piece-info",
        asks: "The trailer shall not state an Info key unless the document catalog states a \
               PieceInfo entry.",
        clauses: Clauses::only_four("6.1.3"),
        applies: Applies::Always,
        check: Check::Implemented(document_information_dictionary_needs_piece_info),
    },
    Requirement {
        id: "file-structure/document-information-dictionary-holds-only-a-modification-date",
        asks: "A document information dictionary that is present shall contain no entry other \
               than ModDate.",
        clauses: Clauses::only_four("6.1.3"),
        applies: Applies::Always,
        check: Check::Implemented(document_information_dictionary_holds_only_a_modification_date),
    },
    Requirement {
        id: "file-structure/cross-reference-keyword-line-endings",
        asks: "The xref keyword and the cross-reference subsection header that follows it shall \
               be separated by a single end-of-line marker.",
        clauses: Clauses::both("6.1.4", "6.1.4"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "the rule is about the bytes of each cross-reference section, and `pdf_syntax` keeps \
             the table it read rather than where it read each section from — so nothing in this \
             tree can say where a section's `xref` keyword stands, least of all for the earlier \
             sections of a `/Prev` chain",
        ),
    },
    Requirement {
        id: "file-structure/no-lzw-filter",
        asks: "No stream shall use the LZWDecode filter.",
        clauses: Clauses::both("6.1.7.2", "6.1.6.2"),
        applies: Applies::Always,
        check: Check::Implemented(no_lzw_filter),
    },
    Requirement {
        id: "file-structure/stream-filters-are-standard",
        asks: "No stream shall use a filter the base standard does not list among its standard \
               filters.",
        clauses: Clauses::both("6.1.7.2", "6.1.6.2"),
        applies: Applies::Always,
        check: Check::Implemented(stream_filters_are_standard),
    },
    Requirement {
        id: "file-structure/crypt-filter-is-identity",
        asks: "A stream shall not use the Crypt filter unless the Name in its decode parameters \
               is Identity.",
        clauses: Clauses::both("6.1.7.2", "6.1.6.2"),
        applies: Applies::Always,
        check: Check::Implemented(crypt_filter_is_identity),
    },
    Requirement {
        id: "file-structure/no-external-stream-data",
        asks: "A stream dictionary shall not contain the F, FFilter or FDecodeParams keys, \
               which would put its data outside the file.",
        clauses: Clauses::both("6.1.7.1", "6.1.6.1"),
        applies: Applies::Always,
        check: Check::Implemented(no_external_stream_data),
    },
    Requirement {
        id: "file-structure/stream-length-matches-the-data",
        asks: "A stream dictionary's Length shall be the number of bytes actually standing \
               between the stream and endstream keywords.",
        clauses: Clauses::both("6.1.7.1", "6.1.6.1"),
        applies: Applies::Always,
        check: Check::Implemented(stream_length_matches_the_data),
    },
    Requirement {
        id: "file-structure/stream-keyword-line-endings",
        asks: "The stream keyword shall be followed by a carriage return and line feed or by a \
               single line feed, and the endstream keyword shall be preceded by an end-of-line \
               marker.",
        // Part 4 dropped both sentences: its §6.1.6.1 keeps only the Length rule and the ban on
        // external data. The row is `only_two` for that reason and not for want of looking.
        clauses: Clauses::only_two("6.1.7.1"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "`pdf_syntax` hands this crate a stream's decoded dictionary and its data, not the \
             bytes that delimited them, and the end-of-line before `endstream` is by §7.3.8 not \
             part of the data — so the fact is gone before a requirement can ask",
        ),
    },
    Requirement {
        id: "file-structure/permissions-dictionary-keys",
        asks: "A permissions dictionary shall contain no keys other than UR3 and DocMDP.",
        clauses: Clauses::both("6.1.12", "6.1.11"),
        applies: Applies::Always,
        check: Check::Implemented(permissions_dictionary_keys),
    },
    Requirement {
        id: "file-structure/document-signature-states-no-digest",
        asks: "Where a permissions dictionary states DocMDP, no signature reference dictionary \
               of that signature shall state DigestLocation, DigestMethod or DigestValue.",
        // The second sentence of part 2's §6.1.12. Part 4's §6.1.11 carries the first sentence
        // and drops this one, so it binds part 2 alone.
        clauses: Clauses::only_two("6.1.12"),
        applies: Applies::Always,
        check: Check::Implemented(document_signature_states_no_digest),
    },
    Requirement {
        id: "file-structure/catalog-version-key",
        asks: "A Version key in the document catalog shall be exactly three characters: a 2, a \
               full stop, and one decimal digit.",
        clauses: Clauses::only_four("6.1.12"),
        applies: Applies::Always,
        check: Check::Implemented(catalog_version_key),
    },
    Requirement {
        id: "file-structure/hexadecimal-string-digits",
        asks: "A hexadecimal string shall always have an even number of digits.",
        clauses: Clauses::both("6.1.6", "6.1.5"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "the lexer completes an odd hexadecimal string the way the base standard requires \
             and does not record that it did, so the fact is gone before this crate sees the \
             object",
        ),
    },
    Requirement {
        id: "file-structure/bound-names-are-valid-utf8",
        asks: "Font names, the colourant names of Separation and DeviceN colour spaces, and \
               structure type names shall be valid UTF-8 once their number-sign escapes are \
               expanded.",
        clauses: Clauses::both("6.1.8", "6.1.7"),
        applies: Applies::Always,
        check: Check::Implemented(bound_names_are_valid_utf8),
    },
    Requirement {
        id: "file-structure/indirect-object-syntax",
        asks: "An indirect object's number and generation, and its generation and the obj \
               keyword, shall each be separated by a single white-space character; its object \
               number and its endobj keyword shall each be preceded by an end-of-line marker, \
               and its obj and endobj keywords each followed by one.",
        clauses: Clauses::both("6.1.9", "6.1.8"),
        applies: Applies::Always,
        check: Check::Implemented(indirect_object_syntax),
    },
    Requirement {
        id: "file-structure/inline-image-filters",
        asks: "An inline image's F entry shall not name LZW, Crypt, or any filter outside the \
               set the base standard allows an inline image.",
        clauses: Clauses::both("6.1.10", "6.1.9"),
        applies: Applies::Always,
        check: Check::Implemented(inline_image_filters),
    },
    Requirement {
        id: "implementation-limits/integer-values",
        asks: "No integer in the file shall be greater than 2147483647 or less than -2147483648.",
        clauses: Clauses::only_two("6.1.13"),
        applies: Applies::Always,
        check: Check::Implemented(integer_values),
    },
    Requirement {
        id: "implementation-limits/real-values",
        asks: "No real number in the file shall lie outside ±3.403×10^38, nor be nearer to zero \
               than ±1.175×10^-38.",
        clauses: Clauses::only_two("6.1.13"),
        applies: Applies::Always,
        check: Check::Implemented(real_values),
    },
    Requirement {
        id: "implementation-limits/string-lengths",
        asks: "No string in the file shall be longer than 32767 bytes.",
        clauses: Clauses::only_two("6.1.13"),
        applies: Applies::Always,
        check: Check::Implemented(string_lengths),
    },
    Requirement {
        id: "implementation-limits/name-lengths",
        asks: "No name in the file shall be longer than 127 bytes once its number-sign escapes \
               are expanded.",
        clauses: Clauses::only_two("6.1.13"),
        applies: Applies::Always,
        check: Check::Implemented(name_lengths),
    },
    Requirement {
        id: "implementation-limits/indirect-object-count",
        asks: "A file shall not contain more than 8388607 indirect objects.",
        clauses: Clauses::only_two("6.1.13"),
        applies: Applies::Always,
        check: Check::Implemented(indirect_object_count),
    },
    Requirement {
        id: "implementation-limits/devicen-colourants",
        asks: "A DeviceN colour space shall not have more than 32 colourants.",
        clauses: Clauses::only_two("6.1.13"),
        applies: Applies::Always,
        check: Check::Implemented(devicen_colourants),
    },
    Requirement {
        id: "implementation-limits/page-boundary-sizes",
        asks: "Every page boundary shall measure at least 3 and at most 14400 units in each \
               direction.",
        clauses: Clauses::only_two("6.1.13"),
        applies: Applies::Always,
        check: Check::Implemented(page_boundary_sizes),
    },
    Requirement {
        id: "implementation-limits/character-identifiers",
        asks: "No CID in the file shall be greater than 65535.",
        clauses: Clauses::only_two("6.1.13"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "a CID is stated by the CMap program that maps codes to it, and reading one means \
             parsing that program's cidrange and cidchar sections; `pdf-font` parses CMaps and \
             this crate does not depend on it, and `pdf-model` exposes no reader for one — the \
             same absence `fonts/embedded-cmap-states-its-own-write-mode` names",
        ),
    },
    Requirement {
        id: "implementation-limits/graphics-state-nesting",
        asks: "A file shall not nest q and Q pairs more than 28 levels deep.",
        clauses: Clauses::only_two("6.1.13"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "the depth is a property of a content stream's operators, and `crate::survey` walks \
             the content but keeps only what the colour, font and transparency rules ask of it — \
             it carries a `q` stack of its own and does not report how deep the stack got, so \
             this is a field the survey could add rather than a reader the tree is missing",
        ),
    },
    Requirement {
        id: "implementation-limits/values-written-in-content-streams",
        asks: "The limits on integers, real numbers, strings and names shall hold for the values \
               written inside content streams as well as for the file's objects.",
        clauses: Clauses::only_two("6.1.13"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "the sibling rows answer for every object a cross-reference section names, which is \
             where all but two of these values live; a number or a string written as an operand \
             inside a content stream is reached only by `crate::survey`, which reports what the \
             content *did* rather than the literals it was written with. Named separately rather \
             than folded into those rows, so that a reader can see exactly which half is answered",
        ),
    },
];

/// ISO 19005-2 §6.1.3, ISO 19005-4 §6.1.3.
fn no_encryption(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    if document.trailer().get("Encrypt").is_some() {
        findings.record(
            Where::file().named("Encrypt"),
            "the trailer states an Encrypt key",
        );
    }
}

/// ISO 19005-2 §6.1.2, ISO 19005-4 §6.1.2.
///
/// Three separate requirements in one clause, and each is checked: the header is at byte zero,
/// it states the version its part admits — `1.0` to `1.7` for part 2, `2.0` to `2.9` for part 4
/// — and a comment of at least four bytes above 127 follows it, which is what marks the file as
/// binary to the tools that copy it.
///
/// **The version digit differs by part**, which is what `Check::PerTarget` exists for: a file
/// whose header says `%PDF-1.7` fails a PDF/A-4 check and passes a PDF/A-2 one, and a predicate
/// that accepted either would under-report both.
fn file_header(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let target = exam.target;
    let head = document.bytes().read(0..1024);
    let Some(marker) = head.get(..5) else {
        findings.record(Where::file(), "the file is too short to carry a header");
        return;
    };
    if marker != b"%PDF-" {
        findings.record(
            Where::file(),
            "the file does not begin with a header at byte zero",
        );
        return;
    }
    // The digit each part admits, which is why this predicate needs the target: ISO 19005-2
    // §6.1.2 admits 1.0 to 1.7 and ISO 19005-4 §6.1.2 admits 2.0 to 2.9, and a header stating
    // the other part's version is a failure rather than a curiosity.
    let (major, top) = match target.part() {
        Part::Two => (b'1', b'7'),
        Part::Four => (b'2', b'9'),
    };
    let version = head.get(5..8).unwrap_or_default();
    let shaped = matches!(version, [stated, b'.', minor]
        if *stated == major && minor.is_ascii_digit() && *minor <= top);
    if !shaped {
        findings.record(
            Where::file().named(String::from_utf8_lossy(version).into_owned()),
            "the header does not state a version this part admits",
        );
    }
    // **Both clauses say "followed by a single EOL marker", and the word doing the work is
    // "single".** The version is three characters and the next byte is the marker: a fourth
    // digit, or a space before the line ends, is a header the clause does not admit even though
    // its first eight bytes are right.
    let Some(&byte) = head.get(8) else {
        findings.record(
            Where::file(),
            "the file ends before the header's end-of-line marker",
        );
        return;
    };
    if !is_end_of_line(byte) {
        findings.record(
            Where::file(),
            "the header is not followed immediately by a single end-of-line marker",
        );
        return;
    }
    // A carriage return and line feed together are one marker, not two.
    let marker = if head.get(8..10) == Some(b"\r\n".as_slice()) {
        2
    } else {
        1
    };
    // The binary comment: the clause has that marker "immediately followed by a % (25h)
    // character followed by at least four bytes" each above 127. Immediately, so the comment is
    // read at the byte the marker ends on rather than looked for further down the file.
    let after = head
        .get(8_usize.saturating_add(marker)..)
        .unwrap_or_default();
    let comment = after
        .split_first()
        .and_then(|(first, rest)| (*first == b'%').then_some(rest));
    let binary = comment
        .is_some_and(|bytes| bytes.len() >= 4 && bytes.iter().take(4).all(|byte| *byte > 127));
    if !binary {
        findings.record(
            Where::file(),
            "no comment of four bytes above 127 follows the header, so a tool copying this file \
             may treat it as text",
        );
    }
}

/// ISO 19005-4 §6.1.3.
///
/// Part 2 says the same thing in a NOTE rather than a requirement, which is why this binds part
/// 4 alone: a NOTE states no obligation, and a row that bound part 2 on one would be inventing a
/// rule.
fn nothing_after_the_end(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let length = document.bytes().len();
    let tail_from = length.saturating_sub(64);
    let tail = document.bytes().read(tail_from..length);
    let Some(at) = tail.windows(5).rposition(|window| window == b"%%EOF") else {
        // A file with no end-of-file marker at all is a different failure, and `pdf-syntax`
        // reports it; this rule is about what follows one.
        return;
    };
    let after = tail.get(at.saturating_add(5)..).unwrap_or_default();
    if !after
        .iter()
        .all(|byte| matches!(byte, b'\r' | b'\n' | b' ' | b'\t' | 0))
    {
        findings.record(
            Where::file(),
            "bytes other than an end-of-line marker follow the last %%EOF",
        );
    }
}

/// ISO 19005-2 §6.1.3, and ISO 19005-4 §5.1 through ISO 32000-2 Table 15.
///
/// The value is ISO 32000-2 §14.4's file identifier, which that clause makes "an array of two
/// byte-strings".
///
/// # Why an empty identifier is a failure and not a curiosity
///
/// §14.4 states two `shall`s, and the second is the one an empty string breaks:
///
/// > The value of this entry shall be an array of two byte strings. The first byte string shall
/// > be a permanent identifier based on the contents of the PDF file at the time it was
/// > originally created and shall not change when the PDF file is updated.
///
/// A zero-length string is a byte string, so the first sentence alone would admit `[<> <>]`. It
/// cannot be "based on the contents of the PDF file", though — every file would produce it — so
/// the second sentence rules it out. The reading is stated here because it is a *derivation*
/// rather than a quotation: the clause never uses the word "empty".
fn file_identifier(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let Some(stated) = document.trailer().get("ID") else {
        findings.record(Where::file().named("ID"), "the trailer states no ID");
        return;
    };
    let Some(parts) = stated.as_array().filter(|parts| parts.len() == 2) else {
        findings.record(
            Where::file().named("ID"),
            "the trailer's ID is not an array of two strings",
        );
        return;
    };
    for part in parts {
        match document.resolve(part).as_string() {
            None => findings.record(
                Where::file().named("ID"),
                "one half of the trailer's ID is not a byte string",
            ),
            Some([]) => findings.record(
                Where::file().named("ID"),
                "one half of the trailer's ID is empty, so it identifies no contents",
            ),
            Some(_) => {}
        }
    }
}

/// ISO 19005-4 §6.1.3.
///
/// **A prohibition with one exception, and the exception is the reason the rule is worth
/// stating.** ISO 32000-2 §14.3.3 deprecates the document information dictionary in favour of
/// XMP metadata, and part 4 makes that deprecation binding — except where a `/PieceInfo` entry
/// in the catalog needs it, which is the one case the clause carves out.
fn document_information_dictionary_needs_piece_info(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    if document.trailer().get("Info").is_none() {
        return;
    }
    let carries_piece_info = document
        .catalog()
        .is_ok_and(|catalog| catalog.get("PieceInfo").is_some());
    if !carries_piece_info {
        findings.record(
            Where::file().named("Info"),
            "the trailer states an Info key and the document catalog states no PieceInfo entry",
        );
    }
}

/// ISO 19005-4 §6.1.3.
///
/// The second half of the same clause, and a separate row because it binds a different document:
/// this one fails where the first passes, a file whose `/PieceInfo` earns it an information
/// dictionary but which then fills that dictionary with more than the clause allows.
fn document_information_dictionary_holds_only_a_modification_date(
    exam: &Examination<'_>,
    findings: &mut Findings,
) {
    let document = exam.document;
    let Object::Dictionary(info) = document.get_key(document.trailer(), "Info") else {
        return;
    };
    for name in keys(&info) {
        if name != "ModDate" {
            findings.record(
                Where::file().named(name),
                "the document information dictionary states an entry other than ModDate",
            );
        }
    }
}

/// Every filter name a stream states, however the entry is written.
///
/// ISO 32000-2 §7.4.1 lets the entry be one name or an array of them, applied in order, so a
/// rule about which filters may appear has to read every element rather than the first.
fn filters(document: &Document, stream: &Stream) -> Vec<String> {
    let named = |object: &Object| {
        object
            .as_name()
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
    };
    match document.get_key(&stream.dict, "Filter") {
        Object::Array(chain) => chain
            .iter()
            .filter_map(|entry| named(&document.resolve(entry)))
            .collect(),
        object => named(&object).into_iter().collect(),
    }
}

/// ISO 32000-2 §7.4's Table 6, the ten filters a stream may name.
///
/// Held as the standard prints it rather than as the set this tree can decode, because the rule
/// is about what a *file* may state: a filter outside the table is forbidden whether or not
/// anybody could have read it.
const STANDARD_FILTERS: &[&str] = &[
    "ASCIIHexDecode",
    "ASCII85Decode",
    "LZWDecode",
    "FlateDecode",
    "RunLengthDecode",
    "CCITTFaxDecode",
    "JBIG2Decode",
    "DCTDecode",
    "JPXDecode",
    "Crypt",
];

/// Walks every stream the cross-reference table reaches.
///
/// Bounded by that table rather than by a traversal of the page tree, and deliberately: both
/// parts exempt an indirect object no cross-reference section names — ISO 19005-2 §6.1.4 and
/// ISO 19005-4 §6.1.4 — so what this iterates is exactly the population the requirements bind.
fn for_each_stream(exam: &Examination<'_>, mut visit: impl FnMut(ObjectId, &Stream)) {
    for (id, object) in exam.objects() {
        if let Object::Stream(stream) = object {
            visit(*id, stream);
        }
    }
}

/// ISO 19005-2 §6.1.7.2, ISO 19005-4 §6.1.6.2.
fn no_lzw_filter(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_stream(exam, |id, stream| {
        for name in filters(document, stream) {
            if name == "LZWDecode" || name == "LZW" {
                findings.record(
                    Where::object(id).named(name),
                    "a stream uses the LZWDecode filter",
                );
            }
        }
    });
}

/// ISO 19005-2 §6.1.7.2, ISO 19005-4 §6.1.6.2.
///
/// A separate row from the `LZWDecode` one because the two sentences forbid different things:
/// `LZWDecode` *is* one of Table 6's filters and is banned by name, while this bans everything
/// the table never listed. A document can fail either without failing the other.
fn stream_filters_are_standard(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_stream(exam, |id, stream| {
        for name in filters(document, stream) {
            if !STANDARD_FILTERS.contains(&name.as_str()) {
                findings.record(
                    Where::object(id).named(name),
                    "a stream names a filter the base standard does not define",
                );
            }
        }
    });
}

/// ISO 19005-2 §6.1.7.2, ISO 19005-4 §6.1.6.2.
///
/// The `Crypt` filter is permitted, and only in the one form that decrypts nothing: both parts
/// require the `Name` of its decode parameters to be `Identity`. A stream that states `Crypt`
/// with no decode parameters at all takes ISO 32000-2 §7.4.10's default, which is `Identity`,
/// so the absence is a pass rather than a failure.
fn crypt_filter_is_identity(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_stream(exam, |id, stream| {
        let names = filters(document, stream);
        let Some(position) = names.iter().position(|name| name == "Crypt") else {
            return;
        };
        // §7.4.1 pairs `/DecodeParms` with `/Filter` element by element where both are arrays,
        // and one dictionary with one name.
        let parameters = match document.get_key(&stream.dict, "DecodeParms") {
            Object::Array(list) => list.get(position).map(|entry| document.resolve(entry)),
            object => Some(object),
        };
        let stated = parameters
            .as_ref()
            .and_then(Object::as_dict)
            .map(|dict| document.get_key(dict, "Name"));
        let identity = match stated {
            // No decode parameters, or none naming a filter: §7.4.10 makes `Identity` the
            // default, so silence here is the conforming case rather than an unknown one.
            None | Some(Object::Null) => true,
            Some(parameter) => parameter
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Identity"),
        };
        if !identity {
            findings.record(
                Where::object(id).named("Crypt"),
                "a stream uses the Crypt filter with decode parameters that name something other \
                 than Identity",
            );
        }
    });
}

/// ISO 19005-2 §6.1.7.1, ISO 19005-4 §6.1.6.1.
fn no_external_stream_data(exam: &Examination<'_>, findings: &mut Findings) {
    for_each_stream(exam, |id, stream| {
        for key in ["F", "FFilter", "FDecodeParams"] {
            if stream.dict.get(key).is_some() {
                findings.record(
                    Where::object(id).named(key),
                    "a stream dictionary points at data outside the file",
                );
            }
        }
    });
}

/// ISO 19005-2 §6.1.7.1, ISO 19005-4 §6.1.6.1.
///
/// Asked of the reader rather than of the bytes: `pdf_syntax` takes the declared `/Length` where
/// it is right and finds the real end of the data where it is wrong, so a disagreement between
/// the entry and the data it handed back *is* the disagreement the clause forbids.
///
/// Skipped where the file is encrypted, because a decrypted stream is not the length its bytes
/// were, and an encrypted file has already failed §6.1.3.
fn stream_length_matches_the_data(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    if document.trailer().get("Encrypt").is_some() {
        return;
    }
    for_each_stream(exam, |id, stream| {
        let Some(stated) = document.get_key(&stream.dict, "Length").as_integer() else {
            return;
        };
        let actual = i64::try_from(stream.data.len()).unwrap_or(i64::MAX);
        if stated != actual {
            findings.record(
                Where::object(id).named("Length"),
                format!("a stream states a Length of {stated} and carries {actual} bytes"),
            );
        }
    });
}

/// ISO 19005-2 §6.1.12, ISO 19005-4 §6.1.11.
fn permissions_dictionary_keys(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let Ok(catalog) = document.catalog() else {
        return;
    };
    let Object::Dictionary(permissions) = document.get_key(&catalog, "Perms") else {
        return;
    };
    for name in keys(&permissions) {
        if name != "UR3" && name != "DocMDP" {
            findings.record(
                Where::file().named(name),
                "a permissions dictionary states a key other than UR3 and DocMDP",
            );
        }
    }
}

/// ISO 19005-4 §6.1.12.
///
/// ISO 32000-2 §7.5.2 lets a catalog's `/Version` override the header's version, and Table 29
/// makes it a name; part 4 pins the shape of that name to the version it admits, which is what
/// the header's own rule does for the header. Part 2 states no such clause — its §6.1.12 is the
/// permissions one — so this binds part 4 alone.
///
/// The value is compared after `#`-escapes are expanded, because `pdf_syntax` stores a name
/// decoded and because that is what the name *is*: `/#32#2E#30` and `/2.0` are the same name
/// written two ways, and the clause is about the characters of the value rather than about how a
/// producer chose to spell them.
fn catalog_version_key(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let Ok(catalog) = document.catalog() else {
        return;
    };
    let stated = document.get_key(&catalog, "Version");
    let Some(name) = stated.as_name() else {
        return;
    };
    let shaped = matches!(name.as_bytes(), [b'2', b'.', digit] if digit.is_ascii_digit());
    if !shaped {
        findings.record(
            Where::file().named(shortened(name.as_bytes())),
            "the document catalog's Version is not a 2, a full stop and one decimal digit",
        );
    }
}

/// ISO 19005-2 §6.1.12.
///
/// **Part 4 could not have kept this sentence, and the reason is worth knowing rather than
/// treating as an omission.** ISO 32000-2's Table 256 has dropped `DigestLocation` and
/// `DigestValue` from the signature reference dictionary altogether — so there is nothing left
/// there to forbid — and it has gone the other way on the third, making `DigestMethod`
/// "(Required)" with the note that it "was also corrected to be required as no default value is
/// defined". A part 4 rule forbidding it would contradict its own base standard. So this binds
/// part 2 alone, where ISO 32000-1's Table 253 is what the clause is written against.
fn document_signature_states_no_digest(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let Ok(catalog) = document.catalog() else {
        return;
    };
    let Object::Dictionary(permissions) = document.get_key(&catalog, "Perms") else {
        return;
    };
    let Object::Dictionary(signature) = document.get_key(&permissions, "DocMDP") else {
        return;
    };
    // §12.8.1's Table 255 makes the signature's `/Reference` an array of signature reference
    // dictionaries, which is where the three forbidden keys would stand.
    let listed = document.get_key(&signature, "Reference");
    let Some(references) = listed.as_array() else {
        return;
    };
    for entry in references {
        let resolved = document.resolve(entry);
        let Some(reference) = resolved.as_dict() else {
            continue;
        };
        for key in ["DigestLocation", "DigestMethod", "DigestValue"] {
            if reference.get(key).is_some() {
                findings.record(
                    Where::file().named(key),
                    "a signature reference dictionary of the DocMDP signature states a digest \
                     entry this part forbids",
                );
            }
        }
    }
}

/// One dictionary's keys, as strings a report can print.
fn keys(dictionary: &Dictionary) -> Vec<String> {
    dictionary
        .iter()
        .map(|(key, _)| String::from_utf8_lossy(key.as_bytes()).into_owned())
        .collect()
}

/// How deep a value is followed when a rule looks inside every object in the file.
///
/// A document controls how deeply its own arrays and dictionaries nest, so the descent is
/// bounded rather than trusted. Reaching the bound stops the descent, which under-reports —
/// this crate's standing direction of error.
const MAX_VALUE_DEPTH: u32 = 64;

/// Visits every value inside one object, saying which dictionary key it was found under.
///
/// The key is what several of §6.1.13's limits need — a name is as much a name for being a
/// dictionary's key as for being its value — and it is `None` for an array's element and for the
/// object handed in.
fn for_each_value(object: &Object, depth: u32, visit: &mut impl FnMut(Option<&Name>, &Object)) {
    if depth == 0 {
        visit(None, object);
    }
    if depth >= MAX_VALUE_DEPTH {
        return;
    }
    let deeper = depth.saturating_add(1);
    match object {
        Object::Array(items) => {
            for item in items {
                visit(None, item);
                for_each_value(item, deeper, visit);
            }
        }
        Object::Dictionary(dict) => {
            for (key, value) in dict.iter() {
                visit(Some(key), value);
                for_each_value(value, deeper, visit);
            }
        }
        Object::Stream(stream) => {
            for (key, value) in stream.dict.iter() {
                visit(Some(key), value);
                for_each_value(value, deeper, visit);
            }
        }
        _ => {}
    }
}

/// Runs `visit` over every value of every object a cross-reference section names.
fn for_each_object_value(
    exam: &Examination<'_>,
    mut visit: impl FnMut(ObjectId, Option<&Name>, &Object),
) {
    for (id, object) in exam.objects() {
        for_each_value(object, 0, &mut |key, value| visit(*id, key, value));
    }
}

/// What a report prints for a value that made a document fail a limit.
///
/// Truncated, because the values these limits catch are long by definition: naming a
/// thirty-two-thousand-byte string in full would put the document's own payload in the report.
fn shortened(bytes: &[u8]) -> String {
    const MOST: usize = 48;
    let head = bytes.get(..MOST).unwrap_or(bytes);
    let text = String::from_utf8_lossy(head).into_owned();
    if bytes.len() > MOST {
        format!("{text}… ({} bytes)", bytes.len())
    } else {
        text
    }
}

/// The largest integer ISO 19005-2 §6.1.13 admits.
const LARGEST_INTEGER: i64 = 2_147_483_647;
/// The smallest integer ISO 19005-2 §6.1.13 admits.
const SMALLEST_INTEGER: i64 = -2_147_483_648;

/// ISO 19005-2 §6.1.13.
///
/// `pdf_syntax` carries an integer as `i64` precisely so that a validator can ask this: a
/// reader that clamped to `i32` would have destroyed the evidence. An integer literal too large
/// for `i64` becomes a real instead, and is caught by [`real_values`] only if it also exceeds
/// that limit — a gap between about 9.2×10^18 and 3.4×10^38 that under-reports rather than
/// misreports.
fn integer_values(exam: &Examination<'_>, findings: &mut Findings) {
    for_each_object_value(exam, |id, _, value| {
        if let Object::Integer(number) = value
            && (*number > LARGEST_INTEGER || *number < SMALLEST_INTEGER)
        {
            findings.record(
                Where::object(id).named(number.to_string()),
                "an integer lies outside the range this part allows",
            );
        }
    });
}

/// The largest magnitude ISO 19005-2 §6.1.13 admits of a real number.
const LARGEST_REAL: f64 = 3.403e38;
/// The smallest non-zero magnitude ISO 19005-2 §6.1.13 admits of a real number.
const SMALLEST_REAL: f64 = 1.175e-38;

/// ISO 19005-2 §6.1.13.
///
/// **Zero is not "closer to zero than ±1.175×10^-38".** The two bounds are IEEE 754 single
/// precision's largest finite value and its smallest normal one, which is what ISO 32000-2's
/// Table C.1 is describing when it says a real number is "often" represented in single or double
/// precision; zero is exactly representable there, and a rule that failed every file stating `0`
/// would be reading the sentence as arithmetic rather than as a limit on representation.
fn real_values(exam: &Examination<'_>, findings: &mut Findings) {
    for_each_object_value(exam, |id, _, value| {
        let Object::Real(number) = value else {
            return;
        };
        let magnitude = number.abs();
        if magnitude > LARGEST_REAL {
            findings.record(
                Where::object(id).named(number.to_string()),
                "a real number is larger in magnitude than this part allows",
            );
        } else if magnitude > 0.0 && magnitude < SMALLEST_REAL {
            findings.record(
                Where::object(id).named(number.to_string()),
                "a non-zero real number is nearer to zero than this part allows",
            );
        }
    });
}

/// The longest string ISO 19005-2 §6.1.13 admits, in bytes.
const LONGEST_STRING: usize = 32_767;

/// ISO 19005-2 §6.1.13.
///
/// **The clause is wider than the limit it derives from, and it is the clause that binds.**
/// ISO 32000-2's Table C.1 restricts the length of "a string object in a content stream" and says
/// in as many words that there were "no effective restrictions on other strings in PDF files";
/// ISO 19005-2 §6.1.13 drops that qualification and states the limit of the file. So an outline
/// title of thirty-three thousand bytes fails, and the corpus reads it the same way.
fn string_lengths(exam: &Examination<'_>, findings: &mut Findings) {
    for_each_object_value(exam, |id, _, value| {
        if let Object::String(bytes) = value
            && bytes.len() > LONGEST_STRING
        {
            findings.record(
                Where::object(id).named(shortened(bytes)),
                "a string is longer than this part allows",
            );
        }
    });
}

/// The longest name ISO 19005-2 §6.1.13 admits, in bytes.
const LONGEST_NAME: usize = 127;

/// ISO 19005-2 §6.1.13.
///
/// **Measured on the decoded name, not on what was written.** ISO 32000-2 Table C.1 puts the
/// limit on "the internal representation of a name object", and `pdf_syntax` stores a name
/// decoded, so `/A#41` is two bytes here and five in the file. A name written with escapes past
/// 127 bytes but decoding under it therefore passes, which is what the standard says and what
/// the corpus expects.
fn name_lengths(exam: &Examination<'_>, findings: &mut Findings) {
    let check = |id: ObjectId, name: &Name, findings: &mut Findings| {
        if name.as_bytes().len() > LONGEST_NAME {
            findings.record(
                Where::object(id).named(shortened(name.as_bytes())),
                "a name is longer than this part allows",
            );
        }
    };
    for_each_object_value(exam, |id, key, value| {
        if let Some(key) = key {
            check(id, key, findings);
        }
        if let Object::Name(name) = value {
            check(id, name, findings);
        }
    });
}

/// The most indirect objects ISO 19005-2 §6.1.13 admits.
const MOST_INDIRECT_OBJECTS: usize = 8_388_607;

/// ISO 19005-2 §6.1.13.
fn indirect_object_count(exam: &Examination<'_>, findings: &mut Findings) {
    let counted = exam.objects().len();
    if counted > MOST_INDIRECT_OBJECTS {
        findings.record(
            Where::file().named(counted.to_string()),
            "the file states more indirect objects than this part allows",
        );
    }
}

/// The most colourants ISO 19005-2 §6.1.13 admits of a `DeviceN` colour space.
const MOST_COLOURANTS: usize = 32;

/// ISO 19005-2 §6.1.13.
///
/// ISO 32000-2 §8.6.6.5 makes a `DeviceN` space the array `[/DeviceN names alternateSpace
/// tintTransform]`, so the count is the length of the second element.
fn devicen_colourants(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for_each_object_value(exam, |id, _, value| {
        let Some(items) = value.as_array() else {
            return;
        };
        if items.first().and_then(Object::as_name).map(Name::as_bytes) != Some(b"DeviceN") {
            return;
        }
        let Some(second) = items.get(1) else {
            return;
        };
        let resolved = document.resolve(second);
        let Some(names) = resolved.as_array() else {
            return;
        };
        if names.len() > MOST_COLOURANTS {
            findings.record(
                Where::object(id).named(names.len().to_string()),
                "a DeviceN colour space names more colourants than this part allows",
            );
        }
    });
}

/// The smallest page boundary ISO 19005-2 §6.1.13 admits, in user-space units.
const SMALLEST_BOUNDARY: f64 = 3.0;
/// The largest page boundary ISO 19005-2 §6.1.13 admits, in user-space units.
const LARGEST_BOUNDARY: f64 = 14_400.0;

/// How far up a page's `/Parent` chain an inheritable boundary is looked for.
const MAX_ANCESTRY: u32 = 64;

/// ISO 19005-2 §6.1.13, and ISO 32000-2 §14.11.2 for which rectangles are page boundaries.
///
/// # The rectangle asked about is the one the page *states*, and neither of the two obvious
/// alternatives
///
/// Not every rectangle written in the file: a page tree whose root states a two-unit media box
/// is conforming if every page overrides it, because §7.7.3.4's inheritance means no page ever
/// has that boundary. So the two inheritable boundaries are resolved up `/Parent`, exactly as a
/// reader resolves them, and the three that §7.7.3.4 does not make inheritable are read from the
/// page object alone.
///
/// But equally *not* the rectangle a reader would finally draw into, which is what
/// `pdf_model::Page::boundary` returns and what this rule first used. §14.11.2.1 has a processor
/// "treat the box as its intersection with the media box" where a crop, trim, bleed or art box
/// extends beyond the medium — so a fourteen-thousand-unit crop box inside a five-hundred-unit
/// media box arrives here already clipped to five hundred, and the value the file states
/// disappears before the limit can be applied to it. That is a processor's compensation for a
/// value outside the limit, not a redefinition of the value; every one of §6.1.13's eleven
/// sentences is about what a *file contains*, and this one is no different.
///
/// A boundary the page does not state is not checked, because §14.11.2.1 defaults it to another
/// boundary that is checked — the crop box to the media box, the other three to the crop box —
/// so there is nothing there that is not already answered for.
fn page_boundary_sizes(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for (index, page) in exam.pages().iter().enumerate() {
        for (name, inheritable) in [
            ("MediaBox", true),
            ("CropBox", true),
            ("BleedBox", false),
            ("TrimBox", false),
            ("ArtBox", false),
        ] {
            let stated = if inheritable {
                inherited_rectangle(document, &page.dict, name)
            } else {
                rectangle(document, &document.get_key(&page.dict, name))
            };
            let Some(sides) = stated else {
                continue;
            };
            for (size, direction) in [(sides[0], "wide"), (sides[1], "high")] {
                if size < SMALLEST_BOUNDARY {
                    findings.record(
                        Where::page(index).named(name),
                        format!("a page boundary is {size} units {direction}, under the minimum"),
                    );
                } else if size > LARGEST_BOUNDARY {
                    findings.record(
                        Where::page(index).named(name),
                        format!("a page boundary is {size} units {direction}, over the maximum"),
                    );
                }
            }
        }
    }
}

/// The rectangle the nearest node of a page's ancestry states for an inheritable boundary.
///
/// ISO 32000-2 §7.7.3.4: an attribute omitted from a page object is inherited from the nearest
/// ancestor that states it, so the search stops at the first node that does. Bounded, and each
/// object visited once, because `/Parent` is a reference the document controls.
fn inherited_rectangle(document: &Document, page: &Dictionary, key: &str) -> Option<[f64; 2]> {
    let mut node = page.clone();
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..MAX_ANCESTRY {
        let stated = document.get_key(&node, key);
        if !matches!(stated, Object::Null) {
            return rectangle(document, &stated);
        }
        let parent = node.get("Parent")?;
        if let Some(id) = parent.as_reference()
            && !seen.insert(id)
        {
            return None;
        }
        node = document.resolve(parent).as_dict()?.clone();
    }
    None
}

/// The width and height of a rectangle object, however its corners are ordered.
///
/// ISO 32000-2 §7.9.5:
///
/// > A rectangle shall be written as an array of four numbers giving the coordinates of a pair
/// > of diagonally opposite corners.
///
/// *Which* pair is only typically the lower-left and the upper-right, so a size is the
/// difference of the extremes rather than of the elements as they were written.
fn rectangle(document: &Document, object: &Object) -> Option<[f64; 2]> {
    let items = object.as_array()?;
    if items.len() < 4 {
        return None;
    }
    let mut corners = [0f64; 4];
    for (slot, item) in corners.iter_mut().zip(items) {
        let number = document.resolve(item).as_number()?;
        if !number.is_finite() {
            return None;
        }
        *slot = number;
    }
    let width = corners[0].max(corners[2]) - corners[0].min(corners[2]);
    let height = corners[1].max(corners[3]) - corners[1].min(corners[3]);
    Some([width, height])
}

/// Which of the three kinds of name ISO 19005-2 §6.1.8 binds a name is.
///
/// The clause binds exactly three and says every *other* name "should" follow suit — a
/// recommendation, not a requirement. A rule that tested every name in the file would be
/// inventing an obligation the standard deliberately declined to state, so the kind is carried
/// rather than assumed.
#[derive(Debug, Clone, Copy)]
enum BoundName {
    /// A font's name.
    Font,
    /// A colourant of a `Separation` or `DeviceN` colour space.
    Colourant,
    /// A structure type, either an element's own or one side of a role map entry.
    StructureType,
}

impl BoundName {
    /// What a report calls this kind of name.
    const fn describe(self) -> &'static str {
        match self {
            Self::Font => "a font name",
            Self::Colourant => "a colourant name",
            Self::StructureType => "a structure type name",
        }
    }
}

/// ISO 19005-2 §6.1.8, ISO 19005-4 §6.1.7.
///
/// The names are already decoded — `pdf_syntax` resolves `#xx` at parse time — so the test is
/// exactly the clause's: are the bytes that remain a valid UTF-8 sequence.
fn bound_names_are_valid_utf8(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for (id, object) in exam.objects() {
        for_each_bound_name(document, object, 0, &mut |kind, name| {
            if name.as_str().is_none() {
                findings.record(
                    Where::object(*id).named(shortened(name.as_bytes())),
                    format!("{} is not a valid UTF-8 sequence", kind.describe()),
                );
            }
        });
    }
}

/// Visits the names of one object that ISO 19005-2 §6.1.8 binds.
///
/// A structure element is recognised by its `/Type`, which ISO 32000-2 Table 355 makes optional;
/// an element that omits it contributes nothing here, and its `/S` is reached anyway wherever the
/// role map names the same type.
fn for_each_bound_name(
    document: &Document,
    object: &Object,
    depth: u32,
    visit: &mut impl FnMut(BoundName, &Name),
) {
    if depth >= MAX_VALUE_DEPTH {
        return;
    }
    let deeper = depth.saturating_add(1);
    match object {
        Object::Array(items) => {
            colourants_of_a_colour_space(document, items, visit);
            for item in items {
                for_each_bound_name(document, item, deeper, visit);
            }
        }
        Object::Dictionary(dict) => bound_names_of_a_dictionary(document, dict, deeper, visit),
        Object::Stream(stream) => {
            bound_names_of_a_dictionary(document, &stream.dict, deeper, visit);
        }
        _ => {}
    }
}

/// The colourant names of a `Separation` or `DeviceN` colour space array, if this is one.
///
/// ISO 32000-2 §8.6.6.4 makes a separation `[/Separation name alternateSpace tintTransform]` and
/// §8.6.6.5 makes a `DeviceN` the same shape with an array of names in that position.
fn colourants_of_a_colour_space(
    document: &Document,
    items: &[Object],
    visit: &mut impl FnMut(BoundName, &Name),
) {
    let Some(second) = items.get(1) else {
        return;
    };
    // Resolved inside the arms rather than before them: every array in the document reaches this
    // function, and `Document::resolve` clones what it is given — resolving first would copy
    // every array in the file to discover it was not a colour space.
    let family = items.first().and_then(Object::as_name).map(Name::as_bytes);
    match family {
        Some(b"Separation") => {
            let resolved = document.resolve(second);
            if let Some(name) = resolved.as_name() {
                visit(BoundName::Colourant, name);
            }
        }
        Some(b"DeviceN") => {
            let resolved = document.resolve(second);
            let Some(names) = resolved.as_array() else {
                return;
            };
            for entry in names {
                let colourant = document.resolve(entry);
                if let Some(name) = colourant.as_name() {
                    visit(BoundName::Colourant, name);
                }
            }
        }
        _ => {}
    }
}

/// The bound names one dictionary states, and then everything below it.
fn bound_names_of_a_dictionary(
    document: &Document,
    dict: &Dictionary,
    deeper: u32,
    visit: &mut impl FnMut(BoundName, &Name),
) {
    for key in ["BaseFont", "FontName"] {
        if let Some(Object::Name(name)) = dict.get(key) {
            visit(BoundName::Font, name);
        }
    }
    let is_element = dict
        .get("Type")
        .and_then(Object::as_name)
        .map(Name::as_bytes)
        == Some(b"StructElem".as_slice());
    if is_element && let Some(Object::Name(name)) = dict.get("S") {
        visit(BoundName::StructureType, name);
    }
    // §14.7.3's role map: both sides of every entry are structure types, and the corpus exercises
    // each of them separately. Resolved rather than read directly, because a structure tree root
    // states it as a reference far more often than not.
    if let Object::Dictionary(map) = document.get_key(dict, "RoleMap") {
        for (key, value) in map.iter() {
            visit(BoundName::StructureType, key);
            if let Object::Name(name) = value {
                visit(BoundName::StructureType, name);
            }
        }
    }
    // §8.6.6.5's colourants dictionary, whose keys are colourant names.
    if let Object::Dictionary(colourants) = document.get_key(dict, "Colorants") {
        for (key, _) in colourants.iter() {
            visit(BoundName::Colourant, key);
        }
    }
    for (_, value) in dict.iter() {
        for_each_bound_name(document, value, deeper, visit);
    }
}

/// How many bytes are read at an indirect object's header.
///
/// `N G obj` is a dozen bytes; the window is wide enough that a cross-reference offset a little
/// short of the header still finds it, and small enough that one read per object stays cheap.
const HEADER_WINDOW: usize = 96;

/// How many bytes are read at the end of an indirect object, to find its `endobj` keyword.
const TAIL_WINDOW: usize = 512;

/// The same, for the object no other object follows in the file.
///
/// Wider because what follows it is the cross-reference table and the trailer rather than another
/// object, and those stand between the last `endobj` and the end of the file.
const LAST_TAIL_WINDOW: usize = 4096;

/// ISO 32000-2 §7.2.3's white-space characters.
const fn is_white_space(byte: u8) -> bool {
    matches!(byte, 0 | 9 | 10 | 12 | 13 | 32)
}

/// One byte of ISO 32000-2 §7.2.3's end-of-line marker, which is a carriage return, a line feed,
/// or the two together.
const fn is_end_of_line(byte: u8) -> bool {
    matches!(byte, b'\r' | b'\n')
}

/// ISO 19005-2 §6.1.9, ISO 19005-4 §6.1.8.
///
/// **The one rule in this tranche that reads the file's bytes rather than its objects**, because
/// that is what it is about: the same document, written with a space where an end-of-line marker
/// belongs, parses to exactly the same objects.
///
/// An object stored inside an object stream has no `N G obj` header of its own and is skipped,
/// which is not an exemption this crate grants but the shape of the construct: ISO 32000-2
/// §7.5.7 puts those objects in a stream's data, where no such keywords exist.
///
/// # What it costs, and what is given up to bound it
///
/// Two reads per object: a window at the header, and a window at the end of the region the
/// object occupies. Finding `endobj` by lexing the whole object would be exact and would cost a
/// pass over the file's every byte, so the tail window is read backwards from where the next
/// object begins instead. An object whose `endobj` stands further than [`TAIL_WINDOW`] before
/// the next object's header is therefore not checked — under-reporting, in the direction this
/// crate errs everywhere else.
fn indirect_object_syntax(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let mut placed: Vec<(usize, ObjectId)> = exam
        .objects()
        .iter()
        .filter_map(|(id, _)| match document.xref().location(id.number) {
            Some(Location::Offset(at)) => Some((at, *id)),
            _ => None,
        })
        .collect();
    placed.sort_unstable();
    let length = document.bytes().len();
    for (index, (start, id)) in placed.iter().enumerate() {
        let next = placed.get(index.saturating_add(1)).map(|(at, _)| *at);
        if object_header(document, *start, *id, findings) {
            end_of_object(document, *start, *id, next, length, findings);
        }
    }
}

/// Checks the `N G obj` at one object's offset, and reports what the clause forbids there.
///
/// False where the offset does not lead to this object's header at all — a cross-reference
/// entry pointing somewhere else is a different fault, reported by `pdf_syntax`, and reading a
/// requirement's verdict off bytes that are not the object's would be worse than saying nothing.
fn object_header(document: &Document, start: usize, id: ObjectId, findings: &mut Findings) -> bool {
    // Two bytes before the offset, which is what "preceded by an EOL marker" asks about.
    let back = start.saturating_sub(2);
    let head = document
        .bytes()
        .read(back..start.saturating_add(HEADER_WINDOW));
    let mut lexer = Lexer::at(&head, start.saturating_sub(back));

    lexer.skip_whitespace();
    let number_at = lexer.position();
    let Some(Token::Integer(stated)) = lexer.next_token() else {
        return false;
    };
    if stated != i64::from(id.number) {
        return false;
    }
    let number_end = lexer.position();

    lexer.skip_whitespace();
    let generation_at = lexer.position();
    let Some(Token::Integer(_)) = lexer.next_token() else {
        return false;
    };
    let generation_end = lexer.position();

    lexer.skip_whitespace();
    let keyword_at = lexer.position();
    let Some(Token::Keyword(b"obj")) = lexer.next_token() else {
        return false;
    };
    let keyword_end = lexer.position();

    let single = |gap: &[u8]| gap.len() == 1 && is_white_space(gap[0]);
    if !head.get(number_end..generation_at).is_some_and(single) {
        findings.record(
            Where::object(id),
            "an object number and generation number are not separated by a single white-space \
             character",
        );
    }
    if !head.get(generation_end..keyword_at).is_some_and(single) {
        findings.record(
            Where::object(id),
            "a generation number and the obj keyword are not separated by a single white-space \
             character",
        );
    }
    if back.saturating_add(number_at) > 0
        && !head
            .get(number_at.saturating_sub(1))
            .is_some_and(|byte| is_end_of_line(*byte))
    {
        findings.record(
            Where::object(id),
            "an object number is not preceded by an end-of-line marker",
        );
    }
    // A window that ran out is not evidence of anything; only a byte that is there and is not an
    // end-of-line marker is.
    if let Some(byte) = head.get(keyword_end)
        && !is_end_of_line(*byte)
    {
        findings.record(
            Where::object(id),
            "an obj keyword is not followed by an end-of-line marker",
        );
    }
    true
}

/// Checks the `endobj` that closes one object, found by reading backwards from the next object.
///
/// The *last* `endobj` in the region is the object's own: a string or a stream's data may spell
/// the word — the corpus is full of outline titles that do — but only after the object's own
/// keyword can nothing else follow.
fn end_of_object(
    document: &Document,
    start: usize,
    id: ObjectId,
    next: Option<usize>,
    length: usize,
    findings: &mut Findings,
) {
    let boundary = next.unwrap_or(length).min(length);
    let window = if next.is_some() {
        TAIL_WINDOW
    } else {
        LAST_TAIL_WINDOW
    };
    let from = boundary.saturating_sub(window).max(start);
    if from >= boundary {
        return;
    }
    // One byte past the boundary, so that a keyword ending exactly there can still be asked what
    // follows it.
    let tail = document
        .bytes()
        .read(from..boundary.saturating_add(1).min(length));
    let owned = boundary.saturating_sub(from);
    let Some(at) = tail
        .windows(6)
        .take(owned)
        .rposition(|word| word == b"endobj")
    else {
        return;
    };
    if at > 0
        && !tail
            .get(at.saturating_sub(1))
            .is_some_and(|byte| is_end_of_line(*byte))
    {
        findings.record(
            Where::object(id),
            "an endobj keyword is not preceded by an end-of-line marker",
        );
    }
    if let Some(byte) = tail.get(at.saturating_add(6))
        && !is_end_of_line(*byte)
    {
        findings.record(
            Where::object(id),
            "an endobj keyword is not followed by an end-of-line marker",
        );
    }
}

/// ISO 32000-2 §8.9.7's Table 92, the filters an inline image may name.
///
/// Shorter than [`STANDARD_FILTERS`] by three, and §8.9.7 says why in its own voice: the three it
/// leaves out — `JBIG2Decode`, `Crypt` and `JPXDecode` — are absent from Table 92 because that
/// clause forbids their use with inline images at all.
const INLINE_IMAGE_FILTERS: &[&str] = &[
    "ASCIIHexDecode",
    "ASCII85Decode",
    "LZWDecode",
    "FlateDecode",
    "RunLengthDecode",
    "CCITTFaxDecode",
    "DCTDecode",
];

/// ISO 19005-2 §6.1.10, ISO 19005-4 §6.1.9.
///
/// **The two parts point at different tables, and the difference is real.** Part 2 forbids "a
/// value not listed in ISO 32000-1:2008, Table 6", which is the ten standard stream filters;
/// part 4 forbids one not listed in ISO 32000-2 §8.9.7's Table 92, which is the seven an inline
/// image may use. `JPXDecode` in an inline image is therefore outside part 4's list and inside
/// part 2's — so the permitted set is chosen by the target rather than shared.
///
/// The names arrive expanded: `crate::survey` reads inline images through
/// `pdf_model::inline_image`, which turns Table 92's abbreviations into their full spellings, so
/// `/F /LZW` is seen here as `LZWDecode`. A name the table never listed is passed through
/// unchanged, which is exactly the case this rule exists to catch.
fn inline_image_filters(exam: &Examination<'_>, findings: &mut Findings) {
    let allowed: &[&str] = match exam.target.part() {
        Part::Two => STANDARD_FILTERS,
        Part::Four => INLINE_IMAGE_FILTERS,
    };
    for (page, dict) in exam.survey().inline_images() {
        let stated = dict.get("Filter");
        let listed: Vec<&Name> = match stated {
            Some(Object::Name(name)) => vec![name],
            Some(Object::Array(items)) => items.iter().filter_map(Object::as_name).collect(),
            _ => Vec::new(),
        };
        for name in listed {
            let spelt = String::from_utf8_lossy(name.as_bytes()).into_owned();
            let forbidden = spelt == "LZWDecode" || spelt == "Crypt";
            if forbidden || !allowed.contains(&spelt.as_str()) {
                findings.record(
                    Where::page(*page).named(spelt),
                    "an inline image names a filter this part forbids there",
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pdf_syntax::{Dictionary, Name, Object};

    use super::{
        LARGEST_INTEGER, LONGEST_NAME, LONGEST_STRING, SMALLEST_INTEGER, for_each_value, shortened,
    };

    /// Builds a dictionary from pairs, for the small hand-made objects the tests walk.
    fn dictionary(entries: &[(&str, Object)]) -> Dictionary {
        let mut dict = Dictionary::new();
        for (key, value) in entries {
            dict.insert(Name::new(key.as_bytes().to_vec()), value.clone());
        }
        dict
    }

    #[test]
    fn a_walk_sees_every_key_once_and_every_value_once() {
        let object = Object::Dictionary(dictionary(&[
            (
                "Kids",
                Object::Array(vec![Object::Integer(1), Object::Integer(2)]),
            ),
            ("Name", Object::Name(Name::new(b"Bare".to_vec()))),
        ]));
        let mut keys = Vec::new();
        let mut integers = Vec::new();
        for_each_value(&object, 0, &mut |key, value| {
            if let Some(key) = key {
                keys.push(String::from_utf8_lossy(key.as_bytes()).into_owned());
            }
            if let Object::Integer(number) = value {
                integers.push(*number);
            }
        });
        keys.sort();
        assert_eq!(keys, vec!["Kids".to_owned(), "Name".to_owned()]);
        assert_eq!(integers, vec![1, 2], "an array's elements are values too");
    }

    #[test]
    fn the_limits_are_the_ones_the_clause_states() {
        // Read off ISO 19005-2 §6.1.13 rather than remembered: the boundary values themselves
        // conform, and only what lies beyond them does not.
        assert_eq!(LARGEST_INTEGER, 2_147_483_647);
        assert_eq!(SMALLEST_INTEGER, -2_147_483_648);
        assert_eq!(LONGEST_STRING, 32_767);
        assert_eq!(LONGEST_NAME, 127);
    }

    #[test]
    fn a_report_names_a_long_value_without_carrying_it() {
        let long = vec![b'x'; 40_000];
        let named = shortened(&long);
        assert!(
            named.len() < 100,
            "the report holds a witness, not a payload"
        );
        assert!(named.contains("40000"), "and says how long the value was");
    }
}

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
//! names being exempt from everything (ISO 19005-2 section 6.1.4, ISO 19005-4 section 6.1.4).
//!
//! Two rules are about *bytes* rather than objects — the indirect object's own `N G obj … endobj`
//! shape, and the header — and those read the file through [`pdf_syntax::FileBytes`] at the
//! offsets the cross-reference table gives. That costs a small read per object, which is written
//! down where it is spent rather than hidden.
//!
//! One rule is about what a *content stream* does: an inline image's filter, which
//! [`crate::survey`] already collects while it walks the pages.
//!
//! # ISO 19005-2 section 6.1.13 is many requirements, and it is many rows
//!
//! The clause is a list of independent limits — on integers, on real numbers, on the length of a
//! string and of a name, on how many indirect objects a file may have, on how deep `q` and `Q`
//! may nest, on a `DeviceN`'s colourants, on a CID, on the size of a page boundary — and this
//! tree answers all but one of them, from the objects, the page tree and [`crate::survey`]'s
//! walk of the content. One row per limit is what makes that difference legible: a single row
//! would either claim the checks it does not make or throw away the ones it does.
//! `doc/questions/Q20`'s discipline applied at the granularity the clause itself uses.
//!
//! The one left is the CID, and it is not left for want of a field: a CID is stated by a `CMap`
//! program, and reading one is a parser this crate does not reach.
//!
//! The two rows that fold two of the clause's sentences together do so because the sentences are
//! two halves of one bound — an integer's ceiling and its floor, a real number's largest
//! magnitude and its smallest — and a document that broke one of a pair would be told the same
//! thing either way.
//!
//! **Part 4 states no implementation limits at all**, which is why every one of those rows is a
//! [`Clauses::only_two`] — see `doc/pdf-a-conversion-limits.md`, and the PDF Association's own
//! issue 626 recording that section 6.1.13 was dropped rather than renumbered.

use pdf_syntax::{Dictionary, Document, Lexer, Location, Name, Object, ObjectId, Stream, Token};

use crate::Examination;
use crate::table::states_name;
use crate::target::{Flavour, Part};

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
        id: "conformance/no-deprecated-features",
        asks: "A conforming file shall not use a feature the base standard describes as \
               deprecated.",
        clauses: Clauses::only_four("5.1"),
        applies: Applies::Always,
        check: Check::Unchecked(
            "unimplemented, and the route this reason used to name is both further away and \
             wronger than it said. `pdf_spec`'s Arlington-derived model carries `deprecated_in` \
             on 419 of its 3983 key rows, 372 of them at 2.0 — but the field is per *key of a \
             named object type*, and asking it about a document's dictionary needs a resolver \
             from that dictionary to its Arlington object, which no crate in this tree has: \
             `pdf_spec::object` takes Arlington's own name, and the one consumer that walks the \
             model (`pdf-model`'s `integer_entry_census`) iterates the whole table rather than \
             resolving a document's objects against it. So the blocker is a type inference, not \
             a field. And the population would be wrong even with one, on the part's own \
             evidence: ISO 32000-2 marks `UR3` deprecated in PDF 2.0, while ISO 19005-4 \
             section 6.1.11 names `UR3` as one of the only two keys a permissions dictionary may \
             hold — a mechanical per-key reading of section 5.1 would make that sentence forbid \
             two keys and permit none. The clause's subject is a *feature*, and ISO 32000-2 \
             deprecates constructs no single key stands for: encryption revisions 2 to 4, the \
             `adbe.x509.rsa_sha1` and `adbe.pkcs7.sha1` subfilters, SHA-1 as a digest, an array \
             of blend mode names, two character collections. Part 2 states no equivalent \
             sentence",
        ),
    },
    Requirement {
        id: "conformance/the-version-number-does-not-decide-conformance",
        asks: "The version number in the header shall not be used in deciding whether a file \
               conforms.",
        clauses: Clauses::only_two("5.1"),
        applies: Applies::Always,
        check: Check::Processor(
            "addressed to whoever judges the file rather than to the file, and this crate is \
             one of them: no row here reads the header's version digit except \
             `file-structure/file-header`, which asks the shape section 6.1.2 states and not \
             whether the digit suits the target. Part 4 states no equivalent sentence, because \
             its own section 6.1.2 admits every PDF 2.0 revision",
        ),
    },
    Requirement {
        id: "conformance/processor-behaviour",
        asks: "A conforming processor shall meet every requirement this part states about \
               processor behaviour, and shall render and otherwise process a conforming file as \
               the base standard defines.",
        clauses: Clauses::both("5.5", "5.2"),
        applies: Applies::Always,
        check: Check::Processor(
            "the subclause that makes every other processor row of this table binding on a \
             program, and it is the one place ISO 19005 says what a conforming reader *is*. \
             Nothing in a document bears on it; what this project's own reading of it amounts \
             to is `doc/PLAN.md` section 5a's ledger. This row carried a third clause — that a \
             processor ignores features the base standard does not describe — until the \
             sentence-level reading in `crate::coverage` found that part 4 states that one as a \
             recommendation and part 2 as a requirement; it is now \
             `conformance/undescribed-features-are-ignored`, which binds part 2 alone",
        ),
    },
    Requirement {
        id: "conformance/undescribed-features-are-ignored",
        asks: "A conforming reader shall ignore features described in PDF specifications that \
               the base standard does not describe.",
        clauses: Clauses::only_two("5.5"),
        applies: Applies::Always,
        check: Check::Processor(
            "an obligation on the program. Part 2's section 5.5 states it with `shall` and part \
             4's section 5.2 states the same thing with `should`, so it is a requirement of one \
             part and a recommendation of the other — which is why this row cites part 2 alone \
             rather than both. Distinct from \
             `file-structure/undescribed-data-never-renders`, whose subject is *data carried in \
             the file* rather than a feature some other specification describes",
        ),
    },
    Requirement {
        id: "conformance/processor-reads-every-conforming-file",
        asks: "A conforming processor shall read and appropriately process every file that \
               conforms to this part.",
        clauses: Clauses::both("5.5", "5.2"),
        applies: Applies::Always,
        check: Check::Processor(
            "the sentence that makes a conforming processor's *coverage* an obligation rather \
             than its behaviour on the files it happens to open: a program that declines a \
             conforming file is not a conforming processor, whatever it does with the rest. \
             Nothing in a document bears on it, and what this project can claim under it is \
             `doc/PLAN.md` section 5a's ledger",
        ),
    },
    Requirement {
        id: "conformance/a-part-two-reader-also-reads-part-one",
        asks: "A conforming PDF/A-2 reader shall also read and appropriately process every \
               PDF/A-1 file.",
        clauses: Clauses::only_two("5.5"),
        applies: Applies::Always,
        check: Check::Processor(
            "an obligation on the program, and the one sentence of either part that reaches \
             outside the two this crate holds: a PDF/A-2 conforming reader owes ISO 19005-1 as \
             well. Part 4 states no equivalent. It bears on no document, and it is worth a row \
             because a claim to be a conforming PDF/A-2 reader is a claim about part 1 too — \
             while `doc/questions/A17` settles that part 1 is not a *target* of this crate, \
             which is a different question and does not discharge this one",
        ),
    },
    Requirement {
        id: "conformance/an-embedded-files-processor-reads-the-plain-profile",
        asks: "A PDF/A-4f conforming processor shall read and appropriately process every \
               PDF/A-4f file, and every file a PDF/A-4 conforming processor is required to read.",
        clauses: Clauses::only_four("A.1"),
        applies: Applies::Flavours(&[Flavour::F]),
        check: Check::Processor(
            "an obligation on the program, stated by the annex that defines the flavour rather \
             than by clause 5, which is why the subclause-level audit recorded ISO 19005-4 \
             Annex A.1 as scoping and no row reached its processor sentences",
        ),
    },
    Requirement {
        id: "conformance/an-engineering-processor-reads-the-plain-profile",
        asks: "A PDF/A-4e conforming processor shall read and appropriately process every \
               PDF/A-4e file, and every file a PDF/A-4 conforming processor is required to read.",
        clauses: Clauses::only_four("B.1"),
        applies: Applies::Flavours(&[Flavour::E]),
        check: Check::Processor(
            "the same obligation as `conformance/an-embedded-files-processor-reads-the-plain-\
             profile`, stated separately by the engineering annex. Two rows rather than one \
             because a row cites one clause per part and these are two clauses of the same part",
        ),
    },
    Requirement {
        id: "file-structure/undescribed-data-never-renders",
        asks: "Data in a conforming file that neither the base standard nor ISO 19005 describes \
               shall not be used to render content on a page.",
        clauses: Clauses::both("6.1.1", "6.1.1"),
        applies: Applies::Always,
        check: Check::Processor(
            "the sentence permits a file to carry such data and forbids a *processor* to draw \
             with it, so no property of a document satisfies or breaks it. What a file may not \
             carry is every other row of this table; this one is the standing instruction about \
             what is left over",
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
        // standard requires it of every PDF 2.0 file and section 5.1 makes that binding. ISO
        // 32000-2 Table 15:
        //
        // > (Required in PDF 2.0 and later, or if an Encrypt entry is present; optional
        // > otherwise; PDF 1.1) An array of two byte-strings constituting a PDF file identifier
        //
        // Cited at section 5.1 for part 4 so that a reader who looks up section 6.1.3 and finds
        // nothing is not left thinking this crate invented the rule.
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
        check: Check::Implemented(cross_reference_keyword_line_endings),
    },
    Requirement {
        id: "file-structure/unreferenced-objects-never-influence-rendering",
        asks: "An indirect object no cross-reference section names is exempt from every \
               requirement, and a processor that does not ignore it shall never let it \
               influence what is rendered.",
        clauses: Clauses::both("6.1.4", "6.1.4"),
        applies: Applies::Always,
        check: Check::Processor(
            "two sentences, and neither is a property of a document. The first is an exemption \
             this crate obeys in its own populations rather than reports on — the object walks \
             this module and `super::fonts` run visit only what a cross-reference section names \
             — and the second binds a processor that chooses to read such an object anyway",
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
        // Part 4 dropped both sentences: its section 6.1.6.1 keeps only the Length rule and the ban
        // on external data. The row is `only_two` for that reason and not for want of looking.
        clauses: Clauses::only_two("6.1.7.1"),
        applies: Applies::Always,
        check: Check::Implemented(stream_keyword_line_endings),
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
        // The second sentence of part 2's section 6.1.12. Part 4's section 6.1.11 carries the first
        // sentence and drops this one, so it binds part 2 alone.
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
        check: Check::Implemented(hexadecimal_string_digits),
    },
    Requirement {
        id: "file-structure/hexadecimal-string-holds-only-digits",
        asks: "A hexadecimal string shall hold nothing but hexadecimal digits and white space.",
        // Neither part states this one: it is the base standard's, and section 5.1 is what binds
        // it. ISO 32000-2 §7.3.4.3, and ISO 32000-1:2008 §7.3.4.3 in the same words for part 2:
        //
        // > A hexadecimal string shall be written as a sequence of hexadecimal digits (0 -9 and
        // > A -F or a -f) encoded as ASCII characters and enclosed within angle brackets
        //
        // followed by the one exemption, that white-space characters "shall be ignored". Cited at
        // Section 5.1 in both parts rather than at ISO 19005's own section 6.1.6/section 6.1.5,
        // which state the digit *count* and say nothing about what a digit is — attributing this to
        // those clauses would be this crate inventing a rule and citing somebody else for it.
        clauses: Clauses::both("5.1", "5.1"),
        applies: Applies::Always,
        check: Check::Implemented(hexadecimal_string_holds_only_digits),
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
        id: "file-structure/linearization-permitted",
        asks: "Linearization shall be permitted, and a processor should ignore any \
               linearization information the file carries.",
        clauses: Clauses::both("6.1.11", "6.1.10"),
        applies: Applies::Always,
        check: Check::Processor(
            "the only `shall` here is addressed to whoever judges the file — linearization is \
             permitted, so a validator that reported a linearized file would itself be wrong — \
             and the rest of the subclause is a recommendation to a processor. Carried as a row \
             because a permission the standard states explicitly is a thing this table can be \
             *checked against*, and silence here would read as an unexamined subclause",
        ),
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
        check: Check::Implemented(character_identifiers),
    },
    Requirement {
        id: "implementation-limits/graphics-state-nesting",
        asks: "A file shall not nest q and Q pairs more than 28 levels deep.",
        clauses: Clauses::only_two("6.1.13"),
        applies: Applies::Always,
        check: Check::Implemented(graphics_state_nesting),
    },
    Requirement {
        id: "implementation-limits/values-written-in-content-streams",
        asks: "The limits on integers, real numbers, strings and names shall hold for the values \
               written inside content streams as well as for the file's objects.",
        clauses: Clauses::only_two("6.1.13"),
        applies: Applies::Always,
        check: Check::Implemented(values_written_in_content_streams),
    },
];

/// ISO 19005-2 section 6.1.3, ISO 19005-4 section 6.1.3.
fn no_encryption(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    if document.trailer().get("Encrypt").is_some() {
        findings.record(
            Where::file().named("Encrypt"),
            "the trailer states an Encrypt key",
        );
    }
}

/// ISO 19005-2 section 6.1.2, ISO 19005-4 section 6.1.2.
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
    // The digit each part admits, which is why this predicate needs the target: ISO 19005-2 section
    // 6.1.2 admits 1.0 to 1.7 and ISO 19005-4 section 6.1.2 admits 2.0 to 2.9, and a header stating
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

/// ISO 19005-4 section 6.1.3.
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

/// ISO 19005-2 section 6.1.3, and ISO 19005-4 section 5.1 through ISO 32000-2 Table 15.
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
///
/// # Which trailer, when a file has several
///
/// [`pdf_syntax::Document::trailer`] is the **merge** of every section on the `/Prev` chain,
/// which is right for a reader resolving `/Root` and wrong for this question. ISO 32000-1:2008
/// §7.5.6 and ISO 32000-2 §7.5.6 both require each appended trailer to restate its predecessor's
/// entries itself —
///
/// > The added trailer shall contain all the entries except the Prev entry (if present) from the
/// > previous trailer, whether modified or not.
///
/// — so "the file trailer dictionary shall contain the ID keyword" is a question about the
/// newest section's own dictionary. The merge answers it wrongly in one direction only, and it
/// is the direction that matters: a file whose newest trailer states nothing and whose oldest
/// stated everything passes the merge and has broken the rule.
///
/// A **linearised** file is where the corpus finds one, and it is not an exotic case.
/// ISO 32000-1:2008 §F.3.4 makes the first-page trailer the one `startxref` names and says a
/// reader "interprets the first-page cross-reference table as an update to an original document
/// that is indexed by the main cross-reference table"; §F.3.11 then says the main trailer "shall
/// not contain any entries other than Size". So in a conforming linearised file the identifier
/// is in the *first-page* trailer, and a producer that leaves it only in the main one has
/// written a file whose newest trailer does not state it.
///
/// Where the chain cannot be read at all — a table [`pdf_syntax::xref::rebuild`] recovered by
/// scanning states no sections — the merge is used instead. That is deliberately the weaker
/// test: the alternative is to fail a document over a trailer this crate synthesised.
fn file_identifier(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    let governing = stated_trailer(document);
    let Some(stated) = governing.get("ID") else {
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

/// ISO 19005-4 section 6.1.3.
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

/// ISO 19005-4 section 6.1.3.
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
/// Bounded by that table rather than by a traversal of the page tree, and deliberately: both parts
/// exempt an indirect object no cross-reference section names — ISO 19005-2 section 6.1.4 and ISO
/// 19005-4 section 6.1.4 — so what this iterates is exactly the population the requirements bind.
fn for_each_stream(exam: &Examination<'_>, mut visit: impl FnMut(ObjectId, &Stream)) {
    for (id, object) in exam.objects() {
        if let Object::Stream(stream) = object {
            visit(*id, stream);
        }
    }
}

/// ISO 19005-2 section 6.1.7.2, ISO 19005-4 section 6.1.6.2.
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

/// ISO 19005-2 section 6.1.7.2, ISO 19005-4 section 6.1.6.2.
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

/// ISO 19005-2 section 6.1.7.2, ISO 19005-4 section 6.1.6.2.
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

/// ISO 19005-2 section 6.1.7.1, ISO 19005-4 section 6.1.6.1.
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

/// ISO 19005-2 section 6.1.7.1, ISO 19005-4 section 6.1.6.1.
///
/// Asked of the reader rather than of the bytes: `pdf_syntax` takes the declared `/Length` where
/// it is right and finds the real end of the data where it is wrong, so a disagreement between
/// the entry and the data it handed back *is* the disagreement the clause forbids.
///
/// Skipped where the file is encrypted, because a decrypted stream is not the length its bytes
/// were, and an encrypted file has already failed section 6.1.3.
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

/// How many bytes are read at a stream object's header, looking for its `stream` keyword.
///
/// A stream dictionary is a few dozen bytes in almost every file, so the narrow window is what
/// is paid for nearly all of them; [`WIDE_STREAM_HEADER_WINDOW`] is tried only where the narrow
/// one did not reach the keyword, which keeps the common case one small read.
const STREAM_HEADER_WINDOW: usize = 512;

/// The same, for the stream dictionary the narrow window did not get to the end of.
///
/// A dictionary longer than this — a cross-reference stream with a long `/Index`, say — is left
/// unjudged rather than judged on bytes that are not its keyword's.
const WIDE_STREAM_HEADER_WINDOW: usize = 8192;

/// How many bytes are read at the end of a stream's data, looking for `endstream`.
///
/// §7.3.8 puts an end-of-line marker between the data and the keyword and nothing else, so the
/// keyword stands within two bytes of the data's end in a well-formed file and within a handful
/// in the malformed ones this rule exists to catch.
const ENDSTREAM_WINDOW: usize = 64;

/// ISO 19005-2 section 6.1.7.1.
///
/// **A bytes rule, like [`indirect_object_syntax`], and for the same reason**: the same document
/// written with a space where §7.3.8 requires an end-of-line marker parses to exactly the same
/// objects, so nothing in the object model can answer it. What makes it answerable is that
/// `pdf_syntax` keeps the file's bytes and the cross-reference table says where each object
/// begins — the two keywords are then found by lexing the dictionary through to `stream`, and by
/// stepping over the data `pdf_syntax` took to reach `endstream`.
///
/// Part 4 dropped both sentences, which is why the row that names this is [`Clauses::only_two`].
///
/// # Why an encrypted file is skipped
///
/// Its streams are decrypted before this crate sees them and are not the length their bytes
/// were, so stepping over the data would land in the wrong place. Such a file has already failed
/// Section 6.1.3, and the same reasoning skips [`stream_length_matches_the_data`].
///
/// # Where it stays silent
///
/// An object the cross-reference table places somewhere its header is not; a dictionary longer
/// than [`WIDE_STREAM_HEADER_WINDOW`]; a stream whose `endstream` is not within
/// [`ENDSTREAM_WINDOW`] of where the data ended. Each of those is a fact this rule could not read
/// rather than one it read as passing — the direction of error this crate keeps everywhere.
fn stream_keyword_line_endings(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    if document.trailer().get("Encrypt").is_some() {
        return;
    }
    for (id, object) in exam.objects() {
        let Object::Stream(stream) = object else {
            continue;
        };
        let Some(Location::Offset(start)) = document.xref().location(id.number) else {
            continue;
        };
        let Some(opens) = stream_keyword_end(document, start, *id) else {
            continue;
        };
        let follows = document.bytes().read(opens..opens.saturating_add(2));
        let opened = match (follows.first(), follows.get(1)) {
            (Some(b'\n'), _) | (Some(b'\r'), Some(b'\n')) => true,
            // No bytes at all is the file ending at the keyword, which says nothing this rule
            // can report: there is no data and no `endstream` either.
            (None, _) => continue,
            _ => false,
        };
        if !opened {
            findings.record(
                Where::object(*id),
                "a stream keyword is followed by neither a carriage return and line feed nor a \
                 single line feed",
            );
        }
        let ends = opens.saturating_add(stream.data.len());
        // One byte earlier, so that the byte before a keyword standing exactly at the data's end
        // is in the window; the search then starts one in, which is why no match can be the
        // data's own last bytes rather than the keyword.
        let tail = document
            .bytes()
            .read(ends.saturating_sub(1)..ends.saturating_add(ENDSTREAM_WINDOW));
        let Some(after) = tail.get(1..) else {
            continue;
        };
        let Some(at) = after.windows(9).position(|word| word == b"endstream") else {
            continue;
        };
        if !tail.get(at).copied().is_some_and(is_end_of_line) {
            findings.record(
                Where::object(*id),
                "an endstream keyword is not preceded by an end-of-line marker",
            );
        }
    }
}

/// Where one stream object's `stream` keyword ends, found by lexing from the object's header.
///
/// `None` where the offset does not lead to this object's header, or where the dictionary did
/// not finish inside the widest window — the same discipline as [`object_header`], which returns
/// false rather than report on bytes that are not the object's.
fn stream_keyword_end(document: &Document, start: usize, id: ObjectId) -> Option<usize> {
    for window in [STREAM_HEADER_WINDOW, WIDE_STREAM_HEADER_WINDOW] {
        let head = document.bytes().read(start..start.saturating_add(window));
        let mut lexer = Lexer::new(&head);
        let Some(Token::Integer(stated)) = lexer.next_token() else {
            return None;
        };
        if stated != i64::from(id.number) {
            return None;
        }
        let Some(Token::Integer(_)) = lexer.next_token() else {
            return None;
        };
        let Some(Token::Keyword(b"obj")) = lexer.next_token() else {
            return None;
        };
        // Every token of the dictionary is stepped over rather than skipped by scanning for the
        // word, because a literal string may spell `stream` and a name may be `/stream`.
        // `true`, `false` and `null` arrive here as keywords too, so only the one keyword ends
        // the search.
        while let Some(token) = lexer.next_token() {
            if token == Token::Keyword(b"stream") {
                return Some(start.saturating_add(lexer.position()));
            }
        }
    }
    None
}

/// ISO 19005-2 section 6.1.12, ISO 19005-4 section 6.1.11.
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

/// ISO 19005-4 section 6.1.12.
///
/// ISO 32000-2 §7.5.2 lets a catalog's `/Version` override the header's version, and Table 29 makes
/// it a name; part 4 pins the shape of that name to the version it admits, which is what the
/// header's own rule does for the header. Part 2 states no such clause — its section 6.1.12 is the
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

/// ISO 19005-2 section 6.1.12.
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
/// The key is what several of section 6.1.13's limits need — a name is as much a name for being a
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

/// The largest integer ISO 19005-2 section 6.1.13 admits.
const LARGEST_INTEGER: i64 = 2_147_483_647;
/// The smallest integer ISO 19005-2 section 6.1.13 admits.
const SMALLEST_INTEGER: i64 = -2_147_483_648;

/// ISO 19005-2 section 6.1.13.
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

/// The largest magnitude ISO 19005-2 section 6.1.13 admits of a real number.
const LARGEST_REAL: f64 = 3.403e38;
/// The smallest non-zero magnitude ISO 19005-2 section 6.1.13 admits of a real number.
const SMALLEST_REAL: f64 = 1.175e-38;

/// ISO 19005-2 section 6.1.13.
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

/// The longest string ISO 19005-2 section 6.1.13 admits, in bytes.
const LONGEST_STRING: usize = 32_767;

/// ISO 19005-2 section 6.1.13.
///
/// **The clause is wider than the limit it derives from, and it is the clause that binds.**
/// ISO 32000-2's Table C.1 restricts the length of "a string object in a content stream" and says
/// in as many words that there were "no effective restrictions on other strings in PDF files"; ISO
/// 19005-2 section 6.1.13 drops that qualification and states the limit of the file. So an outline
/// title of thirty-three thousand bytes fails, and the corpus reads it the same way.
///
/// **Measured on the decoded bytes**, which `pdf_syntax`'s lexer produces: a literal string's
/// backslash and octal escapes are resolved and a hexadecimal string's digit pairs are folded, so
/// a string written across sixty-six thousand bytes of hexadecimal is thirty-three thousand here.
/// `TechNote 0010` A005 resolves that parts 1 to 3 are read as if the length of a name or a string
/// were the length of its internal byte representation after exactly those decodings — so the
/// question the clause leaves open, whether the limit counts what the file wrote or what it means,
/// is answered, and answered the way this row already read it.
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

/// The longest name ISO 19005-2 section 6.1.13 admits, in bytes.
const LONGEST_NAME: usize = 127;

/// ISO 19005-2 section 6.1.13.
///
/// **Measured on the decoded name, not on what was written.** ISO 32000-2 Table C.1 puts the
/// limit on "the internal representation of a name object", and `pdf_syntax` stores a name
/// decoded, so `/A#41` is two bytes here and five in the file. A name written with escapes past
/// 127 bytes but decoding under it therefore passes, which is what the standard says and what
/// the corpus expects.
///
/// `TechNote 0010` A005 is the working group resolving that same reading for parts 1 to 3, of
/// names and strings together, and its background notes that the PDF 1.4 Reference's own example
/// of it counted the leading solidus by mistake — corrected in ISO 32000-1 and stated as Table
/// C.1 states it here. The record is cited beside this row's verdict because a validator counting
/// the bytes the file wrote would disagree with it on every escaped name.
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

/// The most indirect objects ISO 19005-2 section 6.1.13 admits.
const MOST_INDIRECT_OBJECTS: usize = 8_388_607;

/// ISO 19005-2 section 6.1.13.
fn indirect_object_count(exam: &Examination<'_>, findings: &mut Findings) {
    let counted = exam.objects().len();
    if counted > MOST_INDIRECT_OBJECTS {
        findings.record(
            Where::file().named(counted.to_string()),
            "the file states more indirect objects than this part allows",
        );
    }
}

/// The deepest nesting of `q` and `Q` pairs ISO 19005-2 section 6.1.13 admits.
const DEEPEST_NESTING: usize = 28;

/// ISO 19005-2 section 6.1.13.
///
/// The depth is a property of the content stream's operators rather than of any object, so it
/// comes from [`crate::survey`], which carries the `q` stack the walk needs anyway.
/// [`crate::survey::Survey::deepest_graphics_state_nesting`] says what is and is not summed
/// across a form `XObject`'s invocation, and why.
///
/// **Per content stream, and that is the committee's reading rather than this crate's caution.**
/// `TechNote 0010` A004 resolves that parts 1 to 3 are read as if the limit assumed each content
/// stream considered in isolation, ignoring the cumulative effect of nesting form `XObject`s — so
/// a file whose page opens twenty `q`s and invokes a form that opens twenty more breaks nothing.
/// A validator that summed the invocation would fail it, which is why the record is cited beside
/// this row's verdict.
fn graphics_state_nesting(exam: &Examination<'_>, findings: &mut Findings) {
    let Some(deepest) = exam.survey().deepest_graphics_state_nesting() else {
        return;
    };
    if deepest.value > DEEPEST_NESTING {
        findings.record(
            Where::page(deepest.page).named(deepest.value.to_string()),
            "a content stream nests q and Q pairs deeper than this part allows",
        );
    }
}

/// ISO 19005-2 section 6.1.13, for the values written inside a content stream.
///
/// # Why this is a row of its own rather than four lines in the four sibling rows
///
/// The sibling rows read every object a cross-reference section names, which is where all but two
/// of a file's values live; a number, string or name written as an *operand* is inside a stream's
/// data and no object walk reaches it. Both halves are the same sentence of section 6.1.13, so a
/// reader has to be able to see which half a verdict covers — and a single row would say the clause
/// was checked while half of it was not.
///
/// # One finding per kind, from the extreme
///
/// [`crate::survey::ContentLiterals`] keeps how far the operands reached in each direction rather
/// than every operand that broke a bound, and the reason is written there. What it costs a
/// report is that a document stating a thousand out-of-range integers is told about the largest
/// one, with the page it is on.
fn values_written_in_content_streams(exam: &Examination<'_>, findings: &mut Findings) {
    let literals = exam.survey().content_literals();
    let mut report = |page: usize, value: String, what: &'static str| {
        findings.record(Where::page(page).named(value), what);
    };
    if let Some(held) = literals.largest_integer
        && held.value > LARGEST_INTEGER
    {
        report(
            held.page,
            held.value.to_string(),
            "a content stream states an integer larger than this part allows",
        );
    }
    if let Some(held) = literals.smallest_integer
        && held.value < SMALLEST_INTEGER
    {
        report(
            held.page,
            held.value.to_string(),
            "a content stream states an integer smaller than this part allows",
        );
    }
    if let Some(held) = literals.largest_real_magnitude
        && held.value > LARGEST_REAL
    {
        report(
            held.page,
            held.value.to_string(),
            "a content stream states a real number larger in magnitude than this part allows",
        );
    }
    // The survey keeps only non-zero magnitudes here, which is what makes this the bound the
    // clause states rather than a rule against writing `0`; see [`real_values`].
    if let Some(held) = literals.smallest_real_magnitude
        && held.value < SMALLEST_REAL
    {
        report(
            held.page,
            held.value.to_string(),
            "a content stream states a non-zero real number nearer to zero than this part allows",
        );
    }
    if let Some(held) = literals.longest_string
        && held.value > LONGEST_STRING
    {
        report(
            held.page,
            format!("{} bytes", held.value),
            "a content stream states a string longer than this part allows",
        );
    }
    if let Some(held) = literals.longest_name
        && held.value > LONGEST_NAME
    {
        report(
            held.page,
            format!("{} bytes", held.value),
            "a content stream states a name longer than this part allows",
        );
    }
}

/// The most colourants ISO 19005-2 section 6.1.13 admits of a `DeviceN` colour space.
const MOST_COLOURANTS: usize = 32;

/// ISO 19005-2 section 6.1.13.
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

/// The smallest page boundary ISO 19005-2 section 6.1.13 admits, in user-space units.
const SMALLEST_BOUNDARY: f64 = 3.0;
/// The largest page boundary ISO 19005-2 section 6.1.13 admits, in user-space units.
const LARGEST_BOUNDARY: f64 = 14_400.0;

/// How far up a page's `/Parent` chain an inheritable boundary is looked for.
const MAX_ANCESTRY: u32 = 64;

/// ISO 19005-2 section 6.1.13, and ISO 32000-2 §14.11.2 for which rectangles are page boundaries.
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
/// value outside the limit, not a redefinition of the value; every one of section 6.1.13's eleven
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

/// The greatest CID ISO 19005-2 section 6.1.13 admits.
///
/// Two bytes' worth, and the limit is stated about a *CID value* rather than about a glyph
/// count — a file with three glyphs may still select one of them by a CID of 70 000.
const GREATEST_CID: u32 = 65_535;

/// ISO 19005-2 section 6.1.13: no CID in the file shall be greater than 65535.
///
/// # Where a CID is written, and which of those places this reads
///
/// A CID is not stored anywhere as itself. It is *selected*, and ISO 32000-2 §9.7.5.3 gives the
/// selection: an embedded `CMap`'s `cidrange` maps a run of codes onto consecutive CIDs counting
/// up from the one it names, `cidchar` maps one code to one CID, and `notdefrange` gives every
/// code in a run the single CID it names. So the greatest CID a font can select is a fact about
/// the `CMap` program, and reading it means parsing that program.
///
/// **`pdf-font` already parses it**, and `Cargo.toml` explains why this crate depends on
/// `pdf-font` directly. The row's reason used to say the opposite — that this crate does not
/// depend on it — which was true when the row was written and stopped being true when the font
/// tranche landed. A stale reason on an unchecked row is the failure `A20` names: the reason is
/// the report.
///
/// # What this does not reach, and why that is the rule rather than a gap
///
/// A `/W` array also spells CIDs, as the first element of each run, and a `/CIDToGIDMap` stream
/// is indexed by one. Neither is read here, and this used to be recorded as a narrow gap: a file
/// could write an out-of-range CID into `/W` and never select it, and the row would not see it.
///
/// **`TechNote 0010` A007 says that is not a gap.** The working group was asked whether the CID
/// limit applies to the CIDs a content stream selects or to the `CMap` data itself, and resolved
/// that it is applied to a `CMap`'s *syntax*, so that the `CMap` stream can be parsed. A `/W`
/// array is not `CMap` syntax, and neither is a `/CIDToGIDMap` stream; the limit's subject is the
/// program this predicate reads. The second half of the same question — whether a font used only
/// in rendering mode 3 is exempt — falls away with it, because no rule here turns on what the
/// content selects. Where a font names one of §9.7.5.2's predefined `CMap`s rather than embedding
/// one, there is no syntax in the file to hold to the limit, which is why the name form is
/// skipped.
fn character_identifiers(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for (id, object) in exam.objects() {
        let Some(dict) = object.as_dict() else {
            continue;
        };
        if !states_name(document, dict, "Subtype", b"Type0") {
            continue;
        }
        // §9.7.5.2's predefined CMaps are named rather than embedded, and a name states no CIDs
        // of its own. Only the stream form carries a program to read.
        let Object::Stream(stream) = document.get_key(dict, "Encoding") else {
            continue;
        };
        let Some(bytes) = document.decoded_stream_data(&stream) else {
            continue;
        };
        let Some(greatest) = pdf_font::cmap::CMap::parse(&bytes, None).greatest_selector() else {
            continue;
        };
        if greatest > GREATEST_CID {
            findings.record(
                Where::object(*id).named(greatest.to_string()),
                "an embedded CMap selects a CID greater than this part allows",
            );
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

/// Which of the three kinds of name ISO 19005-2 section 6.1.8 binds a name is.
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

/// ISO 19005-2 section 6.1.8, ISO 19005-4 section 6.1.7.
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

/// Visits the names of one object that ISO 19005-2 section 6.1.8 binds.
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

/// ISO 32000-2 §7.2.3's white-space characters — ISO 32000-1:2008, 7.2.2 at a part 2 target,
/// where 7.2.3 is *Comments* and the character set is one subclause earlier.
const fn is_white_space(byte: u8) -> bool {
    matches!(byte, 0 | 9 | 10 | 12 | 13 | 32)
}

/// One byte of ISO 32000-2 §7.2.3's end-of-line marker — ISO 32000-1:2008, 7.2.2 at a part 2
/// target — which is a carriage return, a line feed, or the two together.
const fn is_end_of_line(byte: u8) -> bool {
    matches!(byte, b'\r' | b'\n')
}

/// How many bytes of an indirect object's own syntax are read when looking for its strings.
///
/// An object's bytes run to wherever the next one begins, and that is usually a few hundred; the
/// cap is what stops a stream object whose data is a megabyte from being read whole to reach the
/// dictionary in its first line, since the reading stops at the `stream` keyword anyway. An
/// object stating more than this much syntax — an array of tens of thousands of numbers — is read
/// as far as the cap and no further, which is this crate's standing direction of error:
/// under-report rather than mis-report.
const OBJECT_SYNTAX_WINDOW: usize = 64 * 1024;

/// One span of a document's bytes that is PDF syntax, and what its hexadecimal strings looked
/// like as written.
///
/// Visible to [`crate::Examination`] because that is where the walk producing these is shared;
/// see [`hexadecimal_spans`].
#[derive(Debug)]
pub(crate) struct HexadecimalSpan {
    /// Where a finding about it belongs.
    pub(crate) place: Where,
    /// What ISO 32000-2 §7.3.4.3's strings in that span said, from [`Lexer`]'s own count.
    pub(crate) strings: pdf_syntax::HexadecimalStrings,
}

/// Every span of this document that is PDF syntax, with its hexadecimal strings counted.
///
/// # Why this is three walks and not one
///
/// §7.3.4.3's strings are *syntax*, and a PDF states syntax in three separate places: in the file
/// itself, between an object's `obj` and its `endobj`; inside §7.5.7's object streams, whose data
/// is a run of objects with no headers of their own; and inside a content stream, where a
/// text-showing operator's operand is a string like any other. A walk that read only the first
/// would pass a document whose every page draws with a malformed string, which is what the corpus
/// witnesses for both of these rules are.
///
/// # The fourth place, and how it is reached
///
/// A content stream is not only a page's `/Contents`: a form `XObject` invoked through `Do`, a
/// tiling pattern, a Type 3 glyph procedure and an annotation's appearance stream are content
/// streams too, and each is reached by following a resource dictionary. This used to read the
/// pages alone and say so, which left a hexadecimal string written only inside a form
/// unreported. It is [`crate::survey`] that already follows those dictionaries, so what is done
/// here is to read the streams it says it opened — [`crate::survey::Survey::opened_streams`]
/// carries the object of each — rather than to walk the resources a second time.
///
/// **The inline-image skip needs the resources in force**, and the survey's record does not
/// carry them, so this uses the stream's own `/Resources` and falls back to the page's. That is
/// what the survey itself reads a form against in every case but one — a form with no
/// `/Resources` of its own invoked from another form — and ISO 19005 section 6.2.2 requires
/// every one of these streams to carry the entry anyway.
///
/// Streams whose data this reader cannot decode are skipped, with the consequence that follows:
/// a filter chain nothing here can run is a stream this rule says nothing about. So is the
/// content of a soft mask's group and of a shading's function, which the survey does not walk
/// anywhere. That is the crate's standing direction of error and not a claim that the clause
/// exempts them.
///
/// # What it costs, measured, and where the halving came from
///
/// `examples/cost` on ISO 32000-2's own specification — 1 023 pages, 101 318 objects: this walk
/// was **266 ms** in the first of the two rules that ask and **146 ms** in the second, on a
/// report that took 4.1 s, because it ran once for each of them. They were the two dearest
/// predicates in the crate.
///
/// **They are one walk now**: [`Examination::hexadecimal_spans`] holds it in the `OnceCell`
/// beside the survey, the object population and the annotations, which is where a report's
/// shared work belongs. Following the survey's nested streams (above) added about 25 ms to the
/// walk; sharing it took the second rule's whole cost off the report. The measurement is in
/// this crate's `examples/cost`, which is why the numbers here are numbers rather than a belief.
///
/// Reading the file whole instead of a window per object was tried first and refuted: 274 ms
/// became 266 ms, an 8 ms saving for 19 MB retained, so the cost is the lexing and not the
/// reading.
pub(crate) fn hexadecimal_spans(exam: &Examination<'_>) -> Vec<HexadecimalSpan> {
    let document = exam.document;
    let mut out = Vec::new();

    // The file's own syntax, object by object. Bounded by where the next object begins, because
    // §7.3.10's `endobj` is what ends one and a producer that omitted it would otherwise have
    // this reader lex the object after it twice.
    let mut placed: Vec<(usize, ObjectId)> = exam
        .objects()
        .iter()
        .filter_map(|(id, _)| match document.xref().location(id.number) {
            Some(Location::Offset(at)) => Some((at, *id)),
            _ => None,
        })
        .collect();
    placed.sort_unstable();
    for (index, (start, id)) in placed.iter().enumerate() {
        let next = placed
            .get(index.saturating_add(1))
            .map_or(document.bytes().len(), |(at, _)| *at);
        let end = next.min(start.saturating_add(OBJECT_SYNTAX_WINDOW));
        let window = document.bytes().read(*start..end.max(*start));
        let mut lexer = Lexer::at(&window, 0);
        // Stopping at `stream` is what keeps a stream's *data* out of this: those bytes are an
        // image or a compressed run, not syntax, and lexing them would invent hexadecimal
        // strings out of binary that happens to hold an angle bracket. §7.3.8.1 puts the data
        // immediately after the keyword, so the keyword is the boundary.
        while let Some(token) = lexer.next_token() {
            if matches!(token, Token::Keyword(b"stream" | b"endobj")) {
                break;
            }
        }
        record_span(&mut out, Where::object(*id), &lexer);
    }

    // §7.5.7's object streams: objects with no `N G obj` header, in a stream's decoded data.
    for (id, object) in exam.objects() {
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let is_object_stream = document
            .get_key(&stream.dict, "Type")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"ObjStm");
        if !is_object_stream {
            continue;
        }
        let Some(data) = document.decoded_stream_data(stream) else {
            continue;
        };
        let mut lexer = Lexer::new(&data);
        while lexer.next_token().is_some() {}
        record_span(&mut out, Where::object(*id), &lexer);
    }

    // §7.8.2's content streams, as far as a page's own `/Contents`.
    let mut seen = std::collections::BTreeSet::new();
    for (index, page) in exam.pages().iter().enumerate() {
        let listed = page.dict.get("Contents").cloned().unwrap_or(Object::Null);
        // §7.7.3.3's Table 31 makes `/Contents` "either a single stream or an array of streams",
        // and an array's elements are references. The reference is what identifies a stream two
        // pages share, so it is kept rather than resolved away.
        let items = match document.resolve(&listed) {
            Object::Array(items) => items,
            _ => vec![listed],
        };
        for item in &items {
            if let Some(id) = item.as_reference()
                && !seen.insert(id)
            {
                continue;
            }
            let resolved = document.resolve(item);
            let Some(stream) = resolved.as_stream() else {
                continue;
            };
            let Some(data) = document.decoded_stream_data(stream) else {
                continue;
            };
            let resources = document
                .get_key(&page.dict, "Resources")
                .as_dict()
                .cloned()
                .unwrap_or_default();
            record_content_span(document, &data, &resources, Where::page(index), &mut out);
        }
    }

    // The content streams a resource dictionary reaches, as the survey found them.
    for opened in exam.survey().opened_streams() {
        let Some(id) = opened.object else {
            continue;
        };
        if !seen.insert(id) {
            continue;
        }
        let object = document.get(id);
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let Some(data) = document.decoded_stream_data(stream) else {
            continue;
        };
        let resources = document
            .get_key(&stream.dict, "Resources")
            .as_dict()
            .cloned()
            .or_else(|| {
                exam.pages()
                    .get(opened.page)
                    .and_then(|page| document.get_key(&page.dict, "Resources").as_dict().cloned())
            })
            .unwrap_or_default();
        record_content_span(document, &data, &resources, opened.place.clone(), &mut out);
    }

    out
}

/// Lexes one decoded content stream, stepping over §8.9.7's inline image data, and keeps what
/// its hexadecimal strings looked like.
///
/// **The inline image data is not a program and must never be lexed as one.** Compressed samples
/// hold angle brackets like they hold any other byte, and reading them as syntax invents
/// hexadecimal strings out of an image: five conforming corpus documents were failed by these
/// rules before the skip was here, which is precisely the mis-report the crate's direction of
/// error forbids. `pdf_model::inline_image::scan` is the reader that already knows where the
/// data ends, and its `resume` is the only thing wanted here — which is why it is given the
/// resources in force, since §8.9.7 lets `/CS` name a colour space the resources define and the
/// component count is what says how long the data is.
fn record_content_span(
    document: &Document,
    data: &[u8],
    resources: &Dictionary,
    place: Where,
    out: &mut Vec<HexadecimalSpan>,
) {
    let mut lexer = Lexer::new(data);
    while let Some(token) = lexer.next_token() {
        if matches!(token, Token::Keyword(b"BI")) {
            let scan =
                pdf_model::inline_image::scan(document, data, lexer.position(), resources, true);
            lexer.seek(scan.resume);
        }
    }
    record_span(out, place, &lexer);
}

/// Keeps a span, but only where its lexer read a hexadecimal string worth reporting.
fn record_span(out: &mut Vec<HexadecimalSpan>, place: Where, lexer: &Lexer<'_>) {
    let strings = lexer.hexadecimal_strings();
    if strings.completed > 0 || strings.strayed > 0 {
        out.push(HexadecimalSpan { place, strings });
    }
}

/// ISO 19005-2 section 6.1.6, ISO 19005-4 section 6.1.5.
///
/// Both parts state the same rule in the same words, and both attach the same NOTE saying what it
/// is for: it removes the base standard's provision for a missing final digit. That provision is
/// ISO 32000-2 §7.3.4.3 —
///
/// > If the final digit of a hexadecimal string is missing -that is, if there is an odd number of
/// > digits -the final digit shall be assumed to be 0.
///
/// — and ISO 32000-1:2008 §7.3.4.3 states it in the same terms for part 2. **A conforming reader
/// therefore cannot see this fault in the value**: the string it builds from `<901FA>` is byte
/// for byte the string it builds from `<901FA0>`, which is why the count comes from
/// [`pdf_syntax::HexadecimalStrings`] — the lexer's own record of what the bytes said — rather
/// than from the object.
fn hexadecimal_string_digits(exam: &Examination<'_>, findings: &mut Findings) {
    for span in exam.hexadecimal_spans() {
        if span.strings.completed == 0 {
            continue;
        }
        findings.record(
            span.place.clone(),
            format!(
                "{} hexadecimal {} an odd number of digits, which the base standard completes \
                 with a zero",
                span.strings.completed,
                if span.strings.completed == 1 {
                    "string states"
                } else {
                    "strings state"
                },
            ),
        );
    }
}

/// ISO 19005-2 section 5.1 and ISO 19005-4 section 5.1, through the base standard's §7.3.4.3.
///
/// The sibling of [`hexadecimal_string_digits`], and a separate row because it is a separate
/// clause: ISO 19005 says how many digits a hexadecimal string has and the base standard says
/// what a digit is. §7.3.4.3 allows exactly two things between the angle brackets — the digits
/// themselves, and white space, which "shall be ignored" — so a byte that is neither has been
/// written into a string that shall not hold one.
///
/// A conforming reader cannot see this in the value either: §7.3.4.3 gives no meaning to such a
/// byte, so this reader passes over it and the string comes out as though it were never there.
fn hexadecimal_string_holds_only_digits(exam: &Examination<'_>, findings: &mut Findings) {
    for span in exam.hexadecimal_spans() {
        if span.strings.strayed == 0 {
            continue;
        }
        findings.record(
            span.place.clone(),
            format!(
                "{} hexadecimal {} a byte that is neither a hexadecimal digit nor white space",
                span.strings.strayed,
                if span.strings.strayed == 1 {
                    "string holds"
                } else {
                    "strings hold"
                },
            ),
        );
    }
}

/// How many bytes are read at a cross-reference section's `xref` keyword.
///
/// Enough for the keyword, whatever stands between it and the subsection header, and the header
/// itself; a separator longer than this is already not the single end-of-line marker the clause
/// asks for, so the window is a bound on the reading rather than a limit on the verdict.
const XREF_KEYWORD_WINDOW: usize = 96;

/// The trailer the file itself states most recently, rather than the `/Prev` chain's merge.
///
/// The newest section is the first [`pdf_syntax::xref::sections`] reports, because that walk
/// follows `startxref` and then `/Prev`. An empty dictionary there is not treated as a statement:
/// a section this crate could not read a trailer out of says nothing about what the file states,
/// so the merge answers instead. See [`file_identifier`] for the clause that makes the
/// distinction matter.
fn stated_trailer(document: &Document) -> Dictionary {
    let newest = pdf_syntax::xref::sections(document.bytes(), document.limits())
        .into_iter()
        .next()
        .map(|section| section.trailer)
        .filter(|trailer| trailer.iter().next().is_some());
    newest.unwrap_or_else(|| document.trailer().clone())
}

/// ISO 19005-2 section 6.1.4, ISO 19005-4 section 6.1.4.
///
/// Both parts state the rule in the same words and neither defines the marker itself: that is the
/// base standard's, and ISO 32000-2 §7.2.3 makes it one of three byte sequences. **The
/// subclause is 7.2.2 in ISO 32000-1:2008**, which is the base standard a PDF/A-2 file adheres
/// to, and whose 7.2.3 is *Comments*; both editions state the sentence below in the same words —
///
/// > The CARRIAGE RETURN (0Dh) and LINE FEED (0Ah) characters, also called newline characters,
/// > shall be treated as end-of-line (EOL) markers. The combination of a CARRIAGE RETURN followed
/// > immediately by a LINE FEED shall be treated as one EOL marker.
///
/// — so a SPACE before the marker, or a second marker after it, is a separator the clause does
/// not allow. Those are the two shapes the corpus witnesses, and neither changes what the table
/// parses to, which is why this rule reads the file's bytes rather than [`Examination::objects`].
///
/// # What is judged, and what has no subject
///
/// Every section `startxref` and `/Prev` name is judged, not only the last: the clause says "the
/// xref keyword", and a file's earlier sections each state one. A section written as
/// ISO 32000-2 §7.5.8's cross-reference *stream* has no `xref` keyword at all — §7.5.8.1 forbids
/// one in a file written entirely with streams — so the rule has no subject there and nothing is
/// reported, which is the shape of the construct rather than an exemption this crate grants.
///
/// Two more cases report nothing, both because the bytes do not carry the rule's subject rather
/// than because they satisfy it: a section whose offset does not lead to the keyword (a
/// cross-reference table this crate reached by some other route), and a classic section that
/// states no subsection header after its keyword. A file recovered by scanning states no sections
/// at all, and is silent for the same reason.
fn cross_reference_keyword_line_endings(exam: &Examination<'_>, findings: &mut Findings) {
    let document = exam.document;
    for section in pdf_syntax::xref::sections(document.bytes(), document.limits()) {
        if !section.classic {
            continue;
        }
        let window = document
            .bytes()
            .read(section.offset..section.offset.saturating_add(XREF_KEYWORD_WINDOW));
        // §7.5.5 measures `startxref` to "the beginning of the xref keyword", but a producer
        // that pointed a little short of it has said nothing about *this* rule, so the keyword
        // is looked for rather than assumed and a window without one is passed over.
        let leading = window
            .iter()
            .position(|byte| !is_white_space(*byte))
            .unwrap_or(window.len());
        let Some(after) = window
            .get(leading..)
            .and_then(|rest| rest.strip_prefix(b"xref".as_slice()))
        else {
            continue;
        };
        // §7.5.4 gives the subsection header as two integers, so the first byte of what follows
        // the separator is a decimal digit. Anything else — the `trailer` keyword of a section
        // with no subsection, or the end of the window — is not a header for the separator to
        // be judged against.
        let Some(header_at) = after.iter().position(|byte| !is_white_space(*byte)) else {
            continue;
        };
        if !after.get(header_at).is_some_and(u8::is_ascii_digit) {
            continue;
        }
        let separator = after.get(..header_at).unwrap_or_default();
        if matches!(separator, b"\r" | b"\n" | b"\r\n") {
            continue;
        }
        findings.record(
            Where::file().named("xref"),
            format!(
                "the xref keyword of the cross-reference section at byte {} and the subsection \
                 header after it are separated by {} rather than by a single end-of-line marker",
                section.offset,
                describe_separator(separator),
            ),
        );
    }
}

/// How a separator that is not a single end-of-line marker is named in a finding.
///
/// A witness a reader can act on: "two line feeds" and "a space and a line feed" are different
/// edits to the file, and a byte count alone would send them looking for the difference.
fn describe_separator(separator: &[u8]) -> String {
    if separator.is_empty() {
        return "nothing".to_owned();
    }
    let named: Vec<&str> = separator
        .iter()
        .map(|byte| match byte {
            b'\r' => "a carriage return",
            b'\n' => "a line feed",
            b' ' => "a space",
            b'\t' => "a tab",
            12 => "a form feed",
            0 => "a null",
            _ => "a byte",
        })
        .collect();
    named.join(" and ")
}

/// ISO 19005-2 section 6.1.9, ISO 19005-4 section 6.1.8.
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

/// ISO 19005-2 section 6.1.10, ISO 19005-4 section 6.1.9.
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
    use pdf_syntax::{Dictionary, Document, Name, Object};

    use crate::Examination;
    use crate::finding::Findings;
    use crate::target::Target;

    use super::{
        LARGEST_INTEGER, LONGEST_NAME, LONGEST_STRING, SMALLEST_INTEGER,
        cross_reference_keyword_line_endings, for_each_value, hexadecimal_string_digits,
        hexadecimal_string_holds_only_digits, shortened, stream_keyword_line_endings,
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
        // Read off ISO 19005-2 section 6.1.13 rather than remembered: the boundary values
        // themselves conform, and only what lies beyond them does not.
        assert_eq!(LARGEST_INTEGER, 2_147_483_647);
        assert_eq!(SMALLEST_INTEGER, -2_147_483_648);
        assert_eq!(LONGEST_STRING, 32_767);
        assert_eq!(LONGEST_NAME, 127);
    }

    /// A file assembled from object bodies, with a cross-reference table over them.
    ///
    /// The rule under test is about the file's *bytes*, so the fixture has to be a file the
    /// real parser opens at the real offsets rather than a hand-built object.
    fn document_of(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let number = index.saturating_add(1);
            let _ = writeln!(out, "{number} 0 obj\n{body}\nendobj");
        }
        let start = out.len();
        let size = objects.len().saturating_add(1);
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer << /Size {size} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).unwrap_or_else(|_| Document::empty())
    }

    /// What one predicate said about one document held to PDF/A-2b.
    fn judged(document: &Document, rule: fn(&Examination<'_>, &mut Findings)) -> Findings {
        let mut findings = Findings::default();
        rule(
            &Examination::new(document, Target::Two(crate::target::Level::B)),
            &mut findings,
        );
        findings
    }

    /// §7.3.8 admits `stream` followed by CRLF or by LF, and nothing else — so a lone carriage
    /// return fails, and so does a space before the marker.
    #[test]
    fn a_stream_keyword_is_followed_by_one_of_two_markers_and_no_other() {
        let good = document_of(&[
            "<< /Type /Catalog >>",
            "<< /Length 4 >>\nstream\r\nabcd\r\nendstream",
            "<< /Length 4 >>\nstream\nabcd\nendstream",
        ]);
        assert!(judged(&good, stream_keyword_line_endings).met());

        let carriage_return = document_of(&[
            "<< /Type /Catalog >>",
            "<< /Length 4 >>\nstream\rabcd\r\nendstream",
        ]);
        let findings = judged(&carriage_return, stream_keyword_line_endings);
        assert_eq!(findings.seen(), 1);
        assert!(
            findings.kept()[0]
                .what
                .contains("stream keyword is followed")
        );

        let extra_space = document_of(&[
            "<< /Type /Catalog >>",
            "<< /Length 4 >>\nstream \nabcd\r\nendstream",
        ]);
        assert_eq!(judged(&extra_space, stream_keyword_line_endings).seen(), 1);
    }

    /// The second sentence: an `endstream` that follows the data with nothing between them.
    #[test]
    fn an_endstream_keyword_wants_an_end_of_line_marker_before_it() {
        let touching = document_of(&[
            "<< /Type /Catalog >>",
            "<< /Length 4 >>\nstream\r\nabcdendstream",
        ]);
        let findings = judged(&touching, stream_keyword_line_endings);
        assert_eq!(findings.seen(), 1);
        assert!(findings.kept()[0].what.contains("endstream keyword"));
    }

    /// A stream whose data spells the keyword is still measured from where its data ends,
    /// which is why the search starts at the length rather than at the `stream` keyword.
    #[test]
    fn data_that_spells_the_keyword_does_not_move_where_the_keyword_is_looked_for() {
        let awkward = document_of(&[
            "<< /Type /Catalog >>",
            "<< /Length 15 >>\nstream\r\nendstream x\r\n\r\nendstream",
        ]);
        assert!(judged(&awkward, stream_keyword_line_endings).met());
    }

    /// The same fixture as [`document_of`], with what stands after the `xref` keyword chosen.
    ///
    /// The rule under test is about exactly those bytes, so they are the parameter; every other
    /// byte of the file is the one [`document_of`] writes.
    fn document_with_xref_separator(objects: &[&str], separator: &str) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let number = index.saturating_add(1);
            let _ = writeln!(out, "{number} 0 obj\n{body}\nendobj");
        }
        let start = out.len();
        let size = objects.len().saturating_add(1);
        let _ = write!(out, "xref{separator}0 {size}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer << /Size {size} /Root 1 0 R /ID [<AA> <BB>] >>\nstartxref\n{start}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).unwrap_or_else(|_| Document::empty())
    }

    /// ISO 19005-2 section 6.1.4 and ISO 19005-4 section 6.1.4, over the three separators §7.2.3
    /// allows.
    ///
    /// Written as a comparison because an assertion about one file passes for a reader that never
    /// applies the rule: the same document, differing only in the bytes between `xref` and the
    /// subsection header.
    #[test]
    fn the_xref_keyword_takes_one_end_of_line_marker_and_no_other_separator() {
        let objects = ["<< /Type /Catalog >>"];
        for allowed in ["\n", "\r", "\r\n"] {
            let document = document_with_xref_separator(&objects, allowed);
            assert!(
                judged(&document, cross_reference_keyword_line_endings).met(),
                "§7.2.3 makes {allowed:?} an end-of-line marker"
            );
        }
        for forbidden in [" \n", "\n\n", "\n ", "\r\r", "\t\n"] {
            let document = document_with_xref_separator(&objects, forbidden);
            assert!(
                !judged(&document, cross_reference_keyword_line_endings).met(),
                "{forbidden:?} is not a single end-of-line marker"
            );
        }
    }

    /// ISO 19005-2 section 6.1.6, ISO 19005-4 section 6.1.5, and the base standard's §7.3.4.3
    /// beside it.
    ///
    /// The point of the pair: a conforming reader builds the same value from `<414>` as from
    /// `<4140>` and from `<41!2>` as from `<4142>`, so a check reading the objects would pass all
    /// four. These read what the bytes said.
    #[test]
    fn a_hexadecimal_strings_digits_are_judged_as_written_rather_than_as_read() {
        let even = document_of(&["<< /Type /Catalog /Note <4142> >>"]);
        assert!(judged(&even, hexadecimal_string_digits).met());
        assert!(judged(&even, hexadecimal_string_holds_only_digits).met());

        let odd = document_of(&["<< /Type /Catalog /Note <414> >>"]);
        assert!(!judged(&odd, hexadecimal_string_digits).met());
        assert!(
            judged(&odd, hexadecimal_string_holds_only_digits).met(),
            "an odd count is not a stray byte"
        );

        let strayed = document_of(&["<< /Type /Catalog /Note <41!42> >>"]);
        assert!(!judged(&strayed, hexadecimal_string_holds_only_digits).met());

        let literal = document_of(&["<< /Type /Catalog /Note (414) >>"]);
        assert!(
            judged(&literal, hexadecimal_string_digits).met(),
            "a literal string states no hexadecimal digits at all"
        );
    }

    /// The same rules over a content stream, which is where the corpus's witnesses write it.
    ///
    /// §7.8.2 makes a content stream's operands direct objects, so a text-showing operator's
    /// hexadecimal string is one — and it is invisible to a walk of the file's indirect objects,
    /// which is why this rule reads the decoded content as well.
    #[test]
    fn a_hexadecimal_string_inside_a_content_stream_is_judged_too() {
        let page = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>";
        let show = |text: &str| {
            format!(
                "<< /Length {} >>\nstream\nBT {text} Tj ET\nendstream",
                text.len() + 9
            )
        };
        let good = document_of(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            page,
            &show("<4142>"),
        ]);
        assert!(judged(&good, hexadecimal_string_digits).met());

        let odd = document_of(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            page,
            &show("<41425>"),
        ]);
        assert!(!judged(&odd, hexadecimal_string_digits).met());
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

//! The published clarifications of ISO 19005, as an input to the table rather than hearsay.
//!
//! # A third kind of document, and why it needed its own module
//!
//! [`crate::errata`] reads *corrections*: an approved erratum changes the standard's text, so the
//! corrected sentence is what a requirement means. A **clarification** does neither of those
//! things and is not nothing either. PDF Association `TechNote 0010` says so about itself in its
//! own review section: it interprets the existing specifications and does not change their text.
//! What makes it more than commentary is who resolved each item — the ISO working group
//! responsible for ISO 19005, which considered every ambiguity the PDF Validation Technical
//! Working Group raised and recorded a resolution for each, while deciding not to reopen the
//! published parts.
//!
//! So the text stands, the clause number stands, and what changes is *what the sentence requires
//! of a validator*. That is a category this project had not met before ADR 0931, and the reason
//! it gets a module rather than a comment is the one `errata` gives: a reader checking a verdict
//! against their own copy of the standard has the published sentence in front of them and has to
//! be told which reading this crate applied and where that reading is published.
//!
//! # How a reader tells one this crate acts on from guidance it does not
//!
//! `doc/questions/Q52` rejected two records — a resolution reported inside a third party's test
//! fixture, and a conference summary on a web page — and it was right to. The test that
//! distinguishes them is four conditions, all of which have to hold:
//!
//! 1. **This tree holds the document and a round has read it.** Not a summary of it, not a fact
//!    about it asserted in somebody's outline. `doc/TechNote0010.pdf`, extracted to `doc/md/`.
//! 2. **It is published by the body that publishes the standard's corrections.** The PDF
//!    Association records ISO 19005 errata, and for parts 1 to 3 it records none and directs a
//!    reader to the Technical Notes instead — so this is where those parts' corrections live,
//!    not a stray opinion beside them.
//! 3. **The item carries a resolution of the ISO working group**, under the note's own `ISO WG
//!    Resolution` heading. The problem statement above it, and the TWG's proposal where there is
//!    one, are *not* enough, and the note proves why: at A016, A018 and A019 the working group
//!    left the published requirement exactly as it stood after hearing the objection, and at
//!    A008 it declined a paragraph the TWG had proposed adding, sending it to the next part
//!    instead. Four items, on all of which a project reading the case rather than the verdict
//!    would have changed a rule the committee did not change.
//! 4. **The item names the parts and clauses it reaches**, in its own `Pertaining` line, so its
//!    reach is read rather than inferred. A021 names parts 2 and 3; it does not name part 4, and
//!    ISO 19005-4 is a later standard whose own text governs it.
//!
//! A record failing any of the four is evidence about somebody's reading, which `CLAUDE.md`
//! principle 5 says is not a source of truth. A record meeting all four is the committee that
//! wrote the sentence saying what the sentence asks.
//!
//! # What a clarification does to a row
//!
//! The same two things an erratum does. It **changes what a row means**, which the row's own
//! check has to reflect, and it **is cited in the report** so that a disagreement with another
//! validator is legible as a reading rather than as a bug. Two shapes beyond narrowing a rule have
//! turned up so far: A021 does not narrow a requirement but puts one outside validation
//! altogether, which is what [`crate::Check::OutsideValidation`] exists for; and A002 and A028
//! each *widen* a rule — A002 by binding a part whose own text does not state the sentence, A028
//! by taking away a licence the base standard's own reading of §8.6.5.6 would give.
//!
//! **A row a clarification only confirms is carried here too**, and that is deliberate. The
//! citation is what tells a reader which of two available readings of a published sentence this
//! crate applied — whether `q` nesting was summed across a form `XObject`, whether one painting
//! side's overprinting forbids the other side's colour space — and a verdict that agrees with the
//! committee for a reason nobody can see is indistinguishable from one that agrees by luck.
//!
//! # Every item of `TechNote 0010` that bears on this table
//!
//! Recorded in full because the note was read in full, and because an item nobody wrote down is
//! an item the next round has to rediscover. Items pertaining only to ISO 19005-1 or -3 are
//! absent: neither part is a target (`doc/questions/A17`). **This list is checked against the note
//! rather than against its predecessor** — session 941's copy of it omitted A028 outright and
//! called A002 already true when the row it names bound part 4 alone.
//!
//! **Taken, and a check changed**; each is carried in [`CLARIFICATIONS`] below:
//!
//! - **A002**, ISO 19005-2 section 6.2.2 — the explicitly associated resources dictionary shall
//!   define every named resource its content stream references. Part 2's text requires the
//!   dictionary and does not say that, so `graphics/named-resources-are-defined` bound part 4
//!   alone until session 942 and now binds both parts.
//! - **A020**, ISO 19005-2 section 6.6.2.3.1 — an XMP value is validated on its type alone, and
//!   the note's list of the basic types puts `Rational` among those admitting any string. So
//!   `metadata/properties-use-known-schemas` no longer holds `exif:XResolution` and its like to a
//!   quotient. Two points of that list are open and are `doc/questions/Q53`.
//! - **A021**, ISO 19005-2 section 6.6.6 — the whole of the file-provenance requirement is outside
//!   validation.
//! - **A028**, ISO 19005-2 section 6.2.2 — a default colour space shall itself be defined in the
//!   explicitly associated resources dictionary, and a processor ignores what that dictionary does
//!   not define. With A003, which fixes what the term *explicitly associated* names, that means
//!   a form `XObject`, a tiling pattern or a Type 3 font stating no `Resources` entry of its own
//!   reads no §8.6.5.6 default at all — so the six part 2 rows whose licence turns on one read
//!   [`crate::survey::DefaultSpace::explicit`] where part 4's read
//!   [`crate::survey::DefaultSpace::in_force`]. **This is the second clarification that makes a
//!   rule bind more rather than less**, and the corpus cannot rank it either: not one of
//!   `doc/veraPDF-corpus`'s documents both states a default colour space and runs a content stream
//!   against a dictionary it fell back on, so `over` is unmoved on all six targets. ADR 0935.
//! - **A029**, ISO 19005-2 section 6.6.2.3.2 — an extension schema that describes no custom value
//!   types may omit `pdfaSchema:valueType`, and a value type with no structured fields may omit
//!   `pdfaType:field`; a validator treats each absence as an empty array.
//!
//! **Taken as confirmation**: the row already read the sentence this way, and the record is
//! carried in [`CLARIFICATIONS`] so that a verdict says which reading it applied.
//!
//! - **A003** — an explicitly associated resources dictionary is the `Resources` entry of a page,
//!   a tiling pattern, a form `XObject` including an annotation appearance, or a Type 3 font
//!   dictionary, and never one inherited through the page tree
//!   (`graphics/content-streams-have-an-explicit-resources-dictionary`).
//! - **A004** — `q`/`Q` nesting is counted within one content stream and not summed across a form
//!   `XObject`'s invocation (`implementation-limits/graphics-state-nesting`).
//! - **A005** — a name's or a string's length is that of its decoded internal representation
//!   (`implementation-limits/string-lengths`, `name-lengths`).
//! - **A007** — the character-identifier limit is applied to a `CMap`'s syntax rather than to what
//!   a content stream selects, which makes `/W` and `/CIDToGIDMap` outside the rule rather than a
//!   gap in it (`implementation-limits/character-identifiers`).
//! - **A012**, **A023** — a button field's normal appearance is always a subdictionary, push
//!   buttons included, and the field type is read from the parent field dictionary where the
//!   widget is not merged with it (`annotations/normal-appearance-shape`). One entry carries both,
//!   because they resolve two readings of one sentence. A012's own `Pertaining` line names PDF/A-2
//!   twice where it evidently means PDF/A-3, which costs this crate nothing: part 3 is not a
//!   target.
//! - **A024** — the overprint-mode rule pairs each side with its own parameter: stroking with an
//!   `ICCBased` CMYK space while stroke overprinting is on, or filling with one while fill
//!   overprinting is on (`graphics/no-overprint-mode-one-under-icc-cmyk`). The note's `Pertaining`
//!   line numbers this part 2 section 6.2.4.3 where the copy in `doc/pdfa/` carries the sentence in
//!   section 6.2.4.2; a rule is identified by its sentence, and the row's citation follows the
//!   standard.
//! - **A026** — `DeviceGray` is permitted as the `ColorSpace` of a soft-mask image dictionary with
//!   no default space and no output intent, because there it describes shape rather than colour
//!   (`graphics/device-gray-needs-a-default-or-an-output-intent`, whose entry names A028 beside it
//!   because that row reads a `DefaultGray` too). Session 941 flagged this as the
//!   item most likely to be costing a conforming file a false failure, and it is not:
//!   `crate::survey` records an image's colour space only for an image a content stream draws,
//!   and never descends into an `SMask` entry, so no soft-mask image's colour space reaches the
//!   rule. What the record binds is any later round that widens the walk, which is why the
//!   reading is written at the rule and at the walk's boundary both.
//!
//! **Already true, and nothing to cite** — either because the resolution leaves the published
//! requirement exactly as the row states it, or because it names no part this crate has a target
//! for and a citation would have to reach further than the working group did:
//!
//! - **A008** — vertical glyph metrics are out of scope in parts 1 to 3, and
//!   `fonts/vertical-metrics-agree-with-the-program` already cites part 4 alone.
//! - **A011**, **A015** — a colour space or a device colour set in the graphics state is used
//!   whether or not anything is painted with it, which is how `crate::survey` records one. The
//!   `Pertaining` line names ISO 19005-1 alone while the resolution itself names parts 2 and 3's
//!   section 6.2.4.1, so it is recorded here rather than passed over as a part 1 item.
//! - **A016** — the `CharSet` and `CIDSet` text does not change, which is what the two rows named
//!   for those keys already implement.
//! - **A018**, **A019** — the working group declined both proposals about real-value limits, so
//!   `implementation-limits/real-values` stands as published.
//! - **A025** — an embedded `CMap`'s `usecmap` may name only a predefined `CMap`, which is
//!   `fonts/cmap-uses-only-predefined-cmaps`.
//!
//! **Owed**, each with the row it would touch and what it would take:
//!
//! - **A010** — an unreferenced named resource is exempt from the part *except* its file-structure
//!   and implementation-limit subclauses, sections 6.1.2 to 6.1.13, and shall still conform to the
//!   base standard. No row states the exemption. Half of it holds here by construction: the rows
//!   that read `crate::survey` never see a resource nothing references, because the walk reaches a
//!   form `XObject` only through the `Do` that names it. The other half does not — the rows that
//!   walk every object a cross-reference section names judge an unreferenced image or font like any
//!   other, which is right for sections 6.1.2 to 6.1.13 and an over-report for everything else.
//!   Taking it means the survey recording *which* names each resources dictionary had referenced,
//!   and the object-walking rows reading that set; it is a predicate to write rather than a rule to
//!   withdraw, and it is the one item of this note that could move `over` off zero in either
//!   direction. **Session 943 deferred it deliberately**, and ADR 0935 carries the argument: what
//!   is missing here is section 6.2.2's *published* exemption rather than A010, which only narrows
//!   it; and exempting an object requires proving it is reachable no other way, which no fact this
//!   crate holds today can decide. Getting that wrong withdraws real failures in silence, which is
//!   the one direction a validator may not move by accident. **Session 944 measured how much is
//!   being deferred**, and the answer is `doc/todo/62` with `examples/unreferenced.rs` as the
//!   command that recounts it. What it found: over `doc/veraPDF-corpus`'s six targets, every
//!   document whose verdict turns on an object this exemption reaches fails under a clause the
//!   applicable carve-out **keeps** — part 4's own published sections 6.1.6 to 6.1.9, and part
//!   2's under A010. So the exemption *with* A010 moves nothing here, and part 2's published
//!   sentence *without* it would withdraw failures this crate and the corpus agree on. **A010 is
//!   what makes the exemption safe rather than a refinement to add after it**, which reverses the
//!   order ADR 0935 proposed. ADR 0941.
//!
//! **Reaching no part this crate targets.** One item is left, and it is recorded here rather
//! than passed over so that the next round does not have to re-derive it:
//!
//! - **A014** — its `Pertaining` line names ISO 19005-1 section 6.3.4 alone, and part 1 is not a
//!   target (`doc/questions/A17`). What makes it worth an entry is the paragraph above its
//!   resolution, headed for validators of parts 1, 2 and 3, which would fail any ISO 19005
//!   document holding a font or `CIDFont` program whose `Subtype` the applicable PDF
//!   specification does not support. **That paragraph is the case rather than the verdict**, and
//!   the note's own shape says so: A012, A020 and A029 each carry a `PDF Validation TWG proposal`
//!   whose adoption their resolution states in as many words — the three parts should be read as
//!   if the proposal above were part of the specification — and that is the sentence by which
//!   this crate acts on their validator paragraphs. A014 has no proposal heading at all, and its
//!   resolution states one thing: OpenType is not recognised in the PDF 1.4 Reference and
//!   therefore produces an ISO 19005-1 validation error. Condition 3 above decides it.
//!   **A second reading reaches the same place from the other side**, and is worth having because
//!   it does not depend on how the note is laid out: the paragraph's rule is stated against *the
//!   applicable PDF specification*, and for part 2 that is ISO 32000-1 and for part 4
//!   ISO 32000-2, whose §9.9 and §9.9.1 respectively name `OpenType` as one of the three
//!   `/FontFile3` subtypes and then specify the format in full — so the ambiguity A014 exists to
//!   resolve is one only the PDF 1.4 Reference has. Nothing is owed.
//!   The residue is real and belongs elsewhere: a `/FontFile3` whose `/Subtype` names none of the
//!   three the base standard defines goes unreported here, and that is
//!   `conformance/adheres-to-the-base-standard`'s `Unchecked` — a decision this crate has already
//!   argued — rather than a font row nobody wrote. ADR 0941.

use crate::target::Part;

/// One published clarification, as this crate cites it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clarification {
    /// The technical note that publishes it, as the note names itself.
    pub note: &'static str,
    /// The item number within it — `A021`.
    ///
    /// Two item numbers where two of the note's items reach one row and citing either alone
    /// would send a reader to half of the answer. `A012 and A023` resolve two readings of one
    /// sentence about the normal appearance of a button field's widget; `A026 and A028` resolve
    /// two different questions that meet at ISO 19005-2 section 6.2.4.3's `DeviceGray` rule —
    /// what a soft mask's samples are, and which dictionary the `DefaultGray` that licenses the
    /// rest is read from.
    pub item: &'static str,
    /// Which parts of ISO 19005 the working group's resolution names, for a reader.
    pub parts: &'static str,
    /// The same reach, as the two parts this crate has targets for.
    ///
    /// ADR 0931's fourth condition is that an item names the parts it reaches, so that its reach
    /// is read rather than inferred — and a row that binds both parts would otherwise print a
    /// resolution about ISO 19005-2 beside a PDF/A-4 verdict, which is inferring. Every item of
    /// `TechNote 0010` predates ISO 19005-4 and none of them names it, so this is
    /// [`Part::Two`] throughout today; the field exists because "the note has no part 4 items"
    /// is a fact about one note rather than a rule, and [`clarifying`] asks it rather than
    /// assuming it. ISO 19005-1 and -3 never appear because neither is a target
    /// (`doc/questions/A17`); [`Clarification::parts`] is where a reader sees the whole reach.
    pub reaches: &'static [Part],
    /// What the resolution asks of a validator, in one sentence of this crate's own words.
    pub change: &'static str,
}

/// What A028 asks of every row whose licence turns on §8.6.5.6's default colour space.
///
/// One sentence written once, because six rows of `crate::table::graphics` read that default and
/// the resolution says the same thing to all of them. Five of the six say it in these words; the
/// `DeviceGray` row spells it out again in its own entry, which also carries A026.
/// `crate::survey::DefaultSpace` is where the two readings are recorded, and
/// `licensed_by_default_under_part_two` is where part 2 picks its own.
const DEFAULT_SPACE_IS_EXPLICITLY_ASSOCIATED: &str = concat!(
    "a default colour space shall itself be defined in the resources dictionary explicitly ",
    "associated with the content stream, and a processor ignores any resource that dictionary ",
    "does not define — so a form XObject, a tiling pattern or a Type 3 font stating no ",
    "Resources entry of its own has no default in force, whatever the page it is drawn on ",
    "defines",
);

/// Every clarification that bears on a row of the table, by requirement identifier.
///
/// One place, reviewable against the note in one reading, for [`crate::errata`]'s reason. Ordered
/// by item number rather than by row, because reviewing it means reading the note beside it.
static CLARIFICATIONS: &[(&str, Clarification)] = &[
    (
        "graphics/named-resources-are-defined",
        Clarification {
            note: "TechNote 0010",
            item: "A002",
            parts: "ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: "the resources dictionary explicitly associated with a content stream shall \
                     define every named resource that stream references, which the published \
                     sentence requires the dictionary to exist without saying — so this row \
                     binds a part 2 file on the working group's resolution rather than on part \
                     2's own words",
        },
    ),
    (
        "graphics/content-streams-have-an-explicit-resources-dictionary",
        Clarification {
            note: "TechNote 0010",
            item: "A003",
            parts: "ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: "an explicitly associated resources dictionary is the Resources entry of a \
                     page, a tiling pattern, a form XObject including an annotation appearance \
                     stream, or a Type 3 font dictionary, and never one reached by inheritance \
                     through the page tree",
        },
    ),
    (
        "implementation-limits/graphics-state-nesting",
        Clarification {
            note: "TechNote 0010",
            item: "A004",
            parts: "ISO 19005-1, ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: "the nesting limit assumes each content stream considered in isolation and \
                     ignores the cumulative effect of nesting form XObjects, so the depth is \
                     counted within one stream and not summed across an invocation",
        },
    ),
    (
        "implementation-limits/string-lengths",
        Clarification {
            note: "TechNote 0010",
            item: "A005",
            parts: "ISO 19005-1, ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: "a string's length is the length of its internal byte representation, after \
                     every escape sequence is decoded and a hexadecimal string is folded to the \
                     bytes it spells",
        },
    ),
    (
        "implementation-limits/name-lengths",
        Clarification {
            note: "TechNote 0010",
            item: "A005",
            parts: "ISO 19005-1, ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: "a name's length is the length of its internal byte representation, after \
                     every number-sign escape is decoded",
        },
    ),
    (
        "implementation-limits/character-identifiers",
        Clarification {
            note: "TechNote 0010",
            item: "A007",
            parts: "ISO 19005-1, ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: "the limit on a character identifier is applied to a CMap's syntax, so that \
                     the CMap stream can be parsed, rather than to the identifiers a content \
                     stream selects",
        },
    ),
    (
        "annotations/normal-appearance-shape",
        Clarification {
            note: "TechNote 0010",
            item: "A012 and A023",
            parts: "ISO 19005-1, ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: "every field of type Btn takes an appearance subdictionary as the value of \
                     N, a push button included and even where it holds one entry; and where a \
                     widget is not merged with its field the field type is read from the parent \
                     form field dictionary",
        },
    ),
    (
        "metadata/properties-use-known-schemas",
        Clarification {
            note: "TechNote 0010",
            item: "A020",
            parts: "ISO 19005-1, ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: "an XMP value is validated on its type alone, with any further meaning \
                     inferred from the property's name or description disregarded, and the \
                     note's list of the basic types puts Rational among those admitting any \
                     string",
        },
    ),
    (
        "metadata/provenance-recorded-action-fields",
        Clarification {
            note: "TechNote 0010",
            item: "A021",
            parts: "ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: "requirements on the xmpMM:History property are requirements on the \
                     application that writes it and are therefore irrelevant to ISO 19005 \
                     validation, so no field of a recorded action is a validator's question — \
                     the working group resolved that both parts are to be read as if that \
                     proposal were part of the specification",
        },
    ),
    (
        "graphics/no-overprint-mode-one-under-icc-cmyk",
        Clarification {
            note: "TechNote 0010",
            item: "A024",
            parts: "ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: "the rule pairs each painting side with its own overprint parameter — an \
                     ICCBased CMYK space used for stroking while stroke overprinting is on, or \
                     for filling while fill overprinting is on, or both — so one side's \
                     overprinting does not forbid the other side's space",
        },
    ),
    (
        "graphics/device-gray-needs-a-default-or-an-output-intent",
        Clarification {
            note: "TechNote 0010",
            item: "A026 and A028",
            parts: "ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: "DeviceGray is admitted as the ColorSpace of a soft-mask image dictionary \
                     with neither a default space nor an output intent, because there its \
                     samples describe shape rather than colour; and the DefaultGray that \
                     licenses it elsewhere is read from the resources dictionary explicitly \
                     associated with the content stream that selected it, so a form XObject or \
                     a Type 3 font stating no Resources entry of its own reads none",
        },
    ),
    (
        "graphics/device-rgb-needs-a-default-or-an-rgb-output-intent",
        Clarification {
            note: "TechNote 0010",
            item: "A028",
            parts: "ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: DEFAULT_SPACE_IS_EXPLICITLY_ASSOCIATED,
        },
    ),
    (
        "graphics/device-cmyk-needs-a-default-or-a-cmyk-output-intent",
        Clarification {
            note: "TechNote 0010",
            item: "A028",
            parts: "ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: DEFAULT_SPACE_IS_EXPLICITLY_ASSOCIATED,
        },
    ),
    (
        "graphics/separation-alternate-spaces-obey-the-colour-rules",
        Clarification {
            note: "TechNote 0010",
            item: "A028",
            parts: "ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: DEFAULT_SPACE_IS_EXPLICITLY_ASSOCIATED,
        },
    ),
    (
        "graphics/indexed-and-pattern-base-spaces-obey-the-colour-rules",
        Clarification {
            note: "TechNote 0010",
            item: "A028",
            parts: "ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: DEFAULT_SPACE_IS_EXPLICITLY_ASSOCIATED,
        },
    ),
    (
        "graphics/transparency-group-colour-spaces-obey-the-colour-rules",
        Clarification {
            note: "TechNote 0010",
            item: "A028",
            parts: "ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: DEFAULT_SPACE_IS_EXPLICITLY_ASSOCIATED,
        },
    ),
    (
        "metadata/extension-schema-container-fields",
        Clarification {
            note: "TechNote 0010",
            item: "A029",
            parts: "ISO 19005-1, ISO 19005-2 and ISO 19005-3",
            reaches: &[Part::Two],
            change: "an extension schema that defines no custom value types may omit \
                     pdfaSchema:valueType, and a value type that defines no structured fields \
                     may omit pdfaType:field; a validator shall allow each absence and treat it \
                     as an empty array",
        },
    ),
];

/// The clarification bearing on one requirement under one part, if there is one.
///
/// The part is asked rather than assumed: several rows here state the same rule in both parts and
/// carry a resolution that names only one of them, and citing it against the other part's verdict
/// would be this crate inferring a reach the working group did not state.
#[must_use]
pub fn clarifying(id: &str, part: Part) -> Option<Clarification> {
    CLARIFICATIONS
        .iter()
        .find(|(row, clarification)| *row == id && clarification.reaches.contains(&part))
        .map(|(_, clarification)| *clarification)
}

/// Every clarification in the table, for a caller that wants to list them.
pub fn all() -> impl Iterator<Item = (&'static str, Clarification)> {
    CLARIFICATIONS.iter().copied()
}

#[cfg(test)]
mod tests {
    use super::{CLARIFICATIONS, Part, all, clarifying};
    use crate::table;

    /// A clarification naming a row that does not exist is a table that has drifted.
    #[test]
    fn every_clarification_names_a_requirement_that_exists() {
        for (id, clarification) in all() {
            assert!(
                table::requirements().any(|requirement| requirement.id == id),
                "{} {} names {id}, which is not a requirement",
                clarification.note,
                clarification.item
            );
        }
    }

    #[test]
    fn a_clarification_is_found_by_the_identifier_it_names() {
        for (id, clarification) in CLARIFICATIONS {
            for part in clarification.reaches {
                assert_eq!(
                    clarifying(id, *part).map(|found| found.item),
                    Some(clarification.item)
                );
            }
        }
        assert_eq!(clarifying("nothing/at/all", Part::Two), None);
    }

    /// A resolution is cited only under a part it names, which is ADR 0931's fourth condition.
    ///
    /// The rows this bites on are the ones that state the same rule in both parts —
    /// `annotations/normal-appearance-shape` is one — where a citation naming ISO 19005-2 would
    /// otherwise print beside a PDF/A-4 verdict and read as its ground.
    #[test]
    fn a_clarification_is_not_cited_under_a_part_it_does_not_name() {
        for (id, clarification) in CLARIFICATIONS {
            for part in [Part::Two, Part::Four] {
                if clarification.reaches.contains(&part) {
                    continue;
                }
                assert_eq!(
                    clarifying(id, part),
                    None,
                    "{} {} does not name that part of ISO 19005",
                    clarification.note,
                    clarification.item
                );
            }
        }
    }

    /// Every row a clarification names carries the note in its report, and that is the whole
    /// point of the module: a reading nobody can see is indistinguishable from a bug.
    ///
    /// Read under the parts the resolution reaches, because a row that binds only the *other*
    /// part would report the clarification nowhere at all.
    #[test]
    fn a_clarified_row_says_so_in_every_report_that_binds_it() {
        for (id, clarification) in all() {
            let cited = crate::target::Target::ALL.iter().any(|target| {
                clarification.reaches.contains(&target.part())
                    && table::binding(*target).any(|requirement| requirement.id == id)
            });
            assert!(
                cited,
                "{id} is clarified and binds no target of a part {} {} names, so nothing would \
                 report it",
                clarification.note, clarification.item
            );
        }
    }
}

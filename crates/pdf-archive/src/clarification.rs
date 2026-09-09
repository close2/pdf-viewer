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
//! validator is legible as a reading rather than as a bug. One shape is new: A021 does not narrow
//! a requirement, it puts one outside validation altogether, which is what
//! [`crate::Check::OutsideValidation`] exists for.
//!
//! # Every item of `TechNote 0010` that bears on this table
//!
//! Recorded in full because the note was read in full, and because an item nobody wrote down is
//! an item the next round has to rediscover. Items pertaining only to ISO 19005-1 or -3 are
//! absent: neither part is a target (`doc/questions/A17`).
//!
//! **Taken**, and carried in [`CLARIFICATIONS`] below:
//!
//! - **A021**, ISO 19005-2 section 6.6.6 — the whole of the file-provenance requirement is outside
//!   validation.
//! - **A029**, ISO 19005-2 section 6.6.2.3.2 — an extension schema that describes no custom value
//!   types may omit `pdfaSchema:valueType`, and a value type with no structured fields may omit
//!   `pdfaType:field`; a validator treats each absence as an empty array.
//!
//! **Already true**, so nothing to take and the note is confirmation rather than input:
//!
//! - **A002** — a content stream's explicitly associated resources dictionary shall define the
//!   named resources the stream references, which is `graphics/named-resources-are-defined`.
//! - **A008** — vertical glyph metrics are out of scope in parts 1 to 3, and
//!   `fonts/vertical-metrics-agree-with-the-program` already cites part 4 alone.
//! - **A016** — the `CharSet` and `CIDSet` text does not change, which is what the two rows named
//!   for those keys already implement.
//! - **A018**, **A019** — the working group declined both proposals about real-value limits, so
//!   `implementation-limits/real-values` stands as published.
//! - **A025** — an embedded `CMap`'s `usecmap` may name only a predefined `CMap`, which is
//!   `fonts/cmap-uses-only-predefined-cmaps`.
//!
//! **Owed**, each with the row it would touch, none of them taken here because each is a
//! predicate to write rather than a rule to withdraw:
//!
//! - **A003** — the term section 6.2.2 uses for a content stream's own resources dictionary
//!   reaches only a page, tiling pattern, form `XObject` or Type 3 font dictionary's `Resources`
//!   entry, never one inherited through the page tree
//!   (`graphics/content-streams-have-an-explicit-resources-dictionary`).
//! - **A004**, **A005**, **A007** — three readings of the implementation limits: nesting is
//!   counted per content stream and not across form `XObject`s, a name's or string's length is
//!   that of its decoded internal representation, and the character-identifier limit is about a
//!   `CMap`'s syntax (`implementation-limits/graphics-state-nesting`, `string-lengths`,
//!   `name-lengths`, `character-identifiers`).
//! - **A010** — an unreferenced named resource is exempt from the part *except* the file-structure
//!   and implementation-limit subclauses, which no row states.
//! - **A012**, **A023** — a button field's normal appearance is always a subdictionary, and the
//!   field type is read from the parent field dictionary where the widget is not merged with it
//!   (`annotations/normal-appearance-shape`).
//! - **A024** — the overprint-mode rule is per operation: stroking with an `ICCBased` CMYK space
//!   while stroke overprint is on, or filling with one while fill overprint is on
//!   (`graphics/no-overprint-mode-one-under-icc-cmyk`). The note's `Pertaining` line numbers this
//!   part 2 section 6.2.4.3 where the copy in `doc/pdfa/` carries the sentence in section 6.2.4.2;
//!   the rule is identified by its sentence, and the row's citation follows the standard.
//! - **A026** — `DeviceGray` is permitted as the `ColorSpace` of a soft-mask image dictionary with
//!   no default space and no output intent, because there it describes shape rather than colour
//!   (`graphics/device-gray-needs-a-default-or-an-output-intent`). This is the one owed item that
//!   could cost a conforming file a false failure.
//! - **A020** — an XMP value is validated on its type alone, with a stated list of what each basic
//!   type admits (`metadata/properties-use-known-schemas`). That row already judges shape and
//!   lexical form and judges no property it does not know, so what is owed is the *list*, checked
//!   against the note's, rather than a change of approach.
//!
//! One item is a validator rule this crate has no row for at all: **A014** makes a font or
//! `CIDFont` program whose `Subtype` the applicable PDF specification does not define a failure
//! in parts 1 to 3. `fonts/font-programs-conform-to-their-own-specifications` is the
//! neighbourhood; the rule is not that row's.

/// One published clarification, as this crate cites it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clarification {
    /// The technical note that publishes it, as the note names itself.
    pub note: &'static str,
    /// The item number within it, which is how it is looked up — `A021`.
    pub item: &'static str,
    /// Which parts of ISO 19005 the working group's resolution names.
    pub parts: &'static str,
    /// What the resolution asks of a validator, in one sentence of this crate's own words.
    pub change: &'static str,
}

/// Every clarification that bears on a row of the table, by requirement identifier.
///
/// One place, reviewable against the note in one reading, for [`crate::errata`]'s reason.
static CLARIFICATIONS: &[(&str, Clarification)] = &[
    (
        "metadata/provenance-recorded-action-fields",
        Clarification {
            note: "TechNote 0010",
            item: "A021",
            parts: "ISO 19005-2 and ISO 19005-3",
            change: "requirements on the xmpMM:History property are requirements on the \
                     application that writes it and are therefore irrelevant to ISO 19005 \
                     validation, so no field of a recorded action is a validator's question — \
                     the working group resolved that both parts are to be read as if that \
                     proposal were part of the specification",
        },
    ),
    (
        "metadata/extension-schema-container-fields",
        Clarification {
            note: "TechNote 0010",
            item: "A029",
            parts: "ISO 19005-1, ISO 19005-2 and ISO 19005-3",
            change: "an extension schema that defines no custom value types may omit \
                     pdfaSchema:valueType, and a value type that defines no structured fields \
                     may omit pdfaType:field; a validator shall allow each absence and treat it \
                     as an empty array",
        },
    ),
];

/// The clarification bearing on one requirement, if there is one.
#[must_use]
pub fn clarifying(id: &str) -> Option<Clarification> {
    CLARIFICATIONS
        .iter()
        .find(|(row, _)| *row == id)
        .map(|(_, clarification)| *clarification)
}

/// Every clarification in the table, for a caller that wants to list them.
pub fn all() -> impl Iterator<Item = (&'static str, Clarification)> {
    CLARIFICATIONS.iter().copied()
}

#[cfg(test)]
mod tests {
    use super::{CLARIFICATIONS, all, clarifying};
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
            assert_eq!(
                clarifying(id).map(|found| found.item),
                Some(clarification.item)
            );
        }
        assert_eq!(clarifying("nothing/at/all"), None);
    }

    /// Every row a clarification names carries the note in its report, and that is the whole
    /// point of the module: a reading nobody can see is indistinguishable from a bug.
    #[test]
    fn a_clarified_row_says_so_in_every_report_that_binds_it() {
        for (id, _) in all() {
            let cited = crate::target::Target::ALL
                .iter()
                .any(|target| table::binding(*target).any(|requirement| requirement.id == id));
            assert!(
                cited,
                "{id} is clarified and binds no target, so nothing would report it"
            );
        }
    }
}

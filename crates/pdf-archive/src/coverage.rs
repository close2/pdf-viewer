//! The structural audit: every subclause of the two owned parts, and what the table does about it.
//!
//! # The question this answers, which no other instrument in this crate can
//!
//! [`crate::table`]'s rows are checked from every side except one. A row's clause is checked
//! against the part that states it, its identifier against every other row's, its reason against
//! the tree it names, and its predicate against the corpus. **A requirement with no row at all is
//! invisible to all of that**: it is not `Unchecked`, it fails no document, it is in no
//! denominator, and no sweep of the reasons that *exist* can see it. Two consecutive sessions
//! swept every `Check::Unchecked` and `Check::Processor` reason and found them accurate, which
//! established nothing whatever about the rules nobody had written a row for.
//!
//! So this module walks the standards the other way round — from the document's own table of
//! contents — and records, for **every** subclause of ISO 19005-2 and ISO 19005-4 including their
//! normative annexes, which of five things the requirement table does about it. The tests below
//! then hold the two against each other in both directions, so that the audit cannot rot the way
//! a paragraph of prose would: a row citing a subclause this file calls silent fails the build,
//! and so does a subclause this file calls bound that no row cites.
//!
//! # What it is not, stated plainly
//!
//! **This is a subclause-level instrument, not a sentence-level one.** A subclause holding four
//! normative sentences of which three have rows is [`Binding::Bound`] here, exactly as one holding
//! four with four is. Sentence-level coverage cannot be committed to this tree at all: it would
//! have to enumerate the standard's sentences, and `doc/questions/A16` licenses these two
//! documents to a single reader. What the coarser instrument buys is the thing prose could not —
//! it is *checked*, on every build, and it cannot silently stop being true.
//!
//! The sentence-level reading is a round's work rather than a file's, and the record of it is the
//! ADR: ADR 0972 lists the sentences this audit's own pass found unbound, and the rows it added
//! for them.
//!
//! # Why the clause numbers are here and the clause text is not
//!
//! A clause *number* is a pointer; the [`Subclause::subject`] beside it is this crate's own
//! sentence, written the way a [`crate::Requirement`]'s `asks` is and for the same reason. A
//! reader who wants the standard's words opens `doc/pdfa/` at the number.

use crate::target::Part;

/// What [`crate::table`] does about one subclause of ISO 19005.
///
/// Five states, and the four that are not [`Self::Bound`] are four different reasons for a
/// subclause to have no row — kept apart because folding any of them into "no row" is what makes
/// a gap invisible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// At least one row of the table cites this subclause for this part.
    Bound,
    /// A heading whose own text states nothing: its children carry the requirements.
    Container,
    /// The subclause states no requirement a conforming file or processor could fail.
    ///
    /// A permission, a recommendation, or informative prose — with the reason saying which.
    /// **Not the same as a requirement nobody has implemented**, which is [`crate::Check::Unchecked`]
    /// and lives on a row; there is nothing owed here.
    StatesNoRequirement(&'static str),
    /// The subclause says how the *other* requirements are read, and the table honours it in its
    /// shape rather than in a row.
    ///
    /// A conformance level's exemptions, a population the font rules reach, an annex that
    /// redefines a flavour: each is a column of the table or a decision inside a tranche, and the
    /// reason names which.
    Scoping(&'static str),
    /// The subclause restates a rule the table already carries under another clause of this part.
    ///
    /// One rule, one row: a part that states the same requirement twice gets one row, cited at
    /// the clause the row names, and the other clause says so here.
    Restated(&'static str),
}

impl Binding {
    /// Whether a row of the table is expected to cite this subclause.
    #[must_use]
    pub const fn expects_a_row(self) -> bool {
        matches!(self, Self::Bound)
    }
}

/// One subclause of one part, with what the table does about it.
#[derive(Debug, Clone, Copy)]
pub struct Subclause {
    /// Which part states it.
    pub part: Part,
    /// Its number within that part, as the part writes it — `6.2.11.4.1`, or `B.1` for an annex.
    pub clause: &'static str,
    /// What it is about, in this crate's own words.
    pub subject: &'static str,
    /// What the table does about it.
    pub binding: Binding,
}

/// Every subclause of both owned parts, in each part's own order.
pub fn subclauses() -> impl Iterator<Item = &'static Subclause> {
    SUBCLAUSES.iter()
}

/// The audit itself.
///
/// Clause 5 is here as well as clause 6, because two rows cite it: a conforming file adheres to
/// its base standard, and part 4 forbids the features that standard deprecates. Clause 6 is the
/// bulk. The normative annexes are the part this crate had never looked at — ISO 19005-2's
/// Annex A and Annex B carried between them a method and nine requirements with not one row
/// against them until the session that wrote this file.
///
/// Clauses 1 to 4 are scope, references, terms and notation, and state no requirement; they are
/// not listed, and that is the one boundary of this audit.
static SUBCLAUSES: &[Subclause] = &[
    Subclause {
        part: Part::Two,
        clause: "5",
        subject: "conformance levels",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "5.1",
        subject: "what a conforming file adheres to",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "5.2",
        subject: "Level A conformance",
        binding: Binding::Scoping(
            "Level A, which is a `Level` and an `Applies::FromLevel` column rather than a row",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "5.3",
        subject: "Level B conformance",
        binding: Binding::Scoping(
            "Level B's exemption from sections 6.2.11.7 and 6.7, carried by `Applies::FromLevel`",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "5.4",
        subject: "Level U conformance",
        binding: Binding::Scoping(
            "Level U's exemption from section 6.7, carried by `Applies::FromLevel`",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "5.5",
        subject: "what a conforming reader is",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6",
        subject: "the technical requirements",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1",
        subject: "file structure",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.1",
        subject: "data neither standard describes",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.2",
        subject: "the file header",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.3",
        subject: "the trailer dictionary",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.4",
        subject: "the cross-reference table, and objects no section names",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.5",
        subject: "the document information dictionary",
        binding: Binding::Restated(
            "the same rule as section 6.6.3 — a document information dictionary is permitted and a \
            processor ignores it — and the table carries it there",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.6",
        subject: "hexadecimal strings",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.7",
        subject: "stream objects",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.7.1",
        subject: "a stream's keywords, its stated length and external data",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.7.2",
        subject: "which filters a stream may use",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.8",
        subject: "name objects",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.9",
        subject: "the syntax around an indirect object",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.10",
        subject: "an inline image's filter",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.11",
        subject: "linearization",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.12",
        subject: "the permissions dictionary",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.1.13",
        subject: "the implementation limits a conforming file stays inside",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2",
        subject: "graphics",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.1",
        subject: "what a processor draws, and the interface around it",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.2",
        subject: "content stream operators and resources",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.3",
        subject: "the PDF/A output intent and its destination profile",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.4",
        subject: "colour spaces",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.4.1",
        subject: "colour specified device-independently",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.4.2",
        subject: "ICCBased colour spaces",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.4.3",
        subject: "the three device colour spaces",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.4.4",
        subject: "Separation and DeviceN colour spaces",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.4.5",
        subject: "Indexed and Pattern colour spaces",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.5",
        subject: "the graphics state parameter dictionary",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.6",
        subject: "rendering intents",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.7",
        subject: "flatness",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.8",
        subject: "images",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.8.1",
        subject: "an image dictionary's forbidden and constrained entries",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.8.2",
        subject: "thumbnail images",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.8.3",
        subject: "JPEG 2000 data",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.9",
        subject: "XObjects",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.9.1",
        subject: "form XObjects",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.9.2",
        subject: "reference XObjects",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.9.3",
        subject: "PostScript XObjects",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.10",
        subject: "transparency and the blending colour space",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11",
        subject: "fonts",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.1",
        subject: "what the font requirements apply to",
        binding: Binding::Scoping(
            "the font rules reach every font the file uses, one shown only in text rendering mode 3 \
            included, unless a rule says otherwise; `super::table::fonts` builds its population \
            that way and its module comment says so",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.2",
        subject: "which font types a conforming file may use",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.3",
        subject: "composite fonts",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.3.1",
        subject: "a Type 0 font's CIDSystemInfo against its CMap's",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.3.2",
        subject: "a CIDFont's CIDToGIDMap",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.3.3",
        subject: "embedded and predefined CMaps",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.4",
        subject: "font embedding",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.4.1",
        subject: "which font programs shall be embedded",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.4.2",
        subject: "a subset's CharSet and CIDSet",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.5",
        subject: "glyph widths against the embedded program",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.6",
        subject: "TrueType character encodings",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.7",
        subject: "Unicode character maps",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.7.1",
        subject: "who the Unicode map rules bind",
        binding: Binding::Scoping(
            "a Level B file may ignore the whole of section 6.2.11.7, which is `Applies::FromLevel` \
            rather than a row",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.7.2",
        subject: "the ToUnicode CMap",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.7.3",
        subject: "ActualText over private use characters",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.2.11.8",
        subject: "showing the .notdef glyph",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.3",
        subject: "annotations",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.3.1",
        subject: "which annotation subtypes are permitted",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.3.2",
        subject: "an annotation's flags",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.3.3",
        subject: "an annotation's appearances",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.3.4",
        subject: "displaying an annotation's Contents",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.4",
        subject: "interactive forms",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.4.1",
        subject: "form fields, their actions and their appearances",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.4.2",
        subject: "XFA",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.4.3",
        subject: "digital signatures, which Annex B adds to",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.5",
        subject: "actions",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.5.1",
        subject: "which action types are permitted",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.5.2",
        subject: "additional-actions dictionaries",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.5.3",
        subject: "actions that reach outside the file",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.6",
        subject: "metadata",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.6.1",
        subject: "what the metadata requirements are for",
        binding: Binding::StatesNoRequirement(
            "informative: it says what the metadata subclauses are for and points at them",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "6.6.2",
        subject: "metadata streams",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.6.2.1",
        subject: "the catalog's metadata stream and the XMP in it",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.6.2.2",
        subject: "namespace prefixes",
        binding: Binding::StatesNoRequirement(
            "recommendations only: a table of namespace prefixes a file `should` use, and a \
            statement that the URIs need not resolve",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "6.6.2.3",
        subject: "schemas",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.6.2.3.1",
        subject: "which schemas a property may come from",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.6.2.3.2",
        subject: "embedding an extension schema's description",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.6.2.3.3",
        subject: "the extension schema container schema's fields",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.6.3",
        subject: "the document information dictionary",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.6.4",
        subject: "the identification schema",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.6.5",
        subject: "file identifiers",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.6.6",
        subject: "the provenance history",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.7",
        subject: "logical structure",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.7.1",
        subject: "who the logical structure rules bind",
        binding: Binding::Scoping(
            "the whole of section 6.7 binds a Level A file only, which is `Applies::FromLevel` \
            rather than a row; the rest of the subclause is advice to a writer",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "6.7.2",
        subject: "tagged PDF",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.7.2.1",
        subject: "tagged PDF",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.7.2.2",
        subject: "the mark information dictionary",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.7.3",
        subject: "artefacts",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "6.7.3.1",
        subject: "marking artefacts",
        binding: Binding::StatesNoRequirement(
            "recommendations only: pagination, layout and production features `should` be marked as \
            artefacts",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "6.7.3.2",
        subject: "word boundaries inside show strings",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.7.3.3",
        subject: "the structure tree root",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.7.3.4",
        subject: "the role map",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.7.4",
        subject: "natural language identifiers",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.7.5",
        subject: "alternate descriptions",
        binding: Binding::StatesNoRequirement(
            "recommendations only: an alternate description `should` be supplied where content has \
            no textual analogue",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "6.7.6",
        subject: "annotations that display no text",
        binding: Binding::StatesNoRequirement(
            "recommendations only: an annotation that displays no text `should` describe itself in \
            Contents",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "6.7.7",
        subject: "replacement text",
        binding: Binding::StatesNoRequirement(
            "recommendations only: replacement text `should` be supplied for a non-standard \
            representation",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "6.7.8",
        subject: "expansions of abbreviations",
        binding: Binding::StatesNoRequirement(
            "recommendations only: an abbreviation `should` carry its expansion",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "6.8",
        subject: "embedded files",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.9",
        subject: "optional content",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.10",
        subject: "alternate presentations and transitions",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "6.11",
        subject: "the Requirements key",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "A",
        subject: "the method for determining transparency on a page",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "A.1",
        subject: "the method a processor uses to decide whether a page contains transparency",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "A.2",
        subject: "the page content step",
        binding: Binding::Restated(
            "a step of the method the section A.1 row carries: what the graphics state and the \
            element kinds contribute",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "A.3",
        subject: "the form XObject step",
        binding: Binding::Restated(
            "a step of the method the section A.1 row carries: form XObjects",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "A.4",
        subject: "the image XObject step",
        binding: Binding::Restated(
            "a step of the method the section A.1 row carries: image XObjects",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "A.5",
        subject: "the text object step",
        binding: Binding::Restated(
            "a step of the method the section A.1 row carries: Type 3 glyph procedures",
        ),
    },
    Subclause {
        part: Part::Two,
        clause: "B",
        subject: "requirements for digital signatures",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Two,
        clause: "B.1",
        subject: "what signing produces",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Two,
        clause: "B.2",
        subject: "how a signature is validated",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "5",
        subject: "conformance",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "5.1",
        subject: "what a conforming file adheres to",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "5.2",
        subject: "what a conforming processor is",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6",
        subject: "the technical requirements",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1",
        subject: "file structure",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1.1",
        subject: "data neither standard describes",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1.2",
        subject: "the file header",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1.3",
        subject: "the trailer dictionary and the document information dictionary",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1.4",
        subject: "the cross-reference table, and objects no section names",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1.5",
        subject: "hexadecimal strings",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1.6",
        subject: "stream objects",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1.6.1",
        subject: "a stream's stated length and external data",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1.6.2",
        subject: "which filters a stream may use",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1.7",
        subject: "name objects",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1.8",
        subject: "the syntax around an indirect object",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1.9",
        subject: "an inline image's filter",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1.10",
        subject: "linearization",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1.11",
        subject: "the permissions dictionary",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.1.12",
        subject: "the catalog's Version key",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2",
        subject: "graphics",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.1",
        subject: "what a processor draws, and the interface around it",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.2",
        subject: "content stream operators and resources",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.3",
        subject: "the PDF/A output intent, document-level and per page",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.4",
        subject: "colour spaces",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.4.1",
        subject: "colour specified device-independently",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.4.2",
        subject: "ICCBased colour spaces",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.4.3",
        subject: "the three device colour spaces",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.4.4",
        subject: "Separation and DeviceN colour spaces",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.4.5",
        subject: "Indexed and Pattern colour spaces",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.5",
        subject: "the graphics state parameter dictionary",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.6",
        subject: "flatness",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.7",
        subject: "images",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.7.1",
        subject: "an image dictionary's forbidden and constrained entries",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.7.2",
        subject: "thumbnail images",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.7.3",
        subject: "JPEG 2000 data",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.8",
        subject: "XObjects",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.8.1",
        subject: "form XObjects",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.8.2",
        subject: "reference XObjects",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.9",
        subject: "transparency and the blending colour space",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10",
        subject: "fonts",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10.1",
        subject: "what the font requirements apply to",
        binding: Binding::Scoping(
            "the font rules reach every font the file uses, one shown only in text rendering mode 3 \
            included, unless a rule says otherwise; `super::table::fonts` builds its population \
            that way and its module comment says so",
        ),
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10.2",
        subject: "which font types a conforming file may use",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10.3",
        subject: "composite fonts",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10.3.1",
        subject: "a Type 0 font's CIDSystemInfo against its CMap's",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10.3.2",
        subject: "a CIDFont's CIDToGIDMap",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10.3.3",
        subject: "embedded and predefined CMaps",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10.4",
        subject: "font embedding",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10.4.1",
        subject: "which font programs shall be embedded",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10.4.2",
        subject: "subset embedding",
        binding: Binding::StatesNoRequirement(
            "a permission and a note: subsetting is allowed, and this part states none of the \
            CharSet and CIDSet requirements ISO 19005-2 section 6.2.11.4.2 adds",
        ),
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10.5",
        subject: "glyph widths, vertical metrics and Type 3 glyph procedures against the program",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10.6",
        subject: "TrueType character encodings",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10.7",
        subject: "the ToUnicode CMap",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10.8",
        subject: "ActualText and private use characters",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.2.10.9",
        subject: "showing the .notdef glyph",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.3",
        subject: "annotations",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.3.1",
        subject: "which annotation subtypes are permitted, and in which flavour",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.3.2",
        subject: "an annotation's flags",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.3.3",
        subject: "an annotation's appearances",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.3.4",
        subject: "displaying an annotation's Contents",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.4",
        subject: "interactive forms",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.4.1",
        subject: "form fields, their actions, their appearances and a stripped form's data",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.4.2",
        subject: "XFA",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.5",
        subject: "digital signatures",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.5.1",
        subject: "where a signature lives and what signing may not break",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.5.2",
        subject: "the PAdES profiles a signature conforms to",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.5.3",
        subject: "a document timestamp as a proof of existence",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.5.4",
        subject: "validating a signature",
        binding: Binding::StatesNoRequirement(
            "recommendations only: signatures and timestamps `should` be validated, by a procedure \
            the base standard and ISO 14533-3 state",
        ),
    },
    Subclause {
        part: Part::Four,
        clause: "6.6",
        subject: "actions",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.6.1",
        subject: "which action types are permitted, and in which flavour",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.6.2",
        subject: "when an ECMAScript action may run",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.6.3",
        subject: "additional-actions dictionaries",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.6.4",
        subject: "actions that reach outside the file",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.7",
        subject: "metadata",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.7.1",
        subject: "what the metadata requirements are for",
        binding: Binding::StatesNoRequirement(
            "informative: it says what the metadata subclauses are for and points at them",
        ),
    },
    Subclause {
        part: Part::Four,
        clause: "6.7.2",
        subject: "metadata streams",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "6.7.2.1",
        subject: "the catalog's metadata stream and the XMP in it",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.7.2.2",
        subject: "namespace prefixes",
        binding: Binding::StatesNoRequirement(
            "recommendations only: a table of namespace prefixes a file `should` use, and a \
            statement that the URIs need not resolve",
        ),
    },
    Subclause {
        part: Part::Four,
        clause: "6.7.2.3",
        subject: "a metadata stream's associated schema file",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.7.3",
        subject: "the identification schema",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.7.4",
        subject: "file identifiers",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.7.5",
        subject: "the provenance history",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.8",
        subject: "logical structure",
        binding: Binding::StatesNoRequirement(
            "informative: this part has no conformance levels, so its whole account of logical \
            structure is the value of having it, with the PDF/UA family named for the requirements",
        ),
    },
    Subclause {
        part: Part::Four,
        clause: "6.9",
        subject: "embedded files",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.10",
        subject: "optional content",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.11",
        subject: "alternate presentations and transitions",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.12",
        subject: "the Requirements key",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.13",
        subject: "print scaling",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "6.14",
        subject: "geospatial information",
        binding: Binding::StatesNoRequirement(
            "a permission: geospatial information may be carried by any mechanism the base standard \
            describes",
        ),
    },
    Subclause {
        part: Part::Four,
        clause: "6.15",
        subject: "measurement properties",
        binding: Binding::StatesNoRequirement(
            "a permission: measurement properties may be carried by any mechanism the base standard \
            describes",
        ),
    },
    Subclause {
        part: Part::Four,
        clause: "A",
        subject: "the requirements for a PDF/A-4f file",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "A.1",
        subject: "what a PDF/A-4f file is",
        binding: Binding::Scoping(
            "PDF/A-4f is clauses 5 and 6 as this annex modifies them, which is what \
            `Target::Four(Flavour::F)` and the `Applies::Flavours` column are",
        ),
    },
    Subclause {
        part: Part::Four,
        clause: "A.2",
        subject: "the embedded files a PDF/A-4f file carries",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "A.3",
        subject: "declaring PDF/A-4f",
        binding: Binding::Restated(
            "the identification rule of section 6.7.3, which the table carries there",
        ),
    },
    Subclause {
        part: Part::Four,
        clause: "B",
        subject: "the requirements for a PDF/A-4e file",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "B.1",
        subject: "what a PDF/A-4e file is",
        binding: Binding::Scoping(
            "PDF/A-4e is clauses 5 and 6 as this annex modifies them, which is what \
            `Target::Four(Flavour::E)` and the `Applies::Flavours` column are",
        ),
    },
    Subclause {
        part: Part::Four,
        clause: "B.2",
        subject: "3D content",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "B.2.1",
        subject: "which annotation carries 3D artwork, and what a processor displays",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "B.2.2",
        subject: "the format of a 3D stream",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "B.2.3",
        subject: "colour management of 3D artwork",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "B.3",
        subject: "ECMAScript actions in a PDF/A-4e file",
        binding: Binding::Container,
    },
    Subclause {
        part: Part::Four,
        clause: "B.3.1",
        subject: "a processor that renders 3D content and declines its scripts",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "B.3.2",
        subject: "the OnInstantiate script",
        binding: Binding::Bound,
    },
    Subclause {
        part: Part::Four,
        clause: "B.4",
        subject: "the embedded files a PDF/A-4e file may carry",
        binding: Binding::Restated(
            "the embedded file rules of section 6.9, which the table carries there with the any- \
            type relaxation in the `Applies::Flavours` column of the row that would otherwise \
            forbid it",
        ),
    },
    Subclause {
        part: Part::Four,
        clause: "B.5",
        subject: "declaring PDF/A-4e",
        binding: Binding::Restated(
            "the identification rule of section 6.7.3, which the table carries there",
        ),
    },
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{Part, SUBCLAUSES, subclauses};
    use crate::table;

    /// Every clause the table cites for one part, as the rows write it.
    fn cited(part: Part) -> BTreeSet<&'static str> {
        table::requirements()
            .filter_map(|requirement| match part {
                Part::Two => requirement.clauses.two,
                Part::Four => requirement.clauses.four,
            })
            .collect()
    }

    /// A subclause listed twice would let the two entries disagree and neither test below catch it.
    #[test]
    fn each_subclause_is_listed_once() {
        let mut seen = BTreeSet::new();
        for subclause in subclauses() {
            assert!(
                seen.insert((subclause.part, subclause.clause)),
                "{:?} section {} is listed twice",
                subclause.part,
                subclause.clause
            );
        }
    }

    /// The first direction: a subclause this audit calls bound is one some row cites.
    ///
    /// What it catches is a row *leaving* — a requirement deleted or re-keyed to another clause
    /// takes the last citation off a subclause, and this says so by name instead of quietly
    /// shrinking the table.
    #[test]
    fn every_bound_subclause_is_cited_by_a_row() {
        for part in [Part::Two, Part::Four] {
            let cited = cited(part);
            for subclause in subclauses().filter(|s| s.part == part) {
                if subclause.binding.expects_a_row() {
                    assert!(
                        cited.contains(subclause.clause),
                        "{part:?} section {} is audited as bound and no row cites it",
                        subclause.clause
                    );
                }
            }
        }
    }

    /// The second direction: a subclause this audit calls silent is one no row cites.
    ///
    /// What it catches is the audit going stale in the other direction — a later round writing a
    /// row against a subclause somebody once recorded as stating no requirement. Either the row
    /// or the entry is wrong, and the build says so rather than leaving two answers standing.
    #[test]
    fn no_unbound_subclause_is_cited_by_a_row() {
        for part in [Part::Two, Part::Four] {
            let cited = cited(part);
            for subclause in subclauses().filter(|s| s.part == part) {
                if !subclause.binding.expects_a_row() {
                    assert!(
                        !cited.contains(subclause.clause),
                        "{part:?} section {} is cited by a row and audited as {:?}",
                        subclause.clause,
                        subclause.binding
                    );
                }
            }
        }
    }

    /// Every clause a row cites is one this audit has an entry for.
    ///
    /// The completeness half that does not need the standard: a row citing a clause number that
    /// appears nowhere here is either a typo or a subclause nobody audited, and both are worth
    /// failing on.
    #[test]
    fn every_clause_a_row_cites_is_audited() {
        for part in [Part::Two, Part::Four] {
            let audited: BTreeSet<&str> = subclauses()
                .filter(|s| s.part == part)
                .map(|s| s.clause)
                .collect();
            for clause in cited(part) {
                assert!(
                    audited.contains(clause),
                    "a row cites {part:?} section {clause}, which this audit does not list"
                );
            }
        }
    }

    /// The audit lists both parts, and lists them in each part's own order.
    ///
    /// Order is what makes this file reviewable against a copy of the standard — the property the
    /// requirement table states for itself and that nothing enforced there either. It is checked
    /// as a *segmentation* rather than as a sort: clause numbers are not lexicographically
    /// ordered (`6.10` follows `6.9`), so what is asserted is that each part's entries are
    /// contiguous.
    #[test]
    fn each_part_is_one_contiguous_run() {
        let parts: Vec<Part> = SUBCLAUSES.iter().map(|s| s.part).collect();
        let mut runs = 0;
        for (index, part) in parts.iter().enumerate() {
            if index == 0 || parts[index - 1] != *part {
                runs += 1;
            }
        }
        assert_eq!(
            runs, 2,
            "the two parts' entries are not two contiguous runs"
        );
    }

    /// Against the standards themselves, where the machine running this holds them.
    ///
    /// `doc/pdfa/` is licensed to a single reader (`doc/questions/A16`) and is not in the
    /// repository, so this **skips and says so** where the texts are absent — the same shape as
    /// the KIO worker's build test. Where they are present it closes the last direction: every
    /// clause 6 heading of each part is a subclause this audit lists. The annexes are not read
    /// this way, because in the converted text their headings are not headings; they are covered
    /// by the three tests above only.
    ///
    /// Nothing of the standards' text is read, kept or printed — only the numbers of their
    /// headings.
    #[test]
    fn the_audit_lists_every_clause_six_heading_of_each_part() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdfa");
        for (part, file) in [
            (Part::Two, "ISO_19005-2_2011.md"),
            (Part::Four, "ISO_19005-4_2020.md"),
        ] {
            let path = root.join(file);
            let Ok(text) = std::fs::read_to_string(&path) else {
                println!(
                    "skipping {part:?}: {} is not on this machine",
                    path.display()
                );
                continue;
            };
            let audited: BTreeSet<&str> = subclauses()
                .filter(|s| s.part == part)
                .map(|s| s.clause)
                .collect();
            let mut headings = 0_usize;
            for line in text.lines() {
                if !line.starts_with("###") {
                    continue;
                }
                let rest = line.trim_start_matches('#').trim_start();
                let Some((number, _)) = rest.split_once(' ') else {
                    continue;
                };
                if !number.starts_with("6.") {
                    continue;
                }
                headings += 1;
                assert!(
                    audited.contains(number),
                    "{part:?} section {number} is a heading of the standard and this audit does \
                     not list it"
                );
            }
            assert!(
                headings > 40,
                "{part:?}: only {headings} clause 6 headings were found, so the parser is wrong \
                 rather than the audit"
            );
        }
    }
}

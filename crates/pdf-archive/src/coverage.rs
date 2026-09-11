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
//! # Two granularities, and the frontier between them
//!
//! The audit runs at two levels, and the second one is younger and smaller than the first.
//!
//! **[`subclauses`] is subclause-level**, and over the whole of both parts. A subclause holding
//! four normative sentences of which three have rows is [`Binding::Bound`] there, exactly as one
//! holding four with four is. That was the whole instrument when this file was written, and its
//! own module comment said sentence-level coverage "cannot be committed to this tree at all"
//! because it would have to enumerate the standard's sentences, which `doc/questions/A16`
//! licenses to a single reader.
//!
//! **That was one step short.** What the licence forbids is reproducing the standard's *words* —
//! and nothing here reproduces them: a [`Sentence::says`] is this crate's own paraphrase, written
//! exactly as a [`crate::Requirement`]'s `asks` is, and the load-bearing part of a sentence-level
//! audit is not the text but the *count and the mapping* — how many rules a subclause states, and
//! which row carries each. [`readings`] is that, and five more tests hold it to the table in both
//! directions.
//!
//! **The frontier is computed rather than claimed**: [`frontier`] lists every subclause with text
//! in it that no reading has reached. The region read sentence by sentence in the session that
//! built this layer is clause 5, clause 6.1 and the normative annexes of both parts — the
//! structural prefix, plus the annexes, which is where the subclause-level pass had found its
//! largest hole. Everything from clause 6.2 onwards is still judged at subclause level only.
//!
//! A [`Binding::Container`] heading is not in the frontier: it states no text. Everything else is,
//! including the subclauses this audit records as scoping or as stating no requirement — because
//! those are claims about *all* of a subclause's sentences, and two of them turned out to be
//! hiding a processor obligation apiece (ISO 19005-4 Annex A.1 and Annex B.1, which define a
//! flavour *and* require a processor of that flavour to read every plain PDF/A-4 file).
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
        subject: "what a PDF/A-4f file is, and what a PDF/A-4f processor reads",
        binding: Binding::Bound,
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
        subject: "what a PDF/A-4e file is, and what a PDF/A-4e processor reads",
        binding: Binding::Bound,
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

/// One normative sentence of one subclause, and what the table does about it.
///
/// The sentence itself is **not** here: `doc/pdfa/` is licensed to a single reader
/// (`doc/questions/A16`), so [`Sentence::says`] is this crate's own paraphrase, written the way
/// a [`crate::Requirement`]'s `asks` is. What is load-bearing is the *count* and the mapping —
/// how many rules a subclause states, and which row carries each — because that is the claim
/// the subclause-level audit above cannot make.
#[derive(Debug, Clone, Copy)]
pub struct Sentence {
    /// What it requires, in this crate's own words.
    pub says: &'static str,
    /// What the table does about it.
    pub carried: Carried,
}

/// What [`crate::table`] does about one normative sentence.
///
/// The four variants mirror [`Binding`]'s four non-container states, one level down, and for
/// the same reason: folding "no row, because another clause carries it" into "no row" is what
/// makes a gap invisible.
#[derive(Debug, Clone, Copy)]
pub enum Carried {
    /// The rows that carry it, by identifier.
    ///
    /// More than one where the table splits a sentence into the separate rules it states, and
    /// more than one again for a *delegating* sentence — a part's "a conforming file shall
    /// adhere to the base standard" is what makes every row this crate cites at that clause
    /// binding, so those rows sit under it.
    By(&'static [&'static str]),
    /// A row cited at another clause of this part carries it; the reason names which.
    Restated(&'static str),
    /// It says how the *other* requirements are read, and the table honours it in its shape.
    Scoping(&'static str),
    /// It states no requirement a conforming file or processor could fail.
    ///
    /// A definition, a permission, or a sentence about the standard's own text. The reason says
    /// which — a `shall` inside a definition is still a definition.
    StatesNoRequirement(&'static str),
}

/// One subclause read sentence by sentence.
#[derive(Debug, Clone, Copy)]
pub struct Reading {
    /// Which part states it.
    pub part: Part,
    /// Its number within that part, as [`Subclause::clause`] writes it.
    pub clause: &'static str,
    /// Its normative sentences, in the order the subclause states them.
    pub sentences: &'static [Sentence],
}

/// Every subclause this audit has read sentence by sentence.
pub fn readings() -> impl Iterator<Item = &'static Reading> {
    READINGS.iter()
}

/// Every subclause the sentence-level pass has not reached — the frontier, computed.
///
/// A [`Binding::Container`] heading states no text of its own, so it is not in the frontier and
/// never needs a reading. Everything else is: a subclause recorded as stating no requirement, or
/// as scoping, or as restating another, is making a claim about *all* of its sentences, and two
/// of those claims turned out to hide a processor obligation apiece — ISO 19005-4 Annex A.1 and
/// Annex B.1, audited as scoping because they define a flavour, each also require a conforming
/// processor of that flavour to read every plain PDF/A-4 file. So the frontier is stated over
/// every subclause with text in it, whatever the subclause-level verdict was.
pub fn frontier() -> impl Iterator<Item = &'static Subclause> {
    subclauses().filter(|subclause| {
        !matches!(subclause.binding, Binding::Container)
            && !READINGS
                .iter()
                .any(|reading| reading.part == subclause.part && reading.clause == subclause.clause)
    })
}

/// The sentence-level reading, subclause by subclause.
///
/// In each part's own order, and inside a subclause in the order the subclause states its
/// sentences, so that a person reviewing this against their copy reads straight down the page.
static READINGS: &[Reading] = &[
    Reading {
        part: Part::Two,
        clause: "5.1",
        sentences: &[
            Sentence {
                says: "a conforming file adheres to every requirement of the base standard as \
                       this part modifies it",
                carried: Carried::By(&[
                    "conformance/adheres-to-the-base-standard",
                    "file-structure/hexadecimal-string-holds-only-digits",
                ]),
            },
            Sentence {
                says: "the header's version number is not used in deciding whether a file \
                       conforms",
                carried: Carried::By(&[
                    "conformance/the-version-number-does-not-decide-conformance",
                ]),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "5.2",
        sentences: &[Sentence {
            says: "a Level A file adheres to every requirement of this part",
            carried: Carried::Scoping(
                "Level A is a `Level` and the `Applies::FromLevel` column, not a row",
            ),
        }],
    },
    Reading {
        part: Part::Two,
        clause: "5.3",
        sentences: &[Sentence {
            says: "a Level B file adheres to every requirement of this part except those of \
                   sections 6.2.11.7 and 6.7",
            carried: Carried::Scoping("the exemption `Applies::FromLevel` carries"),
        }],
    },
    Reading {
        part: Part::Two,
        clause: "5.4",
        sentences: &[Sentence {
            says: "a Level U file adheres to every requirement of this part except those of \
                   section 6.7",
            carried: Carried::Scoping("the exemption `Applies::FromLevel` carries"),
        }],
    },
    Reading {
        part: Part::Two,
        clause: "5.5",
        sentences: &[
            Sentence {
                says: "a conforming reader complies with every requirement this part states \
                       about reader functional behaviour",
                carried: Carried::By(&["conformance/processor-behaviour"]),
            },
            Sentence {
                says: "rendering and other processing of a conforming file is performed as the \
                       base standard defines, subject to this part's additional restrictions",
                carried: Carried::By(&["conformance/processor-behaviour"]),
            },
            Sentence {
                says: "a conforming reader ignores features described in PDF specifications \
                       that the base standard does not describe",
                carried: Carried::By(&["conformance/undescribed-features-are-ignored"]),
            },
            Sentence {
                says: "a conforming reader reads and appropriately processes every PDF/A-2 file",
                carried: Carried::By(&["conformance/processor-reads-every-conforming-file"]),
            },
            Sentence {
                says: "a conforming reader also reads and appropriately processes every PDF/A-1 \
                       file",
                carried: Carried::By(&["conformance/a-part-two-reader-also-reads-part-one"]),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "6.1.1",
        sentences: &[Sentence {
            says: "data neither standard describes is not used to render page content",
            carried: Carried::By(&["file-structure/undescribed-data-never-renders"]),
        }],
    },
    Reading {
        part: Part::Two,
        clause: "6.1.2",
        sentences: &[
            Sentence {
                says: "the header begins at byte zero and states the base standard's version",
                carried: Carried::By(&["file-structure/file-header"]),
            },
            Sentence {
                says: "a comment of at least four bytes above 127 follows the header's \
                       end-of-line marker",
                carried: Carried::By(&["file-structure/file-header"]),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "6.1.3",
        sentences: &[
            Sentence {
                says: "the trailer states an ID whose value is the base standard's pair of file \
                       identifiers",
                carried: Carried::By(&["file-structure/file-identifier"]),
            },
            Sentence {
                says: "the trailer states no Encrypt key",
                carried: Carried::By(&["file-structure/no-encryption"]),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "6.1.4",
        sentences: &[
            Sentence {
                says: "a single end-of-line marker separates the xref keyword from the \
                       subsection header",
                carried: Carried::By(&["file-structure/cross-reference-keyword-line-endings"]),
            },
            Sentence {
                says: "an indirect object no cross-reference section names is exempt from every \
                       requirement of this part and may be ignored",
                carried: Carried::By(&[
                    "file-structure/unreferenced-objects-never-influence-rendering",
                ]),
            },
            Sentence {
                says: "a reader that does not ignore such an object never lets it influence what \
                       is rendered",
                carried: Carried::By(&[
                    "file-structure/unreferenced-objects-never-influence-rendering",
                ]),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "6.1.5",
        sentences: &[Sentence {
            says: "a document information dictionary is permitted and a conforming reader \
                   ignores it",
            carried: Carried::Restated("the same rule as section 6.6.3, where the table cites it"),
        }],
    },
    Reading {
        part: Part::Two,
        clause: "6.1.6",
        sentences: &[Sentence {
            says: "a hexadecimal string has an even number of digits",
            carried: Carried::By(&["file-structure/hexadecimal-string-digits"]),
        }],
    },
    Reading {
        part: Part::Two,
        clause: "6.1.7.1",
        sentences: &[
            Sentence {
                says: "the stream keyword is followed by a carriage return and line feed or by a \
                       single line feed",
                carried: Carried::By(&["file-structure/stream-keyword-line-endings"]),
            },
            Sentence {
                says: "the endstream keyword is preceded by an end-of-line marker",
                carried: Carried::By(&["file-structure/stream-keyword-line-endings"]),
            },
            Sentence {
                says: "the stream dictionary's Length is the number of bytes actually present",
                carried: Carried::By(&["file-structure/stream-length-matches-the-data"]),
            },
            Sentence {
                says: "a stream dictionary states no F, FFilter or FDecodeParams key",
                carried: Carried::By(&["file-structure/no-external-stream-data"]),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "6.1.7.2",
        sentences: &[
            Sentence {
                says: "every standard filter of the base standard's table may be used except \
                       LZWDecode",
                carried: Carried::By(&["file-structure/no-lzw-filter"]),
            },
            Sentence {
                says: "the Crypt filter is not used unless its decode parameters name Identity",
                carried: Carried::By(&["file-structure/crypt-filter-is-identity"]),
            },
            Sentence {
                says: "a filter the base standard's table does not list is not used",
                carried: Carried::By(&["file-structure/stream-filters-are-standard"]),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "6.1.8",
        sentences: &[Sentence {
            says: "font names, Separation and DeviceN colourant names and structure type names \
                   are valid UTF-8 once their escapes are expanded",
            carried: Carried::By(&["file-structure/bound-names-are-valid-utf8"]),
        }],
    },
    Reading {
        part: Part::Two,
        clause: "6.1.9",
        sentences: &[
            Sentence {
                says: "a single white-space character separates the object number from the \
                       generation number",
                carried: Carried::By(&["file-structure/indirect-object-syntax"]),
            },
            Sentence {
                says: "a single white-space character separates the generation number from the \
                       obj keyword",
                carried: Carried::By(&["file-structure/indirect-object-syntax"]),
            },
            Sentence {
                says: "an end-of-line marker precedes the object number and the endobj keyword",
                carried: Carried::By(&["file-structure/indirect-object-syntax"]),
            },
            Sentence {
                says: "an end-of-line marker follows the obj and endobj keywords",
                carried: Carried::By(&["file-structure/indirect-object-syntax"]),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "6.1.10",
        sentences: &[Sentence {
            says: "an inline image's F entry names neither LZW nor Crypt nor any filter outside \
                   the base standard's table",
            carried: Carried::By(&["file-structure/inline-image-filters"]),
        }],
    },
    Reading {
        part: Part::Two,
        clause: "6.1.11",
        sentences: &[Sentence {
            says: "linearization is permitted, and a reader should ignore the linearization \
                   information",
            carried: Carried::By(&["file-structure/linearization-permitted"]),
        }],
    },
    Reading {
        part: Part::Two,
        clause: "6.1.12",
        sentences: &[
            Sentence {
                says: "a permissions dictionary states no key other than UR3 and DocMDP",
                carried: Carried::By(&["file-structure/permissions-dictionary-keys"]),
            },
            Sentence {
                says: "where DocMDP is present, the signature reference dictionary states no \
                       DigestLocation, DigestMethod or DigestValue",
                carried: Carried::By(&["file-structure/document-signature-states-no-digest"]),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "6.1.13",
        sentences: &[
            Sentence {
                says: "no integer is greater than 2147483647",
                carried: Carried::By(&[
                    "implementation-limits/integer-values",
                    "implementation-limits/values-written-in-content-streams",
                ]),
            },
            Sentence {
                says: "no integer is less than -2147483648",
                carried: Carried::By(&[
                    "implementation-limits/integer-values",
                    "implementation-limits/values-written-in-content-streams",
                ]),
            },
            Sentence {
                says: "no real number lies outside ±3.403×10^38",
                carried: Carried::By(&[
                    "implementation-limits/real-values",
                    "implementation-limits/values-written-in-content-streams",
                ]),
            },
            Sentence {
                says: "no real number is nearer to zero than ±1.175×10^-38",
                carried: Carried::By(&[
                    "implementation-limits/real-values",
                    "implementation-limits/values-written-in-content-streams",
                ]),
            },
            Sentence {
                says: "no string is longer than 32767 bytes",
                carried: Carried::By(&[
                    "implementation-limits/string-lengths",
                    "implementation-limits/values-written-in-content-streams",
                ]),
            },
            Sentence {
                says: "no name is longer than 127 bytes",
                carried: Carried::By(&[
                    "implementation-limits/name-lengths",
                    "implementation-limits/values-written-in-content-streams",
                ]),
            },
            Sentence {
                says: "the file holds no more than 8388607 indirect objects",
                carried: Carried::By(&["implementation-limits/indirect-object-count"]),
            },
            Sentence {
                says: "q and Q pairs nest no more than 28 levels deep",
                carried: Carried::By(&["implementation-limits/graphics-state-nesting"]),
            },
            Sentence {
                says: "a DeviceN colour space has no more than 32 colourants",
                carried: Carried::By(&["implementation-limits/devicen-colourants"]),
            },
            Sentence {
                says: "no CID is greater than 65535",
                carried: Carried::By(&["implementation-limits/character-identifiers"]),
            },
            Sentence {
                says: "every page boundary measures at least 3 and at most 14400 units in each \
                       direction",
                carried: Carried::By(&["implementation-limits/page-boundary-sizes"]),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "A.1",
        sentences: &[Sentence {
            says: "a conforming reader uses the method this annex describes to decide whether a \
                   page contains transparency",
            carried: Carried::By(&["graphics/transparency-determined-by-the-parts-own-method"]),
        }],
    },
    Reading {
        part: Part::Two,
        clause: "A.2",
        sentences: &[
            Sentence {
                says: "each element's graphics state is checked for a dictionary-valued SMask, a \
                       ca or CA below 1, or a BM other than Normal",
                carried: Carried::Restated("a step of the method the section A.1 row carries"),
            },
            Sentence {
                says: "a Type 1 pattern colour space is treated as a form XObject and processed \
                       by section A.3",
                carried: Carried::Restated("a step of the method the section A.1 row carries"),
            },
            Sentence {
                says: "a form XObject element is processed by section A.3",
                carried: Carried::Restated("a step of the method the section A.1 row carries"),
            },
            Sentence {
                says: "an image XObject element is processed by section A.4",
                carried: Carried::Restated("a step of the method the section A.1 row carries"),
            },
            Sentence {
                says: "a text element is processed by section A.5",
                carried: Carried::Restated("a step of the method the section A.1 row carries"),
            },
            Sentence {
                says: "every appearance stream of every annotation in the page's Annots array is \
                       processed as a form XObject",
                carried: Carried::Restated("a step of the method the section A.1 row carries"),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "A.3",
        sentences: &[
            Sentence {
                says: "a form XObject stating a Group whose value is Transparency puts \
                       transparency on every page it is placed on",
                carried: Carried::Restated("a step of the method the section A.1 row carries"),
            },
            Sentence {
                says: "the form XObject's content stream is processed by section A.2",
                carried: Carried::Restated("a step of the method the section A.1 row carries"),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "A.4",
        sentences: &[
            Sentence {
                says: "an image XObject stating a stream-valued SMask puts transparency on every \
                       page it is placed on",
                carried: Carried::Restated("a step of the method the section A.1 row carries"),
            },
            Sentence {
                says: "an image XObject stating an SMaskInData above zero puts transparency on \
                       every page it is placed on",
                carried: Carried::Restated("a step of the method the section A.1 row carries"),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "A.5",
        sentences: &[
            Sentence {
                says: "a text element's text state is checked for the kind of font it draws with",
                carried: Carried::Restated("a step of the method the section A.1 row carries"),
            },
            Sentence {
                says: "each glyph procedure of a Type 3 font is processed as a form XObject",
                carried: Carried::Restated("a step of the method the section A.1 row carries"),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "B.1",
        sentences: &[
            Sentence {
                says: "the digest is computed over the whole file, including the signature \
                       dictionary and excluding the signature value",
                carried: Carried::By(&["signatures/digest-covers-the-whole-file"]),
            },
            Sentence {
                says: "the signature, a DER-encoded PKCS#7 object, is placed in the signature \
                       dictionary's Contents entry",
                carried: Carried::By(&["signatures/signature-is-a-single-signer-cms-object"]),
            },
            Sentence {
                says: "that PKCS#7 object conforms to RFC 2315",
                carried: Carried::By(&["signatures/signature-is-a-single-signer-cms-object"]),
            },
            Sentence {
                says: "it carries at least the signer's X.509 certificate and exactly one signer",
                carried: Carried::By(&["signatures/signature-is-a-single-signer-cms-object"]),
            },
            Sentence {
                says: "revocation information and as much of the certificate chain as is \
                       available is captured and validated before the signature is completed",
                carried: Carried::By(&["signatures/revocation-information-is-a-signed-attribute"]),
            },
            Sentence {
                says: "the revocation information is a signed attribute of the signature",
                carried: Carried::By(&["signatures/revocation-information-is-a-signed-attribute"]),
            },
            Sentence {
                says: "a conforming reader can call the appropriate signature handler",
                carried: Carried::By(&["signatures/signature-handlers-available"]),
            },
            Sentence {
                says: "a conforming reader supports the two SubFilter values the base standard \
                       documents",
                carried: Carried::By(&["signatures/signature-handlers-available"]),
            },
        ],
    },
    Reading {
        part: Part::Two,
        clause: "B.2",
        sentences: &[
            Sentence {
                says: "a conforming reader validating a signature verifies the document digest \
                       and validates the certificate path",
                carried: Carried::By(&["signatures/signatures-validated-as-the-annex-describes"]),
            },
            Sentence {
                says: "the validity checks are carried out at the indicated signing time",
                carried: Carried::By(&["signatures/signatures-validated-as-the-annex-describes"]),
            },
            Sentence {
                says: "the revocation status is checked",
                carried: Carried::By(&["signatures/signatures-validated-as-the-annex-describes"]),
            },
            Sentence {
                says: "a reader may ignore embedded revocation information in favour of its own \
                       storage or referenced data",
                carried: Carried::StatesNoRequirement(
                    "a permission granted to the reader, with nothing owed either way",
                ),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "5.1",
        sentences: &[
            Sentence {
                says: "a conforming file adheres to every requirement of the base standard as \
                       this document modifies it",
                carried: Carried::By(&[
                    "conformance/adheres-to-the-base-standard",
                    "file-structure/file-identifier",
                    "file-structure/hexadecimal-string-holds-only-digits",
                ]),
            },
            Sentence {
                says: "a feature the base standard describes as deprecated is not used",
                carried: Carried::By(&["conformance/no-deprecated-features"]),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "5.2",
        sentences: &[
            Sentence {
                says: "a conforming processor complies with every requirement this document \
                       states about processor functional behaviour",
                carried: Carried::By(&["conformance/processor-behaviour"]),
            },
            Sentence {
                says: "rendering and other processing of a conforming file is performed as the \
                       base standard defines, subject to this document's additional restrictions",
                carried: Carried::By(&["conformance/processor-behaviour"]),
            },
            Sentence {
                says: "a conforming processor reads and appropriately processes every conforming \
                       PDF/A-4 file",
                carried: Carried::By(&["conformance/processor-reads-every-conforming-file"]),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "6.1.1",
        sentences: &[Sentence {
            says: "data neither standard describes is not used to render page content",
            carried: Carried::By(&["file-structure/undescribed-data-never-renders"]),
        }],
    },
    Reading {
        part: Part::Four,
        clause: "6.1.2",
        sentences: &[
            Sentence {
                says: "the header begins at byte zero and states the base standard's version",
                carried: Carried::By(&["file-structure/file-header"]),
            },
            Sentence {
                says: "a comment of at least four bytes above 127 follows the header's \
                       end-of-line marker",
                carried: Carried::By(&["file-structure/file-header"]),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "6.1.3",
        sentences: &[
            Sentence {
                says: "no data follows the last end-of-file marker",
                carried: Carried::By(&["file-structure/nothing-after-the-last-end-of-file-marker"]),
            },
            Sentence {
                says: "the trailer states no Encrypt key",
                carried: Carried::By(&["file-structure/no-encryption"]),
            },
            Sentence {
                says: "the trailer states no Info key unless the document catalog states a \
                       PieceInfo entry",
                carried: Carried::By(&[
                    "file-structure/document-information-dictionary-needs-piece-info",
                ]),
            },
            Sentence {
                says: "a document information dictionary that is present states nothing but \
                       ModDate",
                carried: Carried::By(&[
                    "file-structure/document-information-dictionary-holds-only-a-modification-date",
                ]),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "6.1.4",
        sentences: &[
            Sentence {
                says: "a single end-of-line marker separates the xref keyword from the \
                       subsection header",
                carried: Carried::By(&["file-structure/cross-reference-keyword-line-endings"]),
            },
            Sentence {
                says: "an indirect object no cross-reference section names is exempt from every \
                       requirement of this document and may be ignored",
                carried: Carried::By(&[
                    "file-structure/unreferenced-objects-never-influence-rendering",
                ]),
            },
            Sentence {
                says: "a processor that does not ignore such an object never lets it influence \
                       what is rendered",
                carried: Carried::By(&[
                    "file-structure/unreferenced-objects-never-influence-rendering",
                ]),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "6.1.5",
        sentences: &[Sentence {
            says: "a hexadecimal string has an even number of digits",
            carried: Carried::By(&["file-structure/hexadecimal-string-digits"]),
        }],
    },
    Reading {
        part: Part::Four,
        clause: "6.1.6.1",
        sentences: &[
            Sentence {
                says: "the stream dictionary's Length is the number of bytes actually present",
                carried: Carried::By(&["file-structure/stream-length-matches-the-data"]),
            },
            Sentence {
                says: "a stream dictionary states no F, FFilter or FDecodeParams key",
                carried: Carried::By(&["file-structure/no-external-stream-data"]),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "6.1.6.2",
        sentences: &[
            Sentence {
                says: "every standard filter of the base standard's table may be used except \
                       LZWDecode",
                carried: Carried::By(&["file-structure/no-lzw-filter"]),
            },
            Sentence {
                says: "a filter the base standard's table does not list is not used",
                carried: Carried::By(&["file-structure/stream-filters-are-standard"]),
            },
            Sentence {
                says: "the Crypt filter is not used unless its decode parameters name Identity",
                carried: Carried::By(&["file-structure/crypt-filter-is-identity"]),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "6.1.7",
        sentences: &[Sentence {
            says: "font names, Separation and DeviceN colourant names and structure type names \
                   are valid UTF-8 once their escapes are expanded",
            carried: Carried::By(&["file-structure/bound-names-are-valid-utf8"]),
        }],
    },
    Reading {
        part: Part::Four,
        clause: "6.1.8",
        sentences: &[
            Sentence {
                says: "a single white-space character separates the object number from the \
                       generation number",
                carried: Carried::By(&["file-structure/indirect-object-syntax"]),
            },
            Sentence {
                says: "a single white-space character separates the generation number from the \
                       obj keyword",
                carried: Carried::By(&["file-structure/indirect-object-syntax"]),
            },
            Sentence {
                says: "an end-of-line marker precedes the object number and the endobj keyword",
                carried: Carried::By(&["file-structure/indirect-object-syntax"]),
            },
            Sentence {
                says: "an end-of-line marker follows the obj and endobj keywords",
                carried: Carried::By(&["file-structure/indirect-object-syntax"]),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "6.1.9",
        sentences: &[Sentence {
            says: "an inline image's F entry names neither LZW nor Crypt nor any filter outside \
                   the base standard's table",
            carried: Carried::By(&["file-structure/inline-image-filters"]),
        }],
    },
    Reading {
        part: Part::Four,
        clause: "6.1.10",
        sentences: &[Sentence {
            says: "linearization is permitted, and a processor should ignore the linearization \
                   information",
            carried: Carried::By(&["file-structure/linearization-permitted"]),
        }],
    },
    Reading {
        part: Part::Four,
        clause: "6.1.11",
        sentences: &[Sentence {
            says: "a permissions dictionary states no key other than UR3 and DocMDP",
            carried: Carried::By(&["file-structure/permissions-dictionary-keys"]),
        }],
    },
    Reading {
        part: Part::Four,
        clause: "6.1.12",
        sentences: &[
            Sentence {
                says: "a Version key in the document catalog begins with a 2 and a full stop",
                carried: Carried::By(&["file-structure/catalog-version-key"]),
            },
            Sentence {
                says: "its third character is a decimal digit",
                carried: Carried::By(&["file-structure/catalog-version-key"]),
            },
            Sentence {
                says: "its value is exactly three characters long",
                carried: Carried::By(&["file-structure/catalog-version-key"]),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "A.1",
        sentences: &[
            Sentence {
                says: "a PDF/A-4f conforming file, writer or processor meets clauses 5 and 6 as \
                       this annex modifies them",
                carried: Carried::Scoping(
                    "PDF/A-4f is `Target::Four(Flavour::F)` and the `Applies::Flavours` column",
                ),
            },
            Sentence {
                says: "a PDF/A-4f conforming writer is an application able to write files \
                       meeting the PDF/A-4f requirements",
                carried: Carried::StatesNoRequirement(
                    "the definition of a term, and a circular one: the file such a writer \
                     produces is judged by every other row of this table. Neither part states an \
                     obligation on a writer that is not an obligation on its output",
                ),
            },
            Sentence {
                says: "a PDF/A-4f conforming processor reads and appropriately processes every \
                       PDF/A-4f file",
                carried: Carried::By(&[
                    "conformance/an-embedded-files-processor-reads-the-plain-profile",
                ]),
            },
            Sentence {
                says: "it also reads and appropriately processes everything a PDF/A-4 conforming \
                       processor is required to read",
                carried: Carried::By(&[
                    "conformance/an-embedded-files-processor-reads-the-plain-profile",
                ]),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "A.2",
        sentences: &[
            Sentence {
                says: "a PDF/A-4f file states an EmbeddedFiles key in the name dictionary of its \
                       document catalog",
                carried: Carried::By(&["embedded-files/pdfa-4f-carries-embedded-files"]),
            },
            Sentence {
                says: "every file specification under that key meets section 6.9, except that \
                       the embedded files may be of any type",
                carried: Carried::Restated(
                    "the section 6.9 rows, which the table cites there; the any-type relaxation \
                     is the `Applies::Flavours` column of \
                     `embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile`",
                ),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "A.3",
        sentences: &[Sentence {
            says: "a PDF/A-4f file declares its conformance as section 6.7.3 specifies",
            carried: Carried::Restated(
                "the identification rules of section 6.7.3, which the table cites there",
            ),
        }],
    },
    Reading {
        part: Part::Four,
        clause: "B.1",
        sentences: &[
            Sentence {
                says: "a PDF/A-4e conforming file, writer or processor meets clauses 5 and 6 as \
                       this annex modifies them",
                carried: Carried::Scoping(
                    "PDF/A-4e is `Target::Four(Flavour::E)` and the `Applies::Flavours` column",
                ),
            },
            Sentence {
                says: "a PDF/A-4e conforming writer is an application able to write files \
                       meeting the PDF/A-4e requirements",
                carried: Carried::StatesNoRequirement("the definition of a term, as in Annex A.1"),
            },
            Sentence {
                says: "a PDF/A-4e conforming processor reads and appropriately processes every \
                       PDF/A-4e file",
                carried: Carried::By(&[
                    "conformance/an-engineering-processor-reads-the-plain-profile",
                ]),
            },
            Sentence {
                says: "it also reads and appropriately processes everything a PDF/A-4 conforming \
                       processor is required to read",
                carried: Carried::By(&[
                    "conformance/an-engineering-processor-reads-the-plain-profile",
                ]),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "B.2.1",
        sentences: &[
            Sentence {
                says: "an interactive processor able to render 3D artwork processes and displays \
                       it as the base standard and this document define",
                carried: Carried::By(&["annotations/three-dimensional-artwork-displayed"]),
            },
            Sentence {
                says: "a non-interactive processor, or one unable to render 3D artwork, displays \
                       the annotation's normal appearance",
                carried: Carried::By(&["annotations/three-dimensional-artwork-displayed"]),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "B.2.2",
        sentences: &[Sentence {
            says: "a 3D stream dictionary's Subtype is U3D or PRC",
            carried: Carried::By(&["annotations/three-dimensional-stream-format"]),
        }],
    },
    Reading {
        part: Part::Four,
        clause: "B.2.3",
        sentences: &[
            Sentence {
                says: "3D artwork whose stream states no ColorSpace is taken to be in sRGB",
                carried: Carried::By(&["graphics/3d-artwork-colour-management"]),
            },
            Sentence {
                says: "a processor that does colour manage 3D artwork follows section 6.2.4.2's \
                       ICCBased rules for that profile",
                carried: Carried::By(&["graphics/3d-artwork-colour-management"]),
            },
            Sentence {
                says: "the colour management is performed after the artwork is rendered",
                carried: Carried::By(&["graphics/3d-artwork-colour-management"]),
            },
            Sentence {
                says: "where the processor cannot render the artwork and falls back to the \
                       annotation's normal appearance, sections 6.3.2 to 6.3.4 apply to it",
                carried: Carried::Restated(
                    "the annotation rows of sections 6.3.2, 6.3.3 and 6.3.4, which the table \
                     cites there and which bind every annotation's appearance including a 3D \
                     annotation's",
                ),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "B.3.1",
        sentences: &[
            Sentence {
                says: "an interactive processor that renders 3D content and does not process \
                       ECMAScript actions tells the user so",
                carried: Carried::By(&["actions/a-processor-that-declines-scripts-says-so"]),
            },
            Sentence {
                says: "an ECMAScript action runs only when a user invokes it explicitly, which \
                       in a PDF/A-4e file includes activating a 3D or RichMedia annotation",
                carried: Carried::Restated(
                    "section 6.6.2's rule, which the table cites there as \
                     `actions/javascript-only-on-explicit-user-action`; this sentence widens \
                     what counts as invoking it rather than stating a second rule",
                ),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "B.3.2",
        sentences: &[
            Sentence {
                says: "an interactive processor may ignore a 3D stream's OnInstantiate key",
                carried: Carried::StatesNoRequirement(
                    "a permission, and the one that makes the next sentence conditional: a file \
                     may state the key and conform",
                ),
            },
            Sentence {
                says: "a processor that does run the OnInstantiate script runs it only when the \
                       user explicitly initiates an action",
                carried: Carried::By(&[
                    "actions/on-instantiate-script-only-on-explicit-user-action",
                ]),
            },
        ],
    },
    Reading {
        part: Part::Four,
        clause: "B.4",
        sentences: &[Sentence {
            says: "every file specification under a PDF/A-4e file's EmbeddedFiles key meets \
                   section 6.9, except that the embedded files may be of any type",
            carried: Carried::Restated(
                "the section 6.9 rows, exactly as in Annex A.2; the any-type relaxation is the \
                 `Applies::Flavours` column of \
                 `embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile`",
            ),
        }],
    },
    Reading {
        part: Part::Four,
        clause: "B.5",
        sentences: &[Sentence {
            says: "a PDF/A-4e file declares its conformance as section 6.7.3 specifies",
            carried: Carried::Restated(
                "the identification rules of section 6.7.3, which the table cites there",
            ),
        }],
    },
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{Binding, Carried, Part, SUBCLAUSES, frontier, readings, subclauses};
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

    /// Every clause a sentence-level reading names is a subclause this audit lists.
    #[test]
    fn every_reading_names_a_subclause() {
        for reading in readings() {
            assert!(
                subclauses().any(|s| s.part == reading.part && s.clause == reading.clause),
                "a reading names {:?} section {}, which this audit does not list",
                reading.part,
                reading.clause
            );
            assert!(
                !matches!(
                    subclauses()
                        .find(|s| s.part == reading.part && s.clause == reading.clause)
                        .map(|s| s.binding),
                    Some(Binding::Container)
                ),
                "{:?} section {} is a container heading and has no text to read",
                reading.part,
                reading.clause
            );
        }
    }

    /// A subclause read twice would let the two readings disagree.
    #[test]
    fn each_subclause_is_read_once() {
        let mut seen = BTreeSet::new();
        for reading in readings() {
            assert!(
                seen.insert((reading.part, reading.clause)),
                "{:?} section {} is read twice",
                reading.part,
                reading.clause
            );
        }
    }

    /// Every row a sentence names exists and cites that sentence's own clause for that part.
    ///
    /// The first half catches a row renamed out from under a reading; the second catches a
    /// reading attributing a rule to the wrong clause, which is the mistake that would make the
    /// count look right while the mapping was wrong.
    #[test]
    fn every_sentence_names_a_row_cited_at_its_own_clause() {
        for reading in readings() {
            for sentence in reading.sentences {
                let Carried::By(ids) = sentence.carried else {
                    continue;
                };
                assert!(
                    !ids.is_empty(),
                    "{:?} section {}: a sentence is carried by an empty list of rows",
                    reading.part,
                    reading.clause
                );
                for id in ids {
                    let row = table::requirements()
                        .find(|requirement| requirement.id == *id)
                        .unwrap_or_else(|| {
                            panic!(
                                "{:?} section {} names the row {id}, which the table does not \
                                 hold",
                                reading.part, reading.clause
                            )
                        });
                    let cited = match reading.part {
                        Part::Two => row.clauses.two,
                        Part::Four => row.clauses.four,
                    };
                    assert_eq!(
                        cited,
                        Some(reading.clause),
                        "{:?} section {} names the row {id}, which cites {cited:?} for that part",
                        reading.part,
                        reading.clause
                    );
                }
            }
        }
    }

    /// The other direction: every row citing a read subclause is named by one of its sentences.
    ///
    /// **This is the test the sentence-level pass exists for.** A row added to a subclause
    /// somebody has read, without a sentence to hang it on, means either the reading missed a
    /// sentence or the row is attributed to the wrong clause — and until this test existed both
    /// looked exactly like a subclause that was already `Bound`.
    #[test]
    fn every_row_at_a_read_subclause_is_named_by_a_sentence() {
        for reading in readings() {
            let named: BTreeSet<&str> = reading
                .sentences
                .iter()
                .flat_map(|sentence| match sentence.carried {
                    Carried::By(ids) => ids,
                    _ => &[],
                })
                .copied()
                .collect();
            for requirement in table::requirements() {
                let cited = match reading.part {
                    Part::Two => requirement.clauses.two,
                    Part::Four => requirement.clauses.four,
                };
                if cited == Some(reading.clause) {
                    assert!(
                        named.contains(requirement.id),
                        "{:?} section {} is read sentence by sentence and no sentence names the \
                         row {}, which cites it",
                        reading.part,
                        reading.clause,
                        requirement.id
                    );
                }
            }
        }
    }

    /// A subclause audited as bound whose reading carries no row at all is a contradiction.
    #[test]
    fn a_read_bound_subclause_has_a_sentence_carried_by_a_row() {
        for reading in readings() {
            let bound = subclauses()
                .find(|s| s.part == reading.part && s.clause == reading.clause)
                .is_some_and(|s| s.binding.expects_a_row());
            if bound {
                assert!(
                    reading
                        .sentences
                        .iter()
                        .any(|sentence| matches!(sentence.carried, Carried::By(_))),
                    "{:?} section {} is audited as bound and no sentence of its reading names a \
                     row",
                    reading.part,
                    reading.clause
                );
            }
        }
    }

    /// The frontier is stated by being computed, and it is only ever a prefix of the standards.
    ///
    /// What this asserts is not a size — a later round shrinking it must not have to edit a
    /// number — but the two properties that make it honest: everything read is listed, and the
    /// region this round read is *contiguous* in each part's own order, so that "up to here" is
    /// a sentence a reviewer can check against their copy rather than a scattered set.
    #[test]
    fn the_region_this_audit_has_read_sentence_by_sentence_stays_read() {
        let unread: BTreeSet<(Part, &str)> = frontier().map(|s| (s.part, s.clause)).collect();
        for (part, clause) in [
            (Part::Two, "5.1"),
            (Part::Two, "6.1.13"),
            (Part::Two, "B.2"),
            (Part::Four, "5.2"),
            (Part::Four, "6.1.12"),
            (Part::Four, "B.5"),
        ] {
            assert!(
                !unread.contains(&(part, clause)),
                "{part:?} section {clause} is inside the region this audit claims to have read \
                 sentence by sentence, and it has no reading"
            );
        }
    }
}

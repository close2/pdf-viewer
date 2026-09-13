//! Which subclause of which part ISO 19005 section 6.2.2's exemption can reach, and which not.
//!
//! # The question this answers, and the fall-through that made it necessary
//!
//! [`crate::reach::exemption_narrows`] keeps section 5.1 and the two parts' file-structure
//! carve-outs and narrows **everything else**, with one `else` arm. That is what the two published
//! sentences say — each exempts an unreferenced named resource from *all* requirements of its part
//! bar the carve-out — but the single arm makes three different situations indistinguishable, and
//! only one of them is a narrowing anybody chose:
//!
//! - the subclause's subject **can** be a named resource, or something only one reaches, so the
//!   exemption really does withdraw findings there ([`Reaches::Resource`]);
//! - the subject is the file, a page, an annotation or the catalog, so no finding it produces can
//!   ever name an exempt object and the narrowing is a fall-through that cannot fire
//!   ([`Reaches::NotAResource`]) — meaningless rather than wrong, and worth **stating**;
//! - every requirement of the subclause binds a conforming processor or has been put outside
//!   validation, so there is no finding at all ([`Reaches::NoFileCanFail`]).
//!
//! This module records which of the four each subclause is, in the two parts' own order, with the
//! reason — [`crate::coverage`]'s vocabulary one question further on. The tests below hold it
//! against that audit in both directions, against `exemption_narrows` for the mechanical half, and
//! against [`crate::table`] for [`Reaches::NoFileCanFail`], so that the reading cannot rot: a row promoted
//! from `Check::Processor` to a predicate under a `NoFileCanFail` subclause fails the build by
//! name.
//!
//! # What it is not
//!
//! It is not a measurement. Whether the exemption *did* withdraw a finding from a row is a
//! question about documents, and `cargo run --release -p pdf-archive --example withdrawn` is the
//! command that answers it over a corpus; the reach a clause *permits* is this file's, and the
//! two are the two denominators `CLAUDE.md` names. A `Resource` entry with no witness in any
//! corpus is not an error here — it is the shape `doc/questions/Q62` found twice.
//!
//! # Why the annexes are here
//!
//! Both parts exempt the resource from every requirement of the *document*, and a normative annex
//! is part of the document, so the exemption reaches Annex A and Annex B as it reaches clause 6.
//! `exemption_narrows` reads a clause number that is not digits and dots — `A.2`, `B.2.2` — as
//! outside every carve-out, which is the same answer; ISO 19005-4 Annex A.2's entry below is where
//! that is stated rather than left to a reader of the parser.

use crate::target::Part;

/// What section 6.2.2's exemption can do to one subclause of one part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reaches {
    /// The part's own carve-out, or section 5.1, keeps the subclause out of the narrowing.
    ///
    /// The mechanical variant: [`crate::reach::exemption_narrows`] answers `false` for exactly
    /// these, and the test below holds the two together.
    Kept(&'static str),
    /// The subclause's subject can be a named resource, or an object only one reaches, so the
    /// exemption can withdraw a finding about it.
    Resource(&'static str),
    /// Nothing a resources dictionary names can be this subclause's subject, so the narrowing is
    /// a fall-through that cannot fire.
    ///
    /// **Meaningless rather than wrong.** The clause does say the resource is exempt from this
    /// requirement too; there is simply no object for the exemption to be about, because the
    /// file, a page, an annotation or the catalog is what the requirement judges.
    NotAResource(&'static str),
    /// Every requirement of the subclause binds a conforming processor, or has been placed
    /// outside validation, so no document states a finding here at all.
    ///
    /// Held to [`crate::table`] by the test below: every row citing such a subclause is
    /// [`crate::Check::Processor`] or [`crate::Check::OutsideValidation`].
    NoFileCanFail(&'static str),
}

impl Reaches {
    /// Whether the exemption can withdraw a finding from this subclause on some document.
    #[must_use]
    pub const fn can_withdraw(self) -> bool {
        matches!(self, Self::Resource(_))
    }

    /// Why this subclause has the answer it has.
    #[must_use]
    pub const fn why(self) -> &'static str {
        match self {
            Self::Kept(why)
            | Self::Resource(why)
            | Self::NotAResource(why)
            | Self::NoFileCanFail(why) => why,
        }
    }
}

/// One subclause of one part, and what the exemption can do to it.
#[derive(Debug, Clone, Copy)]
pub struct Subject {
    /// Which part states it.
    pub part: Part,
    /// Its number within that part, as [`crate::coverage::Subclause::clause`] writes it.
    pub clause: &'static str,
    /// What section 6.2.2's exemption can do to it.
    pub reaches: Reaches,
}

/// Every subclause a row of [`crate::table`] cites, with what the exemption can do to it.
///
/// The population is [`crate::coverage`]'s [`crate::coverage::Binding::Bound`] subclauses and
/// nothing else: a subclause no row cites has no finding to withdraw, whatever its subject, and
/// the test below holds the two lists equal, so a subclause becoming bound arrives here or fails
/// the build.
pub fn subjects() -> impl Iterator<Item = &'static Subject> {
    SUBJECTS.iter()
}

/// What the exemption can do to one subclause, by part and clause number.
#[must_use]
pub fn reaches(part: Part, clause: &str) -> Option<Reaches> {
    SUBJECTS
        .iter()
        .find(|subject| subject.part == part && subject.clause == clause)
        .map(|subject| subject.reaches)
}

/// The reading, subclause by subclause, in each part's own order.
static SUBJECTS: &[Subject] = &[
    Subject {
        part: Part::Two,
        clause: "5.1",
        reaches: Reaches::Kept(
            "A010's second sentence is this row's own: an unreferenced resource is exempt from \
             the part and shall still conform to the base standard, which is what section 5.1 \
             requires. Kept for both parts, and the only sentence of this work that could ever \
             add a failure.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "5.5",
        reaches: Reaches::NoFileCanFail(
            "every sentence here is addressed to a conforming reader, so no document states a \
             finding for the exemption to withdraw.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.1.1",
        reaches: Reaches::NoFileCanFail(
            "the sentence binds a conforming processor — data neither standard describes is to be \
             ignored and never used to render a page — so there is nothing here for a file to \
             fail. The clause and the exemption agree in any case: a resource nothing references \
             renders nothing.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.1.2",
        reaches: Reaches::Kept(
            "A010's carve-out, sections 6.1.2 to 6.1.13. The header is the file's own and no \
             resources dictionary names it, so the carve-out and the subject agree about the \
             answer.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.1.3",
        reaches: Reaches::Kept(
            "A010's carve-out. The trailer is the file's rather than any resource's.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.1.4",
        reaches: Reaches::Kept(
            "A010's carve-out. This subclause's own second sentence is a *different* exemption — \
             an indirect object no cross-reference section names — which `Examination::objects` \
             applies to the population every row sees rather than here.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.1.6",
        reaches: Reaches::Kept(
            "A010's carve-out, and the one place the two parts' carve-outs disagree about a \
             result: a hexadecimal string inside an exempt object is still judged under part 2, \
             and is not under part 4, whose published carve-out starts at its own section 6.1.6.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.1.7.1",
        reaches: Reaches::Kept(
            "A010's carve-out. An exempt stream is still a stream: the `Length` it states and the \
             external-data keys it may not state are judged on it.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.1.7.2",
        reaches: Reaches::Kept("A010's carve-out. The filters an exempt stream names are judged."),
    },
    Subject {
        part: Part::Two,
        clause: "6.1.8",
        reaches: Reaches::Kept(
            "A010's carve-out. A name object inside an exempt object is still a name object.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.1.9",
        reaches: Reaches::Kept(
            "A010's carve-out. The syntax around an indirect object is the file's writing of it, \
             which an exemption about rendering does not reach.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.1.10",
        reaches: Reaches::Kept(
            "A010's carve-out. An inline image is written inside a content stream, and the \
             content stream of a form XObject nothing invokes is read for this rule like any \
             other.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.1.11",
        reaches: Reaches::Kept(
            "A010's carve-out — which is why this subclause is `Kept` rather than \
             `NoFileCanFail`, although its only sentence binds a processor. The carve-out is the \
             mechanical answer and it comes first.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.1.12",
        reaches: Reaches::Kept(
            "A010's carve-out. A permissions dictionary is reached from the catalog.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.1.13",
        reaches: Reaches::Kept(
            "A010's carve-out, and the clearest reason it exists: an implementation limit counts \
             what the file contains, and a validator that stopped counting inside an unreferenced \
             resource would report a smaller file than the one on disk.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.1",
        reaches: Reaches::NoFileCanFail(
            "the subclause is about what a conforming processor draws and what interface it may \
             put around the page.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.2",
        reaches: Reaches::Resource(
            "the subclause states the exemption itself. Its other sentences are about content \
             streams, and a form XObject, a tiling pattern or a Type 3 glyph procedure that only \
             an unreferenced name reaches is exempt from them.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.3",
        reaches: Reaches::NotAResource(
            "an output intent is an entry of the catalog's `OutputIntents` array and its profile \
             is reached through that entry; no resources dictionary names either.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.4.1",
        reaches: Reaches::Resource(
            "a colour space is a `/ColorSpace` entry of a resources dictionary, which is exactly \
             what the sentence exempts.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.4.2",
        reaches: Reaches::Resource(
            "an ICCBased space is a `/ColorSpace` entry and its profile stream is reached through \
             it and, in an unreferenced entry's case, through nothing else.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.4.3",
        reaches: Reaches::Resource(
            "device colour is selected by a content stream's operators and by the colour spaces \
             its resources dictionary names.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.4.4",
        reaches: Reaches::Resource(
            "a Separation or DeviceN space is a `/ColorSpace` entry, and its tint transform and \
             alternate space are reached through it.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.4.5",
        reaches: Reaches::Resource(
            "an Indexed or Pattern space is a `/ColorSpace` entry; a pattern is also a `/Pattern` \
             one.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.5",
        reaches: Reaches::Resource(
            "a graphics state parameter dictionary is an `/ExtGState` entry, and the halftone and \
             transfer function this subclause restricts are reached through it.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.6",
        reaches: Reaches::Resource(
            "a rendering intent is set by the `ri` operator inside a content stream and by an \
             `/ExtGState` entry's `/RI`, and both are inside what the exemption reaches.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.7",
        reaches: Reaches::NoFileCanFail(
            "flatness binds a conforming reader, which is to ignore the value it finds.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.8.1",
        reaches: Reaches::Resource(
            "an image is an `/XObject` entry; the inline-image half of the sentence is inside a \
             content stream, which an unreferenced form XObject has.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.8.2",
        reaches: Reaches::NoFileCanFail(
            "the sentence binds a conforming processor, and a thumbnail is a page's `/Thumb` \
             rather than anything a resources dictionary names — two reasons no finding arises \
             here.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.8.3",
        reaches: Reaches::Resource(
            "JPEG 2000 data is the stream of an image XObject, which is an `/XObject` entry.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.9.1",
        reaches: Reaches::Resource("a form XObject is an `/XObject` entry."),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.9.2",
        reaches: Reaches::Resource(
            "a reference XObject is a form XObject, so an `/XObject` entry.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.9.3",
        reaches: Reaches::Resource(
            "a PostScript XObject is an `/XObject` entry, and this is the exemption's plainest \
             case: ISO 32000-1:2008, 8.8.2 states that such a stream has no effect when the \
             document is viewed or printed on a non-PostScript device, so a page that never \
             invokes it draws exactly what it drew. `doc/questions/Q62` is what follows for the \
             converter.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.10",
        reaches: Reaches::Resource(
            "a transparency group's attribute dictionary sits on a form XObject or on a page, and \
             the first of those is an `/XObject` entry.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.11.2",
        reaches: Reaches::Resource("a font dictionary is a `/Font` entry."),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.11.3.1",
        reaches: Reaches::Resource(
            "a Type 0 font and the CIDFont it descends to are reached through the `/Font` entry \
             that names the first.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.11.3.2",
        reaches: Reaches::Resource(
            "a CIDFont's `CIDToGIDMap` stream is reached through the font a `/Font` entry names.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.11.3.3",
        reaches: Reaches::Resource(
            "an embedded CMap stream is reached through the Type 0 font's `/Encoding`.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.11.4.1",
        reaches: Reaches::Resource(
            "a font program is reached through the descriptor of the font a `/Font` entry names.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.11.4.2",
        reaches: Reaches::Resource("a subset's `CharSet` or `CIDSet` is in that descriptor."),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.11.5",
        reaches: Reaches::Resource(
            "the widths are in the font dictionary a `/Font` entry names, and the program they \
             are checked against is reached through it.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.11.6",
        reaches: Reaches::Resource(
            "a TrueType font's `/Encoding` is in the font dictionary a `/Font` entry names, which \
             is the second of `doc/questions/Q62`'s two rewrites.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.11.7.2",
        reaches: Reaches::Resource("a `ToUnicode` CMap is a stream of the font dictionary."),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.11.7.3",
        reaches: Reaches::Resource(
            "`ActualText` is written in a content stream, and an unreferenced form XObject has \
             one.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.2.11.8",
        reaches: Reaches::Resource(
            "the glyph shown is decided by a content stream and the font its resources dictionary \
             names.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.3.1",
        reaches: Reaches::NotAResource(
            "an annotation is reached from a page's `/Annots` array and never from a resources \
             dictionary, so no finding of this subclause can name an exempt object.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.3.2",
        reaches: Reaches::NotAResource(
            "the same: the flags are in an annotation dictionary, which no resources dictionary \
             names.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.3.3",
        reaches: Reaches::NotAResource(
            "an appearance stream is reached through the annotation's `/AP`, so it is never \
             exempt. What is drawn *inside* it is judged under 6.2, where the exemption does \
             reach what that stream's own resources dictionary names and its content does not.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.3.4",
        reaches: Reaches::NoFileCanFail(
            "displaying an annotation's `Contents` is a conforming interactive reader's \
             obligation.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.4.1",
        reaches: Reaches::NotAResource(
            "a form field and the interactive form dictionary are reached from the catalog's \
             `/AcroForm`.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.4.2",
        reaches: Reaches::NotAResource(
            "the `XFA` key is in that same form dictionary and `NeedsRendering` in the catalog.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.4.3",
        reaches: Reaches::NotAResource(
            "a signature field is a widget annotation, reached from a page's `/Annots` and from \
             the form dictionary.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.5.1",
        reaches: Reaches::NotAResource(
            "an action is reached from an annotation, an outline, a field or the catalog, none of \
             which a resources dictionary names.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.5.2",
        reaches: Reaches::NotAResource(
            "an additional-actions dictionary hangs off those same objects.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.5.3",
        reaches: Reaches::NoFileCanFail(
            "the sentence binds a conforming reader asked to follow a link out of the file.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.6.2.1",
        reaches: Reaches::Resource(
            "ISO 32000-1:2008, 14.3.2 lets any stream carry a `/Metadata` entry, so an XMP packet \
             on a form XObject nothing invokes is reachable through that entry and nothing else. \
             The catalog's own packet never is.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.6.2.3.1",
        reaches: Reaches::Resource(
            "the schema rules are asked of every XMP packet in the file, and a packet can hang \
             off an exempt stream.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.6.2.3.2",
        reaches: Reaches::Resource(
            "the same population: an extension schema's description is written inside such a \
             packet, so it is exempt exactly when the stream carrying the packet is.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.6.2.3.3",
        reaches: Reaches::Resource(
            "the same population a third time: the container schema's own fields are described \
             inside the packet, wherever the packet hangs.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.6.3",
        reaches: Reaches::NoFileCanFail(
            "the document information dictionary's rule here binds a conforming reader.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.6.4",
        reaches: Reaches::NotAResource(
            "the identification schema is in the document-level XMP packet, which the catalog \
             names.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.6.5",
        reaches: Reaches::NotAResource("a file identifier is the trailer's `/ID`."),
    },
    Subject {
        part: Part::Two,
        clause: "6.6.6",
        reaches: Reaches::NoFileCanFail(
            "A021 puts the whole of the file-provenance requirement outside validation, so no \
             finding exists here to withdraw.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.7.2.1",
        reaches: Reaches::NotAResource(
            "tagged PDF is the catalog's `/MarkInfo` and the structure tree it is a claim about.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.7.2.2",
        reaches: Reaches::NotAResource("the mark information dictionary is the catalog's."),
    },
    Subject {
        part: Part::Two,
        clause: "6.7.3.2",
        reaches: Reaches::Resource(
            "word boundaries are a property of the strings a content stream shows, and an \
             unreferenced form XObject has a content stream.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.7.3.3",
        reaches: Reaches::NotAResource("the structure tree root is reached from the catalog."),
    },
    Subject {
        part: Part::Two,
        clause: "6.7.3.4",
        reaches: Reaches::NotAResource("the role map is an entry of that root."),
    },
    Subject {
        part: Part::Two,
        clause: "6.7.4",
        reaches: Reaches::Resource(
            "the language identifiers this subclause asks for sit on the catalog, on a structure \
             element and on a **marked-content property list**, and a property list is a \
             `/Properties` entry of a resources dictionary — so one of the three subjects is \
             inside the exemption's reach even though the rows that judge it today report the \
             file and the page.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.8",
        reaches: Reaches::NotAResource(
            "an embedded file is reached from the catalog's name dictionary or from a file \
             attachment annotation.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.9",
        reaches: Reaches::Resource(
            "ISO 32000-1:2008, 8.11.2 makes an optional content group nameable from a content \
             stream through the `/Properties` category, so a group only an unreferenced entry \
             reaches is exempt. The rows here read the catalog's `/OCProperties`, which is never \
             exempt, so the reach is the clause's rather than today's findings'.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.10",
        reaches: Reaches::NotAResource(
            "`AlternatePresentations` is in the name dictionary and `PresSteps` in a page.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "6.11",
        reaches: Reaches::NotAResource("the `Requirements` key is the catalog's."),
    },
    Subject {
        part: Part::Two,
        clause: "A.1",
        reaches: Reaches::NoFileCanFail(
            "the annex states the method a conforming processor uses to decide whether a page \
             contains transparency.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "B.1",
        reaches: Reaches::NotAResource(
            "what signing produces is judged on the signature dictionary a form field names.",
        ),
    },
    Subject {
        part: Part::Two,
        clause: "B.2",
        reaches: Reaches::NoFileCanFail("how a signature is validated is a processor's behaviour."),
    },
    Subject {
        part: Part::Four,
        clause: "5.1",
        reaches: Reaches::Kept(
            "kept for part 4 as for part 2, although no clarification says so: the direction that \
             withdraws nothing, and it costs nothing today because the row at 5.1 is unchecked.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "5.2",
        reaches: Reaches::NoFileCanFail(
            "the subclause defines what a conforming processor is and what it does.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.1.1",
        reaches: Reaches::NoFileCanFail(
            "the sentence binds a conforming processor, as part 2's section 6.1.1 does.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.1.2",
        reaches: Reaches::NotAResource(
            "part 4's published carve-out keeps only its sections 6.1.6 to 6.1.9, so the \
             exemption reaches this subclause where A010 keeps part 2's equivalent. It can \
             withdraw nothing: the header is the file's own.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.1.3",
        reaches: Reaches::NotAResource(
            "outside part 4's carve-out, and about the trailer and the document information \
             dictionary — neither of which any resources dictionary names.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.1.4",
        reaches: Reaches::NotAResource(
            "outside part 4's carve-out, and about the cross-reference table's own bytes.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.1.5",
        reaches: Reaches::Resource(
            "**the asymmetry worth stating.** Part 4's carve-out keeps sections 6.1.6 to 6.1.9, \
             and hexadecimal strings are its section 6.1.5 — so a hexadecimal string written \
             inside an exempt object is outside part 4's requirements and inside part 2's, where \
             A010's range starts at 6.1.2. The rows that judge it report a span of the file \
             rather than an object, so nothing moves today; the asymmetry is the two published \
             texts', not this crate's.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.1.6.1",
        reaches: Reaches::Kept(
            "part 4's own published carve-out, sections 6.1.6 to 6.1.9. An exempt stream is still \
             a stream: the `Length` it states and the external-data keys it may not state are \
             judged on it.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.1.6.2",
        reaches: Reaches::Kept(
            "part 4's own carve-out. The filters an exempt stream names are judged, which is the \
             sentence that keeps `LZWDecode` out of a resource nothing draws with.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.1.7",
        reaches: Reaches::Kept(
            "part 4's own carve-out. A font or colourant name inside an exempt object is still \
             required to be valid UTF-8.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.1.8",
        reaches: Reaches::Kept(
            "part 4's own carve-out. The syntax around an indirect object is how the file is \
             written, which an exemption about rendering does not reach.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.1.9",
        reaches: Reaches::Kept(
            "part 4's own carve-out, which reaches inline image dictionaries here and does not in \
             part 2, where the same rule is section 6.1.10 and A010's range covers it anyway.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.1.10",
        reaches: Reaches::NoFileCanFail(
            "linearization information is to be ignored by a conforming processor.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.1.11",
        reaches: Reaches::NotAResource(
            "outside part 4's carve-out; a permissions dictionary is reached from the catalog.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.1.12",
        reaches: Reaches::NotAResource(
            "outside part 4's carve-out; the `Version` key is the catalog's.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.1",
        reaches: Reaches::NoFileCanFail(
            "what a conforming processor draws, and the interface it may put around it.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.2",
        reaches: Reaches::Resource(
            "the subclause states the exemption itself; its other sentences are about content \
             streams, which a form XObject nothing invokes has.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.3",
        reaches: Reaches::NotAResource(
            "an output intent is an entry of the catalog's or a page's `OutputIntents` array.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.4.1",
        reaches: Reaches::Resource("a colour space is a `/ColorSpace` entry."),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.4.2",
        reaches: Reaches::Resource(
            "an ICCBased space is a `/ColorSpace` entry and its profile is reached through it.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.4.3",
        reaches: Reaches::Resource(
            "device colour is selected by a content stream and by the spaces its resources \
             dictionary names.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.4.4",
        reaches: Reaches::Resource("a Separation or DeviceN space is a `/ColorSpace` entry."),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.4.5",
        reaches: Reaches::Resource(
            "an Indexed or Pattern space is a `/ColorSpace` entry, a pattern also a `/Pattern` \
             one.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.5",
        reaches: Reaches::Resource(
            "a graphics state parameter dictionary is an `/ExtGState` entry.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.6",
        reaches: Reaches::NoFileCanFail(
            "flatness binds a conforming processor, which is to choose its own value.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.7.1",
        reaches: Reaches::Resource(
            "an image is an `/XObject` entry; the inline-image half is inside a content stream.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.7.2",
        reaches: Reaches::NoFileCanFail(
            "a conforming processor is never to render a page from a thumbnail, and a thumbnail \
             is a page's `/Thumb` in any case.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.7.3",
        reaches: Reaches::Resource("JPEG 2000 data is the stream of an image XObject."),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.8.1",
        reaches: Reaches::Resource("a form XObject is an `/XObject` entry."),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.8.2",
        reaches: Reaches::Resource("a reference XObject is a form XObject."),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.9",
        reaches: Reaches::Resource(
            "a transparency group's attribute dictionary sits on a form XObject or on a page, and \
             the blend mode this subclause restricts is in a graphics state parameter dictionary, \
             which is an `/ExtGState` entry.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.10.2",
        reaches: Reaches::Resource("a font dictionary is a `/Font` entry."),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.10.3.1",
        reaches: Reaches::Resource(
            "a Type 0 font and its descendant are reached through the `/Font` entry.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.10.3.2",
        reaches: Reaches::Resource("a CIDFont's `CIDToGIDMap` is reached through the font."),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.10.3.3",
        reaches: Reaches::Resource(
            "an embedded CMap stream is reached through the Type 0 font's `/Encoding`.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.10.4.1",
        reaches: Reaches::Resource("a font program is reached through the font descriptor."),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.10.5",
        reaches: Reaches::Resource(
            "the metrics are in the font dictionary, and the Type 3 glyph procedures this \
             subclause also covers are reached through the font a `/Font` entry names.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.10.6",
        reaches: Reaches::Resource(
            "a TrueType font's `/Encoding` is in the font dictionary — `doc/questions/Q62`'s \
             second rewrite.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.10.7",
        reaches: Reaches::Resource("a `ToUnicode` CMap is a stream of the font dictionary."),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.10.8",
        reaches: Reaches::Resource("`ActualText` is written in a content stream."),
    },
    Subject {
        part: Part::Four,
        clause: "6.2.10.9",
        reaches: Reaches::Resource(
            "the glyph shown is decided by a content stream and the font it names.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.3.1",
        reaches: Reaches::NotAResource("an annotation is reached from a page's `/Annots` array."),
    },
    Subject {
        part: Part::Four,
        clause: "6.3.2",
        reaches: Reaches::NotAResource("the flags are in the annotation dictionary."),
    },
    Subject {
        part: Part::Four,
        clause: "6.3.3",
        reaches: Reaches::NotAResource(
            "an appearance stream is reached through `/AP`; what is drawn inside it is judged \
             under 6.2, where the exemption reaches what that stream's own resources dictionary \
             names.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.3.4",
        reaches: Reaches::NoFileCanFail(
            "displaying `Contents` is a conforming interactive processor's obligation.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.4.1",
        reaches: Reaches::NotAResource(
            "a form field and the interactive form dictionary are the catalog's.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.4.2",
        reaches: Reaches::NotAResource(
            "`XFA` is in the form dictionary and `NeedsRendering` in the catalog.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.5.1",
        reaches: Reaches::NotAResource(
            "a signature field is a widget annotation and its signature dictionary is reached \
             through it.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.5.2",
        reaches: Reaches::NotAResource(
            "a PAdES profile is a property of that signature dictionary.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.5.3",
        reaches: Reaches::NotAResource(
            "a document timestamp is a signature dictionary of the same shape.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.6.1",
        reaches: Reaches::NotAResource(
            "an action is reached from an annotation, an outline, a field or the catalog.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.6.2",
        reaches: Reaches::NoFileCanFail(
            "when an ECMAScript action may run is a conforming processor's behaviour.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.6.3",
        reaches: Reaches::NotAResource(
            "an additional-actions dictionary hangs off an annotation, a field, a page or the \
             catalog.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.6.4",
        reaches: Reaches::NoFileCanFail(
            "handling an action that reaches outside the file is a processor's behaviour.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.7.2.1",
        reaches: Reaches::Resource(
            "any stream may carry a `/Metadata` entry, so an XMP packet on a form XObject nothing \
             invokes is reachable through that entry and nothing else.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.7.2.3",
        reaches: Reaches::Resource(
            "a metadata stream's associated schema file is reached through that stream, so it \
             inherits the same answer.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.7.3",
        reaches: Reaches::NotAResource(
            "the identification schema is in the document-level XMP packet.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.7.4",
        reaches: Reaches::NotAResource("a file identifier is the trailer's `/ID`."),
    },
    Subject {
        part: Part::Four,
        clause: "6.7.5",
        reaches: Reaches::NotAResource(
            "the provenance history is in the document-level XMP packet.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.9",
        reaches: Reaches::NotAResource(
            "an embedded file is reached from the catalog's name dictionary or from a file \
             attachment annotation.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.10",
        reaches: Reaches::Resource(
            "an optional content group is nameable from a content stream through the \
             `/Properties` category, so a group only an unreferenced entry reaches is exempt; the \
             rows here read the catalog's `/OCProperties`, which never is.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.11",
        reaches: Reaches::NotAResource(
            "`AlternatePresentations` is in the name dictionary and `PresSteps` in a page.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "6.12",
        reaches: Reaches::NotAResource("the `Requirements` key is the catalog's."),
    },
    Subject {
        part: Part::Four,
        clause: "6.13",
        reaches: Reaches::NoFileCanFail("print scaling binds a conforming processor."),
    },
    Subject {
        part: Part::Four,
        clause: "A.1",
        reaches: Reaches::NoFileCanFail(
            "the annex says what a PDF/A-4f file is and what a processor of that flavour reads.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "A.2",
        reaches: Reaches::NotAResource(
            "the embedded files a PDF/A-4f file carries are reached from the catalog's name \
             dictionary. **The exemption reaches an annex at all** because both parts exempt a \
             resource from every requirement of the document, and an annex is part of the \
             document — `exemption_narrows` says so for a clause whose number is not digits, and \
             this entry is where that is stated.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "B.1",
        reaches: Reaches::NoFileCanFail(
            "the annex says what a PDF/A-4e file is and what a processor of that flavour reads.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "B.2.1",
        reaches: Reaches::NoFileCanFail(
            "which annotation carries 3D artwork, and what a processor displays.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "B.2.2",
        reaches: Reaches::NotAResource(
            "a 3D stream is reached through the 3D annotation that names it.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "B.2.3",
        reaches: Reaches::NoFileCanFail(
            "colour management of 3D artwork is what the processor does with it.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "B.3.1",
        reaches: Reaches::NoFileCanFail(
            "a processor that renders 3D content and declines its scripts.",
        ),
    },
    Subject {
        part: Part::Four,
        clause: "B.3.2",
        reaches: Reaches::NoFileCanFail(
            "the `OnInstantiate` script is a processor's to run or decline.",
        ),
    },
];

#[cfg(test)]
mod tests {
    use super::{Reaches, SUBJECTS, reaches, subjects};
    use crate::coverage::{Binding, subclauses};
    use crate::reach::exemption_narrows;
    use crate::requirement::{Check, Clauses};
    use crate::table::requirements;
    use crate::target::{Part, Target};

    /// The population is the audit's, in both directions.
    #[test]
    fn every_bound_subclause_has_exactly_one_answer() {
        for subclause in subclauses().filter(|entry| entry.binding == Binding::Bound) {
            let found: Vec<_> = SUBJECTS
                .iter()
                .filter(|subject| {
                    subject.part == subclause.part && subject.clause == subclause.clause
                })
                .collect();
            assert_eq!(
                found.len(),
                1,
                "{:?} section {} is bound and has {} answers here",
                subclause.part,
                subclause.clause,
                found.len()
            );
        }
    }

    /// And nothing here is about a subclause no row cites.
    #[test]
    fn every_answer_is_about_a_bound_subclause() {
        for subject in subjects() {
            assert!(
                subclauses().any(|subclause| subclause.part == subject.part
                    && subclause.clause == subject.clause
                    && subclause.binding == Binding::Bound),
                "{:?} section {} is answered here and is not a bound subclause",
                subject.part,
                subject.clause
            );
        }
    }

    /// The mechanical half: `Kept` is exactly what `exemption_narrows` refuses to narrow.
    ///
    /// This is the test that makes the table an audit rather than an opinion. Every other
    /// variant is a reading of what a subclause is *about*; this one is a claim about the code,
    /// and it is checked against the code for every target of the part.
    #[test]
    fn kept_is_what_the_carve_out_keeps() {
        for subject in subjects() {
            let clauses = match subject.part {
                Part::Two => Clauses::only_two(subject.clause),
                Part::Four => Clauses::only_four(subject.clause),
            };
            for target in Target::ALL {
                if target.part() != subject.part {
                    continue;
                }
                let narrows = exemption_narrows(clauses, target);
                assert_eq!(
                    narrows,
                    !matches!(subject.reaches, Reaches::Kept(_)),
                    "{:?} section {} is recorded as {:?} and `exemption_narrows` says {narrows} \
                     for {target}",
                    subject.part,
                    subject.clause,
                    subject.reaches
                );
            }
        }
    }

    /// `NoFileCanFail` is held to the table: no row citing such a subclause judges a document.
    #[test]
    fn no_file_can_fail_a_subclause_whose_every_row_binds_a_processor() {
        for subject in subjects() {
            if !matches!(subject.reaches, Reaches::NoFileCanFail(_)) {
                continue;
            }
            for requirement in requirements() {
                let cited = match subject.part {
                    Part::Two => requirement.clauses.two,
                    Part::Four => requirement.clauses.four,
                };
                if cited != Some(subject.clause) {
                    continue;
                }
                assert!(
                    matches!(
                        requirement.check,
                        Check::Processor(_) | Check::OutsideValidation(_)
                    ),
                    "{:?} section {} is recorded as one no file can fail, and {} judges a \
                     document there",
                    subject.part,
                    subject.clause,
                    requirement.id
                );
            }
        }
    }

    /// Every reason is a sentence rather than a placeholder.
    #[test]
    fn every_answer_states_a_reason() {
        for subject in subjects() {
            assert!(
                subject.reaches.why().len() > 30,
                "{:?} section {} answers without a reason",
                subject.part,
                subject.clause
            );
        }
    }

    /// The two Q62 rewrites' clauses, pinned by name.
    ///
    /// `doc/questions/Q62` is open and this test does not answer it: it holds the *evidence*
    /// steady, so that a round reading the question finds the same two answers this one measured
    /// — both subclauses are ones the exemption reaches, and that is why the converter's two
    /// rewrites have no document behind them.
    #[test]
    fn both_rewrites_the_question_names_sit_in_subclauses_the_exemption_reaches() {
        for (part, clause) in [
            (Part::Two, "6.2.9.3"),
            (Part::Two, "6.2.11.6"),
            (Part::Four, "6.2.10.6"),
        ] {
            let found = reaches(part, clause).expect("the subclause is in the audit");
            assert!(
                found.can_withdraw(),
                "{part:?} section {clause} is recorded as {found:?}"
            );
        }
    }
}

//! ISO 32000-2 §12.8.2.2.2's second step: the signed revision of a file, beside the current one.
//!
//! # The clause, and the two steps it is
//!
//! > To validate a signature that uses the DocMDP transform method, a PDF processor first shall
//! > verify the byte range digest. Next, it shall verify that any modifications that have been
//! > made to the document are permitted by the transform parameters.
//!
//! [`crate::signature::Signature::integrity`] is the first step and has been since ADR 0215.
//! This module is the second: it reconstructs the two states the clause names, says object by
//! object how they differ, and ranks each difference against Table 257's `/P` — [`Kind`] is what
//! a change was taken to be and [`Verdict`] is what the level says about it.
//!
//! # Why a comparison is possible at all without mutating anything
//!
//! §12.8.2.2.2 says what the digest buys:
//!
//! > Once the byte range digest is validated, the portion of the document specified by the
//! > ByteRange entry in the signature dictionary (see "Table 255 -Entries in a signature
//! > dictionary") is known to correspond to the state of the document at the time of signing.
//! > Therefore, PDF processors may compare the signed and current versions of the document to see
//! > whether there have been modifications to any objects that are not permitted by the transform
//! > parameters.
//!
//! and §7.5.6 says where that portion stops: "changes shall be appended to the end of the file,
//! leaving its original contents intact", with "[e]ach trailer shall be terminated by its own
//! end-of-file (%%EOF) marker". So the signed version of the document is a **prefix** of the
//! file the current version stands in, and reading it needs no edit of anything — only a second
//! [`Document`] opened over [`FileBytes::prefix`]. `CLAUDE.md`'s immutable `Document` is
//! therefore not an obstacle here but the thing that makes the comparison exact: both states are
//! functions of the same bytes.
//!
//! # Nothing here says "valid", and nothing says "permitted" by default
//!
//! A `/DocMDP` answer that says permitted without having compared is worse than one that refuses,
//! because the whole point of the transform is to be believed. Two rules follow and both are
//! load-bearing:
//!
//! - **Every way this comparison can fail to be a comparison is a named [`NotComparable`]**
//!   rather than a lenient default, and every object whose change cannot be ranked is named in
//!   [`Ranking::unrankable`] rather than counted as benign. [`Judgement`] has three answers where
//!   something changed — inside what the level permits, not inside it, or not ranked — and no
//!   fourth.
//! - **No answer here is a verdict on a signature.** §12.8.2.2.2 makes the byte range digest step
//!   one and this step two, and §12.8.1's third question has no trust store behind it in this
//!   program (ADR 1039). [`Judgement::WithinWhatIsPermitted`] is a statement about objects.
//!
//! ADRs 1043 and 1049.

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::xref::{self, Location, XrefTable};
use pdf_syntax::{Dictionary, Document, Object, ObjectId, SyntaxError};

use crate::signature::{Coverage, Excluded, FieldMdp, Modification, Signature, SignedEnd};

/// Most object numbers a [`Objects`] names before it stops naming and only counts.
///
/// **The standard states no bound on how many objects a revision may change** — §7.5.4's
/// ten-digit offset bounds the *file*, not the table — so the population this walks comes out of
/// the file and a report that named all of it would be the file's length in a diagnostic. The
/// count is always exact; what is bounded is the naming, which is for a person to read
/// (trap 38).
const MAX_NAMED: usize = 64;

/// An object number that is not inside §7.5.7's object stream, for [`placement`]'s key.
///
/// `u32::MAX` is a legal object number and this is an index *within* a stream, which Table 18's
/// type 2 entries bound by the stream's own `/N`; a stream with 2³²−1 objects in it does not
/// exist and could not be addressed if it did.
const NOT_IN_A_STREAM: u32 = u32::MAX;

/// The signed and current versions of one document, and how their objects differ.
///
/// Built by [`Comparison::of`], which is the only way to get one: a caller cannot pair a signed
/// revision with a document it did not come from.
#[derive(Debug)]
pub struct Comparison {
    /// The document as of the signed revision, opened from the prefix the signature signed.
    signed: Document,
    /// Where the signed revision ends, in bytes from the start of the file.
    end: u64,
    /// How many cross-reference sections stand at or after [`Self::end`].
    updates_after: usize,
    /// How the two states' objects differ.
    changes: Changes,
    /// Every changed object number, ascending and unabridged.
    ///
    /// [`Changes`] bounds its *naming* at [`MAX_NAMED`] and [`Ranking`] bounds its detail; this is
    /// the population both are views of, because a selection has to be applied to all of it. It is
    /// bounded by the file's own object count, as [`compare`]'s own set already is.
    changed: BTreeSet<u32>,
    /// Which form field each object of the interactive form belongs to.
    ///
    /// Over the whole form rather than over the changed objects alone, because Table 256's
    /// `/Data` may name a field that did not itself change: the entry says which object the
    /// analysis runs *on*, and answering that needs the tree whether or not the tree moved. The
    /// walk is bounded by [`MAX_FORM_FIELDS`].
    subjects: BTreeMap<u32, Subject>,
    /// The objects Table 256's `/Data` may name to scope the analysis to every field there is.
    ///
    /// Three of them, and the first is the one a real producer writes. Table 15's `/Root` is
    /// "[t]he catalog dictionary for the PDF file", so a `/Data` naming it puts the whole document
    /// under the analysis and therefore every field in it — which is what
    /// `xfa_filled_imm1344e.pdf`'s `FieldMDP` states, the corpus's only one. Table 29's
    /// `/AcroForm` — the catalog's entry for "[t]he document's interactive form" — and Table 224's
    /// `/Fields`, "[a]n array of references to the document's root fields (those with no ancestors
    /// in the field hierarchy)", are the two narrower objects that reach the same population.
    form: BTreeSet<u32>,
    /// What each changed object is, counted exactly and named up to [`MAX_NAMED`] per kind.
    ///
    /// Keyed by [`Kind`] and by the disposition of the update carrying it, because both are
    /// decided once — [`Ranking`] only turns them into a verdict, and it does that per level
    /// rather than per object.
    tally: BTreeMap<(Kind, Disposition), Objects>,
}

/// Why two states of a file could not be compared.
///
/// Every one of these is a refusal, and the reason it is named rather than folded into a
/// "changed" answer is the module comment's: a transform method exists to be believed, and the
/// conditions under which this reader cannot answer are exactly the conditions a forger would
/// like it to guess in.
#[derive(Debug, thiserror::Error)]
pub enum NotComparable {
    /// The `/ByteRange` does not name bytes of this file, so there is no prefix to open.
    #[error("the signature's byte range does not describe this file")]
    RangeNotInThisFile,
    /// The `/ByteRange` names bytes of this file and the file on disk would not give them.
    #[error("the file on disk did not give the bytes the signature's byte range names")]
    RangeNotReadable,
    /// The signed bytes do not stop where §12.8.1 says a signed range stops.
    ///
    /// The range ends in the middle of a revision, so the prefix is not a state the document was
    /// ever in: a reader opening it would see a cross-reference section the signer's own file
    /// continued past. [`Signature::signed_end`] is the check.
    #[error("the signed range stops at {0:?} rather than at an end-of-file marker")]
    NotARevisionBoundary(SignedEnd),
    /// Something besides the signature value sits inside the signed range's hole.
    ///
    /// The prefix would be a *document with unsigned bytes in the middle of it*, and comparing it
    /// with the current file would give a forger the one answer worth having: two states that
    /// agree, over bytes only one of them was hashed over. [`Signature::excluded`] is the check.
    #[error("the signed range leaves out {0:?} rather than the signature value alone")]
    MoreThanTheValueIsUnsigned(Excluded),
    /// The prefix could not be held.
    #[error("the signed revision's bytes could not be held: {0}")]
    PrefixNotHeld(#[source] std::io::Error),
    /// The prefix did not open as a document.
    #[error("the signed revision did not open as a document: {0}")]
    PrefixNotADocument(#[source] SyntaxError),
    /// One of the two states' cross-reference tables was rebuilt by scanning for objects.
    ///
    /// A scanned table is this reader's synthesis and not the file's statement, so an object it
    /// places is placed on the evidence of a header rather than of a cross-reference section.
    /// Comparing two tables where either was recovered would compare one file's word with
    /// another's guess.
    #[error("the {which} document's cross-reference table was recovered by scanning")]
    RecoveredByScan {
        /// Which of the two — `"signed"` or `"current"`.
        which: &'static str,
    },
}

/// Object numbers, named up to [`MAX_NAMED`] of them and counted in full.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Objects {
    named: Vec<u32>,
    count: u64,
}

impl Objects {
    /// How many there are.
    #[must_use]
    pub fn count(&self) -> u64 {
        self.count
    }

    /// The first [`MAX_NAMED`] of them, ascending.
    #[must_use]
    pub fn named(&self) -> &[u32] {
        &self.named
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Folds another set in, keeping the count exact and the naming bounded.
    fn absorb(&mut self, other: &Self) {
        self.count = self.count.saturating_add(other.count);
        for &number in &other.named {
            if self.named.len() < MAX_NAMED {
                self.named.push(number);
            }
        }
    }

    /// Puts the names back in ascending order after a fold across several kinds.
    fn sort_named(&mut self) {
        self.named.sort_unstable();
    }

    /// Records one, keeping the count exact and the naming bounded.
    fn push(&mut self, number: u32) {
        self.count = self.count.saturating_add(1);
        if self.named.len() < MAX_NAMED {
            self.named.push(number);
        }
    }
}

/// How the objects of two states of a file differ.
///
/// **Object identity is by cross-reference placement, not by parsing.** Two states agree about an
/// object when both tables place it at the same byte offset — or at the same index of the same
/// object stream — *and* that offset lies inside the signed prefix. The digest is what makes that
/// sound: §12.8.2.2.2's own sentence is that the signed portion "is known to correspond to the
/// state of the document at the time of signing", so bytes below the boundary cannot have moved
/// and two identical offsets name identical bytes. Reparsing both objects to compare them would
/// answer the same question at the cost of reading the file twice over.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Changes {
    /// Object numbers the current file defines and the signed revision did not.
    pub added: Objects,
    /// Object numbers both define, in different places.
    pub redefined: Objects,
    /// Object numbers the signed revision defined and the current file does not.
    ///
    /// §7.5.6's deletion: "[d]eleted objects shall be left unchanged in the PDF file, but shall
    /// be marked as deleted by means of their cross-reference entries."
    pub removed: Objects,
    /// Object numbers neither state could be placed in, so nothing is claimed about them.
    ///
    /// A refusal held per object rather than for the comparison as a whole: an entry naming a
    /// containing stream the table does not place, or an offset outside the revision that
    /// declared it. §7.5.7 forbids the nesting that would otherwise need a walk: an object stream
    /// "is a stream object in which a sequence of indirect objects may be stored", and the first
    /// entry of that clause's list of what "shall not be stored in an object stream" is "[s]tream
    /// objects". So one level of resolution is the whole of it, and anything deeper is counted
    /// here rather than followed.
    pub unplaceable: Objects,
    /// Whether the trailer's `/Root` names a different object than it did.
    ///
    /// The one entry of the trailer this compares, because it is the one whose change moves the
    /// whole document: Table 15 makes `/Root` "[t]he catalog dictionary for the PDF file". The
    /// rest of the trailer is §7.5.6's required restatement of the previous one, so comparing it
    /// wholesale would report a conforming update as a change.
    pub catalog_moved: bool,
}

impl Changes {
    /// Whether the two states define the same objects in the same places, under the same catalog.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.redefined.is_empty()
            && self.removed.is_empty()
            && self.unplaceable.is_empty()
            && !self.catalog_moved
    }
}

/// What a changed object **is**, in the vocabulary Table 257 ranks changes in.
///
/// Table 257 names what each level permits — "filling in forms, instantiating page templates,
/// and signing" for 2, and those "as well as annotation creation, deletion, and modification"
/// for 3 — so ranking a change means first deciding which of those, if any, it was. Anything
/// this reader cannot place is [`Kind::Unclassified`] and is **never** ranked as permitted; the
/// rule that governs the whole module is that a lenient default is worse than a refusal.
///
/// **Table 257's third permitted operation is [`Kind::TemplateInstantiated`]**, and what tells it
/// apart from any other page added is §12.7.7's own sentence about where a template lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// The update wrote the object again and it says exactly what it said.
    ///
    /// §12.8.2.2.2's step two is about "modifications that have been made to the document", and
    /// an object restated without change is a modification to the *file* and not to the document
    /// — which is why even level 1 permits it, where level 1 permits nothing else. The test is
    /// over the whole object, stream data included, because a dictionary that matches over bytes
    /// that do not is precisely the lenient default this module exists to refuse.
    RestatedUnchanged,
    /// §7.5.8's cross-reference stream: the update's own table, rather than anything it says.
    ///
    /// Not a change to the document at any level, and the standard is what settles that rather
    /// than convenience. §7.5.6 gives every incremental update a cross-reference section of its
    /// own, §7.5.8 lets that section *be* an object, and Table 257 disregards an update that
    /// carries only DSS data — which such an update could not be if its own table counted. So a
    /// level that permits any change at all permits the table that records it.
    CrossReferenceStream,
    /// A field dictionary whose changed entries are the ones filling one in writes.
    ///
    /// Table 257's "filling in forms". Decided by *which keys differ*, not by the object's type:
    /// a widget annotation is an annotation whatever else it is, and level 2 permits filling it
    /// in while forbidding annotating it, so the two have to be told apart by what the update
    /// actually wrote. [`FILLING_WRITES`] is that set and says where each key comes from.
    FieldFilledIn,
    /// A signature applied — Table 257's "signing".
    ///
    /// A `/FT /Sig` field that gained a value, or a Table 255 signature dictionary the update
    /// added. §12.7.5.5 makes the first the shape signing takes: a signature "shall be the value
    /// of a signature field".
    Signing,
    /// A page instantiated from a template — Table 257's "instantiating page templates".
    ///
    /// §12.7.7 is what makes this decidable, and it decides it in two sentences. The first says
    /// where a template is:
    ///
    /// > If the page is not intended to be displayed by the PDF processor, it shall be referenced
    /// > from the name dictionary's Templates tree instead.
    ///
    /// and the second says what instantiating one produces:
    ///
    /// > A script executed by an ECMAScript action can add the named page to the current document
    /// > as a regular page.
    ///
    /// So the operation takes a page the *signed* revision already held out of reach of the page
    /// tree and puts a regular page into it — and the added page's content is that template's,
    /// because it **is** the named page. [`instantiated`] is that test, over the content the two
    /// pages carry rather than over the reference by which either names it: §12.7.7 states the
    /// identity of the page and says nothing about whether an implementation shares the content
    /// stream or copies it, so a reader that insisted on a shared object would be ranking a
    /// mechanic the clause never fixed.
    ///
    /// **A page tree node carries the same kind, and only under the narrowest condition.** Adding
    /// a page to the tree rewrites the `/Pages` node that receives it, and that rewrite is a
    /// change to the document like any other; it is part of *this* operation only where the node
    /// changed in Table 30's `/Kids` and `/Count` alone, gained nothing but instantiated pages,
    /// and lost nothing. Anything else about the page tree is [`Kind::Unclassified`] and refused.
    TemplateInstantiated,
    /// An appearance stream belonging to a field this update filled in.
    ///
    /// Filling a field is not optional about this. Table 166's `/AP`: "Every annotation
    /// (including those whose Subtype value is Widget , as used for form fields), except for the
    /// two cases listed below, shall have at least one appearance dictionary." So an appearance
    /// stream added beside a filled field is part of the fill, and a level that permits the one
    /// permits the other.
    FieldAppearance,
    /// An annotation created, deleted or modified — the whole of what level 3 adds to level 2.
    Annotation,
    /// An appearance stream belonging to an annotation this update created, changed or removed.
    AnnotationAppearance,
    /// §12.8.4's validation material, or §12.8.5's document timestamp.
    ///
    /// What Table 257 carves out of the question entirely, *when a whole update holds nothing
    /// else*: "[c]hanges to a PDF that are incremental updates which include only the data
    /// necessary to add DSS's … and/or document timestamps … to the document shall not be
    /// considered as changes to the document as defined in the choices below." The carve-out is
    /// therefore a judgement about a revision and not about an object, which is what
    /// [`Disposition`] carries.
    ValidationMaterial,
    /// Table 15's `/Root` names a different object than the signed revision's trailer did.
    CatalogReplaced,
    /// Neither state places the object, so nothing is known about it — not even that it changed.
    Unplaceable,
    /// None of the above. A change, named, and ranked as permitted by no level.
    Unclassified,
}

/// The entries filling in a field writes, and the only ones it writes.
///
/// - `V`, Table 226: "The field's value, whose format varies depending on the field type."
/// - `AP`, Table 166: the appearance dictionary every widget "shall have at least one" of, which
///   §12.7.4.3 makes the processor's to construct for a variable text field.
/// - `AS`, Table 166: "The annotation's appearance state , which selects the applicable
///   appearance stream from an appearance subdictionary" — how a checkbox's new value shows.
/// - `M`, Table 166: "The date and time when the annotation was most recently modified."
const FILLING_WRITES: [&str; 4] = ["V", "AP", "AS", "M"];

/// Table 226's entries that *define* a field rather than record its value.
///
/// A field whose entries beyond [`FILLING_WRITES`] changed is not a filled field; if any of
/// these moved it is not an annotation modification either, because what changed is the form.
/// `FT`, `T`, `Ff`, `Kids`, `Parent`, `DV` and `AA` are Table 226's own rows.
const FIELD_DEFINING: [&str; 7] = ["FT", "T", "Ff", "Kids", "Parent", "DV", "AA"];

/// How far a `/Parent` chain is followed looking for Table 226's inheritable `/FT`.
///
/// §12.7.4.1 states no bound — "[a]n interactive PDF processor shall not limit the range of
/// inheritance for field dictionaries" — so this is a guard against a cycle rather than a
/// reading of the clause, and a chain longer than this yields no field type rather than a wrong
/// one, which sends the object to [`Kind::Unclassified`].
const MAX_FIELD_ANCESTRY: usize = 64;

/// How many pages of the signed revision's `/Templates` tree are held for comparison.
///
/// §12.7.7 states no bound on how many pages a document may name, so the population comes out of
/// the file and this is a guard rather than a reading (trap 38). A document naming more than this
/// has pages beyond the cut compared against nothing, which sends a page instantiated from one of
/// them to [`Kind::Unclassified`] — a refusal, which is the direction this module always takes
/// when it runs out of what it can establish.
const MAX_TEMPLATES: usize = 256;

/// How many bytes of one page's content are held for that comparison.
///
/// The same reasoning: §7.3.8's stream length is the file's to state. A template whose content
/// exceeds this is not compared, so the page instantiated from it is refused rather than ranked.
const MAX_TEMPLATE_CONTENT: usize = 1 << 20;

/// How deep §7.9.6's name tree is followed looking for `/Templates`.
///
/// §7.9.6 names the three kinds of node a tree is built of and states no bound on how many levels
/// they may be stacked in, so this is a guard against a cycle rather than a reading of the clause
/// (trap 38): a balanced tree over any population a file can hold is far shallower.
const MAX_NAME_TREE_DEPTH: usize = 64;

/// How many objects of one interactive form are recorded for §12.8.2.4's transform to select on.
///
/// §12.7.3 states no bound — "[a] PDF document may contain any number of fields appearing on any
/// combination of pages" — so the population comes out of the file (trap 38). A form larger than
/// this leaves its remaining objects unrecorded, which makes a changed one of them
/// [`Included::Excluded`] from the analysis rather than silently covered.
const MAX_FORM_FIELDS: usize = 8192;

/// How deep §12.7.4.1's `/Kids` hierarchy is walked building those names.
///
/// The mirror of [`MAX_FIELD_ANCESTRY`], which climbs the same chain from the other end, and for
/// the same reason: "[a]n interactive PDF processor shall not limit the range of inheritance for
/// field dictionaries", so this is a guard against a cycle and a field deeper than it is named by
/// nothing rather than named wrongly.
const MAX_FORM_DEPTH: usize = 32;

/// What one update after the signed revision is, as far as Table 257's carve-out is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Disposition {
    /// The update carries no validation material, so the carve-out does not arise.
    Ordinary,
    /// The update carries validation material and nothing else, so Table 257 disregards it.
    Disregarded,
    /// The update carries validation material **and** objects that are not that.
    ///
    /// The carve-out can then be neither applied nor denied: the clause's condition is that an
    /// update "include only the data necessary" for a DSS or a document timestamp, and an update
    /// this reader cannot read that way is one it has not judged. §12.8.5.2's document timestamp
    /// is the standing reason — it "shall be determined by examining signature fields", so a real
    /// one touches the interactive form as well as the store, and this reader does not follow it
    /// that far. Every object of such an update is [`Verdict::Unrankable`].
    Mixed,
}

/// What one level of Table 257 says about one changed object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Table 257 does not count this as a change to the document at all.
    Disregarded,
    /// This level permits the change.
    Permitted,
    /// This level does not permit it: "other changes shall invalidate the signature".
    NotPermitted,
    /// Not ranked — and therefore **not** permitted either.
    Unrankable,
}

/// One changed object, what it is, and what a level says about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ranked {
    /// The object number.
    pub number: u32,
    /// What the object is.
    pub kind: Kind,
    /// What the level says about it.
    pub verdict: Verdict,
}

/// Table 257's ranking of one comparison's changes against one level.
///
/// Every changed object lands in exactly one of the four buckets, and the counts are exact —
/// the naming is bounded by [`MAX_NAMED`] per bucket the way [`Objects`] is everywhere else.
/// **Nothing is dropped**: an object this reader could not place appears in
/// [`Ranking::unrankable`] rather than being left out of the arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranking {
    /// The level ranked against.
    pub level: Modification,
    /// Objects in updates Table 257 says are not changes to the document.
    pub disregarded: Objects,
    /// Objects this level permits to have changed.
    pub permitted: Objects,
    /// Objects this level does not permit to have changed.
    pub not_permitted: Objects,
    /// Objects that were not ranked, and are therefore not permitted by default either.
    pub unrankable: Objects,
    /// The first [`MAX_NAMED`] objects in full, ascending — for a person to read.
    pub detail: Vec<Ranked>,
}

impl Ranking {
    /// The one-line answer: three possibilities where something changed, and no fourth.
    ///
    /// The order is the conservative one. A single object this level forbids settles the
    /// question whatever else is unranked — "other changes shall invalidate the signature" needs
    /// one change, not all of them. Only when nothing is forbidden does an unranked object
    /// decide, and then it decides against answering rather than for permitting.
    #[must_use]
    pub fn judgement(&self) -> Judgement {
        if !self.not_permitted.is_empty() {
            return Judgement::NotPermitted {
                level: self.level,
                objects: self.not_permitted.count(),
            };
        }
        if !self.unrankable.is_empty() {
            return Judgement::NotClassified {
                level: self.level,
                objects: self.unrankable.count(),
            };
        }
        if self.permitted.is_empty() && self.disregarded.is_empty() {
            return Judgement::NoChangeToRank;
        }
        Judgement::WithinWhatIsPermitted {
            level: self.level,
            objects: self.permitted.count(),
            disregarded: self.disregarded.count(),
        }
    }
}

/// What Table 257's `/P` says about a set of changes.
///
/// Three answers where something changed, and the fourth answer this deliberately does not have
/// is the point: **nothing here says a signature is valid.** §12.8.2.2.2 makes the byte range
/// digest step one and this step two, and §12.8.1's third question — whether the signer is
/// trusted — has no trust store behind it in this program at all (ADR 1039). What a caller gets
/// is what this step alone can support: the change is inside what the level permits, it is not,
/// or it could not be ranked and the objects that could not be are named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judgement {
    /// There is no change to rank: the current file's objects are the signed revision's objects,
    /// at the same places, under the same catalog.
    ///
    /// **Not a verdict on the signature**, for the reason above.
    NoChangeToRank,
    /// Every change is one this level permits.
    ///
    /// Table 257's own vocabulary, object by object — not an absence of evidence. A single
    /// object this reader could not place would have made this [`Judgement::NotClassified`].
    WithinWhatIsPermitted {
        /// The level the change was ranked against.
        level: Modification,
        /// How many objects were ranked as permitted.
        objects: u64,
        /// How many were in updates Table 257 does not count as changes at all.
        disregarded: u64,
    },
    /// At least one change is one this level does not permit.
    ///
    /// "[O]ther changes shall invalidate the signature" for levels 2 and 3; for level 1, "any
    /// change to the document shall invalidate the signature".
    NotPermitted {
        /// The level the change was ranked against.
        level: Modification,
        /// How many objects this level does not permit to have changed.
        objects: u64,
    },
    /// **Refused.** Nothing is forbidden and something was not ranked, so no answer is given.
    ///
    /// [`Ranking::unrankable`] names them. The standing reasons: an object neither state can
    /// place, a change that is none of Table 257's operations, a replaced catalog, a `/P` outside
    /// 1 to 3, and an update carrying validation material beside objects that are not that.
    NotClassified {
        /// The level the change was ranked against.
        level: Modification,
        /// How many objects could not be ranked.
        objects: u64,
    },
}

/// Why §12.8.2.4's analysis has no subject to be performed on.
///
/// §12.8.1's Table 256 is where `/Data` is, and it says what the analysis runs over:
///
/// > (Required when TransformMethod is FieldMDP, shall be an indirect reference) An indirect
/// > reference to the object in the document upon which the object modification analysis should
/// > be performed.
///
/// A transform that states none, or states one naming something this reader cannot resolve to a
/// part of the interactive form, has not scoped anything — and a ranking computed over a subject
/// nobody chose would be this module's standing failure, an answer arrived at by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NotScoped {
    /// The transform states no `/Data`, which Table 256 requires of this method alone.
    #[error("the FieldMDP transform states no /Data, which Table 256 requires of it")]
    NoData,
    /// `/Data` names an object that is neither the interactive form nor a field in it.
    #[error(
        "the FieldMDP transform's /Data names object {0}, which is not this document's \
         interactive form nor a field in it"
    )]
    DataIsNotTheForm(u32),
}

/// Whether §12.8.2.4's transform includes one changed object, and on what footing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Included {
    /// The object belongs to a form field Table 259's selection names.
    ///
    /// §12.8.2.4's own consequence: "any modifications to specific form fields shall invalidate
    /// that recipient's signature".
    Covered,
    /// The object belongs to a form field the selection does not name, so the transform permits
    /// the change: `/Include` names "[o]nly those form fields specified in Fields".
    Outside,
    /// `/Data` does not scope the analysis to this object, so this transform says nothing about
    /// it — §12.8.2.1's "excluded in revision comparison", and the word is the clause's.
    ///
    /// **Not a permission.** Table 257's levels rank a change §12.8.2.4 excludes;
    /// [`Comparison::rank`] is where that answer lives, and the two transforms are asked
    /// separately because the standard states them separately.
    Excluded,
    /// The object is in the interactive form and no chain of `/T` entries names its field.
    ///
    /// §12.7.4.2 builds a fully qualified name "from the partial field names of the field and all
    /// of its ancestors", so a field tree that names none of them states nothing a `/Fields` array
    /// can match — and matching it against nothing would be a lenient default.
    Undecided,
}

/// One changed object, the field it belongs to, and what §12.8.2.4's transform says about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedTo {
    /// The object number.
    pub number: u32,
    /// §12.7.4.2's fully qualified name of the field it belongs to, where it has one.
    pub field: Option<String>,
    /// Whether the transform includes it.
    pub included: Included,
}

/// §12.8.2.4's transform applied to one comparison's changes, object by object.
///
/// The four buckets are [`Included`]'s four answers, counted exactly and named up to
/// [`MAX_NAMED`] apiece, the way [`Ranking`] holds Table 257's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldRanking {
    /// Objects belonging to a field the transform covers.
    pub covered: Objects,
    /// Objects belonging to a field it does not cover.
    pub outside: Objects,
    /// Objects the transform's `/Data` does not scope the analysis to.
    pub excluded: Objects,
    /// Objects in the form whose field this reader could not name.
    pub undecided: Objects,
    /// The first [`MAX_NAMED`] in full, ascending — for a person to read.
    pub detail: Vec<ScopedTo>,
}

impl FieldRanking {
    /// The one-line answer, on [`Ranking::judgement`]'s conservative ordering.
    #[must_use]
    pub fn judgement(&self) -> FieldJudgement {
        if !self.covered.is_empty() {
            return FieldJudgement::CoveredFieldChanged {
                objects: self.covered.count(),
            };
        }
        if !self.undecided.is_empty() {
            return FieldJudgement::NotClassified {
                objects: self.undecided.count(),
            };
        }
        if self.outside.is_empty() && self.excluded.is_empty() {
            return FieldJudgement::NoChangeToRank;
        }
        FieldJudgement::NoCoveredFieldChanged {
            outside: self.outside.count(),
            excluded: self.excluded.count(),
        }
    }
}

/// What §12.8.2.4's transform says about a set of changes.
///
/// **Not a verdict on a signature**, for [`Judgement`]'s reason and one of its own — §12.8.2.4:
///
/// > FieldMDP signatures shall be validated in a similar manner to DocMDP signatures.
///
/// and §12.8.2.2.2 makes the byte range digest the step before this one. Nothing here reaches the
/// word
/// *valid*, which [`crate::verdict::Valid`] holds and only an anchored proof constructs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldJudgement {
    /// Nothing in the transform's scope changed, and nothing outside it did either.
    NoChangeToRank,
    /// A field the transform covers changed.
    ///
    /// §12.8.2.4: "any modifications to specific form fields shall invalidate that recipient's
    /// signature".
    CoveredFieldChanged {
        /// How many objects belonging to a covered field changed.
        objects: u64,
    },
    /// Something changed and none of it was a field this transform covers.
    NoCoveredFieldChanged {
        /// How many changed objects belong to a field outside the selection.
        outside: u64,
        /// How many the `/Data` scope excludes from this analysis altogether.
        excluded: u64,
    },
    /// **Refused.** Nothing covered changed and something in the form could not be named.
    NotClassified {
        /// How many objects could not be attributed to a field.
        objects: u64,
    },
}

impl Comparison {
    /// The state of `current` when `signature` was made, beside `current` itself.
    ///
    /// The structural preconditions are checked first and each failure is its own
    /// [`NotComparable`], because a prefix that is not a revision is not a state this document
    /// was ever in and comparing against one would produce an answer with no meaning.
    ///
    /// **This does not verify anything.** §12.8.2.2.2 puts the byte range digest first and this
    /// second; a caller runs [`Signature::integrity`] itself and decides what to do with the
    /// answer. Pairing the two here would hide the ordering the clause states.
    ///
    /// # Errors
    ///
    /// [`NotComparable`], one variant per way the two states cannot be put beside each other.
    pub fn of(signature: &Signature, current: &Document) -> Result<Self, NotComparable> {
        let file = current.bytes();
        let length = u64::try_from(file.len()).unwrap_or(u64::MAX);
        match signature.coverage(length) {
            Coverage::WholeFile | Coverage::Unsigned { .. } => {}
            Coverage::Malformed => return Err(NotComparable::RangeNotInThisFile),
        }
        match signature.excluded(file) {
            Excluded::TheSignatureValue | Excluded::TheDigitsOfTheSignatureValue => {}
            Excluded::RangeNotInThisFile => return Err(NotComparable::RangeNotInThisFile),
            Excluded::RangeNotReadable => return Err(NotComparable::RangeNotReadable),
            other => return Err(NotComparable::MoreThanTheValueIsUnsigned(other)),
        }
        match signature.signed_end(file) {
            SignedEnd::AtAnEndOfFileMarker => {}
            SignedEnd::RangeNotInThisFile => return Err(NotComparable::RangeNotInThisFile),
            SignedEnd::RangeNotReadable => return Err(NotComparable::RangeNotReadable),
            other @ SignedEnd::Elsewhere => {
                return Err(NotComparable::NotARevisionBoundary(other));
            }
        }
        if current.was_recovered() {
            return Err(NotComparable::RecoveredByScan { which: "current" });
        }
        let Some(&(start, size)) = signature.byte_range.last() else {
            return Err(NotComparable::RangeNotInThisFile);
        };
        let end = start.saturating_add(size);
        let Ok(boundary) = usize::try_from(end) else {
            return Err(NotComparable::RangeNotInThisFile);
        };
        let prefix = file
            .prefix(boundary)
            .map_err(NotComparable::PrefixNotHeld)?;
        let signed = Document::open_with_limits(prefix, current.limits())
            .map_err(NotComparable::PrefixNotADocument)?;
        if signed.was_recovered() {
            return Err(NotComparable::RecoveredByScan { which: "signed" });
        }
        let tables: BTreeSet<usize> = xref::sections(file, current.limits())
            .iter()
            .map(|section| section.offset)
            .filter(|&offset| u64::try_from(offset).unwrap_or(u64::MAX) >= end)
            .collect();
        let updates_after = tables.len();
        let (changes, touched) = compare(&signed, current, end);
        let tally = classify(&signed, current, &touched, changes.catalog_moved, &tables);
        let mut moved: BTreeSet<u32> = touched.iter().map(|&(number, _)| number).collect();
        if changes.catalog_moved
            && let Some(Object::Reference(root)) = current.xref().trailer().get("Root")
        {
            moved.insert(root.number);
        }
        let mut subjects = BTreeMap::new();
        // The current state first and the signed one after it, so that a field both states hold is
        // named as it is named now and a field the update *removed* is still named at all.
        field_subjects(current, &mut subjects);
        field_subjects(&signed, &mut subjects);
        let form = form_objects(current);
        Ok(Self {
            signed,
            end,
            updates_after,
            changes,
            tally,
            changed: moved,
            subjects,
            form,
        })
    }

    /// The document as it stood when the signature was made.
    #[must_use]
    pub fn signed_revision(&self) -> &Document {
        &self.signed
    }

    /// Where the signed revision ends, in bytes from the start of the file.
    #[must_use]
    pub fn end(&self) -> u64 {
        self.end
    }

    /// How many cross-reference sections the file states at or after [`Self::end`].
    ///
    /// §7.5.6 gives every incremental update one, so this is the number of revisions appended
    /// after signing. Table 257 needs it before any object is ranked: an update that "include[s]
    /// only the data necessary to add DSS's … and/or document timestamps … shall not be
    /// considered as changes to the document", which is a question asked of a whole update.
    #[must_use]
    pub fn updates_after(&self) -> usize {
        self.updates_after
    }

    /// How the two states' objects differ.
    #[must_use]
    pub fn changes(&self) -> &Changes {
        &self.changes
    }

    /// Table 257's ranking of those differences against `level`, object by object.
    ///
    /// Every changed object is in exactly one of [`Ranking`]'s four buckets and the counts are
    /// exact, which is the whole of what makes the answer worth having: an object this reader
    /// could not place is in `unrankable` and not quietly absent.
    #[must_use]
    pub fn rank(&self, level: Modification) -> Ranking {
        let mut ranking = Ranking {
            level,
            disregarded: Objects::default(),
            permitted: Objects::default(),
            not_permitted: Objects::default(),
            unrankable: Objects::default(),
            detail: Vec::new(),
        };
        for (&(kind, disposition), objects) in &self.tally {
            let verdict = verdict(level, kind, disposition);
            match verdict {
                Verdict::Disregarded => ranking.disregarded.absorb(objects),
                Verdict::Permitted => ranking.permitted.absorb(objects),
                Verdict::NotPermitted => ranking.not_permitted.absorb(objects),
                Verdict::Unrankable => ranking.unrankable.absorb(objects),
            }
            for &number in objects.named() {
                ranking.detail.push(Ranked {
                    number,
                    kind,
                    verdict,
                });
            }
        }
        ranking.disregarded.sort_named();
        ranking.permitted.sort_named();
        ranking.not_permitted.sort_named();
        ranking.unrankable.sort_named();
        ranking.detail.sort_unstable_by_key(|ranked| ranked.number);
        ranking.detail.truncate(MAX_NAMED);
        ranking
    }

    /// The one-line answer [`Self::rank`] comes to.
    #[must_use]
    pub fn against(&self, level: Modification) -> Judgement {
        self.rank(level).judgement()
    }

    /// §12.8.2.4's transform applied to these changes: which of them it includes, and which
    /// covered field moved.
    ///
    /// This is §12.8.2.1's sentence executed rather than quoted — "[t]ransform methods, along with
    /// transform parameters, shall determine which objects are included and excluded in revision
    /// comparison" — and it is the one transform whose parameters name a subject the file chooses.
    /// §12.8.2.4's Table 259 says which fields:
    ///
    /// > (Required) A name that, along with the Fields array, describes which form fields do not
    /// > permit changes after the signature is applied.
    ///
    /// and Table 256's `/Data` says which object the analysis runs on. Both are read: a field the
    /// selection names is [`Included::Covered`], one it does not is [`Included::Outside`], and an
    /// object the `/Data` scope does not reach is [`Included::Excluded`] — which is a statement
    /// about *this* transform and not a permission, because [`Self::rank`] is where Table 257
    /// ranks the same object.
    ///
    /// `/Fields` is matched against §12.7.4.2's fully qualified name, which Errata Collection 3's
    /// Issue #33 writes into the entry itself; [`crate::signature::FieldSelection::covers`] carries that reading.
    ///
    /// # Errors
    ///
    /// [`NotScoped`], where Table 256's `/Data` names no subject this reader can resolve.
    pub fn against_field_mdp(&self, transform: &FieldMdp) -> Result<FieldRanking, NotScoped> {
        let Some(data) = transform.data else {
            return Err(NotScoped::NoData);
        };
        let whole_form = self.form.contains(&data.number);
        if !whole_form && !self.names_a_field(data.number) {
            return Err(NotScoped::DataIsNotTheForm(data.number));
        }
        let mut ranking = FieldRanking {
            covered: Objects::default(),
            outside: Objects::default(),
            excluded: Objects::default(),
            undecided: Objects::default(),
            detail: Vec::new(),
        };
        for &number in &self.changed {
            let (field, included) = match self.subjects.get(&number) {
                None => (None, Included::Excluded),
                Some(Subject::Unnamed) => (None, Included::Undecided),
                Some(Subject::Field { name, under }) => {
                    if whole_form || under.contains(&data.number) {
                        let included = if transform.selection.covers(name) {
                            Included::Covered
                        } else {
                            Included::Outside
                        };
                        (Some(name.clone()), included)
                    } else {
                        (Some(name.clone()), Included::Excluded)
                    }
                }
            };
            match included {
                Included::Covered => ranking.covered.push(number),
                Included::Outside => ranking.outside.push(number),
                Included::Excluded => ranking.excluded.push(number),
                Included::Undecided => ranking.undecided.push(number),
            }
            if ranking.detail.len() < MAX_NAMED {
                ranking.detail.push(ScopedTo {
                    number,
                    field,
                    included,
                });
            }
        }
        Ok(ranking)
    }

    /// The one-line answer [`Self::against_field_mdp`] comes to.
    ///
    /// # Errors
    ///
    /// [`NotScoped`], for the same reason.
    pub fn against_transform(&self, transform: &FieldMdp) -> Result<FieldJudgement, NotScoped> {
        Ok(self.against_field_mdp(transform)?.judgement())
    }

    /// Whether `number` is a field dictionary in the interactive form.
    ///
    /// [`Subject::Field::under`] begins with the object itself, so a `/Data` naming a field is one
    /// the walk recorded as its own first ancestor.
    fn names_a_field(&self, number: u32) -> bool {
        self.subjects.values().any(|subject| match subject {
            Subject::Field { under, .. } => under.first() == Some(&number),
            Subject::Unnamed => false,
        })
    }
}

/// Where an object's bytes are, as a key two tables can be compared on.
///
/// `(offset, index)`: the offset of the object's own `N G obj` header, or of the object stream
/// holding it, and the index within that stream — [`NOT_IN_A_STREAM`] where there is none.
/// `None` where the table does not place the object at all.
fn placement(xref: &XrefTable, number: u32) -> Option<(usize, u32)> {
    match xref.location(number)? {
        Location::Offset(at) => Some((at, NOT_IN_A_STREAM)),
        Location::InStream { stream, index } => match xref.location(stream)? {
            Location::Offset(at) => Some((at, index)),
            // §7.5.7 excludes "[s]tream objects" from an object stream, and an object stream is
            // one. A file stating the nesting anyway is not followed further.
            Location::InStream { .. } => None,
        },
    }
}

/// [`Changes`] between two states of one file, `end` being where the earlier one stops.
///
/// The second return is every changed object number with the bucket it landed in, unabridged —
/// [`Changes`] bounds its *naming* at [`MAX_NAMED`] and [`classify`] has to see all of them. It
/// is bounded by the file's own object count, as the set of the signed revision's numbers below
/// already is.
fn compare(signed: &Document, current: &Document, end: u64) -> (Changes, Vec<(u32, Bucket)>) {
    let before = signed.xref();
    let after = current.xref();
    let mut changes = Changes {
        catalog_moved: !same_reference(before.trailer().get("Root"), after.trailer().get("Root")),
        ..Changes::default()
    };
    let mut touched = Vec::new();
    let in_the_signed_revision: BTreeSet<u32> = before.object_numbers().collect();
    for number in after.object_numbers() {
        let here = placement(after, number);
        if !in_the_signed_revision.contains(&number) {
            changes.added.push(number);
            touched.push((number, Bucket::Added));
            continue;
        }
        let there = placement(before, number);
        match (there, here) {
            (Some(there), Some(here)) if there == here => {
                // The two tables agree, and the digest is what makes agreement mean identity:
                // bytes below the boundary "correspond to the state of the document at the time
                // of signing" (§12.8.2.2.2). An offset at or past the boundary is a table
                // pointing outside the revision that declared it, which is nothing this can say.
                if u64::try_from(there.0).unwrap_or(u64::MAX) >= end {
                    changes.unplaceable.push(number);
                    touched.push((number, Bucket::Unplaceable));
                }
            }
            (Some(_), Some(_)) => {
                changes.redefined.push(number);
                touched.push((number, Bucket::Redefined));
            }
            _ => {
                changes.unplaceable.push(number);
                touched.push((number, Bucket::Unplaceable));
            }
        }
    }
    for number in before.object_numbers() {
        if after.location(number).is_none() {
            changes.removed.push(number);
            touched.push((number, Bucket::Removed));
        }
    }
    (changes, touched)
}

/// Whether two trailer entries name the same object.
///
/// Compared as references rather than as resolved dictionaries. Table 15 makes `/Root`
/// "( Required; shall be an indirect reference ) The catalog dictionary for the PDF file", so what
/// this asks is whether the *entry* moved; a catalog whose own contents changed is a redefined
/// object and is counted as one.
fn same_reference(before: Option<&Object>, after: Option<&Object>) -> bool {
    match (before, after) {
        (Some(Object::Reference(before)), Some(Object::Reference(after))) => before == after,
        (None, None) => true,
        _ => false,
    }
}

/// Which of [`Changes`]'s buckets an object landed in, which decides where it is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bucket {
    /// The current file defines it and the signed revision did not.
    Added,
    /// Both define it, in different places.
    Redefined,
    /// The signed revision defined it and the current file does not.
    Removed,
    /// Neither state can be trusted to place it.
    Unplaceable,
}

/// What every changed object is, tallied by [`Kind`] and by its update's [`Disposition`].
///
/// Two passes, because an appearance stream is identified by its *owner* and the owner's object
/// number may be higher than its own: the first decides every object on its own evidence, the
/// second raises the ones an owner claimed. Both read objects through [`Document::get`], which
/// caches, so the second pass re-parses nothing.
fn classify(
    signed: &Document,
    current: &Document,
    touched: &[(u32, Bucket)],
    catalog_moved: bool,
    tables: &BTreeSet<usize>,
) -> BTreeMap<(Kind, Disposition), Objects> {
    let material = validation_material(current);
    // Read out of the **signed** revision, which is the whole of what makes the answer mean
    // anything: a template the update itself added is not one the signer put out of the page
    // tree's reach, so instantiating "it" would be a page composed after signing.
    let templates = templates(signed);
    let mut kinds: BTreeMap<u32, Kind> = BTreeMap::new();
    let mut of_a_field: BTreeSet<u32> = BTreeSet::new();
    let mut of_an_annotation: BTreeSet<u32> = BTreeSet::new();
    let mut of_a_template: BTreeSet<u32> = BTreeSet::new();

    for &(number, bucket) in touched {
        if bucket == Bucket::Unplaceable {
            kinds.insert(number, Kind::Unplaceable);
            continue;
        }
        let held = if bucket == Bucket::Removed {
            signed
        } else {
            current
        };
        let offset = placement(current.xref(), number).map(|(at, _)| at);
        let kind = classify_one(
            signed, current, number, bucket, &material, tables, offset, &templates,
        );
        if let Some(dict) = dictionary_of(&held.get(ObjectId::new(number, 0))) {
            let owned = appearances(held, &dict);
            match kind {
                Kind::FieldFilledIn | Kind::Signing => of_a_field.extend(owned),
                Kind::Annotation => of_an_annotation.extend(owned),
                // The page's content is the template's, and §12.7.7 does not say whether an
                // implementation shares the stream object or copies it. Where it copied, the copy
                // is an object the signed revision never held and carries no `/Type` of its own —
                // so it is claimed here by the page it draws, on the evidence that its bytes were
                // already what [`instantiated`] matched.
                Kind::TemplateInstantiated => of_a_template.extend(content_streams(held, &dict)),
                _ => {}
            }
        }
        kinds.insert(number, kind);
    }

    for (number, kind) in &mut kinds {
        if *kind != Kind::Unclassified {
            continue;
        }
        if of_a_field.contains(number) {
            *kind = Kind::FieldAppearance;
        } else if of_an_annotation.contains(number) {
            *kind = Kind::AnnotationAppearance;
        } else if of_a_template.contains(number) {
            *kind = Kind::TemplateInstantiated;
        }
    }

    // A third pass, because a page tree node is decided by what its *kids* turned out to be and a
    // kid's object number may be either side of the node's. §12.7.7's operation puts a page into
    // the tree, so the node that received it changed as part of the same instantiation.
    let instantiated: BTreeSet<u32> = kinds
        .iter()
        .filter(|(_, kind)| **kind == Kind::TemplateInstantiated)
        .map(|(number, _)| *number)
        .collect();
    let extended: Vec<u32> = kinds
        .iter()
        .filter(|(number, kind)| {
            **kind == Kind::Unclassified
                && extended_by_instantiation(signed, current, **number, &instantiated)
        })
        .map(|(number, _)| *number)
        .collect();
    for number in extended {
        kinds.insert(number, Kind::TemplateInstantiated);
    }

    if catalog_moved {
        // The object the *current* trailer points at, which is what a reader now opens. A direct
        // `/Root` names no object at all, and 0 is the free list's head and can be no catalog, so
        // it stands for "the trailer said it" without pretending to be an object number.
        let root = match current.xref().trailer().get("Root") {
            Some(Object::Reference(id)) => id.number,
            _ => 0,
        };
        // An update may both add the new catalog and point at it, in which case this replaces
        // the object's own classification rather than standing beside it — and replacing it is
        // the conservative direction, because no level ranks a replaced catalog as permitted.
        kinds.insert(root, Kind::CatalogReplaced);
    }

    let dispositions = dispositions(current, &kinds, tables);
    let mut tally: BTreeMap<(Kind, Disposition), Objects> = BTreeMap::new();
    for (number, kind) in &kinds {
        let disposition = update_of(current, *number, tables)
            .and_then(|update| dispositions.get(&update).copied())
            .unwrap_or(Disposition::Ordinary);
        tally.entry((*kind, disposition)).or_default().push(*number);
    }
    tally
}

/// Table 257's carve-out, asked of each update rather than of each object.
///
/// §12.8.2.2.2's Table 257, in the `/P` row and above its three values:
///
/// > Changes to a PDF that are incremental updates which include only the data necessary to add
/// > DSS's 12.8.4.3, "Document Security Store (DSS)" and/or document timestamps 12.8.5,
/// > "Document timestamp (DTS) dictionary" to the document shall not be considered as changes to
/// > the document as defined in the choices below.
///
/// "[O]nly" is the whole of the test, so an update carrying anything besides validation material
/// and its own cross-reference stream fails it — and failing it is not the same as being
/// forbidden, which is why such an update becomes [`Disposition::Mixed`] rather than a verdict.
fn dispositions(
    current: &Document,
    kinds: &BTreeMap<u32, Kind>,
    tables: &BTreeSet<usize>,
) -> BTreeMap<usize, Disposition> {
    let mut seen: BTreeMap<usize, (bool, bool)> = BTreeMap::new();
    for (number, kind) in kinds {
        let Some(update) = update_of(current, *number, tables) else {
            continue;
        };
        let entry = seen.entry(update).or_insert((false, false));
        match kind {
            Kind::ValidationMaterial => entry.0 = true,
            Kind::CrossReferenceStream => {}
            _ => entry.1 = true,
        }
    }
    seen.into_iter()
        .map(|(update, (material, other))| {
            let disposition = match (material, other) {
                (false, _) => Disposition::Ordinary,
                (true, false) => Disposition::Disregarded,
                (true, true) => Disposition::Mixed,
            };
            (update, disposition)
        })
        .collect()
}

/// Which update after the signed revision defines an object, named by that update's own section.
///
/// §7.5.6 appends an update's objects before the cross-reference section that names them, so the
/// update an object belongs to is the first section standing at or after its bytes. An object
/// with no offset in the current file — one the update deleted — belongs to no update here, which
/// is what keeps a deletion outside Table 257's carve-out: a DSS update deletes nothing.
fn update_of(current: &Document, number: u32, tables: &BTreeSet<usize>) -> Option<usize> {
    let (offset, _) = placement(current.xref(), number)?;
    tables.range(offset..).next().copied()
}

/// What one changed object is, on its own evidence.
#[expect(
    clippy::too_many_arguments,
    reason = "every one of them is a fact about the object under test that this function may not \
              recompute per object: the two states, its number, which bucket it landed in, the \
              update's own tables, the validation material and the signed revision's templates"
)]
fn classify_one(
    signed: &Document,
    current: &Document,
    number: u32,
    bucket: Bucket,
    material: &BTreeSet<u32>,
    tables: &BTreeSet<usize>,
    offset: Option<usize>,
    templates: &[Vec<u8>],
) -> Kind {
    let id = ObjectId::new(number, 0);
    let held = if bucket == Bucket::Removed {
        signed
    } else {
        current
    };
    let object = held.get(id);
    let Some(dict) = dictionary_of(&object) else {
        return Kind::Unclassified;
    };
    if matches!(object, Object::Stream(_))
        && name(&dict, "Type").as_deref() == Some("XRef")
        && offset.is_some_and(|at| tables.contains(&at))
    {
        return Kind::CrossReferenceStream;
    }
    if material.contains(&number) || name(&dict, "Type").as_deref() == Some("DocTimeStamp") {
        return Kind::ValidationMaterial;
    }
    if name(&dict, "Type").as_deref() == Some("Sig") {
        return Kind::Signing;
    }
    let field = field_type(held, &dict);
    match bucket {
        Bucket::Redefined => {
            let before = signed.get(id);
            if restated(&before, &object) {
                return Kind::RestatedUnchanged;
            }
            let was = dictionary_of(&before).unwrap_or_default();
            let moved = changed_keys(&was, &dict);
            if moved.len() == 1 && moved.contains(b"DSS".as_slice()) {
                // §12.8.4.3: the store "shall be the value of a DSS key in the document catalog
                // dictionary", so the catalog gaining one is part of adding the store.
                return Kind::ValidationMaterial;
            }
            if let Some(field) = field {
                if moved.iter().all(|key| named_in(&FILLING_WRITES, key)) {
                    return if field == "Sig" {
                        Kind::Signing
                    } else {
                        Kind::FieldFilledIn
                    };
                }
                if moved.iter().any(|key| named_in(&FIELD_DEFINING, key)) {
                    return Kind::Unclassified;
                }
            }
            if is_annotation(&dict) {
                Kind::Annotation
            } else {
                Kind::Unclassified
            }
        }
        // An added signature field is signing; an added field of any other type is a form being
        // built rather than filled in, which Table 257 permits at no level.
        Bucket::Added | Bucket::Removed => {
            // Only an *added* page can be an instantiation: §12.7.7's operation "add[s] the named
            // page to the current document", and a page the signed revision already displayed was
            // not added by anybody.
            if bucket == Bucket::Added && instantiated(current, &dict, templates) {
                return Kind::TemplateInstantiated;
            }
            match (field.as_deref(), is_annotation(&dict)) {
                (Some("Sig"), _) => Kind::Signing,
                (None, true) => Kind::Annotation,
                _ => Kind::Unclassified,
            }
        }
        Bucket::Unplaceable => Kind::Unplaceable,
    }
}

/// Every object number the current catalog's `/DSS` reaches, and the store itself.
///
/// Each of Table 261's three arrays is "An array of indirect reference to streams", and a VRI
/// dictionary holds the same shape per signature (§12.8.4.4), so one level of reference from
/// each is the whole reach. Nothing here reads a certificate: what the object *is* decides the
/// ranking, and reading it would be §12.8.1's third question.
fn validation_material(current: &Document) -> BTreeSet<u32> {
    let mut found = BTreeSet::new();
    let Ok(catalog) = current.catalog() else {
        return found;
    };
    if let Some(Object::Reference(id)) = catalog.get("DSS") {
        found.insert(id.number);
    }
    let store = current.get_key(&catalog, "DSS");
    let Some(store) = store.as_dict() else {
        return found;
    };
    for key in ["Certs", "CRLs", "OCSPs"] {
        referenced(&current.get_key(store, key), &mut found);
    }
    let vri = current.get_key(store, "VRI");
    if let Some(vri) = vri.as_dict() {
        for (key, entry) in vri.iter() {
            if let Object::Reference(id) = entry {
                found.insert(id.number);
            }
            let entry = current.get_key_by_name(vri, key);
            if let Some(entry) = entry.as_dict() {
                for (_, value) in entry.iter() {
                    referenced(&current.resolve(value), &mut found);
                }
            }
        }
    }
    found
}

/// The object numbers an array's elements name, for [`validation_material`]'s one level of reach.
fn referenced(object: &Object, into: &mut BTreeSet<u32>) {
    if let Object::Array(items) = object {
        for item in items {
            if let Object::Reference(id) = item {
                into.insert(id.number);
            }
        }
    }
}

/// What §12.8.2.4's analysis takes one object of the interactive form to be.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Subject {
    /// An object belonging to the field §12.7.4.2 gives this fully qualified name.
    Field {
        /// The fully qualified name, "formed by appending the child field's partial name to the
        /// parent's fully qualified name, separated by a PERIOD (2Eh)".
        name: String,
        /// The field objects it sits under, itself first and then each ancestor.
        ///
        /// Table 256's `/Data` may name a field rather than the whole form, and this is what says
        /// whether a given object is inside what it named.
        under: Vec<u32>,
    },
    /// An object in the field tree that no chain of `/T` entries names.
    Unnamed,
}

/// Every object of this document's interactive form, under the field it belongs to.
///
/// Entries already present are kept, so a caller walking the current state first and the signed
/// one after it gets each field named as it is named *now* and still hears about a field the
/// update removed.
fn field_subjects(document: &Document, into: &mut BTreeMap<u32, Subject>) {
    let Ok(catalog) = document.catalog() else {
        return;
    };
    let form = document.get_key(&catalog, "AcroForm");
    let Some(form) = form.as_dict() else {
        return;
    };
    let fields = document.get_key(form, "Fields");
    let Some(fields) = fields.as_array().map(<[Object]>::to_vec) else {
        return;
    };
    let mut seen = BTreeSet::new();
    for field in &fields {
        walk_field_subjects(document, field, None, &[], into, &mut seen, 0);
    }
}

/// One level of [`field_subjects`]'s walk.
///
/// §12.7.4.2 decides the two cases that are not a simple append. A field with no parent takes its
/// own partial name — "[f]or a field with no parent, the partial and fully qualified names are the
/// same" — and one with no `/T` at all is not a field but its parent's widget:
///
/// > A field dictionary that does not have a partial field name ( T entry) of its own shall not be
/// > considered a field but simply a Widget annotation.
///
/// so such an object carries the parent's name rather than none, which is exactly what makes a
/// widget's change a change to the field a `/Fields` array can have named.
fn walk_field_subjects(
    document: &Document,
    field: &Object,
    inherited: Option<&str>,
    ancestry: &[u32],
    into: &mut BTreeMap<u32, Subject>,
    seen: &mut BTreeSet<ObjectId>,
    depth: usize,
) {
    if depth > MAX_FORM_DEPTH || into.len() >= MAX_FORM_FIELDS {
        return;
    }
    if let Some(id) = field.as_reference()
        && !seen.insert(id)
    {
        return;
    }
    let resolved = document.resolve(field);
    let Some(dict) = resolved.as_dict() else {
        return;
    };
    let partial = match dict.get("T") {
        Some(Object::String(bytes)) => Some(pdf_syntax::text_string(bytes)),
        _ => None,
    };
    let name = match (inherited, partial.as_deref()) {
        (Some(prefix), Some(partial)) => Some(format!("{prefix}.{partial}")),
        (None, Some(partial)) => Some(partial.to_owned()),
        (Some(prefix), None) => Some(prefix.to_owned()),
        (None, None) => None,
    };
    let mut under = Vec::with_capacity(ancestry.len().saturating_add(1));
    if let Some(id) = field.as_reference() {
        under.push(id.number);
    }
    under.extend_from_slice(ancestry);
    let subject = match &name {
        Some(name) => Subject::Field {
            name: name.clone(),
            under: under.clone(),
        },
        None => Subject::Unnamed,
    };
    if let Some(id) = field.as_reference() {
        into.entry(id.number).or_insert_with(|| subject.clone());
        // Table 166's `/AP` belongs to the field the way `/V` does — it is what §12.7.4.3 has the
        // processor construct *for* a value — so an appearance stream rewritten by an update is a
        // change to the field it draws, and a transform naming that field reaches it.
        for appearance in appearances(document, dict) {
            into.entry(appearance).or_insert_with(|| subject.clone());
        }
    }
    if let Some(kids) = document
        .get_key(dict, "Kids")
        .as_array()
        .map(<[Object]>::to_vec)
    {
        for kid in &kids {
            walk_field_subjects(
                document,
                kid,
                name.as_deref(),
                &under,
                into,
                seen,
                depth.saturating_add(1),
            );
        }
    }
}

/// The objects a Table 256 `/Data` may name to scope the analysis to every field in the document.
fn form_objects(document: &Document) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    if let Some(Object::Reference(root)) = document.xref().trailer().get("Root") {
        out.insert(root.number);
    }
    let Ok(catalog) = document.catalog() else {
        return out;
    };
    if let Some(Object::Reference(id)) = catalog.get("AcroForm") {
        out.insert(id.number);
    }
    let form = document.get_key(&catalog, "AcroForm");
    if let Some(form) = form.as_dict()
        && let Some(Object::Reference(id)) = form.get("Fields")
    {
        out.insert(id.number);
    }
    out
}

/// The content of every page §12.7.7's `/Templates` name tree holds in this state of the file.
///
/// > If the page is not intended to be displayed by the PDF processor, it shall be referenced
/// > from the name dictionary's Templates tree instead.
///
/// so the tree is reached through §7.7.4's name dictionary, and each of its values is a page. What
/// is kept is the page's **content** rather than its object number, because that is what survives
/// instantiation either way the clause leaves open — the same stream object, or a copy of it.
///
/// A page whose content is empty is not kept, and this is not a saving. Two empty contents match
/// each other, so keeping one would rank every contentless page added after signing as an
/// instantiation of it — a lenient default in exactly the place this module refuses them.
fn templates(document: &Document) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let Ok(catalog) = document.catalog() else {
        return out;
    };
    let names = document.get_key(&catalog, "Names");
    let Some(names) = names.as_dict() else {
        return out;
    };
    let tree = document.get_key(names, "Templates");
    name_tree_pages(document, &tree, &mut out, 0);
    out
}

/// One node of §7.9.6's name tree, collecting the content of every page its values name.
fn name_tree_pages(document: &Document, node: &Object, out: &mut Vec<Vec<u8>>, depth: usize) {
    if depth > MAX_NAME_TREE_DEPTH || out.len() >= MAX_TEMPLATES {
        return;
    }
    let Some(node) = node.as_dict() else {
        return;
    };
    if let Some(entries) = document
        .get_key(node, "Names")
        .as_array()
        .map(<[Object]>::to_vec)
    {
        // Table 36: "[ key 1 value 1 key 2 value 2 …key n value n ]", so the values are the odd
        // positions and the keys — the names a script would instantiate by — decide nothing here.
        for value in entries.iter().skip(1).step_by(2) {
            if out.len() >= MAX_TEMPLATES {
                return;
            }
            let page = document.resolve(value);
            if let Some(dict) = dictionary_of(&page)
                && let Some(content) = page_content(document, &dict)
            {
                out.push(content);
            }
        }
    }
    if let Some(kids) = document
        .get_key(node, "Kids")
        .as_array()
        .map(<[Object]>::to_vec)
    {
        for kid in &kids {
            let kid = document.resolve(kid);
            name_tree_pages(document, &kid, out, depth.saturating_add(1));
        }
    }
}

/// Table 31's `/Contents` as the bytes its streams hold, or `None` where there are none to hold.
///
/// §7.7.3.3's table gives the entry:
///
/// > ( Optional ) A content stream (see 7.8.2, "Content streams") that shall describe the contents
/// > of this page. If this entry is absent, the page shall be empty.
///
/// A stream whose data could not be decrypted yields `None` rather than the empty data
/// [`pdf_syntax::Stream::decryption_failed`] leaves behind, for [`restated`]'s reason: two
/// empties are not evidence that two pages say the same thing.
fn page_content(document: &Document, page: &Dictionary) -> Option<Vec<u8>> {
    let contents = document.get_key(page, "Contents");
    let mut bytes: Vec<u8> = Vec::new();
    let mut absorb = |stream: &pdf_syntax::Stream| -> bool {
        if stream.decryption_failed
            || bytes.len().saturating_add(stream.data.len()) > MAX_TEMPLATE_CONTENT
        {
            return false;
        }
        bytes.extend_from_slice(&stream.data);
        true
    };
    match &contents {
        Object::Stream(stream) => {
            if !absorb(stream) {
                return None;
            }
        }
        Object::Array(items) => {
            for item in items {
                let resolved = document.resolve(item);
                let Object::Stream(stream) = &resolved else {
                    return None;
                };
                if !absorb(stream) {
                    return None;
                }
            }
        }
        _ => return None,
    }
    if bytes.is_empty() { None } else { Some(bytes) }
}

/// Whether an added object is a page instantiated from one of `templates`.
///
/// Three tests, and each is a sentence rather than a heuristic. Table 31 says what a regular page
/// in the tree is — `/Type` "shall be Page for a page object", `/Parent` "( Required; shall be an
/// indirect reference ) The page tree node that is the immediate parent of this page object" — and
/// §12.7.7 says a template is precisely the page that has neither: "[s]uch invisible pages shall
/// have an object type of Template rather than Page and shall have no Parent or B entry". So the
/// object added has to be the *regular* page the operation produces. The third test is the
/// content, which is what identifies *which* page it is.
fn instantiated(current: &Document, page: &Dictionary, templates: &[Vec<u8>]) -> bool {
    if templates.is_empty()
        || name(page, "Type").as_deref() != Some("Page")
        || page.get("Parent").is_none()
    {
        return false;
    }
    let Some(content) = page_content(current, page) else {
        return false;
    };
    templates.contains(&content)
}

/// Whether a page tree node changed only by receiving pages this update instantiated.
///
/// Table 30's two entries are the whole of what receiving a page writes: `/Kids`, "[a]n array of
/// indirect references to the immediate children of this node", and `/Count`, "[t]he number of
/// leaf nodes (page objects) that are descendants of this node within the page tree". A node that
/// moved anything else, lost a kid, or gained one that is not an instantiated page is not part of
/// this operation and stays [`Kind::Unclassified`] — which is a refusal, not a permission.
fn extended_by_instantiation(
    signed: &Document,
    current: &Document,
    number: u32,
    instantiated: &BTreeSet<u32>,
) -> bool {
    let id = ObjectId::new(number, 0);
    let Some(after) = dictionary_of(&current.get(id)) else {
        return false;
    };
    let Some(before) = dictionary_of(&signed.get(id)) else {
        return false;
    };
    if name(&after, "Type").as_deref() != Some("Pages") {
        return false;
    }
    if !changed_keys(&before, &after)
        .iter()
        .all(|key| named_in(&["Kids", "Count"], key))
    {
        return false;
    }
    let was = kids(signed, &before);
    let now = kids(current, &after);
    now.len() > was.len()
        && was.iter().all(|kid| now.contains(kid))
        && now
            .iter()
            .filter(|kid| !was.contains(kid))
            .all(|kid| instantiated.contains(&kid.number))
}

/// The objects Table 31's `/Contents` names, where it names any.
///
/// Table 31 allows the entry both shapes — "[t]he value shall be either a single stream or an
/// array of streams" — and either may be written indirectly, so an indirect entry contributes its
/// own number *and*, where it resolves to an array, the numbers of the streams in it.
fn content_streams(document: &Document, page: &Dictionary) -> Vec<u32> {
    let numbers = |items: &[Object]| -> Vec<u32> {
        items
            .iter()
            .filter_map(Object::as_reference)
            .map(|id| id.number)
            .collect()
    };
    match page.get("Contents") {
        Some(&Object::Reference(id)) => {
            let mut out = vec![id.number];
            if let Object::Array(items) = document.get(id) {
                out.extend(numbers(&items));
            }
            out
        }
        Some(Object::Array(items)) => numbers(items),
        _ => Vec::new(),
    }
}

/// The objects Table 30's `/Kids` names, in the order the array states them.
fn kids(document: &Document, node: &Dictionary) -> Vec<ObjectId> {
    document
        .get_key(node, "Kids")
        .as_array()
        .map(|kids| kids.iter().filter_map(Object::as_reference).collect())
        .unwrap_or_default()
}

/// The object numbers an annotation's `/AP` names, one appearance subdictionary deep.
///
/// §12.5.5: "Each entry in the appearance dictionary may contain either a single appearance
/// stream or an appearance subdictionary . In the latter case, the subdictionary shall define
/// multiple appearance streams corresponding to different appearance states of the annotation."
/// So two levels reach every stream an annotation has and no more.
fn appearances(document: &Document, dict: &Dictionary) -> Vec<u32> {
    let mut found = Vec::new();
    let appearance = document.get_key(dict, "AP");
    let Some(appearance) = appearance.as_dict() else {
        return found;
    };
    for (_, entry) in appearance.iter() {
        match entry {
            Object::Reference(id) => found.push(id.number),
            Object::Dictionary(states) => {
                for (_, state) in states.iter() {
                    if let Object::Reference(id) = state {
                        found.push(id.number);
                    }
                }
            }
            _ => {}
        }
    }
    found
}

/// Table 226's `/FT`, followed up the `/Parent` chain because the entry is inheritable.
fn field_type(document: &Document, dict: &Dictionary) -> Option<String> {
    let mut here = dict.clone();
    for _ in 0..MAX_FIELD_ANCESTRY {
        if let Some(field) = name(&here, "FT") {
            return Some(field);
        }
        let parent = document.get_key(&here, "Parent");
        here = parent.as_dict()?.clone();
    }
    None
}

/// Whether a dictionary is Table 166's annotation.
///
/// `/Type` is optional there — "if present, shall be Annot for an annotation dictionary" — so the
/// test is the two entries the table marks required instead: `/Subtype`, "The type of annotation
/// that this dictionary describes", and `/Rect`, "The annotation rectangle , defining the location
/// of the annotation on the page in default user space units."
fn is_annotation(dict: &Dictionary) -> bool {
    name(dict, "Type").as_deref() == Some("Annot")
        || (dict.get("Subtype").is_some() && dict.get("Rect").is_some())
}

/// The keys whose values two states of one dictionary disagree about, either side missing.
fn changed_keys(before: &Dictionary, after: &Dictionary) -> BTreeSet<Vec<u8>> {
    let mut moved = BTreeSet::new();
    for (key, value) in before.iter() {
        if !same_value(after.get_by_name(key), Some(value)) {
            moved.insert(key.as_bytes().to_vec());
        }
    }
    for (key, value) in after.iter() {
        if !same_value(before.get_by_name(key), Some(value)) {
            moved.insert(key.as_bytes().to_vec());
        }
    }
    moved
}

/// Whether an update rewrote an object without changing what it says.
///
/// A stream is compared over its data as well as its dictionary, and a stream whose data could
/// not be decrypted is never restated: [`pdf_syntax::Stream::decryption_failed`] leaves the data
/// empty rather than wrong, and two empties are not evidence of anything.
fn restated(before: &Object, after: &Object) -> bool {
    match (before, after) {
        (Object::Stream(before), Object::Stream(after)) => {
            !before.decryption_failed
                && !after.decryption_failed
                && before.data == after.data
                && changed_keys(&before.dict, &after.dict).is_empty()
        }
        (Object::Stream(_), _) | (_, Object::Stream(_)) => false,
        (before, after) => same_value(Some(before), Some(after)),
    }
}

/// Whether two entries are the same **value**, which for a number is not the same as the same
/// writing of it.
///
/// §7.3.3 is explicit that one number has two written forms:
///
/// > A real number shall not be present when an integer is expected. Wherever a real number is
/// > expected, an integer may be used instead. For example, it is not necessary to write the
/// > number 1.0 in real format; the integer 1 is sufficient.
///
/// So an update that rewrites `396.0` as `396` has changed how the file is written and not what
/// it says, and a comparison that called that a modified annotation would be reporting its own
/// parser. **`prefilled_f1040.pdf` is exactly that file**: of the eight widgets its updates
/// rewrite, one has a rectangle that differs only in this and seven have rectangles that really
/// moved, and the difference decides whether each is a filled field or a modified annotation.
fn same_value(before: Option<&Object>, after: Option<&Object>) -> bool {
    match (before, after) {
        (Some(Object::Integer(before)), Some(Object::Real(after)))
        | (Some(Object::Real(after)), Some(Object::Integer(before))) => {
            // Exact, not tolerant: an integer PDF can write is one an f64 holds without loss, so
            // this asks whether the two name one number and never whether they are close.
            #[expect(
                clippy::cast_precision_loss,
                reason = "a number too large for f64 to hold exactly is one no `Real` beside \
                          it can equal either"
            )]
            let widened = *before as f64;
            #[expect(
                clippy::float_cmp,
                reason = "the question is whether two writings name one number, which a margin \
                          of error would answer with a different question"
            )]
            let same = widened == *after;
            same
        }
        (Some(Object::Array(before)), Some(Object::Array(after))) => {
            before.len() == after.len()
                && before
                    .iter()
                    .zip(after.iter())
                    .all(|(before, after)| same_value(Some(before), Some(after)))
        }
        (Some(Object::Dictionary(before)), Some(Object::Dictionary(after))) => {
            before.len() == after.len()
                && before
                    .iter()
                    .all(|(key, value)| same_value(after.get_by_name(key), Some(value)))
        }
        (before, after) => before == after,
    }
}

/// Whether a key's bytes are one of a list of names this source spells out.
fn named_in(names: &[&str], key: &[u8]) -> bool {
    names.iter().any(|name| name.as_bytes() == key)
}

/// A dictionary's name-valued entry, as text where the name is one.
fn name(dict: &Dictionary, key: &str) -> Option<String> {
    match dict.get(key) {
        Some(Object::Name(name)) => name.as_str().map(str::to_owned),
        _ => None,
    }
}

/// The dictionary an object carries, a stream's own included.
fn dictionary_of(object: &Object) -> Option<Dictionary> {
    match object {
        Object::Dictionary(dict) => Some(dict.clone()),
        Object::Stream(stream) => Some(stream.dict.clone()),
        _ => None,
    }
}

/// What `level` says about a change of `kind` in an update with this `disposition`.
///
/// The order of the tests is the argument. An update Table 257 disregards is not a change at all,
/// whatever its objects are; an object no table places has not been shown to have changed, so it
/// is refused rather than ranked; an update whose carve-out could not be decided takes its whole
/// contents with it. Only then does the level's own vocabulary apply — and **level 1 needs none
/// of it**, because "[n]o changes to the document shall be permitted" ranks an unclassified
/// change as surely as a classified one.
fn verdict(level: Modification, kind: Kind, disposition: Disposition) -> Verdict {
    if disposition == Disposition::Disregarded {
        return Verdict::Disregarded;
    }
    if kind == Kind::Unplaceable || disposition == Disposition::Mixed {
        return Verdict::Unrankable;
    }
    if kind == Kind::CrossReferenceStream || kind == Kind::RestatedUnchanged {
        return Verdict::Permitted;
    }
    match level {
        // "1 No changes to the document shall be permitted; any change to the document shall
        // invalidate the signature." Nothing is left to classify.
        Modification::None => Verdict::NotPermitted,
        // "2 Permitted changes shall be filling in forms, instantiating page templates, and
        // signing; other changes shall invalidate the signature."
        Modification::FormFilling => match kind {
            Kind::FieldFilledIn
            | Kind::Signing
            | Kind::FieldAppearance
            | Kind::TemplateInstantiated => Verdict::Permitted,
            Kind::Annotation | Kind::AnnotationAppearance => Verdict::NotPermitted,
            _ => Verdict::Unrankable,
        },
        // "3 Permitted changes shall be the same as for 2, as well as annotation creation,
        // deletion, and modification; other changes shall invalidate the signature."
        Modification::FormFillingAndAnnotation => match kind {
            Kind::FieldFilledIn
            | Kind::Signing
            | Kind::FieldAppearance
            | Kind::TemplateInstantiated
            | Kind::Annotation
            | Kind::AnnotationAppearance => Verdict::Permitted,
            _ => Verdict::Unrankable,
        },
        // Table 257 states three values and a default of 2. A `/P` that is none of them is an
        // author's statement this reader cannot read, and guessing which way it leans is exactly
        // the lenient default this module refuses.
        Modification::Unknown(_) => Verdict::Unrankable,
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use pdf_syntax::Document;

    use super::{
        Comparison, FieldJudgement, Judgement, NotComparable, NotScoped, Ranked, ScopedTo,
    };
    use crate::signature::{FieldMdp, FieldSelection, Modification, Signature, signatures};

    /// The signature object every fixture here carries, with a `/ByteRange` to be filled in.
    ///
    /// Written at a fixed width so that the four numbers can be patched in place once the file
    /// is assembled and its own offsets are known — which is what a producer does, and the only
    /// way a fixture's range can name its own bytes rather than a guess.
    const SIGNATURE: &str = "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
                             /ByteRange [0000000000 0000000000 0000000000 0000000000] \
                             /Contents <00112233445566778899aabbccddeeff> >>";

    /// The catalog every fixture here carries, unless a test states its own.
    const CATALOG: &str = "<< /Type /Catalog /Pages 2 0 R /Perms << /DocMDP 4 0 R >> \
                           /AcroForm << /Fields [5 0 R] /SigFlags 3 >> >>";

    /// A one-revision document ending at `%%EOF`, with the signature's range naming its own bytes.
    ///
    /// Object 1 is the catalog, 2 the page tree, 3 a page and 4 the signature; a caller adds more
    /// after those.
    fn signed_file(extra: &[&str]) -> Vec<u8> {
        signed_file_under(CATALOG, extra)
    }

    /// The same, with the catalog spelled out — for a fixture that needs a `/Names` tree in it.
    fn signed_file_under(catalog: &str, extra: &[&str]) -> Vec<u8> {
        let mut objects = vec![
            catalog,
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
            SIGNATURE,
            "<< /FT /Sig /T (Signature1) /V 4 0 R /Subtype /Widget >>",
        ];
        objects.extend_from_slice(extra);

        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let size = objects.len().saturating_add(1);
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        let mut bytes = out.into_bytes();
        fill_in_the_range(&mut bytes);
        bytes
    }

    /// Rewrites the placeholder `/ByteRange` so that it names this file with the value's hole in it.
    fn fill_in_the_range(bytes: &mut Vec<u8>) {
        let text = String::from_utf8(bytes.clone()).expect("the fixture is ASCII");
        let value = text.find("/Contents <").expect("a signature value");
        let start = value.saturating_add("/Contents ".len());
        let end = text[start..]
            .find('>')
            .map(|at| start.saturating_add(at).saturating_add(1))
            .expect("a closing delimiter");
        let range = text.find("/ByteRange [").expect("a placeholder range");
        let at = range.saturating_add("/ByteRange [".len());
        let filled = format!(
            "{:010} {:010} {:010} {:010}",
            0,
            start,
            end,
            text.len().saturating_sub(end)
        );
        bytes.splice(at..at.saturating_add(filled.len()), filled.into_bytes());
    }

    /// Appends §7.5.6's incremental update: the objects given, a section naming them, a trailer.
    ///
    /// `freed` are object numbers the update deletes, which §7.5.6 does by cross-reference entry
    /// rather than by removing bytes.
    fn update(bytes: &mut Vec<u8>, objects: &[(u32, &str)], freed: &[u32], root: u32) {
        let previous = String::from_utf8(bytes.clone())
            .expect("the fixture is ASCII")
            .rfind("startxref\n")
            .map(|at| {
                let tail = &String::from_utf8(bytes.clone()).expect("ascii")[at..];
                tail.lines()
                    .nth(1)
                    .and_then(|line| line.trim().parse::<usize>().ok())
                    .expect("the previous section's offset")
            })
            .expect("a previous section");

        let mut out = String::new();
        let mut placed = Vec::new();
        let mut highest = 0;
        for (number, body) in objects {
            placed.push((*number, bytes.len().saturating_add(out.len())));
            highest = highest.max(*number);
            let _ = write!(out, "{number} 0 obj\n{body}\nendobj\n");
        }
        for number in freed {
            highest = highest.max(*number);
        }
        let xref_at = bytes.len().saturating_add(out.len());
        out.push_str("xref\n");
        for (number, offset) in &placed {
            let _ = write!(out, "{number} 1\n{offset:010} 00000 n \n");
        }
        for number in freed {
            let _ = write!(out, "{number} 1\n0000000000 00001 f \n");
        }
        let size = usize::try_from(highest).unwrap_or(0).saturating_add(1);
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root {root} 0 R /Prev {previous} >>\n\
             startxref\n{xref_at}\n%%EOF\n"
        );
        bytes.extend_from_slice(out.as_bytes());
    }

    /// The one signature every fixture here carries.
    fn only_signature(document: &Document) -> Signature {
        let found = signatures(document);
        let [signature] = found.as_slice() else {
            panic!("one signature, got {found:?}");
        };
        signature.clone()
    }

    /// A file nobody touched after signing has no change to rank, and the prefix *is* the file.
    ///
    /// The control the next test needs (trap 13): a comparison that reported a difference here
    /// would be reporting its own reconstruction, because the two states are the same bytes.
    #[test]
    fn a_document_unchanged_since_signing_has_no_change_to_rank() {
        let bytes = signed_file(&[]);
        let length = bytes.len();
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(comparison.end(), u64::try_from(length).expect("small"));
        assert_eq!(comparison.updates_after(), 0);
        assert!(
            comparison.changes().is_empty(),
            "{:?}",
            comparison.changes()
        );
        assert_eq!(
            comparison.against(Modification::FormFilling),
            Judgement::NoChangeToRank
        );
    }

    /// Every object an incremental update touches is named, by number and by what happened to it.
    ///
    /// The defect this is calibrated against is the one §12.8.2.2.2 exists to catch: bytes
    /// appended after a certification signature that change what the document says. The update
    /// plants one of each — an object added, one redefined, one deleted — and each has to come
    /// back in its own bucket, because Table 257's levels rank those three differently.
    #[test]
    fn an_update_after_signing_names_what_it_added_redefined_and_removed() {
        let mut bytes = signed_file(&["<< /Spare true >>"]);
        let signed_length = bytes.len();
        update(
            &mut bytes,
            &[
                (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] >>"),
                (7, "<< /Added true >>"),
            ],
            &[6],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(
            comparison.end(),
            u64::try_from(signed_length).expect("small")
        );
        assert_eq!(comparison.updates_after(), 1);
        let changes = comparison.changes();
        assert_eq!(changes.added.named(), [7], "{changes:?}");
        assert_eq!(changes.redefined.named(), [3], "{changes:?}");
        assert_eq!(changes.removed.named(), [6], "{changes:?}");
        assert!(changes.unplaceable.is_empty(), "{changes:?}");
        assert!(!changes.catalog_moved, "{changes:?}");
        // Table 257's level 1 needs no classification at all: "any change to the document shall
        // invalidate the signature", and three objects changed. The two other levels are the
        // tests below, which is where the kinds have to be told apart.
        assert_eq!(
            comparison.against(Modification::None),
            Judgement::NotPermitted {
                level: Modification::None,
                objects: 3,
            }
        );
    }

    /// A trailer pointing at another catalog is a change no object count would show.
    ///
    /// The whole document hangs off Table 15's `/Root`, and an update may replace it with an
    /// object the signed revision already held — no object added, none redefined, and a different
    /// document.
    #[test]
    fn a_catalog_replaced_after_signing_is_a_change_of_its_own() {
        let mut bytes =
            signed_file(
                &["<< /Type /Catalog /Pages 2 0 R /Perms << /DocMDP 4 0 R >> \
             /AcroForm << /Fields [5 0 R] /SigFlags 3 >> >>"],
            );
        update(&mut bytes, &[], &[], 6);
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        let changes = comparison.changes();
        assert!(changes.added.is_empty(), "{changes:?}");
        assert!(changes.redefined.is_empty(), "{changes:?}");
        assert!(changes.removed.is_empty(), "{changes:?}");
        assert!(changes.catalog_moved, "{changes:?}");
        assert!(!changes.is_empty(), "{changes:?}");
        // Refused rather than ranked: replacing the catalog is none of Table 257's operations
        // and is not thereby one of the changes it forbids either — the table ranks *what* was
        // done, and this reader does not resolve a whole catalog to an operation.
        assert_eq!(
            comparison.against(Modification::FormFillingAndAnnotation),
            Judgement::NotClassified {
                level: Modification::FormFillingAndAnnotation,
                objects: 1,
            }
        );
        // Level 1 needs no such resolution, and says so.
        assert_eq!(
            comparison.against(Modification::None),
            Judgement::NotPermitted {
                level: Modification::None,
                objects: 1,
            }
        );
    }

    /// An object neither state can place is counted as such, never as unchanged.
    ///
    /// The plant is a cross-reference entry naming an offset outside the revision that declared
    /// it: both tables say the same thing about object 5 and what they say is not a place in the
    /// signed prefix. Calibrated by comparison with the test above, where object 5's entry is
    /// sound and the same update produces an empty `unplaceable` — so the bucket is reached by
    /// the defect and by nothing else.
    #[test]
    fn an_object_no_table_can_place_is_counted_rather_than_assumed_unchanged() {
        let mut bytes = signed_file(&[]);
        let text = String::from_utf8(bytes.clone()).expect("ascii");
        let table = text.rfind("xref\n0 ").expect("the section");
        let entry = text[table..]
            .match_indices(" 00000 n \n")
            .nth(4)
            .map(|(at, _)| table.saturating_add(at).saturating_sub(10))
            .expect("object 5's entry");
        bytes.splice(entry..entry.saturating_add(10), b"0000999999".to_vec());
        update(&mut bytes, &[(7, "<< /Added true >>")], &[], 1);

        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");
        let changes = comparison.changes();
        assert_eq!(changes.unplaceable.named(), [5], "{changes:?}");
        assert_eq!(changes.added.named(), [7], "{changes:?}");
        assert!(changes.redefined.is_empty(), "{changes:?}");
    }

    /// A text field widget the signed revision holds, for an update to fill in.
    const EMPTY_FIELD: &str = "<< /FT /Tx /T (Name) /Subtype /Widget /Type /Annot \
                               /Rect [100 200 300 220] /P 3 0 R >>";

    /// The appearance stream §12.7.4.3 has the processor construct for a filled field.
    const APPEARANCE: &str = "<< /Type /XObject /Subtype /Form /BBox [0 0 200 20] /Length 0 >>\n\
                              stream\n\nendstream";

    /// What `rank` said about each object, as text, so a failure names the object and the reason.
    fn detail(comparison: &Comparison, level: Modification) -> Vec<String> {
        comparison
            .rank(level)
            .detail
            .iter()
            .map(
                |Ranked {
                     number,
                     kind,
                     verdict,
                 }| format!("{number} {kind:?} {verdict:?}"),
            )
            .collect()
    }

    /// Filling a field in is Table 257's level 2, and is not its level 1.
    ///
    /// §12.8.2.2.2's Table 257, second of the `/P` row's three values:
    ///
    /// > 2 Permitted changes shall be filling in forms, instantiating page templates, and
    /// > signing; other changes shall invalidate the signature.
    ///
    /// The update writes what filling in writes and nothing else — `/V`, the `/AP` that Table 166
    /// requires beside it, and `/M` — so both halves of the ranking are exercised: the field
    /// itself and the appearance stream that comes with it, which is an object the signed
    /// revision never held and would be unclassifiable on its own evidence.
    #[test]
    fn a_field_filled_in_after_signing_is_what_level_two_permits() {
        let mut bytes = signed_file(&[EMPTY_FIELD]);
        update(
            &mut bytes,
            &[
                (
                    6,
                    "<< /FT /Tx /T (Name) /Subtype /Widget /Type /Annot /Rect [100 200 300 220] \
                     /P 3 0 R /V (Ada) /AP << /N 7 0 R >> /M (D:20260913120000Z) >>",
                ),
                (7, APPEARANCE),
            ],
            &[],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(
            detail(&comparison, Modification::FormFilling),
            ["6 FieldFilledIn Permitted", "7 FieldAppearance Permitted"]
        );
        for level in [
            Modification::FormFilling,
            Modification::FormFillingAndAnnotation,
        ] {
            assert_eq!(
                comparison.against(level),
                Judgement::WithinWhatIsPermitted {
                    level,
                    objects: 2,
                    disregarded: 0,
                },
                "{level:?}"
            );
        }
        // "1 No changes to the document shall be permitted; any change to the document shall
        // invalidate the signature." The same two objects, and the opposite answer.
        assert_eq!(
            comparison.against(Modification::None),
            Judgement::NotPermitted {
                level: Modification::None,
                objects: 2,
            }
        );
    }

    /// A widget whose rectangle moved is an annotation modified, which level 2 does not permit.
    ///
    /// **The defect this is calibrated against is the one that decides a real file.** A widget
    /// annotation is an annotation whatever else it is, so a reader that ranked by object *type*
    /// would call every filled field an annotation modification and refuse level 2 outright;
    /// one that ranked by "it is a field, so it was filled in" would wave through a
    /// certification-breaking move of the annotation. The pair is this test and the one above:
    /// the same update, differing only in `/Rect`, and the answers differ with it.
    /// `prefilled_f1040.pdf` is where this is not hypothetical — six of its eight rewritten
    /// widgets move their rectangles and two do not.
    #[test]
    fn a_widget_whose_rectangle_moved_is_an_annotation_rather_than_a_filled_field() {
        let mut bytes = signed_file(&[EMPTY_FIELD]);
        update(
            &mut bytes,
            &[
                (
                    6,
                    "<< /FT /Tx /T (Name) /Subtype /Widget /Type /Annot /Rect [100 200 300 221] \
                     /P 3 0 R /V (Ada) /AP << /N 7 0 R >> /M (D:20260913120000Z) >>",
                ),
                (7, APPEARANCE),
            ],
            &[],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(
            detail(&comparison, Modification::FormFilling),
            [
                "6 Annotation NotPermitted",
                "7 AnnotationAppearance NotPermitted"
            ]
        );
        assert_eq!(
            comparison.against(Modification::FormFilling),
            Judgement::NotPermitted {
                level: Modification::FormFilling,
                objects: 2,
            }
        );
        // "3 Permitted changes shall be the same as for 2, as well as annotation creation,
        // deletion, and modification".
        assert_eq!(
            comparison.against(Modification::FormFillingAndAnnotation),
            Judgement::WithinWhatIsPermitted {
                level: Modification::FormFillingAndAnnotation,
                objects: 2,
                disregarded: 0,
            }
        );
    }

    /// The same rectangle written the other way round is not a change, and §7.3.3 is why.
    ///
    /// > Wherever a real number is expected, an integer may be used instead. For example, it is
    /// > not necessary to write the number 1.0 in real format; the integer 1 is sufficient.
    ///
    /// So `300` and `300.0` are one number, and an update that rewrites one as the other has
    /// changed the file and not the document. The defect is the test above's answer given to
    /// this input: a reader comparing written forms would call this an annotation whose rectangle
    /// moved, and refuse a level-2 certification over a producer's formatting.
    #[test]
    fn a_number_rewritten_in_the_other_form_is_not_a_change_to_the_document() {
        let mut bytes = signed_file(&[EMPTY_FIELD]);
        update(
            &mut bytes,
            &[(
                6,
                "<< /FT /Tx /T (Name) /Subtype /Widget /Type /Annot /Rect [100.0 200 300.0 220] \
                 /P 3 0 R >>",
            )],
            &[],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(comparison.changes().redefined.named(), [6]);
        assert_eq!(
            detail(&comparison, Modification::None),
            ["6 RestatedUnchanged Permitted"]
        );
        // Level 1 permits no change to the document, and this is not one.
        assert_eq!(
            comparison.against(Modification::None),
            Judgement::WithinWhatIsPermitted {
                level: Modification::None,
                objects: 1,
                disregarded: 0,
            }
        );
    }

    /// An annotation added after signing is level 3's alone.
    ///
    /// The comment workflow Table 257 separates from the form workflow: level 2 states its
    /// permitted changes and this is not among them, so "other changes shall invalidate the
    /// signature" applies; level 3 names "annotation creation" outright.
    #[test]
    fn an_annotation_created_after_signing_is_level_threes_alone() {
        let mut bytes = signed_file(&[]);
        update(
            &mut bytes,
            &[(
                6,
                "<< /Type /Annot /Subtype /Text /Rect [10 10 30 30] /Contents (a comment) >>",
            )],
            &[],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(
            detail(&comparison, Modification::FormFilling),
            ["6 Annotation NotPermitted"]
        );
        assert_eq!(
            comparison.against(Modification::FormFilling),
            Judgement::NotPermitted {
                level: Modification::FormFilling,
                objects: 1,
            }
        );
        assert_eq!(
            comparison.against(Modification::FormFillingAndAnnotation),
            Judgement::WithinWhatIsPermitted {
                level: Modification::FormFillingAndAnnotation,
                objects: 1,
                disregarded: 0,
            }
        );
    }

    /// A change that is none of Table 257's operations is refused at 2 and 3, and forbidden at 1.
    ///
    /// The rule the whole module is written to: an answer of *permitted* that was never arrived
    /// at is worse than no answer. A page whose `/MediaBox` an update rewrote is not filling in a
    /// form, not signing, not a page template this reader can recognise as instantiated and not
    /// an annotation — so levels 2 and 3 say they did not rank it, and name the object. Level 1
    /// needs no vocabulary and does not refuse.
    #[test]
    fn a_change_that_is_none_of_the_tables_operations_is_refused_rather_than_permitted() {
        let mut bytes = signed_file(&[]);
        update(
            &mut bytes,
            &[(3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] >>")],
            &[],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(
            detail(&comparison, Modification::FormFillingAndAnnotation),
            ["3 Unclassified Unrankable"]
        );
        for level in [
            Modification::FormFilling,
            Modification::FormFillingAndAnnotation,
            Modification::Unknown(7),
        ] {
            assert_eq!(
                comparison.against(level),
                Judgement::NotClassified { level, objects: 1 },
                "{level:?}"
            );
        }
        assert_eq!(
            comparison.against(Modification::None),
            Judgement::NotPermitted {
                level: Modification::None,
                objects: 1,
            }
        );
    }

    /// An update that adds only a document security store is not a change to the document.
    ///
    /// §12.8.2.2.2's Table 257, above the `/P` row's three values:
    ///
    /// > Changes to a PDF that are incremental updates which include only the data necessary to
    /// > add DSS's … and/or document timestamps … to the document shall not be considered as
    /// > changes to the document as defined in the choices below.
    ///
    /// Level 1 is where this has teeth, because level 1 forbids everything else: three objects
    /// change — the store, the certificate stream it holds, and the catalog that gains §12.8.4.3's
    /// `/DSS` key — and the answer is still that nothing forbidden happened.
    #[test]
    fn an_update_adding_only_a_security_store_is_not_a_change_to_the_document() {
        let mut bytes = signed_file(&[]);
        update(
            &mut bytes,
            &[
                (
                    1,
                    "<< /Type /Catalog /Pages 2 0 R /Perms << /DocMDP 4 0 R >> /DSS 6 0 R \
                     /AcroForm << /Fields [5 0 R] /SigFlags 3 >> >>",
                ),
                (6, "<< /Type /DSS /Certs [7 0 R] >>"),
                (7, "<< /Length 0 >>\nstream\n\nendstream"),
            ],
            &[],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(
            detail(&comparison, Modification::None),
            [
                "1 ValidationMaterial Disregarded",
                "6 ValidationMaterial Disregarded",
                "7 ValidationMaterial Disregarded",
            ]
        );
        assert_eq!(
            comparison.against(Modification::None),
            Judgement::WithinWhatIsPermitted {
                level: Modification::None,
                objects: 0,
                disregarded: 3,
            }
        );
    }

    /// An update carrying a security store **and** something else is refused, not carved out.
    ///
    /// Table 257's condition is that the update "include only the data necessary" for a store or
    /// a timestamp, and this one does not — so the carve-out neither applies nor fails to apply,
    /// and every object of the update is unranked. Calibrated against the test above, which is
    /// the same update with the page left alone: the defect is a carve-out that reads the
    /// clause's "only" as "at least", which would hide a page rewritten under cover of a DSS.
    #[test]
    fn an_update_carrying_a_security_store_and_more_is_refused_rather_than_carved_out() {
        let mut bytes = signed_file(&[]);
        update(
            &mut bytes,
            &[
                (
                    1,
                    "<< /Type /Catalog /Pages 2 0 R /Perms << /DocMDP 4 0 R >> /DSS 6 0 R \
                     /AcroForm << /Fields [5 0 R] /SigFlags 3 >> >>",
                ),
                (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] >>"),
                (6, "<< /Type /DSS /Certs [7 0 R] >>"),
                (7, "<< /Length 0 >>\nstream\n\nendstream"),
            ],
            &[],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(
            detail(&comparison, Modification::FormFillingAndAnnotation),
            [
                "1 ValidationMaterial Unrankable",
                "3 Unclassified Unrankable",
                "6 ValidationMaterial Unrankable",
                "7 ValidationMaterial Unrankable",
            ]
        );
        for level in [
            Modification::None,
            Modification::FormFilling,
            Modification::FormFillingAndAnnotation,
        ] {
            assert_eq!(
                comparison.against(level),
                Judgement::NotClassified { level, objects: 4 },
                "{level:?}"
            );
        }
    }

    /// The catalog of a document that holds §12.7.7's template, out of the page tree's reach.
    const CATALOG_WITH_A_TEMPLATE: &str = "<< /Type /Catalog /Pages 2 0 R \
         /Perms << /DocMDP 4 0 R >> /AcroForm << /Fields [5 0 R] /SigFlags 3 >> \
         /Names << /Templates << /Names [(Schedule) 6 0 R] >> >> >>";

    /// §12.7.7's invisible page: "an object type of Template rather than Page … no Parent or B".
    const TEMPLATE: &str = "<< /Type /Template /MediaBox [0 0 10 10] /Contents 7 0 R >>";

    /// The content that tells that template from any other page.
    const TEMPLATE_CONTENT: &str = "<< /Length 9 >>\nstream\n0 0 10 10\nendstream";

    /// Instantiating a page template is Table 257's level 2, and it is told apart by §12.7.7.
    ///
    /// §12.8.2.2.2's Table 257 states three operations in its second value, and this is the one
    /// that had no classifier until now:
    ///
    /// > 2 Permitted changes shall be filling in forms, instantiating page templates, and
    /// > signing; other changes shall invalidate the signature.
    ///
    /// The update does what §12.7.7 says the operation does — a script "can add the named page to
    /// the current document as a regular page" — so a page appears in the tree carrying the content of
    /// a page the signed revision kept in its `/Templates`, and the `/Pages` node that received it
    /// gains a kid and a count. Both are part of the one operation and both are permitted at 2.
    #[test]
    fn a_page_instantiated_from_a_template_is_what_level_two_permits() {
        let mut bytes = signed_file_under(CATALOG_WITH_A_TEMPLATE, &[TEMPLATE, TEMPLATE_CONTENT]);
        update(
            &mut bytes,
            &[
                (2, "<< /Type /Pages /Count 2 /Kids [3 0 R 8 0 R] >>"),
                (
                    8,
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 7 0 R >>",
                ),
            ],
            &[],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(
            detail(&comparison, Modification::FormFilling),
            [
                "2 TemplateInstantiated Permitted",
                "8 TemplateInstantiated Permitted"
            ]
        );
        for level in [
            Modification::FormFilling,
            Modification::FormFillingAndAnnotation,
        ] {
            assert_eq!(
                comparison.against(level),
                Judgement::WithinWhatIsPermitted {
                    level,
                    objects: 2,
                    disregarded: 0,
                },
                "{level:?}"
            );
        }
        // "1 No changes to the document shall be permitted", and this is a change.
        assert_eq!(
            comparison.against(Modification::None),
            Judgement::NotPermitted {
                level: Modification::None,
                objects: 2,
            }
        );
    }

    /// A page whose content is nobody's template is refused, which is what makes the test above
    /// mean something.
    ///
    /// **The defect this is calibrated against is the whole risk of the classifier**: a reader
    /// that ranked "a page was added at level 2" as an instantiation would wave through any page
    /// composed after signing, which is the change a certification exists to catch. The two tests
    /// are the same update differing in one object's content, and the answers differ with it.
    #[test]
    fn a_page_added_that_is_no_templates_is_refused_rather_than_ranked_as_one() {
        let mut bytes = signed_file_under(CATALOG_WITH_A_TEMPLATE, &[TEMPLATE, TEMPLATE_CONTENT]);
        update(
            &mut bytes,
            &[
                (2, "<< /Type /Pages /Count 2 /Kids [3 0 R 8 0 R] >>"),
                (
                    8,
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 9 0 R >>",
                ),
                (9, "<< /Length 9 >>\nstream\n1 1 11 11\nendstream"),
            ],
            &[],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(
            detail(&comparison, Modification::FormFilling),
            [
                "2 Unclassified Unrankable",
                "8 Unclassified Unrankable",
                "9 Unclassified Unrankable"
            ]
        );
        for level in [
            Modification::FormFilling,
            Modification::FormFillingAndAnnotation,
        ] {
            assert_eq!(
                comparison.against(level),
                Judgement::NotClassified { level, objects: 3 },
                "{level:?}"
            );
        }
    }

    /// A template instantiated by *copying* its content is the same operation, and is ranked so.
    ///
    /// §12.7.7 says the operation adds the named page and says nothing about whether an
    /// implementation shares the content stream or copies it, so a reader that insisted on the
    /// shared object would refuse half the conforming instantiations there are. The copy is object
    /// 9 here, byte for byte object 7 — and it is ranked as part of the page it draws rather than
    /// on its own evidence, which it has none of.
    #[test]
    fn a_template_instantiated_by_copying_its_content_is_the_same_operation() {
        let mut bytes = signed_file_under(CATALOG_WITH_A_TEMPLATE, &[TEMPLATE, TEMPLATE_CONTENT]);
        update(
            &mut bytes,
            &[
                (2, "<< /Type /Pages /Count 2 /Kids [3 0 R 8 0 R] >>"),
                (
                    8,
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 9 0 R >>",
                ),
                (9, TEMPLATE_CONTENT),
            ],
            &[],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(
            detail(&comparison, Modification::FormFilling),
            [
                "2 TemplateInstantiated Permitted",
                "8 TemplateInstantiated Permitted",
                "9 TemplateInstantiated Permitted"
            ]
        );
    }

    /// A document with no `/Templates` tree has instantiated nothing, whatever it added.
    ///
    /// The other control: the same update as the first test, over a file whose catalog states no
    /// name tree. Without it the classifier would be reading the *page* and not the template, and
    /// the first test would pass for the wrong reason.
    #[test]
    fn a_page_added_where_the_signed_revision_held_no_template_is_refused() {
        let mut bytes = signed_file(&[TEMPLATE, TEMPLATE_CONTENT]);
        update(
            &mut bytes,
            &[
                (2, "<< /Type /Pages /Count 2 /Kids [3 0 R 8 0 R] >>"),
                (
                    8,
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 7 0 R >>",
                ),
            ],
            &[],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(
            comparison.against(Modification::FormFilling),
            Judgement::NotClassified {
                level: Modification::FormFilling,
                objects: 2,
            }
        );
    }

    /// A page tree node that *lost* a kid is not part of an instantiation, whatever else it did.
    ///
    /// Table 257 permits instantiating a page template; it permits nothing that removes a page.
    /// The plant is the first test's update with object 3 dropped from `/Kids`, which is a page
    /// deleted under cover of a page added.
    #[test]
    fn a_page_tree_node_that_lost_a_page_is_refused_even_beside_an_instantiation() {
        let mut bytes = signed_file_under(CATALOG_WITH_A_TEMPLATE, &[TEMPLATE, TEMPLATE_CONTENT]);
        update(
            &mut bytes,
            &[
                (2, "<< /Type /Pages /Count 1 /Kids [8 0 R] >>"),
                (
                    8,
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 7 0 R >>",
                ),
            ],
            &[],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(
            detail(&comparison, Modification::FormFilling),
            [
                "2 Unclassified Unrankable",
                "8 TemplateInstantiated Permitted"
            ]
        );
        assert_eq!(
            comparison.against(Modification::FormFilling),
            Judgement::NotClassified {
                level: Modification::FormFilling,
                objects: 1,
            }
        );
    }

    /// The transform every `FieldMDP` test here starts from: `/Data` naming the whole document.
    ///
    /// Object 1 is the catalog, which is what `xfa_filled_imm1344e.pdf`'s own `FieldMDP` states —
    /// so the fixture's scope is the corpus's, and not a shape invented for the test.
    fn transform(selection: FieldSelection) -> FieldMdp {
        FieldMdp {
            selection,
            data: Some(pdf_syntax::ObjectId::new(1, 0)),
        }
    }

    /// A catalog whose interactive form holds the signature field **and** a field to fill in.
    ///
    /// Table 224's `/Fields` is "[a]n array of references to the document's root fields", so a
    /// field the array does not reach is no part of the form a `/Data` scopes an analysis to —
    /// which is why the fixture states it rather than leaving the field loose in the file.
    const CATALOG_WITH_A_FIELD: &str = "<< /Type /Catalog /Pages 2 0 R \
         /Perms << /DocMDP 4 0 R >> /AcroForm << /Fields [5 0 R 6 0 R] /SigFlags 3 >> >>";

    /// The update every `FieldMDP` test here ranks: the field named `Name` filled in.
    fn a_filled_field() -> Vec<u8> {
        let mut bytes = signed_file_under(CATALOG_WITH_A_FIELD, &[EMPTY_FIELD]);
        update(
            &mut bytes,
            &[(
                6,
                "<< /FT /Tx /T (Name) /Subtype /Widget /Type /Annot /Rect [100 200 300 220] \
                 /P 3 0 R /V (Ada) >>",
            )],
            &[],
            1,
        );
        bytes
    }

    /// What `against_field_mdp` said about each object, as text a failure can name.
    fn scoped(comparison: &Comparison, transform: &FieldMdp) -> Vec<String> {
        comparison
            .against_field_mdp(transform)
            .expect("a transform scoped to the catalog")
            .detail
            .iter()
            .map(
                |ScopedTo {
                     number,
                     field,
                     included,
                 }| {
                    format!("{number} {} {included:?}", field.as_deref().unwrap_or("-"))
                },
            )
            .collect()
    }

    /// Table 259's `/Fields` selects, and it selects **both ways** over one update.
    ///
    /// This is §12.8.2.1's sentence made to do something — "[t]ransform methods, along with
    /// transform parameters, shall determine which objects are included and excluded in revision
    /// comparison" — and the pair is the calibration (trap 13). One update, one changed field, and
    /// four selections: the two that name it answer that a covered field moved, the two that do
    /// not answer that none did. A reader that ignored `/Fields` would give one answer to all four.
    #[test]
    fn a_field_mdp_transform_selects_by_the_fields_table_259_names() {
        let document = Document::open(a_filled_field()).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        // "All All form fields." — and the field this update wrote is one.
        for selection in [
            FieldSelection::All,
            FieldSelection::Include(vec!["Name".to_owned()]),
            FieldSelection::Exclude(vec!["Other".to_owned()]),
        ] {
            let transform = transform(selection.clone());
            assert_eq!(scoped(&comparison, &transform), ["6 Name Covered"]);
            assert_eq!(
                comparison
                    .against_transform(&transform)
                    .expect("a scoped transform"),
                FieldJudgement::CoveredFieldChanged { objects: 1 },
                "{selection:?}"
            );
        }

        // "Include Only those form fields that specified in Fields" and "Exclude Only those form
        // fields not specified in Fields" (ISO 32000-1's Table 256, which the 2.0 conversion drops
        // — `FieldSelection` says where both readings come from). Neither names this field.
        for selection in [
            FieldSelection::Include(vec!["Other".to_owned()]),
            FieldSelection::Exclude(vec!["Name".to_owned()]),
        ] {
            let transform = transform(selection.clone());
            assert_eq!(scoped(&comparison, &transform), ["6 Name Outside"]);
            assert_eq!(
                comparison
                    .against_transform(&transform)
                    .expect("a scoped transform"),
                FieldJudgement::NoCoveredFieldChanged {
                    outside: 1,
                    excluded: 0,
                },
                "{selection:?}"
            );
        }
    }

    /// An object that is no form field is excluded from this analysis, not permitted by it.
    ///
    /// §12.8.2.4's transform detects "changes to the values of a list of form fields", so a page
    /// rewritten after signing is outside its subject entirely — which is the half of §12.8.2.1's
    /// sentence about objects *excluded*. Table 257's ranking is where that object is answered,
    /// and it refuses it: the two transforms are asked separately because the standard states them
    /// separately.
    #[test]
    fn a_change_that_is_no_form_field_is_excluded_from_the_transforms_analysis() {
        let mut bytes = signed_file_under(CATALOG_WITH_A_FIELD, &[EMPTY_FIELD]);
        update(
            &mut bytes,
            &[(3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] >>")],
            &[],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        let transform = transform(FieldSelection::All);
        assert_eq!(scoped(&comparison, &transform), ["3 - Excluded"]);
        assert_eq!(
            comparison
                .against_transform(&transform)
                .expect("a scoped transform"),
            FieldJudgement::NoCoveredFieldChanged {
                outside: 0,
                excluded: 1,
            }
        );
        assert_eq!(
            comparison.against(Modification::FormFilling),
            Judgement::NotClassified {
                level: Modification::FormFilling,
                objects: 1,
            }
        );
    }

    /// A `/Data` naming one field scopes the analysis to that field's subtree and no further.
    ///
    /// Table 256's entry is "[a]n indirect reference to the object in the document upon which the
    /// object modification analysis should be performed", so what it names is the subject — and a
    /// `/Data` naming the signature field cannot reach the text field beside it, whatever
    /// `/Action /All` would otherwise cover.
    #[test]
    fn a_data_naming_one_field_scopes_the_analysis_to_it() {
        let document = Document::open(a_filled_field()).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        let narrowed = FieldMdp {
            selection: FieldSelection::All,
            // Object 5 is the signature field, which the changed field 6 is not under.
            data: Some(pdf_syntax::ObjectId::new(5, 0)),
        };
        assert_eq!(scoped(&comparison, &narrowed), ["6 Name Excluded"]);
        assert_eq!(
            comparison
                .against_transform(&narrowed)
                .expect("a scoped transform"),
            FieldJudgement::NoCoveredFieldChanged {
                outside: 0,
                excluded: 1,
            }
        );
    }

    /// A transform with no subject is refused by name rather than ranked over everything.
    ///
    /// Table 256 makes `/Data` required for this method and for no other, so a `FieldMDP` without
    /// one has not said what its analysis runs on — and the standing rule of this module is that a
    /// missing input is a refusal rather than a default.
    #[test]
    fn a_field_mdp_transform_with_no_subject_is_refused_by_name() {
        let document = Document::open(a_filled_field()).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        assert_eq!(
            comparison
                .against_field_mdp(&FieldMdp {
                    selection: FieldSelection::All,
                    data: None,
                })
                .expect_err("a transform with no /Data is not scoped"),
            NotScoped::NoData
        );
        assert_eq!(
            comparison
                .against_field_mdp(&FieldMdp {
                    selection: FieldSelection::All,
                    // The page tree root: an object in the document and no part of the form.
                    data: Some(pdf_syntax::ObjectId::new(2, 0)),
                })
                .expect_err("a /Data naming no part of the form is not scoped"),
            NotScoped::DataIsNotTheForm(2)
        );
    }

    /// A widget with no `/T` of its own is its parent field's, which is what §12.7.4.2 says it is.
    ///
    /// > A field dictionary that does not have a partial field name ( T entry) of its own shall
    /// > not be considered a field but simply a Widget annotation.
    ///
    /// So a transform naming `Name` reaches the widget that draws it, and the defect this is
    /// calibrated against is a reader that looked for a `/T` on the changed object, found none,
    /// and excluded from the analysis the very object the update rewrote.
    #[test]
    fn a_widget_with_no_name_of_its_own_belongs_to_the_field_above_it() {
        let mut bytes = signed_file_under(
            CATALOG_WITH_A_FIELD,
            &[
                "<< /FT /Tx /T (Name) /Kids [7 0 R] >>",
                "<< /Subtype /Widget /Type /Annot /Rect [100 200 300 220] /Parent 6 0 R \
                 /P 3 0 R >>",
            ],
        );
        update(
            &mut bytes,
            &[(
                7,
                "<< /Subtype /Widget /Type /Annot /Rect [100 200 300 220] /Parent 6 0 R \
                 /P 3 0 R /AS /Off >>",
            )],
            &[],
            1,
        );
        let document = Document::open(bytes).expect("a valid file");
        let comparison =
            Comparison::of(&only_signature(&document), &document).expect("two comparable states");

        let names_it = transform(FieldSelection::Include(vec!["Name".to_owned()]));
        let does_not = transform(FieldSelection::Include(vec!["Other".to_owned()]));
        assert_eq!(scoped(&comparison, &names_it), ["7 Name Covered"]);
        assert_eq!(scoped(&comparison, &does_not), ["7 Name Outside"]);
        assert_eq!(
            comparison.against_transform(&names_it),
            Ok(FieldJudgement::CoveredFieldChanged { objects: 1 })
        );
        assert_eq!(
            comparison.against_transform(&does_not),
            Ok(FieldJudgement::NoCoveredFieldChanged {
                outside: 1,
                excluded: 0,
            })
        );
    }

    /// A range whose hole holds more than the signature value is refused rather than compared.
    ///
    /// This is the case the refusal exists for: the prefix would parse, the comparison would run,
    /// and the two states would agree over bytes that no digest was taken across. Moving the
    /// hole's near edge back over four bytes of the `/Contents` key is the whole plant — and it
    /// has to be four rather than one, because §7.3.4.3's whitespace "shall be ignored" and a
    /// hole widened over a space alone still holds the value and nothing else.
    #[test]
    fn a_range_hiding_more_than_its_value_is_refused_by_name() {
        let mut bytes = signed_file(&[]);
        let text = String::from_utf8(bytes.clone()).expect("ascii");
        let at = text.find("/ByteRange [").expect("a range") + "/ByteRange [".len();
        let second_at = at.saturating_add(11);
        let second: usize = text[second_at..second_at + 10].parse().expect("a number");
        // Four bytes earlier: the hole then holds "ts <…>", which is the `/Contents` key's own
        // letters — content a reader parses and no digest was taken over.
        let widened = format!("{:010}", second.saturating_sub(4));
        bytes.splice(
            second_at..second_at.saturating_add(10),
            widened.into_bytes(),
        );

        let document = Document::open(bytes).expect("a valid file");
        let refusal = Comparison::of(&only_signature(&document), &document)
            .expect_err("a hole holding more than the value is not comparable");
        assert!(
            matches!(refusal, NotComparable::MoreThanTheValueIsUnsigned(_)),
            "{refusal:?}"
        );
    }

    /// A range stopping short of `%%EOF` names no revision, and is refused rather than guessed at.
    #[test]
    fn a_range_stopping_inside_a_revision_is_refused_by_name() {
        let mut bytes = signed_file(&[]);
        let text = String::from_utf8(bytes.clone()).expect("ascii");
        let at = text.find("/ByteRange [").expect("a range") + "/ByteRange [".len();
        let last_at = at.saturating_add(33);
        let last: usize = text[last_at..last_at + 10].parse().expect("a number");
        let shortened = format!("{:010}", last.saturating_sub(9));
        bytes.splice(last_at..last_at.saturating_add(10), shortened.into_bytes());

        let document = Document::open(bytes).expect("a valid file");
        let refusal = Comparison::of(&only_signature(&document), &document)
            .expect_err("a range stopping mid-revision is not comparable");
        assert!(
            matches!(refusal, NotComparable::NotARevisionBoundary(_)),
            "{refusal:?}"
        );
    }
}

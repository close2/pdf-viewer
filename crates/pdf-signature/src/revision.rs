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

use crate::signature::{Coverage, Excluded, Modification, Signature, SignedEnd};

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
/// **One of Table 257's own permitted operations has no variant here.** "[I]nstantiating page
/// templates" (§12.7.6) is a change to the page tree that this reader does not tell apart from
/// any other change to the page tree, so a document that instantiated one comes back
/// [`Kind::Unclassified`] — refused rather than permitted. Named so that a later round finds it.
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
        Ok(Self {
            signed,
            end,
            updates_after,
            changes,
            tally,
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
    let mut kinds: BTreeMap<u32, Kind> = BTreeMap::new();
    let mut of_a_field: BTreeSet<u32> = BTreeSet::new();
    let mut of_an_annotation: BTreeSet<u32> = BTreeSet::new();

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
        let kind = classify_one(signed, current, number, bucket, &material, tables, offset);
        if let Some(dict) = dictionary_of(&held.get(ObjectId::new(number, 0))) {
            let owned = appearances(held, &dict);
            match kind {
                Kind::FieldFilledIn | Kind::Signing => of_a_field.extend(owned),
                Kind::Annotation => of_an_annotation.extend(owned),
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
        }
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
fn classify_one(
    signed: &Document,
    current: &Document,
    number: u32,
    bucket: Bucket,
    material: &BTreeSet<u32>,
    tables: &BTreeSet<usize>,
    offset: Option<usize>,
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
        Bucket::Added | Bucket::Removed => match (field.as_deref(), is_annotation(&dict)) {
            (Some("Sig"), _) => Kind::Signing,
            (None, true) => Kind::Annotation,
            _ => Kind::Unclassified,
        },
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
            Kind::FieldFilledIn | Kind::Signing | Kind::FieldAppearance => Verdict::Permitted,
            Kind::Annotation | Kind::AnnotationAppearance => Verdict::NotPermitted,
            _ => Verdict::Unrankable,
        },
        // "3 Permitted changes shall be the same as for 2, as well as annotation creation,
        // deletion, and modification; other changes shall invalidate the signature."
        Modification::FormFillingAndAnnotation => match kind {
            Kind::FieldFilledIn
            | Kind::Signing
            | Kind::FieldAppearance
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

    use super::{Comparison, Judgement, NotComparable, Ranked};
    use crate::signature::{Modification, Signature, signatures};

    /// The signature object every fixture here carries, with a `/ByteRange` to be filled in.
    ///
    /// Written at a fixed width so that the four numbers can be patched in place once the file
    /// is assembled and its own offsets are known — which is what a producer does, and the only
    /// way a fixture's range can name its own bytes rather than a guess.
    const SIGNATURE: &str = "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
                             /ByteRange [0000000000 0000000000 0000000000 0000000000] \
                             /Contents <00112233445566778899aabbccddeeff> >>";

    /// A one-revision document ending at `%%EOF`, with the signature's range naming its own bytes.
    ///
    /// Object 1 is the catalog, 2 the page tree, 3 a page and 4 the signature; a caller adds more
    /// after those.
    fn signed_file(extra: &[&str]) -> Vec<u8> {
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R /Perms << /DocMDP 4 0 R >> \
             /AcroForm << /Fields [5 0 R] /SigFlags 3 >> >>",
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

//! ISO 32000-2 §12.8.2.2.2's second step: the signed revision of a file, beside the current one.
//!
//! # The clause, and the two steps it is
//!
//! > To validate a signature that uses the DocMDP transform method, a PDF processor first shall
//! > verify the byte range digest. Next, it shall verify that any modifications that have been
//! > made to the document are permitted by the transform parameters.
//!
//! [`crate::signature::Signature::integrity`] is the first step and has been since ADR 0215.
//! This module is the *foundation* of the second, and not the second: it reconstructs the two
//! states the clause names and says, object by object, how they differ. What it deliberately does
//! not do is rank a difference against Table 257's `/P` — [`Judgement`] refuses that by name.
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
//! # Nothing here says "permitted"
//!
//! A `/DocMDP` answer that says permitted without having compared is worse than one that refuses,
//! because the whole point of the transform is to be believed. Every way this comparison can fail
//! to be a comparison is therefore a named [`NotComparable`] rather than a lenient default, and
//! the ranking itself is [`Judgement::NotClassified`], which names what it has not done and what
//! it would take. The one thing this module will assert is [`Judgement::NoChangeToRank`] — the
//! current file's objects are the signed revision's objects, at the same places, under the same
//! catalogue — and even that is a statement about objects rather than a verdict on a signature.
//!
//! ADR 1043.

use std::collections::BTreeSet;

use pdf_syntax::xref::{self, Location, XrefTable};
use pdf_syntax::{Document, Object, SyntaxError};

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

/// What Table 257's `/P` says about a set of changes — or, here, what this program will not say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judgement {
    /// There is no change to rank: the current file's objects are the signed revision's objects,
    /// at the same places, under the same catalog.
    ///
    /// **Not a verdict on the signature.** §12.8.2.2.2 makes the byte range digest step one and
    /// this step two; a caller that has not asked [`Signature::integrity`] has established
    /// nothing, and even a caller that has is still owed §12.8.1's third question, which this
    /// program does not answer at all.
    NoChangeToRank,
    /// **Refused.** Objects differ, and this program does not decide which of Table 257's levels
    /// admits which difference.
    ///
    /// What the decision needs, and what this variant therefore stands in for: each changed
    /// object resolved to what it *is* in the document — a field's value, an annotation, a page's
    /// content stream, a DSS or a document timestamp — and ranked against the level's own
    /// vocabulary, which Table 257 states as "filling in forms, instantiating page templates, and
    /// signing" for 2 and those "as well as annotation creation, deletion, and modification" for
    /// 3. The table also carves out a whole class of update from the question — "[c]hanges to a
    /// PDF that are incremental updates which include only the data necessary to add DSS's …
    /// and/or document timestamps … to the document shall not be considered as changes to the
    /// document" — which is a
    /// judgement about a *revision* rather than about an object, and is why
    /// [`Comparison::updates_after`] is counted here before anything is ranked.
    NotClassified {
        /// The level the author stated, which a ranking would be against.
        level: Modification,
        /// How many objects differ, over all four of [`Changes`]'s buckets.
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
        let updates_after = xref::sections(file, current.limits())
            .iter()
            .filter(|section| u64::try_from(section.offset).unwrap_or(u64::MAX) >= end)
            .count();
        let changes = compare(&signed, current, end);
        Ok(Self {
            signed,
            end,
            updates_after,
            changes,
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

    /// What Table 257's `level` says about those differences — or what this program will not say.
    #[must_use]
    pub fn against(&self, level: Modification) -> Judgement {
        if self.changes.is_empty() {
            return Judgement::NoChangeToRank;
        }
        Judgement::NotClassified {
            level,
            objects: self
                .changes
                .added
                .count()
                .saturating_add(self.changes.redefined.count())
                .saturating_add(self.changes.removed.count())
                .saturating_add(self.changes.unplaceable.count()),
        }
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
fn compare(signed: &Document, current: &Document, end: u64) -> Changes {
    let before = signed.xref();
    let after = current.xref();
    let mut changes = Changes {
        catalog_moved: !same_reference(before.trailer().get("Root"), after.trailer().get("Root")),
        ..Changes::default()
    };
    let in_the_signed_revision: BTreeSet<u32> = before.object_numbers().collect();
    for number in after.object_numbers() {
        let here = placement(after, number);
        if !in_the_signed_revision.contains(&number) {
            changes.added.push(number);
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
                }
            }
            (Some(_), Some(_)) => changes.redefined.push(number),
            _ => changes.unplaceable.push(number),
        }
    }
    for number in before.object_numbers() {
        if after.location(number).is_none() {
            changes.removed.push(number);
        }
    }
    changes
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

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use pdf_syntax::Document;

    use super::{Comparison, Judgement, NotComparable};
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
        assert_eq!(
            comparison.against(Modification::None),
            Judgement::NotClassified {
                level: Modification::None,
                objects: 3,
            },
            "the levels are not ranked and the refusal says so"
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
        assert!(matches!(
            comparison.against(Modification::FormFillingAndAnnotation),
            Judgement::NotClassified { .. }
        ));
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

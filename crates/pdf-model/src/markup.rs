//! ISO 32000-2 §12.5.6.2: the entries a *group* of markup annotations shares.
//!
//! A file may say that several annotations are one thing. The clause builds that out of two
//! entries in Table 172 — `/IRT`, which names another annotation, and `/RT`, which says what the
//! relationship is:
//!
//! > In PDF 1.6, a set of annotations may be grouped so that they function as a single unit when
//! > a user interacts with them. The group consists of a primary annotation, which shall not have
//! > an IRT entry, and one or more subordinate annotations, which shall have an IRT entry that
//! > refers to the primary annotation and an RT entry whose value is Group .
//!
//! and then says what a subordinate's own copies of nine entries are worth:
//!
//! > Some entries in the primary annotation are treated as "group attributes" that shall apply to
//! > the group as a whole; the corresponding entries in the subordinate annotations shall be
//! > ignored. These entries are Contents (or RC and DS ), M , C , T , Popup , CreationDate ,
//! > Subj , and Open .
//!
//! [`group_source`] is that sentence: the dictionary one of those entries shall be read from.
//! Everything else — `/Rect`, `/F`, `/AP`, `/Subtype`, `/QuadPoints` — stays the annotation's own,
//! which is why this is a function taking a key's *owner* rather than a dictionary that replaces
//! the annotation. A subordinate is still drawn where it says it is, in the shape it says.
//!
//! # What this is not
//!
//! **`/RT`'s other value is `R` and this is not about it.** "Default value: R ", so an annotation
//! with an `/IRT` and no `/RT` is a *reply* rather than a group member, and the clause's `shall`
//! for those is a different one — "[i]nteractive PDF processors shall not display replies to an
//! annotation individually but together in the form of threaded comments" — which asks for a
//! threading this program has no panel for. §12.5.6.2's ledger row carries that half; nothing
//! here treats a reply as a group, and [`group_source`] returns the annotation's own dictionary
//! for one.
//!
//! # One hop, which is the clause's own shape
//!
//! The primary "shall not have an IRT entry", so a group is flat and the walk is a single
//! resolution — no chain to follow and no cycle to guard against. Where a file names a primary
//! that itself states an `/IRT`, the entry still "refers to the primary annotation" as far as that
//! file is concerned, and its entries are the ones taken: reading further would be inventing a
//! hierarchy the clause does not describe.
//!
//! # Nine attributes, ten keys, seven readers
//!
//! The clause writes the first attribute as "Contents (or RC and DS )", so the list of *keys* is
//! `/Contents`, `/RC`, `/DS`, `/M`, `/C`, `/T`, `/Popup`, `/CreationDate`, `/Subj` and `/Open`.
//! Seven of them have a reader in this tree and each of those reads through [`group_source`]:
//! `crate::popup` for the window's title, text, date and colour and for the two entries that open
//! it, `crate::appearance` for the `/C` that is ink and the `/Contents` a free text annotation
//! draws. The other three have no reader to correct — `/DS` is Table 177's default style string,
//! which is XFA's rich-text format and which `CLAUDE.md` principle 5 excludes, and `/Subj` and
//! `/CreationDate` reach a comments panel this program does not have.
//!
//! # Measured
//!
//! `examples/annotation_group_census` over the 964 openable corpus documents finds **one**
//! `/IRT` in 34 835 annotations, and over ISO 32000-2's own PDF **2074** in 11 462 — 322 of them
//! `/RT /Group`, every one naming a primary on the same page, and 213 stating a `/Popup` of their
//! own that this clause says to ignore.

use std::borrow::Cow;

use pdf_syntax::{Dictionary, Document};

/// The dictionary §12.5.6.2 says one of the **group attributes** shall be read from.
///
/// The primary annotation for a subordinate in a group; the annotation itself for everything
/// else, which is every annotation in almost every file. Those attributes, and only those, are
/// the ten keys of [`GROUP_ATTRIBUTES`] — a caller reading anything else from the result would be
/// reading the wrong annotation's, which is the mistake this function exists to fix in the other
/// direction.
///
/// # Cost
///
/// One `Dictionary::get` on the common path, before anything is resolved: an annotation with no
/// `/IRT` at all is the overwhelming majority — 34 834 of the corpus's 34 835 — and cannot be a
/// subordinate. The clone happens only for the annotations that are, where a window's text is
/// being assembled for a person to read rather than a page rasterised.
pub(crate) fn group_source<'a>(
    document: &Document,
    annotation: &'a Dictionary,
) -> Cow<'a, Dictionary> {
    // Cheapest first: the entry the clause makes required of a subordinate, unresolved.
    if annotation.get("IRT").is_none() {
        return Cow::Borrowed(annotation);
    }
    // "an RT entry whose value is Group". `R` — the default — is a reply, and a reply shares
    // nothing: the clause gives group attributes to a group.
    if document
        .get_key(annotation, "RT")
        .as_name()
        .map(pdf_syntax::Name::as_bytes)
        != Some(b"Group")
    {
        return Cow::Borrowed(annotation);
    }
    match document.get_key(annotation, "IRT").as_dict() {
        Some(primary) => Cow::Owned(primary.clone()),
        // A subordinate whose `/IRT` names nothing is a file contradicting the clause it is
        // invoking. Its own entries are all there is, which is what a reader with no group can
        // show — and no report, because nothing is drawn differently from a file that never
        // claimed a group at all.
        None => Cow::Borrowed(annotation),
    }
}

/// The ten keys §12.5.6.2 makes group attributes, as the module header lists them.
///
/// Here so that a test can assert what is *not* on the list, which is the half of the rule a
/// reader gets wrong by being too eager.
#[cfg(test)]
const GROUP_ATTRIBUTES: [&str; 10] = [
    "Contents",
    "RC",
    "DS",
    "M",
    "C",
    "T",
    "Popup",
    "CreationDate",
    "Subj",
    "Open",
];

#[cfg(test)]
mod tests {
    use pdf_syntax::Document;

    use super::{GROUP_ATTRIBUTES, group_source};

    /// A one-page document with the annotations the caller spells, as PDF bytes.
    fn document(annotations: &str, objects: &str) -> Document {
        let body = format!(
            "%PDF-1.7\n\
             1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n\
             2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
             3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [{annotations}] \
             >> endobj\n\
             {objects}\
             trailer << /Root 1 0 R /Size 32 >>\n"
        );
        Document::open(body.into_bytes()).expect("the fixture parses")
    }

    /// The annotation with this object number, as a dictionary.
    fn annotation(document: &Document, number: u32) -> pdf_syntax::Dictionary {
        document
            .get(pdf_syntax::ObjectId::new(number, 0))
            .as_dict()
            .expect("an annotation dictionary")
            .clone()
    }

    /// A **pair** differing only in `/RT`, because that is the whole rule.
    ///
    /// `/RT /Group` makes the annotation a subordinate whose group attributes are the primary's;
    /// `/RT /R` — and an absent `/RT`, which defaults to it — makes it a reply, which shares
    /// nothing. Verified in both directions: without the second half a reader that ignored `/RT`
    /// entirely would pass.
    #[test]
    fn only_a_group_reply_type_takes_the_primarys_entries() {
        for (reply_type, shared) in [("/RT /Group ", true), ("/RT /R ", false), ("", false)] {
            let document = document(
                "4 0 R 5 0 R",
                &format!(
                    "4 0 obj << /Type /Annot /Subtype /Caret /Rect [10 10 30 30] \
                     /Contents (the primary's) /T (the author) /C [1 0 0] >> endobj\n\
                     5 0 obj << /Type /Annot /Subtype /StrikeOut /Rect [10 10 90 30] \
                     /IRT 4 0 R {reply_type}/Contents (the subordinate's) /C [0 1 0] >> endobj\n"
                ),
            );
            let subordinate = annotation(&document, 5);
            let source = group_source(&document, &subordinate);
            let contents = document.get_key(&source, "Contents");
            let expected: &[u8] = if shared {
                b"the primary's"
            } else {
                b"the subordinate's"
            };
            assert_eq!(
                contents.as_string(),
                Some(expected),
                "with {reply_type:?} before /Contents"
            );
        }
    }

    /// An entry outside the clause's list stays the annotation's own.
    ///
    /// The list is nine attributes and this is what makes it a list: a subordinate is drawn in its
    /// own place, in its own shape, with its own flags. `/Rect` is the sharpest of those — the
    /// primary's is a caret's insertion point and the subordinate's spans the struck-out text.
    #[test]
    fn a_subordinate_keeps_everything_the_clause_does_not_name() {
        let document = document(
            "4 0 R 5 0 R",
            "4 0 obj << /Type /Annot /Subtype /Caret /Rect [10 10 30 30] /F 4 >> endobj\n\
             5 0 obj << /Type /Annot /Subtype /StrikeOut /Rect [10 10 90 30] /IRT 4 0 R \
             /RT /Group /F 132 >> endobj\n",
        );
        let subordinate = annotation(&document, 5);
        assert!(!GROUP_ATTRIBUTES.contains(&"Rect"));
        assert!(!GROUP_ATTRIBUTES.contains(&"F"));
        assert!(!GROUP_ATTRIBUTES.contains(&"Subtype"));
        // Read from the annotation, not from `group_source`, which is the point: these keys are
        // not the clause's to move.
        assert_eq!(
            document.get_key(&subordinate, "F"),
            pdf_syntax::Object::Integer(132)
        );
    }

    /// A subordinate whose `/IRT` resolves to nothing falls back to its own entries.
    #[test]
    fn a_group_with_no_primary_is_its_own_source() {
        let document = document(
            "5 0 R",
            "5 0 obj << /Type /Annot /Subtype /StrikeOut /Rect [10 10 90 30] /IRT 99 0 R \
             /RT /Group /Contents (alone) >> endobj\n",
        );
        let subordinate = annotation(&document, 5);
        let source = group_source(&document, &subordinate);
        assert_eq!(
            document.get_key(&source, "Contents").as_string(),
            Some(&b"alone"[..])
        );
    }

    /// A primary that names a primary of its own is followed exactly once.
    ///
    /// The clause says a primary "shall not have an IRT entry", so a file with a chain has already
    /// left it; one hop is what the sentence describes, and it is also why no cycle is reachable
    /// from here.
    #[test]
    fn the_walk_is_one_hop_and_cannot_cycle() {
        let document = document(
            "4 0 R 5 0 R",
            "4 0 obj << /Type /Annot /Subtype /Caret /Rect [10 10 30 30] /IRT 5 0 R /RT /Group \
             /Contents (the middle) >> endobj\n\
             5 0 obj << /Type /Annot /Subtype /StrikeOut /Rect [10 10 90 30] /IRT 4 0 R \
             /RT /Group /Contents (the start) >> endobj\n",
        );
        let subordinate = annotation(&document, 5);
        let source = group_source(&document, &subordinate);
        assert_eq!(
            document.get_key(&source, "Contents").as_string(),
            Some(&b"the middle"[..]),
            "one hop, and the second /IRT is not followed"
        );
    }
}

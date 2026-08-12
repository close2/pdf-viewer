//! What a document asserts about what its reader may do — read, never decided.
//!
//! Four clauses put a restriction on the person holding the file: §7.6.4.2's Table 22, which a
//! security handler encrypts into the document; §12.8.2.2's `/DocMDP`, which an author's
//! certification signature states; §12.8.6's permissions dictionary, which is what makes the
//! second binding rather than advisory; and §12.7.5.5's signature field lock, which is the only
//! one of the four addressed to a **named field** rather than to the document. This module reads
//! all four and answers one question —
//! *what does this document assert about this operation* — and it answers it with **reasons**
//! rather than with a verdict.
//!
//! # Why a reason and not a boolean
//!
//! The verdict is not this crate's to give. `CLAUDE.md`'s "A document's restrictions are the
//! reader's to set, and they have levels" makes how much of somebody else's file a person's own
//! program obeys a *policy*, supplied by whoever is running the program; the four levels the
//! project owner named — `off`, `on`, ask before the operation, warn before the operation — all
//! need the operation to be **describable** before it happens, and two of them need it to be
//! describable to a person. A function answering `false` has thrown away everything the question
//! would need. So [`asserted`] hands back every restriction that applies, each naming its clause,
//! and `viewer_core` decides what its reader does with them.
//!
//! # Every restriction that applies, not the first
//!
//! §12.8.6 states the composition rule outright, and it is why this returns a list:
//!
//! > For a permission to be actually granted for a document, it shall be allowed by each
//! > permission handler that is present in the permissions dictionary as well as by the security
//! > handler.
//!
//! Two handlers can withhold one operation for two different reasons, and a person being asked
//! whether to go ahead is owed both.

use pdf_syntax::{Document, Permissions};

use crate::signature::Modification;

/// One thing a reader does to a document that a clause can restrict.
///
/// One variant per verb this program has and that a clause names, which is deliberately not the
/// whole of Table 22: an operation nothing here performs is an operation no restriction can bite
/// on, and an enum arm for it would claim otherwise. The same discipline
/// [`crate::signature::Right`] follows.
///
/// **What is missing and why**, because the absences are decisions rather than gaps:
///
/// - **Printing** (Table 22 bits 3 and 12) and **assembling** (bit 11) name operations this
///   program does not have at all. That is a capability rather than a permission, and no level
///   would turn it on.
/// - **Copying** (bit 5) is the host's rather than this crate's: what crosses is the readback,
///   and the same query answers a drag that merely *shows* a selection. Table 22 also carves the
///   bit itself — "for the limited purpose of providing this content to assistive technology, a
///   PDF reader should behave as if this bit was set to 1" — so an operation named `Copy` would
///   have to be distinguishable from §14.9's tree at the point it is asked, which is a
///   distinction only a host can make. `doc/todo/38` holds it.
/// - **Saving** is not in §7.6.4.1's list of operations user access can be controlled over, and
///   §12.8.2.3's usage rights are a *grant* rather than a restriction — what a save owes them is
///   the withdrawal in `crate::view::ViewState::save`, which is correctness rather than policy
///   and is not asked here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Putting a value into a field the document already holds — `crate::view::ViewState::set_field`.
    ///
    /// §7.6.4.1's own words for it: "[f]illing in forms (that is, filling in existing interactive
    /// form fields) and signing the document".
    FillInForm,
    /// Adding an annotation to a page — `crate::view::ViewState::add_markup`.
    ///
    /// §7.6.4.1's "[a]dding or modifying text annotations", and Table 257's "annotation creation,
    /// deletion, and modification", which `/DocMDP` permits only at its third level.
    Annotate,
}

impl Operation {
    /// The verb, for a sentence a host words about it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FillInForm => "filling in a form field",
            Self::Annotate => "adding an annotation",
        }
    }
}

/// One reason a document gives for not permitting an operation.
///
/// Never a refusal by itself: it is what the *file* says, and what a reader does about it is
/// [`Operation`]'s caller's business.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restriction {
    /// §12.8.2.2's `/DocMDP`, made binding by §12.8.6's permissions dictionary.
    ///
    /// The sentence that makes it a restriction rather than a statement is §12.8.2.2.1's, in a
    /// parenthesis:
    ///
    /// > (These changes to the document shall also be prevented if the signature dictionary is
    /// > referred from the DocMDP entry in the permissions dictionary.)
    ///
    /// The level is what Table 257's `/P` states, so that a host can say which of the three the
    /// author chose.
    Certified {
        /// Table 257's `/P`, as [`Modification`] reads it.
        level: Modification,
    },
    /// §7.6.4.2's Table 22, as the security handler granted it to whoever opened the document.
    ///
    /// §7.6.4.1 is where the obligation on a reader is stated, and it is a `shall`:
    ///
    /// > PDF readers shall respect the intent of the document creator by restricting user access
    /// > to an encrypted PDF file according to the permissions contained in the file.
    ///
    /// A `shall` on a reader, and the one this module's whole shape is about: `CLAUDE.md` makes
    /// obeying it the reader's own decision, with obeying as the default and a host able to say
    /// otherwise.
    AccessDenied {
        /// The bit position Table 22 numbers from 1, and which would have granted this.
        ///
        /// One position rather than a set: the bit named is the one that decides the operation
        /// at this document's revision, which for filling in a field is 9 where the revision is
        /// 3 or greater and 6 where it is 2.
        bit: u8,
    },
    /// §12.7.5.5's signature field lock, asserted by a signature field that has been signed.
    ///
    /// > The signature field lock dictionary … contains the names of form fields whose values
    /// > shall no longer be changed after this signature has been signed.
    ///
    /// The only one of the four that is about **one field** rather than about the document, and
    /// therefore the only reason [`asserted`] needs to be told which field is being filled in.
    FieldLocked,
}

/// Every restriction this document asserts against this operation.
///
/// Empty means nothing in the file withholds it — which is the answer for a document that is not
/// encrypted and states no `/Perms`, which is 961 of the 968 corpus documents that open.
///
/// # What is read
///
/// §12.8.6's `/Perms /DocMDP` first, because it applies whether or not the document is encrypted
/// — the clause says these permissions "do not require that the document be encrypted" — and
/// then §7.6.4.2's Table 22, which applies only where a security handler granted anything, and
/// last §12.7.5.5's signature field lock.
///
/// # `field`
///
/// The fully qualified name (§12.7.4.2) of the field being filled in, and `None` for every other
/// operation. Two of the three clauses restrict the *document* and the third restricts a named
/// field, so the verb alone cannot decide it — and a `Some` for an operation that is not
/// [`Operation::FillInForm`] is ignored rather than made an error, because no clause here reads a
/// field name for anything else.
#[must_use]
pub fn asserted(
    document: &Document,
    operation: Operation,
    field: Option<&str>,
) -> Vec<Restriction> {
    let mut out = Vec::new();
    if let Some(level) = crate::signature::permissions(document).doc_mdp
        && !certification_permits(level, operation)
    {
        out.push(Restriction::Certified { level });
    }
    if let Some(permissions) = document.permissions()
        && let Some(restriction) = withheld(permissions, operation)
    {
        out.push(restriction);
    }
    if operation == Operation::FillInForm
        && let Some(field) = field
        && crate::signature::field_locks(document)
            .iter()
            .any(|lock| lock.locks(field))
    {
        out.push(Restriction::FieldLocked);
    }
    out
}

/// Whether Table 257's `/P` leaves room for this operation.
///
/// §12.8.2.2's Table 257 states the three levels:
///
/// > 1 No changes to the document shall be permitted; any change to the document shall invalidate
/// > the signature. 2 Permitted changes shall be filling in forms, instantiating page templates,
/// > and signing; other changes shall invalidate the signature. 3 Permitted changes shall be the
/// > same as for 2, as well as annotation creation, deletion, and modification; other changes
/// > shall invalidate the signature.
///
/// So the two operations part company at level 2, which permits the first and not the second —
/// and that is the whole of why a `/DocMDP` restriction has to name its level rather than answer
/// yes or no.
///
/// A `/P` outside 1..=3 permits, because refusing on a value Table 257 does not define would let
/// a malformed number lock a document a person is entitled to fill in.
fn certification_permits(level: Modification, operation: Operation) -> bool {
    match level {
        Modification::None => false,
        Modification::FormFilling => operation == Operation::FillInForm,
        Modification::FormFillingAndAnnotation | Modification::Unknown(_) => true,
    }
}

/// What Table 22 withholds from this operation, given the flags a reader was granted.
///
/// Separate from [`asserted`] because it is a pure function of the flag word and can therefore be
/// stated as arithmetic in a test: building an encrypted fixture would mean running this
/// project's own cipher to check its own reading of a table.
///
/// Three rules, in the order they are applied:
///
/// - **The owner may do anything.** §7.6.4.1: "[o]pening the document with the correct owner
///   password should allow full (owner) access to the document."
/// - **Bit 6 grants both operations.** Table 22: "[a]dd or modify text annotations, fill in
///   interactive form fields, and, if bit 4 is also set, create or modify interactive form
///   fields (including signature fields)."
/// - **Bit 9 grants the narrower one, and only from revision 3.** Table 22: "( Security handlers
///   of revision 3 or greater ) Fill in existing interactive form fields (including signature
///   fields), even if bit 6 is clear." At revision 2 that position is inside the range Table 22
///   reserves and requires to be 1, so reading it there would turn every conforming revision-2
///   document into one that permits form filling — including the clause's own example, whose
///   `/P` of -44 "disallows modifying the contents and annotations".
#[must_use]
pub fn withheld(permissions: Permissions, operation: Operation) -> Option<Restriction> {
    if permissions.owner {
        return None;
    }
    match operation {
        Operation::FillInForm => {
            if permissions.revision >= 3 {
                (!permissions.fill_forms && !permissions.annotate)
                    .then_some(Restriction::AccessDenied { bit: 9 })
            } else {
                (!permissions.annotate).then_some(Restriction::AccessDenied { bit: 6 })
            }
        }
        Operation::Annotate => {
            (!permissions.annotate).then_some(Restriction::AccessDenied { bit: 6 })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Operation, Restriction, withheld};
    use pdf_syntax::Permissions;

    /// A flag word with everything granted, which each case then takes one thing away from.
    fn granted(revision: u8) -> Permissions {
        Permissions {
            owner: false,
            revision,
            print: true,
            modify: true,
            copy: true,
            annotate: true,
            fill_forms: true,
            assemble: true,
            print_faithfully: true,
        }
    }

    /// Table 22 bit 9 is a grant from revision 3 and a reserved bit before it.
    ///
    /// The standard's own example is the witness, under Table 21: "assuming revision 2 of the
    /// security handler, the value -44 permits printing and copying but disallows modifying the
    /// contents and annotations". −44 has bit 9 set — Table 22 requires positions 13 to 32 to be
    /// 1 and every conforming revision-2 word therefore sets it — so a reader that consulted it
    /// at that revision would permit exactly the filling in the example disallows.
    ///
    /// `bug900822.pdf` is the corpus's one revision-2 encrypted document and states `/P −60`,
    /// which is the same shape: bit 6 clear, bit 9 set by the reservation.
    #[test]
    fn bit_nine_only_grants_from_revision_three() {
        let mut permissions = granted(3);
        permissions.annotate = false;
        assert_eq!(withheld(permissions, Operation::FillInForm), None);
        assert_eq!(
            withheld(permissions, Operation::Annotate),
            Some(Restriction::AccessDenied { bit: 6 }),
            "bit 9 says nothing about annotating"
        );

        permissions.revision = 2;
        assert_eq!(
            withheld(permissions, Operation::FillInForm),
            Some(Restriction::AccessDenied { bit: 6 }),
            "at revision 2 the only bit that grants form filling is 6"
        );
    }

    /// Both bits clear withholds both operations; the owner password withholds neither.
    ///
    /// §7.6.4.1: "[o]pening the document with the correct owner password should allow full
    /// (owner) access to the document." `print_protection.pdf` is the corpus document where that
    /// matters — its `/P` of −3392 clears every bit this reads, and `1234` is its *owner*
    /// password, so a reader that honoured the flags anyway would be restricting the person the
    /// clause says has full access.
    #[test]
    fn everything_clear_withholds_both_and_the_owner_is_exempt() {
        let mut permissions = granted(4);
        permissions.annotate = false;
        permissions.fill_forms = false;
        assert_eq!(
            withheld(permissions, Operation::FillInForm),
            Some(Restriction::AccessDenied { bit: 9 })
        );
        assert_eq!(
            withheld(permissions, Operation::Annotate),
            Some(Restriction::AccessDenied { bit: 6 })
        );

        permissions.owner = true;
        assert_eq!(withheld(permissions, Operation::FillInForm), None);
        assert_eq!(withheld(permissions, Operation::Annotate), None);
    }
}

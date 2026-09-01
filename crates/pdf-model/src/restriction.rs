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
//! **A fifth joined them in the four-hundred-and-sixty-ninth session**: §12.5.3's Table 167 bit
//! 10, `LockedContents`, which is addressed to a **named annotation** the way the fourth is
//! addressed to a named field. It arrived here rather than at the point of the edit for this
//! module's whole reason — a refusal that cannot become an *ask* is the thing `CLAUDE.md` says to
//! avoid — and `crate::view::ViewState::set_free_text` therefore does not consult it.
//!
//! **And a sixth, which is the fourth's other half**: §12.8.2.4's `FieldMDP` transform names the
//! same fields the signature field lock does — the standard has a writer copy one into the other
//! — but from *inside* the signature rather than from the field dictionary, and it states a
//! different thing about them. A lock says the value shall not change; a transform says a change
//! invalidates the signature. Both are what the file says, neither is a verdict, and a person
//! being asked whether to go ahead is owed the distinction rather than one sentence covering
//! both. ADR 0403.
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
//! # The policy is here too, and it is a value a host supplies
//!
//! Since the eight-hundred-and-seventy-second session the four levels are one type, [`Level`],
//! and the one place they are applied is [`Level::verdict`] — a pure function from what the
//! document asserts to what the caller does, answered as a [`Verdict`] the caller matches
//! exhaustively. Nothing here refuses: `Refuse` is a value, and the caller that receives it is
//! the one that declines. `viewer_core` supplies its level through `Command::Restrict` and
//! `pdf_transform` through `--restrictions`, and neither decides anything at the point of the
//! operation. ADR 0803.
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

/// One position of §7.6.4.2's Table 22, named whether or not anything here consumes it.
///
/// > PDF readers shall ignore all flags other than those at bit positions 3, 4, 5, 6, 9, 10, 11,
/// > and 12.
///
/// Seven of those eight are the variants, and the other positions are stated here so that the
/// table is read whole rather than as far as the first consumer: **1–2** are "Reserved. Must be
/// zero (0)"; **7–8** and **13–32** are "Reserved. Must be 1"; and **10**, which the sentence
/// above lists and the table's own row retires, is "Not used" — it once carved accessibility
/// out of bit 5, "that restriction has been deprecated in PDF 2.0", and the row ends "PDF
/// readers shall ignore this bit". The row is the later and the more specific statement (the
/// table "was re-titled and corrected in this document (2020)"), so 10 has no variant.
/// `pdf_syntax::Permissions` reads the word and this enumeration names what it read.
///
/// A position's meaning depends on the security handler's revision, and Table 22 says so on the
/// rows it applies to: 9, 11 and 12 are "( Security handlers of revision 3 or greater )", and at
/// revision 2 those positions are inside the range the table reserves and requires to be 1.
/// [`Operation::bit`] is where that reading is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Bit {
    /// 3 — "Print the document", and at revision 3 or greater "(possibly not at the highest
    /// quality level, depending on whether bit 12 is also set)". Consumed by
    /// [`Operation::Print`].
    Print,
    /// 4 — "Modify the contents of the document by operations other than those controlled by
    /// bits 6, 9, and 11": the residual every modification not named elsewhere falls under.
    /// Consumed by [`Operation::Modify`].
    Modify,
    /// 5 — "Copy or otherwise extract text and graphics from the document", with the carve-out
    /// "for the limited purpose of providing this content to assistive technology, a PDF reader
    /// should behave as if this bit was set to 1". Consumed by [`Operation::Extract`], and
    /// deliberately not by anything that feeds §14.9's tree.
    Extract,
    /// 6 — "Add or modify text annotations, fill in interactive form fields, and, if bit 4 is
    /// also set, create or modify interactive form fields (including signature fields)".
    /// Consumed by [`Operation::Annotate`], and by [`Operation::FillInForm`] where bit 9 does
    /// not apply.
    Annotate,
    /// 9 — "( Security handlers of revision 3 or greater ) Fill in existing interactive form
    /// fields (including signature fields), even if bit 6 is clear". Consumed by
    /// [`Operation::FillInForm`] from revision 3.
    FillInForm,
    /// 11 — "( Security handlers of revision 3 or greater ) Assemble the document (insert,
    /// rotate, or delete pages and create document outline items or thumbnail images), even if
    /// bit 4 is clear". **Nothing consumes it**: this program inserts, rotates and deletes no
    /// page, and `doc/todo/57`'s `split`, `merge` and `pages` are the verbs that will. Named so
    /// that the day they land the bit is a lookup rather than a reading.
    Assemble,
    /// 12 — "( Security handlers of revision 3 or greater ) Print the document to a
    /// representation from which a faithful digital copy of the PDF content could be generated,
    /// based on an implementation-dependent algorithm. When this bit is clear (and bit 3 is
    /// set), printing shall be limited to a low-level representation of the appearance,
    /// possibly of degraded quality." **Nothing consumes it**: the algorithm that decides what
    /// "faithful" means is the implementation's, and this tree has not chosen one — a page
    /// raster at any resolution is a "representation of the appearance", and whether a given
    /// resolution is degraded enough is a question the clause hands to the processor. Stated
    /// rather than guessed (trap 11).
    PrintFaithfully,
}

impl Bit {
    /// The position, as Table 22 numbers it from 1.
    #[must_use]
    pub const fn position(self) -> u8 {
        match self {
            Self::Print => 3,
            Self::Modify => 4,
            Self::Extract => 5,
            Self::Annotate => 6,
            Self::FillInForm => 9,
            Self::Assemble => 11,
            Self::PrintFaithfully => 12,
        }
    }

    /// The operation this tree performs that the bit governs, or `None` for a bit nothing here
    /// consumes — which is a statement rather than a gap; see [`Bit::Assemble`] and
    /// [`Bit::PrintFaithfully`].
    #[must_use]
    pub const fn consumed_by(self) -> Option<Operation> {
        match self {
            Self::Print => Some(Operation::Print),
            Self::Modify => Some(Operation::Modify),
            Self::Extract => Some(Operation::Extract),
            Self::Annotate => Some(Operation::Annotate),
            Self::FillInForm => Some(Operation::FillInForm),
            Self::Assemble | Self::PrintFaithfully => None,
        }
    }

    /// Whether the flag word grants this bit, as `pdf_syntax::Permissions` read it.
    ///
    /// Revision is not applied here: a position the table reserves at revision 2 reads as set,
    /// because the word is required to set it, and [`Operation::bit`] is what keeps such a
    /// position from being consulted.
    #[must_use]
    pub const fn granted(self, permissions: Permissions) -> bool {
        match self {
            Self::Print => permissions.print,
            Self::Modify => permissions.modify,
            Self::Extract => permissions.copy,
            Self::Annotate => permissions.annotate,
            Self::FillInForm => permissions.fill_forms,
            Self::Assemble => permissions.assemble,
            Self::PrintFaithfully => permissions.print_faithfully,
        }
    }
}

/// One thing a reader does to a document that a clause can restrict.
///
/// One variant per verb this program has and that a clause names, which is deliberately not the
/// whole of Table 22: an operation nothing here performs is an operation no restriction can bite
/// on, and an enum arm for it would claim otherwise. The same discipline
/// [`crate::signature::Right`] follows. The bits themselves are all named, in [`Bit`], so that
/// the absence of an arm is legible as a decision about this program rather than a reading of
/// the table that stopped early.
///
/// Two of the five are the viewer's, three are `pdf_transform`'s — they were two enums in two
/// crates until the eight-hundred-and-seventy-second session, and one module now reads every
/// restriction source for every operation this tree performs (ADR 0803).
///
/// **What is missing and why**, because the absences are decisions rather than gaps:
///
/// - **Assembling** (bit 11) and **faithful printing** (bit 12) name operations this program
///   does not have; [`Bit::Assemble`] and [`Bit::PrintFaithfully`] say so.
/// - **Copying from a window** is the host's rather than this crate's: what crosses is the
///   readback, and the same query answers a drag that merely *shows* a selection. Table 22 also
///   carves the bit itself — "for the limited purpose of providing this content to assistive
///   technology, a PDF reader should behave as if this bit was set to 1" — so a window's copy
///   would have to be distinguishable from §14.9's tree at the point it is asked, which is a
///   distinction only a host can make. `doc/todo/38` holds it. [`Operation::Extract`] is the
///   batch tool's, where a file written out is unambiguously a copy.
/// - **Saving** is not in §7.6.4.1's list of operations user access can be controlled over, and
///   §12.8.2.3's usage rights are a *grant* rather than a restriction — what a save owes them is
///   the withdrawal in `crate::view::ViewState::save`, which is correctness rather than policy
///   and is not asked here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    /// Putting a value into a field the document already holds — `crate::view::ViewState::set_field`.
    ///
    /// §7.6.4.1's own words for it: "[f]illing in forms (that is, filling in existing interactive
    /// form fields) and signing the document".
    FillInForm,
    /// Adding an annotation to a page, or changing one — `crate::view::ViewState::add_markup` and
    /// `crate::view::ViewState::set_free_text`.
    ///
    /// §7.6.4.1's "[a]dding or modifying text annotations", and Table 257's "annotation creation,
    /// deletion, and modification", which `/DocMDP` permits only at its third level. **One verb for
    /// both halves**, because both of those sentences name both: adding and modifying are one
    /// permission everywhere the standard states one.
    Annotate,
    /// Rasterising a page to a file — `pdf_transform`'s `render`.
    ///
    /// Table 22 bit 3, "Print the document". A choice: a page raster is what a print driver
    /// produces, and it is the nearest of the bits; it is written down as a choice because the
    /// clause does not mention rasterisation. Bit 12's quality distinction is not read, for the
    /// reason [`Bit::PrintFaithfully`] gives.
    Print,
    /// Taking images or embedded files out of the document as files — `pdf_transform`'s
    /// `images` and `attachments --save`.
    ///
    /// Table 22 bit 5, "[c]opy or otherwise extract text and graphics from the document".
    Extract,
    /// Writing something into the document that is neither an annotation, a field value nor a
    /// page — `pdf_transform`'s `attachments --attach` and `--remove`.
    ///
    /// Table 22 bit 4, "[m]odify the contents of the document by operations other than those
    /// controlled by bits 6, 9, and 11". No bit *names* an embedded file; bits 6, 9 and 11 carve
    /// annotations, form filling and page assembly out of bit 4, and an embedded file is none of
    /// those three, so bit 4 is the bit that binds it (ADR 0802).
    Modify,
}

impl Operation {
    /// The verb, for a sentence a host words about it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FillInForm => "filling in a form field",
            Self::Annotate => "adding an annotation",
            Self::Print => "rendering a page",
            Self::Extract => "extracting from the document",
            Self::Modify => "modifying the document",
        }
    }

    /// The bit that decides this operation at this security handler revision.
    ///
    /// One position rather than a set, and the revision is what chooses it: Table 22 marks bit
    /// 9 "( Security handlers of revision 3 or greater )", and at revision 2 that position is
    /// inside the range the table reserves and requires to be 1, so reading it there would turn
    /// every conforming revision-2 document into one that permits form filling — including the
    /// clause's own example, whose `/P` of -44 "disallows modifying the contents and
    /// annotations". Bit 9 is also the only one of the five whose row grants "even if bit 6 is
    /// clear", which is why filling in is the one operation two bits can grant.
    #[must_use]
    pub const fn bit(self, revision: u8) -> Bit {
        match self {
            Self::FillInForm if revision >= 3 => Bit::FillInForm,
            Self::FillInForm | Self::Annotate => Bit::Annotate,
            Self::Print => Bit::Print,
            Self::Extract => Bit::Extract,
            Self::Modify => Bit::Modify,
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
        /// The bit that would have granted this — the one that decides the operation at this
        /// document's revision, [`Operation::bit`].
        bit: Bit,
    },
    /// §12.7.5.5's signature field lock, asserted by a signature field that has been signed.
    ///
    /// > The signature field lock dictionary … contains the names of form fields whose values
    /// > shall no longer be changed after this signature has been signed.
    ///
    /// The only one of the four that is about **one field** rather than about the document, and
    /// therefore the only reason [`asserted`] needs to be told which field is being filled in.
    FieldLocked,
    /// §12.8.2.4's `FieldMDP` transform, asserted by a signature that names this field.
    ///
    /// > The FieldMDP transform method shall be used to detect changes to the values of a list of
    /// > form fields.
    ///
    /// **Not [`Restriction::FieldLocked`] under another name, and the difference is the whole
    /// reason it is its own variant.** §12.7.5.5's lock is a prohibition — the field's value
    /// "shall no longer be changed" — while this clause states a *consequence*, in the sentence
    /// addressed to the author who asks for it: "any modifications to specific form fields shall
    /// invalidate that recipient's signature". A person is owed which of the two they are being
    /// told, because one says the document forbids the edit and the other says the edit costs a
    /// signature; a reader who would accept the second may well refuse the first.
    ///
    /// The two arrive together on a document written the way §12.8.2.4 tells a writer to write
    /// one, because Table 259's entries "shall be copied from the corresponding fields in the
    /// signature field lock dictionary" — and [`asserted`] then returns both, which is §12.8.6's
    /// composition rule doing its job rather than a duplicate.
    FieldCovered,
    /// §12.5.3's Table 167 bit 10, asserted by the annotation being edited.
    ///
    /// > LockedContents … If set, do not allow the contents of the annotation to be modified by
    /// > the user. This flag does not restrict deletion of the annotation or changes to other
    /// > annotation properties, such as position and size.
    ///
    /// **Bit 8 is the one that sounds like this and is not**, and the difference is the table's own
    /// rather than a reading of it: `Locked` is "do not allow the annotation to be deleted or its
    /// properties (including position and size) to be modified by the user", and its row ends
    /// "[h]owever, this flag does not restrict changes to the annotation's contents, such as the
    /// value of a form field". So an annotation carrying bit 8 and not bit 10 may be typed into,
    /// and nothing here consults bit 8.
    AnnotationLocked,
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
/// last the two that name a field: §12.7.5.5's signature field lock and §12.8.2.4's `FieldMDP`
/// transform, which a conforming writer states as copies of each other and which are therefore
/// both returned where a document states both.
///
/// # `field` and `annotation`
///
/// The fully qualified name (§12.7.4.2) of the field being filled in, and the object of the
/// annotation being changed — each `None` for every operation that names no such thing. Two of the
/// clauses restrict the *document*, one restricts a named field and one a named annotation, so the
/// verb alone cannot decide it; a `Some` an operation's clause does not read is ignored rather than
/// made an error.
#[must_use]
pub fn asserted(
    document: &Document,
    operation: Operation,
    field: Option<&str>,
    annotation: Option<pdf_syntax::ObjectId>,
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
    {
        if crate::signature::field_locks(document)
            .iter()
            .any(|lock| lock.covers(field))
        {
            out.push(Restriction::FieldLocked);
        }
        if crate::signature::field_mdp(document)
            .iter()
            .any(|covered| covered.covers(field))
        {
            out.push(Restriction::FieldCovered);
        }
    }
    if operation == Operation::Annotate
        && let Some(annotation) = annotation
        && let Some(dict) = document.get(annotation).as_dict()
        && document
            .get_key(dict, "F")
            .as_integer()
            .is_some_and(|flags| flags & LOCKED_CONTENTS != 0)
    {
        out.push(Restriction::AnnotationLocked);
    }
    out
}

/// Table 167 bit 10, counted from 1 as the table numbers its positions.
const LOCKED_CONTENTS: i64 = 1 << 9;

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
///
/// **Reading and extracting are not changes.** Every level of Table 257 is about "changes to
/// the document", and rendering a page or copying a file out of it changes nothing, so a
/// certification withholds neither at any level; and **attaching a file is a change no level
/// permits** — it is not form filling, a page template, a signature or an annotation — so
/// [`Operation::Modify`] is withheld at all three.
fn certification_permits(level: Modification, operation: Operation) -> bool {
    match operation {
        Operation::Print | Operation::Extract => true,
        Operation::Modify => matches!(level, Modification::Unknown(_)),
        Operation::FillInForm | Operation::Annotate => match level {
            Modification::None => false,
            Modification::FormFilling => operation == Operation::FillInForm,
            Modification::FormFillingAndAnnotation | Modification::Unknown(_) => true,
        },
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
/// - **Bit 6 grants both of the viewer's operations.** Table 22: "[a]dd or modify text
///   annotations, fill in interactive form fields, and, if bit 4 is also set, create or modify
///   interactive form fields (including signature fields)."
/// - **Bit 9 grants the narrower one, and only from revision 3.** Table 22: "( Security handlers
///   of revision 3 or greater ) Fill in existing interactive form fields (including signature
///   fields), even if bit 6 is clear." At revision 2 that position is inside the range Table 22
///   reserves and requires to be 1, so reading it there would turn every conforming revision-2
///   document into one that permits form filling — including the clause's own example, whose
///   `/P` of -44 "disallows modifying the contents and annotations".
///
/// The other three operations each have one bit at every revision, [`Operation::bit`].
#[must_use]
pub fn withheld(permissions: Permissions, operation: Operation) -> Option<Restriction> {
    if permissions.owner {
        return None;
    }
    let bit = operation.bit(permissions.revision);
    let granted = match operation {
        // Bit 9's row says "even if bit 6 is clear", so either grants it.
        Operation::FillInForm if bit == Bit::FillInForm => {
            permissions.fill_forms || permissions.annotate
        }
        _ => bit.granted(permissions),
    };
    (!granted).then_some(Restriction::AccessDenied { bit })
}

/// How much of what a document asserts over its reader this program obeys — `CLAUDE.md`
/// principle 3's four levels, in the project owner's words: "off, on, ask before operations,
/// warn before operation".
///
/// A value a host supplies, never a default this crate chooses for it: `viewer_core` has one
/// per viewer and `pdf_transform` one per run. **Two of the four need somebody to tell**, and a
/// caller that has nobody — a pipe, a batch job — says so where it degrades them rather than
/// here; [`Level::verdict`] answers every level and a [`Verdict`] is exhaustive, so the caller
/// that cannot ask is the one that writes the arm saying what it does instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    /// The document's assertions are not consulted: the operation proceeds. The program is the
    /// reader's, and `CLAUDE.md` makes this level the one that "shall always be possible".
    Off,
    /// The operation is refused with the document's reasons.
    ///
    /// §7.6.4.1's `shall` — "PDF readers shall respect the intent of the document creator by
    /// restricting user access to an encrypted PDF file according to the permissions contained
    /// in the file" — is kept by a reader at this level.
    On,
    /// The person is asked, with the reasons, and the operation waits on the answer.
    Ask,
    /// The operation proceeds and the reasons are said afterwards.
    Warn,
}

impl Level {
    /// The word a command line takes for each, and each takes.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::On => "on",
            Self::Ask => "ask",
            Self::Warn => "warn",
        }
    }

    /// The level a word names, if any.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "off" => Some(Self::Off),
            "on" => Some(Self::On),
            "ask" => Some(Self::Ask),
            "warn" => Some(Self::Warn),
            _ => None,
        }
    }

    /// The policy applied, once, to what the document asserts.
    ///
    /// A pure function: an empty list is [`Verdict::Proceed`] at every level, because a
    /// document that asserts nothing gives nobody anything to obey, ask about or warn of; and
    /// [`Level::Off`] is `Proceed` whatever the list holds, and drops it — the reasons were read
    /// so that the reading is one code path, and a caller at `Off` was told nothing because it
    /// asked to be told nothing.
    #[must_use]
    pub fn verdict(self, restrictions: Vec<Restriction>) -> Verdict {
        if restrictions.is_empty() {
            return Verdict::Proceed;
        }
        match self {
            Self::Off => Verdict::Proceed,
            Self::On => Verdict::Refuse(restrictions),
            Self::Ask => Verdict::Ask(restrictions),
            Self::Warn => Verdict::Warn(restrictions),
        }
    }
}

/// What a caller does about an operation, as [`Level::verdict`] answers it.
///
/// Exhaustive, and deliberately not `#[non_exhaustive]`: a consumer that cannot ask has to say
/// what it does with [`Verdict::Ask`], in an arm of its own, and the compiler is what holds it
/// to that. Each carrying variant holds every restriction that applied, not the first —
/// §12.8.6's composition rule, [`asserted`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Go ahead; nothing to say.
    Proceed,
    /// Go ahead, and then say these.
    Warn(Vec<Restriction>),
    /// Put these to the person and wait; go ahead only on a yes.
    Ask(Vec<Restriction>),
    /// Do not, and say why.
    Refuse(Vec<Restriction>),
}

/// The whole question in one call: what the document asserts against the operation, under the
/// level the host supplied.
///
/// [`asserted`] and then [`Level::verdict`], which is the shape every consumer follows so that
/// the policy is asked exactly once per operation, at the point the caller can still not do it.
#[must_use]
pub fn decide(
    level: Level,
    document: &Document,
    operation: Operation,
    field: Option<&str>,
    annotation: Option<pdf_syntax::ObjectId>,
) -> Verdict {
    level.verdict(asserted(document, operation, field, annotation))
}

#[cfg(test)]
mod tests {
    use super::{Bit, Level, Operation, Restriction, Verdict, withheld};
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
            Some(Restriction::AccessDenied { bit: Bit::Annotate }),
            "bit 9 says nothing about annotating"
        );

        permissions.revision = 2;
        assert_eq!(
            withheld(permissions, Operation::FillInForm),
            Some(Restriction::AccessDenied { bit: Bit::Annotate }),
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
            Some(Restriction::AccessDenied {
                bit: Bit::FillInForm
            })
        );
        assert_eq!(
            withheld(permissions, Operation::Annotate),
            Some(Restriction::AccessDenied { bit: Bit::Annotate })
        );

        permissions.owner = true;
        assert_eq!(withheld(permissions, Operation::FillInForm), None);
        assert_eq!(withheld(permissions, Operation::Annotate), None);
    }

    /// The three batch operations each read one bit, at every revision — Table 22's rows 3, 4
    /// and 5 carry no revision condition — and clearing one bit withholds one operation.
    #[test]
    fn print_extract_and_modify_each_read_their_own_bit() {
        for revision in [2, 3, 4, 6] {
            let mut permissions = granted(revision);
            permissions.print = false;
            assert_eq!(
                withheld(permissions, Operation::Print),
                Some(Restriction::AccessDenied { bit: Bit::Print })
            );
            assert_eq!(withheld(permissions, Operation::Extract), None);
            assert_eq!(withheld(permissions, Operation::Modify), None);

            let mut permissions = granted(revision);
            permissions.copy = false;
            assert_eq!(
                withheld(permissions, Operation::Extract),
                Some(Restriction::AccessDenied { bit: Bit::Extract })
            );
            let mut permissions = granted(revision);
            permissions.modify = false;
            assert_eq!(
                withheld(permissions, Operation::Modify),
                Some(Restriction::AccessDenied { bit: Bit::Modify })
            );
            assert_eq!(
                withheld(permissions, Operation::Annotate),
                None,
                "bit 6 is its own grant, carved out of bit 4 by bit 4's own row"
            );
        }
    }

    /// Every bit names its position as Table 22 numbers it, and the two nothing consumes say so.
    #[test]
    fn every_bit_has_its_position_and_two_have_no_consumer() {
        let all = [
            Bit::Print,
            Bit::Modify,
            Bit::Extract,
            Bit::Annotate,
            Bit::FillInForm,
            Bit::Assemble,
            Bit::PrintFaithfully,
        ];
        assert_eq!(
            all.map(Bit::position),
            [3, 4, 5, 6, 9, 11, 12],
            "§7.6.4.2: the seven positions a reader shall not ignore"
        );
        for bit in all {
            match bit.consumed_by() {
                Some(operation) => assert_eq!(
                    operation.bit(4),
                    bit,
                    "{bit:?} says {operation:?} consumes it, and the operation agrees"
                ),
                None => assert!(matches!(bit, Bit::Assemble | Bit::PrintFaithfully)),
            }
        }
    }

    /// The four levels over one list of reasons: `Off` drops it, the other three carry it, and
    /// an empty list proceeds at every level.
    #[test]
    fn a_level_is_a_pure_function_of_the_reasons() {
        let reasons = vec![Restriction::AccessDenied { bit: Bit::Modify }];
        assert_eq!(Level::Off.verdict(reasons.clone()), Verdict::Proceed);
        assert_eq!(
            Level::On.verdict(reasons.clone()),
            Verdict::Refuse(reasons.clone())
        );
        assert_eq!(
            Level::Ask.verdict(reasons.clone()),
            Verdict::Ask(reasons.clone())
        );
        assert_eq!(
            Level::Warn.verdict(reasons),
            Verdict::Warn(vec![Restriction::AccessDenied { bit: Bit::Modify }])
        );
        for level in [Level::Off, Level::On, Level::Ask, Level::Warn] {
            assert_eq!(level.verdict(Vec::new()), Verdict::Proceed);
            assert_eq!(Level::parse(level.as_str()), Some(level));
        }
        assert_eq!(Level::parse("maybe"), None);
    }
}

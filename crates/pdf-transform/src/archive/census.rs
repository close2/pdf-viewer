//! What this converter has decided about **every** requirement a target binds, counted.
//!
//! # The denominator this file exists to change
//!
//! `CLAUDE.md`'s two-denominator table: *robustness* asks what share of the files that exist
//! convert, and its instrument is `tests/archive_corpus.rs`; *coverage* asks how many of the
//! standard's requirements have an answer, and its denominator is the specification. Every
//! slice of this converter until now was ranked by corpus weight, which answers only the first
//! — and a converter whose gap list is "the documents that failed today" finishes when the
//! corpus goes quiet, with much of ISO 19005 unanswered and nothing able to say which parts.
//!
//! So this is the coverage instrument, and it is a pure function of three tables:
//! `pdf_archive::table::binding` says what a target is held to, and
//! the decision table's `REMEDIES`, `WRITER_EMITS` and `REFUSED_BY_NAME` say what this
//! converter does about each. [`Standing`] is the join, and its last variant —
//! [`Standing::Unconsidered`] — is the product: a requirement the validator can fail a document
//! on and that no table of this converter's names, so that `decide` answers it with the
//! catch-all sentence. Nothing had ever enumerated that set.
//!
//! # Why a refusal is not a gap
//!
//! Four of the seven standings are finished work. A requirement whose subject is a program
//! rather than a file cannot be *asked* of this converter; one the validator never fails cannot
//! reach it; and one refused by argument — ADR 0816's fence, no conforming file for this target,
//! or a caller's own `--no-substitute` — has had its answer written down and will not get a
//! better one. `Unconsidered` is the only standing that means *nobody has looked*.

use pdf_archive::{Check, Requirement, Target, table};

use super::decision::{Answer, Because, REFUSED_BY_NAME, REMEDIES, WRITER_EMITS};

/// What this converter has to say about one requirement, before any document is opened.
///
/// Seven answers, and they are seven different facts rather than degrees of one. The three at
/// the top are reasons the converter is never *asked*; the three in the middle are answers it
/// has; [`Self::Unconsidered`] is the absence of one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// `pdf_archive::Check::Processor`: the requirement binds a conforming processor rather
    /// than a conforming file, so no document can fail it and no conversion can act on it.
    ///
    /// Not a gap of this converter's. It is an obligation on *this project* if it claims to be
    /// a conforming reader, and `doc/PLAN.md` section 5a's conformance ledger is where such a
    /// claim belongs.
    Processor,
    /// `pdf_archive::Check::OutsideValidation`: the clause states the requirement and the
    /// working group responsible has resolved that it is not addressed to a conforming file.
    OutsideValidation,
    /// `pdf_archive::Check::Unchecked`: the validator does not judge it, so the converter is
    /// never asked about it.
    ///
    /// A debt, and `pdf_archive`'s rather than this crate's: until a predicate exists, no
    /// document fails the requirement and the decision table has nothing to answer.
    Unchecked,
    /// A row of `REMEDIES`, carrying that row's class of answer.
    Remedy(Kind),
    /// A row of `WRITER_EMITS`: the serializer satisfies it by construction, so the whole-file
    /// rewrite is the remedy and there is nothing to decide per place.
    WriterEmits,
    /// A row of `REFUSED_BY_NAME`: refused with an argument of its own.
    Refused(Because),
    /// In none of the three tables, so `decide`'s catch-all answers it with `NOT_BUILT_YET`.
    ///
    /// **The product of this census.** A document failing one of these is told only that the gap
    /// is this program's — which is true, and which is all anybody knows about it.
    Unconsidered,
}

impl Standing {
    /// A stable word for a report a person reads.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Processor => "processor",
            Self::OutsideValidation => "outside-validation",
            Self::Unchecked => "unchecked",
            Self::Remedy(kind) => kind.word(),
            Self::WriterEmits => "writer-emits",
            Self::Refused(because) => because.word(),
            Self::Unconsidered => "unconsidered",
        }
    }

    /// Whether this standing is an answer somebody wrote, rather than an absence or a reason
    /// the converter is never asked.
    #[must_use]
    pub const fn is_considered(self) -> bool {
        !matches!(self, Self::Unconsidered)
    }
}

/// Which class of answer a `REMEDIES` row gives.
///
/// The three classes of `doc/pdf-a-conversion-limits.md` sections 3 and 4 plus the one row whose
/// answer is chosen per document, kept apart here for the same reason
/// [`super::Decision`] keeps them apart: a rewrite that loses nothing, one that changes what the
/// file asserts, and one that needs a caller's authorisation are three different things to have
/// promised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `Answer::Mechanical`: a rewrite that loses nothing.
    Mechanical,
    /// `Answer::Stated`: a rewrite that writes down an interpretation the standard defines.
    Stated,
    /// `Answer::Loses`: a rewrite the caller has to authorise.
    Loses,
    /// `Answer::CmykUnderPartTwo`: two licences, chosen between on the profile in hand.
    TwoLicences,
}

impl Kind {
    /// A stable word for a report a person reads.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Mechanical => "mechanical",
            Self::Stated => "stated",
            Self::Loses => "loses",
            Self::TwoLicences => "two-licences",
        }
    }
}

/// What this converter has decided about one requirement.
///
/// A pure function of the four tables and nothing else — no document, no caller's
/// authorisations, no profile. That is what makes it countable ahead of any conversion, and it
/// is why the answer is a [`Standing`] rather than a [`super::Decision`]: whether a *particular*
/// document can take a rewrite is the preparation's question, asked per file.
#[must_use]
pub fn standing(requirement: &Requirement) -> Standing {
    match requirement.check {
        Check::Processor(_) => return Standing::Processor,
        Check::OutsideValidation(_) => return Standing::OutsideValidation,
        Check::Unchecked(_) => return Standing::Unchecked,
        Check::Implemented(_) => {}
    }
    if WRITER_EMITS.contains(&requirement.id) {
        return Standing::WriterEmits;
    }
    if let Some(remedy) = REMEDIES
        .iter()
        .find(|remedy| remedy.requirement == requirement.id)
    {
        return Standing::Remedy(match remedy.answer {
            Answer::Mechanical(_) => Kind::Mechanical,
            Answer::Stated(..) => Kind::Stated,
            Answer::Loses(..) => Kind::Loses,
            Answer::CmykUnderPartTwo => Kind::TwoLicences,
        });
    }
    REFUSED_BY_NAME
        .iter()
        .find(|(id, _)| *id == requirement.id)
        .map_or(Standing::Unconsidered, |(_, because)| {
            Standing::Refused(*because)
        })
}

/// Every requirement one target binds, with what this converter has decided about it.
///
/// In the table's own order, which is clause order, so that two censuses of different targets
/// can be read side by side.
pub fn census(target: Target) -> impl Iterator<Item = (&'static Requirement, Standing)> {
    table::binding(target).map(|requirement| (requirement, standing(requirement)))
}

/// Every requirement any target binds that this converter has no considered answer for.
///
/// Sorted by identifier and free of duplicates: a requirement is unconsidered as a fact about
/// the decision table rather than about a target, so one binding four targets appears once. The
/// targets it binds are [`Requirement::binds`]'s question and the census prints them.
///
/// **This is what `tests/archive.rs` ratchets.** `doc/todo/00`'s discipline: the list is held to
/// equality in both directions, so a requirement arriving here fails the build on arrival and
/// one leaving it has to be struck from the file in the same commit that answered it.
#[must_use]
pub fn unconsidered() -> Vec<&'static Requirement> {
    let mut out: Vec<&'static Requirement> = table::requirements()
        .filter(|requirement| {
            Target::ALL
                .iter()
                .any(|target| table::binding(*target).any(|bound| bound.id == requirement.id))
                && standing(requirement) == Standing::Unconsidered
        })
        .collect();
    out.sort_unstable_by_key(|requirement| requirement.id);
    out
}

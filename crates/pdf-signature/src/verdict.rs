//! The one type in this program that can say *valid*, and the proof it cannot be made without.
//!
//! ISO 32000-2 §12.8.1 divides a signature into three questions — has the document changed, does
//! the signature verify, is the signer anyone to believe — and this project has answered the first
//! two since the three-hundred-and-seventy-seventh and three-hundred-and-ninety-second sessions
//! (ADR 0215, ADR 0229). The third needs an anchor, RFC 5280 section 6.1.1 makes an anchor input
//! (d), and ADR 1039 made it a host's to supply. [`crate::trust::Supply`] is the supplying; this is
//! what a host may be told once it has supplied.
//!
//! # Why a type and not a function
//!
//! Every module in this crate has spent its life refusing the word. [`crate::signature::
//! Authenticity::Verified`] means *this signature was made with the key in a certificate the file
//! itself carried*; [`crate::revision::Judgement::WithinWhatIsPermitted`] is a statement about
//! objects; [`crate::trust::Trust::Anchored`] is a path that reached somebody's anchor. None of
//! them is a verdict, and the discipline that kept them apart was a convention — a rule written in
//! doc comments and kept by every round that read them.
//!
//! A convention is the wrong instrument for the sentence a reader acts on. So the rule is a type
//! here: [`Valid`] has no public constructor and no public field, [`Verdict::reached`] is the only
//! function in this tree that makes one, and it takes an [`Anchored`] — which in turn can only be
//! made from a [`Trust::Anchored`]. A caller with no anchor cannot reach the word by any path,
//! including a wrong one. ADR 1076.
//!
//! # What *valid* is being claimed to mean
//!
//! All of the following, and nothing beyond them:
//!
//! - the bytes `/ByteRange` names still hash to the digest the signature records (§12.8.1's
//!   question 1, [`Integrity::Unchanged`]);
//! - the signature verifies under the key in the signer's certificate (question 2,
//!   [`Authenticity::Verified`]);
//! - a certification path from that certificate reaches an anchor **this host supplied**, and every
//!   check RFC 5280 section 6.1 states over it passed (question 3, [`Anchored`]);
//! - §12.8.4's material in this document says that path is not revoked, or says nothing and the
//!   host's [`Acceptance`] admits that (ADR 1067's rule is untouched: an absence is never a
//!   [`Revocation::Good`], and admitting one is a *host's* decision made in the open);
//! - where the document carries §12.8.5's document timestamps, the chain is established (ADR 1071).
//!
//! It is not a claim that the anchor is a good anchor, that the signer is who the certificate says,
//! or that the document means what it appears to. Those are the host's, the authority's and the
//! reader's respectively, and this program has nothing to add to any of them.

use crate::revocation::{Revocation, Undetermined};
use crate::signature::{Authenticity, Integrity};
use crate::trust::{PathRefusal, Trust};
use crate::x509::Instant;

/// A certification path that reached an anchor a host supplied.
///
/// **The proof, and the only way to one is [`Self::of`].** A [`Trust::NoAnchorSupplied`], a
/// [`Trust::NoPathToAnyAnchor`] and a [`Trust::Refused`] all answer `None`, so a caller holding one
/// of these is a caller for whom RFC 5280 section 6.1 ran to the end over a chain somebody outside
/// this program vouched for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchored {
    length: usize,
    revocation: Revocation,
}

impl Anchored {
    /// The proof a [`Trust::Anchored`] is, and `None` for every other answer.
    #[must_use]
    pub fn of(trust: &Trust) -> Option<Self> {
        match trust {
            Trust::Anchored { length, revocation } => Some(Self {
                length: *length,
                revocation: revocation.clone(),
            }),
            _ => None,
        }
    }

    /// RFC 5280 section 6.1's `n`: how many certificates the path holds, the anchor excluded.
    #[must_use]
    pub const fn path_length(&self) -> usize {
        self.length
    }

    /// What §12.8.4's material in the document said about that path.
    #[must_use]
    pub const fn revocation(&self) -> &Revocation {
        &self.revocation
    }
}

/// What a host will accept where §12.8.4's material answers nothing.
///
/// **A policy, asked once, where a host can supply it** — `CLAUDE.md` principle 3's shape, and the
/// same shape ADR 1039 gave the anchors themselves. The alternative is a refusal hard-coded at the
/// point of the operation, which that principle names as the thing to avoid.
///
/// ADR 1067's rule is not what this relaxes and cannot be: [`Revocation::Good`] stays "only ever
/// the output of arithmetic this program performed", and an absence of material stays
/// [`Revocation::Unknown`] with its reason kept. What this decides is what the *host* does with an
/// `Unknown` — and the two answers are different sentences to a reader rather than the same
/// sentence with a different threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Acceptance {
    /// Nothing short of a [`Revocation::Good`] this program computed will do.
    ///
    /// The default, because a host that has not said is a host that has not decided, and deciding
    /// on its behalf in the permissive direction is how an unchecked certificate comes to look
    /// checked.
    #[default]
    RevocationMustBeGood,
    /// An [`Undetermined`] status is admitted, and the reason travels into the verdict.
    ///
    /// The honest case for this is §12.8.4's own: a document whose store carries nothing says
    /// nothing about revocation, and RFC 5280 section 6.3.3's remedy — fetch a newer list — is the
    /// one branch a document cannot supply and `CLAUDE.md` principle 3 gives this program no
    /// network for. A host that knows its anchors and its documents may reasonably decide that the
    /// other four answers are enough; [`Valid::revocation`] is what keeps that visible afterwards.
    UnknownRevocationAccepted,
}

/// What §12.8.5's chain of document timestamps contributes to a verdict.
///
/// §12.8.5 is optional, so a document carrying no timestamp reserves nothing: [`Self::None`] is the
/// ordinary case and the overwhelming majority of signed documents. Where a document *does* carry
/// one, ADR 1071's four steps decide whether an instant was established, and a chain this program
/// could not establish is a statement the document makes that this program could not check — which
/// is a reservation rather than a detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Timestamps {
    /// The document carries no `/Type /DocTimeStamp` dictionary.
    None,
    /// The outermost token's instant, established under ADR 1071's four steps.
    Established(Instant),
    /// The document carries at least one and this program established no instant from it.
    NotEstablished,
}

/// Why the word was not said.
///
/// One variant per thing that can stop it, each carrying what it was told, because which of them
/// stopped it decides what a person can do about it: a document that moved is a fact about the
/// file, an anchor nobody named is a fact about this program's inputs, and an unknown revocation is
/// a fact about what the document preserved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Reservation {
    /// Nobody supplied an anchor, so §12.8.1's third question was never asked.
    ///
    /// This program's own default answer, and not a defect in the document (ADR 1039).
    #[error("nobody has named a certification authority to end this signature's chain at")]
    NoAnchorSupplied,
    /// No path from the signer's certificate reached any anchor the host supplied.
    #[error(
        "no certification path from the signer's certificate reaches any anchor supplied ({examined} certificate(s) examined)"
    )]
    NoPathToAnyAnchor {
        /// How many certificates the search looked at.
        examined: usize,
    },
    /// Every path the search found was refused by RFC 5280 section 6.1.
    #[error("every certification path to a supplied anchor was refused: {refusal}")]
    PathRefused {
        /// The first refusal any prospective path produced.
        refusal: PathRefusal,
    },
    /// The bytes `/ByteRange` names no longer hash to the digest the signature records.
    #[error("the bytes that signature covers were modified after it was signed (§12.8.1)")]
    TheDocumentChanged,
    /// §12.8.1's question 1 was not answered at all — an unreadable range, an unknown digest, a
    /// signature value that is not a CMS object.
    #[error("whether the document changed since it was signed could not be decided")]
    ChangeNotDecided,
    /// The signature does not verify under the key in the signer's certificate.
    #[error(
        "the signature does not verify under the key in the signer's certificate (§12.8.3.3.1)"
    )]
    TheSignatureDoesNotVerify,
    /// §12.8.1's question 2 was not answered at all — no signer certificate, a key this program
    /// does not compute on, a certificate that would not parse.
    #[error("whether the signature verifies under the signer's key could not be decided")]
    VerificationNotDecided,
    /// §12.8.4's material revokes a certificate on the path.
    #[error(
        "a certificate on the certification path is revoked (certificate {position} of the path)"
    )]
    Revoked {
        /// How far down the path the revoked certificate is, the target being 0.
        position: usize,
    },
    /// No revocation status could be determined, and this host's [`Acceptance`] does not admit one.
    #[error("the revocation status of a certificate on the path is undetermined: {why}")]
    RevocationUndetermined {
        /// RFC 5280 section 6.3.3's `UNDETERMINED` with its reason kept.
        why: Undetermined,
    },
    /// Revocation was not asked at all, because the caller supplied no §12.8.4 material.
    #[error("revocation was not checked, because no §12.8.4 material was supplied")]
    RevocationNotChecked,
    /// The document carries §12.8.5 timestamps and this program established no instant from them.
    #[error("this document's §12.8.5 timestamp chain establishes no instant")]
    TimestampNotEstablished,
    /// The path validation answered something this build has no sentence for.
    ///
    /// Unreachable today and here for the reason [`Verdict::of`]'s last arm states: [`Trust`] is
    /// `#[non_exhaustive]`, and a verdict is the wrong place to discover that by guessing.
    #[error("the certification path answered something this build does not recognise")]
    PathAnswerNotRecognised,
}

/// A signature this program is prepared to call valid, and what that rests on.
///
/// **No public constructor and no public field.** [`Verdict::reached`] is the only function that
/// makes one and it requires an [`Anchored`]; the accessors below are how a report says what the
/// word was based on. That is the whole mechanism, and it is a type rather than a rule in a comment
/// because a rule in a comment is kept by whoever reads it (ADR 1076).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Valid {
    path_length: usize,
    revocation: Revocation,
    at: Instant,
    timestamps: Timestamps,
}

impl Valid {
    /// RFC 5280 section 6.1's `n`: how many certificates the path to the anchor holds.
    #[must_use]
    pub const fn path_length(&self) -> usize {
        self.path_length
    }

    /// What §12.8.4's material said about that path — a [`Revocation::Good`] this program computed,
    /// or an [`Revocation::Unknown`] the host's [`Acceptance::UnknownRevocationAccepted`] admitted.
    #[must_use]
    pub const fn revocation(&self) -> &Revocation {
        &self.revocation
    }

    /// RFC 5280 section 6.1.1's input (b): the instant the path was validated at.
    #[must_use]
    pub const fn at(&self) -> Instant {
        self.at
    }

    /// What §12.8.5's chain contributed.
    #[must_use]
    pub const fn timestamps(&self) -> Timestamps {
        self.timestamps
    }
}

/// §12.8.1's three questions answered together, for one signature.
///
/// **Two variants, closed, and closed on purpose**: a verdict either says the word or says why it
/// does not, and a third state would be a way of saying it partly. [`Reservation`] is where the
/// reasons live and it is the enumeration that grows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Every one of them is answered and every answer is the one that permits the word.
    Valid(Valid),
    /// It is not, and this is the first thing that stopped it.
    Reserved(Reservation),
}

impl Verdict {
    /// The verdict, from every answer §12.8.1 divides the work into.
    ///
    /// `trust` decides whether [`Self::reached`] is reachable at all: an [`Anchored`] is made from
    /// it or the verdict is the reservation that answer names.
    #[must_use]
    pub fn of(
        integrity: &Integrity,
        authenticity: &Authenticity,
        trust: &Trust,
        acceptance: Acceptance,
        timestamps: Timestamps,
        at: Instant,
    ) -> Self {
        match Anchored::of(trust) {
            Some(anchored) => Self::reached(
                integrity,
                authenticity,
                &anchored,
                acceptance,
                timestamps,
                at,
            ),
            None => Self::Reserved(match trust {
                Trust::NoAnchorSupplied => Reservation::NoAnchorSupplied,
                Trust::NoPathToAnyAnchor { examined } => Reservation::NoPathToAnyAnchor {
                    examined: *examined,
                },
                Trust::Refused { refusal, .. } => Reservation::PathRefused {
                    refusal: refusal.clone(),
                },
                // Unreachable while `Anchored::of` answers `Some` for exactly `Trust::Anchored`,
                // and named rather than folded into one of the three above: [`Trust`] is
                // `#[non_exhaustive]`, and a variant added later must arrive as a reservation of
                // its own rather than as somebody else's sentence.
                _ => Reservation::PathAnswerNotRecognised,
            }),
        }
    }

    /// The verdict where a path reached a supplied anchor — **the only route to [`Valid`]**.
    ///
    /// The order of the checks is §12.8.1's own division of the work, so that the reservation a
    /// caller gets names the earliest thing that stopped the word rather than the last.
    #[must_use]
    pub fn reached(
        integrity: &Integrity,
        authenticity: &Authenticity,
        anchored: &Anchored,
        acceptance: Acceptance,
        timestamps: Timestamps,
        at: Instant,
    ) -> Self {
        // Question 1. `UnderTheSignersKey` is not a failure: §12.8.3.2's value and a `SignerInfo`
        // with no signed attributes record no digest in the open, and question 2's answer *is*
        // question 1's for those two (`viewer_core::notes` words the same case).
        match integrity {
            Integrity::Unchanged { .. } | Integrity::UnderTheSignersKey => {}
            Integrity::Changed { .. } => return Self::Reserved(Reservation::TheDocumentChanged),
            _ => return Self::Reserved(Reservation::ChangeNotDecided),
        }
        // Question 2.
        match authenticity {
            Authenticity::Verified { .. } => {}
            Authenticity::NotUnderThatKey { .. } => {
                return Self::Reserved(Reservation::TheSignatureDoesNotVerify);
            }
            _ => return Self::Reserved(Reservation::VerificationNotDecided),
        }
        // Question 3's remaining half: the path reached an anchor, and this is what the document's
        // own material said about it (ADR 1067).
        match anchored.revocation() {
            Revocation::Good { .. } => {}
            Revocation::Revoked { position, .. } => {
                return Self::Reserved(Reservation::Revoked {
                    position: *position,
                });
            }
            Revocation::Unknown { why, .. } => {
                if acceptance == Acceptance::RevocationMustBeGood {
                    return Self::Reserved(Reservation::RevocationUndetermined {
                        why: why.clone(),
                    });
                }
            }
            Revocation::NotChecked => {
                if acceptance == Acceptance::RevocationMustBeGood {
                    return Self::Reserved(Reservation::RevocationNotChecked);
                }
            }
        }
        if timestamps == Timestamps::NotEstablished {
            return Self::Reserved(Reservation::TimestampNotEstablished);
        }
        Self::Valid(Valid {
            path_length: anchored.path_length(),
            revocation: anchored.revocation().clone(),
            at,
            timestamps,
        })
    }

    /// The [`Valid`] where there is one.
    #[must_use]
    pub const fn valid(&self) -> Option<&Valid> {
        match self {
            Self::Valid(valid) => Some(valid),
            Self::Reserved(_) => None,
        }
    }

    /// Why the word was not said, where it was not.
    #[must_use]
    pub const fn reservation(&self) -> Option<&Reservation> {
        match self {
            Self::Valid(_) => None,
            Self::Reserved(reservation) => Some(reservation),
        }
    }
}

#[cfg(test)]
mod tests;

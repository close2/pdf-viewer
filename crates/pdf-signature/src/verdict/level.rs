//! ETSI TS 119 172-4 clause 4.4.2's technical applicability rules check: whether a valid signature
//! is technically suitable to implement an EU qualified electronic signature or seal.
//!
//! # The process, requirement by requirement
//!
//! - **REQ-4.4.2-01** runs ETSI TS 119 615 clause 4.4 for the signing certificate at the best
//!   signature time; [`crate::trusted_list::determination::qualification`] is that run, and
//!   [`crate::verdict::Listing::qualification`] carries it (ADR 2864).
//! - **REQ-4.4.2-02**: where clause 4.4's `QC-Status` is `PROCESS_PASSED` and its `QC-Results`
//!   hold `QC_For_eSig` or `QC_For_eSeal`, the signing certificate is technically determined an EU
//!   qualified certificate for electronic signatures, respectively seals; otherwise the process
//!   stops and the signature is indeterminate — neither an EU qualified electronic signature nor
//!   seal.
//! - **REQ-4.4.2-03** stops the process where "any of the checks specified in REQ-4.4.2-03"
//!   fails. The requirement names itself and states no check; its NOTE 3 points at Article 32(1)
//!   points (c) to (e) of Regulation (EU) No 910/2014, which REQ-4.5-01 b) and c) make content of
//!   the report rather than conditions of the process. So no check is made here, and that is a
//!   reading of a silence, written down as one (ADR 2866).
//! - **REQ-4.4.2-04** runs ETSI TS 119 615 clause 4.5 at the same instant;
//!   [`crate::trusted_list::determination::qscd`] is that run (ADR 2865).
//! - **REQ-4.4.2-05**: where clause 4.5's `QSCD-Status` is `PROCESS_PASSED` and its
//!   `QSCD-Results` hold `QSCD_YES`, the signature is technically determined as created by a
//!   qualified signature or seal creation device; otherwise the process stops, indeterminate.
//! - **REQ-4.4.2-06** joins the two determinations with the result of clause 4.2's validation:
//!   where that result is `TOTAL-PASSED` the signature is technically suitable to implement an EU
//!   qualified electronic signature (respectively seal), and otherwise it is not determined as
//!   either.
//!
//! `PROCESS_PASSED` is read as the value it names: `PROCESS_PASSED_WITH_WARNING` is another value
//! of the same output (ETSI TS 119 615 PRO-4.4.4-32 d) and -36 b) set it), and REQ-4.4.2-02 and -05
//! name only the first.
//!
//! # Clause 4.2's validation
//!
//! REQ-4.2-01 makes it ETSI EN 319 102-1's validation and REQ-4.2-03 fixes its constraints: the
//! trust anchors are the digital identities of the `CA/QC` services clause 4.3 of TS 119 615 finds
//! at the signing certificate's `notBefore` (a), which is what a [`Standing::Listed`] path ended
//! at; revocation is checked by either kind of material (c) i)); and the freshness is zero, so that
//! revocation information "is only accepted if it has been issued after the best signature time"
//! (c) ii)). [`determine`] runs this tree's verdict again over the listed service's identities
//! under exactly those constraints — [`crate::trust::PathInputs::revocation_issuance`] for the
//! freshness and [`Acceptance::RevocationMustBeGood`] for the check — and its [`Valid`] is the
//! `TOTAL-PASSED` REQ-4.4.2-06 c) asks for. A host's own acceptance of an unknown status, which
//! may make the word *valid* sayable, does not make this one.
//!
//! **A locally valid path has no level at all** (ADR 2834): its anchor is the reader's own, which
//! is not REQ-4.2-03 a)'s set, so clause 4.2's validation was not the one that passed.
//!
//! # Annex A's types
//!
//! REQ-4.5-02 lets a report identify the type of a signature with Annex A's object identifiers.
//! The process's outputs support five of its seven types: an EU qualified signature or seal where
//! REQ-4.4.2-06 holds; an advanced signature or seal supported by an EU qualified certificate
//! where the certificate was determined under REQ-4.4.2-02 a) and the validation passed; and an
//! advanced signature or seal where only the validation passed — NOTE 5 to REQ-4.4.2-05 reads
//! Article 32(1) point (h), the requirements of Article 26 for an advanced signature, as met when
//! that validation is `TOTAL-PASSED`. The last does not say whether it is a signature or a seal,
//! because the certificate determined neither, so it carries no single identifier ([`Type`]).

use super::{Acceptance, Anchored, Standing, Timestamps, Valid, Verdict};
use crate::revocation::{Issuance, Material};
use crate::signature::{Authenticity, Integrity, Signature};
use crate::trust::{PathInputs, Purpose as KeyUse, TrustAnchors};
use crate::trusted_list::TrustedList;
use crate::trusted_list::determination::qscd::QscdIndication;
use crate::trusted_list::determination::{Indication, QcStatus, Qualification};

/// Which of the clause's "respectively" a determination is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Electronic signatures: `QC_For_eSig`.
    Signature,
    /// Electronic seals: `QC_For_eSeal`.
    Seal,
}

/// The requirement of clause 4.4.2 that decided a [`Level`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// REQ-4.4.2-02: the certificate was not determined an EU qualified certificate.
    Certificate,
    /// REQ-4.4.2-05: the key was not determined to reside in a qualified creation device.
    Device,
    /// REQ-4.4.2-06: the join.
    Join,
}

impl Requirement {
    /// The requirement's number as the clause prints it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Certificate => "REQ-4.4.2-02",
            Self::Device => "REQ-4.4.2-05",
            Self::Join => "REQ-4.4.2-06",
        }
    }
}

/// The clause's answer about the signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// REQ-4.4.2-06's first branch: technically suitable to implement an EU qualified electronic
    /// signature, respectively seal.
    Qualified(Purpose),
    /// REQ-4.4.2-02 b) or -05 b): the process stopped, and the signature is technically
    /// determined as indeterminate — neither an EU qualified electronic signature nor seal.
    Indeterminate,
    /// REQ-4.4.2-06's second branch: certificate and device were determined, clause 4.2's
    /// validation did not pass, and the signature is not determined as either.
    NotQualified,
}

/// ETSI TS 119 172-4 Annex A's digital signature types the process's outputs support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// i) EU qualified electronic signature.
    QualifiedSignature,
    /// ii) Advanced electronic signature supported by an EU qualified certificate for electronic
    /// signature.
    AdvancedSignatureOnQualifiedCertificate,
    /// iv) EU qualified electronic seal.
    QualifiedSeal,
    /// v) Advanced electronic seal supported by an EU qualified certificate for electronic seal.
    AdvancedSealOnQualifiedCertificate,
    /// iii) or vi): an advanced electronic signature or seal, the certificate having determined
    /// neither.
    Advanced,
}

impl Type {
    /// The type's object identifier under `id-etsi-sars-SigType` (Annex A item 2) b)), dotted;
    /// `None` for [`Self::Advanced`], which is one of two types.
    #[must_use]
    pub const fn identifier(self) -> Option<&'static str> {
        match self {
            Self::QualifiedSignature => Some("0.4.0.191724.1.2.1"),
            Self::AdvancedSignatureOnQualifiedCertificate => Some("0.4.0.191724.1.2.2"),
            Self::QualifiedSeal => Some("0.4.0.191724.1.2.4"),
            Self::AdvancedSealOnQualifiedCertificate => Some("0.4.0.191724.1.2.5"),
            Self::Advanced => None,
        }
    }
}

/// What clause 4.4.2's process determined for one signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    /// The answer about the signature.
    pub answer: Answer,
    /// The requirement that decided it: where the process stopped, or the join.
    pub decided_by: Requirement,
    /// REQ-4.4.2-02 a)'s determination of the signing certificate, where it was made.
    pub certificate: Option<Purpose>,
    /// Whether clause 4.2's validation, under REQ-4.2-03's constraints, was `TOTAL-PASSED`.
    pub total_passed: bool,
}

impl Level {
    /// Annex A's type the outputs support, where they support one: none where clause 4.2's
    /// validation did not pass.
    #[must_use]
    pub const fn annex_a(&self) -> Option<Type> {
        if !self.total_passed {
            return None;
        }
        Some(match (self.answer, self.certificate) {
            (Answer::Qualified(Purpose::Signature), _) => Type::QualifiedSignature,
            (Answer::Qualified(Purpose::Seal), _) => Type::QualifiedSeal,
            (_, Some(Purpose::Signature)) => Type::AdvancedSignatureOnQualifiedCertificate,
            (_, Some(Purpose::Seal)) => Type::AdvancedSealOnQualifiedCertificate,
            (_, None) => Type::Advanced,
        })
    }
}

/// REQ-4.4.2-02 to -06 over clause 4.4's and clause 4.5's outputs and clause 4.2's result.
///
/// A [`Qualification`] other than `Determined` is a clause 4.4 that did not run, whose `QC-Status`
/// therefore holds no `PROCESS_PASSED`: REQ-4.4.2-02 b).
#[must_use]
pub fn join(qualification: &Qualification, total_passed: bool) -> Level {
    let stopped = |decided_by, certificate| Level {
        answer: Answer::Indeterminate,
        decided_by,
        certificate,
        total_passed,
    };
    let Qualification::Determined {
        determination,
        qscd,
        ..
    } = qualification
    else {
        return stopped(Requirement::Certificate, None);
    };
    // REQ-4.4.2-02. Both purposes at once is Table 4's error, which sets `PROCESS_FAILED`
    // (PRO-4.4.4-32 b)), so a passed run never holds the pair; were one to, "respectively" would
    // name no single certificate type, and the process is not let through on it.
    let holds = |indication| determination.results.contains(&indication);
    let certificate = match (
        determination.status,
        holds(Indication::QcForESig),
        holds(Indication::QcForESeal),
    ) {
        (QcStatus::Passed, true, false) => Purpose::Signature,
        (QcStatus::Passed, false, true) => Purpose::Seal,
        _ => return stopped(Requirement::Certificate, None),
    };
    // REQ-4.4.2-03 names no check (the module documentation), so the process goes on.
    // REQ-4.4.2-05.
    if !(qscd.status == QcStatus::Passed && qscd.result == Some(QscdIndication::Yes)) {
        return stopped(Requirement::Device, Some(certificate));
    }
    // REQ-4.4.2-06.
    Level {
        answer: if total_passed {
            Answer::Qualified(certificate)
        } else {
            Answer::NotQualified
        },
        decided_by: Requirement::Join,
        certificate: Some(certificate),
        total_passed,
    }
}

/// What [`determine`] asks clause 4.2's validation over, beside the verdict it determines a level
/// for: the answers §12.8.1's first two questions got, the lists the host supplied and the
/// document's revocation material and timestamp chain.
#[derive(Debug, Clone, Copy)]
pub struct Asked<'a, 'm> {
    /// The signature.
    pub signature: &'a Signature,
    /// §12.8.1's first question's answer.
    pub integrity: &'a Integrity,
    /// Its second question's answer.
    pub authenticity: &'a Authenticity,
    /// The trusted lists the host supplied.
    pub lists: &'a [TrustedList],
    /// §12.8.4's material.
    pub material: &'a Material<'m>,
    /// §12.8.5's chain.
    pub timestamps: Timestamps,
}

/// The level of a [`Valid`] signature, or `None` where its path did not end at a listed service.
///
/// Clause 4.2's validation is run again over the identities of the service the valid path ended
/// at, under REQ-4.2-03's constraints (the module documentation), at the same best signature time;
/// its `TOTAL-PASSED` is that run's [`Valid`].
#[must_use]
pub fn determine(valid: &Valid, asked: &Asked<'_, '_>) -> Option<Level> {
    let Standing::Listed(listing) = valid.standing() else {
        return None;
    };
    let when = valid.when();
    let services = crate::trusted_list::listed_anchors(asked.lists, asked.signature, when.at());
    let identities = services
        .into_iter()
        .find(|(found, _)| found == listing.as_ref())
        .map(|(_, identities)| identities)
        .unwrap_or_default();
    let parsed: Vec<_> = identities
        .iter()
        .filter_map(|der| crate::x509::parse(der).ok())
        .collect();
    let inputs = PathInputs {
        // REQ-4.2-03 c) ii): a freshness of zero against the best signature time.
        revocation_issuance: Issuance {
            fresh_after: Some(when.at()),
            ..Issuance::default()
        },
        ..PathInputs::default()
    };
    let trust = asked.signature.trust_under(
        &TrustAnchors::of(&parsed),
        asked.material,
        when.at(),
        KeyUse::Unstated,
        &inputs,
    );
    let total_passed = Anchored::listed(&trust, listing.as_ref().clone()).is_some_and(|anchored| {
        Verdict::reached(
            asked.integrity,
            asked.authenticity,
            &anchored,
            // REQ-4.2-03 c) i): the status is checked, and an unknown one is not accepted.
            Acceptance::RevocationMustBeGood,
            asked.timestamps,
            when,
        )
        .valid()
        .is_some()
    });
    Some(join(&listing.qualification, total_passed))
}

#[cfg(test)]
mod tests;

//! RFC 5755 section 5's validation of an attribute certificate a signature carries as a certified
//! signer attribute, under the trust trees a policy's attribute condition states (ADR 2797).
//!
//! Section 5 lists seven conditions every valid attribute certificate satisfies, and each is
//! answered here or named:
//!
//! 1. The holder's public-key certificate is found and its path verified. The holder is the
//!    signer: section 4.2.2 binds a `baseCertificateID` to the certificate of that issuer and
//!    serial number, and an `entityName` to the certificate's subject or one of its alternative
//!    names. The signer's path is the one the policy's signing-certificate condition searched.
//! 2. The signature verifies under the issuer's public key, and the issuer's whole path verifies.
//!    The issuer's certificate is the carried certificate whose subject is the issuer's name and
//!    whose key verifies the signature; its path is searched to the condition's trust trees.
//! 3. The issuer's certificate follows section 4.5: its key usage does not refuse signatures, and
//!    it is not a CA.
//! 4. The issuer is directly trusted as an attribute authority "by configuration or otherwise".
//!    A signature policy is that configuration: TR 102 272 clause 6.9's `attrCertificateTrustTrees`
//!    states the path conditions for any attribute certificate, and TS 119 172-2 clause 4.26
//!    item 2 the trust trees of a set's certified attributes, so a path to them is the trust.
//! 5. The instant evaluated at is within the validity period, both ends included.
//! 6. Targeting passes: this program is a signature's verifier and names no server or group a
//!    target could list, so whether it matches is not decided, and a targeted certificate is named.
//! 7. No critical extension this reader does not support is present.
//!
//! Where the policy states a revocation requirement for attribute certificates — TR 102 272 clause
//! 6.9's `attrRevReq`, TS 119 172-2 clause 4.26's `AttributeRevocationTrust`, TS 119 172-3 clause
//! 4.26's `attrRevReq` — the certificate's chain is checked by
//! [`revocation_checks::attribute_checks`]: the certificate and its authority's by the end-entity
//! check, the authority path's CA certificates by the CA check. RFC 5755 section 5's additional
//! check 1 lets a verifier's configuration reject a certificate, and the policy is that
//! configuration, so a check that finds one revoked makes the certificate invalid and one the file's
//! material does not answer leaves it undecided (ADR 2890).

use crate::attribute_certificate::{self, AttributeCertificate, Holder, Unreadable};
use crate::cms::SignedData;
use crate::der::{Reader, SEQUENCE, Value};
use crate::policy_rules::certification_path::{self, Found, Path, Search, TrustTrees};
use crate::policy_rules::revocation_checks::{self, KindCheck, Requirement, Timing};
use crate::policy_rules::{ATTRIBUTE_TRUST, Verdict};
use crate::trust;
use crate::x509::{self, Certificate, Instant};

/// Why an attribute certificate is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invalid {
    /// It is not section 4.1's syntax as section 4.2 profiles it.
    Unreadable(Unreadable),
    /// Its `acinfo` stated an indefinite length.
    Indefinite,
    /// Its holder is not the signer (condition 1).
    NotTheSigners,
    /// The signer's own path failed the policy's signing-certificate condition (condition 1).
    SignerPath,
    /// No carried certificate of the issuer's name verifies its signature (condition 2).
    Signature,
    /// The issuer's path to the trust trees is refused (conditions 2 and 4). Boxed because a
    /// search's finding is many times the size of every other variant.
    Path(Box<Found>, usize),
    /// The issuer's certificate is a CA's (condition 3, section 4.5).
    IssuerIsCa,
    /// The issuer's key usage refuses signatures (condition 3, section 4.5).
    IssuerKeyUsage,
    /// The instant is outside the validity period (condition 5).
    Outside,
    /// A critical extension this reader does not support (condition 7), as dotted decimal.
    UnsupportedCritical(String),
    /// The policy's revocation check found a certificate of its chain revoked at or before the
    /// instant. Boxed as [`Self::Path`] is.
    Revoked(Box<KindCheck>),
}

/// Why an attribute certificate's validity is not decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Undecided {
    /// It holds more entries than this program reads.
    TooMany(&'static str),
    /// The signature does not carry the certificate its signer names.
    NoSignerCertificate,
    /// The holder names the signer in a way this program does not compare.
    Holder,
    /// The signer's path was not decided under the policy's signing-certificate condition.
    SignerPath,
    /// No carried certificate bears the issuer's name.
    NoIssuerCertificate,
    /// Its signature algorithm, as dotted decimal, is one this program does not verify.
    Algorithm(String),
    /// The set of constraints states no trust trees for attribute authorities.
    NoTrustTrees,
    /// The issuer's path to the trust trees is not decided. Boxed as [`Invalid::Path`] is.
    Path(Box<Found>, usize),
    /// No instant was given to evaluate at.
    NoInstant,
    /// It is targeted (condition 6).
    Targeted,
    /// The policy's revocation check is not answered for a certificate of its chain, and whether
    /// the attribute certificate states `noRevAvail` (RFC 5755 section 4.3.6). Boxed as
    /// [`Self::Path`] is.
    Revocation(Box<KindCheck>, bool),
}

/// What section 5 found of one attribute certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Every condition holds.
    Valid,
    /// A condition fails; the first in section 5's order.
    Invalid(Invalid),
    /// No condition fails, and one is not decided; the first in section 5's order.
    Undecided(Undecided),
}

impl Outcome {
    /// The verdict on the policy's conditions on this certificate.
    #[must_use]
    pub const fn verdict(&self) -> Verdict {
        match self {
            Self::Valid => Verdict::Met,
            Self::Invalid(_) => Verdict::Broken,
            Self::Undecided(_) => Verdict::NotEvaluated,
        }
    }

    /// The part of a sentence that says why.
    pub(super) fn words(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Valid => f.write_str(
                "its holder is the signer, its issuer's signature verifies, and its issuer's path validates to the condition's trust trees at an instant within its validity (RFC 5755 section 5)",
            ),
            Self::Invalid(why) => match why {
                Invalid::Unreadable(why) => write!(f, "{why}, so it is no valid attribute certificate"),
                Invalid::Indefinite => f.write_str(
                    "its acinfo states an indefinite length, so the extent its issuer signed is not read from a length",
                ),
                Invalid::NotTheSigners => {
                    f.write_str("its holder is not the signer (RFC 5755 sections 4.2.2 and 5)")
                }
                Invalid::SignerPath => f.write_str(
                    "the signer's path fails the policy's signing certificate trust condition, and RFC 5755 section 5 requires the holder's path verified",
                ),
                Invalid::Signature => f.write_str(
                    "no carried certificate of its issuer's name verifies its signature (RFC 5755 section 5 item 2)",
                ),
                Invalid::Path(found, unexamined) => {
                    certification_path::path_words(f, ATTRIBUTE_TRUST, found, *unexamined)
                }
                Invalid::IssuerIsCa => f.write_str(
                    "its issuer's certificate is a CA's, which RFC 5755 section 4.5 forbids an attribute authority",
                ),
                Invalid::IssuerKeyUsage => f.write_str(
                    "its issuer's key usage does not admit signatures (RFC 5755 section 4.5)",
                ),
                Invalid::Outside => f.write_str(
                    "the instant validated at is outside its validity period (RFC 5755 section 5 item 5)",
                ),
                Invalid::UnsupportedCritical(id) => write!(
                    f,
                    "it carries critical extension {id}, which this program does not support, and RFC 5755 section 5 item 7 rejects it"
                ),
                Invalid::Revoked(check) => {
                    revocation_checks::sentence(f, ATTRIBUTE_TRUST, check, Verdict::Broken.word())
                }
            },
            Self::Undecided(why) => match why {
                Undecided::TooMany(list) => {
                    write!(f, "its {list} hold more entries than this program reads")
                }
                Undecided::NoSignerCertificate => {
                    f.write_str("the signature does not carry the certificate its signer names")
                }
                Undecided::Holder => f.write_str(
                    "its holder names the signer in a form this program does not compare",
                ),
                Undecided::SignerPath => f.write_str(
                    "the signer's path is not decided under the policy's signing certificate trust condition",
                ),
                Undecided::NoIssuerCertificate => f.write_str(
                    "the signature carries no certificate of its issuer's name",
                ),
                Undecided::Algorithm(id) => write!(
                    f,
                    "its signature algorithm {id} is one this program does not verify"
                ),
                Undecided::NoTrustTrees => f.write_str(
                    "the set of constraints states no trust trees for attribute authorities",
                ),
                Undecided::Path(found, unexamined) => {
                    certification_path::path_words(f, ATTRIBUTE_TRUST, found, *unexamined)
                }
                Undecided::NoInstant => f.write_str(
                    "no instant was given to evaluate it at, neither by a host's clock nor by a signature timestamp",
                ),
                Undecided::Targeted => f.write_str(
                    "it is targeted, and whether a signature's verifier is among its targets is not decided (RFC 5755 section 4.3.2)",
                ),
                Undecided::Revocation(check, no_revocation) => {
                    revocation_checks::sentence(
                        f,
                        ATTRIBUTE_TRUST,
                        check,
                        Verdict::NotEvaluated.word(),
                    )?;
                    if *no_revocation {
                        f.write_str(
                            "; the attribute certificate states noRevAvail, its issuer's word that no revocation information will be made available for it (RFC 5755 section 4.3.6)",
                        )?;
                    }
                    Ok(())
                }
            },
        }
    }
}

/// One attribute certificate, judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Judged<'a> {
    /// The types of the attributes it carries; `None` where it was not read.
    pub types: Option<Vec<&'a [u8]>>,
    /// What section 5 found.
    pub outcome: Outcome,
}

impl Judged<'_> {
    /// Whether it may carry an attribute `wanted` accepts: it carries one, or was not read.
    pub(super) fn may_carry(&self, wanted: impl Fn(&[u8]) -> bool) -> bool {
        self.types
            .as_ref()
            .is_none_or(|types| types.iter().any(|kind| wanted(kind)))
    }

    /// Whether it is valid and carries an attribute `wanted` accepts.
    pub(super) fn certifies(&self, wanted: impl Fn(&[u8]) -> bool) -> bool {
        self.outcome == Outcome::Valid && self.may_carry(wanted)
    }

    /// Whether its validity is undecided and it may carry an attribute `wanted` accepts.
    pub(super) fn might_certify(&self, wanted: impl Fn(&[u8]) -> bool) -> bool {
        matches!(self.outcome, Outcome::Undecided(_)) && self.may_carry(wanted)
    }
}

/// What a judgement of attribute certificates is made under.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Conditions<'c> {
    /// The trust trees an issuer's path is searched to.
    pub trees: Option<&'c TrustTrees<'c>>,
    /// The search of the signer's path under the signing-certificate condition, where made.
    pub signer_path: Option<&'c Search>,
    /// The instant evaluated at.
    pub at: Option<Instant>,
    /// The policy's revocation requirement for attribute certificates, where it states one.
    pub revocation: Option<&'c Requirement>,
    /// The revocation material the requirement's checks are answered from.
    pub material: &'c crate::revocation::Material<'c>,
    /// What places the requirement's issuance bounds on the time line.
    pub timing: Timing,
    /// The trusted lists a trust tree's references are determined from, where a host supplied any.
    pub lists: Option<crate::trusted_list::anchors::Supplied<'c>>,
}

/// What made one attribute certificate: the algorithm its issuer signed it under, and the length
/// of the key that verified that signature — what TR 102 272 clause 6.10's
/// `aaCertAlgorithmConstraints` and TS 119 172-2 clause 4.29's usage for "generating attribute
/// certificates" constrain (ADR 2891).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Made {
    /// `signatureAlgorithm`'s object identifier; `None` where the certificate does not read.
    pub algorithm: Option<Vec<u8>>,
    /// The issuer's key length in bits, where a carried certificate of the issuer's name verifies
    /// the signature and this reader measures its family.
    pub key_bits: Option<usize>,
}

/// [`Made`] for each attribute certificate the signature carries, in the order
/// [`super::Carried::certified`] holds them.
///
/// # Errors
///
/// The signer attributes do not read, so which attribute certificates are carried is not known.
pub(crate) fn made(cms: &SignedData<'_>) -> Result<Vec<Made>, super::Unreadable> {
    let carried = super::carried(cms)?;
    Ok(carried
        .certified
        .iter()
        .map(
            |certificate| match attribute_certificate::read(*certificate) {
                Err(_) => Made {
                    algorithm: None,
                    key_bits: None,
                },
                Ok(read) => Made {
                    algorithm: Some(read.signature_algorithm.to_vec()),
                    key_bits: issuer(&read, cms)
                        .ok()
                        .and_then(|(_, issuer)| crate::policy_rules::key_bits(&issuer.public_key)),
                },
            },
        )
        .collect())
}

/// The types of an attribute certificate's attributes, where it reads.
pub(super) fn types_of(certificate: Value<'_>) -> Option<Vec<&[u8]>> {
    attribute_certificate::read(certificate)
        .ok()
        .map(|read| read.types)
}

/// RFC 5755 section 5's conditions on `certificate`, one of `cms`'s signer attributes.
pub(super) fn judge<'a>(
    certificate: Value<'a>,
    cms: &SignedData<'a>,
    conditions: Conditions<'_>,
) -> Judged<'a> {
    let read = match attribute_certificate::read(certificate) {
        Ok(read) => read,
        Err(Unreadable::TooMany(list)) => {
            return Judged {
                types: None,
                outcome: Outcome::Undecided(Undecided::TooMany(list)),
            };
        }
        Err(why) => {
            return Judged {
                types: None,
                outcome: Outcome::Invalid(Invalid::Unreadable(why)),
            };
        }
    };
    let outcome = outcome(&read, cms, conditions);
    Judged {
        types: Some(read.types),
        outcome,
    }
}

/// Every condition, in section 5's order; the first failure decides, else the first undecided.
fn outcome(
    read: &AttributeCertificate<'_>,
    cms: &SignedData<'_>,
    conditions: Conditions<'_>,
) -> Outcome {
    let mut undecided = None;
    let mut fail = |result: Result<(), Result<Invalid, Undecided>>| match result {
        Ok(()) => None,
        Err(Ok(invalid)) => Some(invalid),
        Err(Err(why)) => {
            undecided.get_or_insert(why);
            None
        }
    };
    if read.indefinite {
        return Outcome::Invalid(Invalid::Indefinite);
    }
    let mut authority_path: Option<Path> = None;
    if let Some(invalid) = fail(holder_condition(&read.holder, cms, conditions.signer_path)) {
        return Outcome::Invalid(invalid);
    }
    match issuer(read, cms) {
        Err(why) => {
            if let Some(invalid) = fail(Err(why)) {
                return Outcome::Invalid(invalid);
            }
        }
        Ok((encoding, certificate)) => {
            let searched = path(encoding, cms, conditions);
            for result in [
                profile(&certificate),
                searched.as_ref().map(|_| ()).map_err(Clone::clone),
            ] {
                if let Some(invalid) = fail(result) {
                    return Outcome::Invalid(invalid);
                }
            }
            authority_path = searched.ok().flatten();
        }
    }
    let timely = match conditions.at {
        None => Err(Err(Undecided::NoInstant)),
        Some(at) if at < read.validity.not_before || at > read.validity.not_after => {
            Err(Ok(Invalid::Outside))
        }
        Some(_) => Ok(()),
    };
    let targeted = if read.targeted {
        Err(Err(Undecided::Targeted))
    } else {
        Ok(())
    };
    let critical = read.unsupported_critical.map_or(Ok(()), |id| {
        Err(Ok(Invalid::UnsupportedCritical(
            x509::dotted(id).unwrap_or_else(|| "an unreadable object identifier".to_owned()),
        )))
    });
    for result in [timely, targeted, critical] {
        if let Some(invalid) = fail(result) {
            return Outcome::Invalid(invalid);
        }
    }
    if let Some(requirement) = conditions.revocation {
        match revocation(read, requirement, authority_path.as_ref(), conditions) {
            Ok(()) => {}
            Err(Ok(invalid)) => return Outcome::Invalid(invalid),
            Err(Err(why)) => {
                undecided.get_or_insert(why);
            }
        }
    }
    undecided.map_or(Outcome::Valid, Outcome::Undecided)
}

/// The policy's revocation requirement over the certificate's chain: invalid where a check finds a
/// certificate revoked, undecided where one is not answered, the end-entity check's finding first.
fn revocation(
    read: &AttributeCertificate<'_>,
    requirement: &Requirement,
    authority_path: Option<&Path>,
    conditions: Conditions<'_>,
) -> Result<(), Result<Invalid, Undecided>> {
    let checks = revocation_checks::attribute_checks(
        requirement,
        read.issuer,
        read.serial,
        authority_path,
        conditions.material,
        conditions.at,
        conditions.timing,
    );
    let mut unanswered = None;
    for check in checks {
        match check.verdict() {
            Verdict::Met => {}
            Verdict::Broken => return Err(Ok(Invalid::Revoked(Box::new(check)))),
            Verdict::NotEvaluated => {
                unanswered.get_or_insert(check);
            }
        }
    }
    unanswered.map_or(Ok(()), |check| {
        Err(Err(Undecided::Revocation(
            Box::new(check),
            read.no_revocation,
        )))
    })
}

/// Condition 1: the holder is the signer, and the signer's path is verified.
fn holder_condition(
    holder: &Holder<'_>,
    cms: &SignedData<'_>,
    signer_path: Option<&Search>,
) -> Result<(), Result<Invalid, Undecided>> {
    let signer = crate::signature::signer_certificate(cms)
        .and_then(|value| x509::read(value).ok())
        .ok_or(Err(Undecided::NoSignerCertificate))?;
    match bound(holder, &signer) {
        Some(true) => {}
        Some(false) => return Err(Ok(Invalid::NotTheSigners)),
        None => return Err(Err(Undecided::Holder)),
    }
    match signer_path.map(Search::verdict) {
        Some(Verdict::Met) => Ok(()),
        Some(Verdict::Broken) => Err(Ok(Invalid::SignerPath)),
        _ => Err(Err(Undecided::SignerPath)),
    }
}

/// Whether `holder` names `signer` (section 4.2.2): `Some(true)` where every option it states
/// names the signer, `Some(false)` where none does, and `None` otherwise — options that disagree,
/// or one this program does not compare.
///
/// A `baseCertificateID` names the certificate of its issuer and serial number. An `entityName`
/// names the subject or one of the subject's alternative names; a `directoryName` is compared
/// as [`crate::trust`] compares a path's names, and another form only for identical encoding,
/// since each form has its own matching rule, so a difference in one is not decided.
fn bound(holder: &Holder<'_>, signer: &Certificate<'_>) -> Option<bool> {
    let mut answers: Vec<Option<bool>> = Vec::new();
    if let Some(base) = holder.base {
        answers.push(match base.issuer {
            Some(issuer) if !base.issuer_uid => Some(signer.is_named_by(issuer, base.serial)),
            _ => None,
        });
    }
    if !holder.entity.is_empty() {
        let alternatives = alternative_names(signer);
        let names = holder.entity.iter().map(|name| {
            if name.identifier == 0xA4 {
                let mut inner = name.children().ok()?;
                let rdns = inner.next_value().ok()??;
                Some(rdns.identifier == SEQUENCE && rdns.contents == signer.subject)
            } else if alternatives
                .iter()
                .any(|alternative| alternative.encoding() == name.encoding())
            {
                Some(true)
            } else {
                None
            }
        });
        let names: Vec<Option<bool>> = names.collect();
        answers.push(if names.contains(&Some(true)) {
            Some(true)
        } else if names.iter().all(|name| *name == Some(false)) {
            Some(false)
        } else {
            None
        });
    }
    if holder.digest {
        answers.push(None);
    }
    if answers.iter().all(|answer| *answer == Some(true)) {
        Some(true)
    } else if answers.iter().all(|answer| *answer == Some(false)) {
        Some(false)
    } else {
        None
    }
}

/// The signer's subject alternative names, each `GeneralName` whole.
fn alternative_names<'a>(signer: &Certificate<'a>) -> Vec<Value<'a>> {
    let Some(octets) = signer.extensions.subject_alt_name else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let Ok(mut outer) = Reader::new(octets) else {
        return out;
    };
    let Ok(Some(names)) = outer.next_value() else {
        return out;
    };
    let Ok(mut names) = names.children() else {
        return out;
    };
    while let Ok(Some(name)) = names.next_value() {
        if out.len() == attribute_certificate::MAX_ENTRIES {
            break;
        }
        out.push(name);
    }
    out
}

/// Condition 2's issuer: the first carried certificate of the issuer's name whose key verifies
/// the signature.
fn issuer<'a>(
    read: &AttributeCertificate<'_>,
    cms: &SignedData<'a>,
) -> Result<(Value<'a>, Certificate<'a>), Result<Invalid, Undecided>> {
    let mut named = false;
    let mut unverified = None;
    for value in cms.certificates.iter().take(trust::MAX_CANDIDATES) {
        let Ok(certificate) = x509::read(*value) else {
            continue;
        };
        if certificate.subject != read.issuer {
            continue;
        }
        named = true;
        match crate::certificate_signature::verify(
            read.info,
            read.signature_algorithm,
            read.signature_parameters,
            read.signature,
            certificate.public_key,
        ) {
            Ok(true) => return Ok((*value, certificate)),
            Ok(false) => {}
            Err(algorithm) => {
                unverified.get_or_insert(algorithm);
            }
        }
    }
    Err(match (named, unverified) {
        (false, _) => Err(Undecided::NoIssuerCertificate),
        (true, Some(algorithm)) => Err(Undecided::Algorithm(algorithm)),
        (true, None) => Ok(Invalid::Signature),
    })
}

/// Condition 3: section 4.5's profile of the issuer's certificate.
fn profile(issuer: &Certificate<'_>) -> Result<(), Result<Invalid, Undecided>> {
    if issuer
        .extensions
        .basic_constraints
        .is_some_and(|basic| basic.ca)
    {
        return Err(Ok(Invalid::IssuerIsCa));
    }
    if issuer
        .extensions
        .key_usage
        .is_some_and(|usage| !usage.digital_signature())
    {
        return Err(Ok(Invalid::IssuerKeyUsage));
    }
    Ok(())
}

/// Conditions 2 and 4: the issuer's path to the trust trees, handed back where it validated.
fn path(
    issuer: Value<'_>,
    cms: &SignedData<'_>,
    conditions: Conditions<'_>,
) -> Result<Option<Path>, Result<Invalid, Undecided>> {
    let trees = conditions.trees.ok_or(Err(Undecided::NoTrustTrees))?;
    let search =
        certification_path::search_certificate(trees, cms, issuer, conditions.at, conditions.lists);
    match search.verdict() {
        Verdict::Met => Ok(search.path),
        Verdict::Broken => Err(Ok(Invalid::Path(Box::new(search.found), search.unexamined))),
        Verdict::NotEvaluated => Err(Err(Undecided::Path(
            Box::new(search.found),
            search.unexamined,
        ))),
    }
}

#[cfg(test)]
pub(super) mod tests;

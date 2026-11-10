//! The revocation checks a signature policy's trust condition mandates, answered per kind of
//! certificate over the path built to the policy's own trust points.
//!
//! ETSI TR 102 272 clause 6.6.2 states a trust condition's revocation requirement as two checks,
//! one for end certificates (the signer's, a time-stamping authority's, an attribute
//! certificate) and one for CA certificates, each one of: a CRL check, an OCSP check, both, either,
//! no check, or another mechanism a policy extension defines. TS 119 172-2 clause 4.23.2 states the
//! same six with the same meaning, and TS 119 172-3 numbers them as TR 102 272 does. Which kind of
//! *material* answered is therefore part of the requirement, and [`crate::revocation::status`]
//! stops at the first answer of either kind; [`crate::revocation::status_by`] asks one kind alone,
//! and this module asks it of every certificate the requirement covers (ADR 2720).
//!
//! # What each outcome means
//!
//! - **Met**: no check is mandated; or the path holds no certificate of that kind below its trust
//!   point; or every certificate of the kind is answered by the mandated kind of material — a CRL
//!   that verifies under its issuer's key and is current at the instant, or an authorised OCSP
//!   response — and none of them was revoked at or before that instant.
//! - **Broken**: the mandated check was made and found a certificate of the kind revoked at or
//!   before the instant the path is validated at — RFC 5280 section 5.3.2's `invalidityDate`
//!   where the list states an earlier one — which RFC 5280 section 6.1.3 (a)(3) makes a path that
//!   does not validate.
//! - **Not evaluated**: no path validated, so which certificates are of which kind is not known;
//!   the requirement is `other`, whose mechanism only its extension defines; or the material the
//!   file carries does not answer the mandated check for a certificate. The last is ETSI EN 319
//!   102-1's indeterminate rather than a failure: a validator may fetch what the file lacks, this
//!   program answers revocation only from the file (ADR 1067), and absent evidence is not evidence
//!   of revocation.
//!
//! The trust point itself is not checked: it is where RFC 5280 section 6.1's processing starts, and
//! the policy, not a CA, vouches for it.
//!
//! # When the material was issued
//!
//! A policy can also say *which* material may answer, by when it was issued (ADR 2775):
//!
//! - TS 119 172-2 clause 4.23.3's `Freshness`, which TS 119 172-3 clause 4.23.3.1 adopts, is
//!   either the maximum accepted difference between the issuance of the revocation status data
//!   and the time of validation — ETSI EN 319 102-1 clause 5.2.5's maximum accepted freshness,
//!   whose step 1 takes it in place of the material's own `thisUpdate` to `nextUpdate` interval
//!   — or a time the data must be issued after the best-signature time, which EN 319 102-1 clause
//!   5.5.4's process sets to the signature timestamp's generation time where one covers the
//!   signature and otherwise leaves at the validation time.
//! - TR 102 272 clause 6.8's `cautionPeriod` is a period after the signing time before whose end
//!   the revocation status information that completes the signature's validation data is not to
//!   be collected or used. That information covers every certificate the validation answers
//!   revocation for, so the caution applies to the checks of both trust conditions, measured from
//!   the same best-signature time.
//!
//! Each is a bound on the issuance of the material that answers a check
//! ([`revocation::Issuance`]): material issued outside it does not answer, so a check it alone
//! would have answered is not evaluated rather than broken — EN 319 102-1's "try later", not a
//! failure.
//!
//! # Bounds
//!
//! The certificates asked about are those of a path [`super::certification_path::search`]
//! validated, at most [`crate::trust::MAX_STEPS`] long, and each is asked of the material the
//! document's readers already bounded, at most twice (once per kind) — so the work is a product of
//! two bounds already held.

use super::Verdict;
use super::certification_path::Path;
use super::duration::{self, Duration};
use crate::revocation::{self, Evidence, Issuance, Material, Revocation, Subject, Undetermined};
use crate::x509::{self, Instant};

/// TS 119 172-2 clause 4.23.3's `SigCertIssuedByCAKeepsExpiredRevokedCertsInfo`, as a sentence
/// names it.
pub const RETENTION: &str = "retention of expired revocation notices required of the issuer";

/// One of TR 102 272 clause 6.6.2's `EnuRevReq` values, TS 119 172-2 clause 4.23.2's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// `clrCheck`: checks against current CRLs.
    Crl,
    /// `ocspCheck`: the status checked by OCSP.
    Ocsp,
    /// `bothCheck`: both a CRL and an OCSP check.
    Both,
    /// `eitherCheck`: at least one of the two.
    Either,
    /// `noCheck`: no check is mandated.
    NoCheck,
    /// `other`: a mechanism a signature policy extension defines.
    Other,
}

impl Check {
    /// The enumeration's value, as TR 102 272 and TS 119 172-3 both number it.
    #[must_use]
    pub const fn of_number(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Crl,
            1 => Self::Ocsp,
            2 => Self::Both,
            3 => Self::Either,
            4 => Self::NoCheck,
            5 => Self::Other,
            _ => return None,
        })
    }

    /// TS 119 172-2 clause 4.23.2.2's spelling.
    #[must_use]
    pub fn of_name(name: &str) -> Option<Self> {
        Some(match name {
            "clrcheck" => Self::Crl,
            "ocspcheck" => Self::Ocsp,
            "bothcheck" => Self::Both,
            "eithercheck" => Self::Either,
            "nocheck" => Self::NoCheck,
            "other" => Self::Other,
            _ => return None,
        })
    }

    /// The words a sentence uses for the check.
    #[must_use]
    pub const fn words(self) -> &'static str {
        match self {
            Self::Crl => "a CRL check",
            Self::Ocsp => "an OCSP check",
            Self::Both => "both a CRL and an OCSP check",
            Self::Either => "a CRL or an OCSP check",
            Self::NoCheck => "no check",
            Self::Other => "a check by another mechanism a policy extension defines",
        }
    }
}

/// A trust condition's revocation requirement: TR 102 272 clause 6.6.2's `CertRevReq`, or TS 119
/// 172-2 clause 4.23.3's revocation trust around clause 4.23.2's requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// `endCertRevReq`: the check on the end certificate.
    pub end_entity: Check,
    /// `caCerts`: the check on each CA certificate below the trust point.
    pub authorities: Check,
    /// TS 119 172-2 clause 4.23.3's `Freshness`, where the revocation trust states one.
    pub freshness: Option<Freshness>,
    /// Members of clause 4.23.3's revocation trust this program does not evaluate, by name — the
    /// retention of expired revocation notices.
    pub unprocessed: Vec<&'static str>,
}

/// TS 119 172-2 clause 4.23.3's `Freshness` choice, TS 119 172-3 clause 4.23.3.2's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// `MaxDifferenceRevocationAndValidation`: the material is issued within this of the
    /// validation time.
    MaxDifference(Duration),
    /// `TimeAfterSignature`: the material is issued at least this long after the best-signature
    /// time.
    AfterSignature(Duration),
}

/// What places a requirement's issuance bounds on the time line, beyond the instant validated at.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Timing {
    /// The generation time of a signature timestamp that covers the signature: EN 319 102-1
    /// clause 5.5.4's best-signature time where one establishes it; the instant validated at
    /// where none does.
    pub stamped: Option<Instant>,
    /// TR 102 272 clause 6.8's `cautionPeriod`, in seconds, where the policy states one.
    pub caution_seconds: Option<i64>,
}

impl Timing {
    /// The issuance bounds `freshness` and the caution period place on material asked about at
    /// `at`, or `None` where one of them falls outside the instants this program computes.
    fn issuance(self, freshness: Option<Freshness>, at: Instant) -> Option<Issuance> {
        let best = self.stamped.unwrap_or(at);
        let mut issuance = Issuance::default();
        match freshness {
            Some(Freshness::MaxDifference(within)) => {
                issuance.fresh_after = Some(within.subtracted_from(at)?);
            }
            Some(Freshness::AfterSignature(after)) => {
                issuance.issued_from = Some(after.added_to(best)?);
            }
            None => {}
        }
        if let Some(caution) = self.caution_seconds {
            let ends = Duration::of_seconds(caution).added_to(best)?;
            issuance.issued_from = Some(match issuance.issued_from {
                Some(from) if from.unix_seconds() >= ends.unix_seconds() => from,
                _ => ends,
            });
        }
        Some(issuance)
    }
}

impl Requirement {
    /// No check of either kind, and nothing more.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            end_entity: Check::NoCheck,
            authorities: Check::NoCheck,
            freshness: None,
            unprocessed: Vec::new(),
        }
    }

    /// Whether the requirement mandates anything at all.
    #[must_use]
    pub fn mandates(&self) -> bool {
        self.end_entity != Check::NoCheck
            || self.authorities != Check::NoCheck
            || self.freshness.is_some()
            || !self.unprocessed.is_empty()
    }
}

/// Which certificates of a path a check covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The end certificate, at position 0.
    EndEntity,
    /// Each CA certificate below the trust point.
    Authority,
}

/// What one kind's check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checked {
    /// The requirement mandates no check of this kind.
    NotMandated,
    /// The requirement is `other`.
    OtherMechanism,
    /// No path validated to a trust point, so the certificates of the kind are not known.
    NoPath,
    /// The policy's bound on when the material was issued falls outside the instants this
    /// program computes, so which material may answer is not known.
    TimingUnplaced,
    /// The path holds no certificate of the kind below its trust point.
    NoCertificate,
    /// Every certificate of the kind was answered by the mandated material, none revoked.
    Answered {
        /// How many certificates were answered.
        certificates: usize,
    },
    /// The check found a certificate revoked at or before the instant.
    Revoked {
        /// Its position on the path, the end certificate being 0.
        position: usize,
        /// What said so.
        from: Evidence,
        /// When the revocation took effect: the earlier of the revocation date and a stated
        /// invalidity date.
        effective: Instant,
    },
    /// The material the file carries does not answer the mandated check for a certificate.
    Unanswered {
        /// Its position on the path.
        position: usize,
        /// The kind of material that did not answer.
        missing: Evidence,
        /// Why not.
        why: Undetermined,
    },
}

/// One kind's check: what the requirement mandates and what the material answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindCheck {
    /// Which certificates.
    pub kind: Kind,
    /// The check mandated.
    pub check: Check,
    /// The bounds the material that answered had to meet.
    pub issuance: Issuance,
    /// What it found.
    pub checked: Checked,
}

impl KindCheck {
    /// The check's verdict, by the module comment's rules.
    #[must_use]
    pub const fn verdict(&self) -> Verdict {
        match self.checked {
            Checked::NotMandated | Checked::NoCertificate | Checked::Answered { .. } => {
                Verdict::Met
            }
            Checked::Revoked { .. } => Verdict::Broken,
            Checked::OtherMechanism
            | Checked::NoPath
            | Checked::TimingUnplaced
            | Checked::Unanswered { .. } => Verdict::NotEvaluated,
        }
    }
}

/// Each kind's check of `requirement` over `path`, at `at`, from `material` issued within the
/// bounds `requirement`'s freshness and `timing` place: the end certificate's first and the CA
/// certificates' second.
#[must_use]
pub fn checks(
    requirement: &Requirement,
    path: Option<&Path>,
    material: &Material<'_>,
    at: Option<Instant>,
    timing: Timing,
) -> [KindCheck; 2] {
    let issuance = at.map(|at| timing.issuance(requirement.freshness, at));
    [
        (Kind::EndEntity, requirement.end_entity),
        (Kind::Authority, requirement.authorities),
    ]
    .map(|(kind, check)| KindCheck {
        kind,
        check,
        issuance: issuance.flatten().unwrap_or_default(),
        checked: kind_check(kind, check, path, material, at.zip(issuance)),
    })
}

fn kind_check(
    kind: Kind,
    check: Check,
    path: Option<&Path>,
    material: &Material<'_>,
    at: Option<(Instant, Option<Issuance>)>,
) -> Checked {
    match check {
        Check::NoCheck => return Checked::NotMandated,
        Check::Other => return Checked::OtherMechanism,
        _ => {}
    }
    let (Some(path), Some((at, issuance))) = (path, at) else {
        return Checked::NoPath;
    };
    let Some(issuance) = issuance else {
        return Checked::TimingUnplaced;
    };
    let certificates = path.certificates();
    // Below the trust point, which is the last: the end certificate is position 0, and the CA
    // certificates are every position between it and the trust point.
    let below = certificates.len().saturating_sub(1);
    let positions = match kind {
        Kind::EndEntity => 0..below.min(1),
        Kind::Authority => 1.min(below)..below,
    };
    if positions.is_empty() {
        return Checked::NoCertificate;
    }
    let mut answered = 0_usize;
    for position in positions {
        let (Some(held), Some(issuer)) = (
            certificates.get(position),
            certificates.get(position.saturating_add(1)),
        ) else {
            return Checked::NoPath;
        };
        // Both parsed before: a path is handed out only once every certificate on it validated.
        let (Ok(certificate), Ok(issuer)) =
            (x509::parse(&held.encoding), x509::parse(&issuer.encoding))
        else {
            return Checked::NoPath;
        };
        let subject = Subject {
            certificate: &certificate,
            issuer_name: issuer.subject_encoding,
            issuer_key_bits: issuer.public_key_bits,
            issuer_key: issuer.public_key,
            issuer_key_usage: issuer.extensions.key_usage,
            position,
        };
        match one(&subject, check, material, at, issuance) {
            None => answered = answered.saturating_add(1),
            Some(found) => return found,
        }
    }
    Checked::Answered {
        certificates: answered,
    }
}

/// One certificate under one check: `None` where the check is answered and finds it not revoked at
/// `at`, else what it found.
fn one(
    subject: &Subject<'_, '_>,
    check: Check,
    material: &Material<'_>,
    at: Instant,
    issuance: Issuance,
) -> Option<Checked> {
    let ask = |evidence| answer(subject, material, at, evidence, issuance);
    match check {
        Check::Crl => ask(Evidence::CertificateRevocationList),
        Check::Ocsp => ask(Evidence::OcspResponse),
        Check::Both => {
            let ocsp = ask(Evidence::OcspResponse);
            let crl = ask(Evidence::CertificateRevocationList);
            // A revocation either kind reports decides, whichever kind is missing.
            match (ocsp, crl) {
                (Some(revoked @ Checked::Revoked { .. }), _)
                | (_, Some(revoked @ Checked::Revoked { .. })) => Some(revoked),
                (Some(missing), _) | (None, Some(missing)) => Some(missing),
                (None, None) => None,
            }
        }
        Check::Either => {
            let ocsp = ask(Evidence::OcspResponse);
            match ocsp {
                None | Some(Checked::Revoked { .. }) => ocsp,
                Some(missing) => match ask(Evidence::CertificateRevocationList) {
                    Some(Checked::Unanswered { .. }) => Some(missing),
                    crl => crl,
                },
            }
        }
        Check::NoCheck | Check::Other => None,
    }
}

/// One kind of material's answer for one certificate, read against the instant.
fn answer(
    subject: &Subject<'_, '_>,
    material: &Material<'_>,
    at: Instant,
    evidence: Evidence,
    issuance: Issuance,
) -> Option<Checked> {
    match revocation::status_issued(subject, material, at, evidence, issuance) {
        Revocation::Good { .. } => None,
        Revocation::Revoked {
            at: revoked,
            invalid_from,
            from,
            position,
            ..
        } => {
            let effective = invalid_from.map_or(revoked, |invalid| {
                if invalid.unix_seconds() < revoked.unix_seconds() {
                    invalid
                } else {
                    revoked
                }
            });
            // A revocation that took effect after the instant leaves the certificate good at it:
            // the check was made, and it answered for that instant.
            (effective.unix_seconds() <= at.unix_seconds()).then_some(Checked::Revoked {
                position,
                from,
                effective,
            })
        }
        Revocation::Unknown { why, position } => Some(Checked::Unanswered {
            position,
            missing: evidence,
            why,
        }),
        // `status_by` asks and so never answers `NotChecked`; were it to, nothing was answered.
        Revocation::NotChecked => Some(Checked::Unanswered {
            position: subject.position,
            missing: evidence,
            why: Undetermined::NotCovered,
        }),
    }
}

/// The sentence for one kind's check of `condition`'s requirement, with the verdict in its place.
pub(super) fn sentence(
    f: &mut std::fmt::Formatter<'_>,
    condition: &str,
    check: &KindCheck,
    verdict: &str,
) -> std::fmt::Result {
    let whom = match check.kind {
        Kind::EndEntity => "the end certificate",
        Kind::Authority => "each CA certificate below the trust point",
    };
    write!(
        f,
        "the revocation check the policy's {condition} mandates on {whom} ({}) is {verdict}: ",
        check.check.words()
    )?;
    match &check.checked {
        Checked::NotMandated => f.write_str("it mandates no check"),
        Checked::OtherMechanism => f.write_str(
            "the mechanism is one a signature policy extension defines, and this program reads none",
        ),
        Checked::NoPath => f.write_str(
            "no path validated to the policy's own trust points, so the certificates it covers are not known",
        ),
        Checked::TimingUnplaced => f.write_str(
            "the time the policy requires the material to be issued in falls outside the instants this program computes",
        ),
        Checked::NoCertificate => {
            f.write_str("the path holds no such certificate below its trust point")
        }
        Checked::Answered { certificates } => write!(
            f,
            "the material this file carries answers it for {certificates} certificate(s), and none was revoked at the instant validated at"
        ),
        Checked::Revoked { position, from, .. } => write!(
            f,
            "{} found the certificate at position {position} of the path revoked with effect before the instant validated at (RFC 5280 section 6.1.3 (a)(3))",
            capitalised(from.name())
        ),
        Checked::Unanswered {
            position,
            missing,
            why,
        } => write!(
            f,
            "for the certificate at position {position} of the path the file carries no answer from {}: {why}; this program answers revocation only from the material the file carries",
            missing.name()
        ),
    }?;
    issuance_clause(f, check.issuance)
}

/// What a check's material had to meet, where the policy bounded its issuance.
fn issuance_clause(f: &mut std::fmt::Formatter<'_>, issuance: Issuance) -> std::fmt::Result {
    if let Some(after) = issuance.fresh_after {
        write!(
            f,
            "; the policy's freshness admits material issued after {}",
            duration::utc(after)
        )?;
    }
    if let Some(from) = issuance.issued_from {
        write!(
            f,
            "; the policy admits material issued at or after {}",
            duration::utc(from)
        )?;
    }
    Ok(())
}

fn capitalised(words: &str) -> String {
    let mut characters = words.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(characters).collect()
    })
}

#[cfg(test)]
pub(super) mod tests;

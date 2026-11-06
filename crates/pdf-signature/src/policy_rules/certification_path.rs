//! A signature policy's constraints on the signer's certification path, evaluated over a path
//! built to the policy's own trust points.
//!
//! ETSI TR 102 272 clause 6.7 states the signing-certificate trust condition as a set of trust
//! trees (clause 6.6.1): self-signed certificates where certificate path processing starts, each
//! with the initial conditions of RFC 5280 section 6's path validation. TS 119 172-2 clauses 4.22.2,
//! 4.22.5 and 4.24.4, which TS 119 172-3 adopts for its ASN.1 syntax, state the same trust points
//! with an optional instant until which each is reliable, or as a reference to a trusted list.
//! These are the *policy's* trust points, and they are not the anchors a host supplies for
//! §12.8.1's third question (ADR 1076): the host decides whom the reader believes, the policy decides
//! which hierarchies the agreement admits, and §12.8.3.4.4 makes the second a constraint a
//! validator enforces. So the path is built twice when both are present, once to each, and a
//! policy's verdict can withhold the word *valid* and never grant it (ADR 2714).
//!
//! # What is evaluated, and what is said instead
//!
//! The path is built by [`crate::trust::validate_path`] from the signer's certificate over the
//! certificates the `SignedData` carries, one trust point at a time, at the instant
//! [`SignerFacts`] names — RFC 5280 section 6.1.1's input (b), which this crate never reads off a
//! clock. Each trust point's `pathLenConstraint` is the most CA certificates the path may hold below
//! it (TR 102 272 clause 6.6.1, TS 119 172-2 clause 4.22.5.1 item 1). ETSI EN 319 102-1 clause
//! 5.2.6.4 decides the three outcomes that are not a path validating: no chain at all is
//! indeterminate (step 2 a), a trust point past the instant it was reliable until is indeterminate
//! (steps 1 a and 3), and a chain that RFC 5280 section 6.1 refuses is a failure of the path. A
//! condition is met only where a path validated to a trust point within its length, and it is
//! *not evaluated* — never met — where the trust point also states a mandated path, or where an
//! anchor the policy names by trusted-list reference is one this program cannot read, because a
//! trusted list is fetched from a network it does not use.
//!
//! A trust point's acceptable policy set, name constraints and policy constraints are RFC 5280
//! section 6.1.1's inputs (c) and (e) to (i) for the path built to it (RFC 3125 section 6.6.1, TR
//! 102 272 clause 6.6.1, TS 119 172-2 clauses 4.22.3 to 4.22.5), so each is [`TrustPoint::inputs`]
//! and [`crate::trust::validate_constrained`] evaluates it with the policy tree and the name
//! constraints of every certificate on the path (ADR 2724). The host's own anchors are validated
//! under section 6.1.1's defaults, which is why a path can be the host's and fail a policy's trust
//! point, and each sentence says which of the two it is about.
//!
//! A time-stamp trust condition asks the same question of the authority that issued the
//! signature's time-stamp token, so [`search_authority`] is the same search from that authority's
//! certificate over the certificates the token carries (ADR 2720).
//!
//! # Bounds
//!
//! The policy document is hostile until it is bound, and so are its trust points: at most
//! [`MAX_TRUST_POINTS`] are searched for one condition, and each search is
//! [`crate::trust::MAX_STEPS`] certificates at most. A trust point past the bound is counted as
//! unexamined, which keeps a condition it might have met from being called broken. At most
//! [`MAX_AUTHORITY_SEARCHES`] time-stamp conditions are searched for one signature.

use std::borrow::Cow;

use super::{SignerFacts, Stated, TIMESTAMP_TRUST, TimeSource, Verdict, key_bits};
use crate::cms::SignedData;
use crate::ess::{self, CertificateHash, Version};
use crate::revocation::Material;
use crate::timestamp::Unestablished;
use crate::trust::{self, PathInputs, PathRefusal, Purpose, Trust, TrustAnchors, names};
use crate::x509::{self, Certificate, Instant};

/// The most trust points searched for one condition.
///
/// A policy names the roots of the hierarchies it admits, which in practice is one or a handful;
/// sixteen is room for a national policy naming every qualified issuer's root, and the product
/// with [`crate::trust::MAX_STEPS`] is what keeps a document stuffed with trust points finite.
pub const MAX_TRUST_POINTS: usize = 16;

/// The most time-stamp trust conditions searched for one signature.
///
/// TS 119 172-2 clause 4.25 lets a policy state one condition per set of time evidences, and every
/// one asks about the same token; four is room for a policy distinguishing its evidences, and the
/// bound keeps a document stating thousands of conditions from asking for thousands of searches.
pub const MAX_AUTHORITY_SEARCHES: usize = 4;

/// The searches made for one signature's time-stamp trust conditions, bounded at
/// [`MAX_AUTHORITY_SEARCHES`].
#[derive(Debug, Default)]
pub struct Authorities {
    searched: std::cell::Cell<usize>,
}

impl Authorities {
    /// [`search_authority`], or [`Found::PastSearchBound`] once the bound is reached.
    #[must_use]
    pub fn search(
        &self,
        trees: &TrustTrees<'_>,
        cms: &SignedData<'_>,
        facts: &SignerFacts<'_>,
    ) -> Search {
        let searched = self.searched.get();
        if searched >= MAX_AUTHORITY_SEARCHES {
            return Search {
                found: Found::PastSearchBound,
                unexamined: trees.points.len().saturating_add(trees.trusted_lists),
                path: None,
            };
        }
        self.searched.set(searched.saturating_add(1));
        search_authority(trees, cms, facts)
    }
}

/// One of TR 102 272 clause 6.6.1's `CertificateTrustPoint`s, or TS 119 172-2 clause 4.22.5's,
/// with one certificate trust anchor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustPoint<'a> {
    /// The trust point's certificate, DER.
    pub certificate: Cow<'a, [u8]>,
    /// `pathLenConstraint`: the most CA certificates the path may hold below the trust point.
    pub path_length: Option<i64>,
    /// TS 119 172-2 clause 4.22.2.1 a)'s instant until which the trust anchor is reliable.
    pub reliable_until: Option<Stated>,
    /// `acceptablePolicySet`, `nameConstraints` and `policyConstraints`, as RFC 5280 section
    /// 6.1.1's inputs for a path to this trust point.
    pub inputs: PathInputs,
    /// The members the trust point states that this program does not process, by name.
    pub unprocessed: Vec<&'static str>,
}

/// A `CertificateTrustTrees`: the trust points one condition admits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrustTrees<'a> {
    /// The trust points whose anchor is a certificate.
    pub points: Vec<TrustPoint<'a>>,
    /// How many trust anchors are given as a reference to a trusted list (TS 119 172-2 clause
    /// 4.22.2.1 b)), which this program does not fetch.
    pub trusted_lists: usize,
}

/// What the search for a path to a policy's trust points found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// No instant was given to validate the path at: no host clock and no signature timestamp.
    NoInstant,
    /// The `SignedData` does not carry the certificate its signer names.
    NoSignerCertificate,
    /// No trust point's certificate could be read, so there was nothing to build a path to.
    NoTrustPoint,
    /// A path validated to a trust point within its length.
    Reached {
        /// Certificates on the path, the signer's included and the trust point excluded.
        length: usize,
        /// Members of that trust point this program does not process.
        unprocessed: Vec<&'static str>,
        /// The members of that trust point section 6.1 evaluated as its inputs, by name.
        evaluated: Vec<&'static str>,
        /// The policies the path is valid for under the trust point's acceptable policy set, in its
        /// domain — RFC 5280 section 6.1.6's output; `None` where the trust point states no set.
        policies: Option<Vec<Vec<u8>>>,
    },
    /// A path validated, but it holds more CA certificates below the trust point than its
    /// `pathLenConstraint` admits.
    Longer {
        /// The CA certificates below the trust point.
        intermediates: usize,
        /// The constraint.
        limit: i64,
    },
    /// Every chain built to a trust point was refused by RFC 5280 section 6.1, first refusal, with
    /// the members of that trust point section 6.1 evaluated as its inputs.
    Refused(PathRefusal, Vec<&'static str>),
    /// The only trust points a chain reached are past the instant they were reliable until.
    PastReliable {
        /// The instant as the policy spells it.
        until: String,
    },
    /// No chain from the signer's certificate reaches any trust point.
    NoChain,
    /// The signature states no `signature-time-stamp` attribute, so there is no time-stamping
    /// authority whose path a time-stamp trust condition could judge.
    NoTimestamp,
    /// The signature's time-stamp token did not get past steps 1 to 3 of
    /// [`crate::timestamp::signature_timestamp_established`]: it would not read, its `TSTInfo` is
    /// not bound, or its signature does not verify — so no authority is authenticated by it.
    TimestampNotVerified(Unestablished),
    /// The token's imprint is not the digest of this signature (ETSI EN 319 122-1 clause 5.3),
    /// so its authority is not this signature's.
    TimestampAboutAnotherSignature,
    /// The condition was not searched: [`MAX_AUTHORITY_SEARCHES`] conditions already were.
    PastSearchBound,
}

/// One certificate of a validated path, as far as a policy's constraints read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathCertificate {
    /// The certificate's whole DER encoding, which RFC 5035 section 5.4.1.1's hash covers.
    pub encoding: Vec<u8>,
    /// The algorithm its issuer signed it under.
    pub signature_algorithm: Vec<u8>,
    /// Its subject key's length in bits, where this reader measures the family.
    pub key_bits: Option<usize>,
}

/// One certificate on a path and the key that signed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issued {
    /// The algorithm the issuer signed it under.
    pub algorithm: Vec<u8>,
    /// The issuer's key length in bits, where measured.
    pub issuer_key_bits: Option<usize>,
}

/// A path that validated to one of a policy's trust points, signer first and trust point last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    certificates: Vec<PathCertificate>,
}

impl Path {
    /// The certificates, signer first and trust point last.
    #[must_use]
    pub fn certificates(&self) -> &[PathCertificate] {
        &self.certificates
    }

    /// The signer's certificate and the key that signed it — what TR 102 272 clause 6.10's
    /// `eeCertAlgorithmConstraints` and TS 119 172-2 clause 4.29's signing-certificate usage
    /// constrain.
    #[must_use]
    pub fn end_entity(&self) -> Option<Issued> {
        self.issued(0)
    }

    /// Each CA certificate below the trust point and the key that signed it — what clause 6.10's
    /// `caCertAlgorithmConstraints` and clause 4.29's path-certificate usage constrain. The trust
    /// point itself is the policy's, not a certificate the path validation checked a signature on.
    #[must_use]
    pub fn authorities(&self) -> Vec<Issued> {
        (1..self.certificates.len().saturating_sub(1))
            .filter_map(|index| self.issued(index))
            .collect()
    }

    fn issued(&self, index: usize) -> Option<Issued> {
        let certificate = self.certificates.get(index)?;
        let issuer = self.certificates.get(index.checked_add(1)?)?;
        Some(Issued {
            algorithm: certificate.signature_algorithm.clone(),
            issuer_key_bits: issuer.key_bits,
        })
    }

    /// TR 102 272 clause 6.5.1's `fullPath` certificate reference: whether one signing-certificate
    /// attribute carries a hash of every certificate of the path below the trust point. RFC 5035
    /// section 5.4 evaluates the two attributes independently, so either one may meet it. `None`
    /// where neither attribute that is present can be read.
    #[must_use]
    pub fn referenced_by(&self, cms: &SignedData<'_>) -> Option<bool> {
        let below = self
            .certificates
            .get(..self.certificates.len().saturating_sub(1))
            .unwrap_or(&[]);
        let covers = |hashes: &[CertificateHash<'_>]| {
            below.iter().all(|certificate| {
                hashes.iter().any(|reference| {
                    reference.digest.compute(&[&certificate.encoding]) == reference.hash
                })
            })
        };
        let read = [
            (cms.signing_certificate, Version::One),
            (cms.signing_certificate_v2, Version::Two),
        ]
        .into_iter()
        .filter_map(|(value, version)| value.map(|value| ess::certificate_hashes(value, version)))
        .collect::<Vec<_>>();
        if read.is_empty() {
            return Some(false);
        }
        if read.iter().flatten().any(|hashes| covers(hashes)) {
            return Some(true);
        }
        read.iter().all(Result::is_ok).then_some(false)
    }
}

/// The outcome of one condition's search: what was found, how many anchors could not be examined,
/// how many trust points were stated, and the path where one validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Search {
    /// What was found.
    pub found: Found,
    /// Trust anchors not searched: trusted-list references, unreadable certificates and trust
    /// points past [`MAX_TRUST_POINTS`]. Any of them might have met the condition, so a condition
    /// with one is never called broken.
    pub unexamined: usize,
    /// The path, where one validated to a trust point.
    pub path: Option<Path>,
}

impl Search {
    /// The condition's verdict.
    #[must_use]
    pub fn verdict(&self) -> Verdict {
        verdict(&self.found, self.unexamined)
    }
}

/// [`Search::verdict`], from its parts, as a finding is judged.
pub(super) fn verdict(found: &Found, unexamined: usize) -> Verdict {
    match found {
        Found::Reached { unprocessed, .. } if unprocessed.is_empty() => Verdict::Met,
        Found::Longer { .. } if unexamined == 0 => Verdict::Broken,
        Found::Refused(refusal, _) if unexamined == 0 && !is_limit(refusal) => Verdict::Broken,
        _ => Verdict::NotEvaluated,
    }
}

/// Whether a refusal is this program's limit rather than a defect of the path: an algorithm it
/// does not verify, an extension it does not process, one of its bounds, a name it cannot compare,
/// or its own search and walk disagreeing.
const fn is_limit(refusal: &PathRefusal) -> bool {
    matches!(
        refusal,
        PathRefusal::UnrecognisedCriticalExtension(_)
            | PathRefusal::ExtensionsUnread
            | PathRefusal::AlgorithmNotVerifiable(_)
            | PathRefusal::IssuerNameDoesNotChain
            | PathRefusal::PolicyTreeTooLarge
            | PathRefusal::ExtensionPastBound(_)
            | PathRefusal::SubtreeDistanceStated
            | PathRefusal::NameFormNotProcessed(_)
            | PathRefusal::NameComparisonUndecided(_)
    )
}

/// The members of a trust point that are section 6.1.1's inputs, by name, where stated.
fn evaluated(inputs: &PathInputs) -> Vec<&'static str> {
    let mut named = Vec::new();
    if inputs.policy_set.is_some() {
        named.push("acceptable policy set");
    }
    if inputs.name_constraints.is_some() {
        named.push("name constraints");
    }
    if inputs.require_explicit_policy.is_some() || inputs.inhibit_policy_mapping.is_some() {
        named.push("policy constraints");
    }
    named
}

/// The instant a policy's path is validated at: the one a host gave, else the instant a covering
/// signature timestamp states, which is the instant the signing period is judged at too.
pub(super) fn instant(facts: &SignerFacts<'_>) -> Option<Instant> {
    facts.path_at.or_else(|| {
        facts
            .signing_time
            .as_ref()
            .filter(|time| time.source == TimeSource::SignatureTimestamp)
            .map(|time| time.instant)
    })
}

/// Searches for a path from the signer's certificate to one of `trees`' trust points.
#[must_use]
pub fn search(trees: &TrustTrees<'_>, cms: &SignedData<'_>, facts: &SignerFacts<'_>) -> Search {
    search_from(trees, cms, instant(facts), Purpose::Unstated)
}

/// Searches for a path from the certificate of the authority that issued the signature's
/// time-stamp token to one of `trees`' trust points — TR 102 272 clause 6.8's
/// `ttsCertificateTrustTrees`, TS 119 172-2 clause 4.25.1 item 3 b)'s (ADR 2720).
///
/// The token is the `signature-time-stamp` attribute's, which clause 6.8 says the condition applies
/// to, and only where its imprint is this signature's digest; it must get past steps 1 to 3 of
/// [`crate::timestamp::signature_timestamp_established`] before its signer is anybody's authority,
/// and the path is asked with RFC 3161 section 2.3's purpose, `id-kp-timeStamping`, at the instant
/// the signer's own path is.
#[must_use]
pub fn search_authority(
    trees: &TrustTrees<'_>,
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
) -> Search {
    let unexamined = unsearched(trees);
    let outcome = |found| Search {
        found,
        unexamined,
        path: None,
    };
    let (token, info) = match crate::timestamp::signature_timestamp_token(cms) {
        None => return outcome(Found::NoTimestamp),
        Some(Err(refused)) => return outcome(Found::TimestampNotVerified(refused)),
        Some(Ok(read)) => read,
    };
    if info.imprint_digest.compute(&[cms.signature]) != info.imprint {
        return outcome(Found::TimestampAboutAnotherSignature);
    }
    search_from(trees, &token, instant(facts), Purpose::TimeStamping)
}

/// Trust anchors a search will not reach: trusted-list references and points past the bound.
fn unsearched(trees: &TrustTrees<'_>) -> usize {
    trees
        .trusted_lists
        .saturating_add(trees.points.len().saturating_sub(MAX_TRUST_POINTS))
}

/// The search, from the certificate `cms`'s signer names over the certificates `cms` carries.
fn search_from(
    trees: &TrustTrees<'_>,
    cms: &SignedData<'_>,
    at: Option<Instant>,
    purpose: Purpose,
) -> Search {
    let mut unexamined = unsearched(trees);
    let outcome = |found, unexamined| Search {
        found,
        unexamined,
        path: None,
    };
    let Some(at) = at else {
        return outcome(Found::NoInstant, unexamined);
    };
    let Some(signer) = crate::signature::signer_certificate(cms) else {
        return outcome(Found::NoSignerCertificate, unexamined);
    };
    let Ok(target) = x509::read(signer) else {
        return outcome(Found::NoSignerCertificate, unexamined);
    };
    // The certificates the signature carries, and nothing else: a path the policy's full-path
    // requirement reads is the one the signature itself holds.
    let carried: Vec<_> = cms
        .certificates
        .iter()
        .take(trust::MAX_CANDIDATES)
        .filter_map(|value| x509::read(*value).ok().map(|read| (*value, read)))
        .collect();
    let others: Vec<Certificate<'_>> = carried.iter().map(|(_, read)| *read).collect();
    // The policy's revocation requirement is its own finding; the path is asked without material.
    let material = Material::none();
    let mut readable = 0_usize;
    let mut longer = None;
    let mut refused = None;
    let mut past = None;
    for point in trees.points.iter().take(MAX_TRUST_POINTS) {
        let Ok(anchor) = x509::parse(&point.certificate) else {
            unexamined = unexamined.saturating_add(1);
            continue;
        };
        readable = readable.saturating_add(1);
        let held = [anchor];
        let anchors = TrustAnchors::of(&held);
        let validation = trust::validate_constrained(
            &target,
            &others,
            &anchors,
            &material,
            at,
            purpose,
            &point.inputs,
        );
        let validated = validation.path;
        match validation.trust {
            Trust::Anchored { length, .. } => {
                // EN 319 102-1 clause 5.2.6.4 steps 1 a) and 3): a trust anchor past its sunset
                // date makes the chain indeterminate rather than validated.
                if let Some(until) = &point.reliable_until
                    && at >= until.instant
                {
                    past.get_or_insert_with(|| until.spelled.clone());
                    continue;
                }
                let path = owned_path(&validated, &carried, &target, signer, &point.certificate);
                let intermediates = length.saturating_sub(1);
                if let Some(limit) = point.path_length
                    && i64::try_from(intermediates).is_ok_and(|held| held > limit)
                {
                    longer.get_or_insert((
                        Found::Longer {
                            intermediates,
                            limit,
                        },
                        path,
                    ));
                    continue;
                }
                return Search {
                    found: reached(point, length, validation.policies),
                    unexamined,
                    path: Some(path),
                };
            }
            Trust::Refused { refusal, .. } => {
                refused.get_or_insert((refusal, evaluated(&point.inputs)));
            }
            Trust::NoPathToAnyAnchor { .. } | Trust::NoAnchorSupplied => {}
        }
    }
    if let Some((found, path)) = longer {
        return Search {
            found,
            unexamined,
            path: Some(path),
        };
    }
    let found = match (refused, past) {
        (Some((refusal, named)), _) if !is_limit(&refusal) => Found::Refused(refusal, named),
        (_, Some(until)) => Found::PastReliable { until },
        (Some((refusal, named)), None) => Found::Refused(refusal, named),
        (None, None) if readable == 0 => Found::NoTrustPoint,
        (None, None) => Found::NoChain,
    };
    outcome(found, unexamined)
}

/// [`Found::Reached`] for a path validated to `point`, with the members section 6.1 evaluated and,
/// where the trust point states an acceptable policy set, the policies the path is valid for.
fn reached(point: &TrustPoint<'_>, length: usize, policies: trust::Policies) -> Found {
    Found::Reached {
        length,
        unprocessed: point.unprocessed.clone(),
        evaluated: evaluated(&point.inputs),
        policies: point
            .inputs
            .policy_set
            .is_some()
            .then_some(policies.user_constrained),
    }
}

/// The validated path, owned, with the trust point's certificate appended.
fn owned_path(
    validated: &[&Certificate<'_>],
    carried: &[(crate::der::Value<'_>, Certificate<'_>)],
    target: &Certificate<'_>,
    signer: crate::der::Value<'_>,
    trust_point: &[u8],
) -> Path {
    let mut certificates: Vec<PathCertificate> = validated
        .iter()
        .map(|certificate| {
            let encoding = if certificate.tbs == target.tbs {
                signer.encoding()
            } else {
                carried
                    .iter()
                    .find(|(_, read)| read.tbs == certificate.tbs)
                    .map_or(&[][..], |(value, _)| value.encoding())
            };
            PathCertificate {
                encoding: encoding.to_vec(),
                signature_algorithm: certificate.signature_algorithm.to_vec(),
                key_bits: key_bits(&certificate.public_key),
            }
        })
        .collect();
    if let Ok(point) = x509::parse(trust_point) {
        certificates.push(PathCertificate {
            encoding: trust_point.to_vec(),
            signature_algorithm: point.signature_algorithm.to_vec(),
            key_bits: key_bits(&point.public_key),
        });
    }
    Path { certificates }
}

/// A time-stamp trust condition's name constraints, judged on the authority's certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityName {
    /// Every name of the authority's certificate lies within the constraints.
    Within,
    /// A name lies outside them — RFC 5280 section 6.1.3 (b) or (c)'s test, applied to the one
    /// certificate the condition constrains.
    Outside(PathRefusal),
    /// The test could not be decided by this program.
    Undecided(PathRefusal),
    /// No path to the condition's trust points was found, so no authority's certificate is the one
    /// the condition constrains.
    NoAuthorityPath,
}

impl AuthorityName {
    /// The finding's verdict.
    #[must_use]
    pub const fn verdict(&self) -> Verdict {
        match self {
            Self::Within => Verdict::Met,
            Self::Outside(_) => Verdict::Broken,
            Self::Undecided(_) | Self::NoAuthorityPath => Verdict::NotEvaluated,
        }
    }
}

/// RFC 3125 section 6.8's `ttsNameConstraints` — "naming constraints on the trusted time-stamping
/// authority" — over the certificate of the authority whose path `search` found: its subject and
/// subject alternative names against the constraints, as RFC 5280 section 6.1.3 (b) and (c) test a
/// certificate against the subtrees above it (ADR 2724).
#[must_use]
pub fn authority_name(
    search: Option<&Search>,
    constraints: &names::NameConstraints,
) -> AuthorityName {
    use names::{NameRefusal, NameState};
    let Some(certificate) = search
        .and_then(|search| search.path.as_ref())
        .and_then(|path| path.certificates().first())
    else {
        return AuthorityName::NoAuthorityPath;
    };
    let Ok(read) = x509::parse(&certificate.encoding) else {
        return AuthorityName::NoAuthorityPath;
    };
    let mut state = NameState::default();
    if state.admit(constraints.clone()).is_err() {
        return AuthorityName::Undecided(PathRefusal::ExtensionPastBound("ttsNameConstraints"));
    }
    let Ok(held) = names::names_of(read.subject_encoding, read.extensions.subject_alt_name) else {
        return AuthorityName::Outside(PathRefusal::ExtensionUnreadable("subjectAltName"));
    };
    match state.check(&held) {
        Ok(()) => AuthorityName::Within,
        Err(NameRefusal::NotPermitted(form)) => {
            AuthorityName::Outside(PathRefusal::NameNotPermitted(names::form_word(form)))
        }
        Err(NameRefusal::Excluded(form)) => {
            AuthorityName::Outside(PathRefusal::NameExcluded(names::form_word(form)))
        }
        Err(NameRefusal::UriWithoutDomainHost) => {
            AuthorityName::Outside(PathRefusal::UriWithoutDomainHost)
        }
        Err(NameRefusal::FormNotProcessed(form)) => {
            AuthorityName::Undecided(PathRefusal::NameFormNotProcessed(names::form_word(form)))
        }
        Err(NameRefusal::Undecided(form)) => {
            AuthorityName::Undecided(PathRefusal::NameComparisonUndecided(names::form_word(form)))
        }
    }
}

/// The sentence for a time-stamp trust condition's name constraints.
pub(super) fn authority_name_sentence(
    f: &mut std::fmt::Formatter<'_>,
    condition: &str,
    outcome: &AuthorityName,
    verdict: &str,
) -> std::fmt::Result {
    write!(
        f,
        "the name constraints the policy's {condition} states on the time-stamping authority are {verdict}: "
    )?;
    match outcome {
        AuthorityName::Within => f.write_str(
            "every name of the authority's certificate lies within them, tested as RFC 5280 section 6.1.3 (b) and (c) test a certificate",
        ),
        AuthorityName::Outside(refusal) | AuthorityName::Undecided(refusal) => {
            write!(f, "{refusal}")
        }
        AuthorityName::NoAuthorityPath => f.write_str(
            "no path from the authority's certificate to the condition's trust points was found, so no certificate is the authority's",
        ),
    }
}

/// The sentence for one condition's search, with the verdict in its place.
pub(super) fn sentence(
    f: &mut std::fmt::Formatter<'_>,
    condition: &str,
    found: &Found,
    unexamined: usize,
    verdict: &str,
) -> std::fmt::Result {
    write!(f, "the policy's {condition} is {verdict}: ")?;
    let (whose, holder) = if condition == TIMESTAMP_TRUST {
        (
            "the time-stamping authority's",
            "the signature's time-stamp token",
        )
    } else {
        ("the signer's", "the signature")
    };
    match found {
        Found::NoInstant => f.write_str(
            "no instant was given to validate the path at, neither by a host's clock nor by a signature timestamp",
        )?,
        Found::NoSignerCertificate => {
            write!(f, "{holder} does not carry the certificate its signer names")?;
        }
        Found::NoTimestamp => f.write_str(
            "the signature states no signature-time-stamp attribute, which is the time-stamp TR 102 272 clause 6.8 applies the condition to, so no authority's path was asked",
        )?,
        Found::TimestampNotVerified(why) => write!(
            f,
            "the signature's time-stamp token authenticates no authority, because {}",
            unestablished_words(why)
        )?,
        Found::PastSearchBound => write!(
            f,
            "the policy states more time-stamp trust conditions than the {MAX_AUTHORITY_SEARCHES} this program searches for one signature, and this one is past them"
        )?,
        Found::TimestampAboutAnotherSignature => f.write_str(
            "the imprint of the signature's time-stamp token is not the digest of this signature (ETSI EN 319 122-1 clause 5.3), so its authority is not this signature's",
        )?,
        Found::NoTrustPoint => f.write_str(
            "none of the trust points the policy states is a certificate this program reads",
        )?,
        Found::Reached {
            length,
            unprocessed,
            evaluated,
            policies,
        } => reached_words(
            f,
            *length,
            whose,
            unprocessed,
            evaluated,
            policies.as_deref(),
        )?,
        Found::Longer {
            intermediates,
            limit,
        } => write!(
            f,
            "the path to the policy's trust point holds {intermediates} CA certificate(s) below it, and its pathLenConstraint admits {limit}"
        )?,
        Found::Refused(refusal, named) => {
            write!(
                f,
                "a chain to one of the policy's own trust points was refused by RFC 5280 section 6.1: {refusal}"
            )?;
            if !named.is_empty() {
                write!(
                    f,
                    " (that trust point states its {}, which section 6.1 evaluates as its inputs)",
                    named.join(", ")
                )?;
            }
        }
        Found::PastReliable { until } => write!(
            f,
            "the only trust point a path reached was reliable until {until}, which is before the instant validated at, so the path is indeterminate (ETSI EN 319 102-1 clause 5.2.6.4)"
        )?,
        Found::NoChain => write!(
            f,
            "no chain from {whose} certificate over the certificates {holder} carries reaches any of the policy's own trust points, which ETSI EN 319 102-1 clause 5.2.6.4 calls indeterminate"
        )?,
    }
    if unexamined > 0
        && !matches!(found, Found::Reached { unprocessed, .. } if unprocessed.is_empty())
    {
        write!(
            f,
            "; {unexamined} of its trust anchors were not searched — a trusted-list reference this program does not fetch, a certificate it does not read, or one past the {MAX_TRUST_POINTS} it searches"
        )?;
    }
    Ok(())
}

/// The sentence's middle for [`Found::Reached`].
fn reached_words(
    f: &mut std::fmt::Formatter<'_>,
    length: usize,
    whose: &str,
    unprocessed: &[&'static str],
    evaluated: &[&'static str],
    policies: Option<&[Vec<u8>]>,
) -> std::fmt::Result {
    write!(
        f,
        "a path of {length} certificate(s) from {whose} validates to one of the policy's own trust points under RFC 5280 section 6.1"
    )?;
    if !evaluated.is_empty() {
        write!(
            f,
            ", with that trust point's {} evaluated as section 6.1.1's inputs",
            evaluated.join(", ")
        )?;
    }
    match policies {
        Some([]) => f.write_str(
            ", for none of the policies of its acceptable set — no certificate on the path asserts one, and RFC 5280 section 6.1.5 asks for one only where an explicit policy is required",
        )?,
        Some(policies) => {
            let dotted: Vec<String> = policies
                .iter()
                .map(|oid| {
                    x509::dotted(oid).unwrap_or_else(|| "an unreadable identifier".to_owned())
                })
                .collect();
            write!(f, ", for the policies {} of its acceptable set", dotted.join(", "))?;
        }
        None => {}
    }
    if !unprocessed.is_empty() {
        write!(
            f,
            ", but that trust point also states its {}, which this program's path validation does not process",
            unprocessed.join(", ")
        )?;
    }
    Ok(())
}

/// Why a time-stamp token authenticates nobody, in a sentence's words.
fn unestablished_words(why: &Unestablished) -> String {
    match why {
        Unestablished::TokenUnreadable(refusal) => format!("it would not read: {refusal}"),
        Unestablished::ContentNotBoundToTheSignature => {
            "its signed attributes do not commit to its TSTInfo (RFC 5652 section 5.6)".to_owned()
        }
        Unestablished::NoMessageDigestAttribute => {
            "it states signed attributes and no message-digest among them (RFC 5652 section 11.2)"
                .to_owned()
        }
        Unestablished::SignatureNotVerified(_) => "its own signature does not verify".to_owned(),
        // Neither is reached by steps 1 to 3 over a signature-time-stamp attribute; each is
        // worded for completeness rather than left to a catch-all.
        Unestablished::NotADocumentTimestamp => "it is not a time-stamp token".to_owned(),
        Unestablished::AuthorityNotEstablished(_) => {
            "its authority's path was not established".to_owned()
        }
    }
}

/// The sentence for one issued certificate's algorithm against a list, used by both syntaxes.
pub(super) fn issued_words(algorithm: &str, issuer_key_bits: Option<usize>) -> String {
    match issuer_key_bits {
        Some(bits) => format!("{algorithm} by a {bits}-bit issuer key"),
        None => format!("{algorithm} by an issuer key whose length is not measured"),
    }
}

#[cfg(test)]
pub(super) mod tests;

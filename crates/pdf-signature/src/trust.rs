//! RFC 5280 section 6.1's certification path validation, with the anchors supplied from outside.
//!
//! ISO 32000-2 §12.8.1 divides a signature into three questions and this module is the third's
//! machinery: "[t]he signer's certificate shall be determined and verified by the signature
//! handler to match with any of the validation parameters and other conditions". [`crate::cms`]
//! and [`crate::x509`] determine the certificate and [`crate::signature`] verifies the signature
//! value under its key — questions 1 and 2. What is left is whether that certificate is anybody's,
//! and §12.8.3.4.5 (b) says by whose rules: "validate the certification path according to the
//! validation model with the rules given in Internet RFC 5280".
//!
//! # The trust anchors are an input, and that is the whole design decision
//!
//! RFC 5280 section 6.1.1 lists nine inputs to path validation and makes the anchors one of them,
//! input (d). The RFC is explicit about whose choice they are: "The selection of a trust anchor is
//! a matter of policy: it could be the top CA in a hierarchical PKI, the CA that issued the
//! verifier's own certificate(s), or any other CA in a network PKI."
//!
//! That sentence and `CLAUDE.md` principle 3 say the same thing from two directions, so this
//! module holds no certificate list, reads no file, opens no socket and asks no clock. A host
//! supplies [`TrustAnchors`] and the instant to validate at; an empty set is the default and
//! yields [`Trust::NoAnchorSupplied`], which is a statement about this program rather than about
//! the signature. ADR 1039 prices the two alternatives — a compiled-in root list and the
//! platform's store — and says why neither is the default.
//!
//! # What this validates, and what it refuses rather than assumes
//!
//! Section 6.1 permits an implementation to leave parts out: "support for some of the certificate
//! extensions processed in this algorithm are OPTIONAL for compliant implementations. Clients that
//! do not support these extensions MAY omit the corresponding steps in the path validation
//! algorithm." It also states the sentence that makes an omission safe rather than silent: "Note
//! that clients MUST reject the certificate if it contains an unsupported critical extension."
//!
//! Done, per certificate, in section 6.1.3's and 6.1.4's own lettering:
//!
//! - **(a)(1)** the signature on the certificate, verified under the working public key with
//!   [`crate::pkcs1`], [`crate::pss`], [`crate::dsa`], [`crate::ecdsa`] or [`crate::eddsa`];
//! - **(a)(2)** the validity period against the instant the caller supplied;
//! - **(a)(4)** the issuer name against the working issuer name;
//! - **(k)** `basicConstraints` with `cA` asserted on every certificate that signs another, and
//!   version 1 and version 2 intermediates rejected, which the step permits in as many words;
//! - **(l)** and **(m)** `max_path_length`, decremented and lowered by `pathLenConstraint`;
//! - **(n)** `keyUsage`, where present, asserting `keyCertSign`;
//! - **(o)** every critical extension this reader does not recognise, as a refusal.
//!
//! **Not done, and named at every level rather than assumed:**
//!
//! - **(a)(3), revocation.** Not checked, and no outcome of this module says otherwise:
//!   [`Trust::Anchored`] carries [`Revocation::NotChecked`] and there is no second variant to
//!   carry. A CRL or an OCSP response is §12.8.3.3.2's and §12.8.3.4.6's subject and needs a
//!   network, which is a security argument this project has not had.
//! - **The policy tree** — section 6.1.2 (a), 6.1.3 (d) to (f), 6.1.4 (h) to (j) and 6.1.5 (g).
//!   This module fixes `user-initial-policy-set` to `any-policy` and `initial-explicit-policy`,
//!   `initial-policy-mapping-inhibit` and `initial-any-policy-inhibit` all to false, under which
//!   section 6.1.5's success condition — "the value of `explicit_policy` variable is greater than
//!   zero" — holds for every path, because the only thing that can drive `explicit_policy` to zero
//!   is `requireExplicitPolicy` in a policy constraints extension, and section 4.2.1.11 says
//!   "Conforming CAs MUST mark this extension as critical". The same argument covers name
//!   constraints (section 4.2.1.10) and inhibit anyPolicy (section 4.2.1.14): each is a critical
//!   extension this reader does not recognise, so a certificate that needs the omitted step is
//!   refused by (o) instead of passing without it.
//! - **Name matching is byte comparison**, not section 7.1's. Two encodings of one name that
//!   section 7.1 would call equal compare unequal here, so a path that should chain may fail to —
//!   never the other way round. The cost is a false [`Trust::NoPathToAnyAnchor`] on a producer
//!   that re-encoded a name between certificates; the alternative is a comparison that could join
//!   two names one authority did not, which is the failure that matters.
//!
//! # The bounds, because a certificate chain is a stranger's
//!
//! Every certificate here came out of a PDF somebody else wrote, so nothing is walked without a
//! limit: [`MAX_PATH_LENGTH`] bounds how many certificates a path may hold, [`MAX_CANDIDATES`]
//! how many of the file's certificates are considered as issuers at all, [`MAX_STEPS`] the whole
//! search, and no certificate appears twice in one path — which is also RFC 5280 section 6.1's own
//! rule: "A certificate MUST NOT appear more than once in a prospective certification path."

use crate::cms::Digest;
use crate::x509::{self, Certificate, Instant, PublicKey};
use crate::{dsa, ecdsa, eddsa, pkcs1, pss};

/// How many certificates a prospective certification path may hold, the target included.
///
/// RFC 5280 states no ceiling — section 6.1.2 (k) initialises `max_path_length` to the length of
/// whatever path it was handed — so this is a bound on work over untrusted input rather than a
/// reading of the standard. Eight is far past anything a real hierarchy uses: the deepest chains
/// in public practice are a root, two intermediates and an end entity.
pub const MAX_PATH_LENGTH: usize = 8;

/// How many of the file's certificates are considered as candidate issuers.
///
/// A CMS `SignedData` may carry any number of certificates and the search below looks at each of
/// them at every level, so this is what keeps the product finite alongside [`MAX_STEPS`].
pub const MAX_CANDIDATES: usize = 64;

/// How many certificates the search will validate before giving up.
///
/// The depth-first walk is bounded by [`MAX_PATH_LENGTH`] and [`MAX_CANDIDATES`] already; this is
/// the belt to that pair of braces, and it is what a report means by "examined".
pub const MAX_STEPS: usize = 256;

/// RFC 5280 section 6.1.1 input (d): a CA this reader has been told to believe.
///
/// RFC 5280 section 6.1.1 (d): "The trust anchor information includes: (1) the trusted issuer
/// name, (2) the trusted public key algorithm, (3) the trusted public key, and (4) optionally, the
/// trusted public key parameters associated with the public key."
///
/// The name and the key are held; the algorithm and its parameters travel inside [`PublicKey`],
/// which is how [`crate::x509`] reads a `subjectPublicKeyInfo`. Nothing about where the anchor
/// came from is recorded here, deliberately: this type is the same whether a host read it from a
/// file, a platform store or a command line, and the difference is the host's to explain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrustAnchor<'a> {
    /// The trusted issuer name — the encoded `Name`, compared and never decoded.
    pub name: &'a [u8],
    /// The trusted public key.
    pub key: PublicKey<'a>,
}

impl<'a> TrustAnchor<'a> {
    /// The anchor a self-signed certificate stands for.
    ///
    /// RFC 5280 section 6.1.1 (d): "When the trust anchor information is provided in the form of a
    /// certificate, the name in the subject field is used as the trusted issuer name and the
    /// contents of the subjectPublicKeyInfo field is used as the source of the trusted public key
    /// algorithm and the trusted public key."
    ///
    /// Nothing about the certificate is checked here — not its dates, not its own signature, not
    /// its basic constraints — because section 6.1's paragraph on the subject says why there would
    /// be nothing to check against: "The trust anchor information is trusted because it was
    /// delivered to the path processing procedure by some trustworthy out-of-band procedure." The
    /// host vouched for it; this program's job starts at the certificate below it.
    #[must_use]
    pub const fn of(certificate: &Certificate<'a>) -> Self {
        Self {
            name: certificate.subject,
            key: certificate.public_key,
        }
    }
}

/// The set of trust anchors a host supplies, and the default is empty.
///
/// Empty is not a failure and not a refusal: it is this program saying nobody has told it whom to
/// believe, which is what [`Trust::NoAnchorSupplied`] reports and what every signed document this
/// program opens has said in words since the three-hundred-and-seventy-seventh session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrustAnchors<'a> {
    anchors: Vec<TrustAnchor<'a>>,
}

impl<'a> TrustAnchors<'a> {
    /// An empty set — nobody to believe.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            anchors: Vec::new(),
        }
    }

    /// The set a host's certificates stand for.
    #[must_use]
    pub fn of(certificates: &[Certificate<'a>]) -> Self {
        Self {
            anchors: certificates.iter().map(TrustAnchor::of).collect(),
        }
    }

    /// One more anchor.
    pub fn push(&mut self, anchor: TrustAnchor<'a>) {
        self.anchors.push(anchor);
    }

    /// Whether nobody has been named.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// How many anchors there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.anchors.len()
    }

    /// The anchors, in the order the host supplied them.
    #[must_use]
    pub fn anchors(&self) -> &[TrustAnchor<'a>] {
        &self.anchors
    }
}

/// Whether anything was asked about revocation, which today is always no.
///
/// A one-variant enum rather than an omitted field, and that is the point: RFC 5280 section 6.1.3
/// (a)(3) is a step of path validation — "At the current time, the certificate is not revoked" —
/// so a result that did not mention it would be claiming a check it had not made. The day a CRL
/// or an OCSP response is read, this type gains the variant that says so and every reader of
/// [`Trust::Anchored`] is made to look at it by the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Revocation {
    /// Nothing was asked. §12.8.3.3.2 and §12.8.3.4.6 are what would ask, and both need a network.
    NotChecked,
}

/// Why a prospective path was refused, in RFC 5280 section 6.1's own terms.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PathRefusal {
    /// A certificate carries a critical extension this reader does not recognise.
    ///
    /// RFC 5280 section 4.2's `MUST`, and the sentence that makes the omitted steps safe.
    #[error("a certificate states the critical extension {0}, which this reader does not process")]
    UnrecognisedCriticalExtension(String),
    /// A certificate has more extensions than [`x509::MAX_EXTENSIONS`], so one of the unread ones
    /// may be critical and unrecognised.
    #[error("a certificate states more extensions than this reader walks, so one may be critical")]
    ExtensionsUnread,
    /// A certificate uses X.690's indefinite length, so what its issuer signed is not pinned.
    #[error("a certificate is written with indefinite lengths, which DER forbids")]
    NotDerEncoded,
    /// A certificate's validity period is not one this reader can place on a line.
    #[error("a certificate's validity period is not readable")]
    ValidityUnreadable,
    /// A certificate is not current at the instant validation was asked about — section 6.1.3
    /// (a)(2).
    #[error("a certificate is not valid at the instant asked about")]
    NotCurrent,
    /// The issuer's signature over a certificate does not verify — section 6.1.3 (a)(1).
    #[error("a certificate's signature does not verify under its issuer's key")]
    SignatureNotUnderIssuersKey,
    /// The issuer's key, or the certificate's signature algorithm, is one this program does not
    /// compute — named by the identifier the certificate states.
    #[error("a certificate is signed with {0}, which this program does not verify")]
    AlgorithmNotVerifiable(String),
    /// A certificate that signs another states no `basicConstraints` with `cA` asserted, or is a
    /// version 1 or version 2 intermediate — section 6.1.4 (k).
    #[error("a certificate in the path is not a certification authority")]
    NotACertificationAuthority,
    /// A certificate that signs another states a `keyUsage` without `keyCertSign` — section 6.1.4
    /// (n).
    #[error("a certificate in the path states a key usage that forbids signing certificates")]
    KeyUsageForbidsCertificateSigning,
    /// `max_path_length` reached zero with certificates left — section 6.1.4 (l).
    #[error("the path is longer than a pathLenConstraint in it permits")]
    PathLengthExceeded,
    /// A certificate's issuer is not the name of the certificate above it — section 6.1.3 (a)(4).
    ///
    /// **Reaching this is this program disagreeing with itself rather than a file being wrong.**
    /// [`validate`] builds a path by matching each certificate's subject to the next one's issuer
    /// and closes it on an anchor whose name is the topmost issuer, so the step can only fail if
    /// the search and the walk read those two fields differently. It is checked anyway, because
    /// section 6.1.3 states it and a validation that trusted its own path builder would be
    /// asserting the thing it is for.
    #[error("a certificate's issuer is not the name of the certificate that signed it")]
    IssuerNameDoesNotChain,
}

/// What this program can say about whose certificate signed a document.
///
/// **No variant says *valid* and none says *trusted***, which is the same discipline
/// [`crate::signature::Authenticity`] keeps for question 2 and for the same reason: revocation is
/// a step of path validation and it is not taken, so the strongest true sentence is the one
/// [`Self::Anchored`] spells out.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Trust {
    /// Nobody supplied an anchor, so there was no question to answer.
    ///
    /// The default state of this program, and not a defect in the document.
    NoAnchorSupplied,
    /// A path was built to a supplied anchor and every check this module makes passed.
    Anchored {
        /// How many certificates the path holds, the target included and the anchor excluded —
        /// RFC 5280 section 6.1's `n`.
        length: usize,
        /// What was asked about revocation, which is nothing.
        revocation: Revocation,
    },
    /// No path from the signer's certificate to any supplied anchor could be built.
    ///
    /// Nothing was wrong with any certificate: the search never reached an anchor, which is what a
    /// host supplying the wrong store looks like, and what a file carrying too few certificates
    /// looks like too.
    NoPathToAnyAnchor {
        /// How many certificates the search looked at before giving up, which is zero where no
        /// prospective path ever reached an anchor.
        examined: usize,
    },
    /// Every path the search found was refused, and this is why the furthest one was.
    Refused {
        /// The first refusal any prospective path produced.
        refusal: PathRefusal,
        /// How many certificates the search looked at.
        examined: usize,
    },
}

/// RFC 5280 section 6.1's path validation, over the certificates a file carries.
///
/// `target` is the signer's certificate — section 6.1's certificate `n` — `others` the rest of
/// what the signature carried, `anchors` what the host supplied, and `at` section 6.1.1's input
/// (b), "the current date/time", which is the caller's to know.
///
/// The path is *built* here as well as validated, which section 6.1 leaves open: "The procedure
/// performed to obtain this sequence of certificates is outside the scope of this specification."
/// The search is depth-first from the target upwards, bounded by [`MAX_PATH_LENGTH`],
/// [`MAX_CANDIDATES`] and [`MAX_STEPS`], and it returns the first path that validates.
#[must_use]
pub fn validate<'p, 'c: 'p>(
    target: &'p Certificate<'c>,
    others: &'p [Certificate<'c>],
    anchors: &'p TrustAnchors<'c>,
    at: Instant,
) -> Trust {
    if anchors.is_empty() {
        return Trust::NoAnchorSupplied;
    }
    let candidates = others
        .get(..others.len().min(MAX_CANDIDATES))
        .unwrap_or(&[]);
    let mut search = Search {
        candidates,
        anchors,
        at,
        steps: 0,
        first_refusal: None,
    };
    // The path is held target-first while it is built and reversed before validation, because
    // section 6.1 numbers certificates from the anchor down and every step reads in that order.
    let mut path = vec![target];
    if let Some(length) = search.extend(&mut path) {
        return Trust::Anchored {
            length,
            revocation: Revocation::NotChecked,
        };
    }
    match search.first_refusal {
        Some(refusal) => Trust::Refused {
            refusal,
            examined: search.steps,
        },
        None => Trust::NoPathToAnyAnchor {
            examined: search.steps,
        },
    }
}

/// The depth-first walk from the target certificate up towards an anchor.
///
/// Two lifetimes: `'c` is the buffer the certificates were read out of and `'p` the borrow of the
/// slice holding them, which outlives nothing and is what lets a caller keep its certificates in a
/// local.
struct Search<'p, 'c: 'p> {
    candidates: &'p [Certificate<'c>],
    anchors: &'p TrustAnchors<'c>,
    at: Instant,
    steps: usize,
    /// The first refusal any prospective path produced.
    ///
    /// The first rather than the last, and the distinction is worth the field name: a search that
    /// tried several paths refused each for its own reason, and reporting the newest would make
    /// the answer depend on the order the file happened to list its certificates in.
    first_refusal: Option<PathRefusal>,
}

impl<'p, 'c: 'p> Search<'p, 'c> {
    /// The length of the first extension of `path` that reaches an anchor and validates.
    ///
    /// `path` is ordered target-first, and the length returned is the one it had at the moment it
    /// validated — the recursion unwinds on its way out, so reading `path` afterwards would give
    /// the target alone. The walk is bounded by [`MAX_PATH_LENGTH`] on the way down and by
    /// [`MAX_STEPS`] across the whole search, so a file carrying sixty-four mutually-issuing
    /// certificates cannot make this run long.
    fn extend(&mut self, path: &mut Vec<&'p Certificate<'c>>) -> Option<usize> {
        if self.steps >= MAX_STEPS || path.len() > MAX_PATH_LENGTH {
            return None;
        }
        let &top = path.last()?;
        // An anchor whose name is this certificate's issuer closes the path: section 6.1's
        // condition (b), "certificate 1 is issued by the trust anchor".
        for anchor in self.anchors.anchors() {
            if anchor.name != top.issuer {
                continue;
            }
            match self.walk(anchor, path) {
                Ok(()) => return Some(path.len()),
                Err(refusal) => self.remember(refusal),
            }
        }
        if path.len() >= MAX_PATH_LENGTH {
            return None;
        }
        for candidate in self.candidates {
            if candidate.subject != top.issuer {
                continue;
            }
            // "A certificate MUST NOT appear more than once in a prospective certification path."
            // Compared by the bytes each certificate occupies, which is what identity means for a
            // value read out of one buffer.
            if path.iter().any(|held| held.tbs == candidate.tbs) {
                continue;
            }
            path.push(candidate);
            let found = self.extend(path);
            path.pop();
            if found.is_some() {
                return found;
            }
            if self.steps >= MAX_STEPS {
                return None;
            }
        }
        None
    }

    /// Keeps the first refusal and discards the rest, for the reason the field documents.
    fn remember(&mut self, refusal: PathRefusal) {
        if self.first_refusal.is_none() {
            self.first_refusal = Some(refusal);
        }
    }

    /// RFC 5280 section 6.1.2 to 6.1.5 over one prospective path, anchor first.
    fn walk(
        &mut self,
        anchor: &TrustAnchor<'_>,
        path: &[&Certificate<'_>],
    ) -> Result<(), PathRefusal> {
        // Section 6.1.2: the state this reduced algorithm keeps. `working_issuer_name` (f) and
        // `working_public_key` (h) start at the anchor's; `max_path_length` (k) is "initialized to
        // n", which here is the bound this program imposes rather than a number off the file.
        let mut working_issuer_name = anchor.name;
        let mut working_public_key = anchor.key;
        let mut max_path_length = MAX_PATH_LENGTH;
        let last = path.len().saturating_sub(1);
        for (index, certificate) in path.iter().rev().enumerate() {
            // One certificate looked at, whether it passes or not: `examined` is the cost this
            // search paid, which is what a bound is about, rather than the number that passed.
            self.steps = self.steps.saturating_add(1);
            basic_processing(
                certificate,
                working_issuer_name,
                working_public_key,
                self.at,
            )?;
            if index < last {
                prepare_for_next(certificate, &mut max_path_length)?;
                working_issuer_name = certificate.subject;
                working_public_key = certificate.public_key;
            }
        }
        // Section 6.1.5's wrap-up is (c) to (e) — assigning the target's key to the working key,
        // which nothing after this reads — and (a), (b) and (g), which are the policy tree's and
        // which this module's own documentation argues are satisfied for every path under
        // `any-policy` with no explicit policy required.
        Ok(())
    }
}

/// RFC 5280 section 6.1.3 (a) for one certificate, plus section 4.2's rule about critical
/// extensions, which section 6.1.3 (o) and 6.1.5 (f) each restate for their own step.
fn basic_processing(
    certificate: &Certificate<'_>,
    working_issuer_name: &[u8],
    working_public_key: PublicKey<'_>,
    at: Instant,
) -> Result<(), PathRefusal> {
    if certificate.indefinite_lengths {
        return Err(PathRefusal::NotDerEncoded);
    }
    if certificate.extensions.truncated {
        return Err(PathRefusal::ExtensionsUnread);
    }
    if let Some(oid) = certificate.extensions.unrecognised_critical {
        return Err(PathRefusal::UnrecognisedCriticalExtension(
            x509::dotted(oid).unwrap_or_else(|| "an unreadable object identifier".to_owned()),
        ));
    }
    // (a)(4), before the arithmetic, and by its own name: see `IssuerNameDoesNotChain` for why
    // this cannot fire on a path this module built and is checked regardless.
    if certificate.issuer != working_issuer_name {
        return Err(PathRefusal::IssuerNameDoesNotChain);
    }
    // (a)(2).
    let Some(validity) = certificate.validity else {
        return Err(PathRefusal::ValidityUnreadable);
    };
    if !validity.includes(at) {
        return Err(PathRefusal::NotCurrent);
    }
    // (a)(1).
    if !verify_certificate(certificate, working_public_key)? {
        return Err(PathRefusal::SignatureNotUnderIssuersKey);
    }
    // (a)(3) is revocation and is not taken; `Revocation::NotChecked` is where that is said.
    Ok(())
}

/// RFC 5280 section 6.1.4's steps (k), (l), (m) and (n) — the ones that decide whether a
/// certificate may sign the next one.
fn prepare_for_next(
    certificate: &Certificate<'_>,
    max_path_length: &mut usize,
) -> Result<(), PathRefusal> {
    // (k). "If certificate i is a version 3 certificate, verify that the basicConstraints
    // extension is present and that cA is set to TRUE. (If certificate i is a version 1 or version
    // 2 certificate, then the application MUST either verify that certificate i is a CA
    // certificate through out-of-band means or reject the certificate. Conforming implementations
    // may choose to reject all version 1 and version 2 intermediate certificates.)" This program
    // has no out-of-band means, so it takes the permission the parenthesis grants.
    if certificate.version < 3 {
        return Err(PathRefusal::NotACertificationAuthority);
    }
    if !certificate
        .extensions
        .basic_constraints
        .is_some_and(|constraints| constraints.ca)
    {
        return Err(PathRefusal::NotACertificationAuthority);
    }
    // (l). Self-issued certificates are not counted, which is section 6.1's own rule: "These
    // self-issued certificates are not counted when evaluating path length or name constraints."
    if certificate.subject != certificate.issuer {
        if *max_path_length == 0 {
            return Err(PathRefusal::PathLengthExceeded);
        }
        *max_path_length = max_path_length.saturating_sub(1);
    }
    // (m).
    if let Some(constraints) = certificate.extensions.basic_constraints
        && let Some(stated) = constraints.path_len
    {
        let stated = usize::try_from(stated).unwrap_or(usize::MAX);
        if stated < *max_path_length {
            *max_path_length = stated;
        }
    }
    // (n).
    if certificate
        .extensions
        .key_usage
        .is_some_and(|usage| !usage.key_cert_sign())
    {
        return Err(PathRefusal::KeyUsageForbidsCertificateSigning);
    }
    Ok(())
}

/// RFC 5280 section 6.1.3 (a)(1): "[t]he signature on the certificate can be verified using" the
/// working public key algorithm, the working public key and its parameters — which travel together
/// here as the [`PublicKey`] the certificate above this one carried.
///
/// Section 4.1.1.3 says what is signed — the `signatureValue` is "generated upon the ASN.1 DER
/// encoded tbsCertificate" — so [`Certificate::tbs`] is the message and no re-encoding happens
/// anywhere on this path.
///
/// The pair rather than either alone, for the reason [`crate::signature::Authenticity`] states:
/// an algorithm identifier naming one family over a key of another is two contradictory claims by
/// one producer, and choosing between them would be this program inventing a fact.
fn verify_certificate(
    certificate: &Certificate<'_>,
    key: PublicKey<'_>,
) -> Result<bool, PathRefusal> {
    let named = || {
        x509::dotted(certificate.signature_algorithm)
            .unwrap_or_else(|| "an unreadable object identifier".to_owned())
    };
    let algorithm = crate::cms::SignatureAlgorithm::from_oid(certificate.signature_algorithm);
    let tbs = certificate.tbs;
    match (algorithm, key) {
        (crate::cms::SignatureAlgorithm::RsaPkcs1V15, PublicKey::Rsa(key)) => {
            let Some(digest) = digest_of(certificate.signature_algorithm) else {
                return Err(PathRefusal::AlgorithmNotVerifiable(named()));
            };
            let computed = digest.compute(&[tbs]);
            pkcs1::verify(key, certificate.signature, digest, &computed)
                .map_err(|_| PathRefusal::AlgorithmNotVerifiable(named()))
        }
        (crate::cms::SignatureAlgorithm::RsaPss, PublicKey::Rsa(key)) => {
            let parameters = pss::parameters(certificate.signature_parameters)
                .map_err(|_| PathRefusal::AlgorithmNotVerifiable(named()))?;
            let computed = parameters.hash.compute(&[tbs]);
            pss::verify(key, certificate.signature, parameters, &computed)
                .map_err(|_| PathRefusal::AlgorithmNotVerifiable(named()))
        }
        (crate::cms::SignatureAlgorithm::Dsa, PublicKey::Dsa(key)) => {
            let Some(digest) = digest_of(certificate.signature_algorithm) else {
                return Err(PathRefusal::AlgorithmNotVerifiable(named()));
            };
            let computed = digest.compute(&[tbs]);
            dsa::verify(key, certificate.signature, &computed)
                .map_err(|_| PathRefusal::AlgorithmNotVerifiable(named()))
        }
        (crate::cms::SignatureAlgorithm::Ecdsa, PublicKey::Ec(key)) => {
            let Some(digest) = digest_of(certificate.signature_algorithm) else {
                return Err(PathRefusal::AlgorithmNotVerifiable(named()));
            };
            let computed = digest.compute(&[tbs]);
            ecdsa::verify(key, certificate.signature, &computed)
                .map_err(|_| PathRefusal::AlgorithmNotVerifiable(named()))
        }
        (crate::cms::SignatureAlgorithm::EdDsa, PublicKey::Ed25519(key)) => {
            // RFC 8032 signs the message rather than a digest of it, which is why this arm has no
            // `digest_of` call and why a certificate signed with Ed25519 states no hash anywhere.
            eddsa::verify(key, certificate.signature, &[tbs])
                .map_err(|_| PathRefusal::AlgorithmNotVerifiable(named()))
        }
        _ => Err(PathRefusal::AlgorithmNotVerifiable(named())),
    }
}

/// Which digest a certificate's combined signature algorithm identifier names.
///
/// A certificate differs from a CMS `SignerInfo` here and the difference is the whole reason this
/// function exists: RFC 5652 puts the digest in its own `digestAlgorithm` member, which
/// [`crate::cms`] reads, while RFC 5280 section 4.1.1.2 carries one identifier for the pair — so
/// `sha256WithRSAEncryption` is the only place a certificate says SHA-256.
///
/// The identifiers come from `const_oid`'s database, which is a second party's reading of the
/// registries that assign them rather than digits typed here; `id-RSASSA-PSS` is deliberately
/// absent, because its hash is in its parameters and [`crate::pss::parameters`] is what reads it.
fn digest_of(oid: &[u8]) -> Option<Digest> {
    use const_oid::db::{rfc5912, rfc9688};
    let table: [(const_oid::ObjectIdentifier, Digest); 13] = [
        (rfc5912::MD_5_WITH_RSA_ENCRYPTION, Digest::Md5),
        (rfc5912::SHA_1_WITH_RSA_ENCRYPTION, Digest::Sha1),
        (rfc5912::SHA_256_WITH_RSA_ENCRYPTION, Digest::Sha256),
        (rfc5912::SHA_384_WITH_RSA_ENCRYPTION, Digest::Sha384),
        (rfc5912::SHA_512_WITH_RSA_ENCRYPTION, Digest::Sha512),
        (rfc5912::DSA_WITH_SHA_1, Digest::Sha1),
        (rfc5912::DSA_WITH_SHA_256, Digest::Sha256),
        (rfc5912::ECDSA_WITH_SHA_256, Digest::Sha256),
        (rfc5912::ECDSA_WITH_SHA_384, Digest::Sha384),
        (rfc5912::ECDSA_WITH_SHA_512, Digest::Sha512),
        (rfc9688::ID_ECDSA_WITH_SHA_3_256, Digest::Sha3_256),
        (rfc9688::ID_ECDSA_WITH_SHA_3_384, Digest::Sha3_384),
        (rfc9688::ID_ECDSA_WITH_SHA_3_512, Digest::Sha3_512),
    ];
    table
        .iter()
        .find(|(identifier, _)| identifier.as_bytes() == oid)
        .map(|&(_, digest)| digest)
}

#[cfg(test)]
mod tests;

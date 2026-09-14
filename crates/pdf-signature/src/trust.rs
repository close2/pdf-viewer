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
//! **(a)(3), revocation, is done from the material the document carries** — §12.8.4's document
//! security store, whose whole purpose is that a verifier needs no network to reach it.
//! [`crate::revocation`] is the reader and the algorithm; [`Trust::Anchored`] carries what it
//! answered, and [`Revocation::Unknown`] rather than [`Revocation::Good`] is what a certificate no
//! material covers gets. ADR 1067.
//!
//! **Not done, and named at every level rather than assumed:**
//!
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

use crate::revocation::{self, Material, Subject};
use crate::x509::{self, Certificate, Instant, KeyUsage, PublicKey};

pub use crate::revocation::{Evidence, Revocation, RevocationReason, Undetermined};

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
    /// The same name with its header, which is what RFC 6960 section 4.1.1's `issuerNameHash`
    /// hashes. Carried because an anchor issues the topmost certificate on a path, and an OCSP
    /// response about that certificate names this anchor by the hash of these octets.
    pub name_encoding: &'a [u8],
    /// The trusted public key's `subjectPublicKey` octets, for `issuerKeyHash`.
    pub key_bits: &'a [u8],
    /// The anchor certificate's `keyUsage`, where it stated one.
    ///
    /// RFC 5280 section 6.3.3 (f) asks for it and asks conditionally — "If a key usage extension
    /// is present in the CRL issuer's certificate" — so `None` is a certificate that stated none
    /// and is not a gap.
    pub key_usage: Option<KeyUsage>,
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
            name_encoding: certificate.subject_encoding,
            key_bits: certificate.public_key_bits,
            key_usage: certificate.extensions.key_usage,
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

/// What this program intends to use the target certificate's key for.
///
/// RFC 5280 section 4.2.1.12 makes an `extKeyUsage` a statement about *use* — "If the extension is
/// present, then the certificate MUST only be used for one of the purposes indicated" — so the only
/// party that can check it is the one with a use in mind. That makes the purpose an input to path
/// validation, the same shape as the anchors and the instant and for the same reason (ADR 1039,
/// ADR 1071 section 4).
///
/// **The check is on the target certificate alone.** Section 4.2.1.12: "In general, this extension
/// will appear only in end entity certificates." A CA on the path that states a critical
/// `extKeyUsage` restricts a use this program has no name for, and that stays
/// [`PathRefusal::UnrecognisedCriticalExtension`] — the conservative branch section 4.2 requires of
/// a system that cannot process an extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Purpose {
    /// The caller has not said, so a certificate restricting its purposes cannot be checked.
    ///
    /// A critical `extKeyUsage` is then [`PathRefusal::UnrecognisedCriticalExtension`], which is
    /// what section 4.2 leaves a system that cannot process one: "A certificate-using system MUST
    /// reject the certificate if it encounters a critical extension it does not recognize or a
    /// critical extension that contains information that it cannot process." A *non*-critical one
    /// is passed over, which is the whole of what non-critical means.
    #[default]
    Unstated,
    /// RFC 3161 section 2.3's `id-kp-timeStamping`, for a document timestamp's authority.
    ///
    /// The clause makes it mandatory and makes it critical: "The corresponding certificate MUST
    /// contain only one instance of the extended key usage field extension as defined in [RFC2459]
    /// Section 4.2.1.13 with KeyPurposeID having value: id-kp-timeStamping. This extension MUST be
    /// critical." So a conforming timestamp authority's certificate is refused by every validator
    /// that treats a critical `extKeyUsage` as unrecognised, which is what this variant exists to
    /// stop being true here.
    #[expect(
        clippy::doc_markdown,
        reason = "RFC 3161 section 2.3 is quoted verbatim and spells its KeyPurposeID in camel \
                  case; a quotation with backticks added to please a lint is no longer a quotation"
    )]
    TimeStamping,
}

impl Purpose {
    /// The `KeyPurposeId` a certificate has to indicate, where this purpose names one.
    #[must_use]
    pub const fn key_purpose(self) -> Option<&'static [u8]> {
        match self {
            Self::Unstated => None,
            Self::TimeStamping => Some(x509::ID_KP_TIME_STAMPING),
        }
    }
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
    /// The target certificate's `extKeyUsage` does not indicate the purpose the caller stated.
    ///
    /// RFC 5280 section 4.2.1.12: "Certificate using applications MAY require that the extended key
    /// usage extension be present and that a particular purpose be indicated in order for the
    /// certificate to be acceptable to that application." [`Purpose`] is where this program states
    /// one, and RFC 3161 section 2.3 is why it has to for a timestamp.
    #[error("the certificate does not indicate the key purpose this use requires")]
    PurposeNotIndicated,
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
        /// What §12.8.4's material in this document said about revocation, over every
        /// certificate on the path — [`Revocation::NotChecked`] where the caller supplied none.
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
/// what the signature carried, `anchors` what the host supplied, `material` §12.8.4's CRLs and
/// OCSP responses out of the document, and `at` section 6.1.1's input (b), "the current
/// date/time", which is the caller's to know.
///
/// [`Material::none`] is a caller that asks nothing about revocation and gets
/// [`Revocation::NotChecked`]; a non-empty one runs section 6.3.3 over every certificate on the
/// path and the answer is the worst of theirs.
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
    material: &'p Material<'c>,
    at: Instant,
) -> Trust {
    validate_for(target, others, anchors, material, at, Purpose::Unstated)
}

/// [`validate`] with the use the caller has for the target's key stated.
///
/// The one thing the purpose changes is RFC 5280 section 4.2.1.12's `extKeyUsage` on the *target*:
/// with a purpose stated the extension is processed and a certificate that does not indicate that
/// purpose is [`PathRefusal::PurposeNotIndicated`]; with none it is unrecognised when critical,
/// which is section 4.2's refusal. [`Purpose`] carries the argument.
#[must_use]
pub fn validate_for<'p, 'c: 'p>(
    target: &'p Certificate<'c>,
    others: &'p [Certificate<'c>],
    anchors: &'p TrustAnchors<'c>,
    material: &'p Material<'c>,
    at: Instant,
    purpose: Purpose,
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
        material,
        at,
        purpose,
        target: target.tbs,
        steps: 0,
        first_refusal: None,
        revocation: Revocation::NotChecked,
    };
    // The path is held target-first while it is built and reversed before validation, because
    // section 6.1 numbers certificates from the anchor down and every step reads in that order.
    let mut path = vec![target];
    if let Some(length) = search.extend(&mut path) {
        return Trust::Anchored {
            length,
            revocation: search.revocation,
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
    material: &'p Material<'c>,
    at: Instant,
    /// What the caller will use the target's key for — RFC 5280 section 4.2.1.12's question.
    purpose: Purpose,
    /// The target certificate's `tbsCertificate`, which is how the walk tells it from the rest.
    ///
    /// Compared by the bytes each certificate occupies, the same identity the path's own
    /// no-duplicates rule uses.
    target: &'c [u8],
    steps: usize,
    /// The first refusal any prospective path produced.
    ///
    /// The first rather than the last, and the distinction is worth the field name: a search that
    /// tried several paths refused each for its own reason, and reporting the newest would make
    /// the answer depend on the order the file happened to list its certificates in.
    first_refusal: Option<PathRefusal>,
    /// What section 6.1.3 (a)(3) answered over the path that validated.
    ///
    /// Written by [`Self::walk`] on the one path that passes, so a prospective path that was
    /// refused for some other reason leaves nothing behind here.
    revocation: Revocation,
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
        // Three more pieces of the same issuer, for section 6.3.3 and RFC 6960 section 4.1.1: the
        // name with its header, the key's own octets, and whether the issuer's certificate limits
        // what its key may sign. All three start at the anchor's.
        let mut working_issuer_encoding = anchor.name_encoding;
        let mut working_issuer_key_bits = anchor.key_bits;
        let mut working_issuer_key_usage = anchor.key_usage;
        let mut max_path_length = MAX_PATH_LENGTH;
        let last = path.len().saturating_sub(1);
        let mut revocation = if self.material.is_empty() {
            Revocation::NotChecked
        } else {
            Revocation::Good {
                from: Evidence::CertificateRevocationList,
                covered: 0,
            }
        };
        for (index, certificate) in path.iter().rev().enumerate() {
            // One certificate looked at, whether it passes or not: `examined` is the cost this
            // search paid, which is what a bound is about, rather than the number that passed.
            self.steps = self.steps.saturating_add(1);
            basic_processing(
                certificate,
                working_issuer_name,
                working_public_key,
                self.at,
                if certificate.tbs == self.target {
                    self.purpose
                } else {
                    Purpose::Unstated
                },
            )?;
            // (a)(3). Asked of every certificate on the path rather than of the target alone,
            // which is what the step's position inside section 6.1.3's per-certificate loop makes
            // it: an intermediate whose own certificate was revoked invalidates everything under
            // it.
            if !self.material.is_empty() {
                let subject = Subject {
                    certificate,
                    issuer_name: working_issuer_encoding,
                    issuer_key_bits: working_issuer_key_bits,
                    issuer_key: working_public_key,
                    issuer_key_usage: working_issuer_key_usage,
                    position: last.saturating_sub(index),
                };
                revocation = revocation::worst(
                    revocation,
                    revocation::status(&subject, self.material, self.at),
                );
            }
            if index < last {
                prepare_for_next(certificate, &mut max_path_length)?;
                working_issuer_name = certificate.subject;
                working_public_key = certificate.public_key;
                working_issuer_encoding = certificate.subject_encoding;
                working_issuer_key_bits = certificate.public_key_bits;
                working_issuer_key_usage = certificate.extensions.key_usage;
            }
        }
        // A path is covered only if every certificate on it was, which is why the count is written
        // once at the end rather than accumulated: `worst` keeps the *worst* answer, and the
        // number of certificates it covers is only meaningful when that answer is `Good`.
        if let Revocation::Good { from, .. } = revocation {
            revocation = Revocation::Good {
                from,
                covered: path.len(),
            };
        }
        self.revocation = revocation;
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
    purpose: Purpose,
) -> Result<(), PathRefusal> {
    if certificate.indefinite_lengths {
        return Err(PathRefusal::NotDerEncoded);
    }
    if certificate.extensions.truncated {
        return Err(PathRefusal::ExtensionsUnread);
    }
    // Section 4.2.1.12's extension, processed where the caller stated what the key is for and
    // unrecognised where it did not. The order matters: a stated purpose *recognises* the
    // extension, so this has to run before the refusal below can name it.
    if let Some(wanted) = purpose.key_purpose() {
        if let Some(stated) = certificate.extensions.extended_key_usage {
            if !x509::indicates_purpose(stated, wanted) {
                return Err(PathRefusal::PurposeNotIndicated);
            }
        } else {
            // "Certificate using applications MAY require that the extended key usage extension be
            // present and that a particular purpose be indicated." RFC 3161 section 2.3 does
            // require it — "[t]he corresponding certificate MUST contain … id-kp-timeStamping" —
            // so an absent extension is a refusal rather than a permission.
            return Err(PathRefusal::PurposeNotIndicated);
        }
    }
    if let Some(oid) = certificate.extensions.unrecognised_critical {
        // A critical `extKeyUsage` this call recognised is not unrecognised, and `x509` records it
        // in both places precisely so that the decision can be made here.
        if !(purpose.key_purpose().is_some() && oid == x509::EXTENDED_KEY_USAGE_OID) {
            return Err(PathRefusal::UnrecognisedCriticalExtension(
                x509::dotted(oid).unwrap_or_else(|| "an unreadable object identifier".to_owned()),
            ));
        }
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
/// anywhere on this path. The arithmetic is [`x509::verify_signature`], which a CRL and an OCSP
/// response reach for the same three fields.
fn verify_certificate(
    certificate: &Certificate<'_>,
    key: PublicKey<'_>,
) -> Result<bool, PathRefusal> {
    x509::verify_signature(
        certificate.tbs,
        certificate.signature_algorithm,
        certificate.signature_parameters,
        certificate.signature,
        key,
    )
    .map_err(PathRefusal::AlgorithmNotVerifiable)
}

#[cfg(test)]
mod tests;

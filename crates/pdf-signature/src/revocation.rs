//! RFC 5280 section 5's certificate revocation lists and RFC 6960's OCSP responses, read from
//! §12.8.4's document security store and applied to a certification path.
//!
//! # Why this can exist without a network
//!
//! RFC 5280 section 6.1.3 (a)(3) is the one step of path validation [`crate::trust`] does not
//! take — "the certificate is not revoked" — and every ledger row that named it also named a
//! network, because a CRL is normally fetched from a distribution point and an OCSP response from
//! a responder. §12.8.4.2 is the clause that removes the network from the question: the material
//! is carried *inside the file*, so that a signature stays checkable after the servers that would
//! have answered are gone.
//!
//! > A PDF signature may not be successfully verified unless its collateral validation components
//! > are preserved, e.g., certificates, CRLs, timestamp tokens, revocation lists, and OCSP
//! > responses.
//!
//! Table 261's `/CRLs` and `/OCSPs` are those components, each "an array of indirect references to
//! streams" holding a DER encoding, and [`crate::signature::security_store`] is what reads them
//! out. **Nothing in this module opens a socket**, and nothing in it will: material that is not in
//! the document is material this program does not have, which is what [`Revocation::Unknown`]
//! says.
//!
//! # What the answer may be, and which way it is wrong
//!
//! **Absence of evidence is [`Revocation::Unknown`] and never [`Revocation::Good`]**, at every
//! level: a certificate no CRL covers, a CRL whose signature does not verify under the key that
//! issued the certificate, a response whose responder is not authorised under RFC 6960 section
//! 4.2.2.2, a list this reader could not walk to its end. RFC 5280 section 6.3.3 ends the same
//! way — "if the revocation status has still not been determined, then return the `cert_status`
//! UNDETERMINED" — and a `Good` computed from material nobody checked would be worse than no
//! answer at all, because it reads as the one thing this program has never said.
//!
//! **A `Revoked` stands even where the material is stale**, which is the deliberate asymmetry: a
//! revocation is a statement its issuer made and an expiry window does not withdraw it, while a
//! "not on this list" older than the list's own `nextUpdate` says nothing about today. So the
//! freshness test turns a `Good` into an [`Undetermined::Stale`] and leaves a `Revoked` alone.
//! ADR 1067 section 3.
//!
//! # The reduction, stated rather than assumed
//!
//! RFC 5280 section 6.3.3 walks distribution points; a DSS names none, and the RFC says what to do
//! with material that arrived some other way: "repeat the process above with any available CRLs
//! not specified in a distribution point but issued by the certificate issuer. For the processing
//! of such a CRL, assume a DP with both the reasons and the cRLIssuer fields omitted and a
//! distribution point name of the certificate issuer." Under that assumed distribution point step
//! (b)(1) is "verify that the CRL issuer matches the certificate issuer", step (d)(4) sets
//! `interim_reasons_mask` to all-reasons, and delta CRLs are out — `use-deltas` is an input
//! (section 6.3.1 (b)) and this module sets it false.
//!
//! What is left of the algorithm is done: (b)(1) by name comparison, (b)(2) by the refusal below,
//! (f) by the path [`crate::trust`] has already validated and by `cRLSign`, (g) by the arithmetic,
//! (j) by the serial-number search, and (k) by turning `removeFromCRL` back into unrevoked.
//!
//! **Step (b)(2)'s issuing distribution point is refused rather than processed, on the extension's
//! own instruction.** Section 5.2.5: "Although the extension is critical, conforming
//! implementations are not required to support this extension. However, implementations that do
//! not support this extension MUST either treat the status of any certificate not listed on this
//! CRL as unknown or locate another CRL that does not contain any unrecognized critical
//! extensions." This reader takes the first branch, for every unrecognised critical CRL extension
//! and not only that one — the same argument [`crate::trust`] makes for a certificate's.
//!
//! # The bounds, because every byte here is a stranger's
//!
//! [`crate::der`] caps a value at [`crate::der::MAX_VALUE`], so a list is bounded before this
//! module sees it; on top of that [`MAX_REVOKED_ENTRIES`] bounds the search through one CRL's
//! entries, [`MAX_SINGLE_RESPONSES`] how many statuses one response may state, and
//! [`MAX_RESPONDER_CERTIFICATES`] how many certificates a response may offer as its signer's.
//! A search that hits a bound reports [`Undetermined`] rather than what it managed, for the reason
//! the module's first section gives.

use crate::cms::{ADBE_REVOCATION_INFO_ARCHIVAL, Digest, SignedData};
use crate::der::{self, DerError, INTEGER, NotCanonical, OCTET_STRING, Reader, SEQUENCE, Value};
use crate::x509::{self, Certificate, Instant, KeyUsage, PublicKey};
use crate::{dsa, ecdsa, eddsa, pkcs1, pss};

/// X.690's `BIT STRING`, primitive and universal.
const BIT_STRING: u8 = 0x03;

/// X.690's `ENUMERATED`, primitive and universal — RFC 5280 section 5.3.1's `CRLReason`.
const ENUMERATED: u8 = 0x0A;

/// X.690's `BOOLEAN`, primitive and universal.
const BOOLEAN: u8 = 0x01;

/// RFC 5280 section 5.3.1's `id-ce-cRLReasons`, `{ id-ce 21 }` — 2.5.29.21.
const CRL_REASONS: &[u8] = &[0x55, 0x1D, 0x15];

/// RFC 6960 section 4.2.1's `id-pkix-ocsp-basic`, 1.3.6.1.5.5.7.48.1.1.
const OCSP_BASIC: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x30, 0x01, 0x01];

/// RFC 5280 section 4.2.1.12's `id-kp-OCSPSigning`, `{ id-kp 9 }` — 1.3.6.1.5.5.7.3.9.
const OCSP_SIGNING: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x09];

/// How many entries of one CRL's `revokedCertificates` are walked before the search gives up.
///
/// RFC 5280 states no ceiling, so this is a bound on work over a stranger's bytes rather than a
/// reading of the standard — and [`crate::der::MAX_VALUE`] already caps the whole encoding at two
/// mebibytes, which is about sixty thousand entries of the shape section 5.1.2.6 describes. This
/// is therefore a belt to that brace and is set above what the brace admits, so that a list which
/// fits in a document is never cut short by *this* number.
pub const MAX_REVOKED_ENTRIES: usize = 100_000;

/// How many `SingleResponse` values one `BasicOCSPResponse` may state.
///
/// A response answers a request, and RFC 6960 section 4.1.1's `requestList` is one entry per
/// certificate asked about; a DSS's responses in practice hold one. The bound is generous by three
/// orders of magnitude and exists because nothing in the RFC forbids a thousand.
pub const MAX_SINGLE_RESPONSES: usize = 256;

/// How many certificates one `BasicOCSPResponse`'s `certs` member may offer.
///
/// RFC 6960 section 4.2.1: "The responder MAY include certificates in the certs field of
/// BasicOCSPResponse that help the OCSP client verify the responder's signature." What is being
/// searched is a delegation, which is one certificate and its issuers.
#[expect(
    clippy::doc_markdown,
    reason = "the sentence quotes RFC 6960 section 4.2.1 verbatim, and a quotation is not marked up"
)]
pub const MAX_RESPONDER_CERTIFICATES: usize = 16;

/// How many extensions one CRL, or one `SingleResponse`, is walked for.
///
/// The same reasoning as [`crate::x509::MAX_EXTENSIONS`]: a walk that stopped early could not
/// claim there was no critical extension it did not recognise, so the bound is reported rather
/// than silently applied.
pub const MAX_EXTENSIONS: usize = 32;

/// Why a piece of revocation material would not read.
///
/// Every variant is a refusal by name rather than a value quietly dropped, which is what
/// `CLAUDE.md` principle 1 asks of a parser over untrusted input and what lets
/// [`crate::signature::SecurityStore`] say which of a document's entries it could not use.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MaterialRefusal {
    /// The encoding departs from one of ITU-T X.690's distinguished encoding rules.
    ///
    /// Table 261 says each stream holds "a DER-encoded Certificate Revocation List (CRL)" or "a
    /// DER-encoded Online Certificate Status Protocol (OCSP) response", and RFC 6960 section 4.2.1
    /// says "[t]he value for response SHALL be the DER encoding of BasicOCSPResponse". What the
    /// signature covers is an extent, and an extent found by scanning for an end-of-contents
    /// marker is not one its issuer pinned — the argument [`crate::trust`] already makes for a
    /// certificate.
    #[error("the material departs from DER: {0}")]
    #[expect(
        clippy::doc_markdown,
        reason = "the sentence quotes RFC 6960 section 4.2.1 verbatim, and a quotation is not marked up"
    )]
    NotDerEncoded(NotCanonical),
    /// The ASN.1 would not read at all.
    #[error("the material is not readable ASN.1: {0}")]
    Unreadable(#[from] DerError),
    /// The outer shape is not RFC 5280 section 5.1's `CertificateList`.
    #[error("the material is not an RFC 5280 CertificateList")]
    NotACertificateList,
    /// The outer shape is not RFC 6960 section 4.2.1's `OCSPResponse`.
    #[error("the material is not an RFC 6960 OCSPResponse")]
    NotAnOcspResponse,
    /// `responseStatus` was not `successful (0)`, so RFC 6960 section 4.2.1 says there is nothing
    /// inside: "If the value of responseStatus is one of the error conditions, the responseBytes
    /// field is not set."
    #[error("the OCSP response states responseStatus {0}, which carries no status information")]
    ResponseNotSuccessful(u8),
    /// `responseType` is not `id-pkix-ocsp-basic`, which is the only one RFC 6960 section 4.2.1
    /// requires a client to process.
    #[error("the OCSP response states a response type this reader does not process")]
    ResponseTypeNotBasic,
    /// A date in the material is not one this reader can place on a line.
    #[error("a date in the material is not readable")]
    DateUnreadable,
}

/// RFC 5280 section 5.3.1's `CRLReason`, by the name the ENUMERATED spells.
///
/// The numbers are the clause's own: "unspecified (0), keyCompromise (1), cACompromise (2),
/// affiliationChanged (3), superseded (4), cessationOfOperation (5), certificateHold (6), — value
/// 7 is not used — removeFromCRL (8), privilegeWithdrawn (9), aACompromise (10)". §12.8.3.4.8 reads
/// two of them by name, which is why this is an enumeration here rather than a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RevocationReason {
    /// `unspecified (0)`.
    Unspecified,
    /// `keyCompromise (1)`. §12.8.3.4.8 names it.
    KeyCompromise,
    /// `cACompromise (2)`. §12.8.3.4.8 names it.
    CaCompromise,
    /// `affiliationChanged (3)`.
    AffiliationChanged,
    /// `superseded (4)`.
    Superseded,
    /// `cessationOfOperation (5)`.
    CessationOfOperation,
    /// `certificateHold (6)`.
    CertificateHold,
    /// `removeFromCRL (8)`, which section 6.3.3 (k) turns back into unrevoked.
    RemoveFromCrl,
    /// `privilegeWithdrawn (9)`.
    PrivilegeWithdrawn,
    /// `aACompromise (10)`.
    AaCompromise,
    /// A number the clause does not assign, kept as the file states it.
    ///
    /// Not an error: a reason this reader does not name still means *revoked*, and dropping the
    /// entry because its reason was unfamiliar would turn a revocation into silence.
    Unassigned(u8),
}

impl RevocationReason {
    /// The reason an `ENUMERATED`'s octet names.
    #[must_use]
    pub const fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Unspecified,
            1 => Self::KeyCompromise,
            2 => Self::CaCompromise,
            3 => Self::AffiliationChanged,
            4 => Self::Superseded,
            5 => Self::CessationOfOperation,
            6 => Self::CertificateHold,
            8 => Self::RemoveFromCrl,
            9 => Self::PrivilegeWithdrawn,
            10 => Self::AaCompromise,
            other => Self::Unassigned(other),
        }
    }

    /// The clause's own word for this reason.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unspecified => "unspecified",
            Self::KeyCompromise => "keyCompromise",
            Self::CaCompromise => "cACompromise",
            Self::AffiliationChanged => "affiliationChanged",
            Self::Superseded => "superseded",
            Self::CessationOfOperation => "cessationOfOperation",
            Self::CertificateHold => "certificateHold",
            Self::RemoveFromCrl => "removeFromCRL",
            Self::PrivilegeWithdrawn => "privilegeWithdrawn",
            Self::AaCompromise => "aACompromise",
            Self::Unassigned(_) => "a reason RFC 5280 section 5.3.1 does not assign",
        }
    }
}

/// Which kind of material an answer came off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Evidence {
    /// A CRL from §12.8.4.3's `/CRLs`.
    CertificateRevocationList,
    /// An OCSP response from §12.8.4.3's `/OCSPs`.
    OcspResponse,
}

impl Evidence {
    /// The words a report uses for this kind of material.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::CertificateRevocationList => "a certificate revocation list",
            Self::OcspResponse => "an OCSP response",
        }
    }
}

/// Why no revocation status could be determined — RFC 5280 section 6.3.3's `UNDETERMINED`, with
/// the reason kept rather than collapsed.
///
/// **Every variant here is a sentence about this program or about the document, and none of them
/// is a sentence about the certificate.** That distinction is the whole discipline: a reader who
/// is told "unknown" has been told something true, and a reader told "good" because a list was
/// missing has been told something false.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Undetermined {
    /// The document carries no CRL and no OCSP response at all.
    #[error("this document carries no revocation material")]
    NoMaterial,
    /// No CRL from this certificate's issuer, and no OCSP response naming this certificate.
    #[error("no revocation material in this document covers that certificate")]
    NotCovered,
    /// A CRL or a response that would have covered it did not verify under the issuer's key.
    #[error(
        "the revocation material covering that certificate does not verify under the key that issued it"
    )]
    NotUnderIssuersKey,
    /// The signature on the material is one this program does not compute, named by its identifier.
    #[error(
        "the revocation material covering that certificate is signed with {0}, which this program does not verify"
    )]
    AlgorithmNotVerifiable(String),
    /// An OCSP response's signer is not an authorised responder under RFC 6960 section 4.2.2.2.
    #[error(
        "the OCSP response covering that certificate was not signed by an authorised responder"
    )]
    ResponderNotAuthorised,
    /// The CRL issuer's certificate states a `keyUsage` without `cRLSign` — section 6.3.3 (f).
    #[error("the issuer's certificate states a key usage that forbids signing revocation lists")]
    KeyUsageForbidsCrlSigning,
    /// The material's own validity window does not contain the instant asked about.
    #[error(
        "the revocation material covering that certificate was not current at the instant asked about"
    )]
    Stale,
    /// A CRL carries a critical extension this reader does not recognise — section 5.2.5's own
    /// instruction, and the issuing distribution point is the case it is written about.
    #[error(
        "a revocation list states the critical extension {0}, so a certificate it does not list has an unknown status"
    )]
    UnrecognisedCriticalExtension(String),
    /// A search hit one of this module's bounds, so "not listed" could not be established.
    #[error("the revocation material covering that certificate is larger than this reader walks")]
    TooLarge,
    /// The material would not read.
    #[error("the revocation material covering that certificate would not read: {0}")]
    Unreadable(MaterialRefusal),
}

/// What this program can say about RFC 5280 section 6.1.3 (a)(3) — "the certificate is not
/// revoked" — for a whole certification path.
///
/// **No variant says *valid***, which is the discipline [`crate::trust::Trust`] and
/// [`crate::signature::Authenticity`] already keep. Revocation is one input to a verdict nobody
/// here can give: ADR 1039 records that no host in this tree supplies a trust anchor, so a
/// `Good` here means "the material in this file says this certificate was not revoked", and
/// nothing about whether the signer is anybody.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Revocation {
    /// Nothing was asked, because the caller supplied no material.
    ///
    /// Distinct from [`Self::Unknown`] with [`Undetermined::NoMaterial`], and the difference is
    /// which of two programs is being described: this variant says *this call did not ask*, and
    /// that one says *the document does not answer*.
    NotChecked,
    /// Every certificate on the path is covered by material that verified, and none is revoked.
    Good {
        /// What the answer came off, for the certificate that was hardest to answer.
        from: Evidence,
        /// How many certificates on the path were covered, the target included.
        covered: usize,
    },
    /// A certificate on the path is revoked, and this is what said so.
    Revoked {
        /// When the issuer says the revocation took effect — a CRL entry's `revocationDate` or an
        /// OCSP `RevokedInfo`'s `revocationTime`.
        at: Instant,
        /// RFC 5280 section 5.3.2's `invalidityDate`, where a CRL entry states one.
        ///
        /// Kept apart from [`Self::Revoked::at`] because the clause keeps them apart: the
        /// revocation date "is the date at which the CA processed the revocation" and this is when
        /// the certificate is known or suspected to have become invalid, which "may be earlier".
        /// A caller asking whether the certificate was good at some past instant wants the earlier
        /// of the two; one asking when the CA acted wants `at`. No OCSP response carries this —
        /// RFC 6960's `RevokedInfo` has no such field — so it is `None` for every `OcspResponse`.
        invalid_from: Option<Instant>,
        /// The reason, where the material states one. RFC 5280 section 5.3.1 says it may not: the
        /// reason code extension "SHOULD be absent instead of using the unspecified (0)
        /// reasonCode value".
        reason: Option<RevocationReason>,
        /// What said so.
        from: Evidence,
        /// How far down the path the revoked certificate is, the target being 0.
        position: usize,
    },
    /// No status could be determined for at least one certificate on the path.
    Unknown {
        /// Why — RFC 5280 section 6.3.3's `UNDETERMINED` with its reason kept.
        why: Undetermined,
        /// How far down the path the certificate is, the target being 0.
        position: usize,
    },
}

/// RFC 5280 section 5.1's `CertificateList`, read for what a status decision needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertificateList<'a> {
    /// `tbsCertList.issuer`'s contents — the encoded `Name`, compared and never decoded, exactly
    /// as [`Certificate::issuer`] is.
    pub issuer: &'a [u8],
    /// `tbsCertList.thisUpdate` — "the issue date of this CRL" (section 5.1.2.4).
    pub this_update: Instant,
    /// `tbsCertList.nextUpdate`, which section 5.1.2.5 makes optional in the ASN.1 and required of
    /// a conforming issuer: "Conforming CRL issuers MUST include the nextUpdate field in all
    /// CRLs. Note that the ASN.1 syntax of TBSCertList describes this field as OPTIONAL".
    #[expect(
        clippy::doc_markdown,
        reason = "the sentence quotes RFC 5280 section 5.1.2.5 verbatim, and a quotation is not marked up"
    )]
    pub next_update: Option<Instant>,
    /// `tbsCertList.revokedCertificates`, unwalked — the search is [`Self::entry_for`].
    revoked: Option<Value<'a>>,
    /// `tbsCertList`'s own encoding, which is what the signature covers.
    pub tbs: &'a [u8],
    /// The outer `signatureAlgorithm`'s object identifier.
    pub signature_algorithm: &'a [u8],
    /// The outer `signatureAlgorithm`'s `parameters`, where it states any.
    pub signature_parameters: Option<Value<'a>>,
    /// `signatureValue`'s octets.
    pub signature: &'a [u8],
    /// The first critical `crlExtensions` entry this reader does not recognise.
    ///
    /// Section 5.2.5's instruction is what this is for, and it is quoted in the module comment:
    /// an unsupported critical CRL extension makes "the status of any certificate not listed on
    /// this CRL" unknown, and the issuing distribution point is the extension it is written about.
    pub unrecognised_critical: Option<&'a [u8]>,
}

/// One entry of `revokedCertificates` — section 5.1.2.6's inner `SEQUENCE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevokedCertificate<'a> {
    /// `userCertificate`, the serial number, as the file encodes the integer.
    pub serial_number: &'a [u8],
    /// `revocationDate`.
    pub revoked_at: Instant,
    /// The `invalidityDate` CRL entry extension (section 5.3.2), where the entry states one.
    ///
    /// A different fact from [`Self::revoked_at`] and the clause says so: this is "the date on
    /// which it is known or suspected that the private key was compromised or that the certificate
    /// otherwise became invalid", while the revocation date "is the date at which the CA processed
    /// the revocation" — and "[t]his date may be earlier than the revocation date in the CRL
    /// entry". Which matters wherever something is asked about an instant in the past.
    pub invalid_from: Option<Instant>,
    /// The `reasonCode` CRL entry extension, where the entry states one.
    pub reason: Option<RevocationReason>,
}

/// RFC 5280 section 5.1's `CertificateList`, read out of `bytes`.
///
/// # Errors
///
/// [`MaterialRefusal`], naming what the bytes are instead.
pub fn certificate_list(bytes: &[u8]) -> Result<CertificateList<'_>, MaterialRefusal> {
    if let Some(rule) = der::is_canonical(bytes)? {
        return Err(MaterialRefusal::NotDerEncoded(rule));
    }
    let mut reader = Reader::new(bytes)?;
    let Some(outer) = reader.next_value()? else {
        return Err(MaterialRefusal::NotACertificateList);
    };
    if outer.identifier != SEQUENCE {
        return Err(MaterialRefusal::NotACertificateList);
    }
    let mut parts = outer.children()?;
    // `CertificateList ::= SEQUENCE { tbsCertList, signatureAlgorithm, signatureValue }`.
    let Some(tbs) = parts.next_value()? else {
        return Err(MaterialRefusal::NotACertificateList);
    };
    if tbs.identifier != SEQUENCE {
        return Err(MaterialRefusal::NotACertificateList);
    }
    let (signature_algorithm, signature_parameters) = match parts.next_value()? {
        Some(algorithm) if algorithm.identifier == SEQUENCE => {
            let mut members = algorithm.children()?;
            let oid = members
                .next_value()?
                .and_then(|oid| oid.object_identifier());
            (oid.unwrap_or(&[]), members.next_value()?)
        }
        _ => (&[][..], None),
    };
    let signature = match parts.next_value()? {
        Some(value) if value.identifier == BIT_STRING => bit_string_octets(&value).unwrap_or(&[]),
        _ => &[],
    };
    // `TBSCertList ::= SEQUENCE { version OPTIONAL, signature, issuer, thisUpdate, nextUpdate
    // OPTIONAL, revokedCertificates OPTIONAL, crlExtensions [0] EXPLICIT OPTIONAL }` — four of the
    // seven are optional, so every member after the first two is found by tag rather than by
    // counting, which is the one way to read a sequence this shape without guessing.
    let mut members = tbs.children()?;
    let Some(first) = members.next_value()? else {
        return Err(MaterialRefusal::NotACertificateList);
    };
    // `version Version OPTIONAL, -- if present, MUST be v2`, an INTEGER with no context tag, and
    // `signature AlgorithmIdentifier` is a SEQUENCE — so which of the two the first member is is
    // decided by its tag.
    let inner_algorithm = if first.identifier == INTEGER {
        members.next_value()?
    } else {
        Some(first)
    };
    if inner_algorithm.is_none_or(|value| value.identifier != SEQUENCE) {
        return Err(MaterialRefusal::NotACertificateList);
    }
    let Some(issuer) = members.next_value()? else {
        return Err(MaterialRefusal::NotACertificateList);
    };
    if issuer.identifier != SEQUENCE {
        return Err(MaterialRefusal::NotACertificateList);
    }
    let Some(this_update) = members.next_value()? else {
        return Err(MaterialRefusal::NotACertificateList);
    };
    let this_update = x509::read_time(&this_update).ok_or(MaterialRefusal::DateUnreadable)?;
    let mut list = CertificateList {
        issuer: issuer.contents,
        this_update,
        next_update: None,
        revoked: None,
        tbs: tbs.encoding(),
        signature_algorithm,
        signature_parameters,
        signature,
        unrecognised_critical: None,
    };
    for member in std::iter::from_fn(|| members.next_value().transpose()) {
        let member = member?;
        if member.is_context(0) {
            list.unrecognised_critical = unrecognised_critical(&member)?;
        } else if member.identifier == SEQUENCE {
            list.revoked = Some(member);
        } else if list.next_update.is_none() {
            list.next_update =
                Some(x509::read_time(&member).ok_or(MaterialRefusal::DateUnreadable)?);
        }
    }
    Ok(list)
}

impl<'a> CertificateList<'a> {
    /// The entry naming `serial`, where this list holds one.
    ///
    /// Section 6.3.3 (j): "search for the certificate on the complete CRL. If an entry is found
    /// that matches the certificate issuer and serial number as described in Section 5.3.3". The
    /// issuer half is the caller's — this list's `issuer` is checked against the certificate's
    /// before the search, which is step (b)(1) — so what is left here is the serial number.
    ///
    /// # Errors
    ///
    /// [`MaterialRefusal`] where the entries will not read, and [`MaterialRefusal::Unreadable`]
    /// wrapping [`DerError::TooLarge`] where there are more than [`MAX_REVOKED_ENTRIES`] of them:
    /// a search that stopped early may not report "not listed".
    pub fn entry_for(
        &self,
        serial: &[u8],
    ) -> Result<Option<RevokedCertificate<'a>>, MaterialRefusal> {
        let Some(revoked) = self.revoked else {
            // Section 5.1.2.6: "The revoked certificate list is optional to support the case where
            // a CA has not revoked any unexpired certificates that it has issued." An absent list
            // is an empty one and not a defect.
            return Ok(None);
        };
        let mut entries = revoked.children()?;
        let mut seen = 0usize;
        while let Some(entry) = entries.next_value()? {
            seen = seen.saturating_add(1);
            if seen > MAX_REVOKED_ENTRIES {
                return Err(MaterialRefusal::Unreadable(DerError::TooLarge));
            }
            if entry.identifier != SEQUENCE {
                continue;
            }
            let mut fields = entry.children()?;
            let Some(number) = fields.next_value()? else {
                continue;
            };
            if number.identifier != INTEGER || number.contents != serial {
                continue;
            }
            let Some(date) = fields.next_value()? else {
                return Err(MaterialRefusal::DateUnreadable);
            };
            let revoked_at = x509::read_time(&date).ok_or(MaterialRefusal::DateUnreadable)?;
            let mut reason = None;
            let mut invalid_from = None;
            for extensions in std::iter::from_fn(|| fields.next_value().transpose()) {
                let extensions = extensions?;
                let (found, from) = entry_extensions(&extensions)?;
                reason = found;
                invalid_from = from;
            }
            return Ok(Some(RevokedCertificate {
                serial_number: number.contents,
                revoked_at,
                invalid_from,
                reason,
            }));
        }
        Ok(None)
    }
}

/// RFC 6960 section 4.2.1's `BasicOCSPResponse`, read for what a status decision needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicResponse<'a> {
    /// `tbsResponseData`'s own encoding, which is what the signature covers: "[t]he value for
    /// signature SHALL be computed on the hash of the DER encoding of ResponseData."
    #[expect(
        clippy::doc_markdown,
        reason = "the sentence quotes RFC 6960 section 4.2.1 verbatim, and a quotation is not marked up"
    )]
    pub tbs: &'a [u8],
    /// `ResponseData.responderID` as `byName`, where the responder named itself that way.
    pub responder_name: Option<&'a [u8]>,
    /// `ResponseData.responderID` as `byKey` — "SHA-1 hash of responder's public key (excluding
    /// the tag and length fields)".
    pub responder_key_hash: Option<&'a [u8]>,
    /// `ResponseData.producedAt`.
    pub produced_at: Instant,
    /// The statuses this response makes, bounded by [`MAX_SINGLE_RESPONSES`].
    pub responses: Vec<SingleResponse<'a>>,
    /// `BasicOCSPResponse.certs`, as encoded values, bounded by [`MAX_RESPONDER_CERTIFICATES`].
    pub certificates: Vec<Value<'a>>,
    /// `signatureAlgorithm`'s object identifier.
    pub signature_algorithm: &'a [u8],
    /// `signatureAlgorithm`'s `parameters`, where it states any.
    pub signature_parameters: Option<Value<'a>>,
    /// `signature`'s octets.
    pub signature: &'a [u8],
    /// Whether more than [`MAX_SINGLE_RESPONSES`] statuses were stated, so one may be unread.
    pub truncated: bool,
}

/// RFC 6960 section 4.2.1's `SingleResponse` — one certificate's status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SingleResponse<'a> {
    /// `certID.hashAlgorithm`'s object identifier.
    pub hash_algorithm: &'a [u8],
    /// `certID.issuerNameHash` — "Hash of issuer's DN".
    pub issuer_name_hash: &'a [u8],
    /// `certID.issuerKeyHash` — "Hash of issuer's public key".
    pub issuer_key_hash: &'a [u8],
    /// `certID.serialNumber`, as the file encodes the integer.
    pub serial_number: &'a [u8],
    /// `certStatus`.
    pub status: CertStatus,
    /// `thisUpdate`.
    pub this_update: Instant,
    /// `nextUpdate`, where the response states one.
    pub next_update: Option<Instant>,
}

/// RFC 6960 section 4.2.1's `CertStatus ::= CHOICE { good [0], revoked [1], unknown [2] }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CertStatus {
    /// `good [0] IMPLICIT NULL`.
    Good,
    /// `revoked [1] IMPLICIT RevokedInfo`.
    Revoked {
        /// `RevokedInfo.revocationTime`.
        at: Instant,
        /// `RevokedInfo.revocationReason`, where the responder states one.
        reason: Option<RevocationReason>,
    },
    /// `unknown [2] IMPLICIT UnknownInfo` — "the responder doesn't know about the certificate
    /// being requested" (section 2.2), which is not a statement that it is good.
    Unknown,
}

/// RFC 6960 section 4.2.1's `OCSPResponse`, read out of `bytes`.
///
/// # Errors
///
/// [`MaterialRefusal`], naming what the bytes are instead — including the two cases where the
/// response is well-formed and holds no status: a `responseStatus` that is not `successful`, and
/// a `responseType` this reader does not process.
pub fn ocsp_response(bytes: &[u8]) -> Result<BasicResponse<'_>, MaterialRefusal> {
    if let Some(rule) = der::is_canonical(bytes)? {
        return Err(MaterialRefusal::NotDerEncoded(rule));
    }
    let mut reader = Reader::new(bytes)?;
    let Some(outer) = reader.next_value()? else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    if outer.identifier != SEQUENCE {
        return Err(MaterialRefusal::NotAnOcspResponse);
    }
    let mut parts = outer.children()?;
    let Some(status) = parts.next_value()? else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    if status.identifier != ENUMERATED {
        return Err(MaterialRefusal::NotAnOcspResponse);
    }
    let code = status.contents.last().copied().unwrap_or(0xFF);
    if code != 0 {
        return Err(MaterialRefusal::ResponseNotSuccessful(code));
    }
    let Some(container) = parts.next_value()?.filter(|value| value.is_context(0)) else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    // `ResponseBytes ::= SEQUENCE { responseType OBJECT IDENTIFIER, response OCTET STRING }`,
    // inside the `[0] EXPLICIT`.
    let Some(response_bytes) = container.children()?.next_value()? else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    let mut members = response_bytes.children()?;
    let Some(response_type) = members
        .next_value()?
        .and_then(|oid| oid.object_identifier())
    else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    if response_type != OCSP_BASIC {
        return Err(MaterialRefusal::ResponseTypeNotBasic);
    }
    let Some(encapsulated) = members
        .next_value()?
        .filter(|value| value.identifier == OCTET_STRING)
    else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    basic_response(encapsulated.contents)
}

/// RFC 6960 section 4.2.1's `BasicOCSPResponse`, which the outer response's `response` member
/// encapsulates as an octet string.
fn basic_response(bytes: &[u8]) -> Result<BasicResponse<'_>, MaterialRefusal> {
    let mut reader = Reader::new(bytes)?;
    let Some(basic) = reader.next_value()? else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    if basic.identifier != SEQUENCE {
        return Err(MaterialRefusal::NotAnOcspResponse);
    }
    let mut parts = basic.children()?;
    let Some(tbs) = parts.next_value()? else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    if tbs.identifier != SEQUENCE {
        return Err(MaterialRefusal::NotAnOcspResponse);
    }
    let (signature_algorithm, signature_parameters) = match parts.next_value()? {
        Some(algorithm) if algorithm.identifier == SEQUENCE => {
            let mut members = algorithm.children()?;
            let oid = members
                .next_value()?
                .and_then(|oid| oid.object_identifier());
            (oid.unwrap_or(&[]), members.next_value()?)
        }
        _ => (&[][..], None),
    };
    let signature = match parts.next_value()? {
        Some(value) if value.identifier == BIT_STRING => bit_string_octets(&value).unwrap_or(&[]),
        _ => &[],
    };
    let mut certificates = Vec::new();
    if let Some(certs) = parts.next_value()?.filter(|value| value.is_context(0))
        && let Some(list) = certs.children()?.next_value()?
    {
        let mut entries = list.children()?;
        while let Some(entry) = entries.next_value()? {
            if certificates.len() >= MAX_RESPONDER_CERTIFICATES {
                break;
            }
            certificates.push(entry);
        }
    }
    let data = response_data(&tbs)?;
    Ok(BasicResponse {
        tbs: tbs.encoding(),
        responder_name: data.responder_name,
        responder_key_hash: data.responder_key_hash,
        produced_at: data.produced_at,
        responses: data.responses,
        certificates,
        signature_algorithm,
        signature_parameters,
        signature,
        truncated: data.truncated,
    })
}

/// RFC 6960 section 4.2.1's `ResponseData`, which is the half of a `BasicOCSPResponse` the
/// signature covers.
struct ResponseData<'a> {
    /// `responderID` as `byName`.
    responder_name: Option<&'a [u8]>,
    /// `responderID` as `byKey`.
    responder_key_hash: Option<&'a [u8]>,
    /// `producedAt`.
    produced_at: Instant,
    /// `responses`, bounded by [`MAX_SINGLE_RESPONSES`].
    responses: Vec<SingleResponse<'a>>,
    /// Whether that bound was reached with statuses left unread.
    truncated: bool,
}

/// `ResponseData ::= SEQUENCE { version [0] EXPLICIT Version DEFAULT v1, responderID, producedAt,
/// responses, responseExtensions [1] EXPLICIT Extensions OPTIONAL }`.
fn response_data<'a>(tbs: &Value<'a>) -> Result<ResponseData<'a>, MaterialRefusal> {
    // `ResponseData ::= SEQUENCE { version [0] EXPLICIT DEFAULT v1, responderID, producedAt,
    // responses, responseExtensions [1] EXPLICIT OPTIONAL }`.
    let mut members = tbs.children()?;
    let Some(first) = members.next_value()? else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    let responder = if first.is_context(0) {
        members.next_value()?
    } else {
        Some(first)
    };
    let Some(responder) = responder else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    // `ResponderID ::= CHOICE { byName [1] Name, byKey [2] KeyHash }`, both EXPLICIT.
    let (responder_name, responder_key_hash) = if responder.is_context(1) {
        (
            responder
                .children()?
                .next_value()?
                .map(|name| name.contents),
            None,
        )
    } else if responder.is_context(2) {
        (
            None,
            responder
                .children()?
                .next_value()?
                .map(|hash| hash.contents),
        )
    } else {
        (None, None)
    };
    let Some(produced_at) = members.next_value()? else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    let produced_at = x509::read_time(&produced_at).ok_or(MaterialRefusal::DateUnreadable)?;
    let Some(list) = members
        .next_value()?
        .filter(|value| value.identifier == SEQUENCE)
    else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    let mut responses = Vec::new();
    let mut truncated = false;
    let mut entries = list.children()?;
    while let Some(entry) = entries.next_value()? {
        if responses.len() >= MAX_SINGLE_RESPONSES {
            truncated = true;
            break;
        }
        responses.push(single_response(&entry)?);
    }
    Ok(ResponseData {
        responder_name,
        responder_key_hash,
        produced_at,
        responses,
        truncated,
    })
}

/// One `SingleResponse`.
fn single_response<'a>(entry: &Value<'a>) -> Result<SingleResponse<'a>, MaterialRefusal> {
    if entry.identifier != SEQUENCE {
        return Err(MaterialRefusal::NotAnOcspResponse);
    }
    let mut fields = entry.children()?;
    let Some(cert_id) = fields.next_value()?.filter(|v| v.identifier == SEQUENCE) else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    let mut parts = cert_id.children()?;
    let Some(hash_algorithm) = parts
        .next_value()?
        .filter(|v| v.identifier == SEQUENCE)
        .map(|v| v.children())
        .transpose()?
        .and_then(|mut members| members.next_value().ok().flatten())
        .and_then(|oid| oid.object_identifier())
    else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    let (Some(name_hash), Some(key_hash), Some(serial)) = (
        parts.next_value()?,
        parts.next_value()?,
        parts.next_value()?,
    ) else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    if name_hash.identifier != OCTET_STRING
        || key_hash.identifier != OCTET_STRING
        || serial.identifier != INTEGER
    {
        return Err(MaterialRefusal::NotAnOcspResponse);
    }
    let Some(status) = fields.next_value()? else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    let status = cert_status(&status)?;
    let Some(this_update) = fields.next_value()? else {
        return Err(MaterialRefusal::NotAnOcspResponse);
    };
    let this_update = x509::read_time(&this_update).ok_or(MaterialRefusal::DateUnreadable)?;
    let mut next_update = None;
    for member in std::iter::from_fn(|| fields.next_value().transpose()) {
        let member = member?;
        if member.is_context(0)
            && let Some(inner) = member.children()?.next_value()?
        {
            next_update = Some(x509::read_time(&inner).ok_or(MaterialRefusal::DateUnreadable)?);
        }
    }
    Ok(SingleResponse {
        hash_algorithm,
        issuer_name_hash: name_hash.contents,
        issuer_key_hash: key_hash.contents,
        serial_number: serial.contents,
        status,
        this_update,
        next_update,
    })
}

/// `CertStatus ::= CHOICE { good [0] IMPLICIT NULL, revoked [1] IMPLICIT RevokedInfo, unknown [2]
/// IMPLICIT UnknownInfo }` — implicit, so the tag is the whole of what distinguishes the three.
fn cert_status(value: &Value<'_>) -> Result<CertStatus, MaterialRefusal> {
    if value.is_context(0) {
        return Ok(CertStatus::Good);
    }
    if value.is_context(2) {
        return Ok(CertStatus::Unknown);
    }
    if !value.is_context(1) {
        return Err(MaterialRefusal::NotAnOcspResponse);
    }
    // `RevokedInfo ::= SEQUENCE { revocationTime GeneralizedTime, revocationReason [0] EXPLICIT
    // CRLReason OPTIONAL }`, whose SEQUENCE tag the IMPLICIT [1] replaced.
    let mut members = value.children()?;
    let Some(time) = members.next_value()? else {
        return Err(MaterialRefusal::DateUnreadable);
    };
    let at = x509::read_time(&time).ok_or(MaterialRefusal::DateUnreadable)?;
    let mut reason = None;
    for member in std::iter::from_fn(|| members.next_value().transpose()) {
        let member = member?;
        if member.is_context(0)
            && let Some(inner) = member.children()?.next_value()?
            && inner.identifier == ENUMERATED
        {
            reason = inner
                .contents
                .last()
                .copied()
                .map(RevocationReason::from_code);
        }
    }
    Ok(CertStatus::Revoked { at, reason })
}

/// A CRL entry's `reasonCode` and `invalidityDate` extensions, where its `crlEntryExtensions`
/// state them.
///
/// Both in one walk rather than two, because the bound on how many extensions are read is a bound
/// on the walk and a second pass would double it for one more field.
fn entry_extensions(
    extensions: &Value<'_>,
) -> Result<(Option<RevocationReason>, Option<Instant>), MaterialRefusal> {
    /// `id-ce-invalidityDate`, `{ id-ce 24 }` — 2.5.29.24.
    const INVALIDITY_DATE: &[u8] = &[0x55, 0x1D, 0x18];

    let mut reason = None;
    let mut invalid_from = None;
    let mut entries = extensions.children()?;
    let mut seen = 0usize;
    while let Some(entry) = entries.next_value()? {
        seen = seen.saturating_add(1);
        if seen > MAX_EXTENSIONS {
            break;
        }
        let Some(extension) = extension_parts(&entry)? else {
            continue;
        };
        let Some(inner) = Reader::new(extension.value)?.next_value()? else {
            continue;
        };
        if extension.id == CRL_REASONS && inner.identifier == ENUMERATED {
            reason = inner
                .contents
                .last()
                .copied()
                .map(RevocationReason::from_code);
        } else if extension.id == INVALIDITY_DATE {
            // Section 5.3.2 fixes the encoding — the value "MUST be expressed in Greenwich Mean
            // Time (Zulu)" and read as section 4.1.2.5.2 defines — which is what `read_time` does.
            // An unreadable one is left absent rather than refused: the entry still revokes the
            // certificate, and the only thing lost is the earlier of two instants.
            invalid_from = x509::read_time(&inner);
        }
    }
    Ok((reason, invalid_from))
}

/// The first critical `crlExtensions` entry this reader does not recognise.
///
/// Two are recognised because a path validator reads them elsewhere and neither changes a scope:
/// `cRLNumber` (section 5.2.3) and `authorityKeyIdentifier` (section 5.2.1). Everything else
/// critical — `issuingDistributionPoint` above all — is reported, and the module comment quotes
/// section 5.2.5 for what a reader then owes.
fn unrecognised_critical<'a>(container: &Value<'a>) -> Result<Option<&'a [u8]>, MaterialRefusal> {
    /// `id-ce-cRLNumber`, `{ id-ce 20 }` — 2.5.29.20.
    const CRL_NUMBER: &[u8] = &[0x55, 0x1D, 0x14];
    /// `id-ce-authorityKeyIdentifier`, `{ id-ce 35 }` — 2.5.29.35.
    const AUTHORITY_KEY_IDENTIFIER: &[u8] = &[0x55, 0x1D, 0x23];
    let Some(list) = container.children()?.next_value()? else {
        return Ok(None);
    };
    let mut entries = list.children()?;
    let mut seen = 0usize;
    while let Some(entry) = entries.next_value()? {
        seen = seen.saturating_add(1);
        if seen > MAX_EXTENSIONS {
            // A walk that stopped early cannot claim there was no critical extension it did not
            // recognise, so it reports one — with the identifier of the extension list itself,
            // which is the only name there is for "one of these, unread".
            return Ok(Some(&[]));
        }
        let Some(extension) = extension_parts(&entry)? else {
            continue;
        };
        if extension.critical
            && extension.id != CRL_NUMBER
            && extension.id != AUTHORITY_KEY_IDENTIFIER
        {
            return Ok(Some(extension.id));
        }
    }
    Ok(None)
}

/// `Extension ::= SEQUENCE { extnID OBJECT IDENTIFIER, critical BOOLEAN DEFAULT FALSE, extnValue
/// OCTET STRING }`, read.
struct Extension<'a> {
    /// `extnID`.
    id: &'a [u8],
    /// `critical`, defaulted to false where the encoding omits it.
    critical: bool,
    /// `extnValue`'s octets, which for every extension here hold a further DER encoding.
    value: &'a [u8],
}

/// One `Extension`, or `None` where the entry is not one.
fn extension_parts<'a>(entry: &Value<'a>) -> Result<Option<Extension<'a>>, MaterialRefusal> {
    if entry.identifier != SEQUENCE {
        return Ok(None);
    }
    let mut parts = entry.children()?;
    let Some(oid) = parts.next_value()?.and_then(|id| id.object_identifier()) else {
        return Ok(None);
    };
    let mut critical = false;
    let mut value: &[u8] = &[];
    for part in std::iter::from_fn(|| parts.next_value().transpose()) {
        let part = part?;
        match part.identifier {
            // X.690 clause 11.1 makes `FF` the only DER `TRUE`; any other non-zero octet is read
            // as true too, which is the conservative direction — true is what binds.
            BOOLEAN => critical = part.contents.iter().any(|&octet| octet != 0),
            OCTET_STRING => value = part.contents,
            _ => {}
        }
    }
    Ok(Some(Extension {
        id: oid,
        critical,
        value,
    }))
}

/// A `BIT STRING`'s value octets, where it states no unused trailing bits.
fn bit_string_octets<'a>(bits: &Value<'a>) -> Option<&'a [u8]> {
    let (&unused, rest) = bits.contents.split_first()?;
    (unused == 0).then_some(rest)
}

/// Whether an `ExtKeyUsageSyntax` asserts `id-kp-OCSPSigning`.
///
/// RFC 6960 section 4.2.2.2: "OCSP signing delegation SHALL be designated by the inclusion of
/// id-kp-OCSPSigning in an extended key usage certificate extension included in the OCSP response
/// signer's certificate." `anyExtendedKeyUsage` is accepted with it, on RFC 5280 section 4.2.1.12's
/// own terms — the identifier exists so that "a CA … [may] indicate that the certificate may be
/// used for all key purposes".
fn asserts_ocsp_signing(encoded: &[u8]) -> bool {
    x509::indicates_purpose(encoded, OCSP_SIGNING)
}

/// The revocation material one document carries, read once and applied many times.
///
/// Built from §12.8.4.3's `/CRLs` and `/OCSPs` by [`crate::signature::SecurityStore`]; the bytes
/// stay the caller's, which is what keeps this a view over the file rather than a copy of it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Material<'a> {
    /// The CRLs that read.
    pub lists: Vec<CertificateList<'a>>,
    /// The OCSP responses that read.
    pub responses: Vec<BasicResponse<'a>>,
    /// What would not read, by name — one entry per refused stream, in the order the arrays gave
    /// them.
    pub refused: Vec<MaterialRefusal>,
}

impl<'a> Material<'a> {
    /// Nothing: the state of every caller in this tree before this module existed.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            lists: Vec::new(),
            responses: Vec::new(),
            refused: Vec::new(),
        }
    }

    /// The material `crls` and `ocsps` hold, with whatever would not read named rather than
    /// dropped.
    #[must_use]
    pub fn read(crls: &[&'a [u8]], ocsps: &[&'a [u8]]) -> Self {
        let mut material = Self::none();
        for &bytes in crls {
            match certificate_list(bytes) {
                Ok(list) => material.lists.push(list),
                Err(refusal) => material.refused.push(refusal),
            }
        }
        for &bytes in ocsps {
            match ocsp_response(bytes) {
                Ok(response) => material.responses.push(response),
                Err(refusal) => material.refused.push(refusal),
            }
        }
        material
    }

    /// Whether there is nothing to check against.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lists.is_empty() && self.responses.is_empty()
    }

    /// Everything `other` holds, added to this.
    ///
    /// What lets §12.8.4's store and §12.8.3.3.2's attribute be one supply: the clauses put the
    /// material in two places and a verifier needs whichever of them the producer used.
    pub fn absorb(&mut self, mut other: Self) {
        self.lists.append(&mut other.lists);
        self.responses.append(&mut other.responses);
        self.refused.append(&mut other.refused);
    }
}

/// §12.8.3.3.2's `RevocationInfoArchival`, read out of a signer's signed attributes.
///
/// **The second supply, and the standard prints its whole grammar.** §12.8.3.3.2 states the
/// identifier:
///
/// > adbe-revocationInfoArchival OBJECT IDENTIFIER::= {adbe(1.2.840.113583) acrobat(1)
/// > security(1) 8}
///
/// And then the type:
///
/// > RevocationInfoArchival::= SEQUENCE { crl [0] EXPLICIT SEQUENCE of CRLs, OPTIONAL ocsp [1]
/// > EXPLICIT SEQUENCE of OCSPResponse, OPTIONAL otherRevInfo [2] EXPLICIT SEQUENCE of
/// > OtherRevInfo, OPTIONAL }
///
/// So this is not a structure whose shape had to be guessed at, which is why it is read here and
/// why the ledger row that said "reading them would be inventing a validator" no longer holds.
/// `otherRevInfo` is skipped and the clause says why nothing is lost by it: "[t]he format is not
/// prescribed by this specification, other than that it be encoded as an OCTET STRING."
///
/// The clause also bounds where it applies: "[f]or signatures with a SubFilter value other than
/// ETSI.CAdES.detached, the following rules apply." That is a rule about which signatures a
/// producer writes it on, and this reads what the file states either way — a signature stating
/// that subfilter and carrying the attribute anyway is a file to report rather than material to
/// discard.
#[must_use]
#[expect(
    clippy::doc_markdown,
    reason = "the sentences above quote §12.8.3.3.2 verbatim, and a quotation is not marked up"
)]
pub fn archived<'a>(signed: &SignedData<'a>) -> Material<'a> {
    let mut material = Material::none();
    let Some(attributes) = signed.signed_attributes else {
        return material;
    };
    let Ok(mut reader) = Reader::new(attributes) else {
        return material;
    };
    let mut seen = 0usize;
    while let Ok(Some(attribute)) = reader.next_value() {
        seen = seen.saturating_add(1);
        if seen > crate::cms::MAX_ATTRIBUTES {
            break;
        }
        // `Attribute ::= SEQUENCE { attrType OBJECT IDENTIFIER, attrValues SET OF AttributeValue }`.
        let Ok(mut parts) = attribute.children() else {
            continue;
        };
        let Ok(Some(kind)) = parts.next_value() else {
            continue;
        };
        if kind.object_identifier() != Some(ADBE_REVOCATION_INFO_ARCHIVAL) {
            continue;
        }
        let Ok(Some(values)) = parts.next_value() else {
            continue;
        };
        let Ok(mut set) = values.children() else {
            continue;
        };
        while let Ok(Some(archival)) = set.next_value() {
            read_archival(&archival, &mut material);
        }
    }
    material
}

/// One `RevocationInfoArchival` value's two readable members.
fn read_archival<'a>(archival: &Value<'a>, material: &mut Material<'a>) {
    let Ok(mut members) = archival.children() else {
        return;
    };
    while let Ok(Some(member)) = members.next_value() {
        let (crls, ocsps) = (member.is_context(0), member.is_context(1));
        if !crls && !ocsps {
            continue;
        }
        let Ok(mut list) = member.children() else {
            continue;
        };
        while let Ok(Some(entry)) = list.next_value() {
            if material
                .lists
                .len()
                .saturating_add(material.responses.len())
                >= MAX_ARCHIVED
            {
                return;
            }
            // The whole encoding rather than the contents: what the issuer signed is the structure
            // as the file wrote it, and [`certificate_list`] and [`ocsp_response`] each begin by
            // asking whether that encoding is DER.
            let bytes = entry.encoding();
            let read = if crls {
                certificate_list(bytes).map(|list| material.lists.push(list))
            } else {
                ocsp_response(bytes).map(|response| material.responses.push(response))
            };
            if let Err(refusal) = read {
                material.refused.push(refusal);
            }
        }
    }
}

/// How many structures one `RevocationInfoArchival` attribute may contribute.
///
/// §12.8.3.3.2 states no ceiling and warns in the other direction — "CRLs can be large and
/// therefore require more pre-allocated space in the value of the Contents key" — so this is a
/// bound on work over a stranger's bytes. A certification path is at most
/// [`crate::trust::MAX_PATH_LENGTH`] certificates and each needs one structure, so this is two
/// orders of magnitude past what a validation can consume.
pub const MAX_ARCHIVED: usize = 256;

/// One certificate on a path, with everything about its issuer that a status decision needs.
///
/// Assembled by [`crate::trust`] as it walks a path it has already validated, which is what makes
/// RFC 5280 section 6.3.3 (f) cheap here: "[o]btain and validate the certification path for the
/// issuer of the complete CRL. The trust anchor for the certification path MUST be the same as the
/// trust anchor used to validate the target certificate." Under the assumed distribution point the
/// module comment quotes, the CRL issuer *is* the certificate issuer, whose path is the tail of
/// the one being validated — so it has been validated to the same anchor already, and what is left
/// of the step is the `cRLSign` check.
#[derive(Debug, Clone, Copy)]
pub struct Subject<'a, 'c> {
    /// The certificate whose status is asked about.
    pub certificate: &'a Certificate<'c>,
    /// The issuer's `Name` as encoded, header included — what RFC 6960's `issuerNameHash` hashes.
    pub issuer_name: &'c [u8],
    /// The issuer's `subjectPublicKey` octets, "excluding the tag and length fields" — what
    /// `issuerKeyHash` hashes.
    pub issuer_key_bits: &'c [u8],
    /// The issuer's public key, which signs any CRL it issues.
    pub issuer_key: PublicKey<'c>,
    /// The issuer's `keyUsage`, where the issuer is a certificate on the path rather than an
    /// anchor a host named without one.
    pub issuer_key_usage: Option<KeyUsage>,
    /// How far down the path this certificate is, the target being 0.
    pub position: usize,
}

/// The revocation status of one certificate, from the material this document carries.
///
/// Never [`Revocation::NotChecked`]: that variant is about a *caller* that asked nothing, and this
/// function was asked. The answer for a path is the worst of these, which [`worst`] composes.
#[must_use]
pub fn status(subject: &Subject<'_, '_>, material: &Material<'_>, at: Instant) -> Revocation {
    if material.is_empty() {
        return Revocation::Unknown {
            why: Undetermined::NoMaterial,
            position: subject.position,
        };
    }
    let mut best: Option<Undetermined> = None;
    let mut note = |why: Undetermined| {
        // The first reason rather than the newest, for the reason `trust::Search` gives about its
        // own: the answer must not depend on the order the document happened to list its material.
        if best.is_none() {
            best = Some(why);
        }
    };
    for response in &material.responses {
        match from_response(subject, response, material, at) {
            Ok(Some(answer)) => return answer,
            Ok(None) => {}
            Err(why) => note(why),
        }
    }
    for list in &material.lists {
        match from_list(subject, list, at) {
            Ok(Some(answer)) => return answer,
            Ok(None) => {}
            Err(why) => note(why),
        }
    }
    Revocation::Unknown {
        why: best.unwrap_or(Undetermined::NotCovered),
        position: subject.position,
    }
}

/// The worse of two answers, which is how a path's status is composed from its certificates'.
///
/// The order is `Revoked` worst, then `Unknown`, then `Good`, then `NotChecked` — and `NotChecked`
/// is last because it is a statement about the caller rather than about the path, so any real
/// answer displaces it.
#[must_use]
pub fn worst(left: Revocation, right: Revocation) -> Revocation {
    let rank = |answer: &Revocation| match *answer {
        Revocation::Revoked { .. } => 3u8,
        Revocation::Unknown { .. } => 2,
        Revocation::Good { .. } => 1,
        Revocation::NotChecked => 0,
    };
    if rank(&right) > rank(&left) {
        right
    } else {
        left
    }
}

/// What one CRL says about one certificate, or `Ok(None)` where it says nothing about it.
fn from_list(
    subject: &Subject<'_, '_>,
    list: &CertificateList<'_>,
    at: Instant,
) -> Result<Option<Revocation>, Undetermined> {
    // Section 6.3.3 (b)(1), under the assumed distribution point: "verify that the CRL issuer
    // matches the certificate issuer."
    if list.issuer != subject.certificate.issuer {
        return Ok(None);
    }
    // Section 6.3.3 (f)'s second sentence: "If a key usage extension is present in the CRL
    // issuer's certificate, verify that the cRLSign bit is set."
    if subject
        .issuer_key_usage
        .is_some_and(|usage| !usage.crl_sign())
    {
        return Err(Undetermined::KeyUsageForbidsCrlSigning);
    }
    // Section 6.3.3 (g): "Validate the signature on the complete CRL using the public key
    // validated in step (f)."
    if !verify(
        list.tbs,
        list.signature_algorithm,
        list.signature_parameters,
        list.signature,
        subject.issuer_key,
    )
    .map_err(Undetermined::AlgorithmNotVerifiable)?
    {
        return Err(Undetermined::NotUnderIssuersKey);
    }
    // Section 6.3.3 (j), before the freshness test: a revocation is a statement its issuer made,
    // and the window on the *list* does not withdraw it. ADR 1067 section 3.
    let entry =
        list.entry_for(subject.certificate.serial_number)
            .map_err(|refusal| match refusal {
                MaterialRefusal::Unreadable(DerError::TooLarge) => Undetermined::TooLarge,
                other => Undetermined::Unreadable(other),
            })?;
    if let Some(entry) = entry {
        // Section 6.3.3 (k): "If (cert_status is removeFromCRL), then set cert_status to
        // UNREVOKED." Which leaves this list saying nothing about the certificate — the entry
        // exists to withdraw an earlier one — so the search goes on.
        if entry.reason == Some(RevocationReason::RemoveFromCrl) {
            return Ok(None);
        }
        return Ok(Some(Revocation::Revoked {
            at: entry.revoked_at,
            invalid_from: entry.invalid_from,
            reason: entry.reason,
            from: Evidence::CertificateRevocationList,
            position: subject.position,
        }));
    }
    // Section 5.2.5, quoted in the module comment: an unsupported critical extension makes "the
    // status of any certificate not listed on this CRL" unknown. Checked *here* rather than at the
    // top, because a certificate the list does name is named whatever its scope says.
    if let Some(oid) = list.unrecognised_critical {
        return Err(Undetermined::UnrecognisedCriticalExtension(
            x509::dotted(oid).unwrap_or_else(|| "an unreadable object identifier".to_owned()),
        ));
    }
    // Section 6.3.3 (a)(1): "If the current time is after the value of the CRL next update field"
    // the list is to be replaced, which needs the network this program does not have. Section
    // 5.1.2.5 requires a conforming issuer to state one, and says the behaviour without it "is not
    // specified by this profile" — so an absent `nextUpdate` is undetermined rather than eternal.
    let Some(next_update) = list.next_update else {
        return Err(Undetermined::Stale);
    };
    // **Only one end of the window is a freshness test, and the other end was a defect.** ETSI
    // EN 319 102-1 clause 5.2.5 states the rule as one comparison: with no configured maximum, the
    // accepted freshness is the interval between `thisUpdate` and `nextUpdate`, and the list is
    // fresh where its issuance is no earlier than the validation time minus that interval — which
    // reduces to `nextUpdate` being after the instant asked about. A list issued *after* the
    // instant is not stale by that rule but fresher than one issued before it, and the note under it
    // says why: where the signing time is known, a checker set to demand zero staleness accepts
    // revocation data only if it was issued after that moment. Refusing such a list is what
    // would make §12.8.4's whole construction useless — a document security store is assembled
    // after signing, so every list in one postdates the signature it is evidence about.
    if at.unix_seconds() > next_update.unix_seconds() {
        return Err(Undetermined::Stale);
    }
    Ok(Some(Revocation::Good {
        from: Evidence::CertificateRevocationList,
        covered: 1,
    }))
}

/// What one OCSP response says about one certificate, or `Ok(None)` where it says nothing.
fn from_response(
    subject: &Subject<'_, '_>,
    response: &BasicResponse<'_>,
    material: &Material<'_>,
    at: Instant,
) -> Result<Option<Revocation>, Undetermined> {
    let Some(single) = response
        .responses
        .iter()
        .find(|single| names(single, subject))
    else {
        return Ok(None);
    };
    // RFC 6960 section 4.2.2.2, criteria 2 and 3: the response must have been signed by the CA
    // that issued the certificate in question, or by a certificate that CA issued which asserts
    // `id-kp-OCSPSigning`. Criterion 1 — "a local configuration of OCSP signing authority" — is a
    // host's input this program has no route for, the same shape as a trust anchor (ADR 1039).
    authorised(subject, response, material)?;
    match single.status {
        CertStatus::Revoked { at: when, reason } => Ok(Some(Revocation::Revoked {
            at: when,
            invalid_from: None,
            reason,
            from: Evidence::OcspResponse,
            position: subject.position,
        })),
        // Section 2.2: "unknown" indicates "that the responder doesn't know about the certificate
        // being requested", which is not a statement that it is good.
        CertStatus::Unknown => Err(Undetermined::NotCovered),
        CertStatus::Good => {
            // **The window is one-ended where the responder states one, and the other end is where
            // the two cases differ.** Section 2.4: "If nextUpdate is not set, the responder is
            // indicating that newer revocation information is available all the time." So:
            //
            // - with a `nextUpdate`, freshness is ETSI EN 319 102-1 clause 5.2.5's single
            //   comparison against it — `from_list` carries that argument — and a response produced
            //   *after* the instant asked about is evidence about it rather than stale;
            // - without one the responder has said its answer speaks for the moment it was made, so
            //   the instant it was made is the only end this reader can state, and a question put
            //   before it is not one this response answers.
            match single.next_update {
                Some(until) => {
                    if at.unix_seconds() > until.unix_seconds() {
                        return Err(Undetermined::Stale);
                    }
                }
                None => {
                    if at.unix_seconds() < single.this_update.unix_seconds() {
                        return Err(Undetermined::Stale);
                    }
                }
            }
            Ok(Some(Revocation::Good {
                from: Evidence::OcspResponse,
                covered: 1,
            }))
        }
    }
}

/// Whether a `SingleResponse`'s `CertID` names this certificate — RFC 6960 section 4.1.1's three
/// fields, all three of them.
fn names(single: &SingleResponse<'_>, subject: &Subject<'_, '_>) -> bool {
    if single.serial_number != subject.certificate.serial_number {
        return false;
    }
    let Some(digest) = Digest::from_oid(single.hash_algorithm) else {
        return false;
    };
    digest.compute(&[subject.issuer_name]) == single.issuer_name_hash
        && digest.compute(&[subject.issuer_key_bits]) == single.issuer_key_hash
}

/// RFC 6960 section 4.2.2.2's authorisation check, and the signature that goes with it.
fn authorised(
    subject: &Subject<'_, '_>,
    response: &BasicResponse<'_>,
    material: &Material<'_>,
) -> Result<(), Undetermined> {
    // Criterion 2: the CA that issued the certificate in question signed the response itself.
    //
    // **A failure here is not the end of the search**, and the distinction cost this module a
    // wrong answer on two real documents before it was written down: a response signed by a
    // delegate has a signature the CA's key cannot even be shaped against, so the arithmetic
    // refuses it by *name* rather than by returning false — and a `?` there would report "this
    // program does not verify 1.2.840.113549.1.1.11", which is untrue twice over. Criterion 3 is
    // below and has to be reached.
    let mut refusal = Undetermined::ResponderNotAuthorised;
    match verify(
        response.tbs,
        response.signature_algorithm,
        response.signature_parameters,
        response.signature,
        subject.issuer_key,
    ) {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        Err(named) => refusal = Undetermined::AlgorithmNotVerifiable(named),
    }
    // Criterion 3: a delegation. The responder's certificate is one the response carried, or one
    // the document's own store carried; either way it must be issued by the same CA — "[s]ystems
    // relying on OCSP responses MUST recognize a delegation certificate as being issued by the CA
    // that issued the certificate in question only if the delegation certificate and the
    // certificate being checked for revocation were signed by the same key" — and must assert
    // `id-kp-OCSPSigning`.
    for candidate in &response.certificates {
        let Ok(responder) = x509::read(*candidate) else {
            continue;
        };
        if responder.indefinite_lengths || responder.issuer != subject.certificate.issuer {
            continue;
        }
        let Some(usage) = responder.extensions.extended_key_usage else {
            continue;
        };
        if !asserts_ocsp_signing(usage) {
            continue;
        }
        // The delegation certificate is itself signed by the CA, which is what makes it a
        // delegation rather than a claim; and the response is signed by the delegate.
        match verify(
            responder.tbs,
            responder.signature_algorithm,
            responder.signature_parameters,
            responder.signature,
            subject.issuer_key,
        ) {
            Ok(true) => {}
            // Not this CA's delegate, or a delegation this program cannot check: either way the
            // next candidate gets its turn, for the reason criterion 2 above carries.
            Ok(false) => continue,
            Err(named) => {
                refusal = Undetermined::AlgorithmNotVerifiable(named);
                continue;
            }
        }
        match verify(
            response.tbs,
            response.signature_algorithm,
            response.signature_parameters,
            response.signature,
            responder.public_key,
        ) {
            Ok(true) => return Ok(()),
            Ok(false) => refusal = Undetermined::NotUnderIssuersKey,
            Err(named) => refusal = Undetermined::AlgorithmNotVerifiable(named),
        }
    }
    // Nothing in `material` is consulted for a responder certificate: §12.8.4.3's `/Certs` is "an
    // array of all certificates used for the signatures", and a certificate that is not inside the
    // response is not one the responder offered. Naming the field keeps the reduction visible
    // rather than making it look like an oversight.
    let _ = material;
    Err(refusal)
}

/// One signature over one message, under one key — RFC 5280 section 6.1.3 (a)(1)'s arithmetic,
/// reached from three places rather than one.
///
/// `Err` names the algorithm identifier, which is what a report can print; `Ok(false)` is the
/// arithmetic saying no.
fn verify(
    message: &[u8],
    algorithm: &[u8],
    parameters: Option<Value<'_>>,
    signature: &[u8],
    key: PublicKey<'_>,
) -> Result<bool, String> {
    let named =
        || x509::dotted(algorithm).unwrap_or_else(|| "an unreadable object identifier".to_owned());
    match (crate::cms::SignatureAlgorithm::from_oid(algorithm), key) {
        (crate::cms::SignatureAlgorithm::RsaPkcs1V15, PublicKey::Rsa(key)) => {
            let digest = x509::signature_digest(algorithm).ok_or_else(named)?;
            let computed = digest.compute(&[message]);
            pkcs1::verify(key, signature, digest, &computed).map_err(|_| named())
        }
        (crate::cms::SignatureAlgorithm::RsaPss, PublicKey::Rsa(key)) => {
            let parameters = pss::parameters(parameters).map_err(|_| named())?;
            let computed = parameters.hash.compute(&[message]);
            pss::verify(key, signature, parameters, &computed).map_err(|_| named())
        }
        (crate::cms::SignatureAlgorithm::Dsa, PublicKey::Dsa(key)) => {
            let digest = x509::signature_digest(algorithm).ok_or_else(named)?;
            let computed = digest.compute(&[message]);
            dsa::verify(key, signature, &computed).map_err(|_| named())
        }
        (crate::cms::SignatureAlgorithm::Ecdsa, PublicKey::Ec(key)) => {
            let digest = x509::signature_digest(algorithm).ok_or_else(named)?;
            let computed = digest.compute(&[message]);
            ecdsa::verify(key, signature, &computed).map_err(|_| named())
        }
        (crate::cms::SignatureAlgorithm::EdDsa, PublicKey::Ed25519(key)) => {
            // RFC 8032 signs the message rather than a digest of it.
            eddsa::verify(key, signature, &[message]).map_err(|_| named())
        }
        _ => Err(named()),
    }
}

#[cfg(test)]
mod tests;

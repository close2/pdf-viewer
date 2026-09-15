//! RFC 3161's time-stamp token, and ISO 32000-2 §12.8.5's chain of them over a document.
//!
//! §12.8.5.2 says what a document timestamp's `/Contents` holds — "[t]he timestamp token shall be
//! an Internet RFC 3161 TimeStampToken, as updated by Internet RFC 5816 , obtained from a trusted
//! timestamp authority" — and RFC 3161 section 2.4.2 says what that is: `TimeStampToken ::=
//! ContentInfo`, a CMS `SignedData` whose encapsulated content is a `TSTInfo`. So a document
//! timestamp is read by [`crate::cms`] like any other signature, and what is left over — the
//! `TSTInfo` itself, and what a sequence of them says about a document — is this module.
//!
//! # A time is established, never read
//!
//! `genTime` is a number a stranger wrote in a file. Four things have to hold before this program
//! will call it an instant, and [`Time`] carries each of their failures by name:
//!
//! 1. the `/Contents` reads as a `SignedData` over a `TSTInfo` ([`tst_info`]);
//! 2. the signer's `message-digest` attribute is the digest of that `TSTInfo`, which is what binds
//!    the token's contents to the signature over it — RFC 5652 section 5.6: "For the signature to
//!    be valid, the message digest value calculated by the recipient MUST be the same as the value
//!    of the messageDigest attribute included in the signedAttributes of the SignedData
//!    signerInfo";
//! 3. that signature verifies under the key in the certificate the token carries
//!    ([`crate::signature::Signature::authenticity`]);
//! 4. a certification path from that certificate reaches an anchor somebody supplied
//!    ([`crate::trust`]), with §12.8.4's revocation material applied to every certificate on it.
//!
//! Step 4 is [`crate::trust::Trust::NoAnchorSupplied`] in this tree, because no host here names an
//! anchor (ADR 1039). So every real document's timestamp is [`Time::Unknown`], and that is the
//! honest answer rather than a gap: §12.8.5.2's authority is a *trusted* one, and nothing here
//! decides whom to trust. What the token *claims* is still reported, as a claim ([`Claim`]).
//!
//! # The chain, and why it is an order rather than a list
//!
//! §12.8.5.3 is the reason a document carries more than one: a token "may expire due to the expiry
//! of its certificate or the cryptographic strength of some of their algorithms may not be
//! resistant any longer to some new cryptographic attack", so a later one is applied over the
//! whole structure — the earlier token, and the material proving the earlier authority's
//! certificates were current when it was applied. "The certificates, CRLs or OCSP responses used
//! to demonstrate that the certificates related to the certification path of the previous
//! timestamp token was not revoked for a reason which is either \"keyCompromise\" or
//! \"cACompromise\" just before the time the new timestamp token has been requested shall be
//! included into the DSS dictionary."
//!
//! That makes the sequence load-bearing in one specific place: **the instant at which one
//! timestamp's own path is validated is the next timestamp's `genTime`**, where there is a next
//! one and it is itself established. Validating a token's certificate at the token's own `genTime`
//! would be asking the file to vouch for itself; ADR 1071 section 2 is that argument, and
//! [`AskedAt`] is where a reader can see which of the two an answer used.
//!
//! [`chain`] establishes the order from the byte ranges, and refuses by name where a range does
//! not cover what §12.8.5.3 needs it to ([`ChainRefusal`]).

#![expect(
    clippy::doc_markdown,
    reason = "RFC 3161's and RFC 5652's sentences are quoted verbatim and their ASN.1 names are \
              camel case throughout; a quotation with backticks added to please a lint is no \
              longer a quotation (the same reasoning `pdf_signature::ecdsa` records)"
)]

use std::collections::BTreeMap;

use pdf_syntax::{Document, FileBytes, xref::Location};

use crate::cms::{Digest, ID_CT_TST_INFO, SignedData};
use crate::der::{self, DerError, INTEGER, OBJECT_IDENTIFIER, OCTET_STRING, Reader, SEQUENCE};
use crate::revocation::{Material, Revocation};
use crate::signature::{self, Authenticity, Integrity, Signature, SignedEnd};
use crate::trust::{Purpose, Trust, TrustAnchors};
use crate::x509::{self, Instant};

/// The largest `serialNumber` this reader takes, in octets.
///
/// RFC 3161 section 2.4.2 states the number a reader must accommodate and this is it: the comment
/// beside `serialNumber` says time-stamping users must be ready to accommodate integers up to 160
/// bits. Twenty octets is 160 bits, and one
/// more is the leading zero DER prepends to keep a positive integer positive. A longer one is
/// refused by name rather than truncated, because a serial number is an identifier and half of one
/// identifies nothing (trap 38).
pub const MAX_SERIAL_OCTETS: usize = 21;

/// How many document timestamps one document's chain is walked.
///
/// Neither §12.8.5 nor §12.8.5.3 states a ceiling — "[t]he process may be repeated several times
/// as long as there is a need to reverify the signatures included in the document" — so this is a
/// bound on work over a stranger's file and not a reading of the standard. It sits well under the
/// bound `crate::signature::signatures` already puts on the walk that finds them.
pub const MAX_TIMESTAMPS: usize = 64;

/// What stopped a `TSTInfo` from being read.
///
/// Every variant is a refusal by name, which is what `CLAUDE.md` principle 1 asks of a parser over
/// untrusted input: a token this reader half understood would otherwise become a time.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TokenRefusal {
    /// The signature value is not a CMS object at all, or carries no encapsulated content.
    #[error("the timestamp carries no RFC 3161 TSTInfo")]
    NoEncapsulatedContent,
    /// `eContentType` is not `id-ct-TSTInfo`.
    ///
    /// RFC 3161 section 2.4.2 names it: "For a time-stamp token it is defined as: id-ct-TSTInfo".
    #[error("the encapsulated content is not an RFC 3161 TSTInfo")]
    ContentTypeIsNotTstInfo,
    /// The content states X.690's indefinite length, which DER forbids.
    ///
    /// RFC 3161 section 2.4.2 requires DER of this one structure by name — "[t]he eContent SHALL be
    /// the DER-encoded value of TSTInfo" — so an extent found by scanning for an end-of-contents
    /// marker is not the extent the authority signed. Deliberately narrower than [`crate::der`]'s
    /// tolerance, which exists because the *enclosing* CMS object is BER in real files.
    #[error("the TSTInfo is written with indefinite lengths, which DER forbids")]
    NotDerEncoded,
    /// The ASN.1 would not read.
    #[error("the TSTInfo is not readable ASN.1: {0}")]
    Unreadable(#[from] DerError),
    /// The outer shape is not RFC 3161 section 2.4.2's `TSTInfo ::= SEQUENCE`.
    #[error("the TSTInfo is not the SEQUENCE RFC 3161 section 2.4.2 defines")]
    NotATstInfo,
    /// `messageImprint`'s `hashAlgorithm` is not one of the ten this program computes.
    #[error("the TSTInfo states a message imprint digest this program does not compute")]
    ImprintDigestUnknown,
    /// `serialNumber` is longer than [`MAX_SERIAL_OCTETS`].
    #[error("the TSTInfo states a serial number longer than {MAX_SERIAL_OCTETS} octets")]
    SerialNumberTooLong,
    /// `genTime` is not a `GeneralizedTime` this reader can place on a line.
    #[error("the TSTInfo's genTime is not a time this reader can place")]
    GenTimeUnreadable,
    /// `accuracy` states a `millis` or `micros` outside the range RFC 3161 section 2.4.2 admits.
    ///
    /// The bounds are the grammar's own: "millis [0] INTEGER (1..999) OPTIONAL, micros [1]
    /// INTEGER (1..999) OPTIONAL".
    #[error("the TSTInfo states an accuracy outside RFC 3161's range")]
    AccuracyOutOfRange,
}

/// RFC 3161 section 2.4.2's `Accuracy`, in its own three fields.
///
/// The clause says what adding it to `genTime` buys: "By adding the accuracy value to the
/// GeneralizedTime, an upper limit of the time at which the time-stamp token has been created by
/// the TSA can be obtained. In the same way, by subtracting the accuracy to the GeneralizedTime, a
/// lower limit of the time at which the time-stamp token has been created by the TSA can be
/// obtained."
///
/// So this is a half-width rather than an offset, and it is kept in the units the grammar states
/// rather than reduced to one: "If either seconds, millis or micros is missing, then a value of
/// zero MUST be taken for the missing field", which [`Default`] is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Accuracy {
    /// `seconds INTEGER OPTIONAL`.
    pub seconds: u64,
    /// `millis [0] INTEGER (1..999) OPTIONAL`.
    pub millis: u16,
    /// `micros [1] INTEGER (1..999) OPTIONAL`.
    pub micros: u16,
}

impl Accuracy {
    /// The whole deviation in microseconds.
    #[must_use]
    pub const fn micros_total(&self) -> u64 {
        self.seconds
            .saturating_mul(1_000_000)
            .saturating_add((self.millis as u64).saturating_mul(1_000))
            .saturating_add(self.micros as u64)
    }
}

/// RFC 3161 section 2.4.2's `TSTInfo`, read out of a token's encapsulated content.
///
/// Borrowed from the caller's bytes throughout, the way every other structure in this crate is.
/// [`Claim`] is the owned summary [`chain`] keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TstInfo<'a> {
    /// `version INTEGER { v1(1) }`, as the file states it.
    ///
    /// Kept rather than checked: RFC 3161 requires a *server* to issue version 1 — "Conforming
    /// time-stamping servers MUST be able to provide version 1 time-stamp tokens" — and says of a
    /// requester only that it must recognise one. A token stating something else is reported by
    /// whoever reports, not refused here.
    pub version: i64,
    /// `policy TSAPolicyId`, as its encoded object identifier.
    ///
    /// "The policy field MUST indicate the TSA's policy under which the response was produced."
    /// Nothing here reads the policy: which policies to accept is a host's input, the same shape
    /// as a trust anchor (ADR 1039).
    pub policy: &'a [u8],
    /// `messageImprint`'s `hashAlgorithm`, as the function this program computes.
    pub imprint_digest: Digest,
    /// `messageImprint`'s `hashedMessage OCTET STRING`.
    ///
    /// Table 255 says what a document timestamp's is: "[t]he value of the messageImprint field
    /// within the `TimeStampToken` shall be a hash of the bytes of the document indicated by the
    /// `ByteRange`".
    pub imprint: &'a [u8],
    /// `serialNumber INTEGER`, as the octets the file wrote.
    ///
    /// Not decoded to a number: "[i]t MUST be unique for each TimeStampToken issued by a given TSA
    /// (i.e., the TSA name and serial number identify a unique TimeStampToken)", which makes it an
    /// identifier, and up to 160 bits of one is wider than any integer this crate carries.
    pub serial_number: &'a [u8],
    /// `genTime GeneralizedTime` — "the time at which the time-stamp token has been created by
    /// the TSA".
    ///
    /// Whole seconds. RFC 3161 permits a fraction here and this program's instants are seconds, so
    /// a fraction is dropped rather than rounded; [`Self::accuracy`] is where a deviation of that
    /// size is already carried, and the RFC's own preference is the same one — "when there is no
    /// need to have a precision better than the second, then GeneralizedTime with a precision
    /// limited to one second SHOULD be used".
    pub gen_time: Instant,
    /// `genTime` as the token spells it, which is the form a report shows.
    ///
    /// The precedent is [`crate::signature::Signature::signed_at`]'s and the reason is the same: a
    /// parse is what an answer is computed from and the producer's own characters are what a
    /// person is shown, so that a token stating a fraction of a second is not shown a time it did
    /// not write. The two are checked against each other by construction — this field is only
    /// filled where [`Self::gen_time`] read.
    pub gen_time_as_written: &'a [u8],
    /// `accuracy Accuracy OPTIONAL`.
    ///
    /// `None` is the clause's own answer rather than zero: "When the accuracy optional field is
    /// not present, then the accuracy may be available through other means, e.g., the
    /// TSAPolicyId."
    pub accuracy: Option<Accuracy>,
    /// `ordering BOOLEAN DEFAULT FALSE`.
    pub ordering: bool,
    /// `nonce INTEGER OPTIONAL`, as the octets the file wrote.
    pub nonce: Option<&'a [u8]>,
    /// `tsa [0] GeneralName OPTIONAL`, as the whole tagged value's encoding.
    ///
    /// A hint and nothing more, which the clause is explicit about: "The purpose of the tsa field
    /// is to give a hint in identifying the name of the TSA … However, the actual identification
    /// of the entity that signed the response will always occur through the use of the certificate
    /// identifier (ESSCertID Attribute) inside a SigningCertificate attribute which is part of the
    /// signerInfo". [`crate::signature::signing_certificate_bindings`] is that identification, and
    /// it is what [`crate::signature::Signature::authenticity`] already acts on.
    pub tsa: Option<&'a [u8]>,
    /// Whether the token states `extensions [1] IMPLICIT Extensions OPTIONAL`.
    ///
    /// Read as a fact and not acted on: "extensions is a generic way to add additional information
    /// in the future", and requesters "are not mandated to understand the semantics of any
    /// extension, if present".
    pub extensions: bool,
}

/// RFC 3161 section 2.4.2's `TSTInfo`, read out of `bytes`.
///
/// [`crate::cms::SignedData::timestamp_imprint`] reads the same structure as far as its third
/// member and no further, which is what §12.8.1's first question needs; this is the whole of it,
/// which is what an instant needs. Neither may disagree with the other about the imprint.
///
/// # Errors
///
/// [`TokenRefusal`], naming what the bytes are instead.
pub fn tst_info(bytes: &[u8]) -> Result<TstInfo<'_>, TokenRefusal> {
    if !der::every_length_is_definite(bytes)? {
        return Err(TokenRefusal::NotDerEncoded);
    }
    let mut reader = Reader::new(bytes)?;
    let Some(outer) = reader.next_value()? else {
        return Err(TokenRefusal::NotATstInfo);
    };
    if outer.identifier != SEQUENCE {
        return Err(TokenRefusal::NotATstInfo);
    }
    let mut members = outer.children()?;
    // The first four members are mandatory and in fixed positions; everything after `genTime` is
    // optional, so the tail is read by tag. Reading the head by position is safe here and is not
    // elsewhere in this crate, because none of the four can be absent.
    let (Some(version), Some(policy), Some(imprint), Some(serial), Some(gen_time)) = (
        members.next_value()?,
        members.next_value()?,
        members.next_value()?,
        members.next_value()?,
        members.next_value()?,
    ) else {
        return Err(TokenRefusal::NotATstInfo);
    };
    if version.identifier != INTEGER
        || policy.identifier != OBJECT_IDENTIFIER
        || imprint.identifier != SEQUENCE
        || serial.identifier != INTEGER
    {
        return Err(TokenRefusal::NotATstInfo);
    }
    if serial.contents.len() > MAX_SERIAL_OCTETS {
        return Err(TokenRefusal::SerialNumberTooLong);
    }
    // `MessageImprint ::= SEQUENCE { hashAlgorithm AlgorithmIdentifier, hashedMessage OCTET
    // STRING }`, which is the same shape `cms::SignedData::timestamp_imprint` reads.
    let mut parts = imprint.children()?;
    let (Some(algorithm), Some(hashed)) = (parts.next_value()?, parts.next_value()?) else {
        return Err(TokenRefusal::NotATstInfo);
    };
    if algorithm.identifier != SEQUENCE || hashed.identifier != OCTET_STRING {
        return Err(TokenRefusal::NotATstInfo);
    }
    let oid = algorithm
        .children()?
        .next_value()?
        .and_then(|value| value.object_identifier())
        .ok_or(TokenRefusal::NotATstInfo)?;
    let imprint_digest = Digest::from_oid(oid).ok_or(TokenRefusal::ImprintDigestUnknown)?;
    let mut info = TstInfo {
        version: small_integer(version.contents).unwrap_or(-1),
        policy: policy.contents,
        imprint_digest,
        imprint: hashed.contents,
        serial_number: serial.contents,
        gen_time: generalized_time(&gen_time).ok_or(TokenRefusal::GenTimeUnreadable)?,
        gen_time_as_written: gen_time.contents,
        accuracy: None,
        ordering: false,
        nonce: None,
        tsa: None,
        extensions: false,
    };
    for member in std::iter::from_fn(|| members.next_value().transpose()) {
        let member = member?;
        match member.identifier {
            SEQUENCE => info.accuracy = Some(accuracy(&member)?),
            // `ordering BOOLEAN DEFAULT FALSE`. X.690 clause 11.1 makes `FF` the only true in DER,
            // and a producer writing any other non-zero octet has written BER: the value is still
            // unambiguously true, and this reader takes it rather than calling the token
            // unreadable over a spelling.
            0x01 => info.ordering = member.contents.iter().any(|octet| *octet != 0),
            INTEGER => info.nonce = Some(member.contents),
            _ if member.is_context(0) => info.tsa = Some(member.encoding()),
            _ if member.is_context(1) => info.extensions = true,
            _ => {}
        }
    }
    Ok(info)
}

/// The `TSTInfo` a CMS object encapsulates, where it is a time-stamp token.
///
/// RFC 3161 section 2.4.2: `TimeStampToken ::= ContentInfo`, whose `eContentType` is
/// `id-ct-TSTInfo` and whose "eContent SHALL be the DER-encoded value of TSTInfo".
///
/// # Errors
///
/// [`TokenRefusal`], naming what the object carries instead.
pub fn token_of<'a>(cms: &SignedData<'a>) -> Result<TstInfo<'a>, TokenRefusal> {
    if cms.content_type != ID_CT_TST_INFO {
        return Err(TokenRefusal::ContentTypeIsNotTstInfo);
    }
    let encapsulated = cms
        .encapsulated
        .ok_or(TokenRefusal::NoEncapsulatedContent)?;
    tst_info(encapsulated)
}

/// What a signature timestamp attribute says, and whether it is about *this* signature.
///
/// §12.8.3.3.1's other timestamp: "Timestamp information as an unsigned attribute ( PDF 1.6 ): The
/// timestamp token shall conform to Internet RFC 3161 as updated by Internet RFC 5816 , and shall
/// be computed and embedded into the CMS object as described in Appendix A of Internet RFC 3161 as
/// updated by Internet RFC 5816 ." Appendix A is what makes the attribute checkable without a
/// certificate: "The value of messageImprint field within TimeStampToken shall be a hash of the
/// value of signature field within SignerInfo for the signedData being time-stamped."
///
/// So the imprint has a counterpart this program can recompute, and [`Self::covers_the_signature`]
/// is that comparison. It is not a verdict on anything: the same four steps [`Time`] lists decide
/// whether the instant is one, and this attribute reaches them the same way a document timestamp
/// does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureTimestamp {
    /// What the token claims.
    pub claim: Claim,
    /// Whether the imprint is the digest of the `SignerInfo`'s `signature` field.
    ///
    /// `false` is a token that was applied to some other signature, or to this one before it was
    /// the signature the file now holds — which is the whole of what Appendix A lets a reader say
    /// without asking whether the authority is anybody.
    pub covers_the_signature: bool,
}

/// The `signature-time-stamp` unsigned attribute, where a `SignerInfo` states one.
///
/// `None` where it states none, which is every signature that puts its timestamp in §12.8.5's
/// document timestamp dictionary instead — the arrangement §12.8.3.3.1's NOTE recommends: "Since
/// PDF 2.0 supports two additional dictionaries, i.e. the DSS and the DTS dictionaries, revocation
/// information can be better placed in a DSS dictionary while time-stamping information can also
/// be placed in a DTS dictionary, in addition to being placed in the CMS object."
///
/// # Errors
///
/// [`TokenRefusal`], where the attribute holds something that is not a time-stamp token.
#[must_use]
pub fn signature_timestamp(
    cms: &SignedData<'_>,
) -> Option<Result<SignatureTimestamp, TokenRefusal>> {
    let attribute = cms.signature_timestamp?;
    Some(read_signature_timestamp(cms, attribute))
}

/// §12.8.3.4.8's instant: what this program established about a signature timestamp's own token.
///
/// The clause's first sentence is what asks for this — "[w]hen a timestamp token is already present
/// in the CAdES signature as a signature timestamp attribute (it is an unsigned attribute), the
/// signer's signature shall be verified at the UTC time in the past indicated in that token" — and
/// the phrase that had held it back is *indicated in that token*: an instant a token merely states
/// is a number a stranger wrote, and validating a certification path at it would let the file
/// choose the moment it is judged at. So the same four steps a document timestamp goes through
/// ([`established`]) are applied here, over the token inside the attribute rather than over a
/// `/DocTimeStamp` dictionary's value. `None` where the signature states no such attribute.
///
/// Two differences from [`established`], both of them the attribute's rather than this function's:
///
/// - **there is no document behind the token.** RFC 3161 section 2.4.2 requires a token to
///   encapsulate its `TSTInfo`, so step 3 has the message in hand, and a token without one is
///   refused by name rather than treated as a signature over something unavailable;
/// - **what the token is *about* is not the document either.** ETSI EN 319 122-1 clause 5.3 puts
///   the imprint over the `SignerInfo`'s `signature` field, so whether this token is about *this*
///   signature is [`SignatureTimestamp::covers_the_signature`] and is a separate question from
///   whether it asserts an instant at all. A caller taking §12.8.3.4.8's step needs both: an
///   established instant from a token about some other signature is not this signature's past.
///
/// `asked_at` is RFC 5280 section 6.1.1's input (b) for the *authority's* path. §12.8.3.4.8 does
/// not say what it should be, and this function does not choose: the caller says, exactly as
/// [`established`] makes the document-timestamp caller say, and [`AskedAt`] reports which it was.
#[must_use]
pub fn signature_timestamp_established(
    cms: &SignedData<'_>,
    anchors: &TrustAnchors<'_>,
    material: &Material<'_>,
    asked_at: AskedAt,
) -> Option<Time> {
    let attribute = cms.signature_timestamp?;
    // Step 1.
    let Ok(token) = crate::cms::signed_data(attribute.encoding()) else {
        return Some(Time::Unknown(Unestablished::TokenUnreadable(
            TokenRefusal::NoEncapsulatedContent,
        )));
    };
    let info = match token_of(&token) {
        Ok(info) => info,
        Err(refusal) => return Some(Time::Unknown(Unestablished::TokenUnreadable(refusal))),
    };
    // Step 2, on [`established`]'s rule and for its reason.
    if token.signed_attributes.is_some() {
        let Some(recorded) = token.message_digest else {
            return Some(Time::Unknown(Unestablished::NoMessageDigestAttribute));
        };
        let Some(algorithm) = token.digest else {
            return Some(Time::Unknown(Unestablished::TokenUnreadable(
                TokenRefusal::ImprintDigestUnknown,
            )));
        };
        if algorithm.compute(&[token.encapsulated.unwrap_or_default()]) != recorded {
            return Some(Time::Unknown(Unestablished::ContentNotBoundToTheSignature));
        }
    }
    // Step 3.
    let authenticity = signature::authenticity_of(&token, signature::Detached::Nothing);
    if !matches!(authenticity, Authenticity::Verified { .. }) {
        return Some(Time::Unknown(Unestablished::SignatureNotVerified(
            Box::new(authenticity),
        )));
    }
    // Step 4, with RFC 3161 section 2.3's purpose for the reason `established` states.
    if anchors.is_empty() {
        return Some(Time::Unknown(Unestablished::AuthorityNotEstablished(
            Trust::NoAnchorSupplied,
        )));
    }
    Some(
        match signature::cms_trust(
            &token,
            anchors,
            material,
            asked_at.instant(),
            Purpose::TimeStamping,
        ) {
            Trust::Anchored { revocation, .. } => Time::Established {
                at: info.gen_time,
                accuracy: info.accuracy,
                revocation,
                asked_at,
            },
            other => Time::Unknown(Unestablished::AuthorityNotEstablished(other)),
        },
    )
}

/// [`signature_timestamp`]'s body, with the `?` on a `Result` rather than on an `Option`.
fn read_signature_timestamp(
    cms: &SignedData<'_>,
    attribute: der::Value<'_>,
) -> Result<SignatureTimestamp, TokenRefusal> {
    let inner = crate::cms::signed_data(attribute.encoding())
        .map_err(|_| TokenRefusal::NoEncapsulatedContent)?;
    let info = token_of(&inner)?;
    Ok(SignatureTimestamp {
        covers_the_signature: info.imprint_digest.compute(&[cms.signature]) == info.imprint,
        claim: Claim::from(&info),
    })
}

/// RFC 3161 section 2.4.2's `Accuracy ::= SEQUENCE { seconds, millis [0], micros [1] }`.
fn accuracy(value: &der::Value<'_>) -> Result<Accuracy, TokenRefusal> {
    let mut fields = value.children()?;
    let mut accuracy = Accuracy::default();
    for field in std::iter::from_fn(|| fields.next_value().transpose()) {
        let field = field?;
        let number = small_integer(field.contents).ok_or(TokenRefusal::AccuracyOutOfRange)?;
        if field.identifier == INTEGER {
            accuracy.seconds =
                u64::try_from(number).map_err(|_| TokenRefusal::AccuracyOutOfRange)?;
            continue;
        }
        // The grammar's own range, and the one place this reader holds a producer to a subtype
        // constraint: a `millis` of 1000 would otherwise be added to the seconds twice over.
        let thousandths = u16::try_from(number).map_err(|_| TokenRefusal::AccuracyOutOfRange)?;
        if !(1..=999).contains(&thousandths) {
            return Err(TokenRefusal::AccuracyOutOfRange);
        }
        if field.is_context(0) {
            accuracy.millis = thousandths;
        } else if field.is_context(1) {
            accuracy.micros = thousandths;
        }
    }
    Ok(accuracy)
}

/// A DER `INTEGER`'s contents as an [`i64`], where they fit in one.
///
/// `None` for anything wider, which is every case this module then refuses by name: a version, an
/// accuracy field and nothing else are read as numbers here, and the two identifiers — the serial
/// number and the nonce — are kept as octets.
fn small_integer(contents: &[u8]) -> Option<i64> {
    if contents.is_empty() || contents.len() > 8 {
        return None;
    }
    let negative = contents.first().is_some_and(|octet| octet & 0x80 != 0);
    let mut value: i64 = if negative { -1 } else { 0 };
    for octet in contents {
        value = value.checked_mul(256)?.checked_add(i64::from(*octet))?;
    }
    Some(value)
}

/// RFC 3161 section 2.4.2's `genTime`, which is a `GeneralizedTime` with a fraction permitted.
///
/// The clause spells the form out — "The syntax is: YYYYMMDDhhmmss\[.s...\]Z" — and
/// [`crate::x509::read_time`] reads RFC 5280's form, which forbids the fraction; this strips one
/// where the RFC's own restrictions admit it — "[t]he decimal point element, if present, MUST be
/// the point option \".\". The fractional-seconds elements, if present, MUST omit all trailing
/// 0's" — and hands the whole seconds to the same arithmetic. Anything else is `None`.
fn generalized_time(value: &der::Value<'_>) -> Option<Instant> {
    /// `GeneralizedTime`, primitive and universal.
    const GENERALIZED_TIME: u8 = 0x18;
    if value.identifier != GENERALIZED_TIME {
        return None;
    }
    let digits = value.contents;
    let Some(point) = digits.iter().position(|octet| *octet == b'.') else {
        return x509::read_time_digits(digits, true);
    };
    // "The encoding MUST terminate with a \"Z\"", so the fraction is what lies between the point
    // and that one octet, and it has to be digits for the value to be a time at all.
    let fraction = digits.get(point.checked_add(1)?..digits.len().checked_sub(1)?)?;
    if digits.last() != Some(&b'Z')
        || fraction.is_empty()
        || !fraction.iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    let mut whole = digits.get(..point)?.to_vec();
    whole.push(b'Z');
    x509::read_time_digits(&whole, true)
}

/// What a token *says*, owned, whether or not anything established it.
///
/// Reported as a claim and never as a fact: §12.8.5.1 makes the whole point of a timestamp the
/// instant — "[a] document timestamp dictionary establishes the exact contents of the complete PDF
/// file at the time indicated in the timestamp token" — and *establishes* is what [`Time`] answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// `genTime`, placed on a line.
    pub gen_time: Instant,
    /// `genTime` as the token spells it, which is what a report shows.
    pub stated: String,
    /// `accuracy`, where the token states one.
    pub accuracy: Option<Accuracy>,
    /// `serialNumber`, as octets.
    pub serial_number: Vec<u8>,
    /// Whether the token states a `tsa` hint.
    pub names_an_authority: bool,
    /// The digest `messageImprint` is taken with.
    pub imprint_digest: Digest,
}

impl From<&TstInfo<'_>> for Claim {
    fn from(info: &TstInfo<'_>) -> Self {
        Self {
            gen_time: info.gen_time,
            stated: String::from_utf8_lossy(info.gen_time_as_written).into_owned(),
            accuracy: info.accuracy,
            serial_number: info.serial_number.to_vec(),
            names_an_authority: info.tsa.is_some(),
            imprint_digest: info.imprint_digest,
        }
    }
}

/// Which instant a timestamp's own certification path was asked about.
///
/// §12.8.5.3's mechanism, made visible: a token's certificate is checked at the moment the *next*
/// token says the document was already in this state, because checking it at its own `genTime`
/// would be asking the file to vouch for itself (ADR 1071 section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskedAt {
    /// A later timestamp in the chain established that instant, and its range covers this one.
    ///
    /// The index is into [`Chain::links`].
    ALaterTimestamp {
        /// The instant that timestamp established.
        at: Instant,
        /// Which link in the chain established it.
        link: usize,
    },
    /// Nothing later established an instant, so the caller's is what there is.
    ///
    /// RFC 5280 section 6.1.1's input (b), "the current date/time", which this crate takes from its
    /// caller and never from a clock.
    TheCallersInstant(Instant),
}

impl AskedAt {
    /// The instant itself.
    #[must_use]
    pub const fn instant(self) -> Instant {
        match self {
            Self::ALaterTimestamp { at, .. } | Self::TheCallersInstant(at) => at,
        }
    }
}

/// What this program established about the instant a document timestamp asserts.
///
/// **No variant means *valid***, for [`crate::trust::Trust`]'s reason and ADR 1039's:
/// [`Self::Established`] says a token whose authority chains to an anchor *somebody supplied*
/// asserted an instant, and nothing in this tree supplies one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Time {
    /// All four steps held, and this is the instant the token asserts.
    Established {
        /// `genTime`.
        at: Instant,
        /// `accuracy`, which is the half-width around it.
        accuracy: Option<Accuracy>,
        /// What §12.8.4's material said about the authority's path.
        revocation: Revocation,
        /// Which instant that path was validated at.
        asked_at: AskedAt,
    },
    /// It did not, and this is where it stopped.
    Unknown(Unestablished),
}

/// Why a token's `genTime` is not an instant this program will state.
///
/// One variant per step of the module comment's four, in the order they are taken. Each carries
/// what it was told, because which of them refused decides what a person can do about it: a token
/// whose signature does not verify is a broken file, and one with no anchor is this program's own
/// answer to every document there is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Unestablished {
    /// The dictionary is not a `/DocTimeStamp` at all.
    NotADocumentTimestamp,
    /// Step 1: the token would not read.
    TokenUnreadable(TokenRefusal),
    /// Step 2: the signer's `message-digest` attribute is not the digest of the `TSTInfo`.
    ///
    /// RFC 5652 section 5.6 makes this decisive: the signature covers the attributes, so a
    /// `TSTInfo` the attributes do not commit to is one the authority did not sign.
    ContentNotBoundToTheSignature,
    /// Step 2, the other way: the signer states no `message-digest` attribute to compare.
    ///
    /// RFC 5652 section 11.2: "the message-digest signed attribute type MUST be present when there
    /// are any signed attributes present". A `SignerInfo` with no signed attributes at all signs
    /// the content directly, which needs no binding and reaches step 3 instead.
    NoMessageDigestAttribute,
    /// Step 3: the token's own signature did not verify.
    SignatureNotVerified(Box<Authenticity>),
    /// Step 4: no path from the authority's certificate reached a supplied anchor.
    ///
    /// [`crate::trust::Trust::NoAnchorSupplied`] is what every document in this tree gets.
    AuthorityNotEstablished(Trust),
}

/// What a document timestamp establishes about `file`, at `asked_at`.
///
/// The four steps the module comment lists, in order, with `material` applied to the authority's
/// path the way [`crate::signature::Signature::trust`] applies it to a signer's.
///
/// **This does not ask whether the imprint matches the document.** That is
/// [`crate::signature::Signature::integrity`] — question 1 — and it is a different question from
/// whether the token asserts an instant at all: a timestamp over a document somebody later edited
/// still carries an authority's statement about the earlier bytes. [`Link`] reports both.
#[must_use]
pub fn established(
    timestamp: &Signature,
    file: &FileBytes,
    anchors: &TrustAnchors<'_>,
    material: &Material<'_>,
    asked_at: AskedAt,
) -> Time {
    if !timestamp.timestamp {
        return Time::Unknown(Unestablished::NotADocumentTimestamp);
    }
    let Ok(cms) = timestamp.signed_data() else {
        return Time::Unknown(Unestablished::TokenUnreadable(
            TokenRefusal::NoEncapsulatedContent,
        ));
    };
    let info = match token_of(&cms) {
        Ok(info) => info,
        Err(refusal) => return Time::Unknown(Unestablished::TokenUnreadable(refusal)),
    };
    // Step 2. Only where the signer states signed attributes: with none, RFC 5652 section 5.4 puts
    // the signature over the content itself, and step 3 is the binding.
    if cms.signed_attributes.is_some() {
        let Some(recorded) = cms.message_digest else {
            return Time::Unknown(Unestablished::NoMessageDigestAttribute);
        };
        let Some(algorithm) = cms.digest else {
            return Time::Unknown(Unestablished::TokenUnreadable(
                TokenRefusal::ImprintDigestUnknown,
            ));
        };
        let encapsulated = cms.encapsulated.unwrap_or_default();
        if algorithm.compute(&[encapsulated]) != recorded {
            return Time::Unknown(Unestablished::ContentNotBoundToTheSignature);
        }
    }
    // Step 3.
    let authenticity = timestamp.authenticity(file);
    if !matches!(authenticity, Authenticity::Verified { .. }) {
        return Time::Unknown(Unestablished::SignatureNotVerified(Box::new(authenticity)));
    }
    // Step 4. The purpose is stated because RFC 3161 section 2.3 states it: a timestamp
    // authority's certificate "MUST contain only one instance of the extended key usage field
    // extension … with KeyPurposeID having value: id-kp-timeStamping", and "[t]his extension MUST
    // be critical" — so without a purpose every conforming authority's certificate is refused as
    // an unrecognised critical extension (ADR 1071 section 4).
    match timestamp.trust_for(anchors, material, asked_at.instant(), Purpose::TimeStamping) {
        Trust::Anchored { revocation, .. } => Time::Established {
            at: info.gen_time,
            accuracy: info.accuracy,
            revocation,
            asked_at,
        },
        other => Time::Unknown(Unestablished::AuthorityNotEstablished(other)),
    }
}

/// What this reader would not conclude about a document's chain of timestamps, by name.
///
/// §12.8.5.3's requirement is that a later token "will protect the whole structure", and each of
/// these is a way a file does not let that be checked. Every one names what it was about.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ChainRefusal {
    /// A timestamp's `/ByteRange` does not end at an `%%EOF`, so what it covers is not a whole
    /// number of revisions.
    ///
    /// §12.8.1 fixes the end — "to the end of the \"%%EOF\" comment, possibly followed by an
    /// optional EOL marker, terminating the incremental update that adds the digital signature
    /// dictionary to the document" — and that is what makes "this object is below the boundary"
    /// mean "this object is inside what was signed". A range stopping elsewhere signs a prefix of
    /// a revision, and this reader will not say what such a range covers (ADR 1071 section 3).
    #[error(
        "document timestamp {index}'s byte range does not end at an %%EOF, so what it covers is not decidable"
    )]
    RangeDoesNotEndARevision {
        /// Which link of the chain.
        index: usize,
    },
    /// A timestamp's `/ByteRange` does not cover the whole file.
    ///
    /// §12.8.5.2: "The ByteRange key shall cover the entire PDF file, including the signature
    /// dictionary but excluding the Contents value." A timestamp that covers less has left bytes
    /// of this document outside every statement it makes.
    #[error("document timestamp {index}'s byte range leaves {tail} byte(s) of the file unsigned")]
    RangeDoesNotCoverTheFile {
        /// Which link of the chain.
        index: usize,
        /// How many bytes it leaves out.
        tail: u64,
    },
    /// A later timestamp does not cover an earlier one, so the chain is not one chain.
    #[error("document timestamp {later} does not cover document timestamp {earlier}")]
    DoesNotCoverTheEarlierTimestamp {
        /// The link that should have covered.
        later: usize,
        /// The link it did not cover.
        earlier: usize,
    },
    /// The last timestamp does not cover a piece of §12.8.4's validation material.
    ///
    /// §12.8.5.3 requires the material for the previous authority's path to be "included into the
    /// DSS dictionary" and the new token then "placed into a new document timestamp dictionary
    /// which will protect the whole structure", so material outside every timestamp's range is
    /// material the chain does not protect.
    #[error("document timestamp {index} does not cover the validation material in object {object}")]
    DoesNotCoverMaterial {
        /// Which link of the chain.
        index: usize,
        /// The object number of the stream holding it.
        object: u32,
    },
    /// A piece of validation material is not at a byte offset in this file.
    ///
    /// §7.5.7 forbids the one case that would produce this — its list of what "shall not be stored
    /// in an object stream" opens with stream objects — and a document security store holds streams
    /// (Table 261), so this is a file breaking that rule rather than a shape this reader declines
    /// to handle. What it costs is exactly one answer: nothing can say which timestamps cover that
    /// entry.
    #[error("the validation material in object {object} is not at a byte offset in this file")]
    MaterialNotAtAnOffset {
        /// The object number the store named.
        object: u32,
    },
    /// The document states more document timestamps than [`MAX_TIMESTAMPS`].
    #[error("this document states more than {MAX_TIMESTAMPS} document timestamps")]
    MoreTimestampsThanWalked,
}

/// One document timestamp, and what its byte range covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// Where this timestamp sits among [`crate::signature::signatures`]'s answers.
    pub signature: usize,
    /// The last byte of the file its `/ByteRange` names, exclusive.
    pub covers_to: u64,
    /// Whether that boundary ends a revision.
    pub ends_a_revision: bool,
    /// What the token claims, where it reads.
    pub claim: Result<Claim, TokenRefusal>,
    /// Whether the imprint inside the token is the digest of the bytes the range names.
    ///
    /// [`crate::signature::Signature::integrity`]'s answer, which for a document timestamp is
    /// Table 255's requirement about `messageImprint` rather than a `message-digest` attribute.
    pub integrity: Integrity,
    /// What this program established about the instant.
    pub time: Time,
    /// The signature dictionaries whose revision this timestamp's range covers whole.
    pub covers: Vec<usize>,
    /// Validation-material objects its range covers, by object number.
    pub material_covered: Vec<u32>,
    /// Validation-material objects its range does not, by object number.
    pub material_uncovered: Vec<u32>,
}

/// §12.8.5's document timestamps over one document, in the order their ranges nest.
///
/// [`Self::links`] is ordered by how much of the file each timestamp's range names, least first,
/// which is the order §12.8.5.3 describes a document being built in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Chain {
    /// One per `/Type /DocTimeStamp` dictionary, innermost first.
    pub links: Vec<Link>,
    /// How many of the document's signature dictionaries are not timestamps.
    pub signatures: usize,
    /// What this reader would not conclude, by name.
    pub refused: Vec<ChainRefusal>,
}

impl Chain {
    /// Whether the document carries any document timestamp at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    /// The outermost timestamp — the one whose range names the most of the file.
    #[must_use]
    pub fn outermost(&self) -> Option<&Link> {
        self.links.last()
    }
}

/// §12.8.5's chain over `document`, with each timestamp's authority checked at the next one's
/// instant.
///
/// `anchors`, `material` and `at` are the caller's, exactly as they are for
/// [`crate::signature::Signature::trust`]: RFC 5280 section 6.1.1 makes the anchors input (d) and
/// the time input (b), §12.8.4's material comes out of the document, and nothing here opens a
/// socket or asks a clock. `at` is used only where no later timestamp established one, which is
/// what [`AskedAt`] reports.
///
/// **The links are established from the outside in.** The outermost token is the only one nothing
/// later covers, so it is the only one the caller's instant is spent on; every earlier one is asked
/// about at the instant the token above it established, which is §12.8.5.3's whole argument.
#[must_use]
pub fn chain(
    document: &Document,
    anchors: &TrustAnchors<'_>,
    material: &Material<'_>,
    at: Instant,
) -> Chain {
    let file = document.bytes().clone();
    let length = u64::try_from(file.len()).unwrap_or(u64::MAX);
    let signatures = signature::signatures(document);
    let mut chain = Chain::default();
    let mut found: Vec<(usize, &Signature)> = Vec::new();
    for (index, signature) in signatures.iter().enumerate() {
        if signature.timestamp {
            found.push((index, signature));
        } else {
            chain.signatures = chain.signatures.saturating_add(1);
        }
    }
    if found.len() > MAX_TIMESTAMPS {
        chain.refused.push(ChainRefusal::MoreTimestampsThanWalked);
        found.truncate(MAX_TIMESTAMPS);
    }
    // The order is the file's own nesting rather than the walk's: §12.8.5.3 applies each new token
    // over the whole structure the previous one left, so the range that names the least of the
    // file is the earliest.
    found.sort_by_key(|(index, signature)| (covers_to(signature), *index));
    let extents = material_extents(document, &mut chain.refused);
    for (position, (index, signature)) in found.iter().enumerate() {
        let extent = covers_to(signature);
        let ends_a_revision = signature.signed_end(&file) == SignedEnd::AtAnEndOfFileMarker;
        if !ends_a_revision {
            chain
                .refused
                .push(ChainRefusal::RangeDoesNotEndARevision { index: position });
        }
        // §12.8.5.2's whole-file requirement is asked of the *outermost* timestamp only, and that
        // is §12.8.5.3's doing rather than a softening: the clause has each new token applied over
        // everything the previous one left, so an earlier token covering less than the finished
        // file is the shape the clause describes. What no token covers is what nothing protects.
        if extent < length && position.saturating_add(1) == found.len() {
            chain.refused.push(ChainRefusal::RangeDoesNotCoverTheFile {
                index: position,
                tail: length.saturating_sub(extent),
            });
        }
        let mut covers = Vec::new();
        if ends_a_revision {
            for (other, candidate) in signatures.iter().enumerate() {
                if other != *index && covers_to(candidate) <= extent {
                    covers.push(other);
                }
            }
        }
        let (material_covered, material_uncovered) =
            split_material(&extents, extent, ends_a_revision);
        chain.links.push(Link {
            signature: *index,
            covers_to: extent,
            ends_a_revision,
            claim: signature
                .signed_data()
                .map_err(|_| TokenRefusal::NoEncapsulatedContent)
                .and_then(|cms| token_of(&cms).map(|info| Claim::from(&info))),
            integrity: signature.integrity(&file),
            // Filled in below, outermost first.
            time: Time::Unknown(Unestablished::NotADocumentTimestamp),
            covers,
            material_covered,
            material_uncovered,
        });
    }
    nothing_left_uncovered(&mut chain);
    establish(&mut chain, &signatures, &file, anchors, material, at);
    chain
}

/// §12.8.5.3's two coverage questions over a built chain, each refused by name.
///
/// The clause has each new token applied over everything the one before it left and "will protect
/// the whole structure", so what is asked is: does every link have a covering one above it, and
/// does the outermost cover §12.8.4's material. The material is the *chain's* rather than each
/// link's — an earlier token was applied before the later material existed — which is why the
/// second question is asked of the outermost link alone.
fn nothing_left_uncovered(chain: &mut Chain) {
    for position in 0..chain.links.len().saturating_sub(1) {
        let (Some(inner), Some(outer)) = (
            chain.links.get(position).map(|link| link.covers_to),
            chain.links.get(position.saturating_add(1)),
        ) else {
            continue;
        };
        if !(outer.ends_a_revision && outer.covers_to >= inner) {
            chain
                .refused
                .push(ChainRefusal::DoesNotCoverTheEarlierTimestamp {
                    later: position.saturating_add(1),
                    earlier: position,
                });
        }
    }
    if let Some(outermost) = chain.links.len().checked_sub(1) {
        let uncovered = chain
            .links
            .get(outermost)
            .map(|link| link.material_uncovered.clone())
            .unwrap_or_default();
        for object in uncovered {
            chain.refused.push(ChainRefusal::DoesNotCoverMaterial {
                index: outermost,
                object,
            });
        }
    }
}

/// [`established`] over every link, outermost first, threading [`AskedAt`] down the chain.
///
/// The direction is the whole of ADR 1071 section 2: the outermost token is the only one nothing
/// later covers, so it is the only one the caller's instant is spent on, and every earlier one is
/// asked about at the instant the token above it established.
fn establish(
    chain: &mut Chain,
    signatures: &[Signature],
    file: &FileBytes,
    anchors: &TrustAnchors<'_>,
    material: &Material<'_>,
    at: Instant,
) {
    let mut asked_at = AskedAt::TheCallersInstant(at);
    for position in (0..chain.links.len()).rev() {
        let Some(signature) = chain
            .links
            .get(position)
            .and_then(|link| signatures.get(link.signature))
        else {
            continue;
        };
        let time = established(signature, file, anchors, material, asked_at);
        if let Time::Established {
            at: established_at, ..
        } = time
        {
            asked_at = AskedAt::ALaterTimestamp {
                at: established_at,
                link: position,
            };
        }
        if let Some(link) = chain.links.get_mut(position) {
            link.time = time;
        }
    }
}

/// The last byte a signature's `/ByteRange` names, exclusive.
fn covers_to(signature: &Signature) -> u64 {
    signature
        .byte_range
        .last()
        .map_or(0, |(start, size)| start.saturating_add(*size))
}

/// Where each of §12.8.4's material streams begins in the file, by object number.
///
/// Certificates are not here and that is deliberate: §12.8.5.3's requirement names "[t]he
/// certificates, CRLs or OCSP responses used to demonstrate that the certificates … was not
/// revoked", and this walk takes all three of Table 261's arrays for exactly that reason.
fn material_extents(document: &Document, refused: &mut Vec<ChainRefusal>) -> BTreeMap<u32, u64> {
    let mut extents = BTreeMap::new();
    for entry in signature::security_store_entries(document) {
        match document.xref().location(entry.number()) {
            Some(Location::Offset(offset)) => {
                extents.insert(entry.number(), u64::try_from(offset).unwrap_or(u64::MAX));
            }
            _ => refused.push(ChainRefusal::MaterialNotAtAnOffset {
                object: entry.number(),
            }),
        }
    }
    extents
}

/// Which material objects a range reaching `covers_to` covers, and which it does not.
///
/// An object whose bytes begin below a revision boundary ends below it too, because §7.5.5 puts
/// the boundary at the end of the revision that holds the object; so a range ending at an `%%EOF`
/// covers every object that begins before it. A range that does not end at one says nothing here,
/// which is [`ChainRefusal::RangeDoesNotEndARevision`]'s cost.
fn split_material(
    extents: &BTreeMap<u32, u64>,
    covers_to: u64,
    ends_a_revision: bool,
) -> (Vec<u32>, Vec<u32>) {
    if !ends_a_revision {
        return (Vec::new(), extents.keys().copied().collect());
    }
    let mut covered = Vec::new();
    let mut uncovered = Vec::new();
    for (number, offset) in extents {
        if *offset < covers_to {
            covered.push(*number);
        } else {
            uncovered.push(*number);
        }
    }
    (covered, uncovered)
}

#[cfg(test)]
pub(crate) mod tests;

//! ISO 32000-2 §12.8.3.3's CMS signature value, read for the one thing it states in the clear.
//!
//! §12.8.1 divides verifying a signature into parts, and they do not need the same infrastructure:
//!
//! > The signer's certificate shall be determined and verified by the signature handler to match
//! > with any of the validation parameters and other conditions. If the verification fails, the
//! > signature shall be considered invalid. The digest shall be recomputed and compared with the
//! > one stored in the document. Differences between the two indicates that modifications have
//! > been made since the document was signed and thus the signature shall be considered invalid.
//!
//! The first sentence is a certificate, a chain and a trust decision. The last is **arithmetic
//! over bytes this program already holds** — and for the signature formats §12.8.3 defines, the
//! digest it names is written into the signature value where anyone can read it. This module
//! finds it. [`crate::signature::Integrity`] is what compares it.
//!
//! # What is read, and from where
//!
//! §12.8.3.3.1: "The CMS object shall conform to Internet RFC 5652". Of that structure this
//! module reads `SignedData`'s encapsulated content type, its certificate count, its one
//! `SignerInfo`'s digest algorithm, and the object identifiers of that signer's signed and
//! unsigned attributes — with the contents of `message-digest` (RFC 5652's
//! `id-messageDigest`), which is the digest of the signed content.
//!
//! **This paragraph used to end "[e]verything else in RFC 5652 is deliberately not read: the
//! certificates are X.509 and a trust decision, and the signature value itself needs the signer's
//! public key", and the three-hundred-and-ninety-second session made both halves obsolete.** The
//! certificates are handed over as values for [`crate::x509`] to read, and the signer's signature
//! algorithm, signature value and identifier are read here so that
//! [`crate::signature::Signature::authenticity`] can verify one. What is still not read is
//! anything a *trust* decision would need: no `crls`, no certification path, no validity dates.
//!
//! # What a matching digest proves, and what it does not
//!
//! **A mismatch is decisive and a match is not, on its own.** The digest sits beside the signature
//! rather than inside it, so anyone who alters the document can alter the recorded digest to match
//! — what they cannot do is make the *signature* over that digest verify. Checking that is
//! [`crate::signature::Signature::authenticity`]'s job, and the two answers are worth more
//! together than either is alone: four of the corpus's ten signatures verify over a digest their
//! documents no longer produce, which says the file was re-saved under a real signature. The
//! wording the program uses to a person keeps the two apart.

use crate::der::{DerError, INTEGER, OCTET_STRING, Reader, SEQUENCE, SET, Value};

/// How many of a signer's attributes are recorded.
///
/// RFC 5652 puts no bound on `SignedAttributes`, and this is the one allocation in the module that
/// a file's contents drive. §12.8.3.4.3 lists eleven attributes a `PAdES` signature may carry, so a
/// signature with more than sixty-four has stopped being one this reader has anything to say
/// about; the ones past the bound are dropped and [`SignedData::attributes_truncated`] says so.
const MAX_ATTRIBUTES: usize = 64;

/// How many certificates are kept out of a `SignedData`.
///
/// §12.8.3.3.1 requires one — "[a]t minimum the CMS object shall include the signer's X.509
/// signing certificate" — and permits a whole chain beside it. This bounds the one `Vec` a file's
/// contents size in this module; a signature carrying more has a certificate past the bound
/// ignored, which makes the signer *unmatched* rather than wrongly matched.
const MAX_CERTIFICATES: usize = 64;

/// RFC 5652's `id-signedData`, `1.2.840.113549.1.7.2`.
const ID_SIGNED_DATA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02];
/// RFC 5652's `id-data`, `1.2.840.113549.1.7.1` — §12.8.3.4.3 (a)'s "id-data".
pub const ID_DATA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x01];
/// RFC 5652's `id-contentType`, `1.2.840.113549.1.9.3`.
pub const ID_CONTENT_TYPE: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x03];
/// RFC 5652's `id-messageDigest`, `1.2.840.113549.1.9.4`.
pub const ID_MESSAGE_DIGEST: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x04];
/// RFC 5652's `id-signingTime`, `1.2.840.113549.1.9.5` — §12.8.3.4.3 (g)'s "signing-time".
pub const ID_SIGNING_TIME: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x05];
/// RFC 5652's `id-countersignature`, `1.2.840.113549.1.9.6` — §12.8.3.4.3 (i)'s
/// "counter-signature", which a `PAdES` signature "shall not" use.
pub const ID_COUNTERSIGNATURE: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x06];
/// RFC 3161's `id-ct-TSTInfo`, `1.2.840.113549.1.9.16.1.4` — what a document timestamp
/// encapsulates (§12.8.5).
pub const ID_CT_TST_INFO: &[u8] = &[
    0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x10, 0x01, 0x04,
];
/// §12.8.3.3.2's revocation information attribute, whose identifier the clause prints itself:
///
/// > adbe-revocationInfoArchival OBJECT IDENTIFIER::= {adbe(1.2.840.113583) acrobat(1)
/// > security(1) 8}
pub const ADBE_REVOCATION_INFO_ARCHIVAL: &[u8] =
    &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x2F, 0x01, 0x01, 0x08];

/// An X.501 `Name`'s encoding and a serial number's, which is how RFC 5652 names a certificate.
pub type IssuerAndSerial<'a> = (&'a [u8], &'a [u8]);

/// The public-key algorithm a `SignerInfo` says its signature was made with.
///
/// Table 260 names three families for a PDF signature — "RSA Algorithm Support", "DSA Algorithm
/// Support" and "ECDSA Algorithm Support ( defined by Internet RFC 5480 )" — and this program
/// verifies the first. The other two reach a person as the object identifier the file states,
/// which is why the unrecognised arm carries it rather than dropping it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm<'a> {
    /// RSASSA-PKCS1-v1_5 — `rsaEncryption` or one of the `<hash>WithRSAEncryption` identifiers of
    /// RFC 8017's `pkcs-1` arc, which name the same padding and differ only in the digest that
    /// `digestAlgorithm` states anyway.
    ///
    /// `id-RSASSA-PSS` is deliberately **not** here: it is the same arc and a different padding,
    /// so treating it as this one would verify the wrong construction. It arrives as
    /// [`Self::Unrecognised`].
    RsaPkcs1V15,
    /// Anything else, as the identifier the file wrote.
    Unrecognised(&'a [u8]),
}

impl<'a> SignatureAlgorithm<'a> {
    /// What an `AlgorithmIdentifier`'s object identifier names.
    ///
    /// The RSA identifiers are enumerated rather than matched by their arc, because
    /// `1.2.840.113549.1.1.10` is in the same arc and is RSASSA-PSS — a different padding, which a
    /// prefix test would silently verify as PKCS #1 v1.5.
    #[must_use]
    pub fn from_oid(oid: &'a [u8]) -> Self {
        match oid {
            // `pkcs-1` is 1.2.840.113549.1.1; the last octet is `rsaEncryption` (1) and the
            // `<hash>WithRSAEncryption` identifiers for MD2 (2), MD5 (4), SHA-1 (5), SHA-256 (11),
            // SHA-384 (12), SHA-512 (13) and SHA-224 (14).
            [
                0x2A,
                0x86,
                0x48,
                0x86,
                0xF7,
                0x0D,
                0x01,
                0x01,
                1 | 2 | 4 | 5 | 11 | 12 | 13 | 14,
            ] => Self::RsaPkcs1V15,
            other => Self::Unrecognised(other),
        }
    }
}

/// The digest algorithms Table 260 and Table 256 name, and nothing else.
///
/// Table 260 lists what each `/SubFilter` supports — "SHA1 ( PDF 1.3 ) SHA256 (PDF 1.6) SHA384
/// (PDF 1.7) SHA512 (PDF 1.7) RIPEMD160 (PDF 1.7 )" — and Table 256's `/DigestMethod` adds the
/// MD5 that was PDF 1.5's default. All six are here because a program that recognised five would
/// be silent about the sixth rather than wrong about it, and silence is what this round is
/// removing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Digest {
    /// MD5, Table 256's default for PDF 1.5 to 1.7 and deprecated with PDF 2.0.
    Md5,
    /// SHA-1, deprecated with PDF 2.0 and what four of the corpus's signatures use.
    Sha1,
    /// SHA-256.
    Sha256,
    /// SHA-384.
    Sha384,
    /// SHA-512.
    Sha512,
    /// RIPEMD-160, which Table 260 names for every `/SubFilter` and no corpus document writes.
    Ripemd160,
}

impl Digest {
    /// The algorithm an `AlgorithmIdentifier`'s object identifier names.
    ///
    /// `None` for anything else, which the caller reports rather than guessing at: a digest this
    /// program cannot compute is a question it cannot answer, and answering it with the wrong
    /// function would produce a mismatch that reads as a modified document.
    #[must_use]
    pub fn from_oid(oid: &[u8]) -> Option<Self> {
        // The four in the NIST arc (2.16.840.1.101.3.4.2.x) differ in their last octet; SHA-1 is
        // in the older OIW arc and MD5 and RIPEMD-160 in RSA's and Teletrust's respectively.
        match oid {
            [0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x02, 0x05] => Some(Self::Md5),
            [0x2B, 0x0E, 0x03, 0x02, 0x1A] => Some(Self::Sha1),
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01] => Some(Self::Sha256),
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02] => Some(Self::Sha384),
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03] => Some(Self::Sha512),
            [0x2B, 0x24, 0x03, 0x02, 0x01] => Some(Self::Ripemd160),
            _ => None,
        }
    }

    /// The encoded object identifier of this algorithm — [`Self::from_oid`] the other way round.
    ///
    /// Needed because RFC 8017 section 9.2's `DigestInfo` puts the algorithm identifier *inside* the
    /// block a PKCS #1 v1.5 signature commits to, so verifying one means writing the identifier
    /// out again. The two functions are deliberately one pair of constants read in two directions:
    /// a second table would be a second thing to keep right.
    #[must_use]
    pub fn oid(self) -> &'static [u8] {
        match self {
            Self::Md5 => &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x02, 0x05],
            Self::Sha1 => &[0x2B, 0x0E, 0x03, 0x02, 0x1A],
            Self::Sha256 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01],
            Self::Sha384 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02],
            Self::Sha512 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03],
            Self::Ripemd160 => &[0x2B, 0x24, 0x03, 0x02, 0x01],
        }
    }

    /// Every algorithm Table 260 and Table 256 name, for a caller that must try each in turn.
    ///
    /// §12.8.3.2's `adbe.x509.rsa_sha1` records no digest algorithm anywhere a reader can see it —
    /// the identifier is inside the PKCS #1 block, under the key — so the only way to learn which
    /// of the six a signature used is to build the block for each and compare. That is safe
    /// precisely because RFC 8017 section 8.2.2 compares whole blocks: six comparisons of a fixed-length
    /// string admit no more forgeries than one.
    pub const ALL: [Self; 6] = [
        Self::Md5,
        Self::Sha1,
        Self::Sha256,
        Self::Sha384,
        Self::Sha512,
        Self::Ripemd160,
    ];

    /// The algorithm's name, in the spelling Table 260 uses.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Md5 => "MD5",
            Self::Sha1 => "SHA1",
            Self::Sha256 => "SHA256",
            Self::Sha384 => "SHA384",
            Self::Sha512 => "SHA512",
            Self::Ripemd160 => "RIPEMD160",
        }
    }

    /// The digest of a message given in pieces.
    ///
    /// Pieces rather than one slice because §12.8.1's byte range is "pairs of integers (starting
    /// byte offset, length in bytes)" describing a file with a hole in it, and joining them would
    /// copy the whole document to hash it.
    #[must_use]
    pub fn compute(self, message: &[&[u8]]) -> Vec<u8> {
        use sha2::Digest as _;
        /// One hasher fed every piece, so the six arms differ only in their type.
        macro_rules! hash {
            ($hasher:ty) => {{
                let mut hasher = <$hasher>::new();
                for piece in message {
                    hasher.update(piece);
                }
                hasher.finalize().to_vec()
            }};
        }
        match self {
            Self::Md5 => hash!(md5::Md5),
            Self::Sha1 => hash!(sha1::Sha1),
            Self::Sha256 => hash!(sha2::Sha256),
            Self::Sha384 => hash!(sha2::Sha384),
            Self::Sha512 => hash!(sha2::Sha512),
            Self::Ripemd160 => hash!(ripemd::Ripemd160),
        }
    }
}

/// What stopped a signature value from being read as RFC 5652's `SignedData`.
///
/// Each of these is a fact about the file, and [`crate::signature::Integrity::Unreadable`] carries
/// it to a person rather than collapsing them all into a shrug.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CmsError {
    /// The ASN.1 encoding itself is malformed.
    #[error("the signature value is not well-formed ASN.1: {0}")]
    Encoding(#[from] DerError),
    /// The outermost value is not RFC 5652's `ContentInfo SEQUENCE`.
    #[error("the signature value is not a CMS ContentInfo")]
    NotContentInfo,
    /// `ContentInfo`'s content type is something other than `id-signedData`.
    ///
    /// §12.8.3.3.1 requires "a DER-encoded CMS binary data object containing the signature", and
    /// every `/SubFilter` in Table 260 that is not `adbe.x509.rsa_sha1` means `SignedData`.
    #[error("the signature value is a CMS object that is not SignedData")]
    NotSignedData,
    /// `SignedData` is there but its members are not in RFC 5652's order or shape.
    #[error("the signature value's SignedData is malformed")]
    MalformedSignedData,
    /// There is no `SignerInfo`, so nothing states a digest algorithm.
    ///
    /// §12.8.3.4.3 (d) requires exactly one for a `PAdES` signature; RFC 5652 permits a `SET OF`
    /// with none, which carries no signature at all.
    #[error("the signature value carries no signer")]
    NoSigner,
}

/// What §12.8.3.3's signature value says, as far as this program reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedData<'a> {
    /// `encapContentInfo`'s `eContentType`, as its encoded object identifier.
    pub content_type: &'a [u8],
    /// `encapContentInfo`'s `eContent`, where one is present.
    ///
    /// Absent for a detached signature, which is what `adbe.pkcs7.detached` and
    /// `ETSI.CAdES.detached` are named for. Present for `adbe.pkcs7.sha1`, where §12.8.3.3.1 says
    /// what it holds — "[t]he SHA-1 digest of the document's byte range shall be encapsulated in
    /// the CMS `SignedData` field with `ContentInfo` of type Data" — and for a document timestamp,
    /// where it is RFC 3161's `TSTInfo`.
    pub encapsulated: Option<&'a [u8]>,
    /// The entries of the `certificates [0] IMPLICIT` member, unparsed.
    ///
    /// §12.8.3.3.1: "[a]t minimum the CMS object shall include the signer's X.509 signing
    /// certificate. This certificate shall be used to verify the signature value in Contents ."
    /// So an empty list is a file failing that requirement, and [`crate::x509::read`] is what
    /// turns one of these into a key. **This was a `usize` until the three-hundred-and-ninety-
    /// second session**, which was every fact a program that could not verify a signature had.
    pub certificates: Vec<Value<'a>>,
    /// How many `SignerInfo`s the `signerInfos SET OF` holds.
    pub signers: usize,
    /// The first signer's `digestAlgorithm`, where this program knows the algorithm.
    pub digest: Option<Digest>,
    /// That algorithm's object identifier, whether or not it is one of the six.
    pub digest_algorithm: &'a [u8],
    /// The first signer's `signatureAlgorithm`, as its encoded object identifier.
    ///
    /// RFC 5652 makes this the algorithm "used by the signer", and [`SignatureAlgorithm`] is what
    /// this program does with it. Kept whether or not it is recognised, so that a signature this
    /// program cannot verify can still say by what number it declines.
    pub signature_algorithm: &'a [u8],
    /// The first signer's `signature`, the octets that verification is over.
    pub signature: &'a [u8],
    /// The contents of the first signer's `signedAttrs [0] IMPLICIT`, where it states one.
    ///
    /// The *contents*, because RFC 5652 section 5.4 does not sign the bytes as they appear: "[a]
    /// separate encoding of the signedAttrs field is performed for message digest calculation.
    /// The IMPLICIT [0] tag in the signedAttrs is not used for the DER encoding, rather an
    /// EXPLICIT SET OF tag is used." [`SignedData::signed_attributes_encoding`] is that
    /// re-encoding.
    pub signed_attributes: Option<&'a [u8]>,
    /// The first signer's `sid`, where it is RFC 5652's `issuerAndSerialNumber`.
    ///
    /// The issuer `Name`'s contents and the serial number's, in that order — the pair a
    /// certificate is matched against by [`crate::x509::Certificate::is_named_by`].
    pub signer_issuer_and_serial: Option<IssuerAndSerial<'a>>,
    /// The first signer's `sid`, where it is instead `subjectKeyIdentifier [0]`.
    pub signer_key_identifier: Option<&'a [u8]>,
    /// The first signer's `message-digest` signed attribute — the digest of the signed content.
    pub message_digest: Option<&'a [u8]>,
    /// The object identifiers of the first signer's signed attributes, in the file's order.
    pub signed_attribute_types: Vec<&'a [u8]>,
    /// The same for its unsigned attributes.
    pub unsigned_attribute_types: Vec<&'a [u8]>,
    /// Whether either list stopped at [`MAX_ATTRIBUTES`] rather than at the file's end.
    pub attributes_truncated: bool,
}

impl<'a> SignedData<'a> {
    /// Whether the first signer states an attribute with this identifier.
    #[must_use]
    pub fn has_signed_attribute(&self, oid: &[u8]) -> bool {
        self.signed_attribute_types.contains(&oid)
    }

    /// Whether the first signer states an *unsigned* attribute with this identifier.
    #[must_use]
    pub fn has_unsigned_attribute(&self, oid: &[u8]) -> bool {
        self.unsigned_attribute_types.contains(&oid)
    }

    /// The bytes RFC 5652 section 5.4 says a signature over signed attributes is computed over.
    ///
    /// That clause requires "[t]he IMPLICIT [0] tag in the signedAttrs" not to be used for the DER
    /// encoding, "rather an EXPLICIT SET OF tag is used" — and it says which bytes go into the
    /// digest: "the DER encoding of the EXPLICIT SET OF tag, rather than of the IMPLICIT [0] tag,
    /// MUST be included in the message digest calculation along with the length and content octets
    /// of the `SignedAttributes` value."
    ///
    /// So the attributes are re-tagged as a `SET OF` — X.690's `31` in place of the `A0` the file
    /// wrote — with a definite length. The contents are the file's own bytes and are not
    /// re-ordered: DER requires a `SET OF`'s members to be sorted already, and re-sorting them
    /// here would change what a *non*-conforming producer signed and turn a verifiable signature
    /// into a failing one.
    ///
    /// `None` where the signer states no signed attributes, which RFC 5652 permits and which means
    /// the signature is over the content itself instead.
    #[must_use]
    pub fn signed_attributes_encoding(&self) -> Option<Vec<u8>> {
        let contents = self.signed_attributes?;
        let mut out = Vec::with_capacity(contents.len().saturating_add(6));
        out.push(SET);
        // X.690 clause 8.1.3: the short form below 128 octets, otherwise the long form with as
        // many length octets as the value needs. A signed-attributes set is a few hundred bytes.
        if contents.len() < 128 {
            out.push(u8::try_from(contents.len()).unwrap_or(0));
        } else {
            let length = contents.len().to_be_bytes();
            let significant = length
                .iter()
                .position(|&byte| byte != 0)
                .unwrap_or(length.len().saturating_sub(1));
            let octets = length.get(significant..).unwrap_or(&[]);
            out.push(0x80 | u8::try_from(octets.len()).unwrap_or(0));
            out.extend_from_slice(octets);
        }
        out.extend_from_slice(contents);
        Some(out)
    }

    /// The signer's public-key algorithm, as [`SignatureAlgorithm`] reads it.
    #[must_use]
    pub fn algorithm(&self) -> SignatureAlgorithm<'a> {
        SignatureAlgorithm::from_oid(self.signature_algorithm)
    }

    /// The digest RFC 3161's `TSTInfo` commits to, where this is a timestamp token.
    ///
    /// Table 255 states what a document timestamp's value is: "[t]he value of the messageImprint
    /// field within the `TimeStampToken` shall be a hash of the bytes of the document indicated by
    /// the `ByteRange`". `TSTInfo ::= SEQUENCE { version, policy, messageImprint, … }` and
    /// `MessageImprint ::= SEQUENCE { hashAlgorithm AlgorithmIdentifier, hashedMessage OCTET
    /// STRING }`, so the imprint is the third member's two parts.
    ///
    /// `None` where the encapsulated content is not a `TSTInfo`, where its algorithm is not one of
    /// the six, or where the encoding does not have that shape.
    #[must_use]
    pub fn timestamp_imprint(&self) -> Option<(Digest, &'a [u8])> {
        if self.content_type != ID_CT_TST_INFO {
            return None;
        }
        let mut reader = Reader::new(self.encapsulated?).ok()?;
        let info = reader.next_value().ok()??;
        let mut members = info.children().ok()?;
        // version INTEGER, policy OBJECT IDENTIFIER, messageImprint SEQUENCE.
        let _version = members.next_value().ok()??;
        let _policy = members.next_value().ok()??;
        let imprint = members.next_value().ok()??;
        let mut parts = imprint.children().ok()?;
        let algorithm = parts.next_value().ok()??;
        let hashed = parts.next_value().ok()??;
        if hashed.identifier != OCTET_STRING {
            return None;
        }
        let oid = algorithm.children().ok()?.next_value().ok()??;
        Some((Digest::from_oid(oid.object_identifier()?)?, hashed.contents))
    }
}

/// Reads a signature value as RFC 5652's `SignedData`.
///
/// # Errors
///
/// A [`CmsError`] naming what the value is instead. Nothing here is recovered from or guessed at:
/// a signature this reader cannot read is one it says it cannot read.
pub fn signed_data(bytes: &[u8]) -> Result<SignedData<'_>, CmsError> {
    let mut reader = Reader::new(bytes)?;
    let Some(content_info) = reader.next_value()? else {
        return Err(CmsError::NotContentInfo);
    };
    if content_info.identifier != SEQUENCE {
        return Err(CmsError::NotContentInfo);
    }
    let mut members = content_info.children()?;
    let Some(content_type) = members.next_value()? else {
        return Err(CmsError::NotContentInfo);
    };
    if content_type.object_identifier() != Some(ID_SIGNED_DATA) {
        return Err(CmsError::NotSignedData);
    }
    // `content [0] EXPLICIT ANY`: the tag wraps the SignedData rather than replacing its own.
    let Some(explicit) = members.next_value()? else {
        return Err(CmsError::MalformedSignedData);
    };
    if !explicit.is_context(0) {
        return Err(CmsError::MalformedSignedData);
    }
    let Some(signed) = explicit.children()?.next_value()? else {
        return Err(CmsError::MalformedSignedData);
    };
    read_signed_data(signed)
}

/// The members of `SignedData ::= SEQUENCE { version, digestAlgorithms, encapContentInfo,
/// certificates [0], crls [1], signerInfos }`.
///
/// Read by shape rather than by position, because `certificates` and `crls` are optional and a
/// positional reader would mistake an absent one for the member after it. The three that matter
/// each have a distinct tag at this level: `encapContentInfo` is the only `SEQUENCE`,
/// `certificates` the only `[0]`, and `signerInfos` the last `SET`.
fn read_signed_data(signed: Value<'_>) -> Result<SignedData<'_>, CmsError> {
    let mut members = signed.children()?;
    let mut content_type: &[u8] = &[];
    let mut encapsulated = None;
    let mut certificates = Vec::new();
    let mut signer_infos = None;
    while let Some(member) = members.next_value()? {
        if member.identifier == SEQUENCE && content_type.is_empty() {
            let mut parts = member.children()?;
            let Some(kind) = parts.next_value()? else {
                return Err(CmsError::MalformedSignedData);
            };
            let Some(kind) = kind.object_identifier() else {
                return Err(CmsError::MalformedSignedData);
            };
            content_type = kind;
            // `eContent [0] EXPLICIT OCTET STRING OPTIONAL`, so the octets are one level in.
            if let Some(wrapper) = parts.next_value()?
                && wrapper.is_context(0)
                && let Some(octets) = wrapper.children()?.next_value()?
                && octets.identifier == OCTET_STRING
            {
                encapsulated = Some(octets.contents);
            }
        } else if member.is_context(0) {
            let mut entries = member.children()?;
            while let Some(entry) = entries.next_value()? {
                if certificates.len() >= MAX_CERTIFICATES {
                    break;
                }
                certificates.push(entry);
            }
        } else if member.identifier == SET {
            signer_infos = Some(member);
        }
    }
    let Some(signer_infos) = signer_infos else {
        return Err(CmsError::NoSigner);
    };
    let mut signers = 0usize;
    let mut first = None;
    let mut entries = signer_infos.children()?;
    while let Some(entry) = entries.next_value()? {
        signers = signers.saturating_add(1);
        if first.is_none() {
            first = Some(entry);
        }
    }
    let Some(first) = first else {
        return Err(CmsError::NoSigner);
    };
    let parsed = read_signer_info(first)?;
    Ok(SignedData {
        content_type,
        encapsulated,
        certificates,
        signers,
        digest: Digest::from_oid(parsed.digest_algorithm),
        digest_algorithm: parsed.digest_algorithm,
        signature_algorithm: parsed.signature_algorithm,
        signature: parsed.signature,
        signed_attributes: parsed.signed_attributes,
        signer_issuer_and_serial: parsed.issuer_and_serial,
        signer_key_identifier: parsed.key_identifier,
        message_digest: parsed.message_digest,
        signed_attribute_types: parsed.signed_attribute_types,
        unsigned_attribute_types: parsed.unsigned_attribute_types,
        attributes_truncated: parsed.truncated,
    })
}

/// The parts of one `SignerInfo` this program reads.
struct Signer<'a> {
    digest_algorithm: &'a [u8],
    signature_algorithm: &'a [u8],
    signature: &'a [u8],
    signed_attributes: Option<&'a [u8]>,
    issuer_and_serial: Option<IssuerAndSerial<'a>>,
    key_identifier: Option<&'a [u8]>,
    message_digest: Option<&'a [u8]>,
    signed_attribute_types: Vec<&'a [u8]>,
    unsigned_attribute_types: Vec<&'a [u8]>,
    truncated: bool,
}

/// `SignerInfo ::= SEQUENCE { version, sid, digestAlgorithm, signedAttrs [0] IMPLICIT OPTIONAL,
/// signatureAlgorithm, signature, unsignedAttrs [1] IMPLICIT OPTIONAL }`.
///
/// **Positional, and it has to be**: `sid`, `digestAlgorithm` and `signatureAlgorithm` are all
/// `SEQUENCE`s when the signer is identified by issuer and serial number, which is the ordinary
/// case and every one of the corpus's. So the digest algorithm is the third member — the shape
/// that a first attempt at this module got wrong, reading `sid` as the algorithm and finding no
/// digest at all. The two members after it are found the same way, with the one optional member
/// between them distinguished by its tag rather than by counting.
fn read_signer_info(info: Value<'_>) -> Result<Signer<'_>, CmsError> {
    let mut members = info.children()?;
    let Some(version) = members.next_value()? else {
        return Err(CmsError::MalformedSignedData);
    };
    if version.identifier != INTEGER {
        return Err(CmsError::MalformedSignedData);
    }
    let Some(sid) = members.next_value()? else {
        return Err(CmsError::MalformedSignedData);
    };
    let Some(algorithm) = members.next_value()? else {
        return Err(CmsError::MalformedSignedData);
    };
    let Some(oid) = algorithm.children()?.next_value()? else {
        return Err(CmsError::MalformedSignedData);
    };
    let Some(digest_algorithm) = oid.object_identifier() else {
        return Err(CmsError::MalformedSignedData);
    };
    let mut signer = Signer {
        digest_algorithm,
        signature_algorithm: &[],
        signature: &[],
        signed_attributes: None,
        issuer_and_serial: read_signer_identifier(sid)?,
        key_identifier: sid.is_context(0).then_some(sid.contents),
        message_digest: None,
        signed_attribute_types: Vec::new(),
        unsigned_attribute_types: Vec::new(),
        truncated: false,
    };
    // The three members that follow, in RFC 5652's order: an optional `[0]`, then the signature
    // algorithm and the signature, then an optional `[1]`.
    let mut next = members.next_value()?;
    if let Some(member) = next
        && member.is_context(0)
    {
        signer.signed_attributes = Some(member.contents);
        read_attributes(member, true, &mut signer)?;
        next = members.next_value()?;
    }
    if let Some(member) = next
        && let Some(oid) = member.children()?.next_value()?
        && let Some(identifier) = oid.object_identifier()
    {
        signer.signature_algorithm = identifier;
    }
    if let Some(member) = members.next_value()?
        && member.identifier == OCTET_STRING
    {
        signer.signature = member.contents;
    }
    while let Some(member) = members.next_value()? {
        if member.is_context(1) {
            read_attributes(member, false, &mut signer)?;
        }
    }
    Ok(signer)
}

/// `SignerIdentifier ::= CHOICE { issuerAndSerialNumber IssuerAndSerialNumber,
/// subjectKeyIdentifier [0] SubjectKeyIdentifier }`, in its first spelling.
///
/// `IssuerAndSerialNumber ::= SEQUENCE { issuer Name, serialNumber CertificateSerialNumber }`, and
/// both members are handed back as the contents the file wrote so that
/// [`crate::x509::Certificate::is_named_by`] compares encodings rather than decoding a name.
fn read_signer_identifier(sid: Value<'_>) -> Result<Option<IssuerAndSerial<'_>>, CmsError> {
    if sid.identifier != SEQUENCE {
        return Ok(None);
    }
    let mut parts = sid.children()?;
    let (Some(issuer), Some(serial)) = (parts.next_value()?, parts.next_value()?) else {
        return Ok(None);
    };
    if serial.identifier != INTEGER {
        return Ok(None);
    }
    Ok(Some((issuer.contents, serial.contents)))
}

/// One `SET OF Attribute`, recording every identifier and the `message-digest`'s value.
///
/// `[0]` is `signedAttrs` and `[1]` is `unsignedAttrs`; both are `SET OF Attribute`, and the
/// IMPLICIT tag replaces the SET's own.
fn read_attributes<'a>(
    member: Value<'a>,
    is_signed: bool,
    signer: &mut Signer<'a>,
) -> Result<(), CmsError> {
    let mut attributes = member.children()?;
    let mut names = Vec::new();
    while let Some(attribute) = attributes.next_value()? {
        if names.len() >= MAX_ATTRIBUTES {
            signer.truncated = true;
            break;
        }
        let mut parts = attribute.children()?;
        let Some(kind) = parts.next_value()? else {
            continue;
        };
        let Some(kind) = kind.object_identifier() else {
            continue;
        };
        names.push(kind);
        if is_signed && kind == ID_MESSAGE_DIGEST {
            // `AttributeValue ::= ANY`, in a `SET OF` of one: the digest is the octets of the
            // single value inside.
            if let Some(values) = parts.next_value()?
                && let Some(octets) = values.children()?.next_value()?
                && octets.identifier == OCTET_STRING
            {
                signer.message_digest = Some(octets.contents);
            }
        }
    }
    if is_signed {
        signer.signed_attribute_types = names;
    } else {
        signer.unsigned_attribute_types = names;
    }
    Ok(())
}

/// Hand-built signature values, shared with `signature.rs`'s tests.
///
/// **A fixture rather than a corpus document, and the reason is trap 8's.** Four of the six
/// signature formats §12.8.3 defines have no witness in the 974 — no document timestamp, no `PAdES`
/// signature, no `adbe.x509.rsa_sha1`, and nothing using four of Table 260's six digests — so a
/// corpus can rank none of them. What it *can* do is confirm the reading on the two it has, which
/// `tests/signatures.rs` does over all ten of its signature dictionaries.
#[cfg(test)]
pub(crate) mod fixtures {
    use super::{ID_CONTENT_TYPE, ID_CT_TST_INFO, ID_DATA, ID_MESSAGE_DIGEST, ID_SIGNING_TIME};

    /// A DER `SEQUENCE`, `SET` or context tag around already-encoded children.
    fn tagged(identifier: u8, children: &[Vec<u8>]) -> Vec<u8> {
        primitive(identifier, &children.concat())
    }

    /// One value with the given identifier and contents, in X.690's shortest length form.
    ///
    /// The fixtures reach a few hundred bytes once three certificates are in them, so the
    /// one-octet long form is spelled as well — and that is all: a builder that went further
    /// would be a second encoder to keep right.
    fn primitive(identifier: u8, contents: &[u8]) -> Vec<u8> {
        let mut out = vec![identifier];
        if contents.len() < 128 {
            out.push(u8::try_from(contents.len()).unwrap_or(0));
        } else {
            assert!(contents.len() < 256, "the fixtures are all short");
            out.push(0x81);
            out.push(u8::try_from(contents.len()).unwrap_or(0));
        }
        out.extend_from_slice(contents);
        out
    }

    /// The `AlgorithmIdentifier` for SHA-256.
    fn sha256_algorithm() -> Vec<u8> {
        tagged(
            0x30,
            &[primitive(
                0x06,
                &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01],
            )],
        )
    }

    /// One signed attribute: `SEQUENCE { OID, SET OF value }`.
    fn attribute(oid: &[u8], value: Vec<u8>) -> Vec<u8> {
        tagged(0x30, &[primitive(0x06, oid), tagged(0x31, &[value])])
    }

    /// One `SignerInfo`, with the signed attributes given.
    fn signer(attributes: Option<Vec<Vec<u8>>>) -> Vec<u8> {
        let mut members = vec![
            primitive(0x02, &[0x01]),                  // version
            tagged(0x30, &[primitive(0x02, &[0x2A])]), // sid: issuer and serial number
            sha256_algorithm(),                        // digestAlgorithm
        ];
        if let Some(attributes) = attributes {
            members.push(tagged(0xA0, &attributes));
        }
        members.push(sha256_algorithm()); // signatureAlgorithm
        members.push(primitive(0x04, &[0xDE, 0xAD])); // signature
        tagged(0x30, &members)
    }

    /// `ContentInfo { id-signedData, [0] SignedData }` around one signer.
    fn content_info(content_type: &[u8], encapsulated: Option<&[u8]>, signer: Vec<u8>) -> Vec<u8> {
        let mut encapsulated_info = vec![primitive(0x06, content_type)];
        if let Some(content) = encapsulated {
            encapsulated_info.push(tagged(0xA0, &[primitive(0x04, content)]));
        }
        let body = tagged(
            0x30,
            &[
                primitive(0x02, &[0x01]),
                tagged(0x31, &[sha256_algorithm()]),
                tagged(0x30, &encapsulated_info),
                // Two certificates, stood in for by empty sequences: this program counts them
                // and reads none, so their contents would be a fiction with no reader.
                tagged(0xA0, &[primitive(0x30, &[0x00]), primitive(0x30, &[0x00])]),
                tagged(0x31, &[signer]),
            ],
        );
        tagged(
            0x30,
            &[
                primitive(
                    0x06,
                    &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02],
                ),
                tagged(0xA0, &[body]),
            ],
        )
    }

    /// A detached `adbe.pkcs7.detached` signature value, with a message digest of `digest`.
    ///
    /// Carries a `signing-time` attribute as well, because §12.8.3.4.2 forbids stating that
    /// *and* the signature dictionary's `/M`, and a fixture with only one of the two could not
    /// exercise the rule.
    pub(crate) fn detached(digest: &[u8]) -> Vec<u8> {
        content_info(
            ID_DATA,
            None,
            signer(Some(vec![
                attribute(ID_CONTENT_TYPE, primitive(0x06, ID_DATA)),
                attribute(ID_SIGNING_TIME, primitive(0x17, b"260807000000Z")),
                attribute(ID_MESSAGE_DIGEST, primitive(0x04, digest)),
            ])),
        )
    }

    /// The same with no signed attributes at all, which RFC 5652 permits.
    ///
    /// `bug854315.pdf` is this shape. There is then no `message-digest` recording the document's
    /// digest, so the only thing that could answer question 1 is question 2's public key.
    pub(crate) fn without_signed_attributes() -> Vec<u8> {
        content_info(ID_DATA, None, signer(None))
    }

    /// An `adbe.pkcs7.sha1` value, which encapsulates the document's digest rather than
    /// detaching from it.
    ///
    /// §12.8.3.3.1: "[t]he SHA-1 digest of the document's byte range shall be encapsulated in the
    /// CMS `SignedData` field with `ContentInfo` of type Data. The digest of that `SignedData`
    /// shall be incorporated as the normal CMS digest." So the `message-digest` attribute here is
    /// a digest *of the digest*, and it is written deliberately wrong: a reader that reached for
    /// it would compare the wrong two values and report a document that had not changed as one
    /// that had.
    pub(crate) fn encapsulating(digest: &[u8]) -> Vec<u8> {
        content_info(
            ID_DATA,
            Some(digest),
            signer(Some(vec![
                attribute(ID_CONTENT_TYPE, primitive(0x06, ID_DATA)),
                attribute(ID_MESSAGE_DIGEST, primitive(0x04, &[0xFF; 32])),
            ])),
        )
    }

    /// An `ETSI.RFC3161` timestamp token committing to `digest`.
    ///
    /// `TSTInfo ::= SEQUENCE { version, policy, messageImprint, … }`, encapsulated as
    /// `id-ct-TSTInfo`. Table 255: "[t]he value of the messageImprint field within the
    /// `TimeStampToken` shall be a hash of the bytes of the document indicated by the `ByteRange`".
    pub(crate) fn timestamp_token(digest: &[u8]) -> Vec<u8> {
        let info = tagged(
            0x30,
            &[
                primitive(0x02, &[0x01]),       // version
                primitive(0x06, &[0x2A, 0x03]), // policy
                tagged(
                    0x30,
                    &[sha256_algorithm(), primitive(0x04, digest)], // messageImprint
                ),
            ],
        );
        content_info(ID_CT_TST_INFO, Some(&info), signer(None))
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::detached;
    use super::{CmsError, Digest, ID_DATA, ID_MESSAGE_DIGEST, ID_SIGNING_TIME, signed_data};

    /// A DER `SEQUENCE` around already-encoded children, for the two malformed fixtures below.
    fn tagged(identifier: u8, children: &[Vec<u8>]) -> Vec<u8> {
        primitive(identifier, &children.concat())
    }

    /// One value with the given identifier and contents, in X.690's short length form.
    fn primitive(identifier: u8, contents: &[u8]) -> Vec<u8> {
        let mut out = vec![identifier];
        assert!(contents.len() < 128, "the malformed fixtures are short");
        out.push(u8::try_from(contents.len()).unwrap_or(0));
        out.extend_from_slice(contents);
        out
    }

    /// The one thing this module exists to find, found.
    #[test]
    fn a_detached_signature_states_its_algorithm_and_its_message_digest() {
        let expected = Digest::Sha256.compute(&[b"the signed bytes"]);
        let bytes = detached(&expected);
        let cms = signed_data(&bytes).expect("a SignedData");
        assert_eq!(cms.digest, Some(Digest::Sha256));
        assert_eq!(cms.message_digest, Some(expected.as_slice()));
        assert_eq!(cms.content_type, ID_DATA, "§12.8.3.4.3 (a)'s id-data");
        assert_eq!(cms.encapsulated, None, "detached: no encapsulated content");
        assert_eq!(cms.signers, 1, "§12.8.3.4.3 (d)'s one SignerInfo");
        assert_eq!(cms.certificates.len(), 2);
        assert!(cms.has_signed_attribute(ID_MESSAGE_DIGEST));
        assert!(cms.has_signed_attribute(ID_SIGNING_TIME));
        assert!(!cms.has_unsigned_attribute(ID_MESSAGE_DIGEST));
        assert!(!cms.attributes_truncated);
    }

    /// `digestAlgorithm` is the *third* member of a `SignerInfo`, not the first `SEQUENCE`.
    ///
    /// `sid` is a `SEQUENCE` too whenever the signer is named by issuer and serial number, which
    /// is every signature in the corpus. Reading by shape rather than by position finds the
    /// issuer's `SEQUENCE` and reports no digest at all — a wrong answer wearing the shape of an
    /// unsupported one, which is the failure this test pins.
    #[test]
    fn the_signers_own_sequence_is_not_mistaken_for_its_digest_algorithm() {
        let bytes = detached(&[0x00; 32]);
        let cms = signed_data(&bytes).expect("a SignedData");
        assert_eq!(
            cms.digest_algorithm,
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01],
            "the third member's algorithm identifier and not the second's"
        );
    }

    /// The six algorithms Table 260 and Table 256 name, and one they do not.
    #[test]
    fn the_digest_algorithms_are_the_ones_the_tables_name() {
        assert_eq!(
            Digest::from_oid(&[0x2B, 0x0E, 0x03, 0x02, 0x1A]),
            Some(Digest::Sha1)
        );
        assert_eq!(
            Digest::from_oid(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x02, 0x05]),
            Some(Digest::Md5)
        );
        assert_eq!(
            Digest::from_oid(&[0x2B, 0x24, 0x03, 0x02, 0x01]),
            Some(Digest::Ripemd160)
        );
        assert_eq!(Digest::from_oid(&[0x2A, 0x03]), None);

        // Published vectors, so that the six names are attached to the six functions rather than
        // to whichever crate happened to be listed beside them.
        assert_eq!(
            hex(&Digest::Sha1.compute(&[b"abc"])),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hex(&Digest::Sha256.compute(&[b"ab", b"c"])),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "and the pieces are hashed as one message"
        );
        assert_eq!(
            hex(&Digest::Md5.compute(&[b"abc"])),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            hex(&Digest::Ripemd160.compute(&[b"abc"])),
            "8eb208f7e05d987a9b044a8e98c6b087f15a0bfc"
        );
    }

    /// Lower-case hexadecimal, for comparing a digest with a published vector.
    fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write as _;
        bytes.iter().fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
    }

    /// Every way a signature value can fail to be one, named rather than shrugged at.
    #[test]
    fn what_is_not_a_signed_data_says_what_it_is() {
        assert_eq!(signed_data(&[]), Err(CmsError::NotContentInfo));
        assert_eq!(
            signed_data(&[0x04, 0x01, 0x00]),
            Err(CmsError::NotContentInfo)
        );
        // A ContentInfo whose type is id-data rather than id-signedData.
        let plain = tagged(0x30, &[primitive(0x06, ID_DATA)]);
        assert_eq!(signed_data(&plain), Err(CmsError::NotSignedData));
        // A SignedData with an empty signerInfos.
        let empty = tagged(
            0x30,
            &[
                primitive(
                    0x06,
                    &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02],
                ),
                tagged(
                    0xA0,
                    &[tagged(
                        0x30,
                        &[
                            primitive(0x02, &[0x01]),
                            tagged(0x30, &[primitive(0x06, ID_DATA)]),
                            tagged(0x31, &[]),
                        ],
                    )],
                ),
            ],
        );
        assert_eq!(signed_data(&empty), Err(CmsError::NoSigner));
    }
}

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
//! Everything else in RFC 5652 is deliberately not read: the certificates are X.509 and a trust
//! decision, and the signature value itself needs the signer's public key. Both are named at
//! runtime rather than skipped in silence — see [`crate::signature::Integrity`].
//!
//! # What a matching digest proves, and what it does not
//!
//! **A mismatch is decisive and a match is not.** The digest sits beside the signature rather than
//! inside it, so anyone who alters the document can alter the recorded digest to match — what they
//! cannot do is make the *signature* over that digest verify, and checking that is the question
//! this module does not answer. So a difference proves the document changed after signing, which
//! is exactly what §12.8.1 says it indicates, and agreement proves only that nothing changed
//! carelessly. The wording the program uses to a person keeps the two apart.

use crate::der::{DerError, INTEGER, OCTET_STRING, Reader, SEQUENCE, SET, Value};

/// How many of a signer's attributes are recorded.
///
/// RFC 5652 puts no bound on `SignedAttributes`, and this is the one allocation in the module that
/// a file's contents drive. §12.8.3.4.3 lists eleven attributes a `PAdES` signature may carry, so a
/// signature with more than sixty-four has stopped being one this reader has anything to say
/// about; the ones past the bound are dropped and [`SignedData::attributes_truncated`] says so.
const MAX_ATTRIBUTES: usize = 64;

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
    /// How many entries the `certificates [0] IMPLICIT` member holds.
    ///
    /// Counted and not parsed: §12.8.3.3.1 says "[a]t minimum the CMS object shall include the
    /// signer's X.509 signing certificate", so zero is a file failing that requirement and a
    /// number is what a program without a trust store can say about the rest.
    pub certificates: usize,
    /// How many `SignerInfo`s the `signerInfos SET OF` holds.
    pub signers: usize,
    /// The first signer's `digestAlgorithm`, where this program knows the algorithm.
    pub digest: Option<Digest>,
    /// That algorithm's object identifier, whether or not it is one of the six.
    pub digest_algorithm: &'a [u8],
    /// The first signer's `message-digest` signed attribute — the digest of the signed content.
    pub message_digest: Option<&'a [u8]>,
    /// The object identifiers of the first signer's signed attributes, in the file's order.
    pub signed_attributes: Vec<&'a [u8]>,
    /// The same for its unsigned attributes.
    pub unsigned_attributes: Vec<&'a [u8]>,
    /// Whether either list stopped at [`MAX_ATTRIBUTES`] rather than at the file's end.
    pub attributes_truncated: bool,
}

impl<'a> SignedData<'a> {
    /// Whether the first signer states an attribute with this identifier.
    #[must_use]
    pub fn has_signed_attribute(&self, oid: &[u8]) -> bool {
        self.signed_attributes.contains(&oid)
    }

    /// Whether the first signer states an *unsigned* attribute with this identifier.
    #[must_use]
    pub fn has_unsigned_attribute(&self, oid: &[u8]) -> bool {
        self.unsigned_attributes.contains(&oid)
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
    let mut certificates = 0usize;
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
            while entries.next_value()?.is_some() {
                certificates = certificates.saturating_add(1);
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
        message_digest: parsed.message_digest,
        signed_attributes: parsed.signed_attributes,
        unsigned_attributes: parsed.unsigned_attributes,
        attributes_truncated: parsed.truncated,
    })
}

/// The parts of one `SignerInfo` this program reads.
struct Signer<'a> {
    digest_algorithm: &'a [u8],
    message_digest: Option<&'a [u8]>,
    signed_attributes: Vec<&'a [u8]>,
    unsigned_attributes: Vec<&'a [u8]>,
    truncated: bool,
}

/// `SignerInfo ::= SEQUENCE { version, sid, digestAlgorithm, signedAttrs [0] IMPLICIT OPTIONAL,
/// signatureAlgorithm, signature, unsignedAttrs [1] IMPLICIT OPTIONAL }`.
///
/// **Positional, and it has to be**: `sid`, `digestAlgorithm` and `signatureAlgorithm` are all
/// `SEQUENCE`s when the signer is identified by issuer and serial number, which is the ordinary
/// case and every one of the corpus's. So the digest algorithm is the third member — the shape
/// that a first attempt at this module got wrong, reading `sid` as the algorithm and finding no
/// digest at all.
fn read_signer_info(info: Value<'_>) -> Result<Signer<'_>, CmsError> {
    let mut members = info.children()?;
    let Some(version) = members.next_value()? else {
        return Err(CmsError::MalformedSignedData);
    };
    if version.identifier != INTEGER {
        return Err(CmsError::MalformedSignedData);
    }
    let Some(_sid) = members.next_value()? else {
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
        message_digest: None,
        signed_attributes: Vec::new(),
        unsigned_attributes: Vec::new(),
        truncated: false,
    };
    while let Some(member) = members.next_value()? {
        // `[0]` is signedAttrs and `[1]` unsignedAttrs; both are `SET OF Attribute`, and the
        // IMPLICIT tag replaces the SET's own.
        let is_signed = member.is_context(0);
        if !is_signed && !member.is_context(1) {
            continue;
        }
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
            signer.signed_attributes = names;
        } else {
            signer.unsigned_attributes = names;
        }
    }
    Ok(signer)
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
        assert_eq!(cms.certificates, 2);
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

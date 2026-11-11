//! RFC 5652 section 6's `EnvelopedData`: read, and opened for one recipient.
//!
//! ISO 32000-2 §7.6.5.1 binds the public-key security handler to this structure — "Public-key
//! security handlers use the industry standard Public Key Cryptographic Standard Number 7 (CMS)
//! binary encoding syntax to encode recipient list, decryption key, and access permission
//! information" — and §7.6.5.3 says what is inside it: a key "encrypted for each recipient, using
//! that recipient's public key", and the enveloped data that key decrypts, "a byte array" holding
//! the seed and the permissions. This module is the CMS half of that sentence and knows nothing of
//! PDF: it reads the structure, finds the recipient a certificate names, and returns the bytes
//! the envelope was protecting. What those bytes mean is `pdf_syntax::crypt`'s.
//!
//! # What is read, and what is refused by name
//!
//! RFC 5652 section 6.2 defines five recipient kinds. Key transport (`KeyTransRecipientInfo`,
//! section 6.2.1) is the one a recipient's certificate and RSA private key can open, and the only
//! one any public-key document this tree has seen uses; the other four are read as what they are
//! and are never a match, because none of them can name a certificate's RSA key (section 6.2.2's
//! key agreement needs a Diffie–Hellman key, 6.2.3 a symmetric key, 6.2.4 a password). Of key
//! transport's algorithms, RFC 8017's `rsaEncryption` is opened ([`crate::rsa`]); anything else
//! is refused with its object identifier.
//!
//! §7.6.5.3 lists the ciphers that "shall be used to encrypt the enveloped data": RC4, DES, Triple
//! DES and RC2, all four "deprecated in PDF 2.0", and AES in CBC mode at 128, 192 and 256 bits.
//! The three AES widths are decrypted here, with RFC 3565 section 2.3's parameters — the
//! initialisation vector as an `OCTET STRING` of sixteen octets. The four deprecated ciphers are
//! refused with the cipher's name ([`EnvelopeError::Cipher`]), which is a gap and is recorded as
//! one in §7.6.5.3's ledger row.

use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockModeDecrypt, KeyIvInit};
use aes::{Aes128, Aes192, Aes256};

use crate::der::{DerError, OCTET_STRING, Reader, SEQUENCE, SET, Value};
use crate::rsa::PrivateKey;
use crate::x509::{Certificate, RSA_ENCRYPTION};

/// RFC 5652 section 6.1's `id-envelopedData`, 1.2.840.113549.1.7.3.
pub const ID_ENVELOPED_DATA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x03];

/// RFC 3565 section 4's `id-aes128-CBC`, 2.16.840.1.101.3.4.1.2.
pub const ID_AES128_CBC: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01, 0x02];
/// RFC 3565 section 4's `id-aes192-CBC`, 2.16.840.1.101.3.4.1.22.
pub const ID_AES192_CBC: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01, 0x16];
/// RFC 3565 section 4's `id-aes256-CBC`, 2.16.840.1.101.3.4.1.42.
pub const ID_AES256_CBC: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01, 0x2A];

/// The deprecated ciphers §7.6.5.3 lists, by the identifiers RFC 3370 and RFC 8018 give them, so
/// that a refusal can say which one a document chose.
const DEPRECATED: &[(&[u8], &str)] = &[
    // RFC 8018 appendix B.2.3, `rc2CBC`, 1.2.840.113549.3.2.
    (
        &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x03, 0x02],
        "RC2 in CBC mode",
    ),
    // RFC 8018 appendix B.2.2, `des-EDE3-CBC`, 1.2.840.113549.3.7.
    (
        &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x03, 0x07],
        "Triple DES in CBC mode",
    ),
    // RFC 8018 appendix B.2.1, `desCBC`, 1.3.14.3.2.7.
    (&[0x2B, 0x0E, 0x03, 0x02, 0x07], "DES in CBC mode"),
    // RSA Security's `rc4`, 1.2.840.113549.3.4.
    (&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x03, 0x04], "RC4"),
];

/// The AES block, and RFC 3565 section 2.3's initialisation-vector length.
const AES_BLOCK: usize = 16;

/// Why an envelope could not be read or opened.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EnvelopeError {
    /// The bytes are not DER this crate can read.
    #[error("the CMS object does not parse as DER: {0}")]
    Der(DerError),
    /// The bytes are DER but not RFC 5652's `ContentInfo` around an `EnvelopedData`; the string
    /// names the member that is missing or of the wrong type.
    #[error("the CMS object is not an RFC 5652 EnvelopedData: {0}")]
    Malformed(&'static str),
    /// No key-transport recipient names this certificate, by issuer and serial number or by
    /// subject key identifier (RFC 5652 section 6.2.1's `rid`).
    #[error("no recipient of the CMS object is this certificate")]
    NotARecipient,
    /// The recipient's key is transported under an algorithm other than `rsaEncryption`; the
    /// string is the object identifier in dotted form.
    #[error("the recipient's key is encrypted under {0}, which this reader does not decrypt")]
    KeyTransport(String),
    /// The content is encrypted under a cipher this reader does not decrypt; the string names it.
    #[error("the enveloped data is encrypted with {0}, which this reader does not decrypt")]
    Cipher(String),
    /// The recipient's key or the content did not decrypt: a wrong private key, or a damaged
    /// envelope. One error for every case, as RFC 8017 section 7.2.2 asks of the key's decoding.
    #[error("the CMS object did not decrypt under this private key")]
    Decryption,
}

impl From<DerError> for EnvelopeError {
    fn from(error: DerError) -> Self {
        Self::Der(error)
    }
}

/// How RFC 5652 section 6.2.1's `rid` names the recipient's certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipientIdentifier<'a> {
    /// `issuerAndSerialNumber`: the `Name`'s contents and the `INTEGER`'s contents.
    IssuerAndSerialNumber {
        /// The issuer `Name`'s contents, compared as encoded bytes.
        issuer: &'a [u8],
        /// The serial number `INTEGER`'s contents.
        serial: &'a [u8],
    },
    /// `subjectKeyIdentifier`, `[0]`: the certificate's subject key identifier extension value.
    SubjectKeyIdentifier(&'a [u8]),
}

impl RecipientIdentifier<'_> {
    /// Whether this identifier names `certificate` — §7.6.5.1's "match with a certificate".
    #[must_use]
    pub fn names(&self, certificate: &Certificate<'_>) -> bool {
        match *self {
            Self::IssuerAndSerialNumber { issuer, serial } => {
                certificate.is_named_by(issuer, serial)
            }
            Self::SubjectKeyIdentifier(identifier) => {
                certificate.key_identifier == Some(identifier)
            }
        }
    }
}

/// One `RecipientInfo` (RFC 5652 section 6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipientInfo<'a> {
    /// `ktri`, section 6.2.1.
    KeyTransport {
        /// `rid`, the certificate this entry is for.
        recipient: RecipientIdentifier<'a>,
        /// `keyEncryptionAlgorithm`'s object identifier.
        algorithm: &'a [u8],
        /// `encryptedKey`: the content-encryption key under the recipient's public key.
        encrypted_key: &'a [u8],
    },
    /// `kari`, `kekri`, `pwri` or `ori` — sections 6.2.2 to 6.2.5 — by the context tag number
    /// that chose it. None of them can be opened with a certificate's RSA key.
    Other(u8),
}

/// RFC 5652 section 6.1's `EnvelopedData`, as far as opening it for a recipient needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopedData<'a> {
    /// `recipientInfos`, in the order the encoding states them.
    pub recipients: Vec<RecipientInfo<'a>>,
    /// `encryptedContentInfo.contentEncryptionAlgorithm`'s object identifier.
    pub content_algorithm: &'a [u8],
    /// The same algorithm identifier's parameters, where it states any.
    pub content_parameters: Option<Value<'a>>,
    /// `encryptedContentInfo.encryptedContent`, the octets of a constructed encoding joined.
    pub encrypted_content: Vec<u8>,
}

/// Reads RFC 5652's `ContentInfo` whose content is an `EnvelopedData` (sections 3 and 6.1).
///
/// # Errors
///
/// [`EnvelopeError::Der`] for bytes that are not DER, and [`EnvelopeError::Malformed`] naming the
/// member that is not what RFC 5652 states.
pub fn parse(bytes: &[u8]) -> Result<EnvelopedData<'_>, EnvelopeError> {
    let mut reader = Reader::new(bytes)?;
    let content_info = expect(&mut reader, SEQUENCE, "ContentInfo")?;
    let mut members = content_info.children()?;
    let content_type = members
        .next_value()?
        .and_then(|value| value.object_identifier())
        .ok_or(EnvelopeError::Malformed("contentType"))?;
    if content_type != ID_ENVELOPED_DATA {
        return Err(EnvelopeError::Malformed(
            "contentType is not id-envelopedData",
        ));
    }
    // "content [0] EXPLICIT ANY DEFINED BY contentType" (section 3).
    let explicit = members
        .next_value()?
        .filter(|value| value.is_context(0) && value.is_constructed())
        .ok_or(EnvelopeError::Malformed("content"))?;
    let enveloped = expect(&mut explicit.children()?, SEQUENCE, "EnvelopedData")?;
    let mut members = enveloped.children()?;
    let _version = members
        .next_value()?
        .ok_or(EnvelopeError::Malformed("version"))?;
    let mut next = members
        .next_value()?
        .ok_or(EnvelopeError::Malformed("recipientInfos"))?;
    // "originatorInfo [0] IMPLICIT OriginatorInfo OPTIONAL": certificates for key agreement,
    // which no key-transport recipient needs.
    if next.is_context(0) {
        next = members
            .next_value()?
            .ok_or(EnvelopeError::Malformed("recipientInfos"))?;
    }
    if next.identifier != SET {
        return Err(EnvelopeError::Malformed("recipientInfos"));
    }
    let mut recipients = Vec::new();
    let mut infos = next.children()?;
    while let Some(info) = infos.next_value()? {
        recipients.push(recipient_info(info)?);
    }
    if recipients.is_empty() {
        // "RecipientInfos ::= SET SIZE (1..MAX) OF RecipientInfo".
        return Err(EnvelopeError::Malformed("recipientInfos is empty"));
    }
    let encrypted = expect(&mut members, SEQUENCE, "encryptedContentInfo")?;
    let mut parts = encrypted.children()?;
    let _content_type = parts
        .next_value()?
        .ok_or(EnvelopeError::Malformed("encryptedContentInfo.contentType"))?;
    let algorithm = expect(&mut parts, SEQUENCE, "contentEncryptionAlgorithm")?;
    let mut algorithm_parts = algorithm.children()?;
    let content_algorithm = algorithm_parts
        .next_value()?
        .and_then(|value| value.object_identifier())
        .ok_or(EnvelopeError::Malformed("contentEncryptionAlgorithm"))?;
    let content_parameters = algorithm_parts.next_value()?;
    // "encryptedContent [0] IMPLICIT EncryptedContent OPTIONAL". Absent, the content is carried
    // elsewhere (section 6.1), which §7.6.5.3's "the enveloped data contained in the CMS object"
    // does not allow, so an absence is malformed here rather than empty.
    let content = parts
        .next_value()?
        .filter(|value| value.is_context(0))
        .ok_or(EnvelopeError::Malformed("encryptedContent"))?;
    Ok(EnvelopedData {
        recipients,
        content_algorithm,
        content_parameters,
        encrypted_content: octets(content)?,
    })
}

impl EnvelopedData<'_> {
    /// The first key-transport recipient whose `rid` names `certificate`, as §7.6.5.1's reader
    /// "shall attempt to find a match with a certificate that belongs to the user".
    #[must_use]
    pub fn recipient(&self, certificate: &Certificate<'_>) -> Option<RecipientInfo<'_>> {
        self.recipients.iter().copied().find(|info| match info {
            RecipientInfo::KeyTransport { recipient, .. } => recipient.names(certificate),
            RecipientInfo::Other(_) => false,
        })
    }

    /// Opens the envelope for `certificate`'s holder, whose private key is `key`: RFC 5652
    /// section 6.2.1's key transport undone with RFC 8017's `RSAES-PKCS1-v1_5`, then section
    /// 6.3's content decryption.
    ///
    /// # Errors
    ///
    /// [`EnvelopeError::NotARecipient`] where no recipient names the certificate,
    /// [`EnvelopeError::KeyTransport`] and [`EnvelopeError::Cipher`] naming an algorithm this
    /// reader does not decrypt, and [`EnvelopeError::Decryption`] where the key does not open it.
    pub fn open(
        &self,
        certificate: &Certificate<'_>,
        key: &PrivateKey,
    ) -> Result<Vec<u8>, EnvelopeError> {
        let Some(RecipientInfo::KeyTransport {
            algorithm,
            encrypted_key,
            ..
        }) = self.recipient(certificate)
        else {
            return Err(EnvelopeError::NotARecipient);
        };
        if algorithm != RSA_ENCRYPTION {
            return Err(EnvelopeError::KeyTransport(dotted(algorithm)));
        }
        let content_key = key
            .decrypt(encrypted_key)
            .map_err(|_| EnvelopeError::Decryption)?;
        self.decrypt_content(&content_key)
    }

    /// RFC 5652 section 6.3 run backwards under `content_key`, with section 6.3's padding — "k-(lth
    /// mod k) octets all having value k-(lth mod k)" — removed.
    fn decrypt_content(&self, content_key: &[u8]) -> Result<Vec<u8>, EnvelopeError> {
        let width = match self.content_algorithm {
            id if id == ID_AES128_CBC => 16,
            id if id == ID_AES192_CBC => 24,
            id if id == ID_AES256_CBC => 32,
            other => {
                let name = DEPRECATED
                    .iter()
                    .find(|(id, _)| *id == other)
                    .map_or_else(|| dotted(other), |(_, name)| (*name).to_owned());
                return Err(EnvelopeError::Cipher(name));
            }
        };
        // RFC 3565 section 2.3: "AES-IV ::= OCTET STRING (SIZE(16))".
        let iv = self
            .content_parameters
            .filter(|value| value.identifier == OCTET_STRING)
            .map(|value| value.contents)
            .filter(|iv| iv.len() == AES_BLOCK)
            .ok_or(EnvelopeError::Malformed("AES-IV"))?;
        if content_key.len() != width {
            return Err(EnvelopeError::Decryption);
        }
        let body = self.encrypted_content.as_slice();
        if body.is_empty() || !body.len().is_multiple_of(AES_BLOCK) {
            return Err(EnvelopeError::Decryption);
        }
        let mut out = vec![0u8; body.len()];
        let length = match width {
            16 => cbc::Decryptor::<Aes128>::new_from_slices(content_key, iv)
                .map_err(|_| EnvelopeError::Decryption)?
                .decrypt_padded_b2b::<Pkcs7>(body, &mut out)
                .map(<[u8]>::len),
            24 => cbc::Decryptor::<Aes192>::new_from_slices(content_key, iv)
                .map_err(|_| EnvelopeError::Decryption)?
                .decrypt_padded_b2b::<Pkcs7>(body, &mut out)
                .map(<[u8]>::len),
            _ => cbc::Decryptor::<Aes256>::new_from_slices(content_key, iv)
                .map_err(|_| EnvelopeError::Decryption)?
                .decrypt_padded_b2b::<Pkcs7>(body, &mut out)
                .map(<[u8]>::len),
        }
        .map_err(|_| EnvelopeError::Decryption)?;
        out.truncate(length);
        Ok(out)
    }
}

/// One `RecipientInfo` CHOICE (section 6.2): `ktri` is the untagged `SEQUENCE`, the other four
/// are context-tagged.
fn recipient_info(info: Value<'_>) -> Result<RecipientInfo<'_>, EnvelopeError> {
    if info.identifier != SEQUENCE {
        return Ok(RecipientInfo::Other(info.tag_number()));
    }
    let mut members = info.children()?;
    let _version = members
        .next_value()?
        .ok_or(EnvelopeError::Malformed("KeyTransRecipientInfo.version"))?;
    let rid = members
        .next_value()?
        .ok_or(EnvelopeError::Malformed("KeyTransRecipientInfo.rid"))?;
    let recipient = if rid.identifier == SEQUENCE {
        // "IssuerAndSerialNumber ::= SEQUENCE { issuer Name, serialNumber CertificateSerialNumber }"
        // (section 10.2.4).
        let mut pair = rid.children()?;
        let issuer = expect(&mut pair, SEQUENCE, "IssuerAndSerialNumber.issuer")?;
        let serial = expect(
            &mut pair,
            crate::der::INTEGER,
            "IssuerAndSerialNumber.serialNumber",
        )?;
        RecipientIdentifier::IssuerAndSerialNumber {
            issuer: issuer.contents,
            serial: serial.contents,
        }
    } else if rid.is_context(0) && !rid.is_constructed() {
        // "subjectKeyIdentifier [0] SubjectKeyIdentifier", an implicitly tagged OCTET STRING.
        RecipientIdentifier::SubjectKeyIdentifier(rid.contents)
    } else {
        return Err(EnvelopeError::Malformed("KeyTransRecipientInfo.rid"));
    };
    let algorithm = expect(&mut members, SEQUENCE, "keyEncryptionAlgorithm")?
        .children()?
        .next_value()?
        .and_then(|value| value.object_identifier())
        .ok_or(EnvelopeError::Malformed("keyEncryptionAlgorithm"))?;
    let encrypted_key = expect(&mut members, OCTET_STRING, "encryptedKey")?;
    Ok(RecipientInfo::KeyTransport {
        recipient,
        algorithm,
        encrypted_key: encrypted_key.contents,
    })
}

/// The next value, which must carry `identifier`; `member` names it in the refusal.
fn expect<'a>(
    reader: &mut Reader<'a>,
    identifier: u8,
    member: &'static str,
) -> Result<Value<'a>, EnvelopeError> {
    reader
        .next_value()?
        .filter(|value| value.identifier == identifier)
        .ok_or(EnvelopeError::Malformed(member))
}

/// An implicitly tagged `OCTET STRING`'s octets: its contents where it is primitive, and its
/// `OCTET STRING` segments joined where X.690 clause 8.7.3's constructed form was written.
fn octets(value: Value<'_>) -> Result<Vec<u8>, EnvelopeError> {
    if !value.is_constructed() {
        return Ok(value.contents.to_vec());
    }
    let mut joined = Vec::new();
    let mut segments = value.children()?;
    while let Some(segment) = segments.next_value()? {
        if segment.identifier != OCTET_STRING {
            return Err(EnvelopeError::Malformed("encryptedContent segment"));
        }
        joined.extend_from_slice(segment.contents);
    }
    Ok(joined)
}

/// An object identifier as the dotted decimal a person reads, or its bytes in hexadecimal where
/// it is not a well-formed one.
fn dotted(oid: &[u8]) -> String {
    crate::x509::dotted(oid).unwrap_or_else(|| {
        oid.iter().fold(String::from("0x"), |mut out, byte| {
            use std::fmt::Write as _;
            // Writing to a `String` cannot fail.
            let _ = write!(out, "{byte:02X}");
            out
        })
    })
}

#[cfg(test)]
pub(crate) mod tests;

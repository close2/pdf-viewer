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
//! RFC 5652 section 6.2 defines five recipient kinds, and two of them name a certificate: key
//! transport (`KeyTransRecipientInfo`, section 6.2.1), which an RSA private key opens and which
//! every public-key document this tree has seen uses, and key agreement
//! (`KeyAgreeRecipientInfo`, section 6.2.2), which an elliptic-curve private key opens by RFC
//! 5753 section 3.1's ephemeral-static Diffie–Hellman ([`agreement`]). The other three are read as
//! what they are and are never a match, because none of them names a certificate (6.2.3 a
//! previously distributed symmetric key, 6.2.4 a password, 6.2.5 another scheme). Of key
//! transport's algorithms, RFC 8017's `rsaEncryption` and RFC 3560's `id-RSAES-OAEP` are opened
//! ([`crate::rsa`], [`crate::rsa::oaep`]); of key
//! agreement's, RFC 5753's `dhSinglePass` schemes under its AES key wraps; anything else is
//! refused with its object identifier.
//!
//! §7.6.5.3 lists the ciphers that "shall be used to encrypt the enveloped data": RC4, DES, Triple
//! DES and RC2, all four "deprecated in PDF 2.0", and AES in CBC mode at 128, 192 and 256 bits.
//! All seven are decrypted here, each named by the identifier and read with the parameters its own
//! specification gives it: AES with RFC 3565 section 2.3's sixteen-octet initialisation vector,
//! DES and Triple DES with the eight-octet one of RFC 8018 appendix B.2.1 and RFC 3370 section
//! 5.1, RC2 with RFC 2268 section 6's version-and-vector parameter, which carries the effective
//! key bits, and RC4 with none. "Deprecated" is a word about writing: a document a producer
//! enveloped under one of the four is still a document this reader is asked to open (ADR 2809).

use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockCipherDecrypt, BlockModeDecrypt, InnerIvInit, KeyInit, StreamCipher};
use aes::{Aes128, Aes192, Aes256};
use des::{Des, TdesEde2, TdesEde3};
use rc2::Rc2;
use rc4::Rc4;

use crate::der::{DerError, OCTET_STRING, Reader, SEQUENCE, SET, Value};
use crate::key::PrivateKey;
use crate::rsa::{ID_RSAES_OAEP, OaepParameterError, OaepParameters};
use crate::x509::{Certificate, RSA_ENCRYPTION};

/// RFC 5652 section 6.1's `id-envelopedData`, 1.2.840.113549.1.7.3.
pub const ID_ENVELOPED_DATA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x03];

/// RFC 3565 section 4's `id-aes128-CBC`, 2.16.840.1.101.3.4.1.2.
pub const ID_AES128_CBC: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01, 0x02];
/// RFC 3565 section 4's `id-aes192-CBC`, 2.16.840.1.101.3.4.1.22.
pub const ID_AES192_CBC: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01, 0x16];
/// RFC 3565 section 4's `id-aes256-CBC`, 2.16.840.1.101.3.4.1.42.
pub const ID_AES256_CBC: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01, 0x2A];

/// RFC 8018 appendix B.2.1's `desCBC`, 1.3.14.3.2.7: DES (FIPS 46-3) in CBC mode.
pub const ID_DES_CBC: &[u8] = &[0x2B, 0x0E, 0x03, 0x02, 0x07];
/// RFC 3370 section 5.1's `des-ede3-cbc`, 1.2.840.113549.3.7: Triple DES in CBC mode.
pub const ID_DES_EDE3_CBC: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x03, 0x07];
/// RFC 3370 section 5.2's `rc2-cbc`, 1.2.840.113549.3.2: RC2 (RFC 2268) in CBC mode.
pub const ID_RC2_CBC: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x03, 0x02];
/// RSA Security's `rc4`, 1.2.840.113549.3.4, under `encryptionAlgorithm` beside `rc2-cbc` and
/// `des-ede3-cbc`. No RFC assigns RC4 to CMS content encryption, and RFC 5652 section 6.1 needs no
/// such assignment: `contentEncryptionAlgorithm` is any `AlgorithmIdentifier`.
pub const ID_RC4: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x03, 0x04];

/// RFC 2268 section 6's `Table[]`, which maps a number of effective key bits from 1 to 255 to the
/// `RC2Version` an `RC2-CBCParameter` carries. It "specifies a permutation on the numbers 0-255",
/// so a version below 256 is read back through its inverse ([`rc2_effective_bits`]).
const RC2_VERSION: [u8; 256] = [
    0xbd, 0x56, 0xea, 0xf2, 0xa2, 0xf1, 0xac, 0x2a, 0xb0, 0x93, 0xd1, 0x9c, 0x1b, 0x33, 0xfd, 0xd0,
    0x30, 0x04, 0xb6, 0xdc, 0x7d, 0xdf, 0x32, 0x4b, 0xf7, 0xcb, 0x45, 0x9b, 0x31, 0xbb, 0x21, 0x5a,
    0x41, 0x9f, 0xe1, 0xd9, 0x4a, 0x4d, 0x9e, 0xda, 0xa0, 0x68, 0x2c, 0xc3, 0x27, 0x5f, 0x80, 0x36,
    0x3e, 0xee, 0xfb, 0x95, 0x1a, 0xfe, 0xce, 0xa8, 0x34, 0xa9, 0x13, 0xf0, 0xa6, 0x3f, 0xd8, 0x0c,
    0x78, 0x24, 0xaf, 0x23, 0x52, 0xc1, 0x67, 0x17, 0xf5, 0x66, 0x90, 0xe7, 0xe8, 0x07, 0xb8, 0x60,
    0x48, 0xe6, 0x1e, 0x53, 0xf3, 0x92, 0xa4, 0x72, 0x8c, 0x08, 0x15, 0x6e, 0x86, 0x00, 0x84, 0xfa,
    0xf4, 0x7f, 0x8a, 0x42, 0x19, 0xf6, 0xdb, 0xcd, 0x14, 0x8d, 0x50, 0x12, 0xba, 0x3c, 0x06, 0x4e,
    0xec, 0xb3, 0x35, 0x11, 0xa1, 0x88, 0x8e, 0x2b, 0x94, 0x99, 0xb7, 0x71, 0x74, 0xd3, 0xe4, 0xbf,
    0x3a, 0xde, 0x96, 0x0e, 0xbc, 0x0a, 0xed, 0x77, 0xfc, 0x37, 0x6b, 0x03, 0x79, 0x89, 0x62, 0xc6,
    0xd7, 0xc0, 0xd2, 0x7c, 0x6a, 0x8b, 0x22, 0xa3, 0x5b, 0x05, 0x5d, 0x02, 0x75, 0xd5, 0x61, 0xe3,
    0x18, 0x8f, 0x55, 0x51, 0xad, 0x1f, 0x0b, 0x5e, 0x85, 0xe5, 0xc2, 0x57, 0x63, 0xca, 0x3d, 0x6c,
    0xb4, 0xc5, 0xcc, 0x70, 0xb2, 0x91, 0x59, 0x0d, 0x47, 0x20, 0xc8, 0x4f, 0x58, 0xe0, 0x01, 0xe2,
    0x16, 0x38, 0xc4, 0x6f, 0x3b, 0x0f, 0x65, 0x46, 0xbe, 0x7e, 0x2d, 0x7b, 0x82, 0xf9, 0x40, 0xb5,
    0x1d, 0x73, 0xf8, 0xeb, 0x26, 0xc7, 0x87, 0x97, 0x25, 0x54, 0xb1, 0x28, 0xaa, 0x98, 0x9d, 0xa5,
    0x64, 0x6d, 0x7a, 0xd4, 0x10, 0x81, 0x44, 0xef, 0x49, 0xd6, 0xae, 0x2e, 0xdd, 0x76, 0x5c, 0x2f,
    0xa7, 0x1c, 0xc9, 0x09, 0x69, 0x9a, 0x83, 0xcf, 0x29, 0x39, 0xb9, 0xe9, 0x4c, 0xff, 0x43, 0xab,
];

/// The DES and RC2 block, and the eight-octet initialisation vector RFC 8018 appendix B.2 and
/// RFC 3370 section 5 give all three of their CBC modes.
const SMALL_BLOCK: usize = 8;

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
    /// No key-transport or key-agreement recipient names this certificate, by issuer and serial
    /// number or by subject key identifier (RFC 5652 sections 6.2.1 and 6.2.2's `rid`).
    #[error("no recipient of the CMS object is this certificate")]
    NotARecipient,
    /// The recipient's key is transported under an algorithm other than `rsaEncryption` and
    /// `id-RSAES-OAEP`, or under `id-RSAES-OAEP` with a hash, mask generation function or label
    /// source this reader does not run; the string names it, by its object identifier in dotted
    /// form.
    #[error("the recipient's key is encrypted under {0}, which this reader does not decrypt")]
    KeyTransport(String),
    /// The recipient is a key-agreement one under a scheme this reader does not run — 1-Pass
    /// ECMQV, or an originator key that is not `id-ecPublicKey` — the string is the object
    /// identifier in dotted form.
    #[error("the recipient's key is agreed under {0}, which this reader does not run")]
    KeyAgreement(String),
    /// The content-encryption key is wrapped under an algorithm this reader does not unwrap; the
    /// string is the object identifier in dotted form.
    #[error("the recipient's key is wrapped under {0}, which this reader does not unwrap")]
    KeyWrap(String),
    /// The certificate names a recipient of the other kind than the key handed over can open: an
    /// RSA key for a key-agreement recipient, or an elliptic-curve key for a key-transport one.
    /// The string names the recipient kind.
    #[error(
        "the recipient that names this certificate is a {0} recipient, which this private key cannot open"
    )]
    KeyKind(&'static str),
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecipientInfo<'a> {
    /// `ktri`, section 6.2.1.
    KeyTransport {
        /// `rid`, the certificate this entry is for.
        recipient: RecipientIdentifier<'a>,
        /// `keyEncryptionAlgorithm`'s object identifier.
        algorithm: &'a [u8],
        /// The same algorithm identifier's parameters, where it states any: `NULL` for
        /// `rsaEncryption`, RFC 3560 section 3's `RSAES-OAEP-params` for `id-RSAES-OAEP`.
        parameters: Option<Value<'a>>,
        /// `encryptedKey`: the content-encryption key under the recipient's public key.
        encrypted_key: &'a [u8],
    },
    /// `kari`, section 6.2.2.
    KeyAgreement(KeyAgreement<'a>),
    /// `kekri`, `pwri` or `ori` — sections 6.2.3 to 6.2.5 — by the context tag number that chose
    /// it. None of them names a certificate.
    Other(u8),
}

/// The recipient a certificate matched: the entry, and the encrypted key that is its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recipient<'e, 'a> {
    /// The `RecipientInfo` that names the certificate.
    pub info: &'e RecipientInfo<'a>,
    /// Its `encryptedKey` — for a key agreement, the one `RecipientEncryptedKey` whose `rid`
    /// names the certificate.
    pub encrypted_key: &'a [u8],
}

/// RFC 5652 section 6.1's `EnvelopedData`, as far as opening it for a recipient needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopedData<'a> {
    /// `originatorInfo`'s certificates, each a whole `Certificate` encoding: where a key
    /// agreement's sender names its key by certificate, this is where that certificate is.
    pub originator_certificates: Vec<Value<'a>>,
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
    // "originatorInfo [0] IMPLICIT OriginatorInfo OPTIONAL", "OriginatorInfo ::= SEQUENCE {
    // certs [0] IMPLICIT CertificateSet OPTIONAL, crls [1] IMPLICIT RevocationInfoChoices
    // OPTIONAL }": the certificates a key agreement's originator may name (section 6.2.2). Of
    // section 10.2.2's `CertificateChoices` only the plain `Certificate`, a `SEQUENCE`, can carry
    // a key to agree with; the tagged alternatives are skipped.
    let mut originator_certificates = Vec::new();
    if next.is_context(0) {
        let mut info = next.children()?;
        while let Some(member) = info.next_value()? {
            if member.is_context(0) && member.is_constructed() {
                let mut certificates = member.children()?;
                while let Some(certificate) = certificates.next_value()? {
                    if certificate.identifier == SEQUENCE {
                        originator_certificates.push(certificate);
                    }
                }
            }
        }
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
        originator_certificates,
        recipients,
        content_algorithm,
        content_parameters,
        encrypted_content: octets(content)?,
    })
}

impl<'a> EnvelopedData<'a> {
    /// The first recipient whose `rid` names `certificate`, as §7.6.5.1's reader "shall attempt
    /// to find a match with a certificate that belongs to the user": a key-transport entry, or
    /// the one `RecipientEncryptedKey` of a key-agreement entry.
    #[must_use]
    pub fn recipient(&self, certificate: &Certificate<'_>) -> Option<Recipient<'_, 'a>> {
        self.recipients.iter().find_map(|info| match info {
            RecipientInfo::KeyTransport {
                recipient,
                encrypted_key,
                ..
            } => recipient.names(certificate).then_some(Recipient {
                info,
                encrypted_key,
            }),
            RecipientInfo::KeyAgreement(agreement) => agreement
                .recipients
                .iter()
                .find(|entry| entry.recipient.names(certificate))
                .map(|entry| Recipient {
                    info,
                    encrypted_key: entry.encrypted_key,
                }),
            RecipientInfo::Other(_) => None,
        })
    }

    /// Opens the envelope for `certificate`'s holder, whose private key is `key`: RFC 5652
    /// section 6.2.1's key transport undone with RFC 8017's `RSAES-PKCS1-v1_5` or, under RFC
    /// 3560's `id-RSAES-OAEP`, its `RSAES-OAEP` with the parameters the recipient states, or section
    /// 6.2.2's key agreement undone as RFC 5753 section 3.1.3's receiving agent, then section
    /// 6.3's content decryption.
    ///
    /// # Errors
    ///
    /// [`EnvelopeError::NotARecipient`] where no recipient names the certificate,
    /// [`EnvelopeError::KeyKind`] where the one that does is of the kind the key cannot open,
    /// [`EnvelopeError::KeyTransport`], [`EnvelopeError::KeyAgreement`],
    /// [`EnvelopeError::KeyWrap`] and [`EnvelopeError::Cipher`] naming an algorithm this reader
    /// does not run, and [`EnvelopeError::Decryption`] where the key does not open it.
    pub fn open(
        &self,
        certificate: &Certificate<'_>,
        key: &PrivateKey,
    ) -> Result<Vec<u8>, EnvelopeError> {
        let Some(Recipient {
            info,
            encrypted_key,
        }) = self.recipient(certificate)
        else {
            return Err(EnvelopeError::NotARecipient);
        };
        let content_key = match (info, key) {
            (
                RecipientInfo::KeyTransport {
                    algorithm,
                    parameters,
                    ..
                },
                PrivateKey::Rsa(key),
            ) => match *algorithm {
                RSA_ENCRYPTION => key.decrypt(encrypted_key),
                // RFC 3560 section 3: "The AlgorithmIdentifier parameters field MUST be present,
                // and the parameters field MUST contain RSAES-OAEP-params" (ADR 2826).
                ID_RSAES_OAEP => {
                    let parameters =
                        OaepParameters::read(*parameters).map_err(|error| match error {
                            OaepParameterError::Unsupported { member, algorithm } => {
                                EnvelopeError::KeyTransport(format!(
                                    "id-RSAES-OAEP with the {member} {algorithm}"
                                ))
                            }
                            OaepParameterError::Malformed(what) => EnvelopeError::Malformed(what),
                        })?;
                    key.decrypt_oaep(encrypted_key, &parameters)
                }
                other => return Err(EnvelopeError::KeyTransport(dotted(other))),
            }
            .map_err(|_| EnvelopeError::Decryption)?,
            (RecipientInfo::KeyAgreement(agreement), PrivateKey::Ec(key)) => {
                agreement.content_key(encrypted_key, key, &self.originator_certificates)?
            }
            (RecipientInfo::KeyTransport { .. }, PrivateKey::Ec(_)) => {
                return Err(EnvelopeError::KeyKind("key-transport"));
            }
            (RecipientInfo::KeyAgreement(_), PrivateKey::Rsa(_)) => {
                return Err(EnvelopeError::KeyKind("key-agreement"));
            }
            (RecipientInfo::Other(_), _) => return Err(EnvelopeError::NotARecipient),
        };
        self.decrypt_content(&content_key)
    }

    /// RFC 5652 section 6.3 run backwards under `content_key`, with section 6.3's padding — "k-(lth
    /// mod k) octets all having value k-(lth mod k)" — removed from a block cipher's output.
    ///
    /// Each of §7.6.5.3's seven ciphers is named by the identifier its own specification gives it
    /// ([`ContentCipher::of`]); a key of a length that cipher does not take is the one
    /// [`EnvelopeError::Decryption`], like any other key that does not open the envelope.
    fn decrypt_content(&self, content_key: &[u8]) -> Result<Vec<u8>, EnvelopeError> {
        let body = self.encrypted_content.as_slice();
        match ContentCipher::of(self.content_algorithm, self.content_parameters)? {
            ContentCipher::Aes { width, iv } => {
                if content_key.len() != width {
                    return Err(EnvelopeError::Decryption);
                }
                match width {
                    16 => cbc_decrypt(
                        Aes128::new_from_slice(content_key)
                            .map_err(|_| EnvelopeError::Decryption)?,
                        iv,
                        body,
                    ),
                    24 => cbc_decrypt(
                        Aes192::new_from_slice(content_key)
                            .map_err(|_| EnvelopeError::Decryption)?,
                        iv,
                        body,
                    ),
                    _ => cbc_decrypt(
                        Aes256::new_from_slice(content_key)
                            .map_err(|_| EnvelopeError::Decryption)?,
                        iv,
                        body,
                    ),
                }
            }
            // FIPS 46-3's DES takes "a 64-bit key", eight octets with their parity bits ignored.
            ContentCipher::Des { iv } => cbc_decrypt(
                Des::new_from_slice(content_key).map_err(|_| EnvelopeError::Decryption)?,
                iv,
                body,
            ),
            // RFC 3370 section 5.1: three-key Triple DES "uses a different key for each DES
            // operation", two-key "uses one key for the two encrypt operations and a different
            // key for the decrypt operation", and "[t]he same algorithm identifiers are used for"
            // both — so the key's length, 24 octets or 16, is what tells them apart.
            ContentCipher::TripleDes { iv } => match content_key.len() {
                24 => cbc_decrypt(
                    TdesEde3::new_from_slice(content_key).map_err(|_| EnvelopeError::Decryption)?,
                    iv,
                    body,
                ),
                16 => cbc_decrypt(
                    TdesEde2::new_from_slice(content_key).map_err(|_| EnvelopeError::Decryption)?,
                    iv,
                    body,
                ),
                _ => Err(EnvelopeError::Decryption),
            },
            // RFC 8018 appendix B.2.3: RC2 "has a variable key length, from one to 128 octets, a
            // separate "effective key bits" parameter from one to 1024 bits"; §7.6.5.3 reads RC2
            // "with key lengths up to 128 bits", which is the narrower bound and the one a PDF
            // processor is held to.
            ContentCipher::Rc2 { effective_bits, iv } => {
                if content_key.is_empty() || content_key.len() > 16 {
                    return Err(EnvelopeError::Decryption);
                }
                cbc_decrypt(
                    Rc2::new_with_eff_key_len(content_key, effective_bits),
                    iv,
                    body,
                )
            }
            // §7.6.5.3: "RC4 with key lengths up to 256-bits". A stream cipher, so the content is
            // its own length and section 6.3's padding — which is for block ciphers — is absent.
            ContentCipher::Rc4 => {
                if content_key.is_empty() || content_key.len() > 32 {
                    return Err(EnvelopeError::Decryption);
                }
                let mut out = body.to_vec();
                Rc4::new_from_slice(content_key)
                    .map_err(|_| EnvelopeError::Decryption)?
                    .apply_keystream(&mut out);
                Ok(out)
            }
        }
    }
}

/// §7.6.5.3's ciphers "that shall be used to encrypt the enveloped data", each with the
/// parameters its own specification puts in the `AlgorithmIdentifier`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContentCipher<'a> {
    /// RFC 3565's three `id-aes*-CBC`, whose width the identifier states.
    Aes {
        /// The key's length in octets: 16, 24 or 32.
        width: usize,
        /// RFC 3565 section 2.3's sixteen-octet initialisation vector.
        iv: &'a [u8],
    },
    /// RFC 8018 appendix B.2.1's `desCBC`.
    Des {
        /// The eight-octet initialisation vector.
        iv: &'a [u8],
    },
    /// RFC 3370 section 5.1's `des-ede3-cbc`.
    TripleDes {
        /// The eight-octet initialisation vector.
        iv: &'a [u8],
    },
    /// RFC 3370 section 5.2's `rc2-cbc`.
    Rc2 {
        /// RFC 2268's effective key bits, 1 to 1024.
        effective_bits: usize,
        /// The eight-octet initialisation vector.
        iv: &'a [u8],
    },
    /// `rc4`, which takes no parameters.
    Rc4,
}

impl<'a> ContentCipher<'a> {
    /// The cipher `algorithm` names, with its parameters read as its specification states them.
    ///
    /// # Errors
    ///
    /// [`EnvelopeError::Malformed`] naming the parameter that is not what the cipher's
    /// specification requires, and [`EnvelopeError::Cipher`] naming an algorithm that is none of
    /// §7.6.5.3's.
    fn of(algorithm: &[u8], parameters: Option<Value<'a>>) -> Result<Self, EnvelopeError> {
        // RFC 3565 section 2.3 ("AES-IV ::= OCTET STRING (SIZE(16))"), RFC 8018 appendix B.2.1
        // and RFC 3370 section 5.1 ("IV ::= OCTET STRING -- exactly 8 octets").
        let iv = |length: usize, member: &'static str| {
            parameters
                .filter(|value| value.identifier == OCTET_STRING)
                .map(|value| value.contents)
                .filter(|iv| iv.len() == length)
                .ok_or(EnvelopeError::Malformed(member))
        };
        Ok(match algorithm {
            id if id == ID_AES128_CBC => Self::Aes {
                width: 16,
                iv: iv(AES_BLOCK, "AES-IV")?,
            },
            id if id == ID_AES192_CBC => Self::Aes {
                width: 24,
                iv: iv(AES_BLOCK, "AES-IV")?,
            },
            id if id == ID_AES256_CBC => Self::Aes {
                width: 32,
                iv: iv(AES_BLOCK, "AES-IV")?,
            },
            id if id == ID_DES_CBC => Self::Des {
                iv: iv(SMALL_BLOCK, "desCBC IV")?,
            },
            id if id == ID_DES_EDE3_CBC => Self::TripleDes {
                iv: iv(SMALL_BLOCK, "des-ede3-cbc IV")?,
            },
            id if id == ID_RC2_CBC => {
                let (effective_bits, iv) = rc2_parameters(parameters)?;
                Self::Rc2 { effective_bits, iv }
            }
            // RC4 has no initialisation vector and its identifier no parameters to read; what an
            // encoder put there, `NULL` or an empty `OCTET STRING`, keys nothing.
            id if id == ID_RC4 => Self::Rc4,
            other => return Err(EnvelopeError::Cipher(dotted(other))),
        })
    }
}

/// RC2's parameters, read as RFC 2268 section 6 defines them:
///
/// ```text
/// RC2-CBCParameter ::= CHOICE {
///   iv IV,
///   params SEQUENCE {
///     version RC2Version,
///     iv IV
///   }
/// }
/// ```
///
/// "If the number EKB of effective key bits is in the range 1-255, then the version number is
/// given by Table\[EKB]", and "\[i]f the number EKB of effective key bits is in the range
/// 256-1024, then the version number is simply EKB"; "\[t]he default number of effective key bits
/// for RC2 is 32", and it is the one "supplied as a simple IV". RFC 8018 appendix B.2.3 writes the
/// same `SEQUENCE` with the version `OPTIONAL` and the same default, which is read too.
fn rc2_parameters(parameters: Option<Value<'_>>) -> Result<(usize, &[u8]), EnvelopeError> {
    const DEFAULT_EFFECTIVE_BITS: usize = 32;
    let malformed = EnvelopeError::Malformed("RC2-CBCParameter");
    let parameters = parameters.ok_or(EnvelopeError::Malformed("RC2-CBCParameter"))?;
    let (effective_bits, iv) = match parameters.identifier {
        OCTET_STRING => (DEFAULT_EFFECTIVE_BITS, parameters.contents),
        SEQUENCE => {
            let mut members = parameters.children()?;
            let first = members.next_value()?.ok_or(malformed.clone())?;
            if first.identifier == crate::der::INTEGER {
                let iv = expect(&mut members, OCTET_STRING, "RC2-CBCParameter iv")?;
                let version = small_unsigned(first.contents).ok_or(malformed.clone())?;
                (
                    rc2_effective_bits(version).ok_or(malformed.clone())?,
                    iv.contents,
                )
            } else if first.identifier == OCTET_STRING {
                (DEFAULT_EFFECTIVE_BITS, first.contents)
            } else {
                return Err(malformed);
            }
        }
        _ => return Err(malformed),
    };
    if iv.len() != SMALL_BLOCK {
        return Err(EnvelopeError::Malformed("RC2-CBCParameter iv"));
    }
    Ok((effective_bits, iv))
}

/// The effective key bits an `RC2Version` stands for, or `None` for a version RFC 2268 section 6
/// assigns to none: `Table[]` is a permutation, so a version below 256 names exactly one EKB, and
/// the EKB it names must be at least 1.
fn rc2_effective_bits(version: usize) -> Option<usize> {
    match version {
        0..=255 => RC2_VERSION
            .iter()
            .position(|&entry| usize::from(entry) == version)
            .filter(|&bits| bits >= 1),
        256..=1024 => Some(version),
        _ => None,
    }
}

/// A non-negative DER `INTEGER` of at most two significant octets; RFC 3370 section 5.2's "the
/// value 160 must be encoded as two octets (00 A0), since the one octet (A0) encoding represents
/// a negative number" is why a leading zero is accepted and a set top bit is not.
fn small_unsigned(contents: &[u8]) -> Option<usize> {
    let (&first, _) = contents.split_first()?;
    if first & 0x80 != 0 || contents.len() > 3 {
        return None;
    }
    Some(
        contents
            .iter()
            .fold(0usize, |value, &octet| (value << 8) | usize::from(octet)),
    )
}

/// CBC decryption under `cipher` with `iv`, and RFC 5652 section 6.3's padding removed.
fn cbc_decrypt<C: BlockCipherDecrypt>(
    cipher: C,
    iv: &[u8],
    body: &[u8],
) -> Result<Vec<u8>, EnvelopeError> {
    let block = C::block_size();
    if body.is_empty() || !body.len().is_multiple_of(block) {
        return Err(EnvelopeError::Decryption);
    }
    let mut out = vec![0u8; body.len()];
    let length = cbc::Decryptor::<C>::inner_iv_slice_init(cipher, iv)
        .map_err(|_| EnvelopeError::Decryption)?
        .decrypt_padded_b2b::<Pkcs7>(body, &mut out)
        .map(<[u8]>::len)
        .map_err(|_| EnvelopeError::Decryption)?;
    out.truncate(length);
    Ok(out)
}
/// One `RecipientInfo` CHOICE (section 6.2): `ktri` is the untagged `SEQUENCE`, `kari` is
/// `[1]` around an implicit `SEQUENCE`, and the other three are context-tagged too.
fn recipient_info(info: Value<'_>) -> Result<RecipientInfo<'_>, EnvelopeError> {
    if info.is_context(1) && info.is_constructed() {
        return Ok(RecipientInfo::KeyAgreement(agreement::read(info)?));
    }
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
    let recipient = recipient_identifier(rid, "KeyTransRecipientInfo.rid")?;
    let mut identifier = expect(&mut members, SEQUENCE, "keyEncryptionAlgorithm")?.children()?;
    let algorithm = identifier
        .next_value()?
        .and_then(|value| value.object_identifier())
        .ok_or(EnvelopeError::Malformed("keyEncryptionAlgorithm"))?;
    let parameters = identifier.next_value()?;
    let encrypted_key = expect(&mut members, OCTET_STRING, "encryptedKey")?;
    Ok(RecipientInfo::KeyTransport {
        recipient,
        algorithm,
        parameters,
        encrypted_key: encrypted_key.contents,
    })
}

/// A certificate named by `IssuerAndSerialNumber` or by an implicitly tagged
/// `subjectKeyIdentifier [0]` — `ktri`'s `rid`, and two of `kari`'s `originator` choices and
/// one of its `rid` ones; `member` names the field in the refusal.
fn recipient_identifier<'a>(
    value: Value<'a>,
    member: &'static str,
) -> Result<RecipientIdentifier<'a>, EnvelopeError> {
    if value.identifier == SEQUENCE {
        // "IssuerAndSerialNumber ::= SEQUENCE { issuer Name, serialNumber CertificateSerialNumber }"
        // (section 10.2.4).
        let mut pair = value.children()?;
        let issuer = expect(&mut pair, SEQUENCE, "IssuerAndSerialNumber.issuer")?;
        let serial = expect(
            &mut pair,
            crate::der::INTEGER,
            "IssuerAndSerialNumber.serialNumber",
        )?;
        Ok(RecipientIdentifier::IssuerAndSerialNumber {
            issuer: issuer.contents,
            serial: serial.contents,
        })
    } else if value.is_context(0) && !value.is_constructed() {
        // "subjectKeyIdentifier [0] SubjectKeyIdentifier", an implicitly tagged OCTET STRING.
        Ok(RecipientIdentifier::SubjectKeyIdentifier(value.contents))
    } else {
        Err(EnvelopeError::Malformed(member))
    }
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
pub(crate) fn dotted(oid: &[u8]) -> String {
    crate::x509::dotted(oid).unwrap_or_else(|| {
        oid.iter().fold(String::from("0x"), |mut out, byte| {
            use std::fmt::Write as _;
            // Writing to a `String` cannot fail.
            let _ = write!(out, "{byte:02X}");
            out
        })
    })
}

pub mod agreement;

pub use agreement::{KeyAgreement, Originator, RecipientEncryptedKey};

#[cfg(test)]
pub(crate) mod tests;

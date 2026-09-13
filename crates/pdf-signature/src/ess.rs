//! RFC 5035's signing-certificate attributes: the hash a signer signed over its own certificate.
//!
//! §12.8.3.4.3 (f) requires one of these on a `PAdES` signature and names the document that
//! defines it — "[t]he details of the signing certificate attribute are defined in Internet RFC
//! 5035 ." §12.8.3.4.5 (a) is what a verifier does with it, and it is the first half of that
//! step's first sentence: "[a] signature handler shall compare the hash value of the signer's
//! certificate, with the hash value given in the signing-certificate attribute or the
//! signing-certificate-v2 attribute. If the hashes do not match, then the signature is considered
//! invalid."
//!
//! # What the attribute is for, and why refusing to read one is not neutral
//!
//! RFC 5035 section 5.4 states the threat in one sentence: "[t]he signing certificate attribute is
//! designed to prevent simple substitution and re-issue attacks, and to allow for a restricted set
//! of certificates to be used in verifying a signature." A `SignedData` carries its own
//! certificates, and a signature verifies under whichever of them the `SignerInfo` names — but the
//! `SignerInfo`'s `sid` is *not* covered by the signature, which RFC 5035 section 5.4.1.1 says in
//! as many words: "the sid field is not covered by the signature." The signing-certificate
//! attribute is, and it is the only thing in a CMS object that binds the verifying key to what the
//! signer meant to sign with.
//!
//! So a reader that parses the attribute and gives up on anything awkward has quietly re-opened
//! the attack. Every function here therefore returns an error naming what stopped it rather than
//! `None`, and [`crate::signature::Signature::authenticity`] turns any of them into a refusal
//! rather than into a verified signature.
//!
//! # The two spellings are one requirement
//!
//! RFC 5035 section 5.4: "[t]he only substantial difference between the two attributes is that
//! SigningCertificateV2 allows for hash algorithm agility, while SigningCertificate forces the use
//! of the SHA-1 hash algorithm", and "[i]f both attributes exist in a single message, they are
//! independently evaluated." Both are read here, each by its own function, and the caller evaluates
//! each that is present.
//!
//! # Only the first certificate identifier is read
//!
//! Both structures hold a `SEQUENCE OF` certificate identifiers, and RFC 5035 sections 5.4.1 and
//! 5.4.2 say what the first one is: "[t]he first certificate identified in the sequence of
//! certificate identifiers MUST be the certificate used to verify the signature." The ones after
//! it "limit the set of certificates that are used during validation" — a statement about a
//! certification path, which is §12.8.3.4.5 (b) and needs a trust store this program does not have.
//! Reading them would produce a constraint nothing here could apply.

use crate::cms::Digest;
use crate::der::{DerError, OCTET_STRING, SEQUENCE, Value};

/// What stopped an ESS certificate identifier from being read.
///
/// Each variant is a *refusal*, never a shrug: §12.8.3.4.5 (a) makes the comparison decisive, so a
/// caller that cannot make it has to say so rather than proceed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EssError {
    /// The attribute value is not the `SEQUENCE` RFC 5035 defines, or is empty where it may not be.
    #[error("the signing-certificate attribute is not the structure RFC 5035 defines")]
    Malformed,
    /// `certs` holds no identifier, so nothing names the certificate the signature was made with.
    ///
    /// RFC 5035 sections 5.4.1 and 5.4.2 both require a first one — "[t]he first certificate
    /// identified in the sequence of certificate identifiers MUST be the certificate used to
    /// verify the signature" — so an empty sequence has no answer in it rather than an empty one.
    #[error("the signing-certificate attribute names no certificate")]
    NoCertificate,
    /// `hashAlgorithm` names a digest this program does not compute, by its own identifier.
    ///
    /// Reported rather than guessed at, for [`Digest::from_oid`]'s reason: hashing with the wrong
    /// function would produce a mismatch, and a mismatch is what §12.8.3.4.5 (a) calls invalid.
    #[error("the signing-certificate attribute states digest algorithm {algorithm}")]
    UnknownDigest {
        /// The algorithm's object identifier as dotted decimal.
        algorithm: String,
    },
    /// The encoding itself would not read.
    #[error("the signing-certificate attribute would not parse: {0}")]
    Encoding(#[from] DerError),
}

/// Which of RFC 5035's two spellings of §12.8.3.4.3 (f)'s attribute a file stated.
///
/// Carried alongside every answer because the two are not interchangeable in what they prove: one
/// is SHA-1 by construction and the other states its own algorithm, and RFC 5035 section 5.4 says
/// why that matters — "[w]ith the recent advances in the ability to create hash collisions for
/// SHA-1, it is wise to move forward sooner rather than later."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// `signing-certificate`, RFC 5035 section 5.4.2's `SigningCertificate`.
    One,
    /// `signing-certificate-v2`, RFC 5035 section 5.4.1's `SigningCertificateV2`.
    Two,
}

impl Version {
    /// The attribute's name as §12.8.3.4.3 (f) spells it, for a sentence a person reads.
    #[must_use]
    pub fn attribute_name(self) -> &'static str {
        match self {
            Self::One => "signing-certificate",
            Self::Two => "signing-certificate-v2",
        }
    }
}

/// One `ESSCertID` or `ESSCertIDv2`, as far as §12.8.3.4.5 (a) needs it.
///
/// `issuerSerial` is deliberately absent. RFC 5035 section 5.4.1.1 makes it optional and explains
/// what it is for — identifying certificates that are *not* the signer's, and that this program
/// would need a path validator to use — while the hash is what the step compares: "[t]he use of the
/// hash is required by this structure since the detection of substituted certificates is based on
/// the fact they would map to different hash values."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertificateHash<'a> {
    /// The function `certHash` was computed with.
    pub digest: Digest,
    /// `certHash` itself — RFC 5035 section 5.4.1.1: "computed over the entire DER-encoded
    /// certificate (including the signature)".
    pub hash: &'a [u8],
}

/// RFC 5035 section 5.4.2's `SigningCertificate`, read down to its first `ESSCertID`.
///
/// ```text
/// SigningCertificate ::=  SEQUENCE {
///     certs        SEQUENCE OF ESSCertID,
///     policies     SEQUENCE OF PolicyInformation OPTIONAL
/// }
///
/// ESSCertID ::=  SEQUENCE {
///      certHash                 Hash,
///      issuerSerial             IssuerSerial OPTIONAL
/// }
/// ```
///
/// The digest is not read from the encoding because there is none to read: RFC 5035 section 5.4
/// says this attribute "forces the use of the SHA-1 hash algorithm", so the algorithm is the
/// structure's rather than the file's.
///
/// # Errors
///
/// [`EssError`], which is what a caller reports; see the module documentation for why none of these
/// may be treated as "no attribute".
pub fn signing_certificate(value: Value<'_>) -> Result<CertificateHash<'_>, EssError> {
    let identifier = first_certificate_identifier(value)?;
    let mut parts = identifier.children()?;
    let Some(hash) = parts.next_value()? else {
        return Err(EssError::Malformed);
    };
    if hash.identifier != OCTET_STRING {
        return Err(EssError::Malformed);
    }
    Ok(CertificateHash {
        digest: Digest::Sha1,
        hash: hash.contents,
    })
}

/// RFC 5035 section 5.4.1's `SigningCertificateV2`, read down to its first `ESSCertIDv2`.
///
/// ```text
/// SigningCertificateV2 ::=  SEQUENCE {
///     certs        SEQUENCE OF ESSCertIDv2,
///     policies     SEQUENCE OF PolicyInformation OPTIONAL
/// }
///
/// ESSCertIDv2 ::=  SEQUENCE {
///     hashAlgorithm           AlgorithmIdentifier
///            DEFAULT {algorithm id-sha256},
///     certHash                 Hash,
///     issuerSerial             IssuerSerial OPTIONAL
/// }
/// ```
///
/// **The `DEFAULT` is why the first member is distinguished by its tag rather than by counting.**
/// X.690 clause 11.5 requires DER to omit a member equal to its default, so a producer using
/// SHA-256 writes the `certHash` `OCTET STRING` first and a producer using anything else writes an
/// `AlgorithmIdentifier` `SEQUENCE` first. Counting members would read a SHA-256 identifier's hash
/// as its algorithm.
///
/// # Errors
///
/// [`EssError`]; a `hashAlgorithm` outside [`Digest`]'s ten is [`EssError::UnknownDigest`] and not
/// an approximation.
pub fn signing_certificate_v2(value: Value<'_>) -> Result<CertificateHash<'_>, EssError> {
    let identifier = first_certificate_identifier(value)?;
    let mut parts = identifier.children()?;
    let Some(first) = parts.next_value()? else {
        return Err(EssError::Malformed);
    };
    if first.identifier == OCTET_STRING {
        return Ok(CertificateHash {
            // The `DEFAULT {algorithm id-sha256}` RFC 5035 section 5.4.1.1 states, which a DER
            // producer writes by writing nothing.
            digest: Digest::Sha256,
            hash: first.contents,
        });
    }
    if first.identifier != SEQUENCE {
        return Err(EssError::Malformed);
    }
    let Some(oid) = first.children()?.next_value()? else {
        return Err(EssError::Malformed);
    };
    let Some(oid) = oid.object_identifier() else {
        return Err(EssError::Malformed);
    };
    let Some(digest) = Digest::from_oid(oid) else {
        return Err(EssError::UnknownDigest {
            algorithm: crate::signature::name(oid),
        });
    };
    let Some(hash) = parts.next_value()? else {
        return Err(EssError::Malformed);
    };
    if hash.identifier != OCTET_STRING {
        return Err(EssError::Malformed);
    }
    Ok(CertificateHash {
        digest,
        hash: hash.contents,
    })
}

/// The first member of `certs`, which both structures put first and both require.
fn first_certificate_identifier(value: Value<'_>) -> Result<Value<'_>, EssError> {
    if value.identifier != SEQUENCE {
        return Err(EssError::Malformed);
    }
    let Some(certs) = value.children()?.next_value()? else {
        return Err(EssError::Malformed);
    };
    if certs.identifier != SEQUENCE {
        return Err(EssError::Malformed);
    }
    let Some(first) = certs.children()?.next_value()? else {
        return Err(EssError::NoCertificate);
    };
    if first.identifier != SEQUENCE {
        return Err(EssError::Malformed);
    }
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::{CertificateHash, EssError, signing_certificate, signing_certificate_v2};
    use crate::cms::Digest;
    use crate::der::Reader;

    /// A DER header around already-encoded contents, for lengths below 128 — every fixture here.
    fn tagged(identifier: u8, contents: &[u8]) -> Vec<u8> {
        let mut out = vec![
            identifier,
            u8::try_from(contents.len()).expect("short fixture"),
        ];
        out.extend_from_slice(contents);
        out
    }

    fn sequence(children: &[Vec<u8>]) -> Vec<u8> {
        tagged(0x30, &children.concat())
    }

    fn octets(bytes: &[u8]) -> Vec<u8> {
        tagged(0x04, bytes)
    }

    fn oid(bytes: &[u8]) -> Vec<u8> {
        tagged(0x06, bytes)
    }

    fn read(bytes: &[u8]) -> crate::der::Value<'_> {
        Reader::new(bytes)
            .expect("fixture is under the reader's ceiling")
            .next_value()
            .expect("fixture parses")
            .expect("fixture holds a value")
    }

    /// RFC 5035 section 5.4.2's structure, with the SHA-1 hash that section 5.4 forces.
    #[test]
    fn a_version_one_attribute_states_a_sha1_hash() {
        let attribute = sequence(&[sequence(&[sequence(&[octets(&[0xAB; 20])])])]);
        assert_eq!(
            signing_certificate(read(&attribute)),
            Ok(CertificateHash {
                digest: Digest::Sha1,
                hash: &[0xAB; 20],
            })
        );
    }

    /// RFC 5035 section 5.4.1.1's `DEFAULT {algorithm id-sha256}`, which DER writes by omission.
    #[test]
    fn a_version_two_attribute_that_omits_its_algorithm_is_sha256() {
        let attribute = sequence(&[sequence(&[sequence(&[octets(&[0xCD; 32])])])]);
        assert_eq!(
            signing_certificate_v2(read(&attribute)),
            Ok(CertificateHash {
                digest: Digest::Sha256,
                hash: &[0xCD; 32],
            })
        );
    }

    /// And one that states an algorithm is read from the identifier rather than from the position.
    #[test]
    fn a_version_two_attribute_that_states_its_algorithm_is_read_from_it() {
        let algorithm = sequence(&[oid(Digest::Sha512.oid())]);
        let attribute = sequence(&[sequence(&[sequence(&[algorithm, octets(&[0xEF; 64])])])]);
        assert_eq!(
            signing_certificate_v2(read(&attribute)),
            Ok(CertificateHash {
                digest: Digest::Sha512,
                hash: &[0xEF; 64],
            })
        );
    }

    /// A digest this program does not compute is named, never guessed at (module documentation).
    #[test]
    fn a_digest_this_program_does_not_compute_is_refused_by_its_own_identifier() {
        // `id-sha224`, `2.16.840.1.101.3.4.2.4` — in the same NIST arc as the six that are
        // implemented, and in neither ISO 32000-2 Table 260 nor ISO/TS 32001 section 5.1.4.
        let unknown = [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x04];
        let algorithm = sequence(&[oid(&unknown)]);
        let attribute = sequence(&[sequence(&[sequence(&[algorithm, octets(&[0x11; 28])])])]);
        assert_eq!(
            signing_certificate_v2(read(&attribute)),
            Err(EssError::UnknownDigest {
                algorithm: "2.16.840.1.101.3.4.2.4".to_owned(),
            })
        );
    }

    /// An empty `certs` has no answer in it, and RFC 5035 requires a first entry.
    #[test]
    fn an_attribute_naming_no_certificate_is_refused_rather_than_read_as_absent() {
        let attribute = sequence(&[sequence(&[])]);
        assert_eq!(
            signing_certificate(read(&attribute)),
            Err(EssError::NoCertificate)
        );
        assert_eq!(
            signing_certificate_v2(read(&attribute)),
            Err(EssError::NoCertificate)
        );
    }

    /// Anything that is not the structure is refused, rather than read as far as it goes.
    #[test]
    fn a_value_that_is_not_the_structure_is_refused() {
        for bytes in [
            octets(&[0x00; 4]),
            sequence(&[octets(&[0x00; 4])]),
            sequence(&[sequence(&[octets(&[0x00; 4])])]),
            sequence(&[sequence(&[sequence(&[oid(Digest::Sha256.oid())])])]),
        ] {
            assert_eq!(
                signing_certificate(read(&bytes)),
                Err(EssError::Malformed),
                "v1 accepted {bytes:02X?}"
            );
        }
    }
}

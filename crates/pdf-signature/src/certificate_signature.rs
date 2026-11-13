//! The digest and the arithmetic a certificate, a CRL or an OCSP response is signed with.
//!
//! RFC 5280 section 4.1.1.3 shapes the triple — algorithm, signature, key — the same way for all
//! three, and `pdf_pki::x509` reads the key and the algorithm out of the certificate. Naming the
//! digest and running the verification need [`crate::cms::Digest`] and this crate's `pkcs1`,
//! `pss`, `dsa`, `ecdsa` and `eddsa`, so they are here.

use pdf_pki::der::Value;
use pdf_pki::x509::{PublicKey, dotted};

use crate::{dsa, ecdsa, eddsa, pkcs1, pss};

/// Which digest a combined signature algorithm identifier names.
///
/// A certificate, a CRL and an OCSP response all differ from a CMS `SignerInfo` here, and the
/// difference is the whole reason this exists: RFC 5652 puts the digest in its own
/// `digestAlgorithm` member, which [`crate::cms`] reads, while RFC 5280 section 4.1.1.2 carries
/// one identifier for the pair — so `sha256WithRSAEncryption` is the only place such a structure
/// says SHA-256.
///
/// The identifiers come from `const_oid`'s database, which is a second party's reading of the
/// registries that assign them rather than digits typed here; `id-RSASSA-PSS` is deliberately
/// absent, because its hash is in its parameters and [`crate::pss::parameters`] is what reads it.
pub(crate) fn digest(oid: &[u8]) -> Option<crate::cms::Digest> {
    use crate::cms::Digest;
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

/// One signature over one message under one key, as RFC 5280 section 4.1.1.3 shapes the triple.
///
/// Three structures in this crate are signed the same way — a `tbsCertificate` (section 4.1.1.3),
/// a `tbsCertList` (section 5.1.1.3) and an OCSP `ResponseData` (RFC 6960 section 4.2.1) — so the
/// arithmetic is written once here and reached from [`crate::trust`] and [`crate::revocation`].
///
/// The algorithm and the key are matched as a *pair* rather than either alone, for the reason
/// [`crate::signature::Authenticity`] states: an identifier naming one family over a key of
/// another is two contradictory claims by one producer, and choosing between them would be this
/// program inventing a fact.
///
/// # Errors
///
/// The algorithm identifier, as the dotted decimal a report can print, where this program does not
/// compute that pair. `Ok(false)` is the arithmetic saying no.
pub(crate) fn verify(
    message: &[u8],
    algorithm: &[u8],
    parameters: Option<Value<'_>>,
    signature: &[u8],
    key: PublicKey<'_>,
) -> Result<bool, String> {
    use crate::cms::SignatureAlgorithm;
    let named =
        || dotted(algorithm).unwrap_or_else(|| "an unreadable object identifier".to_owned());
    let key_parameters = key.pss_parameters();
    match (SignatureAlgorithm::from_oid(algorithm), key) {
        (SignatureAlgorithm::RsaPkcs1V15, PublicKey::Rsa(key)) => {
            let digest = digest(algorithm).ok_or_else(named)?;
            let computed = digest.compute(&[message]);
            pkcs1::verify(key, signature, digest, &computed).map_err(|_| named())
        }
        // RFC 4055 section 1.2: an `id-RSASSA-PSS` key verifies this scheme only, and under its
        // own parameters where it states any (section 3.3).
        (SignatureAlgorithm::RsaPss, PublicKey::Rsa(key) | PublicKey::RsaPss { key, .. }) => {
            let parameters = pss::parameters(parameters).map_err(|_| named())?;
            pss::admitted_by_key(parameters, key_parameters).map_err(|_| named())?;
            let computed = parameters.hash.compute(&[message]);
            pss::verify(key, signature, parameters, &computed).map_err(|_| named())
        }
        (SignatureAlgorithm::Dsa, PublicKey::Dsa(key)) => {
            let digest = digest(algorithm).ok_or_else(named)?;
            let computed = digest.compute(&[message]);
            dsa::verify(key, signature, &computed).map_err(|_| named())
        }
        (SignatureAlgorithm::Ecdsa, PublicKey::Ec(key)) => {
            let digest = digest(algorithm).ok_or_else(named)?;
            let computed = digest.compute(&[message]);
            ecdsa::verify(key, signature, &computed).map_err(|_| named())
        }
        (SignatureAlgorithm::EdDsa(stated), PublicKey::EdDsa(key)) if stated == key.curve => {
            // RFC 8032 signs the message rather than a digest of it, which is why this arm has no
            // `digest` call and why a structure signed with EdDSA states no hash.
            eddsa::verify(key, signature, &[message]).map_err(|_| named())
        }
        _ => Err(named()),
    }
}

#[cfg(test)]
mod tests {
    use super::super::x509_fixtures::{CERTIFICATE, PKCS1_SIGNATURE, hex};
    use crate::pkcs1;
    use pdf_pki::x509::{PublicKey, parse};

    /// The key this certificate carries is the key that verifies the signature `pkcs1` tests.
    ///
    /// The two modules are only joined here, and this is the join that matters: a certificate
    /// reader that produced a *plausible* modulus would pass every assertion above.
    #[test]
    fn the_key_read_out_of_a_certificate_verifies_a_signature_made_with_it() {
        let bytes = hex(CERTIFICATE);
        let certificate = parse(&bytes).expect("a certificate");
        let PublicKey::Rsa(key) = certificate.public_key else {
            panic!("rsaEncryption");
        };
        let signature = hex(PKCS1_SIGNATURE);
        let digest = crate::cms::Digest::Sha256.compute(&[b"the signed bytes"]);
        assert_eq!(
            pkcs1::verify(key, &signature, crate::cms::Digest::Sha256, &digest),
            Ok(true)
        );
    }
}

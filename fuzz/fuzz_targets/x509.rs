//! Fuzzes the signer's certificate and the RSA verification that runs on the key inside it.
//!
//! ISO 32000-2 §12.8.3.3.1 requires a signature to carry its signer's X.509 certificate — "[a]t
//! minimum the CMS object shall include the signer's X.509 signing certificate" — so from the
//! three-hundred-and-ninety-second session this program parses one out of a byte string a
//! stranger wrote, takes two integers out of it, and does arithmetic with them. ADR 0229
//! committed to this target with the code.
//!
//! **Two readers and two arithmetics.** `pdf_model::der` and `pdf_model::cms` have their own
//! target (`cms`); what is here is `pdf_model::x509`, which walks RFC 5280's structure, and the
//! modules that run a loop whose trip count comes out of a number in the file —
//! `pdf_model::pkcs1`, from the four-hundred-and-seventy-ninth session `pdf_model::dsa` (ADR
//! 0314), and from the four-hundred-and-eighty-seventh `pdf_model::pss`, whose salt length is a
//! number the file states (ADR 0322). Four properties:
//!
//! **Parsing and verifying terminate and never panic.** The fuzz profile keeps overflow checks
//! on, so the workspace's `arithmetic_side_effects` rule is checked here as well — and this is
//! the module with 128-limb carries in it, where a wrapping bug would otherwise be silent.
//!
//! **Every bound holds and is observable.** `pkcs1::MAX_MODULUS_BITS`, `pkcs1::MAX_EXPONENT_BITS`
//! and `dsa::MAX_SUBGROUP_BITS` are time and space budgets over numbers the file states, and each
//! is refused by name rather than by running out. A modulus or exponent past one of them must come
//! back as a `Pkcs1Error`; a `q` past its bound as a `DsaError`. The DSA one is the sharper of the
//! three: `q` is the exponent of two modular exponentiations, so an unbounded `q` in a hostile
//! certificate is unbounded work.
//!
//! **Nothing that comes back outlives or outgrows its input**, which is `x509`'s whole design:
//! every field is a sub-slice of the caller's buffer and nothing is decoded into an owned type.
//!
//! **Verification never says yes to a digest the caller did not sign.** The target verifies
//! against a digest of its own choosing, which no fuzzer-produced signature can be over, so
//! `Ok(true)` here would be a defect in the comparison rather than a lucky input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_model::cms::Digest;
use pdf_model::dsa::{self, MAX_SUBGROUP_BITS};
use pdf_model::pkcs1::{self, MAX_EXPONENT_BITS, MAX_MODULUS_BITS};
use pdf_model::pss;
use pdf_model::x509::{self, PublicKey};
use pdf_model::{ecdsa, eddsa};

fuzz_target!(|data: &[u8]| {
    let Ok(certificate) = x509::parse(data) else {
        // A refusal is a result: every one is a named `X509Error`, including the `DerError` it
        // carries, rather than a panic.
        return;
    };

    let inside = |slice: &[u8]| {
        assert!(
            slice.len() <= data.len(),
            "{} bytes came out of an input of {}",
            slice.len(),
            data.len()
        );
    };
    inside(certificate.serial_number);
    inside(certificate.issuer);
    inside(certificate.subject);
    if let Some(identifier) = certificate.key_identifier {
        inside(identifier);
    }
    // Both spellings of RFC 5652's signer identifier, over whatever the certificate states.
    let _ = certificate.is_named_by(certificate.issuer, certificate.serial_number);
    let _ = certificate.is_named_by(&[], &[]);

    // A digest this target chose. No signature the fuzzer produces is over it, so the only
    // acceptable answers below are `Ok(false)` and a named refusal.
    let digest = Digest::Sha256.compute(&[b"a digest nobody signed"]);

    let key = match certificate.public_key {
        PublicKey::Rsa(key) => {
            inside(key.modulus);
            inside(key.exponent);
            key
        }
        PublicKey::Dsa(key) => {
            inside(key.p);
            inside(key.q);
            inside(key.g);
            inside(key.y);
            for signature in [
                data,
                &[][..],
                &[0x30, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x01],
            ] {
                match dsa::verify(key, signature, &digest) {
                    Ok(false) => {}
                    Ok(true) => {
                        panic!("a DSA signature verified against a digest that was never signed")
                    }
                    Err(error) => {
                        if key.subgroup_bits() > MAX_SUBGROUP_BITS {
                            assert_eq!(error, dsa::DsaError::SubgroupTooLarge);
                        }
                    }
                }
            }
            return;
        }
        PublicKey::Ec(key) => {
            inside(key.point);
            // Every shape a signature value can arrive in, over a digest nobody signed. The
            // third is a well-formed `ECDSA-Sig-Value` with r = s = 1, so the range check is
            // reached rather than the encoding refusal, and the fourth is BSI TR-03111's plain
            // `r ‖ s`, which two corpus signatures use and which is not this encoding.
            for signature in [
                data,
                &[][..],
                &[0x30, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x01],
                &vec![0x11; 2 * key.curve.field_octets()],
            ] {
                match ecdsa::verify(key, signature, &digest) {
                    Ok(false) => {}
                    Ok(true) => {
                        panic!("an ECDSA signature verified against a digest that was never signed")
                    }
                    // Every refusal is a named `EcdsaError`, which is the property this arm is
                    // for: an encoding or a range this module will not act on, said out loud.
                    Err(_) => {}
                }
            }
            return;
        }
        PublicKey::Ed25519(key) => {
            inside(key.key);
            for signature in [data, &[][..], &[0u8; 64][..]] {
                match eddsa::verify(key, signature, &[b"a message nobody signed"]) {
                    Ok(false) => {}
                    Ok(true) => {
                        panic!("an Ed25519 signature verified over a message nobody signed")
                    }
                    Err(_) => {}
                }
            }
            return;
        }
        PublicKey::EcCurveNotVerifiable { curve } => {
            if let Some(curve) = curve {
                inside(curve);
                // A curve this program does not compute on is reported by *its* number, so
                // decoding one has to be total for the same reason the arm below needs it.
                let _ = x509::dotted(curve);
            }
            return;
        }
        PublicKey::Unverifiable { algorithm } => {
            inside(algorithm);
            // The one thing this program says about a key it cannot verify is the identifier's
            // number, so decoding one has to be total.
            let _ = x509::dotted(algorithm);
            return;
        }
    };

    for signature in [data, &[][..], &vec![0xFF; key.modulus.len()]] {
        match pkcs1::verify(key, signature, Digest::Sha256, &digest) {
            Ok(false) => {}
            Ok(true) => panic!("a signature verified against a digest that was never signed"),
            Err(error) => {
                // The two budgets are the reason this target exists; each must be *reported*
                // where it is exceeded, which is what makes them bounds rather than hopes.
                if key.bits() > MAX_MODULUS_BITS {
                    assert_eq!(error, pkcs1::Pkcs1Error::ModulusTooLarge);
                }
                let exponent_bits = pkcs1::PublicKey {
                    modulus: key.exponent,
                    exponent: key.exponent,
                }
                .bits();
                if exponent_bits > MAX_EXPONENT_BITS && key.bits() <= MAX_MODULUS_BITS {
                    assert_eq!(error, pkcs1::Pkcs1Error::ExponentTooLarge);
                }
            }
        }

        // The same key under the RSA family's other padding: RSASSA-PSS shares RSAVP1 and its
        // budgets with the arm above, and its own unmasking must never say "consistent" for a
        // digest nobody signed either. Two salt lengths, because the second is the shape a
        // hostile parameter block would state.
        for salt_length in [32usize, usize::MAX] {
            let parameters = pss::Parameters {
                hash: Digest::Sha256,
                mgf_hash: Digest::Sha256,
                salt_length,
            };
            match pss::verify(key, signature, parameters, &digest) {
                Ok(false) => {}
                Ok(true) => {
                    panic!("a PSS signature verified against a digest that was never signed")
                }
                Err(_) => {} // the same named refusals the arm above already checked
            }
        }
    }

    // A successful parse is idempotent: the reader carries a depth across nested calls, and state
    // leaking between them is what a single pass cannot see.
    let again = x509::parse(data).expect("the same bytes parsed once already");
    assert!(
        again == certificate,
        "reading the same certificate twice gave two different answers"
    );
});

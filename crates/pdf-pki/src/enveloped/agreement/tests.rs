//! Key-agreement recipients, opened.
//!
//! The envelopes under `tests/envelopes/kari/` were written by this tree's own generator,
//! `gen_kari.py` (kept with the round's record), which states each step it takes from its
//! specification: the recipient keys and their self-signed certificates are its own, the sender's
//! half of RFC 5753 section 3.1.2 is SEC 1 section 3.3.1's primitive, section 3.6.1's KDF over RFC
//! 5753 section 7.2's `ECC-CMS-SharedInfo`, and RFC 3394 section 2.2.1's wrap, each written out in
//! the script, and the content is RFC 3565's AES-256-CBC over the 24 octets of §7.6.5.3's byte
//! array. What each test expects back is that plaintext. Two more, `openssl-*.der`, were written
//! by `openssl cms -encrypt` to the same certificates; they are another program's reading of the
//! same RFCs, and opening them is evidence that the readings agree, not the definition of right.

use sha1::Sha1;
use sha2::{Digest, Sha256};

use super::{ID_AES256_WRAP, shared_info, x963_kdf};
use crate::enveloped::{EnvelopeError, Recipient, RecipientIdentifier, RecipientInfo, parse};
use crate::key::PrivateKey;
use crate::x509;

/// The 24 octets every fixture envelopes: a 20-octet seed and four octets of permissions.
const PLAIN: &[u8] = b"twenty octets of seed\xff\xff\xfc";

macro_rules! fixture {
    ($name:literal) => {
        &include_bytes!(concat!("../../../tests/envelopes/kari/", $name))[..]
    };
}

/// Each recipient's certificate and its key in both encodings.
fn recipient(curve: &str) -> (&'static [u8], &'static [u8], &'static [u8]) {
    match curve {
        "p256" => (
            fixture!("p256.cert.der"),
            fixture!("p256.ec.der"),
            fixture!("p256.p8.der"),
        ),
        "p384" => (
            fixture!("p384.cert.der"),
            fixture!("p384.ec.der"),
            fixture!("p384.p8.der"),
        ),
        "p521" => (
            fixture!("p521.cert.der"),
            fixture!("p521.ec.der"),
            fixture!("p521.p8.der"),
        ),
        "bp256" => (
            fixture!("bp256.cert.der"),
            fixture!("bp256.ec.der"),
            fixture!("bp256.p8.der"),
        ),
        "bp384" => (
            fixture!("bp384.cert.der"),
            fixture!("bp384.ec.der"),
            fixture!("bp384.p8.der"),
        ),
        _ => (
            fixture!("bp512.cert.der"),
            fixture!("bp512.ec.der"),
            fixture!("bp512.p8.der"),
        ),
    }
}

fn open(envelope: &[u8], curve: &str) -> Result<Vec<u8>, EnvelopeError> {
    let (certificate, _, key) = recipient(curve);
    let certificate = x509::parse(certificate).unwrap();
    parse(envelope)
        .unwrap()
        .open(&certificate, &PrivateKey::from_der(key).unwrap())
}

/// RFC 5753 section 3.1.3's receiving agent, over every variation the generator wrote:
/// `dhSinglePass-stdDH` under each of section 7.1.4's five hashes, the cofactor scheme (one
/// primitive on a curve of cofactor 1), each AES wrap of section 7.1.5, a `ukm` placed in
/// `entityUInfo`, `originatorKey` parameters absent, NULL and `namedCurve` (section 3.1.1 allows
/// all three), a compressed ephemeral point, `rid` as `issuerAndSerialNumber` and as `rKeyId`,
/// the matching `RecipientEncryptedKey` second in its list, five curves, and an originator named
/// by `issuerAndSerialNumber` whose certificate is in `originatorInfo` (section 6.2.2:
/// "Implementations MUST support all three alternatives for specifying the sender's public key"). Each opens to the
/// plaintext the generator put in, with the key in either encoding.
#[test]
fn each_generated_key_agreement_envelope_opens_to_its_plaintext() {
    let cases: [(&[u8], &str); 11] = [
        (fixture!("kari-p256-sha256-aes256.der"), "p256"),
        (fixture!("kari-p256-sha1-aes128-ukm.der"), "p256"),
        (fixture!("kari-p256-sha224-aes192-rkeyid.der"), "p256"),
        (fixture!("kari-p256-cofactor-sha256-null.der"), "p256"),
        (fixture!("kari-p256-compressed-named.der"), "p256"),
        (fixture!("kari-p256-second-rek.der"), "p256"),
        (fixture!("kari-p384-sha384-aes256.der"), "p384"),
        (fixture!("kari-p521-sha512-aes256.der"), "p521"),
        (fixture!("kari-bp256-sha256-aes128.der"), "bp256"),
        (fixture!("kari-bp384-sha384-aes256.der"), "bp384"),
        (fixture!("kari-p256-originator-certificate.der"), "p256"),
    ];
    for (envelope, curve) in cases {
        assert_eq!(open(envelope, curve).unwrap(), PLAIN, "{curve}");
        let (certificate, bare, _) = recipient(curve);
        let certificate = x509::parse(certificate).unwrap();
        let opened = parse(envelope)
            .unwrap()
            .open(&certificate, &PrivateKey::from_der(bare).unwrap())
            .unwrap();
        assert_eq!(opened, PLAIN, "{curve}, RFC 5915's bare encoding");
    }
}

/// Envelopes `openssl cms -encrypt` wrote to the generator's certificates: P-256 with its
/// defaults (`dhSinglePass-stdDH-sha1kdf-scheme`, `id-aes128-wrap`, AES-128-CBC), and P-384 with
/// `ecdh_kdf_md:sha384` (`id-aes256-wrap`, AES-256-CBC).
#[test]
fn envelopes_another_tool_wrote_open_too() {
    assert_eq!(open(fixture!("openssl-p256.der"), "p256").unwrap(), PLAIN);
    assert_eq!(
        open(fixture!("openssl-p384-sha384.der"), "p384").unwrap(),
        PLAIN
    );
}

/// RFC 5753 section 7.1.5's fourth key wrap, `id-alg-CMS3DESwrap`, in envelopes `openssl cms
/// -encrypt -des3 -wrap des3-wrap` wrote to the P-256 certificate, under the default KDF hash and
/// `ecdh_kdf_md:sha256`. Each states the wrap with RFC 3370 section 4.3.1's NULL parameters, its
/// KEK is 24 octets and `suppPubInfo` 192 bits, and what RFC 3217 section 3.2 unwraps is the
/// Triple-DES content key `des-ede3-cbc` decrypts with. Then the same tool's `-aes128 -wrap
/// des3-wrap`: a 16-octet key under the Triple-DES wrap is 32 octets wrapped, which section 3.2's
/// first step makes an error, and the reader names it rather than opening it (ADR 2827).
#[test]
fn a_triple_des_wrapped_key_opens_and_a_wrapped_key_of_another_length_is_named() {
    for name in ["sha1", "sha256"] {
        let envelope = if name == "sha1" {
            fixture!("openssl-p256-des3-wrap.der")
        } else {
            fixture!("openssl-p256-sha256-des3-wrap.der")
        };
        let RecipientInfo::KeyAgreement(agreement) = &parse(envelope).unwrap().recipients[0] else {
            panic!("a kari");
        };
        assert_eq!(
            agreement.wrap_algorithm,
            super::ID_ALG_CMS3DES_WRAP,
            "{name}"
        );
        assert_eq!(open(envelope, "p256").unwrap(), PLAIN, "{name}");
    }
    assert_eq!(
        open(fixture!("openssl-p256-aes128-des3-wrap.der"), "p256"),
        Err(EnvelopeError::KeyWrap(
            "1.2.840.113549.1.9.16.3.6 over a 32-octet wrapped key, where RFC 3217 section 3.2 \
             unwraps 40 octets"
                .to_owned()
        ))
    );
}

/// RFC 5652 section 6.2.2: the `KeyAgreeRecipientIdentifier` names "the recipient's certificate
/// by the issuer's distinguished name and the certificate serial number" or by `rKeyId`'s
/// `subjectKeyIdentifier`, which "matches the X.509 subjectKeyIdentifier extension value". In
/// `kari-p256-second-rek` the first entry's `rKeyId` is twenty `0x01` octets, which name no
/// certificate, and the second's is the P-256 certificate's own; a certificate neither names is no
/// recipient.
#[test]
fn the_entry_whose_rid_names_the_certificate_is_the_one_matched() {
    let envelope = parse(fixture!("kari-p256-second-rek.der")).unwrap();
    let RecipientInfo::KeyAgreement(agreement) = &envelope.recipients[0] else {
        panic!("a kari");
    };
    assert_eq!(agreement.recipients.len(), 2);
    assert_eq!(
        agreement.recipients[0].recipient,
        RecipientIdentifier::SubjectKeyIdentifier(&[1; 20])
    );
    let p256 = x509::parse(recipient("p256").0).unwrap();
    let Some(Recipient { encrypted_key, .. }) = envelope.recipient(&p256) else {
        panic!("the P-256 certificate is named");
    };
    assert_eq!(encrypted_key, agreement.recipients[1].encrypted_key);
    assert_eq!(
        agreement.recipients[1].recipient,
        RecipientIdentifier::SubjectKeyIdentifier(p256.key_identifier.unwrap())
    );

    let by_issuer = parse(fixture!("kari-p256-sha256-aes256.der")).unwrap();
    let RecipientInfo::KeyAgreement(agreement) = &by_issuer.recipients[0] else {
        panic!("a kari");
    };
    assert!(matches!(
        agreement.recipients[0].recipient,
        RecipientIdentifier::IssuerAndSerialNumber {
            serial: [0x20, 0x01],
            ..
        }
    ));

    let p384 = x509::parse(recipient("p384").0).unwrap();
    assert!(envelope.recipient(&p384).is_none());
    let p384_key = PrivateKey::from_der(recipient("p384").2).unwrap();
    assert_eq!(
        envelope.open(&p384, &p384_key),
        Err(EnvelopeError::NotARecipient)
    );
}

/// What this reader does not run is refused with its name, never read as "not a recipient": an
/// `originatorKey` whose `namedCurve` is not the recipient's (section 6.2.2's recipients "use the
/// same ... domain parameters"); an AES wrap stated with parameters, which RFC 3565 section 2.3.2
/// says "MUST be absent"; and an RSA key for a key-agreement recipient.
#[test]
fn what_is_not_run_is_refused_by_name() {
    assert!(matches!(
        open(fixture!("kari-p256-wrong-curve.der"), "p256"),
        Err(EnvelopeError::Malformed(reason)) if reason.contains("another curve")
    ));
    assert!(matches!(
        open(fixture!("kari-p256-wrap-parameters.der"), "p256"),
        Err(EnvelopeError::Malformed(reason)) if reason.contains("RFC 3565")
    ));
    let rsa = PrivateKey::from_der(&crate::rsa::tests::hex(crate::rsa::tests::PKCS8)).unwrap();
    let certificate = x509::parse(recipient("p256").0).unwrap();
    assert_eq!(
        parse(fixture!("kari-p256-sha256-aes256.der"))
            .unwrap()
            .open(&certificate, &rsa),
        Err(EnvelopeError::KeyKind("key-agreement"))
    );
    // brainpoolP512r1: the envelope reads and names its certificate, and the key is refused when
    // it is handed over, by the curve's name.
    assert!(matches!(
        PrivateKey::from_der(recipient("bp512").2),
        Err(crate::key::KeyError::Ec(crate::ec::EcError::Curve(name))) if name == "brainpoolP512r1"
    ));
}

/// One flipped bit in the wrapped key is the one decryption error: RFC 3394 section 2.2.3.1's
/// integrity check is what fails ("the unwrap must return an error and not return any key data"),
/// and it is not told apart from a key that does not open the envelope.
#[test]
fn a_damaged_wrapped_key_is_the_one_decryption_error() {
    let original = fixture!("kari-p256-sha256-aes256.der");
    let envelope = parse(original).unwrap();
    let RecipientInfo::KeyAgreement(agreement) = &envelope.recipients[0] else {
        panic!("a kari");
    };
    let wrapped = agreement.recipients[0].encrypted_key;
    let at = original
        .windows(wrapped.len())
        .position(|window| window == wrapped)
        .unwrap();
    let mut damaged = original.to_vec();
    damaged[at + wrapped.len() - 1] ^= 1;
    assert_eq!(open(&damaged, "p256"), Err(EnvelopeError::Decryption));
}

/// RFC 5753 section 7.2's `ECC-CMS-SharedInfo`, encoded by hand from its ASN.1 for
/// `id-aes256-wrap`: `keyInfo` the wrap's identifier with absent parameters, and `suppPubInfo`
/// "the length of the generated KEK, in bits, represented as a 32-bit number ... (For example, for
/// AES-256 it would be 00 00 01 00.)"; then the same with a `ukm` of `ab` in `entityUInfo [0]`.
#[test]
fn shared_info_is_rfc_5753_s_structure_in_der() {
    let without = [
        0x30, 0x15, // ECC-CMS-SharedInfo
        0x30, 0x0B, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01,
        0x2D, // keyInfo
        0xA2, 0x06, 0x04, 0x04, 0x00, 0x00, 0x01, 0x00, // suppPubInfo [2] EXPLICIT
    ];
    assert_eq!(shared_info(ID_AES256_WRAP, false, None, 32), without);
    let with = [
        0x30, 0x1B, // ECC-CMS-SharedInfo
        0x30, 0x0B, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01,
        0x2D, // keyInfo
        0xA0, 0x04, 0x04, 0x02, b'a', b'b', // entityUInfo [0] EXPLICIT
        0xA2, 0x06, 0x04, 0x04, 0x00, 0x00, 0x01, 0x00, // suppPubInfo [2] EXPLICIT
    ];
    assert_eq!(shared_info(ID_AES256_WRAP, false, Some(b"ab"), 32), with);
}

/// SEC 1 section 3.6.1, restated with the hash itself: `K1 || K2 || ...` with `Ki = Hash(Z ||
/// Counter || SharedInfo)`, `Counter` "a 4 octet, big-endian octet string" from `00000001`, and K
/// "the leftmost keydatalen octets". Under SHA-1 a 32-octet key takes two blocks and the second's
/// first twelve octets; under SHA-256 it is one whole block.
#[test]
fn the_kdf_is_sec_1_s_counter_mode_hash() {
    let z = [0x5A; 32];
    let info = b"shared";
    let block = |counter: u32| {
        let mut hash = Sha1::new();
        hash.update(z);
        hash.update(counter.to_be_bytes());
        hash.update(info);
        hash.finalize().to_vec()
    };
    let mut expected = block(1);
    expected.extend_from_slice(&block(2)[..12]);
    assert_eq!(x963_kdf::<Sha1>(&z, info, 32), expected);
    let mut hash = Sha256::new();
    hash.update(z);
    hash.update(1u32.to_be_bytes());
    hash.update(info);
    assert_eq!(x963_kdf::<Sha256>(&z, info, 32), hash.finalize().to_vec());
    assert_eq!(
        x963_kdf::<Sha256>(&z, info, 16),
        x963_kdf::<Sha256>(&z, info, 32)[..16]
    );
}

/// RFC 5753 section 3.2.3's receiving agent under 1-Pass ECMQV, over every variation `gen_mqv.py`
/// wrote: section 7.1.4's five `mqvSinglePass-*` hashes; each AES wrap of section 7.1.5; the
/// sender's static key named by `issuerAndSerialNumber` and by `subjectKeyIdentifier` with its
/// certificate in `originatorInfo` (section 3.2.1: "It SHOULD be one of the alternatives,
/// `issuerAndSerialNumber` or `subjectKeyIdentifier`") and carried as `originatorKey`; the
/// `ephemeralPublicKey`'s parameters absent, NULL and `namedCurve` (section 3.2.1: "MUST be absent,
/// `ECParameters`, or NULL"); an `addedukm`, which section 7.2 puts in `entityUInfo`; and the five
/// curves. The generator ran SEC 1 section 3.4's primitive as the sender with a static and an
/// ephemeral key and the recipient's static key as both of V's (SEC 1 section 5.2), checked against
/// NIST's responder vectors before writing anything; each opens to the plaintext it put in.
#[test]
fn each_generated_one_pass_mqv_envelope_opens_to_its_plaintext() {
    let cases: [(&[u8], &str); 8] = [
        (fixture!("mqv-p256-sha256-aes256.der"), "p256"),
        (fixture!("mqv-p256-sha1-aes128-addedukm-ski.der"), "p256"),
        (
            fixture!("mqv-p256-sha224-aes192-originator-key-null.der"),
            "p256",
        ),
        (fixture!("mqv-p256-sha512-named.der"), "p256"),
        (fixture!("mqv-p384-sha384-aes256.der"), "p384"),
        (fixture!("mqv-p521-sha512-aes256.der"), "p521"),
        (fixture!("mqv-bp256-sha256-aes128.der"), "bp256"),
        (fixture!("mqv-bp384-sha384-aes256.der"), "bp384"),
    ];
    for (envelope, curve) in cases {
        assert_eq!(open(envelope, curve).unwrap(), PLAIN, "{curve}");
    }
}

/// What section 3.2.1 requires of a 1-Pass ECMQV recipient, each failure on its own fixture where
/// only that member differs from `mqv-p256-sha256-aes256.der`:
///
/// - "ukm MUST be present", and without it there is no ephemeral key: malformed, by name;
/// - the `ephemeralPublicKey` named on P-384 for a P-256 recipient: malformed, by name;
/// - section 7.2's "the entityUInfo contains the octet string addedukm" — an envelope whose sender
///   put the whole `ukm` in `entityUInfo` derived another key, and does not open.
#[test]
fn a_one_pass_mqv_recipient_reads_its_ukm_as_section_3_2_1_states_it() {
    assert!(matches!(
        open(fixture!("mqv-p256-no-ukm.der"), "p256"),
        Err(EnvelopeError::Malformed(reason)) if reason.contains("section 3.2.1")
    ));
    assert!(matches!(
        open(fixture!("mqv-p256-ephemeral-wrong-curve.der"), "p256"),
        Err(EnvelopeError::Malformed(reason)) if reason.contains("another curve")
    ));
    assert_eq!(
        open(fixture!("mqv-p256-entity-info-is-ukm.der"), "p256"),
        Err(EnvelopeError::Decryption)
    );
}

/// RFC 5480 section 2.1.2: a key "restricted to the respective algorithm" by `id-ecMQV` opens a
/// 1-Pass ECMQV recipient and is refused a Diffie–Hellman one by name, and one restricted by
/// `id-ecDH` the other way round (`gen_mqv.py`'s `p256.ecmqv.p8.der` and `p256.ecdh.p8.der`, the
/// P-256 recipient's key under each identifier).
#[test]
fn a_restricted_key_opens_only_its_own_scheme_s_recipient() {
    let certificate = x509::parse(recipient("p256").0).unwrap();
    let mqv_only = PrivateKey::from_der(fixture!("p256.ecmqv.p8.der")).unwrap();
    let dh_only = PrivateKey::from_der(fixture!("p256.ecdh.p8.der")).unwrap();
    let mqv = parse(fixture!("mqv-p256-sha256-aes256.der")).unwrap();
    let dh = parse(fixture!("kari-p256-sha256-aes256.der")).unwrap();
    assert_eq!(mqv.open(&certificate, &mqv_only).unwrap(), PLAIN);
    assert_eq!(
        dh.open(&certificate, &mqv_only),
        Err(EnvelopeError::KeyKind("Diffie-Hellman key-agreement"))
    );
    assert_eq!(dh.open(&certificate, &dh_only).unwrap(), PLAIN);
    assert_eq!(
        mqv.open(&certificate, &dh_only),
        Err(EnvelopeError::KeyKind("1-Pass ECMQV key-agreement"))
    );
}

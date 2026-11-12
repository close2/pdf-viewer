//! `EnvelopedData` built by these tests from RFC 5652's own ASN.1, and opened.
//!
//! Every envelope here is assembled octet by octet from RFC 5652 section 6.1's definitions, its
//! content encrypted with RFC 3565's AES-CBC and section 6.3's padding, and its key with RFC 8017
//! section 7.2.1's encoding under [`crate::rsa::tests`]'s key. So what each test expects back is
//! the plaintext it put in, and no other program's reading of an envelope is consulted.

#![expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: offsets into envelopes these tests built, of a known size"
)]

use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockModeEncrypt, KeyIvInit};
use aes::{Aes128, Aes256};

use super::{
    EnvelopeError, ID_AES128_CBC, ID_AES256_CBC, ID_ENVELOPED_DATA, RecipientIdentifier,
    RecipientInfo, parse,
};
use crate::key::PrivateKey;
use crate::rsa::tests::{PKCS8, eme_pkcs1_v1_5, hex, rsaep};
use crate::x509::{self, RSA_ENCRYPTION};

/// A self-signed certificate for [`PKCS8`]'s key, serial `0x1995`, subject and issuer
/// `CN=pdf-pki test recipient`, made with `openssl req -x509 -key <key> -set_serial 0x1995`.
/// Only its issuer, serial number and subject key identifier are read.
const CERTIFICATE: &str = "\
3082020e30820177a00302010202021995300d06092a864886f70d01010b05003021311f301d06035504030c16706466\
2d706b69207465737420726563697069656e743020170d3236313131313030353231315a180f32313236313031383030\
353231315a3021311f301d06035504030c167064662d706b69207465737420726563697069656e7430819f300d06092a\
864886f70d010101050003818d0030818902818100eb365a97770f46d9cf516adba156d86d4a45f4de743c1d0f42d501\
1695197f9905cc16e982af627712a36ebeecad2188874759f439475a4d5b72119d28730c45684d2427652db31da8e8c4\
f295037635754eb1aed97fbb4172ae6272976cde84490032330c9f27bfae7697758d46cf0640538c525889a9cfb49518\
ac4fd4e4c10203010001a3533051301d0603551d0e041604141b0c8fde8f30ccbdbee3b32c179bc0bad090417e301f06\
03551d230418301680141b0c8fde8f30ccbdbee3b32c179bc0bad090417e300f0603551d130101ff040530030101ff30\
0d06092a864886f70d01010b050003818100add86f3cf58951adf30d367ffedeb82821ec48eee8eaa9abb27194b2063d\
46f57cb2eccd9ac02a18b7f472eca608613c51d3ba0801056fde837b5e04acc27a4cf6d6b2d8853857f623125d95057a\
5dea8c763e86fc79fa0983cd14ac74bad770680ee568437be5a3f0df047441c0a9fb9028e36a454d2db4b452c093e065\
cffc";

/// `id-data`, 1.2.840.113549.1.7.1 (RFC 5652 section 4).
const ID_DATA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x01];

/// X.690's definite-length encoding of one value.
fn tlv(identifier: u8, contents: &[u8]) -> Vec<u8> {
    let mut out = vec![identifier];
    let length = contents.len();
    if length < 0x80 {
        out.push(u8::try_from(length).unwrap());
    } else {
        let octets: Vec<u8> = length
            .to_be_bytes()
            .into_iter()
            .skip_while(|&byte| byte == 0)
            .collect();
        out.push(0x80 | u8::try_from(octets.len()).unwrap());
        out.extend_from_slice(&octets);
    }
    out.extend_from_slice(contents);
    out
}

fn sequence(parts: &[Vec<u8>]) -> Vec<u8> {
    tlv(0x30, &parts.concat())
}

/// An `AlgorithmIdentifier` with optional parameters.
fn algorithm(oid: &[u8], parameters: Option<Vec<u8>>) -> Vec<u8> {
    sequence(&[
        tlv(0x06, oid),
        parameters.unwrap_or_else(|| vec![0x05, 0x00]),
    ])
}

/// A `KeyTransRecipientInfo` for `rid`, its key RSA-encrypted under [`PKCS8`]'s public key.
fn key_transport(rid: Vec<u8>, content_key: &[u8], transport: &[u8]) -> Vec<u8> {
    let encrypted = rsaep(&eme_pkcs1_v1_5(content_key, 128));
    sequence(&[
        tlv(0x02, &[0]),
        rid,
        algorithm(transport, None),
        tlv(0x04, &encrypted),
    ])
}

/// The certificate's own `IssuerAndSerialNumber`, or one with a different serial.
fn issuer_and_serial(serial: &[u8]) -> Vec<u8> {
    let bytes = hex(CERTIFICATE);
    let certificate = x509::parse(&bytes).unwrap();
    sequence(&[tlv(0x30, certificate.issuer), tlv(0x02, serial)])
}

/// RFC 3565's AES-CBC with RFC 5652 section 6.3's padding.
fn encrypt(key: &[u8], iv: [u8; 16], plaintext: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; plaintext.len() + 16];
    let length = match key.len() {
        16 => cbc::Encryptor::<Aes128>::new_from_slices(key, &iv)
            .unwrap()
            .encrypt_padded_b2b::<Pkcs7>(plaintext, &mut out)
            .unwrap()
            .len(),
        _ => cbc::Encryptor::<Aes256>::new_from_slices(key, &iv)
            .unwrap()
            .encrypt_padded_b2b::<Pkcs7>(plaintext, &mut out)
            .unwrap()
            .len(),
    };
    out.truncate(length);
    out
}

/// A `ContentInfo` around an `EnvelopedData` with these recipients and this content.
fn envelope(recipients: &[Vec<u8>], cipher: &[u8], key: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let iv = [0x24; 16];
    let encrypted_content = sequence(&[
        tlv(0x06, ID_DATA),
        algorithm(cipher, Some(tlv(0x04, &iv))),
        tlv(0x80, &encrypt(key, iv, plaintext)),
    ]);
    let enveloped = sequence(&[
        tlv(0x02, &[0]),
        tlv(0x31, &recipients.concat()),
        encrypted_content,
    ]);
    sequence(&[tlv(0x06, ID_ENVELOPED_DATA), tlv(0xA0, &enveloped)])
}

/// §7.6.5.3's byte array: a 20-byte seed and four octets of permissions.
const SEED_AND_PERMISSIONS: &[u8; 24] = b"twenty octets of seed\xFF\xFF\xFC";

#[test]
fn an_envelope_opens_for_its_recipient_under_aes_128_and_aes_256() {
    // RFC 5652 section 6.2.1 then section 6.3, with both AES widths a document can state.
    let certificate_bytes = hex(CERTIFICATE);
    let certificate = x509::parse(&certificate_bytes).unwrap();
    let key = PrivateKey::from_der(&hex(PKCS8)).unwrap();
    for (cipher, content_key) in [(ID_AES128_CBC, &[7u8; 16][..]), (ID_AES256_CBC, &[9u8; 32])] {
        let rid = issuer_and_serial(&[0x19, 0x95]);
        let bytes = envelope(
            &[key_transport(rid, content_key, RSA_ENCRYPTION)],
            cipher,
            content_key,
            SEED_AND_PERMISSIONS,
        );
        let parsed = parse(&bytes).unwrap();
        assert_eq!(
            parsed.open(&certificate, &key).unwrap(),
            SEED_AND_PERMISSIONS
        );
    }
}

#[test]
fn the_scan_takes_the_recipient_the_certificate_names_and_skips_the_others() {
    // §7.6.5.1: "scan the recipient list ... and ... attempt to find a match with a certificate
    // that belongs to the user". The first entry is another serial and the second a password
    // recipient (`pwri`, `[3]`); the third is this certificate.
    let certificate_bytes = hex(CERTIFICATE);
    let certificate = x509::parse(&certificate_bytes).unwrap();
    let key = PrivateKey::from_der(&hex(PKCS8)).unwrap();
    let content_key = [3u8; 16];
    let other = key_transport(issuer_and_serial(&[0x19, 0x96]), &[1; 16], RSA_ENCRYPTION);
    let password = tlv(0xA3, &[0x02, 0x01, 0x00]);
    let mine = key_transport(
        issuer_and_serial(&[0x19, 0x95]),
        &content_key,
        RSA_ENCRYPTION,
    );
    let bytes = envelope(&[other, password, mine], ID_AES128_CBC, &content_key, b"x");
    let parsed = parse(&bytes).unwrap();
    assert_eq!(parsed.recipients.len(), 3);
    assert_eq!(parsed.recipients[1], RecipientInfo::Other(3));
    assert_eq!(parsed.open(&certificate, &key).unwrap(), b"x");

    // With this certificate's entry removed nothing matches, and the refusal says so.
    let other = key_transport(issuer_and_serial(&[0x19, 0x96]), &[1; 16], RSA_ENCRYPTION);
    let bytes = envelope(&[other], ID_AES128_CBC, &content_key, b"x");
    assert_eq!(
        parse(&bytes).unwrap().open(&certificate, &key),
        Err(EnvelopeError::NotARecipient)
    );
}

#[test]
fn a_subject_key_identifier_names_the_certificate_that_carries_it() {
    // RFC 5652 section 6.2.1's second `rid`, `[0]`, against RFC 5280's extension.
    let certificate_bytes = hex(CERTIFICATE);
    let certificate = x509::parse(&certificate_bytes).unwrap();
    let identifier = certificate
        .key_identifier
        .expect("openssl writes the extension");
    let rid = RecipientIdentifier::SubjectKeyIdentifier(identifier);
    assert!(rid.names(&certificate));
    assert!(!RecipientIdentifier::SubjectKeyIdentifier(&[0; 20]).names(&certificate));
    let key = PrivateKey::from_der(&hex(PKCS8)).unwrap();
    let content_key = [5u8; 16];
    let bytes = envelope(
        &[key_transport(
            tlv(0x80, identifier),
            &content_key,
            RSA_ENCRYPTION,
        )],
        ID_AES128_CBC,
        &content_key,
        b"by key identifier",
    );
    assert_eq!(
        parse(&bytes).unwrap().open(&certificate, &key).unwrap(),
        b"by key identifier"
    );
}

#[test]
fn an_algorithm_this_reader_does_not_decrypt_is_named() {
    // AES-128 in GCM mode (RFC 5084's `id-aes128-GCM`, 2.16.840.1.101.3.4.1.6), which is none of
    // the seven ciphers §7.6.5.3 lists, and a key transport other than the two this reader
    // opens with.
    let certificate_bytes = hex(CERTIFICATE);
    let certificate = x509::parse(&certificate_bytes).unwrap();
    let key = PrivateKey::from_der(&hex(PKCS8)).unwrap();
    let content_key = [3u8; 16];
    let rid = || issuer_and_serial(&[0x19, 0x95]);
    let gcm = [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01, 0x06];
    let bytes = envelope(
        &[key_transport(rid(), &content_key, RSA_ENCRYPTION)],
        &gcm,
        &content_key,
        b"x",
    );
    assert_eq!(
        parse(&bytes).unwrap().open(&certificate, &key),
        Err(EnvelopeError::Cipher("2.16.840.1.101.3.4.1.6".into()))
    );
    // PKCS #1's `rsaOAEPEncryptionSET`, 1.2.840.113549.1.1.6, which is not RFC 3560's
    // `id-RSAES-OAEP` and which no CMS specification assigns to key transport.
    let set_oaep = [0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x06];
    let bytes = envelope(
        &[key_transport(rid(), &content_key, &set_oaep)],
        ID_AES128_CBC,
        &content_key,
        b"x",
    );
    assert_eq!(
        parse(&bytes).unwrap().open(&certificate, &key),
        Err(EnvelopeError::KeyTransport("1.2.840.113549.1.1.6".into()))
    );
}

/// The envelopes `openssl cms -encrypt -keyopt rsa_padding_mode:oaep` wrote for [`CERTIFICATE`]
/// over [`SEED_AND_PERMISSIONS`] (`gen_envelopes.sh`, kept with the round that wrote them), each
/// under the `rsa_oaep_md`, `rsa_mgf1_md` and `rsa_oaep_label` its name gives, and the hash and
/// MGF1 hash RFC 3560 section 3's parameters must read back. OpenSSL states its choice in the
/// envelope's own `RSAES-OAEP-params`, writing SHA-1 as the absent default RFC 3560 asks a writer
/// for, so the reading is asserted before the opening, and what the opening must give back is the
/// plaintext the script put in.
#[test]
fn rsaes_oaep_envelopes_another_tool_wrote_open_under_their_stated_parameters() {
    use crate::rsa::{OaepHash, OaepParameters};
    let certificate_bytes = hex(CERTIFICATE);
    let certificate = x509::parse(&certificate_bytes).unwrap();
    let key = PrivateKey::from_der(&hex(PKCS8)).unwrap();
    let cases: [(&[u8], OaepHash, OaepHash, &[u8]); 5] = [
        (
            include_bytes!("../../tests/envelopes/oaep-default.der"),
            OaepHash::Sha1,
            OaepHash::Sha1,
            b"",
        ),
        (
            include_bytes!("../../tests/envelopes/oaep-sha256.der"),
            OaepHash::Sha256,
            OaepHash::Sha256,
            b"",
        ),
        (
            include_bytes!("../../tests/envelopes/oaep-sha384-label.der"),
            OaepHash::Sha384,
            OaepHash::Sha1,
            &[0x0A, 0x0B, 0x0C],
        ),
        (
            include_bytes!("../../tests/envelopes/oaep-sha224-mgf1-sha512.der"),
            OaepHash::Sha224,
            OaepHash::Sha512,
            b"",
        ),
        (
            include_bytes!("../../tests/envelopes/oaep-sha512-224.der"),
            OaepHash::Sha512_224,
            OaepHash::Sha512_256,
            b"",
        ),
    ];
    for (bytes, hash, mask_hash, label) in cases {
        let parsed = parse(bytes).unwrap();
        let RecipientInfo::KeyTransport {
            algorithm,
            parameters,
            ..
        } = &parsed.recipients[0]
        else {
            panic!("a ktri");
        };
        assert_eq!(*algorithm, crate::rsa::ID_RSAES_OAEP);
        assert_eq!(
            OaepParameters::read(*parameters),
            Ok(OaepParameters {
                hash,
                mask_hash,
                label,
            })
        );
        assert_eq!(
            parsed.open(&certificate, &key).unwrap(),
            SEED_AND_PERMISSIONS,
            "{hash:?} {mask_hash:?}"
        );
    }
    // RFC 3560 section 3's parameters are what the scheme runs with: the same envelope with its
    // label's last octet changed no longer opens, and is the one decryption error.
    let mut relabelled = cases[2].0.to_vec();
    let at = relabelled
        .windows(5)
        .position(|window| window == [0x04, 0x03, 0x0A, 0x0B, 0x0C])
        .unwrap();
    relabelled[at + 4] = 0x0D;
    assert_eq!(
        parse(&relabelled).unwrap().open(&certificate, &key),
        Err(EnvelopeError::Decryption)
    );
}

#[test]
fn a_damaged_key_or_content_is_the_one_decryption_error() {
    // The plant: one octet of the encrypted key, then one of the content's last block, flipped.
    let certificate_bytes = hex(CERTIFICATE);
    let certificate = x509::parse(&certificate_bytes).unwrap();
    let key = PrivateKey::from_der(&hex(PKCS8)).unwrap();
    let content_key = [3u8; 16];
    let good = envelope(
        &[key_transport(
            issuer_and_serial(&[0x19, 0x95]),
            &content_key,
            RSA_ENCRYPTION,
        )],
        ID_AES128_CBC,
        &content_key,
        SEED_AND_PERMISSIONS,
    );
    assert!(parse(&good).unwrap().open(&certificate, &key).is_ok());
    let mut parsed = parse(&good).unwrap();
    let last = parsed.encrypted_content.len() - 1;
    parsed.encrypted_content[last] ^= 0x01;
    assert_eq!(
        parsed.open(&certificate, &key),
        Err(EnvelopeError::Decryption)
    );

    let mut damaged = good.clone();
    let RecipientInfo::KeyTransport { encrypted_key, .. } = parse(&good).unwrap().recipients[0]
    else {
        panic!("a key-transport recipient");
    };
    let offset = good
        .windows(encrypted_key.len())
        .position(|window| window == encrypted_key)
        .unwrap();
    damaged[offset + 5] ^= 0x40;
    assert_eq!(
        parse(&damaged).unwrap().open(&certificate, &key),
        Err(EnvelopeError::Decryption)
    );
}

#[test]
fn what_is_not_an_enveloped_data_is_malformed_by_name() {
    assert!(matches!(
        parse(&[0x30, 0x00]),
        Err(EnvelopeError::Malformed(_))
    ));
    let signed = sequence(&[
        tlv(
            0x06,
            &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02],
        ),
        tlv(0xA0, &sequence(&[])),
    ]);
    assert_eq!(
        parse(&signed),
        Err(EnvelopeError::Malformed(
            "contentType is not id-envelopedData"
        ))
    );
    assert!(matches!(parse(&[0x30, 0x05]), Err(EnvelopeError::Der(_))));
}

/// The envelopes `openssl cms -encrypt` wrote for [`CERTIFICATE`] over [`SEED_AND_PERMISSIONS`],
/// one per cipher §7.6.5.3 lists "deprecated in PDF 2.0" (`gen_envelopes.py`, kept with the round
/// that wrote them: `openssl cms -encrypt -provider legacy -provider default -binary -outform DER
/// -<cipher>`). OpenSSL states the cipher it chose in the envelope's own
/// `contentEncryptionAlgorithm`, and the test asserts that this reader read the same one, so the
/// expected value is the plaintext the script put in and no reader's output.
const OPENSSL_ENVELOPES: [(&str, &[u8]); 6] = [
    ("des", include_bytes!("../../tests/envelopes/des.der")),
    ("des3", include_bytes!("../../tests/envelopes/des3.der")),
    ("rc2-40", include_bytes!("../../tests/envelopes/rc2-40.der")),
    ("rc2-64", include_bytes!("../../tests/envelopes/rc2-64.der")),
    ("rc2", include_bytes!("../../tests/envelopes/rc2.der")),
    ("rc4", include_bytes!("../../tests/envelopes/rc4.der")),
];

#[test]
fn each_deprecated_cipher_opens_an_envelope_another_tool_wrote() {
    // §7.6.5.3: "RC4 with key lengths up to 256-bits (deprecated in PDF 2.0)" and "DES, Triple
    // DES, RC2 with key lengths up to 128 bits (deprecated in PDF 2.0)". RC2 is written at 40, 64
    // and 128 effective key bits, the three RFC 3370 section 5.2 names, so each of its three
    // `rc2ParameterVersion` values (160, 120 and 58) is read once.
    let certificate_bytes = hex(CERTIFICATE);
    let certificate = x509::parse(&certificate_bytes).unwrap();
    let key = PrivateKey::from_der(&hex(PKCS8)).unwrap();
    for (name, bytes) in OPENSSL_ENVELOPES {
        let parsed = parse(bytes).unwrap();
        let (expected, bits) = match name {
            "des" => (super::ID_DES_CBC, None),
            "des3" => (super::ID_DES_EDE3_CBC, None),
            "rc2-40" => (super::ID_RC2_CBC, Some(40)),
            "rc2-64" => (super::ID_RC2_CBC, Some(64)),
            "rc2" => (super::ID_RC2_CBC, Some(128)),
            _ => (super::ID_RC4, None),
        };
        assert_eq!(parsed.content_algorithm, expected, "{name}");
        if let Some(bits) = bits {
            let Ok(super::ContentCipher::Rc2 { effective_bits, .. }) =
                super::ContentCipher::of(parsed.content_algorithm, parsed.content_parameters)
            else {
                panic!("{name} reads as RC2");
            };
            assert_eq!(effective_bits, bits, "{name}");
        }
        assert_eq!(
            parsed.open(&certificate, &key).unwrap(),
            SEED_AND_PERMISSIONS,
            "{name}"
        );
    }
}

#[test]
fn rc2_versions_read_back_through_rfc_2268_s_table() {
    // RFC 2268 section 6: a version below 256 is `Table[EKB]`, from 256 to 1024 it is EKB itself.
    // RFC 3370 section 5.2 and RFC 8018 appendix B.2.3 each name three pairs of the table, which is
    // a second document's reading of the same 256 octets: 40 bits are 160, 64 are 120, 128 are 58.
    for (version, bits) in [(160, 40), (120, 64), (58, 128), (256, 256), (1024, 1024)] {
        assert_eq!(super::rc2_effective_bits(version), Some(bits));
    }
    // `Table[0]` is 0xBD: zero effective bits is no key, and past 1024 is no version.
    assert_eq!(super::rc2_effective_bits(0xBD), None);
    assert_eq!(super::rc2_effective_bits(1025), None);
    let mut sorted = super::RC2_VERSION;
    sorted.sort_unstable();
    assert!(
        sorted.iter().enumerate().all(|(i, &v)| usize::from(v) == i),
        "\"Table[] specifies a permutation on the numbers 0-255\""
    );
    // An `RC2-CBCParameter` that is a bare IV is the default, 32 effective bits.
    let iv = tlv(0x04, &[0; 8]);
    let value = crate::der::Reader::new(&iv).unwrap().next_value().unwrap();
    assert_eq!(super::rc2_parameters(value).unwrap().0, 32);
    // The version is an `INTEGER`, and 160 is two octets; one octet `A0` is negative.
    let negative = sequence(&[tlv(0x02, &[0xA0]), tlv(0x04, &[0; 8])]);
    let value = crate::der::Reader::new(&negative)
        .unwrap()
        .next_value()
        .unwrap();
    assert_eq!(
        super::rc2_parameters(value),
        Err(EnvelopeError::Malformed("RC2-CBCParameter"))
    );
}

/// Block-level decryption with no padding to remove, for the known-answer tests below.
fn decrypt_blocks<C: aes::cipher::BlockCipherDecrypt>(
    cipher: C,
    iv: &[u8],
    body: &[u8],
) -> Vec<u8> {
    use aes::cipher::block_padding::NoPadding;
    use aes::cipher::{BlockModeDecrypt, InnerIvInit};
    let mut out = vec![0u8; body.len()];
    let length = cbc::Decryptor::<C>::inner_iv_slice_init(cipher, iv)
        .unwrap()
        .decrypt_padded_b2b::<NoPadding>(body, &mut out)
        .unwrap()
        .len();
    out.truncate(length);
    out
}

#[test]
fn each_cipher_decrypts_its_own_specification_s_known_answer() {
    use aes::cipher::{KeyInit, StreamCipher};
    // FIPS 81 Table C1, "AN EXAMPLE OF THE CIPHER BLOCK CHAINING (CBC) MODE": key 0123456789abcdef,
    // initialisation vector 1234567890abcdef, and the ASCII of "Now is the time for all ".
    let key = hex("0123456789abcdef");
    let iv = hex("1234567890abcdef");
    let ciphertext = hex("e5c7cdde872bf27c43e934008c389c0f683788499a7c05f6");
    let des = des::Des::new_from_slice(&key).unwrap();
    assert_eq!(
        decrypt_blocks(des, &iv, &ciphertext),
        b"Now is the time for all "
    );
    // Triple DES is "encrypt, decrypt, and encrypt" (RFC 3370 section 5.1), so with one key three
    // times over the first two cancel and it is FIPS 81's DES again; with two keys it is the
    // three-key cipher under K1, K2, K1.
    let three =
        des::TdesEde3::new_from_slice(&[key.clone(), key.clone(), key.clone()].concat()).unwrap();
    assert_eq!(
        decrypt_blocks(three, &iv, &ciphertext),
        b"Now is the time for all "
    );
    let two = des::TdesEde2::new_from_slice(&[key.clone(), key].concat()).unwrap();
    assert_eq!(
        decrypt_blocks(two, &iv, &ciphertext),
        b"Now is the time for all "
    );
    // RFC 2268 section 5's vectors, which state the effective key bits beside the key: one block,
    // so a zero initialisation vector makes CBC the bare cipher.
    for (key, bits, plaintext, ciphertext) in [
        (
            "0000000000000000",
            63,
            "0000000000000000",
            "ebb773f993278eff",
        ),
        (
            "ffffffffffffffff",
            64,
            "ffffffffffffffff",
            "278b27e42e2f0d49",
        ),
        (
            "88bca90e90875a7f0f79c384627bafb2",
            64,
            "0000000000000000",
            "1a807d272bbe5db1",
        ),
        (
            "88bca90e90875a7f0f79c384627bafb2",
            128,
            "0000000000000000",
            "2269552ab0f85ca6",
        ),
    ] {
        let rc2 = rc2::Rc2::new_with_eff_key_len(&hex(key), bits);
        assert_eq!(
            decrypt_blocks(rc2, &[0; 8], &hex(ciphertext)),
            hex(plaintext)
        );
    }
    // RFC 6229 section 2, "Key length: 128 bits" and "Key length: 256 bits": the keystream's
    // first sixteen octets, which a stream cipher applied to zeros returns.
    for (key, keystream) in [
        (
            "0102030405060708090a0b0c0d0e0f10",
            "9ac7cc9a609d1ef7b2932899cde41b97",
        ),
        (
            "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20",
            "eaa6bd25880bf93d3f5d1e4ca2611d91",
        ),
    ] {
        let mut out = [0u8; 16];
        rc4::Rc4::new_from_slice(&hex(key))
            .unwrap()
            .apply_keystream(&mut out);
        assert_eq!(out.to_vec(), hex(keystream));
    }
}

#[test]
fn a_two_key_triple_des_envelope_opens_as_k1_k2_k1() {
    // RFC 3370 section 5.1: "[t]he same algorithm identifiers are used for Three-Key Triple-DES
    // and Two-Key Triple-DES", so a 16-octet content key is two-key: K1, K2 and K1 again.
    use aes::cipher::block_padding::Pkcs7;
    use aes::cipher::{BlockModeEncrypt, KeyIvInit};
    let certificate_bytes = hex(CERTIFICATE);
    let certificate = x509::parse(&certificate_bytes).unwrap();
    let key = PrivateKey::from_der(&hex(PKCS8)).unwrap();
    let (k1, k2) = ([0x13u8; 8], [0x57u8; 8]);
    let iv = [0x24u8; 8];
    let mut ciphertext = vec![0u8; 32];
    let length = cbc::Encryptor::<des::TdesEde3>::new_from_slices(&[k1, k2, k1].concat(), &iv)
        .unwrap()
        .encrypt_padded_b2b::<Pkcs7>(SEED_AND_PERMISSIONS, &mut ciphertext)
        .unwrap()
        .len();
    ciphertext.truncate(length);
    let content_key = [k1, k2].concat();
    let enveloped = sequence(&[
        tlv(0x02, &[0]),
        tlv(
            0x31,
            &key_transport(
                issuer_and_serial(&[0x19, 0x95]),
                &content_key,
                RSA_ENCRYPTION,
            ),
        ),
        sequence(&[
            tlv(0x06, ID_DATA),
            algorithm(super::ID_DES_EDE3_CBC, Some(tlv(0x04, &iv))),
            tlv(0x80, &ciphertext),
        ]),
    ]);
    let bytes = sequence(&[tlv(0x06, ID_ENVELOPED_DATA), tlv(0xA0, &enveloped)]);
    assert_eq!(
        parse(&bytes).unwrap().open(&certificate, &key).unwrap(),
        SEED_AND_PERMISSIONS
    );
}

//! RFC 8017 section 7.1's encoding written by these tests and decrypted, and RFC 3560 section 3's
//! parameters read.
//!
//! Each encoded message is built here by section 7.1.1 step 2 from a seed the test chooses, so
//! what a decryption must give back is the message the test put in. The envelopes another tool
//! wrote under these parameters are opened in `crate::enveloped::tests`, which is what pins this
//! module's MGF1 against a second implementation's.

#![expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: offsets into encodings these tests built, of a known size"
)]

use super::{OaepHash, OaepParameterError, OaepParameters, decode_eme_oaep, mgf1};
use crate::der::Reader;
use crate::rsa::tests::{PKCS8, hex, rsaep};
use crate::rsa::{PrivateKey, RsaError};

/// RFC 8017 section 7.1.1 steps 2.a to 2.c, `DB = lHash || PS || 0x01 || M`, PS of
/// `k - mLen - 2hLen - 2` zero octets.
fn data_block(message: &[u8], label: &[u8], hash: OaepHash, k: usize) -> Vec<u8> {
    let mut db = hash.digest(&[label]);
    db.resize(k - message.len() - hash.length() - 2, 0x00);
    db.push(0x01);
    db.extend_from_slice(message);
    db
}

/// Section 7.1.1 steps 2.e to 2.i over `db`: `EM = 0x00 || maskedSeed || maskedDB`, with `seed` in
/// place of step 2.d's random octets.
fn mask(db: &[u8], mask_hash: OaepHash, seed: &[u8]) -> Vec<u8> {
    let db_mask = mgf1(mask_hash, seed, db.len());
    let masked_db: Vec<u8> = db.iter().zip(&db_mask).map(|(a, b)| a ^ b).collect();
    let seed_mask = mgf1(mask_hash, &masked_db, seed.len());
    let masked_seed: Vec<u8> = seed.iter().zip(&seed_mask).map(|(a, b)| a ^ b).collect();
    [vec![0x00], masked_seed, masked_db].concat()
}

/// Section 7.1.1 step 2 whole, for a `k`-octet modulus.
fn eme_oaep(
    message: &[u8],
    label: &[u8],
    hash: OaepHash,
    mask_hash: OaepHash,
    seed: &[u8],
    k: usize,
) -> Vec<u8> {
    mask(&data_block(message, label, hash, k), mask_hash, seed)
}

const ALL: [OaepHash; 7] = [
    OaepHash::Sha1,
    OaepHash::Sha224,
    OaepHash::Sha256,
    OaepHash::Sha384,
    OaepHash::Sha512,
    OaepHash::Sha512_224,
    OaepHash::Sha512_256,
];

#[test]
fn each_hash_pair_and_a_label_decrypt_what_section_7_1_1_encoded() {
    // RFC 8017 sections 7.1.1 and 7.1.2 are inverses for every `Hash` and MGF1 hash of appendix
    // A.2.1's set, the two chosen apart, with and without a label. SHA-512 as `Hash` needs
    // `k >= 2 * 64 + 2 = 130`, past this 128-octet key, which step 1.c refuses.
    let key = PrivateKey::from_der(&hex(PKCS8)).unwrap();
    let message = b"sixteen octets!!";
    for hash in ALL {
        for mask_hash in ALL {
            for label in [&b""[..], b"a label"] {
                let parameters = OaepParameters {
                    hash,
                    mask_hash,
                    label,
                };
                if hash == OaepHash::Sha512 {
                    assert_eq!(
                        key.decrypt_oaep(&[0x01; 128], &parameters),
                        Err(RsaError::Decryption)
                    );
                    continue;
                }
                let seed = vec![0x5C; hash.length()];
                let ciphertext = rsaep(&eme_oaep(message, label, hash, mask_hash, &seed, 128));
                assert_eq!(
                    key.decrypt_oaep(&ciphertext, &parameters).unwrap(),
                    message,
                    "{hash:?} {mask_hash:?} {label:?}"
                );
            }
        }
    }
}

#[test]
fn every_step_3_g_failure_is_the_one_decryption_error() {
    // The plant: section 7.1.2 step 3.g's three conditions, a PS octet that is neither 0x00 nor
    // the 0x01 that ends it, a label other than the one encoded, and step 1.b's length.
    let key = PrivateKey::from_der(&hex(PKCS8)).unwrap();
    let hash = OaepHash::Sha256;
    let seed = [0x33; 32];
    let parameters = OaepParameters {
        hash,
        mask_hash: hash,
        label: b"",
    };
    let good = eme_oaep(b"m", b"", hash, hash, &seed, 128);
    assert_eq!(
        decode_eme_oaep(&good, &hash.digest(&[b""]), hash),
        Some(b"m".to_vec())
    );
    // "if Y is nonzero".
    let mut nonzero_y = good.clone();
    nonzero_y[0] = 0x01;
    // "if lHash does not equal lHash'": decoded under another label.
    let other_label = hash.digest(&[b"x"]);
    // A PS octet that is neither 0x00 nor the 0x01 that ends it, written into DB before masking.
    let mut stray_db = data_block(b"m", b"", hash, 128);
    stray_db[32] = 0x02;
    let stray = mask(&stray_db, hash, &seed);
    // "no octet with hexadecimal value 0x01": an empty M whose 0x01 is turned into 0x00, so that
    // everything after lHash is zero.
    let mut bare_db = data_block(b"", b"", hash, 128);
    let last = bare_db.len() - 1;
    bare_db[last] = 0x00;
    let no_separator = mask(&bare_db, hash, &seed);
    assert_eq!(
        decode_eme_oaep(&nonzero_y, &hash.digest(&[b""]), hash),
        None
    );
    assert_eq!(decode_eme_oaep(&good, &other_label, hash), None);
    assert_eq!(decode_eme_oaep(&stray, &hash.digest(&[b""]), hash), None);
    assert_eq!(
        decode_eme_oaep(&no_separator, &hash.digest(&[b""]), hash),
        None
    );
    for planted in [nonzero_y, stray, no_separator] {
        assert_eq!(
            key.decrypt_oaep(&rsaep(&planted), &parameters),
            Err(RsaError::Decryption)
        );
    }
    let labelled = OaepParameters {
        label: b"x",
        ..parameters
    };
    assert_eq!(
        key.decrypt_oaep(&rsaep(&good), &labelled),
        Err(RsaError::Decryption)
    );
    assert_eq!(
        key.decrypt_oaep(&[0; 127], &parameters),
        Err(RsaError::Decryption)
    );
}

#[test]
fn an_empty_message_after_the_separator_is_a_message_of_no_octets() {
    let hash = OaepHash::Sha1;
    let encoded = eme_oaep(b"", b"", hash, hash, &[7; 20], 128);
    assert_eq!(
        decode_eme_oaep(&encoded, &hash.digest(&[b""]), hash),
        Some(Vec::new())
    );
}

/// One DER value.
fn tlv(identifier: u8, contents: &[u8]) -> Vec<u8> {
    let mut out = vec![identifier, u8::try_from(contents.len()).unwrap()];
    out.extend_from_slice(contents);
    out
}

/// `AlgorithmIdentifier` with `parameters` encoded after the object identifier.
fn identifier(oid: &[u8], parameters: &[u8]) -> Vec<u8> {
    tlv(0x30, &[tlv(0x06, oid), parameters.to_vec()].concat())
}

/// Reads `parameters` as the one value they encode.
fn read(parameters: &[u8]) -> Result<OaepParameters<'_>, OaepParameterError> {
    let value = Reader::new(parameters).unwrap().next_value().unwrap();
    OaepParameters::read(value)
}

const SHA256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
const SHA512_256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x06];
const SHA3_256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x08];
const MGF1: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x08];
const P_SPECIFIED: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x09];

#[test]
fn absent_members_are_rfc_3560_s_defaults_and_stated_ones_are_read() {
    // "Implementations that perform decryption MUST recognize both the id-sha1 object identifier
    // and an absent hashFunc field as an indication that SHA-1 was used", and likewise for MGF1
    // with SHA-1 and the empty label.
    assert_eq!(read(&tlv(0x30, &[])), Ok(OaepParameters::DEFAULT));
    // RFC 4055 section 2.1: "NULL and absent parameters" are "legal and equivalent encodings".
    let null = identifier(SHA256, &[0x05, 0x00]);
    let absent = identifier(SHA512_256, &[]);
    let stated = tlv(
        0x30,
        &[
            tlv(0xA0, &null),
            tlv(0xA1, &identifier(MGF1, &absent)),
            tlv(0xA2, &identifier(P_SPECIFIED, &tlv(0x04, b"label"))),
        ]
        .concat(),
    );
    assert_eq!(
        read(&stated),
        Ok(OaepParameters {
            hash: OaepHash::Sha256,
            mask_hash: OaepHash::Sha512_256,
            label: b"label",
        })
    );
}

#[test]
fn what_rfc_3560_does_not_state_is_malformed_and_what_is_not_run_is_named() {
    // "The AlgorithmIdentifier parameters field MUST be present".
    assert!(matches!(
        OaepParameters::read(None),
        Err(OaepParameterError::Malformed(_))
    ));
    assert!(matches!(
        read(&[0x05, 0x00]),
        Err(OaepParameterError::Malformed(_))
    ));
    // DER writes the members in their definition's order: `[1]` before `[0]` is not the syntax.
    let sha256 = identifier(SHA256, &[]);
    let out_of_order = tlv(
        0x30,
        &[tlv(0xA1, &identifier(MGF1, &sha256)), tlv(0xA0, &sha256)].concat(),
    );
    assert!(matches!(
        read(&out_of_order),
        Err(OaepParameterError::Malformed(_))
    ));
    // A hash identifier with parameters other than NULL.
    let with_parameters = tlv(0x30, &tlv(0xA0, &identifier(SHA256, &[0x04, 0x00])));
    assert!(matches!(
        read(&with_parameters),
        Err(OaepParameterError::Malformed(_))
    ));
    // SHA3-256 (2.16.840.1.101.3.4.2.8) is outside appendix A.2.1's set, as a hash and as MGF1's.
    let sha3 = identifier(SHA3_256, &[]);
    assert_eq!(
        read(&tlv(0x30, &tlv(0xA0, &sha3))),
        Err(OaepParameterError::Unsupported {
            member: "hashFunc",
            algorithm: "2.16.840.1.101.3.4.2.8".into(),
        })
    );
    assert_eq!(
        read(&tlv(0x30, &tlv(0xA1, &identifier(MGF1, &sha3)))),
        Err(OaepParameterError::Unsupported {
            member: "MGF1 hash",
            algorithm: "2.16.840.1.101.3.4.2.8".into(),
        })
    );
    // A mask generation function other than MGF1, and a label source other than id-pSpecified:
    // here `id-pSpecified`'s neighbour `{ pkcs-1 10 }` stands in for both.
    let other = [0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0A];
    assert_eq!(
        read(&tlv(0x30, &tlv(0xA1, &identifier(&other, &sha256)))),
        Err(OaepParameterError::Unsupported {
            member: "maskGenFunc",
            algorithm: "1.2.840.113549.1.1.10".into(),
        })
    );
    assert_eq!(
        read(&tlv(0x30, &tlv(0xA2, &identifier(&other, &tlv(0x04, b""))))),
        Err(OaepParameterError::Unsupported {
            member: "pSourceFunc",
            algorithm: "1.2.840.113549.1.1.10".into(),
        })
    );
}

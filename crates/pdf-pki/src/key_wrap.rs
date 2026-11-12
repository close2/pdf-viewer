//! RFC 3394's AES key unwrap and RFC 3217's Triple-DES one: how a key-agreement recipient's
//! content-encryption key arrives.
//!
//! RFC 5652 section 6.2.2's `encryptedKey` "is the result of encrypting the content-encryption key
//! in the pairwise key-encryption key", and RFC 5753 section 7.1.5 names the algorithms that do it
//! for an elliptic-curve recipient: "Triple-DES, AES-128, AES-192, and AES-256", the AES three
//! being RFC 3565 section 2.3.2's `id-aes128-wrap`, `id-aes192-wrap` and `id-aes256-wrap`, which
//! are RFC 3394's algorithm under a key of each width. This module is RFC 3394 section 2.2.2's
//! unwrap and section 2.2.3.1's integrity check, in the index-based form the RFC states beside the
//! register-shifting one: the two "produce\[] the same results", and the indexed one is the shorter
//! to read against the text.
//!
//! The algorithm is a dozen lines over the block cipher `aes` already supplies, so it is written
//! here rather than taken as a package (ADR 2821): the RFC's own section 4 vectors pin it. The
//! fourth algorithm, Triple DES, is RFC 3370 section 4.3.1's `id-alg-CMS3DESwrap`, which is RFC
//! 3217 section 3's wrap — two CBC passes under `des` and a SHA-1 checksum — and
//! [`unwrap_triple_des`] is its section 3.2, pinned by the section 3.4 example (ADR 2827).

use aes::cipher::consts::U16;
use aes::cipher::{BlockCipherDecrypt, KeyInit};
use aes::{Aes128, Aes192, Aes256, Block};
use des::TdesEde3;
use sha1::{Digest, Sha1};

/// RFC 3394 section 2.2.3.1: "A\[0] = IV = A6A6A6A6A6A6A6A6".
const DEFAULT_IV: [u8; 8] = [0xA6; 8];

/// One of RFC 3394's 64-bit blocks, in octets.
const SEMIBLOCK: usize = 8;

/// Why a wrapped key could not be unwrapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum KeyWrapError {
    /// The key-encryption key is not one the algorithm takes: 16, 24 or 32 octets for AES, 24
    /// for Triple DES.
    #[error("the key-encryption key is not a length the key wrap takes")]
    KeyLength,
    /// The ciphertext is not a length the algorithm unwraps. For AES, not a whole number of
    /// 64-bit blocks, or fewer than three: RFC 3394 section 2's "\[t]he only restriction the key
    /// wrap algorithm places on n is that n be at least two", and the ciphertext is n + 1 blocks.
    /// For Triple DES, not RFC 3217 section 3.2's 40 octets.
    #[error("the wrapped key is not a length the key wrap unwraps")]
    Length,
    /// RFC 3394 section 2.2.3.1: the recovered `A[0]` is not the default initial value, so "the
    /// unwrap must return an error and not return any key data"; or RFC 3217 section 3.2's
    /// checksum or parity check fails.
    #[error("the wrapped key fails RFC 3394's integrity check")]
    Integrity,
}

/// RFC 3394 section 2.2.2's unwrap of `ciphertext` under the AES key `kek`, checked against
/// section 2.2.3.1's default initial value.
///
/// # Errors
///
/// [`KeyWrapError::KeyLength`] for a key that is no AES width, [`KeyWrapError::Length`] for a
/// ciphertext that is not three or more whole blocks, and [`KeyWrapError::Integrity`] where the
/// check fails: a wrong key-encryption key or a damaged ciphertext, which the RFC does not tell
/// apart.
pub fn unwrap(kek: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, KeyWrapError> {
    match kek.len() {
        16 => unwrap_with(
            &Aes128::new_from_slice(kek).map_err(|_| KeyWrapError::KeyLength)?,
            ciphertext,
        ),
        24 => unwrap_with(
            &Aes192::new_from_slice(kek).map_err(|_| KeyWrapError::KeyLength)?,
            ciphertext,
        ),
        32 => unwrap_with(
            &Aes256::new_from_slice(kek).map_err(|_| KeyWrapError::KeyLength)?,
            ciphertext,
        ),
        _ => Err(KeyWrapError::KeyLength),
    }
}

/// Section 2.2.2's index-based unwrap, once the cipher is a type.
fn unwrap_with<C: BlockCipherDecrypt<BlockSize = U16>>(
    cipher: &C,
    ciphertext: &[u8],
) -> Result<Vec<u8>, KeyWrapError> {
    // "(n+1) 64-bit values" with n of at least two.
    let (Some((first, rest)), true) = (
        ciphertext.split_first_chunk::<SEMIBLOCK>(),
        ciphertext.len().is_multiple_of(SEMIBLOCK),
    ) else {
        return Err(KeyWrapError::Length);
    };
    let mut registers: Vec<[u8; SEMIBLOCK]> = rest
        .chunks_exact(SEMIBLOCK)
        .filter_map(|chunk| <[u8; SEMIBLOCK]>::try_from(chunk).ok())
        .collect();
    if registers.len() < 2 {
        return Err(KeyWrapError::Length);
    }
    // "1) Initialize variables. Set A = C\[0]; For i = 1 to n R\[i] = C\[i]" — `check` is A,
    // `registers` is R, `blocks` is n.
    let mut check = *first;
    let blocks = u64::try_from(registers.len()).map_err(|_| KeyWrapError::Length)?;
    // "2) Compute intermediate values. For j = 5 to 0, For i = n to 1: B = AES-1(K, (A ^ t) |
    // R\[i]) where t = n*j+i; A = MSB(64, B); R\[i] = LSB(64, B)" — `pass` is j, `index` is i,
    // `step` is t.
    for pass in (0..6u64).rev() {
        let mut index = blocks;
        for register in registers.iter_mut().rev() {
            // A ciphertext is bounded by `der::MAX_VALUE`, so `n * j + i` is far below 2^64.
            let step = blocks
                .checked_mul(pass)
                .and_then(|product| product.checked_add(index))
                .ok_or(KeyWrapError::Length)?;
            let mut block = Block::default();
            let (high, low) = block.split_at_mut(SEMIBLOCK);
            for (out, (check_octet, step_octet)) in
                high.iter_mut().zip(check.iter().zip(step.to_be_bytes()))
            {
                *out = check_octet ^ step_octet;
            }
            low.copy_from_slice(register);
            cipher.decrypt_block(&mut block);
            let (high, low) = block.split_at(SEMIBLOCK);
            check.copy_from_slice(high);
            register.copy_from_slice(low);
            index = index.saturating_sub(1);
        }
    }
    // "3) Output results. If A is an appropriate initial value (see 2.2.3), Then For i = 1 to n
    // P\[i] = R\[i] Else Return an error". Every octet is compared, whichever differs first.
    let difference = check
        .iter()
        .zip(DEFAULT_IV)
        .fold(0u8, |acc, (got, want)| acc | (got ^ want));
    if difference != 0 {
        return Err(KeyWrapError::Integrity);
    }
    Ok(registers.concat())
}

/// RFC 3217 section 3.2's "IV of 0x4adda22c79e82105", the outer CBC pass's.
const TRIPLE_DES_IV: [u8; 8] = [0x4A, 0xDD, 0xA2, 0x2C, 0x79, 0xE8, 0x21, 0x05];

/// RFC 3217 section 3.2's one length: "If the wrapped key is not 40 octets, then error."
pub const TRIPLE_DES_WRAPPED: usize = 40;

/// RFC 3217 section 3.2's Triple-DES key unwrap of `ciphertext` under the Triple-DES key
/// `kek`: what `id-alg-CMS3DESwrap` (RFC 3370 section 4.3.1) names, and the fourth key-wrap
/// algorithm RFC 5753 section 7.1.5 lists for an elliptic-curve recipient.
///
/// It unwraps a Triple-DES key and nothing else: the section's wrapped key is always 40 octets,
/// a 24-octet key, its 8-octet checksum and an 8-octet initialisation vector. The steps are
/// written out against the block cipher `des` already supplies, as [`unwrap`] is against `aes`
/// (ADR 2827).
///
/// # Errors
///
/// [`KeyWrapError::KeyLength`] for a key-encryption key that is not 24 octets,
/// [`KeyWrapError::Length`] for a wrapped key that is not 40 octets, and
/// [`KeyWrapError::Integrity`] where step 7's checksum or step 8's parity fails — a wrong
/// key-encryption key or a damaged ciphertext, which the RFC does not tell apart.
pub fn unwrap_triple_des(kek: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, KeyWrapError> {
    let cipher = TdesEde3::new_from_slice(kek).map_err(|_| KeyWrapError::KeyLength)?;
    // Step 1.
    if ciphertext.len() != TRIPLE_DES_WRAPPED {
        return Err(KeyWrapError::Length);
    }
    // Step 2: "Decrypt the wrapped key in CBC mode using the key-encryption key. Use an
    // initialization vector (IV) of 0x4adda22c79e82105. Call the output TEMP3."
    let temp3 = cbc_decrypt_des(&cipher, &TRIPLE_DES_IV, ciphertext);
    // Step 3: "Reverse the order of the octets in TEMP3".
    let temp2: Vec<u8> = temp3.into_iter().rev().collect();
    // Step 4: "IV is the most significant (first) 8 octets, and TEMP1 is the least significant
    // (last) 32 octets."
    let (iv, temp1) = temp2.split_at(DES_BLOCK);
    // Step 5: "Decrypt TEMP1 in CBC mode using the key-encryption key. Use the IV value from the
    // previous step".
    let cekicv = cbc_decrypt_des(&cipher, iv, temp1);
    // Step 6: "CEK is the most significant (first) 24 octets, and ICV is the least significant
    // (last) 8 octets."
    let (cek, icv) = cekicv.split_at(TRIPLE_DES_KEY);
    // Step 7, with section 2's checksum: "the most significant (first) eight octets" of the
    // key's SHA-1 digest. Every octet is compared, whichever differs first.
    let digest = Sha1::digest(cek);
    let mut difference = digest
        .iter()
        .zip(icv)
        .fold(0u8, |acc, (computed, stated)| acc | (computed ^ stated));
    // Step 8: "Check for odd parity each of the DES key octets comprising CEK."
    for octet in cek {
        difference |= u8::from(octet.count_ones() % 2 == 0);
    }
    if difference != 0 {
        return Err(KeyWrapError::Integrity);
    }
    // Step 9: "Use CEK as a Triple-DES key."
    Ok(cek.to_vec())
}

/// The DES block, and the length of RFC 3217's checksum and initialisation vector.
const DES_BLOCK: usize = 8;

/// A three-key Triple-DES key: three DES keys of eight octets.
const TRIPLE_DES_KEY: usize = 24;

/// CBC decryption of whole blocks under `cipher` from `iv`, no padding removed: `P[i] =
/// D(C[i]) ^ C[i-1]`, `C[0]` the vector. `body` is a whole number of blocks, which both callers'
/// lengths are by construction.
fn cbc_decrypt_des(cipher: &TdesEde3, iv: &[u8], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len());
    let mut previous = iv;
    for chunk in body.chunks_exact(DES_BLOCK) {
        let mut block = des::cipher::Block::<TdesEde3>::default();
        block.copy_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        out.extend(
            block
                .iter()
                .zip(previous)
                .map(|(plain, chain)| plain ^ chain),
        );
        previous = chunk;
    }
    out
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::arithmetic_side_effects,
        reason = "test code: offsets into the RFC's own vectors, of a known size"
    )]

    use super::{KeyWrapError, unwrap, unwrap_triple_des};

    fn hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
            .collect()
    }

    const KEK_256: &str = "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F";
    const KEY_DATA: &str = "00112233445566778899AABBCCDDEEFF000102030405060708090A0B0C0D0E0F";

    /// RFC 3394 section 4's six vectors, each "Ciphertext" unwrapped back to its "Key Data"
    /// under its "KEK": 4.1 to 4.3 a 128-bit key under each width, 4.4 and 4.5 192 bits, 4.6
    /// 256 bits.
    #[test]
    fn each_of_rfc_3394_section_4_s_vectors_unwraps_to_its_key_data() {
        let vectors = [
            (16, 16, "1FA68B0A8112B447AEF34BD8FB5A7B829D3E862371D2CFE5"),
            (24, 16, "96778B25AE6CA435F92B5B97C050AED2468AB8A17AD84E5D"),
            (32, 16, "64E8C3F9CE0F5BA263E9777905818A2A93C8191E7D6E8AE7"),
            (
                24,
                24,
                "031D33264E15D33268F24EC260743EDCE1C6C7DDEE725A936BA814915C6762D2",
            ),
            (
                32,
                24,
                "A8F9BC1612C68B3FF6E6F4FBE30E71E4769C8B80A32CB8958CD5D17D6B254DA1",
            ),
            (
                32,
                32,
                "28C9F404C4B810F4CBCCB35CFB87F8263F5786E2D80ED326CBC7F0E71A99F43BFB988B9B7A02DD21",
            ),
        ];
        let kek = hex(KEK_256);
        let data = hex(KEY_DATA);
        for (kek_octets, data_octets, ciphertext) in vectors {
            assert_eq!(
                unwrap(&kek[..kek_octets], &hex(ciphertext)).unwrap(),
                data[..data_octets],
                "RFC 3394 section 4, a {}-bit key under a {}-bit KEK",
                data_octets * 8,
                kek_octets * 8
            );
        }
    }

    /// Section 2.2.3.1: "If unwrapping produces A\[0] any other value, then the unwrap must
    /// return an error and not return any key data" — one flipped bit anywhere, or the wrong KEK.
    /// Section 2: n "be at least two", so a ciphertext of two blocks or a ragged one is refused.
    #[test]
    fn a_damaged_ciphertext_or_the_wrong_kek_fails_the_integrity_check() {
        let kek = hex(KEK_256);
        let good = hex("1FA68B0A8112B447AEF34BD8FB5A7B829D3E862371D2CFE5");
        for at in [0, 8, 23] {
            let mut damaged = good.clone();
            damaged[at] ^= 1;
            assert_eq!(unwrap(&kek[..16], &damaged), Err(KeyWrapError::Integrity));
        }
        assert_eq!(unwrap(&kek[16..], &good), Err(KeyWrapError::Integrity));
        assert_eq!(unwrap(&kek[..16], &good[..16]), Err(KeyWrapError::Length));
        assert_eq!(unwrap(&kek[..16], &good[..23]), Err(KeyWrapError::Length));
        assert_eq!(unwrap(&kek[..15], &good), Err(KeyWrapError::KeyLength));
    }

    /// RFC 3217 section 3.4's example, every value as the RFC prints it.
    const EXAMPLE_CEK: &str = "2923bf85e06dd6ae529149f1f1bae9eab3a7da3d860d3e98";
    const EXAMPLE_KEK: &str = "255e0d1c07b646dfb3134cc843ba8aa71f025b7c0838251f";
    const EXAMPLE_IV: &str = "5dd4cbfc96f5453b";
    const EXAMPLE_RESULT: &str = "690107618ef092b3b48ca1796b234ae9fa33ebb415960403\
         7db5d6a84eb3aac2768c632775a467d4";

    /// RFC 3217 section 3.1, steps 2 to 8, over `cek` as given (step 1's parity left to the
    /// caller, so that a test can wrap a key that breaks it), with `iv` as step 4's random octets.
    fn wrap_triple_des(kek: &[u8], cek: &[u8], iv: &[u8]) -> Vec<u8> {
        use des::TdesEde3;
        use des::cipher::{BlockCipherEncrypt, KeyInit};
        use sha1::{Digest, Sha1};
        let cipher = TdesEde3::new_from_slice(kek).unwrap();
        let cbc = |iv: &[u8], data: &[u8]| {
            let mut out = Vec::new();
            let mut previous = iv.to_vec();
            for chunk in data.chunks(8) {
                let mut block = des::cipher::Block::<TdesEde3>::default();
                for (slot, (a, b)) in block.iter_mut().zip(chunk.iter().zip(&previous)) {
                    *slot = a ^ b;
                }
                cipher.encrypt_block(&mut block);
                previous = block.to_vec();
                out.extend_from_slice(&block);
            }
            out
        };
        let icv = &Sha1::digest(cek)[..8];
        let cekicv = [cek, icv].concat();
        let temp1 = cbc(iv, &cekicv);
        let temp2 = [iv, &temp1].concat();
        let temp3: Vec<u8> = temp2.into_iter().rev().collect();
        cbc(&hex("4adda22c79e82105"), &temp3)
    }

    #[test]
    fn rfc_3217_section_3_4_s_example_unwraps_to_its_cek() {
        // Section 3.2 is section 3.1 run backwards: the example's RESULT under its KEK is its CEK.
        let kek = hex(EXAMPLE_KEK);
        let result = hex(EXAMPLE_RESULT);
        assert_eq!(result.len(), 40);
        assert_eq!(unwrap_triple_des(&kek, &result).unwrap(), hex(EXAMPLE_CEK));
        // The test's own wrap reproduces the RFC's RESULT, so the parity plant below is wrapped by
        // the algorithm the RFC states.
        assert_eq!(
            wrap_triple_des(&kek, &hex(EXAMPLE_CEK), &hex(EXAMPLE_IV)),
            result
        );
    }

    #[test]
    fn a_wrong_checksum_parity_length_or_kek_is_refused() {
        // The plant: one bit of the RESULT flipped fails step 7's checksum; a CEK whose last octet
        // has even parity, wrapped with a checksum that matches it, fails step 8; step 1's 40
        // octets; and a key-encryption key that is not three DES keys.
        let kek = hex(EXAMPLE_KEK);
        let result = hex(EXAMPLE_RESULT);
        for at in [0, 20, 39] {
            let mut damaged = result.clone();
            damaged[at] ^= 0x10;
            assert_eq!(
                unwrap_triple_des(&kek, &damaged),
                Err(KeyWrapError::Integrity)
            );
        }
        let mut even = hex(EXAMPLE_CEK);
        even[23] ^= 0x01;
        assert_eq!(even[23].count_ones() % 2, 0);
        let wrapped = wrap_triple_des(&kek, &even, &hex(EXAMPLE_IV));
        assert_eq!(
            unwrap_triple_des(&kek, &wrapped),
            Err(KeyWrapError::Integrity)
        );
        assert_eq!(
            unwrap_triple_des(&kek, &result[..32]),
            Err(KeyWrapError::Length)
        );
        assert_eq!(
            unwrap_triple_des(&kek[..16], &result),
            Err(KeyWrapError::KeyLength)
        );
    }
}

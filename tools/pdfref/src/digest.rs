//! SHA-256, because a cache key that can collide is a comparison against the wrong file.
//!
//! # Why a cryptographic digest, and why written here
//!
//! [`crate::cache`] keys a stored reference render on everything that decides what the
//! renderer produced, and the largest of those things is the document itself. A key that
//! collides does not fail; it silently hands back another file's picture, which is the exact
//! failure mode `doc/HANDOVER.md` warns about when it calls a cache of reference renders "the
//! obvious lever, with the equally obvious risk that a cache key omitting one variable would
//! compare against stale renders in silence". A 64-bit non-cryptographic hash is fine on
//! expectation and gives no argument; 256 bits gives one.
//!
//! Writing it here rather than taking a crate is the opposite of the choice ADR 0014 made
//! for JBIG2, and for the reason stated there: that decision turned on 19 400 lines of the
//! most error-prone code in the format. This is sixty lines of a fully specified algorithm
//! with published test vectors, in a tool that otherwise depends on nothing, and it is
//! checked against FIPS 180-4's own examples below.
//!
//! The specification here is FIPS 180-4, not ISO 32000-2 — which is why every reference below
//! spells out "FIPS 180-4 clause 6.2" rather than using the section sign. `tools/conformance`
//! reads a section sign anywhere in the tree as naming a clause of ISO 32000-2, and it would
//! be right to complain.

/// The 64 round constants: the first 32 bits of the fractional parts of the cube roots of
/// the first 64 primes. FIPS 180-4 clause 4.2.2.
const K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// The initial hash value: the first 32 bits of the fractional parts of the square roots of
/// the first eight primes. FIPS 180-4 clause 5.3.3.
const INITIAL: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// A SHA-256 computation in progress.
///
/// Incremental so that a 40 MB document can be hashed without a second copy of it in memory,
/// and so that the several parts of a cache key can be fed in one after another.
#[derive(Debug, Clone)]
pub struct Sha256 {
    state: [u32; 8],
    /// Bytes not yet part of a complete 64-byte block.
    pending: [u8; 64],
    /// How many of `pending` are occupied, always less than 64 between calls.
    pending_len: usize,
    /// Total message length in bytes, which the padding encodes in bits.
    length: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    /// A fresh computation.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: INITIAL,
            pending: [0; 64],
            pending_len: 0,
            length: 0,
        }
    }

    /// Adds `bytes` to the message.
    pub fn update(&mut self, bytes: &[u8]) {
        self.length = self
            .length
            .wrapping_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        let mut rest = bytes;
        while !rest.is_empty() {
            let take = 64_usize.saturating_sub(self.pending_len).min(rest.len());
            let (head, tail) = rest.split_at(take);
            // In range because `pending_len < 64` between calls and `take` is what is left.
            self.pending[self.pending_len..self.pending_len.saturating_add(take)]
                .copy_from_slice(head);
            self.pending_len = self.pending_len.saturating_add(take);
            rest = tail;
            if self.pending_len == 64 {
                let block = self.pending;
                self.compress(&block);
                self.pending_len = 0;
            }
        }
    }

    /// Adds `bytes` preceded by their length, so that concatenation cannot be ambiguous.
    ///
    /// Without this, a key built from `"a"` then `"bc"` and one built from `"ab"` then `"c"`
    /// would be the same digest — which for a cache key means two different renderer
    /// invocations sharing an entry.
    pub fn update_field(&mut self, bytes: &[u8]) {
        self.update(&u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
        self.update(bytes);
    }

    /// Finishes the computation and returns the digest.
    ///
    /// FIPS 180-4 clause 5.1.1: append a `1` bit, then `0` bits until the length is 56 modulo 64,
    /// then the message length in bits as a 64-bit big-endian integer.
    #[must_use]
    pub fn finish(mut self) -> [u8; 32] {
        let bits = self.length.wrapping_mul(8);
        self.update(&[0x80]);
        while self.pending_len != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());

        let mut digest = [0_u8; 32];
        for (word, out) in self.state.iter().zip(digest.chunks_exact_mut(4)) {
            out.copy_from_slice(&word.to_be_bytes());
        }
        digest
    }

    /// The digest as lowercase hexadecimal, which is what names a cache entry.
    #[must_use]
    pub fn hex(self) -> String {
        let digest = self.finish();
        let mut text = String::with_capacity(64);
        for byte in digest {
            // Two nibbles, most significant first.
            for nibble in [byte >> 4, byte & 0x0f] {
                text.push(char::from_digit(u32::from(nibble), 16).unwrap_or('0'));
            }
        }
        text
    }

    /// One 64-byte block, FIPS 180-4 clause 6.2.2.
    fn compress(&mut self, block: &[u8; 64]) {
        /// The message schedule's two σ functions, FIPS 180-4 clause 4.1.2.
        fn small_sigma0(x: u32) -> u32 {
            x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
        }
        fn small_sigma1(x: u32) -> u32 {
            x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
        }

        // The message schedule. Every index below is provably inside it: the first loop is
        // bounded by `chunks_exact(4)` over 64 bytes, and the second runs `16..64` and
        // subtracts at most 16.
        let mut w = [0_u32; 64];
        for (word, bytes) in w.iter_mut().zip(block.chunks_exact(4)) {
            *word = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }
        for t in 16..64 {
            w[t] = small_sigma1(w[t.saturating_sub(2)])
                .wrapping_add(w[t.saturating_sub(7)])
                .wrapping_add(small_sigma0(w[t.saturating_sub(15)]))
                .wrapping_add(w[t.saturating_sub(16)]);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for (schedule, constant) in w.iter().zip(K) {
            // FIPS 180-4 clause 4.1.2's Σ and the two choice functions, inline because naming them
            // separately buys nothing here and the standard writes them in this order.
            let big_sigma1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(big_sigma1)
                .wrapping_add(choose)
                .wrapping_add(constant)
                .wrapping_add(*schedule);
            let big_sigma0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = big_sigma0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }

        for (slot, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
}

/// The digest of a whole file, read in blocks.
///
/// # Errors
///
/// Whatever reading the file produced. The caller decides what an unreadable file means;
/// for the cache it means "do not cache", which is the safe direction.
pub fn of_file(path: &std::path::Path) -> std::io::Result<[u8; 32]> {
    use std::io::Read as _;

    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1 << 16];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            return Ok(hasher.finish());
        }
        hasher.update(buffer.get(..read).unwrap_or_default());
    }
}

#[cfg(test)]
mod tests {
    use super::Sha256;

    /// FIPS 180-4's own worked examples, plus the empty message from the same document's
    /// appendix. These are the reason this file is allowed to exist: an unverified hash
    /// implementation is worse than no cache at all, because it fails silently.
    #[test]
    fn the_published_test_vectors_are_reproduced() {
        let hex = |message: &[u8]| {
            let mut hasher = Sha256::new();
            hasher.update(message);
            hasher.hex()
        };

        // FIPS 180-4 Appendix B.1, one-block message.
        assert_eq!(
            hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // Appendix B.2, two-block message.
        assert_eq!(
            hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // The empty message, whose padding is the whole of what is being checked.
        assert_eq!(
            hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// The digest must not depend on how the message was split across calls, or a key fed
    /// in pieces would differ from the same key fed at once.
    #[test]
    fn feeding_the_message_in_pieces_gives_the_same_digest() {
        let message: Vec<u8> = (0..1000_u32)
            .map(|i| u8::try_from(i % 251).unwrap_or_default())
            .collect();
        let mut whole = Sha256::new();
        whole.update(&message);

        let mut pieces = Sha256::new();
        for chunk in message.chunks(7) {
            pieces.update(chunk);
        }
        assert_eq!(whole.hex(), pieces.hex());
    }

    /// Length-prefixed fields must not be able to run together, which is the property a
    /// cache key needs and a bare concatenation does not have.
    #[test]
    fn a_field_boundary_changes_the_digest() {
        let mut left = Sha256::new();
        left.update_field(b"a");
        left.update_field(b"bc");

        let mut right = Sha256::new();
        right.update_field(b"ab");
        right.update_field(b"c");

        assert_ne!(left.hex(), right.hex());
    }
}

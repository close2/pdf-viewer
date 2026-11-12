# 2821 — An elliptic-curve recipient opens by RFC 5753's key agreement

Session 2001, a ledger round (§7.6.5). Status: implemented. Rows: §7.6.5 to §7.6.5.3.
Builds on ADRs 2801, 2802, 2809 and 2810; carries out ADR 2801's item 4 under the owner's A800.

## Context

§7.6.5.3 has the key that encrypts the enveloped data "encrypted for each recipient, using that
recipient's public key", and §7.6.5.1's reader matches "a certificate that belongs to the user". A
certificate whose key is on an elliptic curve cannot be encrypted to; RFC 5652 section 6.2.2's
`KeyAgreeRecipientInfo` is how CMS reaches it, and RFC 5753 section 3.1 fixes that for elliptic
curves: ephemeral-static ECDH (SEC 1 section 3.3.1), SEC 1 section 3.6.1's KDF over section 7.2's
`ECC-CMS-SharedInfo`, and a key wrap. `pdf-pki` read `kari` as "other" and never matched it.

## Decision

1. **The user's key is one type of two families.** `pdf_pki::key::PrivateKey` is `Rsa` or `Ec`,
   read from any of RFC 5958's `OneAsymmetricKey`, RFC 8017's `RSAPrivateKey` and RFC 5915's
   `ECPrivateKey`, told apart by the second member. `pdf_syntax::RecipientKey::new` keeps its
   signature and takes either. A matched recipient of the kind the key cannot open is
   `EnvelopeError::KeyKind`, which `pdf-syntax` reports as `RecipientKey`: the document is fine.
2. **ECDH runs on the five curves the workspace already verifies on** (P-256, P-384, P-521,
   brainpoolP256r1 and brainpoolP384r1), each with its reviewed `RustCrypto` package. That is ADR
   0031's and the bigint swap's rule, and the packages are the ones `pdf-signature` pins. The scalar
   lives in `elliptic_curve::SecretKey`, hidden from `Debug` and zeroed on drop. The product is the
   package's constant-time multiplication. brainpoolP512r1 has no package and is refused by name,
   as `pdf_signature::ecdsa` refuses it. **The cofactor schemes are the standard ones here**: SEC 2
   and RFC 5639 section 3 give every one of these curves `h = 1`.
3. **No package is new to the workspace.** `pdf-pki` now depends on `p256`, `p384`, `p521`,
   `bp256`, `bp384`, `sha1` and `sha2`. `Cargo.lock` gains seven edges and no package. `pdf-syntax`
   builds the curve code, but nothing runs before a public-key document is opened with an EC key,
   so the launch path is unchanged.
4. **RFC 3394's unwrap is written in-tree** (`pdf_pki::key_wrap`), over `aes`, in the RFC's
   index-based form. It is a dozen lines, and section 4's six vectors pin it. `aes-kw` would be a
   new package for one loop.
5. **What runs**: the ten `dhSinglePass-stdDH-*` and `-cofactorDH-*` schemes, under SHA-1, -224,
   -256, -384 and -512; `id-aes128-wrap`, `-192-wrap` and `-256-wrap`; a `ukm`; all three
   `originatorKey` parameter forms RFC 5753 section 3.1.1 allows; compressed and uncompressed
   points (hybrid is refused, as section 7.2 requires). Section 6.2.2 says implementations "MUST
   support all three alternatives" for the originator, so an originator named by certificate is
   looked up in `originatorInfo`. `rid` matches by `issuerAndSerialNumber` or by `rKeyId`'s
   subject key identifier; `date` and `other` do not change which certificate is meant.
6. **What is refused by name**: 1-Pass ECMQV (`mqvSinglePass-*`, "OPTIONAL" in section 8, and it
   needs the sender's static key) and `id-alg-CMS3DESwrap` (RFC 3217, a "MAY"), each with its
   object identifier. An AES wrap stated with parameters is refused, because RFC 3565 section 2.3.2
   says they "MUST be absent". An `originatorKey` naming another curve is refused, because section
   6.2.2's recipients "use the same ... domain parameters".
7. **Fuzz targets from the first commit**: `ec_private_key` (both key readers and the primitive)
   and `key_unwrap`. `enveloped` now opens with a P-256 pair as well as the RSA one.

## Evidence

The expected value of every test is either a specification's own vector or the plaintext a
generator put in. The vectors are RFC 5903 section 8 and RFC 7027 appendix A for the primitive, and
RFC 3394 section 4 for the unwrap. The generator is `gen_kari.py`, and the session worked in
`~/scratch/agent/r2001/gen/`. It writes this tree's keys and certificates, with KDF, SharedInfo and wrap
written out step by step. `crates/pdf-syntax/tests/public_key_witnesses/key-agreement-s5.pdf` opens
end to end. `openssl cms` decrypts the generator's envelopes, except the static originator, which
it does not run. Envelopes it wrote open here too. Both are evidence, not the definition.

## What is left

ADR 2801's item 1, the host input, is the one item left (round 1997's lane). ADR 2801 recorded
that no document this tree holds uses a key-agreement recipient.

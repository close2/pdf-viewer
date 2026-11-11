# 2801 — The public-key encodings move below `pdf-syntax`, into `pdf-pki`

Session 1995, a ledger round (§7.6.5). Status: implemented. Rows: §7.6.5 to §7.6.5.3, §7.6.6.
Carries out the owner's answers A66 (the shape) and A800 (build now). ADR 2802 is the handler.

## Context

§7.6.5's public-key security handler reads RFC 5652 `EnvelopedData`, matches its recipients
against an X.509 certificate and decrypts with RSA. The reader of the first two and the big-integer
seam lived in `pdf-signature`, above `pdf-syntax`, where `crypt.rs` is; ADR 1134 found the call
impossible without a cycle and A66 chose the remedy: a crate below both, one implementation, one
fuzz surface, one audit point. A800 (2026-11-06) lifted A66's trigger and ordered the build.

## Decision

1. **`crates/pdf-pki`** depends on nothing in the workspace and holds `der`, `bigint` and `x509`,
   moved out of `pdf-signature`, and two new modules: `enveloped` (RFC 5652 section 6, opened for a
   key-transport recipient with RFC 3565's AES-CBC) and `rsa` (RFC 8017's private key in both
   encodings, RSADP and section 7.2.2's `RSAES-PKCS1-v1_5` decryption). `pdf-syntax` and
   `pdf-signature` both depend on it.
2. **`x509` keeps the public-key data types it builds** (`RsaPublicKey`, the DSA, ECDSA and EdDSA
   keys and their curves): a certificate's `subjectPublicKeyInfo` is X.509's to read. The
   verifications that need a curve package or a digest stay in `pdf-signature`, and so do
   `certificate_signature` (a certificate's signature checked) and the X.509 test fixtures.
   `pdf-signature` re-exports `der` and `x509` and each moved key type under its old name, because
   those types are in its own API; `pdf_signature::x509::Instant` stays the path its callers use.
3. **`cms` stays in `pdf-signature`.** A66 named a `der`/`cms`/`x509`/`bigint` seam, and A66's
   reason is one ASN.1 reader. `cms.rs` is not an ASN.1 reader of its own: it reads `SignedData` over
   `der`, and it calls the four signature families (`dsa`, `ecdsa`, `eddsa`, `pss`) and `timestamp`.
   Moving it would move the whole verification stack below the object reader, which A66 did not
   ask for. `SignedData` and `EnvelopedData` share only `der` and `x509`, and those moved.
4. **`bigint`'s "there is no secret here" ends at RSADP.** `d` is secret. `rsa` keeps it out of
   every `Debug`, and it exponentiates with `crypto-bigint`'s constant-time `pow`. RFC 8017 section
   7.2.2's note asks that the padding check not tell its failures apart, so it does not, by message
   or by work.
5. **Fuzz targets from the first commit**: `fuzz/fuzz_targets/enveloped.rs` (parse and open under a
   fixed test pair) and `fuzz/fuzz_targets/rsa_private_key.rs` (the key reader and the decryption).

## What is left, in order

1. The host input (ADR 1134 decision 2): a `RecipientKey` supplied by a host through
   `viewer_host::policy`, carried as a `viewer_core::Secret` on the open command into the confined
   worker, with the C ABI and the FFI counts. Until then `Document::open_with_recipient` is the
   handler's one caller outside the tests. It is a public entry point, and no host calls it yet.
2. §7.6.5.3's four deprecated envelope ciphers (RC4, DES, Triple DES, RC2), each refused by name.
   No document this tree holds uses one.
3. Table 27's per-stream recipient list: a crypt filter reached only from a stream's `/Crypt`
   decode parameters, with its own `/Recipients` string and its own key. It is refused by name,
   because `Encryption` holds one key.
4. RFC 5652's `KeyAgreeRecipientInfo` (an elliptic-curve recipient), which no document this tree
   holds uses either.

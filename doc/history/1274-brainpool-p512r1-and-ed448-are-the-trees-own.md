# 1274 — brainpoolP512r1 and Ed448 are the tree's own, behind the named-curve seam

Batch forty-two, the owner's answer A170. ADRs 1385 and 1386.

## Built
- **brainpoolP512r1** (`pdf-signature/src/brainpool_p512.rs`, private): RFC 5639 section 3.7's
  six constants over `primefield` + `primeorder` (the frames `bp256`/`bp384` are made of, on their
  `crypto-bigint` backend). `ecdsa::Curve` gains the variant; `UnsupportedCurve` is gone.
  Vectors: base point on the curve, `(q−1)G = −G`, RFC 7027 A.3's key agreement, an ECDSA
  signature under RFC 7027's `d_A` from an independent script, negatives (r, s, message, key x/y,
  r = q, s = q). `hybrid-array` 0.4.13 → 0.4.15 (the 513 array size); `cargo deny` green.
- **Ed448** (`pdf-signature/src/ed448.rs`, private): RFC 8032 section 5.2 over `crypto-bigint`'s
  constant-modulus Montgomery form, section 5.2.4's formulas, 5.2.3's decoding, 5.2.7's
  cofactored equation, SHAKE256 to 114 from `shake`. All nine section 7.4 vectors and both 7.5
  (Ed448ph through the same `dom4`) verify; negatives derived. `eddsa::Curve` is the seam;
  `x509::PublicKey::EdDsa`, `cms::SignatureAlgorithm::EdDsa`, `Family::EdDsa` carry the curve.
- **The fuzz run found a real case**: an order-4 Ed448 key (57 zero octets) made the all-zero
  signature verify over any message — the cofactored equation, correct per RFC 8032, under a key no
  key pair has. `EdDsaError::SmallOrderKey` refuses it on both curves; the crasher is a test.
- `cms`: `id-shake256-len` with 512 is SHAKE256 (RFC 8419 section 3.1); RFC 8702 section 3.1
  makes ADR 0390's 512 bits derived.
- End to end: `openssl` certificates and signatures for both curves, raw (no signed attributes)
  and CMS with signed attributes, through `Signature::integrity` and `authenticity`; the Ed448
  certificate's own signature through `x509::verify_signature`.

## Rows
§12.8.1, §12.8.3.1, §12.8.3.3, §12.8.3.3.1 partial → implemented. §12.8.3, §12.8, §12.1 stay
aggregates (§12.8.3.4.4's policy; §12.10). `doc/todo/65` bucket 2 bullet removed.

## Documents
`doc/stack.md` (A170's exception and the swap condition), `doc/state-of-play.md`, `doc/todo/51`,
`doc/todo/README.md`, `fuzz/seed_x509.py`; `viewer-core/src/notes.rs`'s three stale sentences.
RFC 7027 fetched to `scratchpad/r1274/` only: `doc/md/` is the owner's checkout.

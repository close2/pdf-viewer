# Q170 — May the tree implement the two unverified curves of its own?

Source: this round, on the owner's question of 2026-09-28 — "could we implement the missing
functionality for the signature verification?" — and `doc/todo/65`'s frontier list, which names the
two curves the packages have left. **Corrected in the same conversation it was asked in**: the
first writing of this file carried `doc/todo/51`'s four-curve table, measured 2026-08-23, and the
tree has moved since — the `bp256` and `bp384` crates landed on their stable 0.14.0 line and were
taken, so the missing set is two curves, and this file now says what the code says.

## What is missing, and what has landed since the older measurement

ISO/TS 32002 section 5.1.3's Table 3 names six ECDSA curves and its Table 4 two EdDSA ones. **Five of the
six and Ed25519 compute**: `ecdsa.rs`'s `Curve` is P-256, P-384, P-521, brainpoolP256r1 and
brainpoolP384r1, the last two over the `bp256` and `bp384` crates, and `eddsa.rs` computes
Ed25519 over `ed25519-dalek`. The missing set is **two curves**:

- **brainpoolP512r1** — RFC 5639 section 3.7's curve, carried as `ecdsa.rs`'s `UnsupportedCurve`,
  because **no crate on crates.io carries it at all**;
- **Ed448** — RFC 8032's second curve, named and not computed in `eddsa.rs`, because
  `ed448-goldilocks`' stable line carries the field arithmetic and no signature scheme, and its
  0.14 line is a pre-release (measured 2026-08-23, ADR 0532 — re-derive before believing it).

Both are refused at runtime by the certificate's own identifier, never skipped silently.

## Why own arithmetic is possible here, and what it would cost

The tree's established seam is the floor for both: every curve that has a crate is behind a crate,
and `bigint.rs`'s own rule — the budgets and refusal names ours while the multiplications are
reviewed code — is the shape the own implementations would take, over the reviewed `crypto-bigint`
that `pdf-signature` already depends on for RSA. brainpoolP512r1 needs field arithmetic for its
512-bit prime plus the Weierstrass group law, with the parameters and test vectors RFC 5639
section 3.7 prints; Ed448 needs the Goldilocks prime's field, the edwards448 curve and RFC 8032's
verification equation, hashed with the SHAKE256 that is already `cms::Digest`'s, with that RFC's
own test vectors. **Verification handles no secrets** — the signer's public key and the signature
are public — so the constant-time discipline that makes cryptographic arithmetic dangerous to
write does not bind this path; the danger is correctness, a wrong verification accepting a forged
signature, and the RFCs' positive vectors, negatives derived from them, and the module's existing
fuzz targets are the gate. `#![forbid(unsafe_code)]` holds as everywhere.

The cost is ownership: the tree becomes the maintainer of math no corpus or crawl document is
known to carry — the refusals are named at runtime; the build buys coverage of ISO/TS 32002's set,
not robustness any real file has asked for.

## What the tree does meanwhile

The two curves are refused by name at runtime — `UnsupportedCurve` for the Brainpool one,
`eddsa.rs`'s own naming for Ed448 — and the ledger rows state the measurement and its date.

## Recommendation

**Yes — implement both of the tree's own, as stopgaps behind the named-curve seam.** Parameters
and algorithms from RFC 5639 and RFC 8032 (both free, both held as licence-position entries),
positive vectors from the RFCs' own suites with negatives derived from them, the arithmetic in
`pdf-signature` as private modules beside the curves they join, fuzzed with the module's existing
targets, advertised nowhere — the crate's public surface stays the signature questions, never a
crypto API. The shape is switchable by construction: the day a stable, reviewed crate covers a
curve — `ed448-goldilocks`' 0.14.0 release is the one on the horizon — the round that takes the
swap decides it on `doc/stack.md`'s terms and records it in an ADR, module for module, the seam and
the named refusals untouched. For brainpoolP512r1 there is nothing to switch to today, and until
somebody publishes one, this is the only implementation there is.
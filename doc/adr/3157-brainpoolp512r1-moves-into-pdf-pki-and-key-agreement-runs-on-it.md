# 3157 — brainpoolP512r1 moves into `pdf-pki`, and key agreement runs on it

Session 2114, a clause round in the `lane3` worktree. Status: accepted and built. Rows: §7.6.5.3
(now `implemented`), §12.8.3.1 (the module's path). Context: `doc/questions/A170`, `A800`; ADRs
1385, 1538, 2801, 2821, 2827, 2904; RFC 5639 section 3.7, RFC 7027 appendix A.3, RFC 5753, SEC 1
sections 3.3.1 and 3.4.

## Context

§7.6.5.3's row was `partial` on one thing: a recipient key on brainpoolP512r1 was refused by the
curve's name, because no reviewed package carries the curve (ADRs 2827, 2904). `pdf-signature` has
carried the curve since ADR 1385 under the owner's exception A170 — RFC 5639 section 3.7's constants
over `primefield` and `primeorder`, the frames `bp256` and `bp384` are built from — and ADR 2827
declined to borrow it because it was "a verification module, variable-time over public values".

## The premise, checked against the code

That premise does not hold. `primeorder`'s `mul_backend::VariableOnly` names only the absence of a
precomputed base-point table; `ProjectivePoint::mul`, which `*` calls, is a fixed radix-16 walk over a
lookup table read by conditional selection, the same constant-time product `bp256` and `bp384` give
a secret scalar. The variable-time products are `mul_vartime` and `ecdsa`'s `u1·G + u2·Q`, which
key agreement does not call. So the module was never verification-only arithmetic; only its
documentation said so.

## Decision

1. **The module moves below `pdf-signature`** to `pdf_pki::ec::brainpool_p512`, and both crates
   use it: `ecdsa.rs` verifies on it unchanged (`pdf-signature` names it through
   `use pdf_pki::ec::brainpool_p512`), and `pdf_pki::ec::PrivateKey` holds a scalar on it and runs
   SEC 1's Diffie–Hellman and 1-Pass ECMQV primitives through the same generic functions as the
   five package curves. One statement of the six constants rather than two. `pdf-pki` implements
   `ecdsa::EcdsaCurve` for it because a trait may only be implemented beside its type; `ecdsa`,
   `primefield` and `primeorder` are already locked, so the lock gains edges and no package.
2. **A170's terms are unchanged**: a stopgap, advertised nowhere, swapped the day a stable
   reviewed crate covers the curve (`doc/todo/65`'s re-check). The swap is now one line per seam.
3. **§7.6.5.3 moves to `implemented`.** Nothing else in the row was owed.

## Tests

`ec::tests::the_primitive_agrees_with_rfc_5903_and_rfc_7027_s_vectors` gains RFC 7027 appendix
A.3 through `PrivateKey::agree`; the generated key reads in both encodings; envelopes open under
Diffie–Hellman (`kari-bp512-sha512-aes256.der`, written by round 2001's generator with the
recipient the tree already held), 1-Pass ECMQV (`mqv-bp512-sha512-aes256.der`, written by
`gen_mqv_bp512.py`, round 2044's script with RFC 5639's constants added and calibrated against
the `cryptography` package's own multiplication) and one `openssl cms -encrypt` wrote
(`openssl-bp512-sha512.der`, another program's reading, evidence only). The module's own four
tests moved with it.

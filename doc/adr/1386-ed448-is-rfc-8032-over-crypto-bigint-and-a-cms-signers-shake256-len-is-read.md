# ADR 1386 — Ed448 is RFC 8032 over `crypto-bigint`, and a CMS signer's `id-shake256-len` is read

## Status

Accepted, 2026-09-28. Session 1274. Carries out the owner's answer `doc/questions/A170` for its
second curve; amends ADR 0532, which named Ed448 and did not compute it, and ADR 0390's reading of
SHAKE256's output length, which the errata's deferral now settles.

`§N` is ISO 32000-2 and nothing else.

## Context

ISO/TS 32002 Table 4 pairs Ed448 with SHAKE256. `ed448-goldilocks` carries the signature scheme only
on its 0.14 pre-release line, which this tree does not take; `ed448` is a type crate with no
verification. A170: implement it here as a stopgap, unadvertised, easy to switch.

## Decision

1. **`crates/pdf-signature/src/ed448.rs` is RFC 8032 section 5.2, verification only**: GF(p) as
   `crypto-bigint`'s constant-modulus Montgomery form (no multiplication, reduction or
   exponentiation written here); section 5.2.4's projective addition and doubling; section 5.2.3's
   decoding with each of its three failures; section 5.2.7's cofactored equation — the one the RFC
   states, as `eddsa.rs` already chose for Ed25519; `H` as SHAKE256 to 114 octets from the `shake`
   package `cms::Digest` uses. `dom4(F, C)` is general inside the module so that section 7.4's
   context vector and section 7.5's Ed448ph vectors test the same function a document reaches; a
   document reaches only `F = 0` and the empty context, which is all RFC 8419 section 1 admits in
   CMS. No Ed448ph path is exposed.
2. **The seam is `eddsa::verify`'s `match` on the new `eddsa::Curve`.** `PublicKey`,
   `x509::PublicKey::EdDsa`, `cms::SignatureAlgorithm::EdDsa` and `signature::Family::EdDsa` carry
   the curve, and a `SignerInfo` naming one curve over a key on the other is
   `KeyDoesNotMatchAlgorithm` (RFC 8419 section 2.4 makes the two identifiers one number).
3. **The swap condition, verbatim from A170: the day a stable, reviewed crate covers the curve, the
   swap is decided on `doc/stack.md`'s terms.** The module's `verify` takes and answers exactly what
   the Ed25519 arm hands `ed25519-dalek`; a crate's verifier goes behind that arm, the vectors move
   to its tests, and the module is deleted.
4. **`id-shake256-len` with a parameter of 512 is SHAKE256** where a `SignerInfo` states it: RFC
   8419 section 3.1 requires it of an Ed448 signer with signed attributes. Any other length is
   reported by number. With RFC 8702 section 3.1 fixing `id-shake256`'s CMS digest at 64 octets,
   the 512 bits ADR 0390 took as a documented choice is derived from the two texts Errata
   Collection 3's issue #404 defers to.

5. **A key of small order is refused by name on both curves** (`EdDsaError::SmallOrderKey`;
   `is_weak` for Ed25519, `[4]A` neutral for Ed448). The `x509` fuzz target's first run found it:
   the Ed448 fixture certificate with its key mutated to 57 zero octets, `(±1, 0)` of order 4,
   under which the all-zero signature satisfies section 5.2.7's cofactored equation for every
   message. RFC 8032's verification does not reject such a key, and this is not a reading of that
   section: it is a refusal of a key no section 5.1.5 or 5.2.5 key pair has, under which
   "verified" would say nothing about any signer. The crasher is a permanent test in `eddsa.rs`.

## What is and is not constant-time

Field operations are constant-time; the double-and-add scalar multiplication and the decoding's
comparisons are not, deliberately (ADR 0229: no secret on a verification path).

## Evidence

Table 2's decimal `B` and `L` equal the constants; `B` is on the curve and `[L]B` is neutral; all
nine of section 7.4's vectors and both of section 7.5's verify, and each of 7.5's fails as plain
Ed448; one bit of message, `R`, `S`, key or context moved fails; `S + L` fails; the three decoding
failures and a wrong width are named refusals. An `openssl` Ed448 certificate verifies its own
signature through `x509::verify_signature`, and `openssl`'s raw and CMS (signed-attribute)
signatures verify through `Signature::authenticity`. OpenSSL writes `id-shake256` where RFC 8419
section 3.1 asks for `id-shake256-len` — evidence about OpenSSL, admitted by RFC 8702.

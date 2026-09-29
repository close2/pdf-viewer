# ADR 1385 — brainpoolP512r1 is the tree's own constants over the packages' own frames

## Status

Accepted, 2026-09-28. Session 1274. Carries out the owner's answer `doc/questions/A170` for the
first of its two curves; amends ADR 1063, which left brainpoolP512r1 refused for want of a package,
and stands beside ADR 0331 as the exception A170 grants to it.

`§N` is ISO 32000-2 and nothing else.

## Context

ISO/TS 32002 Table 3's sixth curve is RFC 5639 section 3.7's brainpoolP512r1. No stable, reviewed
crate carries it (`cargo search bp512` finds `bp512-nestler`, whose PolyForm-Noncommercial licence
this project cannot take; RustCrypto's `bp512` is an unmerged pull request). The owner's answer: the
algorithm belongs in a crypto library, implementing it here is a workaround until one exists, it is
not advertised, and it is to be built so that switching to a real library later is easy. A170's
`Owes:` line names "the reviewed `crypto-bigint` floor".

## Decision

1. **The module is constants, not arithmetic.** `bp256` and `bp384` are each `primefield` (a prime
   field over `crypto-bigint`'s Montgomery form) plus `primeorder` (the complete short-Weierstrass
   formulas) plus a curve's numbers. `crates/pdf-signature/src/brainpool_p512.rs` is exactly that
   construction with RFC 5639 section 3.7's `p`, `A`, `B`, `x`, `y` and `q`, using the frames'
   `crypto-bigint` backend (the packages default to generated `fiat-crypto` code, which does not
   exist for this prime). This meets the `Owes:` line's floor one level up and writes less of the
   verifier here than a hand-written group law would: the only numbers of the tree's own are the six
   constants and two quadratic non-residues (`2` modulo `p`, `7` modulo `q`, the smallest, as the
   frame's own test checks — whether either generates the whole group would need `p − 1` factored,
   and no verification step uses it).
2. **The seam is `ecdsa::verify`'s `match` on `Curve`**, which gains `BrainpoolP512r1`; the arm
   instantiates the one generic `verify_on`. `UnsupportedCurve` is deleted, because no Table 3
   curve is unsupported. A curve outside Table 3 is still `CurveNotVerifiable` by its own number.
3. **The swap condition, verbatim from A170: the day a stable, reviewed crate covers the curve, the
   swap is decided on `doc/stack.md`'s terms.** `BrainpoolP512r1` implements exactly the traits a
   curve package's type does, so the swap is that arm naming the package's type and this file
   deleted.
4. **One lock change**: `hybrid-array` 0.4.13 to 0.4.15, whose 0.4.15 adds the 513-element array a
   512-bit scalar's wNAF buffer needs. `primefield` and `primeorder` become direct dependencies and
   were already locked.

## What is and is not constant-time

The frames' field arithmetic and complete formulas are constant-time; `mul_backend::VariableOnly`
and `ecdsa`'s `u1·G + u2·Q` are not, deliberately. A verifier holds no secret (ADR 0229): key, `r`,
`s` and digest came out of the file.

## Evidence

RFC 5639 section 3.7's base point is on its curve and `(q − 1)·G = −G`; RFC 7027 appendix A.3's
key agreement is reproduced through SEC1 decoding (both `d·G = Q` and the shared point); an ECDSA
signature under RFC 7027's `d_A`, computed by an independent script of ANSI X9.62's equations,
verifies, and a bit of `r`, `s`, the message, or either coordinate of the key, `r = q` and `s = q`
each do not; an `openssl` certificate and signature verify end to end, with and without signed
attributes. BSI TR-03111's examples are not held and were not used.

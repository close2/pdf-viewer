# ADR 1063 — Two Brainpool curves taken, and two curves left for want of reviewed arithmetic

## Status

Accepted, 2026-09-14. Session 1048. Amends ADR 0532's arithmetic, which counted four of ISO/TS
32002's eight curves computed and four refused, and stands on ADR 0331's owner decision.

`§N` is ISO 32000-2 and nothing else.

## Context

ISO/TS 32002 section 5.1.3 appends Table 3 — three NIST curves and three Brainpool ones — to
§12.8.3.1, and its section 5.1.2 adds an EdDSA row over Ed25519 and Ed448 to Table 260. **This
tree does not hold ISO/TS 32002**, so the curve *names* reach it through §12.8.3's ledger rows and
ADR 0532, and its refusals were written as refusals by name. What ISO/TS 32002 does not carry
either way is the curves' parameters: RFC 5639 states all three Brainpool curves (sections 3.4,
3.6, 3.7) and RFC 8032 states Ed25519 and Ed448. Both are free, and `doc/md/rfc/` now holds RFC
5639 — downloaded rather than worked around, which is the owner's standing rule about a free
specification.

So the curves are not unspecified. What was missing, and is still missing for two of them, is a
*reviewed implementation*: ADR 0331 is the owner's decision that a verifier takes its group law
from a widely-used package rather than from arithmetic written here, because the defect class that
matters in a verifier is wrong arithmetic and a library has been run over shapes nobody here
thought of.

## Decision

**`bp256` 0.14.0 and `bp384` 0.14.0 are taken, and brainpoolP256r1 and brainpoolP384r1 are
computed.** Their line went stable on 2026-09-10 and 09-11 — ADR 0532 measured 0.14.0-rc.15 on
2026-08-23, and a pre-release is what this tree declines, not the package. They are the same
supplier as `p256`, `p384` and `p521`, on the same `digest` 0.11 line, `Apache-2.0 OR MIT`, MSRV
1.85, `#![forbid(unsafe_code)]`, and they add **no transitive package at all**: the lock gains the
two crates and nothing else, because `primefield` and `primeorder` are already here under the NIST
three. `ecdsa.rs`'s one generic `verify_on` covers all five NIST
and Brainpool curves, so the security-critical path is written once and not five times.

**brainpoolP512r1 and Ed448 are not taken, and are refused by their own identifiers.** No `bp512`
package exists on crates.io at all; `ed448-goldilocks`'s only line carrying the signature scheme
is 0.14.0-pre.15, and its stable 0.9.0 is field arithmetic on the old `rand_core` 0.6 stack with
no Ed448 signature verification in it (re-measured 2026-09-14). Implementing either here would be
in-tree arithmetic in a verifier, which ADR 0331 declines — so the answer a reader gets is
`Authenticity::CurveNotVerifiable` or `KeyNotVerifiable` naming the curve, never a guess and never
a silence.

**The distinction this ADR exists to fix is between two kinds of gap**, because the rows had been
recording one where there are two: a curve whose *parameters* nobody here holds is a silence about
the specification, and a curve whose parameters a free RFC states while no reviewed package
computes it is a silence about the *supply*. All four of the curves this project owed were the
second kind. Two of them stopped being a gap the week a package shipped, and nothing about the
standard had changed.

## Consequences

- §12.8.3.1, §12.8.3.3 and §12.8.3.3.1's rows say two of eight rather than four, and say which two
  and why, with the measurement dated.
- `Family::Ecdsa` carries two more curves, so `Family::name` and every report built on it name
  them.
- The refusal keeps a witness: `a_brainpool_p512_certificate_carries_its_curve_out_to_the_report`
  and `a_curve_this_program_does_not_compute_on_is_named_by_its_own_identifier` are the refusal,
  and `a_brainpool_signature_verifies_through_the_whole_path_a_document_takes` is its control — a
  refusal about brainpoolP512r1 rather than about everything Brainpool.
- The rows stay `partial`, and for trust as much as for the two curves: no host supplies an
  anchor, so no `Judgement` says valid (ADR 1039).

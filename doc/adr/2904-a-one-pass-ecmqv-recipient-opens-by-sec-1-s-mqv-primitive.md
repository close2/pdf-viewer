# 2904 — A 1-Pass ECMQV recipient opens by SEC 1's MQV primitive

Session 2044, a ledger round (§7.6.5.3). Status: implemented. Rows: §7.6.5, §7.6.5.3.
Takes the fourth of the effort-only refusals ADR 2821 left on §7.6.5.3, and re-checks the fifth.

## Context

RFC 5753 section 7.1.4 names fifteen key-agreement identifiers; ADR 2821 ran the ten
`dhSinglePass-*` and refused the five `mqvSinglePass-*` by identifier. Those are section 3.2's
1-Pass ECMQV, "method C(1, 2, ECC MQV) from [SP800-56A]": the sender has a static and an ephemeral
key, the recipient only its static one, and the receiving agent (section 3.2.3) takes the sender's
static key from `originator`, its ephemeral key from `ukm`'s `MQVuserKeyingMaterial`, and runs the
scheme with SEC 1 section 3.6.1's KDF. ADR 2827 said what it needs: arithmetic the reviewed curve
packages expose and `pdf_pki::ec` did not call.

## Decision

1. **The primitive is SEC 1 section 3.4's, run by the static-only party.** SEC 1 section 5.2 makes
   `Q2,V = Q1,V` "the default choice" for a 1-pass variant, and SP 800-56A section 5.7.2.3.2 has the
   static-only party use "his static private key as his second private key". So
   `ec::PrivateKey::agree_one_pass_mqv` computes `s = d + Q̄(Q)·d mod n` and `P = s·(Qe + Q̄(Qe)·Qs)`,
   `Q̄(x) = (x mod 2^⌈(log2 n)/2⌉) + 2^⌈(log2 n)/2⌉`, `⌈(log2 n)/2⌉` taken as `⌈L/2⌉` of the order's bit
   length `L` (they are equal for any `n` that is not a power of two). The scalar field and the group
   law are the curve packages' (`p256`, `p384`, `p521`, `bp256`, `bp384`), constant-time on the secret;
   nothing new enters the lock. Cofactor 1 on all five makes `h` vanish, as for ECDH.
2. **`entityUInfo` is `addedukm`, not `ukm`** (section 7.2), and `ukm` "MUST be present": its absence
   is malformed by name. The ephemeral key's algorithm is checked as `originatorKey`'s is (one function
   now serves both).
3. **RFC 5480 section 2.1.2's restriction is kept.** A `OneAsymmetricKey` naming `id-ecMQV` is now read
   (it was `NotEc`) and opens only an MQV recipient; one naming `id-ecDH` only a Diffie–Hellman one;
   the other is `EnvelopeError::KeyKind` by name.
4. **The known answers are NIST's.** CAVP's `KASTestVectorsECC2016` one-pass MQV responder vectors pin
   the primitive on P-256, P-384 and P-521 (`Z`, the scheme's own published value); the round's
   generator (`gen_mqv.py`, with the record) reproduces them before it writes the eleven envelopes,
   and was calibrated against the `cryptography` package's own scalar multiplication on all five curves
   — which caught a mistyped P-521 order before anything was written. No tool here writes ECMQV in CMS.

## brainpoolP512r1, re-checked 2026-11-16

`cargo info bp512` finds no package; RustCrypto's `elliptic-curves` repository holds `bp256` and
`bp384` and no `bp512`; the registry's only brainpoolP512r1, `bp512-nestler` 0.2.1, is still
PolyForm Noncommercial. ADR 2827's reasons for not borrowing `pdf-signature`'s verifier module stand,
so the key stays refused by the curve's name, and `doc/todo/65` section 5's re-check is the trigger.

## Consequences

All fifteen of section 7.1.4's schemes and all four of section 7.1.5's wraps run. What §7.6.5.3 still
refuses for effort is the one curve with no reviewed package.

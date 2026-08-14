# ADR 0331 — The arithmetic moves out and the construction stays

Status: accepted, 2026-08-14 (session 496). **An owner decision**: after reading ADR 0314, the
project owner decided that signature verification's big-integer arithmetic moves from the in-tree
`crates/pdf-model/src/bigint.rs` to a reviewed external dependency. Supersedes ADR 0314's
Decision 3 and part of its Decision 2's grounds, and ADR 0229's Decision 1 in part; both are
amended in place in the same commit. Follows ADRs 0215, 0229, 0314 and 0322.

## Context

Three verifiers — `pkcs1.rs` (RFC 8017's RSASSA-PKCS1-v1_5), `pss.rs` (RSASSA-PSS, sharing
`pkcs1::rsavp1`) and `dsa.rs` (FIPS 186-4 section 4.7) — ran on `bigint.rs`: a fixed 128-limb
array, Montgomery multiplication in the coarsely-integrated operand scanning form, and
square-and-multiply, written in tree on ADR 0229's argument that a verification has no secret and
therefore nothing for wide review's side-channel protection to act on.

That argument was true and is not the whole of what review buys. The defect class that matters in
a verifier is *wrong arithmetic* — a carry that propagates one limb short on one operand shape —
because RSA's encode-and-compare is only safe-in-one-direction if the exponentiation is right, and
DSA's `v = r'` comparison has no safe direction at all. A test vector pins the shapes it contains;
a widely-used integer library has been run over shapes nobody here thought of. The owner's
decision trades a small dependency surface for that second set of eyes, and this round's job was
to choose the dependency and do the port.

## Decision 1 — `crypto-bigint`, not the whole-scheme `rsa`/`dsa` crates

Both options were resolved in a scratch crate on 2026-08-14 and read with `cargo tree -e normal`,
which is ADR 0229's method:

| candidate | packages | line | verdict |
|---|---|---|---|
| `crypto-bigint` 0.7.5, `default-features = false, features = ["alloc"]` | 4 in a clean crate; **3 new compiled here** — itself, `ctutils`, `cmov` (`cpubits` and `num-traits` already in this tree; the lockfile also pins `base16ct` and `serdect`, optional and compiled by nothing) | no `digest` dependency at all | **taken** |
| `rsa` 0.9.10 (latest stable) | ~30, including `num-bigint-dig` with `rand`, `lazy_static`, `spin` | **`digest` 0.10** — the second hash stack ADR 0229 declined it for, still | declined |
| `dsa` 0.7.0 | ~20, including `crypto-primes`, `rfc6979`, `pkcs8`, strict `der` 0.8 | `digest` 0.11 — the right line, and on `crypto-bigint` itself | declined |

The wholesale option fails on three separate grounds, any one of which would do:

- **`rsa` stable is still on the old hash line.** ADR 0229 measured this in session 392 and the
  measurement has not expired: 0.10 remains a release candidate, and this tree takes no
  pre-release dependency.
- **The scheme crates parse with a strict DER stack this corpus contradicts.** Four of the ten
  corpus signature values begin `30 80` — X.690 clause 8.1.3.6's indefinite length — and the
  BER-tolerant `pdf_model::der`/`cms`/`x509` readers exist precisely because a strict reader loses
  them. A whole-scheme dependency re-parses keys and signatures on `der` 0.8 and would put the
  strict reader back into the path the tolerant one was written to guard.
- **The refusal vocabulary is part of the product.** `Pkcs1Error::ModulusTooLarge`,
  `DsaError::SubgroupTooLarge` and the rest are budgets this program states and reports by name;
  a scheme crate returns its own opaque errors, so keeping the census byte-identical and the
  reporting honest would mean wrapping every call in a re-implementation of exactly the layer the
  crates were supposed to replace. `dsa` 0.7.0 additionally brings signing machinery —
  deterministic-nonce generation, prime generation — that a verifier never calls.

`crypto-bigint` is the opposite shape: it is *only* the arithmetic, it is RustCrypto's — the
supplier of every cipher and digest this tree already takes (ADR 0031's precedent) — and its
packages are all `Apache-2.0 OR MIT` — inside `deny.toml`'s allow list, checked by running `cargo
deny check` over the changed graph: advisories, bans, licenses, sources all ok — with MSRV 1.85
against the pinned 1.97.1.
`default-features = false` is deliberate: the default `rand` feature would pull `rand_core` into
code that only ever verifies public numbers.

## Decision 2 — the port is a seam, not a rewrite of the verifiers

`bigint.rs` keeps its crate-internal API — `Integer`, `Modulus`, `modpow`, `significant_bits`,
`MAX_BITS` — and now contains **no arithmetic**: `from_be_bytes`/`be_bytes` convert between the
file's octet strings and `BoxedUint` (heap-allocated, precision chosen from the value's own
significant octets), and every operation is one `crypto-bigint` call — `BoxedMontyParams::
new_vartime` + `BoxedMontyForm::pow` for `modpow`, Montgomery-form `Mul` for `multiply_reduced`,
`rem_vartime` for `reduce`, `invert_vartime` for `invert`. The three verifiers are untouched but
for `dsa.rs`'s digest truncation, which no longer pokes limbs (`Integer::shifted_right` over the
library's unbounded shift) and lost its hand-rolled `shift_right` loop.

Four consequences of the seam, each a choice:

- **The budgets stay ours and stay in front.** `crypto-bigint`'s boxed integers would hold
  whatever a hostile file wrote; `MAX_BITS` 8192 and the callers' exponent/subgroup budgets are
  what keep work a constant of the module, and they are checked before any library call.
- **The `_vartime` spellings are deliberate.** There is no secret (ADR 0229's argument, which
  survives the port); taking the constant-time forms would claim a property nothing relies on.
- **`invert` changed its failure mode in the closed direction.** The old Fermat inversion answered
  a non-invertible value with a number that was simply not the inverse — safe, because `v ≠ r'`
  fails the signature; `crypto-bigint`'s inversion answers `None`, and `dsa::verify` already reads
  `None` as "does not verify". Same verdict, said sooner. FIPS 186-4 Appendix C.1 admits "an
  algorithm that produces an equivalent result", which this is.
- **A caller that hands a value above the modulus is reduced rather than wrong.** The old code's
  precondition ("base must already be below `n`") was unchecked; the seam's `try_resize` fallback
  reduces first, which is the honest repair. No current caller reaches it — `rsavp1` rejects an
  unreduced signature by RFC 8017 section 5.2.2 step 1 and `dsa` reduces `g` and `y` explicitly.

The evidence the port changed nothing observable, in the order it was gathered:

- **`signature_algorithm_census` over the same 67 460 documents (811 signature dictionaries in
  681), before and after: byte-identical**, verified by `diff` on the two captured outputs. Every
  verdict, every count, every identifier, every key width unchanged — 777 `Verified` across four
  key widths and three families' paddings, the same 17 `NotUnderThatKey`, the same refusals by
  number.
- Every existing test and KAT green: the workspace suite (1801, one more than the 1800 base — the
  seam's new shift test), the ten corpus signatures with their bit-flip counterparts, the
  `openssl` vectors for PKCS #1 v1.5, PSS at three parameter sets and DSA at `(2048, 224)`, and
  the RFC-derived PSS construction at both `emBits` widths.
- The `cms` and `x509` fuzz targets, re-seeded (28 corpus certificates) and run clean — the x509
  target exercises the new seam on every parsed key, asserting the budgets are still reported by
  name and that no signature ever verifies against a digest the target chose.

What was *not* kept: `bigint.rs`'s internal Montgomery machinery (`to_montgomery`, `multiply`,
`double`, `subtract`) and the two unit tests that addressed it directly, whose subject no longer
exists; the wide-modulus consistency test was rewritten against the seam's API (`x³` two ways at
2048 bits) so the property it pinned survives its subject.

## Decision 3 — ECDSA/EdDSA re-examined: one premise superseded, the refusal re-grounded

ADR 0314 refused the elliptic-curve family on four grounds, the heaviest being that the eight
curves' domain parameters are in no document this tree holds. **The owner's decision removes that
premise**: a reviewed curve crate carries P-256's prime as reviewed constants on exactly the
footing `crypto-bigint` carries its carry chains, so "we would be transcribing constants" no
longer decides anything. The family was therefore re-priced this round rather than left refused on
a dead argument, and the new measurements are:

- **`p256`, `p384` and `p521` 0.14.0 are now stable on the `digest` 0.11 line** — ADR 0229's
  pre-release objection has expired for the NIST curves. Cost, measured in the scratch crate:
  about **twenty new packages** for the first curve (`elliptic-curve`, `ecdsa`, `sec1`, `ff`,
  `group`, `primefield`, `primeorder`, `wnaf`, `hmac`, `rfc6979`, `der`, `spki`, `pkcs8`,
  `subtle`, `zeroize`, `rand_core`, `signature`, `pem-rfc7468`, `base64ct`, the curve itself),
  marginal per additional curve, all permissively licensed.
- **The Brainpool half is not takeable stably**: `bp256`'s current line is 0.14.0-rc — this tree
  takes no pre-release — its stable 0.6 is on the old hash line, and brainpoolP512r1 has no
  RustCrypto crate at all. Ed25519 is takeable (`ed25519-dalek` 3.0, BSD-3-Clause); Ed448 is not,
  and its SHAKE256 digest needs the `sha3` package this tree owes ISO/TS 32001 anyway.
- **The witnesses were finally identified by curve**, by extracting the three EC signers'
  certificates from the census's three documents: the one `ecdsa-with-SHA256` signature
  (`6100006.pdf`) is **P-256** with RFC 3279's DER `Dss-Sig-Value`; the two BSI TR-03111 plain
  signatures (`0300892.pdf`, `6696036.pdf` — Bundesanzeiger Verlag) are one **brainpoolP256r1**
  and one **P-256**, both in TR-03111's fixed-width `r ‖ s` encoding, which is defined in a BSI
  Technical Guideline this tree does not hold — writing that split from memory is the thing
  principle 5 forbids, independent of any curve crate.

So the arithmetic today: taking the stable packages would close **one signature of 811** (the DER
P-256 one), leave both plain witnesses refused (one for the encoding specification, one for the
encoding *and* the curve), and cover three of ISO/TS 32002's eight curves. That is ADR 0229's
"moving the silence rather than closing it", re-derived on current numbers. **ECDSA stays
declined-for-now**, with `doc/todo/51` re-grounded so the standing refusal no longer cites the
superseded premise. What would change it: a stable Brainpool pair on the current line plus the BSI
TR-03111 text in `doc/`, or a population that makes the family more than a rounding error — and
`doc/todo/51`'s own rule cuts the other way now too: EdDSA has zero witnesses, so it queues behind
the three ECDSA ones whatever the crates support.

## Consequences

- `Cargo.toml` gains `crypto-bigint` with the reasoning beside the setting; `deny.toml` needed no
  change — every added package passes the existing allow list, verified by `cargo deny check`, and no
  advisory, ban or source rule fires.
- `doc/stack.md`'s public-key section retitled: the *constructions* are in the tree, the
  arithmetic is not, and the dependency table a future round needs is restated with 2026-08-14
  measurements.
- Ledger: §12.8.3.1's note no longer grounds the ECDSA refusal on the domain-parameters premise;
  the §12.8.3 family's rows keep their statuses — nothing about *what is verified* moved, which
  is the census's byte-identical statement.
- **No gate that draws anything moves**, for the same reason as ADRs 0215, 0229, 0314 and 0322:
  no gate in this tree looks at a signature, and nothing outside §12.8's machinery was touched.
  The corpus and oracle raster gates cannot see this change.
- Nothing on the launch path changed: the verification still runs in `notes::about`, on the
  document's own thread, only for a document carrying a signature.

## The lesson

**A refusal can outlive its premise while its verdict stays right, and the two decay separately.**
ADR 0314's "the constants are in no document this tree holds" was a real argument for five days:
the moment the owner accepted reviewed arithmetic as a dependency, reviewed constants stood on the
same footing, and the ECDSA refusal was resting on a sentence that no longer decided anything —
while the *verdict* survived re-measurement for reasons (stable-line coverage, witness encodings)
the original ADR had ranked lower. Re-ground a refusal when its heaviest premise dies, even when
the conclusion does not move: the next round to read it will otherwise inherit the dead premise as
if it were load-bearing.

# ADR 0532 — The curves that were takeable, and the four that were not

Status: accepted, 2026-08-23 (session 689). Answers what was left of §12.8's question 2: Table 260's
third algorithm family and the fourth row ISO/TS 32002 adds beside it. **Supersedes ADR 0314's
Decision 2 and ADR 0331's Decision 3**, both of which refused the elliptic-curve family; each is
amended in place in the same commit. Follows ADRs 0215, 0229, 0314, 0322, 0331, 0390.

## Context

Two rounds refused this family and each refused it for different reasons, which is the shape
`doc/habits.md` says to distrust.

ADR 0314's heaviest premise was that the curves' domain parameters are in no document this tree
holds. ADR 0331 killed that premise — the owner's decision to take reviewed *arithmetic* puts
reviewed *constants* on the same footing — and re-grounded the refusal on two measurements instead:
that the stable packages covered three of eight curves, and that taking them would close **one
signature of 811**.

This round was asked to take the family. The three things it had to establish first were whether
those two measurements still hold, whether "three of eight" is a partial implementation or a
complete one with a named boundary, and where the object identifiers come from.

## The reading, and the sentence nobody had used

ISO/TS 32002 section 5.1.3's Table 3 names six ECDSA curves and its Table 4 two EdDSA ones. The
clause then does three things this round leaned on, and the third had not been quoted in this tree
before.

1. It **requires the named form**: "Certificates for ECDSA keys used in PDF signatures shall specify
   curve parameters (ECParameters) for the subject's public key using the namedCurve option … The
   implicitCurve and specifiedCurve options shall not be used."
2. It **permits a processor to stop at its own list**: "PDF processors may ignore or handle in an
   implementation-dependent manner PDF documents which are signed with elliptic curves not listed in
   Table 3 or Table 4."
3. Its **NOTE 2 settles the signature encoding**: "This restriction implies that ECDSA signature
   values are required to be represented using the DER-encoded ECDSA-Sig-Value type in IETF RFC
   5753:2010, section 7.2."

**The third retires a blocker `doc/todo/51` had been carrying since ADR 0314.** Two of the corpus's
three elliptic-curve witnesses are BSI TR-03111 *plain* signatures — `r ‖ s` as fixed-width octets
under `0.4.0.127.0.7.1.1.4.1.3` — and that file recorded them as a principle-5 blocker, because
TR-03111 is a document this tree does not hold. It is not a blocker and never needed to be: **the
standard states which encoding a conforming ECDSA signature uses, and it is not that one.** Those
two files are outside what ISO/TS 32002 admits, and reporting them by their own algorithm identifier
is the specification's answer rather than a gap. Nothing has to be read from a document we do not
have.

`tools/spec-errata emit doc/ISO_TS_32002-2022_sponsored_EC3.pdf` was run before any of this was
written, as `doc/todo/02` §4 requires. Two annotations touch the subject and neither changes it:
issue #404 strikes the footnote text pinning `id-sha512` and `id-shake256` in section 5.1.2,
deferring to RFC 8419 exactly as it does in ISO/TS 32001; issue #602 is a typographical strike in
clause 3. Table 3, Table 4 and NOTE 2 stand.

## The prices, re-derived rather than quoted (2026-08-23)

ADR 0331's table is five days old and `doc/todo/51` warns that a price decays. Re-measured against
crates.io and against a scratch crate resolved with this tree's `Cargo.lock`:

| ISO/TS 32002 names | package | line | taken |
|---|---|---|---|
| P-256 | `p256` 0.14.0 | stable | **yes** |
| P-384 | `p384` 0.14.0 | stable | **yes** |
| P-521 | `p521` 0.14.0 | stable | **yes** |
| brainpoolP256r1 | `bp256` | 0.14.0-**rc.15**; stable line is 0.6 on the old `digest` | no |
| brainpoolP384r1 | `bp384` | 0.14.0-**rc.15**; same | no |
| brainpoolP512r1 | — | **no crate on crates.io at all** | no |
| Ed25519 | `ed25519-dalek` 3.0.0 | stable | **yes** |
| Ed448 | `ed448-goldilocks` | stable 0.9.0 has the field arithmetic and **no signature scheme**, on `rand_core` 0.6; 0.14 is `-pre.15` | no |

So ADR 0331's measurement held for the Brainpool pair and this round adds the two facts it did not
have: `bp512` does not exist, and Ed448's stable crate is not a signature crate at all. **Four of
the eight curves are refusals of the packages, not of the standard**, and each is named at runtime
by the identifier the certificate states.

The dependency cost, as a lockfile delta against this tree's own `Cargo.lock` rather than against a
clean crate: **23 packages**, plus two patch bumps (`hybrid-array` 0.4.13 → 0.4.14, `keccak` 0.2.1 →
0.2.2). `cargo deny check` is green on all four sections with **no exception added**: `BSD-3-Clause`
— which `ed25519-dalek` and `curve25519-dalek` are under, and they are the only two — has been in
the allow list since it was written.

## Decision 1 — take the curves the standard names and the packages have, and name the rest

`crates/pdf-model/src/ecdsa.rs` and `crates/pdf-model/src/eddsa.rs`. The split is the standard's:
Table 3's short-Weierstrass curves are one module and Table 4's Edwards ones another, because they
are a different group law and — the difference that reaches the crate's shape — **EdDSA signs the
message rather than a digest of it**. `Signature::authenticity` therefore carries the signed bytes
as *parts* now, not only their digest, so a signature over a whole document still costs no copy of
it.

What is ours and what is the dependency's is drawn where ADR 0331 drew it. Ours: the encodings
(RFC 5753's `ECDSA-Sig-Value` read with this tree's own BER-tolerant `der`, not the strict one the
curve packages carry — four corpus signature values begin `30 80` and a strict reader loses them),
the budgets (a scalar wider than the field, a signature that is not 64 octets), and the refusal
vocabulary. Theirs: the group law, the point validation, and the digest truncation ANSI X9.62
applies.

**Three properties of the dependency are stated rather than assumed**, because a verifier that omits
one accepts forgeries and fails silently in the dangerous direction — ADR 0314's fourth ground, and
the one this decision has to answer rather than dismiss:

- `VerifyingKey::from_sec1_bytes` rejects a point that is not on the curve, is not a point, or is
  the identity.
- `Signature::from_scalars` rejects `r` or `s` that is zero or is not less than the curve order —
  ANSI X9.62's step 1 — before any arithmetic runs. `ecdsa.rs`'s own test feeds it those four cases
  one at a time and then verifies the untouched pair through the same builder, so the four failures
  are the check's rather than the test builder's.
- `ed25519-dalek` rejects an unreduced `S` in every `verify_*`, which is what RFC 8032 section 5.1.7
  makes mandatory.

**Constant time is *not* what these packages are taken for, and saying so is part of the decision.**
ADR 0229's argument survives unchanged: there is no secret. The point, the scalars, the message and
the digest all came out of a file a stranger wrote, so a timing channel has nothing to leak — which
is why `ecdsa`'s verification uses `_vartime` scalar multiplication and why that is correct here.
What review buys is the defect class that *does* matter in a verifier: wrong arithmetic on a shape
nobody here thought of.

## Decision 2 — RFC 8032's own equation, not the stricter one the crate offers

`ed25519-dalek` offers `verify_strict`, which adds the small-order rejection RFC 8032 section 5.1.7
calls optional — "It's sufficient, but not required, to instead check \[S\]B = R + \[k\]A'" — and
whose own documentation calls itself "technically non-RFC8032 compliant". `eddsa::verify` takes
`multipart_verify`, which is the equation the RFC states.

Principle 5 decides it and the direction of the mistake confirms it. The stricter check would refuse
a signature RFC 8032 says is valid, and a viewer reporting a conforming signature as failing is
wrong about the file. What the strict form closes is not forgery: it lets somebody who *already
holds* a valid signature produce a second value over the same message under the same key, which
changes nothing a document signature asserts.

## Decision 3 — the identifiers come from `const-oid`'s database, at zero new packages

This family needs a dozen object identifiers — `id-ecPublicKey`, six `namedCurve`s, four
`ecdsa-with-SHA*`, four `id-ecdsa-with-sha3-*`, `id-Ed25519`, `id-Ed448` — and **no document under
`doc/` prints one digit of any of them.** `cms::Digest::oid` records what that costs and takes it as
a documented transcription; doing the same again a dozen times over would have been the same cost
paid twelve times.

`const-oid` is already in this graph as `digest` 0.11's dependency. Naming it directly turns on its
`db` feature — a compiled-in table grouped by the RFC that assigns each identifier — at **zero new
packages**, `static` data so no parse cost at launch, and dead-code-eliminated down to the constants
actually named. Every number in both new modules is `const_oid::db::rfc5912::…`,
`rfc5639::…`, `rfc9688::…` or `rfc8410::…`. Nothing is typed from memory.

**And it retires a claim that had stood for a hundred and thirty-four sessions.** ADR 0390 recorded
that `id-shake256` "has no second reading here, because the package that computes it publishes no
identifier". That was true of `shake` 0.1 and stopped deciding anything the moment this feature came
into the crate for an unrelated family: `const_oid::db::fips202::ID_SHAKE_256` is a second party's
reading of the same registry. `cms.rs`'s identifier test now compares **nine of the ten** digests
against it — RIPEMD-160 is the one exception and stays one, because no package in this graph
publishes TeleTrusT's arc. The lesson is `CLAUDE.md`'s own, one directory over: *a silence about a
second reading decays exactly the way a claim about the specification does*, and this one outlived
its reason because nobody re-asked whether the number was already in the tree.

## Decision 4 — a curve is refused by *its own* identifier, not by the key algorithm's

`Authenticity::CurveNotVerifiable` is a new variant rather than a use of `KeyNotVerifiable`, and the
reason is that the existing variant would have told a reader nothing: **every** certificate on
**every** one of Table 3's six curves states the same `1.2.840.10045.2.1`. What distinguishes them
is a second identifier, in the `AlgorithmIdentifier`'s parameters, and that is what the report
carries — with Table 3's own spelling beside it where the curve is one the standard names, so
"brainpoolP256r1, refused" and "1.3.36.3.3.2.8.1.1.7, refused" are the same sentence and only one of
them can be looked up.

`ECParameters` that are not a `namedCurve` at all get the same variant carrying `None`, because
section 5.1.3 forbids that form outright and a file using it has left the Technical Specification.

## Evidence

- **`signature_algorithm_census` over the same 67 460 documents (811 signature dictionaries in
  681), before and after.** The report is about 750 lines and **the diff is 15 lines, all of them
  the intended one**: `1 AlgorithmNotVerifiable 1.2.840.10045.4.3.2` becomes
  `1 Verified (256-bit ECDSA (P-256))`; the key census resolves `3 1.2.840.10045.2.1` into
  `2 … P-256` and `1 … curve 1.3.36.3.3.2.8.1.1.7`, which is ADR 0331's hand identification of the
  three signers **re-derived by the program itself**; and the "names an algorithm this program does
  not verify" list loses `6100006.pdf` and keeps the two BSI ones. Every other line — 777 RSA
  verifications across four key widths, the 17 `NotUnderThatKey`, the digest counts, the 186
  indefinite-length values, the 338 revocation attributes, the 20 document timestamps — is
  byte-identical.
- **The real witness has a permanent test.**
  `tests/signatures.rs::the_crawls_one_ecdsa_signature_verifies_under_its_own_p256_certificate`
  opens `6100006.pdf` from the crawl, asserts `Family::Ecdsa(Curve::P256)`, and — the half that
  makes it an instrument rather than an assertion — turns one bit of the signature value over and
  requires that it stops verifying. It **skips and says so** where the machine-local crawl is
  absent, like every other corpus-dependent test here.
- **Fixtures for what no corpus can supply.** A positive verification over bytes *this tree* chose
  needs a key this tree made, and P-384, P-521 and Ed25519 have no witness in 67 460 documents at
  all (trap 8, and the same footing DSA is on). One self-signed certificate and one signature per
  curve, made once with `openssl`, plus a real brainpoolP256r1 certificate so that the refusal has a
  witness rather than a comment.
- Every existing test green, the ten pdf.js corpus signatures unchanged, `cargo deny check` green on
  advisories, bans, licenses and sources.

## Consequences

- **`Signature::authenticity` verifies every algorithm family the standard names.** What is left
  inside question 2 is four curves, each reported by its own identifier; `doc/todo/51` is rewritten
  around that rather than around a family.
- `Family` gains `Ecdsa(Curve)` and `EdDsa`, so the sentence a person reads says *which curve* did
  the verifying — Table 3 is a list of curves where Table 260's other rows are lists of key sizes.
- `Authenticity` gains `CurveNotVerifiable`, `RefusedEcdsa` and `RefusedEdDsa`; `viewer-core`'s
  `notes.rs` words all three.
- `x509::PublicKey` gains `Ec`, `Ed25519` and `EcCurveNotVerifiable`. It still reads no field that
  is a *trust* decision, which is unchanged and deliberate.
- **Nothing on the launch path changed**, for the reason ADRs 0215, 0229, 0314, 0322 and 0331 all
  give: the verification runs in `notes::about`, on the document's own thread, only for a document
  that carries a signature. What the dependency does cost is image size, which is why
  `precomputed-tables` is off on all four packages — a table is paged in at launch and a
  verification happens at most a handful of times per document, off that path.
- **No gate that draws anything moves.** No gate in this tree looks at a signature and nothing
  outside §12.8's machinery was touched.
- **`#![forbid(unsafe_code)]` is unchanged and still means what it always meant**: compiler-enforced
  over *this project's* source. `curve25519-dalek`'s SIMD backend — which its build script selects
  by default on x86_64 — carries `#[unsafe_target_feature]` intrinsics, and that is the same shape
  as `sha2`, `cmov`, `block-buffer` and `hybrid-array`, all of which have been on `pdf-model`'s path
  since ADR 0031 and ADR 0331. It is not a new class of exposure and it is written down in
  `Cargo.toml` beside the dependency rather than left to be discovered. `ed25519-dalek` itself
  carries `#![forbid(unsafe_code)]`; so do `p384`, `sec1`, `ff`, `wnaf`, `primefield` and
  `signature`.

## The lesson

**A blocker that names a document you do not hold should be checked against the document you do.**
`doc/todo/51` recorded BSI TR-03111's plain encoding as a principle-5 blocker for two of three
witnesses, and it survived two rounds and two ADRs. The answer was one sentence inside the clause
being implemented: ISO/TS 32002 section 5.1.3's NOTE 2 says which encoding a conforming ECDSA
signature uses, so those files are outside the standard and reporting them by number *is* the
correct behaviour. The blocker was never about a missing document; it was about a requirement nobody
had gone back to read.

The same round found the same shape twice more, which is why it is the lesson rather than an
anecdote: ADR 0390's "SHAKE256 has no second reading" was true of one package and false of the tree,
and ADR 0331's "three of eight curves" was right about the count while missing that two of the
remaining five are refused by *absence of a crate* rather than by a pre-release — a stronger and
more durable fact than the one that was written down.

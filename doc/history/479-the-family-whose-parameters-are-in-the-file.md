# 479 — The family whose parameters are in the file

**Finding.** Table 260 gives a PDF signature three public-key algorithm families and this program
verified one. The round was told to take ECDSA if the reading supported it and DSA if it did not,
and **the reading did not**: ISO/TS 32002 §5.1 appends six named ECDSA curves to Table 260 —
P-256, P-384, P-521 and three Brainpool — and adds EdDSA over Ed25519 and Ed448 beside them, so
"ECDSA" in the standard family is eight curves across two group laws rather than the five
`doc/todo/51` had recorded; and every one of their **domain parameters is in no document this tree
holds**, which is a principle-5 blocker rather than an arithmetic one. DSA's four numbers are in
the certificate. So DSA is implemented — FIPS 186-4 §4.7 over RFC 3279's `Dss-Parms` and
`Dss-Sig-Value` — the elliptic-curve family is refused with the argument written down, and no
dependency was taken.

**Date.** 2026-08-13.
**ADR.** [0314](../adr/0314-the-family-whose-parameters-are-in-the-file.md).
**Touched.** `crates/pdf-model/src/dsa.rs` (new), `crates/pdf-model/src/bigint.rs` (new — `pkcs1`'s
integers moved out with `reduce`, `multiply_reduced` and `invert` added),
`crates/pdf-model/src/pkcs1.rs` (trimmed to RFC 8017), `crates/pdf-model/src/x509.rs`
(`PublicKey::Dsa`, `read_dsa_key`, two errors), `crates/pdf-model/src/cms.rs`
(`SignatureAlgorithm::Dsa`, the digest claim corrected, `fixtures::detached_dsa`),
`crates/pdf-model/src/signature.rs` (`Family`, the two-way dispatch,
`KeyDoesNotMatchAlgorithm`, `UnknownDigest`'s identifier, two tests),
`crates/pdf-model/examples/signature_algorithm_census.rs` (new),
`crates/viewer-core/src/notes.rs` (the sentences name the family),
`fuzz/fuzz_targets/x509.rs` (the DSA arm and its budget),
`doc/conformance/ledger.toml` (§12.8, §12.8.3, §12.8.3.1, §12.8.3.2, §12.8.3.3, §12.8.3.3.1,
§12.8.3.4, §12.8.3.4.5), `doc/stack.md`, `doc/verify.md`, `doc/crate-map.md`,
`doc/todo/51-*`, `doc/todo/README.md`, `doc/HANDOVER.md` (one claim that stopped being whole),
`doc/adr/0314-*`, this file.

## What the population said, and it named a third thing

The census reads 67 460 documents — `doc/pdf.js`'s 974, `doc/corpora`'s 275 and all 66 211 of the
SafeDocs crawl — and finds 811 signature dictionaries in 681 of them. **Not one states a DSA key or
a DSA signature algorithm**, so this round's code has no demand witness and its fixture is
hand-built, which is trap 8's own prescription. ECDSA's witness is three signatures, and two of
those three are BSI TR-03111 *plain* ECDSA, whose value is `r ‖ s` rather than a DER
`Dss-Sig-Value` — so the packages a dependency decision would have taken would close one signature
of 811.

**And `id-RSASSA-PSS` is twice as common as ECDSA.** Six real signatures use it, it sits *inside*
the "RSA Algorithm Support" row this tree calls done — that row states key sizes and names no
padding — and it needs no dependency and no external constant. It is the top of `doc/todo/51` now,
which is a thing the round was not sent to find.

## Two claims about the standard that had decayed

`cms::Digest` said its six were "the digest algorithms Table 260 and Table 256 name, and nothing
else". ISO/TS 32001 §5.1.3 and §5.1.4 add SHA3-256, SHA3-384, SHA3-512 and SHAKE256 to both tables,
and this program computes none of them; `Authenticity::UnknownDigest` now carries the identifier so
that the gap has a number.

The §12.8.3.2 ledger row said PKCS #1's DSA was a gap this program owed. Table 260's own "DSA
Algorithm Support" row says **No** in the `adbe.x509.rsa_sha1` column, so it never was one — the
clause's sentence is about what the PKCS #1 standard supports and the table is what says which of
those a `/SubFilter` may carry. DSA is therefore implemented in the CMS path only, and that is a
reading rather than a shortcut.

## What the tests had to be built around

No private key in this tree can sign the *signed attributes* of a CMS object, because the digest
inside them is a digest of the document that contains them. So the end-to-end fixture uses the one
shape RFC 5652 permits that closes the circle: a `SignerInfo` with no signed attributes at all,
whose signature is therefore over the byte range itself — `bug854315.pdf`'s shape — which lets a
signature made once by `openssl` over sixteen chosen bytes drive the whole path a document takes.

The vector `openssl dsaparam 2048` produced is `(L, N) = (2048, 224)`, which turned out to matter:
SHA-256 is 256 bits, so FIPS 186-4 §4.7's `z = the leftmost min(N, outlen) bits` is a real
truncation in this fixture and a verifier that skipped the rule fails it. On `(2048, 256)` it would
have passed.

The test that discriminates is step 1's, and it was written by deleting nothing: `r = 0`, `s = 0`,
`r = q` and `s = q` each re-encoded into the fixture's own `Dss-Sig-Value`, and then the real pair
put through the same builder so that the four failures are their own rather than the builder's.

## The gates

The whole of `doc/todo/02` §2 ran after the last edit: 1734 tests pass, the corpus gate's
incomplete list is unchanged, both text gates are unchanged, quorra is unchanged, the oracle's
verdicts are unchanged at a 99.8% reference-cache hit rate, and the conformance checker passes with
the new quotations verified against `doc/md/`. `cargo deny` is clean on all four checks with
**`Cargo.toml` and `Cargo.lock` untouched**, which is the claim this round most needed to be able
to make.

The `x509` fuzz target covers the new decoder and the new arithmetic, seeded with the DSA
certificate — without it nothing reaches `dsa::verify` at all, since no corpus document has a DSA
key — and it is clean at **1 000 000 runs** with no artefact, in 2325 s at 440 executions a second
against the RSA-only target's microseconds: a DSA verification with a fuzzer-chosen key at both
budgets is the expensive thing in that target, which is the budgets being exercised rather than a
problem. `cms` was re-run for the `SignatureAlgorithm` change and is clean at 1 000 000 in 6 s.

Three documentation sweeps ran — `conformance`'s rustdoc quotation checker, `spec-errata check`
over the standard's PDFs, and the prose quotation sweep over every Markdown file this project wrote
— and none of them finds anything in this round's files.

# 689 — The curves that were takeable, and the blocker that was a sentence away

Date: 2026-08-23. ADR [0532](../adr/0532-the-curves-that-were-takeable-and-the-four-that-were-not.md).

Touched: `crates/pdf-model/src/ecdsa.rs` (new), `crates/pdf-model/src/eddsa.rs` (new),
`crates/pdf-model/src/{lib.rs,x509.rs,cms.rs,signature.rs,pkcs1.rs,dsa.rs}`,
`crates/pdf-model/examples/signature_algorithm_census.rs`,
`crates/pdf-model/tests/signatures.rs`, `crates/viewer-core/src/notes.rs`,
`fuzz/fuzz_targets/x509.rs`, `Cargo.toml`, `crates/pdf-model/Cargo.toml`, `Cargo.lock`;
`doc/conformance/ledger.toml` (§12.8, §12.8.3, §12.8.3.1, §12.8.3.2, §12.8.3.3, §12.8.3.3.1),
`doc/todo/51-signatures-and-public-keys.md`, `doc/stack.md`, `doc/state-of-play.md`.

The subject was what was left of §12.8's question 2: Table 260's ECDSA family and the EdDSA that
ISO/TS 32002 adds beside it, both of which two previous rounds had refused with an argument.

## What landed

`pdf_model::ecdsa` verifies ISO/TS 32002 Table 3's P-256, P-384 and P-521 over RFC 5753's DER
`ECDSA-Sig-Value`; `pdf_model::eddsa` verifies its Table 4's Ed25519 over RFC 8032's own
verification equation. `Signature::authenticity` therefore answers question 2 for **every algorithm
family the standard names**. What is left inside it is four curves — the three Brainpool ones and
Ed448 — each named at runtime by the identifier the certificate states.

The arithmetic is `p256`, `p384`, `p521`, `ecdsa` and `ed25519-dalek`, on ADR 0331's owner decision.
The encodings, the budgets and the refusal vocabulary stay in the tree, for the reason ADR 0331 gave
about the strict DER reader those packages carry: four of the corpus's signature values begin
`30 80` and a strict reader loses them.

## The three things the briefing said to check, and what checking them found

**"A price is a claim that decays."** All eight of ADR 0331's curve prices were re-derived against
crates.io on the day. Six held. Two were *stronger* than what was written down: `bp512` does not
exist on crates.io at all rather than merely being a pre-release, and `ed448-goldilocks`'s stable
0.9.0 is field arithmetic with **no signature scheme** on `rand_core` 0.6 rather than simply an old
line. Both are more durable facts than the ones they replace, which is why they are now in
`doc/todo/51` as a table with a "what would change it" column rather than as prose.

**"Probe a zero before believing it."** The census in `doc/todo/51` was documented as
`find corpus-cache … -name '*.pdf'`, and in a parallel worktree `corpus-cache` is a **symlink** into
the main checkout. `find` without `-L` descends nothing: the first run of that documented command
returned **0 paths** and would have made the whole crawl invisible. `-L` returns 67 460. The
instruction is corrected in place, with the reason, because the next round in a worktree meets it
too. That is a fifth distinct way this project has produced a false zero.

**"Read the clauses before the crates."** This is the one that changed the shape of the round, and
it is the round's lesson. `doc/todo/51` recorded BSI TR-03111's plain `r ‖ s` encoding — two of the
three corpus witnesses — as a **principle-5 blocker**, because TR-03111 is a document this tree does
not hold. It survived two rounds and two ADRs. ISO/TS 32002 section 5.1.3's NOTE 2 answers it
outright: "[t]his restriction implies that ECDSA signature values are required to be represented
using the DER-encoded ECDSA-Sig-Value type in IETF RFC 5753:2010, section 7.2." Those two files are
outside what the Technical Specification admits, and reporting them by their own algorithm
identifier **is** the correct behaviour. The blocker was never a missing document; it was a clause
nobody had gone back to read.

## The measurement

`signature_algorithm_census` over the same 67 460 documents (811 signature dictionaries in 681),
before and after, into two files and `diff`ed. **The report is about 750 lines and the diff is 15,
all of them the intended one.**

- `1 AlgorithmNotVerifiable 1.2.840.10045.4.3.2` → `1 Verified (256-bit ECDSA (P-256))`.
- The key census resolves `3 1.2.840.10045.2.1` into `2 … (id-ecPublicKey, P-256)` and
  `1 … (id-ecPublicKey, curve 1.3.36.3.3.2.8.1.1.7)` — which is ADR 0331's hand identification of
  the three EC signers, **re-derived by the program rather than by a person**.
- The "names an algorithm this program does not verify" list loses `6100006.pdf` and keeps the two
  BSI ones.

Everything else is byte-identical: 777 RSA verifications across four key widths, the same 17
`NotUnderThatKey`, the same digest counts, the same 186 indefinite-length values, the same 338
revocation attributes, the same 20 document timestamps.

`tests/signatures.rs::the_crawls_one_ecdsa_signature_verifies_under_its_own_p256_certificate` is
that one witness as a permanent test — with the bit-flip half, so it is an instrument rather than an
assertion — skipping and saying so where the machine-local crawl is absent.

## What it cost

23 new locked packages plus two patch bumps (`hybrid-array`, `keccak`). `cargo deny check` green on
advisories, bans, licenses and sources with **no exception added**: `BSD-3-Clause`, which
`ed25519-dalek` and `curve25519-dalek` are under and nothing else in this graph is, has been in the
allow list since it was written.

`default-features = false` on all four curve packages, with `precomputed-tables` deliberately off —
it trades image size for scalar-multiplication speed, and a signature is verified a handful of times
per document off the launch path while every byte of a table is paged in at launch. The
before-and-after size of the `pdf-viewer` binary under one profile is below.

**`const-oid`'s `db` feature came in at zero new packages** — it is already here through `digest` —
and is where every one of the family's dozen object identifiers comes from, grouped by the RFC that
assigns each. Nothing is transcribed. It also retired a claim of ADR 0390's that had stood for a
hundred and thirty-four sessions: `id-shake256` did have a second reading available, and nine of
`cms::Digest`'s ten identifiers now carry one.

**`curve25519-dalek`'s SIMD backend carries `unsafe`**, selected by its own build script on x86_64.
That is written down beside the dependency in `Cargo.toml` rather than left to be found. It is the
same shape as `sha2`, `cmov`, `block-buffer` and `hybrid-array`, all already on `pdf-model`'s path,
and `#![forbid(unsafe_code)]` is unchanged — it is compiler-enforced over *this project's* source.

## What the sweeps say, before and after

`doc/todo/02` §4's sweeps were run **twice** — once on this tree and once on the same tree with the
round reverted by patch (`git diff`, `git apply -R`, the two new modules and two new documents moved
aside; **not** `git stash`, which is shared between worktrees) — and every delta in the summary
lines is one this round caused:

- **`unread`** loses a hit: `signature.rs:400`'s "the third has no arm here: an ECDSA signature
  never reaches a verification to be named after" was an unwitnessed claim about the program and is
  now false, so the sentence went.
- **`capabilities`**: 150 → **151** of 182 capability sentences witnessed by the tree.
- **`owed`**: 182 debts-named-in-a-word over 114 rows → **177 over 112**, and the reading list of
  rows whose every stated term the tree already names goes 110 → **112**. §12.8 and §12.8.3 are the
  two that moved.
- **`overstated`**: 129 → 131 terms asserted by a parent, 59 → **60** corroborated by a child, and
  the 8 contradicted are unchanged.
- **`callers`**: 134 → 136 names no crate under `crates/` asks, and both are new public items in a
  new module named by its own crate, by the census example or by the fuzz target — which is what
  that sweep's own header says to check rather than a level to hold.
- **`pointers`**: **absent unchanged at 123** and undefined symbols unchanged at 13. The +2 in "not
  carried" is the crawl document this round now names in three places, and `corpus-cache` is a
  directory the repository does not carry.
- **`tables`**: 2250 attributed key citations, 2088 agreeing, and **0 under no such table** — all
  unchanged, so nothing this round wrote cites a table wrongly.
- `entries`, `blockers`, `retired`, `inapplicable`, `counts` and `overtaken` are unchanged but for
  `overtaken` counting one more decision record, which is ADR 0532.

`spec-errata emit` was run on ISO/TS 32002 **before** anything was written, which is the order
`doc/todo/02` §4 asks for.

## The gates, and the machine they ran on

`doc/todo/02` §2 whole, because the change is in `pdf-model`. Numbers are in the run rather than
here; what belongs here is the condition they were taken under.

**The machine was loaded and the load is recorded because §2 says a gate that spawns a reference is
a measurement of two programs and a loaded machine is a silent third.** At the start of the round
the one-minute load average was 50 on 24 cores; the gate sequence began at 16 and the reference
cache was the shared warm one (`PDFREF_CACHE`), which is what §2's warning asks for. The oracle
reported an 85.9% cache hit rate rather than the 99% trap 10a calls the tell — that is a fact about
which entries the shared cache happened to hold, not about this tree, since a reference render is
keyed on the document and the renderer's version and nothing in this round can reach either. Every
ratchet held.

**No gate that draws anything could move**, for the reason ADRs 0215, 0229, 0314, 0322 and 0331 all
give: no gate in this tree looks at a signature. They were all run anyway, because §2 says a change
in `pdf-model` runs everything and trap 1's whole subject is a change nobody expected to draw
differently. `cargo nextest run --workspace` is 2467 tests with 17 skipped; the oracle's 902 agrees
/ 60 contradicted / 42 not comparable and quorra's 933 agree / 22 differ of 957 are what they were.

**The fuzz targets that cover what this round touched were run and are clean at 1 000 000 apiece**:
`x509`, whose corpus was re-seeded with the four new certificates plus the brainpool one, and `cms`.
The `x509` target gained arms for the three new key kinds, and each asserts the same property the
RSA and DSA arms do — that no signature ever verifies against a digest the target chose, over every
signature shape the certificate's curve admits, including BSI TR-03111's plain `r ‖ s`.

**The one number a dependency of this size owes is image size**, since ADR 0532's whole reason for
turning `precomputed-tables` off is that a table is paged in at launch. Measured A/B in one sitting,
same profile, same machine, the reverted tree against this one: the `gates`-profile `pdf-viewer`
binary goes **26 992 736 → 27 288 480 bytes, +295 744 (+1.10%)** for four curve packages and their
nineteen transitive ones. Nothing on the launch path itself changed — the verification runs in
`notes::about`, on the document's own thread, only for a document that carries a signature — so no
launch timing was taken, and on a machine at this load one would not have been worth having.

## What a later round should know

- **`doc/todo/51` is now about four curves and question 3**, and each curve's row says what would
  change it. Take the curve the standard names rather than the crate that exists — `bp512` is the
  instance where those differ.
- **§7.6.5's public-key handler reverses ADR 0229's central argument and the reversal is sharper
  now than when it was written.** Every dependency this clause family runs on was chosen with the
  `_vartime` spelling on purpose, because a verifier has no secret. A *decryption* does, and none of
  those choices carries over.
- **Question 3 is the only thing between §12.8.3 and `implemented`** now, which it was not before:
  every `partial` in that family names either trust or one of the four curves.

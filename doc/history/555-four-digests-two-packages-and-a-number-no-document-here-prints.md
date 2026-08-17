# 555 — Four digests, two packages, and a number no document here prints

**Finding.** The round set out to compute ISO/TS 32001's four digests and found two things nobody
had looked for. **The clause it was implementing had been amended and the tree could not see it**:
the copy in `doc/` is `_EC3`, its errata are PDF *annotations*, `doc/md/` cannot show them, and
`spec-errata emit` says that issue #236 **deletes clause 5.1.3 entirely** — so Table 256's
`/DigestMethod` is not extended with the SHA-3 family at all, which this tree had been asserting
since ADR 0314 — while issue #404 strikes the sentence pinning `id-shake256` and defers to two RFCs
this tree does not hold, leaving the NOTE that fixes the output at 512 bits standing over a
requirement that is gone. And **the dependency `doc/stack.md` had already chosen was half an
answer**: `sha3` 0.12.0 removed `Shake128` and `Shake256` into a separate `shake` crate, so the
named package covers three of the standard's four and is silent about the fourth. Both were found
by three or four minutes of looking at the thing itself rather than at the note about it.

**Date.** 2026-08-17.
**ADR.** [0390](../adr/0390-four-digests-two-packages-and-a-number-no-document-here-prints.md).
**Touched.** `Cargo.toml` and `crates/pdf-model/Cargo.toml` (`sha3`, `shake`), `Cargo.lock`,
`crates/pdf-model/src/cms.rs` (`Digest`'s four variants, `SHAKE256_OCTETS`, `shake256`, `ALL` split
into `ALL` and `TRIED_WHEN_UNSTATED`, the `oid` and output-length decisions,
`fixtures::detached_stating` and the fixtures' digest parameter, four tests),
`crates/pdf-model/src/signature.rs` (the trial set, two tests, three claims),
`crates/pdf-model/src/pss.rs` (the SHA-3 refusal and the test that had to move),
`crates/pdf-model/src/pkcs1.rs` (the `DigestInfo` bound's arithmetic),
`crates/pdf-model/tests/signatures.rs`, `crates/pdf-model/tests/page_geometry.rs` (a pre-existing
lint), `doc/conformance/ledger.toml` (§12.7.5.5, §12.8, §12.8.1, §12.8.3, §12.8.3.2, §12.8.3.3),
`doc/errata-read.md`, `doc/stack.md`, `doc/todo/51-signatures-and-public-keys.md`,
`doc/todo/README.md`, `doc/adr/0390-*` (new), this file.

## The clause, read twice

**First from the text.** ISO/TS 32001:2022 is four pages and edits three tables, and the three are
not one requirement: §5.1.2 extends Table 237's seed value `/DigestMethod` (a constraint on a
signer, not this program), §5.1.3 extends Table 256's, and §5.1.4 extends Table 260's Message Digest
row — and §5.1.4 says *where*: "to the Message Digest value entry for adbe.pkcs7.detached,
ETSI.CAdES.detached or ETSI.RFC3161", three of five `/SubFilter` columns. That sentence is what
split `Digest::ALL` in two: §12.8.3.2's `adbe.x509.rsa_sha1` is the one sub-filter whose digest has
to be found by *trying each in turn*, and it is not one of the three, so `TRIED_WHEN_UNSTATED` stays
the base standard's six. Ten trials would have widened a table the standard did not.

**Then from the annotations**, which is the part that mattered. `spec-errata emit` on the one file:

- **#236, `Review/Accepted`** — a strikeout over the whole of §5.1.3, on the contents page and in
  the body, commented "Delete all of clause 5.1.3".
- **#404, `Review/Accepted`** — a strikeout over "specified, the message digest algorithm identified
  by the id-shake256 object identifier (OID) in section 2.3 of RFC 8419 shall be used." and a caret
  inserting "used, the applicable stipulations on algorithm identifiers in RFC 8702, 3.1 and RFC
  8419, 3.1, 3.2 shall be followed." **The NOTE was not struck.**

Neither changes what the code computes. Both change what may be said about why, and #404 turns
SHAKE256's 512 bits from a requirement into a choice — taken deliberately, as the reading the
retired sentence and the surviving NOTE agree on, with the cost bounded in the safe direction: any
*other* identifier is simply not in `from_oid` and is reported by its own digits rather than
computed at a guessed length.

`spec-errata check` caught three quotations of retired text in this round's own work before the
commit — which is the tool doing its job, and also the tool's limit: **it compares the tree's
quotations against struck passages, so an erratum over text nobody has written yet is invisible
until somebody writes it.** `doc/errata-read.md` now carries the rule that follows and the §5.1.3
row it never had.

## The dependency

`doc/stack.md` predicted one package, `sha3` 0.12, on the `digest` 0.11 line. The line was right —
and the line was the deciding test, because a second hash stack is what ADR 0229 declined `rsa` 0.9
for. The package was not: `sha3` 0.12.0's changelog reads "Removed — `Shake`, `Shake128`, and
`Shake256` types (moved to the `shake` crate)". Two options, both stable, both on `digest` 0.11:
`sha3` 0.12.0 + `shake` 0.1.0 at **four** new compiled packages (`sha3`, `shake`, and the shared
`keccak` 0.2.1 and `sponge-cursor` 0.1.0), or the superseded `sha3` 0.11.0 alone at two. Taken: the
current line, because a dead minor buys nothing a later upgrade does not pay back with the same
split. All four `MIT OR Apache-2.0`, MSRV 1.85 against the pinned 1.97.1, all RustCrypto — the
supplier of every cipher and digest already here.

## The population

`signature_algorithm_census` over 67 460 documents (SafeDocs' 66 211, the four corpora's 275,
pdf.js's 974): 681 documents, 811 signature dictionaries, four distinct digest identifiers —
SHA-256 568, SHA-1 197, SHA-512 22, MD5 6 — plus three signatures stating `1.2.840.113549.1.1.5`, a
*signature* algorithm, where a digest algorithm belongs. **Not one SHA-3.** Trap 8 governs: the
witnesses are built by hand.

## The witnesses

NIST's published example values for FIPS PUB 202 — the publication §5.1.1 names as where these are
defined — for the empty message and the 1600-bit message, 200 octets of `0xA3`. The second earns its
place twice: it is longer than all four rates, so it is the multi-block case, and splitting it into
three pieces pins that `compute`'s pieces hash as one message, which is what §12.8.1's byte range
needs. Beside them the 512-bit squeeze **as a length**, because a prefix of a SHAKE256 stream is a
valid SHAKE256 output of its own length and no vector can catch a wrong one; and one whole signed
file per algorithm, each checked both ways. All four reported `Integrity::UnknownDigest` before this
round.

The identifiers are transcribed from a registry no document here holds — a documented decision with
its cost priced on `Digest::oid` — and three of the four carry a second party's reading as
corroboration, against `sha3`'s own `AssociatedOid`. **That is evidence and not truth**, principle
5's direction of inference; `id-shake256` has no second reading, because `shake` publishes none.

## A finding the round did not go looking for

**Table 256's `/DigestMethod` is read by nothing.** "(Required)" in the base standard, and no source
under `crates/` names the string. It costs no mark — the digest it names belongs to §12.8.2.2.2's
comparison, which the ledger already records as not done — and §12.8.1's row now says so, with the
correction that its value list is the base standard's six and nothing more.

## Every gate

- `cargo fmt --all --check`: silent.
- `cargo clippy --workspace --all-targets`: silent. **Two lints had to be fixed to say that**, and
  one was not mine: `crates/pdf-model/tests/page_geometry.rs:486` has carried a `doc_markdown`
  warning since `c4467b45`. §2's own note about eleven warnings living on `main` for five rounds is
  why it is fixed here rather than reported.
- `cargo nextest run --workspace`: **2063 passed, 15 skipped** — six more than the base's 2057.
- `cargo test --workspace --doc`: 1 passed, 0 failed.
- `cargo test -p conformance`: 5 passed. 8460 citations, 807 quotations, 875 subclauses, **0
  unreviewed**.
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok — over the changed graph.
- `spec-errata check doc/*.pdf`: **0 quotations quote text struck out of the clause they cite**, and
  its cross-clause bucket read before the commit rather than left as noise. It is what found the
  three retired quotations this round wrote.
- `cargo +nightly fuzz run cms` and `x509`, each 1 000 000 runs, corpora re-seeded (11 signature
  values, 28 certificates): clean, 17 s and 44 s.
- **`signature_algorithm_census` over the same 67 460 documents, before and after: byte-identical**
  (`diff` on the two captured runs). Nothing in the population states one of the four, so every
  verdict, count and identifier is unchanged — which is also the check that splitting
  `Digest::ALL` moved no `adbe.x509.rsa_sha1` answer.
- **No raster reach, and this is a statement rather than an omission**: no gate in this tree looks
  at a signature, nothing outside §12.8's machinery was touched, and the corpus, oracle,
  text-extraction and quorra gates cannot see this change. ADRs 0215, 0229, 0314, 0322 and 0331 all
  say the same sentence about the same subject.

## The one gate that had an opinion about prose

The quotation checker refuses a rustdoc blockquote it cannot attribute, and a `§` after a foreign
document's name is a separate error — so **a blockquote is reserved for ISO 32000-2**, whose words
`doc/md/` can check. Three blockquotes of ISO/TS 32001 became inline quoted prose, which is what the
rest of the tree already does for ISO/TS 32002 and RFC 8017. The quotation marks still mean
verbatim; what is gone is the machine check, and `cms.rs` now says that in one parenthesis so the
next round does not rediscover it by failing a gate.

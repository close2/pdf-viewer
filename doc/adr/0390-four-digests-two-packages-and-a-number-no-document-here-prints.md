# ADR 0390 — Four digests, two packages, and a number no document here prints

Status: accepted, 2026-08-17 (session 555). Closes `doc/todo/51`'s item 2. Follows ADRs 0215,
0229, 0314, 0322 and 0331, and takes ADR 0331's dependency method as its own.

## Context

ISO/TS 32001:2022 is four pages of substance, all of it about hash algorithms, and this tree has
held it and its Markdown since the specifications came in. Its section 5.1.1 states the addition
outright:

> This document adds support for digitally signing PDF documents using the SHA3-256, SHA3-384,
> SHA3-512 and SHAKE256 hash algorithms in the secure hash algorithm 3 (SHA-3) hash algorithm
> family as defined in FIPS PUB 202.

It then edits three of the base standard's tables, and **the three are not the same requirement**,
which is the reading that decided the shape of the code:

- **Section 5.1.2, Table 237** — the signature field *seed value* dictionary's `/DigestMethod`,
  "[a]n array of names indicating acceptable digest algorithms to use while signing". A constraint
  on a processor that signs. Not this one.
- **Section 5.1.3, Table 256** — the signature *reference* dictionary's `/DigestMethod`, extended
  with the same four. **Struck out of the document entirely by Errata Collection 3; see below.**
- **Section 5.1.4, Table 260** — the Message Digest row, and it says *where*: "the following values
  are added to the Message Digest value entry for adbe.pkcs7.detached, ETSI.CAdES.detached or
  ETSI.RFC3161". Three of Table 260's five `/SubFilter` columns. The four values are "SHA3-256 (PDF
  2.x)", "SHA3-384 (PDF 2.x)", "SHA3-512 (PDF 2.x)" and a SHAKE256 entry that pinned an identifier,
  under a NOTE saying the pinning "fixes the SHAKE256 output length for the digest at 512 bits and
  serves to prohibit variable length SHAKE256 algorithm usage".

`cms::Digest` computed the base standard's six and none of these four; a signature stating one was
reported by its dotted-decimal identifier, correctly and uselessly.

### The errata, which changed two of those three sentences

**The copy in `doc/` is `_EC3`, and its errata are PDF annotations rather than text** — the cover
page says so and `doc/md/` cannot show them, so `pdftotext` and the conversion both give the *2022*
document. `tools/spec-errata emit` gives the amendments, and there are two, both `Review/Accepted`:

- **Issue #236 deletes section 5.1.3.** The strikeout covers the whole subclause, on the contents
  page and in the body, and its comment is "Delete all of clause 5.1.3". So **Table 256's
  `/DigestMethod` is not extended at all** and the four land in Table 237 and Table 260 only. Every
  sentence in this tree that said otherwise — `cms::Digest`'s own documentation, three ledger notes,
  `doc/todo/51`, and, for about an hour, three passages of this round's own work — was quoting text
  ISO struck.
- **Issue #404 unpins `id-shake256`.** The strikeout covers "specified, the message digest algorithm
  identified by the id-shake256 object identifier (OID) in section 2.3 of RFC 8419 shall be used."
  and the caret beside it inserts "used, the applicable stipulations on algorithm identifiers in RFC
  8702, 3.1 and RFC 8419, 3.1, 3.2 shall be followed." **The NOTE was not struck.** So the amended
  document defers the identifier question to two RFCs this tree does not hold, while still carrying
  an informative NOTE that describes, in the present tense, a requirement it no longer states.

Neither erratum changes what the code must *compute*. Both change what may be said about why, and
one of them turns a requirement into a choice.

**The population says nobody writes one.** `signature_algorithm_census` over 67 460 documents
(SafeDocs, the four corpora, pdf.js) finds 811 signature dictionaries in 681 documents and four
distinct digest identifiers: SHA-256 (568), SHA-1 (197), SHA-512 (22) and MD5 (6), plus three
signatures stating `1.2.840.113549.1.1.5` — a *signature* algorithm — where a digest algorithm
belongs. Not one SHA-3. So this is spec-driven work with hand-built witnesses, and trap 8 governs
the tests: a corpus that does not exercise a requirement cannot confirm a reading of it.

## Decision 1 — `sha3` 0.12.0 **and** `shake` 0.1.0, four new packages

`doc/stack.md` predicted one package and the measurement found two, which is the reason to measure.

| | packages it adds here | line | verdict |
|---|---|---|---|
| `sha3` 0.12.0 + `shake` 0.1.0 | **4**: `sha3`, `shake`, and the shared `keccak` 0.2.1 and `sponge-cursor` 0.1.0 | `digest` 0.11 | **taken** |
| `sha3` 0.11.0 alone | 2: itself and `keccak` | `digest` 0.11 | declined |

**`sha3` 0.12.0 has no SHAKE at all.** Its changelog: "Removed — `Shake`, `Shake128`, and `Shake256`
types (moved to the `shake` crate)". So the current line covers three of the standard's four in one
package and the fourth in another, and the two-package option is the *superseded* 0.11.0, which
still has both. Pinning to a dead minor to save two packages would buy nothing that a later upgrade
does not have to pay back with the same split; the split is upstream's and following it now is
cheaper than following it later.

What the line test asks, and both options pass it: `digest` 0.11, the same trait stack as the
`sha2`, `sha1`, `md-5` and `ripemd` already here. **A second hash stack is the shape ADR 0229
declined `rsa` 0.9 for and ADR 0348 declined `ttf-parser` for**, and it is the one thing that would
have disqualified a package outright. Measured with `cargo tree -e normal`, ADR 0331's method.

The rest of the arithmetic:

- **Licences**: `sha3` and `shake` and `sponge-cursor` are `MIT OR Apache-2.0`, `keccak` is
  `Apache-2.0 OR MIT` — read out of each package's own manifest, all inside `deny.toml`'s allow
  list, and `cargo deny check` run over the changed graph.
- **MSRV** 1.85 for all four, against the toolchain's pinned 1.97.1.
- **Supplier**: RustCrypto, which is where every cipher and digest in this tree already comes from
  (ADR 0031's precedent, ADR 0331's most recent application).
- **Features**: `sha3`'s defaults, which are `alloc` and `oid`. `oid` costs no package —
  `const-oid` 0.10.2 is already in this graph, reached through `digest` — and buys the one
  corroboration available for the identifiers, below. `shake` has no such feature.

## Decision 2 — the four are computed, and *where* they apply is part of the code

`Digest` gains `Sha3_256`, `Sha3_384`, `Sha3_512` and `Shake256`. Three of them are one more arm of
the existing macro. SHAKE256 is not, and the difference is the specification rather than the API:
an extendable-output function has no length of its own, and `SHAKE256_OCTETS` is the 512 bits
Decision 3a settles. `shake256()` squeezes exactly that many and there is no output length read
from any file.

**`Digest::ALL` became two constants**, and this is the part a reader should not skim. It was six
entries with two jobs: *everything the tables name*, and *everything to try where a file states
nothing*. Section 5.1.4 separates them, because it adds its four to three `/SubFilter` columns and
§12.8.3.2's `adbe.x509.rsa_sha1` — the one sub-filter whose digest has to be found by trying each
in turn, since the identifier is inside the PKCS #1 block under the key — is not one of the three.
So `ALL` is now the ten and `TRIED_WHEN_UNSTATED` is the six, and the trial loop takes the second.
Ten trials would have been this program widening a table the standard did not.

Two more places the compiler forced a decision, both answered from a document rather than by
pattern:

- **`pss::pss_digest` refuses all four**, as `HashNotAdmitted` rather than `HashNotComputed`. The
  hash a PSS signature uses comes from RFC 8017 Appendix A.2.1's `OAEP-PSSDigestAlgorithms`, which
  names no SHA-3; ISO/TS 32001 extends Table 260's Message Digest entry, which is a different
  statement in a different place. A test that used SHA3-256 as its example of *not computed* had to
  move to SHA-224, which is in the RFC's set and outside `Digest` — a better example than the one
  it replaced, and a small demonstration that a test's expected value decays with its premise.
- **`pkcs1::encode`'s 83-octet bound is unchanged**: SHA3-512 and SHAKE256 are 64-octet digests
  under nine-octet identifiers, which is SHA-512's shape exactly.

Everything a refusal did before, it still does. An identifier outside all ten falls out of
`from_oid` as `None` and is reported by `x509::dotted` as digits — including
`2.16.840.1.101.3.4.2.11`, the SHAKE128 slot immediately beside one this program now computes,
which has a test of its own for that reason.

## Decision 3a — SHAKE256 stays at 512 bits, as a choice and not a requirement

After issue #404 the only statement about an output length in any document under `doc/` is a NOTE
whose subject the same errata collection deleted, and the normative sentence in its place points at
RFC 8702 section 3.1 and RFC 8419 sections 3.1 and 3.2, neither of which this tree holds. Three
options were on the table:

1. **Refuse SHAKE256** on the ground that no document here states its length. Rejected: it would
   leave a value Table 260 explicitly permits uncomputable, and the four-page document exists to
   permit it.
2. **Read a length from somewhere** — the signature, a parameter, a default of the caller's.
   Rejected outright: nothing in a CMS `digestAlgorithm` carries one for `id-shake256`, and
   inventing a place to read it from would be this program specifying rather than reading.
3. **Keep 512 bits under `id-shake256` and say why.** Taken. It is the reading the retired sentence
   and the surviving NOTE agree on, it is the narrow choice rather than the permissive one, and the
   cost of being wrong is bounded in the safe direction: any *other* identifier — including whatever
   variable-length ones RFC 8702 may define — is simply not in `from_oid`, so it is reported by its
   own dotted decimal instead of being computed at a guessed length. A reader who later holds those
   RFCs widens the table; nobody has to un-guess anything first.

`Digest::Shake256`'s doc comment carries this, with both the struck sentence and the caret's
replacement quoted, so that the next reader meets the amendment where the decision is.

## Decision 3 — the identifiers are transcribed, and the cost is written down

**Neither ISO 32000-2 nor ISO/TS 32001 prints a single digit of an object identifier for any of
these algorithms**, and the one the published text was most specific about it named by symbol and
deferred — "the id-shake256 object identifier (OID) in section 2.3 of RFC 8419" — to a document this
tree does not hold, in a sentence issue #404 has since struck. So the four constants below the
`oid()` match arm are transcribed from a registry nobody here can open — exactly as the six that
preceded them were, and as `dsa.rs`'s `id-dsa-with-sha2` arc was.

This is a shortcut, so principle 1 says it is recorded with its cost rather than taken quietly.
What bounds the cost:

- **A transcription that is simply wrong costs a report and never a verdict.** An unmatched
  identifier is `None`, reported by its own digits — the behaviour a file stating that digest
  already had.
- **A transcription wrong by *swapping two* would be a wrong answer**, and that is what the tests
  are aimed at. `x509::dotted` reads each constant back as digits, and each digest round-trips
  through `from_oid`/`oid`; then, for the three SHA-3 ones, the constant is compared against
  `sha3`'s own `AssociatedOid`. **That comparison is evidence and not truth** — principle 5's
  direction of inference, the same one the oracle runs on: a second party's reading of the same
  registry agreeing with ours raises confidence that the registry was read correctly, and would
  never have been allowed to *define* the number.
- **SHAKE256 has no second reading.** `shake` publishes no identifier, so `id-shake256` stands on
  the transcription alone, and the doc comment says so where a reader will meet it.

The alternative considered and rejected: implement `compute` and refuse the identifiers, on the
ground that no document here assigns them. That would have left the standard's own `shall`
unimplementable — a file may state SHAKE256 and there is exactly one way to say so — while adding a
hash function nothing could reach. A refusal that closes nothing is worse than a transcription that
says it is one.

## Decision 4 — the witnesses are the algorithms' publisher's, and the vectors are byte-exact

Trap 8: no corpus document states one of the four, so nothing in the population can confirm the
reading. The tests are therefore:

- **NIST's own example values for FIPS PUB 202** — the publication ISO/TS 32001 section 5.1.1 names
  as where these algorithms are defined — for two messages each: the empty one, and its 1600-bit
  message, 200 octets of `0xA3`. The second earns its place twice over: it is longer than all four
  rates, so it is the multi-block absorption case, and splitting it into three pieces pins that
  `compute`'s pieces are hashed as one message, which is what §12.8.1's byte range needs.
- **The 512-bit pin as a length**, because a prefix of a SHAKE256 stream is a valid SHAKE256 output
  of its own length — squeeze 32 octets and every byte still agrees with the 64-octet vector, so no
  vector can catch a wrong output length and a separate assertion has to.
- **An end-to-end file per algorithm**: `signed_document` with a `detached_stating` fixture, and
  each is checked both ways — the digest recomputes to what the signature recorded, and one byte
  moved inside the signed range turns `Unchanged` into `Changed`. Each of these four reported
  `Integrity::UnknownDigest` before this round, which is what makes the test worth its lines.

## A finding the round did not go looking for

**Table 256's `/DigestMethod` is read by nothing.** The entry ISO/TS 32001 section 5.1.3 extends is
"(Required)" in the base standard — "[a] name identifying the algorithm that shall be used when
computing the digest if not specified in the certificate" — and no source under `crates/` names the
string. It costs no mark today, and the reason is worth stating rather than assuming: the digest it
names is the one a transform method's comparison would be computed with, and that comparison is
§12.8.2.2.2's, which the ledger already records as not done. It is now written in §12.8.1's row so
that whoever does the comparison finds the entry named — and what is missing there is a reader for
a *name*, not a function, because all ten names now have one behind them.

## Consequences

- `Cargo.toml` gains `sha3` and `shake` with the reasoning beside the settings; `deny.toml` needed
  no change and `cargo deny check` passes over the changed graph.
- `doc/stack.md`'s owed-dependency bullet is spent, and is replaced by what the spending measured —
  including the correction that it is two packages and not one.
- Ledger: §12.8, §12.8.1, §12.8.3, §12.8.3.2, §12.8.3.3 and §12.7.5.5 amended; every one of them
  said "six" or "not computed" about something that is now ten and computed, and three of them
  attributed the four to a subclause the errata deleted.
- `doc/errata-read.md`: §5.1.4's row moves from `untouched` to `implements` — a verdict about the
  *tree* expires when the tree implements the clause — and §5.1.3 gets a row it never had, because
  `check` can only name an erratum over text somebody has already written.
- **No gate that draws anything moves**, for the same reason as ADRs 0215, 0229, 0314, 0322 and
  0331: no gate in this tree looks at a signature, and nothing outside §12.8's machinery was
  touched. There is no raster reach at all.
- Nothing on the launch path changed. A digest is computed in `notes::about`, on the document's own
  thread, only for a document carrying a signature.
- `doc/todo/51` loses its item 2 and keeps its other two; the elliptic-curve refusal is untouched
  and so is question 3.

## The lesson

**Two of them, and they are the same shape twice.**

**A clause is not read until its errata are read, and the instrument for that runs *before* the
code rather than after it.** This round quoted a deleted subclause in three places and unpinned an
identifier it had described as pinned, and both were caught — but by `spec-errata check`, which can
only see an erratum over text the tree has already written. Everything the tree had said about ISO/TS
32001 since ADR 0314 was written from `doc/md/`, which cannot show an annotation, and was therefore
partly about a document ISO had amended. `doc/errata-read.md` is amended with the rule that follows:
**a round implementing a clause runs `spec-errata emit` on that document before it writes.** Four
pages took four seconds to read that way.

The second is subtler and is the same failure aimed at a dependency. **A dependency question can be
answered in a document and still be wrong by the time it is spent.** `doc/stack.md` carried a correct, careful bullet — the line is `digest` 0.11, the package
is `sha3` 0.12 — written by a round that had read the registry and not the crate. Between that
sentence and this one, upstream moved SHAKE out of `sha3`, so the *package* named in the answer
covers three of the four requirements and is silent about the one the standard is most specific
about. The bullet was not stale in any way a sweep could see; it was a prediction, and a prediction
about somebody else's release is a thing to re-measure at the moment of spending rather than to
read. The three minutes that found this were `grep Shake` over the downloaded source, before a line
of code was written.

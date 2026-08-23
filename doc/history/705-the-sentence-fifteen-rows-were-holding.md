# 705 — The sentence fifteen rows were holding, and the caret that made a reading a requirement

§12.8.3's `partial` and `reported` rows read as a family, on ADR 0538's method for the fourth block
running. The family was picked by a *measurement* of ADR 0560's criterion rather than by eye — rank
the families by how much their `partial` notes restate each other — and the thing the ranking pointed
at turned out to be larger than it could show: a paragraph of 92 words standing byte-for-byte
identical in fifteen rows, ending in two counts that had been stale for four rounds.

Date: 2026-08-24.
ADR: [0567](../adr/0567-the-sentence-fifteen-rows-were-holding-and-the-caret-that-named-a-field.md).

Touched: `doc/conformance/ledger.toml` (§12.8.2.4 and all fifteen rows of §12.8.3's subtree),
`crates/pdf-model/src/signature.rs` (two doc comments and one test),
`doc/errata-read.md`, `doc/todo/01-ledger-partial-rows.md`, the ADR and this file.
No status moves, no pixel moves, no report added or removed.

## Why §12.8.3

The blame ordering was re-derived on this base rather than read out of any document (616's rule):
`git blame --line-porcelain doc/conformance/ledger.toml`, each `partial` or `reported` row's own
`note =` line, ranked by where its commit falls in `git log --reverse`. 951 commits, 875 rows, 222
`partial`, 240 `partial`-or-`reported` with a blamed note. §7.6.4.4 is still rank 1, §11.3.4 is 2,
§11.3.7, §12.5, §8.6.6, §8.9.6, §8.9.6.2, §9.8.3 and §9.8.3.1 share 3–9, §7.7 is 10.

**§7.6.4.4 was read first, as the briefing asked, and left alone.** Its arithmetic holds in every
part against the twelve rows below it — Algorithms 3 to 13 plus §7.6.4.4.1's framing; six of the
eleven compute what a writer stores; three of those six are `writer-side` because a reader re-runs
the other three. A row left uncorrected beside four its round rewrote is worth the reading; this one
was right.

The family was then chosen by running ADR 0560's criterion as a search: for every parent whose
subtree holds two or more `partial` rows, count the rare five-word sequences the notes share
pairwise. §12.8 heads it, §12.8.3's subtree is the reason, and `doc/todo/01` now carries the recipe.

## The four findings

- **One sentence, fifteen rows, two stale counts.** "The disjointness this row used to record —
  eight signed documents, twenty-six encrypted, no overlap — was therefore false; that document is
  both" stands identically in every row of §12.8.3's subtree. Measured over `doc/pdf.js`:
  `signature_algorithm_census` says **nine** documents state a signature dictionary, ten between
  them; `encryption_census` says **twenty-five** state an `/Encrypt` — which §7.6's row has said
  since the six-hundred-and-ninety-first session, four rounds ago, while this family went on saying
  twenty-six in fifteen places. ADR 0101's shape at fourteen times the usual scale. §12.8.3 keeps
  the account with both commands; the fourteen rows below it keep the part that is about them and
  defer the counting.
- **§12.8.3.4 and §12.8.3.4.5 answered the signature-value question for RSA and DSA**, four rounds
  after ADR 0532 added ECDSA and EdDSA and while four sibling rows recorded all four.
  `Signature::authenticity` matches the `signatureAlgorithm` against the certificate's key and never
  against `/SubFilter`, so the curves reach a `PAdES` signature like any other detached CMS one —
  and ISO/TS 32002 puts them here by name: "Table 3 defines ECDSA elliptic curves and associated
  message digest algorithms supported for the adbe.pkcs7.detached, ETSI.CAdES.detached or
  ETSI.RFC3161 SubFilter values in ISO 32000-2:2020, Table 260."
- **`Signature::authenticity`'s own doc comment listed three of its five constructions**, twelve
  lines under a module comment that says "for all four" and names both missing modules. The same
  staleness as the two rows, in the code the rows cite.
- **§12.8.3.1 said six of ISO/TS 32002's eight curves are computed and then named four refusals in
  its next clause.** Four are computed, which is ADR 0532's arithmetic, `signature.rs`'s module
  comment's, and what §12.8.3, §12.8.3.3 and §12.8.3.3.1 all say. The six counted the two Brainpool
  curves whose package exists as a release candidate.

## The test, calibrated

`signature.rs::a_pades_signature_verifies_under_an_elliptic_curve_key`: a P-256 signature under
`/SubFilter /ETSI.CAdES.detached` verifies, and stops verifying when the signed bytes move. Nothing
in the tree asserted it — `a_pss_signature_verifies_through_the_whole_path_a_document_takes` is the
only other test carrying that `/SubFilter` through `authenticity`, and the three curve tests all use
`adbe.pkcs7.detached`. Calibrated per trap 13 by making `authenticity` refuse an
`ETSI.CAdES.detached` `/SubFilter` outright: the new test fails there with its own message while
`an_ecdsa_signature_verifies_through_the_whole_path_a_document_takes` passes. Plant removed.

## Three claims that looked wrong and were not

Recorded so the next round does not spend the hour again, and one of them is the reason to measure
before writing.

- **§12.8.3.4's "all ten signature dictionaries in the 974" against `pades_departures`'s "all six
  the 974 carry in a signature field".** Two populations, both exact, measured with a throwaway
  program (ADR 0481's method, not committed): `signature::signatures` walks the AcroForm and finds
  six in six documents; `signature_algorithm_census` adds §12.8.6's permissions route, where three
  more documents keep their only signature dictionary. §12.8.3.4's row now says so, because two
  figures a screen apart with nothing between them are a defect waiting to be "found".
- **§12.8.3.2's "Table 260 permits five — so each of the six is tried"**: five in that column, and
  MD5 is the sixth, from Table 256's `/DigestMethod`.
- **§12.8.3.4.3's "three of the eleven are checked"**: (a) to (k) is eleven, (a), (d) and (e) are
  checked whole, and half of (i) beside them, which the row's next sentence says.

## The erratum, and it is §12.8.2.4's

`emit` files five annotations inside §12.8.3; three ask nothing of a reader (Issue #4's footnote
marker, an editor note about Table 260's italics, Issue #649's placeholder NOTE 1). **The other two
belong one subclause back**, which is 0551's finding about the instrument met again: §12.8.3's
heading is at the foot of page 592 and the top of that page is Table 259, §12.8.2.4's.

**Issue #33 inserts "fully qualified" and " (see 12.7.4.2 \"Field names\")" into Table 259's
`/Fields` row.** Both are a Caret with nothing struck out, so `check` is blind to them. The rects
are `[249.018 594.369 256.952 600.833]` and `[298.23 594.369 306.163 600.833]` — 241.1 to 247.6 from
the top of an 841.92-tall page — and `pdftotext -bbox` puts `containing field names.` at 235.4–247.1
with `field` starting at x 253.0 and `names.` ending at x 304.1: one caret before `field`, one after
`names`. §12.8.2.4's row already said the fully-qualified reading in `FieldSelection::covers` "has a
producer's file behind it and not only an argument"; it has the clause's now. **Only for one of the
function's two callers** — §12.7.5.5's Table 236 is worded identically and gains no such insertion,
over the eighteen annotations `emit` files under that subclause — so the comparison is *required*
for the `FieldMDP` transform and remains the argued reading for the signature field lock, and
`covers`'s comment says which is which.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`.
The full sequence was run rather than the documents-only column: the round adds a test and two doc
comments under `pdf-model`, and `tools/round.sh` says this is a fifth round. The machine carried
three other rounds and stood at a load average of 38 when the corpus gate ran, so the lines that
spawn a reference renderer were held until it fell below 12 — §2's rule that such a gate measures
two programs and a loaded machine is a silent third.

`fmt`, `clippy -D warnings`, `nextest`, the doctests, the fuzz `check`, the sandbox worker, corpus,
`pdfref-hayro`, oracle, text extraction, selection, accessibility, dates, XMP, JPEG 2000, quorra,
`fixed_documents` and `cargo test -p conformance` all green, the last of them after the final edit.
The `viewer-qt` `-Wmaybe-uninitialized` lines are gcc's on a cold build and are documented as such.
§5's binaries rebuilt and installed.

Thirteen sweeps run before the edits, after them, and a third time on the committed tree carrying
the ADR, this file and `doc/todo/01`'s new section, which are `SOURCE_ROOTS` too. Both errata
instruments print no hit at any line this round wrote, and the ISO/TS 32002 sentence quoted in
§12.8.3.4's note is verbatim against `doc/md/` — checked directly as well as through the eleventh
sweep, because a quotation this project invented is the one failure principle 5 forbids outright.

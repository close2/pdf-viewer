# 508 — The prefix a damaged stream keeps, and the sentence it never said

**Finding.** `doc/todo/03` §8 asked whether drawing the decoded prefix of a stream that then
fails is a recovery a reader may perform. **The question rested on a false premise about this
tree, which already kept the prefix** — `FilterRefusal::Corrupt`'s own doc comment said so, and
`stream_length_bound.rs` has asserted it since ADR 0306. So the decision was never whether to
recover but whether a recovery that says nothing is one the principles permit, and it is not.
§7.4.1's sentence has two halves — a reader "shall invoke the corresponding decoding filter or
filters to convert the information back to its original form" — and a damaged stream is a decode
that did the first and could not finish the second. Both are stated now.

Two defects were under the question and neither was the one §8 predicted. **The recovery was
silent**, so a page cut short was indistinguishable from a page meant to be sparse. **And it was
unreliable**: `read_to_end` discards whatever the erroring `read` call produced, so the prefix
survived only to the last whole call — which is why `498264.pdf` reported `Undecodable` while the
module promised a prefix, and why an ICC profile beside it kept exactly 1024 bytes. `flate` is
driven through `flate2::Decompress` now, which also tells RFC 1951's final block from an input
that merely ran out; **a truncated stream had been indistinguishable from a whole one since the
first parser**.

**And the corpus overturned the rule's own scope within the round.** With the prefix reliably
kept, `issue13316_reduced.pdf`'s corrupt `/FontFile2` went from too-short-to-parse to **863 bytes
that parse**, and the page drew **`A C E F`** where `pdftoppm` draws its six CJK glyphs — trap 1
exactly, a command count that rose while the picture got worse. §7.8.2 makes a content stream "a
sequence of instructions", so a prefix of one is a shorter sequence of the same kind; a font
program is a table directory whose offsets point forward, so a prefix is a directory describing
bytes that are not there. Trap 5's substitutive test (ADR 0106) settles it and `pdf_font::program`
refuses one.

**Date.** 2026-08-14.
**ADR.** [0343](../adr/0343-the-prefix-a-damaged-stream-keeps.md).
**Touched.** `crates/pdf-syntax/src/filter.rs` (`Damage`, `Decoded`, `inflate` and `finish`
replacing the `Read` adapter, `lzw` and `run_length` naming their end-of-data, the three reported
entry points), `crates/pdf-syntax/src/lib.rs` (two re-exports),
`crates/pdf-syntax/src/document.rs` (`decoded_stream_data_reported` carries the damage, the
decoded-stream cache memoises it, the chain keeps the first),
`crates/pdf-syntax/tests/stream_length_bound.rs` (three tests assert the new statement),
`crates/pdf-font/src/program.rs` (a damaged program is refused, both routes),
`crates/pdf-model/src/page.rs` (`ContentIssue::Damaged`),
`crates/pdf-model/tests/contents_entry.rs` (the `RunLengthDecode` witness),
`crates/pdf-model/tests/silent_fonts.rs` (the witness's report moved; the mechanism it used to
witness keeps a test of its own), `crates/pdf-model/examples/damaged_stream_census.rs` (new),
`doc/conformance/ledger.toml` (§7.4, §7.4.1, §7.4.4.1, §7.4.4.2, §7.4.5, §9.9),
`doc/HANDOVER.md` (trap 5's list was one behind before this round and is now two longer),
`doc/todo/03-more-corpora.md` (§8 settled, §9 records the chunk),
`doc/adr/0343-*` (new), this file.

## The chunk: damaged streams, over every corpus on this disk

`examples/damaged_stream_census`, one process per archive — the surveys' method, for their
reason. A baseline for this population, never a ratchet.

| population | documents | opened | `/Contents` damaged | of those, draw | `/Contents` undecodable | damaged streams | documents holding one |
|---|---|---|---|---|---|---|---|
| SafeDocs crawl | 65 944 | 65 703 | **90** (69 truncated, 21 corrupt) | **85** | 24 | **2260** of 6 085 918 | **726** |
| pdf.js | 974 | 964 | 0 | — | 2 | 57 of 17 057 | 20 |
| `format-corpus` (3 dirs) | 167 | 165 | 2 | 1 | 0 | 296 of 7 284 | 29 |

Three readings:

- **The rule §8 asked about had been buying real marks all along.** 85 of 90 crawled documents
  put at least one drawing command on the page from a recovered prefix — 0.14% of the web,
  drawing in silence. 9 632 556 bytes are kept across those parts.
- **The wider silence is the number to take away**: 2260 damaged streams against 90 damaged
  `/Contents`, so the route this round made loud is about 4% of the population. The rest reach a
  font program, an ICC profile, an image or a function through `Document::decoded_stream_data`,
  which drops the damage by design and is what keeps this change off sixty-one call sites.
  `pdf_font::program` is the one other route closed, and a corpus document forced it.
- **`govdocs1-error-pdfs` is where the witnesses are readable.** `507676.pdf` keeps 67 923 bytes
  and draws **33 854 commands** from a corrupt content stream — the document session 505's ink
  ranking put at −1.719 without anyone asking why.

## `498264.pdf`, which is the witness §8 named and not the interesting one

Its 18 recovered bytes are `q\n30 31.16 552 729` and yield **0 drawing commands**. `poppler`'s
three lines are *past* the invalid distance code, so recovering them means resynchronising a
broken deflate stream — a guess about bits nobody wrote. The clause bought a sentence here and
not a mark, and that is written down as the outcome rather than dressed up.

## What the change cost, measured rather than argued

**Nothing drawn moved in the gate corpus, and the proof is the artefact rather than a summary
number.** `examples/display_list_digest` over all 974 pdf.js documents is **byte-identical**
before and after — which is the interesting statement, because the two halves of the change
cancel there exactly: `issue13316_reduced.pdf` would have started drawing four commands of wrong
glyphs, and the font-program refusal takes them back off. The corpus gate's incomplete count is
**62 before and 62 after**, measured both ways.

Three pdf.js documents changed what they *say*, none what they draw: `issue13316_reduced.pdf`,
`issue11651.pdf` and `bug1050040.pdf` refused their font programs before and refuse them now,
with the filter's own reason in place of `sfnt::truncation`'s structural guess. Two of them
recovered more bytes on the way to the same refusal — 512 → 847 and 45 240 → 59 211 — which is
the reliability half of the fix visible on its own.

`doc/todo/00` step 7 needs no re-run and the digest is why: the ink ranking reads the oracle's
artefacts, and its input did not move.

## Gates

`fmt` clean. `clippy --workspace --all-targets` silent. `nextest --workspace` **1836 tests run:
1836 passed, 14 skipped** — 1834 at the base, plus the `RunLengthDecode` witness in
`contents_entry.rs` and the one that keeps `no outline for any` witnessed after its old document
moved to a different report. Doctests pass. Corpus **974 documents in 9.3s: 0 unopenable, 8 locked, 2 encrypted
beyond us, 6 pageless, 62 incomplete, 0 slow**. Oracle **1794 pages, 905 agrees, 68 contradicted,
786 ambiguous**. `text_extraction`, `dates`, `xmp`, `jpeg2000`, `render-quorra` corpus and
`conformance` all green.

A `clamp` in the new `inflate` panicked when `max_stream_len` was set below the buffer floor, and
`flate_past_the_bound_is_refused_rather_than_clamped` caught it — a test written for ADR 0306
finding a defect in ADR 0343's replacement of the code it was written against.

# 983 — The tail a salvage drops, and the default a cross-reference stream never had

Date: 2026-09-11. Branch `round-945/the-fifth-round`, from `9fac224b`. Topic: the copy of the
standard this project checks itself against (`doc/todo/48` steps 4 and 5) and the lexer's one silent
salvage (`doc/todo/53` item 1), with the scanner rule session 977 left. Four sibling rounds were
live in the same working tree throughout — 979 (transparency), 980 (colour), 981 (fonts), 982
(`pdf-archive`).

Argued in **ADR 1004**.

## What was done

- **The errata remainder, re-derived.** `doc/todo/48`'s "nineteen" was step 3b's count, spent in
  session 594. The recipe's own greps over a fresh `emit` give 302 issues carrying a strike or a
  caret and **44 named nowhere** (310 / 46 under the multi-issue parse); 30 touch only a settled row,
  7 a live one, 7 no row, 4 sit wholly inside the clause-13 exclusion. All 44 read to a verdict in
  `doc/errata-read.md`'s new section: 43 confirm, one implements.
- **The sixth clause implemented against struck text**: §7.5.8.3, Table 18's type 1 offset
  "Default value: 0" (Issue #500), executed by `xref::entry_location`'s `[1, 0, 0]` start for four
  hundred and eighty sessions. A type 1 entry with no offset field, and a type 2 entry with no
  stream-number field, are refused with their section now; the type 2 *index* stays zero as a
  documented choice, because the first form of the fix refused it too and two siblings' gates found
  the cost within the hour (297 XMP packets of 318, `issue3371.pdf` without a first page, 174 outline
  documents of 176) — attributed to the lexer change beside it, settled by bisection.
- **`5f` reports.** `Lexer::salvaged` records what a salvage dropped; `content::reader::
  next_content_token` hands the interpreter the whole run as a keyword where the tail names one of
  §8.2 Table 50's operators, and the existing dispatch reports it. `12pt` is still 12. The line is
  §7.8.2's — an action lost against a spelling dropped — and neither side has a corpus witness
  outside damaged streams: one `pdf.js` document gains three reports and no ink moves.
- **The scanner rule**: `ISO 32000-1:2008 §…` is a foreign citation; `ISO 32000-2:2020 §…` is ours.
  Three misquotations of `CLAUDE.md` in files this round owns are gone.

## Files

`crates/pdf-syntax/src/lexer.rs`, `crates/pdf-syntax/src/xref.rs`,
`crates/pdf-syntax/tests/cross_references.rs`, `crates/pdf-model/src/content/reader.rs`,
`crates/pdf-model/tests/numbers_without_digits.rs`, `tools/conformance/src/citation.rs`,
`tools/conformance/src/unread.rs`, `doc/conformance/ledger.toml` (§7.2.3, §7.3.3, §7.8.2,
§7.5.8.3, §14.3.2, §12.7.5.2.4, §12.5.6.24, §12.8.7, Annex Q.3), `doc/errata-read.md`,
`doc/todo/48`, `doc/todo/53`, `doc/todo/01`, `doc/adr/1004`, this file.

## Gates

This round was terminated by the session limit before it could print its own; **Figures below are off the merged run of the nine-hundred-and-eighty-fifth session**, which ran the whole of `doc/todo/02` §2 over this round's work beside its four siblings' after the session limit terminated this round mid-sequence:

- core: `cargo fmt --all --check` clean, `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` exit 0, `cargo nextest run --workspace` 4370 run / 4370 passed / 35 skipped after the two repairs the merge made, `cargo test --workspace --doc` exit 0, both `fuzz/` lines exit 0, `cargo test -p conformance` exit 0; oracle exit 0 (990 agree / 62 contradicted held by name / 835 ambiguous, `issue14438.pdf p1` diagnosed).
- `corpus`: exit 0 — incomplete: issue13916.pdf: [Text { operations: 3 }, Font { detail: "font /C2_0 cannot be substituted: the file states /Encoding /Identity-H over a de
- `text_extraction`: exit 0 — 292  no words in the reference
- `selection`: exit 0 — the find (Command::Find → Query::Selection): 1002/1002 words selected (100.00%) over 451 documents, 1002 lookups answered out of the readback cache
- `accessibility`: exit 0 — 999 documents in 13.9s: page one of each, and every page of every document with structure
- `launch_path`: exit 0 — launch-path: 4 documents measured, 0 absent, 0 not counted, 21 figures banded, 0 not judged, 0 outside
- `dates`: exit 0 — test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.88s
- `xmp`: exit 0 — 37  pdf:Keywords
- `jpeg2000`: exit 0 — test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.81s
- `render_raster`: exit 0 — 958 pages compared in 29.7s: 930 agree, 21 differ, 7 refused, 16 not comparable
- `fixed_documents`: exit 0 — fixed-documents: 77 checked, 0 absent, 77 rows
- `transform_gate`: exit 0 — transform: 1-200 of ISO 32000-2 rendered at 150 dpi in 1.291 s: 154.9 pages/s (floor 40), 24 threads
- `transform_writer_corpus`: exit 0 — transform-writer: 974 documents in 7.0s, 24 threads
- `transform_split_corpus`: exit 0 — transform-split: 974 documents in 51.0s, 24 threads, 48 dpi
- `transform_merge_corpus`: exit 0 — transform-merge: 974 documents in 53.0s, 24 threads, 48 dpi
- `transform_pages_corpus`: exit 0 — transform-pages: 974 documents in 81.0s, 24 threads, 48 dpi, up to 4 pages each
- `transform_optimize_corpus`: exit 0 — transform-optimize: 974 documents in 30.8s, 24 threads, 48 dpi
- `transform_foreign_corpus`: exit 0 — transform-foreign:   pages: drew differently: 0
- `transform_archive_corpus`: exit 101 on the merged tree, stopping in 0.02 s on the first signed document — sibling 982's new predicate catching the converter re-serialising a conforming source (ADR 1006); **re-run after that fix: exit 0**, every conforming document still conforming at all six targets
- `vfs_write`: exit 0 — vfs-write: 974 documents in 46.2s, 24 threads, 48 dpi, 3 pages a document
- `vfs_read`: exit 0 — vfs-read:   jbig2                    119 document(s), 1186 files read (47.2 MiB), 5 refused, 0 killed

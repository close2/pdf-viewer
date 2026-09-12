# 981 — A ranking applied per font on one route and per code on the other

Date: 2026-09-11. Branch `round-945/the-fifth-round`, from `9fac224b`. Stream: fonts, ISO 32000-2
clause 9 — `doc/todo/21`'s gaps and the clause-9 `partial` rows. Four sibling rounds were live in
the same working tree throughout (979 transparency, 980 colour, 982 `pdf-archive`, 983
`pdf-syntax` and `tools/conformance`).

Argued in **ADR 1002**.

## Which gap, and why

The corpus gate was run first and its four silence lines read: **129 codes over 7 documents**
reaching no glyph in silence, **57 over 9** reaching a glyph the font draws blank, **0 over 0**
drawn upright where §9.7.5.1 named a vertical form, **1227 over 42** that §9.10.2 could not name.
Every document behind the first line is a simple font showing code 0, code 10 or a byte of
UTF-8 — `doc/todo/21` §3 had already characterised all seven — so the silence lines were at their
floor for this corpus and the consequence had to be found by reading the code beside the clause.

The brief's first item was taken: `doc/todo/21` §1's "per-character fallback", which that section
had written about a second *face*. Reading `load_composite` beside `LoadedFont::text` found a
different thing wearing the same words. §9.10.2 ranks its methods "in the priority given" and its
closing sentence is about methods that "fail to produce a Unicode value" — a ranking per *code* —
and §9.7.4.2 makes that clause the whole glyph selection algorithm for a substituted composite
font ("CIDs shall not participate in glyph selection"). The readback applied the ranking per code;
the drawing route held a copy of **one** table, chosen once per font, and a code the producer's
`/ToUnicode` omitted never reached the collection's `registry-ordering-UCS2` table. Two routes
over one clause, disagreeing, and a page that could read back a character it did not draw.

## What changed

- `crates/pdf-font/src/loading.rs`: `Meaning` deleted; `CodeMapping::Substituted` holds no table;
  `LoadedFont::substituted_character` reads `to_unicode` then `collection` per code, the
  `/ToUnicode` final wherever it *states* the code; the `/ToUnicode` is parsed once where it was
  parsed twice; the `NoSubstitute` refusal is unchanged in effect. The pin is
  `a_code_the_to_unicode_omits_is_selected_through_the_collection`, a hand-built `Type0` over
  `UniJIS-UCS2-H` whose `/ToUnicode` deliberately disagrees with Adobe-Japan1 about its one stated
  code, asserting relations between three codes' glyphs and no glyph index — calibrated to fail
  under either single-table route.
- `crates/pdf-font/src/tounicode.rs`: `ToUnicode::states`, the question `char_for` could not ask
  (an omitted code and a stated sequence both answered `None`), with two tests.
- `crates/pdf-font/src/composite.rs`: `collection_meaning` → `collection_table`, answering the
  CID-keyed `ToUnicode` it always was.
- `crates/pdf-font/src/fixture.rs`: `document_of`, a multi-object fixture builder carrying bytes.
- `crates/pdf-font/src/lib.rs`: `Meaning` no longer exported (nothing outside the crate used it).
- `crates/pdf-font/examples/partial_to_unicode_census.rs`: the population instrument.
- `doc/conformance/ledger.toml`: §9.10.2 and §9.7.4.2, a sentence and the test each; both stay
  `implemented`. No `partial` row was closed: none of the clause-9 `partial` reasons the brief
  named is what this change touched, and each was read rather than edited.
- `doc/todo/21-font-substitution.md`: §1 split into the face it still owes and the table this
  round took; the status line amended.
- `doc/todo/53` item 2 was read with §9.6.5.2's neighbours (§9.6.5.3, §9.6.5.4) and left as ADR
  0932 closed it: nothing there is a residue this tree still has.

## The population

`partial_to_unicode_census` over every corpus on this disk — 90 129 documents opened of 90 537 —
finds the shape in **four**: `sumatrapdf-1550-0.pdf`, `sumatrapdf-LINK-1532-0.pdf` (tika),
`2514637.pdf`, `3621086.pdf` (SafeDocs). Every page of each was traced with
`PDFVIEWER_TRACE_MISSING_GLYPH=1` on the old route and the new one: identical (63, 171, 0, 0
lines, none from a font of this shape). The codes those pages show are the codes their producers
state, so **no silence line moves** — before and after, 129/7, 57/9, 0/0, 1227/42 — and the pin is
the fixture.

## Gates

The whole of `doc/todo/02` §2, `pdf-font` being under everything. Corpus: 974 documents, 63
incomplete, exit 0, the four lines above unchanged. Oracle: 1956 pages — 990 agree, 62
contradicted, 835 ambiguous, 47 not comparable, 17 no render — exit 0. Text extraction: 11095 of
11132 matched words in bounds (99.67%), 494 of 504 documents fully in bounds, exit 0. Selection
census, accessibility census, launch path, dates, xmp, jpeg2000, the quorra corpus, fixed
documents, the transform gate and its writer, split, merge, pages and optimize walks: exit 0 each.

**Figures below are off the merged run of the nine-hundred-and-eighty-fifth session**, which ran the whole of `doc/todo/02` §2 over this round's work beside its four siblings' after the session limit terminated this round mid-sequence:

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


## What a neighbour's sequence did to mine, which is worth one paragraph

Sibling 983's gate sequence started after mine had checked the machine was quiet and ran beside
it for a quarter of an hour. Its `foreign_corpus` overlapped mine on the shared
`tmp/foreign-readback/` directory and mine failed with `No such file or directory` on files the
other run had just replaced, plus the §14.7 parent-tree faults `doc/todo/02` §2 already documents
as the load-average shape; two `write_corpus` walks then ran side by side, which is the memory
rule's rule 1 broken, and mine was stopped. A quiet-machine predicate checked once at the start of
a sequence is not a predicate over the sequence; ADR 1002 does not carry this because it is a
habit rather than a decision, and `doc/todo/02` §2 already states it.

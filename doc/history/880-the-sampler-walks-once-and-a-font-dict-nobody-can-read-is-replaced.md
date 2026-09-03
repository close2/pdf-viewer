# 880 — The sampler walks once, and a Font DICT nobody can read is replaced: round 877 merged, `tools/bounded.sh`'s tree sampler is linear with a deadline and stops a tree it has gone blind on, `batch5/FOP` is walked, and a CID-keyed CFF draws the glyphs under an unreadable Font DICT against an empty Private DICT while keeping the rest

Date: 2026-09-03.
ADRs: [0807](../adr/0807-a-bound-that-is-not-being-measured-is-not-there.md),
[0808](../adr/0808-the-font-dicts-are-replaced-and-what-they-cannot-hold-is-counted.md).
Touched: `tools/bounded.sh`, `tools/conformance/tests/bounded.rs` (new), `crates/pdf-font/src/cff.rs`,
`crates/pdf-font/src/loading.rs`, `crates/pdf-model/src/content.rs`, `crates/pdf-model/src/content/text.rs`,
`crates/pdf-model/tests/corpus.rs`, `doc/checks/fixed-documents.toml`, `doc/conformance/ledger.toml`,
`doc/todo/03-more-corpora.md`; and the merge commit before this round's own.

## The merge

`round-877` (b125c8dd, ADR 0796: ICCBased 'CMYK' blending spaces converting in through the
profile's B2A, and a one-component CIE-based mask group taking §11.5.3's Y) is on `main` as
`3ff7ac41`, `--no-ff`, with no conflicts: the two files both sides edited, `oracle.rs` and
`ledger.toml`, merged at different lines. `r877` stays open, because round 879 branched from it.

## The sampler

Round 874 watched `tools/bounded.sh`'s `tree_rss` hang for minutes and killed it by pid. Read
against the script: its `awk` scanned every row of the process table for every node of the tree
— 0.07 s a sample over a flat tree of a thousand on a synthetic table, 6.4 s over eight thousand,
16 s over sixteen thousand — with no guard against visiting a pid twice, and the loop was
`sample; sleep 1`, so a `ps` that stalled under the memory pressure the ceiling exists for stalled
the bound with it and said nothing. ADR 0807: children gathered per parent in one pass, indexed
rather than concatenated (a string of a hundred thousand pids grown one at a time was a second and
a half by itself), the tree walked once with each pid marked seen — a hundred and thirty
milliseconds over a hundred thousand rows; each sample run in the background against a five-second
deadline and abandoned rather than waited for when it misses; six misses in a row stop the tree
with a line that says `KILLED BLIND`, distinct from the ceiling's. `--self-test` exercises the
synthetic tables, a live tree of two hundred children, a child holding 1.5 GiB under a 1 GiB
ceiling, and a sampler replaced by one that never returns; `tools/conformance/tests/bounded.rs`
runs it under `cargo test -p conformance`.

One mistake of the round's own, recorded because it is general: the script was edited — one
citation added to its header — while the first gate sequence's `nextest` was running the self-test,
and the self-test's inner invocation died of a syntax error at a line that had moved. `bash` reads a
script from disk as it runs it; a script under test is not edited until the test is done.

## The walk, and the head

`batch5/FOP`, 808 documents, surveyed whole under the four rules — twelve rayon threads, `--data 8
--tree 12`, 2.5 s, 0.99 GiB peak: 2 unopenable, 0 locked, 1 encrypted beyond us, 0 pageless, 34
incomplete, 0 slow. `doc/todo/03` §42 has the reports by kind and the ranking — ours flattened on
white against `pdftoppm -cropbox` and `mutool draw` at 72 dpi over all 34, round 876's script
under `tools/bounded.sh`.

The head was at the light end with both references agreeing to a hundredth: `FOP-2736-4.pdf`, ours
0 against `poppler` 4.35 and `mupdf` 4.36, "font /F20's program has no outline for any of the 1816
code(s)". A bare CID-keyed CFF from Apache FOP 2.3.0-SNAPSHOT's subsetter, 7925 intact
charstrings, whose Top DICT puts the `FDArray` at offset 8001 — inside its own format-0 `FDSelect`,
208 to 8133 — so every glyph selects a Font DICT no reader can find; `read-fonts` answers
`InvalidIndexOffsetSize(0)`, `fontTools` asserts, and both references draw the page through
FreeType's tolerance (trap 9). Table 124 says the program "shall conform to Adobe Technical Note
#5176" and it does not; §9.7.4.2 reaches the outline through the CharStrings INDEX, which is whole.
ADR 0808: `cff::readable_font_dicts` re-encodes the Top DICT with fixed-width offsets and appends a
fresh `FDArray` — every Font DICT that reads kept with its Private offset shifted, every one a glyph
selects that does not an empty Private DICT — and names the glyphs under a replaced DICT that call
a local subroutine it cannot hold, the one way a Type 2 outline depends on its Private DICT; a page
that shows such a code says so once per font, at the end, with the program's own sentence. The
sheet draws at 4.38 by the ranking's instrument, looked at glyph for glyph against `poppler`'s; the
fixed-documents gate reads 8.759 and that is the band. `FOP-2491-1.pdf`, the same subsetter with
9 of 13 glyphs calling local subroutines, went from nothing drawn to four glyphs and *12 of the
code(s) the page shows through it reach such a glyph*. The tracker is 33 incomplete.

**The first draft replaced every Font DICT whenever one was unreadable, and the corpus gate
refused it.** `doc/pdf.js`'s `issue9278.pdf` — iText 2.0.8, two fonts of 65 535 charstrings with
nineteen Font DICTs, the first four stating no Private DICT beside fifteen that read — went from
`complete` to seven and eleven lost glyphs, because the fifteen readable DICTs' subroutines had gone
with the four. The gate's rule that every new report sentence has a `whose_defect` row is what
stopped it; the second draft keeps what reads, and that page is `complete` with its glyphs under the
four drawing where they drew nothing. `cff.rs`'s fixture builders pin both shapes: the witness's
(an `FDArray` pointing at zeros) and `issue9278.pdf`'s (a readable DICT with a subroutine beside one
a glyph selects and the INDEX lacks).

## Gates and binaries

The whole `doc/todo/02` §2 sequence on the merged and changed `main`, each line under
`tools/bounded.sh` (`--data 8`, `--tree 8` for a build and `12` for a walk), each walk waiting on
any other round's, twice: the first pass was refused by `nextest` (the self-test edited under it, and
`§18` in a doc comment read as a clause of ISO 32000-2 — "section" for another document's numbers)
and by the corpus gate (the regression above); the second pass green throughout. Formatting and
`clippy` under `-D warnings` silent for the workspace and for `fuzz/`; **3006 tests passed, 20
skipped**; doctests green; the corpus gate at **974 documents, 64 incomplete**; the oracle at
**1945 pages, 1841 complete, 104 incomplete**; the three text gates green (99.67% of matched words
in bounds, 493 of 503 documents fully in); the two censuses green (102 853 elements reached); dates
1514 of 1545; XMP and JPEG 2000 green; quorra at **958 pages, 932 agree, 22 differ, 4 refused**;
fixed documents **59 of 59**, this round's row among them; the transform gate at 185.8 pages/s over
a floor of 40; the writer over 974 documents, 941 attached and read back, nothing unexplained;
conformance green, re-run after the last ledger edit together with the fixed-documents line;
`--bin quotations` and `--bin pointers` name nothing of this round's. A fifth round, so §5's
binaries, `libviewer_ffi.so` among them, were rebuilt and installed after the sequence.

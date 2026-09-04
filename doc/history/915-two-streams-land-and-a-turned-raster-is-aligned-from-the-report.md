# 915 — Two streams land, and a turned raster is aligned from the report

Date: 2026-09-04.
ADRs: [0872](../adr/0872-two-streams-write-into-one-clause-and-the-merge-is-where-the-count-is-wrong.md),
[0873](../adr/0873-the-renderer-states-where-it-put-the-page-and-a-turned-raster-is-then-aligned-rather-than-searched.md).
Touched: two merge commits and, after them, `crates/pdf-transform/src/render.rs`,
`crates/pdf-transform/src/lib.rs`, `crates/pdf-transform/src/json.rs`,
`crates/pdf-transform/src/update.rs` (one table number),
`crates/pdf-transform/tests/pages_corpus.rs`, `doc/conformance/ledger.toml` (§7.7.3.3 and, in the
merge, §12.4.2), `doc/state-of-play.md`, `doc/todo/57-the-transform-suite.md`, two ADRs, this
file. **No pixel moves**: nothing here is on a path that draws, and the eight corpus walks say so.

A merge round of two long-lived streams, plus the one item `doc/todo/57` still named that was
small enough to take beside them.

## The two merges

**`round-867` (e27804c8) is on `main` as `cb6278de`**, `--no-ff`. The branch carried nothing main
did not already have but session 910: `split --at-bookmarks`, and the three document-level
constructs a piece now carries. ADR 0862's point is that the three clauses give three different
answers — §12.3.3's outline is *permitted*, so what binds is the shape once there is one and every
conditional entry of Tables 150 and 151 is rebuilt over the kept subset; §12.4.2's labels are
permitted and the source's own tree is **forbidden**, because a key is "the page index of the first
page in a labelling range" and "[t]he tree shall include a value for page index 0"; §12.3.2.4's
named destinations are carried by whatever still names them, because a name is not an indirect
reference and §7.3.10's null cannot stand in for one. `NOT_CARRIED` is eight entries instead of
twelve. **Git found no conflict**, and the ledger was checked row by row: main had moved §7.3.7,
§9.6.4 and §11.5.3 against the common ancestor and the branch §7.7.4, §7.9.6, §12.3.2.4, §12.3.3
and §12.4.2; the two sets are disjoint, no third row moved, 875 rows on all four versions.

**`round-911` (92821fcc) is on `main` as `5fdc52f2`**, `--no-ff`, on top of it. That is the whole
file-system faces stream but for round 899's core, which main already had: 902's
`confined-transport` crate and the vfs worker under `Profile::Interpreter` unchanged, 906's five
write verbs and the fourth writer `pdf_transform::update` they needed, 909's write-side corpus walk
and the `pdf-fuse` face, and 911's mount by hand with the ten defects it found. `doc/todo/02` §2
gains two lines — the `pdf-vfs --bins` build, which is trap 10 one crate over, and `write_corpus`
— so the sequence now carries **eight** corpus walks; §5 gains `pdffs` and `pdf-vfs-worker`.

**One conflict, and it is the only clause both streams wrote into.** §12.4.2's row: session 906
added `update` as a page-label writer and session 910 added `split`, each as a pure append to the
`code` list, the `test` list and the end of the note. ADR 0872 is why the union is not the
resolution — session 910's sentence calls a piece "the third caller of that construction", counting
`merge` and `pages` before it, and on the merged tree `update` makes it the **fourth**. The two
sentences go in session order and the word changes with them.

**And the row-by-row check found something a clean auto-merge would have kept**: session 906's
sentence cites "Table 159's `/P`", and Table 159 is the entries in a *folder* dictionary. The page
label dictionary is Table 161, which the same row's own first sentence says. Corrected in the row
and in `update.rs`'s doc comment. Sessions 905, 908 and 910 each did this check and reported
nothing; a check that has not yet failed is one whose population has not yet held a defect.

Everything else auto-merged and was read rather than assumed: `split.rs` and `merge.rs` took
session 910's rewrite beside session 911's `Host::reserve_slot` and `replace_object` returning a
`Result` where they returned an `Option`, and `doc/state-of-play.md` and `doc/todo/02` each moved
in sections the other side had not touched.

## What `render` now says about where it put the page

ADR 0831 §1 priced this in session 893 and said what it would cost: "a change to the renderer's
report, not to the walk". `pdf_transform::render::Overrun` is two numbers in `0.0..1.0` — the strip
of raster the page does not reach on each axis — carried by `Rendered` and by `Origin::Page`, so
RFC 0002 §4.5's JSON states it beside the `width` and `height` it already had. It is computed from
the same three numbers `TargetSpec::for_page` builds the target from, and **not** recomputed in the
walk, which is the whole reason it is in the report.

`pages_corpus.rs` now derives the whole-column shift instead of hunting for one. Over the 905
rotated pages the worst mean falls from **26.44 levels to 15.84**, the least similar tile from
**−0.4325 to 0.0021**, and **17 pages that differed become byte-identical**. The overrun puts 194
pages at 0 columns and 711 at 1.

**Shifted the other way the same walk reports 39.19, −0.4272 and none**, which is the calibration
trap 13 asks for and the reason the first two figures mean anything. Read the *mean* rather than
the worst tile across the pair: trap 26's fixed grid moves when a column is cropped, which is why
the worst tile error rises to 47.03 while the mean falls by ten levels.

It is still measured and not asserted, and ADR 0873 names what is left: the grid, which turns with
the page and which no integer shift undoes, and the sub-pixel remainder an integer shift leaves —
at most half a pixel and 0.5000 at its worst here. `doc/todo/57`'s item narrows to *the tolerance*,
which is a statement about this renderer's antialiasing and has to be derived rather than picked.

## The gates

**The whole `doc/todo/02` §2 sequence, all twenty-eight lines, every one exit 0**, on the merged
tree with this round's work in it, each walking line under `tools/bounded.sh` (`--data 8 --tree 8`
for a build, `--data 12 --tree 12` for a walk), one walk on the machine at a time, waiting on
`/proc/PID/exe` and on the load average before each.

`Summary [69.299s] 3272 tests run: 3272 passed (1 slow), 28 skipped`; doctests green; both `fuzz/`
lines clean. Corpus **974 documents in 10.1s — 0 unopenable, 9 locked, 1 encrypted beyond us, 5
pageless, 64 incomplete, 0 slow**; oracle **1945 pages in 29.9s (1841 complete, 104 incomplete)**,
979 agrees, **61 contradicted**, 836 ambiguous, 47 not comparable, with
`our_rendering_agrees_with_the_reference_consensus_across_the_corpus ... ok`; text extraction
**99.3% (24014/24193 words), 22 below 90%**, the PDFBox lane **99.8% (14257/14281)** and the
position verdict **11094/11131 in bounds (99.67%), 493 of 503 documents fully in**; selection
census **1000/1011 words (98.91%) over 453 documents**; accessibility census **102 853 elements
reached, 57 116 a caret can move through**; dates **1514 of 1545 (97.99%)**; XMP **318 of 319
streams read**; JPEG 2000 green; quorra **958 pages compared in 30.0s: 929 agree, 22 differ, 7
refused, 16 not comparable**; fixed documents **71 checked, 0 absent, 71 rows**; the transform gate
**180.0 pages/s over a floor of 40**; `writer_corpus` **941 attached, read back and removed, 0
unexplained refusals**; `split_corpus` **965 bit-identical, §12.3.3 outlines carried 147,
§12.4.2 labels 22, §12.3.2.4 destinations 68, `--at-bookmarks` over 23 documents with 0 lost or
duplicated pages**; `merge_corpus` **965 bit-identical, every reconciliation counter at zero**;
`pages_corpus` **0 label faults and the alignment above**; `optimize_corpus` **26.71% saved, every
property counter at zero**; `foreign_corpus` **203 of 974, the bookmarks lane 5 written and 5 held
by qpdf, poppler and mupdf alike**; `write_corpus` **935 insertions, 84 deletions, 941 attachments,
4955 pages bit-identical to the page they were, 0 prefix failures**; conformance **875 subclauses,
14 371 citations, 1264 quotations verbatim**.

**Two lines failed on the way and neither was the tree's.** The corpus gate failed at exit 101 on
the first run — `ContentStreamCycleType3insideType3.pdf` at **31.65 s** against the 30 s budget —
while a neighbouring round was running its own workspace tests at a load of 44. Alone and quiet the
same tree walked 974 documents in **10.5 s with 0 slow**. That is `doc/todo/02` §2's recorded false
positive for the third time, and its rule held: a `slow` failure is a thing to re-run alone before
it is a thing to diagnose. And `cargo nextest` failed twice on the round's *own writing* — a `§`
after "ADR 0831" in `render.rs` and in the §7.7.3.3 row, which the conformance gate rejects because
a `§` is resolved against ISO 32000-2's clauses and would pass by landing on one. Both are now
"section 1", which is what that gate exists to insist on.

**§5 ran, this being a fifth round**, and it now installs **ten** binaries plus
`libviewer_ffi.so`: `pdffs` and `pdf-vfs-worker` joined the list with the merge.

**And trap 10's third copy was dealt with rather than left.** `<target>/release/examples/pdf-sandbox-worker`
is a hand-made file no `cargo build` line reaches; the copy here was from 00:13 and **differed from
the release worker byte for byte**, so any `--release` example run in between measured a build
older than its tree. It was refreshed from §5's fresh binary — the fix trap 10 states — rather than
removed, because removing it would make every release example refuse §7.4.6's, §7.4.7's and
§7.4.9's codecs instead of decoding them, and `examples/open_one` on `jbig2_symbol_offset.pdf`
reports `unsupported []` afterwards.

## The worktrees

Seven closed, and five left open with a reason apiece. Closed: **r867, r899, r902, r904, r906,
r907 and r909** — each verified an ancestor of `main` with `git merge-base --is-ancestor`, each
with nothing uncommitted, and none of them a branch a running round was created from. About 197 GB
of build directories came back with them.

**`r911` stays open**, and it is the one the round was told to close: rounds 913 and 914 were both
created from `round-911`, and the rule the instruction states beside the list — only branches "from
which no running round has branched" — excludes it. Its commits are all on `main`, so deleting the
ref would lose nothing; what it would do is take a ref out from under two rounds that are still
running, for 25.4 GB. The next main round can close it. **`r913`, `r914`, `r916` and `r917` stay
open because they are running** — the last two were created while this round was in its gates, and
`r916` from `round-913`. The build root is 267.8 GB afterwards, which `tools/worktree.sh list`
prints and which `doc/todo/02` §5a's threshold is about; nearly half of what is left belongs to
directories this project's tools did not make.

# 1419 — Two backends compared draw the same frames, and an outline's topology is read of its own flattening

Status: accepted. Session 1291. Closes ADR 1407 section 4 (the 4× residue) and its last paragraph
(the shared `OnceLock`); amends ADR 1389's "asked of whichever flattening reaches it first".
Supersedes nothing. Context: raster's ADR 0054 (a frame's bytes do not depend on its thread
count); ISO 32000-2 §8.5.3.3, §10.7.2; traps 54, 56, 58, 62, 65, 66, 67, 71; habits 52–55.
Code: `crates/render-raster/tests/corpus.rs` (`OneThread::compare`, `outcome`, the artefact
write), `raster/crates/raster-gpu/src/resources.rs` (`StoredOutline::winds_two_values`),
`encode/parallel.rs` (`rasterise`).
Tests: `raster/crates/raster-gpu/tests/encode_threads.rs`
(`the_page_after_a_refused_frame_is_the_same_bytes_at_every_thread_count`),
`encode/parallel.rs`'s `an_outlines_two_values_answer_does_not_depend_on_who_asks_first`, the
corpus gate's `OneThread` list, now held empty at every scale.

**I did not open `crates/render-cpu/`.** Every expected value is the clause's or raster measured
against itself; the oracle appears only as the corpus gate's verdicts.

## 1. The first differing insert at 4×, and why the arms disagreed there

Instrumented in an export with its own target directory (removed): every atlas insert (key, size,
position), every room probe and every `settle_atlas` decision, per backend, with the page name
between frames. Outline ids are per-device resource ids, so the logs were compared with them
stripped. The sequences agree through `issue12798_page1_reduced.pdf`; the first differing insert is
the first glyph of `issue12810.pdf` (`Quantised(9, 11)`, even-odd, 11×11 at 14,267), which the
fanned-out backend inserted and the one-threaded backend never asked for. That page is **refused**
at 4× (609 086 160 scene-derived bytes against 268 435 456), and the gate drew the one-threaded
arm only when the other had drawn: the refused frame had committed its inserts before the refusing
charge (`Encoder::charge` drains first, so both thread counts commit the same ones), and from the
next page on the two backends kept different atlases. `issue12823.pdf`, the first page the survey
named, is the page after it. A second asymmetry of the same shape: a page differing from the oracle
was drawn a second time, by the fanned-out backend only, to write its artefacts.

**None of the three hypotheses in the brief was the cause** — a pending insert past its bound, a
repack reading counts the queue changes, a commit out of one thread's order — because the two arms
were not drawing the same frames; once they do, no insert, probe answer or repack decision differs
anywhere in the walk. The invariant the one-threaded arm keeps is the history: the same frames,
in the same order. The comparison now draws every page on both backends whatever the first
answered, holds a refusal equal to a refusal of the same words, and writes artefacts from the
frame it judged. With that, the insert logs of the whole 4× page-turn walk are identical line for
line (the room probes and the `settle_atlas` decisions too), and **0 of 966 pages differ (176
before)**, so the list is held at every scale. The fixture
states the shape in isolation: a long page refused inside its run of glyphs, then a text page, on
an atlas the refused frame filled — the same refusal, counters and bytes at 1, 2, 3, 7 and 64
threads, and counters a fresh device does not reach.

## 2. An outline's two-values answer is asked of its own flattening

`winds_two_values_as` let a path-lane worker answer ADR 1389's question from its placement's
flattening where nobody had asked yet, and the compute route asked of the outline's own. The curves'
crossing and nesting are placement-free; a flattening's are not: an arc whose control points lie
0.2 px off its chord at a tenth of its size flattens to the chord (§10.7.2's quarter pixel), and a
subpath nested in its bulge falls outside. The test builds exactly that outline and asks in both
orders: before, the coarse placement asking first fixed `true` where the outline's own flattening
answers `false`, and the compute route would have integrated a winding of 2 on that answer.
`winds_two_values_as` is gone; every caller asks the outline's own flattening, where the relative bound keeps each curve read at a fixed fraction of its
size. Cost: one extra flattening per outline for the life of the store.

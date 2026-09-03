# 890 — A rectangle of no area stops only the marks it places: `batch5/sumatrapdf` walked, a damaged font prefix held by ADR 0459's name, and thirteen annotations that stated their geometry whole are drawn and said

Date: 2026-09-03.
ADR: [0825](../adr/0825-a-rectangle-of-no-area-stops-only-the-marks-it-places.md).
Touched: `crates/pdf-model/src/appearance.rs`, `crates/pdf-model/src/annotation.rs`,
`crates/pdf-model/tests/annotations.rs`,
`crates/pdf-model/examples/point_rectangle_census.rs` (new),
`doc/checks/fixed-documents.toml`, `doc/conformance/ledger.toml`,
`doc/todo/03-more-corpora.md`.

No merge: this round branched from `main` at `db6f7a3c` — round 889's merge of 885 — and leaves
`round-890` for a main round to take. `r867` (round 888) and `main` (round 889) were running beside it throughout, which is why
every walk below waited on `pgrep` first and why the gate sequence was run after round 891's had
finished rather than beside it.

## The walk, and the two-row head

`batch5/sumatrapdf`, 320 documents, surveyed whole under the four rules — `--data 8 --tree 12`,
3.1 s, 1.83 GiB peak: 2 unopenable, 2 locked, 0 encrypted beyond us, 3 pageless, **17 incomplete**,
0 slow. `doc/todo/03` §44 has the reports by kind and the ranking. **5.31% incomplete is the second
lowest rate of any tracker walked**, below the pdf.js gate's 6.98% and far below `PDFIUM`'s 17.4%,
because a SumatraPDF issue attachment is most often a document somebody could not read rather than
a fuzzer's output.

The ink ranking against `pdftoppm -cropbox` and `mutool draw` put two rows within a hundredth of
each other at the head, and **the first of them was already answered**: `sumatrapdf-378-0.pdf`,
ours **0** against 3.298 and 3.310, 339 text operations behind a `/FontFile2` whose Flate data is
`Corrupt` after 409 275 bytes. ADR 0459 asks two conditions of such a stream — Table 125's stated
extent reached, *and* the damage a truncation rather than a grammar violation — and its own witness
`issue13316_reduced.pdf` is why: that one reaches its extent to the byte and, read as a whole
program, draws **A C E F** where six CJK glyphs belong. Held by name, with `-854-0` and `-1505-4`
in the same tracker as the same shape.

## The head this round took, and why the report could not have found it

`sumatrapdf-LINK-1618-0.pdf` is `pdfcomment.sty`'s demonstration sheet, ours **4.19** against
`poppler` 7.48 and `mupdf` 7.93. Its reports named three `/DA` fonts the `/DR` does not define, a
cloudy `Square` and three `Text` icons outside §12.5.6.4's seven — all four held decisions, none of
them 3.3 levels of ink. **Looking at the page is what found it** (trap 1): a line with arrowheads,
a strikeout, six underlines, two squiggly underlines and two highlights that both references draw
and this tree drew nowhere **and reported nowhere**.

All thirteen state no `/AP` at all and a `/Rect` written as a point — `/Rect [ 100.2 732.197 100.2
732.197 ]` and twelve like it — with their own geometry whole in `/QuadPoints`, `/L` and
`/Vertices`. `annotation::decide` dropped them on one line, before `crate::appearance` was reached.

**The rule that line implements is §12.5.5's, and §12.5.5 is about a stored stream**: the algorithm
scales the appearance's transformed `/BBox` onto `/Rect`, so a rectangle of no extent leaves no
mark. A *construction* goes through none of it — `crate::appearance` writes its stream in the
page's own default user space and `annotation::construct` places it with the identity — and what
decides whether `/Rect` bounds one is the subtype's own clause. `Constructed::bounded` had been
answering exactly that since ADR 0193, for exactly the six subtypes involved. **So the tree held
two answers to one question and the wrong one ran first.**

That the standard's silence here is a silence is checkable rather than assumed, and that is the
half of the argument worth keeping: §12.5.6.5's Table 176 gives a *link's* `/QuadPoints` a fallback
to `/Rect` in as many words, and §12.5.6.10's Table 182 — the same array, for a mark rather than
for a region — gives none, nor do Tables 178, 180, 186 and 177. Table 166's `/AP` row points the
same way rather than against: it frees a *writer* from supplying an appearance dictionary for
annotations whose `/Rect` is a point, so a point `/Rect` beside a whole `/QuadPoints` is a file the
standard anticipates, and a reader excused from finding a stream is one that has to construct one.

ADR 0825, and it is one line of behaviour and one of design: `Constructed::bounded` becomes
`appearance::bounded_by_rect(subtype)`, asked by both the `/BBox` clip and the empty-`/Rect` guard,
so the list exists once; and the guard returns `Decision::Nothing` only where the rectangle is what
places the marks. Every construction `/Rect` *does* place is untouched — §12.5.6.8's square is
"inscribed within the annotation rectangle", an icon sits on the largest square inside it — and
`a_construction_that_rect_places_draws_nothing_where_rect_covers_no_area` is the test that passes
against the old code, beside the two that fail against it.

The page draws twelve of the thirteen and **names the thirteenth** where it was silent: the
`PolyLine`'s `/BE` is `/S /C`, so §12.5.4's cloudy-border refusal reports it as the `Square` beside
it already was. From 4.19 to 4.54 by the ranking's instrument and from **8.385 to 9.078** by the
fixed-documents gate's, whose row carries a band of a tenth rather than a level because the twelve
marks are worth 0.693 of one on an A4 sheet — the second row in that file to be narrowed, and it
says so where the first does.

**The population was measured rather than guessed.** `examples/point_rectangle_census` reads
`/Annots` on every page with `pdf_syntax` alone (trap 8): over `batch5`'s 6119 files, 6074 open,
**122 documents state an annotation whose `/Rect` covers no area, 1237 of those state no `/AP` at
all, and 57 of those — over six documents — still state the entry their own clause puts in default
user space**. The other 1180 are `Link` (1009), `Widget` (213), `Text` (87) and one `RichMedia`:
subtypes `/Rect` places, unchanged by this, and the census is what says so.

## The lesson

**A rule this project derives once belongs in one named place that every caller asks.** `bounded`
was the correct reading of six clauses, written down, tested and carried in a struct field, while a
second copy of the same question — spelled `is_empty(rect)` — answered it the other way forty lines
off. Neither was wrong alone; what was wrong is that there were two. And the cost was trap 5's
shape: the drop happened *in front of* the code that knows what a subtype's clause asks for, and a
report is written from what a clause asks for, so twelve marks went missing from a page with
nothing able to say so.

## Gates

The whole `doc/todo/02` §2 sequence in the worktree, each line under `tools/bounded.sh` (`--data 8`,
`--tree 8` for a build and `12` for a walk), each walk waiting on any other round's — round 891's
sequence was in `writer_corpus` when this round's was ready, and this one waited for it rather than
running beside it, which is the section's own rule about a machine that is not quiet.

Formatting and `clippy` under `-D warnings` silent for the workspace and for `fuzz/`; **3061 tests
passed, 22 skipped**; doctests green; the corpus gate at **974 documents, 64 incomplete**; the
oracle at **1945 pages, 1841 complete, 104 incomplete**; the three text gates green (99.67% of
matched words in bounds, 493 of 503 documents fully in); the two censuses green; dates (1514 of
1545 strings conforming), XMP and JPEG 2000 green; quorra at **958 pages, 929 agree, 22 differ, 7
refused, 16 not comparable**; **fixed documents 61 of 61**, this round's `sumatrapdf-LINK-1618-0.pdf`
row among them; the transform gate at 195.2 pages/s over a floor of 40; the writer over 974
documents, 941 attached and read back; conformance green — **875 ledger subclauses, 0 owing a
review, 13 037 citations and 1182 quotations**.

**The sequence was run twice and the second run is the one these numbers are from.** The first
found `every_quotation_is_the_standards_own_words` red on this round's own new example: its module
comment blockquotes Table 166's `/AP` bullet and named the table without a clause, and an
unattributed quotation is one nothing can check. `§12.5.2` in front of it is the whole of the fix,
and the six lines the edit could reach — formatting, both `clippy` lines, `nextest`, the doctests
and `conformance` — were re-run after it, along with the corpus, oracle and conformance lines whose
own summary the first run's `tail` had cut off. §5's binaries were rebuilt in release and installed
into `target/`, which is this round's cadence.

`tools/state.sh` was not used for any figure here: every number above and in `doc/todo/03` §44 came
off a run this round watched print.

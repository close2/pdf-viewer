# 903 — The thickness a clause bounds, and a table whose second page has no caption

Date: 2026-09-03.
ADRs: [0848](../adr/0848-the-thickness-a-clause-bounds-and-the-adjustment-it-names.md),
[0849](../adr/0849-a-table-that-runs-onto-a-second-page-and-six-parameters-filed-under-the-wrong-one.md).
Touched: `crates/pdf-model/examples/stroke_adjustment_census.rs` (new),
`crates/render-cpu/tests/stroke_width.rs`, `crates/pdf-model/src/content.rs`,
`crates/pdf-model/src/content/ext_gstate.rs`, `crates/pdf-model/src/content/transparency.rs`,
`crates/pdf-model/src/content/pattern.rs`, `crates/pdf-render/src/sub_pixel.rs`,
`crates/pdf-model/tests/oracle.rs`, `doc/conformance/ledger.toml` (§10.7.5, §8.4.1, §8.4.3.2),
`doc/todo/_scan-conversion.md`, `doc/todo/11-shapes-that-still-disappear.md`, `doc/verify.md`,
two ADRs, this file; and the merge commit before this round's own. **No pixel moves.**

## The merge

**`round-899` (9eed3f7c) is on `main` as `7e04ab5c`**, `--no-ff`, on top of round 901. It is
RFC 0003's first landing and the read side only: the new `pdf-vfs` crate holding the mount's layout
as one declarative table with two write meanings per row, the generation key that makes a splice of
two documents unrepresentable rather than merely unlikely, refusals separated into by-design and
not-yet-implemented, a worker seam for ADR 0812's transport, three lines on `pdf_transform::Source`,
and `doc/todo/58` as the stream's standing item. `r899` is **not** closed: round 902 branched from
it and is building the confined worker on top.

**Git found no conflict at all.** `doc/conformance/ledger.toml` auto-merged although round 901 had
written to it on `main` — the branch's change there is four rows gaining a consumer (§7.11.4,
§12.3.3, §14.3.2, §14.3.3) and `git diff main HEAD -- doc/conformance/ledger.toml | grep '^-'`
prints nothing, which is the check that `main` lost no line rather than a hope about three-way
merging.

**The whole `doc/todo/02` §2 sequence then ran on the merged `main`, all twenty-five lines, every
one exit 0**, on a quiet machine, the walking lines under `tools/bounded.sh` (`--tree 8` for a
build, `--data 12 --tree 12` for a walk) one at a time. Verbatim where it matters:
`Summary [69.569s] 3171 tests run: 3171 passed (1 slow), 26 skipped`; corpus **974 documents in
11.5s — 0 unopenable, 9 locked, 1 encrypted beyond us, 5 pageless, 64 incomplete, 0 slow**; oracle
**1945 pages in 31.8s (1841 complete, 104 incomplete)** with
`our_rendering_agrees_with_the_reference_consensus_across_the_corpus ... ok`; text extraction
**11 094/11 131 matched words in bounds (99.67%), 493 of 503 documents fully in**; selection census
**1000/1011 words (98.91%) over 453 documents**; accessibility census green over 57 116 elements a
caret can move through; dates **1514 of 1545 (97.99%)**; XMP **318 of 319 streams read**; quorra
**958 pages compared in 30.6s: 929 agree, 22 differ, 7 refused, 16 not comparable**; fixed documents
**69 checked, 0 absent, 69 rows**; the transform gate **151.3 pages/s over a floor of 40**; the four
transform walks and the foreign readback green; conformance **875 subclauses, 13 711 citations,
1236 quotations verbatim**.

## §10.7.5's first requirement: measured, and declined

The row has been `partial` since the nineteenth session because the clause has two requirements and
this tree implements the second. ADR 0848 is the first time the *first* one was measured rather
than argued, and the reason nobody had is worth the sentence: uniformity is a claim across
**placement**, and every instrument this clause had been argued from — ADR 0419's seventeen widths,
ADR 0844's four resolutions — varies something else.

So a **phase ladder**: eight rules of one width on one page, each an eighth of a device pixel
further along, thickness read as ink over the rule's device length. Ours against `poppler`, `mupdf`,
`ghostscript` and `hayro` at 72 dpi, each document built twice — with `/SA true` and with no
`/ExtGState` at all.

At a requested 0.6 of a device pixel, thickness over the eight placements: **ours 0.5961 .. 0.6000**,
`mupdf` 0.5882 .. 0.6471, `ghostscript` 0.7373 .. 0.8039, `hayro` 1.0000 .. 1.0039, `poppler` 1.0000
flat. The clause bounds thickness at half a device pixel from the requested width; **we are 0.0039
from it and `poppler` — the only renderer here that grid-fits — is 0.4000 from it.** `/SA` moves
nothing for any of the four, at either width, in either direction, which is ADR 0688's finding
confirmed a third time on a document written for the purpose.

**So the fit is declined on the sentence's own second half**, not on cost and not on the
anti-aliasing departure. Both edges of a rule land on integers only if the device width is a whole
number, so quantising the coordinates of a 0.6-pixel rule draws it 1.0 thick — a hundredfold worse
on the quantity the same sentence bounds. The two halves are compatible on the aliased device
§10.7.4 describes and pull against each other on the anti-aliasing device §10.7.1's NOTE permits.

**The architecture question was asked first so that it could not become the answer**, and it does
not: the fit is a device-space translation, so it needs one function beside `Stroke::device_width`
and three call sites, with no path rewritten and quorra's `StrokeKey` untouched. "It cannot be done
here" would have been false.

**The population is the crawl's**, and the widening changes it: `stroke_adjustment_census` (new)
counts what reaches the display list rather than what a dictionary states, and finds **6 first pages
of `doc/pdf.js`'s 974** carrying a stroke a fit could move — 460 strokes, 438 of them on
`issue14297.pdf` — against **4832 of the 65 679 crawl pages that open**, 7.4%. The old note's "there
is no page on which this device could do better" was a claim about the corpus. The same census says
which half of the clause the world asks for: of 1 836 739 crawl strokes drawn with the parameter
enabled, **1 343 558 are under half a device pixel**, so three in four want the promotion.

The row stays `partial`, under the ledger's own vocabulary rather than under the reading:
`implemented` means every requirement is *executed*, and this one is not, whatever one concludes
about its outcome. What is new under it is a gate —
`stroke_width.rs::stroke_adjustment_holds_the_thickness_within_half_a_pixel_at_every_placement`,
held to the clause's half-pixel bound and to a tenth of it, failing at 0.005 (trap 13) and failing
under `poppler`'s construction.

**And the worst number this device produces is its own**: 0.1802 of a device pixel of thickness
spread on a 45° rule at or under one pixel, where `substitute_width` acts rather than the closed
form. Inside the clause's bound, worse than `mupdf` and `hayro` there, and better than all four
references above the substitution band at 0.0028. `doc/todo/11`'s, not this clause's.

## The table whose second page has no caption

Found while reading §10.7.5's condition. **Table 51 runs onto a second page of ISO 32000-2 (158)
whose header carries no caption**, so its last six parameters — stroke adjustment, blend mode, soft
mask, alpha constant, alpha source, black point compensation — sit immediately above Table 52's
caption. Ten citations in this tree filed one of them under Table 52, one of them under **Table 58**
(the path construction operators), and four ADRs carry it, including ADR 0620 — the ADR that found
this exact shape for the rendering intent and put stroke adjustment on the wrong side of its own
line.

Checked against both instruments before believing the conversion: `doc/md/` shows the continuation
as an uncaptioned table between the two captions, and `pdftotext -layout -f 173 -l 173` over the
PDF shows page 158 whole. The stroke adjustment row's own NOTE settles it — *This is considered a
device-independent parameter, even though the details of its effects are device-dependent* — which
only makes sense in the device-independent table.

No pixel moves; two readings narrow. §8.4.1's advice about a device-independent page description and
§8.7.3.1's prohibition on a pattern's content stream are both about Table 52's eight and about none
of those six, so a tiling cell may state `/SA`, a blend mode, a soft mask and the alpha constants.
ADR 0849 has the grep that finds the family and why it is not made a sweep.

## Gates

The merge's sequence is above. After this round's own work the **whole sequence ran again on
`main`** — the change→gate map puts `pdf-render`, `pdf-model` and `render-cpu` under everything,
and although nothing here changes a mark, a round that touches those crates runs it all. **All
twenty-five lines exit 0 again**, and every figure is the merge run's unmoved but one:
`Summary [71.229s] 3172 tests run: 3172 passed (1 slow), 26 skipped` — the one test this round
added — with corpus **974 documents in 12.2s, 64 incomplete, 0 slow**, oracle **1945 pages in 42.2s
(1841 complete, 104 incomplete)** and its consensus test `ok`, quorra **929 agree, 22 differ, 7
refused, 16 not comparable**, fixed documents **69 checked, 0 absent**, the transform gate **189.4
pages/s**, and conformance **875 subclauses, 13 714 citations, 1237 quotations verbatim** — three
citations and one quotation more than before, which is the two ADRs and the amended rows.

## What is left

§10.7.5's first requirement is still not executed and the row still says so; what it no longer rests
on is an argument. `doc/todo/11` gains nothing new but now has the turned rule's 0.1802 measured
against four references. The `--bin tables` sweep still cannot see a citation that attributes a
prose parameter name rather than a `/Key`, which is ADR 0620's finding with ten instances under it
instead of one.

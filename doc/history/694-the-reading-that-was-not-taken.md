# 694 — The reading that was not taken

The round was asked to work the queue the six-hundred-and-ninety-second session created: 63
`doc/corpora/pdfbox` pages in `ambiguous_undiagnosed.txt`, a file that had been empty since the
three-hundred-and-seventy-ninth. It did that, and its own baseline run found something first.

## The baseline was red, and the gate was right

The round's first act was an oracle run on its branch, which is the six-hundred-and-ninety-third's
merge with nothing of this round in it. It failed:

```text
1 page(s) newly contradicted by the reference consensus: ["function_based_shading_cmyk.pdf page 2"]
```

That page had left `CONTRADICTED_DEVICE_CMYK_CONVERSION` six rounds earlier, on a note that checked
the two things a round would think to check — our raster is byte-identical across that round's
change, and the mechanism the group names was untouched — and concluded that *the consensus
dissolved*.

It had not. **The consensus was there and one of its members was not in the run.** The figure the
removal was written on, 29.06, is `poppler` against `mupdf` on this page to the hundredth, while
`mupdf` against `ghostscript` is 0.192% of channels where the page's bound is 1.00% — a consensus
by any bound this gate holds. All three cached reference panels for the page carry an mtime of
2026-07-29 and today's `pdftoppm`, `mutool` and `gs` reproduce them with the gate's own arguments
to the fourth decimal of ink, so nothing on the reference side moved either. The page is back where
it was, and the section that removed it stays where it is with a sentence in front of it saying
which of its claims this supersedes.

**The instrument defect underneath is the general half, and it is trap 9's ninth entry now.**
`render_references` tolerates one of three references failing as long as two remain — correct, and
its doc comment argues for it — but the `Ok` arm dropped the failures on the floor, so a page judged
on two readings printed a line indistinguishable from the same page judged on three. A verdict
computed from a pair is a different measurement from one computed from a trio, and nothing said
which had happened. It says so now, on the page's own line and in the summary, beside the
abstention line ADR 0513 added for the other way a reading can be missing. The run that
established this prints six such pages, every one of them previously invisible. ADR 0542.

## The 63, in two groups and one population argument

`AMBIGUOUS_TEXT_AT_DOCUMENT_SIZE` (59) and `AMBIGUOUS_PAGE_PLACED_A_ROW_APART` (4); ADR 0543 has
the whole of it and the group notes carry the measurements. Four things are worth repeating here
because they are about method rather than about these pages.

**The shape the queue was filed under was wrong about two of the four bounds.** It said 62 of the
63 fail the differing fraction and the similarity *while sitting well inside the mean and the worst
tile*. Counted off the gate's own lines: differing fraction 63, similarity 47, **mean 47**, **worst
tile 4**. Nothing acted on the sentence and the correction is free; what it is worth is that a
shape read off a listing is a hypothesis in exactly the way `doc/todo/00` says.

**The failing bound is one the references miss against each other on every page of the set.** The
*smallest* of the three reference-to-reference differing fractions exceeds the 5.00% bound on all
63, from 5.11% to 14.27%, and our worst is at or below the largest of them on 51 of 63. That is
`doc/oracle-and-corpus.md` §3c's result — ADR 0243 found this bound alone among the four rejecting
29.4% of reference pairs on text pages — arriving on a corpus that had no part in setting it. The
mean converts the same way: ours is at or below the largest reference-to-reference mean on 61 of
63.

**Four names are one page**, which `doc/todo/00` says to check before taking a name off the list.
The `PDFBox.GlobalResourceMergeTest` documents print the same four metrics to the digit; our render
is one md5 across all four and `poppler`'s is another, and their `pdftotext` readback is one digest
and is not empty, so the match is evidence rather than that file's known false positive.

**And the gate's line for these pages is `poppler`'s on 53 of 63 and `ghostscript`'s on 10, never
`mupdf`'s.** One measurement says why: the best whole-pixel offset between our raster and `mupdf`'s
is (0, 0) on all 63, and between ours and `poppler`'s it is one device row down on 50. On the four
`PDFBOX-3110-poems-beads` pages that row is 72% of the difference, and those are the only four of
the 63 whose worst tile fails — which is why they are their own group.

The closed forms are in the notes: on `cweb.pdf` page 4 two independent ladders land 0.022 of 255
apart and ours at 8× lies between them; on `poems-beads` page 1 they land 0.0033 apart and ours is
0.028 under; on `PDFBOX-5840-410609` page 3 they agree to 0.024 and ours is 0.13 above at every
scale, which is §9.6.2.2's *these fonts, or their font metrics and suitable substitution fonts* and
this tree's Foxit outlines against the machine's URW.

**One trap sprang inside a ladder and is worth the line.** The first `ghostscript` ladder was taken
without `-dTextAlphaBits=4`, which the gate passes, and read 16.57 at 72 dpi where the panel the
gate compares against is 18.25. Trap 3 is about the invocation of a reference in the oracle; it is
equally about the invocation of one in a by-hand measurement.

## What did not move

**No pixel.** The change under `crates/` is `tests/oracle.rs` and one data file — a gate's
diagnoses, its ratchet lists and its reporting — so no quorra lane and no step-7 ink sweep were
owed, and none of the four verdict counts moved. The contradicted list is one name longer and the
undiagnosed list is empty.

## Instruments

`doc/todo/02` §2's core, the conformance gate, the corpus gate and the oracle, plus
`text_extraction`, both censuses, `dates`, `xmp`, `jpeg2000`, `fixed_documents` and the quorra
corpus gate; §4's `overtaken` and `quoted` against this round's own oracle log, neither of which
names a note this round wrote; `quotations` and `pointers`, whose hits are the ones already
recorded. Every number is in the run and none of it is in this file, which is ADR 0281's rule.

§5's binaries were **not** rebuilt: this is not a fifth round and the round took no measurement of
the launch path, a frame or a page turn.

## The machine

Shared with three other rounds and heavily loaded throughout — the load average was 104 when the
round's second oracle run started and had been 234 a minute before. The same gate took 39.6 s on
the baseline run, 103.0 s at that load, and 39.9 s on the final run after the last edit, at a load
of 5.3. **Three runs, one tree, and the wall clock is a measurement of the machine**; the verdict
counts are identical across all three and are not. `PDFREF_CACHE` pointed at the shared warm cache at
`/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`, whose entries are keyed on the invocation and
the document's digest — which is also what made ADR 0542's first check possible, because a panel
that has not been rewritten since July is a panel three rounds ago read.

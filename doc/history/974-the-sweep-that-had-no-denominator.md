# 974 — The ink sweep had no denominator, and sixty-three pages had never been in it

Date: 2026-09-11. ADR: 0985.
Files: `tools/pdfref/src/undrawn.rs` (new), `tools/pdfref/src/bin/undrawn.rs` (new),
`tools/pdfref/src/lib.rs`.

Sent to `doc/todo/00`'s named heads — the robustness denominator, which had had no attention for
five rounds. The oracle was re-run whole in this tree and all three of its rankings were read
against the record. The undiagnosed ranking is empty; every one of the thirteen marked rows on the
*we are alone* list is read and held; the contradicted pool's 62 pages are held by a group by name,
each. Step 7's ink sweep reproduces the negative head to the thousandth, with the alarm at 18 names
where the last recorded run said 19 — the one that left is `bug866395.pdf`, which ADR 0940 made
drawable — and with `issue12295.pdf` at −3.198 where it said −2.362, which is ADR 0945's stated
direction rather than a regression. The positive head, which had never been read as a list, is
held too: its new top is `bug1552113.pdf` at +50.365, and that is ADR 0674's priced consequence of
obeying §12.5.4 on a page where two references draw no border at all.

So the pages came back null, and the **instrument** turned out to be the round.

Step 7 has been a paragraph since the two-hundred-and-sixty-fifth session and has been rebuilt by
hand at least fifteen times. This round's rebuild measured **775 of the 838 pages it listed**,
losing 63 in silence: the gate
prints a page from a labelled corpus as `pdfbox/cweb.pdf page 10` and writes its evidence to
`pdfbox/cweb/p10/cweb-p10-ours.png`, so the label is a directory in the path and not part of the
file's name, and a loop written from the recipe looks in the wrong place and finds nothing. The
head, the alarm and every printed row looked exactly as they always had. That is trap 25 with the
sign reversed, and the answer is the one that trap already gives: derive the population, and print
the denominator.

`cargo run --release -p pdfref --bin undrawn -- <the oracle's log>` is that sweep as a program. Its
population and its *exclusions* both come off the gate's own report, its greyscale recipe is
compiled in so that no round can take the ink its own way, and a page it cannot measure is a
failure of the instrument rather than a line in the middle of a report. It carries
`doc/todo/00`'s three corrections and a fourth of this round's: **a panel file is not a render** —
`issue21436.pdf`'s `mupdf.png` is zero bytes, because `mutool draw` creates its output file before
deciding it cannot draw the page, which is trap 3 reaching a reader of the evidence rather than a
caller of the renderer. The gate already says so on the page's own line; nothing had ever read it.

Over the whole bucket: **839 listed, 839 measured, exit 0**, in 117 seconds. The 63 pages nobody
had ever measured sit between +0.048 and +0.749, so the five rounds that read this sweep's null
were reading a right answer over an unstated population.

Two documents want a line each and neither was edited here, because both belong to other hands
this block: `doc/todo/00` step 7, whose recipe is now a description of a program, and
`doc/verify.md`'s list of instruments that are not §2 gates.

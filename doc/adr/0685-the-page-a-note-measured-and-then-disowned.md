# 0685 — The page a note measured and then disowned

**Status.** Accepted. Session 761.

`freeculture.pdf` page 255 was in `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE` and is
`AMBIGUOUS_IMAGE_REDUCTION`'s. It is the book's second full-page cartoon: one 9258 × 12259
`CCITTFaxDecode` stencil reduced 23 to 1 onto 397 × 595 device pixels, with a running foot for text.
Four ink ladders converge inside 0.066 of 255 and our render is the most central of the five.

## Context, which is a note contradicting itself

Two paragraphs of one doc comment were about this page and they did not agree.

The older, from the two-hundred-and-thirty-third session, is a closed-form table: two reference
limits at `36.144 / 36.149` against ours 36.206, `hayro` 36.791, `poppler` 36.091, `mupdf` 36.062
and `ghostscript` 36.252, under the sentence *nobody here is drawing anything anybody else is not*.
The newer, from the round that re-derived the group's band, says of the same page that **it has never
been opened** and that *whatever it is, it is not the diagnosis these paragraphs make*.

Both were in the comment at once, and both were right about what they measured. This round re-took
the older table and every figure of it reproduces to the thousandth, five hundred rounds later.
**The ink answers how much and is silent about where**, and this page's whole disagreement is where:
mean 16.63 against a book whose next-highest is 8.99, worst tile 51.93 against 29.05, 19.98% of
channels differing against 15.37% — the extreme member of its group on three of the four measures
at once.

## What the page is

`pdfimages -list` reports one image and no other, a bilevel `CCITTFaxDecode` stencil at 2182 ppi.
`pdftotext` returns *BALANCES 247* and the margin's line numbers 1 to 33. It is not dense text at
book size, which is what its group is named for and what every other page of that group is.

## The measurement

Four independent ink ladders, `(1 − mean) × 255` after `-alpha off`, each renderer asked at its own
rising resolution (`doc/todo/00` steps 5b and 6, `pdftoppm -cropbox`, `mutool draw -r`,
`gs -dUseCropBox`):

```text
                    72 dpi    288 dpi   576 dpi
ours (1x/4x/8x)    36.0541    36.1299   36.1291
poppler            36.0913    36.0787   36.1436
mupdf              36.0617    36.1426   36.1485
ghostscript        36.2774          -   36.0832
```

Four limits inside **0.066 of 255**, ours between `ghostscript`'s and `poppler`'s. At the page's own
scale all five sit inside 0.74, `hayro`'s 36.7909 being the widest.

The ten pairwise comparisons at the page's own scale, mean of 255 and structural similarity, over
the gate's own panels:

```text
ours vs poppler          7.7509  0.89105        poppler vs mupdf        5.0690  0.95142
ours vs mupdf            6.8331  0.90454        poppler vs hayro        9.5472  0.84007
ours vs hayro            6.4309  0.92111        mupdf vs hayro          9.9300  0.83218
ours vs ghostscript     16.6313  0.75319        poppler vs ghostscript 16.2858  0.73813
                                                mupdf vs ghostscript   15.1531  0.78301
                                                ghostscript vs hayro   16.0906  0.75754
```

Averaged over the other three, **ours is the most central of the four smooth renderers** — 7.00
against `mupdf`'s 7.28, `poppler`'s 7.46 and `hayro`'s 8.64 — while `ghostscript` is 15.2 to 16.6
from every one of them. `magick identify -format '%k'` says why in one number: four panels carry 255
or 256 distinct levels and `ghostscript`'s carries **20**, so it quantises the reduction where the
others average it. The page is `ambiguous` because the four that average do not agree either, and
the ranking's divisor is the closest two of the voting three, `poppler` and `mupdf` at 1.01 floors.

## Why this page was taken at all, being below the mark

ADR 0684's criterion says a round works the `[widened: outside]` head and stops. This page is at
1.35× and unmarked — 1.37 ours over 1.01 between them — so on the criterion alone it would have been
left. **The exception is a page whose own note disclaims it**: a group note saying it cannot explain
one of its members is a page nobody is holding, whatever its rank, and `grep` finds those in a
second. The band re-derivation that produced that disclaimer is `doc/todo/00`'s own instrument
working; what it could not do is finish the job, and this is the other half.

## Consequences

- The page moves group. `diagnosed_ambiguous()` chains both, so no ratchet, count or verdict moves;
  the two arrays go 370 → 369 and 17 → 18.
- `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`'s band is re-derived from this run's own per-page lines and no
  longer needs a page set aside: `freeculture`'s 317 at mean 2.43 to 8.99, worst tile 11.60 to
  29.05, differing 5.61% to 15.37%, similarity 0.7210 to 0.9648.
- Its first table loses the page-255 row, and the note now records the contradiction rather than
  deleting it: **a closed form that clears a page for the measure it can see is not a diagnosis of a
  page failing the measures it cannot.**

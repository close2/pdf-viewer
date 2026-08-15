# ADR 0379 — The trailer eight megabytes from the end of the file

Status: accepted, 2026-08-15. Session 544. Takes `doc/todo/03` §1's chunk — the 126 documents of
`openpreserve/format-corpus` that its sparse checkout leaves behind — and the one document of them
this tree showed no page of at all. Amends §7.5.5's ledger row.

## The chunk, and why this one

`doc/todo/03` §11 was taken in the five-hundred-and-thirty-first and names no successor, so §1's
rule decides: a chunk is *somewhere nobody has been*, and it is either a different corpus or a
population out of the one on disk. The largest population on this disk that no round has ranked is
**`format-corpus` outside its three sparse-checked-out directories** — `jhove-errors` (99),
`office-examples` (13), `ebooks` (8), `variations` (4), `fully-featured-pdf` (1),
`desktop-publishing` (1) — **126 documents, 261 MB, no download and no network**. Session 467
surveyed the first two of those; nobody has ever *ranked* them against a reference, and the other
26 documents were never in any survey at all, because the five-directory question §2c answered was
about the PDF-corpus directories and these sit in `format-corpus`'s file-format directories.

The alternative §1 offers is SafeDocs' issue-tracker corpus, 31 GB in six archives. It is a corpus
for measuring a *rate*, and the crawl on disk has been surveyed whole three times; this chunk was
chosen on §1's own finding that **a corpus built to be diagnostic outranks a corpus built to be
large** when what a round wants is a defect. `jhove-errors` is diagnostic by construction — one
directory per JHOVE error code, real published papers inside — and it produced one.

## The defect: `startxref` where the file's end is not the document's end

`jhove-errors/PDF-HUL-138/6.2017-0960.pdf` is a 20 MB, 21-page AIAA paper. `pdftoppm`, `mutool`
and `gs` all draw its first page. This tree drew **nothing**, reporting "no first page" — the
whole document lost, in the one category `doc/todo/03` has recorded as *never this tree's fault*
for five populations running.

The file is one complete document with a **truncated copy of itself** appended after its `%%EOF`:
8 037 471 bytes of second copy, carrying objects and no cross-reference section of its own. So the
file's last `startxref` is the first copy's, and it is correct — and it sits eight megabytes from
the end of the file.

`xref::find_startxref` looked in the last **2048 bytes** and gave up. The constant's own comment
stated the premise: "a `startxref` further back than this is not a trailer, it is a coincidence."
The witness disproves it. What happened next is the part that costs the page: with no `startxref`,
`xref::read` falls to `rebuild`, which scans the body for object headers and takes the **last**
definition of each number — so every object of the document was replaced by the truncated copy's,
the catalogue's `/Pages 90 0 R` resolved into bytes that stop early, and the page tree emptied.

## The reading

§7.5.5 says where the trailer is and how a reader gets to it:

> PDF processors should read a PDF file from its end. The last line of the file shall contain only
> the end-of-file marker, %%EOF.

The file breaks that sentence, so the standard does not decide the recovery outright — but it does
rank the two candidates, and that ranking is the decision here. §C.4 licenses reconstruction for
one case:

> When a PDF processor reads a PDF file with a damaged or missing cross-reference table, it may
> attempt to rebuild the table by scanning all the objects in the file.

This file's cross-reference table is neither damaged nor missing. It is complete, self-consistent,
and merely not last. Reading further back is still reading the file *from its end*; rebuilding
discards what the file says about where its objects are and substitutes a rule the standard states
nowhere — the body's serial order, which §7.5.1 warns against in so many words ("[r]eading a
non-linearized file in a serial manner is not reliable"). **So the file's own statement outranks a
reconstruction, and the window is a fast path rather than a bound.**

`find_startxref` now looks in the last two kilobytes first — every conforming file answers there,
for two kilobytes of reading — and searches the rest of the file backwards only when that fails.
`rebuild` remains where it was, after both.

## What it costs, measured rather than argued

The extra work is one backwards byte scan, and only for a file that has already failed the window:
**425 of the 65 944 crawled documents, 0.64%**. For the 189 of those whose body does hold a
`startxref`, it is paid *instead of* `rebuild`'s scan of every object header rather than on top of
one — `jhove-errors/PDF-HUL-138/mattgib_1.pdf` opens in 8.5 ms where it took 22.6 ms. No
conforming file reaches the second search at all.

## What it moves

The population is the 192 documents on this disk whose tail holds no `startxref` while their body
does — 189 crawled, two in `format-corpus`, one in the 974:

- **Over the 189 crawled**, surveyed on both trees: *10 pageless, 10 incomplete* becomes *5
  pageless, 8 incomplete*. Five documents that produced no first page now draw one
  (`0669760`, `1899687`, `2760354`, `3006578`, `5958883`), and two that were drawing a page from
  the wrong copy's objects — reporting a fistful of `/Font` resources their own page never named —
  now draw complete (`5589735`, `7188997`). **Nothing moved in the other direction.**
- **`6.2017-0960.pdf`** draws its 612×792 first page, which is the page `pdftoppm` and `mutool`
  draw, at an ink of 9.77 against their 8.88 and 8.78 (`gs` 14.28, its own text weight).
  `jhove-errors`' survey line goes *1 pageless* → *0 pageless*.
- **`scan-bad.pdf`**, the one document of the 974 the change can reach, does not move at all: its
  table and its scan agree. The display-list digest over all 974 first pages is **byte-identical**,
  which is why no quorra lane and no ink sweep were run.

The rule is pinned by a hand-built pair in `cross_references.rs` — the same document with and
without a copy of itself appended, padded past the window — because the corpus witness lives in a
directory the submodule's sparse checkout does not take. The pair fails on the tree as it was.

## The rest of the chunk, and what it says

126 documents: **117 complete, 5 locked, 2 incomplete, 1 pageless, 1 unopenable**.

- The five locked want an open password (§7.6.4.1) and the two `simple-password-*.pdf` files that
  are *not* locked are the same producer's permissions-only encryption, opened with the default
  user password. Correct on both counts.
- The one unopenable file is an **AppleDouble sidecar** — 213 bytes of `Mac OS X` attribute block
  and a `com.apple.quarantine` string, saved under a `.pdf` name. So *nothing failed to open for a
  reason that is this tree's* still holds, now for the sixth population.
- The two incomplete ones are named populations: a `/Font` and an `/ExtGState` a page names and
  the file never defines (§7.8.3, ADR 0255).
- Ranked by our ink minus the lightest live reference's, page one at 72 dpi with every reference
  explicit about the page box, the whole negative tail is **−0.744 and shallower** — glyph
  rasterisation weight, and the largest of them was opened side by side to confirm it. There is no
  second whole-page row: session 505's ranking separated its defect by two orders of magnitude and
  this one separates nothing, which is the instrument saying the population is clean.
- **Two documents render here and in no reference at all**: `jhove-errors/PDF-HUL-29`'s pair, where
  poppler, mupdf and ghostscript each refuse the page tree ("Kid object (page 1) is not an indirect
  reference (null)") and this tree draws a complete journal page and a complete book title page.
  Recorded because a ranking cannot see it: they sit in the positive tail with no reference to
  subtract.

## The directory question, decided again with a number

`doc/third-party-data.md` left `jhove-errors` out of the submodule's sparse checkout on two
grounds: 275 MB, and "surveying it produced two ordinary reports". **The second ground is
disproved** — it produced a whole-page defect that four earlier populations did not — and the
first stands unchanged. The decision is to leave the sparse set as it is and say why: the guard
this defect needs is the hand-built pair, which every clone gets for a few hundred bytes, and a
round that wants the population itself can fetch the whole five-directory corpus into
`corpus-cache/` the way session 467 did. What changes is the record, not the pin: a directory left
"on size **and** on value" is now left on size alone.

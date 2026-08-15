# 544 — The trailer eight megabytes from the end of the file

**Finding.** `doc/todo/03` §1's chunk, taken over the 126 documents of
`openpreserve/format-corpus` that its sparse checkout leaves behind — 26 of which had never been
in any survey — produced the first document in six populations that failed for a reason of this
tree's own. `jhove-errors/PDF-HUL-138/6.2017-0960.pdf` is one complete 21-page paper with a
**truncated copy of itself** appended after `%%EOF`, so its correct `startxref` sits eight
megabytes from the end; `find_startxref` looked in the last 2048 bytes, gave up, and `rebuild`
then took the truncated copy's objects and emptied the page tree. Three references draw that page
and this tree drew none of it. §7.5.5's "PDF processors should read a PDF file from its end" is
read past the window now, ahead of a reconstruction §C.4 licenses only for a table that is
"damaged or missing" — which one that is merely not last is not.

**Date.** 2026-08-15.
**ADR.** [0379](../adr/0379-the-trailer-eight-megabytes-from-the-end.md).
**Touched.** `crates/pdf-syntax/src/xref.rs` (`STARTXREF_SEARCH_WINDOW`'s premise,
`find_startxref`, `last_startxref_from`), `crates/pdf-syntax/tests/cross_references.rs` (a
hand-built pair), `doc/conformance/ledger.toml` (§7.5.5), `doc/todo/03-more-corpora.md` (§12, the
chunk), `doc/oracle-and-corpus.md` §2b, `doc/third-party-data.md` (`jhove-errors`' entry),
`doc/adr/0379-*` (new), this file.

## The chunk, and why this one rather than 31 GB

§11 was taken in the five-hundred-and-thirty-first and names no successor, so §1 decided: a chunk
is somewhere nobody has been, and it is either a different corpus or a population out of the one
on disk. The two offers were SafeDocs' issue-tracker corpus — 31 GB, six archives, a *rate*
instrument for a crawl already surveyed whole three times — and a population here. The population
won on §1's own finding that a corpus built to be diagnostic outranks one built to be large when
what a round wants is a defect, and `jhove-errors` is one directory per JHOVE error code.

| directory | documents | complete | locked | incomplete | pageless | unopenable |
|---|---|---|---|---|---|---|
| `jhove-errors` | 99 | 95 | 0 | 2 | **1** | 1 |
| `office-examples` | 13 | 11 | 2 | 0 | 0 | 0 |
| `ebooks` | 8 | 5 | 3 | 0 | 0 | 0 |
| `variations` | 4 | 4 | 0 | 0 | 0 | 0 |
| `fully-featured-pdf` | 1 | 1 | 0 | 0 | 0 | 0 |
| `desktop-publishing` | 1 | 1 | 0 | 0 | 0 | 0 |

The five locked want an open password; the two `simple-password-*.pdf` files that are *not* locked
are the same producer's permissions-only encryption and open under the default user password. The
unopenable one is an AppleDouble sidecar saved under a `.pdf` name — 213 bytes of `Mac OS X`
attributes and a `com.apple.quarantine` string — so the streak holds for a sixth population. The
two incomplete are §7.8.3's named population.

## The ranking, and the two things it cannot see

Page one at 72 dpi against `pdftoppm`, `mutool` and `gs`, every invocation explicit about the page
box, ranked by our ink minus the lightest live reference's. **The whole negative tail is −0.744
and shallower** — glyph weight, and the largest of them was opened side by side to check rather
than assumed. Session 505's ranking on `pdfCabinetOfHorrors` and `govdocs1-error-pdfs` separated
its defect from the next row by two orders of magnitude; this one separates nothing, which is what
a clean population looks like through this instrument.

Two rows are outside what a difference can express, and both were read by hand:

- The **pageless** document, which has no reference difference because it has no render at all.
  That is the round's defect, and it is the argument for reading the survey's categories beside
  the ranking rather than after it.
- **`PDF-HUL-29`'s pair**, where every reference fails and this tree draws: poppler, mupdf and
  ghostscript all refuse the page tree for a `/Kids` entry that is not an indirect reference, and
  the two pages are a complete journal article and a complete book title page. They sit in the
  positive tail with nothing to subtract, which is exactly where a defect of *ours* would sit too.

## What the fix moved, measured on both trees

The population is the documents whose last 2048 bytes hold no `startxref` while their body does:
**189 of the 65 944 crawled, two in `format-corpus`, one of the 974**. Surveyed on both trees, the
189 go from *10 pageless, 10 incomplete* to *5 pageless, 8 incomplete* — five documents that
produced no page now draw one, two that were drawing a page out of the wrong copy's objects now
draw complete, and **nothing moves the other way**. `scan-bad.pdf`, the one of the 974, does not
move: its table and its scan agree, and the display-list digest over all 974 first pages is
byte-identical, which is why no quorra lane and no ink sweep were run.

The cost is one backwards byte scan for the 0.64% of files that miss the window, and for the ones
it succeeds on it is paid *instead of* `rebuild`'s scan of every object header:
`PDF-HUL-138/mattgib_1.pdf` opens in 8.5 ms where it took 22.6.

The corpus witness is in a directory the sparse checkout does not take, so the rule is pinned by a
hand-built pair — the same document with and without a copy of itself appended, padded past the
window — which fails on the tree as it was. That is also the answer to the directory question the
round reopened: `jhove-errors` stays out of the pin on size, and the "no value" half of the reason
`doc/third-party-data.md` gave is struck, because it produced this.

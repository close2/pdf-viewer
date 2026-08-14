# 521 — The samples a stream stops short of

**Finding.** `doc/todo/03` §9 left the other 96% of the crawl's damaged streams owed *per
consumer* — an image, an ICC profile and a function, each wanting the same two questions. The
chunk was taken and the questions turned out to have a better predicate than damage: for two of
the three the standard states the stream's **extent** independently, so a stream that falls short
is wrong whether a filter failed or the producer simply wrote too little. §7.3.8.2 infers an
image's length from its own dictionary — its EXAMPLE *is* an image, with the verdict spelled out —
and §7.10.2 states a sample array's outright. Both were being completed with **zeros**.

The witness was already in this file's own history: `178360.pdf`, which session 505's ink ranking
put in the positive tail as *ours 40.7, `poppler` 26.2* without anyone asking why. It is a
133 × 2944 `/ImageMask` whose flate stream is corrupt 359 bytes into the 50 048 its grid needs, and
**99.3% of it was marking the page in the fill colour** — a solid bar no reference draws, because
§8.9.6.2's default `/Decode` paints where a sample is 0 and every absent sample read back as 0.

Three answers, one per consumer:

- **An image** draws the samples it carries — they are the producer's own bytes at the positions
  §8.9.5.1's byte-aligned rows give them — and leaves the rest of the grid **unpainted** rather
  than zero, with `image::short_of_its_grid` reporting the shortfall beside the drawing. Ninth
  entry in trap 5's report-while-drawing list.
- **A sampled function** is refused. Its missing samples are not places on a page but *values* of a
  mapping evaluated over its whole domain, read as 0, decoded through `/Decode` and interpolated
  into the real samples beside them: substitutive, so a short table does not draw part of a
  gradient, it draws a different one.
- **A damaged ICC profile** needed no report. Table 65 states the whole recovery — `/Alternate`,
  else the device space `/N` implies — and `colour.rs` already took it. What it needed was not to
  be *parsed*: a tag table's prefix describes bytes that are not there, and an `A2B1` past the end
  drops into the curve-and-matrix branch where a missing `rTRC` reads as no curve at all.

**Date.** 2026-08-14.
**ADR.** [0356](../adr/0356-the-samples-a-stream-stops-short-of.md).
**Touched.** `crates/pdf-model/src/image.rs` (`unpack`'s partial rows and unpainted samples,
`short_of_its_grid`), `crates/pdf-model/src/content/image.rs` (the report beside the drawing),
`crates/pdf-model/src/function.rs` (`holds_the_sample_array`),
`crates/pdf-model/src/colour.rs` (`parse_icc_based`, extracted from `parse_at` and asking
`Decoded::damage` first), `crates/pdf-model/tests/image_extent.rs` (new, four),
`crates/pdf-model/tests/sampled_function_extent.rs` (new, three),
`crates/pdf-model/examples/damaged_stream_census.rs` (a role per damaged stream, both extent
arithmetics, and the shipped functions rather than a second copy of them),
`doc/conformance/ledger.toml` (§7.3.8.2, §7.10.2, §8.6.5.5, §8.9.5.1),
`doc/HANDOVER.md` (trap 5's list is nine, and the prefix test has a sharper form),
`doc/todo/03-more-corpora.md` (§9's fraction corrected, §10 records the chunk),
`doc/adr/0356-*` (new), this file.

## The chunk: the damaged-stream population by consumer

`govdocs1-error-pdfs` first, as §9 instructed — 29 of its 54 documents carry a damaged stream and
every file opens by hand — then `format-corpus`'s 167, the 974, and all 65 944 crawled documents
with one process per archive. 145 archives, 0 failures. The census's three older lines reproduce
session 508's numbers to the digit, which is what says the instrument did not move under this
round's changes.

| population | documents | damaged streams | short of their stated extent |
|---|---|---|---|
| SafeDocs crawl | 65 944 | **2260** in 726 documents | **54 images in 8**, 0 functions |
| pdf.js | 974 | 57 in 20 | 0 images, 0 functions |
| `format-corpus` (3 dirs) | 167 | 296 in 29 | **51 images in 2**, 0 functions |

**The 2260, by the consumer that reads them:**

| consumer | streams | what happens today |
|---|---|---|
| a page's `/Contents` | **841** | drawn and reported (ADR 0343) |
| an image | **529** | drawn; reported where short of its grid |
| a font program | **371** | refused and reported (ADR 0343) |
| unclassified | 296 | a Type 3 glyph, an `Indexed` palette, an appearance |
| an object stream | 144 | §7.5.7 |
| a form `XObject` | **46** | drawn, and **still silent** — the one thing left |
| an ICC profile | 19 | Table 65's alternate, deliberately |
| a cross-reference stream / metadata / function | 10 / 2 / 2 | — |

**§9's own arithmetic was pessimistic and this corrects it**: it read the loud route as 90 of 2260
because 90 counts documents whose *page one* holds one. Every one of the 841 reports when the page
holding it is drawn, so the loud share is 37% rather than 4%.

## The triage

| | documents | opened | refused loudly | reported (incomplete) |
|---|---|---|---|---|
| `govdocs1-error-pdfs` | 54 | 54 | 0 | 7 |
| `format-corpus`, three directories | 167 | 165 | 1 locked, 3 pageless | 22 |
| SafeDocs crawl (census pass) | 65 944 | 65 703 | — | — |

**Both survey lines are identical before and after this round's changes**, which is `doc/todo/02`
§7's entry in its second form: a count that does not move is not evidence that nothing happened.
`178360.pdf` was already incomplete for its twelve unparsable font programs, and its new image
report joins them; what changed is that 99.3% of a stencil came off the page.

## `178360.pdf` read against three references, at 72 dpi and the crop box

| | ink |
|---|---|
| ours, before | 39.83 |
| **ours, after** | **42.12** |
| `poppler` | 26.23 |
| `mutool` | 110.70 |
| `gs` | 51.38 |

**The number moved away from `poppler` and the page is better, which is trap 1 in its plainest
form.** The three references span 26 to 111 and settle nothing — this is a document whose twelve
embedded font programs are all damaged, so each reference draws a different amount of substituted
text. What the crop shows is what the clause decides: before, the left margin is a flat block of
the stencil's fill colour; after, the stencil marks only the 21 rows its stream carries and what
the page draws underneath is visible again. Ink rose because what was under the fabricated bar is
darker than the bar.

## What the change cost, measured rather than argued

**Nothing drawn moved in the gate corpus.** `examples/display_list_digest` over all 974 pdf.js
documents is **byte-identical** before and after — 958 interpreted first pages — which is the
census's own prediction: not one of the 974 is short of a grid, states a short sample array or
carries a damaged ICC profile. The corpus gate's incomplete count is **61 before and 61 after**,
measured both ways with the change stashed and unstashed. `doc/todo/00` step 7 needs no re-run for
ADR 0343's reason: the ink ranking reads the oracle's artefacts and its input did not move.

## Gates

`fmt` clean. `clippy --workspace --all-targets` silent. `nextest --workspace` **1894 tests run:
1894 passed, 15 skipped** — seven of them this round's two new files. Doctests pass. Corpus **974
documents in 3.9s: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 61 incomplete, 0
slow**. Oracle **906 agrees, 67 contradicted, 786 ambiguous**. `text_extraction` **10969/11163
matched words in bounds (98.26%), 486 of 508 documents fully in bounds**; `dates`, `xmp`,
`jpeg2000` green. `render-quorra` corpus **956 pages compared: 934 agree, 20 differ, 2 refused, 18
not comparable**, and the GPU lane at scale 4 **951 pages: 937 agree, 9 differ, 5 refused, 23 not
comparable**. `conformance` green: 7938 citations, 765 quotations all verbatim.

One thing the ledger cost a second run: a `\uXXXX` escape is valid TOML and this parser rejects it,
so the four in the new prose are literal characters now.

## What is left, and it is one report rather than a reading

A damaged **form `XObject`, tiling pattern, appearance stream or Type 3 glyph description** is
still silent, and §7.8.2's argument for a page's `/Contents` covers every one of them word for
word. 46 of the crawl's damaged streams and 7 of the pdf.js corpus's 57 are form `XObject`s;
`comments.pdf` and `highlights.pdf` are witnesses in the gate corpus. It was left out deliberately:
it is a report to place in five call sites rather than a reading to make, and folding it into a
round that changed what gets drawn would have made the byte-identical digest above prove less.
`doc/todo/03` §10 carries it.

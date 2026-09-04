# ADR 0859 — `batch5/cairo` walked, and a ranking whose light tail was its own instrument

Status: accepted. Session 908.
Clauses: ISO 32000-2 §7.3.7, §7.3.10, §9.6.4, §10.7.4.
Code: none of its own — the finding it carries is ADR 0858's.
Instruments: `tools/safedocs survey --dir`, `pdf-model`'s `examples/render_at` and
`examples/open_one`, `pdftoppm -cropbox` and `mutool draw -b CropBox` at 72 dpi, and
`doc/todo/00` step 6's four-render ladder.
Continues ADRs 0794, 0798, 0832, 0836, 0844. Beside ADR 0858.

## The directory

`corpus-cache/tika-issue-tracker/batch5/cairo`, surveyed whole under the four rules of
2026-09-02 — twelve rayon threads, `--data 8 --tree 12`, 0.5 s, 0.04 GiB peak. The line, a
baseline for this directory and never a ratchet:

| directory | documents | line |
|---|---|---|
| `batch5/cairo` | 166 | 0 unopenable, 0 locked, 0 encrypted beyond us, 2 pageless, 18 incomplete, 0 slow |

**10.84% incomplete is the highest rate of any tracker walked so far**, above `PDFBOX`'s 7.25%
and `REDHAT`'s 6.07%, and the shape is the tracker's: a cairo issue attachment is a file
somebody filed *because* cairo produced or refused it, so the directory is a bug reporter's
selection rather than a sample of documents in the world. Seven of the eighteen are one
attachment set, `cairo-85141-0.zip-*`, whose members are `dvips`-produced TeX output with
Type 3 bitmap fonts and damaged cross-reference sections.

Ranked by report, the eighteen are 10 `Font`, 4 `MediaBox` or `PageDictionary`, 2
`MissingResource`, 1 `Shading`, 1 `Operator`, and every population among them is one this
project has already argued: a program with no outline for the codes a page shows (§9.9's
closed-by-decision case), `/Encoding /Identity-H` over a descendant with no program
(`doc/todo/21`), a `/MediaBox` enclosing no area (ADR 0389's substitution), a `/FontFile2`
that decodes only as far as its damage (ADR 0836), and a Type 3 font with no `/CharProcs`
dictionary at all.

## The ranking, and the instrument that had to be repaired first

Ours flattened on white against `pdftoppm -cropbox` and `mutool draw -b CropBox` at 72 dpi, over
**all 164 documents with a first page** rather than over the eighteen incomplete ones — which is
what sections 44, 45 and 46 did and is where each of their findings came from. Ranked by distance
**outside the interval the two references bracket**, rather than by distance from the lighter of
them: a row between the two references is a row neither of them disagrees with us about, and the
first ranking of this directory put `cairo-9987-2.pdf` at its head (+6.19 from `mutool`) on a page
where `poppler` is *darker* than we are.

**The first run of that ranking was wrong at the light end, and it was the instrument** — the
fourth way to be wrong with this measurement, now in `doc/oracle-and-corpus.md` §3d. Five rows
came back at −8.7 to −22.8 levels, every one of them a page this tree drew blank while both
references drew ink:

| document | ours | poppler | mupdf |
|---|---|---|---|
| `cairo-71861-2.pdf` | 0 | 22.837 | 22.791 |
| `cairo-71861-0.pdf` | 1.002 | 23.228 | 23.225 |
| `cairo-48349-6.pdf` | 0.016 | 16.394 | 15.574 |
| `cairo-48349-7.pdf`, `-17.pdf` | 0.033 | 11.786 | 11.204 |
| `cairo-86093-0.pdf` | 0 | 8.733 | 8.738 |

All five are JBIG2, and none of them was ours. `examples/render_at` runs from
`<target>/release/examples/`, `pdf_sandbox::worker_program` searches beside the running executable
**first**, and that directory held a `pdf-sandbox-worker` some earlier round had copied there — ten
hours behind this tree, and nothing rebuilds it. `open_one` says so in one sentence, naming the
stale path and both build hashes, which is trap 5 earning its keep: a decoder that had failed
*quietly* would have made five findings out of nothing. With the copy refreshed, every one of the
five agrees with both references to within 0.13.

**The corrected ranking has 105 of its 162 comparable rows inside the references' own interval**,
and after the head below is fixed nothing in either direction is worth half a level except four
rows, all of them held with a reason.

## What the head was

`cairo-85141-0.zip-3.pdf` page 1, ours **4.6304** against `poppler` 1.7573 and `mutool` 1.6622 —
2.87 outside the interval where the next row is 0.50 — and reporting nothing at all. The ladder
does not converge (4.630 / 4.588 / 4.586 / 4.645 at 72, 144, 288 and 576 dpi), so it is content
rather than scan conversion, and the page confirms it: this tree draws a paragraph and a
ten-item numbered list that neither reference draws.

The cause is §7.3.7's dictionary parser walking out of its own object, and ADR 0858 is the whole
of it. What belongs here is what it cost to find and what makes it a *corpus* finding rather than
a clause one: the defect is invisible to every gate this tree has, it changes **no page of
`doc/pdf.js`**, and **not one of the 65 944 documents of the SafeDocs crawl** states it. It needed
a corpus of files people filed because a program choked on them, and it needed the ranking's head
to be read as a page.

## What is held, and why

Four rows are left outside the references' interval and each is held with its reason on the
record. Every one of them was put through the ladder before it was held.

- **`cairo-48349-6.pdf`, −3.71** — and it **converges**: the references fall from 16.39 / 15.57 at
  72 dpi to 12.56 / 12.31 at 576 while ours is flat at 11.86, so the gap closes to 0.48. That is
  §10.7.4's anti-aliasing departure on a dense inline-image page, which `doc/todo` already prices,
  read from the light side rather than the dark one.
- **`cairo-55799-0.pdf`, −2.52**, and **`cairo-54950-0.pdf`, −0.91** — both converge to within 0.16
  and 0.15 by 576 dpi, `poppler` moving 7.7 and 2.2 levels to meet us. Scan conversion on a
  227 × 114 and a small sheet, where a level is worth very little.
- **`cairo-31878-2.pdf`, −1.76** — this one does **not** converge (9.92 against 11.71 and 11.71,
  flat across the ladder), and it is `doc/todo/21`'s standing population rather than a new finding.
  The page is the four letters `test`, a code the font has no glyph for, and a `¢`; `poppler` draws
  a hollow rectangle for the code and this tree draws nothing, which is the whole of the 1.78
  levels. The survey's own census counts it as one of this directory's three "codes reaching no
  glyph *in silence*", and `doc/todo/21` is where that population and the question of what a
  reader owes for such a code already live. **What this round adds is only the witness**: nothing
  here decides whether a box is right, and the ink difference is a reason to read that row rather
  than an answer to it.

**Two documents of the 166 have no first page, and §29's claim about pageless files needs a
qualification here rather than passing.** `cairo-101530-0.pdf` and `cairo-101531-0.pdf` both open
onto `TrailerMissing { key: "/Root (not a dictionary)" }`, and `pdfinfo` reports **5** pages for
the first and **1** for the second — so poppler's reconstruction reaches a catalogue where this
tree's does not. What stops that being a finding is the other half of the same run: `pdftoppm`
produces **no raster at all** for either, and neither does `mutool draw`, both refusing on the same
syntax errors their `info` tools printed. So no reader draws these, and the disagreement is about
what a reconstruction may claim rather than about a page. It is held, and it is named here so that
a round wanting a reconstruction case has two files to start from.

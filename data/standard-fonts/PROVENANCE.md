# The standard 14 fonts, and where these bytes came from

ISO 32000-2 §9.6.2.2 names fourteen fonts a PDF may reference without embedding, on the
understanding that the processor has them.

**This file quoted that understanding as a `shall` until the four-hundred-and-thirty-first
session, and Errata Collection 3 struck the sentence outright** — "These fonts, or their font
metrics and suitable substitution fonts, shall be available to the PDF processor." (Issue #47 and
#48, `/State` `Review` `Completed`), while turning the paragraph above it into an informative NOTE.
`crates/pdf-font/src/standard.rs` was corrected in the four-hundred-and-eighteenth and this copy
was missed, because `tools/spec-errata` sweeps `crates/`, `tools/`, `fuzz/`, `doc/adr/` and the
ledger and does not reach `data/`.

What binds instead has not moved and is the stronger warrant: **Table 109** marks `/FirstChar`,
`/LastChar`, `/Widths` and `/FontDescriptor` as required but "optional in PDF 1.0-1.7 for the
standard 14 fonts", so a file may name one of the fourteen and state no metrics at all, and **§6.3.2.2**
requires a rendering processor to render the page contents as this document defines them. A
processor that has neither the programs nor the metrics cannot lay out one line of such a file.
That is a requirement about what is drawn rather than a sentence about what a processor happens to
have.

Until the hundred-and-forty-eighth session this tree took that "suitable substitution" from
whatever the machine happened to have installed, which made a page's appearance a property of the
computer rather than of the file. `crates/pdf-font/src/substitute.rs` described itself as *the
only machine-dependent code in the tree*. These fourteen files end that for the fourteen faces a
document names without supplying — the only faces where the file's intent is known and a
substitute is not a guess.

## What is here

| files | face | licence |
|---|---|---|
| `FoxitFixed*.pfb` (4) | Courier, in four styles | `LICENSE_FOXIT` — **BSD-3-Clause** |
| `FoxitSerif*.pfb` (4) | Times, in four styles | `LICENSE_FOXIT` |
| `FoxitSymbol.pfb` | Symbol | `LICENSE_FOXIT` |
| `FoxitDingbats.pfb` | ZapfDingbats | `LICENSE_FOXIT` |
| `LiberationSans-*.ttf` (4) | Helvetica, in four styles | `LICENSE_LIBERATION` — **SIL OFL 1.1** |

804 KB in total. The licence files are verbatim copies, next to what they cover, which is what
both licences require of a redistribution. `/NOTICE` at the root of this tree is the other half of
that obligation and is what `pdf-viewer --licences` prints.

## Where they came from

Both sets were taken from the pdf.js checkout this repository already carries as a submodule, at
`doc/pdf.js/external/standard_fonts/`, revision **`2ea8820d92ec48457bb3432876dcfff3bdd3f10e`**
(`v6.1.200-233-g2ea8820d9`). pdf.js's own `README.md` in that directory records where *it* got
them:

> The pfb files in this directory were extracted from Pdfium
>
> Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

The Liberation faces are Red Hat's, under the SIL Open Font License with `Liberation` as a
reserved font name — which permits shipping and using them freely and forbids modifying them while
keeping the name. Nothing here modifies them.

They are copied rather than read out of the submodule at build time, deliberately: the submodule is
optional (`doc/pdf.js` is 974 test documents and a clone of this repository is useful without it),
and a font a page needs in order to render is not an optional dependency.

## Checking them

`SHA256SUMS` is a digest of each file as committed. `cd data/standard-fonts && sha256sum --check
SHA256SUMS` verifies that what is here is what was vetted. It is not a security boundary — anyone
who can change the fonts can change the sums — it is a record that these fourteen files are the
fourteen that were read, licensed and measured, so that a later "where did this glyph come from"
has an answer.

## Why not the metrics as well

A standard-14 font may omit `/Widths` entirely on the grounds that the reader knows them, which is
Table 109's permission. The advances now come from these programs themselves, which is exact for
the ten Foxit faces — they are Courier-, Times-, Symbol- and Dingbats-metric by construction — and
metric-compatible rather than identical for Liberation Sans against Helvetica. Carrying Adobe's own
AFM tables would close that last gap and is a separate decision with a separate licence to read.

## What the sans faces are, measured

**Metric compatibility is about advances and says nothing about the vertical.** The
four-hundred-and-thirty-first session measured the one number that separates these bytes from the
faces the reference renderers resolve through `fontconfig`, by drawing a capital `I` straight from
each file:

| face | `I` at 144 px/em | cap height |
|---|---|---|
| `LiberationSans-Regular.ttf` | 99 rows | **0.687500 em** |
| `NimbusSans-Regular.otf` (the machine's Helvetica) | 105 rows | **0.729167 em** |
| `LiberationSans-Bold.ttf` at 288 px/em | 198 rows | 0.687500 em |
| `NimbusSans-Bold.otf` at 288 px/em | 210 rows | 0.729167 em |

5.7% shorter capitals, in both weights, and it is the whole of what six pages of the oracle's
`CONTRADICTED_SUBSTITUTED_FONT` differ by — 1.0% to 7.7% of a page's ink, largest on a page of
nothing but capitals. `FoxitSerif.pfb` has no such gap: on `bad-PageLabels.pdf` at 12 pt the drawn
cap is 63 device rows in ours and in both references alike.

Nothing in ISO 32000-2 requires a substitute to match the named font's cap height — §9.5 NOTE 5
puts font substitution beyond the standard in so many words — so this is recorded rather than
closed. ADR 0267 has the decision and its cost.

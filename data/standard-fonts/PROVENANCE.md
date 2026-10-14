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

## Adobe's Core 14 AFM files, for the pairs

`afm/` holds Adobe's metrics files for the fourteen and the notice that ships with them, as Adobe
publishes them, byte for byte (CRLF line ends included, `MustRead.html`'s CR ones too):

| files | what they are |
|---|---|
| `afm/Courier*.afm` (4), `afm/Helvetica*.afm` (4), `afm/Times-*.afm` (4), `afm/Symbol.afm`, `afm/ZapfDingbats.afm` | Adobe Font Metrics, version 4.1, one per face (`StartFontMetrics 4.1`, dated 1997) |
| `afm/MustRead.html` | Adobe's notice: use, copying and distribution for any purpose, on four conditions |
| `afm/SHA256SUMS` | a digest of each of the fifteen, as committed |

They came from Adobe's own server, `https://download.macromedia.com/pub/developer/opentype/tech-notes/Core14_AFMs.zip`
(127 150 bytes, SHA-256 `8c892c3c49553cfd2d2a27c4495b4bb12e2875115be7fd127ed3876df19d8654`, served
2026-10-14 with `Last-Modified: Thu, 02 Nov 2017`), unzipped unchanged. The archive's earlier home,
`partners.adobe.com/public/developer/en/pdf/Core14_AFMs.zip`, now answers 404; the Internet
Archive's copy of it from 2009 has the same digest. Apache PDFBox carries the same files under
`pdfbox/src/main/resources/org/apache/pdfbox/resources/afm/` (the `doc/corpora/pdfbox` submodule),
with their line ends normalised by its repository, which is why these are Adobe's and not those.

**Only their pair kerning is read**, and only by `crates/pdf-font/build.rs`, which writes each
file's `KPX` lines as numbers into the table `pdf_font::pairs` kerns §9.6.2.2's fourteen by
(`doc/questions/A308`, ADR 1916). That table is a **modified form of the files** — the
glyph names numbered, the pairs sorted, a face whose pairs repeat another's sharing them — and its
head says so and carries each file's notice, as `MustRead.html`'s third condition requires;
`/NOTICE` section 1c carries the paragraph unmodified, as its fourth does. The files themselves
are not modified. Helvetica states 2 705 pairs, Times-Roman 2 073; the Couriers, Symbol and
ZapfDingbats state none.

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
metric-compatible rather than identical for Liberation Sans against Helvetica. Adobe's own AFM
files are held now (`afm/`, above) and are read for their pairs alone; their widths would close
that last gap, and taking them is a separate decision from the one that brought them here.

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

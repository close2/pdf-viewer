# 613 — A division and a byte that each cost a whole page

`doc/todo/03`'s chunk again, over the population that produced the last one: the SafeDocs crawl.
Five whole archives this time, and two defects — one at each end of the ranking.

Date: 2026-08-20.
ADR: [0448](../adr/0448-a-division-and-a-byte-that-each-cost-a-whole-page.md).

Touched: `crates/pdf-model/src/image.rs`, `crates/pdf-syntax/src/filter.rs`,
`crates/pdf-model/tests/dct_components.rs`, `doc/conformance/ledger.toml` (§8.6.6.3, §7.4.5),
`doc/todo/03-more-corpora.md` §17, `doc/todo/_image-codecs-and-the-sandbox.md` §7, the ADR and this
file.

## The chunk

**`0300`, `1653`, `3252`, `4851`, `6327` — 5000 documents**, none of them 603's. An archive is a
hash bucket (ADR 0261), so any set of them is an unbiased sample. 603's instrument unchanged: page
one at 72 dpi against `pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box,
ranked by our ink minus the lightest live reference's, each panel's raster size beside it.

**Checked before it was trusted.** 612 made this tree apply §14.11.2.1's crop on every target, so
archive `0100` was re-ranked whole and diffed against 603's artefacts: one row of 1000 differs, and
it is 603's own fix. A page-sized target is the crop box, which is why that clause could not move
this measurement.

## What the two ends said

**Negative head — `4851434.pdf` at −20.341**, a bilevel scan drawn by all three references and
blank here, reported `stream did not decode`. Its `RunLengthDecode` data decodes to exactly the
266 456 bytes the dictionary describes and then carries one run header with no bytes behind it;
`run_length` answered that with `Corrupt`, which throws the prefix away. §7.4.5 gives the filter no
invalid byte, so running out of input is truncation wherever it lands.

**Positive head — `6327194.pdf` at +244.885**, a **solid black page**, one command and no report at
all, where three references agree on 8.9 to 9.2. A greyscale JPEG under
`[/Indexed /DeviceRGB 255 …]` whose palette is a grey ramp: the `DCTDecode` route divided every
sample by 255 before the lookup, so 256 entries were addressed at two of them. §8.6.6.3 — "[a] PDF
reader shall treat each sample value as an index into the colour table" — and the four other image
routes had it right.

## What moved

Seven archives re-ranked whole with the fixed tree and diffed row by row: **5 rows of 7000 move**,
four improved. `4851434` −20.341 → +0.127, `6327194` +244.885 → +0.058, and three `Indexed`-over-
JPEG documents in **603's own archives** — `0100408` +3.859 → +0.102, `0100681` +0.227 → +0.063,
`7680631` −0.050 → −0.102. Every other panel, ours and each reference's, is identical to the
thousandth; the one further difference is a `poppler` panel that timed out in the first run.

**No gate number moves**: no document of the 974 states either construct.

## What the head still holds

Named in the ADR with numbers: `hayro-jbig2` 0.3.0's flat 10 000-instance cap against a page
declaring 13 264 (upstream has already replaced it — a release to take, `doc/todo/_image-codecs`
§7); an aerial photograph drawn as ~1700 JPEGs one sample tall, which is `doc/todo/11` on a real
document; a `/DefaultCMYK` `ICCBased` conversion, trap 9's family; and `0300856.pdf`, whose eight
corrupt content streams salvage into 484 nonsense operators and a black page — ADR 0343's rule at
its extreme, recorded rather than traded away.

**And the positive tail is mostly not ours**: 22 of 5000 documents are pages where `poppler` alone
draws almost nothing while this tree, `mupdf` and `ghostscript` agree. A ranking against the
lightest live reference is sensitive to one reference failing quietly, which is a fact about the
instrument worth carrying.

## Gates

The full §2 sequence, because the change is in `pdf-syntax` and `pdf-model`. §5's binaries were
**not** rebuilt: this is not a fifth round and nothing on the launch path was measured.

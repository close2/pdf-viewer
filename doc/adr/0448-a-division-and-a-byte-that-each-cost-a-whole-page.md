# ADR 0448 — A division and a byte that each cost a whole page

Status: accepted, 2026-08-20. Session 613. Takes `doc/todo/03` §16's named successor — the next
chunk of the SafeDocs crawl — and fixes the two defects its five thousand documents produced.
Amends §8.6.6.3's and §7.4.5's ledger rows.

## The chunk, and why these archives

§16 leaves "63 944 crawled documents unranked … in archive-sized pieces a round can take one or
two of". This round took **five whole archives — `0300`, `1653`, `3252`, `4851` and `6327`, 5000
documents** — none of them the two session 603 ranked. *Which* archives is immaterial and that is
ADR 0261's finding: the crawl is sorted by SHA-256 and cut into 7933 pieces, so an archive is a
hash bucket and any set of them is an unbiased sample.

The instrument is 603's, unchanged: page one at 72 dpi from this tree and from `pdftoppm`,
`mutool` and `gs`, every invocation explicit about the page box (trap 3), ranked by our ink minus
the lightest live reference's with each panel's raster size beside it. About 0.12 s a document at
sixteen workers here, so an archive is two minutes.

**The instrument was checked rather than assumed, and it had a reason to be.** Session 612 made
this tree apply §14.11.2.1's crop on every target, so a harness written before it might have been
comparing different questions afterwards. Archive `0100` was re-ranked whole with the current tree
and diffed against 603's own artefacts: **exactly one of its 1000 rows differs, and it is 603's own
fix** (`0100223.pdf`, 0.000 → 225.476). Every other row, ours and all three references, is
identical to the thousandth. A page-sized target *is* the crop box, which is trap 14 read from the
other side: 612's clause could not move this measurement because the raster's own edge had been
enforcing it.

## The negative head: a run-length prefix thrown away

`4851434.pdf` at **−20.341**: a 1216×1753 bilevel scan the three references each draw and this
tree drew as a blank sheet, reporting `I1: malformed image: stream did not decode`.

The image is `/Filter /RunLengthDecode`, and its runs decode to **exactly the 266 456 bytes its own
dictionary describes** — 152 bytes a row for 1753 rows. What follows them is one more run header,
promising 33 bytes that the stream does not contain, and no EOD.

`run_length` answered that with `FilterRefusal::Corrupt`, which discards everything decoded so far;
the whole page was lost for one trailing byte. §7.4.5 gives this filter **no invalid byte** — every
header 0 to 127 is a literal run, 129 to 255 a repeat and 128 is the EOD — so the only way it can
fail is the input running out, which is `Damage::Truncated` wherever in a run it happens. The two
`ok_or(Corrupt)?` sites are a `break` now and the existing `salvage` at the end of the loop says
which damage it was.

**The claim was already written down and was false.** `FilterRefusal::Corrupt`'s own doc comment
says "`FlateDecode`, `LZWDecode` and `RunLengthDecode` all keep what they decoded before the
damage" — true of the first two since ADR 0343, and true of the third only when the data ran out
*between* runs.

## The positive head, which no round had read before

603's ranking looked at its negative tail. The positive one — where this tree deposits ink nobody
else does — held the round's second defect at the top of it.

`6327194.pdf` at **+244.885**: ours 253.823 of 255, a **solid black page**, against `poppler` 9.182,
`mupdf` 8.939 and `ghostscript` 8.953. One command, no report at all: an image, drawn wrong in
silence, which is trap 1's archetype.

The image is a greyscale JPEG whose dictionary states
`/ColorSpace [/Indexed /DeviceRGB 255 233 0 R]`, and object 233 is an ordinary grey ramp — index
*i* is (*i*, *i*, *i*). Its samples sit near 250, so the page is nearly white.

§8.6.6.3 states what a sample means there:

> A PDF reader shall treat each sample value as an index into the colour table and shall use the
> colour value it finds there.

This tree has **five** routes that turn an image sample into a colour — `unpack` for an
unfiltered or Flate stream, and one each for `DCTDecode`, `JPXDecode`, `CCITTFaxDecode` and
`JBIG2Decode` — and four of them obey it. `unpack` reads the index through `Decode`, whose default
range for an `Indexed` space is the index range itself; `CCITTFaxDecode` and `JBIG2Decode` go
through `unpack`; `jpx_samples_to_rgba` carries the rule as an explicit `scale` of one. The
`DCTDecode` route's `convert_channels` divided every sample by 255 before the lookup, which sends
the whole of a 256-entry table onto **entries 0 and 1** — the two the file happens to have written
as black. `convert_channels` now asks the same question the same way, and the page draws at 8.997.

**The defect is exactly as old as the code that was right about it.** The ledger's §8.6.6.3 row
records session 11 finding the *other* sentence of the same clause unapplied on the `unpack`
route — a `Lab` base scaled wrongly — and the sentence quoted above was never carried to the route
that arrived later.

## What moved, measured on the population that found it

All **seven** archives — the five new ones and 603's two — were re-ranked whole with the fixed tree
and diffed row by row against the ranking that named the defects:

- **`4851434.pdf`** −20.341 → **+0.127** (0.000 → 20.468 against 20.519 / 20.341 / 20.392).
- **`6327194.pdf`** +244.885 → **+0.058** (253.823 → 8.997).
- **Three more rows move, all of them the `Indexed` fix**, and 603's archives hold them:
  `0100408.pdf` +3.859 → **+0.102**, `0100681.pdf` +0.227 → **+0.063**, `7680631.pdf` −0.050 →
  −0.102. Each states `/Indexed` and `DCTDecode`; the last is 0.05 of 255 further from the lightest
  reference and is recorded rather than tidied.
- **The other 6994 rows are identical**, our panel and every reference panel alike, with one
  exception that is the instrument rather than the tree: `4851156.pdf`'s `poppler` panel was
  missing in the first run and present in the second, ours unchanged at 122.928 to the thousandth.
  A reference renderer is given 30 seconds and then killed.

So the measured reach of the two fixes over this population is **5 documents in 7000**, four of
them improved and none of them reachable by any gate: no document of the 974 states an `Indexed`
space over a `DCTDecode` image, and none carries a run-length stream that ends inside a run.

## What the head still holds, and what each is

Neither of the two deepest remaining rows is a fix this round could take, and both are stated with
their evidence so that a later one does not have to find them again.

- **`1653119.pdf` at −35.695** — a 4256×6258 scan, the deepest row in the chunk, refused with
  `Im0: JBIG2: too many symbol instances`. The refusal is **the dependency's**: `hayro-jbig2`
  0.3.0 caps a text region at a flat 10 000 instances with the comment "[a]rbitrarily chosen, but
  we need some limit to prevent timeouts", and this page's text region declares **13 264** — a
  full page of characters. **Upstream has already replaced that cap** with pdfium's heuristic
  (`segment_data_len × 32`, which is 852 096 here) in `hayro-jbig2: Use heuristic for maximum
  symbol instances (#1278)`, and the published 0.3.0 predates it. This is a *release* to take
  rather than a defect to fix, and `doc/todo/_image-codecs-and-the-sandbox.md` §7 now carries it
  beside the JPEG 2000 one.
- **`6327043.pdf` at −18.991** — an encrypted planning document whose aerial photograph is drawn
  as about 1700 `DCTDecode` images **one sample tall**, 2206 × 1 apiece, into a page 1191 device
  rows high. The layout is identical to `poppler`'s panel and the photograph is uniformly paler:
  ours 53.611 against 73.173 / 72.602 / 72.687. That is `doc/todo/11`'s subject — what an
  eight-bit raster does to a mark thinner than a pixel — measured on a real document for the first
  time, and it is a round of its own.
- **`6327765.pdf` at −17.158** is a `DeviceCMYK` JPEG under a `/DefaultCMYK` `ICCBased` space
  (§8.6.5.6), which is trap 9's shared-data family and not new.

## The positive tail is mostly one reference

Above +10, and excluding the two black pages, **every row of the 5000 is the same shape**: `poppler`
draws almost nothing where this tree, `mupdf` and `ghostscript` agree. `3252286.pdf` is the witness
opened side by side — a text page whose body font `poppler` renders as blank space while drawing
its rules and its header. **22 of 5000 documents, 0.44%**, are that shape by the arithmetic test
"poppler under a quarter of ours while both others are within 30% of it", and they occupy the whole
positive head down to +2.6.

It is worth stating as a fact about the *instrument*: a ranking against the **lightest** live
reference is by construction sensitive to one reference failing quietly, and on this population
that is more common than a defect of ours.

## The other end of the same list: what has no gap at all

**37 rows of 5000 produce no number**, because this tree drew nothing and no reference drew
anything either, or because ours drew nothing at all. They are outside a ranking by ink and were
opened by hand:

- **Crawl artefacts** — HTML pages saved under a `.pdf` name and truncated PDFs. `NoHeader` from
  this tree, nothing from `poppler` and `gs`, and a blank or error sheet from `mutool`, which is
  what puts a non-zero ink beside a file that is not a PDF.
- **One document this tree refuses on the clause**: `0300701.pdf` states `/R 5`, which
  §7.6.4.2's Table 21 does not define — `doc/todo/51`'s remainder — and `poppler` and `mupdf` open
  it. That refusal is a reading, not a defect, and it is already written down.
- **`0300856.pdf` at +240.314**, the one row in the positive head that is neither of the two
  fixes: all eight of its `/Contents` parts are `Damaged { Corrupt }` after about 114 KB of
  `FlateDecode` output apiece, the salvaged prefix lexes into 484 commands of nonsense operators,
  and what it draws is a black page. `poppler` and `mupdf` refuse the file outright and `gs` draws
  14.686. ADR 0343's prefix rule is deliberate and this is its cost at the extreme; the round
  records it rather than trading the rule away for one file.

## Consequences

- A `DCTDecode` image under an `Indexed` colour space draws its palette's colours.
- A `RunLengthDecode` stream that ends inside a run keeps every run before it.
- Two tests pin both, each calibrated by putting the defect back:
  `dct_components.rs::an_indexed_space_over_a_jpeg_reads_the_sample_as_an_index` (which fails with
  `(255, 0, 0)` — table entry 1, exactly what dividing by 255 selects — when the scale is restored)
  and `filter.rs::run_length_keeps_what_it_decoded_when_the_data_ends_inside_a_run`.
- `dct_components.rs`'s JPEG fixture is parameterised by component count rather than fixed at
  three, which is what let a greyscale frame be written at all.
- No gate number moves, and that is the measured form of the reach.

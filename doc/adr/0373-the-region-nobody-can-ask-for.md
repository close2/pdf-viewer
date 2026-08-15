# 0373 — The region nobody can ask for, and the decode nobody had counted

Date: 2026-08-15 (session 538)
Status: accepted

## Context

`doc/todo/46` — deleted by this round, so every reference to it below is to a file whose argument
is now this one — was opened by the project owner on reading session 396's reduced-resolution work:
*"Can't we do better, like only decoding part of the jpeg2000?"* The item's own answer was that
the two mechanisms answer different questions — the resolution progression answers *zoomed out*
(ADRs 0233, 0321), and tiles, precincts and packets would answer *zoomed in*, where a
211.9-megapixel scan magnified to 1:1 puts about half a percent of itself in a window. It also
said what a round taking it owes first, and in what order:

> **A census.** Thirty corpus codestreams; how many state more than one tile, how many state a
> precinct partition, and what progression order each uses. That decides whether this is one
> file's problem or a population's.

This round took the census, and the census decided the round.

## The instrument

`crates/pdf-model/examples/image_region_census.rs`, which ships. It has two sides because the
question has two denominators, and a third section for the codestreams themselves:

- **stated** — every image `XObject`'s `/Width`, `/Height` and `/Filter` chain, in every object
  of the file, decoding nothing. The denominator for *how many large images exist*.
- **drawn** — page one interpreted, with the placement read out of the display list, because ISO
  32000-2 §8.9.5.1 puts every image on the unit square and how large it is drawn is the `Do`'s
  transform and nothing in the dictionary. The device grid comes from
  `pdf_render::Grid::for_placement`, scaled — the renderer's own function rather than a
  restatement of it, so the census cannot disagree with a backend about what "at device
  resolution" means.
- **JPEG 2000** — ISO/IEC 15444-1 A.5.1's SIZ and A.6.1's COD out of each `JPXDecode`
  codestream: the tile partition, whether `Scod` states a precinct partition, the progression
  order and the decomposition levels. Cross-checked against `opj_dump` on `S2.pdf`'s codestream
  (`tw=2 th=2`, `csty=0`, `numresolutions=6`) and against `doc/todo/46`'s own independently
  taken reading of `issue19517.pdf`.

What a magnified view *needs* is, per axis, `min(samples, device) × min(1, viewport ÷ device)`;
what is decoded is the whole raster. The ratio is quoted at 64×, `viewer_core`'s `ZOOM_RANGE`
ceiling, which is deliberately the strongest case the proposal can be given: `needed` peaks
where the image exactly fills the window and falls away on both sides of that, so no zoom a
reader is likelier to use makes region decoding look better.

## What the census found

Three populations, each with its own invocation and its own denominator:

| | documents opened | images ≥ 64 Mpx stated | drawn on page one ≥ 8 Mpx | of those, JPEG 2000 |
|---|---|---|---|---|
| pdf.js | 964 | 4 | 7 | 0 |
| `doc/corpora` + `doc/corpora-own` | 489 | 0 | 14 | 1 |
| SafeDocs, a 1-in-10 sample | 6622 | 20 | 374 | 5 |

**The large-image population is real and it is not JPEG 2000's.** In the SafeDocs sample 1804
rasters of a megapixel or more are drawn on page one, 1783 of them wasting sixteen decoded
samples or more per sample a magnified window shows, 8.54 × 10⁹ samples decoded and never seen.
Of the 374 at eight megapixels or more, **five are JPEG 2000**; the largest groups are
`DCTDecode` (228), `CCITTFaxDecode` (65), `FlateDecode` (30) and `JBIG2Decode` (26) — scans of
paper, with the remainder chains of two of those. For those,
"decode the region" means decode it all and keep less, which is a different construction from
the one this item proposed.

**And where JPEG 2000 is large, it states neither mechanism.** Over the SafeDocs sample's 10 485
codestreams: 5790 state more than one tile, **none states a precinct partition**, and the
progression order is RLCP 9774 times to LRCP's 711. Sliced by size, the picture inverts:

| codestreams | ≥ 1 Mpx | ≥ 8 Mpx |
|---|---|---|
| total | 2088 | 125 |
| more than one tile | 1686 | **4** |
| a precinct partition | 0 | 0 |

Of the 125 codestreams big enough for a region decode to be worth anything, **121 are a single
tile with no precinct partition**, and the largest codestream in the whole sample is 15.4 Mpx.
The four exceptions — `0546350.pdf` twice at 2550×3300 in 10×13 tiles, `6050010.pdf` at
3050×2646 in 12×11, `6327910.pdf` at 3291×3166 in 13×13 — are 8 to 10 megapixels, which is a
decode of well under a second. The corpora agree: pdf.js's 28 codestreams are 17 tiled and 4
with precincts, all of them small, while its one image that provoked the item at all —
`issue19517.pdf`, 12608×16806 — is **one tile, no precincts, LRCP, 5 levels**, exactly as
`doc/todo/46` read out of it by hand.

## Decision

### 1. The region decode is refused, with the number

Not built. The population it serves is *four codestreams in a 6622-document sample*, none of
them larger than ten megapixels, and the one file in this project's corpora that is large enough
to want it has no tile partition and no precinct partition — so on the witness the construction
would have to walk the whole codestream, parse every packet header (tag trees are cumulative) and
skip only the arithmetic decoding, per view, where the tree today decodes once and keeps the
result. `doc/todo/46` predicted that shape and called it the discouraging case; the census says
it is the *population's* shape and not one file's.

The refusal is about the region and not about the item's subject: the zoomed-out half is
implemented and measured (ADR 0321), and this decision leaves it exactly where it is.

### 2. What would change it, stated so that it can be checked rather than argued

A JPEG 2000 codestream of **64 megapixels or more that states a tile partition or a precinct
partition**, in a document somebody reads. `image_region_census` is the command that answers it,
the largest codestreams are what it prints, and today the largest anywhere in reach is 212 Mpx
with one tile. The second thing that would change it is a decoder API to build against:
`hayro-jpeg2000` has `DecodeSettings::target_resolution` and nothing shaped like
`opj_set_decode_area`, so this would be a fork branch before it is a viewer change
(`doc/JPEG2000_FEEDBACK.md` §9's route).

### 3. What the census found instead, which is larger than what it refused

`22060_A1_01_Plans.pdf` — one page, fourteen image `XObject`s, all `DCTDecode` — draws rasters of
one 2480×2630 grid **36 times**, and `crate::image::decode_parts` is called at each `Do`.
Interpreting page one takes **3.09 / 3.16 / 3.14 s** over three runs (`examples/open_one`, gates
profile), and a temporary probe around the call site attributes **3.23 of the 3.24 s** to
`decode_parts` over **72 calls**: four ICC-based 2480×2630 images, each with a `DeviceGray`
`/SMask` of its own grid, decoded **nine times each** at 0.73 to 0.88 s per image in total. Eight
of every nine decodes reproduce a raster the interpreter has already produced — about **2.9 of
the 3.1 seconds** — and a soft *mask* has been cached since ADR 0210 while the base image has
not.

**The first reading of that page said 21.3 s, and it was the instrument rather than the page**:
the SafeDocs census was running beside it, and `doc/habits.md`'s *wall-clock benchmarks lie under
load* is exactly what a seven-fold error looks like. The three figures above were taken with the
machine otherwise idle, and the probe's own attribution — a sum over 72 spans that agrees with
the whole — is what makes them more than a stopwatch.

That is a bigger number than any region decode would have saved on this corpus, it was found by
this round's own instrument, and it is not this item: it is a cache with a soundness question of
its own — `decode_parts` takes the resource dictionary, the fill colour and the compositing half
as well as the stream, and `shading::Cache`'s key needed the colour space in it for exactly that
reason. It is written down as `doc/todo/47` with the measurement attached rather than taken
here, because a round that refuses one construction on a census should not build another one
without its own.

## Consequences

- `doc/todo/46` is deleted: its census is taken, its question is answered, and the argument lives
  here. The zoomed-out half it also carried is ADR 0321's and stays implemented.
- §7.4.9's ledger row records that NOTE 2's *location* progression was measured and refused, with
  the count that refused it, so a later round finds the number rather than the idea.
- Nothing drawn moves. No shipped code changed this round — the census is an example — and the
  corpus, oracle, quorra, text, dates, XMP and JPEG 2000 gates are all as they were.
- The census is repeatable over any set of paths, which is what makes the refusal checkable: the
  population it rests on is a command's output rather than a sentence in a document.

## Alternatives considered

**Build the region decode anyway, for `issue19517.pdf`.** It is the file that provoked the item
and it *is* served today — at a reduced resolution level, magnified past 1:1 (ADR 0321). What a
region decode would add is sharpness above 0.25× magnification on one corpus file, at the price
of a per-view codestream walk and a cache keyed by identity *and* rectangle. Refused on the
census: one file is not a population, and the vocabulary to express it (`ImageSource::AtDeviceScale`
one axis further) stays available if one arrives.

**Extend the census to every page rather than page one.** Page one is a sample of placements and
the stated side says how much it misses — 93 386 image dictionaries against 20 406 rasters drawn
on the sampled pages. Interpreting every page of 6622 documents is hours, and it would sharpen a
number that is already three orders of magnitude from the decision boundary.

**Run the whole SafeDocs cache rather than a tenth of it.** 65 944 documents at the observed rate
is most of a day, and the sample's own JPEG 2000 counts — 10 485 codestreams, zero precinct
partitions — are not close enough to any threshold for ten times the population to move the
decision.

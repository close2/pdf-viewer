# 0585 — The chain an image ran four times

Status: accepted
Date: 2026-08-24
Session: 712

## Context

`doc/todo/41`'s remainder was two lines, and this ADR is about the first: **`image_stream` has no
memo at all.** ADR 0317 built the decoded-stream memo, ADR 0437 put the *refusal* in it, and the
image route stayed outside both — on a reason the todo file states in one clause, "decoding outside
this one by construction (a codec's bytes are not a filter chain's)".

That clause is true about the *codec* and false about everything in front of it. Table 5 lets
`/Filter` be a chain whose last entry is an image codec — `[/ASCIIHexDecode /JBIG2Decode]` is
§7.4.7's own worked example, `[/FlateDecode /DCTDecode]` occurs in the corpus — and
`Document::image_stream` runs every stage before that codec and hands the codec's own bytes back.
Where there is **no** codec at all, "everything before it" is the whole chain, and the call inflates
the samples themselves. That is an ordinary §7.4 filter chain over an ordinary buffer, which is
exactly what the memo holds for every other stream in the file.

**The price was re-derived before anything was built, and the re-derivation changed what got
built.** `crates/pdf-model/examples/image_prefix_census.rs` is the instrument: over the
964-document pdf.js corpus, **2420 of 2997 image `XObject`s run a filter before the codec, producing
467.7 MB per pass over the corpus's images**; over ISO 32000-2 itself, 307 of 360 do, producing
145.7 MB. So the population is most images rather than a corner.

The multiplier was the surprise, and it was found by reading `pdf_model::content::image::draw`
rather than by profiling. **One `Do` asks `Document::image_stream` four times:**

- `image::contradicted_frame` — §7.4.8 puts a JPEG's dimensions in the codestream, so a dictionary
  that contradicts them is reported. It decoded the chain and *then* asked whether the codec was
  `DCTDecode`.
- `image::short_of_its_grid` — §7.3.8.2 infers an image's extent from its own dictionary. It
  decoded the chain and then returned `None` for every stream that has a codec.
- `image::ccitt_bound_below_its_height` — §7.4.6 Table 11's `/Rows` under `/EndOfBlock false`. Same
  shape again, for a third codec.
- `image::decode_parts`, for the samples, on a `RasterCache` miss.

`RasterCache` (ADR 0374) does not cover the first three: it is consulted *after* them, and it dies
with the page's display list, so an image drawn on forty pages is decoded on forty pages whatever it
does. **A codec-less `/FlateDecode` image was therefore inflating its whole sample buffer four times
per `Do`.**

## Decision

Two changes, and the measurement below is why both are here rather than one.

**1. Table 5's codec is read rather than run.** `Document::image_codec` answers which image codec a
stream's `/Filter` ends in, decoding nothing; `Document::codec_position` is the one place the "only
the last entry may be a codec" rule lives, shared with `image_stream`. The three reports above ask
it first and decline before spending a decode on an image that is not theirs.

**2. `Document::chain_over` is the one place a §7.4 chain is run**, and the image route goes through
it. `decoded_under` asks for a stream's whole `/Filter`; `image_stream` asks for the part in front
of the codec. Same memo, same budget, same eviction, same liveness invariant — no second cache and
no second constant.

### What the key includes, and why each part is necessary

The key is `DecodedStreams`' existing one and gains nothing, which is the point of routing through
`chain_over` rather than building a second table:

- **the allocation** — the address *and* length of the encoded bytes, with the entry holding the
  `Arc` alive. The pin is the whole soundness argument (ADR 0317): a freed buffer's address is
  handed to the next allocation, so an entry that named an address it did not keep alive could
  answer a lookup for a different stream.
- **the chain, with each stage's `/DecodeParms`**, compared exactly on every hit. This is what makes
  the image route safe beside the other one: an image's prefix `[/ASCIIHexDecode]` and the whole
  chain `[/ASCIIHexDecode /DCTDecode]` are two different questions about one buffer, and answering
  either from the other's entry would hand back a codec's input where its output was asked for.
  `an_images_prefix_is_not_the_whole_chains_answer` is that test.
- **the bound a refusal was reached under** (ADR 0437), unchanged.

What the key deliberately does **not** include is everything a *raster* depends on — the colour
space, the decode array, the mask, the fill colour, the conversion, the scale. It does not need
them, and that is the reason this memo is cheap where a general image cache is not: `image_stream`
answers a question about §7.4 alone. **The samples' own cache is `image::RasterCache`, whose key does
carry those four things**, and this change touches neither it nor its key.

**One entry per allocation, so a second chain over one buffer replaces the first.** That is
`DecodedStreams::held`'s key rather than a new decision, and no shipped path asks both of one
stream: an image's bytes arrive by the image route, a font program's and an `ICCBased` profile's and
a content stream's by the other. `pdf_model::thumbnail::significant` builds the one second `Stream`
over another's `data` and leaves `/Filter` untouched, so its chain is *equal* and it shares the
entry.

**An empty prefix is not memoised**, for the reason `decoded_under` returns early on an empty chain:
nothing ran, and an entry holding a second `Arc` to the bytes it is keyed by would charge the budget
twice for a decode that never happened.

## The measurement

`callgrind` under `RAYON_NUM_THREADS=1`, because the machine carried three other rounds throughout
and a wall clock taken there is a measurement of the neighbours — the first attempt at this A/B gave
`before` 3.40 s, 4.45 s, 6.90 s on one binary while the load went from 3.2 to 11.0, which is not a
number. Every count below is instructions, and the four arms were built as four binaries and run
against each other rather than rebuilt between runs.

**A whole-document cold sweep — ISO 32000-2's 1023 pages, `viewer-core/examples/find_cost`:**

| arm | instructions | against `before` |
|---|---|---|
| `before` | 38 011 452 243 | — |
| the codec read rather than run, alone | 37 117 065 563 | **−2.35%** |
| the memo, alone | 37 228 012 960 | −2.06% |
| both | 37 152 970 322 | −2.26% |

**Both is worse than the reordering alone, and the reason is the thing to keep.** Over the first 200
pages of that sweep the decoded-stream cache goes from **306 entries and 4 194 227 bytes held, 507
misses and 201 evictions** to **13 entries and 2 574 504 bytes, 599 misses and 582 evictions**: the
image entries are two orders of magnitude larger than the content streams and font programs that
cache was holding, and they push them out. ISO 32000-2 has 307 images over 1023 pages and repeats
**none** of them across pages, so the memo pays the displacement and collects nothing.

**A document that repeats an image — twenty pages, one 512×512 `/FlateDecode` image drawn once
apiece, which is the population `doc/todo/41` names as the reader's:**

| arm | instructions | against `before` |
|---|---|---|
| `before` | 1 863 750 280 | — |
| the codec read rather than run, alone | 1 292 444 973 | −30.7% |
| the memo, alone | 735 463 726 | −60.5% |
| both | **735 431 933** | **−60.5%** |

So the two halves are not redundant and neither is the whole answer: the reordering takes the three
reports' wasted decodes off *every* image page, and the memo takes the repeat off a document that
has one. The memo's cost where nothing repeats is **0.10% of a thousand-page sweep**; its worth
where something does is **43% on top of the reordering**. That is the trade, stated in both
directions, and the LRU budget is what arbitrates it.

**Single pages, `callgrind_interpret`,** both halves together:

| page | before | after | |
|---|---|---|---|
| `images.pdf` 1 — ten images, 53.2 MB of prefix | 4 761 378 441 | 4 511 355 330 | −5.25% |
| `issue12963.pdf` 1 — twenty-four images, 52.2 MB | 1 343 821 836 | 1 272 468 332 | −5.31% |
| `issue19971.pdf` 1 | 1 111 904 159 | 1 110 514 549 | −0.13% |
| ISO 32000-2 p101 ×50 — no image at all | 1 293 589 573 | 1 293 607 160 | +0.001% |

The last row is the control: a page that draws no image pays 17 587 instructions across fifty
interpretations for a `Vec` that is never built.

## Consequences

- **What the cache holds changes shape**, and a round measuring it should expect that: fewer, larger
  entries, more evictions, fewer bytes held. The numbers above are the before and after on 200 pages
  of ISO 32000-2, and `viewer-core/examples/find_cost … split` is what prints them.
- **An inline image adds an entry the address will never name again.** §8.9.7's stream is built at
  every `BI`, so its allocation dies with the draw — the pin keeps the entry sound, and the LRU is
  what keeps it bounded. This is ADR 0399's shape for a different cache, and the difference is that
  this one is a hash map with a byte budget rather than a linear probe, so a page full of `BI`s
  costs it evictions rather than time.
- **`image_stream` can now be asked cheaply, and the three reports show what that is for.** A fourth
  report about a fourth codec would cost an `image_codec` call and nothing else.
- `doc/todo/41`'s first remaining line is closed. Its second is ADR 0586.

# 0374 — The raster a page decodes thirty-six times, and the key that lets it decode once

Date: 2026-08-15 (session 539)
Status: accepted

## Context

`doc/todo/47` — deleted by this round, so its argument is now this one — was written by session 538
with the measurement that opened it (ADR 0373). `image::decode_parts` runs at **every** `Do`, so a
page that draws one image `XObject` thirty-six times decodes it thirty-six times.
`22060_A1_01_Plans.pdf` is that page: four 2480×2630 `DCTDecode` photographs in an ICC-based space,
each with a `DeviceGray` `/SMask`, drawn nine times apiece. A temporary probe attributed **3.23 of
the 3.24 seconds** its page one took to that one call over 72 calls.

A soft mask has been memoised since ADR 0210 and the base image has not, and the item said why the
base is the harder one: `decode_parts` takes more than the stream. The item left the **bound** as
the open question and named the two candidates — a least-recently-used cache by size, or a cache of
the last raster only, "which serves the witness's nine-in-a-row exactly and costs one entry".

## What the key claims, which is the decision this ADR is mostly about

ADR 0317's lesson is that a key is a claim, and this one has more inputs than the decoded-stream
memo did. `image::RasterCache`'s entry says: *a `Do` that agrees with me on four things would have
produced these samples.* The four, with the clause that makes each one an input:

- **The stream**, by the identity of its allocation — and the entry **holds** the `Arc`. That is
  ADR 0317's pin, and it is not a copy of anything: a freed buffer's address is handed to the next
  allocation, so an entry naming an address it does not keep alive can answer a lookup for a
  different stream. Holding it makes the collision impossible rather than unlikely, and it is what
  lets an inline image — a fresh `Arc` at every `Do`, built by `content::run` — share the table
  safely while never hitting in it.
- **The resource dictionary in force.** §8.6.5.1 resolves a colour space named `/CS0` through it,
  and §8.6.5.6's `/DefaultGray`, `/DefaultRGB` and `/DefaultCMYK` reach even the device names, so
  one stream under two resource dictionaries is two pictures. Compared by value: two equal
  dictionaries name the same objects of the same document.
- **The fill colour.** §8.9.6.2's stencil "does not specify colours; instead, it designates places
  on the page that should either be marked with the current colour or masked out", so the same
  stencil under two fill colours is two rasters. Held as bit patterns, which is conservative in the
  one direction that matters — `-0.0` and `0.0` paint alike and miss.
- **What the samples are composited into.** §11.4.7's four-component page is interpreted twice,
  once per half (ADR 0262), and a `/Luminosity` mask group paints in the ink §10.4.2.3 weighs
  rather than in colour (ADR 0220). This one changes *within* an interpretation, which is why it
  cannot be left to the cache's lifetime the way the fifth input is.

The fifth input is the `Document`, and it is out of the key because the cache is a field of the
`Interpreter` and cannot outlive one interpretation of one page — the same lifetime `MaskCache`
and `shading::Cache` already have.

**Each component has a test, and each test was confirmed to fail when its component is dropped.**
`crates/pdf-model/tests/image_reuse.rs`, run four times against a mutated key:

| mutation | test that failed |
|---|---|
| `resources` out of the comparison | `a_raster_is_not_shared_across_resource_dictionaries` |
| `fill` out of the comparison | `a_raster_is_not_shared_across_fill_colours` |
| `into` out of the comparison | `a_raster_is_not_shared_across_compositing` |
| the pin replaced by a bare `*const Stream` | `a_stream_cannot_inherit_the_raster_of_one_whose_allocation_it_reuses`, on **round 2 of 64** |

The mutation tests are what make them tests rather than decoration, and the last row is ADR 0317's
"a pin is not a copy" reproduced one crate over: the second arm of each is compared against a
*fresh, uncached decode*, so a wrong answer is the wrong picture rather than a suspicious pointer.

## The bound, and the measurement that chose it

Every `Do` on an image over the pdf.js corpus's page ones was recorded — 4789 calls in 228
documents, each with its key, the bytes the entry charges and the bytes of base raster the decode
produced — and a least-recently-used simulation replayed that sequence at each budget. This is
ADR 0317's method, and the second column is the work:

| budget | base raster decoded | decode calls | evictions |
|---|---|---|---|
| none — the code before this | 1796.8 MB | 4789 | — |
| 8 MiB | 1635.7 MB | 4544 | 124 |
| 16 MiB | 1635.7 MB | 4544 | 105 |
| 32 MiB | **781.5 MB** | 4466 | 13 |
| 64 MiB | 781.5 MB | 4466 | 4 |
| 128 MiB | 781.5 MB | 4466 | 1 |
| unbounded | 781.5 MB | 4466 | 0 |

**The knee is at 32 MiB**, and it is where it is because the repeats that cost anything are a
handful of large photographs: the witness's four rasters are 26.1 MB each, so 8 and 16 MiB save
almost nothing — the entry does not fit at all. The median entry over the corpus is **1 KB**, and
7 of the 4466 exceed 32 MiB.

**`RASTER_BUDGET` is 64 MiB**: the knee, doubled, and the doubling buys a shape the corpus does not
contain and a page can state as easily as the witness's — two large images drawn alternately. At
32 MiB two 26.1 MB rasters thrash and every `Do` decodes; at 64 MiB both are held. What being wrong
in that direction costs is bounded by the memory section below, and what being wrong in the other
direction costs is a page that gets nothing at all.

**The item's other candidate was simulated on the same sequence and is 2.9% short**: a cache of the
last raster only decodes 804.2 MB over 4534 calls. It serves the witness exactly — the witness's
nine `Do`s are consecutive — and it is the interleave it cannot serve, which is one `Do` order away
from the page we have.

### One document says why the two byte columns are kept apart

`issue16263.pdf` repeats a `Do` forty times over an entry charged 18.9 MB and decodes **no base
raster at all** for it. That 18.9 MB is §11.6.5.2's packed mask, which `MaskCache` has shared since
ADR 0210; counting it as work saved would have credited this cache with a saving somebody else
already made. Its peak resident memory is unmoved by this change (41.4 MB → 41.7 MB), which is what
says so. A first pass of this analysis charged the mask to the saving and read a 2763.8 MB baseline;
the correction is why the table above reports the base raster.

The entry still *charges* the mask against the budget, deliberately: over-charging bounds what a
page can hold, while under-charging would let a mask nobody else holds — one written inline, which
`MaskCache` declines to remember — go unbudgeted.

## What it costs and what it buys

Wall clock is not evidence on this machine: the load average was 130 to 140 throughout the session,
five other rounds running. So the numbers are callgrind's, under `RAYON_NUM_THREADS=1` for
`doc/habits.md`'s reason, two binaries built from one tree an hour apart:

| `examples/callgrind_interpret` | instructions |
|---|---|
| `22060_A1_01_Plans.pdf` page 1, before | 58 665 139 034 |
| `22060_A1_01_Plans.pdf` page 1, after | **6 544 740 674** — **−88.8%** |
| ISO 32000-2 page 101 ×50, before | 1 235 040 931 |
| ISO 32000-2 page 101 ×50, after | 1 234 947 809 — −0.008% |

The control is a page of text and vector graphics with no image on it at all, and it moves by less
than a hundredth of a percent: what a page with no `Do` on an image pays is a `Vec` that is never
touched.

**Peak resident falls, which is the direction a cache does not usually move**, `ru_maxrss` of the
same interpretation, three runs an arm:

| | before | after |
|---|---|---|
| `22060_A1_01_Plans.pdf` | 1031.8 / 1032.1 / 1031.8 MB | **235.6 / 235.5 / 235.8 MB** |
| ISO 32000-2 page 101 ×50 | 39.4 / 39.6 MB | 39.2 / 39.8 MB |
| `issue16263.pdf` | 41.4 MB | 41.7 MB |

The reason is that **the cache spends no memory the display list was not already spending**: every
raster it holds is the raster a `Command::Image` in the list under construction is holding, because
the same `Arc` goes into both. Before this, the nine `Do`s of one photograph put nine separate
26.1 MB allocations into the list; 940.6 MB of that page's list was nine copies of four pictures.
The one raster that can outlive its command is §10.5's transferred one, which is a copy by
construction — 1 of the 974 corpus documents states a non-identity transfer — and the budget is what
bounds that case rather than an argument that it cannot arise.

## Nothing moved, and the one thing that did

`examples/display_list_digest` over the corpus, both arms, same `pdf-sandbox-worker` on disk:
**964 documents opened, 958 first pages interpreted, `md5 f9eb6ec03bdee3e9d4edc60e82c508e4`
byte-identical**. Every gate is unchanged, and `crates/pdf-model/tests/image_reuse.rs`'s sixth test
is trap 5's half of that claim: a page drawing one short-of-its-grid image twice reports exactly
what a page drawing it once reports, because every report about an image is made from its
*dictionary* before anything is decoded.

**One ratchet moved, and it is a hole closed rather than a difference.**
`render-quorra::corpus`'s `REFUSED_AT_FOUR` held `22060_A1_01_Plans.pdf` because at 4× that page
held 522 014 748 resident *resource* bytes and the next upload would have taken it to 548 104 348
against the 536 870 912 `max_resource_bytes` default. Those bytes were 72 uploads of 8 distinct
rasters. The page now draws at 4×, the list is down to four, and the lesson is the one that
comment already argued from the other end: **a refusal that is arithmetic against a byte budget is
a question about who is spending the bytes**, and this time the answer was upstream of the backend
entirely. `max_resource_bytes` was not raised — a budget raised to admit one page is a budget
chosen by that page.

## What was considered and not done

- **Keying on the `XObject`'s object number**, which is what `MaskCache` and `shading::Cache` do.
  It would work for every `Do` that reaches `draw_xobject` and for none of §8.9.7's inline images,
  and it would need `draw_image` to carry a reference its callers have already resolved. The `Arc`
  identity is available at the call site, needs no plumbing, and — with the pin — is the stronger
  claim, since it says *these bytes* rather than *this name*.
- **Caching a decode that failed.** `shading::Cache`'s reason, unchanged: an error is rare, the
  caller reports it, and remembering one would mean deciding whether the error is a property of the
  object or of the moment.
- **A cache on `Document` rather than on the `Interpreter`**, which would survive a page turn. It
  would need the raster's inputs to include everything a resource dictionary can say — the key
  above holds a *value*, and holding one per document is a memory question this measurement does
  not answer. The per-page cache is what the witness needs; a per-document one is a different item
  with a different denominator, and nothing measured here asks for it.
- **A map instead of the `Vec`.** A probe that misses costs one pointer comparison per entry and
  the byte budget keeps the entry count small; a map would need the key *constructed* to probe,
  and constructing it means cloning a resource dictionary at every `Do`.

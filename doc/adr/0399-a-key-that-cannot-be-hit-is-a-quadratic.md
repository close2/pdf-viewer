# ADR 0399 — A key that cannot be hit is a quadratic

Status: accepted, 2026-08-17. Session 564. Takes the latency finding session 560 recorded for
`doc/todo/10` and `doc/todo/16` (ADR 0395's round), and amends §8.9.5's, §8.9.6's and §8.9.7's
ledger rows. The cache it is about is ADR 0374's, and this is the second half of that decision
rather than a correction of it.

## The two documents, and what they are

Session 560's `page` fuzz run left two `timeout-` artefacts and said they were not the fuzzer's
noise: both read their own cross-reference table, and in a release build one took **2 m 13 s** and
the other **35.6 s**. They are byte-identical to two SafeDocs members of the `CC-MAIN-2021-31`
archive — `4851530.pdf` and `3375489.pdf` — which is what makes them documents rather than
inputs. Neither is committed and neither may be: `.gitignore`'s licence position and
`doc/third-party-data.md` keep somebody's web pages out of this history, so every fixture below is
generated.

Both are `CubePDF` output through `iTextSharp 5.5.13`, and both draw a hatching the same way:

- **A**, a bus timetable, 126 251 bytes, two pages. Its first page states **3 198 pattern fills**
  of thin rectangles — the grid rules of the timetable — through two `/PatternType 1`
  `/PaintType 2` patterns whose `/BBox` is `[0 0 1 1]` with `/XStep` and `/YStep` of one. The cell's
  whole content is fourteen operators, and one of them is §8.9.7's inline image: an 8 × 8 one-bit
  stencil.
- **B**, 202 512 bytes, one page, 2.73 MiB of `/Contents`. It states **12 092 inline images
  directly**, of which **five are distinct**, plus 56 pattern fills of the same shape.

So neither is a bomb. Each is an honest drawing whose cost the file states: A's 3 198 fills over
the lattice `/XStep 1` describes come to about 327 000 cells, which is where `MAX_OPERATIONS` stops
it, and B's 56 fills plus its own 12 092 `BI`s come to about 178 000 image draws inside every bound
the tree has.

## The attribution, which is not the tiling

`examples/callgrind_interpret` under `RAYON_NUM_THREADS=1`, one page each:

| | total Ir | first line of the profile | second |
|---|---|---|---|
| **A** | 330 490 549 519 | `Interpreter::draw_image` **240 239 444 483 (72.69%)** | libc `memmove` **76 581 243 980 (23.17%)**, 64 608 calls, all from `draw_image` |
| **B** | 71 859 030 215 | `Interpreter::draw_image` **63 568 580 427 (88.46%)** | `image::unpack` 1 049 406 031 (1.46%) |

`image::unpack` is where the samples are actually decoded, and on A it is **1 650 450 240, half a
percent**. So the reader was spending two hundred times as long *finding* the decode as doing it,
and both lines above are the same defect twice:

- **`image::RasterCache`'s probe is a linear scan** — `entries.iter().position(…)`, one
  `Arc::ptr_eq` per entry, by design and measured that way in ADR 0374.
- **An inline image added one entry per draw and could never hit one of them.** The cache names a
  stream by the address of its allocation, which is sound and cheap for an `XObject` the resource
  dictionary hands out; `content::run` builds §8.9.7's stream afresh at every `BI`, so its address
  named one *draw*. ADR 0374's own doc comment said so — "it is what lets an inline image … share
  the table safely **while never hitting in it**" — and read that as a harmless property.

It is not harmless, and the arithmetic is the whole finding: *N* draws that each insert an entry
nothing can find cost *N*²/2 probes. A's 327 000 give 5 × 10¹⁰, which is the 240 G. The second line
is the same *N* one layer down: an 8 × 8 stencil charges 256 bytes against `RASTER_BUDGET`, so the
list reached 64 MiB at 262 144 entries and then evicted on **every** further insertion —
`entries.remove(0)`, a `memmove` of the whole tail, 64 608 times.

**A resource image cannot do this**, which is why the cache had been linear on every page anybody
had measured: its allocation recurs, so it adds one entry however often it is drawn. The population
of entries is meant to be the *distinct images the page draws*, and for one kind of stream it was
the *draws*.

## The reading, and why the fix is not a number

§8.9.7's first paragraph is why an inline image has no address that recurs:

> This type of image shall be defined directly within the content stream in which it will be
> painted rather than as a separate object.

There is no object for §7.8.3's resource dictionary to hand out twice. What recurs is the
*content*, and §8.9.5's own definition of an image is its dictionary and its samples — which is
exactly what `image::decode_parts` reads. So the fix is to name the thing by what it is:

`image::StreamIdentity` has two variants. `Allocation` is what an `XObject` gets, unchanged in
every respect including the pin that ADR 0317 argued for. `Content(u64)` is what §8.9.7's image
gets: a digest of its samples and its dictionary's length, narrowing the search, with the content
compared **exactly** beside it — the construction `pdf_render::DisplayList::add_clip` already uses
for a clip found by digest, and for the same reason. A collision costs a decode and never an answer
about a different image.

`image::NamedStream` carries the stream and its name together, because the whole of this defect was
a caller that had the stream and did not have the name; its two constructors are the only way to
build one, and each says which route the stream came by.

**Nothing arbitrary was introduced and nothing arbitrary was moved** — `doc/todo/10` §6's first
rule. `MAX_TILES` is 4096 as before, `MAX_OPERATIONS` is four million as before, `RASTER_BUDGET` is
64 MiB as before. What changed is which streams the population of entries counts.

## What it is worth

Release build, `pdf-retrieve page <doc> 0`, and `callgrind_interpret` in one sitting per arm:

| | before | after |
|---|---|---|
| **A** wall clock | 122.7 s (112.9–122.7 over four samples) | **0.93–1.26 s** |
| **A** peak resident | 657.6 MB | **129.8 MB** |
| **A** instructions | 330 490 549 519 | **10 663 108 878** (**−96.77%**) |
| **B** wall clock | 12.0–14.3 s | **0.47–0.63 s** |
| **B** peak resident | 383.6 MB | **75.4 MB** |
| **B** instructions | 71 859 030 215 | **6 295 686 734** (**−91.24%**) |
| ISO 32000-2 page 101 × 50, which draws no image | 1 216 998 583 | 1 216 999 004 (**+0.00003%**) |

The control is the point of the last row: this costs a page with no inline image on it 421
instructions in 1.217 G, because the digest is computed at `BI` and nowhere else.

After the change neither document's profile mentions `draw_image` at all. What is left is the
ordinary interpreter — `Lexer::next_token` at 16.60% of A and 14.16% of B, the dispatch, `get_key`,
`add_clip` — and A still reports `MAX_OPERATIONS`, because four million operators is what its
327 000 cells state.

## Output identity

The whole change is a memo, so nothing about any page may move, and that is checked as bytes rather
than as a verdict:

- `examples/display_list_digest` over **1222 first pages** of 1231 opened documents — the pdf.js
  corpus and the four submodule corpora — **byte-identical** between the arms, both run with the
  same `pdf-sandbox-worker` on disk.
- `examples/readback` over **all 1023 pages** of ISO 32000-2, concatenated: 2 730 201 bytes,
  `sha256 ed074b1c…`, `cmp` silent. That is the same digest session 500 recorded.

So no pixels move, and no quorra `gpu` lane or `doc/todo/00` step 7 ink sweep is owed — proven
rather than asserted, which is ADR 0379's precedent for the same claim.

## The crasher the fix uncovered, and the clause that bounds it

The seeded `page` run this round owes wrote a **`crash-`** artefact, and it is not this round's
change: `AddressSanitizer: stack-overflow` down a recursion of `decode_parts` →
`apply_explicit_mask` → `decode` → `decode_parts`, two hundred and fifty frames of it. An image
whose §8.9.6.3 `/Mask` names an image mask that states a `/Mask` of its own descends for ever, and
`CLAUDE.md` principle 3 is what makes that worse than slow: unbounded recursion exhausts the
*stack*, which the confined worker's address-space ceiling cannot see and which Rust turns into an
abort rather than a report.

**The bound needs no constant, because the standard's is one.** Table 87 forbids it twice — of
`/ImageMask`:

> If this flag is true , the value of BitsPerComponent , if present, shall be 1 and Mask and
> ColorSpace shall not be specified

and of `/Mask` itself, "shall not be present for image masks" — while §8.9.6.3 makes the `/Mask`
"an image mask, as described in 8.9.6.2". So a mask's mask does not exist, and `explicit_entry`
returns `MaskEntry::Unusable`, which the interpreter already reports by name and the base image is
drawn unmasked.

**The same door was open on §11.6.5.2's side and half of it was already shut.** Table 143 says of a
soft-mask image's `SMask` "Shall be absent" and of its `Mask` the same; `soft_mask_entry` guarded
the first and not the second, so `apply_soft_mask` could reach `apply_explicit_mask` and descend
from there. Both are now `SoftMaskEntry::Unusable`, reported through `unapplied_soft_mask`.

The artefact itself is a mutation of a Common Crawl member and is therefore not committed either;
`tests/hostile_budgets.rs` states both shapes as generated fixtures, and each was confirmed to fail
with its guard removed. The artefact runs in **192 ms**.

## What this leaves, and what it does not claim

- **The probe is still linear in the entries**, and the entries are now the *distinct* images a page
  draws — the property a resource image always had. A page stating tens of thousands of *distinct*
  inline images would still be quadratic in them, at a rate three orders of magnitude below the one
  measured here (B's 12 092 draws are five distinct images). No document in reach exercises it, so
  it is written down in `doc/todo/10` rather than built.
- **`doc/todo/49`'s evidence that `MAX_TILES` bounds the wrong quantity was partly this defect.**
  That entry contrasts `7680183.pdf` — 42 282 tiles, 14.2 s — with `2760154.pdf` — 765 440 tiles,
  8.7 s — and concludes a count cannot rank cost. The conclusion still holds on its own argument
  (an empty cell at 0.89 µs a tile is four days at a trip count a file may state), but the *pair of
  numbers* was measured through a quadratic probe and a round re-running the survey should expect
  both to fall and the inversion to be smaller or gone. Amended there.
- **A `memmove` is still 15.17% of B**, now the display list's own growth rather than an eviction.
  That is a residue, not a defect, and it is named so that nobody re-derives it from the same
  profile.

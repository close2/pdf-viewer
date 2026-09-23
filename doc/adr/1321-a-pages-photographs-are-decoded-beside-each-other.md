# 1321 — A page's photographs are decoded beside each other, and the list waits for each

Session 1242. Status: **accepted, built.** Takes the item [ADR 1272](1272-a-page-turn-that-does-not-wait-for-the-photograph.md)
section 5 recorded and `doc/questions/A121` names as the road that needs no answer from the owner:
decoding a page's images in parallel with each other and with the rest of the interpretation,
without presenting anything earlier. Code: `crates/pdf-model/src/image/ahead.rs` (the table),
`crates/pdf-model/src/content/image.rs` (`plan_decodes_ahead`, `reachable`, `walk_ahead`,
`WalkAhead`), the miss path of `image::RasterCache::parts`, and the scope in `content::interpret_into`.

## 1. What was measured first

`crates/pdf-model/examples/image_decode_census.rs` over the pdf.js corpus's first pages: 958
pages, **761 name no image, 158 one, 39 two or more**, and on 17 of them the images' decodes
summed less the longest is at least a millisecond — the part a parallel decode could take off the
critical path. The large ones are photographs: `22060_A1_01_Plans.pdf` (four 2480 × 2630 `DCTDecode`
photographs with soft masks, drawn through §8.7.3 tiling patterns), `images.pdf` (five `FlateDecode`
images in one form), `issue1905.pdf` (eight images spread over three thousand lines of vector
content). So the population is small and the witnesses are not shaped alike, which decided where
the decode is issued.

## 2. The design

**The decode is issued ahead of the `Do`, and its result is awaited at the `Do`.** A `Do` that
meets an image whose decode is running waits for it; nothing reaches the display list before the
picture exists. The list is built on the interpreter's thread in content order as before, so it
stays a pure function of the bytes and the view state: an answer from ahead is
`image::decode_parts` called with exactly the inputs `RasterCache` would have passed — the stream,
the resource dictionary's §7.8.3 Table 34 `/ColorSpace` entry alone, the conversion — and it
answers a `Do` only where all three agree. The fill colour is left out because `decode_parts`
reads it only for §8.9.6.2's stencil, and no stencil is decoded ahead. Anywhere the inputs
disagree, the interpreter decodes for itself as it always did: wasted work, never another picture.
`tests/decode_ahead.rs` interprets one fixture on a one-thread pool and on a four-thread pool and
requires the two `Interpretation`s to be one value; `raster_golden` held 974, moved 0.

**Issued by a walk, not at the `Do`.** A decode issued at the `Do` that meets it cannot run beside
anything, and the bytes a `Do` could look ahead through are the reader's window, which reaches the
next image on none of the three witnesses: two draw theirs through forms or pattern cells, the
third three thousand lines later. So one pool task reads the page's content stream ahead of the
interpreter — tokens only, following `Do` into forms and `scn`/`SCN` into coloured tiling
cells — and offers each image it finds. It tracks the two state parameters that choose the
conversion, §8.6.5.8's intent and §8.6.5.9's `/UseBlackPtComp`, through `ri`, `gs` and `q`/`Q`;
§11.4's blending space it takes as the page begins, so an image inside a group with a `/CS` of its
own is decoded by the interpreter. It declines what §8.11 may hide: an image or a form stating
`/OC`, and anything inside a `BDC` tagged `/OC`.

**The pool is the one rayon already has**, inside a `rayon::in_place_scope` around the content
stream's run. The scope cannot end while a decode it started runs, so the table is closed first:
nothing queued is started after the content stream has been read, and a queued slot the `Do`
reaches first is taken back and decoded on the interpreter's thread, so the interpreter never
waits on a decode no thread has begun. A decode that unwinds leaves its slot abandoned, and the
waiter decodes for itself.

**The sandboxed codecs are not decoded ahead.** `pdf_sandbox::Sandbox` serves every caller through
one worker behind one mutex, so a `JPXDecode`, `JBIG2Decode` or `CCITTFaxDecode` request issued
ahead would queue in front of the interpreter's own rather than beside it, and hold a pool thread
while it did. Those stay on the `Do`, as before; a pool of workers is the field that comment says
grows when a profile asks for it.

**Three bounds.** `AHEAD_BUDGET` (128 MiB) on rasters decoded and not yet taken, which the four
photographs of the witness need (104 MB); `AHEAD_FLOOR` (128 × 128 samples), measured: a round
trip through the pool and a condition variable costs 13.8 µs on average and a 64 × 64 grey image
decodes in 12.8 µs, while 128 × 128 takes 46.9 µs grey and 97.4 µs RGB; and
`REACHABLE_ENTRIES` (64) on the resource entries asked before starting.

## 3. What a single-image page pays: nothing, by two measured rules

The first A/B, unpinned, had `issue12841_reduced.pdf` — one photograph — about 20 ms slower. It
was two things, found in turn:

- **This machine's cores are of two classes** (4 Zen 5 at 5.16 GHz, 8 Zen 5c at 3.29 GHz,
  `doc/environment.md`), and a pool thread on the slower class decodes that photograph in about
  100 ms against 68 ms on the interpreter's own. So **the first image a walk finds is the
  interpreter's**: it is the one reached soonest, a decode ahead of it can at best be early by a
  little and at worst run on the slower core while the interpreter waits.
- **Asking rayon its size starts rayon's threads.** On a page naming no image, callgrind put that
  at 171 372 484 → 172 211 340 Ir before the question was moved behind the dictionary checks. And
  a page whose dictionaries reach fewer than two images a walk could offer — `reachable`, lookups
  only, on the interpreter's thread — starts nothing, so it takes exactly the path it took before.

## 4. The A/B, one sitting, two exported trees

`git archive HEAD` twice under `scratchpad/r1242/`, this round's hunks applied to the second,
separate target directories, `md5sum` of every binary compared (trap 50). `RAYON_NUM_THREADS=4`,
behind the lock, load average 2.3–4.1. **Pinned** (`taskset -c` on the CPUs whose
`cpuinfo_max_freq` is the highest), minimum of fifteen fresh processes, interpretation of page one
on a freshly opened document, ms:

| page | images | before | after |
|---|---|---|---|
| `22060_A1_01_Plans.pdf` | 4 photographs + masks | 265.73 | **99.88** |
| `images.pdf` | 5 | 201.82 | **128.30** |
| `issue1905.pdf` | 8 | 77.29 | **29.49** |
| `issue12841_reduced.pdf` | 1 | 67.13 | 67.97 |
| `issue13931.pdf` | 1 | 164.78 | 162.31 |
| `freeculture.pdf` | 1 | 33.30 | 31.17 |

`examples/frame_budget` (the page turn, minimum of three runs of five rounds, pinned), `turn` and
its `interp`: `22060_A1_01_Plans.pdf` 290.22 (253.06) → **134.08 (100.69)**; `images.pdf` 220.10
(203.78) → **143.75 (129.95)**; `issue12841_reduced.pdf` 122.76 (68.83) → 122.31 (67.98);
ISO 32000-2 page 101, text, 8.71 (1.36) → 8.63 (1.30). The transfer is unchanged, as it must be.
`examples/callgrind_interpret`: page 101, 171 372 467 → 171 367 886 Ir; the witness, 6 640 726 715
→ 6 648 809 444 (+0.12%, the walk — the decodes are the same work on more cores). The census over
all 958 first pages, three processes an arm, pinned: 6050.8 → 5680.1 ms in sum; the pages with no
image 4865.8 → 4784.4, with one 397.8 → 393.8, with two or more **787.3 → 501.9**.

Unpinned, the same binaries give the single-image page a median of 68.9 before and 98.7 after on
an identical code path — which is `doc/habits/measuring.md`'s lottery, not this change.

## 5. What is left

Images inside a transparency group with its own blending space, which the walk does not follow;
annotation appearance streams, which run after the scope; the sandboxed codecs, serial behind one
worker; and a turn onto a photograph-dominated page, whose remaining cost is one baseline JPEG's
Huffman decode (ADR 1272 section 2) — no parallelism between images reaches it.

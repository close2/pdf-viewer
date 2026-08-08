# 0233 — A reduction that cost what the full image costs

Date: 2026-08-08 (session 396)
Status: accepted

## Context

`doc/todo/24` had one item still owed: **JPEG 2000 at a reduced resolution level**.
`issue19517.pdf` is a 12608×16806 scan — 212 megapixels in four channels, 847 million
samples — drawn onto a page a screen will show at about four megapixels, and this tree has
reported it undrawn since the seventh session. ISO 32000-2 §7.4.9 NOTE 2 says the format
answers this itself:

> From a single JPEG 2000 data stream, multiple versions of an image can be decoded. These
> different versions form progressions along four degrees of freedom: sampling resolution,
> colour depth, band, and location.

and NOTE 3 addresses the answer to this program by name:

> Viewing and printing applications can gain performance benefits by using the resolution
> progression. If the full-resolution image is densely sampled, an application can select and
> decode only the data making up a lower-resolution version, thereby spending less time
> decoding.

The item was written down as blocked on an API. The ledger's §7.4.9 row said "**the decoder
cannot be given a target resolution**"; `pdf-sandbox`'s own comment beside
`target_resolution: None` said the lever "would need the display list to carry the scale a page
is about to be drawn at, which it does not"; `doc/todo/24` said "what is missing now is an API
on `hayro-jpeg2000` — a decode that can be told where to stop".

**All three were false, and had been for months.** `hayro_jpeg2000::DecodeSettings` has carried
`target_resolution: Option<(u32, u32)>` since `1dfc6e2f`, 10 December 2025, which is inside the
revision this workspace already pins. The display list's half arrived in the
three-hundred-and-seventieth session: `pdf_render::Grid::for_placement` is the device grid and
`ImageSource::AtDeviceScale` is how the interpreter carries a raster it has not produced
(ADR 0210). Both halves existed while three places in this tree named one of them as the
blocker.

That is `CLAUDE.md`'s decay rule wearing a dependency's clothes: *"the specification defines
nothing here" is itself a claim, and it decays.* So is "the library cannot do this".

## What the measurement found instead

Asking `hayro-jpeg2000` for a reduced level does what it says on time: it skips the bit-planes
of the levels above the cut (`decode.rs`) and the wavelet synthesis of their decompositions
(`idwt.rs`). Decoding `issue19517.pdf`'s codestream through `Image::decode`, wall clock and
resident size fall with the level as they should:

| asked for | raster | wall clock | peak resident |
|---|---|---|---|
| 788×1051 | 788×1051 | 0.19 s | 88 MB |
| 1576×2101 | 1576×2101 | 0.39 s | 213 MB |
| 3152×4202 | 3152×4202 | 0.89 s | 715 MB |
| full | 12608×16806 | 12.26 s | 10 755 MB |

**And under the confined worker's gigabyte of address space, not one of them completed.**
`build_decompositions` sized the coefficient buffer from the component *tile*'s rectangle —
the full-resolution image — whatever level had been asked for, so every one of those decodes
began with a single allocation of **3 390 240 768 bytes**. Resident size never showed it: the
buffer is `calloc`'d and the pages of the levels that are never decoded are never touched, so
the cost is address space and nothing else. Address space is exactly what
`pdf_sandbox::lockdown` bounds (`RLIMIT_AS`, 1 GiB), what a 32-bit target bounds, and what a
`no_std` allocator with a fixed arena bounds.

So the reduction bought time and not memory, and this program's problem is memory.

## Decision

### 1. The fix goes to `hayro-jpeg2000`, because it is that crate's

`close2/hayro`, branch **`feat/reduced-resolution-allocates-less`**, commit **`1dc833f7`**:
size the coefficient buffer from the highest resolution level that will actually be decoded,
and give the levels above it an empty range.

The identity that makes this exact is B-15's: the sub-bands of resolution levels 0 to *r*
partition the rectangle of resolution level *r*. With nothing skipped, the highest kept level
*is* the component tile, so the full-resolution path is unchanged to the byte.

**Their code-blocks are still built**, and that is the part worth writing down: a packet header
is what says how long its body is, and a tile-part's packets are read in sequence. In LRCP
order — which is what this file states — the packets of the skipped levels are interleaved with
the ones that are kept, so they have to be *read past* rather than not described. Only their
coefficients are unwanted.

Measured on the same codestream, peak address space:

| target | before | after |
|---|---|---|
| 788×1051 | 3336 MB | **115 MB** |
| 1576×2101 | 3424 MB | **241 MB** |
| 3152×4202 | 3775 MB | **743 MB** |
| 6304×8403 | 5176 MB | 2751 MB |

Under a 1 GiB `RLIMIT_AS`, none of the four ran before and the first three run after.

It is written as a change *hayro's* users want rather than one shaped around this viewer, which
is `doc/JPEG2000_FEEDBACK.md` §9's standing condition: a decoder whose reduced mode costs the
full image's address space cannot be used by anything with a ceiling, and that is a wider
population than one PDF reader. Output is unchanged — all 183 assets of the crate's own test
suite are byte-identical to snapshots taken from `main` before the change, and
`test_jpeg2000_standard_example_b4`, Annex B.4's worked example, still passes.

**This tree cannot push it and does not depend on it yet.** The branch is committed in
`tmp/hayro`; the project owner pushes it to `origin` and opens the pull request, and the
workspace's `rev` moves when it is there. A `path` dependency into `tmp/` in a committed
manifest is a tree that does not build from a fresh clone, which is not one of the options.

### 2. What this tree commits today, measured

**`pdf_sandbox::decode::MAX_SAMPLES` goes from 2^27 to 2^26**, and this is not bookkeeping
around the above — it is a bound that was wrong on its own terms. Its documentation estimated
"roughly five bytes per sample" and concluded that 2^27 samples was "around 670 megabytes,
which leaves headroom inside the gigabyte". The bound exists to refuse *cheaply*, "before the
address-space limit has to end the process the expensive way". Measured, on codestreams built
for the purpose and decoded at full resolution — so the number is the same with or without the
branch above:

| codestream | samples | peak address space | inside the gigabyte |
|---|---|---|---|
| 4096×4096, four channels | 2^26 | 600 MB | yes |
| 6690×6690, three channels | 2^27 + 50 572 | 1253 MB | **no — the allocation fails** |

Nine to thirteen bytes a sample, not five. At 2^27 the bound admitted an image that kills the
worker, which is the one outcome it was written to prevent. **No corpus codestream lies between
the two** — the largest that decodes is 2.4 million samples — so this narrows a bound nothing
reaches rather than refusing anything that was drawing, and every gate is unmoved.

The three false claims are corrected in place, each saying what it used to say.

### 3. What the *page* does, and where the number comes from

**Measured against a patched build, and only against a patched build.** With the hayro branch
in a local `[patch]` and `pdf-sandbox` stepping down resolution levels until the budget is met,
`issue19517.pdf` page one:

| | commands | wall clock | peak resident, whole process tree | reported |
|---|---|---|---|---|
| committed today | 0 | 15 ms | 19 MB | `Im0: JPX: 12608x16806 in 4 channels is 847560192 samples, beyond the 67108864 this decoder is given room for` |
| with the branch | **1** | 5.60 s | 2545 MB | nothing |

The raster chosen is 3152×4202 — two levels down, 53 million samples, inside 2^26 — and the
page it draws agrees with `poppler`: a flat orange field with faint text, which is what this
document is. The 2545 MB is not attributed line by line here and should not be quoted as
though it were; the confined decode is about 715 MB of it and this document also states a
12608×16806 `DeviceGray` `/SMask` whose `RunLengthDecode` stream expands to 212 MB, which
§11.6.5.2's device-scale route (ADR 0210) then keeps packed rather than combining.

**None of that is in the commit.** Against the pinned revision the same code would ask for a
reduced level, the worker would allocate 3.4 GB, `RLIMIT_AS` would abort it, and the page would
report a dead worker where it now reports an accurate size — trading a cheap, correct refusal
for a process abort. `doc/todo/24` states the four edits that follow the fork's push.

## Consequences

- The clause is not closed and §7.4.9 stays `partial`, with its row now naming what is actually
  owed instead of an API that has existed since December.
- One dependency defect found, fixed, measured and offered — the second under
  `doc/JPEG2000_FEEDBACK.md` §9's route, after §8's reconstruction midpoint.
- A security-relevant bound in this tree is now measured rather than estimated. The neighbouring
  `MAX_PIXELS` is **not** measured and its "about a byte per pixel" is still an estimate; saying
  so is the honest state, not a claim that it is wrong.
- The general lesson is the one `CLAUDE.md` already states about clauses, restated about
  dependencies: a note that says *the library cannot do this* is a claim with a date on it, and
  the cost of checking is one `grep` of the crate's public API.

## Alternatives considered

**Decide the reduction at interpret time, from the memory budget alone.** Simpler, keeps the
display list a pure function of the file, and for this document would give a raster finer than
any screen. Rejected as the *first* step because it answers a different question from the one
ADR 0210 built the vocabulary for, and because it would have been indistinguishable from the
right answer on this corpus while being wrong at high zoom. It stays available as the
lower-risk half of `doc/todo/24` if the fork's push is slow.

**Ask `hayro-jpeg2000` for a "no larger than" target instead of the "at least this large" one
it has.** `target_resolution` picks the finest level whose dimensions are at least the request,
so a raster can come back up to twice the requested size per axis. That is the right rounding
for fidelity and the wrong one for a budget, and it is a real gap in the API — but it is
bounded (four times the samples, worst case) and `pdf_render::Image::area_averaged` already
reduces on this side. Not worth asking a maintainer to carry a second knob when the first one's
allocation was the actual defect. Recorded here so that the next round does not rediscover it
as new.

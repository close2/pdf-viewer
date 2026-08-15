# Quorra — the API delta of the 2026-08-15 round

**Written by the renderer side for the viewer side.** This is the one document to work
from when you take the next quorra bump: everything a `render-quorra` author has to *do*
is here, and the reasoning behind each item is in the ADR named beside it.

Your `Cargo.lock` pins `87898c6`. This round takes you **twenty commits** past it. Four
ADRs land — 0049, 0050, 0051, 0052 — and one of them **changes pixels**, which is the item
to read first because it is the one that touches your ratchets rather than your code.

Nothing was **removed** and nothing was **renamed**. Every item below is additive.

---

## 0. Read this first — one change moves pixels

**ADR 0049 fixed a defect in our coverage rasteriser, and it moves your corpus.**

`fill_mask` computes a shape's coverage over a rectangle of device pixels. When an edge
entered that rectangle from outside, we clamped the edge piece's two endpoints to the
border and interpolated between them. That preserves the row's *total* winding — every
column past the crossing reads the correct value, which is why neither side's tests ever
saw it — but it hands the columns **at the border** the height the piece spent outside.
Measured against the same device pixels of a wider region: **2 684 differing pixels out of
2 863 228, the worst by 185 of 255.**

We now cut the piece at the border instead. The test states its expected value from the
geometry — 0.625 of a pixel of area is 159 of 255 — rather than from any rasteriser; the
tree was producing 128.

**What you will see on your corpus**, measured in one copy of your tree with base and
change run in the same hour:

| | before | after |
|---|---|---|
| scale 1 | 930 agree / 24 differ / 2 refused / 18 not comparable | **931** / 23 / 2 / 18 |
| scale 4 | 936 / 10 / 5 / 23 | **unchanged** |

`issue2177.pdf` stops differing. `issue6081.pdf` at 4× moves toward your oracle (worst tile
9.17 → 8.86) and `issue11473.pdf` at scale 1 moves by 0.0001 of a mean. Nothing moved away
from the oracle at either scale.

**Only tiles that a clip or the page edge cuts in x can move at all** — a piece wholly
inside the rectangle takes the same arithmetic it always did, to the bit.

> **One caution about your own baseline.** Do not compare against the `934/20/2/18` in our
> `PLAN.md` dated 2026-08-14. Re-running that same quorra commit against your tree on
> 2026-08-15 reads `930/24/2/18` — *your* tree moved, not ours. The `930 → 931` pair above
> is the honest measure of this change, because both halves were run the same hour in one
> copy. If you re-baseline, re-baseline both sides.

---

## 1. `Counters` gains four fields

```rust
pub struct Counters {
    // …
    /// Atlas bytes this frame's distinct glyph keys asked for, hits included.
    pub atlas_working_set_bytes: u64,          // ADR 0050
    /// Whether the atlas was repacked after this frame.
    pub atlas_repacked: bool,                  // ADR 0050
    /// Distinct residue clip regions this frame rasterised.
    pub clip_residue_regions: u32,             // ADR 0049
    /// Residue rasterisations charged to a single command's tile instead.
    pub clip_residue_tiles: u32,               // ADR 0049
}
```

`Counters` still derives `Debug, Clone, Copy, Default, PartialEq, Eq`.

### What you must do

**Nothing, unless you build a `Counters` with an exhaustive struct literal.** Reading
fields, copying, comparing and `..Default::default()` all keep compiling. We return this
type and never take it, so the only plausible literal is a test fixture or a mock. It is
written down because a struct without `#[non_exhaustive]` makes a new field a breaking
change whether or not anyone trips over it.

`render-quorra/src/present.rs` already copies `commands`, `commands_culled` and
`bytes_uploaded` out beside `encode_source`. These four belong in the same place.

### What each is for

- **`atlas_working_set_bytes`** — what holding *all* of a page's distinct glyph tiles would
  cost. This is the number to compare against the `atlas_budget` the device was built with,
  and the only one that tells *"the atlas is too small for this page"* apart from *"the
  atlas is holding another page"*. Raising `Options::atlas_budget` is the lever.
- **`atlas_repacked`** — true on a frame after which the atlas was thrown away and
  re-packed. **This is the one event that makes a `RetainedScene`'s encode stale**, so a
  viewer watching its retained frames re-encode can now see whether the atlas is the
  reason. A page that changes settles after at most one; true frame after frame means the
  atlas is thrashing between two pages that do not fit beside each other.
- **`clip_residue_regions`** — distinct clip regions rasterised for chains whose links are
  not all rectangles.
- **`clip_residue_tiles`** — the number of times a chain was rasterised over one command's
  tile instead; the work the regions did *not* remove.

The last two answer, from our side, the question your `QUORRA_FEEDBACK.md` §15 asks from
yours. A page reporting `1` region and `600` tiles states one curved clip, draws six
hundred marks under it, and pays one rasterisation for the clip. A page reporting `0`
regions and `40` tiles has forty chains each used once — **and that shape is worth telling
us about**, because it is what the next lever on this seam is for.

All four are exact functions of the scene and the viewport, so all four compare by equality
across machines and adapters. All four are counts of **keys**, never hit rates, for the
reason your own ADR 0132 states.

---

## 2. `DeviceError` gains one variant (ADR 0050)

```rust
DeviceError::ResourceIdsExhausted { limit: u32 }
```

Returned by `Device::upload_outline`, `upload_image`, `upload_ramp` and `upload_mesh` when
a device has issued all `u32::MAX` resource identifiers.

**What you must do: nothing, unless you match `DeviceError` exhaustively without a `_`
arm.** No document reaches four billion uploads; this is a bound that is now *stated*
rather than a wrap that was not.

The reason it exists is an audit finding rather than a feature. The id counter used to
`wrapping_add`. After 2³² uploads an id would be reissued, the live resource silently
replaced, and — because the generation counter counts *releases* and would not have moved —
a retained encode would have drawn bytes it never named with **every staleness check
agreeing that nothing had changed**. Unreachable in practice, and cheap to make true.

---

## 3. Behaviour: a page that overflows the atlas now replays (ADR 0050)

Before, a frame whose glyph tiles overflowed the atlas could repack it afterwards, and the
repack invalidated the encode of the frame that caused it — so in one measured band a
**still page re-encoded on every frame, for ever**. It now repacks only when there is space
to reclaim from an *earlier* frame's tiles, which bounds a page at **one repack and two
encodes, then replays**.

We should correct our own earlier description of this while handing it over: we had told
you the trigger was atlas *overflow*. It was not. Sweeping seventeen (budget, magnification)
pairs, sixteen settled after one encode; the pathology is the narrow band where a page fits
the atlas **by bytes** but not **by shelves**.

**No pixel changes.** A tile drawn through the atlas and the same tile drawn through the
scratch sheet are the same bytes; what changed is when a layout is discarded.

**What to expect:** on magnified text with a modest atlas budget, `Frame::encode_source()`
becoming `Replayed` where it stayed `Encoded`, and `Timings::encode` dropping to zero on
those frames. If your corpus run carries per-page timings at scale 4, that is where it
shows.

Nothing in `QUORRA_RETAINED_FRAME.md`'s adoption instructions changes: the obligation is
still to call `set_scene` when the content changes, and nothing else.

---

## 4. Performance: the residue clip seam, half taken (ADR 0049)

A clip chain whose links are not all axis-aligned rectangles has a *residue* that must
become coverage. We used to re-flatten and re-rasterise that residue **once per clipped
command**. We now rasterise it **once over the region it occupies**, and every mark takes a
window on it — which is what the border-cut fix in §0 had to make legitimate first.

On the artwork archetype (your corpus's p99 clip shape), headless into a texture, base and
change alternating, three rounds each, minima, load 3.8–4.8:

| | before | after |
|---|---:|---:|
| encode geometry | 37.78 ms | **28.89 ms** (−24 %) |
| encode | 46.26 ms | **37.17 ms** (−20 %) |
| wall | 51.72 ms | **40.35 ms** (−22 %) |

600 residue rasterisations became 185.

**Two corrections to what we told you earlier**, both ours:

- We described this seam as *"35 ms of a 43 ms frame re-rasterising the same clip coverage
  every frame"*. That read the whole geometry phase as though it were all residue. Measured,
  the residue span is **17.3 ms of the 65.6 ms** artwork spends flattening and rasterising —
  about a quarter, not all of it.
- We said three pages at 4× and one at scale 1 refuse with `ScratchExhausted`, and that this
  was *"the only reason any frame of the corpus is refused"*. On your tree today it is
  **two** pages at 4× (`bug1703683_page2_reduced.pdf`, `issue1905.pdf`) and **none** at
  scale 1, and the corpus's other three refusals are each something else: one on
  `max_resource_bytes` at upload, and two clause refusals that are *correct*.

**The refusals did not move, and that is by design.** A region is host memory that never
reaches the coverage sheet; `Counters::tiles` is unchanged on every archetype, which is the
evidence. What refuses those two pages is the *sheet* — a clipped shape still becomes one
coverage tile of its own device bounds, and at 4× a full-page clipped shape is a full-page
tile. That is tiling work and it is still open on our side.

---

## 5. Nothing to do, listed so you can confirm it

- **ADR 0051 — three of our source files were split along their responsibility seams**
  (`scene.rs` 1216 → 7 files, `compose.rs` 1014 → 6, `winding.rs` 844 → 4). **No public
  item changed name, path or signature**; the split modules are private and re-exported.
  Verified by comparing the exported item set before and after, and by a 39-command page
  through every lane hashing identically on both coverage lanes at two scales.
- **ADR 0052 — our readback perf gate now counts allocations instead of timing them.**
  Test-suite only; nothing you consume.

---

## 6. Two things we would like back from you

Neither blocks the bump.

1. **A row your corpus profile does not carry: how many of a page's commands are
   *rectangular fills*.** This decides your `QUORRA_FEEDBACK.md` §19 outright, and it is a
   one-line addition to the walk that produced `doc/corpus-profile.md`. If your p99 page has
   a dozen rectangles, §19 closes as already-handled; if it has four thousand, it is worth a
   morning on your side. (ADR 0047 has since made the ask worth about a third of what it
   was, because a *solid* fill of a rectangular outline now takes our analytic lane too.)
2. **Whether you can draw the page and the overlays as two `render` calls into one
   target.** This is the cheaper half of the encode-cache question in your `doc/todo/44` §3.
   Under a device-side cache keyed on scene identity, a host that rebuilds the frame's scene
   every frame with fresh `Arc`s misses every time. If two calls work, the page's `Scene` is
   stable across frames and hits the cache, and the overlays cost microseconds of their own
   encode — and neither side needs new vocabulary. If there is a reason it does not work —
   a blend that must see the page beneath it inside one transparency group, an overlay that
   must be clipped by page geometry — **that reason is the specification for scene-fragment
   composition**, and we would rather design it from that than from the general shape.

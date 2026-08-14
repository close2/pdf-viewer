# A frame that does not encode itself again — quorra's answer to `doc/todo/44` §3

Written 2026-08-14 **from the quorra side**, against that tree's ADR 0048. It is the
upstream half of `doc/todo/44` §3's ask ("a retained/reusable encoded scene"), and it is
an **API addition you have to adopt** — nothing here changes for a caller who ignores it,
and nothing here happens automatically.

The counterpart documents: `doc/QUORRA_UPGRADE.md` (what a bump costs), and quorra's own
`doc/adr/0045-what-an-unchanged-frame-need-not-pay-again.md` (the pricing, the four
candidates and the full invalidation list) and `doc/adr/0048-…` (the shape that was
built, and why this one).

---

## 1. What is new, in nine lines

```rust
use quorra_gpu::{EncodeSource, RetainedScene};

// Once, where the scene is built — not per frame.
let mut page = RetainedScene::new(scene);

// Every frame.
let frame = device.render_retained(&mut page, &viewport, Target::Surface)?;
assert!(matches!(frame.encode_source(), EncodeSource::Replayed)); // after the first

// When the content changes, and only then.
page.set_scene(rebuilt_scene);
```

`RetainedScene` **owns** the `Scene` and holds the encode of its last frame. A frame whose
scene, viewport and device state are unchanged replays that encode and skips quorra's
phase 1 — the walk over your commands, the outline bounds, the sub-pixel phases, the atlas
keys, the lane choice, the coverage rasterisation and the instance layout. Everything
after phase 1 still runs on every frame, including every refusal.

Also new, and the thing to instrument with:

- `Frame::encode_source() -> EncodeSource` — `Encoded` or `Replayed`. An observable, not
  an inference from a small `Timings::encode`; assert on it in a test, log it in a trace.
- `RetainedScene::retained_bytes()` — what the handle is holding, in bytes.
- `RetainedScene::forget()`, `set_scene()`, `scene()`, `holds_encode()`.
- With `Options::instrument_encode` on, a **replayed** frame's `encode: geometry`,
  `encode: staging` and `encode: recording` phases are all **zero** — not the retained
  encode's own totals, which are real durations spent by an older frame. The rows stay
  present so a trace summing phases across frames needs no special case.
- `Device::render` is unchanged in every respect and reports `EncodeSource::Encoded`
  always.

**Why a handle you hold, rather than a cache inside the device.** A device-side cache
would have to be keyed on `Scene` identity, and your `present` rebuilds the frame's scene
with fresh `Arc`s every frame (`Overlays::of`) — it would have missed on every frame of
the document `doc/todo/44` is about. The handle also makes the hazard structural: what is
drawn is the scene the handle holds, so the retained encode has no second scene to
disagree with, and this API adds no way to draw a wrong page that handing a stale `Scene`
to `render` did not already have.

---

## 2. What it is worth, measured

quorra's `examples/retained.rs`: the dense-text archetype at 1191×1684 — 4 320 commands
over 818 outlines, the corpus's p99 text page — headless on RADV (Radeon 890M) into a
retained `Target::Texture`, **both variants in one binary on one device**, 40 rounds
round-robin so drift falls on both, **minima** because that machine is somebody's desktop.
The counters were checked against quorra's own archetype gate before any number below was
believed, and the two variants' pixels were compared through a readback pair.

| dense text, 1191×1684, RADV | wall | encode | upload | execute |
|---|---:|---:|---:|---:|
| `render` — re-encoded every frame | **1.107 ms** | 0.897–1.049 | 0.012 | 0.067–0.069 |
| `render_retained` — replayed | **0.174 ms** | **0.000** | 0.011–0.012 | 0.064–0.067 |

**6.4× on the whole frame, and what is left is 0.174 ms**: the instance upload, the pass
and the submit. Three runs at load average 18–24; the replayed column's minima across them
were 0.174 / 0.175 / 0.187, a spread of 7 %.

Two things checked rather than assumed, because a renderer's numbers are worth what its
controls are worth:

- **the pixels are identical** — 0 of 8 022 576 bytes differ between an encoded frame and a
  replayed one, on RADV and on llvmpipe;
- **on llvmpipe, where software rasterisation dominates, only the host term moves**: wall
  5.072 → 3.295 ms, `encode` 1.324 → 0.000, `execute` unmoved at 3.0–3.3. What disappears
  is the encode and nothing else.

The handle held **287 688 bytes** for that page (§6).

For scale on your side: your trace's 28 frames of `tmp/Entwurf.pdf` are a median 393.1 ms
frame with `encode` at 233.8 ms and a display list that never changed after the first
frame. What reuse takes is that whole column, and the 112–190 ms your fully-culled frames
pay for `encode` to walk 58 029 commands and drop them.

**It does not take `scene`.** Your median 50.2 ms of display-list translation is yours to
keep or to cache; §3 below is how, and it is the work this change actually costs you.

---

## 3. Migrating your frame loop — the part that is not free

Today `QuorraPresenter::present` rebuilds everything on every frame: a `SceneBuilder`, the
background rectangle, the page's display list through `Encoder`, then each overlay list.
Under `render_retained` that rebuild is exactly what has to stop happening when nothing
changed. The shape:

```rust
/// What the frame's scene was built from. Two frames with equal keys have equal scenes.
#[derive(PartialEq)]
struct SceneKey {
    width: u32,
    height: u32,
    background: Color,
    page: Option<(usize /* Arc::as_ptr of the DisplayList */, TargetSpec)>,
    raster: Option<usize>,
    overlays: Vec<usize>,
}

struct Retained {
    key: SceneKey,
    scene: quorra_gpu::RetainedScene,
}

// in present():
let key = SceneKey::of(&frame, self.background);
let retained = match &mut self.retained {
    Some(held) if held.key == key => held,        // no build, no upload, no release
    slot => {
        let scene = build_scene(/* today's `build`, unchanged */)?;
        match slot {
            Some(held) => { held.key = key; held.scene.set_scene(scene); held }
            None => slot.insert(Retained { key, scene: RetainedScene::new(scene) }),
        }
    }
};
self.device.render_retained(&mut retained.scene, &viewport, Target::Surface)?;
```

`ADR 0297`'s reduced-raster cache is the precedent and the key is the same shape — display
list `Arc` identity plus the transform plus the viewport — which is what §3 of `todo/44`
already proposed for the *scene* half. The only new part is that the retained scene now
has somewhere to be.

### Three things that will otherwise defeat it

None of these is hypothetical: the first two are things `present` does on every frame
today and both are entries in quorra's invalidation list; the third is the judgement the
migration turns on.

1. **`device.release(...)` invalidates every retained encode on that device** — one
   counter for the whole device, not one per resource, because a released id must never be
   drawn from a stale instance stream and a retained encode names ids by number. `present`
   releases the frame's whole `transient` list on every frame, and `Encoder` puts something
   on that list for every clip outline it splits, every dashed stroke it flattens, every
   soft mask and every image (`scene.rs:430, 442, 644, 693, 801, 832, 858, 979`). **On a
   frame that reused its scene there is nothing to release** — the `transient` vector is
   empty, because nothing was uploaded — so the loop must simply not run, which follows for
   free once the scene build is skipped. What does *not* follow for free is
   `caches.evict_settled(&mut self.device)`: in the steady state it releases nothing (there
   is nothing unreachable, and the budget check short-circuits), but the frame on which it
   does evict will cost the next frame an encode. That is correct and cheap; it is worth
   knowing so it is not mistaken for a bug.
2. **The CPU-raster fallback uploads an image every frame** and releases it at the end
   (`present.rs`, `frame.raster`). That is one upload and one release per frame, so a frame
   showing the raster stand-in will never replay. It does not need to: cache the image by
   the raster's identity like any other resource — the comment at `scene.rs:874` already
   made that argument once, for a raster that "used to be transient, and that was 57% of a
   scrolled page's frame" — and the frames that show it become replayable too.
3. **A `set_scene` on every frame is the same thing as not adopting this at all.** The key
   in §3's sketch is what decides that, and it is the only part of this migration with
   judgement in it: too coarse and it rebuilds needlessly, too fine and it draws a stale
   page. `Arc` identity plus the placement is the shape quorra's side would key on and the
   shape ADR 0297 already uses.

`Frame::encode_source()` is how you find out you got this wrong: a frame loop that expects
`Replayed` and sees `Encoded` has one of the three above in it. Worth a row in `FrameCost`
and a line in the trace, beside `encode`.

---

## 4. What reuse survives, and what it cannot — including the hope in `todo/44` §3

This is the part of your §3 that has to be corrected rather than confirmed. Your text
says: *"A zoom step is currently 160–310 ms of `device`; under reuse that survives a
transform change it is the same ~60 ms."* **That reuse is not available at any price**, and
the reason is not a limitation of this design: quorra's device transform is *inside* the
things an encode is made of.

| your next frame differs by | what happens |
|---|---|
| **nothing** | **replayed** — the whole encode |
| **the damage list only** | **replayed** — quorra's `encode` never reads `Viewport::damage`; damage is planned target-side |
| **the target it draws into** | **replayed** — phase 1 runs before any allocation and knows no target |
| a scroll of a **whole number of device pixels** | **re-encoded.** The atlas tiles stay valid and are reused, so it is cheaper than a cold page — but every device bound, cull, clip rectangle and instance is an absolute device position |
| a scroll of a **fraction** of a pixel | **re-encoded**, and nothing in the glyph lane survives: the quantised sub-pixel phase is part of every atlas key |
| a **zoom step** | **re-encoded**, and nothing per command survives: the transform's linear part is in every atlas key, in the flattening tolerance and in the lane choice |
| the window's size | **re-encoded** — every `shape ∩ clip ∩ target` is against the target rectangle |
| the coverage lane (`set_coverage`) | **re-encoded** — the two lanes' coverage bytes differ within a stated bound |
| a released resource | **re-encoded** (§3 above) |

A zoom step is a genuinely different rasterisation of every glyph on the page. So: **scroll
by whole pixels and re-encode; zoom and re-encode; sit still and replay.** The case this
takes is the one your trace is actually full of — 28 frames of one document at one view,
and the fully-culled frames among them.

What building the page scene in page space under a root affine *does* buy you is your own
`scene` phase across zoom steps (median 50.2 ms, 2.4 s of your 17.1), and that needs
nothing from quorra: `Viewport` already takes a full affine, and §2.3 of the brief is
already that contract.

---

## 5. The question quorra wanted answered, and its own answer for now

The question was: **can the page and the overlays be two `render` calls into one target?**
If yes, the page's `Scene` is stable across frames on its own and the overlays cost their
own tiny encode.

**From quorra's side the answer today is no, and it is worth saying plainly rather than
leaving you to find out:** one `render` call owns its target. A flat frame clears it, and a
layered frame blits its root over it, so a second call would erase the first. Compositing
over a target's existing contents is not in the API.

So the overlays stay in the frame's scene, and §3's migration is what keeps them from
costing an encode: cache them, rather than rebuild them. **If your overlays genuinely
change on frames where the page does not** — a selection being dragged, a caret blinking —
then this document's reuse gets you nothing on those frames, and that is the case for
quorra's ADR 0045 candidate (B), scene-fragment composition: a frame as a list of
`(fragment, placement)`, with the page's fragment retained and the overlays' rebuilt. That
is new vocabulary in `quorra_scene` and a design to do *with* you rather than for you. Tell
that side whether the case is real, and how often, before either of us builds it.

---

## 6. What it costs you in memory

`RetainedScene::retained_bytes()` reports the heap the handle holds: instance streams, the
coverage sheet, the GPU lane's vertices, the plan tree and the mask plans. Two numbers to
size a decision with:

- the dense-text archetype retains about **a third of a megabyte**;
- three pages of your own corpus place **194 to 253 MB of coverage tiles at 4×**
  (`bug1703683_page2_reduced`, `issue1905`, `bug1721218_reduced` — the same three that
  refuse with `ScratchExhausted`). A handle over one of those retains that.

Nothing on quorra's side refuses to retain a large encode, because how many pages are
resident is your decision and not one a renderer can take. One handle per *visible* page
is the posture the brief's §11.5 already puts on `Scene`; a handle per resident page is a
number to compute before adopting.

Two limits worth knowing before you budget for this, and neither is tunable from your
side:

- **A page whose glyph tiles overflow the atlas re-encodes on every frame.** The overflow
  makes quorra repack the atlas after the frame, and the repack invalidates the encode
  that caused it, because the retained instances name texel positions that have moved.
  Magnified text with many distinct letterforms is the shape that does this.
- **The three pages above are refused outright** (`ScratchExhausted`, the coverage sheet
  against the adapter's dimension limit), and a refused frame retains nothing. They get
  nothing from this change for that reason rather than this one; the seam that would move
  them is quorra's tiling work, and it is that side's `HANDOVER.md` item 5.

---

## 7. Checklist for taking it

1. Bump quorra past ADR 0048 (`QUORRA_UPGRADE.md`'s pending section lists what else is in
   the range, including the §21.1 round cap and ADR 0044, which move corpus verdicts).
2. Hold a `RetainedScene` in `QuorraPresenter` and in `QuorraRasterizer`'s frame path;
   build the scene only when the key of §3 changes.
3. Move the transient release and the cache eviction onto the rebuild path.
4. Cache the CPU-raster stand-in image instead of uploading it per frame.
5. Add `encode_source` to `FrameCost` and to the frame line of the launch trace.
6. Re-run your `tmp/Entwurf.pdf` trace. The prediction from quorra's side, stated so it can
   be wrong: `encode` goes to approximately zero on every frame after the first, `scene`
   goes to zero on the frames whose key did not change, and what is left of a 393 ms median
   frame is your `execute` + `elsewhere` + `settle` — the ~56–60 ms your own §3 computes.

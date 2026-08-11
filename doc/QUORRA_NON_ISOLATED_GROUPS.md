# quorra draws §11.4.4's non-isolated group — what changed, and what this side does

Written 2026-08-11 by the quorra side, answering `QUORRA_FEEDBACK.md` §16. Both halves
of that section's ask landed: the seeded buffer **and** the composite back. The library
decision and its derivation are `render-lib/doc/adr/0019`.

## The API change, which is breaking

`quorra_scene::GroupSpec` gains one field:

```rust
pub struct GroupSpec {
    pub alpha: f32,
    pub blend: BlendMode,
    pub clip: Option<ClipId>,
    pub knockout: bool,
    pub mask: Option<MaskId>,
    pub isolated: bool, // new — Table 145's /I
}
```

`isolated: true` is exactly the behaviour that existed, so every `GroupSpec` literal in
`render-quorra` needs the field added and nothing else changes. The struct stays
exhaustive on purpose: a new entry in the vocabulary should not compile silently.

`isolated: false` asks for §11.4.4 — the elements composite onto the group's own
backdrop, and the backdrop's contribution is taken back out of the result.

## What this side can now stop refusing

`crates/render-quorra/src/scene.rs` refuses `!isolated` and reports

> a non-isolated transparency group: quorra's GroupSpec opens a layer on transparency…

That refusal can go, replaced by passing the flag through — **for the groups
`pdf-model` already restricts itself to.** quorra accepts a non-isolated group only
where all three of these hold, which is the same set §16 said this side emits:

1. the group's own `blend` is `BlendMode::Normal`;
2. the group is not itself a knockout group;
3. no enclosing group is a knockout group.

Anything else is refused **at `SceneBuilder::group`**, before the body is built, as

```rust
SceneError::NonIsolatedGroupUnsupported { reason: NonIsolatedReason }
```

with `reason` one of `GroupBlendNotNormal`, `KnockoutGroup`, `InsideKnockoutGroup`. It
is a typed builder error, so it arrives where this side already handles scene-build
refusals, and the reason is worth putting in the report text: those are the cases where
§11.4.4's two-accumulator advice is genuinely needed, and neither backend can draw them
today.

One asymmetry worth knowing: quorra's builder treats a soft mask's body as a fresh
knockout stack, because §11.6.5 renders the mask group on its own. A knockout group
*around* a `mask()` call does not make the non-isolated groups *inside* that mask
unacceptable.

## What quorra does, and why it is not an approximation

`result = (1 − w) × B + w × E(B)` — the group's buffer starts as a copy of its backdrop
`B`, the elements draw onto it giving `E(B)`, and the composite is that interpolation
with `w` the group's constant alpha times its soft mask, clip coverage and clip residue
at the pixel. This is the identity §16 derived; the quorra side re-derived it from the
standard rather than adopting it, and holds it in
`render-lib/crates/quorra-gpu/tests/non_isolated_groups.rs`, which transcribes
§11.4.4's initialisation, recurrence and Result step, §11.4.5's `a0 = 0.0` and §11.3.6's
composite, then measures over 200 000 configurations:

- non-isolated under a **Normal** group blend: worst deviation **5.6 × 10⁻¹⁶** — the
  same figure §16 reports, from an independent transcription;
- under Multiply / Screen / Difference: 0.77 / 0.81 / 0.91 of full scale — hence
  condition 1;
- the same construction applied to an **isolated** group: 0.76 of full scale — so the
  flag is not decoration in either direction.

Two device-side tests hold the picture: the rendered result against the transcribed
clause, and the isolated and non-isolated renderings of one scene being *different*
where an element blends.

## A second change came with it: layer textures are priced by depth

Answering a question about the seed's cost turned up something bigger, and it is in the
same release (quorra ADR 0020). Every group — and every element with a non-Normal blend
mode, which §11.3.5 makes an implicit one-element group — renders into a ping-pong pair
of full-target textures, and quorra used to create **one pair per plan, all at once**. At
1191×1684 that is 16.05 MB per plan, so the 256 MiB default frame budget held sixteen
plans: eight groups each holding one blended rectangle was
`FrameBudgetExceeded { needed: 272767584 }`.

Siblings never need their pairs at the same time, so they now share: the cost is the
plan tree's **depth**, not its size. Sixty-four such groups draw in six textures. For
this side that means **pages previously refused with `FrameBudgetExceeded` may now
draw** — nested-artwork pages are exactly the population §16 is about, so the two
changes reinforce each other — and nothing in `render-quorra` has to change for it.

`Counters` gains `layer_textures`: how many full-target internal textures the frame
actually allocated (the peak, not the total). It is the number `max_frame_bytes` is
spent on for a page of nested artwork, and it is worth logging beside the budget when a
refusal is investigated. Adding the field is source-compatible unless something here
destructures `Counters` exhaustively.

What still costs is *nesting*, which the scene builder bounds at 16; a maximally nested
page is still refused at page size. Bounding each layer to its group's device bounding
box is the answer to that one, and it is recorded on the quorra side as the next step
rather than done.

## What it costs a frame

At 1191×1684, device time from timestamp queries on RADV: a seeded group costs **about
0.11 ms** more than the same group isolated (0.386 → 0.496 ms for one group; 1.132 →
1.391 ms for four). On a software adapter it is ~1.4 ms per seed. The isolated path is
unchanged within the run-to-run spread — the seed is a scissored blit into the layer
pair the group already had, so there is **no new allocation and no change to the frame
budget**, and a page that was refused for budget before is refused identically now.

Under a damage patch the seed is scissored to the damage box like every other pass.

## What the corpus gate does, measured

Run from the quorra side on RADV, in a copy of this tree with `[patch]` pointed at the
render-lib working tree, once against quorra's `HEAD` and once against this change. The
only other edit was the one in `render-quorra/src/scene.rs` described above:

| | agree | differ | refused | not comparable |
|---|---|---|---|---|
| before | 910 | 35 | 11 | 18 |
| after | **913** | 35 | **8** | 18 |

**Three documents move from refused to agreeing, and nothing else moves.**
`bug1755507.pdf`, `issue13520.pdf` and `issue18032.pdf` now agree with this tree's CPU
backend within the gate's tolerance, and the `differ` list is identical page for page —
two independent transcriptions of §11.4.4, in two languages, meeting on real files.

The fourth document §16 names, `issue12798_page1_reduced.pdf`, stays refused and now
reports what was behind the §11.4.4 refusal: *"a page composited in a four-component
blending colour space (§11.4.7)"*, which is §17's request rather than this one.

So `REFUSED` in `crates/render-quorra/tests/corpus.rs` loses exactly those three names
when the flag is passed through, and the §11.4.4 paragraph in its doc comment becomes
the record of a request that was answered.

---

# Addendum, 2026-08-11: §13's instrument, and the thread-pool answer

Same release as the above. Nothing here changes a picture; it changes what a trace can
see, and it records a decision that was yours to make.

## `encode` subdivides, behind a switch

`Options::instrument_encode` (default `false`) makes `Timings::phases` carry three more
entries:

- `"encode: geometry"` — flattening outlines, expanding strokes, running the scanline
  rasteriser.
- `"encode: staging"` — packing that coverage into the scratch sheet and the glyph atlas.
- `"encode: recording"` — the remainder, computed rather than measured: clip resolution,
  culling, atlas lookups, instance building, plan assembly.

The three sum to `Timings::encode` exactly, which a test holds.

**Why a switch.** Encode's parts interleave per command, so subdividing costs a clock
read per seam — about 0.2 ms over 5 933 commands. That is three times the entire encode
of a page of rectangles here, so always-on would have moved the number it reports by
300%. Turn it on for `--trace=frames` and leave it off otherwise; off costs one `Option`
check.

## What it says on our fixtures, which is a prediction to test on yours

3 675 curved fills at reading size, 1191×1684, RADV, release:

| | encode | geometry | staging | recording |
|---|---|---|---|---|
| 107 distinct outlines, cold atlas | 2.549 ms | **1.533** | 0.155 | 0.861 |
| 107 distinct outlines, warm atlas | 0.995 ms | 0.000 | 0.000 | 0.995 |
| 3 675 distinct outlines, cold atlas | 8.919 ms | **6.229** | 0.795 | 1.895 |
| 3 675 distinct outlines, warm atlas | 1.758 ms | 0.000 | 0.000 | 1.758 |

**It is geometry whenever the atlas is cold, and recording when it is warm.** The cold
all-distinct row is 2.4 µs a command — the same order as the 3.86 µs your least squares
found — which suggests your page turns are largely cold-atlas frames. If that holds on
your corpus, the lever is the rasteriser or the atlas's reuse across pages, and neither
is a threading problem.

## The other half of §13: which clock, and the unnamed remainder

Both of your suggested ways out, because both were cheap:

- `"target acquire"` and `"present"` are phases now, always on (two clock reads a frame),
  so the two host-side steps outside the three phases have names.
- `Timings::host_total()` returns `encode + upload + readback` — the three spans on
  **your** clock. `execute` is deliberately excluded, and the rustdoc says why:
  subtracting the adapter's clock from a host measurement is what left your `elsewhere`
  row a quantity you stopped believing. Subtract `host_total`, then read what is left
  against those two named phases and the device wait.

## Threads: taking one, never making one

Your answer is recorded as quorra's decision (ADR 0023) with your reasoning attached —
the `rayon` pool already sized to the machine, the confined worker that is
single-threaded because `glibc` sizes its arenas from a `/sys` read your seccomp filter
kills, `viewer-core`'s rule 4, and pool construction landing on the launch path that page
one already shares. **quorra spawns no threads, and if parallelism is ever wanted it will
take a pool rather than make one**, in the shape `create_instance` and
`create_instance_with` already established.

Nothing is added for it now, because the measurement above says the first move is not
more cores.

## Three optimisations came with this release

Not requested, but they change numbers you gate on, so they are worth knowing:

- **Layer textures are priced by depth, not count** (ADR 0020) — described above; pages
  refused with `FrameBudgetExceeded` may now draw, and `Counters::layer_textures` reports
  what a frame actually allocated.
- **The scratch sheet is as wide as it is used** (ADR 0021) — it used to be committed at
  the device's maximum dimension, so one 180-pixel tile moved 2.95 MB. A GPU-coverage
  frame with eight blobs went from 10.54 MB and 3.00 ms to 0.46 MB and 1.96 ms. The
  sheet's own bytes are charged to the frame budget now, including the gaps shelf packing
  leaves, so a page can be refused for a sheet it would really have allocated.
- **The readback reads once and divides never** (ADR 0022) — it copied the mapped range
  into a `Vec` the conversion then read once, and ran three integer divisions per pixel.
  **A dense page's offscreen frame is three times faster: 4.94 ms → 1.65 ms**, of which
  readback 3.84 → 1.32. This is the phase your corpus gate mostly measures, and on that
  gate the median page went 2.17× → 1.90× your CPU backend with no page changing verdict.

---

# Addendum 2, 2026-08-11: §14 answered, §17 closed, and the zoom cliff

## §17 needs nothing — two rasters of one page already work

Your section offered to close itself: *"If the second is already true — if calling
`render(..., Target::Readback)` twice against one device is simply supported and cheap —
then say so."* It is, and `quorra-gpu/tests/two_rasters.rs` now holds it so it stays
true:

- **Both rasters come back whole**, one per call, each its own `Raster`.
- **Resources are device-scoped**: upload the outlines once, reference the same
  `OutlineId`s from both display lists. The second pass's `bytes_uploaded` is strictly
  smaller.
- **The second interpretation pays no geometry at all.** The glyph key is
  `(outline, linear part, phase, rule)` and **colour is not in it**, so the pass carrying
  the complement of black hits every tile the C-M-Y pass rasterised. Measured with
  `instrument_encode`: `encode: geometry` is 1.772 ms on pass one and **0.000 ms** on
  pass two.
- **Neither pass changes what the other draws** — each raster equals the same scene
  rendered on a device that has drawn nothing else.

Two caveats, stated rather than buried. A frame whose tiles overflow the atlas can leave
the next pass cold (ADR 0024 below narrowed when that happens, so it is rarer than it
was). And each pass pays its own readback, which is irreducible when both rasters are
wanted — about 1.3 ms at page size after ADR 0022.

So: `personwithdog.pdf` and `bug1365930.pdf` need no change from this side. The refusal
is on your display list, and it can come off.

## §14 is implemented — `Compose::DestOut` and `Compose::Plus`

Exactly the two operators you asked for, and they were the smaller change here too:
**the pipelines already existed**, because our knockout lane *is* those two marks —
`(Zero, OneMinusSrcAlpha)` through a shape-only fragment entry, then `(One, One)`. This
release gives the scene vocabulary a way to ask for one of them alone.

**`DestOut` weights by shape, not by the paint's alpha.** Our shape entry point returns
coverage under the mark's clip and ignores the paint entirely, which is §11.6.4.2's
shape — so draw the object with every source of opacity removed and the weight is right.
A test holds it: the same mark with a quarter-opaque paint erases exactly what an opaque
one does.

Measured on a wedge with a diagonal edge, half-opaque object over an opaque backdrop,
worst premultiplied deviation from `P' = (1 − f) × P + S` across every pixel:

| | deviation |
|---|---|
| the staged pair | **0.77 of 255** — unorm rounding |
| the same object with source-over | **114.95 of 255** |

Your fixture pins the same phenomenon at 32; the size depends on the backdrop.

**Two positions refuse a staged mark**, both because they already stage the clause and
applying it twice is not a picture:

```rust
SceneError::StagedComposeUnsupported { compose, reason }
// reason: BlendNotNormal | InsideKnockoutGroup
```

A mark carrying a blend mode is in an implicit one-element group (§11.3.5), and a mark
inside a knockout group is already erased-and-deposited per element. Neither should be
anything you emit.

**And one thing we cannot refuse you for, so it is written down instead.** `Plus` alone
saturates — without the matching `DestOut` it drives a premultiplied channel past its
alpha, and one mark cannot tell us whether the other is coming. It is the first item in
this vocabulary whose correctness is the caller's obligation. `Compose::Plus`'s
documentation says so; the alternative you offered (a per-element shape channel) would
have kept the obligation inside the library, and if `Plus` ever gets used for something
that is *not* §11.4.6's second stage, that becomes the better design.

## The zoom cliff, which you may notice as a speed change

Not from your list, but it changes numbers you may gate on. Our glyph cache admitted
tiles by a **dimension** (128 px a side) while what it protected was a **budget**, so past
about 10× magnification every visible letterform left the atlas and was re-rasterised
every frame. Held at a magnification, encode went **13.6 → 0.65 ms at 12×** and
**19.4 → 0.50 ms at 20×**; a zoom sweep's worst frame went 35.8 → 16.2 ms. Admission is
now a share of `Options::atlas_budget`, so it scales with what you give us.

Taking that policy uncovered a correctness defect worth telling you about even though it
never reached your corpus: **the glyph key had no fill rule in it**, so the same outline
requested under non-zero and even-odd got the first tile twice wherever a subpath nests.
It is fixed and pinned. Nothing in the 957-page gate moved — 914 agree, 35 differ, 8
refused, 17 not comparable, differ list identical page for page — which is consistent
with your pages not hitting it, not with it being harmless.

`Counters::tiles` also counts now; it had reported zero since M5 shipped.

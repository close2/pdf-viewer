# 0700 — The lane follows the view

**Status.** Accepted. The hybrid coverage policy quorra's ADR 0080/0081 made possible
and deliberately did not design: which of quorra's three coverage lanes `auto` picks,
per frame, in `crate::surface::lane_for`.

## Context

Quorra now has three producers of the same coverage bytes, each measured on both page
shapes this project uses as poles (their ADR 0080/0081; the 890M):

| | still page (cache warm) | new magnification (everything cold) |
|---|---:|---:|
| `Cpu` (exact, atlas in front) | **best** — the atlas's 20–60× | dense text 8.84 ms encode; the 58k-fill page ~270 ms |
| `Gpu` (sampled winding) | — | 9.8 ms; §10.7.4-non-conformant pixels |
| `Compute` (exact, resident outlines) | pays per frame | **0.93 ms; ~150 ms** |

No single lane wins both columns, which is what makes the choice a policy; and the
regimes are legible to the host — the viewer knows whether the view moved.

## Decision

`--coverage auto` (the default) picks per frame:

- **The view moved** — any coefficient of the arrangement's transform differs, by bits,
  from the shown frame's — → **`Compute`**. Every cached tile is cold in this regime,
  and the compute lane wins it on both poles.
- **The view is the one being shown** (a chrome-only ask: selection, find bar, caret)
  → **the lane the shown frame used, unchanged.** Quorra keys a retained encode on the
  lane, so stickiness is what keeps a selection change a replay rather than a full
  re-encode; the direction of the stick is deliberately not a preference.
- **Nothing shown yet** (the launch path) → the pre-existing magnification rule.
  Time-to-first-page is a gate with numbers, the compute lane's first frame pays its
  pipeline compile and its outlines' residency, and the launch path defers everything
  it can.
- **A software adapter keeps the processor's lanes throughout** — llvmpipe runs the
  dispatch without the scanline rasteriser's shape and loses (600 against 229 ms on the
  worst page's zoom step) — read from the adapter description quorra formats, a string
  test with a named source to be replaced by a typed accessor when quorra grows one.

The settled sharpening pass (ADR 0699) takes `Compute` under `auto` on a hardware
adapter: it is a moved view by construction, and the lane having no atlas keeps the 2×
tiles out of it — the thrash that ADR's cost table watched for now cannot happen.

**The sampled `Gpu` lane leaves `auto` for the moved-view case entirely**: dominated on
the cold sweep, matched at held 100×, non-conformant where `Compute` is exact. It stays
reachable by `--coverage gpu` and in the launch rule until a first-frame measurement
moves it.

**And a fourth case, added by the first measurement below**: a moved view whose
magnification the atlas has already drawn — the linear part, by bits, in a short ring
of frames that went through the `Cpu` lane — goes back to **`Cpu`**, because quorra's
tiles are keyed by that linear part and survive until a repack: the revisit hits at
~69 ms where the compute lane re-rasterises at ~130. The common shape it catches is
zooming back out to the fit the document opened at. A revisit to a magnification only
the *compute* lane has drawn stays compute, correctly — there is nothing in the atlas
to come back to.

## Verified

- `lane_for`'s cases and the software guard are unit-tested, including both stick
  directions and the revisit rule's positive case.
- Windowed under Xvfb (llvmpipe): `auto` after two zoom steps differs from
  `--coverage cpu` by 335 pixels of half a million at ≤ 2 unorm steps — the documented
  atlas-phase quantisation, inside the relaxed contract (quorra's ADR 0082) with room
  to spare.
- Windowed on the owner's 890M through their measurement loop
  (`tmp/policy2-{auto,cpu}.trace.txt`; view changes driven by window resizes, each a
  new fit transform): under `auto`, view changes settle at **129–142 ms** with one
  first compute frame at 335 (pipeline compile and the outlines' residency, paid
  once); under `--coverage cpu` the same changes read **187–221 ms** cold and
  **68–71 ms** on an exact atlas revisit. So `auto` wins every novel movement by
  ~1.5×, holds a *uniform* latency where the CPU lane's is bimodal, and cedes only the
  revisit of a compute-drawn view — the case the atlas cannot serve either, and the
  case a per-frame hybrid inside quorra (atlas for repeated glyphs, compute for the
  rest) would be the design for.

## What this is not

Not the recording fix. A zoom step under `auto` still pays the walk — ~90 ms of clip
resolution, culling and instance building on the worst page — which is ADR 0368's
floor, now the largest remaining term, and the retained-scene question this policy was
sequenced before deliberately: the policy needed only numbers this session had, where
retained records need a design against the reprojection architecture.

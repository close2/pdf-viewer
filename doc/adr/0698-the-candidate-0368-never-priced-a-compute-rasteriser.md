# 0698 — The candidate 0368 never priced: a compute rasteriser, measured on the real adapter

**Status.** Accepted. A measurement round at the owner's direct request: it prices, it
records, and it builds nothing in this tree. The one thing it built lives in quorra —
their ADR 0078, below.

## Context

The owner asked, for the second time and generalised beyond one document:

> Can't we find an IR, GPU format, where we can send the whole page in perfect quality to
> the GPU and have it display and zoom completely on the GPU? Have we really exhausted the
> possibilities of the current GPUs?

ADR 0368 answered "build nothing" for `tmp/Entwurf.pdf`, and its refutation of the
device-side candidate (c2) was a measurement of **quorra's tessellation lane** —
`Coverage::Gpu`, Wallace winding, where a 52-segment outline is ≥ 5 000 bytes of vertices
against a tile of three pixels. That is the right refusal of that lane and it says nothing
about the other device-side shape: a **compute rasteriser** of the vello class, where the
scene's outlines are resident on the device and flattening, binning and per-pixel coverage
run as compute passes with the viewport arriving as a uniform. Nobody had priced that,
and this tree carries an instrument that can: `crates/render-gpu`, the retired vello
backend, still builds and still draws — `doc/gpu.txt`'s closing question ("how much of a
frame is CPU encoding versus GPU execution?") has an example to itself,
`render-gpu/examples/frame_split.rs`.

A second trigger arrived with the question: the owner's Windows trace of Entwurf
(`tmp/entwurf.trace.txt`, Intel UHD, DX12) shows one real frame in a sixteen-second
session, 9.0 s expected a frame, of which **6.4 s is quorra's transfer phase moving
4.9 MB** — ~58 000 `write_texture` calls at ~110 µs each, one per coverage tile. That
number is not about this question at all, and conflating them would have been the mistake:
it is a per-call price, fixed by batching, in any architecture.

## What was done in quorra first, so the architecture question could be read clean

Quorra's ADR 0078 (`render-lib` commit `5507493`): the atlas keeps a CPU sheet of its own
texels and the flush uploads dirty row spans — one `write_texture` per span, never one per
tile; a cold frame's shelves coalesce to one span. Measured on their own `examples/zoom`
(Radeon 890M, RADV): the cold sweep's worst upload 22.98 → 9.82 ms; every readback gate
unchanged. On this tree, through a temporary path-patch, `render-quorra`'s sixty tests
pass and Entwurf's cold headless frame is 295.6 → 249.2 ms. And windowed, on the owner's
display through their measurement loop, this tree's HEAD against quorra HEAD-plus-0078
(`tmp/run-on-gpu.stdout.txt`) against the Aug-16 baseline (`tmp/probe.entwurf.txt`, an
older build of both trees, so the delta is not the atlas change alone): **first present
2 720.9 → 1 041.6 ms; the cold frame's `scene` 619.6 → 121.7 ms and `device` 1 077.2 →
174.9 ms**, at the same 58 030 uploads. The DX12 number it was built for is the owner's
to confirm on the Windows machine once the pin moves — RADV is the adapter on which the
defect was smallest.

## The measurement

All on the owner's AMD Radeon 890M (RADV, Vulkan), headless, release, fastest-of-N; two
temporary probes, removed as ADR 0368's were and named here instead: a path-taking
`first_frame` variant against `render-quorra`, and a prefix-truncating `frame_split`
variant against `render-gpu` (valid because Entwurf's list holds no clip and no mask, so a
prefix is a scene).

**The dense text page** (ISO 32000-2 page 6: 5 933 fills, 107 outlines), vello at 2×,
1191×1684:

| | ms |
|---|---:|
| scene encoding (CPU, removable by a retained scene) | 1.35 |
| whole frame, readback included | 11.56 |
| the same target from one rectangle — the readback floor a window never pays | 8.26 |
| **the page's own drawing, device-side** | **3.31** |

Quorra's own zoom sweep on the same page and adapter (their `examples/zoom`, every tile
cold, worst frame, after their ADR 0078): encode 8.0 ms + upload 9.8 ms + execute 0.15 ms
≈ **18 ms**. So on the page shape this project calls typical, the compute rasteriser's
device-side cost of a *new magnification* is ~3.3 ms against a 8.33 ms refresh — a zoom
that re-renders every frame — where the shipping design pays ~18 ms of host work for the
same step.

**Entwurf** (58 009 commands, 3.0 M segments), vello over prefixes, encode / whole in ms:

| commands | at 0.55× (917×261) | at 1.5× (2500×711) |
|---:|---|---|
| 10 000 | 6.9 / 25.6 | 7.2 / 25.8 |
| 20 000 | 16.6 / 31.9 | 19.5 / 45.1 |
| 30 000 | 26.4 / 64.4 | 24.0 / 51.8 |
| 40 000 | 32.8 / 67.2 | 31.5 / 106.7 |
| 58 009 | **refused: a 69.9 MB slice against a 48 MiB buffer** | refused |

Extrapolating the linear region to the whole page: ~48 ms of CPU encode (which a retained
scene deletes) and roughly **50–75 ms device-side per magnification** — against 250–295 ms
for today's zoom step on the same adapter, and against ADR 0368's 235 ms floor-if-
everything-threads. At 3× not even the first 10 000 commands fit vello's buffers.

## What the numbers say

1. **The GPUs are not exhausted.** On the same integrated adapter, device-side
   re-rasterisation of a full transparency-correct page costs ~3 ms (dense text) to
   ~50–75 ms (the 58 000-command worst case) per magnification. For every page shape
   except the monster, that is a real frame *every refresh* of a zoom gesture — the thing
   ADR 0391 downgraded to "a picture every refresh" because no arrangement of the current
   design could afford a rendering. For the monster it is 3–5× today's step, taken off
   the interaction thread besides.
2. **ADR 0368's conclusion narrows rather than falls.** Its boundary argument (this
   tree's `scene` share is 2.5%) and its IR argument (page-space, curve-retaining, no
   coarsening) stand — the IR is *already* what a resident scene needs. What falls is
   only the sentence "a new magnification: no, and not by any change either tree can make
   cheaply": the change is not cheap, but it exists, and it is quorra's to make — a
   compute coverage lane beside the two it has, fed from the outlines it already keeps
   resident.
3. **Vello's refusal is the design brief, restated by a panic.** 48 MiB of hand-picked
   buffer against a 69.9 MB page is exactly the failure quorra's principle 6 exists to
   exclude (count-then-allocate, a refusal that names the limit). A compute lane built to
   quorra's contracts inherits the answer vello never had.
4. **The bars that remain are named, not waved at**: byte-determinism across adapters
   (both projects' CI rests on it, and f32 compute flattening does not promise it —
   the one question that must be answered by design, in an ADR of quorra's, before code);
   §10.7.4's no-disappearance rule expressed device-side (quorra's sampled winding lane
   already carries a recorded non-conformance there — the compute lane must do better,
   with analytic coverage); stroke widths, meshes and per-placement filters moving
   scene-side (quorra's own list, their `retained.rs` survival table); and pipelines
   compiled lazily so the launch path stays what CLAUDE.md demands.

## Decision

**Record the price, hand the design question to quorra, change nothing in this tree's
IR or boundary.** The next step is quorra's determinism ADR — GPU flattening that either
promises adapter-identical bytes or states the tolerance its oracle comparison will hold —
and it gates everything else. This tree's part is already true: the display list is
page-space and curve-retaining, and `TargetSpec` arrives separately, which is all a
resident scene will ever ask of it.

Meanwhile the shipping remedies for the two symptoms the owner reported stand on their
own: the conflation seam at low zoom is ADR 0308/0582's artifact and shrinks by half per
zoom doubling (nothing here changes it — though a compute lane with headroom is what would
someday afford the supersampled cure `doc/todo/11` item 5 priced and declined on the CPU);
and the Windows open is the DX12 per-call price, fixed in quorra's ADR 0078, pin bump
pending the owner's push of `render-lib` main.

## The instruments

`render-gpu/examples/frame_split.rs` (kept, unchanged, already in-tree), quorra's
`examples/zoom` (theirs), and the two temporary probes described above, removed with this
round as ADR 0368 removed its three. The Windows trace this began from is
`tmp/entwurf.trace.txt`; the Linux baseline it was read against is `tmp/probe.entwurf.txt`.

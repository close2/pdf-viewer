# 75 — The page resident on the device, and a zoom that sends only a matrix

Status: **open** — the evaluation is ADR 1992; items 1 and 5 are built (ADRs 1993, 2030); the rest are the order,
item 3 after item 2 (ADR 2008); the window's trace prints each frame's streams (ADR 2009).
Priority: the owner's, 2026-10-19: "evaluate: keep the page's intermediate representation resident
on the GPU and have a compute stage re-rasterise it at the new transform, so a zoom sends only a
matrix."
Witness: `/home/AI/witness/demo_heavy_engineering_A1.pdf`, the owner's, **not in the repository**
(4 408 406 commands, 4 398 932 of them the line lane's). Instrument: `render-raster`'s
`examples/zoom_frame.rs`, whose `crossed:` line divides `bytes_uploaded` by stream and whose `lanes:`
line counts the marks behind each (ADR 1992). The rows of `doc/checks/turn-path.toml` are the second
population. Clauses the design touches: §8.4.3.2, §8.5.3.2, §10.7.4, §10.7.5.

**What crosses on a zoom step today** is ADR 1992 section 2's table: on the witness the line lane's
instances are 95 % of the bytes under `Cpu` coverage and 87 % under `Compute`, the fills under
`Compute` are already resident outlines plus an 80-byte record each, and a page of photographs pays
its images' finer level once. Each item below is priced there; this file is the order.

## 1. Built — a replayed encode sends none of its instance streams (ADR 1993)

A retained encode keeps the device's copies of its rectangle, quad and line instances
(`raster_gpu::device::KeptInstances`, held by `retained::Held`), so a frame that replays it at the
same view reads them from the device. It is the mechanism every item below stands on — a stream
held on the device, keyed by the encode it was made from, given back with it — measured where it is
reachable today: the replay of the witness's page.

## 2. Line instances in page space, the view a uniform (ADR 1877 §5) — built, measured, taken back (ADR 2022)

**The owner accepted the change of bar (`doc/questions/A473`), and the build is not in the tree**: it
saved the witness's step its 264 MB of line bytes and 29–47 ms of transfer, and its walk was 60–80 ms
slower in most processes for no cause instructions, caches, branches, the makers or page faults
explain — and in the window, where a line the view does not admit became an instance, the step went
from 144–151 ms to 249–252. ADR 2022 has the figures and what is owed first: the walk's cost found by a
hardware profile or a bisected build, and the host's cull kept until item 3. The patches are outside
the repository, in `/home/AI/r1593-line-lane/`.

`line.wgsl` reads the endpoints after the line's own transform, the width and the clip rectangle in
the same space, and maps all three by a view uniform (a scale and a translation; a turned view stays
a re-encode). §8.4.3.2's zero width and §10.7.5's adjustment, which `raster::resolve_width` decides
per view on the host, move into the vertex stage, and the cull moves with them: an instance outside
the admitted rectangle collapses its quad. **The bytes move**: the device mapping is computed in
WGSL, which section 15.7.5 of that standard lets a driver fuse, so a line's endpoints may differ by
an ulp from the host's, held by raster ADR 0082's close best effort and the lane's bound against the
processor, and a kept frame equals a fresh one by construction. `doc/questions/Q473` asks the owner
to accept that change of bar. Saves the line stream's transfer on every step.

## 3. The op list built once, the view-dependent remainder re-encoded per view — after item 2

A retained encode whose line batches are view-free replays at a new view by writing the uniform and
re-encoding only the commands whose answer is the view's: glyph atlas keys, host-expanded strokes,
residue clips, image levels, compute tile records. On the witness that remainder is 9 474 marks of
4 408 406, so the walk that is the step's encode drops to the remainder's. The batches interleave
lines with the remainder in page order, so the remainder's instances are placed into the kept
batches' slots rather than rebuilding them.

**It stands on item 2 and is not built before it** (ADR 2008): until the cull is the vertex stage's,
a line the host culls leaves the list, so the batches' ranges are the view's — in the window on the
witness the admitted lines go 438 940 → 332 995 → 235 905 over two steps — and 40 of a line
instance's 60 bytes are device-space. Two things the item also owes, found there: the witness
leaves the record replay (ADR 0087) at its twenty-sixth command for a residue clip, so the kept
list is held apart from the record list or a residue clip's frame-wide rasterisation is made
replayable; and the step's encode to beat is HEAD's 114–121 ms (`Compute`, ADR 1992 §2), its walk
137.5 ms of 179.1 instrumented.

## 4. §10.7.4's dot asked per view on the device

ADR 1877 asks every dot drawn as a line again at the new view (13–40 ms for 28 684 on the witness).
Under item 3 the ask is the shader's: a dot whose disc reaches no level at this view takes the
substitution in the vertex stage, so no mark disappears and the host asks nothing.

## 5. Built — a residency budget that refuses by name (ADR 2030)

The kept streams are charged, before a buffer is made, against `Limits::max_resident_bytes`: the
host's request held to the frame's ceiling, since the adapter states no memory size and the largest
buffer it makes and binds is the figure it does state. A charge is given back when its encode goes.
An encode past the budget is drawn the same with its streams made per frame,
`RetainedScene::residency_refused` names the three numbers, and the window's terminal says it once a
scene. One page retained per device is always admitted, because the frame budget admitted its
streams; the budget bites where a host retains several encodes on a device or asks for less.

## 6. Strokes the line lane does not take, expanded on the device

A stroke with joins, dashes or a turned transform is expanded and rasterised on the host per view
into the coverage sheet, whose bytes grow with the area: the next stream at a deep zoom once items
2–3 have taken the lines. A compute expansion over resident outlines, as the fills' flattening is
(ADRs 0080, 0081), is the item; it is not priced until 2–3 show what is left.

## 7. Not owed

Fills under `Compute` coverage are resident already (records only); glyphs take the same lane; an
image's texture is resident per reduction level and a step writes only the texels it newly samples
(ADR 1493); a ramp is made once. `Cpu` coverage is the still page's lane, and a zoom gesture in the
window takes `Compute` (`viewer-ui`'s `lane_for`), so nothing above is owed to it.

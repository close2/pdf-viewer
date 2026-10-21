# 75 — The page resident on the device, and a zoom that sends only a matrix

Status: **open** — the evaluation is ADR 1992; items 1, 2, 3 and 5 are built (ADRs 1993, 2022, 2056, 2134, 2030);
the rest are the order; the window's trace prints each frame's streams (ADR 2009).
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

## 2. Built — line instances in page space, the view a uniform (ADRs 2022, 2036, 2056)

`line.wgsl` reads the endpoints after the line's own transform, the width and the clip rectangle in
the same space, and maps all three by a view uniform (a scale and a translation; a turned view stays
a re-encode). §8.4.3.2's zero width and §10.7.5's adjustment are resolved per view in the vertex stage,
which collapses the quad of a line the view admits nothing of. The device mapping is computed in WGSL,
which section 15.7.5 of that standard lets a driver fuse, so a line's endpoints may differ by an ulp
from the host's, held by raster ADR 0082's close best effort (the owner's `A473`). The line makers read
what they read from a 128-byte-aligned `Ahead` of their own (ADR 2036). **A walk with no encode of the
scene before it still culls** a line its view admits nothing of, so a page's first frame in a window
sends the window's lines and not the page's (ADR 2056).

## 3. Built — the op list once a page, a step's lines taken in runs (ADR 2056)

A walk that culled no line — the fit a page opens at — wrote the page's whole stream, and keeps it as
runs: consecutive top-level commands, consecutive instances, one chain, one soft mask. The encode of
another view of the scene walks every other command and takes each run in one step — the batch noted
once, the stream shared and adopted on the device, the plan's box asked per line only while it can
still grow — so a step from the fit sends no line byte and walks 9 474 commands where it walked
4 408 406. A line the new view does not admit stays in the kept stream and draws nothing.

**A window opened zoomed keeps its lines from the idle build** (ADR 2134): its first frame culls as
before, and once the view has been still as long as that frame took, the render thread walks the
page again at the same view culling no line, draws it into a texture nobody presents, and puts its
encode in the first one's place (`raster_gpu::Device::build_whole`), so the next step takes its lines
in runs. Last in the idle turn, after the sharp pass and the whole page drawn small, each of which
puts its own encode in the page's place; under the lane the step will take; declined once a scene
where it would keep nothing or would pass the residency budget.

**Selection built and measured** (ADR 2188): conservative 256-instance blocks omit
3,218,004 and 3,624,276 of the witness's 4,398,932 resident lines at 3× and 3.6×, without reordering
or re-uploading them. Three quiet cache-free pairs improve the content pass and retained whole frame;
idle construction costs another 5.0–13.4 ms. Optional metadata refusal preserves whole-run drawing.
Final turn, reference-window launch and both toolkit launch timing gates pass.

**Record replay remains deferred** (ADR 2189). The census now establishes the population: 957 of
968 first pages drew, 850 record-replayed, and 61 carried lines; eleven refusals remain uncensused.
The tested prototype eliminated 983,040 uploaded line bytes per changed view on its largest witness,
but did not consistently improve whole-frame latency, so its extra reuse was removed. The exact
patch, tests, exports and all timing attempts are preserved at `/home/AI/r1676-deferred-replay/`.
A future attempt must explain the measured variance and demonstrate worthwhile benefit; it no
longer needs to rediscover whether this population exists.

## 4. §10.7.4's dot asked per view on the device

ADR 1877 asks every dot drawn as a line again at the new view (13–40 ms for 28 684 on the witness).
Under item 3's runs the ask is the shader's: a dot whose disc reaches no level at this view takes the
substitution in the vertex stage, so no mark disappears and the host asks nothing.

## 5. Built — a residency budget that refuses by name (ADR 2030)

The kept streams are charged, before a buffer is made, against `Limits::max_resident_bytes`: the
host's request held to the frame's ceiling, since the adapter states no memory size and the largest
buffer it makes and binds is the figure it does state. A charge is given back when its encode goes.
An encode past the budget is drawn the same with its streams made per frame,
`RetainedScene::residency_refused` names the three numbers, and the window's terminal says it once a
scene. One page retained per device is always admitted, because the frame budget admitted its
streams; the budget bites where a host retains several encodes on a device or asks for less.

The compute lane's segment arena is held too (ADR 2037): a release that would leave more hole words
than live ones compacts it on the device, so its buffer is at most four times its live bytes or its
starting 1 MiB. A zoom releases nothing; the holes are a turn's and the cache's eviction's.

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

## 8. Estimate provenance and live surface observation

A fixed GPU estimate may be a seed or a retained measurement. Samples apply only to matching
view, output extent, command population and coverage; incompatible feedback uses an explicitly
labelled seed. Arrangement trial flags stay independent, so a new view does not introduce an
automatic GPU probe. See [ADR 2196](../adr/2196-a-frame-price-names-the-view-that-measured-it.md)
and the [validation record](../history/1681-frame-estimates-name-their-measured-view.md).

The remaining obligation is a quiet hardware surface observation of the owner's large-document
fit → deep zoom → pan → fit gesture, then page/document and coverage changes. Inspect the displayed
provenance alongside actual frame work, including held scenes. Headless read-back eligibility
does not establish live-surface behavior or a performance improvement. Do not force extra device
work or scale the whole graphics cost by visible share merely to make the number change.

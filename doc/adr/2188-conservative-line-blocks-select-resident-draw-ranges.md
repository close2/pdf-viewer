# 2188 — Conservative line blocks select resident draw ranges

Status: **accepted** at the merge that took it, `020a86f3` (line added by ADR 2260).

Session 1676, slot 6 of batch 1671–1678. Context: ADRs 2022, 2056, 2134 and `doc/todo/75` item 3.
Accepted: quiet comparison, final correctness and canonical shipping timing gates passed. No clause or ledger row changes: the selected frame must be
byte-identical to the existing frame on the same adapter.

## The current premise

The fresh HEAD profile on the Radeon 890M through RADV still submitted all 4,398,932 resident line
instances on the engineering witness's retained zoom steps. At 3× its content pass was 24.678 ms;
at 3.6×, 16.858 ms. The idle build cost 379.6 ms in that run. These are a new diagnostic baseline,
not the older ADR 2134 timings and not an accepted before/after improvement.

The instrument is `render-raster`'s `zoom_frame`, page 1 of `demo_heavy_engineering_A1.pdf`, base
scale 0.3775, a 900×1100 window, sequence 2.4,3.0,3.6, Compute coverage, idle build enabled,
24 encode threads and three rounds. Both `RAYON_NUM_THREADS=24` and
`ZOOM_FRAME_ENCODE_THREADS=24` are set; the former alone does not set the encoder's Options.
The observed frame digests were `e0ff5b9a63ba9363`, `343c65dbeca606e8` and `374d61520afa3f34`.
Final paired timing, initial/idle construction cost and digest evidence belong to the shared
clock hold, with the round history recording the accepted result.

## Decision

Partition the resident 60-byte line-instance stream into consecutive blocks of 256 instances.
Each block holds endpoint extrema, maximum scaled pen radii and maximum fixed pen radii.
Adjusted strokes contribute both their scaled pen and the possible fixed half-pixel pen.
The 4,398,932-instance witness needs 17,184 bounds: 549,888 payload bytes, plus shared ownership.
Bounds are built once from the resident bytes and shared by later encodes. Their construction
reads the stream once; the benchmark must account for that cost rather than hide it behind idle.

At a new view, each kept run intersects these global blocks and emits only admitted ranges,
coalescing adjacent ranges through the existing batch builder. No instance is reordered, rewritten
or fetched through a new shader indirection. Partial blocks at run boundaries stay in their
original order. A block may retain invisible lines; selection is conservative, not exact culling.

The predicate tests the viewport, leaving the shader to map and intersect each instance's own
clip. It promotes the actual f32 uniforms to f64, maps the endpoint bounds, inflates for pen reach
and the shader's short f32 arithmetic chains, and retains uncertain, nonfinite or potentially
overflowing cases. Touching a viewport edge remains admitted. The guard covers fused and separate
mapping operations and flushed subnormals; it is not permission to change shader arithmetic.

The frame budget admits the metadata before allocation, and retained accounting includes it.
If that optional allocation does not fit, the frame draws the original complete runs and reports
`line_bounds_refused`. This optimization cannot introduce a fatal refusal for an otherwise
drawable frame. `line_instances_omitted` counts skipped draw instances separately from
`lines_kept` and the original lane count. Both are printed by `zoom_frame`.

## Paired measurements — not accepted

Three interleaved HEAD/candidate process pairs used the instrument settings above. Each listed
frame sample is the instrument's minimum of three rounds, not a tail-latency measurement.
The candidate contained both selection and the separate readback-cache prototype; it contained
none of this batch's font, image-conversion or cropped-alpha changes, and no later record replay.

| Pair | 3× content pass, HEAD → candidate (ms) | 3.6× content pass (ms) | Idle build (ms) |
| --- | --- | --- | --- |
| 1 | 24.6 → 21.5 | 17.2 → 13.3 | 346.6 → 349.4 |
| 2 | 24.7 → 21.6 | 16.9 → 13.3 | 331.8 → 335.6 |
| 3 | 24.5 → 21.6 | 17.2 → 13.0 | 346.8 → 372.7 |

All frame and idle-build digests matched HEAD in all pairs. These timings are **not accepted as
performance evidence**: the subsequent turn gate observed 76–94% GPU activity outside its own
children and 80–83% during them, exceeding its 20% quiet-machine threshold. The clock scheduler
excludes other agent walks, not external GPU work. Those samples cannot support a performance verdict; the separate quiet rerun below supersedes them. Even within the unaccepted samples, total frame time at 3× did not improve in every pair
(170.9 → 181.1 ms in the third), and extra construction cost is visible above.
The raw logs and source exports are under `scratchpad/r1673/ab` and `scratchpad/r1673/{head,gpu}`
while the batch is open; the history preserves the final verdict. The clock hold was stopped
*after all twelve frame instruments exited 0*: invoking the launch gates through Cargo began
an unexpected feature-graph rebuild. Launch/turn/toolkit gates therefore run from the already
compiled matching test binaries in a separate clock hold. GTK/Qt bands failed and the turn gate
refused timing judgment because of external GPU activity; that hold was stopped. The canonical
shipping rerun below subsequently passed; these earlier timing samples remain unaccepted.

## Quiet cache-free comparison

A fresh cache-free selection-only export and a selection-plus-replay export were made from HEAD
`5ded4916304afc8b9dd931fa8b21a86db6644ef8`, preserving HEAD font/model/alpha code in both.
The selection-only export changes nine source paths; the replay export twelve. Their SHA-256
manifests and matching sandbox workers are recorded in `scratchpad/r1676/cf-{before,after}.export`
and the corresponding `.sources.sha256` files. The existing frozen HEAD binary is the baseline.
Only workers and `zoom_frame` were built; no UI build or other agent walk overlapped measurement.

The first cache-free attempt stopped at a post-run GPU mean of 20.3%; a no-build repeat stopped at
20.15%. Both exceed the unchanged 20% threshold. Driver lag can include the child's own tail;
these small overruns do **not** prove an external process caused them. Rejected samples stay
rejected. The final attempt adds the same fixed ten-second rest **before every process**, then
uses the same immediate 20 readings at 25 ms cadence before and after it. It never waits after a
failed sample to rescue that sample. Three interleaved triples ran under one exclusive clock hold,
nice 0, exit 0 after 235 seconds, peak 2.61 GiB. All 34 activity means were 3.0–18.85%; one-minute
load was 1.04–4.56 against ceiling 12. Every engineering and replay digest matched.

These are the complete final engineering samples, each the instrument's minimum of three rounds.
H is HEAD, S selection only, and R selection plus replay; none includes the rejected readback cache.

| Pair/view | Content pass H → S → R (ms) | Whole frame H → S → R (ms) |
| --- | --- | --- |
| 1 / 3× | 29.5 → 26.3 → 25.8 | 108.5 → 106.8 → 104.4 |
| 1 / 3.6× | 21.7 → 17.6 → 18.0 | 99.7 → 95.9 → 99.6 |
| 2 / 3× | 29.8 → 26.2 → 25.9 | 110.1 → 107.0 → 102.6 |
| 2 / 3.6× | 20.8 → 18.2 → 17.9 | 104.0 → 101.4 → 95.1 |
| 3 / 3× | 30.4 → 26.7 → 24.2 | 114.9 → 103.0 → 106.8 |
| 3 / 3.6× | 22.0 → 17.8 → 16.6 | 102.3 → 98.0 → 99.6 |

Idle construction H → S → R was 235.2 → 248.6 → 244.5 ms, 237.0 → 242.0 → 244.3 ms, and
243.7 → 250.7 → 258.4 ms. First-frame totals were 579.5 → 578.4 → 556.4 ms,
580.4 → 571.7 → 559.7 ms, and 518.4 → 493.6 → 508.8 ms. Selection omitted 3,218,004 and
3,624,276 of the 4,398,932 kept instances at the two retained views, with no line re-upload.
This supports retaining selection: every final paired content pass and retained whole-frame
sample improves, at a measured 5.0–13.4 ms extra idle construction cost. It does not establish a
sub-millisecond content pass, a universal latency gain, or a tail-latency guarantee.

Earlier complete prefixes remain robustness evidence, not substitutes for the final three-pair
set: selection content passes improved there too, while replay had an unfavorable 52.1 versus
31.6 ms whole-frame sample (ADR 2189). Logs for all three attempts remain separately under
`scratchpad/r1676/cachefree-{ab,repeat,rested}`. The final shipping launch/turn/toolkit gates below
passed independently; this comparison alone does not waive those bands.

## Validation and limits

The stateful selection tests passed: four spatial blocks with fixed and adjusted widths,
translucent overlaps, edge crossings, zoom, reflection, quarter turns and return to fit all drew
fresh-device bytes. Zoomed frames omitted at least one block without uploading a line byte.
A budget admitting exactly the original line-instance allowance declined bounds and preserved
pixels under CPU coverage. Five arithmetic tests passed, including 200,000 deterministic
fused/unfused comparisons, fixed anisotropic pens, large cancellation, nonfinite and overflow
retention, and allocation charge consistency. The final scoped chain passed formatting, warnings-denied
Clippy and all 876 GPU/render-raster tests (five skipped), exit 0. Before replay was deferred, the cache-free scoped rerun passed 868 tests (five skipped), and the
whole workspace passed 7,519 (53 skipped). All six combined pixel arms passed with no error increases; the only additional
changes beyond the font fix were two pages attributed by revert controls to alpha demultiplication.
These checks do not replace the separately passed launch/turn/toolkit timing gates.

The deliberate fused-arithmetic reference probe initially lived under `src`, and the unchanged
`mul_add_hazard` source scan correctly rejected it. It now lives in
`raster/crates/raster-gpu/tests/common/line_bounds.rs`, included only as a test module; no production allowlist
was widened. The frozen performance export predates that test-only relocation and the later
record-replay change (ADR 2189), while containing the same production block-selection arithmetic.

Cold corpus arms cannot exercise a stale retained view. The final gate therefore includes both
ordinary pixel arms and stateful kept-versus-fresh selection witnesses. The original 257/259-line
fixture exercised the now-deferred record-replay route and is archived with that prototype.
The shipping `raster/crates/raster-gpu/tests/kept_line_order.rs` adapts the same unaligned boundaries
to scene walking, with translucent fills between runs, CPU and Compute coverage, view changes,
reflections and quarter turns, checking fresh pixels, 1,032 kept lines and zero line upload. A speedup
is accepted only with unchanged witness digests and passing existing launch, turn and toolkit gates
under the shared clock discipline.

Record replay reuse was subsequently removed (ADR 2189) because its timing did not justify its
extra state. Selection remains; the final tree matches the selection-only S column above in its
GPU production paths. Scoped correctness and combined arms were rerun after that removal; the
pre-removal counts above remain distinct from the final-tree results below.

After removal, a scoped suite reported 864 passed and one failed: the expanded tight-budget fixture
still depended on the deferred Compute record reuse. Adding visible or empty groups to force scene
walking introduced a separate compositor texture-budget refusal (135,428 and 103,572 bytes), so
those attempted fixture changes were discarded. The original CPU-only optional-bounds fixture is
restored exactly, preserving its refusal, kept-count, omitted-count and fresh-pixel assertions.
Both stateful selection tests still cover CPU and Compute. Final scoped fmt and warnings-denied
Clippy passed, and all three selection witnesses passed, exit 0; the final workspace/arms evidence
below is measured separately, not inferred from this focused repair.

## Final selection-only correctness evidence

After replay removal and the fixture correction, workspace formatting and warnings-denied Clippy
passed; final workspace nextest reported 7,517 passed and 53 skipped, exit 0: 102.784 seconds of
tests, 202-second bounded hold, peak 9.33 GiB. All six shipping pixel arms passed: CPU/GPU/Compute each covered 968 comparable pages at 1× and 965 at 4×. Every page key,
digest, rounded error and sorted exclusion diagnostic exactly matched the previous combined
candidate. Against the original baseline, no errors increased; font deltas and the two independently
attributed alpha pages remain the complete set of changes. The arm hold exited 0 after 1,549 seconds,
peak 3.76 GiB. Source inventory: 349 paths accounted for, zero omissions; manifest SHA-256
`a627da3e325bac9ec265d52420e86de9059c5ca698bdf759bbc70733b4cec93e`. Evidence and worker/binary
hashes are in `/home/AI/arms-1679-selection-final/`; its corpus/vendor links are inventoried external
inputs, not immutable copies. These final-tree results supersede no rejected timing sample.
The canonical shipping timing gates subsequently passed on the same production source, checkpoint
`2b6b0939`, using the batch runner's exclusive clock holds and nice 0:

- `t2-turn_path`: exit 0, two tests passed, 186-second hold (94.37 seconds of tests).
- `t2-launch_path`: exit 0, 361-second hold; six documents, 59 banded figures, zero outside,
  zero absent/not-counted/unmeasured script stages. Thirteen figures were not judged under the
  existing carve-out; none was unjudged for load.
- `t2-toolkit_launch`: the combined GTK/Qt Cargo command exited 0, 293-second hold.

Because the canonical summary alone does not prove that the toolkit figures were judged, the
current shipping test executables were identified through their release `.d` source provenance
and run directly, with matching shipping window binaries and sandbox/script workers, under a
fresh exclusive nice-0 hold. No feature-graph rebuild occurred. Both explicitly reported `judged`:
GTK load 4.01 before/after, Qt 3.26 before/after. All seven banded figures per window were present
and within range, with nine fresh launches per lane (18 per window). External activity guards read
0% GPU before each and 2.05%/2.25% afterward. The hold exited 0 after 26 seconds, peak 0.18 GiB.
Full output, the seven figures and nine samples each, dep-info provenance, and binary/worker hashes
are preserved at `/home/AI/toolkit-judged-1676/`. This closes the unjudged-success ambiguity rather
than treating exit 0 alone as proof that load permitted judgment.

The recorded gate log is `/home/AI/batch-gates-1671-final.log`. Selection is accepted on this
combined evidence; replay reuse remains deferred. No earlier rejected sample is rehabilitated,
and neither the observed gain nor the passed bands establish a universal latency guarantee.

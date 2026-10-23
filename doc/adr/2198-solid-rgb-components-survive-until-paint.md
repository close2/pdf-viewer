# 2198 — Solid RGB components survive until paint

Status: **accepted** at the merge that took it, `b9d05ee8` (line amended by ADR 2260). The round wrote “accepted; final batch integration pending”. Session 1682.
Context: ADR 2187; ISO 32000-2 §11.7.5.3; stability review 1679.

The black-generation bullet puts the functions at the painting operation. A cached
colour selected by `rg` cannot answer that requirement after `gs` changes the pair.
The previous state retained only the result, so invalidating it could not recover
its inputs. RGB (0.2, 0.4, 0.6), BG(k)=1−k and UCR(k)=k/2 owe CMYK
(0.6, 0.4, 0.2, 0.6), whose device-press result is [52, 62, 79]. Selecting RGB
before the pair instead painted [51, 102, 154].

Retain the resolved DeviceRGB components beside each fill/stroke colour. All three
selection routes update them: device operators, colour-space initial values, and
`sc`/`scn` including component-count recovery. `q`/`Q` copies and restores them with
the pair. `solid_colour` resolves them in the current interpreter context when a
path, text mark or solid image stencil consumes the colour; an inherited Form thus
uses its current group too. The stencil's resolved colour participates in the raster
cache key. Other images keep their existing conversion path. Marks-only predicates
can still read cached opacity: a DeviceRGB source is opaque before constant alpha.

Uncoloured pattern cells install an already-converted tint, so they clear inherited
RGB source metadata. This protects their tint from the parent stream's source;
it does not fix the tint conversion's separate missing dependency (ADR 2199).
Non-RGB cached colours and the deliberate profile bypass remain outside this fix.

Unconditional conversion at every paint was rejected by three quiet paired runs on
fixed baseline 020a86f3. For twenty interpretations of 5000 repeated fills, default
CMYK cost 59.803/61.799/112.718 ms before versus 683.177/369.039/567.387 ms after.
Stated-pair CMYK was 62.751/62.004/117.467 versus 106.510/68.903/68.607 ms; ordinary
RGB was 23.372/23.157/24.476 versus 22.827/20.580/20.461 ms. All values are retained,
including the slower third baseline. Correctness did not justify the default-CMYK loss.

The revised candidate keeps two interpreter-owned slots, one fill and one stroke.
Each holds exact RGB component bits, the existing complete `Conversion` and a
pre-alpha `Color`. Equality includes the target/press, spots, rendering intent/BPC,
output/group profiles, current function identity and separation mode. Holding the
conversion keeps identity-bearing references alive. There is no manual invalidation:
a source or context change misses, including q/Q and group entry/exit. Constant alpha
is applied after reuse; a stencil's resolved colour still enters the image-cache key.
Both slots are fixed-size and disappear with the interpreter; resumptions recompute.
`Conversion::paint` calls the same `Compositing::paint_beside` with the same target,
spots, rendering and pair as the prior solid conversion helper.

The first cache revision passes nine stateful regressions; baseline fails five of
those nine, including same-source target changes. Its second quiet comparison still
costs more: default CMYK before 115.938/59.207/57.932 ms versus after
105.481/110.120/127.057; stated pair 118.712/59.627/58.996 versus
128.635/129.596/129.576; ordinary RGB 22.946/23.207/21.716 versus
21.899/24.586/41.761; alternating RGB in CMYK 464.148/458.763/263.495 versus
643.160/726.265/468.885. All runs and the noisy values remain evidence, not exclusions.

The next revision compares a borrowed complete context on hits, using the same
seven-field key as owning `Conversion`; only misses retain references. Selection's
existing conversion seeds its fill/stroke slot on all three assignment routes, so
first paint does not repeat it. These target ownership churn and duplicate conversion;
their performance contribution is a hypothesis until the next paired comparison.
Borrowed/owned equality tests vary every context field, including profiles, spots
and function identity. Nine paint regressions and both key tests pass; warnings-denied
model/colour Clippy passes. The cache stores pre-alpha colour; a reused-alpha paint
must exactly match a cold equivalent and remain within one byte of ideal source-over.

Pictures show the stale blue patch become the clause-derived dark patch while the
other region stays unchanged. The original baseline fails four of the initial seven
regression cases and the first candidate passes seven.

The short third comparison remained bimodal even on unchanged baseline. A fresh
attempt refused all timing before collecting a sample when CPU load was 16.935,
above the unchanged limit of 12 (GPU 4%). The next hold used a fixed 60-second
settling period, then the same ten-second rests and load/GPU guards for twelve
balanced before/after pairs pinned to CPU 2. Exact existing release binaries were
hash-checked; no rebuild or source change separated those measurements. All 48
before/after guard samples passed, and every timing sample is retained.

Whole-interpretation medians, milliseconds for twenty interpretations of 5000 fills:

| Case | Before median (range) | After median (range) | Median paired ratio | Median paired delta ms |
| --- | --- | --- | --- | --- |
| Ordinary RGB | 20.592 (19.778–27.649) | 22.615 (21.764–24.145) | 1.08475 | 1.741 |
| Default CMYK | 58.2845 (57.564–62.729) | 60.9365 (59.936–66.011) | 1.03922 | 2.2885 |
| Stated CMYK | 59.1245 (57.836–61.952) | 61.9675 (60.901–63.254) | 1.04387 | 2.592 |
| Alternating RGB in CMYK | 261.912 (257.041–286.360) | 267.6785 (262.028–303.950) | 1.01597 | 4.1785 |

The amortized deltas are 17.41/22.89/25.92/41.79 ns per paint, not isolated lookup
costs: the timer includes content interpretation and command construction; document
opening and rasterisation are outside it. Aggregate after/before ratios are
1.02807/1.03950/1.04943/1.02550. Paired-ratio ranges are respectively
0.80437–1.17766, 0.95576–1.09949, 1.02076–1.08269 and 0.92120–1.16575.
This measured cost is accepted for the required current-state correctness. No
performance-neutral claim is made, and further cache state or API changes are not
justified by these measurements. The pinned hold exits 0 after 339 seconds; scoped
model tests pass 2134 with 21 skipped, plus both borrowed-key equivalence tests.
Combined corpus, six-arm and launch integration remain the batch's final gates.

Reproduction: `black_generation_order --cost` runs the four interpretation cases.
Fixed baseline is 020a86f3; frozen before-cache/after-borrow sources and eleven-path
hash manifests, exact binary hashes, all three short comparisons, the zero-sample
refusal and all twelve pinned pairs are retained under `scratchpad/r1682/` during
integration. `paired-pinned-settled/summary.json` preserves every per-case sample;
`summarize-pinned.py` derives the table without dropping outliers.

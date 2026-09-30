# 1409 — A mark with no geometry queues behind the marks before it, and a zoom step's strokes are the fan-out's

Status: accepted. Session 1286. Extends ADR 1395 (raster's encode seam, `encode/parallel.rs`, raster's
ADR 0051); supersedes nothing. Context: `CLAUDE.md` principle 2 (latency before throughput);
`doc/todo/36`, `doc/todo/46`, `doc/todo/47`; `doc/habits/measuring.md` 45, 48, 52, 54; traps 50, 65, 66.
Code: `raster/crates/raster-gpu/src/encode/parallel.rs` (`Place::Follows`, `Place::Rect`,
`Place::Compute`), `encode/parallel/commit.rs` (`follow_queued`, `commit`, `drain_queue`,
`deferrable_bounds`), `encode/instance.rs` (`push_rect_instance`, `write_rect_instance`),
`encode/fill.rs` (three call sites: `fill_solid` asks `follow_queued`, `fill_compute` queues;
`SolidFill` is `Copy`).
Tests: `raster/crates/raster-gpu/tests/encode_threads.rs`
(`a_text_page_is_the_same_bytes_at_every_thread_count`,
`the_compute_lanes_strokes_are_the_same_bytes_at_every_thread_count`).

## 1. What kept a text page's turn on one thread

A probe (a `#[track_caller]` print in `drain_queue`, in a measurement export only), ISO 32000-2
page 101's turn, one frame: **85 drains forced by `prospect_for`'s repeated-key guard, carrying
57 831 of weight, and 91 by `push_rect_instance`, carrying 28 690** — none reaching
`PARALLEL_FLOOR_SEGMENTS` (4 096). The two are one wall seen twice: with only rectangles queued
the guard forces 95 drains (85 434, none past the floor); with only repeats queued the rectangles
force 91 (86 521, three past it). Turn row, `frame_budget`, pinned, min of 5, two alternations:
neither alone moves it (10.15–10.36 and 10.13–10.24 ms against 10.29–10.31); both, 7.03–7.05 —
the page's glyphs become one drain of 84 355 at the frame's end.

## 2. The constructions

**A rectangle instance behind a queued mark is a job** with no geometry (`Place::Rect`), written
at its commit; with nothing queued it is written at once, as before.

**A repeat of a queued atlas key is a job** (`Place::Follows`) that reads the atlas entry at its
commit. Under `Coverage::Cpu` with no residue clip, the walk's lane for a solid fill turns on
`AtlasStore::prospect` alone; of what that reads, only the entry depends on the queue — the tile's
admission is its size against the atlas's extent (asked at the walk, `admits`), and the room probe
is asked only where there is no entry. So where the commit finds the entry, the one-threaded walk
would have drawn the placement from it; where it does not (the first placement made no geometry,
or found the atlas full), the fill rides in the job and is walked again from the commit, at the
point in the order where the walk asked. A commit that queues is why `drain_queue` loops, and why
the rectangle, repeat and compute commits drain before they read or write: a walk-again's job is
earlier in the order. Planted: skipping the walk-again, the rectangle commit's drain, or the loop
each fails `a_text_page_is_the_same_bytes_at_every_thread_count` (the full-atlas arm); the repeat
commit's own drain guards the room probe, which llvmpipe does not ask, and no fixture reaches it.

**A zoom step's strokes are the fan-out's.** `deferrable_bounds` answered only under
`Coverage::Cpu`; on `Coverage::Compute` every stroke and every curve-clipped fill was flattened,
expanded and rasterised on the walk's thread, because the compute kernels fill outlines and
`gpu_lane_admissible` answers no on that lane — so it drew with the processor exactly as the
page-turn lane does, and now leaves the thread the same way. The fills the kernels take drained the
queue in their charge, so **a compute tile behind a queued mark is a job too** (`Place::Compute`),
seated at its commit. On that lane a stroke whose pieces tile its set now keeps its integral as on
the page-turn lane (ADR 1397): the two lanes' strokes are one construction.

## 3. Numbers

Callgrind, one round of `frame_budget`, whole process, arms `md5sum`-distinct, own target dirs:

| page | total M Ir | walk thread (`Encoder::command`, incl.) | the scope's workers |
|---|---|---|---|
| p101 | 856.9 → 858.1 | 153.8 → 51.4 | 0 → 92.0 |
| Type 3 page | 2 086.0 → 2 087.3 | 1 101.0 → 25.3 | 463.2 → 1 468.3 |
| `issue14415.pdf` | 1 423.4 → 1 394.3 | 569.5 → 32.0 | 289.2 → 748.3 |

Every raster function within 1% except `Topology::of` on `issue14415.pdf` (216.0 → 205.5 M): its
compute-lane strokes that tile no longer ask. `stroke_subpath` 801.2 M on the Type 3 page, both arms.

`frame_budget`, base against this change in one sitting (load 1.9–2.1), pinned min of 3 × 5,
budget / encode ms (unpinned budget in brackets):

| page | row | base | this change |
|---|---|---|---|
| ISO p101 | turn | 9.89 / 7.31 (9.93) | **6.78 / 4.07** (6.51) |
| `issue19802.pdf` | turn | 6.90 / 5.38 (6.88) | 4.13 / 2.60 (4.11) |
| `issue14415.pdf` | turn | 18.34 / 12.93 (18.90) | 12.13 / 7.03 (11.46) |
| | step | 21.31 / 19.97 (21.24) | **6.06 / 4.61** (5.79) |
| Type 3 page | turn | 15.89 / 14.55 (16.00) | 11.23 / 9.79 (9.08) |
| | step | 54.00 / 53.42 (53.74) | **12.65 / 12.05** (10.71) |

Mesh, photograph, plans and `images.pdf` rows unmoved within the runs' spread.

**Exactness at the corpus's scale** (`render-raster --test corpus`, which draws every page again at
one encode thread, trap 66): at 1× no page differs; at 4× on the page-turn lane 353 pages differ at
HEAD and 354 with this change alone, and with ADR 0093's room probe switched off (a measurement
export only) **none in either arm** — the whole of that population is the probe's (ADR 1395
section 5, ADR 1407), which is asked where an atlas entry is absent and which this change leaves
where it was. The compute and sampled lanes at 4×: none.

## 4. Measured and dropped: the three-valued outline cache

ADR 1397 named it: keep the outline's answer as yes, no or cut short, and let a curve-clipped fill
take a yes answered to the end. Built and measured on the Type 3 page with every lever above:
2 087.1 → 2 083.9 M (−0.15%), `Topology::of` 103.4 → 101.4 M. On the page-turn lane nothing is
filled inline — every `fill_mask_settled` call on that page comes from `rasterise` (368) or a
clip chain's intersection (75) — and the regions still asked are strokes' pieces, which are not
the outline. It
does not pay for a second answer beside ADR 1389's; not kept.

## 5. Left

- The repeat is read at the commit on `Coverage::Cpu` only; the other lanes drain as before.
- ADR 1407's room-probe construction and this one share `drain_queue` and `fill_solid`; the 4×
  page-turn corpus at 1 and N threads is the one run that judges the two together.
- The Type 3 page's turn and step are one fan-out of `stroke_subpath` (801 M): ADR 1375's tiling is
  the cost, and ADR 1397 section 1 prices it.
- p101's turn is 3.7 ms of encode past the fan-out: the walk and the commit (atlas inserts, instance
  bytes) are serial by construction (ADR 0034's order).

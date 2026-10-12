# 1859 — A page is cut into as many strips as its pool is wide, and a toolkit window's zoom is priced as the band it shows

Session 1511. Status: **accepted**; section 2 **built**, section 4 **priced, not built**. The project
owner's target for batch eighty-one is performance — "the first frame, zooming in and out, the CPU
renderer and the GPU path". Amends ADR 0218's reading of the processor count; ADR 0219's property,
ADR 1742's stated counts and ADR 1758's bound stand. Context: ADRs 0137, 0139, 0218, 0219, 1742,
1817, 1825, 1841, 1843; traps 1, 134. Code: `crates/render-cpu/src/lib.rs` (`plan_strips`,
`MAX_STRIPS`). Instrument: `crates/pdf-model/examples/strip_count_price.rs`. No ledger row moves.

## 1. What the planner read

`plan_strips` asked `available_parallelism` for the strip count where a caller stated none, capped
at `MAX_STRIPS` (16) and at a page's rows over 64. The strips run on the rayon pool the draw is
called from. The two numbers agree in every shipped configuration: the global pool's default width
is `available_parallelism` too, an affinity mask or a cgroup quota narrows both alike. They part
where a pool is narrower than the machine — `RAYON_NUM_THREADS`, which `tools/bounded.sh` sets to
four for every locked run, or a pool a caller installed. There the page was cut into sixteen strips
for four threads. Each strip replays every command that reaches it (`pdf_render::replay_ratio`);
twelve of them bought only the balance work stealing gives a division the cost estimate got wrong.

## 2. Decision

**The count asked for is `rayon::current_num_threads()`**: the width of the pool the strips are about
to run on, which inside an installed pool is that pool's and outside any is the global one's.
`MAX_STRIPS` and `MIN_STRIP_ROWS` bound it as before, and the planner may still grant fewer. ADR
0218's rule is unchanged in substance: the global pool's first use asks `available_parallelism`, which
reads `/proc/self/cgroup`, so a caller in a confinement with no filesystem states its count with
`with_strips` and the pool is then not asked at all. Every gate that holds pixels states its count
(ADR 1742), so no digest a gate holds can move.

## 3. Evidence

The witness, one clock hold, pinned to the eight fast cores (0–3, 12–15), each stated count drawn in
turn three times, the quickest in ms. Load 9–14 from siblings' builds, so these figures are not
judged against any band (trap 149); the order inside each row is what they show, and it held in every
row.

| target | pool 4: 4 / 8 / 16 asked | pool 8: 4 / 8 / 16 asked |
|---|---|---|
| page at 150 dpi, 4 967 × 3 509 | **1 151** / 1 178 / 1 238 | 1 391 / **1 197** / 1 199 |
| fit window, 1 416 × 1 000 | **690** / 733 / 761 (10 granted) | 678 / **673** / 743 |
| 4× window, 1 600 × 1 000 | **161** / 171 / 185 (10 granted) | 179 / **167** / 205 |
| 4× whole page, 5 663 × 4 000 | **1 303** / 1 362 / 1 409 | 1 386 / **1 237** / 1 304 |
| 4× band, 5 663 × 1 000 | **444** / 470 / 510 (9 granted) | 452 / **416** / 436 |

The pool's own width is the quickest count in all ten rows. On four threads sixteen asked costs
8–15 % more; on eight, four asked leaves half the pool idle and costs 1–16 % more. Each count draws
its own digest, which is ADR 0219's shifted origin at each division and is why the gates state theirs.

## 4. The toolkit windows' zoom, priced and not built

A toolkit window asks `viewer_core` for the whole page at the magnification (`Viewer::schedule`,
`TargetSpec::for_page`), so ADR 1841's cull keeps every command and the frame costs the whole page.
At four times the fit that is 1 303 ms; the band of rows the window shows, across the page's width,
is 444 ms (1 127 050 of 4 408 405 commands reach it); the window itself is 161 ms. Ten notches put
the page at several times that. A band is a strip of the page, so its rows are ADR 0139's exact cuts
(a window is not cut into columns, ADR 1841 section 2).

It is not built here because the change is to a protocol several hosts read as "the whole page":

- the first request precedes the host's tier, and `quorra` reads `RenderRequest::target`'s transform
  for its own frame;
- `quorra-confined`, `viewer-ffi` and the confined worker answer `Rendered::Raster` as the page;
- `viewer_host::stand_in::placed` scales a frame whose size is not the page's extent (ADR 1843);
- `viewer_host::over_frames` puts a page-sized look over the frame (ADR 1825), so a whole-page
  draw behind a sharp band would show the band replaced by a part-built page.

The build that follows needs four pieces in one round: a host saying it places a raster at a stated
row offset; `FrameView` carrying the page extent its pixels were drawn for, so the stand-in scales
them correctly; a frame that grows band by band at one magnification, the visible band first and the
rest after, each band's edges on rows the planner permits; and a look shown only over the band it
draws. The files are `viewer-core`'s `viewer.rs` and `open.rs`, `viewer-host`'s `stand_in.rs` and
`drawing.rs`, and the two toolkit hosts.

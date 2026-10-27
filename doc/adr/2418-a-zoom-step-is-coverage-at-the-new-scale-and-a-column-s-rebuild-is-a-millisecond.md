# 2418 — A zoom step is coverage at the new scale, and a column's rebuild is a millisecond

Status: accepted — the owner's question answered with a measured split; ADR 2408's lever 1 priced
again and not built. Session 1792, 2026-10-26. Context: ADRs 0702 (a scene in page space is one
page), 0699 (the settled 2× pass), 1877, 1992, 2056, 2134, 2315, 2408; raster's ADRs 0087, 0091,
0095; `doc/todo/75`; traps 262, 271, 274, 275, 276; habit 164.

## 1. The question

The owner, 2026-10-26: "Why is it still not possible to put a page / document into GPU memory and
just change the view-point? I thought we wanted to do this? I have the impression, that we still
need to calculate a lot even when drawing using the GPU. What si blocking us here?"

**The answer: for one page it is done, and what a step still computes is the part a GPU computes
for any vector renderer at a new scale.** The page's outlines live in the device's segment arena,
its lines in page space under a view uniform, its images per reduction level (ADRs 1992, 2022,
2056). A step sends the view and 80 to 420 KB of per-command records and instances, and the device
then works out which pixels each shape covers *at the new scale* — flatten, count, emit, deposit.
That last part is not a cache miss: an outline's coverage of a pixel grid is a function of the
grid, and the only picture that skips it is a texture resampled — the blurry stand-in. What still
recomputes on the host is three things, each named below with what blocks it.

## 2. The split, measured

`render-raster`'s `zoom_frame` on the Radeon 890M (`AMD Radeon 890M Graphics (RADV STRIX1)`),
`Coverage::Compute`, a window target of 800 × 1600 device pixels, twelve steps of 1.25 (four in,
six out, two in), minima of three rounds, two passes in one `--clock --nice 0` hold; every arm on
`pp_dpm_sclk`'s middle line, 642 to 1112 MHz, no other process of this account on the render node.
The compute lane's emit and deposit were timed apart by a measurement patch of three
timestamp queries that the tree does not carry (raster's lane stamps the two as one span). A
column is the new `ZOOM_FRAME_COLUMN=2` (pp. 101–102 of ISO 32000-2, the 12.8-pixel gap). Per
step, in ms unless stated; *read back* is the headless target's and a window does not pay it.

| term | `issue19802` p. 1 | EC3 p. 101 | pp. 101–102 column | `22060_A1_01_Plans` |
|---|---|---|---|---|
| step | 1.8–4.0 | 2.4–5.0 | 2.5–8.8 | 9.7–21.4 |
| step less read back | 1.0–2.7 | 1.7–3.3 | 1.8–7.9 | 8.9–20.0 |
| scene rebuild (host) | 0 | 0 | 0.2–0.7 | 0 |
| encode (host) | 0.1–0.2, record replay | 0.2–0.6, record replay | 0.1–1.3, a walk | 2.7–9.0, a walk |
| transfer (host) / bytes crossed | 0.1–0.2 / 79–142 KB | 0.1–0.8 / 143–423 KB | 0.1–0.7 / 122–813 KB | 0.3–6.8 / 0.8–10.6 MB |
| tile records (host) | 0.01–0.03 | 0.02–0.07 | 0.01–0.07 | 0 |
| count pass: flatten + count (device) | 0.11–0.13 | 0.12–0.57 | 0.11–1.00 | 0.02–1.43 |
| emit pass: flatten + emit (device) | 0.13–0.21 | 0.15–0.69 | 0.13–0.52 | 0.01–0.81 |
| deposit pass (device) | 0.22–1.03 | 0.42–0.84 | 0.38–1.23 | 0.05–2.12 |
| content pass (device) | 0.06–0.17 | 0.10–0.19 | 0.25–0.35 | 0.27–0.68 |

Flatten has no span of its own: it is fused into both the count and the emit kernels
(`flatten_tiles`), so a step flattens every visible outline twice.

## 3. What is inherent, what is removable, and what blocks each

- **Inherent: count, emit, deposit and the content pass** — coverage at the new scale, 0.5 to 2.5 ms
  on a text page, up to 5 ms on the plan. A vector page drawn sharply at a scale it was never drawn
  at needs it; only the stand-in's texture escapes it, and that is the blur the owner sees.
- **Removable, inside the kernels: the second flatten.** The count exists to size and place the
  emit (raster's ADR 0095); a design that keeps last frame's capacity and lays edges out by a bump
  allocator could flatten once. Its ceiling is the count pass, 0.1 to 1.4 ms a step; raster's ADR
  0091 already names the kernels' occupancy as the next measurement.
- **Removable, on the host: the heavy page's walk.** The plan never record-replays: every step is
  a walk (encode 2.7 to 9.0 ms) that sends coverage rasterised on the processor on the scratch
  sheet (1.3 to 1.8 MB a step) — where that coverage is strokes, `doc/todo/75` item 6, a stroke
  expanded on the device as the fills already are — and its images cross a finer level the first
  time a zoom samples it (the `paint` stream, up to 6 MB, then resident). Which of the page's 71
  commands keeps it off record replay is not read here; `replay_line_census` names refusals by
  name and is the next instrument. What blocks item 6 is that nothing has been built for it.
- **Removable, on the host: a column's rebuild — and it is small.** A column keeps the baked path
  (ADR 0702: page space admits one page), so each step rebuilds its scene and walks it. Headless
  that costs 0.2 to 0.7 ms of rebuild plus a walk that is at most 0.8 ms dearer than the two
  pages' record replays (and cheaper at a deep zoom, where the walk culls): **at most 1.2 ms a
  step**, and every headless column step is already inside 8.33 ms.
- **What a window adds.** quorra in a nested KWin of this round's own (2880 × 1800 at scale 1.6,
  Control+wheel through EIS, the same twelve notches, HEAD's release build) read notch to sharp
  frame: `issue19802` 2–22 ms, 20 of 24 inside 8.33 (the two misses a run are the lane's first use
  and the step after it, trap 271); the column 6–29 ms, 7 of 24 inside, its device span 4.0 to 13.9
  ms against 1.8 to 7.9 headless; the plan 14–49 ms, 0 of 24. Run one of the column read the middle
  line at 1228 to 1464 MHz and run two at 600 to 1018 MHz (6 against 1 of 12 inside), so the two
  are not one level (trap 275). **The column's window steps upload 33 to 82 resources each where
  the headless steps upload 2**, and the likely cause is the settled 2× pass (ADR 0699) after each
  notch: it rebuilds a baked scene at another view, the caches evict what that scene did not look
  up (`cache::evict_settled`), and the next notch uploads it again — an inference from the counts,
  which the arm named in section 4 tests. A one-page scene is view-free and survives the 2× pass, which is why a page is
  `re-placed` in the window on every step and a column is `encoded`.

## 4. Lever 1, priced again, not built

ADR 2408's lever 1 — a column in page space, one scene per page under its own placement — has two
ceilings now. In raster's own step it buys at most 1.2 ms (section 3), which moves no headless step
across the refresh. In the window it buys what a column loses to the alternation with the 2× pass:
its ceiling remains ADR 2408's, the column at its first page's cost. **What blocks building it is
raster's one affine per frame**: the per-command compositions (`encode::device_space::compose`,
called per fill, stroke, rectangle, line, clip and replayed record) would take a placement each
cheaply, but the line lane's view uniform is one value per pass (`Encoded::line_view`), the
retained encode is keyed by one affine (`retained::EncodeKey`) and holds one encode, and the walk
gates line adoption on that one affine. Changing all four, held byte for byte, is larger than what
this round had left after the measurement; the handover the session left names the sites.
**The cheaper lever the window measurement found** is the alternation itself: a column's settled
2× pass held to the scene the step will want, or skipped for a baked scene. Its ceiling is the
window's column device span falling towards the headless one; it is unmeasured and is the next
arm to take, with the 2× pass off, before either lever is built.

## Costs

The headless figures are minima of three rounds and the window's single samples; the window arms
started at one-minute loads of 2.2 to 30.2 (the first `issue19802` arm with another process of this
account on the render node), so the window's column figure carries the clock and the load beside it
and is read as a range, not a band.

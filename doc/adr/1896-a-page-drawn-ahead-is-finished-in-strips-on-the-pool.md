# 1896 — A page drawn ahead is drawn and finished in strips on the pool, and the planner's ten strips are what the draw does not divide past

Session 1530, the drawer ADR 1888 §4.3 chose, on the project owner's target of 2026-10-10 and
2026-10-14 ("continue improving performance (and fixing bugs)"). Status: **accepted** and **built**
in `render-cpu` and `viewer-host`. Context: ADRs 0139, 0219, 0731, 1758, 1819, 1837, 1841, 1855,
1859, 1880, 1882, 1888. Code: `render_cpu::Underway::{in_strips, take_all, cuts}`,
`CpuRasterizer::with_cuts`, `viewer_host::Ahead`'s drawer. Tests:
`render-cpu/tests/drawn_ahead_in_strips.rs`; `viewer-host/tests/drawn_ahead_in_strips.rs` (the
corpus). Instruments: `viewer-host/examples/pooled_draw.rs` (new), `examples/drawn_while_read`.
Witness: `demo_heavy_engineering_A1.pdf` of github.com/cyfung1031/BergPDF (`sample/`), downloaded
again on 2026-10-14 after `/tmp` was emptied by a reboot; its display list's digest is still
`07eb7445…68e3`. No clause is read anew and no ledger row moves.

## 1. The premise and the hypothesis, checked

**The premise held, and it hid a regression.** `loading.rs` drew a page read past `GLIMPSE_AFTER`
on one thread (`Underway::new`), and since ADR 1880 the read is divided: it ends 0.57–0.64 s in,
the drawer has seen the first piece's parts only, and `finish` drew the other forty-seven pieces on
that one thread. Headless on 24 threads the frame drawn ahead landed at 1.73–1.75 s, the frame
drawn after the read on the pool at 0.92–1.04 s: **drawing ahead cost the window 0.8 s.** `quorra`,
which finishes every page one so drawn, showed it on Xvfb and on the owner's display (§3).

**The hypothesis held in what it named and not in what it blamed.** The draw whole does not divide
past ten strips: the planner asks `min(pool, MAX_STRIPS = 16, rows / MIN_STRIP_ROWS = 15)` at a
1 000-row target, the cost-balanced cut under the 64-row floor makes ten, and the halving search
stops there — the same ten cuts on 12 threads and on 24, so the extra twelve threads draw nothing.
The strips are not equal either: by a probe build's timers, under load, the quickest of the ten
ended 91 ms in and the slowest 296 ms on 12 threads (155 and 360 ms on 24). No serial part after
the strips is large: the per-pixel passes and the conversion are 1–2 ms. Before the strips there are
two serial walks of the 4.4 M commands — the group-blit check (13–18 ms, `pdf-render`'s) and the
image warm (14–21 ms, this crate's) — beside the bounds and the plan on the pool. On a quiet machine
the draw whole is 296–324 ms on 12 and 299–319 ms on 24, not round 1526's 490–525 ms: that hold ran
beside five siblings.

## 2. Decision

1. **`Underway::in_strips`**: the target cut evenly into as many strips as the pool is wide, within
   `MAX_STRIPS` and `MIN_STRIP_ROWS` (fifteen on the witness), each strip with its own pixels and
   masks, the mask budget divided as the whole draw divides it. Each batch of parts is one list of
   the tables so far; its bounds are measured once, and every strip draws it on the pool through
   `encode_page`, the loop the whole draw uses. `finish` draws the rest of the list from where the
   parts stopped, the same way. The page is defined as **the page drawn whole at those cuts**, and
   `CpuRasterizer::with_cuts` draws it so. `Underway::new` is one surface as before, used by the
   confined worker and by `pdf-model`'s own test.
2. **A cut is a row no mark forbids**, asked of every batch through `strip_inputs` as the planner
   asks it of the whole list (ADRs 0139, 1819, 1837). A batch that forbids a cut ends the drawing
   ahead with a sentence, and the window draws the page whole. Even cuts are a guess at balance,
   because the cost of a part that has not been read is unknown. On the witness, fifteen even
   strips draw the page in 271–311 ms against the planner's ten strips in 296–344 ms.
3. **The waiting parts are drawn in one pass** (`take_all`). A part holds the marks of a few rows,
   so a pass over one part keeps one strip busy while the others wait.
4. **The image warm stays serial.** A version that found the images on the pool and reduced them
   afterwards was built and measured. Run against HEAD on the second lane, three pairs, it drew
   the page whole slower in every pair (358–378 against 315–343 ms, then 474–541 against 369–391,
   then 407–444 against 391–441, at load 7.2–7.9), so it was reverted.
5. **The join's hand-over is not taken.** Offering each joined piece as a part (`parts.offer` after
   `Interpreter::join`, round 1529's hunk) lets the frame land 0.10 s after the read. But the read
   slows from 0.57–0.60 s to 1.005–1.009 s, and the frame lands at 1.10–1.11 s against
   0.87–0.92 s without it. The copy runs on the joining thread, which is the critical path, and the
   drawer takes the pool from the pieces still being read.

## 3. Measured

One `--clock` hold (`scratchpad/r1530/hold.sh`, load 2.0–6.0). Three arms, alternated three times:
HEAD `3fc3e7f1` (`head`, quorra `150d71e9…`); this build (`build`, `4a30b491…`); this build with
the join's hand-over (`joined`, `9b584a4a…`). All arms are exports with their own target
directories. Seconds from process start, except the draw.

| | HEAD | this build | with the join's hand-over |
|---|---|---|---|
| draw whole on 12 threads, the planner's 10 strips (ms) | 296–324 | 321–338 † | |
| draw whole on 24 threads, the planner's 10 strips (ms) | 299–319 | 310–344 † | |
| the same, 15 even strips, 12 / 24 threads (ms) | | 284–311 / 271–305 | |
| headless, drawn ahead: read ends · first frame | 0.57–0.60 · **1.73–1.75** | 0.57–0.60 · **0.87–0.92** | 1.005–1.009 · 1.10–1.11 |
| headless, drawn after the read (pool) | 0.92–1.01 | 0.92–1.04 | 0.92–0.99 |
| `quorra` on Xvfb: read · look · first frame | 0.66–0.77 · 0.60–0.64 · **1.77–1.91** | 0.70–0.77 · 0.63–0.71 · **1.09–1.15** | 1.19–1.23 · 0.72–0.76 · 1.32–1.35 |
| `quorra` on the owner's display: read · look · frame | 0.72–0.77 · 0.59–0.63 · **1.94–2.02** | 0.75–0.77 · 0.71–0.71 · **1.11–1.17** | 1.18–1.32 · 0.74–0.81 · 1.31–1.44 |

† Each round of this arm drew the even division first, so its planner draw ran second in the
process, which HEAD's arm never did. That build also carried the parallel warm of §2.4, which was
since reverted. A second `--clock` hold (`hold2.sh`, load 7.1–9.5) re-took the window figures for
the tree as handed over (`final`, quorra `acba4957…`) against HEAD, alternated three times. Headless
drawn ahead: 1.74–1.75 → **0.90–0.95 s**. `quorra` on Xvfb: 1.88–1.99 → **1.07–1.15 s**. On the
owner's display: 1.96–2.02 → **1.08–1.17 s**. The reads end at the same 0.60–0.79 s in both arms.

The look is 0.03–0.12 s later than HEAD's, because the first batch is drawn on the pool before
it is shown. **Exactness**: on the witness, every drawn-ahead frame of every arm is the page
drawn whole at its fifteen cuts. The fixtures hold this at two, four and seven strips, and a
planted defect fails them: a forbidden cut taken, or the finish drawn from one command early.
The corpus does the same at four stated strips (`viewer-host`'s gate, parts of eight commands).
Of 974 first pages, 610 are finished in strips and 281 as one surface, each byte for byte the
page drawn whole at its cuts. 66 are drawn whole instead, each with its reason: 19 for a mark
across a cut, 27 for a soft mask, 19 that finished no list for the hand-over, and 1 with a plane
of its own. `pdf-model`'s gate still finds 910 pages drawn as one surface identical to one strip.
Photographed on Xvfb, the first frame is the whole drawing, with no seam at any cut. Fewer than
one pixel in a million differs from HEAD's one-strip frame, and none does at a 2 % fuzz.

## 4. What it leaves

- **The planner's strip count.** Asked for the pool's width at a 1 000-row target, it would draw
  the witness about as fast as even cuts do. In exploration on the second lane, even cuts at 24
  strips took 303–304 ms against 384–397 ms for the planner's ten. But the cut moves the
  processor arms' digests by ADR 1758's bound, and that moving is owed six arms. It is the next
  build for the draw whole.
- **The read that the drawer shares.** A drawer that takes the pieces as they are read pays off
  only where the pool has room beside the read. The pipelined inflate (round 1529) is when to
  price it again, with the copy off the joining thread.

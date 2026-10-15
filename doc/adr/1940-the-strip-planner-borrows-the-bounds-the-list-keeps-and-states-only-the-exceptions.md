# 1940 — The strip planner borrows the bounds the list keeps, and states only the exceptions

Session 1552, under the project owner's "switch to code improvements … until I say otherwise" of
2026-10-14. Status: **accepted** and **built** in `pdf-render`. Context: ADRs 1841, 1882, 1915 §3,
1924 §3, which named this cut. Code: `pdf_render::strips::{StripInputs, Piece, Extents,
MOST_EXCEPTIONS}`. Test: `strips::tests::the_inputs_read_on_the_pool_are_the_serial_inputs`, now
asked of a page of many exceptions and of one of few. Cites no clause and moves no ledger row: what
the planner states is unchanged, bit for bit.

## 1. What was found

**The premise held.** At the heap's peak of the owner's witness drawn headless (`quorra-transform
render --dpi 42.75 --format ppm`, massif with `--fair-sched=yes`, twenty-four threads, built
`--profile bench`), two vectors of `Option<Rect>` of 88 168 100 bytes each live at once:
`MarkBounds::measure`'s, which the list keeps, and the strip planner's copy of the extents
(`Piece::read`). **A third the brief did not name** stands beside them: the bands' keys, which say
which extents a band of rows sums, 62 571 264 bytes at eight bytes a key.

**Why one vector does.** A top-level command's extent is its own device bound met with its clip
chain's extent (`gather_one`), and `meet` returns its left operand's own bits wherever the right
one holds it. So the extent *is* the bound for every command with no clip, and for every clipped
command whose clip contains it. The exceptions are a clip that narrows the bound, a degenerate
bound under a clip (`meet` answers it `None`), and a group, whose extents are its elements'. On the
witness, 87 of 4 408 405 commands are clipped and none is an exception.

## 2. Decision

1. **A run whose bounds were measured under the target's own transform** (`MarkBounds::exactly_at`,
   ADR 1882 §2.1) borrows them, and states beside them only its exceptions, each command's extents
   in order (`Extents::Borrowed`). A run of more than one exception in four states every extent,
   as before (`Extents::Stated`, `MOST_EXCEPTIONS`): an exception costs 16 bytes of indices beside
   its extents' 20, so past about one in two the borrowed form is the larger, and at one in four it
   holds under half of the copy and keeps the search for a key short. The two forms state the same
   extents in the same order, which the test asserts against the serial `command_extents`, so no
   number the cut reads can depend on which a run took.
2. **A band's keys are 32 bits**: in the borrowed form a key is the command's index in its run, a
   group's elements sharing it, and otherwise the extent's position. A piece any of whose keys
   would pass 32 bits keeps none, and every band then reads every extent of it — correct and slow,
   for a run of more than four thousand million.
3. **Where the bounds were measured at another translation** — a scroll at a magnification
   already measured — each top-level command's own bound is measured once, for its extent and its
   forbidden rows both, where each of the two measured it for itself. A pure function asked once
   instead of twice: the same number.

## 3. Measured

Two exports of HEAD `7bf264b3`, one with this round's `pdf-render`; one `--clock` hold.

| massif's peak, `--fair-sched=yes`, 24 threads | HEAD | this |
|---|---|---|
| useful heap, MiB | 1 381.6 | 1 266.5 |
| with the allocator's own, MiB | 1 515.9 | 1 400.8 |
| the list's kept bounds, MiB | 84.1 | 84.1 |
| the planner's extents, MiB | 84.1 | — |
| the bands' keys, MiB | 59.7 | 28.6 |

The frame is the same file, SHA-256 `e6fde03d…2b56`, in both. **The hypothesis — 84 MiB saved and
no time — held, and 31 MiB more**: the whole page's frame on the window example, the clock hold of
ADR 1941 §3, reads inside HEAD's spread at every size, on four threads and on twenty-four. The peak
that is left is the list's own: its commands' 504.5 MiB, their paths' 604.4 and the bounds the draw
reads.

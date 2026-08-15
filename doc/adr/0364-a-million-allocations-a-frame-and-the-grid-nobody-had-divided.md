# ADR 0364 — A million allocations a frame, and the grid nobody had divided

Status: accepted, 2026-08-15. Session 529. Makes §8.7.4.5.2's device-resolution grid cheap without
changing one bit of it. Amends §8.7.4.5.2's ledger row; changes no clause reading and no pixel.

## The report

The project owner's witness is `tmp/pi.pdf` — untracked, so no test names it: a `ShadingType 1`
over a 400×400 page whose `/FunctionType 4` program is 2580 bytes of a seven-segment display
driven by `2 index 2 index dup … ge exch … le and`, with the BBP series for π expanded three
terms. `mutool draw -r 96` draws it in 15–16 ms. This viewer, headless under `Xvfb` on `llvmpipe`,
three runs:

```
frame p1 1cmd presented 1202.5 | host 0.0 scene 1175.2 device 27.2 | 3 up, 0 culled
frame p1 1cmd presented 1079.6 | host 0.0 scene 1059.2 device 20.4 | 3 up, 0 culled
frame p1 1cmd presented 1143.9 | host 0.0 scene 1111.7 device 32.2 | 3 up, 0 culled
```

**One display-list command, and a second of it.** Interpretation is 0.4 ms of that and the device
is 20–32 ms: the whole cost is between them, in `Shading::sampled_at` — the grid the *backend*
asks for once it knows how many device pixels the domain covers.

## What was wrong, and what was not

**ADR 0339's decision is not what was wrong, and this round does not touch it.** Session 504
replaced a fixed 128×128 grid with the device's own so that a discontinuity the clause explicitly
permits — "[t]he function need not be smooth or continuous" — lands on the right pixel, and
`a_function_based_shadings_discontinuity_lands_at_the_device_pixel` is what pins it. The grid's
resolution and the half-pixel rule that places each cell centre are untouched here. The round's job
was to make the *same answer* cheap.

Two things made it expensive, and neither is arithmetic:

1. **Every cell allocated, three or four times.** `Function::eval` built a `Vec` for the clipped
   inputs; `evaluate_postscript` built the operand stack with `inputs.to_vec()` and returned it;
   `Sampled::eval` built three more; `shading::colour_from` built one to concatenate the group's
   outputs into; and `copy` — which this program executes once per cell — did a `to_vec` of its
   own. A full-page type 1 shading at this window is ~640 000 cells, so a frame was on the order of
   a million heap round trips out of code that needs none.

2. **The grid was a serial nested loop on the thread the frame was built on**, on a machine with
   24 of them, while every cell of it is a pure function of its own two coordinates.

## The decision

### 1. One buffer for the whole grid, and the operand stack *is* the answer

`Function::eval_into(&self, inputs, outputs: &mut Vec<f32>)` is the body; `Function::eval` is a
two-line wrapper that allocates one `Vec` and calls it, so every existing caller is unchanged.
Inside:

- The clipped inputs live in a `[f32; MAX_VALUES]` on the stack. That is sound rather than
  hopeful: `pairs` **refuses** a `/Domain` longer than `MAX_VALUES`, so a parsed function cannot
  have more, and `MAX_STITCH_DEPTH` bounds the recursion at eight frames of it.
- `Sampled::eval_into` does the same for `/Size`'s per-dimension base and fraction, on `numbers`'
  matching refusal, and accumulates into the caller's buffer instead of a fresh `vec![0.0; n]`.
- A stitching function's sub-function writes into the *same* buffer, so a chain costs one
  allocation rather than one per link.
- **A type 4 program runs on the caller's buffer directly**, because §7.10.5 leaves its outputs on
  the operand stack: the inputs are pushed onto the caller's `Vec` and the program is executed
  there, so the stack and the result are one object and neither is allocated.
- `Operator::Copy` uses `extend_from_within`, which is the same copy without the intermediate
  `Vec`. The range cannot be out of bounds because its start came from a `saturating_sub` on the
  same length.
- `shading::Components` is the two buffers a grid reuses across cells — the group's concatenated
  components, and one member's outputs where the group has more than one member. A group of one,
  which is what almost every shading has, writes straight into the first and never touches the
  second.

### 2. The grid is divided across rows, above a threshold and off the pool

`FunctionColours::row` is the unit, and both paths call it, so the two run the same arithmetic in
the same order. `rows_in_parallel` decides, on two conditions:

- **Below `PARALLEL_CELLS` (4096) it stays on one thread.** A shading covering a swatch is a few
  hundred cells and a fork-join costs more processor time than it divides.
- **A caller already on a rayon worker keeps the grid whole.** `render-cpu` splits a page into
  strips across the pool and rebuilds this shading's pattern inside each; forking again would ask a
  pool with no idle thread for a job per strip. It is `colour::build_ink_table`'s branch, for a
  different reason — that one avoids a deadlock, this one avoids dividing what is already divided.

**This is rasterisation-side work, and the distinction is the project's rule rather than a
convenience.** Interpreting a content stream is sequential by construction: each operator reads the
graphics state the one before it left, so nothing in `content.rs` is a candidate for a pool. A grid
of cells is the opposite shape and is the shape `image::band_pixels` already divides — the question
`doc/habits.md` says to ask before dividing anything is *what does a parallel unit's answer depend
on*, and here it is the cell's own two coordinates and nothing else. So a row boundary decides
which thread evaluates a cell and never what the cell evaluates to. That is an argument, and
`a_function_based_shadings_grid_is_the_same_however_it_is_divided` is the check: the same page
drawn with the grid divided and with it whole, compared byte for byte, one strip in both so that
the division is the only difference. Run with `self.row(row.wrapping_add(1), …)` in the parallel
arm it fails, which is what says it reaches the path.

## The measurements

### Step 1 alone — instructions, `RAYON_NUM_THREADS=1`

`valgrind --tool=callgrind --callgrind-out-file=/dev/null tmp-target/release/examples/callgrind_rasterise <file> <page> 3`,
A/B in one sitting. The repeat count is an argument this round added to that example: twenty
renders of a page whose one command is a second of work is an hour under callgrind where three is a
minute, and the count is part of the invocation because two arms are only comparable when they
share it.

| document | before | after | |
|---|---|---|---|
| `tmp/pi.pdf` p1 | 8 750 282 543 | 8 220 153 240 | **−6.06%** |
| `doc/corpora-own/type4_pi.pdf` p1 | 9 733 404 286 | 8 576 488 706 | **−11.89%** |
| `function_based_shading.pdf` p1 | 1 937 619 973 | 1 355 855 860 | **−30.02%** |
| ISO 32000-2 p101 — **the control, no shading** | 991 723 441 | 991 723 021 | −420 instructions |

The control is the point of the table: a page with no shading on it moves by 420 instructions in a
billion, which is the two `Vec::new`s `eval` now makes instead of one. The spread across the other
three is the ratio of allocation to arithmetic — `pi.pdf`'s program is ~700 instructions a cell and
buries its allocations, `function_based_shading.pdf`'s is short enough that they were a third of it.

### Step 2 alone — wall clock, and the load it was taken under

Instructions are the wrong instrument for a division: a parallel change makes the count go *up*
while the page appears sooner. So this is a clock, and **the machine was not quiet** — this tree is
worked by several rounds at once and the load average moved between 4.5 and 45 during the session.
Every figure below was taken with `uptime` reading **4.5 on 24 cores**, and the arms were run
alternately in one sitting.

Five renders of `tmp/pi.pdf` page 1 through `callgrind_rasterise`, natively, the same binary both
ways (`RAYON_NUM_THREADS=1` takes the serial branch, because `rows_in_parallel` asks
`current_num_threads`):

| | run 1 | run 2 | run 3 |
|---|---|---|---|
| serial | 1.365 s | 1.371 s | — |
| divided | 0.195 s | 0.187 s | 0.172 s |

and the ink sum of the five rasters is the same digit in both, which is the same byte-identity
check one level cheaper than a PNG.

### The threshold, and what it costs a small grid

The same program on pages of 20, 40, 64, 128 and 200 points, so the grid is 400 to 40 000 cells,
600 renders each, at load 4.5:

| grid | serial | divided |
|---|---|---|
| 400 cells | 0.419 s | 0.194 s |
| 1 600 | 1.671 s | 0.370 s |
| 4 096 | 4.120 s | 0.772 s |
| 16 384 | 19.483 s | 5.832 s |
| 40 000 | 45.778 s | 18.561 s |

**In wall clock the division wins at every size measured, and the threshold is kept anyway**, on
the second column no table of clocks shows. At 400 cells, 3000 renders: serial 2.137 s wall and
2.061 s of processor; divided 1.137 s wall and 8.696 user + 2.927 sys. So the fork buys 1.0 s of
clock for **5.6× the processor time**, and it buys it only where there is an idle core to spend —
re-run at load 45 the same 4096-cell arm read 9.145 s serial against **11.944 s divided**, the
division losing outright. A viewer is not the only thing on a person's machine, and a page may hold
many shadings where this measurement holds one. 4096 cells is a 64×64 tile: below it the whole grid
is a few milliseconds of work and the round declines to spend a pool on it; above it the division is
2–5× on this machine.

### What the owner sees

`DISPLAY=:91 target/pdf-viewer --trace=launch,frames /home/cl/projects/pdf-viewer/tmp/pi.pdf`,
three runs each side, `Xvfb` on `llvmpipe`, off the installed release binary. The load moved
between 4.5 and 30 across the session and the *spread* below is mostly that; the ratio is not.

| | before | after |
|---|---|---|
| `scene` | 1175.2 / 1059.2 / 1111.7 ms | **119.1 / 284.4 / 105.3 ms** |
| `presented` | 1202.5 / 1079.6 / 1143.9 ms | **154.3 / 324.5 / 123.7 ms** |

`document joined` (90–110 ms) and `interpreted` (0.2–0.4 ms) do not move, which is the check that
the change is where it was aimed. **`mutool draw -r 96` is still 15–16 ms** and this is not a claim
to have caught it: what is left is the type 4 program itself, priced below.

## What guards the identity

This round changes no pixel, so the evidence is that nothing moved:

- **Seven rasters, MD5-identical before and after**: `tmp/pi.pdf` at 1× and 2.7×,
  `type4_pi.pdf` at 1× and 3×, `function_based_shading.pdf` at 1×,
  `function_based_shading_cmyk.pdf` at 4×, and ISO 32000-2 page 101 at 1.5× as the control.
- **Every gate identical**, run on both sides of the change in this worktree: the corpus gate's
  whole line, the oracle's seven verdict counts, the quorra gate on both coverage lanes, and the
  text gates' two percentages.
- `a_function_based_shadings_grid_is_the_same_however_it_is_divided`, above, which is the one that
  will still be there when somebody changes the division.

## What is left, and what it would cost

**The remaining second is not allocation and not scheduling: it is the program.** `pi.pdf`'s type 4
function is ~700 instructions a cell over ~640 000 cells at this window, and after this round that
is most of what is left. Three levers, priced but not taken:

1. **Compile the program rather than interpret it.** `Instruction` is already a compiled form; what
   it is not is a form that avoids a `match` per operator. A register machine, or folding constant
   subexpressions at parse time, is the ordinary answer and would need its own evidence that the
   arithmetic is bit-identical — `f32` reassociation is not free.
2. **Memoise on the input pair.** ADR 0068's precedent is exactly this shape and was 3× there. Here
   the inputs are two floats spanning the domain and every cell is distinct, so a memo would hold a
   million entries and hit nothing. Declined on the arithmetic rather than on taste.
3. **Evaluate adaptively — a quadtree that subdivides only where the value changes.** This is the
   real lever and it is the one that must not be taken casually, because **it is in direct tension
   with what ADR 0339 bought**. The clause says the function "need not be smooth or continuous", so
   a subdivision that stops where four corners agree will average a step it did not sample —
   precisely the blur session 504 removed, reappearing as an adaptive rule instead of as a fixed
   grid. Any such round owes: what the refinement predicate is, why a discontinuity narrower than
   the current cell cannot hide from it, and the discontinuity test above passing unchanged at
   several scales. It is a round's work and it is not this round's.

**And one cost this round found and did not pay**: `render-cpu` rebuilds the shading's pattern
inside *every strip*, so a page drawn in sixteen strips evaluates the whole grid sixteen times. It
is invisible on the path the owner reported — quorra builds the scene once, on one thread — and it
is why `rows_in_parallel` declines on a worker rather than forking sixteen times over. Hoisting the
resolved grid out of the strip loop is a `render-cpu` change with its own cache-lifetime question,
and it is named here so that the next round measuring that backend finds it.

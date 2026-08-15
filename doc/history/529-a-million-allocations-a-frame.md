# 529 — A million allocations a frame, and the grid nobody had divided

**Finding.** The project owner reported that `mutool` draws their `tmp/pi.pdf` in 15–16 ms where
this viewer takes over a second, and the trace said the whole of it was **one display-list
command**: a `ShadingType 1` whose function ADR 0339 evaluates once per device pixel. Neither half
of the cost was arithmetic. Each cell allocated three or four times — the clipped inputs, the
operand stack (`inputs.to_vec()` per evaluation), the outputs, the group's concatenation, and
`copy`'s own `to_vec` — which is about a million heap round trips a frame; and the grid was a
serial nested loop on one thread of twenty-four, while every cell of it is a pure function of its
own two coordinates. **The fix changes not one bit of the answer**: the evaluation writes into a
reused buffer, a type 4 program runs *on* that buffer because §7.10.5 leaves its outputs on the
operand stack, and the grid is divided across rows above 4096 cells and off rayon's own pool.
ADR 0339's resolution and half-pixel rule are untouched, and an adaptive grid — the remaining lever
— is priced in the ADR and declined, because a subdivision that stops where four corners agree
would re-introduce exactly the blur session 504 removed.

**Date.** 2026-08-15.
**ADR.** [0364](../adr/0364-a-million-allocations-a-frame-and-the-grid-nobody-had-divided.md).
**Touched.** `crates/pdf-model/src/function.rs` (`eval_into`, `eval_stitching_into`,
`Sampled::eval_into`, `evaluate_postscript` on a caller's stack, `Operator::Copy` through
`extend_from_within`), `crates/pdf-model/src/shading.rs` (`Components`, `colour_into`,
`FunctionColours::row`, `rows_in_parallel`, `PARALLEL_CELLS`),
`crates/pdf-model/tests/shadings.rs` (one test), `crates/pdf-model/examples/callgrind_rasterise.rs`
(a repeat count, so a page whose one command is a second of work is measurable under callgrind),
`doc/conformance/ledger.toml` (§8.7.4.5.2), `doc/performance.md`, `doc/todo/45` (a fifth item),
`doc/adr/0364-*` (new), this file.

## The two numbers, and which instrument each came from

**Step 1, instructions, `RAYON_NUM_THREADS=1`** — the deterministic half, three renders through
`examples/callgrind_rasterise`:

| | before | after | |
|---|---|---|---|
| `tmp/pi.pdf` p1 | 8 750 282 543 | 8 220 153 240 | −6.06% |
| `doc/corpora-own/type4_pi.pdf` p1 | 9 733 404 286 | 8 576 488 706 | −11.89% |
| `function_based_shading.pdf` p1 | 1 937 619 973 | 1 355 855 860 | −30.02% |
| ISO 32000-2 p101, no shading | 991 723 441 | 991 723 021 | −420 |

The control is the row that matters most: a page with no shading on it moves by 420 instructions in
a billion. The spread across the other three is the ratio of allocation to arithmetic, and it is
`pi.pdf`'s own program — some seven hundred instructions a cell — that makes its share the smallest.

**Step 2, wall clock, at a stated load.** Five renders of `pi.pdf`: **1.365 / 1.371 s serial**
against **0.195 / 0.187 / 0.172 s divided**, the same ink digit both ways. And the line the owner
actually reads, `--trace=launch,frames` under `Xvfb`:

```
before   frame p1 1cmd presented 1143.9 | host 0.0 scene 1111.7 device 32.2 | 3 up, 0 culled
after    frame p1 1cmd presented  154.3 | host 0.0 scene  119.1 device 35.2 | 3 up, 0 culled
```

`document joined` and `interpreted` do not move, which is what says the change landed where it was
aimed rather than somewhere else on the launch path.

## What the round would not do

**Wall clock was worth nothing for most of this session** and the round nearly recorded a number
that said so. The 4096-cell arm measured 4.120 s serial against 0.772 s divided at load 4.5, and
9.145 s against 11.944 s an hour later at load 45 — the division *losing* — on identical binaries.
Every clock in the ADR carries the load it was taken under, and the threshold was chosen on
processor time (5.6× at 400 cells, for 1 s of clock) rather than on the clock alone, precisely
because the clock is the reading that moved.

**And the round did not take the lever that would have been faster still.** An adaptive or lazy
grid is where the remaining second lives, and it is in direct tension with what ADR 0339 bought:
§8.7.4.5.2 says the function "need not be smooth or continuous", so a quadtree that stops
subdividing where four corners agree averages a step it never sampled. Any round that takes it owes
the refinement predicate, an argument about discontinuities narrower than a cell, and
`a_function_based_shadings_discontinuity_lands_at_the_device_pixel` passing at several scales.

## What proves nothing moved

Seven rasters MD5-identical across the change — `tmp/pi.pdf` at 1× and 2.7×, `type4_pi.pdf` at 1×
and 3×, `function_based_shading.pdf` at 1×, `function_based_shading_cmyk.pdf` at 4×, and ISO 32000-2
page 101 as a control — and every gate run on **both** sides of the change in this worktree rather
than trusted from a document. The new test is the one that will still be there afterwards: the same
page drawn with the grid divided and with it whole, byte for byte, with a pool of one putting the
second arm on a worker where `rows_in_parallel` declines. Run with the parallel arm's row index
shifted by one it fails, which is how the round knows it reaches the path it claims to.

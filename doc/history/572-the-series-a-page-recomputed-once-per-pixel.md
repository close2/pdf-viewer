# 572 — The constant a page recomputed once per device pixel

**Finding:** the type 1 shading whose zoom dropped frames was evaluating a *compile-time constant*
per pixel — π by the BBP series, every operand of every `div` and `exp` in it a literal — and
folding it made the page's program admissible to the graphics device under quorra's agreement rule
unweakened, taking a zoom step from 96.1 ms to 3.5 ms.

Date: 2026-08-18. Argued by [ADR 0406](../adr/0406-the-series-a-page-recomputed-once-per-pixel.md).
Written up for quorra in [`doc/QUORRA_FUNCTION_PAINT_EXACTNESS.md`](../QUORRA_FUNCTION_PAINT_EXACTNESS.md).

Touched: `crates/pdf-model/src/function.rs` (`fold_constants`, `operand_demand`, `Islands`,
`rewrite`, eight tests), `crates/render-quorra/tests/headless_quorra.rs` (one test).

## The round in order

The project owner reported that zooming `tmp/pi.pdf` visibly drops frames, and handed over two
measurements: the page's Type 4 shading is refused by the device, and the cost is entirely CPU
scene-building with the GPU idle.

**Both were confirmed rather than trusted.** `examples/function_paint_census` reproduced the
refusal and its ground; `examples/zoom_frame` reproduced the table (scene 21.1 ms at 400×400,
93.5 at 800×800, `encode` 0.0). Then the attribution `doc/todo/16` asks for, which is a profile and
not an inference: `callgrind` over `examples/callgrind_rasterise` under `RAYON_NUM_THREADS=1` put
**94.09%** of the page in `Function::eval_into`, 15 240 instructions per device pixel.

Two of the four candidates the brief listed were then eliminated **by measurement**, which is worth
recording because both looked promising on paper:

- **Parallelism was already there.** ADR 0364 divides the grid across `rayon`, and an A/B in one
  sitting said so: 781.3 ms serial against 93.5 ms on 24 threads at load 17. The 8.35× is the count
  of free cores on a shared machine, not a defect.
- **The interpreter carries no fat.** ADRs 0364 and 0371 had already removed the per-cell
  allocations and the untyped stack; `libm` was 0.91% and `truncf` 0.46% of the page. Twenty
  instructions per PostScript operator is what a `Vec<Value>` stack machine costs.

So the loop was efficient and the work was real. **The question that was left is why the work was
being done**, and reading the program answered it in one sitting: the middle of it is the BBP
series for π three times over, and every operand of every `div` and `exp` in that section traces
back to a literal the file wrote down. The page computed the constant 3141 once per pixel, on every
frame, at every magnification, and had done since the feature existed.

That is also why the device refused it. quorra declines a program in which an operator WGSL
§15.7.4.1 gives an error budget reaches one that turns a last-bit difference into a whole unit —
here `div` at 234 reaching `truncate` at 354 — and on a page whose content is *digits* that is
exactly right. **What was wrong was asking a device to compute a constant.**

## What was built

`pdf_model::function::fold_constants`: one pass after `compile_postscript`, replacing each *closed
island* — a run beginning with a literal push, containing no jump and no jump target, never reading
below the depth it began at, ending as one value — with that value, computed by
`evaluate_postscript` itself on a stack holding only the run's own values. Bit-identical by
construction rather than by argument, which is what made it safe to ship in one round against an
oracle of 1 794 pages.

Three things the walk has to model, and each was a defect in a draft before it was a paragraph in
the ADR: the absolute stack depth (because `MAX_STACK` is the one context-dependence an isolated
evaluation has, and a join has to be checked rather than assumed); §B.5's counting operators, whose
demand is a value; and §7.10.5.3's underflow, whose first treatment — decline the whole program —
would have left the one document the pass exists for exactly where it was, because
`pi_seven_segment.pdf` reads an empty stack in three places.

## What it moved

Zoom step 96.1 ms → **3.5 ms**, 2.5 MB of uploaded grid → **none**, measured at a *higher* load
average than the baseline. Over 1 251 corpus documents the device-evaluated shadings went 8 → 10
and the agreement refusals 2 → **0**; the one refusal left is a typing ground and is correct.

On the *processor* path the same fold is worth **9.6%** and no more (2.591 G instructions → 2.343
G), because what it removes are the cheap instructions and the seven-segment lookup that survives
is where the interpreter's time goes. Both numbers are in ADR 0406 and they are deliberately not
added together: they are two mechanisms, and a page that stays refused gets only the second.

Three pages in 1 251 files is the honest denominator and the write-up says so. The corpus barely
exercises §8.7.4.5.2, so these numbers rank a *shape* — every agreement refusal anybody here has
seen had literal operands — rather than a population.

## The second question, and where its answer already was

Mid-round the owner asked whether Type 4 functions can run on the GPU at all, on the argument that
IEEE 754 makes `+ − × ÷ sqrt` correctly rounded so only the transcendentals are irreproducible.

**True of IEEE 754, false of the language quorra generates — and quorra had written both halves
down before the question was asked.** `function_ops.wgsl` cites WGSL §15.7.4.1's 2.5 ULP for `x / y`
at the site, and ADR 0053's own amendment, dated three days earlier, withdraws the stronger claim
for `add`, `sub` and `mul` too on §15.7.5's licence to reassociate and fuse. So no narrowing
follows, and none was needed.

The habit that is worth keeping: **before proposing that a neighbour's rule is too strict, read
what the neighbour wrote about it.** The sibling checkout is on this disk, the amendment is one
`grep` away, and it is a stronger argument than the one this round was handed. It is `doc/habits.md`'s
"a negative answer from a tool is a claim about that tool" with *tool* replaced by *premise*.

## What was considered and not taken

Three levers, each because the profile did not point at it and each written into ADR 0406 §7 so
that the next round does not re-derive them: a cache keyed on the resolved grid (exact, but a zoom
step changes the grid, so it buys repeat frames rather than the gesture); a grid coarser than
§10.7.4's one sample per device pixel (a departure, and on a page of digits a visible one); and
clipping the grid to the part of the domain the target actually covers — exact, quadratic in
magnification once a page is zoomed past the window, and the one worth doing next.

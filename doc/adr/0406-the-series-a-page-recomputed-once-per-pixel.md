# ADR 0406 — The series a page recomputed once per pixel

Status: accepted, 2026-08-18. Session 572. Adds a constant-folding pass to
`pdf_model::function`'s compiled §7.10.5 type 4 program. Extends ADR 0376 without amending it:
the grid stays, the agreement rule stays, and what changes is the *program* both paths evaluate.
Consumes ADR 0364's parallel grid and ADR 0371's typed operand stack, and neither had to move.

## The defect, as reported

The project owner: **zooming `tmp/pi.pdf` visibly drops frames.** The page is 400×400 points and
its whole content is one `sh` of a §8.7.4.5.2 type 1 shading whose function draws π's first four
digits on a seven-segment display.

`examples/zoom_frame`, real Radeon 890M, minima of five rounds, load average 17.4:

| frame | pixels | total | scene | device | transfer | bytes uploaded |
|---|---|---:|---:|---:|---:|---:|
| before | 400×400 | 24.6 | **21.1** | 3.4 | 0.5 | 640 032 |
| zoom | 800×800 | 96.1 | **93.5** | 2.7 | 0.7 | 2 560 032 |

`encode` 0.0, `device` flat, and scene time linear in output pixels — the signature of a
per-output-pixel evaluation.

## 1. The attribution, which is a profile rather than an inference

`doc/todo/16` states the rule this project was taught by ADR 0399: *a latency finding is a defect
until it has been attributed.* So, `callgrind` over `examples/callgrind_rasterise` at one repeat
under `RAYON_NUM_THREADS=1`, which is 400×400 = 160 000 cells:

```
2,591,402,574 (100.0%)  PROGRAM TOTALS
2,438,368,875 (94.09%)  <pdf_model::function::Function>::eval_into
   23,680,148 ( 0.91%)  libm.so.6                       ← three `exp`, 49 Ir each
   12,000,082 ( 0.46%)  truncf                          ← three `truncate`
```

15 240 instructions per device pixel, in the interpreter's own loop. Nothing else on the page is
visible.

**Two candidates were then eliminated by measurement rather than by reading.** The grid is already
divided across `rayon` (ADR 0364): A/B in one sitting, `RAYON_NUM_THREADS=1` against the default,
781.3 ms against 93.5 for the zoom frame — 8.35×, which is the number of free cores on a machine at
load 17 rather than a defect. And the interpreter itself carries no fat: ADRs 0364 and 0371 already
removed the per-cell allocations and the untyped stack, and 20 instructions per PostScript
operator over a `Vec<Value>` is what the construction costs.

So the work was real and the loop was efficient. The question left is *why the work was being done
at all*.

## 2. What the page actually computes

Comments stripped, the middle of the program is the BBP series three times over:

```postscript
0 dup 8 mul 0 index 1 add 4 exch div 1 index 4 add 2 exch div sub
  1 index 5 add 1 exch div sub 1 index 6 add 1 exch div sub
  exch pop exch 16 exch exp div
```

for k = 0, 1, 2, then `add add 1000.0 mul truncate cvi`.

**Every operand of every `div` and every `exp` in it is a literal.** The k that drives each block
is the literal the block pushes; the divisors 8k+1, 8k+4, 8k+5, 8k+6 are computed from it; the
16^k likewise. The series is a *constant* — 3141 — and this program recomputed it once per device
pixel, on every frame, at every magnification, for as long as the feature has existed. The rest of
the program (the seven-segment checks, the digit selection) reads the shading's own coordinates and
is genuinely per-pixel.

That is also, exactly, why the graphics device refused the page. quorra's `Agreement::Unbounded`
(their ADR 0053) declines a program in which an operator WGSL §15.7.4.1 gives an error budget
reaches one that turns a last-bit difference into a whole unit:

```
`div` at 234 reaches `truncate` at 354, so no bound on the disagreement
with an independent evaluation can be stated
```

The refusal is right. The `div` it names is `4/(8k+1)`, whose result feeds `truncate` through the
`×1000`, and a device that computes the series to 2.5 ULP of ours could produce a different digit.
**What is wrong is that the device was being asked to compute a constant.**

## 3. The decision

`pdf_model::function::fold_constants`, run once after `compile_postscript` and before the program
is stored, replaces each run of instructions whose value is settled at compile time with that
value.

A run is folded only where it is a **closed island**:

- it begins with a literal push;
- it contains no jump and no jump target after its first instruction;
- it never reads below the operand-stack depth it began at;
- it ends as exactly one value above that depth.

Such a run is then evaluated **by `evaluate_postscript` itself** — the same instructions, the same
`apply_operator`, the same `Value` semantics, on a stack holding only the run's own values. The
answer is the same in every bit by construction rather than by argument, which is the property that
made this safe enough to ship in one round against an oracle of 1 794 pages.

Three things the walk has to model, and each was a defect in an earlier draft:

- **The absolute stack depth**, because `MAX_STACK` is the one context-dependence an isolated
  evaluation has: a push is dropped at the ceiling and `copy` declines above it. The walk abandons
  the whole program the moment the depth it models could reach the ceiling, after which neither
  guard can fire anywhere a fold happens. Modelling the depth needs each `if`'s two arms to leave
  the same one, which is **checked at the join** rather than assumed.
- **§B.5's three counting operators**, whose operand demand is a value rather than a constant.
  `operand_demand` reads the count off the stack, and a count the walk cannot name ends the fold —
  because a wrong depth makes the ceiling reasoning above wrong.
- **§7.10.5.3's underflow.** The first draft declined any program that reads past the bottom of its
  own operand stack, which would have left the one document the pass was written for exactly where
  it was: `pi_seven_segment.pdf` does that in three places. The walk now runs each operator over the
  window that is *actually there*, so the length `apply_operator` leaves is the length the evaluator
  will have, whatever the arm does. `EMPTY_STACK` costs it nothing.

## 4. What it bought

The witness's whole π computation collapses to one `PushInt(3141)`, and **no operator that WGSL
§15.7.4.1 gives an error budget is left in the program a device is handed.** So quorra's rule admits
it, unweakened, and the shading is evaluated per fragment.

Same command, same machine, at a *higher* load average (36.7 against 17.4):

| frame | pixels | total | scene | device | bytes uploaded |
|---|---|---:|---:|---:|---:|
| before | 400×400 | **19.8** | **0.2** | 19.6 (5.7 of it the shader's first compile) | 32 |
| zoom | 800×800 | **3.5** | **0.1** | 3.4 | 32 |

A zoom step: **96.1 ms → 3.5 ms**, and 2.5 MB of uploaded grid → none. Well inside a 60 Hz refresh,
which is what the report asked for.

`examples/function_paint_census` over 1 251 documents — pdf.js, the four `doc/corpora/`
submodules, `doc/corpora-own/`:

| | before | after |
|---|---:|---:|
| pages carrying a §8.7.4.5.2 program | 3 | 3 |
| device-evaluated shadings | 8 | **10** |
| refused on `Agreement::Unbounded` | 2 | **0** |
| refused on a typing ground (`mod` given a real) | 1 | 1 |

**Both agreement refusals in the population were the same shape and both are gone.** Three pages in
1 251 files is the honest denominator, and it is stated in `doc/QUORRA_FUNCTION_PAINT_EXACTNESS.md`
§4 rather than dressed up: this corpus barely exercises type 1 shadings.

## 5. Correctness, which is not negotiable here

The page draws digits, so a difference of *branch* rather than of colour would be a wrong digit.
Three things assert it:

- `folding_agrees_with_the_evaluator_over_every_operator` runs every arm of `apply_operator`
  through the fold over nine programs and three input sets, comparing **bit patterns and
  discriminants** — because a `-0.0` where a `0.0` was, or a real where an integer was, is a
  difference `==` hides and seven of Table 42's operators can see.
- `the_witness_pages_program_keeps_no_inexact_operator` reads the lowered `ProgramStep` list off
  the real document and asserts the class of operator is empty.
- `cpu_and_quorra_agree_on_the_witness_pages_folded_program` asserts the pair that matters: that
  the device evaluated the shading, *and* that its raster agrees with `render-cpu`'s. Either half
  alone would be worth little.

## 6. The owner's second question, answered

*Is there really no way to run type 4 functions on the GPU?* There is, and it was not a relaxation.

It was put to this round that quorra's rule might be unnecessary because IEEE 754 requires
`+ − × ÷ sqrt` to be correctly rounded. **That is true of IEEE 754 and false of the language quorra
generates**, and quorra had already written both halves down: `function_ops.wgsl` says WGSL
§15.7.4.1 allows `x / y` 2.5 ULP "where IEEE 754 requires the host's `/` to be correctly rounded",
and ADR 0053's own amendment of three days earlier withdraws the stronger claim for `add`, `sub`
and `mul` too, on §15.7.5's licence to reassociate and fuse. A narrowing argued from IEEE 754 would
be arguing about a language nobody compiles to.

**The lesson is `doc/habits.md`'s, one repository over**: before proposing that a neighbour's rule
is too strict, read what the neighbour wrote about it. Their amendment is dated 2026-08-15 and is
in this tree's sibling checkout; it is a stronger argument than the one this round was handed, and
it took ten minutes to find.

The full write-up, including the one narrowing that survives as a *lowering* rather than a rule
change (a `div` by a compile-time power of two is `x × 2⁻ᵏ`, which uses the operator WGSL treats
better), and the one question worth asking them — whether `function::analyse` should fold too,
since `Cell::literal` already exists and is populated only for the counting operators — is
`doc/QUORRA_FUNCTION_PAINT_EXACTNESS.md`.

## 7. What this does not do

- **It does not touch the grid.** A program whose `div` reads the shading's own coordinates is
  still refused, still drawn from the grid, and still costs one evaluation per device pixel. What
  such a page gets from this round is only whatever constants its own program carries, and on the
  witness — where the folded island is most of the arithmetic and none of the branching — that is
  **9.6%**: the same `callgrind` run reads 2 591 402 574 instructions before and 2 343 050 113
  after, with `libm`'s three `powf` and one of the three `truncf` gone entirely. The number is
  small because the folded instructions are the cheap ones; the seven-segment lookup that survives
  is where the interpreter's time goes. **That is the number to quote for a page that stays
  refused**, and quoting the 96.1 → 3.5 ms beside it would be quoting a different mechanism.
- **It does not make the grid cheaper in general.** Three levers were considered and not taken,
  each because the profile did not point at it: a cache keyed on the resolved grid (exact, but a
  zoom step changes the grid, so it buys repeat frames rather than the gesture); a coarser grid
  than §10.7.4's one-sample-per-pixel (a departure, and on a page of digits a visible one); and
  clipping the grid to the part of the domain the target covers (exact, and quadratic in
  magnification once a page is zoomed past the window — the one worth doing next, and it is a
  change to `ColourGrid`'s contract across three backends rather than a pass in one file).
- **It does not change `Shading::sampled_at`, `DeferredColours` or any backend.** The whole change
  is inside the compiler for one function type, and every consumer sees a shorter program that
  computes the same thing.

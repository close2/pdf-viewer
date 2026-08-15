# ADR 0376 — The colour a device computes, and the two witnesses it will not

Status: accepted, 2026-08-15. Session 541. Adopts quorra's `Paint::Function` for ISO 32000-2
§8.7.4.5.2's type 1 shading: the display list gains a second statement of a function-based
shading's colours — the §7.10.5 program itself — and `render-quorra` hands it to the device to
evaluate per fragment where the device will take it. Consumes ADR 0371's typed operand stack.
Extends ADR 0339 without amending it, and does **not** amend ADR 0364: the grid stays, because
it is what draws whenever the device declines.

The ask is `doc/QUORRA_FUNCTION_PAINT.md`, the answer `…_ANSWER.md`, the built release
`…_BUILT.md`, and the release's own section is in `doc/QUORRA_UPGRADE.md`.

## What was adopted, and what it cost to compile

Nothing. The release is additive in every part this tree touches: no `match` here scrutinises
`quorra_scene::Paint`, `ResourceId`, `DeviceError`, `RenderError` or `ReportKind`, so the
fourth `Paint` variant and the fifth `ResourceId` — breaking changes in general — broke
nothing, and `cargo check --workspace --all-targets` was clean with two hashes in `Cargo.lock`
as the only change. **That is the fourth release in a row to cost nothing to compile**, and
what this one cost instead is the boundary below.

## 1. The program is display-list vocabulary, and it is not the evaluator's

`render-quorra` can see `pdf-render` and nothing above it; the compiled program lives in
`pdf-model`. So a program that reaches a device has to cross that boundary, and the decision
was *which form crosses*.

**A new module, `pdf_render::program`, states the device-facing form**, and
`pdf_model::function` keeps its own. They differ in exactly two places, and both are the
difference between a thing that is *evaluated* and a thing that is *compiled*:

- **a literal carries its type in the instruction** (`PushInt`/`PushReal`/`PushBool`) rather
  than inside a `Value`, because a code generator has to decide between Table 42's two `not`s
  before it writes a line, where an evaluator can look at the operand;
- **a jump target is a `u32`**, because a program handed to a device is bounded by that
  device's budget.

The alternative — move `Instruction`, `Operator` and `Value` down into `pdf-render` and
re-export them — was considered and declined for one reason that is about layering rather than
effort: `Value`'s Annex B coercions (`number`, `integer`, `truth`, `equals`) would have to
become public API of a crate that evaluates nothing, so `pdf-render` would carry the semantics
of a language no backend runs. A crate with one stated responsibility is principle 4's, and
"the vocabulary a backend is asked to draw" is a smaller and truer responsibility than "the
vocabulary plus its meaning".

The cost is a thirty-eight-arm lowering in `pdf_model::function::device_step` and another in
`render_quorra::scene::function_op`. **Neither has a wildcard arm, and `ProgramStep` and
`ProgramOperator` are deliberately not `#[non_exhaustive]`** — the opposite of this crate's
habit — so that a Table 42 operator added on either side stops the other compiling instead of
reaching a device as something else. Table 42 is closed; a vocabulary that mirrors a closed
list should be closed too.

`ShadingKind::Sampled` gains `program: Option<ShadingProgram>` **beside** `source`, never
instead of it. `Shading::with_alpha` drops it, and has to: a device evaluating a program
produces a colour and nothing else, so §11.6.4.4's constant alpha has nowhere to go on that
path, while `DeferredColours::faded` carries it on the one that draws.

## 2. The conditions are the places the two paths would part

`pdf_model::shading::device_program` builds a program only where the grid's colour and the
device's are the *same arithmetic*, not merely a similar one. Each condition is a clause:

- **One function, not a group.** §7.10.5.3 lets `/Function` be *n* one-output functions; only
  one of *n* outputs is a single program, and stitching *n* programs into one would be
  arithmetic this tree invented.
- **`Compositing::Device`.** A `/Luminosity` mask group weighs colour into ink (§10.4.2.3) and
  a four-component page paints half a `DeviceCMYK` blend (§11.4.7); neither is a colour the
  program's own outputs are.
- **`DeviceGray` or `DeviceRGB`.** These are the two spaces where a component *is* a device
  component — §8.6.4.2 and §8.6.4.3 state no transformation — and `ColourSpace::to_rgb`'s arms
  for them are the identity plus a clamp. `DeviceCMYK` is §10.4.2.5's conversion and a
  calibrated space is §8.6.5.2's; a conversion is not something to restate in a shader, which
  is also quorra's own reason for leaving `DeviceCMYK` out of `FnRange`.
- **A `/Range` of the space's width**, which §7.10.5.3 requires and makes an error to differ
  from.
- **A function `/Domain` containing the shading's rectangle.** §7.10.1 clips a function's
  *inputs* to its domain; §8.7.4.5.2 makes the shading's rectangle a *region*, not a clamp. Where
  the first contains the second the clip is the identity over every point the device will ask
  about. Where it does not, one path would fold a strip onto its edge and the other would not.

**The `Range` handed over is intersected with `[0, 1]`, and that is exact rather than a
narrowing.** The grid clips to `/Range` and then clamps each component into the unit interval;
clamping into `[lo, hi]` and then into `[0, 1]` is clamping into the intersection. So a
document that declares a wider range — which §8.7.4.5.2's Table 78 explicitly provides for —
keeps drawing, on either path, with the same arithmetic.

## 3. The fallback is per paint, and the refusal has a voice

Quorra's admission (`quorra_gpu::function::admit`, plus `Analysis::admits` for the `Range`)
needs no adapter, so both questions are asked **before** the upload, where a refusal costs no
resource and a scene is still being built. A refused program returns `Ok(None)` and the
encoder draws `sampled_fill` — ADR 0364's parallel grid, uploaded as an image clipped to the
path, exactly as before this ADR. **The page never leaves the device for a program the device
declined**, which is a strictly smaller fallback than the whole-page one `surface.rs` has for a
refused *frame*.

Nothing failed, so nothing is an error — but a page that quietly stopped taking a path four
orders of magnitude faster is exactly the regression no timing attributes on its own. So
`render_quorra::FunctionPaints` carries a count and one ground per refusal, `--trace` prints
each ground on the frame it happens, and the two accessors are public so a test and a census
can assert on them.

**The program is cached, and it is the one resource here for which that is not an
optimisation.** A device keys its generated shader on the program's contents and drops the
shader when the last id naming them is released (quorra's ADR 0053), so a transient upload
would recompile the shader on every frame of a still page — quorra measured that compile at
6.3 ms cold. A fourth map in `render_quorra::cache`, keyed by the step list's `Arc` under the
module's own ABA argument, is what keeps it.

## 4. The tolerance, which is the decision this round owes — and it is *no tolerance*

`…_BUILT.md` §0 withdraws the exactness guarantee `…_ANSWER.md` §4 gave: WGSL §15.7.5 permits
reassociation and fusion, WGSL fixes no rounding mode, and ADR 0006's store rounding sits
between shader and texel regardless. The classification is now `Bounded`/`Unbounded` and they
recommend budgeting **1e-3 relative-or-absolute** for function-shading pages.

**This tree adds no tolerance and the corpus gate keeps its current strictness.** Two reasons,
and the second is the one that decides it.

### The argument: ADR 0339 bought a branch, not a bit

ADR 0339 replaced a fixed 128-cell grid with the device's own because the fixed grid blurred a
discontinuity across three device pixels at 4×, and §10.7.4's centre rule says which value a
device pixel carries. What that bought is **where the step falls** — a difference of *branch*.
It did not buy an exact colour and could not have: §7.3.3 defers a number's precision to "the
internal representations used in the computer on which the PDF processor is running", §7.10.5.2
defers Table 42 to PLRM3, and PLRM3 defers to the hardware. There is no clause either side can
be measured against.

So a difference *of colour* is not a loss of what ADR 0339 bought, and §10.7.3 — "each output
device may have internal limits" — is the clause that already measures a shading's accuracy in
that currency. A difference *of branch* would be a loss, and it is precisely what quorra's
admission refuses: any program where an inexact operator's value reaches a comparison, a
conditional or a truncation is `Unbounded` and never reaches the device at all.

### The measurement, which is what a gate is entitled to

An argument that the gate *should* hold is not the gate holding. `function_paint_census` over
1 479 corpus files finds **three** pages carrying a §8.7.4.5.2 program, and exactly one whose
program the device takes: `function_based_shading.pdf`, eight of its nine shadings evaluated
on the device and the ninth refused. Both arms, one working copy, RADV, the change the only
variable:

| `function_based_shading.pdf` | grid | device | gate's bound |
|---|---:|---:|---:|
| mean, scale 1 | 0.0178 | **0.0392** | 1.5 |
| worst tile, scale 1 | 1.171 | **1.201** | 7.0 |
| differing fraction, scale 1 | 0.000413 | 0.000414 | — |
| ssim, scale 1 | 0.99975 | 0.99952 | 0.99 |
| mean, scale 4 | 0.0047 | **0.0191** | 1.5 |
| worst tile, scale 4 | 1.555 | **1.582** | 7.0 |

The page moves *away* from the oracle — four times the mean at 1×, and its worst tile by
**0.03 of 255 against 5.4 of headroom**. One more channel of 1 453 824 differs at all. The
corpus gate's verdicts do not move: 931 agree, 23 differ, 2 refused, 18 not comparable, the
same as the round before.

A tolerance would therefore be a bound this project invented for a movement two orders of
magnitude below the bounds it already has — and it would have to be applied to *every* page,
because the gate has no way to know which page carries a function paint. Quorra's 1e-3 is
their own device conformance test's instrument and they say so; `QUORRA_FEEDBACK.md` §26.6 had
already told them nothing here depends on it, and that is still true.

### What is new and has to be written down

**The corpus gate's numbers are now, on one page, a property of the adapter.** Quorra declines
cross-adapter identity for this paint outright: a function-shading page rendered under
lavapipe is not evidence about the same page on RADV. The headroom above says a driver's
disagreement cannot reach the bound — 0.03 used of 5.4 — but that is an argument rather than a
measurement until a second adapter runs it, and the page to watch is that one.

**The paint is on by default and behind no flag.** The brief offered the other outcome and it
would have been legitimate; what rules it out is the census rather than an opinion. One
document in 1 479 takes this path, and it moves 0.03 of 255. There is no population for a flag
to protect, and a knob nothing needs is a knob that goes stale.

**The project owner may want to review this**, and it is the one item in this round that is a
judgement rather than a measurement: the gate's meaning on a function-shading page is now
"agrees with the oracle to within the bounds", where before it was "agrees by two evaluations
of the same code". The numbers say the bounds are ample. The change of kind is real anyway.

## 5. What it buys, honestly: nothing at all on the two documents that asked for it

`doc/QUORRA_FUNCTION_PAINT.md` was written about two files — the owner's seven-segment π and
`type4_pi.pdf` — whose whole content is one `sh` and whose scene build is ~100 ms of function
evaluation. **The device refuses both**, and by name:

```
`div` at 234 reaches `truncate` at 354, so no bound on the disagreement with an
independent evaluation can be stated          (pi_seven_segment.pdf)
`div` at 2 reaches `truncate` at 35 …          (type4_pi.pdf)
```

Which is correct. Both compute a digit of π by dividing and truncating, and a last-bit
difference in the divide is a different digit — the discontinuity §8.7.4.5.2 licenses, working
exactly as the classification says it will. It is also, precisely, the case `…_ANSWER.md` §4
measured at **zero differing pixels on both adapters** before the classification was
tightened. The two facts are not in conflict: an observation about two documents on two
drivers is not a bound, which is the correction quorra's own §0 makes.

So the frame line on `pi.pdf` is unchanged, the picture is unchanged, and the win is on a page
nobody had complained about. That is written here rather than smoothed over, and it is what
went back to quorra in `QUORRA_FEEDBACK.md` §27: the classification is sound and its cost is
the entire population the ask was written about.

## Consequences

- `pdf-render` gains `program.rs` and a public `ShadingProgram` on `ShadingKind::Sampled`;
  every backend but `render-quorra` ignores it, `render-cpu` included, so the correctness
  oracle draws what it always drew.
- A **stroke** painted with a function shading now draws on the device, where a sampled
  shading is still refused by name (`stroke.rs`). That follows from the paint being a paint
  rather than an image, is a strict improvement, and no corpus document exercises it.
- `FrameCost` is unchanged; the new observable is `FunctionPaints`, reached from both
  `QuorraRasterizer` and `QuorraPresenter`. It describes the **scene**, not the frame, so a
  replayed frame keeps reporting the paints its build chose.
- `cargo run --release -p render-quorra --example function_paint_census` is the population's
  instrument and answers both halves — which pages take the path, and the device's ground for
  each that does not.
- Types 0, 2 and 3 stay out of scope on both sides, exactly as `QUORRA_FUNCTION_PAINT.md` §7
  permits and `…_BUILT.md` §6 confirms.

# A paint the device evaluates — an ask about §8.7.4.5.2's function-based shading

Written 2026-08-15 from **this** side, against quorra at `87898c69` plus the retained-frame
revision. It is a request for an API addition, and it is the first thing this tree has asked
quorra for that would move *arithmetic* across the boundary rather than data. `doc/QUORRA_FEEDBACK.md`
is the standing document; this one is separate because it is a design question rather than a defect,
and because the answer may reasonably be **no** — §6 says what happens then, and that path is
already being built here in parallel.

## 1. The measurement that prompts it

A one-page document whose whole content is a single function-based shading, on this machine
(AMD 890M/RADV, and under `Xvfb`/llvmpipe for the trace below — the shape holds on both):

```
document joined          95.247 ms
interpreted, 1 cmd       95.632 ms  (+0.385)   ← the whole page is one command
first scene built      1238.480 ms  (+1142.848) ← this
first present          1269.331 ms  (+30.851)
frame p1 1cmd presented 1173.6 | host 0.0 scene 1142.7 device 30.8 | 3 up, 0 culled
```

`mutool draw -r 96` on the same file: **15–16 ms**, three runs. Interpretation is four tenths of a
millisecond and the device is thirty; **the second is spent evaluating the shading's function once
per device pixel, on the processor, before anything reaches you.**

Two documents, both a single `Sh`: one is 2580 bytes of PostScript calculator with `ifelse`
branches driving a seven-segment display, the other 1605 bytes computing π by the BBP series. The
cost is the same shape in both.

## 2. Why this tree evaluates per device pixel, and why that is not going to change

§8.7.4.5.2's type 1 shading is "a function of two variables" over a domain, mapped by a matrix. It
is not a gradient and no gradient expresses it: the function may be a §7.10.5 PostScript calculator
program, and at a discontinuity — which the two documents above are made of — the colour changes
between one pixel and the next.

This tree used to sample such a function into a fixed 128×128 grid and let the raster interpolate.
Session 504 (ADR 0339) replaced that with the device's own grid, because the fixed grid blurred
every discontinuity across three pixels at 4× magnification, and §10.7.4's centre rule is what says
which value a device pixel carries. **That decision stands**; the ask below is how to make the same
answer cheap, not how to go back.

What this tree is doing about it on its own side, independently of any answer here: removing the
per-evaluation allocations and parallelising the grid with `rayon` (session 529, in flight as this
is written). That is expected to be worth a large factor and to leave the *structure* unchanged —
one evaluation per device pixel, on the processor, uploaded as a raster.

## 3. Why the device is the right place for it

A function-based shading is a fragment shader written in another language. Every property that
makes the processor slow at it makes a GPU fast:

- it is **pure** — colour is a function of position and of nothing else, with no ordering between
  pixels and no state carried between them;
- it is **uniform** — the same program at every pixel, which is the shape a warp executes best;
- it is **exactly as parallel as the raster**, and the raster is already yours;
- and the result is **consumed where it is produced**: today this tree evaluates a grid, uploads it
  as an image, and asks you to sample it — an upload whose only reader is the pixel it was computed
  for. At a 1000×1000 placement that is four megabytes of round trip per frame per shading, and at
  a zoom step the whole grid is computed again because the device grid changed.

The zoom case is the one that will not be fixed by making the processor faster: the retained frame
(quorra ADR 0048, adopted here in session 516) replays an unchanged frame, but a zoom step is a new
scene, and a new scene means the whole grid again. A device-evaluated paint would make a zoom step
cost what any other paint costs.

## 4. What we would ask for, in the shape we think fits

**A paint whose colour is a small program, evaluated per fragment**, with the program supplied as
data rather than as source text. The vocabulary a PDF function needs is unusually small and
completely bounded, which is what makes this worth asking:

| PDF function type | what it is | what the device would need |
|---|---|---|
| **type 2** (§7.10.3) | `C0 + t^N (C1 − C0)` | a closed form; three constants |
| **type 3** (§7.10.4) | stitching — pick a sub-function by interval, remap `t` | a bounded loop over ≤ *k* intervals |
| **type 0** (§7.10.2) | a sampled table with interpolation | **a texture lookup** — this one you already do |
| **type 4** (§7.10.5) | a PostScript calculator program | a stack machine, below |

Type 4 is the one that carries the argument, and its subset is *defined by the standard* to be
tiny (§7.10.5.1): expressions over integers, reals and booleans; comments; **no strings or arrays**;
**no procedures** except the two branches of `if`/`ifelse`; **no variables and no names**. Table 42
is 42 operators — arithmetic, comparison, boolean, bit shifts, a five-operator stack group
(`copy dup exch index pop roll`) and `if`/`ifelse`. Nothing loops. Nothing allocates. Nothing can
fail to terminate: this tree compiles the program to a flat instruction list with **forward-only
jumps**, so its length bounds its own execution.

So the concrete shape, as a sketch rather than a proposal we are attached to:

```rust
// A program the device evaluates at each fragment, in the paint's own space.
pub struct FunctionPaint {
    pub program: Arc<[FnOp]>,     // flat, forward-only jumps, no loops
    pub domain:  [f32; 4],        // §8.7.4.5.2's Domain
    pub matrix:  Affine,          // unit square → the shading's space
    pub stack_depth: u32,         // maximum, known at compile time
    pub outputs: u32,             // components the program leaves
}
```

with `FnOp` a `#[repr(u32)]` enum quorra can lower to WGSL — either as a switch inside a loop over
the instruction list, or by generating a shader per distinct program and caching it by the
program's hash. **Which of those two you choose is entirely yours**; the second is likely faster and
the first needs no shader compilation on the frame path, which is the launch-path property this
tree cares about (`CLAUDE.md`'s startup rules: nothing on the launch path waits for warmth).

We would hand you the compiled form, not PostScript text: this tree already compiles §7.10.5 to
exactly such a list (`pdf_model::function`), and the lexical work — comments, tokens, `{}`
matching — is ours and stays ours.

## 5. The three things that decide whether this is a good idea, and one of them is ours

**5.1 Arithmetic agreement, and it is the sharp one.** `render-cpu` is this project's correctness
oracle: every backend is compared against it, pixel by pixel, and quorra's own corpus gate is that
comparison. If the device evaluates the function and the processor also evaluates it, the two
compute the same mathematics in different arithmetic — and a `truncate`, a `cvi`, a `ge` at a
boundary, or an `atan` will not agree at the last bit. On a **discontinuous** function that is not a
last-bit difference in the output: a comparison that lands the other side of `0.8 ge` gives a
different colour for that pixel entirely.

That is a real cost and it is ours to state, not yours to solve. Three answers we can see, and we
have not chosen:

- the gate accepts a bounded number of differing pixels along discontinuity boundaries, which
  weakens exactly the property ADR 0339 bought;
- or the processor keeps evaluating for the oracle and the device evaluates for the screen, so the
  two answers exist for different purposes — honest, and it doubles nothing because the oracle runs
  offline;
- or the shared operators are specified to the bit (IEEE 754 with stated rounding, and the
  transcendentals given an exact reference), which is a heavy contract to put on a device.

**We would rather hear which of these you can live with than propose one.**

**5.2 What it must not cost.** No shader compilation on the first-frame path (this tree treats
GPU bring-up as measured and bounded, and `Device::warm_up` exists for exactly that); no
per-frame allocation proportional to the program; and a refusal by name — `UnsupportedPaint` or
its successor — for any program the device declines, so this tree can fall back to the raster it
builds today rather than draw nothing. A silent wrong colour is worse than a slow right one.

**5.3 Whether the population justifies it.** Honest numbers from this side: function-based
shadings are **rare** in the wild — the corpus's witnesses are a handful of documents — but they are
*catastrophic* when present, which is the profile that makes a document unusable rather than slow.
If your answer is "not worth a shader path for a rare paint", that is a defensible answer and §6
is what happens.

## 6. If the answer is no

Nothing breaks and nothing waits. Session 529's work — one allocation-free evaluation and a
parallel grid — stands on its own and is what this tree ships regardless. What stays true without
you is the zoom case: every zoom step recomputes the whole grid on the processor, and the retained
frame cannot help because the scene genuinely changed. We would then look at adaptive subdivision
(evaluate coarsely, subdivide only where the value changes) with the accuracy question ADR 0339
raised — a quadtree must not re-blur the discontinuity that motivated the device grid in the first
place.

## 7. What we are not asking for

- Not a callback. A paint that calls back into this tree per fragment is not a thing a device can
  do, and we are not asking for a CPU shim that pretends otherwise.
- Not PostScript. The text, the comments and the `{}` are ours; you would receive a compiled list.
- Not types 0, 2 and 3 first. If only **type 4** is interesting to you, that is the whole win —
  types 2 and 3 are closed forms this tree can flatten into a gradient or a small table, and type 0
  is already a texture.
- Not a change to `Command::Image` or to the sampled path. `ImageSource::AtDeviceScale` (this
  tree's ADR 0210) stays exactly as it is; this is `Paint::Shading`'s question and no other.

---

## 8. The answer, received the same day

**Yes** — type 4 only, generated shader only, nothing built on either side. The reply is
`doc/QUORRA_FUNCTION_PAINT_ANSWER.md`, written from the renderer side against this document and
carried by the release this tree took in its five-hundred-and-thirty-second session (ADR 0367;
`QUORRA_UPGRADE.md`'s `a64a9084` section). Their ADR 0053 is the decision behind it. Four things
belong in *this* document, because they are answers to what *it* asked.

**§4's choice between the two shapes is made, and the expectation in it was wrong.** This document
guessed the interpreter would trade throughput for the launch-path property `CLAUDE.md` requires.
It loses on both: 133.7 ms against **0.060 ms** on RADV for the seven-segment witness, and a cold
pipeline compile of 596 ms to 4.5 s against **6.3 ms** for the generated shader. At 4× the
interpreter's pass *took the device down* with a hard recovery. So the shape that existed to keep a
compile off the frame path is the one that must never be near it — and the generated shader's 6.3 ms
is a warm-set question rather than a frame-path one.

**§5.1's question is answered by a fourth option this document did not list, and it is better than
all three.** Not a tolerance, not two answers for two purposes, and not a bit-exact contract:
**classify the program at admission.** A program reaching only the exactly-agreeing operators is
accepted and the device and the processor are the *same* answer — measured at zero differing pixels
over four million device pixels of both witnesses on both adapters — and a program that can reach a
transcendental on any path into a comparison is refused by name, falling back to the raster this
tree builds today. ADR 0339's property survives with no tolerance introduced anywhere, which is
what §5.1 feared it could not.

**§5.1's failure mode is real and was reproduced.** Their two adapters disagree bitwise on every
transcendental measured — and on `div` and `sqrt` — and `sin 0 ge` flips between them on 2 of 4 096
inputs before any oracle is involved. What follows for this tree's CI is the one promise they
decline to make: **cross-adapter identity is not offered for this paint**, so a function-shading
page under lavapipe is not evidence about the same page on RADV.

**§5.3's population question is unanswered and stays this tree's.** Their answer prices the
arithmetic, not the frame: the 0.060 ms is a bare full-viewport pass rather than a `Paint::Shading`
clipped, grouped and composited.

### What it costs to adopt, and why this tree has not started

Priced rather than begun, and the reason is in ADR 0367: what the answer converts is not a
dependency version but **the meaning of this tree's corpus gate**. An admission classifier decides
which paints the oracle comparison is exact for, and that is the instrument every other backend
claim rests on — a decision with its own ADR, not a line in a bump. What it would take, in order:

1. **The type tag §6.3 of the answer asks for**, which is this tree's alone and worth having
   regardless: `Instruction::Push(f32)` carries no type, so Table 42's integer `not` is
   unimplementable — `63 not` yields `0.0` where the standard says `-64`. Static slot-type
   inference on the pass that already computes stack depth, at zero run-time cost.
2. **The empty-stack contract written down** rather than inherited from an `unwrap_or`: a pop from
   an empty stack yields `0`, which is what `pi_seven_segment.pdf` depends on and what makes its own
   unlit-segment branch dead code. A choice, recorded as one.
3. **The classifier itself**, a dataflow walk over the same flat list, plus the conformance test
   their §7 requires before it is a contract: one program per dangerous operator refused, one per
   safe operator exact.
4. **The refusal path wired to the existing fallback**, which is the shape §5.2 asked for and which
   this tree already has — a paint quorra declines by name falls back to the grid built today.

Session 529's allocation-free evaluation and rayon grid are unaffected and ship regardless, exactly
as §6 said they would.

### And two defects in this tree, found while they read our compiled form

Both against PLRM3, which §7.10.5.2 makes normative for Table 42's semantics, and both left for a
round of their own so that a bump's page-level attribution stays readable (ADR 0367 §4):
`Operator::Round` rounds half away from zero where the clause requires half toward greater, and
`Operator::Eq`/`Ne` compare with an `f32::EPSILON` tolerance where PostScript `eq` is exact.
`QUORRA_FEEDBACK.md` §25.6 carries both with the reply that went back.

---

## 9. Answered, built, and adopted — and what the answer turned out to be worth

**This ask is closed.** Quorra built `Paint::Function` (`doc/QUORRA_FUNCTION_PAINT_BUILT.md`),
this tree took it in its five-hundred-and-forty-first session, and ADR 0376 is the adoption.
`doc/QUORRA_UPGRADE.md`'s `05fadc52` section has the range and what it cost. Four things belong
in *this* document, because they answer what *it* asked.

**§4's shape is decided and this document's guess about it was wrong, twice over.** It expected
the interpreter to trade throughput for the launch-path property; the generated shader wins on
both. And it offered the two shapes as though the choice were quorra's alone — it was, and the
one they took puts a shader *compile* on the frame path, which §5.2 said must not happen. What
makes that acceptable is not a change of mind but a cache: the compile is keyed on the program's
contents and a program is an uploaded *resource*, so this tree keys its own upload on the step
list's `Arc` and the compile happens once per program rather than once per frame.

**§5.1's question has a fourth answer, and then that answer was corrected.** Not a tolerance,
not two answers for two purposes, not a bit-exact contract: a static classification at
admission. But the classification's first name — `Exact` — was withdrawn by quorra themselves
(`…_BUILT.md` §0), on the ground that WGSL §15.7.5 permits reassociation and fusion and that
their zero-differing-pixels result was an observation about two documents on one driver on one
day. So what this tree gets is `Bounded`: a difference **of colour**, never of branch. **ADR
0376 decides that this needs no tolerance in the corpus gate**, on the argument that ADR 0339
bought a branch rather than a bit, and on the measurement that the one corpus page taking the
device path uses 0.03 of the gate's 5.4 of worst-tile headroom.

**§5.2's requirements are all met, and one of them is now stronger than asked.** A refusal is by
name and it happens at `Device::upload_function`, *before* a scene exists. And the fallback is
per **paint** rather than per page: a refused program draws from the grid inside the same scene,
so the page stays on the device. §5.2 asked only that this tree be able to "fall back to the
raster it builds today"; it can do so without giving up the frame.

**§5.3's population question is now answered, and the answer is the sharp one.** Function-based
shadings are rare, as §5.3 guessed. `cargo run --release -p render-quorra --example
function_paint_census` over 1 479 corpus files finds **three** pages carrying a §7.10.5 program
and **one** whose program the device will take. The other two are the very documents §1 measured
to write this ask: both divide and then truncate to extract a digit of π, which is an inexact
operator into an amplifier, and both are refused. **So the measured win on the pages that
prompted the ask is zero**, and what §6 said would happen if the answer were "no" — session
529's allocation-free evaluation and rayon grid carrying those documents — is what carries them
anyway.

That is not a failure of the answer. It is the answer working: a page whose colour is decided by
the last bit of a division is a page where two evaluations of one mathematics are entitled to
disagree, and drawing it twice as fast and sometimes differently is the trade this project does
not take.

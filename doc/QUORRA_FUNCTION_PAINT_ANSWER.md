# A paint the device evaluates — the answer

**Yes**, for type 4 only, as a generated shader only. Written 2026-08-15 from the renderer
side, against `QUORRA_FUNCTION_PAINT.md`. Everything below was measured or read from a
clause; where it is neither, it says so.

Your §5.1 asked us to choose among three answers rather than propose one. **We are not
choosing any of the three**, and §3 says why: the measurement says a fourth answer is
available that is *stronger* than all of them for the programs it accepts, and honest
about the ones it does not.

---

## 1. The performance question, settled

A spike: a WGSL stack machine for Table 42, built both ways your §4 left to us, run on
your own two witnesses read out of `doc/corpora-own/` at run time. The seven-segment
program compiles to **482 instructions**, the BBP π to **311**, both max depth 8, all jumps
forward — your compiled form's properties hold.

Full page, 1191×1684, device time, minimum of five, verified on a machine at load 29:

| | seven-segment | BBP π |
|---|---:|---:|
| **generated shader, RADV** | **0.060 ms** | 0.059 ms |
| interpreter, RADV | 133.7 ms | 105.9 ms |
| **generated shader, llvmpipe** | **1.97 ms** | 1.41 ms |
| interpreter, llvmpipe | 825.0 ms | 714.9 ms |
| your processor path, 1 thread, allocation-free | 4 988 ms | 4 083 ms |

At 4×, the generated shader is **1.08 ms** on RADV and 33.2 ms on llvmpipe.

Against your §1 baseline — 1 142.8 ms of scene building and 30.8 ms of device — that is
**four to five orders of magnitude**, and the 4 MB-per-frame grid upload becomes a **3.9 kB
program buffer**. The zoom case your §3 names as the one that survives a faster processor
becomes a uniform change: nothing is recomputed at all.

### Do not build the interpreter — it loses on both axes

We expected shape (i) to trade throughput for the startup property your §5.2 and our
`PLAN.md` §1.8 both require. **It does not; it loses that too.** Cold-cache pipeline
compile: the generated shader **6.3 ms**, the interpreter **596 ms at best of four cold
samples and 4.5 s at worst**. So the shape that was supposed to avoid a frame-path compile
is the one that cannot be allowed near a frame path.

And it is worse than slow. At 4× the interpreter's pass **took the device down** —
`radv/amdgpu: The CS has been cancelled … guilty of a hard recovery`, then
`Parent device is lost`. A 482-instruction loop over 32 million fragments exceeds what the
driver will let a single dispatch hold. That is a refusal that has to happen *before* the
frame as a fragment-count budget, not a `Report` after it — a paint that can lose the
device cannot be refused retrospectively.

(Method note, in case you measure this yourselves: RADV's pipeline cache keys on SPIR-V, so
perturbing a comment does **not** force a recompile. Every cold number above came from a
fresh `XDG_CACHE_HOME`.)

---

## 2. What the specification requires of the arithmetic: **nothing**

We went looking for the precision contract your §5.1 assumes exists on the PDF side. It
does not, and the silence is load-bearing enough to quote.

**ISO 32000-2 §7.3.3**, verbatim:

> The range and precision of numbers **may be limited by the internal representations used
> in the computer on which the PDF processor is running**; Annex C, "Advice on maximising
> portability", gives these limits for typical implementations.

Annex C is **informative**, and its "Real numbers" row only observes that modern computers
*often* use IEEE 754. **"IEEE 754" occurs exactly twice in the whole of ISO 32000-2** — that
informative row, and the Bibliography. It is **not** in clause 2, Normative references. We
verified that count by direct extraction, not by search.

**§7.10.5.2** makes PLRM3 normative for Table 42's operator semantics, and PLRM3 defers
again: *"the limits for real numbers in any implementation are those imposed by the native
floating-point representation of the underlying hardware platform … Not all implementations
adhere to this standard."*

Two deferrals, no number. **So there is no clause either of us can be measured against
here**, and the accuracy language that does exist nearby — §8.7.4.4 and §10.7.3 — is about
*not evaluating at every point*, and §8.7.4.5.2 explicitly says the function "need not be
smooth or continuous". It is unavailable at a discontinuity by construction.

---

## 3. Your option 3 is not merely unpurchasable — it is contradicted by measurement

We could not verify the WGSL specification's accuracy table remotely, so we measured the
thing instead: a compute shader running the operators a type 4 program can reach, on both
our adapters, against an `f64` reference.

| op | RADV max abs err | llvmpipe max abs err | **bitwise RADV vs llvmpipe** |
|---|---:|---:|---|
| `atan` | 3.2e-6 | 3.2e-6 | 375 of 4096 differ |
| `sin` | 8.5e-7 | 5.9e-8 | 3 201 of 4096 differ |
| `cos` | 7.9e-7 | 7.5e-8 | 3 334 of 4096 differ |
| `exp` | — | — | 2 660 of 4096 differ |
| `sqrt` | 2.0e-7 | 1.2e-7 | 618 of 4096 differ |
| `div` | 4.4e-6 | 3.3e-6 | 398 of 4096 differ |

**Our two adapters do not agree bitwise on a single one of these — including division and
square root.** And your §5.1's failure mode is reproducible rather than theoretical:

```
FLIP: `sin 0 ge` disagrees between the two adapters on 2 of 4096 inputs
FLIP: `cos 0 ge` disagrees between the two adapters on 2 of 4096 inputs
```

Two adapters, one shader, different boolean — before your CPU oracle is involved at all.
A contract specified to the bit would have to be honoured by a driver that has not agreed
to it, so option 3 is out on evidence and not on opinion.

(One correction we owe you, because we nearly sent it: a first pass measured this in ULP
and reported RADV's `sin` diverging by 3×10⁹ ULP. That was the metric, not the hardware —
ULP distance explodes across a zero crossing while the absolute difference is 1e-8. The
table above is absolute error. RADV's `sin` is accurate to six decimals at x = 12.)

---

## 4. The fourth answer: classify the program, and be exact for what you accept

Here is the finding that changes the shape of the question. **On both of your witnesses,
the device and the processor agreed with **zero** differing pixels** — four million device
pixels of deliberately discontinuous function, on both adapters, both shapes:

```
(ii) gen  seven-segment   exact 1759600   off-by-one 246044   differing 0   worst 1
(ii) gen  bbp-pi          exact 2005644   off-by-one      0   differing 0   worst 0
```

The 246 044 off-by-one on the seven-segment page is **not the program**. It is the
fixed-function float→unorm8 store conversion our ADR 0006 already documents and bounds —
llvmpipe alone reports zero of them, and BBP π has none on either adapter.

**Why it is exact:** neither witness calls a transcendental. Their operators are arithmetic,
comparison, integer and stack ops — the set on which IEEE 754 and every shipping driver
agree exactly. §5.1's danger is real, but it is **carried by a specific, statically
identifiable subset of Table 42**, not by function evaluation as such.

So the answer we propose is a **static classification at admission**:

- **A program that reaches only the exactly-agreeing operators is accepted, and the oracle
  relationship stays exact** — not bounded, exact, as measured above. Your corpus gate
  keeps the property ADR 0339 bought, with no tolerance added anywhere.
- **A program that can reach `atan`, `sin`, `cos`, `exp`, `ln`, `log`, `sqrt` or `div` on
  any path into a comparison is refused by name**, and you fall back to the raster you build
  today. That is the mechanism your §5.2 already asked for, used as the gate rather than as
  an afterthought.

This is stronger than your option 1 (no bound is asserted that the standard does not
supply), stronger than your option 2 (the two answers are not merely "for different
purposes" — for accepted programs they are the same answer), and it does not need option 3.

The classification is a dataflow walk over the same flat list, and it is cheap: the spike
already computes stack depth and slot types in one pass and would add this to it.

### The one thing we will not promise

**Cross-adapter identity for this paint.** The ±1 above is ADR 0006's known store rounding
and is inside the bound your CI already lives with, so nothing breaks today. But a
discontinuous function amplifies: if a future accepted program sits a comparison exactly on
a value the arithmetic reaches differently, one adapter can take a branch the other does
not. **A function-shading page rendered under lavapipe is not evidence about the same page
on RADV**, and we would rather say that now than have you find it in CI.

---

## 5. What we would refuse, with the ground stated

Each was demonstrated on a program that reaches it — a ground nobody can reach is not a
ground:

| ground | reason |
|---|---|
| an operator outside Table 42 | the compiled form must be closed |
| an operand stack deeper than the shader has slots | a WGSL array needs a constant size; both your witnesses need 8, the spike allows 64 |
| `copy`/`index`/`roll` whose count is not a literal | a generated shader cannot name a slot it cannot compute |
| a transcendental upstream of a comparison | §4 above |
| a fragment count × instruction count above the device's tolerance | the hard-recovery case in §1 |

---

## 6. Two defects in your own tree, independent of any of this

Found while reading `pdf-model/src/function.rs` to learn your compiled form. Both are
yours, neither depends on our answer, and both are against the PLRM3 that §7.10.5.2
normatively incorporates.

1. **`Operator::Round` uses Rust's `f32::round`, which is half-away-from-zero.** We ran it:
   `(-6.5).round()` is `-7`. PLRM3 requires half-toward-greater, i.e. **`-6`**. WGSL's
   `round` is half-to-even, which also gives `-6`. That is a genuine three-way disagreement
   with your implementation as the odd one out.
2. **`Operator::Eq`/`Ne` compare with an `f32::EPSILON` tolerance** where PostScript `eq` is
   exact equality. A tolerance on `eq` makes two distinct values equal near zero and does
   nothing at all at large magnitudes, which is the opposite of what it looks like it does.

And two contract questions your compiled form leaves open, which a device paint would force
either way:

3. **`Instruction::Push(f32)` carries no type**, and Table 42's `not` is two operators
   wearing one name — logical negation on a boolean, one's complement on an integer. Yours
   implements the logical one, so `63 not` yields `0.0` where the standard says `-64`.
   Unreachable in either witness. Static slot-type inference fixes it at zero run-time cost.
4. **`pi_seven_segment.pdf` pops an empty stack three times** and depends on the result
   being `0`, which makes its own "unlit segment" branch dead code. Your `unwrap_or`
   silently adopts that reading — as does ours, because the alternative is refusing your own
   witness. It is a choice, it is defensible, and it should be written into the contract
   rather than inherited.

---

## 7. What we are not committing to yet

This is an answer to a design question, not a milestone. Before anything ships:

- **The spike measured a bare full-viewport pass**, not the paint inside our compositor. A
  real `Paint::Shading` is clipped, possibly inside a group, and composited — the 0.060 ms
  is the arithmetic, not the frame.
- **Types 0, 2 and 3 are not in scope**, exactly as your §7 permits. Type 0 is already a
  texture, and 2 and 3 are closed forms.
- **The classification of §4 needs its own conformance test** — a program per dangerous
  operator, refused, and a program per safe operator, exact — before it is a contract rather
  than a plan.
- **We would want your answer on §6.3 and §6.4** (the type tag, and what a pop from an
  empty stack means) written down, because a contract neither side writes down is a
  contract neither side has.

The spike is at `crates/quorra-gpu/examples/function_paint/` in our tree with its numbers in
`doc/spike-function-paint.md`, and the clause work is in
`doc/research-function-paint-arithmetic.md`. Both are readable without building anything.

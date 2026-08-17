# Is the agreement rule necessary? — what we measured, and what we are not asking for

Written 2026-08-18 from **this** side, session 572, against quorra at the revision
`doc/QUORRA_UPGRADE.md` records for `Paint::Function`. It is a **finding rather than an ask**:
after the work below, this tree has **no page left that quorra refuses on agreement grounds**,
and the one thing we would still like from you is an answer to §6 — which is a question about
your own vocabulary and costs nothing if the answer is "no".

**Nothing here proposes weakening `Agreement::Unbounded`.** The project owner asked whether the
refusal was *necessary*; the honest answer turned out to be that it is, that your ADR 0053's
amendment already argues it better than we could, and that the page which prompted the question
did not need it lifted.

## 1. What prompted this

`doc/corpora-own/pi_seven_segment.pdf` — the seven-segment π page from
`doc/QUORRA_FUNCTION_PAINT.md` §1 — was refused:

```
`div` at 234 reaches `truncate` at 354, so no bound on the disagreement
with an independent evaluation can be stated
```

so the page fell back to the grid, and the grid is one processor evaluation of a 700-instruction
program per device pixel. `examples/zoom_frame` on the real Radeon 890M, minima of five rounds:

| frame | pixels | total | scene | device | transfer | bytes uploaded |
|---|---|---:|---:|---:|---:|---:|
| before | 400×400 | 24.6 | **21.1** | 3.4 | 0.5 | 640 032 |
| zoom | 800×800 | 96.1 | **93.5** | 2.7 | 0.7 | 2 560 032 |

`encode` was 0.0 in both. A `callgrind` profile of the same page under `RAYON_NUM_THREADS=1` put
**94.09%** of the whole rasterisation in `pdf_model::function::Function::eval_into`, so the
attribution is a profile rather than an inference. The evaluation was already divided across
`rayon` (781.3 ms serial against 93.5 ms on 24 threads at load 15–17, measured by A/B in one
sitting), so parallelism was not the missing thing.

## 2. The premise we were handed, and why it does not hold

It was put to us that the refusal might be unnecessary because IEEE 754 requires `+ − × ÷ sqrt`
to be **correctly rounded**, so two conforming implementations agree bit for bit, and only the
transcendentals are genuinely irreproducible.

**That is true of IEEE 754 and it is not true of the language you generate.** Your own
`function_ops.wgsl` says so at the site:

> WGSL §15.7.4.1 allows `x / y` 2.5 ULP where IEEE 754 requires the host's `/` to be correctly
> rounded, which is why `Binary::is_inexact` names this operator even though the caller did not.

and your ADR 0053's amendment of 2026-08-15 goes further than we would have dared:

> **Bit-exactness is not available even for `add`, `sub` and `mul`.** WGSL §15.7.5 permits an
> implementation to **reassociate and fuse** floating-point operations … §15.7.4's "correctly
> rounded" is weaker than IEEE 754's because WGSL specifies no rounding mode; and ADR 0006's
> store rounding still sits between the shader and the texel.

So `Binary::is_inexact` naming `Div` is right, and a narrowing argued from IEEE 754 would be
arguing about a language nobody here compiles to. We checked the tree for the other half of the
premise — `VK_KHR_shader_float_controls`, `SPV_KHR_float_controls`, `NoContraction`, a rounding
mode — and found no occurrence of any of them in `render-lib`. That is consistent with the
amendment rather than a gap: the amendment's position is that the guarantee is not available, not
that it was left on the table.

**One narrowing that was proposed does survive as a *lowering* rather than as a rule change**, and
we mention it only because it may be cheap for you: `div` by a divisor that is a compile-time power
of two is an exponent adjustment, and `x * 2⁻ᵏ` uses the operator WGSL treats better than `/`. It
would move such a `div` out of `is_inexact` without weakening anything. We have not measured how
many programs it would reach and, after §3, this tree has no page that needs it.

## 3. What actually answered it: the operands were literals

The BBP series `pi_seven_segment.pdf` evaluates is

```postscript
0 dup 8 mul 0 index 1 add 4 exch div 1 index 4 add 2 exch div sub
  1 index 5 add 1 exch div sub 1 index 6 add 1 exch div sub
  exch pop exch 16 exch exp div
```

three times over, for k = 0, 1, 2, and then `add add 1000.0 mul truncate cvi`. Every operand of
every `div` and every `exp` in it traces back to a literal the file wrote down. **The series is a
constant** — 3141 — recomputed once per device pixel, on both paths, for as long as the feature
has existed.

`pdf_model::function::fold_constants` now computes it at compile time. The pass folds a run of
instructions that is a *closed island*: it begins with a literal push, contains no jump and no
jump target, never reads below the depth it started at, and ends as one value. Such a run is
evaluated by this tree's own `evaluate_postscript`, on a stack holding only the run's own values,
so the answer is the same in every bit by construction rather than by argument. The witness's
whole π computation collapses to one `PushInt(3141)`.

**After that, the program a device is handed carries no operator WGSL §15.7.4.1 gives an error
budget at all** — no `div`, no `exp`, no transcendental — so `Agreement` has nothing to bound and
your rule admits it unchanged. Two tests pin it: one in `pdf-model` asserting the lowered
`ProgramStep` list holds no such operator, and one in `render-quorra` asserting that the device
evaluates the shading *and* that its raster agrees with `render-cpu`'s, which is the oracle.

The measurement, same command, same machine, at a *higher* load average (36.7 against 17.4):

| frame | pixels | total | scene | device | bytes uploaded |
|---|---|---:|---:|---:|---:|
| before | 400×400 | **19.8** | **0.2** | 19.6 (5.7 of it the shader's first compile) | 32 |
| zoom | 800×800 | **3.5** | **0.1** | 3.4 | 32 |

A zoom step went from 96.1 ms to 3.5 ms and from 2.5 MB of uploaded grid to none.

**On the processor path the same fold is worth 9.6%** — 2 591 402 574 instructions before against
2 343 050 113 after, on the same `callgrind` run — because the instructions it removes are the
cheap ones and the seven-segment lookup that survives is where the interpreter's time goes. That is
the number a page which *stays* refused would get, and it is the honest one to set beside the
device figure rather than under it.

## 4. The population, and the honest size of it

`examples/function_paint_census` over 1 251 documents — the pdf.js corpus, the four
`doc/corpora/` submodules and `doc/corpora-own/` — before and after the fold. The census drives
the shipping path, so these are refusals a page would really take.

| | before | after |
|---|---:|---:|
| pages carrying a §8.7.4.5.2 program | 3 | 3 |
| device-evaluated shadings | 8 | **10** |
| refused, drawn from the grid | 3 | **1** |

Bucketed by the ground, which names the operators:

| ground | before | after |
|---|---:|---:|
| `div` reaches `truncate` (`Agreement::Unbounded`) | 2 | **0** |
| `mod` was given a real, and requires two integers (a typing refusal) | 1 | 1 |

**Both agreement refusals in the whole population were the same shape and both are gone.** What is
left is `function_based_shading.pdf`'s `mod`, which is `function::analyse` refusing an operand
type rather than anything about arithmetic, and which is correct.

**Three pages in 1 251 files is the honest denominator**, and we say it plainly: this corpus barely
exercises type 1 shadings, so none of the numbers above rank a narrowing of your rule. They rank
the *shape* — every agreement refusal anybody here has ever seen had literal operands — and that is
a claim about two documents, both of which the project owner wrote.

## 5. What we are not claiming

- Not that `div` is exact on a GPU. Your `function_ops.wgsl` and ADR 0053's amendment say
  otherwise and we agree with both.
- Not that folding makes any *general* program admissible. It removes an operator only where every
  operand of it is a literal; a `div` by a value derived from the shading's own coordinates is
  untouched and must stay refused.
- Not that the corpus supports a change to `Agreement`. It does not: two witnesses, one author.
- Not that bit-exactness holds anywhere across the boundary. What folding buys is that the
  operators whose two evaluations could disagree are *not evaluated on the device*, which is a
  statement about the program rather than about the arithmetic.

## 6. The one thing worth asking you

`Agreement::Unbounded` is decided by a walk over the program you are handed. This tree now folds
before handing it over, so you would see the same effect on any caller that does — but a caller
that does not will keep hitting the rule for programs whose operands are constants.

**Would you fold in `function::analyse` as well?** The information is already there: `Cell` carries
`literal: Option<f32>`, and it is populated only for the counts `copy`, `index` and `roll` need. An
operator all of whose operand cells carry a literal has a literal result, and propagating that
through the arithmetic would (a) let `Walk::amplify` skip a taint that no device will ever compute
and (b) let `generate` emit a constant instead of a call. Neither weakens the rule: an operation
the shader does not perform cannot disagree with one this tree did.

If the answer is no, nothing here is blocked. We fold on our side and the two witnesses draw on
the device either way.

## 7. Where this lives on our side

- `crates/pdf-model/src/function.rs` — `fold_constants`, `operand_demand`, `Islands`, `rewrite`,
  and the eight tests under them, including the differential one that runs every arm of
  `apply_operator` through the fold and compares bit patterns and discriminants.
- `crates/render-quorra/tests/headless_quorra.rs` —
  `cpu_and_quorra_agree_on_the_witness_pages_folded_program`.
- ADR 0406 has the argument and the numbers; `doc/history/572-*.md` the round.

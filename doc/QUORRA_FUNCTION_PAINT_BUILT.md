# The function paint is built — what you must do, and one correction we owe you

Written 2026-08-15 from the renderer side. This supersedes the "not committing to yet" list
in `QUORRA_FUNCTION_PAINT_ANSWER.md` §7, and **corrects that document's §4**.

`Paint::Function` renders end to end: a §7.10.5 type 4 program is uploaded once, admitted or
refused by name at upload, generated into a WGSL shader cached by the program's content
hash, and drawn through the same coverage, clip and soft-mask weighting every other paint
uses. **385 tests pass on both RADV and llvmpipe.**

**Not pushed yet** — the release round you took (`a64a908`) does not contain this. Say when
you want it.

---

## 0. The correction, first, because we told you something stronger than we can hold

`QUORRA_FUNCTION_PAINT_ANSWER.md` §4 said that for accepted programs *"the oracle
relationship stays exact — not bounded, exact."* **That was overclaimed and we withdraw it.**

Two of our own implementers objected while building it, independently, and they were right.
**WGSL §15.7.5 permits an implementation to reassociate and fuse** floating-point
operations, and the straight-line expression tree a generated shader hands the compiler is
exactly the shape that invites it. WGSL also specifies no rounding mode, so its "correctly
rounded" is weaker than IEEE 754's. And ADR 0006's fixed-function store rounding still sits
between the shader and the texel regardless — the spike measured 246 044 texels off by one
from that step alone.

The zero-differing-pixels result we sent you was real. It was an observation about **two
documents on one driver on one day**, and we promoted it to a guarantee.

**What we can hold, and what the classification now says:** for an accepted program the
disagreement stays a difference *of colour*, not a difference *of branch* — which is the
currency ISO 32000-2 §10.7.3 already measures a shading's accuracy in, and the currency your
own ADR 0339 works in. The classification is now `Agreement::Bounded` / `Unbounded` rather
than `Exact` / `Approximate`, and the rename is the whole of the correction.

For your gate, the practical consequence: **budget a small tolerance for function-shading
pages rather than expecting byte equality.** Our own device conformance test uses **1e-3
relative-or-absolute**, derived from WGSL §15.7.4.1's loosest row (`atan` at 4 096 ULP ≈
4.9e-4 relative). That is our test's instrument, not a claim about ISO 32000-2, which
supplies no precision contract at all.

---

## 1. What you must build to emit a function paint

### 1.1 Upload the program, once

```rust
let program: Vec<quorra_scene::FnOp> = /* your compiled §7.10.5 list */;
let id: FunctionId = device.upload_function(&program)?;   // may refuse, by name
```

**Refusal happens here, before you build a scene** — which is the point. If your program is
one we cannot draw, you learn it while you can still fall back to the raster you build
today, rather than mid-page. Release it with the same path as any other resource.

### 1.2 The instruction vocabulary

`quorra_scene::FnOp` is Table 42, closed. Three things differ from your
`pdf_model::function` compiled form and are work on your side:

1. **Literals are typed** — `PushInt(i32)`, `PushReal(f32)`, `PushBool(bool)`. This is the
   fix for the defect we reported: Table 42's `not` is two operators wearing one name, and
   an untyped literal cannot tell them apart. `63 not` is `-64`, and it is now expressible.
2. **`if`/`ifelse` are already lowered** to `JumpUnless { target }` / `Jump { target }`,
   **forward only**. We validate that rather than trust it — it is what bounds execution.
   The `{}`, the comments and the tokenising stay yours.
3. **Stack depth is not supplied.** We compute it. A caller cannot lie about it.

### 1.3 The paint

```rust
Paint::Function {
    program: FunctionId,
    domain: Rect,            // §8.7.4.5.2 Domain, in the shading's own space
    matrix: Affine,          // §8.7.4.5.2 Matrix: shading space -> scene space
    range: FnRange,          // §7.10 Range, and the component count with it
    background: Option<Color>,
}
```

`FnRange` is `Gray([f32; 2])` or `Rgb([[f32; 2]; 3])` — **the variant is the component
count**, so a count and a bounds array cannot disagree. DeviceCMYK is refused by name:
colour conversion is settled upstream, and our dependency policy forbids a colour-management
crate here.

**`background` must be `None` for a shading painted with `sh`.** ISO 32000-2's `Background`
entry says so as a `shall`: *"The background colour shall be applied only when the shading
is used as part of a shading pattern, not when painted directly with the `sh` operator."*
Both your witnesses are single `sh` commands, so both are `None`.

---

## 2. Breaking changes

**`Paint` gains a fourth variant.** Any exhaustive `match` on it stops compiling. `Paint`
**keeps its `Copy`** — we went out of our way to preserve that, and the first design that
lost it was rejected for exactly this reason.

`quorra_scene` gains `FnOp`, `FnRange`, `FunctionId`, `MAX_PROGRAM_LENGTH`, `check_program`;
`ResourceId` gains a `Function` variant; `DeviceError` and `RenderError` gain the refusal
variants; `ReportKind` gains `FunctionEmptyStackRead`. All additive except the `Paint`
variant and the `ResourceId` variant.

---

## 3. Two decisions we took that are properly yours, and are visible rather than silent

ADR 0053 §3.2 left two contract questions with you. We had to choose to build at all, so we
chose, documented the choice, and made it observable:

1. **A pop from an empty operand stack yields integer `0`** — matching your evaluator, which
   your own `pi_seven_segment.pdf` depends on three times. The standard defines nothing here.
   **It raises a `Report`**, so the reading is never adopted invisibly. The report is worded
   for what is actually known: *"reads an empty operand stack in N places"* — a **static**
   count over the program, not a claim that a fragment relied on it. We chose **integer**
   rather than real because seven operators (`not`, `and`, `or`, `xor`, `idiv`, `mod`,
   `bitshift`) can tell the difference, and your float-only `unwrap_or(0.0)` cannot express
   it either way.
2. **`gt`/`ge`/`lt`/`le` compare a boolean numerically** where PLRM3 raises `typecheck`. Left
   deliberately, because it is the same shape as every other guarded error and it is your
   question. Tell us and we will refuse instead.

---

## 4. What we refuse, and what that costs you

Fifteen grounds, each demonstrated by a program that reaches it. The ones you are most
likely to meet:

| ground | what to do |
|---|---|
| an operator outside Table 42 | cannot happen — `FnOp` is closed |
| a program that can reach a transcendental **on a path into a comparison** | fall back to your raster; this is ADR 0053 §3's whole point |
| `copy`/`index`/`roll` with a count we cannot resolve statically | fall back |
| output count ≠ the `Range`'s component count | a **defect in the file or in the compile**; §7.10.5.3 makes it an error |
| DeviceCMYK | convert upstream |
| a program past `MAX_PROGRAM_LENGTH` | fall back |

Note the fourth: we had it as a floor and the conformance corpus caught it. §7.10.5.3 says
it *"shall be an error"* for the counts to **differ**, so it is an equality, and a program
leaving three values under a one-component `Range` used to be drawn.

---

## 5. Two defects we found in your tree while building this

Both against the PLRM3 that §7.10.5.2 normatively incorporates, both independent of our
answer, both in `pdf-model/src/function.rs`:

1. **`Operator::Round` uses Rust's `f32::round`**, which is half-away-from-zero: `(-6.5)`
   gives `-7`. PLRM3 requires half-toward-greater, i.e. **`-6`**. We ran it to confirm. Note
   that WGSL's `round` is *also* wrong here in the other direction — `6.5 round` is `7` in
   PLRM3 and `6` in WGSL — so **neither built-in is the clause** and the tie has to be
   implemented by hand. Ours is.
2. **`Operator::Eq`/`Ne` compare with an `f32::EPSILON` tolerance** where PostScript `eq` is
   exact. A tolerance on `eq` makes distinct values equal near zero and does nothing at large
   magnitudes.

And one we found in **our own** wave-1 code by running your corpus against it, which is worth
your knowing because your evaluator has the same shape: **`true 1 eq` returned `true`.** On
the operand stack a boolean *is* `1.0`, so a numeric comparison silently inverts PLRM3's rule
that a boolean and a number are never equal. It is a wrong colour that no test we had could
see. Ours is now decided from the static types.

---

## 6. What is still not done

- **No test draws a knockout group over a function paint.** The pipeline pair compiles and is
  selected; nothing exercises it.
- **No retained-frame test** over a function paint.
- **No generated-compile duration of our own.** The spike's 6.3 ms cold is still the only
  figure; our machine reached load 66 during the round and its wall clocks are worthless at
  that load, so the test asserts the property — one compile, then none — instead.
- **Types 0, 2 and 3 are still out of scope**, exactly as your §7 permits.

# 536 — The three types a clause states, and the two contracts a device asked for

2026-08-15. One round, taking the defect the quorra team reported in
`doc/QUORRA_FUNCTION_PAINT_BUILT.md` §5 and the two contract questions in their §3. ADR 0371 is the
decision and the argument; this is the record.

## What was taken

**A type 4 operand stack holds typed values, and ours held `f32`.** The consequence quorra found in
their own device-side evaluator, and reported because ours had the same shape, was verified here
before anything was changed: `{ true 1 eq }` answered **true**, `{ true 1 ne }` answered false, and
`{ 63 not }` answered `0` where §B.3's integer operator says `-64`.

The stack now carries §7.10.5.1's three types — `Value::Integer(i32)`, `Value::Real(f32)`,
`Value::Boolean(bool)` — and the round is really about how much of that reading turned out to be the
standard's own. Annex B's **operand and result columns** decide `eq` and `ne` (`any 1 any 2`, so
across the types), the four ordering operators (`num 1 num 2`, so not on a boolean at all), `and`,
`or`, `xor` and `not` (`bool | int`, and only `not`'s two disagree), `cvi` against `truncate`, and
`if`'s condition. And §7.10.5.2 hands the operand *syntax* to PDF rather than to PostScript, so
§7.3.2 and §7.3.3 give a literal its type: `63` is an integer because the file wrote it without a
PERIOD. The static slot inference `doc/QUORRA_FEEDBACK.md` §25.5 had promised was not needed.

## The one rule that replaced a family of decisions

PostScript raises `typecheck` where an operand's type is wrong and §7.10.5.1's subset has no value
meaning *error*. Rather than decide operator by operator, one rule is stated once and applied
everywhere: **such an operand is converted by the reading that loses least** — a boolean is 1 or 0
where a number is wanted, a number is false exactly when it is zero where a boolean is wanted, a
real is truncated where an integer is wanted — with `eq`/`ne` outside it because `any 1 any 2`
admits both types already. §7.3.3's "[w]herever a real number is expected, an integer may be used
instead" is what makes one of those three directions not a choice at all.

## The two contract answers, which were the round's other half

- **A pop from an empty operand stack is the integer `0`.** Not a refusal, because refusing would
  refuse a document that draws; integer rather than real because §7.3.3 makes an integer an operand
  everywhere a real is expected and not the reverse; integer rather than boolean because a `false`
  would silently satisfy `if` and `not`. We do not report it, and the reason is a difference between
  the evaluators rather than a disagreement: quorra can count underflows statically because they
  refuse a dynamic `copy`, `index` or `roll`, and this one admits those.
- **`gt`, `ge`, `lt` and `le` read a boolean as the number it stands for** — the general rule above
  — rather than refusing. Quorra offered to refuse and does not need to: a refusal falls back to this
  evaluator, which answers this.

## The census

`crates/pdf-model/examples/type4_type_census.rs`, new, over 67 690 paths: **7 360 type 4 functions
in 2 102 documents, every one compared arm against arm, and 0 moved.** Exactly one program in the
population can put a boolean where an `eq` will see it — the owner's `pi_seven_segment.pdf` — and
both witnesses for the ordering answer are his two files.

The "before" arm is a source rewrite that is a *derivation* rather than an approximation, because
the old evaluator is this one with every value forced to a real: integer literals rewritten as
reals, `true`/`false` as `1.0`/`0.0`, a `cvr` after every operator that now answers an integer or a
boolean, `eq` as `sub 0 eq cvr` and `not` as `0 sub 0 eq cvr`. Validated on ten synthetic files
first, five built to move and five built not to; it moved exactly the five.

## The performance, which is where the round's time went

A typed value is bigger than an `f32` and the first arrival was **+22% instructions on all three**
type 4 documents. Four measured changes closed it — an `i32` payload so a value is eight bytes,
matching on the operand *pair* with the real case first, writing a one- or two-operand answer where
its first operand already sits instead of popping and pushing, and hoisting the step ceiling out of
the interpreter's loop — and the end state is ADR 0364's two big documents **faster** than before:
`type4_pi.pdf` −15.0%, `pi_seven_segment.pdf` −5.2%. `function_based_shading.pdf` is **+6.2%** and
stays there: its nine programs are a handful of operators each, so the per-evaluation framing is the
whole cost, and the operand stack can no longer *be* the output buffer. The control page moved by
8 428 instructions in a billion.

## What moved: nothing, and it is proven

- `display_list_digest` over all 974 pdf.js documents, before and after in one working copy with one
  `pdf-sandbox-worker` on disk: `diff` empty.
- Every gate on its previous number: corpus 974 / 64 incomplete; oracle 1 794 pages, 906 / 67 / 786;
  quorra scale 1 on 931/23/2/18 and the `gpu` lane at 4× on 937/9/5/23; text extraction 99.2%, 99.8%
  and 98.26%; nextest 1968/1968, doctests, conformance, dates, xmp, jpeg2000 all pass.
- The owner's two type 4 files rendered at 2× before and after: byte-identical, and looked at — the
  seven-segment display still reads 3.141.

`doc/todo/00`'s step 7 ink sweep is not owed: it is for a round that changes what gets drawn.

## Written back

`doc/QUORRA_FEEDBACK.md` §26 answers their §3 and §5 where they asked, accepts their §0 correction,
and sends two things they can use: that a literal's type comes from §7.3.2 and §7.3.3 rather than
from an inference, and that integer arithmetic here is exact where an `f32` shader will round.

## Files

`crates/pdf-model/src/function.rs`, `crates/pdf-model/src/shading.rs`,
`crates/pdf-model/examples/type4_type_census.rs` (new), `doc/conformance/ledger.toml` (§7.10.5,
§7.10.5.2, §7.10.5.3), `doc/adr/0371-…`, `doc/QUORRA_FEEDBACK.md`, this file.

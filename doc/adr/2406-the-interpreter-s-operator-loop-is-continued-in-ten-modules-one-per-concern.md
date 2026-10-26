# 2406 — `Interpreter`'s operator loop is continued in ten modules, one per concern

Session 1786, the standing code-quality slot of batch 118. Status: implemented; no behaviour changed, no bug fixed. ADR 2407 is unused.

`tools/state.sh gods` named the impl block of `Interpreter` in `crates/pdf-model/src/content/run.rs` next (ADR 2390
left `Carry`; `App` in `typing.rs` is the second): one impl block of 1 579 lines in a file of 2 241. The handover's
candidates were not taken first because `gods` names this one first.

**What it is now.** `run.rs` keeps the loop itself: `run`, `run_source`, `run_from`, `settle_step`, `run_operator`
(the one flat `match` over Table 50, argued in `run`'s doc comment) and the stream's end. Beside it, in `run/`:

- `steps.rs` — `next_step`, `Step`, `Keyword`: one token read into what the loop acts on;
- `operands.rs` — `operands_before`, `Operator`'s operand counts, the typed readers, and the tests of Table 50;
- `machine.rs` — `Machine`, `Composed`, `Leftover`: what a run holds and what it leaves;
- `graphics.rs` — graphics state, path construction and painting, colour operators;
- `text_ops.rs` — the four text groups of Table 50;
- `xobject_ops.rs` — `Do`, `sh`, the inline image operators;
- `marked_ops.rs` — the marked-content operators, `BDC`'s property list and `EMC`'s readback range;
- `compat_type3_ops.rs` — `BX`/`EX`, `d0`/`d1`;
- `nested.rs` — a nested content stream's decode and §8.9.7's inline image;
- `notes.rs` — the report entries the loop raises.

**Why these seams.** The operator families are Table 50's own rows, so a reader asking "what does `B*` do" finds the
family's file by the table; the loop's argument for one `match` is unchanged because the `match` is still one. The
hypothesis (four to six modules) was exceeded because the file also held three kinds of non-method code (the machine's
types, the token step and the operand readers).

**Cost.** Methods a sibling or another module of `content` calls kept their reach as `pub(in crate::content)`; the
seventeen methods the loop or a sibling calls across the new seams became `pub(super)` (E0624 named them); `Machine`'s and `Leftover`'s
fields became `pub(super)` because the families destructure `Machine`; `Composed` and `Leftover` stay reachable to
`divide`. The root is 532 lines, of which the loop and its module documentation are all but 25.

Alternatives not taken: one module per Table 50 row (eight files under 60 lines); leaving the machine's types in the root
(the root at 700 lines).

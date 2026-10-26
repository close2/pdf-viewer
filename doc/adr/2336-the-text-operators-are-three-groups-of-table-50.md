# 2336 — The text object, text state and text positioning operators are three groups of Table 50

Session 1751, the standing code-quality slot of batch 114. Status: implemented; no arm's behaviour changed, no bug fixed.

`Interpreter::run_text_op` in `crates/pdf-model/src/content/run.rs` was the longest arm-method the operator
loop's family split (ADR 2304) left: one `match` of thirteen operators, about 125 lines, in which `BT`'s
reading-scope bookkeeping, seven one-line parameter setters and four matrix updates sat together. Table 50 groups
them, and the groups use different state, so the split follows the table.

**What it is now.** `run_text_op` is a dispatch of 20 lines that sends each operator to the method for its group:

- `run_text_object_op` — `BT` and `ET` (§9.4.1); it destructures `state`, `text_object` and `in_text`, the three
  fields the object's opening and closing touch;
- `run_text_state_op` — `Tc`, `Tw`, `Tz`, `TL`, `Tf`, `Tr`, `Ts` (§9.3); it is handed the `GraphicsState` and
  nothing else of the `Machine`, so its signature says that a text-state operator cannot reach the text matrices;
- `run_text_positioning_op` — `Td`, `TD`, `Tm`, `T*` (§9.4.2); an associated function without `self`, because it
  reads the two matrices and the leading and calls nothing of the interpreter (`clippy::unused_self` said so).

**Proof.** The multiset of the file's non-comment lines against HEAD's (script `split.py`, anchors asserted):
the arms' bodies moved byte for byte and the file loses no line; it gains the four signatures, three
`match operator {` heads with their wildcard arm and closing braces, two destructuring prefixes, the dispatch arms
and three doc comments.

**Choices.** Each group keeps the wildcard arm ADR 2304 explains; `run_operator` remains the one exhaustive match.
The dispatch lists the operators a second time, which is the price of the group boundary being visible at the
call and not only in the callee's signature.

**Gates.** See `doc/history/1751`.

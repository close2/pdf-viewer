# 2390 — `Carry` is continued in six modules, one per concern

Session 1778, the standing code-quality slot of batch 117. Status: implemented; no behaviour changed, no bug fixed. ADR 2391 is unused.

`tools/state.sh gods` named `Carry` in `crates/pdf-transform/src/structure.rs` next (ADR 2368 left it): one impl block of
1 085 lines and 21 methods in a file of 1 417 lines. It is not one `match`; the methods are six kinds of work on the one
state, so it is split as `Vfs` was.

**What it is now.** `structure.rs` (451 lines) keeps the module documentation (the argument of §14.7's namespaces),
`Host`, the data types (`Child`, `Element`, `Outcome`, `Carry`, `Wrapper`, `CarriedPage`), `Carry::plan`, which drives
the others, and the free functions that read a `/K` or a `/StructTreeRoot`. Beside it, each continuing `impl Carry`:

- `reconcile.rs` — `read_root`, `check_ids`, `read_mark_info`: Table 354's root entries, the `/IDTree` refusal and
  Table 353's flags;
- `hierarchy.rs` — `decide`, `keep`, `child`, `number`: which elements reach a carried page, and the slots they take;
- `wrapper.rs` — `wrap_documents`, `pdf_2_0_namespace`: Annex L's one `Document`;
- `keys.rs` — `page_key`, `object_key`, `root`, `mark_info`: the parent-tree keys and the plan's small answers;
- `write.rs` — `finish`, `element`, `wrapper_element`, `parent_tree`, `source_entry`, `element_reference`;
- `report.rs` — `report`.

**Why these seams.** `plan` calls the reconcile, hierarchy and wrapper steps in the order the module comment argues, so
it stays beside the types; the write half reads state the first three fill and nothing there calls back. The
hypothesis of four to six modules and a root under 500 lines held.

**Cost.** Eight methods a sibling calls became `pub(super)` (the compiler's E0624 named them); the child modules reach the
root's private types and fields as descendants, so no field was widened.

Alternatives not taken: three larger children (read, decide, write) leave `write.rs` at 700 lines; a `Carry` per concern
would split the state the walk threads through every step.

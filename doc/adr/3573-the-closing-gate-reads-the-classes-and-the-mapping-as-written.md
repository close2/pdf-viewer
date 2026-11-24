# 3573 — The closing gate reads the classes and the mapping as written

Session 2218, the silent-repair campaign's closing round. Status: **accepted**. Context: ADR 3477
(the gate's three conditions), ADR 3505 (the classes), ADR 3506 (`pdf_archive::repairs::MAPPING`),
the owner's instruction of 2026-11-23, step 3.

## Context

ADR 3477's closing gate fails when a class the three reader crates declare has no mapping, when a
mapping names no class, and when a mapping names a row `pdf_archive::table::requirements` does not
hold. It belongs in `tools/conformance/tests/`, where every round runs it. The conformance crate
depends on nothing but `thiserror`, and its manifest says why: a checker that has to build the
document stack before it can speak is a checker that can stop running.

## Decision

- **`tools/conformance/tests/repair_classes.rs` parses the sources** rather than linking the
  crates: the three `classes! { … }` blocks (`IDENT = "name", "clause", "instead";`), the
  `MAPPING` initialiser (`base`, `rows`, `no_row` calls; a row named by a `const` is resolved in
  the same file), and every `id: "…"` of `crates/pdf-archive/src/table/` before its tests.
- The parse checks its own population — each crate declares at least one class, every name is in
  its crate's family, every call in the table is one of the three — so a grammar that drifts fails
  the gate rather than reading as an empty set.
- Besides the three conditions it fails on a class mapped twice and a `no_row` with no reason.
- `the_gate_names_each_planted_defect` plants each condition into the tree's own population on
  every run (trap 13); the round also planted a class into `pdf_syntax::repair` and saw the gate
  name it.
- `pdf-archive`'s own unit test keeps the font half by type.

## Consequences

- A new class cannot be added without a decision in `MAPPING`.
- A class declared in another grammar (a second macro, a hand-written `static`) is invisible to
  the parse; the population check catches a crate whose declarations all moved, not one that
  moved.

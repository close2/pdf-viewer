# 2046 — The structure tree is written in one module per concern

Session 1605, slot 7 of batch ninety-five, a docs refactoring round (Sonnet 5.5); `doc/todo/74`
entry 5. Status: **accepted** and **built** in `crates/pdf-model/src/structure.rs` and `structure/`.
Context: the owner's words of 2026-10-19, "Refactor god classes,..." and "Do not fix any bugs.";
ADRs 2004, 2018 and 2032 (`Viewer`, `ViewState` and `Walk`, split the same way) and ADR 2005 (the
ranking). Clauses: none; no behaviour moves. ADR 2047 is unused.

## 1. What the census found

`Tree`, the reading of ISO 32000-2 §14.7's logical structure, was one `impl` of 49 methods spread
over 2 240 lines of an 8 430-line `structure.rs`, the largest source file in the tree outside the
test binaries; about 2 900 of the file's lines were its tests.

## 2. The decision

`structure.rs` stays the root. It keeps the state (`Tree`), the entry points that read `/K` (`of`,
`children`, `child`, `walk`, `identified_children`), `logical_range`, and every type and free function
the concerns share. Five modules each hold an `impl Tree` for one concern, and the tests are
`structure/tests.rs`:

| module | concern |
|---|---|
| `roles` | a type through the role map, its namespace, the foreign and MathML counts (§14.7.3, §14.8.6) |
| `attributes` | attribute objects, owners and inheritance, the artifact and print field entries (§14.7.6, §14.8.5) |
| `tables` | spans, header scope, a cell's headers, layout spacing, the writing mode, list continuation |
| `owners` | from an object, widget, appearance or identifier to its element (§14.7.5.4) |
| `logical` | §14.8.2.5's order, the text it gives, and the descent both use |

The entry named "roles and namespaces" as one concern and it is one module: both end in the same role
map walk. `logical_range` is in the root and not in `logical` on purpose: the same batch's clause
slot (ADR 2042) edits that body with `-U0` hunks, and a function moved under another slot's patch
would turn their hunk into a rejected one. It is the one method of the logical order that is not in
`logical`; moving it there is a single cut after both rounds are merged.

**Visibility is what a sibling reads.** A method is `pub(super)` only where the compiler named a
sibling, the root or the tests that call it (`descend`, `declares_the_mathml_namespace`,
`declares_a_namespace_outside_the_standard`); every other private method stayed private to its
module, and every `pub` method stays `pub`. `Tree`'s one field stays private to `structure` and its
children. The free functions and types stayed in the root (about 3 000 lines: the standard types, the
table grid, the artifact and language helpers); moving them is a different split.

## 3. The proof that nothing moved

The multiset of non-blank lines, whitespace stripped (habit 93), of the old file against the root,
the five modules and `tests.rs`. Removed-only: one line, `fn descend(`, which gained `pub(super)`.
Added-only: the module doc comments, each module's `use` block, five `impl Tree {` wrappers with
their closing braces, the five `mod` lines and the root's paragraph of module doc. `cargo test -p
pdf-model --lib` passes unchanged (657 tests) and clippy with `-D warnings` is clean over all
targets. Ledger rows naming `structure.rs` now also name the module that cites their clause
(42 rows), and the 91 `test` entries naming a moved test name `structure/tests.rs` (habit 96).

## 4. What this costs

The root is 3 600 lines still, nearly all types and free functions. A reader looking for
`Tree::mapped_role` opens `structure/roles.rs`; one looking for `StandardType` opens the root.

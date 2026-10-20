# 2112 — The archive rewriter is written in one module per concern

Session 1638, slot 8 of batch ninety-nine, a docs refactoring round (Sonnet 5.5); `doc/todo/74`
entry 4. Status: **accepted** and **built** in `crates/pdf-transform/src/archive/rewrite.rs` and
`rewrite/`. Context: ADRs 2004 to 2097. Clauses: none; no behaviour moves.

## 1. What the census found

`rewrite.rs` was 5 470 lines and held one `impl Rewriter` of 2 612 lines and 65 methods: the archive
conversion's restatements of values, appearance states, configurations, page structure, attachments,
annotations, the catalogue and form, and streams. The rest of the file is `Rewrite` (the enum and its
descriptions), the state, the walk and the free helpers, which stay.

## 2. The decision

`rewrite.rs` stays the root and keeps the routing: `rewrite`, `rewrite_array`, `rewrite_dictionary`,
`owed_rewrites`, `subtype_is*`, `wants*`. Eight modules, each an `impl Rewriter<'_>`:

| module | holds |
|---|---|
| `values` | `restate_*`, `agree_separations`, `write_colorants`, `write_default_cmyk`, profile and encoding writers, the `/CIDToGIDMap` and descriptor-set rewrites |
| `appearance_states` | the extra and automatic states and the reduced `/AP` dictionary |
| `configurations` | `name_*` and `complete_order` |
| `page_structure` | page resources and boundaries, `take_out_of_the_page`, `append_the_pages`, `preserve` |
| `attachments` | the file specification, its names, the name dictionary and the filing |
| `annotations` | the removals, the relocation and the in-place entries |
| `catalogue` | the role map, the output intents, `rewrite_catalog`, the form, `unsign` and permissions |
| `streams` | `rewrite_stream`, `rewrite_xobject`, `reencode`, `hexadecimal_repair` |

Visibility is what a sibling reads: private fields and root items are reachable from the modules as
descendants, so only methods needed across modules are `pub(super)` (40, each widened from the
compiler's E0624 and none narrowable). Every `super::` in a moved body became `super::super::`.

## 3. The proof that nothing moved

The multiset of the old file's non-blank lines against the root and eight modules: removed-only 65
lines, all signatures that gained `pub(super)`, `super::` paths re-qualified, and import lists;
added-only 136, the module docs, `use` and `mod` lines, eight `impl` wrappers and those same lines.
`pdf-transform` is not in the corpus arms' binaries' graph (`cargo tree -i pdf-transform`: the
virtual file systems and the hosts only), so the arms are not owed; `cargo test -p pdf-transform` is the
proof of behaviour.

## 4. What this costs

A reader looking for `rewrite_stream` opens `streams.rs`. Nineteen ledger rows whose `code` named
`rewrite.rs` now name the modules that cite their clause (habit 96), read by `§<clause>` in each file.
`rewrite.rs` is still 3 200 lines, 1 100 of them `Rewrite`'s descriptions.

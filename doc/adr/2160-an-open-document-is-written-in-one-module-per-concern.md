# 2160 — An open document is written in one module per concern

Session 1662, slot 8 of batch one hundred and two, a docs refactoring round (Sonnet 5.5);
`doc/todo/74` entry 11. Status: **accepted** and **built** in `crates/viewer-core/src/open.rs` and
`open/`. Context: ADRs 2004 to 2144. Clauses: none; no behaviour moves. (2161 unused.)

## 1. What the census found

`tools/state.sh gods` ranked `Open` (`crates/viewer-core/src/open.rs`) the fourth impl block by lines
once the entries before it were split: 1 356 lines and 44 methods in a 2 545-line file. Slot 1 owns
`pdf-archive` and `pdf-transform`'s `archive/`, so `Walk` in `survey.rs` was not taken; the other two
candidates the brief named were `Interpreter` in `content/text.rs` and this one. A document's pages,
its edit log, its zoom and its fragment identifier were one block read top to bottom.

## 2. The decision

`open.rs` stays the root and keeps what every concern reads: `Open` and its fields, the types the
concerns share (`Spot`, `Chosen`, `OnScreen`, `Interpreted`, `Done`, `Committed`, `Read`, `Pending`,
`OpeningFile`, `Highlighted`), the three constructors (`new`, `read`, `around`), `first_read` and
`interpret`, `Preparation` and `Prepared`, and the tests. Seven modules, each an `impl Open`:

| module | holds |
|---|---|
| `deferred` | `outline`, `page_indices`, `section_at`, `about`, `forget_about`, `preparation`, `prepared` |
| `interpretation` | `stale`, `reinterpret`, `on`, `on_mut`, `interpreted`, `interpretation`, `spans`, `placed_at` |
| `pages` | `page`, `position_at`, `page_size`, `shown_page`, `placed_page`, `recount` |
| `edits` | `resolve`, `replay`, `apply`, `dirty`, `saved`, `commit`, `commit_field`, `toggle_popup` |
| `magnification` | `magnification`, `stepped`, `origin`, `hold`; `raster_extent`, `content_box`, `fitted` |
| `fragments` | `apply_fragment`, `apply_parameter`, `highlight`, `import_from`, `go_to_named`, `go_to_structure_element`, `go_to`, `in_default_user_space`, `text` |
| `destinations` | `apply_view`, `scroll_to` |

`raster_extent` is re-exported from the root under the path `layout.rs` already uses
(`crate::open::raster_extent`). Visibility is what a sibling reads: `content_box` and `fitted` are
`pub(super)` (two modules and the root's tests read them); every method that was `pub(crate)` is as
it was and every private method is private to its module, since no sibling called one.

## 3. The proof that nothing moved

The multiset of the old file's lines (indentation removed) against the root and the seven modules:
removed-only 5 lines (the old import list, two `use` lines, the signatures of `content_box` and
`fitted`, which gained `pub(super)`), added-only 81 non-blank (module docs, `use` and `mod` lines,
six `impl Open {` and their closing braces, the widened signatures). `viewer-core` is not in the
corpus arms' graph (`cargo tree -p render-raster -e normal,dev,build` and `-p pdf-sandbox` print no
`viewer-core`), so the six arms are not owed; `cargo test -p viewer-core` is the proof of behaviour.

## 4. What this costs

A reader looking for `Open::commit` opens `open/edits.rs`. Eighteen ledger rows that named `open.rs`
name the module holding the row's item as well (those whose items are fields and types stay on the
root); `how-a-form-field-is-filled.md`, `how-text-is-selected-and-copied.md` and `doc/crate-map.md`
name the new files. About 1 150 lines of the root are the field documentation and the types, which
are a different split.

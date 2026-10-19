# 2018 — `ViewState` is written in one module per concern

Session 1591, slot 7 of batch ninety-three, a docs refactoring round (Sonnet 5.5); `doc/todo/74`
entry 2. Status: **accepted** and **built** in `crates/pdf-model/src/view.rs` and `view/`. Context:
the owner's words of 2026-10-19, "Refactor god classes,..." and "Do not fix any bugs."; ADR 2004
(`Viewer`, split the same way) and ADR 2005 (the ranking). No ledger row changes status; the `code`
paths of the rows that named `view.rs` move (habit 96). Clauses: none; no behaviour moves.

## 1. What the census found

After `Viewer`, the type with the most methods in the tree's second-largest block was `ViewState`:
a 3 034-line `impl` of 86 methods in `view.rs`, a further 88 in the `view/script_*.rs` files, and about
1 700 lines of free functions the first block called (the field-tree walk, the `Update` that `save`
builds, the widget and annotation hit tests). The file was 6 571 lines, of which about 500 were tests.

## 2. The decision

The root `view.rs` stays and holds what every concern shares: the state (`ViewState`), the types a
host names (`Added`, `Filing`, `Request`, `Entered`, `AnnotationView`, `Appearance`, ...), `Entry`
(which `ViewState`'s own field and `scripts.rs` both name), the constructor, the number allocator,
the module table, the re-exports and the tests. Ten modules each hold an `impl ViewState` for one
concern, and the free functions only that concern used:

| module | concern |
|---|---|
| `settings` | what a window states: magnification, audience, purpose, paper, separation simulation, modification time, widget appearances |
| `optional_content` | the configuration's state: `reapply_usage`, the snapshot and its restore, `set_group` |
| `annotations` | looking at an annotation: its appearance state, hidden state, view, and hit test |
| `markup` | annotations a person places or an FDF file supplies, free text, note text |
| `attachments` | the attach and detach log |
| `fields` | typing, choosing, reading, the caret and selection, clearing, importing |
| `field_tree` | §12.7.4.1's walk from a name to its widgets (`MAX_FIELD_DEPTH` bounds it) |
| `writing` | `save`, the seven `write_*` that feed it, and `Update` |
| `pages` | appended pages and the named-page files an import waits for |
| `actions` | `perform`, hide, reset |

`doc/todo/74` entry 2 named eight modules; two differ and the reasons are the code's. The walk from a
field's name to its widgets is 400 lines that the fields, the actions and the writer all call and
that touch no state, so it is `field_tree` and not part of `fields`. A reader looking at an
annotation (`annotation`, `appearance_for`, `annotation_at`) is neither placing one nor editing a
field, so it is `annotations`. `Entry` stays in the root because `ViewState` has a field of its type.

**Visibility is what a sibling reads.** An item private to the old module is `pub(super)` only where
the compiler named a sibling that calls it; an item only its own module uses stayed private. Every
`pub` method of `ViewState` stays `pub` (it is the type's interface); the free `pub fn`s are
re-exported from the root under their old paths, so nothing outside `view` changed a path.
`ViewState`'s fields stay private to `view` and its children, so no field was widened.

## 3. The proof that nothing moved

The multiset of non-blank lines, whitespace stripped (habit 93): the old file's against the root's
and the ten modules'. Removed-only: 35 lines, which are the old `use` block, six doc lines whose
intra-doc link is now qualified, and signatures that gained `pub(super)` or were re-wrapped by
`rustfmt`. Added-only: the module doc comments, each module's `use` block, nine `impl ViewState {`
wrappers, the re-exports, and `pub(super)` prefixes. In the three old script files, the paths
`super::chosen` and the like now name the module that holds the item. `pdf-model`'s library tests
pass unchanged and clippy with `-D warnings` is clean over all targets.

## 4. What this costs

A reader looking for `ViewState::set_field` finds it in `view/fields.rs`, and a method that calls a
sibling's reads `pub(super)` on the callee. A private helper is no longer private to the whole
`impl`; the module is the boundary now.

# 2004 — `Viewer` is written in one module per concern

Session 1584, slot 8 of batch ninety-two, a docs refactoring round; `doc/todo/74` entry 1. Status:
**accepted** and **built** in `crates/viewer-core/src/viewer.rs` and `viewer/`. Context: the owner's
words of 2026-10-19, "Refactor god classes,..." and "Do not fix any bugs."; ADRs 1910 and 1912
(`image/` and `renderer/` split the same way). Ranking: ADR 2005. No ledger row changes status;
thirty-seven rows' `code` paths move (habit 96). Clauses: none; no behaviour moves.

## 1. What the census found

`tools/god-census.py` ranks `impl` blocks by lines. The largest was `impl Viewer` in
`viewer-core/src/viewer.rs`: 4 636 lines and 95 methods, followed by a second block of 12 methods and
about 1 100 lines of free functions the first block called. The file ranked sixth by lines, so a
census by lines alone would have taken `structure.rs`; the block is the class, and the block was
larger than any other in the tree by 1 600 lines.

## 2. The decision

The root file `viewer.rs` stays, as `image.rs` and `render-cpu/src/lib.rs` did, and holds what
every concern shares: the state (`Viewer`, `Hold`, `Turn`), the identities a host holds, the bounds,
`act` (the one place a command is routed), `handle` and the tests. Fourteen modules each hold an
`impl Viewer` for one concern, and the free functions only that concern called: `opening`,
`layout` (the one place a render is scheduled), `navigation`, `pointer`, `outcomes`, `editing`,
`printing`, `search`, `geometry`, `accessibility`, `answers`, `media`, `scripts`, `anticipation`.

**Visibility is what a sibling reads.** A method private to its old module is `pub(super)` only
where the compiler named a sibling that calls it (seventy-five methods, because the
concerns call each other: `settle` is called from the root and from `anticipation`); a free function is private unless
another module uses it. `Viewer`'s fields stay private to `viewer` and its children, which is what a
child module may read of its parent, so no field was widened. What `lib.rs` re-exports and what
`open.rs`, `layout.rs` and `command.rs` import (`px`, `DocumentId`, `RenderToken`, the four
`anticipation` handles) are re-exported from the root under their old paths.

## 3. The proof that nothing moved

A mechanical move, proved by the multiset of its lines (habit 93): the lines of the old file less the
lines of the root and the fourteen modules, and the reverse. What differs is only the `use`
statements each module needs, `impl Viewer {` and `}` wrappers, `pub(super)` before an item a sibling
reads, signatures `rustfmt` re-wrapped for the extra width, and the root's module table. No other line
is added or removed. `viewer-core`'s 400 tests pass, `viewer-core` is the dependent of no crate the
six corpus arms build (`render-raster`, `render-cpu` and `pdf-render` do not depend on it), and its
dependents compile.

## 4. What this costs

A reader looking for `Viewer::pointer` finds it in `viewer/pointer.rs`, and a method that calls a
sibling's reads `pub(super)` on the callee. The price of the visibility is that a concern's private
helpers are no longer private to the whole `impl`; the module is the boundary now, and a helper only
its own module calls stayed private.

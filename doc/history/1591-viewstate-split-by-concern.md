# 1591 — `ViewState` split by concern

Session 1591, slot 7 of batch ninety-three, a docs refactoring round (Sonnet 5.5); 2026-10-19. ADR
2018 (the split); 2019 is unused. The owner's words: "Refactor god classes,..." and "Do not fix any
bugs." No bug was fixed; none was found in the code moved.

**Built.** `doc/todo/74` entry 2: `crates/pdf-model/src/view.rs` is the root (state, the types a host
names, `Entry`, the constructor and number allocator, the module table, re-exports, tests; 6 571 to
1 738 lines) plus ten modules under `view/`: `settings`, `optional_content`, `annotations`, `markup`,
`attachments`, `fields`, `field_tree`, `writing`, `pages`, `actions`. The `script_*.rs` files and
`scripts.rs` changed only in the paths that named the old flat module (13 lines added, 6 removed).

**The premise that did not hold.** The entry named eight modules; the field-tree walk (400 lines,
called by three concerns) and the reading of an annotation are two the entry did not name, and the
fields did not have to be cut last: they call the others through `pub(super)`.

**The proof that nothing moved.** The multiset of non-blank lines, whitespace stripped, old file
(6 349) against root and ten modules (6 466): 40 removed-only and 157 added-only. Removed: the old
`use` block, six doc lines whose intra-doc link is now qualified (a path that resolves from the new
module), and signatures that gained `pub(super)` or were re-wrapped. Added: 37 module doc lines, 38
`use` lines, nine `impl ViewState {` wrappers, the re-exports, 25 `pub(super)` prefixes. Corpus
arms: the round's export (HEAD plus these 14 paths) walked in all six arms against HEAD's arms
by page and frame digest: 968, 968, 968, 965, 965 and 965 pages, 0 differing, 0 present in one only.

**Ledger.** Forty-five rows' `code` paths re-pointed to the modules holding each row's item (habit
96); two notes naming `view.rs` for the field-tree walk now name `view/field_tree.rs`;
`doc/crate-map.md`'s `pdf-model` row names the directory; `doc/todo/74` entry 2 is built. No
`doc/teaching/` diagram names `view.rs`.

**Unfinished.** Entries 3 to 11 of `doc/todo/74`. `ViewState` is still 174 methods (the census counts the type, not a
block), now over 16 files, and no `impl ViewState` block is in the census's top five by lines.

**Gates.** `cargo fmt --check -p pdf-model`: 0. `RUSTFLAGS=-D warnings cargo clippy -p pdf-model
--all-targets`: no diagnostic in `view/` (the run stops on a sibling's `transparency.rs` doc line).
`cargo test -p pdf-model --lib`: 652 passed. `cargo test -p conformance --no-fail-fast`: every test
but `names` passed (`teaching`, `pointers`, `quotations` among them); `names` fails on slot 2's
`raster/crates/raster/src/lib.rs:15`, not on this round. `cargo doc -p pdf-model`: no new link
warning in `view/`. The six arms: 0 pages moved (above). `tools/state.sh --round 1591 docs` was still in its rustdoc step
over `render-cpu`, `raster-gpu` and `viewer-ui` (not this round's crates) when the round ended, so
its verdict is not claimed. Duration: about 5 400 s, most of it the arms' queue and walk.

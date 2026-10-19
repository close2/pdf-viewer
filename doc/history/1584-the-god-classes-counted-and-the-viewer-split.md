# 1584 — The god classes counted, and `Viewer` split by concern

Session 1584, slot 8 of batch ninety-two, a docs refactoring round; 2026-10-19. ADRs 2004 (the
split) and 2005 (the ranking rule). The owner's words: "Refactor god classes,..." and "Do not fix
any bugs." No bug was fixed; none was found in the code moved.

**Built.** `tools/god-census.py` (four tables: files, `impl` blocks, types by methods, functions;
`--top`, `--count`), `tools/state.sh gods` (a `quick` section), `doc/todo/74-the-god-classes.md`
(eleven entries, each with its concern split). The top entry by block, `impl Viewer` in
`viewer-core/src/viewer.rs` (95 methods in one block), is now `viewer.rs` as the root plus fourteen
modules under `viewer/`.

**The premise that did not hold.** The brief took the largest file, `structure.rs`, as the top
entry; by block `Viewer` is (4 636 lines against 3 034), and `Tree` is entry 5, with five concerns.

**The proof that nothing moved.** The multiset of lines of the old file (6 804) against the root and
the fourteen modules (7 128): 103 lines removed and 427 added, and after ignoring `pub(super)` and
whitespace the only removed-only lines are the old `use` block (replaced by each module's own), the
doc line of the file, and twelve signatures `rustfmt` re-wrapped. Added lines are the module doc
comments and imports, 13 `impl Viewer {`/`}` wrappers, 75 `pub(super)` prefixes on methods and 15
on free functions, and 15 `#[cfg(doc)]` imports that keep intra-doc links resolving. The six
arms are not owed a re-run by a dependency argument: `render-raster`, `render-cpu` and `pdf-render`
do not depend on `viewer-core` (`cargo tree -i viewer-core`: `pdf-transform`, `pdf-vfs`,
`viewer-*` only), so the arms' binary does not contain the code that moved.

**Ledger.** Thirty-seven `code` paths naming `viewer.rs` re-pointed to the modules holding each
row's item (habit 96); `doc/crate-map.md`'s `viewer-core` row names the directory.

**Unfinished.** Entries 2 to 11 of `doc/todo/74`; `viewer-ui` and `viewer-qt` were not built,
because `raster-gpu` did not compile at that moment under a sibling's edit.

**Gates.** `rustfmt --check --edition 2024` on the moved files: 0. `cargo clippy -p viewer-core
--all-targets`: no warning in `viewer-core` (the `-D warnings` run stopped in `pdf-archive`, a
sibling's mid-edit, exit 101; the same run on `viewer-core` alone before that was 0). `cargo
nextest run -p viewer-core`: 400 passed, 2 skipped. `cargo check` of `viewer-host`, `viewer-ffi`,
`viewer-confined`, `pdf-vfs`, `viewer-accessibility`, all targets: 0. `cargo test -p conformance
--no-fail-fast`: 0 (the first run failed `every_python_run_in_tools_writes_no_bytecode` on the new
section's `python3` line, fixed with the `env PYTHONDONTWRITEBYTECODE=1` the gate asks for).
`--bin quotations`: 0. `--bin pointers`: 0. `tools/state.sh --round 1584 docs`: 0. `tools/state.sh
gods`: 0. Duration: about 1 500 s.

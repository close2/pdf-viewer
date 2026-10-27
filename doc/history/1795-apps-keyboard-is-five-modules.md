# 1795 — `App`'s keyboard in `typing.rs` is continued in five modules

Session 1795, slot 8 of batch 119, the standing Sonnet 5.5 code-quality slot; ADR 2424; ADR 2425 unused; Q661 unused.
No bug fixed; no model change; no ledger row names `typing.rs` (none re-pointed).

**Choice.** `tools/state.sh gods` named the `App` impl block in `crates/viewer-ui/src/bin/quorra/typing.rs` first (1 039
lines of twenty-five methods, in a file of 1 354). `typing.rs` keeps the types, the caret arithmetic and its tests (315
lines); the children in `typing/` are `clipboard`, `aiming`, `choices`, `annotations` and `keys`, each with its `impl App`
continued. The hypothesis (four to six modules, root under 500) held.

**Proof.** Token multiset of the non-comment lines, visibility qualifiers and `use`/`mod`/`#[path]` lines removed, HEAD's
file against the root and its five children (`proof.py`, scratch): nothing lost; the additions are four `impl App {`
heads with their closers and one trailing comma rustfmt wrote. Visibility was widened by E0624 (eight methods to
`pub(super)`). Teaching documents `how-text-is-selected-and-copied` and `how-a-form-field-is-filled` re-pointed to
`typing/clipboard.rs` and `typing/aiming.rs`; `quorra.rs`'s module map and `doc/todo/74` section 21 name the split.

**Gates.** `cargo fmt --check -p viewer-ui` exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p viewer-ui --all-targets`
exit 0; `RUSTDOCFLAGS="-D warnings" cargo doc -p viewer-ui --no-deps --document-private-items --bins` exit 0; `cargo test
-p viewer-ui --no-fail-fast` exit 0 (the two caret tests ran in the binary). The move holds every token, so no pixel can
move: `cargo tree -i viewer-ui` exit 0 prints viewer-ui alone, so no arm binary depends on it and the six arms are not in play; `tools/drive-windows.sh --window quorra` on the export (HEAD plus this round's `quorra.rs` and `typing` paths, release build, own Xvfb display, stopped by the script) exit 0: 128 works, 0 wrong, 4 not offered, 0 to look at. `cargo test -p conformance --no-fail-fast` exit 101: two reds, one mine and fixed (`teaching`, a diagram node naming `typing/clipboard.rs` without its path; re-run exit 0, 3 passed) and one not mine (`round_numbers`, `crates/render-raster/examples/resident_triangles/main.rs:45`, a sibling round's untracked file).

**Duration.** 778 s for the split and its gates; the drive's export, build and run took 1004 s more. The export target was deleted.

**Found, not fixed.** `typing/keys.rs`'s `typed_into_value` is still about 160 lines of one `match`; `Pressed` and `Aim`
could move beside `aiming`. `quorra-confined/typing.rs` is a second `typing` module with a different shape.

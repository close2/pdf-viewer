# 1536 — The Qt host is a module tree by concern

Slot 3 of batch eighty-five, 2026-10-14, a host round of the owner's switch to code improvement.
ADR 1908; no ADR 1909, no question. No ledger row moved; §12.2's row names its two Qt tests at
`host/tests.rs`, where they now are.

**Premise.** Held: `host.rs` was 6 483 lines, and `frames()` (line 3028, not 3021) is the drawing
path. **Hypothesis.** Held. The concerns map one to one onto the GTK host's. Where they differ, it
is the event model: Qt's C++ pulls through getters, and GTK pushes into widgets it builds in Rust.
Thirty methods (430 lines) are identical in both hosts; ADR 1908 §3 names what a shared
`viewer-host` module would need, and it is not built.

**Built.** The root plus eighteen children (`opening`, `react`, `drawing`, `stand_in`,
`presentation`, `window`, `input`, `tabs`, `faces`, `panels`, `find`, `forms`, `notes`, `questions`,
`restrictions`, `requests`, `saving`, `printing`) and `tests`, generated from HEAD's file by
`scratchpad/r1536/py/gen.py` so every item is carried verbatim. 71 `pub(super)`, each checked by
removing it: 71 compiles broke, and the one that passed (a field) was taken back. `window_act` has
its documentation back; it had been left above `showing_what_would_print`. The bridge's four note
functions are re-exported at `crate::host`, so `bridge.rs` and `cpp/` are untouched.

**Proof that nothing moved.** HEAD's lines against the tree's, as multisets, differ only in imports,
`pub(super)`, rustfmt's re-wrapping, the test indent, two `super::notes::` paths and the module
documentation. The `quorra-qt` drive ran whole on Xvfb, HEAD's script with each export's binaries,
before (`1e814f2a…`) and after (`d03117ba…`): 90 works, 1 not offered (`27-processor-fallback`),
every verdict the same. 87 of 107 photographs are pixel-identical. The other 20 differ only in the
status line or tab label (output directory, loopback port, pid, loading percentage) or a 1 px caret's
blink. The workers' digests are the same in both exports.

**Files.** `crates/viewer-qt/src/host.rs`, `crates/viewer-qt/src/host/*.rs` (19 new);
`doc/conformance/ledger.toml` (§12.2's two test paths); `doc/crate-map.md` (`viewer-qt`'s row);
ADR 1908; this record.

**Gates.** `rustfmt --check --edition 2024` on the 20 files: exit 0. In the export of HEAD plus
`crates/viewer-qt`, own target directory: `RUSTFLAGS="-D warnings" cargo clippy -p viewer-qt
--all-targets` exit 0; `cargo nextest run -p viewer-qt` 35 passed; `cargo doc
--document-private-items` gives HEAD's three warnings and no new ones. Drives (`--long --tree 6`,
lane 2): before held 304 s, exit 0, peak 2.15 GiB; after held 315 s, exit 0, peak 1.42 GiB.
`cargo test -p conformance` in the worktree: 397 passed, 1 failed (`names`, a sibling's mid-split
`image::contradicted_frame`). `tools/batch.sh check` clean. `launch_path` not run: it cannot move,
since `cargo tree -i viewer-qt --workspace` names no dependant. Duration 2 900 s.

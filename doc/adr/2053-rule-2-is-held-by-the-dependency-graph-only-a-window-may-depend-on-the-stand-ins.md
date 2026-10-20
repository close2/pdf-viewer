# 2053 — Rule 2 is held by the dependency graph: only a window may depend on the stand-ins

Session 1608, slot 2 of batch ninety-six. Status: **accepted**; it restates ADR 0378's rule 2 and
`doc/todo/37`'s, and the purpose does not move. Context: ADR 2052, which put `stale` in a library.

## 1. The rule, and why the move reached it

ADR 0378's rule 2 is *nothing that judges a picture ever sees one*, and its enforcement was
**structural**: every stand-in sat in `quorra`'s binary, a binary is not a dependency, so the
corpus and oracle gates, `Query::Frame`, the confined worker, `render_at` and the headless harness
could not link one in principle; a test walked every `.rs` outside `viewer-ui/src/bin` for the word
`reprojection`. ADR 2049 planned S0 without reading it, and the move made that test fire on
`viewer-device`'s five files — which is the instrument working.

## 2. The decision

**The guarantee becomes the dependency graph, held by a test.**
`stale::tests::nothing_but_a_window_depends_on_the_library_that_holds_the_stand_ins` walks every
`Cargo.toml` in the repository (46 today, links not followed) and fails on any that names
`viewer-device` but the workspace's table and a window's; `WINDOWS` is `quorra`'s alone, and a
toolkit window joins it in the round that argues it (G1, Q1). The word test keeps walking, now
outside `viewer-ui/src/bin` and `viewer-device`. Both were planted and failed, then passed.

**What it costs**: "cannot, in principle" becomes "does not, by a test", and a window's own tests
can link the stand-ins. Neither weakens the purpose, because no judge is a window. **Why not keep
the stand-ins in the binary** and move the chooser alone: the render thread draws the retained pages
and places the stand-ins (`renderer/retained.rs`, `window.rs`), so that split is a rewrite of the
thread rather than a move, and every window that takes the device would write its stand-ins again,
which `doc/todo/30`'s rule is against.

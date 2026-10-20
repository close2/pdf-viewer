# 1608 — The device renderer leaves `quorra`'s binary

Session 1608, slot 2 of batch ninety-six, a "pixels" round, 2026-10-20: S0 (ADR 2049); ADRs 2052-2053.

**Built.** `crates/viewer-device/`: `renderer` and `stale` moved whole from `bin/quorra/`, `lane`
from `surface.rs`'s device half and `CoverageChoice`, `compose` from `viewer-ui`'s `software.rs`.
A window supplies a `Wake`, `FrameLines` and `FrameClock`; its surface is the `SurfaceTarget`
`QuorraWindowRenderer::new` already took, so no window type and no `unsafe`. `quorra` names
`viewer_device::{renderer, stale}` at its root; `viewer_ui::software` re-exports `compose`.

**The premise that did not hold.** Two winit seams were five ties: the stand-in vocabulary, the
coverage choice, the lane functions and the composition were the binary's too, and `renderer/`
names no `winit::window::Window`. And ADR 0378's rule 2 (stand-ins in a binary, so no judge links
one) was not in ADR 2049; its test fired on five moved files. ADR 2053 holds it by the graph: a test
walks every manifest (46) and fails on any naming `viewer-device` but the workspace's and
`viewer-ui`'s. Both rule-2 tests were planted, failed, and pass.

**The proof that nothing moved.** Multiset of lines (habit 93), HEAD's 35 files against 38:
23 497 to 23 785; 84 only before and 372 only after, each a `#[path]` removed, a seam's rewrite or
its reflow, a doc link a library cannot resolve named as text, or new docs, 29 `#[must_use]`, the
seams, `ComposeError` and the rule-2 tests; 137 `pub(crate)` widened, each one rustc named, and
`pub(super)` 50 to 50. `#[test]`s 163 to 164. `cargo tree -i viewer-device --workspace` is
`viewer-ui` alone: the arms' corpus binary, `raster_golden` and `turn_path` are outside (habit 119).

**Ledger and siblings.** §11.3.6's row to `compose.rs` (habit 96); slot 7 re-pointed the teaching
walk (habit 108). `-U0` lines: `render-raster`'s `corpus.rs`, `pipeline_price.rs`, `doc/todo/74`.

**Unfinished.** `viewer-device` under `RUSTDOCFLAGS=-D warnings`: 43 `private_intra_doc_links`
(module docs about private items); not on ADR 1938's list. G1 and Q1 are the next batch's.

**Gates.** `cargo fmt --check -p viewer-device -p viewer-ui`: 0. `RUSTFLAGS=-D warnings cargo
clippy -p viewer-device -p viewer-ui --all-targets`: 0. `cargo nextest run`: `viewer-device` 80
passed, 1 skipped; `viewer-ui` 131 passed, 1 skipped. `cargo test -p conformance`: all but
`records`, on slot 8's `1614`. `tools/state.sh --round 1608 docs`: rustdoc 0 after three links
fixed. Launch gate, one `--clock` hold, HEAD and this tree twice each, clocks on: this tree 0 and
0 (53 banded, 0 outside); HEAD 101 twice, `opt_demo.pdf`'s cold open 0.342 and 0.296 ms over
0.280, code this change does not reach; six first-frame digests identical. Drive of `quorra` in
each tree: 102 works, 0 wrong, 4 not offered, verdicts identical; 16 of 110 photographs differ,
and `script_goto` driven again gave this tree HEAD's first picture exactly and HEAD a third, so a
photograph is when it was taken, not which tree drew it. Duration: 9 400 s.

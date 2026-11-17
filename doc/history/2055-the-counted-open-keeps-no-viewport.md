# 2055 — The counted open keeps no viewport, because the program's open has none

Session 2055, a measure round alone in the audit worktree. No ledger row; ADR 2926.

**The slice.** The brief's three items: check the premise, give the counted child the window's size,
restate the moved bands.

**The premise.** It did not hold. The brief read ADR 2916 as "a window always has a size, so the gate
no longer counts work a reader's open still does". The program's open has no viewport either:
`quorra.rs`'s `open_document`, which the harness copies, and `viewer-host`'s `Launch::start` both
build `Viewer::new(0, 0, 1.0)`. The size arrives with the window's first `Resize`, after the join and
after `anticipate` has interpreted page one. That `Resize`'s `settle` raises Table 197's `/PV`
(§12.6.3) through `visibility_of`. So `/PV` left the program's open at `79d0c6c4`, not only the
gate's. It is in the first-page clocks, which give the viewer the gate's `VIEWPORT` the same way.

**The size.** `quorra` opens an 800 × 1000 logical window (`quorra/window.rs`,
`with_inner_size`), as does `quorra-confined`. `viewer-gtk` asks 1000 × 1100, and the confined
worker starts at 794 × 1123. No ADR argues the first two (ADR 2926 §5).

**The callgrind comparison.** I built the counted child with `Viewer::new(800, 1000, 1.0)` and
compared it with the same release build without a viewport, counting this program's own object.
On `bug1815476.pdf` the count went from 1744.3 to 66839.3 thousand: `raise` inclusive rose from
188.4 to 286.3, which is ADR 2916's 285.0, and `interpret_arranged` added 81922.9. On `opt_demo.pdf`
it went from 396.5 to 3247.6: `raise` rose from 343.8 to 386.5, against 2916's 386.7, and
`interpret_arranged` added 3365.5. The `settle` that ends `Command::Open` with a viewport interprets
page one, which the program does in `anticipate`, outside the open. I reversed the plant.

**Decision (ADR 2926).** The counted child keeps no viewport. No band moves. The two rows'
comments and `phase_count_open`'s doc comment say why. Counting the first frame would need a new
figure (open, `anticipate`, the first `Resize`) with its own bands, not this one redefined.

**Gates.** fmt and `rustfmt --check` 0. clippy `-D warnings --all-targets -p viewer-ui` 0.
`launch_path` judged run 1 exited 0, with every figure inside and none outside, at a load of 8.85.
Run 2 exited 0, again with none outside, at 19.89; its six counted figures matched
run 1's to 0.03 thousand. `cargo nextest run -p viewer-ui` under `xvfb-run -a` exited 0
(143 passed, 1 skipped). `cargo test -p conformance --no-fail-fast` exited 0 (474 passed) after
`git add -N`. A re-run with the record and the ADR in the tree gave 473 and 1: `bounded`'s self-test
failed, "one sample cost 1107 ms", at a load of 80. Run alone, `--test bounded` exited 0 (24 passed).

**Left.** A counted first-frame figure, if one is wanted. The window's default size is unargued.

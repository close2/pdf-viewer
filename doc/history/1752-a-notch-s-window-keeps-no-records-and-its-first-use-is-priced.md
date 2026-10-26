# 1752 — A notch's window keeps no records, and its first use is priced

Session 1752, 2026-10-26, a "measure" round of batch 115 (Slot 1). ADRs 2338, 2339; no Q filed.

**Premise.** ADR 2323's second remainder — the compute lane walking instead of replaying because a
`Cpu` page comes between two windows — is not in the code: the read-back thread draws every job
through `Rasterizer::rasterize`, whose `render_whole` discards the frame slot, so no window is ever
record-replayed. Its first remainder held: four pipelines compiled in the first `Compute` frame,
and the edges capacity's first guess (99 200) under the 246 217 counted, so the frame runs twice.

**The ceilings** (habit 164). A scratch example in an export of HEAD drew the read-back thread's
sequence; two `--clock` holds, three passes. The second hold's gated arms (loads 2.0 to 3.0), the
window's device job: windows 2 and 3 4.4–5.9 ms as shipped, 4.2–6.2 with no `Cpu` page between,
4.1–6.3 record-replayed (18 of 18 replayed), 2.6–4.9 with no idle between — records worth nothing,
the 400 ms idle 1.3 to 2.0 ms. The device has two speeds: emit and deposit 0.8–1.6 or 3.2–4.2 ms
in runs of seconds, every arm alike (ADR 2338). The first window: 8.6–13.9 shipped, 7.0–8.6 with
no second run, 5.1–6.3 with every first-use cost paid before — the second window's figure, and
the second notch settles past 8.33 ms on the owner's adapter (ADR 2323), so neither lever clears
the bar and neither is built (ADR 2339).

**Hypotheses.** "Records kept across a `Cpu` draw take the remainder to the replayed figure": not
held, there are none and keeping them moves nothing. "The bring-up moves off the notch": priced,
3 to 8 ms of notch 1, not built. "Notch 3 under 8.33 ms in three of three": not tested, nothing
built; the owner's loop was not run for the same reason, and every figure is the Radeon's headless.

**Changed.** Two doc comments only (32 comment lines, 0 other lines in the hunks): the read-back
module no longer says the window replays records, and `lane_for` names the first use's three parts.
`doc/todo/75` section 7 carries the two ADRs.

**Where.** I worked in scratchpad/r1752/; the scratch example lived only in its export and was
deleted with it.

**Gates.** `cargo fmt --check -p viewer-device` exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p
viewer-device --all-targets` exit 0; `cargo nextest run -p viewer-device` exit 101 in the worktree
on a sibling's `pdf-model` mid-edit, and in an export of HEAD with my two files 94 passed, exit 0;
rustdoc of `viewer-device` with warnings denied exit 0; `rustfmt --check` on both files exit 0;
`tools/state.sh --round 1752 docs` exit 101 on three unresolved links in `viewer-ui`'s
`quorra-confined`, a sibling's files. `cargo test -p conformance --no-fail-fast` exit 0, 69
binaries, none red. The six arms, raster's tests and the launch gates are not owed: 0 lines of
code changed. Both holds exit 0.

# 1644 — The toolkits' launch is banded, and Qt's wait is the loader's lock

Session 1644, slot 6 of batch one hundred, a "pixels" round, 2026-10-20: the toolkits' launch
banded, and Qt's thirty milliseconds profiled first (habit 59); ADRs 2124-2125.

**The profile** (ADR 2124). Per-thread `schedstat` sampled from outside, `strace -f` and callgrind,
HEAD's export, device lane against `--cpu`: Qt's window thread is blocked 10.6 ms against 2.5
before its first resize (medians of nine), on the dynamic loader's lock while the device's thread
loads `libvulkan_radeon` and `libLLVM`, and runs 2.8 ms longer beside it with the same work
(515 528 206 against 515 455 697 instructions to `resized`). After the window the difference is the
two visible pages: 10.2 and 4.4 ms on the device, 2.2 on the processor, of which the device's own
passes are 0.2 — fixed costs per page and a cold first use. GTK's window is not later: its own
start loads the same `libLLVM`.

**The premises that did not hold.** The adapter request and the surface are not on the GUI thread:
`ReadBack::bring_up` already has its own thread, and the device has no surface. The thirty
milliseconds did not reproduce: 4 to 9 ms by the gate's minima today. The read-back copy is 0.5 to
1.3 ms of a page, not most of it. **No lever is taken**: each priced in ADR 2124 section 4 costs the
page more than it saves the window.

**Built.** `read_back.rs` traces each device page's stages from `render_raster::FrameCost`; one
harness, `crates/viewer-host/tests/toolkit_launch/mod.rs`, included by each toolkit crate's
`tests/launch_path.rs`, holds seven figures a window to `doc/checks/toolkit-launch.toml`, banded by
ADR 1969's method over three runs in one hold (ADR 2125).

**Shared files** (`-U0`): `doc/todo/30`, `doc/todo/42`, `doc/state-of-play.md`, `Cargo.lock`.
**Unfinished**: a cold page cache; the gate is not in the merge's tier 2; the absolute Qt window
differs by 12 to 18 ms between the harness and the sampler, unexplained (ADR 2125 section 3).

**Gates.** In the export (HEAD and this round's files): `cargo fmt --check` and `RUSTFLAGS=-D
warnings cargo clippy --all-targets` over `viewer-device viewer-qt viewer-gtk`: 0, 0; `cargo
nextest run` the three: 139 passed, 3 skipped; the phases test planted (the push removed) failed;
`cargo doc -D warnings -p viewer-device`: 0. `cargo test -p conformance` in the worktree, with
this record and both ADRs: exit 0. The bands' hold: HEAD's and the tree's
three runs each inside every band, the gate on each against them exit 0; the final tree's, by the
band file's own line, exit 0. Drive of the export's windows, device lane then `--cpu`: GTK 94
works, 18 not offered, Qt 96 and 97 works, 15 not offered; 44 `wrong` in both lanes was the script
worker missing beside them and works with it, HEAD and tree alike; 83 is `wrong` in some runs of
HEAD and tree alike. Arms not owed: `cargo tree -i viewer-device` is the three windows (habit 119).
Duration: 7 700 s.

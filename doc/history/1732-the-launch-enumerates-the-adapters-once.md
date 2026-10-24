# 1732 — The launch enumerates the adapters once

Session 1732, 2026-10-23. Batch 112, slot 5 (measure). ADRs 2298, 2299; Q595.

**The change.** `raster_gpu::LaunchInstance` is the launch's instance and the adapters its check
enumerated; `Device::headless_on_launch` and `Device::for_surface_on_launch` choose among them as
`request_adapter` does under `HighPerformance` (surface support, then discrete, integrated, other,
virtual, processor, first of the best), and ask `wgpu` only where none can present. `render-raster`'s
instance calls return it and its constructors, `QuorraRasterizer::with_options` among them, build on
it; `quorra` and `quorra-confined` carry it on their instance thread. A test holds the restated order
to `request_adapter`'s on this machine; planted with the processor first, it failed (lavapipe
against RADV). The band header and `doc/state-of-play.md` say one enumeration.

**Headless**, against HEAD's export in one `--clock` hold at loads of 4.9 to 8.3, twelve gate
bring-up children each, interleaved: the adapter step 0.228 to 0.344 ms against 0.001, the same RADV
in all 24; the bring-up is instance creation's spread. Callgrind: 682.30 and 682.37 M instructions
against 680.66 and 680.65 M. The band is not re-taken (ADR 2299).

**On the owner's adapter**, one loop run in one `--clock` hold, staged binaries, no capture, loads
3.1 to 12.4: the gate's child's adapter step 6.29 to 14.05 ms against 0.001; `quorra`'s `device up
in` 18.7 to 23.1 against 11.4 to 13.8 ms, its first present 74.9 to 90.3 against 63.1 to 78.0 at a
falling load; `quorra-qt`'s adapter step 5.8 and 8.1 against 0.0.

**Hypotheses.** Held: one enumeration takes 6.2 to 11.1 ms off `quorra`'s device step pair by pair,
and 0.3 ms headless; the band does not move. Open: the launcher setting is Q595 — the remaining
asking is 8 to 15 ms of the check (12.1 to 21.0 against 3.7 to 5.8 under `NODEVICE_SELECT=1`), and
the variable also silences `DRI_PRIME` and `MESA_VK_DEVICE_SELECT`. Premise not held: the tree
installs no desktop entry (A270), so Q595 asks for the day it ships one.

**Gates.** `cargo fmt --check` on `raster-gpu`, `render-raster`, `viewer-ui` exit 0; warnings-denied
clippy `--all-targets` on the three exit 0; warnings-denied rustdoc on the three exit 0 (the
workspace's `tools/state.sh docs` walk not run). `cargo nextest run` on the three in the worktree:
1005 passed, 4 failed, all four `raster-gpu` tests that read slot 4's mid-edit of the lattice; in my
export `raster-gpu` 758 passed, exit 0. In the hold, HEAD then the tree: `t2-launch_path` exit 101
and 101, each on two sub-millisecond warm opens whose instruction counts match (habit 101);
`t2-toolkit_launch` exit 0 and 0. The six arms in my export, each exit 0: 5 799 pages by page
against HEAD's six arms, 0 digests differ. `cargo test -p conformance` exit 101 on `records` alone
(other sessions' records, mid-round); `--test evidence --test adr_status` pass.

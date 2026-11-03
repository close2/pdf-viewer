# 1916 — The shared card's resets are read from the kernel, and a hang stops every later device test

Session 1916, single round (pixels + instruments). ADR 2665, Q776, trap 317, a `git add -N` habit.

**Question.** Our tests reset the shared RX 580 five times on 2026-11-03: which code hangs it?

**Read (ADR 2665 §1).** `journalctl -k` names a process at each timeout. 09:38:57 `headless_quorra`
and 14:50:50 vello's `headless_gpu` came through OpenGL (radeonsi's `cs0` thread; a VM write fault
from `TC3` before the first). 14:02:33 is the clip test's 2× frame, the command processor stuck. 14:02:35
(`a_zoom_step_misses`) and 14:02:49 (`a_sub_pixel_rule_at_the_rasters_edge…`) each raise "Illegal
register access in command stream" in the first frame of the next process after a recovery — the card
failing after recovery, not those tests. Every reset was inside a parallel suite.

**Not found.** The clip hang's cause: every loop its shaders reach is bounded; on lavapipe the three
files' 21 tests pass under validation core, sync and GPU-AV with no error of ours (one wgpu staging-copy
VUID in two other tests). The RADV dump needs a run on the card the conduct rule forbids; Q776 asks.

**Built (ADR 2665 §2–3).** `test-support/backends.rs`: tests honour `WGPU_BACKEND` (raster's ADR 0017
leaves it to hosts); fourteen raster-gpu test files and `render-gpu`'s context use it; vello's holds
follow the pin. The hook logs each bring-up's adapter type and binary; `tools/gpu-holds.sh` counts them.
A hardware hold past 9 s leaves `<lock>.pdf-viewer-hung`; every later hardware section refuses until a
person removes it.

| lavapipe-pinned `nextest`, three crates | bring-ups `Cpu` | bring-ups `Other` (card via GL) | holds |
|---|---|---|---|
| before | 927 | 28 | 8 241 |
| after | 933 | 0 | 0 |

**RADV, alone, `--exact`, kernel read after each (no reset line):** `headless_gpu::renders_without…`
pass (RADV, Vulkan). Of round 1915's eleven, the eight outside the hanging files: seven pass
(`dotted_rule_end`, `lines_across_views`, `mitre_limit`, `a_resized_window_misses`,
`singular_transform`, `stencil_through_a_pattern`, `transfer_edge`) — reset collateral; `group_cost`
asks for lavapipe by name — the environment's. The three `sub_pixel_coverage` ones and the named
hangers were not run. Resets caused: none (`journalctl -k --since 15:15`).

**Gates.** Tier 1 fmt + `clippy -D warnings`, raster-gpu, render-gpu, render-raster: exit 0. Lavapipe
`nextest` of the three: 948/948, exit 0. `render-raster --test gpu_lock`: 4 passed, exit 0. `cargo test
-p conformance` after `git add -N`: exit 0.

**Unfinished.** The clip hang's cause (Q776); `Device::headless` still opens the card's node for EGL.

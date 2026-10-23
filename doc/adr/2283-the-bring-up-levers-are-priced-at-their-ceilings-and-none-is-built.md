# 2283 — The bring-up levers are priced at their ceilings, and none is built

Status: deferred — the one lever with a ceiling worth having, enumerating once, waits for a gate that
can see it; what reopens it is below. Session 1724, 2026-10-23.

## Context

ADR 2282 split the device's cold bring-up on the owner's session and on the agent's own adapter
access. Batch one hundred and eleven's brief named three levers to price before building (habit
164): RADV's pipeline cache, a cache file of this program's own, and the first frame drawn with its
pipelines ready and the rest compiled behind it. The split adds a fourth, the term it found
dominant on a display session. Every figure below is ADR 2282's, on the Radeon 890M through RADV, at
one-minute loads of 1.8 to 7.4.

## The four, at their ceilings

| lever | what it can remove | ceiling | where it would land |
|---|---|---|---|
| RADV's on-disk pipeline cache (`MESA_SHADER_CACHE`) | the first frame's compiles on a second launch | **already in force**: Mesa enables it by default, and disabling it took the gate's first page from 54.7 to 68.6 ms to 137.8 to 161.3, `quorra`'s first present from 71.3 to 79.1 to 144.3, and `quorra-qt`'s first frame from 122.7 to 144.1 to 202.1 | nothing to build |
| a cache file of this program's own | what RADV's cache leaves: `LineOver` 0.6 to 1.3 ms and the read-back copy 0.3 to 0.8 ms in the first frame | 0.9 to 2.0 ms, 1.3 to 3.7 % of the gate's first page | raster, through `wgpu`'s pipeline cache, whose constructor is `unsafe`; raster's ADR 0013 declined that exception, and its revisit trigger — "a measured startup regression attributing real latency to compilation" — is not met by 2 ms |
| `LineOver` and the read-back copy in the warm set | the same 0.9 to 2.0 ms, moved to the warm thread | the same, and less where the warm thread is still compiling when the frame asks: the store holds one lock across a compile, and the warm set took 7.1 to 13.9 ms | raster's warm set (its ADRs 0040, 0043) |
| one physical-device enumeration instead of two | the device-selection layer's second probe, `request_adapter`'s 5.5 to 13.0 ms where a display is named against 0.2 to 2.0 ms where none is | **0 ms on the gate**, whose children carry no display variables; **0 ms on the toolkit windows' first frame**, whose device was up before the window's first size in all fourteen launches; **5 to 9 ms of `quorra`'s 71.3 to 79.1 ms first present** on the owner's session | a raster constructor taking the adapter `create_launch_instance_timed`'s check found, and the host handing it over |

**The layer itself is not a lever this program holds.** `NODEVICE_SELECT=1` removes all of it, and
the layer reads it from the process's environment; setting it from inside the process is
`std::env::set_var`, which is `unsafe` in edition 2024, so it is a launcher's environment, as
`lavapipe`'s 3.5 ms is (`doc/questions/Q270`). `request_device`'s 4.7 to 8.1 ms on the owner's
carve-out is ADR 1872's `MemoryHints::MemoryUsage`, raster's to take with the frame gates beside it.

## Decision

- **Nothing is built in this round.** The first three have ceilings of 2 ms or less against first
  pages of 55 to 160 ms, and one is already the machine's.
- **Enumerating once is deferred, not declined.** Its ceiling is real on the one window whose
  bring-up is on its critical path, but no gate this tree has can see it, because the launch gate's
  children are headless by design (ADR 2282) and the toolkit gate runs on Xvfb, where the layer's
  cost is the display's. A change no gate can hold is a change whose regression no gate can catch.
- **What reopens it**: a gate that measures `quorra`'s first present on a session with a display
  named, or the device's bring-up arriving on a toolkit window's critical path — `toolkit_launch`
  printing `device_up_ms` past `device_window_ms` on the owner's adapter.

## Costs

`quorra`'s first present keeps the layer's second probe, 5 to 9 ms on the owner's Wayland session
and whatever an X server's setup reply costs on another. The decision rests on figures from one
adapter and one compositor.

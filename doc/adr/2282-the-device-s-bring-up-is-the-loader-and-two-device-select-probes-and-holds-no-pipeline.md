# 2282 — The device's bring-up is the loader and two device-select probes, and holds no pipeline

Status: accepted. Session 1724, 2026-10-23.

## Context

Principle 2 makes cold bring-up a gate of its own, and round 1716 read `quorra-qt`'s `graphics
device up in` at 135 to 227 ms on the owner's Wayland session with its split unknown (ADR 2266). The
launch gate's bring-up child reads 13.6 .. 20.6 ms on its band in a process with no display
variables (`doc/checks/launch-path.toml`). Batch one hundred and eleven's brief asked for the split —
adapter enumeration, device creation, the pipelines the first frame waits for, the swapchain's first
configure, the first present — headless first and then once on the owner's adapter, and gave three
hypotheses: pipeline compilation is over half of the owner's figure; a persisted pipeline cache halves
it on a second launch; the nested KWin's extra second is Mesa's device-selection layer and not the
owner's.

## What was measured

**The instrument is the gate's own.** The launch gate's test binary run as one child with
`PDFVIEWER_LAUNCH_PHASE=bring-up` (or `first-page`) prints the device thread's steps — render-node
wake, instance, adapter check, adapter, `request_device` — and the carve-out's placement. It was run
on the owner's session in three environments interleaved, five children each: the gate's (no
`DISPLAY`, no `WAYLAND_DISPLAY`), the session's own, and the session's with `NODEVICE_SELECT=1`. The
windows were read off their own `--trace=launch,frames`; `quorra-qt` and `quorra-gtk` now print the
same steps on their `graphics device up in` line (below). Two loop runs, each in one `--clock` hold,
staged binaries built by the agent user, every process ending itself under a timeout, no capture of
any kind. The one-minute load average is beside each figure.

| on the owner's session, Radeon 890M | environment | load | ms |
|---|---|---|---|
| the gate's child | no display variables | 2.8 to 3.8 | 26.1 to 30.7: instance 13.6 to 17.4, adapter check 4.0 to 5.0, adapter 0.2 to 0.35, `request_device` 5.9 to 6.2 |
| the gate's child | the session's | 2.8 to 3.8 | 42.4 to 53.8: instance 13.1 to 17.4, adapter check 12.4 to 22.3, adapter 5.7 to 13.0, `request_device` 4.7 to 8.1 |
| the gate's child | the session's, `NODEVICE_SELECT=1` | 2.6 to 3.5 | 25.6 to 30.7: adapter check 3.7 to 4.6, adapter 0.19 to 0.25 |
| `quorra-qt`, device up | the session's | 2.2 to 4.6 | 59.0 to 87.0, small page and ISO 32000-2's page 20 alike: instance 18.6 to 39.3, adapter check 11.0 to 51.1, adapter 5.5 to 9.3, `request_device` 4.7 to 6.9 |
| `quorra-qt`, device up | `NODEVICE_SELECT=1` | 2.5 to 5.8 | 45.8 to 53.3, adapter 0.2 to 0.3 |
| `quorra`, first present | the session's | 1.8 to 7.4 | 71.3 to 104.1; at the quiet end the device step is +15.9 to 20.3 (adapter 7.3 to 10.9, device 5.6 to 6.8) and the first configure +10.7 to 14.7 |
| `quorra`, first present | `NODEVICE_SELECT=1` | 3.6 to 6.8 | 60.7 to 60.9, adapter 1.5 to 2.0 |

Every child read the carve-out held — 436 560 of 524 288 KiB used and 356 KiB of the device's
asking placed in it — so its `request_device` is ADR 1872's system-memory block, and the gate would
decline these figures (ADR 1891).

## Decision

1. **The dominant term, named in both places.** Headless, in the gate's condition, it is instance
   creation — the Vulkan loader and both drivers' initialisation — at 13.6 to 17.4 ms of 26.1 to
   30.7. On the owner's session it is Mesa's `VK_LAYER_MESA_device_select`: it asks the display
   server for its device on every physical-device enumeration, and a launch enumerates twice (the
   adapter check, then `request_adapter`), which is 13 to 25 ms of a lone bring-up there and 10.7
   ms of `quorra`'s first present at the quiet end (71.5 against 60.8). In `quorra-qt` the loader
   is also slower beside Qt's own start (instance 18.6 to 39.3 against 13.1 to 17.4 alone).
2. **Round 1716's 135 to 227 ms was the machine's load**, 30 to 44 on 24 threads (trap 237): the
   same window on the same page read 60.4 to 66.8 ms at 2.7 to 3.1.
3. **The toolkit windows' bring-up is off their critical path at a quiet load**: the device was up
   before the window's first size in all fourteen launches, and in the interleaved pairs the first
   frame read 115 to 136 ms with the layer (loads 3.3 to 4.6) and 118 to 130 without it.
4. **No pipeline is in the figure.** raster returns the device before its warm set and the trace
   says `pipelines still compiling`. The first frame compiles two pipelines itself — `LineOver`,
   which the warm set does not hold, and the read-back copy — at 0.6 to 1.3 and 0.3 to 0.8 ms with
   RADV's on-disk cache, and 55.1 to 57.6 and 2.4 to 2.7 ms with `MESA_SHADER_CACHE_DISABLE=true`.
   On the owner's adapter no frame waited on the warm thread's lock.
5. **The toolkit windows' `graphics device up in` line carries the device thread's steps**, the
   gate's child's own five and the remainder, so that a window's figure is split where it is
   taken; `viewer-device`'s `Steps` is the code, and `toolkit_launch`'s reader still takes the
   duration before the first colon.

## The three hypotheses

The first did not hold: the figure contains no compilation at all. The second held in a form
already in force — RADV's cache took the first page from 137.8 to 161.3 ms to 54.7 to 68.6 on the
owner's adapter — and ADR 2283 prices what is left. The third held as far as this session can say:
the layer is 13 to 25 ms on the owner's own session, so the nested compositor's second is the nested
compositor's, which this round did not re-measure.

**On this agent's Xvfb the layer reads 145 to 220 ms more**, because each of its four X connections
waited 38 to 56 ms for an idle Xvfb's setup reply. The gate removes both display variables from its
children for exactly this reason, and the figure is the display's rather than the owner's.

# 1558 — What the device thread does before page one, and the one read that waited for warmth

Status: accepted. Amends raster's ADR 0018 and ADR 0042 in one respect: the warm-up's state is no
longer kept under the pipeline store's compile lock. Supersedes nothing.
Context: `CLAUDE.md` principle 2 ("[n]othing on the launch path waits for warmth"; "[t]he graphics
library must return a usable device before it is warm"); ADRs 1531, 1532, 1543, 1544, 1557;
`doc/todo/42` item 2.
Code: `crates/viewer-ui/tests/launch_path.rs` (`print_first_frame`, `frame_fields`);
`raster/crates/raster-gpu/src/pipeline.rs` (`PipelineStore::warm_up`, a field);
`raster/crates/raster-gpu/src/pipeline/warm.rs` (`warm_state`, the guard, `warm_up`,
`warm_duration`, `wait_until_warm`); `doc/checks/launch-path.toml`.
Tests: `raster-gpu`'s `pipeline::tests::the_warm_up_is_read_while_a_compile_holds_the_store`.

## 1. The span, divided

Round 1354's timeline put "graphics device up" and "first render requested" at one instant on all
five rows and "first frame drawn" 7.7 to 10.1 ms later. The gate now prints the first frame's own
stages beside the timeline, read off `render_raster::FrameCost` and raster's named spans after the
clock stops. One run, the real adapter, headless:

| row | device up → drawn | scene | device | encode | transfer | readback | named one-offs |
|---|---|---|---|---|---|---|---|
| `PDF20_AN001-BPC.pdf` | 7.7 | 0.1 | 6.0 | 2.8 | 0.4 | 0.9 | — |
| `Well-Tagged-PDF-WTPDF-1.0.pdf` | 10.1 | 0.2 | 6.8 | 2.9 | 0.5 | 1.0 | a pipeline on first use, 0.3 |
| `ISO_32000-2_sponsored_EC3.pdf` | 9.5 | 0.3 | 7.2 | 2.9 | 0.6 | 0.8 | the same, 0.3 |
| `bug1815476.pdf` | 10.0 | 0.3 | 9.2 | 3.8 | 2.6 | 0.7 | the same, 0.3 |
| `xfa_filled_imm1344e.pdf` | 7.7 | 0.1 | 5.4 | 1.6 | 0.2 | 0.6 | — |

The drawing passes are 0.0 to 0.1 ms by timestamp; 1.1 to 1.8 ms more is raster's "content beyond
pass", the submission's own wait. **Nothing in it is warmth**: `render-raster`'s `first_frame`
example on the real adapter draws its first frame in 8.48 to 8.65 ms with no settling and 8.39 to
9.68 after 100 ms of it, three fresh processes each. The readback is the headless stand-in for a
present and a window does not pay it. What `FrameCost` does not cover, 0.5 to 3.1 ms of the span,
is outside raster's call — the backend's own pass over the readback and the core's `RenderReady` —
and is the next thing to attribute.

## 2. The read that did wait

A window reads `Device::startup` at the moment it detaches its presenter, to print the bring-up.
`startup` read `warm_duration`, which took the store's lock — held by the warm-up thread for the
whole of each compile and retaken the moment it lets go. A scratch probe on the real adapter,
reading `startup` at once after construction, six fresh processes each arm: **3.2 to 4.3 ms,
answering the whole warm set's 4.3 to 5.4 ms, before; 0.000 ms, answering "still compiling",
after.** Under `Xvfb` on `llvmpipe` the same read was 22 ms of the 31.7 between "graphics device"
and "surface configured" in `quorra --trace=launch`, and the trace's line said the pipelines had
compiled in 22 ms while the next one says nothing waited for them.

So the warm-up's state has a lock of its own, which only the guard writes and every reader takes;
the condition variable waits on it. Nothing else changed: compilation stays under the store's lock,
so no pipeline is compiled twice.

## 3. What is left

**On `llvmpipe` the window still waits**: after the change the 31.7 ms is the ground's own present
(the presenter's first `get`), so the total did not move. Compiling the presenting lanes first was
tried and measured no change, and was not kept. Whether the wait is the store's lock or the driver's
is not separated here. On the real adapter a window cannot be measured from this account (`Xvfb`
has no DRI3, so RADV does not present there); a headless first frame on it shows no such wait, so
the startup read was the one this round could find, and the window figure is the owner's to take.

`raster-gpu`'s `startup.rs` holds no lever: what it does — the instance, the adapter — is before
"device up", and ADR 1532 already took GL off it. The bands moved are `bug1815476.pdf`'s, for ADR
1557's reasons; the others' first page is the device's and did not move.

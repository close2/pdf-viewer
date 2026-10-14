# 1898 — The device's scene is built while nobody waits, and an upload does not take it away

Session 1531, on the project owner's target of 2026-10-10 and 2026-10-14, "improve performance".
Status: **accepted** and **built** (ADR 1886 §4's first item); ADR 1877 §5's device half is
priced again in section 4 and not built. Context: ADRs 0351, 0702, 1289, 1831, 1877, 1886; traps
143, 148, 165. Code: `QuorraWindowRenderer::prepare` and `uploads`, `FrameSlot::scene_for` and
`prepare` (`render-raster/src/present.rs`); `Job::build_ahead`, `Costs::build_ahead`,
`Finished::Scene`, `prepare` (`quorra/renderer.rs`); the `built ahead:` line (`quorra/surface.rs`).
Tests: `render-raster/tests/scene_built_ahead.rs`; `a_scene_is_built_ahead_only_where_the_device_
would_draw_from_it`, `a_scene_built_ahead_after_a_processor_frame_is_kept_by_the_device_s_next_frame`.
Witness: `sample/demo_heavy_engineering_A1.pdf` of github.com/cyfung1031/BergPDF. Cites no clause
and moves no ledger row: the standard says nothing about when a viewer builds what it draws from.

## 1. The premise, checked, and what it missed

The premise held: the device's first frame of the witness after page one builds its scene
(headless 247–255 ms of a 507–522 ms frame at the fit, `zoom_frame`, this round's hold), and every
later notch keeps it. **What it missed is that the scene did not survive a processor frame.**
`render-raster` drew the processor's raster through the page's own `FrameSlot`, and a raster frame
never keeps a scene (`SceneKey::raster`, ADR 0351), so every processor frame, every look while a
page is read and every retained page drawn on the processor replaced whatever the device had built
of the page. `Costs::holds` already assumed the opposite ("a frame the processor drew instead leaves
it what it had"); a build ahead with that slot would have been thrown away by the first look.

## 2. Decision

1. **An upload goes through a lane of its own** (`QuorraWindowRenderer::uploads`): its own
   `FrameSlot` and caches, for the reason `Lane` gives for the chrome's. It costs the last upload's
   image held on the device until the next replaces it, a window's pixels once.
2. **`prepare` builds a frame's page scene and draws nothing.** It is the half of `FrameSlot::render`
   that is now `scene_for`, so the scene it leaves is the one that frame would have built, and a
   later frame keeps it by ADR 1877's own test (`Retained::draws`, the dots asked again).
3. **The window asks for the build with a frame the processor draws** (`Job::build_ahead`): where the
   device does not hold these pages' scene, no refusal is known for them, and a kept frame of them
   would be the device's by `Costs::forecast`'s own rule. The figure sent is what a built frame costs
   over a kept one (the seeds: (187 − 73) ns × 4 408 405 commands = 502.6 ms).
4. **The render thread builds after that frame, once the view has been still that long** — ADR
   1289's rent-or-buy bound, the build as the pass. A job arriving in the wait is drawn instead. Only
   after a frame the processor drew and the device was not asked (a refusal is its answer), before the
   retained pages (trap 148: the idle work follows where the frame went, and this is the device's
   work because the next frame is). `Finished::Scene` tells the window, which then prices the next
   frame as kept; the waker makes that true before the next key, and the trace says `built ahead:`.

## 3. Measured

One clock hold for the headless and Xvfb figures, HEAD `3fc3e7f1`'s export against HEAD plus this
round's hunks, each in its own target directory, alternating. `zoom_frame` (`Cpu`, RADV, the fit at
0.594 then 1.25×, 1.5×, 1×; three runs an arm) does not pass through `quorra`'s idle and did not move:
first fit 507.3–514.5 against 503.3–522.1 ms, the 1.25× step 287.3–322.6 against 292.4–320.3 ms.

`quorra` under Xvfb (llvmpipe, which refuses the witness's frame on its budget, 282 441 152 bytes over
134 217 728), ten `plus` notches 300 ms apart two seconds after the first present, three runs an arm:

| | HEAD | this |
|---|---|---|
| built ahead, after page one | — | 258.3–262.9 ms (248.8–253.2 the scene) |
| notch 1, the device's refusal | 341.2–374.3 ms, the processor not started | **31.4–35.8 ms** |
| notches with a frame of their own, of ten | 4, 4, 4 | **6, 6, 6** |
| last notch to its frame | 0.448–0.454 s | **0.129–0.143 s** |

Xvfb's adapter refuses the frame, so the table says what the build ahead takes out of an ask the
device answers with a refusal, and that more of the notches got the processor's frame for it.

**The owner's display was not measured**, and the frame the build is for — the device's first notch,
0.75–0.84 s in round 1525 — is therefore priced, not measured: kept at the seeds, 0.32 s. Five
attempts through the owner's loop between 09:37 and 10:58 drew no notch. The first could not write
its directory, which this user had made with its own group rather than the owner's. Three more drew
the window, but `xdotool` found no visible window of the run by its pid, its title or its class.
The fifth named a Wayland socket that does not exist, and winit then refused to start. The window
did build ahead on the Radeon 890M once, at 272.7 ms, 263.2 ms of it the scene.

## 4. What is left, priced

**ADR 1877 §5's resident instances.** The 1.25× kept step with the encode instrumented (shares, not
absolutes): geometry 38.7 ms, staging 7.7, recording 100.3, transfer 29.2, of a 303.6 ms step. A
line's instance in page space with the view a uniform would keep the instance stream resident and
save the staging and transfer of its 60 bytes, at most 37 ms here. It would not make the step the
replay: the recording walk — cull, clip chain, mask, queue, batch — is per view while the op list
interleaves lines with marks whose coverage is drawn per view and drops what the view culls, and a
zoomed view culls most of the page (ADR 1877 §3: 3.92–4.16 million of 4.41 million). Keeping the
batches would mean drawing every line at every view and culling on the device. Not built this round.

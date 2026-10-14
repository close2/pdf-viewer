# 1531 — The device's scene is built while nobody waits, and an upload does not take it away

Slot 4 of batch eighty-four, 2026-10-14, a pixels round under the owner's performance target; the
partial-row word yields to it. ADR 1898; ADR 1899 and Q426 unused. No ledger row moves. The witness
is the original file from github.com/cyfung1031/BergPDF (`sample/`), 39 533 504 bytes, re-fetched
after the reboot; every figure here was taken on it.

**Premise, held, and what it missed.** The device's first frame after page one builds its scene
(headless 247–255 ms of the fit's 507–522). But every processor frame, look and retained page was
uploaded through the page's own `FrameSlot`, and a raster frame keeps no scene, so a build ahead
would have been thrown away by the next upload; `Costs::holds` assumed it was not.
**Hypothesis (b)**: the first notch keeps the scene (held on Xvfb), its time unmeasured; **(a), not
built**: the walk's batches are per view, so resident instances save at most 37 ms of 304 (§4).

**Built, ADR 1898.** Uploads take a lane of their own. `QuorraWindowRenderer::prepare` builds a
frame's page scene and draws nothing (`FrameSlot::scene_for`, the half `render` shares). `quorra`
sends `Job::build_ahead` with a frame the processor draws, where the device holds no scene of the
pages and a kept frame would be its; the render thread waits that long still (ADR 1289), builds,
and sends `Finished::Scene`, so the next frame is priced kept; the trace says `built ahead:`.

**The figure moved.** Xvfb (llvmpipe, which refuses the frame on budget), three runs an arm: notch
one's device refusal 341.2–374.3 → 31.4–35.8 ms; notches with a frame of their own 4, 4, 4 → 6, 6, 6;
last notch to its frame 0.448–0.454 → 0.129–0.143 s. `zoom_frame` unmoved (it is not `quorra`'s
idle): first fit 507.3–514.5 → 503.3–522.1 ms, 1.25× step 287.3–322.6 → 292.4–320.3 ms. **Not
measured: the owner's display.** Five tries through the loop drew no notch. The directory was made
with this user's group. Then no visible window was found by pid, title or class. Then a dead
Wayland socket stopped winit. The device's first notch (0.75–0.84 s, round 1525) is priced kept at
0.32 s, not measured. The window built ahead on the 890M once: 272.7 ms, 263.2 the scene.

**Files.** `crates/render-raster/{src/present.rs,tests/scene_built_ahead.rs}`, `crates/viewer-ui/src/
bin/quorra/{renderer,surface}.rs`, `doc/state-of-play.md`, `doc/todo/69`, ADR 1898, this record.

**Gates.** `rustfmt --check` on the four Rust files: 0. Clippy `--no-deps -D warnings` on
`render-raster` and `viewer-ui`: 0 (with dependencies: a neighbour's `pdf-model` mid-edit).
`cargo nextest run -p render-raster -p viewer-ui`: 0, 291 passed, 5 skipped; the upload test fails
with the lane planted back. `raster-gpu` `line_lane`, `function_conformance`, `encode_threads`: 0,
22 passed (untouched). `turn_path` (second hold): 0, 33 judged, 0 outside. Six arms: none owed, no
display list moves and a kept scene's bytes are ADR 1877's held claim. `cargo test -p
conformance`: 0, 436 passed. Holds: one for the figures (1 050 s, 900 s of it a wait on the owner's loop that
had already failed) and four for turn_path and the owner's display. Duration 10 405 s.

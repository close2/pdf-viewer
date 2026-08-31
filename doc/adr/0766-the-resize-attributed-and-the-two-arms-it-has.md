# 0766 — The resize attributed, and the two arms it turned out to have

Status: accepted — an attribution and a decision **not** to build, with the prices written down.
Context: `doc/todo/47-the-resize-frames.md`, parked by the project owner as point 8 of the
seven-point GPU round and never attributed; ADR 0704 (which recorded 9–19 ms per step in passing
while fixing something else), ADR 0699's settled-view sharp pass and ADR 0761's budget on it.
Code read, none changed: `crates/viewer-ui/src/bin/pdf-viewer/window.rs` (`WindowEvent::Resized`),
`crates/viewer-core/src/viewer.rs` (`Command::Resize`, `settle`),
`crates/viewer-ui/src/bin/pdf-viewer/renderer.rs` (the render thread and `sharp_pass_affordable`).

The todo file's own rule was the brief: *do not build before attributing*.

## What was driven, and on what

`doc/environment.md`'s `Xvfb` recipe, with `xdotool windowsize` for the drag — the owner's
measurement loop has not ticked since 2026-08-29 and a window needs somebody's session. The
scripts are `tmp/839/`; the commands are in the todo file so they survive that directory.

**The device is llvmpipe**, because a swapchain on `Xvfb` has no DRI3 and the viewer says so on its
first line. So `present` and `device` below are this software stack's. What is *not* the software
stack's is everything the attribution turns on: which events a resize produces, what the core
spends, what the host and the scene build spend, and which arm a document is in. The one number
that wanted the real adapter was taken on it, headless (§3).

## 1. The core is microseconds, and interpretation is not on this path at all

`resize WxH at S -> N event(s) in T` is the whole of `Viewer::handle` for a resize. Over four
39-step drags and three 12-step sequences on three documents, **T is 6–12 µs**, every time. The
command sets the viewport and the scale and pushes a damage; `settle` then re-places Table 29's
arrangement and asks for whatever raster is now wrong.

§12.5.3's re-interpretation *is* reachable from here — `settle` derives the magnification from the
viewport, so a view mode whose magnification the window decides changes it on every step, and a
page carrying a `NoZoom` annotation would then re-interpret exactly as `doc/todo/46`'s wheel tick
does. Driven with `0` (fit page) pressed first, on the ISO specification — which has such
annotations — the line stays at 3–12 µs, because the pages on the screen are not the ones that
carry them. **So the mechanism is on the resize path and the cost belongs to `doc/todo/46`**, whose
file now says the gesture is either of two; nothing here is owed for it.

## 2. Which arm the document is in decides everything

Two arms, and they are not a matter of degree:

- **The page's raster does not follow the window.** The magnification is a number the reader
  chose, the page fits, and a resize emits **`damage` and nothing else** — no `NeedsRender` at all.
  The step is a re-composition: the retained page pictures are placed again under a chrome scene
  rebuilt at the new extent. Per step on the ISO specification with its outline panel open:
  host 0.2 ms, scene 0.2 ms, `present` 4.0–4.4 ms, one frame presented per step.
- **The page's raster follows the window.** A fit mode, or a page wide enough that the opening
  view is one — `tmp/Entwurf.pdf` is both. Every step emits a `NeedsRender` per page on the screen
  and the whole cost is that render. In a 39-step drag of 1.26 s the window presented **one** real
  frame and stood in for the other 89 ticks, at 2.9 ms of `present` apiece.

The retained-page stand-in is what keeps the second arm from freezing, and it works: no step of
any drag measured here showed the person nothing.

## 3. On the real adapter, the second arm is `encode` and nothing else

`render-quorra/examples/zoom_frame` draws one display list at a sequence of magnifications against
one warm device, which is a resize step of the second arm exactly — the same commands, a target a
few per cent larger. On **AMD Radeon 890M (RADV STRIX1)**, headless, minima of three rounds:

| page | step | total | scene | encode | transfer | execute |
|---|---|---|---|---|---|---|
| ISO 32000-2 p1, 548 cmd | 596×842 → 610×863 | **2.0 ms** | 0.0 | 1.3 | 0.2 | 0.1 |
| `Entwurf.pdf` p1, 58 010 cmd | 1667×474 → 1707×485 | **132.5 ms** | 0.0 | 129.0 | 0.7 | 0.5 |

`scene` and `handover` are zero after the first frame — the display-list walk and the outline
upload are cache hits, which is `doc/todo/47`'s own prediction that a page-space scene would make
the scene free and leave the encode as the term. It does, and the term is quorra's `encode`: the
same one `doc/todo/47-the-encode-term.md` and `doc/todo/46-the-kernel-floor.md` already own.

## 4. Three candidates the file named, and what each is worth

- **The surface reconfigure** — the todo file's first guess. A resize changes the surface extent
  and quorra's presenter reconfigures at the next acquire, which is inside `Stages::present`. The
  within-run control is the run's own frames split by whether a resize preceded them: `present`
  medians **4.14 ms after a resize against 3.89 ms not** on one document, and **7.58 against
  11.60** — the wrong way round — on another. It is below the spread of `present` itself and is
  not the term.
- **The chrome rebuild** — the second guess. `host` (the geometry, selection, focus, caret, popup
  and panel queries) is 0.2–0.4 ms and `scene` 0.2–0.3 ms per step with the sidebar open. Not the
  term either.
- **The re-render at the new extent** — the third, and it is the whole of arm 2 and none of arm 1.

## 5. Two things found that the file did not predict

- **ADR 0699's sharp pass runs once per drag step, and on this adapter costs the drag nothing.**
  A size change invalidates the sharp picture, so the render thread redraws the settled view at 2×
  after every step: 39 passes in a 39-step drag, 3.9–4.1 ms each. The A/B is `--supersample 2`
  against `--supersample 1`, alternating in one sitting on a quiet machine: presented-frame totals
  4.90 / 5.00 ms against 5.15 / 4.60, `present` 4.22 / 4.14 against 4.60 / 4.01. Indistinguishable.
  **And on the arm where it would hurt it never starts**: `Entwurf`'s drag ran **zero** passes,
  because `sharp_pass_affordable` predicts 4 × 210 ms against ADR 0761's 400 ms budget and
  declines. That budget was set from two machines' zoom frames and is here observed doing its job
  on a gesture it was not measured on.
- **§14.7's tree is republished on every step.** `App::attend` compares
  `viewer_accessibility::Showing::of(&self.viewer, width, height)`, and the viewport is in it, so a
  drag republishes the accessibility tree per step: **0.8–1.3 ms** beside each frame. On llvmpipe
  that is a fifth of the step; on an adapter whose `present` is a tenth of this one's it would be a
  larger share than anything else the event thread does.

## The decision

**Nothing is built.** Arm 2's cost is quorra's `encode` under an item that already owns it, and
buying it here would mean either coalescing a gesture's renders — which shows the person a raster
of the wrong size and is what the retained-page stand-in already does correctly — or a
partial re-encode, which is `doc/todo/47-the-encode-term.md`'s device-resident records. Arm 1
costs about a refresh on a software rasteriser and there is no reason to think it costs more on
the owner's.

The one term that is neither and is priced above is the accessibility republication, and it is
left open on purpose: debouncing it to gesture-settle is a decision about what a screen reader is
owed during a drag, not a performance patch, and it wants the owner's view. `doc/todo/47` carries
it as the one shape worth taking.

## Held by

Nothing new — no code changed. What holds the reading is that the commands are in
`doc/todo/47-the-resize-frames.md`, so the next round re-derives rather than quotes it (this
project's rule about a price, `doc/habits.md`'s *Measuring*).

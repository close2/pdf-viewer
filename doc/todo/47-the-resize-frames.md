# The resize frames: attributed, and the two arms a resize turns out to have

Status: **open — attributed, and nothing is owed on the arm that was suspected.** ADR 0766 is the
reading; what is left is one shape worth a decision and one term that belongs to another item.
Priority: 47 — performance, measured, and now priced.
Corpus: `doc/ISO_32000-2_sponsored_EC3.pdf` (a page whose raster does not follow the window) and
`tmp/Entwurf.pdf` (one whose raster does); `doc/PDF20_AN001-BPC.pdf` as a third.
Code: `crates/viewer-ui/src/bin/pdf-viewer/window.rs` (`WindowEvent::Resized`),
`crates/viewer-core/src/viewer.rs` (`Command::Resize` and `settle`),
`crates/viewer-ui/src/bin/pdf-viewer/renderer.rs` (the render thread, `sharp_pass_affordable`),
`crates/viewer-ui/src/bin/pdf-viewer/stale.rs` (`Refusal::Resized` and the retained-page answer).

## The attribution, and the command that reproduces it

`doc/environment.md`'s `Xvfb` recipe with `xdotool windowsize` for the drag. **The device is
llvmpipe** — a swapchain on `Xvfb` has no DRI3 and the viewer's first line says which adapter it
got — so `present` and `device` below are that software stack's. Everything the attribution turns
on is not: which events a resize produces, what the core spends, what the host queries and the
scene build spend, and which arm a document is in.

```sh
Xvfb :78 -screen 0 1600x1200x24 &
DISPLAY=:78 target/pdf-viewer --trace=frames,events,window,panel <document> > trace.txt &
sleep 25                                     # a large document takes its time to open
id=$(DISPLAY=:78 xdotool search --name . | tail -1)
for step in $(seq 0 39); do                  # one drag: forty steps, no settle between them
    DISPLAY=:78 xdotool windowsize "$id" $(( 800 + (step % 20) * 8 )) 1000
    sleep 0.03
done
```

Then read the trace's own three lines per step: `resize WxH at S -> N event(s) in T` is the core,
`frame … | present … | host … scene … device …` is each frame, and `sharpened: the settled view at
2x, T ms on the render thread` is ADR 0699's pass. Add `xdotool key 0` before the loop to put the
view into a mode whose magnification the *window* decides, which is the second arm below.

### 1. Interpretation and the core: microseconds, on every document

`resize … -> N event(s) in **6–12 µs**`, over four 39-step drags and three 12-step sequences on
three documents. `Command::Resize` writes the viewport and the scale and pushes a damage; `settle`
re-places Table 29's arrangement and asks for whatever raster is now wrong. **None of the 9–19 ms
is interpretation and none of it is core re-layout.**

§12.5.3's re-interpretation *is* reachable from here — `settle` derives the magnification from the
viewport, so a fit mode changes it on every step and a page with a `NoZoom` annotation
re-interprets exactly as a wheel tick does. Driven with `0` pressed first on the ISO
specification, which carries such annotations, the line stays at 3–12 µs because the pages on the
screen are not the ones that carry them. That cost is [`46-a-wheel-tick-that-interprets.md`](46-a-wheel-tick-that-interprets.md)'s
and that file now names the resize as its second gesture.

### 2. The two arms, and which one a document is in decides everything

| | arm 1 — the raster does not follow the window | arm 2 — it does |
|---|---|---|
| when | the magnification is a number the reader chose and the page fits | a fit mode, or an opening view that is one (`Entwurf.pdf`) |
| events per step | `damage`, and **nothing else** | `damage` + one `NeedsRender` per page shown |
| what the step is | a re-composition: retained pictures re-placed under a chrome scene rebuilt at the new extent | a whole re-render of the page |
| per step (ISO spec / `Entwurf`) | host 0.2, scene 0.2, `present` 4.0–4.4 ms, one frame presented per step | 38 renders asked in 39 steps; **one** real frame presented in a 1.26 s drag and 89 stand-ins at 2.9 ms of `present` |

The retained-page stand-in is what keeps arm 2 from freezing and it works: no step of any drag
measured showed the person nothing.

**The 1.3 s `resize` line in the owner's first Windows trace is not this item and never was**:
`tmp/win/entwurf.2.trace.txt`'s `resize 1200x1500 … in 1.3014148s` at t=1.862 is the same instant
and the same duration as that launch table's `interpreted, 58009 cmd … (+1302.964)` — the first
resize is where page one is interpreted, which is the launch path's known largest step
(`doc/todo/44` §6), not a per-step drag cost (ADR 0761 §1). §1 above is the general form of that:
a resize *can* interpret, and when it does the line says so in milliseconds rather than
microseconds.

### 3. Arm 2 on the real adapter — corrected by ADR 0767: the table below is the lane the window leaves

`render-quorra/examples/zoom_frame` draws one display list at a sequence of magnifications against
one warm device — the same commands at a target a few per cent larger, which is an arm-2 resize
step exactly. On **AMD Radeon 890M (RADV STRIX1)**, headless, minima of three rounds:

```sh
ZOOM_FRAME_ROUNDS=3 ZOOM_FRAME_SEQUENCE=1,1.024,1.048,1.072 \
  cargo run --release -p render-quorra --example zoom_frame -- <document> 1 1.0
```

| page | step | total | scene | encode | transfer | execute |
|---|---|---|---|---|---|---|
| ISO 32000-2 p1, 548 cmd | 596×842 → 610×863 | **2.0 ms** | 0.0 | 1.3 | 0.2 | 0.1 |
| `Entwurf.pdf` p1, 58 010 cmd | 1667×474 → 1707×485 | **132.5 ms** | 0.0 | 129.0 | 0.7 | 0.5 |

`scene` and `handover` are zero after the first frame, which is this file's own prediction coming
true: a page-space scene makes the scene free. **But the Entwurf row is `Coverage::Cpu`, the
example's then-fixed lane — and `surface::lane_for` takes `Coverage::Compute` for a moved view on
a real adapter, which an arm-2 step is on every step.** Re-measured on both lanes in one sitting
(ADR 0767): the step the window actually pays is **63–66 ms**, of which the GPU kernels are 44–46
and host `encode` 9.4–10.1 — so arm 2 is the *kernels*, [`46-the-kernel-floor.md`](46-the-kernel-floor.md)'s
item, and the encode term is [`47-the-encode-term.md`](47-the-encode-term.md)'s ~10 ms, not this
table's 129. `ZOOM_FRAME_COVERAGE=compute` is the knob that measures the shipped gesture.

### 4. The three candidates this file named

- **The surface reconfigure** — *not the term.* A resize changes the surface extent and quorra's
  presenter reconfigures at the next acquire, inside `Stages::present`. The control is the run's
  own frames split by whether a resize preceded them: `present` medians **4.14 ms after a resize
  against 3.89 ms not** on one document and **7.58 against 11.60** — the wrong way round — on
  another. Below the spread of `present` itself.
- **The chrome rebuild** — *not the term.* `host` is 0.2–0.4 ms and `scene` 0.2–0.3 ms per step
  with the sidebar open.
- **The re-render at the new extent** — the whole of arm 2 and none of arm 1.

### 5. Two things this file did not predict

- **ADR 0699's sharp pass runs once per drag step and costs the drag nothing here.** A size change
  invalidates the sharp picture, so the render thread redraws the settled view at 2× after every
  step — 39 passes in a 39-step drag, 3.9–4.1 ms each. A/B, alternating in one sitting on a quiet
  machine, `--supersample 2` against `--supersample 1`: presented-frame totals 4.90 / 5.00 ms
  against 5.15 / 4.60, `present` 4.22 / 4.14 against 4.60 / 4.01. Indistinguishable. **And on the
  arm where it would hurt it never starts** — `Entwurf`'s drag ran zero passes, because
  `sharp_pass_affordable` predicts 4 × 210 ms against ADR 0761's 400 ms budget and declines. That
  budget was chosen from two machines' *zoom* frames and is here watched working on a gesture it
  was never measured on.
- **§14.7's tree is republished on every step.** `App::attend` compares
  `viewer_accessibility::Showing::of(&self.viewer, width, height)` and the viewport is in it, so a
  drag republishes the accessibility tree per step: **0.8–1.3 ms** beside each frame. On llvmpipe
  that is a fifth of the step; on an adapter whose `present` is a tenth of this one's it is a
  larger share than anything else the event thread does.

## What is left, and what is not

**Nothing is owed on arm 1**, which is what "9–19 ms per step" was measured on: about a refresh of
composition and present, with no core work and no interpretation in it. **Arm 2 is owed to
[`46-the-kernel-floor.md`](46-the-kernel-floor.md) first and `47-the-encode-term.md` second**
(ADR 0767's split: kernels 44–46 ms, host terms ~19) rather than here — buying it here would mean
coalescing a gesture's renders, which shows a raster of the wrong size and is what the stand-in
already does correctly and honestly.

**The one shape worth a decision is the accessibility republication**, and it is a decision rather
than a patch: debouncing §14.7's publication to gesture-settle is a statement about what a screen
reader is owed *during* a drag, and this program has no evidence about what a client does with a
tree that changes forty times a second. It is 0.8–1.3 ms a step, it is on the event thread, and it
is the largest term on that thread once `present` belongs to the adapter. **Ask the owner before
building it**; `doc/todo/31` is where the accessibility argument lives.

The number this file used to carry — 9–19 ms per step on the 890M, from ADR 0704's traces — was
not reproducible here and did not need to be: the owner's window has an adapter this account
cannot open one on, and what the attribution needed was the *shape*, which is the same on any
adapter. What the 890M would still settle, in one trace of the recipe above, is whether arm 1's
`present` there is the 4 ms this machine's software stack shows or the 9–19 the passing
measurement recorded — and if it is the latter, the term is the acquire, which is the swapchain's
and not this program's.

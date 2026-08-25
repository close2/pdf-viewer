# 0699 — The settled frame is drawn twice as large and shown half size

**Status.** Accepted. Built at the owner's direct request, in the same session as ADR 0698:

> It is pretty clear that rendering everything at high resolution and then scaling it down
> would give a more correct image. In other words, the low-res image should look like a
> scaled down version of the high-res rendering.

## Context

The conflation seam (ADR 0308, corrected by ADR 0582): abutting fills each anti-aliased
independently leak backdrop at every shared edge — 19.4% of a layer showing through the
owner's 58 003-fill drawing at a page fit, halving per zoom doubling. All analytic-coverage
renderers do it; the renderers that visibly do not (ghostscript at `-dGraphicsAlphaBits=4`,
poppler's Splash) *supersample and downfilter*, which is the owner's sentence above.

`doc/todo/11` item 5 priced that cure and declined it: "N² of the rasteriser … on the one
path where this project has said latency is the feature … paid on every page including the
ones that do not need it." **Both halves of that objection are about scheduling, and
scheduling is what changed since it was written.** ADR 0378/0391/0443 built a presenter
that shows *something* on every view change while the real frame is drawn on a thread of
its own — so a pass that runs only after the real frame has landed and the view is quiet
pays nothing on the launch path, nothing on any gesture, and nothing on the frame
time-to-first-page measures. And the N² is not N² here: ADR 0443 measured that a frame of
this page costs its display-list walk, not its pixels (406 ms at 254×72 against 408 ms at
2027×576), so the 2× frame costs about one extra frame, once per settled view.

## Decision

**When a view settles, the render thread draws the page lane once more at twice the
window's resolution, and the presenter shows that texture at half scale.** `--supersample 1`
turns it off; 2 is the default and the only other value.

- **The downfilter is the presenter's existing bilinear tap, and at half scale it is
  exact.** quorra's `present.wgsl` samples the layer at the placement's inverse of the
  pixel centre; under `scale(0.5)` that lands precisely on the corner of each 2×2 texel
  block, so the four weights are ¼ each — a true box average, no new shader, no new
  sampler. This exactness is why only 2 is offered: at 4 the same tap would *skip* samples
  and look like the feature working.
- **The sharp texture lives beside the base, never through `Stale::settled`.** A 2× frame
  recorded as the settled view would make the next view change refuse to reproject
  (`Refusal::Resized`); as a presentation substitute the stale machinery never sees it,
  rule 1 is untouched, and under a reprojection it rides the same placement the base would
  have, composed after the halving.
- **The chrome stays at 1×**, drawn by its own quad from its own lane — text chrome drawn
  at 2× and filtered down would be a blur nobody asked for, and a 2× chrome frame would
  thrash that lane's scene key besides.
- **On the render thread it is idle work, after the frame and before the proxies** — the
  sharp picture is what the person is looking at now; a proxy is for a view change that
  has not happened yet. One attempt per settled view, a refusal does not spin
  (`draw_whole_page`'s discipline), and a job of the same pages at the same size — a
  selection, the find bar — does not re-pay it.
- **It says so in the trace** (`sharpened: the settled view at 2x, … ms on the render
  thread`), and the event loop is woken when it lands (the render thread now carries the
  `EventLoopProxy` the accessibility bridge already used), because a settled view is
  exactly when the loop is at rest.
- `--cpu` is untouched: the composing thread draws at window resolution by construction,
  and the oracle's comparison never sees any of this — the pass changes what is
  *presented*, not what `interpret` or any backend produces for a given `TargetSpec`.

## Measured

Xvfb at 1000×500, llvmpipe, `tmp/Entwurf.pdf`, the artwork band of the settled window,
`--supersample 2` against `--supersample 1`:

| | mean R | mean G | mean B | mean gradient |
|---|---:|---:|---:|---:|
| plain | 247.7 | 167.9 | 87.2 | 3.02 |
| sharpened | 247.8 | **162.0** | **75.4** | **3.49** |

Darker and more saturated — the backdrop leak roughly halves, exactly ADR 0308's
0.1937 → 0.1282 curve predicts for one doubling — and sharper, with 33 649 of 37 704
artwork pixels moved (max 51/255). The pass itself: **348.8 ms on the render thread** for
this worst-case page, 1.5 s after launch, wall-clock cost to the person zero. On the
owner's screenshots the same band read G ≈ 188 against Acrobat's ≈ 154 before this; one
doubling closes roughly half that distance, which is what a 2× pass can honestly buy.

## Costs, written down

- One extra page-lane render per settled view. On ordinary pages, milliseconds; on the
  58k-command monster, ~350 ms of idle-thread time during which a newly arrived job waits
  (ADR 0443 accepted the same bound for a proxy).
- A texture four windows large, allocated per pass rather than pooled — a sharp pass is
  per settled view, not per frame, and the texture it replaces is still on the window
  until the new one is adopted.
- The next same-view frame re-encodes rather than replaying (the 2× pass moved the page
  lane's scene key), and the glyph atlas holds two scales per page. `FrameCost::
  atlas_repacked` is the observable if this ever bites; the sweep that motivated ADR
  0443's fixed proxy edge needed seven scales before the atlas gave way.

## What this is not

Not the conflation cure. The seam is still drawn, half as wide; §11.2's ideal — no seam at
all — needs per-sample compositing, which is the compute lane's question (quorra's court,
with the determinism probe of this same session as its first artifact). When that lane
exists, this pass becomes its cheap fallback rather than dead code: the presenter-side
half-scale tap is how any higher-resolution rendering reaches the window.

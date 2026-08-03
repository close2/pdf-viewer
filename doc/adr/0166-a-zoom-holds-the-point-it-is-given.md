# ADR 0166 — A zoom holds the point it is given

Status: accepted, 2026-08-03. Session 214. `doc/todo/35-zoom-with-the-wheel.md`, asked for by the
project owner and now deleted.

## What was asked for

**Ctrl + the mouse wheel zooms**, the wheel alone goes on scrolling, and the page magnifies about
the thing being pointed at. Nothing in ISO 32000-2 decides any of it: §12.3.2.1's magnification is
a *document's* opinion about how to look at a page (ADR 0162) and this is a *reader's*, so every
choice below is a documented choice rather than a derivation — which is what `CLAUDE.md` asks for
where the standard is silent.

## The interesting half is not the modifier

The host half is a field and an arm: `winit` reports a modifier change as its own event and puts
nothing in the wheel's, so `pdf-viewer.rs` remembers whether Ctrl is down. Where the branch goes is
the only judgement in it: **after** the About card, which is modal and covers the page, and
**before** the sidebar, which has no scale to change — so a notch over the sidebar still magnifies
the page, with **no anchor**, because there is no point of the page under the pointer to hold and
`None` is the core's word for that. The first version put the branch first and zoomed about a
negative coordinate, which the trace said out loud: `zoom In at Some((-100.0, 300.0))`.

The half worth an ADR is that **`Command::Zoom(Zoom)` carried no position**, and
`Viewer::set_zoom` recentred the scroll about the viewport's middle:

> Both scrolls are in device pixels of a raster whose size changed by exactly this ratio, so
> scaling them about the viewport's centre keeps that point where it was.

That is right for a keyboard `+`, which names no point. It is wrong for a wheel: what makes wheel
zooming feel like magnification rather than like a jump is that the point under the cursor stays
under the cursor, and the further from the centre a person points the worse the centre's answer
gets.

## Two shapes, and why the variant changed rather than a second one arriving

- **`Command::Zoom { zoom, at: Option<(f32, f32)> }`** — one variant changes shape, every consumer
  fails to compile, and the arithmetic generalises from "half the extent" to "the given point".
- **A separate `Command::ZoomAt { zoom, at }`** — nothing existing changes, one more message.

The second is cheaper to land and worse to live with: two commands meaning the same thing with a
parameter between them is exactly what §0 of the handover says this vocabulary has avoided for ten
sessions, and the *reason* nothing in `viewer-core` is `#[non_exhaustive]` is that a host should be
made to recompile when a message changes rather than silently ignoring it in a catch-all arm.
`None` is the viewport's centre, so the keyboard's meaning is stated rather than inherited.

## The arithmetic, and the case the old one could not express

The obvious generalisation is to scale the *scroll* about the anchor —
`(scroll + at) * ratio - at` — and it is wrong, silently, on exactly the pages a wheel is most
often pointed at.

A page **smaller than the viewport is centred**, and its scroll is zero. `Open::origin` is the one
place that distinction lives: it answers the centring slack where there is slack and `-scroll`
where there is not. So the page point under a viewport point is `at - origin`, never `at + scroll`,
and the two agree only while there is something to scroll. Zooming a 400-pixel-wide page in a
1000-pixel window by four, anchored at the window's middle, the scroll-based arithmetic asks for
1500 where the answer is 300.

`Open::hold` therefore reads the origin:

```text
page point   = at - origin(viewport, raster before)
new scroll   = page point × ratio - at
```

clamped to zero here and against the new raster in `Viewer::settle`, which is where the new
raster's size is known. Two properties fall out and both are checked: a ratio of exactly one — what
`Open::stepped`'s clamp produces at either end of `ZOOM_RANGE` — leaves the scroll where it was
rather than moving it by a ratio it did not get; and a page that is smaller than the viewport
before and after cannot honour an anchor at all, so anchoring is a no-op there rather than a
jitter as the page crosses the size of the window.

## Cost

Two `f32` multiplies more than the old expression, on a path that runs once per wheel notch.

`WHEEL_ZOOM_PIXELS` is 50 and is the one number here with no argument behind it: a wheel notch is
one step by construction and a touchpad reports a stream of pixels instead, so *something* has to
say how many pixels a notch is worth. Fifty is about a finger's width on this machine's touchpad.
The accumulator is spent one step at a time, so a pinch does not fire forty commands, and the step
count is bounded at sixty-four — `ZOOM_RANGE` spans thirty-six steps of 1.25 end to end, so the
bound cannot hide a magnification anybody could have reached, and it is there because an `f32` cast
saturates and a device reporting nonsense would otherwise be a loop of two billion commands.

## How it is checked

`viewer-core/tests/headless.rs::a_zoom_holds_the_point_it_is_given`, with no display at all, on the
600×800 fixture in a 300×400 viewport where every number is checkable by hand. What it asserts is
the invariant the choice was made for — `(at - origin) / scale` is the same page point before and
after — at an exact fit, out of a centred page into a larger one, and across a zoom that changes
nothing. Confirmed to fail when `hold` ignores its anchor, which is the only thing that establishes
it guards it.

## And in the window, which is where the loop is

`Xvfb` + `xdotool` (the handover's Environment recipe), `doc/PDF20_AN001-BPC.pdf`, sidebar open so
the page's viewport is 500×1000 of an 800×1000 window. Ctrl held, two notches at screen (700, 250),
which is viewport (400, 250):

- Fitted, the page is 500×707 in a 1000-tall viewport, so it is **centred**: `origin` is (0, 146.5)
  and the scroll is zero in both directions. The page point under the pointer is
  (400 − 0)/0.84 = 476 and (250 − 146.5)/0.84 = 123 user units.
- Two steps is 1.5625, so holding x wants a scroll of 400 × 1.5625 − 400 = 225, which the new
  781-wide raster permits. Holding y wants −88, which is a request to pull the page *below* its own
  top edge; the clamp takes it to zero.
- Measured off the screenshots, the page's orange rectangle moves from screen (620, 391) to
  (576, 380). The arithmetic above predicts (575, 382).

So the horizontal anchor held exactly and the vertical one was refused for a reason — the top of
the page — which is the same "a step that is refused is not a scroll" property the headless test
asserts, arriving from the other side. Worth stating because a person testing this at the top of a
fitted page will see the y anchor *not* hold and it is not a defect.

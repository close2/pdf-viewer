# Zoom with the wheel, anchored where the pointer is

Status: **not started.** Asked for by the project owner.
Priority: 35 — capability, and the smaller half of it is a vocabulary question
Corpus: none; this is what a person does, not what a file says
Clauses: none. §12.3.2.1's magnification is a *document's* opinion (ADR 0162) and this is a
reader's, so nothing in the standard decides it — which makes every choice below a documented
choice rather than a derivation.
Code: `crates/viewer-ui/src/bin/pdf-viewer.rs`, `crates/viewer-core/src/command.rs`,
`crates/viewer-core/src/viewer.rs`

## What is asked for

**Ctrl + mouse wheel zooms**, which is the convention every desktop viewer the owner uses has
converged on, and the wheel alone goes on scrolling. It is not a clause and there is no corpus
evidence for it; it is written down as a preference with its reason, which is what `CLAUDE.md`
asks of a choice the standard does not make.

## What exists

`Command::Zoom(Zoom)` with `In`, `Out`, `FitPage`, `FitWidth`, `FitHeight` and `Scale(f32)`, and
`Viewer::set_zoom` behind it. `viewer-ui` binds `+`, `-` and `0` to the first three and routes
`WindowEvent::MouseWheel` through `wheel()`, which scrolls the About card, or the sidebar's list,
or the page — in that order, decided by what the pointer is over.

So the *step* already exists and is already the right size: `ZOOM_STEP` is 1.25 with its own
comment saying it is a choice and not a derivation.

## The two things it needs, and only one of them is in the host

### 1. The host has to know Ctrl is down — small

`pdf-viewer.rs` never handles `WindowEvent::ModifiersChanged` and keeps no modifier state at all.
Adding it is a field and one arm. `wheel()` then branches before its existing three-way routing:
Ctrl is a zoom over the *page* and should not zoom the sidebar's list or the About card, both of
which are chrome and neither of which has a magnification.

One caution the existing code already earned: a wheel notch arrives as `LineDelta` from a mouse
and `PixelDelta` from a touchpad, and `wheel()` converts the first at sixteen logical pixels a
line. A zoom must not reuse that conversion — sixteen *pixels* means nothing to a magnification.
The natural reading is one `Zoom::In`/`Zoom::Out` step per notch of `LineDelta`, and an
accumulator over `PixelDelta` so that a touchpad pinch does not fire forty steps.

### 2. The zoom has to be anchored at the pointer — a vocabulary question

This is the part worth thinking about before writing anything.

`Viewer::set_zoom` recentres the scroll **about the viewport's centre**:

> Both scrolls are in device pixels of a raster whose size changed by exactly this ratio, so
> scaling them about the viewport's centre keeps that point where it was.

That is right for a keyboard `+`, where there is no other point to prefer. It is wrong for a
wheel: what makes wheel zooming feel like magnification rather than like a jump is that **the
point under the cursor stays under the cursor**. Anchoring at the centre moves whatever the person
was pointing at, and the further from the centre they point the worse it is.

`Command::Zoom(Zoom)` carries no position. `Open` does keep a pointer — `pointer` and `inside` —
but those are §12.5.5's *appearance* state and §12.6.3's enter/exit bookkeeping, filtered to
annotations; neither is "where the cursor is", and reusing them would be the kind of
same-name-different-meaning mistake this tree has paid for before.

So the honest options are:

- **`Command::Zoom { zoom, at: Option<(f32, f32)> }`** — a viewport point to hold fixed, `None`
  meaning the centre. One variant changes shape, every consumer fails to compile (which is the
  point of nothing being `#[non_exhaustive]`), and the arithmetic in `set_zoom` generalises
  from "half the extent" to "the given point" in three lines.
- **A separate `Command::ZoomAt { zoom, at }`** — no change to the existing variant, one more
  message. Cheaper to land and worse to live with: two commands that mean the same thing with a
  parameter between them is exactly what §0 says the vocabulary has avoided for ten sessions.

The first is the one to argue for, and it should be argued in an ADR rather than chosen here.

## What to check when it lands

- The point under the cursor stays under the cursor **across a step at the clamp**: `ZOOM_RANGE`
  is `(0.02, 64.0)` and `Open::stepped` clamps into it, so a step that is refused must move the
  scroll by nothing rather than by the ratio it did not get.
- **A page smaller than the viewport is centred, not scrolled** — `Open::origin` returns the slack
  when the raster is smaller, and the scroll is clamped to zero. Anchoring has to be a no-op
  there, or the page will jitter as it crosses the size of the window.
- `viewer-core/tests/headless.rs` can drive the whole of this with no display, which is what that
  test file is for: a `Resize`, a `Zoom` at a named point, and `Query::PageGeometry` to say where
  the page went. `an_open_action_states_where_to_look_at_the_page_and_how_large` is the pattern —
  numbers checkable by hand on a fixture whose geometry is a round number.

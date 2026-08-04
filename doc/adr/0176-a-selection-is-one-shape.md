# ADR 0176 — A selection is one shape, and a blend mode is priced per layer

Status: accepted, two-hundred-and-fifty-second session.

## Context

The project owner reported that dragging a selection across `issue14821.pdf` made the window stop
answering. The two-hundred-and-fifty-first session reproduced it in a window on `Xvfb`, measured
it, and settled the design without fixing it; `doc/todo/13` carries the whole reproduction. The
arithmetic closed to the byte:

```text
internal_texture_bytes = (layers + 1) × 2 × width × height × 4
(63 + 1) × 2 × 3 200 000 = 409 600 000      the number the refusal printed
```

`highlight_list` drew **one fill per selection quad**, each with `BlendMode::Multiply`. A
compositor gives every non-`Over` blend its own layer, so at 800 × 1000 a selection quad cost
6.4 MB of frame budget and one short paragraph of selected text — 63 quads — spent the whole
256 MiB. Before any refusal it was already costing **1.9 ms of present time per quad**.

## Decision

**One fill of one path, with one subpath per quad, under one `Multiply` layer.**

Two things are being separated, and only one of them was load-bearing:

- **The blend mode stays.** §11.3.5.2 makes `Multiply` the mode whose "result colour is always at
  least as dark as either of the two constituent colours" — the standard's own guarantee that
  what is under the wash survives it. This tree already cites that clause for §12.5.6.10's
  highlight annotations. A photograph of a live selection confirms it: every character stays
  legible through the blue.
- **The fill *per quad* was preserving a behaviour nobody wants.** Its comment said "overlapping
  quads darken twice, which is what overlapping selections look like", and that was an assertion
  nobody had checked. `Query::Selection` answers one quad per *run* of text, and runs tile rather
  than overlap: three lines of `tracemonkey.pdf` give 19 quads and **two overlapping pairs out of
  171**, overlapping by 0.28 and 0.17 of a device pixel. Where the behaviour arises it is
  invisible, and if it were visible it would read as a defect — no reader draws a darker patch
  inside one selection.

Under the non-zero winding rule one path with many subpaths is one shape, so the sub-pixel slivers
stop darkening twice as a consequence of the fix rather than as a separate decision.

## What it costs, measured

The same window, the same recipe, the same document, after:

```text
trace: SELECTION quads 268    present -> presented in 16.5 ms
```

against 37 quads in 105.6 ms and a refusal at 63 before. **The compositor's cost no longer depends
on what is selected at all** — `(1 + 1) × 2 × width × height × 4`, once, whatever the drag covers —
and present time is back to the baseline a page costs.

The `--trace` line printing the quad count is now in the tree rather than in a todo file. It is the
number every part of the diagnosis turned on, and the session that measured it had to add it by
hand.

## What this did not fix, and what closed it

**A refused frame left the window blocking one second a present, for ever** — the second half of
`doc/todo/13`, reachable without the first, since any frame the device refuses got there. Reading
quorra's source named the mechanism: `bind_target` acquired the swapchain texture *before* the
frame budget was checked, so a refused frame dropped an acquired surface texture, and
`SurfaceProblem::Timeout` was the one swapchain state that did not set `needs_reconfigure` — which
is why a resize recovered the window and nothing else did.

**Answered in the library, at `4aab7e2`**, in all three shapes `doc/QUORRA_FEEDBACK.md` §7 asked
for: the budget is priced before the target is bound, `Timeout` invalidates the surface, and
`Device::invalidate_surface` exists for a host that needs to say so itself. Verified by restoring
this ADR's own defect locally and running the original report's recipe against the new library:
every refused present costs **6 ms instead of 1.008 s**, nothing reports `Timeout`, and the drag
keeps updating throughout. `doc/todo/13` is deleted.

**The order of the two repairs was the right way round and worth saying so.** Fixing only the cost
would have hidden a wedge that any other over-budget frame could still reach; fixing only the
wedge would have left a selection costing 1.9 ms a quad. Each was reported to whoever owned it.

## The lesson

**A comment asserting what a person wants is a claim, and it decays like any other.** The per-quad
fill was justified by one sentence about overlapping selections, and the behaviour it described
does not arise — 169 of 171 quad pairs in the measurement never touch, and the two that do overlap
by a quarter of a pixel. The cost of that unchecked sentence was the only defect on this project's
list that a person met by using the program normally.

**And a display list's shape is a bill.** Nothing in `pdf-render` distinguishes one fill of *n*
subpaths from *n* fills of one — they draw the same pixels — but a backend that gives every
non-`Over` blend its own compositor layer charges per command. A host that hands over geometry
rather than pixels is choosing what the backend will be asked to allocate.

# A presentation player

Status: seven of Table 164's twelve styles are drawn; five are reported by name.
Priority: 32
Clauses: §12.4.4
Code: `crates/viewer-core/src/transition.rs`, `crates/viewer-ui/src/bin/pdf-viewer.rs`

§12.4.4's whole presentation is read — Table 164's transition styles, `/Dur`'s auto-advance,
§12.4.4.2's sub-page navigation — the core has *advanced* a slide show since the hundred-and-fiftieth
session (`Command::Tick`, ADR 0135), and since the **three-hundred-and-ninety-third** a host draws
the frames (ADR 0230): `viewer_core::transition::frame` shapes the frame at a fraction of the way
through, `Frame::draw` turns it into a display list of two page rasters so both backends draw it,
and `viewer-ui`'s `p` is the clock.

What is left, in the order the cost rises:

- **`Blinds`** — one number the clause does not state: how many "[m]ultiple lines, evenly spaced
  across the screen" there are. The reveal vocabulary already expresses it (a list of rectangles).
- **`Glitter`** — a band width and what a dissolve looks like inside it, "in a wide band moving
  from one side of the screen to the other in the direction specified by the Di entry". `/Di 315`
  is this style's alone and `quarter` refuses it today, so a diagonal band is part of the work.
- **`Dissolve`** — the one of the four the display list's vocabulary does not already express: a
  per-pixel pattern rather than a region. A coarse cell grid would be a *choice* and would have to
  say so.
- **`Fly`** — "[c]hanges are flown out or in", so the flown object is the **difference** between
  two pages, with `/SS` scaling it and `/B` deciding whether it is rectangular and opaque. That is
  a page diff and a different kind of problem from the other three.

Each is reported by name today rather than drawn as a cut, which is the rule that keeps this
honest debt rather than a silence.

Two other things this clause still owes:

- **§12.4.4.2's sub-page navigation has no control.** `navigation::steps` reads the `/PresSteps`
  chain and `ViewState::perform_all` performs a node's actions; nothing walks them, because the
  arrow keys turn pages. A presentation that is running should step *within* a page first.
- **Full screen.** Chrome, and therefore the host's — §12.4.4.2's NOTE 3 says a processor "needs
  to respect navigation nodes only when in presentation mode", and this program's presentation
  mode is a clock rather than a window state.

**No corpus document exercises any of it**: `pdf-model/examples/presentation_census` walks the page
tree of all 978 documents this tree opens and finds no `/Trans`, no `/Dur` and no `/PresSteps`, so
every witness is hand-built (`examples/presentation_fixture` writes one).

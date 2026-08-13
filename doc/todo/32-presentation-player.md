# A presentation player

Status: seven of Table 164's twelve styles are drawn; five are reported by name. §12.4.4.2's states
are walked.
Priority: 32
Clauses: §12.4.4
Code: `crates/viewer-core/src/transition.rs`, `crates/viewer-core/src/presentation.rs`,
`crates/viewer-ui/src/bin/pdf-viewer.rs`

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

One other thing this clause still owes:

- **Full screen.** Chrome, and therefore the host's. It is the *window*, and deliberately not the
  mode: §12.4.4.2's NOTE 3 asks a processor to respect navigation nodes "only when in presentation
  mode", and since the four-hundred-and-eighty-first session that condition is a value a host states
  — `Command::Present(PresentationMode)`, ADR 0316 — rather than a window this program does not
  have. So what is left here is a host drawing a page with no chrome round it, which no clause asks
  for.

**§12.4.4.2's states are walked**, since that same session: the current navigation node the clause
opens by requiring, `/PresSteps` on arrival, `/NA` then `/Next`, `/PA` then `/Prev`, Table 165's
per-node `/Dur` — which nothing had read — and NOTE 2's save and restore of §8.11's groups across
the mode. `crates/viewer-core/src/presentation.rs`.

**No corpus document exercises any of it**: `pdf-model/examples/presentation_census` walks the page
tree of every document this tree opens and finds no `/Trans`, no `/Dur` and no `/PresSteps`, so
every witness is hand-built — `examples/presentation_fixture` writes one, whose fourth slide is the
one with states, and `viewer-core/tests/sub_page_navigation.rs` writes the pair that differs in the
single entry.

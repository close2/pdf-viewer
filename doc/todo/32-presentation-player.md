# A presentation player

Status: seven of Table 164's twelve styles are drawn; five are reported by name. §12.4.4.2's states
are walked, **and since the six-hundred-and-thirty-eighth session there is a window**: all three
hosts present full screen (ADR 0470).
Priority: 32
Clauses: §12.4.4, §12.2, Table 29
Code: `crates/viewer-core/src/transition.rs`, `crates/viewer-core/src/presentation.rs`,
`crates/viewer-host/src/presentation.rs`, `crates/viewer-ui/src/bin/pdf-viewer/presentation.rs`,
`crates/viewer-gtk/src/host.rs`, `crates/viewer-qt/src/host.rs`

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

~~One other thing this clause still owes: **full screen**~~ — **taken in the
six-hundred-and-thirty-eighth** (ADR 0470), and this entry was wrong in the way a refusal usually
is. It said full screen is "a host drawing a page with no chrome round it, **which no clause asks
for**". Two clauses ask for it, neither of them §12.4.4: Table 29's `/PageMode` names `FullScreen`
— "[f]ull-screen mode, with no menu bar, window controls, or any other window visible" — as one of
the six ways a document "shall be displayed when opened", and §12.2's Table 147 states the same
subject in the smaller with `/HideToolbar`, `/HideMenubar` and `/HideWindowUI`, plus
`/NonFullScreenPageMode` for the way back out. `viewer_host::Presenting` is those five sentences,
shared by all three hosts because only the toolkit call differs; `p` enters, Escape leaves, and a
document stating `/PageMode /FullScreen` opens presenting. **No message was added and no variant
changed shape** — `Command::Present`, `Query::Opening` and `Query::Preferences` were all already
there.

What the *window* left behind, and it is the next item under `doc/todo/30`'s "all three hosts stay
level":

- **The two native hosts have the mode and not the clock.** `viewer-gtk` and `viewer-qt` send
  `Command::Present` now, so §12.4.4.2's states are walked in both — but neither drives
  `Command::Tick`, so `/Dur` does not advance a page there and §12.4.4.1's transition frames are
  still `viewer-ui`'s alone. A `glib` timeout and a `QTimer` are what that costs, plus a way for a
  tier-1 host to hold the two page rasters `viewer_core::transition::frame` places.

**§12.4.4.2's states are walked**, since that same session: the current navigation node the clause
opens by requiring, `/PresSteps` on arrival, `/NA` then `/Next`, `/PA` then `/Prev`, Table 165's
per-node `/Dur` — which nothing had read — and NOTE 2's save and restore of §8.11's groups across
the mode. `crates/viewer-core/src/presentation.rs`.

**No corpus document exercises any of it**: `pdf-model/examples/presentation_census` walks the page
tree of every document this tree opens and finds no `/Trans`, no `/Dur` and no `/PresSteps`, so
every witness is hand-built — `examples/presentation_fixture` writes one, whose fourth slide is the
one with states, and `viewer-core/tests/sub_page_navigation.rs` writes the pair that differs in the
single entry. **The fixture writes the window too since the six-hundred-and-thirty-eighth**:
`--opens-full-screen` adds Table 29's `/PageMode /FullScreen` and a §12.2 `/ViewerPreferences`
beside it, which is the file all three hosts were driven on.

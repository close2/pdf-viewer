# A presentation player

Status: seven of Table 164's twelve styles are drawn; five are reported by name. §12.4.4.2's states
are walked, **there is a window since the six-hundred-and-thirty-eighth session** (ADR 0470), and
**all three hosts drive the clock since the six-hundred-and-forty-second** (ADR 0473). What is left
is the five styles.
Priority: 32
Clauses: §12.4.4, §12.2, Table 29
Code: `crates/viewer-core/src/transition.rs`, `crates/viewer-core/src/presentation.rs`,
`crates/viewer-host/src/presentation.rs`, `crates/viewer-host/src/clock.rs`,
`crates/viewer-ui/src/bin/pdf-viewer/presentation.rs`,
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

~~What the *window* left behind~~ — **taken in the six-hundred-and-forty-second** (ADR 0473). It
read: "[t]he two native hosts have the mode and not the clock", so `/Dur` advanced no page in GTK
or Qt and §12.4.4.1's frames were `viewer-ui`'s alone. All three drive the clock now, on one shared
decision: `viewer_host::Clock` answers how often to tick, what a tick carries, that the clock is
*held* while a transition is drawn — §12.4.4.1's EXAMPLE puts the effect "[b]efore the page is
displayed" — and when Table 164's `/D` has elapsed. The event loop is all that differs: a re-armed
`glib` one-shot, a `QTimer` whose interval the host sets, and winit's `ControlFlow::WaitUntil`.
`viewer-ui` adopted it and is shorter for it. **No message was added and no variant changed
shape**, which is the third round running.

**A clock that runs when nothing is presenting is a defect, and there is none**: `Clock` is an
`Option` per host with no paused state, so leaving full screen removes the GTK source and stops the
`QTimer`. A page stating no `/Dur` produces no events, and both native hosts repaint nothing on
such a tick — "the page shall not advance automatically" costs a still window one wakeup and no
texture.

**§12.4.4.2's states are walked**, since that same session: the current navigation node the clause
opens by requiring, `/PresSteps` on arrival, `/NA` then `/Next`, `/PA` then `/Prev`, Table 165's
per-node `/Dur` — which nothing had read — and NOTE 2's save and restore of §8.11's groups across
the mode. `crates/viewer-core/src/presentation.rs`.

**And the end of the `/Next` chain is the clause's business rather than this program's**, found in
the six-hundred-and-forty-second by `spec-errata emit` over clause 12. Errata **issue #304** inserts
into item (b) *If there is no node specified by Next then navigate to the next page. If the current
page is the last page, then the current navigation node remains unchanged*, and the same of `/Prev`
into item (d) — so the request that runs the **last** node's `/NA` is the request that turns the
page, where this reader used to swallow it and turn the page on the next one, and on the last page
the node stays current instead of being cleared. ADR 0473 has the reading and the two callers that
decline the page turn with the clause that says so.

**No corpus document exercises any of it**: `pdf-model/examples/presentation_census` walks the page
tree of every document this tree opens and finds no `/Trans`, no `/Dur` and no `/PresSteps`, so
every witness is hand-built — `examples/presentation_fixture` writes one, whose fourth slide is the
one with states, and `viewer-core/tests/sub_page_navigation.rs` writes the pair that differs in the
single entry. **The fixture writes the window too since the six-hundred-and-thirty-eighth**:
`--opens-full-screen` adds Table 29's `/PageMode /FullScreen` and a §12.2 `/ViewerPreferences`
beside it, which is the file all three hosts were driven on.

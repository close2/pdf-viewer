# A presentation player

Status: seven of Table 164's twelve styles are drawn; **four** are reported by name, and the fifth
of the unshaped is `R`, which the table defines as the cut and which therefore needs nothing.
§12.4.4.2's states are walked, **there is a window since the six-hundred-and-thirty-eighth session**
(ADR 0470), and **all three hosts drive the clock since the six-hundred-and-forty-second** (ADR
0473). What is left is the four styles below. (This line said *five* until the
six-hundred-and-sixty-third session, which is the wording §12.6.4.15's and §12.4's ledger rows
retired in the five-hundred-and-fifty-third and which was still standing in eight other places, one
of them a sentence a host shows a person and one of them this line — ADR 0490.)
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

**And the four are ranked by demand since the six-hundred-and-sixty-third session, which is a
different order from the one above.** The curated corpora state no presentation at all — 0 of 1133
documents, which is why this list has only ever been ordered by what it would cost — and the
SafeDocs crawl states 276 of them. Over the 65 703 documents of `CC-MAIN-2021-31` that open
(`examples/presentation_census`, chunked through `xargs -P 8`, under a minute):

| style | pages | documents | drawn? |
|---|---|---|---|
| `R` | 4084 | 187 | the cut, by definition |
| `Fade` | 596 | 33 | yes |
| `Wipe` | 258 | 15 | yes |
| **`Dissolve`** | **221** | **11** | no |
| `Push` | 162 | 12 | yes |
| `Box` | 66 | 7 | yes |
| `Cover` / `Uncover` | 23 / 21 | 5 / 4 | yes |
| **`Blinds`** | **16** | **4** | no |
| `Split` | 2 | 1 | yes |
| **`Glitter`**, **`Fly`** | **0** | **0** | no |
| a name not in Table 164 | 106 | 3 | reported as the thirteenth case |

So **`Dissolve` is the whole of the demand in practice**, and it is the one the display list's
vocabulary does not express — the cheapest item on the list above is the one nobody asks for and the
dearest but one is the one everybody does. `Glitter` and `Fly` are refusals no file has ever
reached, which is a reason to leave them rather than to do them: a sentence nobody reads costs
nothing and a choice this reader would have to invent is worse than a report. All three documents in
the last row write `/Trans<</S/>>` — an **empty** name, which is a syntactically valid name object
and not one of Table 164's values — and this reader keeps it as `Style::Unrecognised`, shows the
page at once and says so. The page a conforming reader draws is the same one Table 164's default
`R` would give; what differs is that a report is made, and that is the tree's own rule (trap 5's
channel) rather than a clause, because the table states a default for an *absent* `/S` and nothing
at all for a present one whose value it does not list.

**And there are real files to press `p` on now**, which this entry could not say before:
`corpus-cache/safedocs/cc-main-2021-31/7680/7680405.pdf` is a slide deck with `/PresSteps` on four
of its 39 pages, Table 165's nodes over §12.6.4.13's `/SetOCGState`, and fourteen §12.6.4.15
transition *actions*. It is the only crawled document that states any of those three.

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

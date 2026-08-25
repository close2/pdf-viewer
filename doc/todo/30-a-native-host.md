# A native host, then the C ABI

Status: **all three built.** GTK4 in the four-hundred-and-eighth session (`crates/viewer-gtk`,
ADR 0244); Qt in the four-hundred-and-tenth (`crates/viewer-qt`, ADR 0246), and with it
`crates/viewer-host`; **the C ABI in the four-hundred-and-eleventh** (`crates/viewer-ffi`,
ADR 0247), with its three amendments taken first. **This file absorbed `doc/todo/37` in the
four-hundred-and-ninth**, whose one open decision was taken (ADR 0245). **And its remaining surface
was taken in the five-hundred-and-eleventh** (ADR 0346): the ABI's entry points were the whole
vocabulary *then*, Table 229 bit 26 is obeyed, and ADR 0245's scale question is answered with the
messages that already existed. **"The whole vocabulary" has since decayed and is now counted rather
than claimed** — `tools/state.sh hosts` prints how much of `Command` and `Query` a C caller can
reach and names what it cannot (ADR 0509), and **`tools/state.sh windows` prints the same for each
window** (ADR 0577). **Item 5 closed in the seven-hundred-and-ninth**: every `Query` variant reaches
a symbol, and a test that matches exhaustively over the enum is what keeps it that way (ADR 0576).
Priority: 30 — what is left is *surface* rather than architecture, and the file says which
Code: `crates/viewer-gtk`, `crates/viewer-qt`, `crates/viewer-host`, `crates/viewer-ffi`

## The goal, stated by the owner

The viewer is to be **embeddable in native frameworks** — Win32/WinUI, AppKit, KDE/Qt, GTK — not
built on a cross-platform toolkit. `viewer-core` is that interface: `Command` in, `Event` out,
`Query` → `Answer` beside them, with no type from a windowing or graphics library anywhere in its
API (ADRs 0116 to 0121, and `doc/ui-boundary.md` for the
vocabulary and the three pixel tiers).

## The order, and it was not negotiable

1. ~~**GTK4 via `gtk4-rs`.**~~ **Done in the four-hundred-and-eighth** (ADR 0244).
2. ~~**Qt/KDE via `cxx-qt`.**~~ **Done in the four-hundred-and-tenth** (ADR 0246), and the order was
   right for the reason it was given: the bridge *is* where the awkwardness showed up, and none of
   it was the boundary's.
3. ~~**`viewer-ffi` last.**~~ **Done in the four-hundred-and-eleventh** (ADR 0247). *"Do not freeze
   a C ABI until two Rust consumers have shaken the API out"* was met, and the three amendments the
   two hosts named were taken before a line of the ABI was written.

**The vocabulary needed no new message for a third host running**, and it still needs none: eleven
messages in eleven rounds of hosts, and three whole hosts — one of them in another language — added
no *message* between them. **What they did find was a variant carrying too little**, which is a
different thing and has its own mechanism: `Edit::SetField`'s value changed shape in the
four-hundred-and-twelfth so that §12.7.5.4's list box could say which items are selected (ADR 0248),
and every consumer failed to compile until it said what it does. A message added is a channel that
did not exist; a variant changed is a channel that was too narrow, and only the second is something
`#[non_exhaustive]` would have hidden.

## What the three amendments cost, because it was not what this file predicted

All three are in ADR 0247. Two are worth knowing here because the file was **wrong about one of
them**:

- `pdf_render::RasterFormat` lost `#[non_exhaustive]`, and four consumers lost a runtime refusal for
  a condition a build now catches.
- `Answer::Outline` is owned. All five consumers cloned it anyway; measured at 481 ns for 14 rows
  and 80.7 µs for 988.
- **`Answer::Field`'s password value was not "one sentence in a doc comment", as this file said.** It
  was a live bug: `viewer-ui` read the value back after every keystroke (ADR 0201) and a password
  field answers with bullets, so typing into one produced `Edit::SetField` with the bullets and a
  character appended. The variant changed shape — `Option<pdf_model::view::ShownValue>`, characters
  and `obscured` from one reading — and every consumer failed to compile, which is what that is for.
  Reading the clause for it found a **second** sentence nobody had read: Table 231 bit 14's NOTE
  forbids storing the value in the file, and `ViewState::save` was storing it.

## What is left, and none of it is architecture

**Every item below is a function to add to the C ABI, a widget to place, or a clause to obey.** No
new message, and no new decision about the boundary. **Four of the five are closed in the
five-hundred-and-eleventh** (ADR 0346) and **the fifth in the five-hundred-and-seventy-first**
(ADR 0407). **The tail — Qt's measurement being on the far side of a `cxx` bridge — was taken in
the six-hundred-and-first** (ADR 0436), and reading §12.7.5.4 against the mapping the hosts build
their controls from left one item in its place, below. Plus the standing note about where the
`unsafe` is.

- ~~**`viewer-gtk` does not obey Table 234's `/TI`.**~~ **Closed in the
  six-hundred-and-seventy-eighth** (ADR 0508), and **this entry was wrong about both of the things
  it asserted.** The clause half was right — the entry is "the index in the Opt array of the first
  option visible in the list", `pdf-model` has read it since the three-hundred-and-ninety-eighth
  session, the page's own appearance obeys it since ADR 0407, `viewer_host::form::ControlKind::List`
  dropped it until the six-hundred-and-first, and `viewer-qt` scrolls to it with
  `scrollToItem(..., PositionAtTop)`. What was wrong is what followed.

  **The floor was not the blocker.** It was raised — `gtk4` 0.11.4 offers `v4_12` and this machine
  runs GTK 4.22 — and `GtkListView::scroll_to` then moved nothing at `/TI 1` and nothing at `/TI 5`,
  because what GTK's method does is put an item *into view*: its `GtkScrollInfo` argument is
  documented as "%NULL to scroll into view" and carries two booleans about which axes may move and
  no alignment at all. Qt states a position and GTK has no counterpart. The floor went back to
  `v4_10`, because what a feature floor costs is a *runtime* requirement and it is not worth raising
  for an API that does not answer the question; `Cargo.toml`'s comment carries the experiment.
  `controls.rs::scroll_to_top_index` uses the `GtkScrolledWindow`'s own adjustment, and the one
  non-obvious line is measured rather than reasoned: a `GtkListBase` recomputes the adjustment from
  its *anchor item* at every allocation, so the value has to be written from an idle, after GTK's
  layout, where the adjustment moving is what updates the anchor.

  **And the corpus witness exists.** This entry said "[n]o corpus document is known to state one",
  on the evidence of a census that counts list boxes and does not count `/TI` — a statement about an
  instrument wearing the clothes of a statement about the corpus.
  `doc/pdf.js/test/pdfs/annotation-choice-widget.pdf` object 62 is a multiple-selection list box
  with eight options, `/TI 1`, and a `/V` naming four *different* ones, on a page that also carries
  two list boxes with no `/TI` — so one screenshot shows the entry obeyed beside two controls with
  nothing to obey. Driven under `Xvfb`; both native hosts now show *Ipsum* first on that file, and
  `viewer-ui` is level by a third route because it is the host that does not delegate, so its list
  is the page's own appearance.

- ~~**The ABI is 43 entry points and not the whole vocabulary.**~~ **Closed in the
  five-hundred-and-eleventh** (ADR 0346): 43 → **111**, and the list this entry carried is all of
  it — `Command::Pointer` and `Command::Select` with the selection, the caret and §12.5.6.6's drag;
  `Query::Fields` and all four `Edit`s; `Command::Save` and `Command::Extract` with a **byte**
  accessor apiece, because a file is not text and the NUL idiom would cut one at its first zero
  byte; §8.11.4.3's layers and §7.11.4's files as a second flattened panel; §12.4.4's clock and its
  transitions; and the three policy values. **`PDFV_EVENT_KIND_COUNT` is 16 before and after**,
  which is the third demonstration in three rounds of what the shape was chosen for: a `Command` is
  a symbol, a `Query` is a symbol, and only an `Event` is a number. Two enumerations are now
  *answered with* — `ControlKind` and `RowKind` — and each has a count and a name of its own rather
  than a place in `pdfv_abi_check`, because an event arrives unasked and a control kind is the
  answer to a call the caller wrote. **And a `#define` had been missing since the count last moved**:
  `PDFV_EVENT_SEARCHED` was never added in the four-hundred-and-fourteenth, so a C caller wrote `15`
  by hand — `header_and_library_agree.rs` compares the header against a *hand-written* map, and a
  constant absent from both sides agrees with itself.
- ~~**The scale a native form host draws the page at.**~~ **Answered in the
  five-hundred-and-eleventh, and it needed no message** (ADR 0346). `viewer_host::ControlFit` is the
  one piece that did not exist: a control's minimum does not change with the page's magnification
  and its `/Rect` does, in proportion, so the magnification at which everything fits is the current
  one times the worst ratio of minimum to asked. `Query::Fields` gives the rectangles,
  the toolkit gives the minimums, `Zoom::Scale` applies the answer. Driven under `Xvfb` on
  `160F-2019.pdf`: `11 of 76 control(s) wider than their /Rect (worst +85 on 120 px), 76 taller
  (worst +22 on 12 px); every control fits at 3.278`, and after `w` sends it, `0 of 76 … 0 taller`.
  **And the second host feeds the same arithmetic since the six-hundred-and-first** (ADR 0436): the
  `(asked, minimum)` pairs cross the `cxx` bridge as `QtMeasure`, one call per placement, and
  `cpp/window.cpp` lost the counting it used to do for itself. Qt answers 4.667 where GTK answers
  3.278 on the same page, which is the finding standing rather than a disagreement — a minimum size
  is a *style's*, and the two hosts now differ in the measurement alone. *When* to apply it is still
  deliberately not decided: a viewer that magnified a page by itself because a form is on it would
  be answering a question nobody asked, so `w` offers it and nothing takes it.
- ~~**Table 229 bit 26's `RadiosInUnison` crosses and is not obeyed.**~~ **Obeyed since the
  five-hundred-and-eleventh, and this entry was wrong about it in both directions** (ADR 0346). The
  half it describes — turning on every button of a set that shares an on state — was already
  happening, by construction and without the flag ever being read: `/V` is a name and a widget is on
  when its `/AP /N` holds it. What was *not* obeyed is §12.7.5.2.3's requirement for the flag being
  **clear**, "at most one radio button in a field shall be set at a time", and this tree turned them
  all on. The sentence is in the **check box** subclause, attached to `/Opt`, which is why a round
  reading §12.7.5.2.4 for a radio button's flag would not have found it. Which button stays on is a
  documented choice — the first in `/Kids` — because the clause states none and a file that gave two
  buttons one name cannot say. No corpus witness in either direction over 1293 documents
  (`field_flag_census`, which counts bit 26 by field type since that round).
- ~~**§12.7.5.4's list box is the one place the boundary genuinely limits a host.**~~ **Closed in
  the four-hundred-and-twelfth** (ADR 0248), and it was the only thing on this list that changed
  `viewer-core`. `Edit::SetField`'s value is `pdf_model::view::Entered` now — characters,
  §12.7.5.4's chosen options by index, or a clear — so `viewer-gtk` builds a `GtkMultiSelection` and
  `viewer-qt` an `ExtendedSelection` where Table 233 bit 22 is set, and each writes `/V` in both the
  shapes the clause states with Table 234's `/I` beside it. Driven under `Xvfb` on `issue17492.pdf`
  in both hosts, which wrote **byte-identical** files. The variant changed and every consumer failed
  to compile, which is the shape ADRs 0166, 0167 and 0247 established and the fourth time it has
  been used.
- ~~**§12.7.5.4's list box still draws nothing on the page.**~~ **Closed in the
  five-hundred-and-seventy-first** (ADR 0407), and the entry was wrong in the way a refusal is
  usually wrong: the clause states no highlight for the *selection* and states the options
  outright — "each of which shall be represented by a text string that shall be displayed on the
  screen", in the array's own order by Table 233 bit 20, from Table 234's `/TI`. A mark added over
  an item that is drawn either way may not take the item down with it (ADR 0106's test), so the
  options are drawn and the unmarked selection is reported. What is left on the page is the mark,
  which no clause states and which this tree will not invent; a host draws it from
  `ChoiceControl::selected`, as it draws a text selection, in its own colour.
- **Both native windows rasterise on the event thread, so neither can stop a draw** — open, found
  by asking which hosts ADR 0657's interrupt policy has. `viewer-gtk` and `viewer-qt` both call
  `self.rasterizer.rasterize(&request.list, request.target)` **inside** their `Event::NeedsRender`
  arm, on the toolkit's main thread. `pdf_render::Interrupt` is a flag *another* thread raises, so
  in those two there is no other thread to raise it and nothing for it to interrupt but the loop
  that would do the raising: a page written to draw for 27.6 s (ADR 0650) takes the window with
  it — no repaint, no key, no way to say stop.

  **The answer is not a watchdog**, and that is worth writing down so a later round does not reach
  for one: a thread whose only job is to raise the flag after a fixed duration is precisely the
  automatic deadline ADR 0657 §1 measured and refused, and it would refuse legitimate pages while
  a document that chose its cost passed. What these two need is `viewer-ui`'s arrangement — the
  draw on a thread of its own (`crate::composer`, ADR 0461) — after which they get the policy
  unchanged and it moves into `viewer-host`, which is where it belongs the moment there are two
  hosts to write it twice.

  `viewer-ffi` is the third and the case is different: a C caller is *told* to move the request to
  a thread of its own, so the structure is already right and what is missing is an ABI entry point
  to raise a flag with. That is a header change and a levelness question of its own.

## Three hosts, and what turned out not to be a toolkit's

`crates/viewer-host` exists since the four-hundred-and-tenth because the second host wanted four of
`viewer-gtk`'s eight modules **unchanged**: the three panel answers as one row shape, §12.7.5's
field as the control it is, §12.7.6.4's file policy, and the launch timeline. None of them named a
GTK type. **The third host takes `panel` too**, which is the finding tested rather than repeated: a
C ABI is a native host, and a native host on this boundary is mostly not toolkit code. It is
deliberately *not* in `viewer-core` — a mapping from three answers into one row shape is a
convenience for whoever draws a tree, not a statement about a document (ADR 0246).

Adding `egui` buys a widget set for a large dependency and no architectural proof: winit + a GPU
*is* the unnative UI. The thing worth adding was the headless consumer, and it is there.

## Where the `unsafe` is, now that both crates that have any exist

`doc/todo/30` used to say `viewer-ffi` would be "the **only** crate in the tree permitted `unsafe`".
Two crates lift `deny(unsafe_code)` and no more: `viewer-qt`, for `#[cxx::bridge]`'s expansion, with
**one** hand-written token; and `viewer-ffi`, whose `src/abi.rs` holds one lint lift, an
`#[unsafe(no_mangle)]` attribute per entry point (`tools/state.sh hosts` counts them) and **no
`unsafe` block at all**. Each has a
test that reads its own sources back, and `viewer-ffi`'s additionally asserts that `pdf-syntax`,
`pdf-model`, `pdf-font`, `pdf-render`, `render-cpu`, `viewer-core` and `viewer-host` still hold
`#![forbid(unsafe_code)]` — the compiler-enforced rule this file promised would survive, checked
rather than promised. A third name appearing in either list is a change to a rule the project owner
stated and belongs in an ADR.

## The UI itself is now work, and all three hosts stay level — the owner, 2026-08-20

> even though low priority, I think we should start investing time into the UI (and its API for the
> native versions).

Two decisions come with it, and the second is the one that costs.

**All three hosts stay level.** A feature lands on the boundary and `viewer-ui`, `viewer-gtk` and
`viewer-qt` all adopt it, rather than one being a flagship the others follow. That is roughly three
times the host-side work per feature and it is chosen deliberately: this file's proudest claim is
that **six consumers have never asked for a new message**, and that claim is only evidence while the
consumers are actually made to carry what is added. A feature living in one host is a message nobody
has tested.

~~**`/PageLayout` is the first item**~~ **Taken in the six-hundred-and-sixth, and it did demand new
vocabulary — one `Command` and one `Answer`'s shape** (ADR 0441). All six of Table 29's values are
arranged by `viewer_core::layout`; `Command::Layout(PageLayout)` is the fourth policy value, because
the clause states the arrangement a document *opens* in and says nothing about what a reader chooses
afterwards; and `Answer::Frame` carries one entry per page on the screen, because a column has
several. `Query::PageGeometry` needed nothing at all.

**The second item is `/PageMode`'s other half, and it landed in the six-hundred-and-thirty-eighth**
(ADR 0470). Table 29's `FullScreen` — "with no menu bar, window controls, or any other window
visible" — and §12.2's `/HideToolbar`, `/HideMenubar`, `/HideWindowUI` and `/NonFullScreenPageMode`
are `viewer_host::Presenting`, and all three hosts present full screen on `p` and come back on
Escape. **It needed no new message**, which is the claim above tested for the fifth time since the
six-hundred-and-seventh: `Command::Present` had existed since ADR 0316 and `Query::Opening` and
`Query::Preferences` since the hundred-and-thirty-seventh session, each because a clause needed a
channel. What it did need was the two native hosts gaining a presentation mode they had never sent,
which is what "all three stay level" costs and is exactly what that decision is for.

**And §12.4.4.1's *clock* stopped being `viewer-ui`'s alone in the six-hundred-and-forty-second**
(ADR 0473), which is the same sentence a round later and the whole shape this decision predicts: a
window with the mode and not the clock is full screen and *static*, so `/Dur` advanced no page in
GTK or Qt and Table 164's effects did not animate. `viewer_host::Clock` is the decision — how often
to tick, what a tick carries, that the clock is held while an effect is drawn, and when `/D` has
elapsed — and the three event loops supply only the wall clock. **It needed no new message either**,
which is the sixth time since the six-hundred-and-seventh: `Command::Tick` has carried milliseconds
since ADR 0135. `viewer-ui` adopted the shared clock and lost a private type doing it, which is what
distinguishes this from a third copy.

**All three hosts have it since the six-hundred-and-seventh** (ADR 0442), which is where this
file's "all three stay level" decision stands: `viewer-gtk` draws one `gdk::MemoryTexture` per page,
`viewer-qt` one `QImage`, `viewer-ui` one frame carrying every page of the arrangement — and all
three bind `l` to `viewer_host::next_layout`, which is in that crate because the third copy of a
function is where two hosts stop agreeing. Both native hosts also gained a wheel binding they had
never had, because under `SinglePage` at `Zoom::FitPage` there is nothing to scroll.

**The route the tier-2 host took, and the argument, because 606 named two and chose neither.** Not a
display-list merge in `pdf-render`: `DisplayList` carries §11.4.7's page-group blending space and its
companion black list *per list*, so two pages stating different page groups are not one list at all;
`Command::Group` carries no transform, so placing a page inside a merged list means rewriting the
transform of every command, clip and soft mask, at every magnification; and the merged list is a new
allocation, which is the identity `render-quorra`'s retained scene and `crate::cache`'s pinned
resources are keyed on (ADR 0351). The frame carries **several placed lists** instead — which is
what `PresentFrame::overlays` already was — and trap 2 is satisfied by there being one statement of
the arrangement rather than by where the drawing happens: `viewer_core::layout` decides where the
pages go and states it as a `TargetSpec` apiece, so a backend executes an arrangement and never
chooses one.

**And it needed no message**, which is this file's own claim tested rather than repeated:
`Query::PageGeometry` answers `Answer::None` for a page the arrangement does not show, and that is
the whole of what a tier-2 host needs in order to know which of the render requests it is holding
have scrolled off.

It sharpens [`37`](37-a-frame-that-says-it-is-stale.md)'s new item rather than competing with it: a
continuous scroll reveals area that was not on the screen *constantly*, which is exactly what the
retained low-resolution page is for — and on the tier-2 host the two items are now the same item.

### What the column still owes, named rather than left implicit — 607

Three things, and none of them is architecture. **Two are taken**; the third is below and is a
change to both rasterisers.

- ~~**A selection is a range of one page's readback.**~~ **Taken in the six-hundred-and-ninth**
  (ADR 0444), by the mechanism this entry named: `Open::selection` is two `(page, offset)` ends,
  `Answer::Selected`'s `text` is a `Cow` — borrowed for a selection inside a page, which is the
  identity `selection_census` and `pdf-retrieve` both rest on, and assembled only for one that
  crosses a boundary — and five consumers failed to compile. **No host needed a line about pages**:
  each asks `Query::Selection` per repaint and draws the quadrilaterals it is handed in the
  viewport's own device pixels, so all three draw a selection over two pages without a change.
  This entry's reading of §12.4.2 was right and its conclusion was not: the standard offers no
  document-wide offset — §9.4.1 says the text position "shall not persist from one text object to
  the next" — and a *pair* of `(page, offset)` composes across a boundary where a single number
  could not exist. §12.5.6.10's mark-up over such a selection is one annotation per page, which
  §12.5.2 requires outright.
- ~~**`Query::Reports`, `Query::Readback` and `Query::AccessibilityTree` answer for the current page
  alone**~~ — **taken in the six-hundred-and-tenth** (ADR 0445), by the mechanism this entry named
  and for all three at once: each answers with one entry per page the arrangement is showing, in
  page order, and each entry says which page it is. The notes stay borrowed, so a screen's answer
  is four slices and one allocation rather than four copies of the prose. Five consumers failed to
  compile and the C ABI gained two entry points — `pdfv_reported_pages` and `pdfv_reported_page` —
  because a C caller cannot fail to compile and so has to be able to *ask* how many pages have
  anything to say.

  **This entry left one question open and the standard answered it.** §14.7's structure is a
  document's, but §14.7.5.2's marked-content identifier "uniquely identifies the marked-content
  sequence within its content stream" and §14.7.5.4 keys the route in from *the page's own*
  `/StructParents` — so two pages' trees share no numbering, and §14.8.2.5 states an order within a
  tree and none between two pages. They cross as siblings with page-local indices, and what a
  column *publishes* is AT-SPI's question: `viewer-accessibility` gives each page a
  `Role::Document` node of its own with its own identifier band, its own extents and its own status
  group, and an untagged page in a mixed column keeps the one sentence saying the document states
  no structure. Read back off a real bus.
- ~~**The gap between pages is invisible on `viewer-ui`**~~ — **taken in the six-hundred-and-eleventh**
  (ADR 0446), and this entry was wrong about one thing in a way worth keeping. The reading it
  recorded holds: §11.4.7's 𝑊 is Table 141's "[i]nitial colour of the page", so it is a property of
  **the page** and stops at §14.11.2.1's crop box, while what a window shows where there is *no*
  page is no clause's subject — searched across §11.3, §11.4.5, §11.4.7, Table 141, §14.11.2 and
  Table 147, with `spec-errata`'s `emit` reporting nothing in clause 11. `pdf_render::medium` is the
  pair now, `page_area` is where 𝑊 stops, and `impose_within` is the composite all three rasterisers
  end with, so trap 2's rule holds. Nothing moved on any gate: `examples/raster_digest` is new and
  says so — 957 corpus first pages byte-identical, because a page-sized target asks one colour on
  both sides of a boundary.

  **What this entry got wrong was "[t]he two native hosts never had it".** That was an inference
  about somebody else's defaults, not an observation, and looking at the screen refuted it: GTK's
  Adwaita background and Qt's palette are both within a few levels of paper white, so the gap was a
  hairline in both. Both hosts take `pdf_render::SURROUND` now — GTK through a `CssProvider` rule,
  Qt through `PageArea`'s palette with the value crossing the `cxx` bridge — because a toolkit has
  no notion of *the surface a document is laid on* and so offers no platform value to inherit.

- ~~**§14.11.2.1's clip is not applied on a window-sized target**~~ — **taken in the
  six-hundred-and-twelfth** (ADR 0447), and this entry was right about where it could go and wrong
  about the population it named. The clip is stated once on the display list —
  `DisplayList::content_clip`, carrying §12.2's `/ViewClip` boundary rather than the crop box by
  name — mapped into a target by `pdf_render::crop_area` and applied by `crop_to_page`, which every
  rasteriser runs immediately before `impose_within`. `render-quorra`'s window path took the outer
  clip chain this entry predicted, for the reason the medium already needed one: a frame drawn onto a
  swapchain has no raster afterwards to cut. `Interpreter::view_clip` and its `Option<ClipId>` are
  gone, because one rectangle says what the chain said for every document rather than for the zero
  that state the preference.

  **The population is measured and it is not the one this entry implied.** `examples/crop_box_census`
  over 66 887 first pages of the pdf.js corpus, `doc/corpora` and the whole crawl: **1121 documents
  state a crop box smaller than their medium and 3690 actually mark outside the boundary**, with only
  804 in both sets — 2886 of the 3690 crop to the medium and draw beyond it anyway. Counting the
  structural condition would have named the wrong documents in both directions, which is trap 11's
  own shape.

  **And the arithmetic was the finding.** §10.7.4 makes a clipping region "the set of pixels that
  would be included by a fill operation", and a fill paints "any pixel whose half-open square region
  intersects the shape, no matter how small the intersection is" with the painted area "always at
  least as large as the area of the original shape". A fractional boundary pixel moved 37 of 957
  corpus first pages and §10.7.4's intersection moved 11; the clause's own set rule moves none.
  Trap 14 is what the whole item turned out to be an instance of.

## The order UI work is taken in, and the criterion — 678, the first round on the owner's sentence

**ADR 0509 is the argument; this is the list.** It replaces "pick something from the gaps above"
with a ranking, because a list is taken in whatever order somebody wrote it down — which is how the
`/TI` entry above sat behind a diagnosis nobody re-checked for seventy-seven sessions. The
criterion, in order: what a *reader* can do with a document and cannot do here; then what costs no
new message; then what makes the level-hosts decision checkable. A toolkit floor does not rank an
item last, but it does oblige a round to say what it actually tried.

**Four consumers, and they are not four of a kind**, which is why the decision above cannot mean
one thing for all of them: `viewer-ui` is tier 2 and draws its own chrome, the two native hosts
place somebody else's widgets, and `viewer-ffi` draws nothing and hands a caller data. Today
"level" is false in *both* directions — `viewer-ui` is ahead on everything that reads a document
(§12.3.5's collection, §12.5.6.14's popup windows, the caret) and behind on everything that changes
one (no native controls; **the password prompt came off that
list in the six-hundred-and-ninety-fifth**, ADR 0545). **The markup keys, the copy key and the undo
binding came off that list in the six-hundred-and-eighty-seventh** (ADR 0526): they are rows of one
key table now, so being ahead or behind on a *binding* is no longer a thing a host can be. **And the
six sidebar tabs came off it in the seven-hundred-and-fourth** (ADR 0564), by the same mechanism one
panel over: the list of panels is `viewer_host::Tab` and every host matches exhaustively on it, so
being ahead or behind on a *panel* is no longer a thing a host can be either — which left
§12.3.5's collection and §12.5.6.14's popups, neither of which is a tab, as the whole of the first
half of that sentence. **And §12.5.6.14's window came off it in the seven-hundred-and-twenty-sixth**
(ADR 0613), by the mechanism one layer down from a tab: `viewer_host::popup` is the reading and each
host places its own furniture — so §12.3.5's collection is what is left of that half.

1. ~~**A selection that can leave the program.**~~ **Taken in the six-hundred-and-eighty-third**
   (ADR 0519), and the claim it was ranked on held: **no new message**, checked rather than assumed.
   `viewer_host::copying` is the §14.8.2.5 choice all four consumers now make once —
   `Query::Selection`'s page content order, or `Query::LogicalSelection`'s where the structure tree
   reaches every byte of the selection — and each host supplies only its platform:
   `gdk::Clipboard`, `QClipboard`, `arboard` for the tier-2 host, and `pdfv_selection_copy_text`
   for a C caller. Driven under `Xvfb` and read back with `xclip`: all four put **byte-identical**
   text on the X11 `CLIPBOARD` selection for `PDF20_AN001-BPC.pdf`, in logical content order, and
   the C caller reports `PDFV_ORDER_LOGICAL` for it.

   **Two things this entry was wrong about, and one shape worth keeping.** *"The C ABI has
   `pdfv_selection_text` already"* (ADR 0509 §3) was true and not sufficient: that entry point
   answers in page content order and `Query::LogicalSelection` reached no symbol at all, so the
   fourth consumer could copy and could not copy *right* — a fifth of item 5 came with item 1
   because item 1 is not finished without it. And `viewer-qt` could not call `QClipboard` from Rust,
   which is this tree's own shape rather than Qt's: `crate::bridge` states that C++ owns the `Host`
   for the life of `QApplication::exec` and Rust never calls a Qt object, so the copy goes out as a
   `QtUpdate` flag with a getter beside it — the same shape `window` has since ADR 0470, and chosen
   over adding a second declaration to the one hand-written `unsafe extern "C++"` block.

   **The remaining asymmetry is a platform's and is named here rather than left silent**: Ctrl + C
   inside a §12.7 *field* is the toolkit's own binding in the two native hosts, because they place a
   real `GtkEntry` and a real `QLineEdit`; `viewer-ui` draws its own field and now makes the same
   call the page's copy makes. The C ABI has no field-level copy and needs none — a caller that
   placed its own controls owns their keyboard, and `pdfv_field_text` answers with the value.
2. ~~**A statement of what a key means, in `viewer-host`.**~~ **Taken in the
   six-hundred-and-eighty-seventh** (ADR 0526), and the three disagreements it named were all real:
   `f` was the find bar in GTK and a free-text drag in `viewer-ui`, the arrow keys scrolled in one
   host and turned the page in two, and Escape cleared the selection in the native hosts and *quit*
   `viewer-ui`. `viewer_host::keys` is the one table now — a closed `Key`, a `Meaning` that is
   either a `Command` or a `WindowAct`, and a `Mode` on the lookup because two rows depend on
   whether §12.4.4's presentation is running. **It needed no message**, which is the eighth time
   since the six-hundred-and-seventh.

   **This entry said "[f]our key tables" and there were three.** `viewer-ffi` has no keyboard and
   never had one — `include/pdf_viewer.h` says so where it mentions the only key the standard names,
   *"§12.5.1's tab key. The order is the document's (Table 31's `/Tabs`); the key is yours"* —
   because a C caller places its own toolkit and owns its own keyboard entirely. The fourth consumer
   is a different kind of thing here rather than a host that is behind, which is ADR 0509 §2's own
   finding turned on its own list.

   **What the standard decides is two rows and it decided the sharpest disagreement.** §12.4.4.2
   gives "[p]ressing an arrow key" as its EXAMPLE of a navigation request and asks for one "such as
   an arrow key press" — *inside the presentation subclause*, which is why all four arrows navigate
   while a presentation is running and Up and Down move the view while one is not. It also fixed a
   gap in the other direction: `viewer-ui`'s arrows scrolled *during* a presentation, so two of the
   four keys the clause names made no navigation request at all on the tier-2 host. §12.5.1's tab
   key is the other row, bound in one host of three before and in all three now. And §7.7.2's
   Table 29 takes a binding *away*: while a presentation is running the four keys that ask for
   chrome — `f`, `/`, `o`, `?` — mean nothing, which is "no … other window visible" applied to the
   keyboard rather than to the widgets.

   **The instrument is what ADR 0509's third criterion asked for.** Each host carries
   `every_key_the_table_states_has_one_in_this_toolkit`, which walks `Key::ALL` through a match that
   is exhaustive over the enumeration — so a binding added to `viewer-host` fails to compile in all
   three hosts — and then asserts that the host's runtime translation agrees, so a key named in the
   test and forgotten in the translation fails rather than drifting.

   **And it found a licence obligation nobody had noticed.** `?` had to mean something in all three,
   and what it means in `viewer-ui` is the card of third-party notices that exists because both
   licences covering the compiled-in standard 14 font programs (§9.6.2.2) require a *binary*
   distribution to reproduce their notices. `pdf-viewer-gtk` and `pdf-viewer-qt` ship the same font
   programs and reproduced them nowhere at all — no card, no dialog, not even a `--licences` flag.
   `viewer_host::NOTICE` is the one text and all three hosts now put it on the screen.

   **And asking what `?` should do found a defect that was not a key's.** In `viewer-ui` the key set
   the flag, the card's display list was built for the frame, and the window kept the pixels it
   already had — because `surface.rs`'s "a rendering of exactly these lists at exactly these targets
   needs no successor" compared the *pages* and not the chrome drawn over them. Every overlay that
   host draws was unreachable on the graphics device's path: the find bar, the panel, §12.5.1's
   ring, the selection and the notices card. Fixed here, measured A/B in one sitting: **zero frames
   after ten seconds idle on both builds**, and 14 frames against 28 for six chrome-changing key
   presses, which is one render and one present per change.

   **Two asymmetries remain and both are named rather than left silent.** §12.5.6.6's free-text drag
   is `t` in the table and is **refused by name** in the two native hosts, because authoring that
   annotation is a drag mode plus an editor and both are `doc/todo/33`'s. And on a *delegated* form a
   real `GtkEntry` or `QLineEdit` has the focus, so the toolkit's own traversal takes Tab before any
   window controller sees it — what is walked there is the toolkit's order rather than Table 31's
   `/Tabs`, which is a platform's behaviour and not something this tree can take back.
3. ~~**`viewer-ui`'s password prompt**~~ — **taken in the six-hundred-and-ninety-fifth**
   (ADR 0545), and **it needed no message**, which is the ninth time since the
   six-hundred-and-seventh. `viewer_ui::chrome::PasswordCard` is the tier-2 counterpart of
   `gtk4::PasswordEntry` and a `QLineEdit` at `QLineEdit::Password`, and `viewer_host::password` is
   the *policy* all three now share: the attempts, the two sentences, and one format string for the
   question each of them used to build for itself.

   **Three things this entry did not predict, and each is worth more than the item was.**

   The clause's modal verb is `should` and both native hosts quoted it as `shall`, in quotation
   marks, with four words added — *"the interactive PDF processor shall … prompt the user for a
   password"*, a sentence ISO 32000-2 does not contain. The quotation gate could not see it because
   it reads rustdoc blockquotes and these were `//` comments, which is a limit of the instrument
   worth knowing. And §7.6.4.1's NOTE 2 is what makes `exit(1)` the *wrong reading* rather than
   merely awkward: it describes the processor that genuinely cannot ask as "non-interactive PDF
   readers that do not have a person running them such as printing off-line or on a server", and a
   window on a screen is not one whatever it was launched from.

   **The password was one struct field's declaration order away from a launch log.** `Command`
   derives `Debug`, `viewer-gtk` and `viewer-qt` trace a command with `format!("{command:?}")`
   truncated to 120 characters, and `bytes` happens to be declared before `password` — so a
   `Vec<u8>`'s five-characters-a-byte `Debug` cut the line before the secret. `viewer_core::Secret`
   is the type now: it prints how many characters it holds and not which, has no `Display`, zeroes
   its buffer on drop, and reserves §7.6.4.1's own 127-byte truncation point so that a password the
   standard reads whole never reallocates. Five consumers failed to compile, which is the fifth use
   of that mechanism.

   **And a piece of chrome had a third way not to reach the screen**, one round after 687 found the
   first two. `App::present` began `let first = pages.first()?;`, so a window with **no page** drew
   no frame at all — and a document that has not authenticated is exactly that window.
   `Surface::without_a_page` draws the chrome over `pdf_render::SURROUND` on both surfaces.
4. ~~**§12.3.4's thumbnails, §12.4.3's articles and §14.3.3's properties in the two native hosts.**~~
   **Taken in the seven-hundred-and-fourth** (ADR 0564), and **it needed no message**, which is the
   tenth time since the six-hundred-and-seventh. `viewer_host::Tab` is the deliverable rather than
   the three panels: a closed list of the six this program shows, with Table 29's `/PageMode`
   mapping on it, matched exhaustively in all three hosts — so a seventh panel fails to compile in
   three places, which is ADR 0526's key mechanism applied to the other thing a window shows.
   `viewer-ui` lost `chrome::Tab` adopting it, and `article_rows` and `property_rows` with it.

   **§12.3.4 is the one answer that is not a row**, and the split it forced is where this crate's
   "no widget and no pixel format" line actually falls: `page_entry` and `Miniatures<T>` are shared —
   the two queries a row needs and the *policy* of decoding on demand, keeping what is near and
   dropping what is far — while the picture stays a `gdk::Texture`, a `QPixmap` or a
   `pdf_render::Image`.

   **And the host that was *ahead* held the clause violation.** `viewer-ui` built the whole page
   list the first time its tab was shown, and Table 29's `/PageMode /UseThumbs` opens that tab as
   the document opens — so a thousand-page document stating it spent **121 ms of a 156 ms first
   present** decoding miniatures for rows nobody was looking at, which is `CLAUDE.md` section 2's
   forbidden thumbnail generation on the launch path reached by a road nobody had checked. Eight
   rows in 0.3 ms and a 48 ms first present after. The two native hosts never had it, because a
   `GtkListView` and a `QAbstractListModel` are virtual by construction — **writing the same panel
   twice in toolkits that are lazy is what showed that the host drawing its own rows was not.**

   **Two things only the screen could say.** GTK binds a row *synchronously* when the list is put
   into a realised window, which happens inside a call that holds the host borrowed — caught by trap
   5's own note rather than by a blank panel, and fixed with an idle. And six tab labels do not fit
   across a sidebar in either toolkit, so both put their tabs down the side; a notebook that cannot
   fit its tabs hides the rest behind an arrow nobody looks for.

   **What is still `viewer-ui`'s alone is named in ADR 0564 §7**: §12.3.5's collection and
   §12.5.6.14's popup windows, neither of which is a *tab* — **and the second of those two was taken
   in the seven-hundred-and-twenty-sixth** (ADR 0613), leaving the collection. ~~Nothing counts what a window cannot
   reach, the way `tools/state.sh hosts` counts what a C caller cannot.~~ **`tools/state.sh windows`
   counts it since the seven-hundred-and-ninth** (ADR 0577), per host and for both enums, and names
   any variant no window reaches at all.

   **Its first run was wrong, and how it was wrong is worth more than the section.** It reported both
   native hosts reaching §12.3.5's collection, on the evidence of one doc comment in
   `viewer-host/src/panel.rs` that says in so many words *"a different answer … that this host does
   not yet ask"*. A count whose condition was "the name occurs in the crate" reported the opposite of
   what the sentence said, four words later — trap 11 caught in the act. Both sections strip comments
   before matching now, through one shared helper, and `state.sh hosts` had the same latent flaw.

   **And a zero is not automatically a debt**, which the section's own note says: all three windows
   learn about an edit from `Event::Dirty` and none asks `Query::Dirty`; a tier-2 host never asks
   `Query::Frame` because it draws its own pixels; and most of what the native hosts do not ask is a
   *delegation* — a real `GtkEntry` owns its own caret. What is left after that reading is the short
   list the section exists to keep visible.

   **And the residue item 3 named was taken with it.** `Event::OpenFailed` and a page tree with no
   leaves both called `std::process::exit(1)` in `viewer-ui`; `viewer_host::cannot_open` and
   `no_pages` are the two sentences and `viewer_ui::chrome::Refusal` is the card. **Neither native
   host said anything about a document with no pages either**, which is the half nobody had noticed:
   §7.7.3.2 states no floor on `/Count`, so such a document is *correctly read* and has nothing to
   show, and a blank window looks exactly like a broken file.
5. ~~**The C ABI's other half**~~ — **taken in the seven-hundred-and-ninth** (ADR 0576), and **it
   needed no message**, which is the eleventh time since the six-hundred-and-seventh. Every one of
   `Query`'s variants reaches a symbol now, and `PDFV_ABI_VERSION` did **not** move for the largest
   addition this ABI has had: not one of the entry points takes or returns a struct by value, which
   is the one kind of change that constant exists to catch.

   **The deliverable is the instrument rather than the eleven**, and the round's own reasoning turns
   on it: `PDFV_EVENT_KIND_COUNT` is the right protection for a message that *arrives* and no
   protection at all for a *question*, which is exactly how eleven accumulated in silence.
   `tests/every_query_reaches_the_abi.rs` matches exhaustively over `Query`, so a question added to
   the boundary fails to compile in a test whose name says what it is for — and it has **no
   allow-list**, which is why all eleven had to land at once: a test that permits "this variant
   reaches nothing" is the drift it was built against, wearing a comment. The enumeration's size is
   counted out of `viewer-core`'s own source rather than written down, and all three of its
   assertions were run against injected defects before being believed (trap 13).

   **Six of the eleven needed no new shape and two cost one `pub` between them**: §12.4.3's threads
   and §14.3.3's properties are `viewer_host::article_rows` and `property_rows` already, so they
   cross as the panel handle this ABI has had since ADR 0346 — ADR 0246 decision 3 holding for a
   third kind of host. The five that needed a handle are in `crates/viewer-ffi/src/answers.rs` with
   the argument for each shape on the type, because a C entry point cannot change shape once a
   caller exists. Table 147 is a **keyed accessor** rather than a struct or nineteen symbols, for
   the reason the header gives: a struct by value would put that table's *size* in the ABI.

   **§12.3.4 is the one that had to be designed against a defect rather than for a feature.** There
   is deliberately no `pdfv_thumbnails_read`, and `pdfv_page_label` is a separate call, so that the
   seven-hundred-and-fourth session's launch-path defect has no road into a C host. Measured from a
   C program outside this tree against the installed `libviewer_ffi.so`, on a 233-page document
   carrying 231 miniatures: eight rows cost 0.81 ms and 210 KiB, every page 21.6 ms and 6.95 MiB.

   **What it did not carry, named rather than left silent**: `AccessibilityNode::lines` — the
   per-character byte counts and boxes AT-SPI's `Text` interface wants. An element's own text is its
   `PDFV_ELEMENT_NAME`, so a C caller building a screen reader has the tree and the extents and not
   the character offsets. Two accessors and no new decision.

   **And one addition item 2 declined to make here, written down so it is decided rather than
   rediscovered**: `viewer_host::keys` is a *table*, and a C host that wants the keys this program
   binds has to re-derive all thirty of them. `pdfv_key_meaning(key, shift, presenting)` with a
   `PDFV_KEY_*` enumeration would hand it over, and the same test shape applies — a count beside the
   enumeration, because a C caller cannot fail to compile. It was not taken in the
   six-hundred-and-eighty-seventh because it is an addition to the ABI's *surface* and this is where
   surface is decided; nothing in item 2 is blocked on it, since a C caller owns its keyboard by
   construction and so has no table to be out of step with.
6. ~~**Undo and redo in `viewer-ui`**~~ — **taken with item 2 in the six-hundred-and-eighty-seventh**,
   which is what ADR 0509 §3 predicted of it ("it is a keyboard binding once item 2 exists") and the
   whole reason for stating an order. `z` and `y` are rows of `viewer_host::keys`, so the tier-2 host
   got them by adopting the table rather than by a round of its own; it gained `w` the same way, and
   lost Escape-quits.
7. ~~**Table 233 bit 19's editable combo box in `viewer-gtk`** — the one item here that is a genuine
   toolkit floor.~~ **Taken in the seven-hundred-and-seventeenth** (ADR 0596), and **it was not a
   floor**, which is ADR 0508's rule paying for the second time on this one clause. The block was
   read off the *widget list* — `GtkDropDown` has no entry and `GtkComboBoxText` is deprecated in
   the release this crate binds, both true — and Table 233 bit 19 does not ask for a widget. It
   asks for "an editable text box as well as a drop-down list", which a `GtkEntry` beside a
   `GtkMenuButton` over a `GtkListBox` is, in one `linked` box, with nothing deprecated and **the
   `v4_10` feature floor untouched**. It needed no message, which is the twelfth time since the
   six-hundred-and-seventh.

   **The half of the flag nobody had read is worth more than the half this entry named**, and the
   host it caught was the one that is *ahead*. The bit's second clause — if clear, the combo box
   "shall include only a drop-down list" — is a `shall` in the other direction, and `viewer-ui`
   broke it: `Answer::Field` answers a combo box with characters whether or not the flag is set,
   correctly, because the value *is* text and §12.7.4.3 lays it out — and that host read *has a
   text value* as *takes typed characters*. So a person could type *Purple* into a drop-down whose
   options are Red and Blue and the file took it. This is Table 229 bit 26's shape one flag over
   (ADR 0346): a bit reported as unimplemented in its set direction while its clear direction was
   being broken in silence, because a flag reads as a permission and half of these are
   prohibitions. `viewer_host::form::ControlKind::takes_typed_characters` is the one statement now,
   exhaustive over the enumeration.

   **And the tier-2 host could not choose an option at all, in either of §12.7.5.4's two
   controls.** It sends no `Command::Delegate`, so no `GtkDropDown` was ever placed over its page,
   and `Entered::Chosen` — the variant `Edit::SetField` grew in the four-hundred-and-twelfth
   session so that a list box could say which options are selected (ADR 0248) — occurred nowhere in
   `viewer-ui`. A list box drew its options and no press could select one; a combo box could only be
   given a value by typing a label, which is the violation above. `viewer_ui::chrome::ChoiceList`
   is the drop-down *drawn* rather than placed, with one layout answering both where the rows are
   and which option a press landed on.

   What it deliberately did not do: give that list a keyboard. Up, Down and Enter would be this
   host's convention and no clause states one.

**That is all seven, and the ordering is spent.** What a next UI round has to choose from is named
here rather than left to be re-surveyed, and none of it is architecture:

- ~~**`AccessibilityNode::lines` does not cross the C ABI**~~ — **taken in the
  seven-hundred-and-twenty-sixth** (ADR 0613), and **"[t]wo accessors and no new decision" was wrong
  about the first half and right about the second.** It is **three** entry points, and the reason is
  this ABI's own convention rather than an oversight: a count is asked before an indexed accessor,
  and a line has two counts — how many lines an element drew and how many character codes a line
  holds. `pdfv_structure_lines`, `pdfv_structure_line` (the text *and* the code count in one call,
  because the byte counts summing to the text's length is the invariant a text interface rests on and
  a caller asking twice could see them disagree) and `pdfv_structure_character`.

  **The decision the entry did not predict is which text crosses.** These are the readback and not
  §14.9's substitutions: `PDFV_ELEMENT_NAME` applies `/Alt` and `/E`, and a caret moves over what is
  on the page — `GetCharacterExtents` asks where the *glyph* is, and a phrase substituted for an
  element's content has none. An element stating one has zero lines here, which is what
  `pdfv_structure_node`'s `substituted` says from the other side. `PDFV_ABI_VERSION` did not move:
  no struct crosses by value.
- ~~**`tools/state.sh windows` prints eleven queries each native host does not ask, and the list is
  uninterpreted.**~~ **Sorted in the seven-hundred-and-twenty-first** (ADR 0603), and **the list
  could not be sorted as it stood**, which is the finding rather than the sort. Two of its entries
  were the instrument's own: `Command::[A-Za-z]+` with no word boundary matched the tail of
  `PathCommand::Close` — `pdf_render`'s *path* close, which `viewer-ui` writes on every rounded
  rectangle of its chrome — and `viewer-ui`'s trace formatter matches `Command` exhaustively in
  order to *print* a name, which `section_hosts`' own comment gives as its reason for asking
  `viewer-ffi` alone and which `section_windows` was then built over anyway. So the section reported
  `viewer-ui reaches 25 of 25` and `every Command reaches at least one window`, and both were false.

  **The reading now lives in the script, under the counts**, one line per unreached variant saying
  *debt* or *not a debt* and why — because no command can decide whether a `GtkEntry` owning its own
  caret is a gap or a delegation, and a count printed without that decision is what two rounds read
  as "eleven queries" and walked past. It is checked in both directions: a variant with no reason
  prints `UNREAD`, and a reason for a variant every window now reaches prints `SPENT`.

  **What the reading found, in the row nobody had been asked to read**, is `doc/todo/38`'s and
  `CLAUDE.md`'s: `Command::Restrict` reached one window of three, so a document's restrictions could
  not be turned off in either native host — while both of them answered every refusal with a
  sentence naming `--ignore-restrictions`, a word their own argument parsers rejected. Closed in the
  same round (ADR 0604), which is why it is no longer on any missing list.

  **Five debts were left and they ranked**, and **two of them were taken in the
  seven-hundred-and-twenty-sixth** (ADR 0613) in the order ADR 0509's criterion puts them:
  §12.5.6.14's popup windows, which two windows of three drew nothing of — so a comment on a page was
  invisible in two programs of three — and §12.5.6.5's link cursor, which one host set and two did
  not, so a reader could not see that a link was there until after clicking it. **It needed no
  message**, which is the thirteenth time since the six-hundred-and-seventh: `Query::Popups` has
  existed since ADR 0191 and `Query::LinkAt` since the vocabulary was frozen, and what was missing
  was consumers.

  **`tools/state.sh windows` said so before this file did**, which is what ADR 0603's second
  direction is for: both rows printed `SPENT` the moment the hosts reached the variants, and the
  reasons are deleted rather than left to be read as debts. **And one of them was wrong about its own
  population** — it said "[s]even of the corpus's documents state an open one" where the measurement
  is seven open popups on **two** documents, `issue14438.pdf` with six and `pr7352.pdf` with one. The
  durable fix is in the instrument rather than in the sentence: `examples/open_annotation_census`
  counted popups stating Table 186's `/Open true` into its totals and named no document holding one,
  so a round wanting to *look* at an open window had a number and no file.

  **Three were left after that round, and the largest was taken in the
  seven-hundred-and-thirty-first** (ADR 0623): §14.7's accessibility tree with §9.10.2's readback
  beside it, which reached one window of three — so a screen reader on either native host was handed
  a picture. **It needed no message**, which is the fourteenth time since the six-hundred-and-seventh:
  `Query::AccessibilityTree` has answered since ADR 0134 and `Query::Readback` since ADR 0422, and
  what was missing was consumers. What it *did* need was a decision this file could not make for
  itself — whether a native host publishes through its own toolkit's accessibility layer or drives
  AccessKit as `viewer-ui` does — and the argument that settled it is the standard's rather than
  either toolkit's: §14.7.3's role map is a `shall` on this reader, and mapping §14.8.4's forty-one
  types onto a platform vocabulary **twice, differently in each toolkit**, is exactly what "all three
  hosts stay level" forbids in the one place a person cannot check for themselves.

  **What it cost is a second application root on the accessibility desktop** — `accesskit_unix`
  embeds its own — measured with `busctl` rather than predicted, and written down as the price of the
  decision rather than found later.

  **And it refuted a row of this section's own reading.** `Query::FieldAt` was "not a debt, a
  delegation: these hosts place one control per widget, so which field a press belongs to is the
  control it landed on". An assistive technology's press lands on no control — it arrives as a point
  on the page — so a click on a §12.7 widget did nothing at all in both native hosts while answering
  `true`. It was refused by name, the row prints `SPENT`, and **the other half was built in the
  seven-hundred-and-thirty-fifth** (ADR 0630): `viewer_host::form::clicked` performs the click
  against the *field* rather than against the control, which is where a value belongs in every host
  anyway, and `Clicked` is matched exhaustively in three windows. What stays refused is the click a
  delegated `GtkEntry` or `QComboBox` would have taken — a caret at a point, or Table 234's options
  on the screen — which is `Clicked::Aimed` and is the same item as `Action::Focus`.

  **And the round that built it found the shape this section exists to prevent, already sprung.**
  §12.7.5.2's rule was written three times — once per window — and the three had stopped agreeing:
  only `viewer-ui` asked Table 227 bit 1 before sending an edit, because the other two were relying
  on a control the toolkit had disabled, which is a fact about a *person's* click and not about the
  two other ways one arrives. One function in `viewer-host` now, which is this file's own answer.

  **The instrument was wrong for one run and its own second direction said so.** Moving the six
  queries into `viewer_accessibility::Reading` — so that three windows publish one tree rather than
  three — made `state.sh windows` report `viewer-ui` reaching *fewer* queries than before, with
  `AccessibilityTree` credited to no window on the day all three began asking. Its population was the
  host crates plus `viewer-host`, and the crate a host's non-toolkit half had just moved into was not
  in it. Trap 11, caught by the `SPENT` check ADR 0603 added for exactly this.

  **What is left is two, and `tools/state.sh windows` prints them with their reasons**: §12.3.5's
  collection, whose `shall` is addressed to a viewer outright and which no corpus document states, and
  which is now the largest; and §12.5.6.6's free text, which is already refused **by name** and is
  `doc/todo/33`'s.

  **What the popup cost that this entry could not have predicted is a trap** (19, and ADR 0613 §1). A
  `GtkFixed` measures the union of its children and a popup's `/Rect` is the *document's*:
  `issue14438.pdf` states six open windows beside its page, so placing them in the layer the page is
  in let the file decide how wide the window was — 509 to 1229 device pixels in nine frames, measured
  off `--trace`, with nothing on the screen looking wrong. The windows are an unmeasured `GtkOverlay`
  child now, and Qt's `PageArea` has no layout at all. What is shared is `viewer_host::popup` — the
  title bar's two texts, the body, the upright box and the one refusal for a window with no area —
  which `viewer-ui` adopted, losing three private derivations and gaining the rotation correction its
  own arithmetic did not have.

A third thing is worth writing down because this round nearly rediscovered it: **the criterion in
ADR 0509 outlives its list.** What a reader can do and cannot do here, then what costs no new
message, then what makes the level-hosts decision checkable — and a toolkit block is a claim to
check rather than a rank. **The seven-hundred-and-twenty-first session declined to write a second
numbered list and said why** (ADR 0603 §5): what the first one bought was three rounds not spending
themselves on a survey each, and the survey is now a command that reruns itself with its reasons
attached. A ranking that a script prints beside the count it ranks cannot go stale between rounds
the way a list in a file did.

**Two places the API forces a host into an awkward shape**, neither an argument for changing the
vocabulary today. The per-page answers are two shapes — `Reports`, `Readback`, `Accessibility` and
`Frame` name a page apiece, while `Fields`, `Popups` and a selection's quadrilaterals are flat
lists — and the enum does not say which rule it follows, which is how `Query::Fields`' own
documentation came to say "the page being shown" for rounds after the code walked the whole
arrangement (corrected in 678). And `Command::Delegate` is a policy about a *document* while
`Query::Fields` answers about a *screen*: the pair is safe only while both follow the arrangement,
and if they ever disagree the result is a form with holes and no report anywhere.

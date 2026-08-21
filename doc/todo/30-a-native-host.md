# A native host, then the C ABI

Status: **all three built.** GTK4 in the four-hundred-and-eighth session (`crates/viewer-gtk`,
ADR 0244); Qt in the four-hundred-and-tenth (`crates/viewer-qt`, ADR 0246), and with it
`crates/viewer-host`; **the C ABI in the four-hundred-and-eleventh** (`crates/viewer-ffi`,
ADR 0247), with its three amendments taken first. **This file absorbed `doc/todo/37` in the
four-hundred-and-ninth**, whose one open decision was taken (ADR 0245). **And its remaining surface
was taken in the five-hundred-and-eleventh** (ADR 0346): the ABI's entry points are the whole
vocabulary now, Table 229 bit 26 is obeyed, and ADR 0245's scale question is answered with the
messages that already existed.
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

- **`viewer-gtk` does not obey Table 234's `/TI`.** The entry is "the index in the Opt array of the
  first option visible in the list"; `pdf-model` has read it since the
  three-hundred-and-ninety-eighth session, the page's own appearance obeys it since ADR 0407, and
  `viewer_host::form::ControlKind::List` dropped it until the six-hundred-and-first — so every
  native host's list box started at row 0 over a picture that started somewhere else
  (`doc/habits.md`'s fifth sweep: *the model implements this, who calls it?*). `viewer-qt` obeys it
  now with `scrollToItem(..., PositionAtTop)`. GTK's half is a *floor* rather than a decision:
  `GtkListView::scroll_to` is GTK 4.12 and this crate binds `v4_10` (`Cargo.toml`), so it means
  raising the feature or driving `GtkListBase`'s `list.scroll-to-item` action on a view that is not
  yet in a window — and the second wants a witness, because a document whose `/TI` is not 0 is what
  says the action fired. No corpus document is known to state one; `examples/variable_text_census`
  counts 10 list-box widgets and is where to look first.

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
**one** hand-written token; and `viewer-ffi`, whose `src/abi.rs` holds one lint lift, 111
`#[unsafe(no_mangle)]` attributes and 103 signatures, and **no `unsafe` block at all**. Each has a
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

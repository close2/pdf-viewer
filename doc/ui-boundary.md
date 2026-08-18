# The UI boundary — `viewer-core`, its vocabulary, and the three pixel tiers

Status: **built, shaken out, and frozen** — six consumers on it, **three of them hosts somebody
else's widgets sit on**: GTK4 since the four-hundred-and-eighth session (`crates/viewer-gtk`, ADR
0244), Qt 6 through a C++ bridge since the four-hundred-and-tenth (`crates/viewer-qt`, ADR 0246),
and **a C ABI since the four-hundred-and-eleventh** (`crates/viewer-ffi`, ADR 0247). One is behind
a confinement. `doc/todo/30`'s condition — *"do not freeze a C ABI until two Rust consumers have
shaken the API out"* — was met, the three amendments it named were taken, and **no host added a
message, three running**. **One variant changed shape in the four-hundred-and-twelfth**, which is
the other mechanism and not the same thing: `Edit::SetField`'s value became
`pdf_model::view::Entered` so that §12.7.5.4's list box could say which of Table 234's options are
selected (ADR 0248). **And the four-hundred-and-fourteenth added a `Command` *and* an `Event`, the
first `Event` since the vocabulary was frozen**: `Command::Find` and `Event::Searched` are Annex O's
document-wide `search`, which `Query::Find` is not — that one answers for the page showing, out of a
readback that exists, and this one interprets pages nobody is looking at. Six consumers failed to
compile and `PDFV_EVENT_KIND_COUNT` moved 15 → **16** for the first time, which is what a C caller's
`pdfv_abi_check` is for (ADR 0250). **And the four-hundred-and-eighty-first added a `Command` and no
`Event`**: `Command::Present(PresentationMode)` is §12.4.4.2's own condition, which that clause
requires and no state machine over a file can deduce (ADR 0316). Two consumers failed to compile —
`viewer-confined`'s wire protocol and `viewer-ui`'s trace line, the two that match `Command`
exhaustively — and the C ABI did not, because commands there are *functions* and
`PDFV_EVENT_KIND_COUNT` stayed where it was. **And the five-hundred-and-twenty-second added a
`Query` and an `Answer`**: `Query::Highlight` is Annex O's `highlight` parameter, whose rectangle a
host cannot place for itself because no host sees the fragment — it arrives inside `Command::Open`
undecoded — and whose *look* the annex hands to a processor outright ("[t]he nature of the
highlighting is implementation-dependent"), which is this boundary's own rule stated by the
standard. Three consumers failed to compile and the C ABI gained its hundred-and-twelfth entry
point (ADR 0357).
Read by: anybody writing a host, adding a `Command`, `Event` or `Query`, or asking what the
crate boundary permits. `doc/HANDOVER.md`'s reader table points a round writing a host here, and ADRs 0116 to 0121
are the argument.

This was `doc/HANDOVER.md`'s section 0 until the three-hundred-and-ninety-fifth session moved it
here whole: it is the specification of an interface, which is read when the interface is being
used and not every round.

**Everything a viewer still owes was blocked on one missing interface.** Since the
hundred-and-thirty-first session that interface is code — `crates/viewer-core`, ADRs 0116 to
0121 — with `viewer-ui` on it as a tier-2 host and `tests/headless.rs` driving it with no display
at all. This section is now half description and half instruction: read the first half to know
what is there, the second to know what is next.

#### Why it was the headline

Five owed items were the same item, and **four of them are done**: a password prompt, which this
file called "the missing piece, not the clause" for twenty sessions and which session 132 landed
in eleven lines of host code; an editable field (sessions 135 and 136); the layer panel, whose
data `Query::Layers` has answered with since 131 and which session 167 drew; and presentation
mode's clock (`Command::Tick`, 150). **And the fifth, AccessKit, landed in the
three-hundred-and-seventy-sixth** — `crates/viewer-accessibility`, verified by reading the tree
back off a real AT-SPI bus (ADR 0214). All five are done; what §0 still owes is elsewhere.

#### The goal, stated by the owner

The viewer is to be **embeddable in native frameworks** — Win32/WinUI, AppKit, KDE/Qt, GTK — not
built on a cross-platform toolkit. Later it must support **text selection** and **annotation
editing**, possibly form-field text editing. `CLAUDE.md`'s exclusion list was amended in the
hundred-and-thirtieth session to permit the writing that implies.

#### What exists

Six consumers: `viewer-ui`'s `pdf-viewer.rs` (winit + vello, tier 2),
`viewer-core/tests/headless.rs` (no display at all, tier 1), `viewer-confined`'s `pdf-view-worker`
(a process with no filesystem, tier 1), **`viewer-gtk`'s `pdf-viewer-gtk` — a real GTK4
application, tier 1** (ADR 0244) and **`viewer-qt`'s `pdf-viewer-qt` — a real Qt 6 Widgets
application with a C++ bridge, tier 1** (ADR 0246) and **`viewer-ffi`'s C ABI — 112 entry points, a
hand-written header and a C program that drives it, tier 1** (ADR 0247). The first two could not
prove the interface alone — one is a toolkit, the other is not a program — and the last three are
what `doc/todo/30` calls the proof the answers are enough for *somebody else's widgets*: a
`GtkListView` and a `QTreeView` over §12.3.3's outline, a `GtkEntry` and a `QLineEdit` over
§12.7.5.3's text field, a `GtkCheckButton` and a `Qt::CheckStateRole` over §12.7.5.2.3's check box,
and **three whole hosts that between them needed no new message**.

**What the second host cost, and where.** Not the vocabulary: it cost one word in a crate root.
`#[cxx::bridge]` expands to `unsafe` and a `forbid` cannot be lifted, so `viewer-qt` holds
`#![deny(unsafe_code)]` with one exemption on `mod bridge` and **one hand-written `unsafe` token**
— the `unsafe extern "C++"` header, which is `cxx` asking the author to assert that the C++
declared there exists and is safe to call. `cpp/host.h` declares one function and names no Qt type,
and `tests/unsafe_position.rs` asserts the file, the token, and — since the
four-hundred-and-eleventh — that **exactly two** crates in the tree lift the denial.
`doc/todo/30`'s "`viewer-ffi` is the only crate permitted `unsafe`" was a rule about promises a
reviewer has to check, and two promises in two places with a test on each is what keeps it (ADRs
0246, 0247).

**What the third host cost.** Not the vocabulary either, and not a word in a crate root: it cost
the three amendments below and nothing else. `viewer-ffi/src/abi.rs` holds one lint lift, 112
`#[unsafe(no_mangle)]` attributes and 104 signatures, and **no `unsafe` block anywhere in the
crate**; its own test asserts that and that every crate touching PDF bytes still *forbids* the
permission (ADR 0247).

```
host toolkit  ──Command──▶  viewer-core (no threads, no I/O, no clock)  ──Event──▶  host
 Win32/AppKit/Qt/GTK/winit  ◀──Answer──   query(&self, Query)   ◀──Query──
                                    │                                  ▲
                                    └──NeedsRender──▶ worker ──RenderReady──┘
```

- `Viewer::handle(&mut self, Command) -> impl Iterator<Item = Event>`, and
  `Viewer::query(&self, Query) -> Answer` beside it. **Selection cannot wait for a render round
  trip**, which is why the second channel is not a command.
- **And one method that is neither, since the four-hundred-and-twentieth**:
  `Viewer::readback_cache(DocumentId) -> Option<ReadbackCache>`, which says how much of this
  document's readback the search cache is holding, against what budget, with hits, misses and
  evictions. It is an *instrument* rather than a message, and that is the whole reason it is not a
  `Query`: a `Query` is a question a host asks in order to **draw** something, and six consumers
  match that enum exhaustively while `viewer-confined` puts every variant on a wire — a cost worth
  paying for a panel and not for a number no interface displays. It cost no consumer a line.
  `pdf-viewer --trace=search` is what prints it. The rule this establishes for the next such
  number: **if a host would draw it, it is a `Query`; if a person would read it, it is a method**
  (ADR 0256).
- `Command`: `Open { id, bytes, password, fragment }`, `Close`, `Focus`, `Resize { width, height, scale }`,
  `GoTo(PageTarget)`, `Zoom`, `Scroll`, `SetGroup`, **`Activate(ObjectId)`**, `Pointer { at, action }`,
  `Select`, **`Focused(FocusMove)`** (§12.5.1's tab key), `Edit(Edit)` — four of them now, with
  §12.5.6.6's `FreeText` and `SetFreeText` beside `SetField` and `Markup` (ADR 0238) —
  `Undo`, `Redo`, `Save`, **`Extract { name }`**, **`Find(Find)`** — Annex O's `search` and a find
  bar's *next*, one page per step because rule 4 forbids blocking and rule 3 leaves no clock to
  budget with; 5.84 s is what a 1023-page sweep costs and no host may be blocked for it (ADR 0250) —
  `Supply { purpose, bytes }`, **`Restrict(RestrictionLevel)`**, **`Present(PresentationMode)`**,
  `Tick { millis }`, `RenderReady { token, rendered }`.
  **`Present` is the third policy value and the four-hundred-and-eighty-first session's**, on
  §12.4.4.2's NOTE 3: that clause conditions a *state machine* — the current navigation node it
  opens by requiring — on being in presentation mode, and whether a window is showing a slide show
  is a fact no state machine over a file can see. It amends ADR 0135's "there is no presentation
  state in this crate", which deduced the mode from `Tick` and therefore answered *no* for exactly
  the case the clause is about: a person stepping through a slide show by hand drives no clock.
  Two states, `Off` (the default) and `On`; the forward and backward *requests* are the `GoTo`
  this vocabulary already had, because "the user requests to navigate forward (such as an arrow key
  press)" is what that message already means. ADR 0316.
  **`Restrict` is the one policy value in the crate**, and rule 2 is the whole reason it exists:
  how much of what a document asserts over its reader this program obeys is the *reader's*,
  never the file's and never a deduction from it. Two levels, `On` (the default) and `Off`; the
  project owner's other two — ask and warn — are a question, and a question needs a host that
  can answer it (ADR 0212). **`Activate` is what a panel row sends** — the object, not a
  payload, so the *document* decides what activating it means (ADR 0144). **`Open`'s `fragment` is
  Annex O's**, added in the three-hundred-and-sixty-ninth: the text after `#` in the URI the bytes
  came from, undecoded, because splitting a URI is the host's and percent-decoding belongs to
  whoever knows which component it is decoding (ADR 0209).
- `Event`: `Opened`, `OpenFailed`, **`PasswordRequired`**, `Closed`, `PageChanged`,
  `NeedsRender(RenderRequest)`, `Damage(Rect)`, `OpenUri`, `NeedsFile`, `Transition`, `Dirty`,
  `Saved { bytes }`, **`Extracted { name, bytes }`**,
  **`Searched { document, found, remaining, wrapped }`** — one step of a document-wide search, and
  the only event a host has to *pump*: `remaining` above zero means send `Find::Continue` again,
  which is the same division `NeedsRender` makes and forced by the same two rules (ADR 0250) —
  **`Refused { document, operation, notes }`** — an operation this reader declined *on the
  document's instructions*, and deliberately not `Reported`: that one says what the **document**
  could not do, and this says what the reader's own policy did. It carries the operation so that
  it can become a question (ADR 0212) —
  `Reported { document, page: Option<usize>, notes }` — the `None` page is what the *document*
  says about itself (§12.11, §12.8, §7.11.4), said before any page is drawn.
- `Query` → `Answer`: `PageCount`, `CurrentPage`, `PageGeometry`, `LinkAt`, `FieldAt`,
  **`Fields`** — §12.7's whole form on the page being shown, as controls a host builds itself
  (ADR 0235) —
  **`Caret { at, offset }`** (§12.7.4.3's layout, ADR 0211), **`Offset { at, point }`** — that
  question's inverse — and **`FieldSelection { at, from, to }`**, the shapes over a range of a
  field's value (ADR 0225),
  **`FreeTextAt { at }`** — §12.5.6.6's annotation at a point and its `/Contents`, which is how a
  host aims a keyboard at one (ADR 0238) —
  `Selection`, **`LogicalSelection`** (§14.8.2.5), **`Focus`** (§12.5.1's ring),
  **`Highlight`** (Annex O's rectangle, ADR 0357), `Find`, `Dirty`, `Outline`, `Layers`, `Attachments`, `AccessibilityTree`,
  **`Opening`** (Table 29's `/PageMode` and `/PageLayout`), **`Properties`** (§14.3.3's Table 349),
  `Preferences`, `Frame`, `Reports`. **`Selection` answers in device pixels
  and produces no events**: a drag emits `Damage` and never `NeedsRender`, which is what keeps
  chrome off the rendering path.
- **Nothing is `#[non_exhaustive]`**, deliberately: it forces a catch-all arm on every host, and
  a catch-all arm is where a message added later goes to be ignored in silence. A new `Event`
  should fail to compile in every consumer. **The rule reaches types this crate does not declare**,
  which the two native hosts found and the third settled: `pdf_render::RasterFormat` crosses inside
  `Rendered::Raster` and `Answer::Frame`, was `#[non_exhaustive]`, and cost four consumers a
  catch-all apiece — so it is not, since ADR 0247. **And a C consumer cannot fail to compile at
  all**, which is what `viewer-ffi` exists to answer rather than to pretend away: every event kind
  has a name and a one-sentence description whatever the caller knows about it, and the *count* of
  kinds is what a C caller checks against its header at startup.
- **This crate interprets; the host rasterises.** `NeedsRender` carries an `Arc<DisplayList>` and
  a `TargetSpec`, so a zoom or a scroll re-rasterises *without re-interpreting* — asserted by
  pointer equality of the list in `zooming_rasterises_again_without_interpreting_again`.
- **A stale token is dropped**, so a page turned mid-render cannot be overwritten by the frame
  the previous page produced.
- `Rendered::{Raster, Presented, Failed}` is where tier 1 and tier 2 differ and the only place
  they do: a host drawing onto its own surface has no raster to hand back and has not failed.

#### What is still owed

**The writer's two recorded costs are closed** — encryption on the way out (ADR 0129) and
§12.7.4.3's appearance stream written into the file rather than owed to the next reader (ADR
0130). **And the `Edit` variant carrying a *new object* rather than a field's value landed in the
three-hundred-and-twenty-first session** (ADR 0196): `Edit::Markup` marks up what is selected in
one of §12.5.6.10's four ways, `ViewState::add_markup` builds the annotation, and §7.5.6's update
writes it and appends the reference to the page's `/Annots`. Two things came with it that are
worth knowing about — `Page` now carries its own `ObjectId`, because an edit has to be filed
against a page and the interpreter may not walk the page tree to find out which; and the edit
*log* records what was done rather than what was asked for, because "mark up what is selected" is
a fact about the moment the command arrived and undo is a replay. **And a caret since the three-hundred-and-seventy-first** (ADR 0211): `Query::Caret { at, offset }`
answers with the segment the next character will be drawn against, computed inside §12.7.4.3's own
layout rather than from the text layer — an empty field has no glyphs, and 147 of the corpus's
first-page widgets are empty text fields. **And the caret's inverse since
the three-hundred-and-eighty-eighth** (ADR 0225): `Query::Offset` turns a point into a byte offset
so a click places the caret where it landed, and `Query::FieldSelection` answers the shapes over a
range so a drag can select inside a value — a *third* question rather than two carets, because
§12.7.5.3's Multiline flag lets the layout break a value into lines a host cannot see. Copying,
cutting and pasting inside a field needed **no** message: the characters are a slice of the value a
host has already read back and the edit is the `Edit::SetField` a keystroke already sends.

**And §12.5.6.6's free text since the four-hundred-and-first** (ADR 0238), which is the last thing
[todo 33](todo/33-annotation-editing.md) called missing. `Edit::FreeText { from, to, colour }`
takes the two corners of a **drag** rather than a selection — that subtype "displays text directly
on the page" and so has nothing on the page to be over — `Edit::SetFreeText { annotation, text }`
says what it says, and `Query::FreeTextAt { at }` is how a host learns which annotation the drag
made. Three things came with it. The caret needed *no* new question: §12.5.6.6 sends its own
subtype to §12.7.4.3, so `Query::Caret`, `Query::Offset` and `Query::FieldSelection` answer for an
annotation exactly as they answer for a field and only the box underneath them differs. The
annotation is named by **object** where a field is named by §12.7.4.2's qualified name, because an
annotation has no name for anything to address it by. And an annotation the *file* states is
deliberately not answered by `Query::FreeTextAt` at all: appending an object is the writing
`CLAUDE.md` permits and replacing the producer's is a decision nobody has made.

**And a *form* since the three-hundred-and-ninety-eighth** (ADR 0235). `Query::Fields` answers with
every field that has a widget on the page being shown — §12.7.5's type, the flags of Tables 227, 229,
231 and 233 that decide what kind of control it is, Table 234's items and selection, Table 232's
`/MaxLen`, and the appearance-state name §12.7.5.2.3 makes a check box's value — so a host builds a
real `QLineEdit`, a real combo box and a real check box instead of taking a form as pixels off the
raster. `doc/todo/37` was the audit that found it the last of six chrome populations not to cross —
absorbed into [30](todo/30-a-native-host.md) once its last decision was taken — and the round that
closed it found something the audit had not: **a check box could not be checked at all**, because
§12.7.5.2.3's "[t]he value of the V key shall also be the value of the AS key" binds the processor
that changes `/V` and this tree read only the sentence after it.

**And the page drawn *without* those widget appearances since the four-hundred-and-ninth** (ADR
0245), which is the last thing §0 owed here. `Command::Delegate(WidgetAppearances)` is §6.3.2.2's
"unless otherwise instructed" as a value only a host can supply — the second policy in this
vocabulary after `Restrict`, and a different kind: that one says how much of what a *document*
asserts over its reader this program obeys, this says which half of the page the *host* draws. It
reaches `interpret` through `ViewState`, where rule 1 says a statement about the view belongs and
where the magnification already sits, and it removes **exactly the widgets `Query::Fields` answered
for** — a widget §12.7.4.2 leaves "simply a Widget annotation" keeps its appearance, because no
control replaced it. `ViewState::of` is the other value, so every existing caller's display list is
unchanged: 974 corpus documents digested before and after, an empty diff.

**The vocabulary is complete**, and ten sessions of building on it added five messages rather than
changing any — ten now, with `Query::Caret` in the three-hundred-and-seventy-first,
`Query::Offset` and `Query::FieldSelection` in the three-hundred-and-eighty-eighth,
`Query::Fields` in the three-hundred-and-ninety-eighth, `Query::FreeTextAt` in the
four-hundred-and-first and `Query::Highlight` in the five-hundred-and-twenty-second, and it is the
same rule those six followed: a *question* a host cannot answer for itself, never a second way to
say something it can — `Command::Activate`, `Command::Extract`, `Event::Extracted`, `Query::Opening`,
`Query::Properties`, each because a *clause* needed a channel — with one variant **removed** the
session after it was added, because the fuller reading of §12.3.3 made it a path nobody takes
(ADR 0144). **Two variants changed shape in the two-hundred-and-fourteenth**, both for the same
reason and neither adding a message: `Command::Zoom` gained the viewport point to hold still
(ADR 0166) and `Answer::Field` gained §14.9.3's second name (ADR 0167) — where a host needs two
things a variant carried one of, the variant changes and every consumer fails to compile, which is
what nothing being `#[non_exhaustive]` is *for*. **`Edit::SetField` changed shape in the
four-hundred-and-twelfth**, on that rule and with three hosts behind it: its value is
`pdf_model::view::Entered` — characters, §12.7.5.4's chosen options as indices into Table 234's
`/Opt`, or a clear — because Table 233 bit 22 lets a list box hold several items and one string
could not say which. GTK4, Qt and the headless harness had each asked their control for single
selection *deliberately*, which is what a message-shaped gap looks like when three people find it
independently. Six consumers failed to compile; the C ABI did not, because `Command::Edit` is not
among its 39 entry points at the time, and `PDFV_EVENT_KIND_COUNT` stayed 15 (ADR 0248). **`Answer::Field` changed shape a second time in
the four-hundred-and-eleventh**, for that rule's own reason and with a bug behind it: its value is
`Option<pdf_model::view::ShownValue>` now, the characters beside Table 231 bit 14's `obscured`, and
what the compiler failure found was `viewer-ui` writing a password field's bullets back as its next
value on every keystroke. `Answer::Fields` carries the same type, so a host cannot learn the
exception from one question and miss it in the other (ADR 0247).

So what is left of §0 is **hosts**, and each has a file: [30](todo/30-a-native-host.md), whose
three landed in the four-hundred-and-eighth, the four-hundred-and-tenth and the
four-hundred-and-eleventh, and whose remainder is *surface* rather than architecture. **The three
amendments the ABI waited on are taken** (ADR 0247): `RasterFormat` is no longer
`#[non_exhaustive]`, `Answer::Outline` is owned like its two siblings, and `Answer::Field` carries
`pdf_model::view::ShownValue` — the characters *and* whether Table 231 bit 14 replaced them — because
the third of those was not a doc sentence but a bug `viewer-ui` had been shipping: it read a
password field's value back after every keystroke and sent the bullets as the next value —
[31](todo/31-accessibility-host.md), whose four named edges — a `TH` cell's axis, a `Form` element's
control role, AT-SPI's `Text` interface and the actions a client may request — are all four closed
(ADRs 0300, 0338, 0394, 0425), the last of them without adding a message: an action resolves to a
*place* and `Command::Scroll` and `Command::Pointer` already take places. What is left there is
upstream and geometric rather than a vocabulary question — and
[32](todo/32-presentation-player.md) a presentation player. **Ctrl + wheel zooming landed in the
two-hundred-and-fourteenth session**, and the interesting half was in the core rather than in the
host: a zoom anchored at the pointer has to hold a page point that `Open::origin` knows about and
the scroll does not, because a page smaller than the viewport is *centred* (ADR 0166).

**The panels this file called "the largest single thing this project owes" for thirty sessions
are drawn.** `viewer_ui::chrome` is a sidebar of four tabs and a modal card, in a `pdf-render`
display list at an identity transform so that both backends draw it (ADRs 0142 to 0145). None of
the gates can see any of it, which is why `viewer-ui/tests/panel.rs` rasterises the panel's own
display list with `render-cpu` and **counts ink** rather than asserting a command count — checked
by deleting the glyph fill, which fails four of its eight cases.

**The one thing it needed that did not exist is worth naming, because it is not a UI problem.**
Text this program generates for *itself* had no font: every route into `pdf-font` takes a
`&Document` beside a `&Dictionary`, and an interface has neither. `LoadedFont::standard` loads one
of §9.6.2.2's fourteen through the ordinary `LoadedFont::load` against a new `Document::empty`, so
the encoding is §9.6.5.2's and the widths are the clause's own — and an interface set in Helvetica
is set in the same Helvetica on a machine with no fonts installed.

#### Crates

- `viewer-core` — the state machine. **Exists**; depends on `pdf-model`, `pdf-render` and
  `pdf-syntax` and nothing else. Owns the open-document set, page/zoom/scroll, links and
  §12.6's actions, the selection, the edit log and the render scheduler's *bookkeeping* (not its
  threads). **Search landed in the four-hundred-and-fourteenth** — `Command::Find`, one page per
  step (ADR 0250) — so what this line still owes is a navigation history.
- `viewer-render` (new, optional) — a default worker a host may use instead of writing one.
- `viewer-gpu` (new, later) — tier 2. The only crate that may name `raw-window-handle`, `wgpu` or
  `vello` in its API.
- `viewer-ffi` — **exists** since the four-hundred-and-eleventh, and it is the last host
  `doc/todo/30` names. **112 `extern "C"` entry points since the five-hundred-and-twenty-second**, which
  is the whole of this vocabulary — the pointer and the selection, §12.7's form and the four edits,
  save and extract, the other two panels, §12.4.4's clock and the three policy values (ADR 0346) —
  a hand-written `include/pdf_viewer.h`, and
  `c/open_a_page.c` which a test compiles with `-Werror` and runs. Commands are functions rather
  than a tagged union (a union's size is part of an ABI; a symbol is not); events and answers
  arrive owned so no borrow of the viewer crosses; a render request is an opaque handle a caller
  may move to a thread of its own; and a frame is copied into a buffer the caller owns. Depends on
  `viewer-core`, `viewer-host`, `pdf-render`, `pdf-model`, `pdf-syntax` and `render-cpu` — the last
  only so that a caller has something to draw a display list with. Cross-compiles to both the
  Windows and macOS targets, unlike either toolkit host. ADR 0247.
- `viewer-accessibility` — **exists** since the three-hundred-and-seventy-sixth. §14.7's tree onto
  AccessKit, and the only crate permitted to name `accesskit_unix` and therefore an async runtime.
  Depends on `viewer-core`, `pdf-model` and `accesskit`; nothing depends on it but `viewer-ui`.
- `viewer-ui` — consumer #1 since session 132, and a tier-2 host.
- `viewer-host` — **exists** since the four-hundred-and-tenth, and it is what a second host
  discovered: the three panel answers as one row shape, §12.7.5's field as the control it is,
  §12.7.6.4's file policy, §O.2.1's extraction policy, the launch timeline, and — since the
  five-hundred-and-eleventh — **the magnification at which a platform control fits the `/Rect` the
  document states for it** (`fit::ControlFit`, ADR 0346), which is the piece ADR 0245's third
  decision was missing and which needed no message: `Query::Fields` gives the rectangles, the
  toolkit gives the minimums, and `Zoom::Scale` takes the answer. Toolkit-free, depended
  on by all three hosts since the four-hundred-and-seventy-fifth — `viewer-ui` took the dependency
  rather than keep a third copy of the decision (ADR 0310) —
  and deliberately *not* in `viewer-core` — a mapping from three answers into one row shape is a
  convenience for whoever draws a tree, not a statement about a document. ADR 0246.
- `viewer-gtk` — **exists** since the four-hundred-and-eighth. GTK4 through `gtk4-rs`, tier 1,
  `#![forbid(unsafe_code)]` held; its whole public interface is `Host` and `HostError`, because
  four of its eight modules turned out not to be GTK's. Depends on `viewer-core`, `viewer-host`,
  `pdf-render`, `render-cpu`, `pdf-model` and `pdf-syntax`; nothing depends on it. ADR 0244.
- `viewer-qt` — **exists** since the four-hundred-and-tenth, and it is the only C++ in the tree.
  Qt 6 Widgets through `cxx` and `cxx-qt-build`, tier 1, `#![deny(unsafe_code)]` with one exemption
  and one hand-written token. **C++ owns the host** for the life of `QApplication::exec`, which is
  the ownership inverted from `viewer-gtk` and why this one needs no `RefCell`. ADR 0246.
- `pdf-model` — has the text layer (ADR 0118). The edit log lives in `viewer-core` and reaches
  interpretation through `ViewState`, which was already the log §12.6.4's actions write to — so
  `interpret` did not need a third input after all, and rule 1 holds without one.

#### Five rules, and each has a reason that already exists in the tree

1. **`pdf_syntax::Document` is immutable, forever.** An edit is a log beside it, not a change to
   it — the pattern `view.rs` already uses for §12.6.4's actions, and which the edit log joined
   in the hundred-and-thirty-fifth session rather than displacing. `interpret` stays a pure
   function of the document and the view state, which is what keeps the oracle's comparison of
   1665 pages meaning anything. Stated in `CLAUDE.md`, and held.
2. **No filesystem in the core.** The host supplies bytes; the core produces bytes. Not new
   policy — `Request::Import` and `Request::Resolve` already do exactly this, argued in ADRs 0090
   and 0104: "a document naming a file is a document asking this machine for something, and
   whether to give it is not a rendering decision."
3. **No clock.** §12.4.4's transitions and `/Dur` auto-advance arrive as `Command::Tick { millis }`.
4. **No threads the core was not handed**, and no blocking.
5. **No toolkit or graphics type in `viewer-core`'s public API.** Tier 2 (below) lives in a
   separate crate.

#### Pixels: three tiers, and interactive chrome is not pixels

| tier | what crosses | hosts | cost |
|---|---|---|---|
| **1** | a CPU `Raster` | everything, today, no unsafe | one copy per frame |
| **2** | a raw window handle; we drive wgpu/vello | anything producing `raw-window-handle` | a graphics dependency in the *binding* |
| **3** | the host's own GPU device/texture | one toolkit at a time | interop per platform |

**Define the interface at tier 1**, because that is what makes the core toolkit-free by
construction rather than by discipline. **This paragraph used to add "and it is not a compromise
here, because `CLAUDE.md` makes the CPU backend the startup path", and that reason is gone**: the
project owner decided in the two-hundred-and-seventy-third session that page one goes to the
graphics device, so tier 1 is a portability choice and not a startup one. **And neither native
host had a choice about it**, for two different reasons — which is what makes the agreement worth
something. GTK4 gives a widget no native surface for tier 2 and hands out no device for tier 3, so
tier 1 is what its public API admits and nothing else (ADR 0244). Qt *has* `QOpenGLWidget` and
`QVulkanWindow`, and neither is the comparable host — they are different widgets with different
rules about being composited inside a `QSplitter` beside a sidebar — while `QRhi`, which owns the
device Qt draws through, is a private module a release may change without notice (ADR 0246).

**The copy is now measured on both, cold and warm, rather than estimated.** `Raster` is
`GDK_MEMORY_R8G8B8A8` and `QImage::Format_RGBA8888` *exactly*, so there is no conversion at all in
either direction:

| | first frame's copy, median of five | steady state, after a page turn |
|---|---|---|
| `gdk::MemoryTexture`, 2 687 100 B | 748 µs — 3.6 GB/s | **234 µs — 11.5 GB/s** |
| `QImage`, 2 765 244 B | 1078 µs — 2.6 GB/s | **231 µs — 12.0 GB/s** |

**ADR 0244's ≈3.2 GB/s was a first-frame number**, and the steady state is three to four times
faster on both toolkits; the two agree to within 4%, which is what "only a `memcpy`" predicts and is
what one host could not have said. Cost, with a number: 1920×1080 RGBA is 8.3 MB, so full-window
repaint at 60 fps is ~500 MB/s of memcpy — a few percent of a core even at the *cold* rate, and only
during smooth scroll. `TargetSpec::transform` already carries "any tile offset", so tiled repaint is
the first lever if it matters.

**Interactive chrome crosses as geometry, not pixels.** Selection highlights, an in-progress
annotation rubber-band, resize handles, a caret — these change at pointer speed and must not
force a page re-render. Emitting them as quads and points lets a native host draw selection in
**macOS's selection colour, KDE's accent, the Windows highlight brush**, with its own caret blink
and focus ring. That is most of what makes an embedded view feel native and is unreachable if we
hand over finished pixels. It also means a slow render never blocks feedback.

**And the two native hosts found that the *colour* is a fact about each platform.** GTK 4.22 exposes
no accent colour to application code at all — there is no symbol containing `accent` in `gtk4-sys`,
and `@accent_bg_color` is a CSS name libadwaita defines — so `viewer-gtk` draws the selection fill
and §12.5.1's ring in `gtk_widget_get_color`, the theme's own foreground, which follows a light or
dark theme without the program knowing which is on. **Qt has both**: `QPalette::Highlight` and,
since Qt 6.6, `QPalette::Accent`, which KDE writes from the colour scheme. `viewer-qt` says which it
used and the pixel proves it — selection drawn in `#3daee9`, sampled back out of the window as
`srgb(187,227,248)`, which is that colour at the overlay's 0.35 alpha over white.

**So the sentence above is satisfiable on one of the two platforms and not on the other, and that
sharpens the argument rather than weakening it**: handing over finished pixels would have made even
GTK's answer impossible, and which colour a platform will part with is a fact about each platform
rather than about this boundary (ADRs 0244, 0246).

| | crosses as | changes at | drawn by |
|---|---|---|---|
| page content | `Raster` | page, zoom, edit | us |
| interactive chrome | geometry | pointer speed | the host |

#### Two artefacts, both of which now exist

**A text layer — done in the hundred-and-thirty-third session (ADR 0118), and selection on it in
the hundred-and-thirty-fourth (ADR 0119).**
`Interpretation::text_layer` is one `Placed` per character code: the range of the readback it
produced and the quadrilateral its glyph occupies, in the display list's coordinates. The box is
the glyph's advance by Table 120's `/Ascent` and `/Descent`, and it is built for rendering modes
3 and 7 too, because an OCR layer under a scanned page is exactly the text a person selects.
Measured at **+1.69%** of interpretation by an A/B in one sitting, and kept unconditional with the
cost written down.

**Search is the layer's third consumer** since the hundred-and-fortieth session: `Query::Find`
answers with the same shapes `Query::Selection` does, case-insensitively, and cost one function
because the geometry was already there. **And it reached a program at last in the
four-hundred-and-fourteenth**, three of them: a find bar drawn by `viewer-ui`, a `GtkSearchBar` and a
`QToolBar`, each drawing the matches under the selection in its own colours. What the round found on
the way is the *scope*: `Query::Find` is the page and Annex O wants the document, so `Command::Find`
is the second question rather than the first one looped, and a match highlight needed no new answer
because it is the same kind of thing as a selection highlight (ADR 0250). **§14.8.2.5's *logical* order is the layer's fourth consumer** since the
two-hundred-and-ninety-sixth session: a selection is taken in content order — which is what its
shapes are in — so `Tree::logical_range` maps a *range* of the readback through the structure
tree's order and `Query::LogicalSelection` is what a host asks when a person presses copy. It
answers nothing where the tree does not reach every byte of the range, because a copy that
silently dropped what the tree missed would be worse than one handing back content order.
**What is still not built on it**: word and paragraph selection. The caret is *not* built on it and
deliberately so — §12.7.4.3's layout is what knows where the next character in a *field* goes, and
the text layer cannot answer for a field with no glyphs in it (ADR 0211).

**An edit log — done in the hundred-and-thirty-fifth session (ADR 0120).** `Open::log` is what a
person did, with a cursor; undo moves the cursor and the surviving prefix is *replayed* rather
than inverted, because an inverse would have to remember what each edit replaced and would drift
the moment two edits touched one field. `ViewState::set_field` is the fourth statement about a
field's value beside Table 226's `/V`, §12.7.6.3's `/DV` and §12.7.8's imported one, and the last
one made stands.

**And it is saved** since the hundred-and-thirty-sixth session (ADR 0121): `ViewState::save`
produces the file with §7.5.6's incremental update appended, the host writes the bytes, and
`pdftotext` and `mutool` both read the value back out of what it wrote. The producer's bytes are
still there underneath, which is the clause's whole point. **Both costs written down are closed.**
§12.7.4.3's appearance stream is *written* since the hundred-and-forty-fifth session (ADR 0130)
rather than owed to the next reader behind Table 224's `/NeedAppearances` — the bytes are the ones
this program draws, so writing them is not a new opinion about the file — and the flag is now set
only for a widget whose stream this program could not produce or could produce only part of. A
widget that had no `/AP` gets an object *added*, which is the half of §7.5.6's "changed, replaced,
or deleted" the writer did not do; the number it starts from is the larger of `/Size` and the
highest the cross-reference table holds, because 68 corpus documents understate the first. **An
encrypted document is written since the hundred-and-forty-fourth session** (ADR 0129): §7.6.2's
ciphers run on the way out through `decrypt_object`'s mirror, so the clause's exceptions are
stated once rather than twice, and the six corpus documents covering every revision and method
§7.6 states take a string and give it back — `mutool` and `pdftotext` read all six. The cost of
*that* is one testing habit: §7.6.3.2 requires a fresh random initialisation vector per AES
string, so an encrypted document's save is no longer byte-identical from one run to the next, and
its tests read the file back rather than compare it.

#### The prize: one boundary, not two — **taken in the three-hundred-and-eighty-first**

Principle 3 wants the interpreter and rasteriser confined, and this file recorded the open question
as "the protocol would have to carry a display list rather than an image, which is a real design
question". **It dissolved exactly as predicted**: `viewer-confined` is `Command`/`Event` with
`Raster` payloads, the confined process owns document, interpretation and rasterisation, and the
host receives pixels and events — one protocol instead of two.

What the prediction did not say, and what building it showed: **`viewer-core` needed no change at
all**. The five rules below are a description of a confined process — no filesystem, no clock, no
threads it was not handed — so the crate written to be free of a *toolkit* turned out to be free of
a *kernel* too. Everything that had to be decided was on the transport's side: eleven questions
answer with `pdf-model` types and are refused by name, and a page draws on one thread because
`glibc`'s allocator asks the kernel how many processors there are by reading a file. ADR 0218,
`doc/todo/34`.

#### Near, and far

Form-field editing landed in the hundred-and-thirty-fifth session and saving in the
hundred-and-thirty-sixth; §14.8.2.5's logical order in the two-hundred-and-ninety-sixth, the caret
in the three-hundred-and-seventy-first, the click that places it in the three-hundred-and-eighty-eighth
and §12.5.6.6's free text in the four-hundred-and-first. What is left of *using* a document is one
file, [todo 33](todo/33-annotation-editing.md), and it is two items rather than a feature: editing
an annotation the **file** states, and Table 177's callout line. Editing the page's own text is far
and deliberately out of scope.

# The UI boundary — `viewer-core`, its vocabulary, and the three pixel tiers

Status: **built** — two consumers on it, a third behind a confinement.
Read by: anybody writing a host, adding a `Command`, `Event` or `Query`, or asking what the
crate boundary permits. `doc/HANDOVER.md` §0 is the pointer to this file, and ADRs 0116 to 0121
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

Two consumers: `viewer-ui`'s `pdf-viewer.rs` (winit + vello, tier 2) and
`viewer-core/tests/headless.rs` (no display at all, tier 1). Neither can prove the interface
alone — one is a toolkit, the other is not a program — and together they are why the vocabulary
is worth trusting.

```
host toolkit  ──Command──▶  viewer-core (no threads, no I/O, no clock)  ──Event──▶  host
 Win32/AppKit/Qt/GTK/winit  ◀──Answer──   query(&self, Query)   ◀──Query──
                                    │                                  ▲
                                    └──NeedsRender──▶ worker ──RenderReady──┘
```

- `Viewer::handle(&mut self, Command) -> impl Iterator<Item = Event>`, and
  `Viewer::query(&self, Query) -> Answer` beside it. **Selection cannot wait for a render round
  trip**, which is why the second channel is not a command.
- `Command`: `Open { id, bytes, password, fragment }`, `Close`, `Focus`, `Resize { width, height, scale }`,
  `GoTo(PageTarget)`, `Zoom`, `Scroll`, `SetGroup`, **`Activate(ObjectId)`**, `Pointer { at, action }`,
  `Select`, **`Focused(FocusMove)`** (§12.5.1's tab key), `Edit(Edit)` — four of them now, with
  §12.5.6.6's `FreeText` and `SetFreeText` beside `SetField` and `Markup` (ADR 0238) —
  `Undo`, `Redo`, `Save`, **`Extract { name }`**,
  `Supply { purpose, bytes }`, **`Restrict(RestrictionLevel)`**, `Tick { millis }`, `RenderReady { token, rendered }`.
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
  `Selection`, **`LogicalSelection`** (§14.8.2.5), **`Focus`** (§12.5.1's ring), `Find`, `Dirty`, `Outline`, `Layers`, `Attachments`, `AccessibilityTree`,
  **`Opening`** (Table 29's `/PageMode` and `/PageLayout`), **`Properties`** (§14.3.3's Table 349),
  `Preferences`, `Frame`, `Reports`. **`Selection` answers in device pixels
  and produces no events**: a drag emits `Damage` and never `NeedsRender`, which is what keeps
  chrome off the rendering path.
- **Nothing is `#[non_exhaustive]`**, deliberately: it forces a catch-all arm on every host, and
  a catch-all arm is where a message added later goes to be ignored in silence. A new `Event`
  should fail to compile in every consumer.
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
raster. `doc/todo/37` is the audit that found it the last of six chrome populations not to cross, and
the round that closed it found something the audit had not: **a check box could not be checked at
all**, because §12.7.5.2.3's "[t]he value of the V key shall also be the value of the AS key" binds
the processor that changes `/V` and this tree read only the sentence after it. What §0 still owes
here is a page drawn *without* its widget appearances, which is a change to `interpret` and a round
of its own.

**The vocabulary is complete**, and ten sessions of building on it added five messages rather than
changing any — ten now, with `Query::Caret` in the three-hundred-and-seventy-first,
`Query::Offset` and `Query::FieldSelection` in the three-hundred-and-eighty-eighth,
`Query::Fields` in the three-hundred-and-ninety-eighth and `Query::FreeTextAt` in the
four-hundred-and-first, and it is the
same rule those five followed: a *question* a host cannot answer for itself, never a second way to
say something it can — `Command::Activate`, `Command::Extract`, `Event::Extracted`, `Query::Opening`,
`Query::Properties`, each because a *clause* needed a channel — with one variant **removed** the
session after it was added, because the fuller reading of §12.3.3 made it a path nobody takes
(ADR 0144). **Two variants changed shape in the two-hundred-and-fourteenth**, both for the same
reason and neither adding a message: `Command::Zoom` gained the viewport point to hold still
(ADR 0166) and `Answer::Field` gained §14.9.3's second name (ADR 0167) — where a host needs two
things a variant carried one of, the variant changes and every consumer fails to compile, which is
what nothing being `#[non_exhaustive]` is *for*.

So what is left of §0 is **hosts**, and each has a file: [30](todo/30-a-native-host.md) a native
host and then `viewer-ffi`, [31](todo/31-accessibility-host.md) the four edges the AccessKit
bridge does not yet cover — a `TH` cell's axis, a `Form` element's control role, AT-SPI's `Text`
interface and the actions a client may request — and
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
  threads). Still owes search and a navigation history.
- `viewer-render` (new, optional) — a default worker a host may use instead of writing one.
- `viewer-gpu` (new, later) — tier 2. The only crate that may name `raw-window-handle`, `wgpu` or
  `vello` in its API.
- `viewer-ffi` (new, last) — the C ABI, and the only crate in the tree permitted `unsafe`.
- `viewer-accessibility` — **exists** since the three-hundred-and-seventy-sixth. §14.7's tree onto
  AccessKit, and the only crate permitted to name `accesskit_unix` and therefore an async runtime.
  Depends on `viewer-core`, `pdf-model` and `accesskit`; nothing depends on it but `viewer-ui`.
- `viewer-ui` — consumer #1 since session 132, and a tier-2 host.
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
graphics device, so tier 1 is a portability choice and not a startup one. Cost, with a number: 1920×1080 RGBA is 8.3 MB,
so full-window repaint at 60 fps is ~500 MB/s of memcpy — a few percent of a core, and only
during smooth scroll. `TargetSpec::transform` already carries "any tile offset", so tiled repaint
is the first lever if it matters.

**Interactive chrome crosses as geometry, not pixels.** Selection highlights, an in-progress
annotation rubber-band, resize handles, a caret — these change at pointer speed and must not
force a page re-render. Emitting them as quads and points lets a native host draw selection in
**macOS's selection colour, KDE's accent, the Windows highlight brush**, with its own caret blink
and focus ring. That is most of what makes an embedded view feel native and is unreachable if we
hand over finished pixels. It also means a slow render never blocks feedback.

| | crosses as | changes at | drawn by |
|---|---|---|---|
| page content | `Raster` | page, zoom, edit | us |
| interactive chrome | geometry | pointer speed | the host |

#### Two artefacts, both of which now exist

**A text layer — done in the hundred-and-thirty-third session (ADR 0118), and selection on it in
the hundred-and-thirty-fourth (ADR 0119).**
`Interpretation::text_layer` is one `Placed` per character code: the range of the readback it
produced and the quadrilateral its glyph occupies, in the display list's coordinates. The box is
the glyph's advance by Table 122's `/Ascent` and `/Descent`, and it is built for rendering modes
3 and 7 too, because an OCR layer under a scanned page is exactly the text a person selects.
Measured at **+1.69%** of interpretation by an A/B in one sitting, and kept unconditional with the
cost written down.

**Search is the layer's third consumer** since the hundred-and-fortieth session: `Query::Find`
answers with the same shapes `Query::Selection` does, case-insensitively, and cost one function
because the geometry was already there. **§14.8.2.5's *logical* order is the layer's fourth consumer** since the
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

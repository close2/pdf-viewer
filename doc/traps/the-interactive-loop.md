# Traps: the interactive loop, and the spaces a point lives in

Status: **standing** — each is a mistake somebody actually made in this tree.
Read by: a round that turns a press into a command, a command into a request, or a request into a
frame — and any round that converts between the page's space, the display list's and the raster's.
`doc/ui-boundary.md` is the interface; `doc/environment.md`'s `Xvfb` recipe is the only way to
exercise the whole loop.

[`README.md`](README.md) is the index, one row per trap, and says why each keeps its number.

## Traps

### 12a. The display list's space is not the raster's, and a doc comment said it was

PDF's y axis points up from the bottom of the page; a raster's points down from its top row. The
flip lives in `TargetSpec::for_page` — "the page's top edge must land on pixel row zero", ADR 0064 —
and **not** in `base_transform`, so a caller holding a pixel position must subtract it from the
page's height before asking `user_space_at` anything.

`user_space_at`'s own doc comment said it took a point in "the page's space — the display list's,
and the raster's" for seventy-five sessions, and **every click followed that sentence into the
mirror of the point it meant**. No gate clicks, so nothing saw it; the tests written for it took
their point from a grid scan of the broken mapping and asked whether *a* link was there, and on
the test document the mirror of a link is another link. ADR 0118.

Two rules out of it: **flip about the *page's* height, not the raster's** — the raster is rounded
up to contain the page and the spare fraction of a row is at the bottom — and **when a test needs
a point, take it from the document rather than from the code under test**.

### 17. A toolkit's widget list is a catalogue, not a statement of what it can do

`doc/todo/30` carried *"Table 233 bit 19's editable combo box in `viewer-gtk` — the one item here
that is a genuine toolkit floor"* for thirty-nine sessions, and ADR 0509 ranked it last on the
strength of it. Every sentence in the block was true: `GtkDropDown` has no entry, `GtkComboBox` and
`GtkComboBoxText` are deprecated in GTK 4.10, and this workspace binds `v4_10` with warnings as
errors. There is no GTK 4 *widget* that is an editable combo box.

The clause did not ask for a widget. Table 233 bit 19 asks for "an editable text box as well as a
drop-down list" — two things, named separately — and a `GtkEntry` beside a `GtkMenuButton` over a
`GtkListBox` in one `linked` box is both of them, with nothing deprecated and the feature floor
untouched (ADR 0596).

**A block written from a widget list is a claim about a catalogue.** Before writing that a host
cannot obey a clause, say what the clause asks for in the clause's own nouns and then ask whether
the toolkit will compose them — which is ADR 0508's rule (*call the API before writing that
something is blocked on it*) one step further out: the API to call may not be the one the block
names.

It has now cost this project twice on the same clause. ADR 0508 was Table 234's `/TI` in the same
host, where the block named `GtkListView::scroll_to`, the floor was raised, the method turned out
not to answer the question, and the answer was the scrolled window's own adjustment. **Both times
the capability was one composition away and the block named a symbol.**

The mirror of this trap is worth having beside it: **a flag reads as a permission and half of them
are prohibitions**. Bit 19's second clause — if clear, the combo box "shall include only a
drop-down list" — was broken in silence by the host that had no trouble with the first, for the
whole of that host's life. Read the sentence after the semicolon.

### 19. A widget the *document* placed can decide how big the window is

`viewer-gtk` puts the page's pictures and §12.7's controls in a `GtkFixed`, and the seven-hundred-and-twenty-sixth
session put §12.5.6.14's popup windows there too. A `GtkFixed` **measures the union of its
children**, and a popup's `/Rect` is the document's — `issue14438.pdf` states six open windows
*beside* its page, at x 598 to 785 on a 612-unit page — so the window asked the `GtkPaned` for more
room, which widened the viewport, which moved the popups further out, which asked for more room
again. Measured off `--trace`: the page area walked 509 → 653 → 838 → 1035 → 1134 → … → 1229 device
pixels in nine frames, a geometric series that converged only because each step halved.

Nothing about it looked like a defect. The windows appeared, the page drew, no gate could see it,
and the only tell was that one `Resize` line in the trace had become nine.

**The rule is about the direction of the arithmetic, not about GTK.** Everything `viewer-core`
answers with is in *device pixels of the viewport*, and the viewport is what the host tells the core
it is — so a host that lets an answer feed back into the viewport has closed a loop the boundary
cannot see. A page's raster is safe because the core sized it to the viewport in the first place;
a `/Rect` the document states is not, and neither is anything else the file decides.

Two answers, one per toolkit, and both are *explicit*: in GTK the popups are a `GtkOverlay` child,
which `GtkOverlay` does not measure unless `set_measure_overlay` says so and which it allocates its
own size (so a window outside the viewport is clipped instead of moving it); in Qt they are children
of a `PageArea` that has no layout at all, so a child's size hint reaches nothing. The Qt half was
right by accident and is now right on purpose, which is the difference worth keeping.

**And a report on what could be *placed* would not have caught it either.** The first version of
this host's trace line fired on the count of windows it placed, so a document whose windows all had
zero area printed nothing at all — the same silence as a document with no windows. It fires on what
the *answer* held, which is trap 11 pointed at the one line that would otherwise say nothing about
a refusal. ADR 0613.

### 21. A toolkit's main loop cannot dispatch your poll while it is inside its own frame

`viewer_host::Drawing` is *pulled* rather than pushed, because one of the two toolkits cannot be
pushed to (ADR 0668 §5): each host arms a one-shot for `Drawing::POLL` and asks whether a page has
landed. A poll is a request for a **turn of the main loop**, and the whole arrangement's latency is
therefore the interval *plus whatever the loop is busy with* — which is nothing at all, right up
until the one moment it is everything.

That moment is the launch. `open_document` runs inside the first size allocation, so the instant the
pump returns, GTK begins its own first frame: GSK's renderer bring-up, which under `Xvfb`'s software
Vulkan holds the loop for the better part of sixty milliseconds. Page one drew in **3.3 ms** and the
window waited **61.5 ms** for it, on a quiet machine, twenty runs an arm with no overlap between the
before and after ranges (ADR 0678). `GSK_RENDERER=cairo` is the control and the same wait falls to
about twelve.

**The tell is a wait far larger than the work, in a host that is doing nothing else** — the frame
line prints both, `rasterised … in 3.252476ms, waited 61.528955ms`, and a round that reads only the
first number sees a fast rasteriser. It is not a scheduling problem and no priority fixes it: a
source cannot be dispatched while the loop is inside another dispatch.

`Drawing::settle` is the answer for the one case where a host may block — a window with nothing on
the screen has no frame to spoil and no input to lose — and it is bounded by one refresh over the
whole launch. **What the trap is about is the general shape rather than that fix**: an interval is a
floor on a pull, never a ceiling, and the ceiling belongs to somebody else's loop.

**And the two-host reading is what located it.** `viewer-qt` paints a `QImage` with no device
bring-up in the way, so its loop was free and its column showed no regression at all. One host
cannot tell its toolkit's cost from its own; two on one arrangement can, in a single sitting. That is
a reason to keep them level beyond fairness to a user — and see trap 1 for the sting in Qt's tail,
whose *faster* launch number named a first frame carrying half the pages the arrangement showed.

### 20. `Rendered::Failed` marks a page as answered, so it is not the word for "I gave up on this draw"

The core's `Viewer::rendered` sets `on_screen.shown` for a `Rendered::Failed` exactly as it does for
a raster. That is deliberate and its comment says why — a rasteriser that refused this page at this
size will refuse it again, and without it the two spin, ask-refuse-ask, for as long as the page is
shown. What changes the answer is the *question* changing: another page, another zoom, another
interpretation.

So the variant carries two claims and only the first is on the label: *this could not be drawn*,
and *do not ask me for it again at this target and revision*. Both are true of a rasteriser's
refusal. Only the second is true of a draw a **host** abandoned — `pdf_render::Interrupt`, ADR 0650
— and a host that reports one as the other freezes that page for the rest of the view: the person
is left with `doc/todo/37`'s stand-in, permanently, and a status line blaming the document for a
decision the host made about its own thread.

ADR 0650 section 6 predicted the opposite in writing — that a host raising an interrupt would *have*
to say the render failed or the stand-in would become permanent — and it is the stand-in becoming
permanent that saying so causes. The prediction was reasonable and reading `viewer.rs` is what
settled it (ADR 0657 section 3).

**The rule: a host answers `Rendered::Failed` for what the document or the machine refused, and
answers nothing at all for what it decided.** Nothing is safe because an outstanding token holds the
page's place — `settle` skips a page whose `pending` already matches the target and revision — so
silence is not a leak and the frame that supersedes it is what answers.
`viewer-core/tests/headless.rs::a_refusal_is_final_for_this_view_and_a_token_never_answered_is_not_re_asked`
asserts both halves, because neither was written down anywhere a host could read it.

**Where the rule now lives, so that a host cannot get it wrong by writing it a third time:**
`viewer_host::drawing::Finished::outcome` is an `Option<Rendered>` and `None` means nothing is owed
(ADR 0668). The native windows share one consumer of it, `viewer_host::native_window`, whose `let
else` continues past a `None`, so the trap is visible at the call site instead of being a sentence
somebody has to remember. The same shape is
what makes the *positive* rule checkable: a tier-1 host abandons only a draw whose token
`viewer_core` has already replaced or dropped, so silence is not merely safe, it is what the core
would have done with the answer anyway.

### 22. A shared key table is only as level as the narrowest path a key takes to reach it

`viewer_host::keys` states what a press means for every window and each host translates its
toolkit's key into it — and that arrangement is checked in exactly one direction. Every host has a
test that it can translate the whole of `Key::ALL`, and `WindowAct` is matched exhaustively in each
host so that a new binding fails to compile in each. Both of those ask whether a host *can* obey
the table. Neither asks whether a press actually gets there.

**`viewer-qt` had been swallowing Escape entirely**, and did so from the round the table was written
(ADR 0526) until the seven-hundred-and-ninety-fifth found it. The key reaches that window through a
`QAction` shortcut, because a shortcut consumes a press before `keyPressEvent` ever sees it; the
action forwarded the key to the table in full screen, closed the find bar if one was open, and
otherwise **returned**. So §12.4.2's "Escape clears the selection" — one of the three disagreements
the table was created to settle — never reached the table in that host at all, while the crate's
tests, the exhaustive match and the shared documentation all said it did.

Two things about the shape are worth more than the instance:

- **The guard was legitimate where it started.** Chrome takes a key before the page does, and
  `keys.rs`'s "What a host still owns" says so: which widget has the focus is not something a shared
  value can know. What made it a defect is the *else* — a path that owns the ordering and then
  declines to pass the key on has quietly taken the decision as well.
- **The hosts that were fine call `meaning` from one place each.** The one that was not calls it
  from a second place as well, and that place is C++. Count the call sites before believing a host
  is level; a table with one entrance is checkable by the compiler and a table with a second one is
  not.

Found by pressing the key at a window that was showing a sentence naming it and watching nothing
happen — which is the only instrument this has: `doc/environment.md`'s `Xvfb` recipe, a real
window, a real key, and looking (ADR 0729).

### 44. A notebook page owns its child, so the view leaves before the page is destroyed

GTK's `remove_page` and Qt's `removeTab` followed by `delete` destroy the page widget — and in a
design where one widget tree is moved between tabs, the page being removed is the one holding it.
Closing a tab would have taken the splitter, the panels and the page area down with it; both hosts
now reparent the view first (ADR 1264). Caught by reading, not by a test, and a third toolkit
will spring it again.

### 45. A host method reachable from `react` takes the queue; it does not start one

`viewer-qt`'s `show_document` sent `Command::Focus` and was reached from the `Event::Opened` arm —
a `pump` nested inside a `pump`. Split into `take_the_front` (state only) and `show_document`
(state plus the message), which is the shape `viewer-gtk` already has by taking the queue as a
parameter (ADR 1264).

### 70. The launch gate measures launch minus the window, so a chrome cost is invisible to it

ADR 1382's fallback walked the machine's font catalogue on the drawing thread for a label the
built-in faces could not draw; `launch_path` showed nothing because the path it measures holds no
chrome, and a single-document launch does not draw a tab strip. A document that opens on its
outline panel with a Chinese item put first present at about a second cold against a hundred
milliseconds (ADR 1406). After any change to the chrome's text path, take time-to-first-page with
`--trace=frames,launch` under Xvfb on a document whose first frame's chrome holds the characters
in question, with the font files evicted from the page cache first.

### 72. A string handed to a toolkit must be logical and unshaped

`quorra` now shapes and orders a chrome label itself (`pdf_font::shaping::Label`, ADR 1417), and
`quorra-gtk` and `quorra-qt` never did, because Pango and QTextLayout apply UAX #9 and joining to
whatever string they are given. A future round that "shares" the shaping by moving it into the rows
`viewer_host` hands every window would have the toolkits reorder an already-reordered string. Shape
at the point of drawing, never in shared data; the GTK test that the rows reach Pango as stored is
what guards it.

### 76. The first frame is drawn before the hidden chrome has left the viewport

`/FitWindow` measured against the first frame's viewport fitted the wrong size in every window,
because tool bars and status lines hidden by `/HideToolbar` and `/HideWindowUI` were still in that
frame's layout. `Presenting::place` retries against the settled viewport, up to four frames, for the
first displayed page only (ADR 1429). And GTK 4 will not size a window below its natural size, which
a `GtkFixed` computes from its children's positions: the page had to become an overlay child over
an unmeasured ground before the window could shrink to the page.

### 86. A panel list takes the page's keys, and a popup is missing from the root capture

Driving the three windows end to end after two batches of layout changes (ADR 1453) found
`quorra-gtk`'s outline list holding keyboard focus at launch, after a click on the page, and after
an arrow key nobody claimed, so Home, End and the zoom keys never reached the key table — trap
57's shape on every key but Tab; `MenuButton::popup` filled the restrictions menu while the host
was borrowed and the fill was dropped, so the menu opened empty; and a check box saved as a
string `/V` with `/AS` untouched reopened unticked in all three windows. After any change to focus
or chrome, run `tools/drive-windows.sh`: it drives the key after an arrow and after a page click,
and photographs each top-level window of the process, because without a window manager a popup
is missing from the root capture and looks like a feature that did nothing.

#### 57. A toolkit that owns keyboard focus swallows Tab before the shared key table sees it

Qt's own focus chain took every Tab, so §12.5.1's walk over the document's `/Tabs` never reached
the Rust side at all, and no test could see it; GTK's move-focus did the same for its own controls
(ADR 1357). The page now declines the toolkit's focus chain and an event filter hands Tab to the
shared key table. After any focus or controls change, drive Tab under Xvfb in every window.

### 88. A generated table of pointers is relocated before `main`

Round 1315's first fold table was `[(char, &str)]`; its roughly 900 string pointers are dynamic
relocations the loader applies before any code runs, and `launch_path`'s open count on
`xfa_filled_imm1344e.pdf` rose from under its 1 820 k ceiling to 1 826 k (ADR 1465). Rebuilt as a
letter pool indexed by `(u16, u8)` it read 1 818.7 k. Any `build.rs` table is written as a pool plus
integer indices; the launch gate is what notices when it is not.

### 93. A settle that leaves a wake in flight is not settled

`MachineFaces::settle` returned once the answer had landed, while the worker thread called the
window's wake after the signal; under a full workspace test run the test's wake counter read 0,
and alone it read 1 — the signature of a race, not of a defect in the logic under test. The worker
now calls the wake first and signals after, and `settle` waits while any answer's wake is still
owed (ADR 1478 §5). A test that counts callbacks after a settle is only as good as the settle's
definition; when one fails under load and passes alone, the code's ordering is what is fixed.

### 102. A rare path inlined into a hot path de-inlines the hot path

Stepping over a string no font would show was written inside the generic `with_token`; inlined, it
stopped `with_token` inlining into the operand loop and every ordinary page paid +0.18%, visible as
`FnOnce::call_once` and `with_token` appearing as separate functions in a by-function callgrind diff.
`#[inline(never)]` on the rare path restored the hot one (ADR 1521). A/B an ordinary page by function,
not by total. And a toolkit's accessible-text answer is checked for a nonzero size: GTK's bare
`GtkText` answered `GetCharacterExtents` with a 0×0 box at the field's origin, which a drive step
reading presence alone would have called "works" (ADR 1516).

### 104. An asynchronous answer through `Supply` goes to whichever tab is in front

`doc/todo/39` designed the fetched form data to cross as `Command::Supply(ImportData)`; a fetch
can arrive after the person has changed tabs, and `Supply` answers the front tab or, when no
import is pending there, `interact::import` returns nothing and the data is dropped without a
word. The fetch now crosses as `Command::Respond { answers: Answered::Import, .. }`, addressed to
the document that asked (ADR 1527). Anything that answers later than it was asked names its
document.

### 106. Two windows hold their own viewer, and a delegating host threw its anticipated page away

The GTK and Qt windows each own a `viewer_core::Viewer`; neither passes through `viewer-ffi`'s
session, and a brief that routed the anticipation through it was corrected by reading `host.rs`.
Bringing `Viewer::anticipate` to them found a defect `quorra` could not show: `anticipate` ignored
`Command::Delegate`, so a delegating host's first resize dropped the page interpreted ahead and
interpreted it again, 18–25 ms on `bug1815476.pdf` (ADR 1539). A path on the launch route is tested
under a delegating host and a plain one, and a window's launch is measured in that window.

### 113. A step that takes the keyboard away moves the pointer off the window first

Drive step `32-window-active` gives the keyboard to the root window and expects the window to lose
AT-SPI's `ACTIVE` state. Under X, keys go to the window under the pointer, and GTK then reports
itself active, so the step reads `works` for the wrong reason or `wrong` for none: move the pointer
off the window before the keyboard leaves it (ADR 1565). And a probe arm's ceiling covers only what
it skips: ADR 1555 skipped every small region's fill and read 4.5 ms; ADR 1567 skipped the regions
and everything they lead to and read 30 ms — the same lever, two ceilings. Name what the arm skips
beside its figure.

### 115. A colour is counted through the alpha channel, and a golden gate draws page one

A drive step counted red pixels in a `quorra` screenshot with a fill and read 0: the screenshot was
all grey, ImageMagick saved it as a greyscale PNG, and a red fill matches nothing in it. Count a
colour by making it transparent and extracting the alpha (`-fuzz 6% -transparent COLOUR -alpha
extract`), which works whatever the PNG's channels (round 1372). And before a round predicts that a
change moves golden pages it reads what the gate draws: `raster_golden` and the corpus gate draw page
one of each document, and the only page-one widgets Tier 0's formatting would have changed are
Hidden (`/F` bit 2) — no page moved, and the prediction was the brief's (round 1371).

### 117. A saved file is no witness of a commit, and a grammar's bound wants a seed

A save draws every field value through its `/F` whether or not the value was committed, so a drive
step that types `12.5`, saves and reads `$12.50` has shown the format, not the commit. The witness
is a value the commit refuses and puts back — a lone `-` in a number field — so only a commit that
ran leaves the saved `/V` empty (ADR 1592). And a hand-written tokeniser with a byte bound is found
quadratic by a seed AT the bound: one letter repeated to 64 KiB took `AFDate_FormatEx`'s picture
reader 991 ms a call (ADR 1597). Every grammar with a stated bound gets a seed that states it.

### 120. A toggle key can land twice, and a frame's first event is not the one you asked for

Drive step `38-located` presses `m` to enter measuring and then clicks; in one full drive `quorra`
logged "measuring" and "measuring off" before the click — xdotool delivered the key twice — and the
step read wrong, then worked on two reruns (round 1390). A step that toggles a mode waits for the
window's own line that the mode is on before it acts. And over the confined wire a late answer rides
the worker's next frame (ADR 1553): the outline preparation's re-raised `PageChanged` for page one
rode a page turn's answer, and a test that read the frame's first `PageChanged` read 0 where it
expected 1, once in a hundred runs under tier 1's load (round 1388's merge). A test reads the event
it asked for wherever it sits in the frame.

### 123. A composed GTK control takes no focus; its named child does

Drive step `45-combo-commit` found GTK giving the keyboard to an editable combo box's composed
`GtkBox`, which takes no focus, so nothing was typed; `child_focus` into the box is no fix either,
because GTK's own Tab may already have put the keyboard in the entry, and moving forward from
there leaves it — which is a commit. `controls::give_the_keyboard` names the entry (ADR 1629). A
multiline field's `GtkScrolledWindow` is the next composed control a drive step will meet.

### 126. A module's "API" is checked for visibility, and a "cannot happen" is read against its code

Batch sixty-three's brief told the host round that `rich_text`'s layout API was what to call; every
item was `pub(crate)` and `lay_out` wrote a content stream, so the round built `popup::RichNote` and
the enabling hunk landed in a sibling's file (round 1403). A brief that names an API has checked
`pub`. And ADR 1628 said a late answer to a withdrawn question "is dropped" — true only when nothing
was queued: a press on a withdrawn card could answer the next queued script's question, which nobody
had seen (ADR 1641 fixed it). A claim about what cannot happen is read against the code that
enforces it, and a test pins it.

### 132. A `GtkOverlay` sizes its overlay child by preference, and a resize queued inside an allocation is dropped

To hand a right-to-left paragraph's tab stops over per width, round 1427 needed the label's
allocated width. With the label as the overlay child it was allocated its preferred size — a wrapped
label got its narrowest-width height, 35 px inside an 18 px overlay, and its words were drawn through
the next line — and a `queue_resize` issued inside the allocation was dropped, so the tab array took
effect while the label kept its old height. The construction that works: the widget being measured is
the overlay's main child, a `GtkDrawingArea` over it (drawing nothing, taking no pointer) reports its
`resize`, and the label is re-measured once from an idle callback (ADR 1690).

The second instance: a threaded popup's column under an overlay was cut off below the window's edge
in `quorra-gtk`, because an overlay child holding wrapped text is allocated the height it prefers at
its narrowest width; the column now sits in a scrolled window that scrolls nothing, and only the
drive's photograph of step 65 saw the clipping (round 1445, ADR 1727).

And being inside the `GtkFixed` that holds the placed controls is not being on a control: the page
picture is a child of the same fixed, so every press on the page first counted as a control's, and
GTK's entry and Qt's line edit swallowed a press before the core saw it, so no click in a field had
ever reached a script site (round 1458, ADR 1752). A hit test asks the placed widgets themselves, and
a capture-phase controller reads the press without taking it from the control.

### 137. A replayed log re-runs every side effect inside it

`Open::commit` rebuilt the view state by replaying the whole edit log at every edit, so every
keystroke script already logged ran again under whatever keys were held at that moment: one arrow
selection logged two `/K` runs and the first carried the earlier option (round 1475, ADR 1787). A
log is replayed to *rebuild* — after an undo or a redo — and an entry's side effects happen once,
when it is committed. The tell is a script, a report or a verdict that happens once per later edit.

### 141. A join on the toolkit thread is a wait with no limit

Every window started a thread to overlap the launch (ADR 1531) and then joined it with no time
limit, so the owner's 15 s page froze the toolkit thread for the whole read while the window drew
nothing and answered nothing (round 1487, ADR 1811). `viewer_host::Loading` waits at most `WARN`
and then polls every 33 ms, showing how far the read has got. A wait on the toolkit thread is
bounded by `WARN`; what runs longer is a sentence and a bar, and the window keeps answering.

### 143. A frame's cost read from its last call is the cost of that call

`quorra`'s sharp pass predicted its cost from `last_built`, which after a processor fallback was the
raster upload — 3 to 5 ms for a frame whose processor draw had taken seconds — so the device was
asked again after every fallback frame and the whole-page picture retried for ever (round 1492,
ADR 1821). A frame's cost is the whole frame's, the fallback included, and every prediction
downstream reads that figure; a refusal that is a fact about the frame is remembered by the pages
and the zoom level, and the device is not asked again for it.

### 148. A flag for another thread's idle work is raised after the job is sent, and the work is routed by where the frame went

Two mistakes in round 1497's exploratory runs each put an uninterruptible device whole-page draw of
several seconds in front of a zoom notch: an interrupt flag handed to the render thread's idle work
was raised before the job carrying it had been sent, so nobody saw it; and the idle work was routed by
whether the frame's draw had begun rather than by which lane the frame had been sent to, so a frame
sent to the processor still started the device's whole-page picture (ADR 1831). The flag is raised
after the send, and idle work follows the frame's lane.

### 151. A `GtkFixed` child is never smaller than its texture

A `GtkFixed` gives each child its natural size: a size request can enlarge a `GtkPicture` but
cannot shrink it below the texture it holds, so a frame asked to stand in at a smaller extent after
a notch out stayed at its old size (round 1503, ADR 1843). A stand-in's scale goes through
`GtkFixed::set_child_transform` — translate, then scale — for the stand-in only, and every frame
drawn at its own size keeps `move_`.

### 152. What a frame depicts is marked where it is asked for, because it lands a tick later

`quorra` asked for a transition frame as a `Placed` carrying the arriving page's identity at the
identity target, forgot the view on that same tick, and then the frame landed a tick later and was
recorded as "the arriving page, settled at the identity"; the first tick after the transition read
that as a zoom, predicted a missed refresh and put up a two-page reprojection for 12–32 ms — the
owner's "last frame shows something incorrect" (round 1508, ADR 1853). A frame carries what it
depicts (`Depicts::Transition`) from the place it is asked for, the landing reads the mark, and a
transition frame settles no view, feeds no prediction and is never retained.

### 154. The launch's critical path stays on the thread the launch placed

This machine has four fast cores and eight slower ones. Round 1509's first design read the page on
a newly spawned thread and drew on the opening thread; the read took 3.05–3.37 s against 2.14–2.24 s
and the first frame landed later than before the change (ADR 1855). Reversed — the read on the
opening thread, the drawer spawned — the first look came at 0.54 s and the first frame no later. Work
on the critical path keeps the thread the launch already placed; the helper is what moves.

### 156. A face matches the window's settled picture, edge row included

A page whose device extent is fractional has its last row rounded up (ADR 0064). `viewer_host::clock::face`
drew a transition's faces over the window's surround, so the half of that last row outside the page
carried the surround's colour where the settled page carries the page's own, and every mid-transition
frame in `quorra-gtk` differed from the settled page along the page's edge by about 2 970 pixels;
`quorra-qt` passed under Xvfb only because its viewport happened to give a whole-pixel page, and
showed the same seam on the owner's display (round 1514, ADR 1865). A face, a stand-in or an end
frame drawn apart from the page it gives way to is drawn the way that window draws the page, at the
whole pixel the page is shown at, and the drive judges it against the settled frame.

### 161. A window's trace or a drive's ink test can say a picture is shown when nothing is painted

Qt's look fired its trace line when the look was stored; the paint is C++'s and came later, and
for a while never. Step 81's `drawn_on_the_shot` said "inked: yes" for that unpainted look, and
"inked: no" for HEAD's Qt, so the test was not calibrated for Qt at all (round 1515, ADR 1867). The
bug was found only by photographing the window by its id under Xvfb and looking at the picture.
Before a line or a test is trusted to say *shown*, it is calibrated against a window that paints
nothing, and the photograph is looked at.

### 165. A rule that waits for a measured price waits for ever when the frame measured nothing

ADR 1860's ask-once rule asked the device only after a *measured* processor price. On the witness
page one is finished from the drawing made while it was read, which prices nothing, and each of the
next notches is taken back before it lands, so the window never asked the device during a gesture
(round 1525, ADR 1886). The condition is "the lane drew it", not "the lane priced it", and a frame
that finished ahead or was taken back counts as drawn.

### 167. A sentence from a function every command runs through is said at every pointer move

The owner's "endless rejection loop" in the toolkit windows was the pointer: `schedule` pushed
"this page cannot be drawn at this size" at every settle, every command settles, and a pointer move
is a command — 51 refusals in ten seconds with the pointer over the Qt window, one without it (round
1528, ADR 1892). A sentence is said once for the state it is about, and the state is remembered. The
same round found the events trace formatting a 718 MB `RenderReady` whole before cutting to 120
characters, 8–11 s a frame: a cut that comes after the formatting has paid for all of it.

### 170. A window's figure counts only if the quiet window outlasts the program's longest silent stage

Round 1532 read PdfCraft's "Rendering…" placeholder as a 0.56 s first frame and a 0.08 s zoom, and
threw both away when the settled picture was looked at (ADR 1900). The quiet window a figure waits
for is longer than the longest stage in which the program shows nothing new, and the picture at the
end is looked at before the figure is written. And a window started through the owner's loop may
draw and be found by nothing, so a run that pressed no key says "not pressed" and reports no figure —
trap 174 is that incident and its probe.

### 174. A locked session photographs nothing, and the owner's loop does not say so

On 2026-10-14 rounds 1531 and 1537 spent five and one holds placing instruments on the owner's
display: every window opened and presented in its trace, `xdotool` found none by pid, title or class,
and seven of eight toolkit runs that morning read "not pressed". A probe read `LockedHint=yes` from
`loginctl`: the session was locked, so no window was visible to anything. Before a clock hold places
an instrument through the loop, a probe checks `LockedHint` and counts the visible windows, and a run
that finds the session locked says "locked" and takes no figure — the hold is the batch's scarcest
resource (ADR 1863). The owner's word of 2026-10-14: the loop may unlock the session, with
`loginctl unlock-session <id>` and no other command — not a typed password, not `xdotool` on the lock
screen, not a killed locker — and the script says it did.
And the count that tells locked from open is a window of the loop's own: `xdotool search --onlyvisible
--name .` read 0 on the owner's *open* Wayland session exactly as it did on the locked one, so a loop
script opens one `quorra` window under X11 as a control and writes "locked" only when that window is
not visible either — calibrated at 0 while locked and 1 while open (round 1550, ADR 1936).

### 176. A missing Wayland socket does not give X11; unsetting both variables does

`tmp/r1531/owner.sh` set `WAYLAND_DISPLAY` to a socket that does not exist to make winit take X11.
winit 0.30 does not fall back: it panics with `NoCompositor`, and the script measured nothing on the
owner's display (round 1545). winit chooses X11 only when both `WAYLAND_DISPLAY` and `WAYLAND_SOCKET`
are absent (`platform_impl/linux/mod.rs`), so a loop script writes `env -u WAYLAND_DISPLAY -u
WAYLAND_SOCKET …` and names nothing.

### 178. A drive counts the window's own sentence, never the trace's echo of it

Under `--trace=events` every `Event::Reported` appears twice in a window's log: once as the window's
sentence (`page 1: this page cannot be drawn at this size`) and once as the trace's `<- Reported`
line. Step 82 counted both and read ten refusals where the confined window had said five — once a
size, as ADR 1892 §2.4 asks of a window that keeps the bound — and the step read `wrong` from round
1528 until round 1542 found the doubling (ADR 1921). A count of a sentence filters the trace
(`grep -v '^trace:'`), and a step is calibrated on HEAD's log before it judges a tree. The filter is
for a *sentence*: the `frames` topic prints its own lines under the `trace:` prefix, so a step that
reads frames must keep them (round 1547's first drive run filtered them away and read `wrong`).

### 181. An Xvfb display is yours only when its lock file was absent and your own pid holds it

Round 1556's drive script started `Xvfb :157` without looking at `/tmp/.X157-lock`. A neighbour's
server had held that display for forty minutes, so the new Xvfb died at once and silently, and the
Qt window with its `a` and `c` keys went to the neighbour's display for about ten seconds — into the
middle of that round's measurements. Before any window is launched on a display: the lock file is
absent, the server is started, and `/tmp/.X<n>-lock` holds *your* pid. A display that fails either
test is somebody else's; pick another number.

### 188. A refusal in an adapter's words is several lines

Round 1563's drive showed a four-line refusal in the window title: `wgpu`'s error carries its
chain of causes, one per line, and `surface.rs` had placed the whole of it where a title goes and in
the middle of the terminal's sentence. The title now takes the first line and the note reads "…
because the graphics device {problem}", so the lines end the sentence rather than split it (ADR
1962). Any sentence built around a library's error text is checked in a window, not only in a test,
because the text's shape is the library's and changes with it.

### 200. The first frame after the last command may be the one before the window landed

Round 1575's first clock hold photographed `quorra-confined` at the first device frame after its
twentieth notch and read the surround alone, with the page's window still on its way from the worker;
the second hold waited for the window's landing line and the photograph showed the page. On a path
where the device refuses and the processor fills in, the first frame is the refusal's and the picture
arrives a frame later. A photograph is taken after the line that says what it should show, and after
the pointer has moved once so that the frame is the presented one.

### 209. `xdotool --shell` writes `X` over whatever `X` held

Round 1602's scratch check kept its Xvfb pid in `X`, then evaluated `xdotool getwindowgeometry
--shell`, which prints `X=0` among its four assignments; the later `kill $X` became `kill 0` and ended
the round's own process group. The helper's four names are the tool's, not the script's: every
function that evaluates the geometry declares `X`, `Y`, `WIDTH` and `HEIGHT` `local`, and a pid lives
in a name that says so.

### 215. A feature's arrival is grepped for the refusals its absence justified

Round 1620 gave the confined window a sender and found "this window has no network" standing as the
refusal reason in two arms its brief had not named — the import fetched by absolute URI and the
signature policy's fetch — each about to become a false sentence beside the new code (ADR 2076). A
refusal's reason is a claim about what the program cannot do, and it decays the moment the program
can: when a capability lands, `grep` the tree for the sentence that named its absence, and either lift
the refusal to the new level or give it the reason that is still true.

### 217. A drive step about the drawing thread pins its lane once the window has a device

The drive's `page_as_built` step watches a page of two hundred thousand fills appear part-way, which is
a statement about the drawing thread (ADR 1825). When round 1617 gave the toolkit windows a device lane
the device drew the whole page in one call of about a second and nothing appeared part-way, so the step
failed while meaning something else (ADR 2070). A step whose subject is one lane's behaviour names that
lane — `--cpu` here — in its launch and says in its comment why the lane is pinned, so that a later
round reading a red step knows whether the lane or the behaviour moved.

### 224. `xdotool type` drops a character of a non-Latin word

Round 1642's drive judged GTK's `25-find-arabic` step `wrong`; the step's shot showed the needle typed
without its ya, and a re-run found the word (ADRs 2120, 2121). The character was lost between
`xdotool type` and the toolkit's input method, not by the window. A find verdict on a non-Latin needle
is read against the needle visible in the shot before it is believed, and re-run once before it is
attributed to a change (habit 143's shape for photographs applies to typed text too).

### 229. A window that delegates the widgets builds their controls in the same step

Round 1650's WinUI window opened its documents with `WidgetAppearances::Delegated`, so the page left out
every text, check-box, radio and choice widget on the understanding that the host would draw them, and
step one logged the `CONTROLS` record and built nothing: every form field's value was drawn by nobody,
and no gate said so because the C program checks the record, not the screen (round 1658, ADR 2153; trap
5's shape). A host that asks the page to leave the widgets out owes the controls in the same change, or
opens with the widgets drawn until it has them.

### 234. A monitor capture on the owner's display shows what is in front, not this program's window

Round 1708 asked the owner's loop for one display run and had it capture the monitor to compare the
window's pixels with the compositor's; the viewer window was not in front, and six captures of the
owner's own desktop landed under `tmp/r1708/loop/`, in directories the agent account cannot delete
(ADR 2251). A capture made on the owner's display is of the window by its own identity, after a check
that it is the surface in front, and never of the monitor; the one verdict that needed the monitor —
the window's pixels against the compositor's output — is taken on the nested compositor
(`tools/zoom-probe-wayland.py`), where nothing else is on the screen.

### 245. A drive's camera starts before the key that starts the clock it photographs

Step 83 of the windows' drive was flaky on HEAD at load 20 to 53: six wrong verdicts in thirty-two. Two
were the camera, started after the `p` that starts the presentation, seeing page one for under 0.4 s and
never counting it settled; four were a load-delayed tick holding a transition's last frame — one unit off
the final page in one edge row — long enough to count as a fourth page (ADR 2300). A camera that judges a
clock is started before the key that starts the clock, the first interval it must see carries a margin the
load can eat, and two settled pictures within one unit of each other count as one page.

### 249. A dead Xvfb's lock file still takes its display

Trap 181 made a display a round's own only after its lock file was absent and the round's pid held it;
it said nothing about leaving. Rounds killed their Xvfb by process group or lost it with a wrapper, and
`/tmp/.X<n>-lock` and `/tmp/.X11-unix/X<n>` stayed behind with a dead pid inside — a hundred and
forty-seven of them by batch one hundred and twelve's merge, when `t2-toolkit_launch` reported "no display
from :160 to :259 could be taken" and failed in one second. A round stops its Xvfb with TERM, waits for it,
and removes its own lock and socket once the server is gone; the instrument that takes a display reads
the pid in a lock it meets and, where that pid is dead and the file is the agent user's, removes it and
takes the display — another user's lock, dead or alive, is never touched.

### 253. A running drive's script is edited by nobody, not even its own round

Habit 161 moved the drive onto the export's copy of `tools/drive-windows.sh` so a sibling's edit to the
worktree's copy could not reach a running loop; round 1741 then edited the export's copy itself while its
drive was running, and bash — which reads a script as it executes — shifted its read offset into the
middle of a line (ADR 2317). Any copy a running drive reads is immutable until the drive ends; an edit for
the next run goes into a fresh copy, named for the run, and the record says which copy each run read.

### 254. A typed flags value drops the bits the bindings' feature floor does not name

The tree binds GTK at the `v4_10` floor (ADR 2250), and `gdk4` converts `Toplevel::state` through
`from_bits_truncate`, which keeps only the bits the enabled features define; `SUSPENDED` (1 << 16) is named
under `v4_12`, so it was dropped before round 1741's parking code could read it, and the first form parked
nothing (ADR 2317). Where a window state or any GDK flag is newer than the floor, the property's raw
number is read and the bit tested by value, with a comment naming the version that defines it; raising
the floor is the owner's question, and a dropped bit never is.

### 259. A ceiling on the sentence after a key is a ceiling on the key's handler

Drive step 82 turned back to a page whose picture the window retains and gave the window 2 s to print
"its retained"; it failed six of six reruns on GTK and the brief's hypothesis was a camera started late
(trap 245). GTK's key trace and a stack taken mid-wait showed the Left key arriving every time, and
`NativeWindow::turn` reading page one on the toolkit thread for 4.9–9.7 s before the handler printed
anything (trap 154's read, ADR 1886). The step also judged the photograph against the drawing thread's
`rasterised` line, which comes 0.55 s before `RenderReady`, the line printed when the frame is on the
screen (round 1747, ADR 2328). A drive's ceiling after a key covers everything the handler does before it
prints, so the step waits for the frame it needs, gives the read its time, and counts a photograph late
only after `RenderReady`; in GTK it reads the window's own paint record and asks that the first paint
after the turn be the retained picture.

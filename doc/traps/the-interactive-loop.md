# Traps: the interactive loop, and the spaces a point lives in

Status: **standing** — each is a mistake somebody actually made in this tree.
Read by: a round that turns a press into a command, a command into a request, or a request into a
frame — and any round that converts between the page's space, the display list's and the raster's.
`doc/ui-boundary.md` is the interface; `doc/environment.md`'s `Xvfb` recipe is the only way to
exercise the whole loop.

`doc/HANDOVER.md` is the index and names which group holds which trap. **Every trap keeps its
number**, because `crates/`, `tools/`, `doc/conformance/ledger.toml` and dozens of ADRs cite them
by number and an ADR is not edited to follow a file that moved underneath it (ADR 0232 §2).

## Traps

### 12a. The display list's space is not the raster's, and a doc comment said it was

PDF's y axis points up from the bottom of the page; a raster's points down from its top row. The
flip lives in `TargetSpec::for_page` — "the page's top edge is raster row zero", ADR 0064 — and
**not** in `base_transform`, so a caller holding a pixel position must subtract it from the page's
height before asking `user_space_at` anything.

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
(ADR 0668). Both native windows write the same two lines — a `let else` that continues — so the trap
is visible at the call site instead of being a sentence somebody has to remember. The same shape is
what makes the *positive* rule checkable: a tier-1 host abandons only a draw whose token
`viewer_core` has already replaced or dropped, so silence is not merely safe, it is what the core
would have done with the answer anyway.

### 22. A shared key table is only as level as the narrowest path a key takes to reach it

`viewer_host::keys` states what a press means for all three windows and each host translates its
toolkit's key into it — and that arrangement is checked in exactly one direction. Every host has a
test that it can translate the whole of `Key::ALL`, and `WindowAct` is matched exhaustively in three
hosts so that a new binding fails to compile in each. Both of those ask whether a host *can* obey
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

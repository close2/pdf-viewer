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

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

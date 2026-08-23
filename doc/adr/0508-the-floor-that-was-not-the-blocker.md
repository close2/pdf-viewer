# ADR 0508 — The floor that was not the blocker

Status: accepted, 2026-08-23. Session 678. `viewer-gtk` obeys Table 234's `/TI`, the last thing
`doc/todo/30` recorded that host as owing §12.7.5.4. The workspace's `gtk4` feature floor was
raised to `v4_12` on that file's own diagnosis, measured, and **put back**: the method the floor
buys does not say what the entry says. Driven under `Xvfb` on a **corpus** witness that the same
file said did not exist.

## 1. What the entry is, and what it is not

ISO 32000-2 Table 234, `/TI`:

> (Optional) For scrollable list boxes, the top index (the index in the Opt array of the first
> option visible in the list).

Two things follow and both matter to a host. It is the first option **visible**, which is not the
option **selected** — the table states `/I` and the field states `/V` for that, and a control that
read either for the other agrees with itself on every list where they happen to be the same row.
And it is stated "[f]or scrollable list boxes", so a list showing all of its options has nothing to
obey.

`pdf-model` has read the entry since the three-hundred-and-ninety-eighth session, the page's own
appearance has obeyed it since ADR 0407, and `viewer_host::form::ControlKind::List` has carried it
since ADR 0436. `viewer-qt` scrolls to it with `QAbstractItemView::PositionAtTop`. `viewer-gtk`
started every list at row 0.

## 2. The diagnosis this round was handed, and why it was wrong

`doc/todo/30` recorded the GTK half as a *binding floor* rather than a decision:

> GTK's half is a *floor* rather than a decision: `GtkListView::scroll_to` is GTK 4.12 and this
> crate binds `v4_10` (`Cargo.toml`), so it means raising the feature or driving `GtkListBase`'s
> `list.scroll-to-item` action on a view that is not yet in a window

Every fact in that sentence is true. The conclusion drawn from it is not, and the way to find out
was to raise the floor and try:

- `gtk4` 0.11.4 carries `v4_12` through `v4_24`; this machine runs GTK 4.22.4. The floor raised in
  one line, and cost one deprecation — `CssProvider::load_from_data` became `load_from_string` in
  4.12, and this workspace turns warnings into errors.
- `GtkListView::scroll_to` then compiles, and **moved nothing**. Not at `/TI 1`, where the option
  is already on the screen, and not at `/TI 5`, where it is not.

The reason is in GTK's own documentation of the call rather than in this tree. The `GtkScrollInfo`
argument is "details of how to perform the scroll operation or %NULL to scroll into view", and
`GtkScrollInfo` carries exactly two booleans — whether the horizontal and vertical axes may move.
There is no alignment anywhere in it. **`scroll_to` is *into view*, and Table 234 says *first
visible*.** Qt's `PositionAtTop` states a position and GTK has no counterpart to it, which is the
one place in this pair of hosts where the toolkits genuinely differ in what they will say.

So the floor went back to `v4_10` and `load_from_data` with it. **What a feature floor costs is a
runtime requirement** — these crates link dynamically against the platform's own GTK — and it is
not worth raising for an API that does not answer the question. `Cargo.toml`'s comment carries the
experiment so that the next round does not repeat it.

## 3. Where a scrollbar's position is stated in GTK

`controls.rs::scroll_to_top_index` sets the `GtkScrolledWindow`'s own vertical
`GtkAdjustment`, which is what a scrollbar *is*. Three things decide the shape and each was
measured rather than assumed:

- **It acts on the first `changed` that has something to scroll.** `upper` and `page-size` are
  nothing until the view is allocated. The handler stands down for good once it has acted, so a
  person scrolling the list afterwards is not fought with, and it stands down where
  `upper <= page-size`, which is the clause's own "[f]or scrollable list boxes".
- **The row height is `upper / options`.** That is exact here rather than an estimate: every row of
  this control is built by one `GtkSignalListItemFactory` into one `GtkLabel`, so the rows are
  uniform by construction. A list whose rows differed would need the position of the row itself,
  which `GtkListView` exposes to nobody outside the widget.
- **The value is written from an idle, and that is the one line a reader would delete.**
  `GtkListView` is a `GtkListBase`, and a `GtkListBase` holds an *anchor item* and recomputes the
  adjustment from it every time it is allocated — so a value written while the geometry is being
  computed is overwritten by the anchor, which is item 0 until somebody scrolls. Measured both
  ways on the witness: written inside the handler the trace prints `upper=176 page=65 -> 22` and
  the list still starts at *Lorem*; written from an idle it starts at *Ipsum*. An idle runs after
  GTK's layout phase, where the adjustment moving is what **updates** the anchor rather than what
  the anchor overwrites.

An index the array does not have is declined outright, because the page's own appearance clamps
such an entry to the last option (ADR 0111's rule, and
`variable_text.rs::the_top_index_says_which_option_the_list_starts_at`) and a control scrolling to
its own end would be saying the same thing twice. The `min` in the arithmetic is the other end of
the same question and is a valid entry rather than a broken one: an index near the end of a long
list asks for a position past the furthest a viewport can scroll to.

## 4. The witness, which `doc/todo/30` said did not exist

That file recorded: *"No corpus document is known to state one; `examples/variable_text_census`
counts 10 list-box widgets and is where to look first."* The census counts list boxes and does not
count `/TI`, so the sentence was a statement about an instrument rather than about the corpus.
Decompressing every object stream in `doc/pdf.js` and grepping the result finds exactly one
document in 974, and it is a list box:

`doc/pdf.js/test/pdfs/annotation-choice-widget.pdf`, object 62 — `/FT /Ch`, `/Ff 2097152` (Table
233 bit 22 set, bit 18 clear: a multiple-selection **list box**), eight `/Opt` entries,
`/V [(Ipsum) (Amet) (Consectetur) (Elit)]`, `/I [1 4 5 7]`, **`/TI 1`**, `/TU (List box, multiple
selection)`. The `/Rect` fits three rows of the eight, so the entry has something to do.

It is the discriminating fixture as well as the only one: the same page carries a
single-selection list box and a read-only one with **no** `/TI`, so one screenshot shows a control
that obeys the entry beside two that have nothing to obey. And its `/TI` and its `/V` name
different options, which is the distinction §2 is about.

## 5. What was measured

`Xvfb :78` at 900×1100, the release binary, `xwd` after a key press (`doc/environment.md`'s rule
about photographing a window that has had no reason to repaint).

| | first visible option in the multiple-selection list |
|---|---|
| before | Lorem — option 0 |
| `v4_12` + `GtkListView::scroll_to`, `/TI 1` | Lorem — option 0 |
| `v4_12` + `GtkListView::scroll_to`, `/TI 5` | Lorem — option 0 |
| adjustment written in the handler, `/TI 1` | Lorem — option 0 |
| **after**, `/TI 1` | **Ipsum — option 1** |
| **after**, `/TI 5` | **Consectetur — option 5** |
| `viewer-qt`, same document, unchanged | Ipsum — option 1 |

The `/TI 5` document is the corpus witness with that one entry changed through `qpdf --qdf`, which
is a real document's structure with a different number in it rather than a hand-built fragment. The
last row is the point of the exercise: **the two native hosts now show the same first row on the
same file**, and they arrived there through two different toolkit mechanisms because one of the
toolkits will not state a position.

`viewer-ui` is level here by a third route and needs no change: it is the one host that does not
send `Command::Delegate`, so its list box is the page's own appearance, which has obeyed the entry
since ADR 0407.

## 6. What this changes about how a blocker is written down

`doc/todo/30`'s entry named a real fact (the method is 4.12), a real constraint (the crate binds
4.10) and inferred a blocker from the pair without asking what the method does. That inference
survived seventy-seven sessions. The rule it earns is small and is `doc/habits.md`'s shape:
**a blocker that names an API this tree has never called is a claim about somebody else's
documentation, and it costs one round-trip to check.** Raising the floor took a line; the answer
took one screenshot.

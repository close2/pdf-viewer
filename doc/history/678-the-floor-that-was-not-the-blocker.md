# 678 — The floor that was not the blocker, and the first ordering of the UI work

Date: 2026-08-23. ADRs [0508](../adr/0508-the-floor-that-was-not-the-blocker.md) and
[0509](../adr/0509-what-a-ui-round-does-next.md).

Touched: `crates/viewer-gtk/src/controls.rs`, `crates/viewer-core/src/query.rs`, `Cargo.toml`
(comment only — the dependency line and `Cargo.lock` are unchanged), `tools/state.sh`,
`doc/conformance/ledger.toml` (§12.7.5.4), `doc/todo/30-a-native-host.md`, `doc/ui-boundary.md`.

The first round taken on the project owner's *"even though low priority, I think we should start
investing time into the UI (and its API for the native versions)."*

## The item

`doc/todo/30` named two things as open. **One of them was not**: the colour of the gap between two
pages was decided in the six-hundred-and-eleventh session and is `pdf_render::SURROUND`, whose doc
comment says in as many words that it is *a documented choice, not a reading*, with the search over
§11.3, §11.4.5, §11.4.7, Table 141, §14.11.2 and Table 147 that establishes the standard states
nothing about the area outside a page, and with the two things that decide the value. Both native
hosts take it. Nothing was owed there, so the round took the other item.

## What the other item turned out to be

`viewer-gtk` did not obey Table 234's `/TI`, and the file recorded the reason as a *binding floor*:
`GtkListView::scroll_to` is GTK 4.12 and the workspace binds `v4_10`. The floor was raised — it is
one line, `gtk4` 0.11.4 offers the feature and this machine runs 4.22 — and the method **moved
nothing**, at `/TI 1` or at `/TI 5`. GTK's own documentation says why: the call puts an item *into
view*, its `GtkScrollInfo` carries two booleans about which axes may move and no alignment, and
*into view* leaves an option that is already visible where it is. Qt's `PositionAtTop` states a
position and GTK has no counterpart.

So the floor went back to `v4_10` — what a feature floor costs is a *runtime* requirement, and it
is not worth raising for an API that does not answer the question — and the entry is obeyed through
the `GtkScrolledWindow`'s own adjustment. The one line that is not obvious is measured rather than
reasoned: a `GtkListBase` recomputes the adjustment from its *anchor item* at every allocation, so
a value written in the `changed` handler is overwritten (the trace prints the value set correctly
and the list still starts at option 0), and it has to be written from an idle, after GTK's layout,
where the adjustment moving is what updates the anchor.

**And the witness the file said did not exist, does.** *"No corpus document is known to state one"*
rested on a census that counts list boxes and does not count `/TI` — a statement about an instrument
wearing the clothes of a statement about the corpus. Decompressing every object stream in
`doc/pdf.js` finds one: `annotation-choice-widget.pdf` object 62, a multiple-selection list box with
eight options, `/TI 1`, and a `/V` naming four *different* ones, on a page that also carries two
list boxes with no `/TI` at all. One screenshot shows the entry obeyed beside two controls with
nothing to obey, and the two native hosts now show the same first option on that file.

## The part the owner actually asked for

`doc/todo/30` gains an ordered list — seven items with the criterion that ranks them — and ADR 0509
has the argument. The survey behind it found three things worth naming here:

- **"All three hosts stay level" is false in both directions today**, and there are four consumers
  rather than three. `viewer-ui` is ahead on everything that *reads* a document and behind on
  everything that *changes* one, and its password prompt reads `stdin` and exits the process when
  there is no terminal.
- **No host can copy a selection out of the program.** All three windowed hosts draw one;
  `viewer-ui` keeps an in-process `String` and its own comment says a native host "owns that end by
  construction", and neither native host took it. That is item 1, and it needs no message.
- **"The ABI's entry points are the whole vocabulary" has decayed**, so it is counted now rather
  than claimed: `tools/state.sh hosts` is new and prints how much of `Command` and `Query` a C
  caller can reach, naming what it cannot. The sharpest instance is a C caller that can run Annex
  O's document-wide search and cannot draw a match.

One defect fell out of the survey and is fixed: `Query::Fields` and `Answer::Fields` both said "the
page being shown" while `Viewer::form_fields` has walked every page of the arrangement since
`Command::Layout` existed. The code was right and the documentation was wrong, which is the shape
that costs a *host author* rather than a gate — a host following the comment would place controls
for one page of a column and leave the rest of the form with holes in it, because
`Command::Delegate` has already taken the appearances away.

## What was run

`fmt`, `clippy --workspace --all-targets` under `-D warnings`, `nextest --workspace`, the workspace
doctest, the fuzz targets' `check`, `cargo test -p conformance`, and both censuses — the change map
puts `viewer-core` under `selection_census` and `accessibility_census` and `viewer-gtk` under the
core. The `pointers` and `quotations` sweeps were run because the round moved documents; neither
names anything this round wrote. §5's binaries were rebuilt and installed, which the round owed
before measuring anything.

The `/TI` measurements are `Xvfb :78`, the release binary, `xwd` after a key press. **Nothing here
needs the owner's session**: a GTK window under `Xvfb` is a real window with a real event loop, and
the finding is about which row a list starts at rather than about a swapchain. There is nothing
queued for the measurement loop.

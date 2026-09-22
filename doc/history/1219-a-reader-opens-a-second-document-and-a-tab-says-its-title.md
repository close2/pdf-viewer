# 1219 — A reader opens a second document, and a tab says its title

The HOST-UI round of batch thirty-four. ADR 1275.

## What was built

**Ctrl + O, in all three windows.** `WindowAct::OpenDocument` is the key table's row. Each window
shows its own control for it: a `gtk4::FileDialog`, a `QFileDialog` (a bridge flag, `open_chosen`),
and in `quorra` a line over the page, where the path is typed. **Every path after the first on a
command line** opens as a tab behind the first, starting at the first frame. Every route goes
through `viewer_host::open_chosen`, the one gate, and then through `viewer_host::Arrivals`, one
document at a time. Each opens under `Command::Open`, and `Viewer::open` now calls `adopt`, so
there is one list of the reader's answers. **A tab says §14.3.3's `/Title`**
(`viewer_host::documents::titled`), taken from the Properties answer the document panel already
asks for. The command-line word is one shared reading, `viewer_host::Named`.

## What driving the windows found

All three windows were driven by hand under `Xvfb :91`, with three generated documents on the
command line: two with `/Title`, one without. The test was a Ctrl + Tab through each tab, then
Ctrl + O on a fourth document, with screenshots throughout. `quorra` ran with `--cpu`. The tabs
built in round 1213 had never been driven, and doing so found six defects:

- a page request was keyed by page alone, in `viewer_host::Drawing` and in `Viewer::rendered`;
- `quorra` kept one list of drawn pages for the whole window, not one per tab;
- GTK used Ctrl + Tab for focus movement before this program saw it;
- GTK's `switch-page` idle swapped two tabs back and forth without end;
- GTK built panels before `Command::Focus` had run;
- `quorra` drew its tab strip over the find bar.

All six are fixed. Only the first has tests, because no gate drives a toolkit and the other five
live in a window's own event handling. ADR 1275 section 5 describes them.

## Left owed

- GTK prints "the host was busy, so page 1's row was drawn without its /Thumb" once per tab that
  arrives. The pages list binds a row while the host is borrowed, because the view moves between
  notebook pages. The fixtures state no `/Thumb`, so whether a real one is lost was not seen.
- No test drives a dialogue, which is the same cost ADR 1240 records.
- `quorra-confined` still holds one document, per ADR 1190.

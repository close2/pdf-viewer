# 638 — The window two other clauses had been holding

**Finding.** §12.4.4 says nothing about a window, and for four hundred sessions this tree read that
as the standard saying nothing about a window — so a presentation played inside a sidebar and a
status line. Table 29 and §12.2 had been holding it the whole time: `/PageMode /FullScreen` is
"[f]ull-screen mode, with no menu bar, window controls, or any other window visible", stated as one
of the six ways a document "shall be displayed when opened", and Table 147 adds `/HideToolbar`,
`/HideMenubar`, `/HideWindowUI` and `/NonFullScreenPageMode` for the way back out.

Date: 2026-08-21. ADR: [0470](../adr/0470-the-window-a-presentation-never-had.md).

Touched: `crates/viewer-host/src/presentation.rs` (new), `crates/viewer-host/src/lib.rs`,
`crates/viewer-ui/src/bin/pdf-viewer{.rs,/app.rs,/presentation.rs,/window.rs}`,
`crates/viewer-gtk/src/host.rs`, `crates/viewer-qt/{src/bridge.rs,src/host.rs,src/keys.rs,
cpp/window.h,cpp/window.cpp}`, `crates/pdf-model/src/viewer_preferences.rs`,
`crates/pdf-model/src/navigation.rs`, `crates/pdf-model/examples/presentation_fixture.rs`,
`doc/conformance/ledger.toml` (§12.2, §12.4.4, §12.4.4.1, §12.4.4.2), `doc/todo/30`, `doc/todo/32`,
`doc/ui-boundary.md`, `doc/running-the-viewer.md`, `doc/environment.md`.

## What the round did

`viewer_host::Presenting` is the decision — five sentences from two clauses, no toolkit — and all
three hosts adopt it, which is `doc/todo/30`'s "all three hosts stay level" spent on its second
item. `p` enters, Escape leaves, a document stating `/PageMode /FullScreen` opens presenting, and
§12.2's three hide flags are obeyed by the two native hosts whether a presentation is running or
not.

**Nothing was added to the boundary and no variant changed shape**, which is the second time that
has been the whole answer (the six-hundred-and-seventh was the first). Every channel already
existed and each because a *clause* had needed one: `Command::Present` from ADR 0316,
`Query::Opening` and `Query::Preferences` from the hundred-and-thirty-seventh session.

## Two errata, one of which decides what a person sees

`spec-errata emit` over clause 12 before writing anything, which is the habit five of the last ten
rounds have been repaid for.

**Issue #275** inserts a `UseAttachments` row into `/NonFullScreenPageMode`'s own value cell in
Table 147. `pdf_model::viewer_preferences` refused that name there, on the comment "a name this
entry does not define" — and the printed table is simply short by one row. The refusal of
`FullScreen` beside it was right and stays, because Table 147 makes that name the entry's
*condition*. The fixture this round writes states `/NonFullScreenPageMode /UseAttachments`, and all
three hosts were driven leaving full screen onto §7.11.4's panel, so the erratum is on the screen
rather than in a footnote.

**Issue #36** tightens Table 164's `/Di` from *number* to *integer*. `Direction::Degrees` keeps an
`f32` and is therefore more permissive than the corrected table; recorded rather than changed, with
the erratum cited where the type is. Refusing a page over the type of a number that decides only
which way a wipe travels would be enforcing a grammar at the cost of a picture.

## What the screen said, and what it could not

Under `Xvfb :78` at 1200×900. `viewer-gtk` and `viewer-qt` both photograph as the page and
`pdf_render::SURROUND` edge to edge — no panel, no tool bar, no status line, no titlebar — and
pressing `p` brings all of it back on the **Files** tab. `viewer-ui` prints the same sequence and
opens Files after Escape with §8.11's bullets put back, which is §12.4.4.2 NOTE 2 visible.

Two things the instrument could not show, and both are now in `doc/environment.md` because both
cost this round time:

- **There is no window manager on this machine.** `mutter` is Wayland-only in this build. Full
  screen on X11 is `_NET_WM_STATE_FULLSCREEN`, a *request* to a window manager, so
  `GtkWindow::fullscreen`, `QWidget::showFullScreen` and `winit`'s `set_fullscreen` all leave the
  window the size it was. The chrome can be photographed; the extent cannot.
- **`xwd` returns stale content for a window that has not repainted.** Four captures were
  byte-identical across a rebuild *and* across two different GTK renderers, and showed chrome the
  program had already taken away. The picture changed only after a key press. A screenshot of a
  window with no reason to redraw is a photograph of the past, and it reads exactly like a change
  that did not work — which is very nearly the mistake this round wrote a code comment about
  before catching itself.

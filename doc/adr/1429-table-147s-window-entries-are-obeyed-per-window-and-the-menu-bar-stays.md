# 1429 — Table 147's window entries are obeyed per window, and the menu bar stays

Session 1297. Status: **accepted**.
Context: `crates/viewer-host/src/presentation.rs` (`Placing`, `Presenting::place`, `fitted`,
`centred`), `crates/viewer-host/src/documents.rs` (`document_title`),
`crates/viewer-ui/src/bin/quorra/presentation.rs`, `crates/viewer-gtk/src/host.rs`,
`crates/viewer-qt/{src/host.rs,src/bridge.rs,cpp/window.cpp}`.
Amends: ADR 1145 section 3 (what the three windows do with Table 147), ADR 0470.
Clauses: ISO 32000-2 §12.2 (Table 147).

## 1. What was claimed and what was so

`quorra`'s `report_unobeyable_chrome` said "the GTK and Qt hosts obey all three" of `/HideToolbar`,
`/HideMenubar` and `/HideWindowUI`, and §12.2's ledger note said `/HideMenubar` "is executed by
having nothing to hide — none of the three hosts draws a menu bar". Both were false since ADR 1145
gave `quorra-gtk` and `quorra-qt` a menu bar holding the reader's restriction levels. Driven under
`Xvfb` with one fixture stating all six window entries, one stating none and one stating only
`/DisplayDocTitle` (session 1297's record has the figures), this was what each window did:

| entry | `quorra` (winit) | `quorra-gtk` | `quorra-qt` |
|---|---|---|---|
| `/HideToolbar` | has none; says so | header bar's buttons and find bar hidden | both tool bars and find bar hidden |
| `/HideMenubar` | has none (the levels are a card, `r`); says so | Restrictions menu kept; `NOT_THE_DOCUMENTS_TO_HIDE` said | menu bar kept; the same sentence |
| `/HideWindowUI` | has no scroll bars or controls; says so | status line hidden (the page area has no scroll bars) | status bar hidden (likewise) |
| `/FitWindow` | ignored | ignored | ignored |
| `/CenterWindow` | ignored | ignored | ignored |
| `/DisplayDocTitle` | obeyed (ADR 0186) | ignored: the file's name | ignored: the file's name |

## 2. The reading

Table 147's first five entries carry no modal verb of their own: each is "[a] flag specifying
whether to" hide, resize or position. What makes them obligations is the clause's own sentence —
the dictionary is "controlling the way the document shall be presented on the screen or in print".
`/DisplayDocTitle` carries its own: "the window's title bar should display the document title taken
from the dc:title element of the XMP metadata stream", and otherwise "the title bar should instead
display the name of the PDF file containing the document".

ADR 1145's argument is about one entry and one widget: a document that could hide the menu holding
the reader's restriction levels would be taking away the control over what that document may do,
against `CLAUDE.md` principle 3's "it shall always be possible to turn them off". It says nothing
about a title bar, a window's size or its position. So the three ignored entries are **gaps**, not
that departure, and are built:

- **`/DisplayDocTitle`**: `viewer_host::documents::document_title` is the one rule — `dc:title`,
  then §14.3.3's `/Info /Title` where the stream states none or could not be read (ADR 0186) — and
  all three windows take it. The two native windows read §14.3.2's stream only for a document that
  asks, so a document that does not costs its opening nothing.
- **`/FitWindow`**: the size of "the first displayed page" is known only once it is displayed, so
  `Presenting::place` answers at a frame, and `fitted` keeps the chrome and makes the page's
  viewport the page's drawn extent, held to the screen. **It is owed until the window fits**,
  bounded at four frames and to the first displayed page: a window that hides a tool bar or a
  status line draws its first frame before that chrome has left the viewport, so the first fit is
  measured against a viewport about to grow. Measured under `Xvfb`: `quorra` fits in one frame,
  `quorra-gtk` and `quorra-qt` in two. **`quorra-gtk`'s page `GtkFixed` became an overlay child**
  over an unmeasured ground, because a `GtkFixed` measures its children's union and GTK 4 will not
  size a window below its natural size: a page 812 pixels tall centred in 1019 held the viewport at
  915, and each retry halved the gap rather than closing it. `set_default_size` measures the
  surface, shadow included, and the host adds it.
- **`/CenterWindow`**: `centred` over the screen the window is on, once the window fits. `quorra`
  asks winit's `set_outer_position` and `quorra-qt` `QWidget::move`. **`quorra-gtk` cannot**: GTK 4
  removed the call that placed a top-level window, and on Wayland no client places its own window
  in any toolkit. Reaching past the toolkit to X11 would add an `unsafe` Xlib path to a host for one
  flag on one platform, which is not proportionate; so it is said out loud when a document asks
  (`NO_POSITION_TO_SET`, trap 5). That is a departure the toolkit made, recorded here as one, and
  §12.2's note names it beside ADR 1145's.

**What stays ADR 1145's**: `/HideMenubar` in the two native windows, unchanged. `quorra` has no
menu bar; its sentence now says what it has rather than what the other two do.

## 3. Consequences

- `Presenting` carries `Placing` and the frames asked; `Presenting::place(page)`, `owes_placing`,
  `placed`. A page other than the first displayed one ends the owing, so a later page turn never
  resizes a window a person has settled.
- `quorra-qt`'s bridge gains `QtUpdate::placement` and `place_window`, which takes the window's
  extents and answers `[resize, width, height, move, x, y]`; Rust never calls a Qt object.
- `quorra-qt` no longer hands a machine face to Qt before the first frame (trap 70): measured with a
  Chinese outline title at launch, a face that landed first put a five-megabyte
  `addApplicationFontFromData` in front of the first frame in one run of four.
- `launch_path` and `raster_golden` are unmoved: nothing here runs before the first frame, and no
  pixel of a page is decided here.

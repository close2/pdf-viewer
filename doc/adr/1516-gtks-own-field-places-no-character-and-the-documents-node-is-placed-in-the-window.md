# 1516 — GTK's own field places no character, and the document's node is placed in the window

Session 1340. Status: **accepted**.
Context: `viewer-gtk`'s `access.rs` (`Host::attend`, `Host::speak`) and `Host::viewport_origin`;
`viewer_accessibility::Reading::at` and `DocumentView::origin`, which the document node carries as
its transform; `tools/drive-windows.sh` step `29-field-extents`. Follows ADR 1501, whose drive left
GTK "not offered".

## 1. Why the GTK window answered an error (GTK 4.22.5, `gtk4-rs` 0.11.4 at `v4_10`)

The field a person types into is GTK's own `GtkEntry`, placed over the widget's `/Rect`; the page
area beside it publishes the document's tree through AccessKit. Read in GTK's source at the tag
installed here (`gtk/a11y/gtkatspitext.c`, `gtk/gtktext.c`, `gtk/gtktextview.c`,
`gtk/a11y/gtkatspiutils.c`):

- AT-SPI's `Text` reaches a widget through `GtkAccessibleText` where it implements that interface
  and through `GtkEditable` otherwise, and the editable route answers `GetCharacterExtents`,
  `GetRangeExtents` and `GetBoundedRanges` with `NOT_SUPPORTED` and an empty message. `GtkEntry`
  is only an editable; the `GtkText` inside it implements `GtkAccessibleText` but its class role is
  `NONE`, so it is not on the bus. That is the drive's "an error with no message".
- Built alone with the text-box role (tried, and driven), `GtkText` answers — in window coordinates
  only — with a 0 × 0 box at the field's origin: the handler asks `get_extents (offset, offset)` and
  `GtkText` reads the pair as an empty range, and reads character offsets as byte offsets. GTK's
  `main` has the same two lines after MR 10409 (2026-09-28). A wrong box is worse than an error
  (trap 5), so the entry stays.
- `GtkTextView`, the multi-line field, reads the same pair as running to the end of the line.
- Every GTK 4 text widget refuses screen coordinates ("Unsupported coordinate space"), and a
  component's screen origin is reported as 0, 0.

The tracker holds no open issue for the entry's case: #4942 ("not supported in gtk4", 2022) was
closed by MR 4754 for `GtkTextView` alone, and MR 7104 (2024) corrected sizes. A widget of this
tree's own implementing `GtkAccessibleText` could answer, but its `get_extents` is GTK 4.16
(`AccessibleTextImpl::extents` behind `v4_16`), above this crate's floor and above the 4.14 that
CI's `ubuntu-latest` builds against, and it would have to read GTK's `(offset, offset)` as one
character. That is the GTK limit, priced and not built.

## 2. What was this tree's, and is fixed

The document's own node for the field (ADR 1501) answered `GetCharacterExtents(1)` in the GTK
window — at 82, 376, while GTK's entry for the same field sat at 447, 406. AccessKit reads a node's
bounds as the window's, and this host's are the viewport's: the page area's place beside the panels
never reached the bridge. `Host::viewport_origin` asks GTK where the page area is in its root, and
`Reading::at` hands it to `tree::build`, which sets it as the document node's transform — so a
client is told every node's place in the window, the space GTK's own widgets answer in, while a
request still resolves against the untransformed bounds, in the viewport, where the host acts. A
change of that place re-publishes the tree, as a change of page does.

Driven (release, Xvfb, at-spi): the document node's character 1 at 463, 422, 9 × 17, inside GTK's
entry at 447, 406, 299 × 40. `29-field-extents` now asks the toolkit's field and the document's
node, each in the screen's coordinates and then the window's, and in GTK reads the document's
box against the entry's place, naming the entry's refusal beside it.

## 3. Left

`viewer-qt` places its bridge by the main window's client area (`QWidget::geometry`), not its page
area's, so its document nodes are likely offset the same way; its step reads `QLineEdit`, which
answers, and nothing measured the other. The construction above is the remedy there too, with the
page area's position sent from C++.

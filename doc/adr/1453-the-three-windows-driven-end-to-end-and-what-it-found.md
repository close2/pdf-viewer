# 1453 — The three windows driven end to end, and the six things that were wrong

Session 1309. Status: **accepted**.
Context: `tools/drive-windows.sh` (new), `crates/pdf-model/src/view.rs` (`ViewState::save`,
`Update::write_state`, `toggles`, `state_name`), `crates/viewer-ui/src/bin/quorra/overlays.rs`
(`highlight_list`, `WASH`), `crates/viewer-gtk/src/host.rs` (`Host::key`,
`Host::page_takes_the_keyboard`, the find entry's Shift+Enter), `crates/viewer-gtk/src/pages.rs`,
`crates/viewer-qt/cpp/window.cpp` (the find bar's Enter).
Clauses: ISO 32000-2 §12.7.5.2.3, §12.7.5.2.4, §12.3.4, §12.5.1, §7.5.6.

## 1. What was done

Every step a reader takes with an open document — open, Table 147's window entries, the outline,
page turns by key and wheel, zoom, find, a popup, a link, a markup, a save read back, the pages
panel, the restriction levels, print, §7.6.4.1's password, a form's tab order, check box, choice
and push button saved and re-opened, and §14.7's tree on AT-SPI — was driven in `quorra`,
`quorra-gtk` and `quorra-qt` under `Xvfb` on four fixtures the script writes, and every screenshot
was looked at. The list and its observables are `doc/verify.md`'s; the script makes them one
command.

## 2. What was wrong, and the decision for each

- **A saved check box drew unticked when the file was opened again, in all three windows.**
  `ViewState::save` wrote the edit's appearance-state name as a *string* `/V` and left `/AS`
  alone. §12.7.5.2.3 makes `/V` "a name object representing the check box's appearance state" and
  says "[t]he value of the V key shall also be the value of the AS key. If they are not equal, then
  the value of the AS key shall be used instead of the V key to determine which appearance to use."
  So the file said ticked in one entry and drew the other. A toggling button's value is now
  written as a name, and each of its widgets' `/AS` is that name where the widget's `/AP` `/N`
  holds a state of it and `Off` where it does not — which is §12.7.5.2.4's radio set, one widget on
  and its siblings off, from one value. `pdf-model`'s `tests/saving.rs` holds both directions.
- **`quorra`'s selection, find matches and Annex O rectangle hid the words they marked.** The
  wash was drawn with `Multiply` into an overlay layer over a transparent backdrop, and only then
  composited over the page; a multiply against nothing is the colour itself, in both lanes. The
  wash is now the colour at 0.45 of its opacity in the normal mode. A blend that reaches the page
  would need the overlays drawn into the page's own layer, which is a compositor change for one
  appearance; a translucent wash is what the two native windows already draw.
- **In `quorra-gtk` the keyboard left the page.** GTK gave the outline's `GtkListView` the focus at
  launch and after any press on the page (which no GTK widget can take), and moved it there on an
  arrow key nobody claimed; the list then took Home, End, `+` and `-`. The page area is now
  focusable, takes the keyboard as the window opens and on a press outside a placed control, and a
  key the shared table gave a meaning stops at the window — except §12.5.1's Tab, whose GTK focus
  move `follow_focus` already corrects. Trap 57's shape, on keys other than Tab.
- **`quorra-gtk`'s `r` put up an empty menu.** `MenuButton::popup` runs the create-popup hook that
  fills the menu synchronously, through the host the key handler was holding, so the fill was
  dropped with "the host was busy". The popup is asked from the idle queue, as the find bar's
  reveal already is.
- **`quorra-gtk`'s pages panel moved on a double click only.** §12.3.4's thumbnails are for
  "allowing the user to navigate to a page by clicking its thumbnail image"; `GtkListView`
  activates on a double click unless told otherwise. It is told.
- **Shift+Enter in the find bar meant three things**: the previous occurrence in `quorra`, nothing
  in `quorra-gtk` (Ctrl+Shift+G is `GtkSearchEntry`'s), the *next* one in `quorra-qt`. The standard
  describes no find bar, so this is a choice: Shift+Enter is the previous occurrence in all three,
  and each toolkit's own key stays.

## 3. What was found and left

- **A right-to-left word on the page is not found** in any window: `ArabicCIDTrueType.pdf` maps
  its glyphs to Arabic presentation forms (U+FExx) and shows them in visual order, so the readback
  holds `ﺔﻴﺑﺮﻌﻟا` where a person types `العربية`. `viewer_core::select::find` compares character for
  character after lower-casing. Folding needs the presentation forms' decompositions — which
  `pdf-font` already compiles in for shaping — and the readback in logical order, which §14.8.2.3
  gives a tagged document and nothing gives an untagged one. `doc/todo/27` carries it.
- Control and the wheel zoom in `quorra` and scroll in the two native windows; `quorra-qt`'s popup
  body has no fill or border, so on a white page its extent is invisible. Neither is a clause.

## 4. Consequences

`launch_path`, the censuses and `save_round` are unmoved by construction: nothing here runs before
the first frame, and the save change is to the two entries a toggled widget writes.

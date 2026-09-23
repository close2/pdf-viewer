# 1303 — A window is the front document's, and a transition is drawn on the window's ground

Session 1233. Status: **accepted** and built.
Context: `crates/viewer-host/src/clock.rs` (`face`), `crates/viewer-host/src/restriction.rs`
(`Restrictions::headings`, `Subject`), `crates/viewer-gtk/src/{host.rs,pages.rs}`,
`crates/viewer-qt/src/{host.rs,bridge.rs}`, `crates/viewer-qt/cpp/window.cpp`,
`crates/viewer-ui/src/chrome.rs` (`RestrictionsCard::press`),
`crates/viewer-ui/src/bin/quorra/{app.rs,dispatch.rs,overlays.rs,presentation.rs,window.rs}`.
Builds: ADR 1275 (a document opened beside), ADR 1291 (the submission's levels), ADR 1299 (the
four styles), ADR 1145 (the menu).
Clauses: ISO 32000-2 §7.7.2 (Table 29), §12.4.4 (Table 164), §12.3.4 and §7.7.3.3 (Table 31),
§12.7.6.2.

Every item below was found by driving a window under Xvfb with two documents open, and none of it
was visible to a test that opens one.

## 1. A document opened behind obeys `/PageMode` when it first comes to the front

Table 29's `/PageMode` is "how the document shall be displayed when opened", and a document opened
behind the one showing is not displayed. All three windows applied it on `Event::Opened` for the
newcomer, while it was briefly focused, so a presentation running when a second document arrived
lost its full screen to that document's `UseNone`, and a `UseThumbs` document behind changed the
panel of the one in front. The layout is still taken at once, because it is the tab's own. The
rest waits on a `catalog_due` flag carried in each window's per-tab state, and is obeyed at the end
of the pump that first runs that tab's `Command::Focus`, when the core answers `Query::Opening`
for it. Only the front document's page becomes a transition's outgoing face: the Qt window had
been animating from the behind document's first page.

## 2. Full screen hides the strip of tabs, and a hidden strip takes no keys

Table 29's `FullScreen` is "no menu bar, window controls, or any other window visible". The strip
is a window control. In GTK, while the notebook's tab held the keyboard, Right both turned the
presented page and changed the document. All three windows now hide the strip under full screen
and show it again on the way out.

## 3. A transition's faces are rasterised on `Medium::WINDOW`

A face is the whole viewport. `CpuRasterizer::new()` treats its target as the page, so every
transition frame had a white surround where the window shows `pdf_render::SURROUND`.
`viewer_host::face` is now the one rasterisation the three windows share.

## 4. The menu's groups are the rows' own, and a question is titled by what it asks

The Qt bar was built from `Scope::ALL`, which has two entries, while the rows have three groups.
The third group, whose one act is sending a form, therefore had no menu, and the submission's
level could not be set in Qt at all. `Restrictions::headings` takes the headings from the rows.
The bar carries each label alone, and a group's note is the first line inside its menu. Qt
titled every question *Restricted*. `Subject::title` is the one set of titles, and GTK uses the
same set.

## 5. `quorra`'s cards are modal to the pointer, and the menu answers a press

A press behind the question card reached the page, where it could press the button that asked
the question. The question and the menu now take the pointer the way they take the keys.
`RestrictionsCard::press` hit-tests the same layout that `draw` uses, then chooses through
Enter's own path. That discharges `doc/todo/38`'s first remaining item, and the choice that item
recorded against a pointer is withdrawn: a menu is chrome, but a press on chrome this window draws
is as much this window's to answer as a key is.

## 6. A GTK row bound while the host is held is asked for again

When a tab arrives, its list is laid out while the host is borrowed. That list is then replaced
by one built from the idle queue, so no `/Thumb` a reader would see was lost. It was only the first
draw of a list that was about to be discarded. `pages::bind` still asks for the row again from the
idle queue, so a list that survives is never left without its picture. The sentence it printed is
now a `--trace=panel` line.

## Not done, and why

- GTK's first transition after launch is drawn at the viewport the page had before full screen
  took the chrome away, and the page then jumps to its new size. From the second transition on it
  is steady. The core has not heard the resize by then, and this round did not trace it.
- A click on the first heading of Qt's menu bar under Xvfb chose "copy: off" three times in five
  tries, and not afterwards. That was not reproduced reliably, so it is recorded here rather than
  fixed.
- The status line says "import-data:" for a submission's FDF answer, because `Command::Respond`
  shares §12.7.6.4's notes. The wording lives in `viewer-core`.

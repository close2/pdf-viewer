# 1418 — A Qt panel row is handed the machine's face as a tab is

Session 1290. Status: **accepted** and built.
Context: `crates/viewer-qt/src/host.rs` (`rows`, `page_row`, `ask_machine`, a test),
`crates/viewer-qt/cpp/window.{h,cpp}` (`pumpFaces`, `rebuildPanels`).
Extends: ADR 1406 section 2 (the tab strip) to the panels. Clauses: ISO 32000-2 §12.3.3, §12.4.2.

## 1. What was driven

`quorra-qt` under `Xvfb` (`QT_XCB_NO_XI2=1`) with an outline whose titles are "多边形批注",
"الفصل 12: السلام" and "Chapter 3": the Arabic item drew joined and right to left through Qt's own
fallback, and **the Chinese item drew as five boxes** — ADR 1406's cause, reached in a `QTreeView`
rather than a tab bar: Qt falls back by family, and the family that states the characters here
shares its name with Latin faces. `quorra-gtk` drew all three (Pango), and so does `quorra` (its
chrome asks `viewer_host::machine_faces` for every character, ADR 1406).

## 2. Decision

The panels take the tab strip's answer. `Host::rows` and `Host::page_row` ask `machine_faces` for
every character of a row's label and detail that `compiled_in_lacks`, the same test the strip
applies; `rebuildPanels` arms the look for the answer, which stays stopped where nothing was
asked; and `pumpFaces` sets every panel view — each tree, §12.3.4's list and §12.3.6's scatter — in
the platform's family followed by every family it registered, and resizes each tree's first column.
Nothing is searched on the event thread, and a document whose rows the compiled-in face covers arms
no timer at all.

The Arabic item did not need this and is not why it exists: registering a covering file changes
nothing Qt already drew, because the platform's family stays first.

## 3. Measured

Driven again after: the Chinese title drawn, the Arabic one unchanged. `host::tests::
a_chinese_outline_title_asks_the_machine_for_a_face` asks for nothing before the rows are, then
finds the title's characters asked for and answered by a face (skipping with a sentence on a
machine that offers none, as ADR 1154's tests do).

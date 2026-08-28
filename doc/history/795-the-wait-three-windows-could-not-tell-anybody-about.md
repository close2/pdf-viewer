# 795 — The wait three windows could not tell anybody about

`doc/todo/15`'s longest-standing remainder, carried out: the owner's *"warn the user and allow the
user to abort, however don't block"* reaches the three established windows through
`viewer_host::keys`, so a draw a person is waiting for now says so and offers Escape — and two
things had to be found before it could work at all, one of them a key the Qt host had been
swallowing ever since the table was written (ADR 0729). 2026-08-28.

## Why this item

The briefing offered four remainders. The real-adapter measurement is the owner's session; moving
the established windows onto the confined boundary is several rounds; breach-as-refusal needs a
fallible allocation on a path `viewer-confined` does not own. The warn-before-abort input is the
one that is a whole piece: a decision in `viewer-host`, three windows adopting it, and an
instrument (`Xvfb` plus the amplification fixture) that can show it working end to end.

## What was decided

**A clock is admissible for a warning and was not for a deadline**, which is the whole of
`viewer_host::drawing::WARN`'s argument. ADR 0657 refused a duration because none separates the
two populations; that is still true and was measured again this round rather than quoted —
`examples/host_draw` at twice device scale over `doc/pdf.js`'s 957 first pages gives a median of
1.8 ms, a p99 of 85.6 ms, a slowest of 315.9 ms and **nothing at all over half a second**, against
27 600 ms for a 1567-byte document written to be expensive. What differs is the failure cost: a
deadline that fires wrongly throws away a page, and a warning that fires wrongly is a line of
chrome. So a second — three times the slowest legitimate first page — decides when a *sentence*
appears, and nothing raises an interrupt when it passes.

**Escape, and only while the window is saying so.** `keys::Waiting` is `meaning`'s fourth
argument and moves exactly one row; a test walks the whole of `Key::ALL` to show that. A
presentation still leaves full screen first, because Table 29's `FullScreen` forbids the sentence
that offers the key and a key acting on an unseen offer is the guess the sentence exists to
remove.

**An abort reports nothing** (trap 20), leaves the queue behind it alone, and says what that costs
— `viewer_host::stopped_drawing` names the re-draw, because a person told only that something
stopped would conclude the program had given up on the page.

## Two findings, neither of which was the feature

- **`viewer-qt` forwarded Escape to the shared table only in full screen.** The key arrives
  through a `QAction` shortcut — a shortcut consumes a press before `keyPressEvent` sees it — and
  the action closed the find bar or returned. So §12.4.2's clear-the-selection row, which
  `viewer_host::keys` has stated for all three hosts since ADR 0526, had never reached that host
  at all. Found by pressing the key at a window that was showing this round's own warning and
  watching nothing happen. **A shared key table is only as level as the narrowest path a key takes
  to reach it**, and no instrument in this tree looks at that path.
- **On `viewer-ui`'s composing surface an abort without a record is a loop.** That host asks for
  its own frame once a tick for as long as the pixels on hand are not of this view, so the
  abandoned frame was asked for again at the next tick, warned about a second later and stopped
  again — photographed doing it three times over before `Composer::declined` existed. A native
  window needs no such field, and the difference is the sharper half: a viewer's token never
  answered is never re-issued, and a host's own ask is.

A third, smaller: `viewer_host::status`'s module documentation said the third host showed its
sentences in "a line of chrome". There is no such line and never was — `viewer-ui` has cards, a
find bar, a sidebar and a title bar — so the paragraph is corrected in place, with the correction
kept rather than tidied away.

## What was built

- `crates/viewer-host/src/drawing.rs` — `WARN`, `Drawing::overlong`, `Drawing::abandon`, and the
  module's rule grown from two ways of taking the thread back to three.
- `crates/viewer-host/src/keys.rs` — `Waiting`, `WindowAct::AbortDrawing`, Escape's third row, and
  the argument for all three in the module documentation.
- `crates/viewer-host/src/status.rs` — `still_drawing`, `stopped_drawing`, `drew_after_all`.
- `crates/viewer-gtk/src/host.rs`, `crates/viewer-qt/src/host.rs`,
  `crates/viewer-qt/cpp/window.cpp`, `crates/viewer-ui/src/bin/pdf-viewer/{app,composer,surface,
  window}.rs` — the three windows, and the Qt key path's fix.
- Seven new tests, each calibrated against an injected defect (trap 13; ADR 0729 has the table).

## Proof

Driven under `Xvfb` on the release binaries, on the amplification fixture written out to a file.
Both sentences photographed in the GTK status bar and in the Qt one; the flagship's are in its
title bar, where it already puts what the pages on the screen could not draw. In all three the
window answered a key press throughout, the abort returned the drawing thread mid-draw, and a
zoom afterwards re-asked the page and warned about it again. An ordinary five-page document warns
about nothing.

## What is still owed

`doc/todo/15` keeps breach-as-refusal, the move of the established windows onto the confined
boundary, and ADR 0725's real-adapter measurement. `Content::Refused` outliving a zoom in the
confined window — noted by round 790 — was **not** taken: it is a different window and a different
file, and folding it in would have made this round two changes rather than one.

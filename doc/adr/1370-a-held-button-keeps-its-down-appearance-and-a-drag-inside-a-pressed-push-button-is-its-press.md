# 1370 — A held button keeps its down appearance, and a drag inside a pressed push button is its press

Session 1266. Status: **accepted** and built.
Context: `crates/viewer-core/src/viewer.rs` (`pointer`, `is_push_button`).
Amends: ADR 1357 section 4 (the GTK cost it recorded). Keeps: ADR 0424.
Clauses: ISO 32000-2 §12.5.5, §12.5.1, §12.7.5.2.2, Table 226, Table 229.

## 1. The finding

§12.5.5 (not Table 168, which the brief named and which is the border style dictionary): "[t]he down
appearance shall be used when the mouse button is pressed or held down within the annotation's
active area." GTK's motion controller reports a `Moved` right after each press, and `pointer` read
every `Moved` as "button up", so the down appearance was dropped before release.

Driving it found a second defect. GTK's drag gesture also reports a `Dragged` at the press's own
point. The press anchors the selection in the readback of the page drawn with the normal
appearance; the drag reads an offset in the page drawn with the down appearance. Where the down
appearance carries text of its own, the two offsets name different characters, the click became a
non-empty selection, and the push button's `/A` was withheld.

## 2. Decisions

- A `Moved` between a press and its release is a move with the button held. Over the annotation the
  press went down on it keeps `/D`; elsewhere it shows neither `/D` nor a rollover, and it counts as
  button down for §12.6.3's `/E`.
- A `Dragged` still inside a pressed **push button** showing its down appearance does not move the
  selection. §12.7.5.2.2 makes a push button respond immediately to input, so a press on it is its
  activation. The rule is limited to push buttons: a text widget lying over the page's text is still
  somewhere a selection may start (ADR 0424; `a_press_over_an_annotation_still_anchors_a_selection`).

## 3. What it leaves

A press on no annotation, followed by a `Moved` onto one, still shows that annotation's rollover,
because nothing records that the button is down when `pressed_on` is empty.

## 4. Driven

`quorra-gtk` under Xvfb with `xdotool mousedown`: the red "Pressed" `/D` appearance is on the screen
mid-press after GTK's `Dragged` and `Moved`, and the release turns to page two. `quorra-qt`: the same.
